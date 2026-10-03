//! The local password and secrets vault.
//!
//! Entries are decrypted in memory only while the vault is unlocked; locking
//! (by hand, after idle time, or when Windows locks) drops and zeroes them.

pub mod crypto;
pub mod generator;
pub mod hello;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use crate::events::EventBus;
use crypto::{CryptoError, KdfParams, VaultKeys, FLAG_DPAPI, MAGIC};

pub const MIN_MASTER_LEN: usize = 10;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum EntryKind {
    #[default]
    Login,
    Email,
    Note,
    Card,
    Wifi,
    Other,
}

// Field-level defaults: a container-level `#[serde(default)]` would move
// fields out of a default `Entry`, which its `Drop` impl forbids.
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub kind: EntryKind,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub created: i64,
    #[serde(default)]
    pub updated: i64,
    #[serde(default)]
    pub password_changed: i64,
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry").field("id", &self.id).field("title", &self.title).finish_non_exhaustive()
    }
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.password.zeroize();
        self.notes.zeroize();
        self.username.zeroize();
        self.email.zeroize();
    }
}

/// What the list view shows: everything except the secret fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntrySummary {
    pub id: String,
    pub kind: EntryKind,
    pub title: String,
    pub username: String,
    pub email: String,
    pub url: String,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub updated: i64,
    pub has_password: bool,
    pub has_notes: bool,
    /// Looks like the password of a primary account (Google, Microsoft,
    /// Apple...). The UI warns and suggests passkeys or app passwords.
    pub primary_account: bool,
    pub password_score: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct EntryInput {
    pub id: Option<String>,
    pub kind: EntryKind,
    pub title: String,
    pub username: String,
    pub email: String,
    /// `None` keeps the current password when editing.
    pub password: Option<String>,
    pub url: String,
    pub notes: Option<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub exists: bool,
    pub unlocked: bool,
    pub entries: usize,
    pub dpapi: bool,
    pub hello_available: bool,
    pub hello_enabled: bool,
    pub auto_lock_minutes: u32,
    /// Seconds until auto-lock while unlocked.
    pub locks_in: Option<u64>,
    /// Seconds to wait before the next unlock attempt is accepted.
    pub retry_after: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum VaultError {
    #[error("no vault yet; create one first")]
    Missing,
    #[error("a vault already exists")]
    Exists,
    #[error("the vault is locked")]
    Locked,
    #[error("entry not found")]
    NotFound,
    #[error("the master password must be at least {MIN_MASTER_LEN} characters")]
    WeakPassword,
    #[error("too many attempts; wait {0} s")]
    Throttled(u64),
    #[error("{0}")]
    Crypto(#[from] CryptoError),
    #[error("{0}")]
    Hello(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Serialize, Deserialize, Default)]
struct Payload {
    version: u32,
    entries: Vec<Entry>,
}

struct Open {
    keys: VaultKeys,
    entries: Vec<Entry>,
}

struct Throttle {
    failures: u32,
    until: Option<Instant>,
}

pub struct Vault {
    path: PathBuf,
    hello_path: PathBuf,
    kdf: KdfParams,
    use_dpapi: bool,
    open: Mutex<Option<Open>>,
    last_activity: Mutex<Instant>,
    auto_lock: Mutex<Duration>,
    throttle: Mutex<Throttle>,
    events: EventBus,
}

const PRIMARY_DOMAINS: &[&str] = &[
    "google.com", "gmail.com", "googlemail.com", "accounts.google", "microsoft.com", "live.com", "outlook.com", "hotmail.com",
    "msn.com", "apple.com", "icloud.com", "me.com", "appleid", "yahoo.com", "proton.me", "protonmail.com",
];

pub fn is_primary_account(url: &str, username: &str, email: &str) -> bool {
    let hay = format!("{} {} {}", url, username, email).to_lowercase();
    PRIMARY_DOMAINS.iter().any(|d| hay.contains(d))
}

const HELLO_AAD: &[u8] = b"omnihub-hello-v1";

impl Vault {
    pub fn new(path: &Path, events: EventBus) -> Self {
        Self::with_options(path, events, KdfParams::default(), crate::system::dpapi::available())
    }

    pub fn with_options(path: &Path, events: EventBus, kdf: KdfParams, use_dpapi: bool) -> Self {
        Vault {
            path: path.to_path_buf(),
            hello_path: path.with_extension("hello"),
            kdf,
            use_dpapi,
            open: Mutex::new(None),
            last_activity: Mutex::new(Instant::now()),
            auto_lock: Mutex::new(Duration::from_secs(300)),
            throttle: Mutex::new(Throttle { failures: 0, until: None }),
            events,
        }
    }

    pub fn set_auto_lock(&self, minutes: u32) {
        *self.auto_lock.lock() = Duration::from_secs(minutes.max(1) as u64 * 60);
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn is_unlocked(&self) -> bool {
        self.open.lock().is_some()
    }

    pub fn status(&self) -> VaultStatus {
        let open = self.open.lock();
        let auto = *self.auto_lock.lock();
        let idle = self.last_activity.lock().elapsed();
        VaultStatus {
            exists: self.exists(),
            unlocked: open.is_some(),
            entries: open.as_ref().map_or(0, |o| o.entries.len()),
            dpapi: self.use_dpapi,
            hello_available: hello::supported(),
            hello_enabled: self.hello_path.exists(),
            auto_lock_minutes: (auto.as_secs() / 60) as u32,
            locks_in: open.as_ref().map(|_| auto.saturating_sub(idle).as_secs()),
            retry_after: self.throttle.lock().until.map_or(0, |u| u.saturating_duration_since(Instant::now()).as_secs()),
        }
    }

    pub fn touch(&self) {
        *self.last_activity.lock() = Instant::now();
    }

    /// Lock if idle for longer than the auto-lock time. Call periodically.
    pub fn tick(&self) {
        let idle = self.last_activity.lock().elapsed();
        if self.is_unlocked() && idle >= *self.auto_lock.lock() {
            self.lock_with_reason("idle");
        }
    }

    pub fn lock(&self) {
        self.lock_with_reason("manual");
    }

    pub fn lock_with_reason(&self, reason: &str) {
        if self.open.lock().take().is_some() {
            self.events.emit("vault:locked", serde_json::json!({ "reason": reason }));
        }
    }

    fn read_file(&self) -> Result<(u8, Zeroizing<Vec<u8>>), VaultError> {
        let raw = std::fs::read(&self.path).map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { VaultError::Missing } else { e.into() })?;
        if raw.len() < 9 || &raw[..8] != MAGIC {
            return Err(CryptoError::Corrupt.into());
        }
        let flags = raw[8];
        let body = if flags & FLAG_DPAPI != 0 {
            crate::system::dpapi::unprotect(&raw[9..], MAGIC).map_err(|_| CryptoError::Dpapi)?
        } else {
            Zeroizing::new(raw[9..].to_vec())
        };
        Ok((flags, body))
    }

    fn write_file(&self, open: &Open) -> Result<(), VaultError> {
        let payload = Zeroizing::new(serde_json::to_vec(&PayloadRef { version: 1, entries: &open.entries }).map_err(std::io::Error::other)?);
        let body = crypto::encode_body(&open.keys, &payload);
        let mut out = MAGIC.to_vec();
        if self.use_dpapi {
            out.push(FLAG_DPAPI);
            out.extend(crate::system::dpapi::protect(&body, MAGIC)?);
        } else {
            out.push(0);
            out.extend(body);
        }
        crate::settings::write_atomic(&self.path, &out)?;
        Ok(())
    }

    pub fn create(&self, password: &str) -> Result<(), VaultError> {
        if self.exists() {
            return Err(VaultError::Exists);
        }
        if password.chars().count() < MIN_MASTER_LEN {
            return Err(VaultError::WeakPassword);
        }
        let keys = VaultKeys::create(password, self.kdf)?;
        let open = Open { keys, entries: Vec::new() };
        self.write_file(&open)?;
        *self.open.lock() = Some(open);
        self.touch();
        self.events.emit("vault:unlocked", serde_json::json!({}));
        Ok(())
    }

    fn check_throttle(&self) -> Result<(), VaultError> {
        let t = self.throttle.lock();
        if let Some(until) = t.until {
            let now = Instant::now();
            if until > now {
                return Err(VaultError::Throttled((until - now).as_secs().max(1)));
            }
        }
        Ok(())
    }

    fn note_failure(&self) {
        let mut t = self.throttle.lock();
        t.failures += 1;
        if t.failures >= 3 {
            let secs = 2u64.saturating_pow(t.failures - 2).min(60);
            t.until = Some(Instant::now() + Duration::from_secs(secs));
        }
    }

    pub fn unlock(&self, password: &str) -> Result<(), VaultError> {
        self.check_throttle()?;
        let (_, body) = self.read_file()?;
        match crypto::decode_body(&body, password) {
            Ok((keys, data)) => {
                let payload: Payload = serde_json::from_slice(&data).map_err(|_| CryptoError::Corrupt)?;
                *self.open.lock() = Some(Open { keys, entries: payload.entries });
                *self.throttle.lock() = Throttle { failures: 0, until: None };
                self.touch();
                self.events.emit("vault:unlocked", serde_json::json!({}));
                Ok(())
            }
            Err(CryptoError::WrongPassword) => {
                self.note_failure();
                Err(CryptoError::WrongPassword.into())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Check the master password without changing the lock state (used to
    /// authorise phone access).
    pub fn verify_password(&self, password: &str) -> Result<(), VaultError> {
        self.check_throttle()?;
        let (_, body) = self.read_file()?;
        match crypto::decode_body(&body, password) {
            Ok(_) => {
                *self.throttle.lock() = Throttle { failures: 0, until: None };
                Ok(())
            }
            Err(CryptoError::WrongPassword) => {
                self.note_failure();
                Err(CryptoError::WrongPassword.into())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Decrypt the entries with the master password without unlocking the
    /// vault here (phone sessions get their own short-lived copy).
    pub fn open_snapshot(&self, password: &str) -> Result<Vec<Entry>, VaultError> {
        self.check_throttle()?;
        let (_, body) = self.read_file()?;
        match crypto::decode_body(&body, password) {
            Ok((_, data)) => {
                *self.throttle.lock() = Throttle { failures: 0, until: None };
                let payload: Payload = serde_json::from_slice(&data).map_err(|_| CryptoError::Corrupt)?;
                Ok(payload.entries)
            }
            Err(CryptoError::WrongPassword) => {
                self.note_failure();
                Err(CryptoError::WrongPassword.into())
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn summarize(e: &Entry) -> EntrySummary {
        Self::summary(e)
    }

    pub fn change_password(&self, old: &str, new: &str) -> Result<(), VaultError> {
        if new.chars().count() < MIN_MASTER_LEN {
            return Err(VaultError::WeakPassword);
        }
        self.verify_password(old)?;
        let mut guard = self.open.lock();
        let open = guard.as_mut().ok_or(VaultError::Locked)?;
        open.keys = open.keys.rewrap(new)?;
        self.write_file(open)?;
        Ok(())
    }

    fn with_open<T>(&self, f: impl FnOnce(&mut Open) -> Result<T, VaultError>) -> Result<T, VaultError> {
        let mut guard = self.open.lock();
        let open = guard.as_mut().ok_or(VaultError::Locked)?;
        self.touch();
        f(open)
    }

    fn summary(e: &Entry) -> EntrySummary {
        EntrySummary {
            id: e.id.clone(),
            kind: e.kind,
            title: e.title.clone(),
            username: e.username.clone(),
            email: e.email.clone(),
            url: e.url.clone(),
            tags: e.tags.clone(),
            favorite: e.favorite,
            updated: e.updated,
            has_password: !e.password.is_empty(),
            has_notes: !e.notes.is_empty(),
            primary_account: is_primary_account(&e.url, &e.username, &e.email),
            password_score: if e.password.is_empty() { 0 } else { generator::strength(&e.password).score },
        }
    }

    pub fn list(&self) -> Result<Vec<EntrySummary>, VaultError> {
        self.with_open(|o| {
            let mut v: Vec<EntrySummary> = o.entries.iter().map(Self::summary).collect();
            v.sort_by(|a, b| b.favorite.cmp(&a.favorite).then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase())));
            Ok(v)
        })
    }

    pub fn get(&self, id: &str) -> Result<Entry, VaultError> {
        self.with_open(|o| o.entries.iter().find(|e| e.id == id).cloned().ok_or(VaultError::NotFound))
    }

    pub fn save(&self, input: EntryInput) -> Result<EntrySummary, VaultError> {
        let now = crate::db::now();
        self.with_open(|o| {
            let idx = match input.id.as_ref().filter(|i| !i.is_empty()) {
                Some(id) => o.entries.iter().position(|e| &e.id == id).ok_or(VaultError::NotFound)?,
                None => {
                    let mut fresh = Entry::default();
                    fresh.id = uuid::Uuid::new_v4().to_string();
                    fresh.created = now;
                    o.entries.push(fresh);
                    o.entries.len() - 1
                }
            };
            let e = &mut o.entries[idx];
            e.kind = input.kind;
            e.title = input.title.trim().to_string();
            e.username = input.username.trim().to_string();
            e.email = input.email.trim().to_string();
            e.url = input.url.trim().to_string();
            e.tags = input.tags.iter().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
            e.favorite = input.favorite;
            if let Some(pw) = input.password {
                if pw != e.password {
                    e.password = pw;
                    e.password_changed = now;
                }
            }
            if let Some(n) = input.notes {
                e.notes = n;
            }
            e.updated = now;
            let summary = Self::summary(e);
            self.write_file(o)?;
            Ok(summary)
        })
    }

    pub fn delete(&self, id: &str) -> Result<(), VaultError> {
        self.with_open(|o| {
            let before = o.entries.len();
            o.entries.retain(|e| e.id != id);
            if o.entries.len() == before {
                return Err(VaultError::NotFound);
            }
            self.write_file(o)
        })
    }

    /// Copy one secret field to the clipboard, cleared after `clear_after`.
    pub fn copy_field(&self, id: &str, field: &str, clear_after: Duration) -> Result<(), VaultError> {
        let entry = self.get(id)?;
        let value = match field {
            "password" => entry.password.clone(),
            "username" => entry.username.clone(),
            "email" => entry.email.clone(),
            "url" => entry.url.clone(),
            "notes" => entry.notes.clone(),
            _ => return Err(VaultError::NotFound),
        };
        let value = Zeroizing::new(value);
        crate::system::clipboard::copy_secret(&value, clear_after)?;
        Ok(())
    }

    /// Encrypted, password-protected backup (without the DPAPI layer, so it
    /// can be restored on another PC with the master password).
    pub fn export_backup(&self, dest: &Path) -> Result<(), VaultError> {
        self.with_open(|o| {
            let payload = Zeroizing::new(serde_json::to_vec(&PayloadRef { version: 1, entries: &o.entries }).map_err(std::io::Error::other)?);
            let mut out = MAGIC.to_vec();
            out.push(0);
            out.extend(crypto::encode_body(&o.keys, &payload));
            crate::settings::write_atomic(dest, &out)?;
            Ok(())
        })
    }

    /// Merge entries from a backup (decrypted with its own master password).
    pub fn import_backup(&self, src: &Path, password: &str) -> Result<usize, VaultError> {
        let raw = std::fs::read(src)?;
        if raw.len() < 9 || &raw[..8] != MAGIC || raw[8] & FLAG_DPAPI != 0 {
            return Err(CryptoError::Corrupt.into());
        }
        let (_, data) = crypto::decode_body(&raw[9..], password)?;
        let payload: Payload = serde_json::from_slice(&data).map_err(|_| CryptoError::Corrupt)?;
        self.with_open(|o| {
            let mut added = 0;
            for e in payload.entries {
                if !o.entries.iter().any(|x| x.id == e.id) {
                    o.entries.push(e);
                    added += 1;
                }
            }
            self.write_file(o)?;
            Ok(added)
        })
    }

    pub fn enable_hello(&self) -> Result<(), VaultError> {
        let key = self.with_open(|o| Ok(o.keys.key.clone()))?;
        let challenge = crypto::random_bytes::<32>();
        let secret = hello::create_secret(&challenge).map_err(VaultError::Hello)?;
        let mut file = challenge.to_vec();
        file.extend(crypto::seal_with(&secret, key.as_ref(), HELLO_AAD));
        let file = if self.use_dpapi { crate::system::dpapi::protect(&file, b"omnihub-hello")? } else { file };
        crate::settings::write_atomic(&self.hello_path, &file)?;
        Ok(())
    }

    pub fn disable_hello(&self) {
        let _ = std::fs::remove_file(&self.hello_path);
        hello::delete();
    }

    pub fn unlock_with_hello(&self) -> Result<(), VaultError> {
        let raw = std::fs::read(&self.hello_path).map_err(|_| VaultError::Hello("Windows Hello is not enabled for the vault".into()))?;
        let raw = if self.use_dpapi { crate::system::dpapi::unprotect(&raw, b"omnihub-hello").map_err(|_| CryptoError::Dpapi)? } else { Zeroizing::new(raw) };
        if raw.len() < 32 {
            return Err(CryptoError::Corrupt.into());
        }
        let (challenge, sealed) = raw.split_at(32);
        let secret = hello::secret(challenge).map_err(VaultError::Hello)?;
        let key = crypto::open_with(&secret, sealed, HELLO_AAD).ok_or_else(|| VaultError::Hello("Windows Hello key changed; unlock with your master password and enable it again".into()))?;
        let key: [u8; 32] = key.as_slice().try_into().map_err(|_| CryptoError::Corrupt)?;
        let (_, body) = self.read_file()?;
        let (keys, data) = crypto::decode_body_with_key(&body, &key)?;
        let payload: Payload = serde_json::from_slice(&data).map_err(|_| CryptoError::Corrupt)?;
        *self.open.lock() = Some(Open { keys, entries: payload.entries });
        self.touch();
        self.events.emit("vault:unlocked", serde_json::json!({ "via": "hello" }));
        Ok(())
    }

    /// Delete the vault file (after the user confirmed; requires unlock).
    pub fn destroy(&self) -> Result<(), VaultError> {
        if !self.is_unlocked() {
            return Err(VaultError::Locked);
        }
        self.lock();
        self.disable_hello();
        std::fs::remove_file(&self.path)?;
        Ok(())
    }
}

#[derive(Serialize)]
struct PayloadRef<'a> {
    version: u32,
    entries: &'a [Entry],
}

/// Is the interactive Windows session locked (lock screen showing)?
#[cfg(windows)]
pub fn session_locked() -> bool {
    use windows::Win32::System::RemoteDesktop::{WTSFreeMemory, WTSQuerySessionInformationW, WTSSessionInfoEx, WTSINFOEXW, WTS_CURRENT_SERVER_HANDLE, WTS_CURRENT_SESSION};
    unsafe {
        let mut buf = windows::core::PWSTR::null();
        let mut len = 0u32;
        if WTSQuerySessionInformationW(Some(WTS_CURRENT_SERVER_HANDLE), WTS_CURRENT_SESSION, WTSSessionInfoEx, &mut buf, &mut len).is_err() || buf.is_null() {
            return false;
        }
        let info = &*(buf.0 as *const WTSINFOEXW);
        // WTS_SESSIONSTATE_LOCK = 0 (Windows 8 and later).
        let locked = info.Level == 1 && info.Data.WTSInfoExLevel1.SessionFlags == 0;
        WTSFreeMemory(buf.0 as *mut _);
        locked
    }
}

#[cfg(not(windows))]
pub fn session_locked() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault(dir: &Path) -> Vault {
        Vault::with_options(&dir.join("v.ohv"), EventBus::new(), KdfParams { m_cost: 256, t_cost: 1, p_cost: 1 }, false)
    }

    #[test]
    fn lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let v = vault(dir.path());
        assert!(matches!(v.unlock("x"), Err(VaultError::Missing)));
        assert!(matches!(v.create("short"), Err(VaultError::WeakPassword)));
        v.create("a long master password").unwrap();
        assert!(matches!(v.create("a long master password"), Err(VaultError::Exists)));
        let s = v
            .save(EntryInput {
                title: "Google".into(),
                email: "me@gmail.com".into(),
                password: Some("hunter2hunter2".into()),
                tags: vec!["mail".into(), " ".into()],
                ..Default::default()
            })
            .unwrap();
        assert!(s.primary_account);
        assert_eq!(s.tags, vec!["mail"]);
        v.save(EntryInput { title: "Router".into(), kind: EntryKind::Wifi, password: Some("p".into()), favorite: true, ..Default::default() }).unwrap();
        let list = v.list().unwrap();
        assert_eq!(list[0].title, "Router", "favourites first");
        v.lock();
        assert!(matches!(v.list(), Err(VaultError::Locked)));
        // Nothing readable in the file.
        let raw = std::fs::read(dir.path().join("v.ohv")).unwrap();
        assert!(!raw.windows(8).any(|w| w == b"hunter2h"));
        assert!(!raw.windows(6).any(|w| w == b"Google"));

        assert!(v.unlock("wrong password!").is_err());
        v.unlock("a long master password").unwrap();
        let e = v.get(&s.id).unwrap();
        assert_eq!(e.password, "hunter2hunter2");
        // Editing without a password keeps it.
        v.save(EntryInput { id: Some(s.id.clone()), title: "Google account".into(), ..Default::default() }).unwrap();
        assert_eq!(v.get(&s.id).unwrap().password, "hunter2hunter2");
        v.delete(&s.id).unwrap();
        assert_eq!(v.list().unwrap().len(), 1);

        v.change_password("a long master password", "another long password").unwrap();
        v.lock();
        assert!(v.unlock("a long master password").is_err());
        // Throttling kicks in after repeated failures.
        let _ = v.unlock("bad 2");
        let _ = v.unlock("bad 3");
        assert!(matches!(v.unlock("another long password"), Err(VaultError::Throttled(_))));
    }

    #[test]
    fn auto_lock_and_backup() {
        let dir = tempfile::tempdir().unwrap();
        let v = vault(dir.path());
        v.create("a long master password").unwrap();
        v.save(EntryInput { title: "A".into(), password: Some("1".into()), ..Default::default() }).unwrap();
        let backup = dir.path().join("backup.ohv");
        v.export_backup(&backup).unwrap();

        *v.auto_lock.lock() = Duration::from_millis(1);
        std::thread::sleep(Duration::from_millis(5));
        v.tick();
        assert!(!v.is_unlocked());

        let dir2 = tempfile::tempdir().unwrap();
        let v2 = vault(dir2.path());
        v2.create("second vault password").unwrap();
        assert_eq!(v2.import_backup(&backup, "a long master password").unwrap(), 1);
        assert_eq!(v2.import_backup(&backup, "a long master password").unwrap(), 0, "no duplicates");
        assert_eq!(v2.list().unwrap()[0].title, "A");
    }

    #[test]
    fn primary_detection() {
        assert!(is_primary_account("https://accounts.google.com", "", ""));
        assert!(is_primary_account("", "someone@outlook.com", ""));
        assert!(!is_primary_account("https://github.com", "octocat", ""));
    }
}
