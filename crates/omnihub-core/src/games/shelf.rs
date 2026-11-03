//! The game library's own notes on this PC (`library.json`): each game's
//! status (playing, backlog, finished, dropped), favourite, notes, custom
//! artwork, save folders and automatic backups; play sessions (from any
//! launcher, as the game's program runs) for recaps and playtime; and your
//! Steam Web API and SteamGridDB keys, encrypted for this Windows account.
//! Save backups are zips in `save-backups/<game>/`, the newest 10 kept.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ShelfEntry {
    /// "", "playing", "backlog", "finished" or "dropped".
    pub status: String,
    pub favorite: bool,
    pub notes: String,
    /// Custom artwork: a file name in `library-art/` or an https URL.
    pub cover: Option<String>,
    pub hero: Option<String>,
    pub logo: Option<String>,
    /// Folders backed up as this game's saves.
    pub save_dirs: Vec<String>,
    /// Back the saves up when a session ends.
    pub auto_backup: bool,
    /// Daily limit in minutes (0: none).
    pub daily_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRec {
    pub key: String,
    pub name: String,
    /// Unix ms.
    pub start: i64,
    pub end: i64,
}

impl SessionRec {
    pub fn minutes(&self) -> u64 {
        ((self.end - self.start).max(0) / 60_000) as u64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
struct ShelfData {
    entries: HashMap<String, ShelfEntry>,
    sessions: Vec<SessionRec>,
    /// DPAPI-protected, base64.
    steam_key: String,
    sgdb_key: String,
    /// Wishlist discounts already told about (app → %).
    sale_seen: HashMap<u32, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub id: String,
    pub at: i64,
    pub bytes: u64,
    pub files: u32,
}

/// What a page can change (unset fields stay).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ShelfPatch {
    pub status: Option<String>,
    pub favorite: Option<bool>,
    pub notes: Option<String>,
    pub auto_backup: Option<bool>,
    pub save_dirs: Option<Vec<String>>,
    pub daily_limit: Option<u32>,
}

const ENTROPY: &[u8] = b"omnihub-library-key";
const KEEP_BACKUPS: usize = 10;
const MAX_BACKUP: u64 = 4 << 30;
const IMAGE_TYPES: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

pub struct Shelf {
    file: PathBuf,
    pub art_dir: PathBuf,
    backups: PathBuf,
    data: Mutex<ShelfData>,
    /// Files the pages may show or play, by token.
    media: Mutex<HashMap<String, PathBuf>>,
    events: EventBus,
}

/// A name safe for a folder ("steam:730" → "steam_730").
pub fn safe(key: &str) -> String {
    key.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).take(80).collect()
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

impl Shelf {
    pub fn new(data_dir: &Path, events: EventBus) -> Shelf {
        let file = data_dir.join("library.json");
        let data = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Shelf { file, art_dir: data_dir.join("library-art"), backups: data_dir.join("save-backups"), data: Mutex::new(data), media: Mutex::new(HashMap::new()), events }
    }

    fn change<T>(&self, f: impl FnOnce(&mut ShelfData) -> Result<T, String>) -> Result<T, String> {
        let (out, snap) = {
            let mut d = self.data.lock();
            let out = f(&mut d)?;
            (out, d.clone())
        };
        let json = serde_json::to_vec_pretty(&snap).map_err(|e| e.to_string())?;
        crate::settings::write_atomic(&self.file, &json).map_err(|e| e.to_string())?;
        self.events.emit("library:changed", serde_json::json!({}));
        Ok(out)
    }

    pub fn entries(&self) -> HashMap<String, ShelfEntry> {
        self.data.lock().entries.clone()
    }

    pub fn entry(&self, key: &str) -> ShelfEntry {
        self.data.lock().entries.get(key).cloned().unwrap_or_default()
    }

    pub fn patch(&self, key: &str, p: ShelfPatch) -> Result<ShelfEntry, String> {
        if let Some(s) = &p.status {
            if !["", "playing", "backlog", "finished", "dropped"].contains(&s.as_str()) {
                return Err("Unknown status".into());
            }
        }
        self.change(|d| {
            let e = d.entries.entry(key.to_string()).or_default();
            if let Some(v) = p.status {
                e.status = v;
            }
            if let Some(v) = p.favorite {
                e.favorite = v;
            }
            if let Some(v) = p.notes {
                e.notes = v.chars().take(4000).collect();
            }
            if let Some(v) = p.auto_backup {
                e.auto_backup = v;
            }
            if let Some(v) = p.save_dirs {
                e.save_dirs = v.into_iter().map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).take(20).collect();
            }
            if let Some(v) = p.daily_limit {
                e.daily_limit = v.min(24 * 60);
            }
            Ok(e.clone())
        })
    }

    /// Use a picture (copied in) or an https URL for a game's cover, hero or logo; `None` goes back to the default.
    pub fn set_art(&self, key: &str, kind: &str, src: Option<&str>) -> Result<ShelfEntry, String> {
        if !["cover", "hero", "logo"].contains(&kind) {
            return Err("Unknown kind of artwork".into());
        }
        let value = match src {
            None => None,
            Some(u) if u.starts_with("https://") => Some(u.to_string()),
            Some(path) => {
                let p = Path::new(path);
                let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                if !IMAGE_TYPES.contains(&ext.as_str()) {
                    return Err("Use a PNG, JPG, WebP or GIF picture".into());
                }
                if std::fs::metadata(p).map_err(|e| e.to_string())?.len() > 20 << 20 {
                    return Err("That picture is over 20 MB".into());
                }
                std::fs::create_dir_all(&self.art_dir).map_err(|e| e.to_string())?;
                let name = format!("{}-{kind}-{}.{ext}", safe(key), now_ms());
                std::fs::copy(p, self.art_dir.join(&name)).map_err(|e| e.to_string())?;
                Some(name)
            }
        };
        let (old, e) = self.change(|d| {
            let e = d.entries.entry(key.to_string()).or_default();
            let slot = match kind {
                "cover" => &mut e.cover,
                "hero" => &mut e.hero,
                _ => &mut e.logo,
            };
            let old = std::mem::replace(slot, value.clone());
            Ok((old, e.clone()))
        })?;
        if let Some(old) = old.filter(|o| !o.starts_with("https://")) {
            let _ = std::fs::remove_file(self.art_dir.join(old));
        }
        Ok(e)
    }

    /// A file the pages may show (picture, screenshot, track): a token for `gamemedia://`.
    pub fn media_token(&self, path: &Path) -> String {
        let token = blake3::hash(path.to_string_lossy().as_bytes()).to_hex()[..20].to_string();
        let mut m = self.media.lock();
        if m.len() > 20_000 {
            m.clear();
        }
        m.insert(token.clone(), path.to_path_buf());
        token
    }

    pub fn media_path(&self, token: &str) -> Option<PathBuf> {
        self.media.lock().get(token).cloned()
    }

    /// Where custom art is shown from: an https URL, or a token for a file here.
    pub fn art_src(&self, value: &str) -> Option<String> {
        if value.starts_with("https://") {
            return Some(value.to_string());
        }
        let p = self.art_dir.join(value);
        p.is_file().then(|| format!("gamemedia://localhost/{}", self.media_token(&p)))
    }

    // ---------- keys ----------

    fn secret(&self, which: &str) -> Option<String> {
        use base64::Engine;
        let d = self.data.lock();
        let b64 = if which == "steam" { &d.steam_key } else { &d.sgdb_key };
        if b64.is_empty() {
            return None;
        }
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
        let plain = crate::system::dpapi::unprotect(&bytes, ENTROPY).ok()?;
        String::from_utf8(plain.to_vec()).ok()
    }

    pub fn steam_key(&self) -> Option<String> {
        self.secret("steam")
    }

    pub fn sgdb_key(&self) -> Option<String> {
        self.secret("sgdb")
    }

    /// Keep (or with "" remove) your Steam Web API key or SteamGridDB key, encrypted for this account.
    pub fn set_key(&self, which: &str, key: &str) -> Result<(), String> {
        use base64::Engine;
        let key = key.trim();
        if key.len() > 200 || key.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err("That doesn't look like a key".into());
        }
        if which == "steam" && !key.is_empty() && !(key.len() == 32 && key.chars().all(|c| c.is_ascii_hexdigit())) {
            return Err("A Steam Web API key is 32 letters and digits (steamcommunity.com/dev/apikey)".into());
        }
        let enc = if key.is_empty() { String::new() } else { base64::engine::general_purpose::STANDARD.encode(crate::system::dpapi::protect(key.as_bytes(), ENTROPY).map_err(|e| e.to_string())?) };
        self.change(|d| {
            match which {
                "steam" => d.steam_key = enc,
                "sgdb" => d.sgdb_key = enc,
                _ => return Err("Unknown key".into()),
            }
            Ok(())
        })
    }

    pub fn has_key(&self, which: &str) -> bool {
        let d = self.data.lock();
        !(if which == "steam" { &d.steam_key } else { &d.sgdb_key }).is_empty()
    }

    // ---------- sessions ----------

    pub fn record(&self, s: SessionRec) -> Result<(), String> {
        if s.end - s.start < 60_000 {
            return Ok(()); // under a minute: not a session
        }
        self.change(|d| {
            d.sessions.push(s);
            let n = d.sessions.len();
            if n > 5000 {
                d.sessions.drain(..n - 5000);
            }
            Ok(())
        })
    }

    /// Sessions (newest first), for one game or all.
    pub fn sessions(&self, key: Option<&str>, limit: usize) -> Vec<SessionRec> {
        self.data.lock().sessions.iter().rev().filter(|s| key.is_none_or(|k| s.key == k)).take(limit).cloned().collect()
    }

    /// Minutes played (tracked here) and the last session's end, by game.
    pub fn totals(&self) -> HashMap<String, (u64, i64)> {
        let mut out: HashMap<String, (u64, i64)> = HashMap::new();
        for s in &self.data.lock().sessions {
            let e = out.entry(s.key.clone()).or_default();
            e.0 += s.minutes();
            e.1 = e.1.max(s.end);
        }
        out
    }

    /// Minutes played today (local time), including a session running since `running_since`.
    pub fn minutes_today(&self, key: &str, running_since: Option<i64>) -> u64 {
        let midnight = chrono::Local::now().date_naive().and_hms_opt(0, 0, 0).and_then(|t| t.and_local_timezone(chrono::Local).single()).map_or(0, |t| t.timestamp_millis());
        let done: i64 = self.data.lock().sessions.iter().filter(|s| s.key == key && s.end > midnight).map(|s| s.end - s.start.max(midnight)).sum();
        let live = running_since.map_or(0, |st| now_ms() - st.max(midnight));
        ((done + live).max(0) / 60_000) as u64
    }

    // ---------- wishlist sales ----------

    /// The discounts not told about yet (and remembers them).
    pub fn new_sales(&self, items: &[super::steam::WishItem]) -> Vec<super::steam::WishItem> {
        let fresh: Vec<_> = {
            let d = self.data.lock();
            items.iter().filter(|w| w.discount > 0 && d.sale_seen.get(&w.app).copied().unwrap_or(0) < w.discount).cloned().collect()
        };
        let _ = self.change(|d| {
            d.sale_seen = items.iter().filter(|w| w.discount > 0).map(|w| (w.app, w.discount.max(d.sale_seen.get(&w.app).copied().unwrap_or(0)))).collect();
            Ok(())
        });
        fresh
    }

    // ---------- save backups ----------

    fn backup_dir(&self, key: &str) -> PathBuf {
        self.backups.join(safe(key))
    }

    pub fn backups(&self, key: &str) -> Vec<Backup> {
        let Ok(rd) = std::fs::read_dir(self.backup_dir(key)) else { return Vec::new() };
        let mut v: Vec<Backup> = rd
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let id = name.strip_suffix(".zip")?.to_string();
                let at = chrono::NaiveDateTime::parse_from_str(id.split('-').take(2).collect::<Vec<_>>().join("-").as_str(), "%Y%m%d-%H%M%S").ok()?.and_local_timezone(chrono::Local).single()?.timestamp_millis();
                let files = zip::ZipArchive::new(std::fs::File::open(e.path()).ok()?).map(|z| z.len().saturating_sub(1) as u32).unwrap_or(0);
                Some(Backup { id, at, bytes: e.metadata().map(|m| m.len()).unwrap_or(0), files })
            })
            .collect();
        v.sort_by_key(|b| std::cmp::Reverse(b.at));
        v
    }

    /// Zip a game's save folders. `tag` ends the backup's name ("auto", "before-restore").
    pub fn backup(&self, key: &str, tag: &str) -> Result<Backup, String> {
        let dirs = self.entry(key).save_dirs;
        if dirs.is_empty() {
            return Err("Pick this game's save folder first".into());
        }
        let out_dir = self.backup_dir(key);
        std::fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
        let id = format!("{}-{}", chrono::Local::now().format("%Y%m%d-%H%M%S"), safe(tag));
        let path = out_dir.join(format!("{id}.zip"));
        let tmp = out_dir.join(format!("{id}.zip.part"));
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&tmp).map_err(|e| e.to_string())?);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let (mut total, mut files) = (0u64, 0u32);
        for (i, dir) in dirs.iter().enumerate() {
            let root = Path::new(dir);
            if !root.is_dir() {
                continue;
            }
            for e in walkdir::WalkDir::new(root).into_iter().flatten().filter(|e| e.file_type().is_file()) {
                let rel = e.path().strip_prefix(root).map_err(|e| e.to_string())?;
                total += e.metadata().map(|m| m.len()).unwrap_or(0);
                if total > MAX_BACKUP {
                    drop(zip);
                    let _ = std::fs::remove_file(&tmp);
                    return Err("Those saves are over 4 GB: pick the save folder itself".into());
                }
                zip.start_file(format!("{i}/{}", rel.to_string_lossy().replace('\\', "/")), opts).map_err(|e| e.to_string())?;
                let mut f = std::fs::File::open(e.path()).map_err(|e| e.to_string())?;
                std::io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
                files += 1;
            }
        }
        if files == 0 {
            drop(zip);
            let _ = std::fs::remove_file(&tmp);
            return Err("The save folders are empty or missing".into());
        }
        // Where each numbered folder goes back to.
        zip.start_file("omnihub-saves.json", opts).map_err(|e| e.to_string())?;
        zip.write_all(serde_json::to_string(&dirs).unwrap_or_default().as_bytes()).map_err(|e| e.to_string())?;
        zip.finish().map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        // Keep the newest few.
        for old in self.backups(key).into_iter().skip(KEEP_BACKUPS) {
            let _ = std::fs::remove_file(out_dir.join(format!("{}.zip", old.id)));
        }
        Ok(Backup { id, at: now_ms(), bytes: std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0), files })
    }

    /// Put a backup back (the saves there now are backed up first).
    pub fn restore(&self, key: &str, id: &str) -> Result<(), String> {
        if id.contains(['/', '\\', '.']) {
            return Err("No such backup".into());
        }
        let path = self.backup_dir(key).join(format!("{id}.zip"));
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&path).map_err(|_| "No such backup".to_string())?).map_err(|e| e.to_string())?;
        let mut manifest = String::new();
        zip.by_name("omnihub-saves.json").map_err(|_| "That backup has no list of folders".to_string())?.read_to_string(&mut manifest).map_err(|e| e.to_string())?;
        let dirs: Vec<String> = serde_json::from_str(&manifest).map_err(|e| e.to_string())?;
        let _ = self.backup(key, "before-restore");
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
            let Some(name) = f.enclosed_name() else { continue };
            let mut parts = name.components();
            let Some(idx) = parts.next().and_then(|c| c.as_os_str().to_str()?.parse::<usize>().ok()) else { continue };
            let Some(root) = dirs.get(idx) else { continue };
            let dest = Path::new(root).join(parts.as_path());
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut out).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn delete_backup(&self, key: &str, id: &str) -> Result<(), String> {
        if id.contains(['/', '\\', '.']) {
            return Err("No such backup".into());
        }
        std::fs::remove_file(self.backup_dir(key).join(format!("{id}.zip"))).map_err(|e| e.to_string())
    }
}

/// Folders that look like a game's saves: Steam Cloud's copy, and folders
/// named after the game in the usual places (Documents\My Games, Saved
/// Games, AppData). `home` is the user's folder.
pub fn guess_save_dirs(home: &Path, game: &str, steam: Option<(&Path, u32, u32)>) -> Vec<String> {
    let want = super::steam::squash(game);
    let mut out = Vec::new();
    if let Some((steam, account, app)) = steam {
        let remote = steam.join("userdata").join(account.to_string()).join(app.to_string()).join("remote");
        if remote.is_dir() {
            out.push(remote.to_string_lossy().into_owned());
        }
    }
    if want.len() < 4 {
        return out;
    }
    let docs = dirs::document_dir().unwrap_or_else(|| home.join("Documents"));
    let mut roots = vec![docs.join("My Games"), docs.clone(), home.join("Saved Games"), home.join("AppData").join("Local"), home.join("AppData").join("Roaming")];
    // AppData\LocalLow\<company>\<game>.
    if let Ok(rd) = std::fs::read_dir(home.join("AppData").join("LocalLow")) {
        roots.extend(rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
    }
    for root in roots {
        let Ok(rd) = std::fs::read_dir(&root) else { continue };
        for e in rd.flatten() {
            let n = super::steam::squash(&e.file_name().to_string_lossy());
            if n.len() >= 4 && (n == want || (n.len() >= 6 && want.starts_with(&n)) || (want.len() >= 6 && n.starts_with(&want))) && e.path().is_dir() {
                let p = e.path().to_string_lossy().into_owned();
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_art_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let s = Shelf::new(dir.path(), EventBus::new());
        assert!(s.patch("steam:1", ShelfPatch { status: Some("bored".into()), ..Default::default() }).is_err());
        s.patch("steam:1", ShelfPatch { status: Some("backlog".into()), favorite: Some(true), ..Default::default() }).unwrap();
        assert_eq!(s.entry("steam:1").status, "backlog");
        let png = dir.path().join("c.png");
        std::fs::write(&png, b"x").unwrap();
        let e = s.set_art("steam:1", "cover", Some(png.to_str().unwrap())).unwrap();
        let src = s.art_src(e.cover.as_deref().unwrap()).unwrap();
        assert!(src.starts_with("gamemedia://localhost/"));
        assert_eq!(s.media_path(src.rsplit('/').next().unwrap()).unwrap().extension().unwrap(), "png");
        s.set_art("steam:1", "cover", None).unwrap();
        assert_eq!(std::fs::read_dir(&s.art_dir).unwrap().count(), 0);
        s.record(SessionRec { key: "steam:1".into(), name: "G".into(), start: 0, end: 30_000 }).unwrap();
        s.record(SessionRec { key: "steam:1".into(), name: "G".into(), start: 0, end: 600_000 }).unwrap();
        assert_eq!(s.sessions(Some("steam:1"), 10).len(), 1, "under a minute isn't a session");
        assert_eq!(s.totals()["steam:1"].0, 10);
        // Still there after a restart.
        assert!(Shelf::new(dir.path(), EventBus::new()).entry("steam:1").favorite);
    }

    #[test]
    fn backup_and_restore() {
        let dir = tempfile::tempdir().unwrap();
        let saves = dir.path().join("Saves");
        std::fs::create_dir_all(saves.join("slot")).unwrap();
        std::fs::write(saves.join("slot").join("1.sav"), b"level 5").unwrap();
        let s = Shelf::new(&dir.path().join("data"), EventBus::new());
        assert!(s.backup("g", "manual").is_err(), "no folder yet");
        s.patch("g", ShelfPatch { save_dirs: Some(vec![saves.to_string_lossy().into_owned()]), ..Default::default() }).unwrap();
        let b = s.backup("g", "manual").unwrap();
        assert_eq!(b.files, 1);
        std::fs::write(saves.join("slot").join("1.sav"), b"level 1 (oops)").unwrap();
        s.restore("g", &b.id).unwrap();
        assert_eq!(std::fs::read(saves.join("slot").join("1.sav")).unwrap(), b"level 5");
        assert!(s.backups("g").len() >= 2, "the overwritten saves were kept too");
        assert!(s.restore("g", "../x").is_err());
    }

    #[test]
    fn finds_save_folders() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("Saved Games").join("Hades II")).unwrap();
        std::fs::create_dir_all(home.path().join("AppData/LocalLow/Team Cherry/Hollow Knight")).unwrap();
        std::fs::create_dir_all(home.path().join("AppData/Roaming/Discord")).unwrap();
        assert!(guess_save_dirs(home.path(), "Hollow Knight", None).iter().any(|p| p.ends_with("Hollow Knight")));
        assert!(guess_save_dirs(home.path(), "Hades II", None).iter().any(|p| p.ends_with("Hades II")));
        assert!(guess_save_dirs(home.path(), "Celeste", None).is_empty());
    }
}
