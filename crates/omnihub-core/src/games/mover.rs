//! Move a game to another drive: its folder is copied, checked file by file,
//! the old one is deleted, and a link (a directory junction) is left where
//! it was, so Steam, Epic and every other launcher still find it there.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

fn size_of(dir: &Path) -> u64 {
    walkdir::WalkDir::new(dir).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_file()).filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum()
}

/// Copy a folder, reporting (bytes copied, total); verifies each file's size.
pub fn copy_dir(src: &Path, dest: &Path, stop: &AtomicBool, progress: &dyn Fn(u64, u64)) -> Result<u64, String> {
    let total = size_of(src);
    let mut done = 0u64;
    for e in walkdir::WalkDir::new(src).into_iter() {
        let e = e.map_err(|e| e.to_string())?;
        if stop.load(Ordering::Relaxed) {
            return Err("Stopped".into());
        }
        let rel = e.path().strip_prefix(src).map_err(|e| e.to_string())?;
        let to = dest.join(rel);
        if e.file_type().is_dir() {
            std::fs::create_dir_all(&to).map_err(|e| format!("Couldn't make {}: {e}", to.display()))?;
        } else if e.file_type().is_file() {
            let n = std::fs::copy(e.path(), &to).map_err(|err| format!("Couldn't copy {}: {err}", e.path().display()))?;
            let want = e.metadata().map(|m| m.len()).unwrap_or(n);
            if n != want || std::fs::metadata(&to).map(|m| m.len()).unwrap_or(0) != want {
                return Err(format!("{} didn't copy completely", e.path().display()));
            }
            done += n;
            progress(done, total);
        }
    }
    Ok(done)
}

/// Move `src` into `dest_parent`, leaving a link at `src`. `link(at, to)`
/// makes the link (a junction on Windows). Returns the new folder.
pub fn move_game(src: &Path, dest_parent: &Path, free: u64, stop: &AtomicBool, progress: &dyn Fn(u64, u64), link: &dyn Fn(&Path, &Path) -> Result<(), String>) -> Result<PathBuf, String> {
    if !src.is_dir() {
        return Err("The game's folder isn't there".into());
    }
    if std::fs::symlink_metadata(src).map(|m| m.file_type().is_symlink()).unwrap_or(false) {
        return Err("This game was already moved (its folder is a link)".into());
    }
    let name = src.file_name().ok_or("That folder has no name")?;
    let dest = dest_parent.join(name);
    if dest.exists() {
        return Err(format!("{} already exists there", dest.display()));
    }
    if src.components().next() == dest.components().next() && dest_parent.starts_with(src.parent().unwrap_or(src)) {
        return Err("Pick another drive".into());
    }
    let need = size_of(src);
    if free < need + (1 << 30) {
        return Err(format!("Not enough room: it needs {:.1} GB", need as f64 / 1e9));
    }
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    if let Err(e) = copy_dir(src, &dest, stop, progress) {
        let _ = std::fs::remove_dir_all(&dest);
        return Err(e);
    }
    std::fs::remove_dir_all(src).map_err(|e| format!("Copied, but the old folder couldn't be removed (is the game open?): {e}"))?;
    if let Err(e) = link(src, &dest) {
        return Err(format!("Moved to {}, but the link couldn't be made ({e}); point the launcher to the new place", dest.display()));
    }
    Ok(dest)
}

/// A directory junction at `at` pointing to `to` (no administrator needed).
#[cfg(windows)]
pub fn junction(at: &Path, to: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(at).arg(to).creation_flags(0x0800_0000).output().map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[cfg(not(windows))]
pub fn junction(at: &Path, to: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(to, at).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moves_and_leaves_a_link() {
        let t = tempfile::tempdir().unwrap();
        let game = t.path().join("C").join("Games").join("Hollow");
        std::fs::create_dir_all(game.join("data")).unwrap();
        std::fs::write(game.join("game.exe"), vec![1u8; 4096]).unwrap();
        std::fs::write(game.join("data").join("pak0"), vec![2u8; 10_000]).unwrap();
        let dest = t.path().join("D").join("Games");
        std::fs::create_dir_all(&dest).unwrap();
        let stop = AtomicBool::new(false);
        assert!(move_game(&game, &dest, 1000, &stop, &|_, _| {}, &junction).unwrap_err().contains("Not enough room"));
        let last = std::sync::Mutex::new((0, 0));
        let to = move_game(&game, &dest, u64::MAX, &stop, &|d, n| *last.lock().unwrap() = (d, n), &junction).unwrap();
        assert_eq!(*last.lock().unwrap(), (14_096, 14_096));
        assert_eq!(std::fs::read(to.join("data").join("pak0")).unwrap().len(), 10_000);
        // The old path still works, through the link.
        assert_eq!(std::fs::read(game.join("game.exe")).unwrap().len(), 4096);
        assert!(move_game(&game, &dest, u64::MAX, &stop, &|_, _| {}, &junction).unwrap_err().contains("already moved"));
    }
}
