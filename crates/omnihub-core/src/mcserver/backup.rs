//! World backups: zips of the world folders in the server's `backups`
//! folder, made by hand or on a schedule (the server is asked to save and
//! pause saving while it's copied), kept to a number, and put back.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub name: String,
    pub at: i64,
    pub size: u64,
    /// Made on the schedule (and pruned with it), not by hand.
    pub auto: bool,
}

pub fn folder(server: &Path) -> PathBuf {
    server.join("backups")
}

/// The world's folders: the level, and the Nether and End beside it on older Paper servers.
pub fn worlds(server: &Path, level: &str) -> Vec<String> {
    [level.to_string(), format!("{level}_nether"), format!("{level}_the_end")].into_iter().filter(|w| server.join(w).is_dir()).collect()
}

pub fn list(server: &Path) -> Vec<Backup> {
    let mut v: Vec<Backup> = std::fs::read_dir(folder(server))
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if !name.ends_with(".zip") {
                        return None;
                    }
                    let m = e.metadata().ok()?;
                    let at = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0);
                    Some(Backup { auto: name.starts_with("auto-"), name, at, size: m.len() })
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by(|a, b| b.at.cmp(&a.at).then(b.name.cmp(&a.name)));
    v
}

fn ok_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") || !name.ends_with(".zip") {
        return Err("No such backup.".into());
    }
    Ok(())
}

/// Zip the world folders (the server should be stopped, or saving paused).
pub fn make(server: &Path, level: &str, auto: bool) -> Result<Backup, String> {
    let ws = worlds(server, level);
    if ws.is_empty() {
        return Err("There's no world to back up yet: start the server once first.".into());
    }
    let dir = folder(server);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let name = format!("{}{level}-{stamp}.zip", if auto { "auto-" } else { "" });
    let out = dir.join(&name);
    let part = out.with_extension("zip.part");
    let r = (|| -> Result<(), String> {
        let mut z = zip::ZipWriter::new(std::fs::File::create(&part).map_err(|e| e.to_string())?);
        for w in &ws {
            let base = server.join(w);
            for e in walkdir::WalkDir::new(&base).follow_links(false).into_iter().flatten() {
                if !e.file_type().is_file() || e.file_name() == "session.lock" {
                    continue;
                }
                let rel = e.path().strip_prefix(server).map_err(|e| e.to_string())?.to_string_lossy().replace('\\', "/");
                let len = e.metadata().map(|m| m.len()).unwrap_or(0);
                // Region files are compressed already.
                let method = if rel.ends_with(".mca") || rel.ends_with(".dat") || rel.ends_with(".zip") { zip::CompressionMethod::Stored } else { zip::CompressionMethod::Deflated };
                let o = zip::write::SimpleFileOptions::default().compression_method(method).large_file(len >= 1 << 32);
                z.start_file(rel, o).map_err(|e| e.to_string())?;
                let mut f = match std::fs::File::open(e.path()) {
                    Ok(f) => f,
                    // A file the running server holds open (rare): skipped, as Minecraft does.
                    Err(_) => continue,
                };
                std::io::copy(&mut f, &mut z).map_err(|e| e.to_string())?;
            }
        }
        z.finish().map_err(|e| e.to_string())?.flush().map_err(|e| e.to_string())
    })();
    if let Err(e) = r {
        let _ = std::fs::remove_file(&part);
        return Err(format!("The backup failed: {e}"));
    }
    std::fs::rename(&part, &out).map_err(|e| e.to_string())?;
    let size = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    Ok(Backup { name, at: crate::db::now(), size, auto })
}

/// Keep the newest `keep` scheduled backups; hand-made ones stay.
pub fn prune(server: &Path, keep: usize) {
    for b in list(server).into_iter().filter(|b| b.auto).skip(keep.max(1)) {
        let _ = std::fs::remove_file(folder(server).join(&b.name));
    }
}

pub fn delete(server: &Path, name: &str) -> Result<(), String> {
    ok_name(name)?;
    std::fs::remove_file(folder(server).join(name)).map_err(|e| e.to_string())
}

/// Put a backup's world back (the server must be stopped). The world it
/// replaces is kept beside it as `<level>.before-restore-<time>`.
pub fn restore(server: &Path, level: &str, name: &str) -> Result<String, String> {
    ok_name(name)?;
    let zip_path = folder(server).join(name);
    let mut z = zip::ZipArchive::new(std::fs::File::open(&zip_path).map_err(|_| "That backup is gone.".to_string())?).map_err(|e| format!("That backup can't be read: {e}"))?;
    let tops: std::collections::HashSet<String> = z.file_names().filter_map(|n| n.split('/').next().map(String::from)).collect();
    let allowed = [level.to_string(), format!("{level}_nether"), format!("{level}_the_end")];
    if tops.is_empty() || !tops.iter().all(|t| allowed.contains(t)) {
        return Err("That backup isn't of this server's world.".into());
    }
    let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let kept = format!("{level}.before-restore-{stamp}");
    for w in worlds(server, level) {
        let to = if w == level { server.join(&kept) } else { server.join(format!("{w}.before-restore-{stamp}")) };
        std::fs::rename(server.join(&w), to).map_err(|e| format!("Couldn't move the current world aside (is the server stopped?): {e}"))?;
    }
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        let rel = crate::mods::install::safe_rel(f.name()).ok_or("The backup has a file that would land outside the server.")?;
        let out = server.join(&rel);
        std::fs::create_dir_all(out.parent().unwrap_or(server)).map_err(|e| e.to_string())?;
        let mut w = std::fs::File::create(&out).map_err(|e| e.to_string())?;
        std::io::copy(&mut f, &mut w).map_err(|e| e.to_string())?;
    }
    Ok(kept)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_up_prune_and_restore() {
        let t = tempfile::tempdir().unwrap();
        let s = t.path();
        assert!(make(s, "world", false).is_err(), "no world yet");
        std::fs::create_dir_all(s.join("world/dimensions/minecraft/overworld/region")).unwrap();
        std::fs::write(s.join("world/level.dat"), b"level one").unwrap();
        std::fs::write(s.join("world/session.lock"), b"x").unwrap();
        std::fs::write(s.join("world/dimensions/minecraft/overworld/region/r.0.0.mca"), vec![7u8; 9000]).unwrap();
        let b = make(s, "world", false).unwrap();
        assert!(b.size > 100 && !b.auto);
        std::fs::write(s.join("world/level.dat"), b"level two").unwrap();
        for _ in 0..3 {
            std::thread::sleep(std::time::Duration::from_millis(1100));
            make(s, "world", true).unwrap();
        }
        prune(s, 2);
        let l = list(s);
        assert_eq!((l.iter().filter(|b| b.auto).count(), l.iter().filter(|b| !b.auto).count()), (2, 1), "two scheduled ones kept, the hand-made one stays");
        let kept = restore(s, "world", &b.name).unwrap();
        assert_eq!(std::fs::read(s.join("world/level.dat")).unwrap(), b"level one");
        assert_eq!(std::fs::read(s.join(&kept).join("level.dat")).unwrap(), b"level two", "the replaced world is kept");
        assert!(!s.join("world/session.lock").exists());
        assert!(restore(s, "world", "../x.zip").is_err());
        delete(s, &b.name).unwrap();
        assert_eq!(list(s).len(), 2);
    }
}
