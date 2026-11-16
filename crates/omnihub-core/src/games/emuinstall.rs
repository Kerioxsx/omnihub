//! Install and update emulators from their official GitHub releases, and
//! only those: each download's SHA-256 must match the checksum GitHub
//! publishes for the file, or nothing is installed. Emulators go in
//! OmniHub's data folder and are added to the emulator list by themselves.
//! Emulators that don't publish on GitHub (Dolphin, PPSSPP, RetroArch) are
//! linked to their own sites instead. BIOS files and games are never
//! downloaded.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

pub struct Source {
    pub id: &'static str,
    pub name: &'static str,
    pub repo: &'static str,
    /// A fixed release tag (a rolling "latest"), else the newest release.
    pub tag: Option<&'static str>,
    /// The asset's name contains all of these (lowercase)…
    pub has: &'static [&'static str],
    /// …and none of these.
    pub not: &'static [&'static str],
    /// The program inside (lowercase).
    pub exe: &'static str,
    pub platforms: &'static [&'static str],
}

pub const SOURCES: &[Source] = &[
    Source { id: "duckstation", name: "DuckStation", repo: "stenzek/duckstation", tag: Some("latest"), has: &["windows-x64-release", ".zip"], not: &["symbols"], exe: "duckstation-qt-x64-releaseltcg.exe", platforms: &["psx"] },
    Source { id: "pcsx2", name: "PCSX2", repo: "PCSX2/pcsx2", tag: None, has: &["windows-x64-qt", ".7z"], not: &["symbols"], exe: "pcsx2-qt.exe", platforms: &["ps2"] },
    Source { id: "melonds", name: "melonDS", repo: "melonDS-emu/melonDS", tag: None, has: &["windows", "x86_64", ".zip"], not: &[], exe: "melonds.exe", platforms: &["nds"] },
    Source { id: "mgba", name: "mGBA", repo: "mgba-emu/mgba", tag: None, has: &["win64", ".7z"], not: &[], exe: "mgba.exe", platforms: &["gba", "gb", "gbc"] },
    Source { id: "cemu", name: "Cemu", repo: "cemu-project/Cemu", tag: None, has: &["windows-x64", ".zip"], not: &[], exe: "cemu.exe", platforms: &["wiiu"] },
    Source { id: "azahar", name: "Azahar", repo: "azahar-emu/azahar", tag: None, has: &["windows", "msvc", ".zip"], not: &["installer"], exe: "azahar.exe", platforms: &["3ds"] },
    Source { id: "rpcs3", name: "RPCS3", repo: "RPCS3/rpcs3-binaries-win", tag: None, has: &["win64", ".7z"], not: &[], exe: "rpcs3.exe", platforms: &["ps3"] },
];

/// Emulators without GitHub downloads: (name, their download page).
pub const ELSEWHERE: &[(&str, &str)] = &[("Dolphin", "https://dolphin-emu.org/download/"), ("PPSSPP", "https://www.ppsspp.org/download/"), ("RetroArch", "https://www.retroarch.com/?page=platforms")];

#[derive(Debug, Clone, PartialEq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

/// The release's version and the file to download for `src`.
pub fn pick(src: &Source, release: &serde_json::Value) -> Result<(String, Asset), String> {
    let assets = release.get("assets").and_then(|a| a.as_array()).ok_or("That release has no files")?;
    let a = assets
        .iter()
        .find(|a| {
            let n = a.get("name").and_then(|n| n.as_str()).unwrap_or("").to_lowercase();
            src.has.iter().all(|h| n.contains(h)) && !src.not.iter().any(|x| n.contains(x))
        })
        .ok_or_else(|| format!("{} has no Windows download in its newest release", src.name))?;
    let name = a.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
    let sha256 = a.get("digest").and_then(|d| d.as_str()).and_then(|d| d.strip_prefix("sha256:")).filter(|h| h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit())).ok_or_else(|| format!("GitHub doesn't list a checksum for {name}, so OmniHub won't install it"))?.to_lowercase();
    let url = a.get("browser_download_url").and_then(|u| u.as_str()).filter(|u| u.starts_with(&format!("https://github.com/{}/releases/download/", src.repo))).ok_or("The download isn't on the emulator's own GitHub page")?.to_string();
    let tag = release.get("tag_name").and_then(|t| t.as_str()).unwrap_or("");
    // A rolling tag says nothing; the file's checksum tells versions apart.
    let version = if src.tag.is_some() { format!("{} {}", tag, &sha256[..8]) } else { tag.trim_start_matches('v').to_string() };
    Ok((version, Asset { name, url, sha256, size: a.get("size").and_then(|s| s.as_u64()).unwrap_or(0) }))
}

pub fn release_url(src: &Source) -> String {
    match src.tag {
        Some(t) => format!("https://api.github.com/repos/{}/releases/tags/{t}", src.repo),
        None => format!("https://api.github.com/repos/{}/releases/latest", src.repo),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    pub version: String,
    pub exe: String,
    pub dir: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EmuOffer {
    pub id: String,
    pub name: String,
    pub platforms: Vec<String>,
    pub installed: Option<String>,
    /// Filled after a check for updates.
    pub latest: Option<String>,
}

pub struct EmuInstaller {
    root: PathBuf,
    file: PathBuf,
    installed: Mutex<HashMap<String, Installed>>,
    latest: Mutex<HashMap<String, String>>,
}

fn find_exe(dir: &Path, name: &str) -> Option<PathBuf> {
    walkdir::WalkDir::new(dir).max_depth(4).into_iter().filter_map(Result::ok).find(|e| e.file_type().is_file() && e.file_name().to_string_lossy().eq_ignore_ascii_case(name)).map(|e| e.into_path())
}

impl EmuInstaller {
    pub fn new(data: &Path) -> EmuInstaller {
        let file = data.join("emulator-installs.json");
        let installed = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        EmuInstaller { root: data.join("emulators"), file, installed: Mutex::new(installed), latest: Mutex::new(HashMap::new()) }
    }

    pub fn offers(&self) -> Vec<EmuOffer> {
        let inst = self.installed.lock();
        let latest = self.latest.lock();
        SOURCES.iter().map(|s| EmuOffer { id: s.id.into(), name: s.name.into(), platforms: s.platforms.iter().map(|p| p.to_string()).collect(), installed: inst.get(s.id).filter(|i| Path::new(&i.exe).is_file()).map(|i| i.version.clone()), latest: latest.get(s.id).cloned() }).collect()
    }

    /// Ask GitHub for each emulator's newest version.
    pub fn check(&self) -> Vec<EmuOffer> {
        for s in SOURCES {
            if let Ok((v, _)) = crate::web::get_json(&release_url(s)).and_then(|r| pick(s, &r)) {
                self.latest.lock().insert(s.id.into(), v);
            }
        }
        self.offers()
    }

    /// Download, check and unpack one emulator; returns its program.
    pub fn install(&self, id: &str, progress: &mut dyn FnMut(u64, u64)) -> Result<(PathBuf, &'static Source), String> {
        let src = SOURCES.iter().find(|s| s.id == id).ok_or("OmniHub doesn't install that emulator")?;
        let release = crate::web::get_json(&release_url(src))?;
        let (version, asset) = pick(src, &release)?;
        std::fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let part = self.root.join(format!("{id}.download"));
        let got = crate::capture::airplay::download(&asset.url, &part, progress).map_err(|e| format!("Download failed: {e}"))?;
        if !got.eq_ignore_ascii_case(&asset.sha256) {
            let _ = std::fs::remove_file(&part);
            return Err(format!("{} didn't match GitHub's checksum, so it was deleted", asset.name));
        }
        // The unpacker reads the archive type from its first bytes.
        let dir = self.root.join(id).join(version.replace(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'), "_"));
        let _ = std::fs::remove_dir_all(&dir);
        let r = crate::mods::install::unpack(&part, &dir);
        let _ = std::fs::remove_file(&part);
        r?;
        let exe = find_exe(&dir, src.exe).ok_or_else(|| format!("{} isn't in the download", src.exe))?;
        // Older versions go once the new one is in place (their settings stay in the emulator's own folder or AppData).
        let old = self.installed.lock().get(id).cloned();
        if let Some(o) = old.filter(|o| Path::new(&o.dir) != dir.as_path() && Path::new(&o.dir).starts_with(&self.root)) {
            let _ = std::fs::remove_dir_all(&o.dir);
        }
        let mut inst = self.installed.lock();
        inst.insert(id.into(), Installed { version: version.clone(), exe: exe.display().to_string(), dir: dir.display().to_string() });
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&*inst).unwrap_or_default()).map_err(|e| e.to_string())?;
        drop(inst);
        self.latest.lock().insert(id.into(), version);
        Ok((exe, src))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_checked_windows_file() {
        let hex = "a".repeat(64);
        let rel = serde_json::json!({"tag_name":"v2.2.0","assets":[
            {"name":"pcsx2-v2.2.0-linux-appimage-x64-Qt.AppImage","digest":format!("sha256:{hex}"),"browser_download_url":"https://github.com/PCSX2/pcsx2/releases/download/v2.2.0/a.AppImage","size":1},
            {"name":"pcsx2-v2.2.0-windows-x64-Qt.7z","digest":format!("sha256:{hex}"),"browser_download_url":"https://github.com/PCSX2/pcsx2/releases/download/v2.2.0/pcsx2-v2.2.0-windows-x64-Qt.7z","size":42},
            {"name":"pcsx2-v2.2.0-windows-x64-Qt-symbols.7z","digest":format!("sha256:{hex}"),"browser_download_url":"https://github.com/PCSX2/pcsx2/releases/download/v2.2.0/s.7z","size":3}]});
        let pcsx2 = SOURCES.iter().find(|s| s.id == "pcsx2").unwrap();
        let (v, a) = pick(pcsx2, &rel).unwrap();
        assert_eq!((v.as_str(), a.name.as_str(), a.size), ("2.2.0", "pcsx2-v2.2.0-windows-x64-Qt.7z", 42));
        // No checksum, no install.
        let bare = serde_json::json!({"tag_name":"v1","assets":[{"name":"pcsx2-v1-windows-x64-Qt.7z","browser_download_url":"https://github.com/PCSX2/pcsx2/releases/download/v1/x.7z"}]});
        assert!(pick(pcsx2, &bare).unwrap_err().contains("checksum"));
        // A file hosted anywhere else is refused.
        let away = serde_json::json!({"tag_name":"v1","assets":[{"name":"pcsx2-v1-windows-x64-Qt.7z","digest":format!("sha256:{hex}"),"browser_download_url":"https://evil.example/x.7z"}]});
        assert!(pick(pcsx2, &away).is_err());
        let duck = SOURCES.iter().find(|s| s.id == "duckstation").unwrap();
        assert_eq!(release_url(duck), "https://api.github.com/repos/stenzek/duckstation/releases/tags/latest");
        let rolling = serde_json::json!({"tag_name":"latest","assets":[{"name":"duckstation-windows-x64-release.zip","digest":format!("sha256:{hex}"),"browser_download_url":"https://github.com/stenzek/duckstation/releases/download/latest/duckstation-windows-x64-release.zip"}]});
        assert_eq!(pick(duck, &rolling).unwrap().0, "latest aaaaaaaa");
        let t = tempfile::tempdir().unwrap();
        assert!(EmuInstaller::new(t.path()).offers().iter().all(|o| o.installed.is_none()));
    }
}
