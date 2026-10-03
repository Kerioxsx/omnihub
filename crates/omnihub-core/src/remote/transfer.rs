//! File transfers between phone and PC.
//!
//! Phone → PC uploads are chunked and resumable: each chunk is written at its
//! offset into a `.part` file with a CRC-32 check, the server tells the phone
//! how much it already has, and the finished file is hashed (SHA-256) before
//! it is moved into place. Upload state lives next to the partial file, so
//! an upload survives the app restarting.
//!
//! PC → phone transfers are an "inbox" of files the desktop offered; the
//! phone downloads them with HTTP Range support, which its browser resumes.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::events::EventBus;

pub const MAX_CHUNK: usize = 16 * 1024 * 1024;
const PARTIAL_DIR: &str = ".omnihub-partial";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UploadMeta {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub dest_dir: PathBuf,
    pub device_id: String,
    pub device_name: String,
    pub created: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UploadStatus {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FinishedUpload {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    #[error("unknown upload")]
    Unknown,
    #[error("upload belongs to another device")]
    Forbidden,
    #[error("chunk does not start where the upload left off (expected {0})")]
    WrongOffset(u64),
    #[error("chunk is larger than allowed or past the end of the file")]
    TooLarge,
    #[error("chunk checksum mismatch")]
    Checksum,
    #[error("upload is not complete ({0} of {1} bytes)")]
    Incomplete(u64, u64),
    #[error("invalid file name")]
    BadName,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Make a phone-supplied file name safe to create on Windows.
pub fn safe_file_name(name: &str) -> Option<String> {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = base
        .chars()
        .map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .collect();
    let cleaned = cleaned.trim().trim_end_matches(['.', ' ']).to_string();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        return None;
    }
    let stem = cleaned.split('.').next().unwrap_or("").to_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"];
    let cleaned = if reserved.contains(&stem.as_str()) { format!("_{cleaned}") } else { cleaned };
    Some(cleaned.chars().take(200).collect())
}

/// `name.ext`, `name (2).ext`, ... — the first that does not exist.
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    (2..).map(|i| dir.join(format!("{stem} ({i}){ext}"))).find(|p| !p.exists()).unwrap()
}

pub struct Uploads {
    staging: PathBuf,
    events: EventBus,
    last_emit: Mutex<HashMap<String, Instant>>,
    // Serialises writes per upload.
    locks: Mutex<HashMap<String, std::sync::Arc<Mutex<()>>>>,
}

impl Uploads {
    pub fn new(incoming: &Path, events: EventBus) -> Self {
        Uploads { staging: incoming.join(PARTIAL_DIR), events, last_emit: Mutex::new(HashMap::new()), locks: Mutex::new(HashMap::new()) }
    }

    fn meta_path(&self, id: &str) -> PathBuf {
        self.staging.join(format!("{id}.json"))
    }

    fn part_path(&self, id: &str) -> PathBuf {
        self.staging.join(format!("{id}.part"))
    }

    fn valid_id(id: &str) -> bool {
        id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
    }

    fn lock_for(&self, id: &str) -> std::sync::Arc<Mutex<()>> {
        self.locks.lock().entry(id.to_string()).or_default().clone()
    }

    pub fn start(&self, name: &str, size: u64, dest_dir: &Path, device_id: &str, device_name: &str) -> Result<UploadStatus, TransferError> {
        let name = safe_file_name(name).ok_or(TransferError::BadName)?;
        std::fs::create_dir_all(&self.staging)?;
        #[cfg(windows)]
        {
            // Keep the staging folder out of the way in Explorer.
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = self.staging.as_os_str().encode_wide().chain(Some(0)).collect();
            unsafe {
                let _ = windows::Win32::Storage::FileSystem::SetFileAttributesW(windows::core::PCWSTR(wide.as_ptr()), windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN);
            }
        }
        let id = uuid::Uuid::new_v4().to_string();
        let meta = UploadMeta { id: id.clone(), name: name.clone(), size, dest_dir: dest_dir.to_path_buf(), device_id: device_id.into(), device_name: device_name.into(), created: crate::db::now() };
        std::fs::File::create(self.part_path(&id))?;
        crate::settings::write_atomic(&self.meta_path(&id), &serde_json::to_vec(&meta).unwrap())?;
        self.events.emit("transfer:started", serde_json::json!({ "id": id, "direction": "upload", "name": name, "size": size, "device": device_name }));
        Ok(UploadStatus { id, name, size, offset: 0 })
    }

    fn meta(&self, id: &str, device_id: &str) -> Result<UploadMeta, TransferError> {
        if !Self::valid_id(id) {
            return Err(TransferError::Unknown);
        }
        let bytes = std::fs::read(self.meta_path(id)).map_err(|_| TransferError::Unknown)?;
        let meta: UploadMeta = serde_json::from_slice(&bytes).map_err(|_| TransferError::Unknown)?;
        if meta.device_id != device_id {
            return Err(TransferError::Forbidden);
        }
        Ok(meta)
    }

    pub fn status(&self, id: &str, device_id: &str) -> Result<UploadStatus, TransferError> {
        let meta = self.meta(id, device_id)?;
        let offset = std::fs::metadata(self.part_path(id))?.len();
        Ok(UploadStatus { id: meta.id, name: meta.name, size: meta.size, offset })
    }

    /// Append a chunk at `offset` (must equal the current length).
    pub fn write_chunk(&self, id: &str, device_id: &str, offset: u64, data: &[u8], crc: Option<u32>) -> Result<u64, TransferError> {
        let meta = self.meta(id, device_id)?;
        if data.len() > MAX_CHUNK || offset + data.len() as u64 > meta.size {
            return Err(TransferError::TooLarge);
        }
        if let Some(expected) = crc {
            if crc32fast::hash(data) != expected {
                return Err(TransferError::Checksum);
            }
        }
        let lock = self.lock_for(id);
        let _g = lock.lock();
        let mut f = std::fs::OpenOptions::new().write(true).open(self.part_path(id))?;
        let len = f.metadata()?.len();
        if offset != len {
            return Err(TransferError::WrongOffset(len));
        }
        f.seek(SeekFrom::Start(offset))?;
        f.write_all(data)?;
        let new_len = offset + data.len() as u64;
        let due = {
            let mut last = self.last_emit.lock();
            let e = last.entry(id.to_string()).or_insert_with(|| Instant::now() - std::time::Duration::from_secs(1));
            if e.elapsed().as_millis() >= 250 || new_len == meta.size {
                *e = Instant::now();
                true
            } else {
                false
            }
        };
        if due {
            self.events.emit("transfer:progress", serde_json::json!({ "id": id, "direction": "upload", "name": meta.name, "done": new_len, "size": meta.size, "device": meta.device_name }));
        }
        Ok(new_len)
    }

    /// Verify the size, hash the file and move it into its destination.
    pub fn finish(&self, id: &str, device_id: &str) -> Result<FinishedUpload, TransferError> {
        let meta = self.meta(id, device_id)?;
        let lock = self.lock_for(id);
        let _g = lock.lock();
        let part = self.part_path(id);
        let len = std::fs::metadata(&part)?.len();
        if len != meta.size {
            return Err(TransferError::Incomplete(len, meta.size));
        }
        let mut hasher = Sha256::new();
        let mut f = std::fs::File::open(&part)?;
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        drop(f);
        std::fs::create_dir_all(&meta.dest_dir)?;
        let dest = unique_path(&meta.dest_dir, &meta.name);
        if std::fs::rename(&part, &dest).is_err() {
            // Different volume: copy then delete.
            std::fs::copy(&part, &dest)?;
            std::fs::remove_file(&part)?;
        }
        let _ = std::fs::remove_file(self.meta_path(id));
        self.locks.lock().remove(id);
        self.last_emit.lock().remove(id);
        let done = FinishedUpload { id: id.into(), name: dest.file_name().unwrap().to_string_lossy().to_string(), path: dest.to_string_lossy().to_string(), size: len, sha256: hex::encode(hasher.finalize()) };
        self.events.emit("transfer:done", serde_json::json!({ "id": id, "direction": "upload", "name": done.name, "path": done.path, "size": len, "sha256": done.sha256, "device": meta.device_name }));
        Ok(done)
    }

    pub fn cancel(&self, id: &str, device_id: &str) -> Result<(), TransferError> {
        self.meta(id, device_id)?;
        let _ = std::fs::remove_file(self.part_path(id));
        let _ = std::fs::remove_file(self.meta_path(id));
        self.events.emit("transfer:cancelled", serde_json::json!({ "id": id, "direction": "upload" }));
        Ok(())
    }

    /// Remove partial uploads untouched for `max_age_days`.
    pub fn cleanup_stale(&self, max_age_days: u64) {
        let Ok(rd) = std::fs::read_dir(&self.staging) else { return };
        let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(max_age_days * 86_400);
        for e in rd.flatten() {
            if e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t < cutoff) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InboxItem {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub created: i64,
    /// Only this device sees it (all paired devices when `None`).
    pub device_id: Option<String>,
    #[serde(skip)]
    pub path: PathBuf,
}

/// Files offered from the PC to phones.
#[derive(Default)]
pub struct Inbox {
    items: Mutex<Vec<InboxItem>>,
}

impl Inbox {
    pub fn offer(&self, path: &Path, device_id: Option<String>) -> std::io::Result<InboxItem> {
        let meta = std::fs::metadata(path)?;
        if !meta.is_file() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "only files can be sent"));
        }
        let item = InboxItem {
            id: uuid::Uuid::new_v4().to_string(),
            name: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
            size: meta.len(),
            created: crate::db::now(),
            device_id,
            path: path.to_path_buf(),
        };
        let mut items = self.items.lock();
        items.push(item.clone());
        // Keep the newest 200.
        let len = items.len();
        if len > 200 {
            items.drain(..len - 200);
        }
        Ok(item)
    }

    pub fn for_device(&self, device_id: &str) -> Vec<InboxItem> {
        let mut v: Vec<InboxItem> = self.items.lock().iter().filter(|i| i.device_id.as_deref().is_none_or(|d| d == device_id)).cloned().collect();
        v.reverse();
        v
    }

    pub fn all(&self) -> Vec<InboxItem> {
        let mut v = self.items.lock().clone();
        v.reverse();
        v
    }

    pub fn get(&self, id: &str, device_id: &str) -> Option<InboxItem> {
        self.items.lock().iter().find(|i| i.id == id && i.device_id.as_deref().is_none_or(|d| d == device_id)).cloned()
    }

    pub fn remove(&self, id: &str) {
        self.items.lock().retain(|i| i.id != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resumable_upload() {
        let dir = tempfile::tempdir().unwrap();
        let incoming = dir.path().join("incoming");
        let u = Uploads::new(&incoming, EventBus::new());
        let data: Vec<u8> = (0..300_000u32).map(|i| (i % 253) as u8).collect();
        let st = u.start("../../evil\\report.pdf", data.len() as u64, &incoming, "dev1", "Phone").unwrap();
        assert_eq!(st.name, "report.pdf");
        assert!(matches!(u.status(&st.id, "dev2"), Err(TransferError::Forbidden)));

        let (a, b) = data.split_at(100_000);
        assert_eq!(u.write_chunk(&st.id, "dev1", 0, a, Some(crc32fast::hash(a))).unwrap(), 100_000);
        // Retrying an old chunk or skipping ahead is rejected with the real offset.
        assert!(matches!(u.write_chunk(&st.id, "dev1", 0, a, None), Err(TransferError::WrongOffset(100_000))));
        assert!(matches!(u.write_chunk(&st.id, "dev1", 100_000, b, Some(1)), Err(TransferError::Checksum)));
        assert!(matches!(u.finish(&st.id, "dev1"), Err(TransferError::Incomplete(100_000, 300_000))));

        // A new server instance resumes from the files on disk.
        let u2 = Uploads::new(&incoming, EventBus::new());
        assert_eq!(u2.status(&st.id, "dev1").unwrap().offset, 100_000);
        u2.write_chunk(&st.id, "dev1", 100_000, b, Some(crc32fast::hash(b))).unwrap();
        let done = u2.finish(&st.id, "dev1").unwrap();
        assert_eq!(std::fs::read(&done.path).unwrap(), data);
        assert_eq!(done.sha256, hex::encode(Sha256::digest(&data)));
        assert!(matches!(u2.status(&st.id, "dev1"), Err(TransferError::Unknown)));

        // Same name again gets a numbered copy.
        let st = u2.start("report.pdf", 3, &incoming, "dev1", "Phone").unwrap();
        u2.write_chunk(&st.id, "dev1", 0, b"abc", None).unwrap();
        assert!(u2.finish(&st.id, "dev1").unwrap().path.ends_with("report (2).pdf"));
        assert!(matches!(u2.write_chunk("not-an-id", "dev1", 0, b"x", None), Err(TransferError::Unknown)));
    }

    #[test]
    fn file_names() {
        assert_eq!(safe_file_name("a/b/c.txt").as_deref(), Some("c.txt"));
        assert_eq!(safe_file_name("C:\\x\\y?.txt").as_deref(), Some("y_.txt"));
        assert_eq!(safe_file_name("con.txt").as_deref(), Some("_con.txt"));
        assert_eq!(safe_file_name("..").as_deref(), None);
        assert_eq!(safe_file_name("name. ").as_deref(), Some("name"));
    }

    #[test]
    fn inbox_visibility() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("photo.jpg");
        std::fs::write(&f, b"jpg").unwrap();
        let inbox = Inbox::default();
        let all = inbox.offer(&f, None).unwrap();
        let mine = inbox.offer(&f, Some("dev1".into())).unwrap();
        assert_eq!(inbox.for_device("dev1").len(), 2);
        assert_eq!(inbox.for_device("dev2").len(), 1);
        assert!(inbox.get(&mine.id, "dev2").is_none());
        inbox.remove(&all.id);
        assert_eq!(inbox.all().len(), 1);
        assert!(inbox.offer(dir.path(), None).is_err());
    }
}
