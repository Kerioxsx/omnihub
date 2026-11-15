//! Fortnite without its optional texture packs, for a session: the "remove
//! textures" trick the potato apps use on AMD and Intel cards.
//!
//! Only Fortnite's optional packs are touched: files in
//! `FortniteGame/Content/Paks` whose name contains "optional"
//! (`pakchunk10optional-WindowsClient.utoc`, `.ucas`, `.pak`, `.sig`) — the
//! High Resolution Textures and similar extras that Epic's own launcher can
//! leave out. They are moved, never deleted, into a folder beside `Content`
//! on the same drive (a rename: instant, no copy), with a list of what moved.
//! They go back as soon as Fortnite has run and closed, when OmniHub
//! starts, or with Restore. Nothing is moved while Fortnite runs, and a
//! half-finished move is put back.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const STASH: &str = "OmniHub-stashed-textures";
const LIST: &str = "moved.json";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextureStatus {
    /// Fortnite's install was found.
    pub found: bool,
    /// Optional packs in place (that could be taken out).
    pub files: usize,
    pub bytes: u64,
    /// Packs taken out right now (waiting to go back).
    pub stashed: usize,
    pub stashed_bytes: u64,
}

fn paks(install: &Path) -> PathBuf {
    install.join("FortniteGame").join("Content").join("Paks")
}

fn stash_dir(install: &Path) -> PathBuf {
    install.join("FortniteGame").join(STASH)
}

/// A file Fortnite can do without: an optional pack.
pub fn is_optional_pack(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("optional") && [".pak", ".utoc", ".ucas", ".sig"].iter().any(|e| n.ends_with(e)) && !n.contains("..") && !n.contains('/') && !n.contains('\\')
}

fn optional_packs(install: &Path) -> Vec<(String, u64)> {
    let mut v: Vec<(String, u64)> = std::fs::read_dir(paks(install))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            is_optional_pack(&name).then(|| (name, e.metadata().map(|m| m.len()).unwrap_or(0)))
        })
        .collect();
    v.sort();
    v
}

fn read_list(install: &Path) -> Vec<String> {
    std::fs::read(stash_dir(install).join(LIST)).ok().and_then(|b| serde_json::from_slice::<Vec<String>>(&b).ok()).unwrap_or_default().into_iter().filter(|n| is_optional_pack(n)).collect()
}

pub fn status(install: Option<&Path>) -> TextureStatus {
    let Some(install) = install.filter(|i| paks(i).is_dir()) else { return TextureStatus::default() };
    let here = optional_packs(install);
    let list = read_list(install);
    let stashed_bytes = list.iter().filter_map(|n| std::fs::metadata(stash_dir(install).join(n)).ok()).map(|m| m.len()).sum();
    TextureStatus { found: true, files: here.len(), bytes: here.iter().map(|(_, b)| b).sum(), stashed: list.len(), stashed_bytes }
}

/// Take the optional packs out. `running` must be false (Fortnite closed).
pub fn stash(install: &Path, running: bool) -> Result<TextureStatus, String> {
    if running {
        return Err("Close Fortnite first: its files can't be moved while it runs.".into());
    }
    if !paks(install).is_dir() {
        return Err("Fortnite's game files weren't found.".into());
    }
    if !read_list(install).is_empty() {
        return Ok(status(Some(install)));
    }
    let packs = optional_packs(install);
    if packs.is_empty() {
        return Err("This Fortnite install has no optional texture packs to take out (High Resolution Textures isn't installed), so there is nothing to remove.".into());
    }
    let dir = stash_dir(install);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // Write the list first, so a crash mid-way still knows what to put back.
    let names: Vec<String> = packs.iter().map(|(n, _)| n.clone()).collect();
    std::fs::write(dir.join(LIST), serde_json::to_vec(&names).unwrap_or_default()).map_err(|e| e.to_string())?;
    let mut moved: Vec<&String> = Vec::new();
    for n in &names {
        if let Err(e) = std::fs::rename(paks(install).join(n), dir.join(n)) {
            // Put back what already moved and stop.
            for m in &moved {
                let _ = std::fs::rename(dir.join(m), paks(install).join(m));
            }
            let _ = std::fs::remove_file(dir.join(LIST));
            let _ = std::fs::remove_dir(&dir);
            return Err(format!("Couldn't move {n} ({e}); everything was put back."));
        }
        moved.push(n);
    }
    Ok(status(Some(install)))
}

/// Put the packs back. A pack Fortnite's launcher has downloaded again in
/// the meantime stays; the old copy is removed. Returns how many went back.
pub fn restore(install: &Path, running: bool) -> Result<usize, String> {
    if running {
        return Err("Fortnite is running; its textures go back when it closes.".into());
    }
    let dir = stash_dir(install);
    let list = read_list(install);
    let mut back = 0;
    let mut failed = Vec::new();
    for n in &list {
        let from = dir.join(n);
        let to = paks(install).join(n);
        if !from.is_file() {
            continue;
        }
        if to.exists() {
            let _ = std::fs::remove_file(&from);
            continue;
        }
        match std::fs::rename(&from, &to) {
            Ok(()) => back += 1,
            Err(e) => failed.push(format!("{n}: {e}")),
        }
    }
    if !failed.is_empty() {
        return Err(format!("Some textures couldn't go back yet ({}). OmniHub tries again next time.", failed.join("; ")));
    }
    let _ = std::fs::remove_file(dir.join(LIST));
    let _ = std::fs::remove_dir(&dir);
    Ok(back)
}

pub fn has_stash(install: &Path) -> bool {
    !read_list(install).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install() -> tempfile::TempDir {
        let t = tempfile::tempdir().unwrap();
        let p = paks(t.path());
        std::fs::create_dir_all(&p).unwrap();
        for (n, size) in [("pakchunk0-WindowsClient.pak", 10), ("pakchunk0-WindowsClient.utoc", 5), ("pakchunk10optional-WindowsClient.pak", 7), ("pakchunk10optional-WindowsClient.utoc", 3), ("pakchunk10optional-WindowsClient.ucas", 40), ("pakchunk10optional-WindowsClient.sig", 1), ("notes-optional.txt", 1)] {
            std::fs::write(p.join(n), vec![0u8; size]).unwrap();
        }
        t
    }

    #[test]
    fn only_optional_packs_move_and_all_come_back() {
        let t = install();
        let s = status(Some(t.path()));
        assert_eq!((s.files, s.bytes), (4, 51), "only the optional .pak/.utoc/.ucas/.sig");
        assert!(stash(t.path(), true).is_err(), "never while Fortnite runs");
        let s = stash(t.path(), false).unwrap();
        assert_eq!((s.files, s.stashed, s.stashed_bytes), (0, 4, 51));
        assert!(paks(t.path()).join("pakchunk0-WindowsClient.pak").is_file(), "required packs stay");
        assert!(paks(t.path()).join("notes-optional.txt").is_file(), "other files stay");
        assert!(restore(t.path(), true).is_err());
        assert_eq!(restore(t.path(), false).unwrap(), 4);
        let s = status(Some(t.path()));
        assert_eq!((s.files, s.stashed), (4, 0));
        assert!(!stash_dir(t.path()).exists(), "nothing left behind");
    }

    #[test]
    fn a_redownloaded_pack_wins_and_names_are_checked() {
        let t = install();
        stash(t.path(), false).unwrap();
        // The launcher put one back itself.
        std::fs::write(paks(t.path()).join("pakchunk10optional-WindowsClient.pak"), b"new").unwrap();
        assert_eq!(restore(t.path(), false).unwrap(), 3);
        assert_eq!(std::fs::read(paks(t.path()).join("pakchunk10optional-WindowsClient.pak")).unwrap(), b"new");
        assert!(is_optional_pack("pakchunk10optional-WindowsClient.ucas"));
        assert!(!is_optional_pack("pakchunk10-WindowsClient.ucas"));
        assert!(!is_optional_pack("..optional.pak"));
        assert!(stash(tempfile::tempdir().unwrap().path(), false).is_err());
    }
}
