//! Hidden files: any file, of any size, moved into OmniHub encrypted and
//! opened only with the vault.
//!
//! ```text
//! "OHHIDE01"
//! key nonce (12) | file key (32 + 16 tag)                 AES-256-GCM(vault key)
//! meta nonce (12) | meta length (u32) | meta (+16 tag)     AES-256-GCM(file key)
//! stream prefix (7)
//! chunks of 1 MiB (+16 tag each)                          AES-256-GCM(file key)
//! ```
//!
//! Every file has a random key of its own, wrapped by the vault key, so the
//! master password can change without touching the files. The contents are
//! encrypted 1 MiB at a time as they stream, so size is limited only by the
//! disk. Chunk nonces are prefix | chunk number | last flag (the STREAM
//! construction): a chunk moved, dropped or cut off fails to decrypt. The
//! name, size and folder live in the encrypted meta; the store itself holds
//! only `<random id>.ohh` files.
//!
//! A file is stored on its own drive (in a hidden `OmniHub Hidden` folder at
//! the drive's root) so a big file on D: doesn't fill C:. Those folders are
//! remembered in `stores.txt` next to the default store.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Component, Path, PathBuf, Prefix};
use std::sync::atomic::{AtomicBool, Ordering};

use aes_gcm::aead::{AeadInPlace, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce, Tag};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use super::crypto::{self, CryptoError};
use super::VaultError;

pub const MAGIC: &[u8; 8] = b"OHHIDE01";
pub const EXT: &str = "ohh";
const CHUNK: usize = 1 << 20;
const TAG: usize = 16;
const MAX_META: usize = 64 << 10;
const AAD_KEY: &[u8] = b"omnihub-hidden-key-v1";
const AAD_META: &[u8] = b"omnihub-hidden-meta-v1";
const AAD_CHUNK: &[u8] = b"omnihub-hidden-data-v1";
/// The folder at the root of other drives.
pub const DRIVE_STORE: &str = "OmniHub Hidden";

/// What the list shows about a hidden file (decrypted from its meta).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HiddenFile {
    pub id: String,
    pub name: String,
    pub size: u64,
    /// The original file's last change (Unix seconds).
    pub modified: i64,
    pub hidden_at: i64,
    /// The folder it was hidden from (where Unhide puts it back).
    pub folder: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    name: String,
    size: u64,
    modified: i64,
    hidden_at: i64,
    folder: String,
}

/// The result of hiding a file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hidden {
    pub file: HiddenFile,
    /// The original couldn't be removed (open in another app): it's still there.
    pub original_left: bool,
}

pub struct Store {
    home: PathBuf,
    open_dir: PathBuf,
}

fn nonce(prefix: &[u8; 7], n: u32, last: bool) -> [u8; 12] {
    let mut out = [0u8; 12];
    out[..7].copy_from_slice(prefix);
    out[7..11].copy_from_slice(&n.to_be_bytes());
    out[11] = last as u8;
    out
}

/// Fill `buf` unless the input ends first; returns how much was read.
fn read_full(r: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]) {
            Ok(0) => break,
            Ok(k) => n += k,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(n)
}

fn cancelled(cancel: &AtomicBool) -> Result<(), VaultError> {
    if cancel.load(Ordering::Relaxed) {
        Err(VaultError::Invalid("Stopped.".into()))
    } else {
        Ok(())
    }
}

fn encrypt(key: &[u8; 32], meta: &Meta, src: impl Read, out: &mut impl Write, progress: &mut dyn FnMut(u64), cancel: &AtomicBool) -> Result<(), VaultError> {
    let file_key = Zeroizing::new(crypto::random_bytes::<32>());
    out.write_all(MAGIC)?;
    out.write_all(&crypto::seal_with(key, file_key.as_ref(), AAD_KEY))?;
    let meta_ct = crypto::seal_with(&file_key, &serde_json::to_vec(meta).map_err(std::io::Error::other)?, AAD_META);
    out.write_all(&(meta_ct.len() as u32).to_le_bytes())?;
    out.write_all(&meta_ct)?;
    let prefix = crypto::random_bytes::<7>();
    out.write_all(&prefix)?;

    let cipher = Aes256Gcm::new_from_slice(file_key.as_ref()).expect("32-byte key");
    let mut src = BufReader::with_capacity(CHUNK, src);
    let mut buf = Zeroizing::new(vec![0u8; CHUNK]);
    let (mut i, mut done) = (0u32, 0u64);
    loop {
        cancelled(cancel)?;
        let n = read_full(&mut src, &mut buf)?;
        let last = n < CHUNK || src.fill_buf()?.is_empty();
        let tag = cipher.encrypt_in_place_detached(Nonce::from_slice(&nonce(&prefix, i, last)), AAD_CHUNK, &mut buf[..n]).expect("AES-GCM encryption cannot fail");
        out.write_all(&buf[..n])?;
        out.write_all(&tag)?;
        done += n as u64;
        progress(done);
        if last {
            return Ok(());
        }
        i = i.checked_add(1).ok_or_else(|| VaultError::Invalid("That file is too big.".into()))?;
    }
}

/// What the start of a hidden file holds.
struct Header {
    file_key: Zeroizing<[u8; 32]>,
    meta: Meta,
    prefix: [u8; 7],
}

/// The file key, meta and stream prefix at the start of a hidden file.
fn read_header(key: &[u8; 32], r: &mut impl Read) -> Result<Header, VaultError> {
    let mut magic = [0u8; 8];
    if read_full(r, &mut magic)? != 8 || &magic != MAGIC {
        return Err(CryptoError::Corrupt.into());
    }
    let mut wrapped = [0u8; 12 + 32 + TAG];
    if read_full(r, &mut wrapped)? != wrapped.len() {
        return Err(CryptoError::Corrupt.into());
    }
    let fk = crypto::open_with(key, &wrapped, AAD_KEY).ok_or(CryptoError::WrongPassword)?;
    let file_key: Zeroizing<[u8; 32]> = Zeroizing::new(fk.as_slice().try_into().map_err(|_| CryptoError::Corrupt)?);
    let mut len = [0u8; 4];
    if read_full(r, &mut len)? != 4 {
        return Err(CryptoError::Corrupt.into());
    }
    let len = u32::from_le_bytes(len) as usize;
    if !(12 + TAG..=MAX_META).contains(&len) {
        return Err(CryptoError::Corrupt.into());
    }
    let mut meta_ct = vec![0u8; len];
    if read_full(r, &mut meta_ct)? != len {
        return Err(CryptoError::Corrupt.into());
    }
    let meta = crypto::open_with(&file_key, &meta_ct, AAD_META).ok_or(CryptoError::Corrupt)?;
    let meta: Meta = serde_json::from_slice(&meta).map_err(|_| CryptoError::Corrupt)?;
    let mut prefix = [0u8; 7];
    if read_full(r, &mut prefix)? != 7 {
        return Err(CryptoError::Corrupt.into());
    }
    Ok(Header { file_key, meta, prefix })
}

fn decrypt(key: &[u8; 32], src: impl Read, out: &mut impl Write, progress: &mut dyn FnMut(u64), cancel: &AtomicBool) -> Result<Meta, VaultError> {
    let mut src = BufReader::with_capacity(CHUNK + TAG, src);
    let Header { file_key, meta, prefix } = read_header(key, &mut src)?;
    let cipher = Aes256Gcm::new_from_slice(file_key.as_ref()).expect("32-byte key");
    let mut buf = Zeroizing::new(vec![0u8; CHUNK + TAG]);
    let (mut i, mut done) = (0u32, 0u64);
    loop {
        cancelled(cancel)?;
        let n = read_full(&mut src, &mut buf)?;
        if n < TAG {
            return Err(CryptoError::Corrupt.into());
        }
        let last = n < CHUNK + TAG || src.fill_buf()?.is_empty();
        let (ct, tag) = buf[..n].split_at_mut(n - TAG);
        cipher.decrypt_in_place_detached(Nonce::from_slice(&nonce(&prefix, i, last)), AAD_CHUNK, ct, Tag::from_slice(tag)).map_err(|_| CryptoError::Corrupt)?;
        out.write_all(ct)?;
        done += ct.len() as u64;
        progress(done);
        if last {
            return Ok(meta);
        }
        i = i.checked_add(1).ok_or(CryptoError::Corrupt)?;
    }
}

/// A file name that is safe to create on Windows.
fn safe_name(name: &str) -> String {
    // The last part, whichever separator a name from another PC used.
    let base = name.rsplit(['/', '\\']).next().unwrap_or_default();
    let clean: String = base.chars().map(|c| if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') { '_' } else { c }).collect();
    let clean = clean.trim().trim_end_matches('.').to_string();
    if clean.is_empty() { "hidden file".into() } else { clean }
}

/// `folder/name`, or `name (2).ext`… when that is taken.
fn free_path(folder: &Path, name: &str) -> PathBuf {
    let first = folder.join(name);
    if !first.exists() {
        return first;
    }
    let p = Path::new(name);
    let (stem, ext) = (p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(), p.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default());
    (2..).map(|i| folder.join(format!("{stem} ({i}){ext}"))).find(|c| !c.exists()).unwrap()
}

/// "D:" for a path on drive D.
fn drive(p: &Path) -> Option<String> {
    match p.components().next()? {
        Component::Prefix(x) => match x.kind() {
            Prefix::Disk(l) | Prefix::VerbatimDisk(l) => Some(format!("{}:", (l as char).to_ascii_uppercase())),
            _ => None,
        },
        _ => None,
    }
}

fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn mtime(meta: &std::fs::Metadata) -> i64 {
    meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64)
}

impl Store {
    /// `home`: the default store; `open_dir`: where Open puts decrypted copies.
    pub fn new(home: PathBuf, open_dir: PathBuf) -> Self {
        Store { home, open_dir }
    }

    fn stores_file(&self) -> PathBuf {
        self.home.join("stores.txt")
    }

    /// Every store folder that exists right now.
    fn stores(&self) -> Vec<PathBuf> {
        let mut v = vec![self.home.clone()];
        if let Ok(s) = std::fs::read_to_string(self.stores_file()) {
            v.extend(s.lines().map(str::trim).filter(|l| !l.is_empty()).map(PathBuf::from));
        }
        v.dedup();
        v.into_iter().filter(|d| d.is_dir()).collect()
    }

    /// Where a file from `src` is stored: its own drive's store, else the default.
    fn store_for(&self, src: &Path) -> PathBuf {
        if let (Some(a), Some(b)) = (drive(src), drive(&self.home)) {
            if a != b {
                let dir = PathBuf::from(format!("{a}\\")).join(DRIVE_STORE);
                if std::fs::create_dir_all(&dir).is_ok() {
                    imp::hide_folder(&dir);
                    let listed = std::fs::read_to_string(self.stores_file()).unwrap_or_default();
                    if !listed.lines().any(|l| Path::new(l.trim()) == dir) && std::fs::create_dir_all(&self.home).is_ok() {
                        let _ = std::fs::write(self.stores_file(), format!("{listed}{}\n", dir.display()));
                    }
                    return dir;
                }
            }
        }
        self.home.clone()
    }

    fn files(&self) -> Vec<PathBuf> {
        self.stores()
            .into_iter()
            .filter_map(|d| std::fs::read_dir(d).ok())
            .flat_map(|rd| rd.flatten().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == EXT) && p.file_stem().is_some_and(|s| valid_id(&s.to_string_lossy())))
            .collect()
    }

    fn find(&self, id: &str) -> Result<PathBuf, VaultError> {
        if !valid_id(id) {
            return Err(VaultError::NotFound);
        }
        self.stores().into_iter().map(|d| d.join(format!("{id}.{EXT}"))).find(|p| p.is_file()).ok_or(VaultError::NotFound)
    }

    /// How many hidden files there are (no key needed).
    pub fn count(&self) -> usize {
        self.files().len()
    }

    /// The hidden files this vault key opens, newest first.
    pub fn list(&self, key: &[u8; 32]) -> Vec<HiddenFile> {
        let mut v: Vec<HiddenFile> = self
            .files()
            .into_iter()
            .filter_map(|p| {
                let id = p.file_stem()?.to_string_lossy().into_owned();
                let mut f = std::fs::File::open(&p).ok()?;
                let m = read_header(key, &mut f).ok()?.meta;
                Some(HiddenFile { id, name: m.name, size: m.size, modified: m.modified, hidden_at: m.hidden_at, folder: m.folder })
            })
            .collect();
        v.sort_by_key(|f| std::cmp::Reverse(f.hidden_at));
        v
    }

    /// Encrypt `src` into the store, then delete the original.
    pub fn hide(&self, key: &[u8; 32], src: &Path, progress: &mut dyn FnMut(u64, u64), cancel: &AtomicBool) -> Result<Hidden, VaultError> {
        let info = std::fs::metadata(src).map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { VaultError::NotFound } else { e.into() })?;
        if info.is_dir() {
            return Err(VaultError::Invalid("That's a folder: zip it first (right-click → Send to → Compressed folder), then hide the zip.".into()));
        }
        let src_full = crate::paths::canonical(src).unwrap_or_else(|_| src.to_path_buf());
        if self.stores().iter().any(|d| crate::paths::canonical(d).is_ok_and(|d| src_full.starts_with(d))) {
            return Err(VaultError::Invalid("That file is already hidden.".into()));
        }
        let dir = self.store_for(&src_full);
        std::fs::create_dir_all(&dir)?;
        let id = uuid::Uuid::new_v4().simple().to_string();
        let meta = Meta {
            name: src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "hidden file".into()),
            size: info.len(),
            modified: mtime(&info),
            hidden_at: crate::db::now(),
            folder: src_full.parent().map(|p| crate::paths::simplify(p.to_path_buf()).display().to_string()).unwrap_or_default(),
        };
        let part = dir.join(format!("{id}.part"));
        let done = dir.join(format!("{id}.{EXT}"));
        let total = info.len();
        let r = (|| {
            let input = std::fs::File::open(src)?;
            let mut out = std::io::BufWriter::with_capacity(CHUNK, std::fs::File::create(&part)?);
            encrypt(key, &meta, input, &mut out, &mut |d| progress(d, total), cancel)?;
            let f = out.into_inner().map_err(|e| e.into_error())?;
            f.sync_all()?;
            drop(f);
            std::fs::rename(&part, &done)?;
            Ok::<_, VaultError>(())
        })();
        if let Err(e) = r {
            let _ = std::fs::remove_file(&part);
            return Err(e);
        }
        let original_left = std::fs::remove_file(src).is_err();
        Ok(Hidden { file: HiddenFile { id, name: meta.name, size: meta.size, modified: meta.modified, hidden_at: meta.hidden_at, folder: meta.folder }, original_left })
    }

    /// Decrypt a hidden file to `dest` (written whole or not at all).
    fn reveal_to(&self, key: &[u8; 32], id: &str, dest: &Path, progress: &mut dyn FnMut(u64, u64), cancel: &AtomicBool) -> Result<(), VaultError> {
        let src = self.find(id)?;
        let total = {
            let mut f = std::fs::File::open(&src)?;
            read_header(key, &mut f)?.meta.size
        };
        let part = dest.with_file_name(format!(".{}.omnihub-part", dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
        let r = (|| {
            let mut out = std::io::BufWriter::with_capacity(CHUNK, std::fs::File::create(&part)?);
            let meta = decrypt(key, std::fs::File::open(&src)?, &mut out, &mut |d| progress(d, total), cancel)?;
            let f = out.into_inner().map_err(|e| e.into_error())?;
            if meta.modified > 0 {
                let _ = f.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(meta.modified as u64));
            }
            f.sync_all()?;
            drop(f);
            std::fs::rename(&part, dest)?;
            Ok::<_, VaultError>(())
        })();
        if r.is_err() {
            let _ = std::fs::remove_file(&part);
        }
        r
    }

    fn meta(&self, key: &[u8; 32], id: &str) -> Result<HiddenFile, VaultError> {
        let p = self.find(id)?;
        let m = read_header(key, &mut std::fs::File::open(p)?)?.meta;
        Ok(HiddenFile { id: id.to_string(), name: m.name, size: m.size, modified: m.modified, hidden_at: m.hidden_at, folder: m.folder })
    }

    /// Put a hidden file back (in `folder`, else where it came from) and
    /// remove it from the store. Returns where it went.
    pub fn unhide(&self, key: &[u8; 32], id: &str, folder: Option<&Path>, progress: &mut dyn FnMut(u64, u64), cancel: &AtomicBool) -> Result<PathBuf, VaultError> {
        let m = self.meta(key, id)?;
        let folder = folder.map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from(&m.folder));
        std::fs::create_dir_all(&folder).map_err(|e| VaultError::Invalid(format!("Couldn't put it back in {}: {e}. Choose another folder.", folder.display())))?;
        let dest = free_path(&folder, &safe_name(&m.name));
        self.reveal_to(key, id, &dest, progress, cancel)?;
        std::fs::remove_file(self.find(id)?)?;
        Ok(dest)
    }

    /// Decrypt a copy to `dest` and keep the file hidden.
    pub fn save_copy(&self, key: &[u8; 32], id: &str, dest: &Path, progress: &mut dyn FnMut(u64, u64), cancel: &AtomicBool) -> Result<(), VaultError> {
        self.reveal_to(key, id, dest, progress, cancel)
    }

    /// Decrypt a copy into the temporary open folder (removed when the vault
    /// locks and when OmniHub starts) and return its path.
    pub fn open_copy(&self, key: &[u8; 32], id: &str, progress: &mut dyn FnMut(u64, u64), cancel: &AtomicBool) -> Result<PathBuf, VaultError> {
        let m = self.meta(key, id)?;
        let dir = self.open_dir.join(uuid::Uuid::new_v4().simple().to_string());
        std::fs::create_dir_all(&dir)?;
        let dest = dir.join(safe_name(&m.name));
        self.reveal_to(key, id, &dest, progress, cancel)?;
        Ok(dest)
    }

    /// Delete a hidden file for good (it is encrypted, so removing it is enough).
    pub fn delete(&self, id: &str) -> Result<(), VaultError> {
        std::fs::remove_file(self.find(id)?)?;
        Ok(())
    }

    /// Remove the copies Open made. Ones still open in an app stay until next time.
    pub fn clean_opened(&self) {
        let Ok(rd) = std::fs::read_dir(&self.open_dir) else { return };
        for e in rd.flatten() {
            let _ = std::fs::remove_dir_all(e.path());
        }
        let _ = std::fs::remove_dir(&self.open_dir);
    }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_SYSTEM};

    /// Hidden + system, so Explorer doesn't show the folder by default.
    pub fn hide_folder(dir: &Path) {
        let w: Vec<u16> = dir.as_os_str().encode_wide().chain([0]).collect();
        unsafe {
            let _ = SetFileAttributesW(PCWSTR(w.as_ptr()), FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM);
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn hide_folder(_dir: &std::path::Path) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_progress(_: u64, _: u64) {}

    fn store(dir: &Path) -> Store {
        Store::new(dir.join("hidden"), dir.join("open"))
    }

    fn round_trip(len: usize) {
        let key = crypto::random_bytes::<32>();
        let data: Vec<u8> = (0..len).map(|i| (i * 31 % 251) as u8).collect();
        let meta = Meta { name: "a.zip".into(), size: len as u64, modified: 0, hidden_at: 1, folder: String::new() };
        let mut out = Vec::new();
        encrypt(&key, &meta, &data[..], &mut out, &mut |_| {}, &AtomicBool::new(false)).unwrap();
        assert_eq!(out.len(), 8 + 60 + 4 + (12 + TAG + serde_json::to_vec(&meta).unwrap().len()) + 7 + len + TAG * len.div_ceil(CHUNK).max(1));
        let mut back = Vec::new();
        let m = decrypt(&key, &out[..], &mut back, &mut |_| {}, &AtomicBool::new(false)).unwrap();
        assert_eq!((m.name.as_str(), back == data), ("a.zip", true), "length {len}");
    }

    #[test]
    fn any_size_round_trips() {
        for len in [0, 1, CHUNK - 1, CHUNK, CHUNK + 1, 3 * CHUNK] {
            round_trip(len);
        }
    }

    #[test]
    fn tampering_reordering_and_cutting_are_caught() {
        let key = crypto::random_bytes::<32>();
        let data = vec![7u8; 2 * CHUNK + 5];
        let meta = Meta { name: "x".into(), size: data.len() as u64, modified: 0, hidden_at: 0, folder: String::new() };
        let mut out = Vec::new();
        encrypt(&key, &meta, &data[..], &mut out, &mut |_| {}, &AtomicBool::new(false)).unwrap();
        let body = out.len() - (data.len() + 3 * TAG);
        let try_open = |b: &[u8]| decrypt(&key, b, &mut Vec::new(), &mut |_| {}, &AtomicBool::new(false)).is_ok();
        assert!(try_open(&out));
        // A flipped bit.
        let mut bad = out.clone();
        bad[body + 10] ^= 1;
        assert!(!try_open(&bad));
        // Cut after the first chunk: that chunk wasn't the last one.
        assert!(!try_open(&out[..body + CHUNK + TAG]));
        // The two full chunks swapped.
        let mut swapped = out[..body].to_vec();
        swapped.extend_from_slice(&out[body + CHUNK + TAG..body + 2 * (CHUNK + TAG)]);
        swapped.extend_from_slice(&out[body..body + CHUNK + TAG]);
        swapped.extend_from_slice(&out[body + 2 * (CHUNK + TAG)..]);
        assert!(!try_open(&swapped));
        // Another vault's key.
        assert!(decrypt(&crypto::random_bytes::<32>(), &out[..], &mut Vec::new(), &mut |_| {}, &AtomicBool::new(false)).is_err());
    }

    #[test]
    fn hide_list_open_unhide_delete() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        let key = crypto::random_bytes::<32>();
        let docs = dir.path().join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        let secret = docs.join("secret plans.zip");
        let data: Vec<u8> = (0..CHUNK + 12345).map(|i| (i % 256) as u8).collect();
        std::fs::write(&secret, &data).unwrap();

        let mut last = (0, 0);
        let h = s.hide(&key, &secret, &mut |d, t| last = (d, t), &AtomicBool::new(false)).unwrap();
        assert!(!secret.exists(), "the original is gone");
        assert!(!h.original_left);
        assert_eq!(last, (data.len() as u64, data.len() as u64));
        assert_eq!((h.file.name.as_str(), h.file.size), ("secret plans.zip", data.len() as u64));
        // Nothing readable in the store: no name, no contents.
        let stored = std::fs::read(s.find(&h.file.id).unwrap()).unwrap();
        assert!(!stored.windows(6).any(|w| w == b"secret"));
        assert!(!stored.windows(64).any(|w| w == &data[1000..1064]));
        assert_eq!(s.list(&key), vec![h.file.clone()]);
        assert!(s.list(&crypto::random_bytes::<32>()).is_empty(), "another vault sees nothing");
        assert_eq!(s.count(), 1);

        let opened = s.open_copy(&key, &h.file.id, &mut no_progress, &AtomicBool::new(false)).unwrap();
        assert_eq!(std::fs::read(&opened).unwrap(), data);
        s.clean_opened();
        assert!(!opened.exists());

        let copy = dir.path().join("copy.zip");
        s.save_copy(&key, &h.file.id, &copy, &mut no_progress, &AtomicBool::new(false)).unwrap();
        assert_eq!(std::fs::read(&copy).unwrap(), data);
        assert_eq!(s.count(), 1, "a copy keeps it hidden");

        // Something new took its name: it comes back beside it.
        std::fs::write(&secret, b"new").unwrap();
        let back = s.unhide(&key, &h.file.id, None, &mut no_progress, &AtomicBool::new(false)).unwrap();
        assert_eq!(back, docs.join("secret plans (2).zip"));
        assert_eq!(std::fs::read(&back).unwrap(), data);
        assert_eq!(s.count(), 0);

        let h2 = s.hide(&key, &back, &mut no_progress, &AtomicBool::new(false)).unwrap();
        assert!(matches!(s.hide(&key, &docs, &mut no_progress, &AtomicBool::new(false)), Err(VaultError::Invalid(_))), "folders are refused");
        s.delete(&h2.file.id).unwrap();
        assert_eq!(s.count(), 0);
        assert!(matches!(s.delete("../../etc"), Err(VaultError::NotFound)));
    }

    #[test]
    fn stopping_leaves_the_original_and_no_scraps() {
        let dir = tempfile::tempdir().unwrap();
        let s = store(dir.path());
        let f = dir.path().join("big.bin");
        std::fs::write(&f, vec![1u8; 3 * CHUNK]).unwrap();
        let stop = AtomicBool::new(false);
        let r = s.hide(&crypto::random_bytes::<32>(), &f, &mut |d, _| stop.store(d >= CHUNK as u64, Ordering::Relaxed), &stop);
        assert!(r.is_err());
        assert!(f.exists());
        assert_eq!(std::fs::read_dir(dir.path().join("hidden")).unwrap().count(), 0);
    }

    #[test]
    fn names_are_made_safe() {
        assert_eq!(safe_name("..\\..\\evil.exe"), "evil.exe");
        assert_eq!(safe_name("a:b?.txt"), "a_b_.txt");
        assert_eq!(safe_name(""), "hidden file");
        assert_eq!(drive(Path::new(r"D:\games\x.zip")).as_deref(), if cfg!(windows) { Some("D:") } else { None });
    }
}
