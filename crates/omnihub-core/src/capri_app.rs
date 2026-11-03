//! Capri Desktop (github.com/lachydotmcg/capri-desktop): a free,
//! open-source (PolyForm Noncommercial) fan-made companion who sits on your
//! windows and chats. OmniHub never ships it: "Install" downloads the
//! developer's own installer from their GitHub releases, checks it against
//! the checksum GitHub publishes when there is one, and runs it. OmniHub
//! also opens and closes it, can start it with OmniHub, and helps with
//! Ollama, which Capri's local brain can use.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;

pub const RELEASES: &str = "https://api.github.com/repos/lachydotmcg/capri-desktop/releases/latest";
const DOWNLOADS: &str = "https://github.com/lachydotmcg/capri-desktop/releases/download/";
pub const SITE: &str = "https://capriai.app/";
pub const OLLAMA: &str = "http://127.0.0.1:11434";
pub const OLLAMA_PAGE: &str = "https://ollama.com/download";
/// The model Capri's site suggests for Ollama.
pub const MODEL: &str = "gemma4:e4b";
const SETUP: &str = "Capri-Setup.exe";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CapriSettings {
    pub start_with_omnihub: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct OllamaStatus {
    pub installed: bool,
    pub running: bool,
    pub models: Vec<String>,
    pub has_model: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CapriStatus {
    pub installed: bool,
    pub exe: Option<String>,
    pub version: Option<String>,
    pub running: bool,
    pub settings: CapriSettings,
    pub ollama: OllamaStatus,
    pub supported: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Latest {
    pub version: String,
    pub url: String,
    pub size: u64,
    /// SHA-256 GitHub reports for the file, when it does.
    pub sha256: Option<String>,
    pub notes: String,
}

/// The Windows installer in a GitHub "latest release" answer.
pub fn parse_latest(v: &serde_json::Value) -> Result<Latest, String> {
    let version = v["tag_name"].as_str().ok_or("Capri's releases didn't say a version")?.trim_start_matches('v').to_string();
    let asset = v["assets"].as_array().and_then(|a| a.iter().find(|x| x["name"].as_str() == Some(SETUP))).ok_or("This Capri release has no Windows installer")?;
    let url = asset["browser_download_url"].as_str().unwrap_or_default().to_string();
    if !url.starts_with(DOWNLOADS) {
        return Err("The installer isn't on Capri's own releases page".into());
    }
    let sha256 = asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")).filter(|h| h.len() == 64).map(str::to_lowercase);
    Ok(Latest { version, url, size: asset["size"].as_u64().unwrap_or(0), sha256, notes: v["body"].as_str().unwrap_or_default().chars().take(4000).collect() })
}

pub struct CapriApp {
    file: PathBuf,
    cache: PathBuf,
    settings: Mutex<CapriSettings>,
    busy: Mutex<bool>,
    events: EventBus,
}

impl CapriApp {
    pub fn new(data: &Path, cache: &Path, events: EventBus) -> CapriApp {
        let file = data.join("capri-desktop.json");
        let settings = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        CapriApp { file, cache: cache.join("capri"), settings: Mutex::new(settings), busy: Mutex::new(false), events }
    }

    pub fn settings(&self) -> CapriSettings {
        self.settings.lock().clone()
    }

    pub fn save(&self, s: CapriSettings) -> Result<(), String> {
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        *self.settings.lock() = s;
        Ok(())
    }

    pub fn status(&self, running: bool) -> CapriStatus {
        let found = imp::find();
        CapriStatus { installed: found.is_some(), exe: found.as_ref().map(|f| f.0.to_string_lossy().into_owned()), version: found.and_then(|f| f.1), running, settings: self.settings(), ollama: ollama_status(), supported: cfg!(windows) }
    }

    pub fn latest(&self) -> Result<Latest, String> {
        parse_latest(&crate::web::get_json(RELEASES)?)
    }

    /// Download the latest installer, check it, and run it.
    pub fn install(&self) -> Result<Latest, String> {
        {
            let mut b = self.busy.lock();
            if *b {
                return Err("Capri is already being downloaded".into());
            }
            *b = true;
        }
        let r = (|| {
            let latest = self.latest()?;
            std::fs::create_dir_all(&self.cache).map_err(|e| e.to_string())?;
            let dest = self.cache.join(SETUP);
            let events = self.events.clone();
            let mut progress = |done: u64, total: u64| events.emit("capri:download", serde_json::json!({ "done": done, "total": total }));
            let got = crate::capture::airplay::download(&latest.url, &dest, &mut progress).map_err(|e| e.to_string())?;
            if let Some(want) = &latest.sha256 {
                if !got.eq_ignore_ascii_case(want) {
                    let _ = std::fs::remove_file(&dest);
                    return Err("The download didn't match the checksum GitHub publishes for it, so it was deleted.".into());
                }
            }
            imp::run(&dest)?;
            Ok(latest)
        })();
        *self.busy.lock() = false;
        r
    }

    pub fn open(&self) -> Result<(), String> {
        let (exe, _) = imp::find().ok_or("Capri Desktop isn't installed yet")?;
        std::process::Command::new(&exe).current_dir(exe.parent().unwrap_or(Path::new("."))).spawn().map(|_| ()).map_err(|e| format!("Couldn't start Capri: {e}"))
    }
}

/// The process name to look for.
pub const PROCESS: &str = "Capri.exe";

pub fn ollama_status() -> OllamaStatus {
    let installed = imp::ollama_installed();
    match crate::web::get_text_as(&format!("{OLLAMA}/api/tags"), crate::web::UA, 2).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) {
        Some(v) => {
            let models: Vec<String> = v["models"].as_array().map(|a| a.iter().filter_map(|m| m["name"].as_str().map(String::from)).collect()).unwrap_or_default();
            let has_model = models.iter().any(|m| m == MODEL || m.starts_with(&format!("{MODEL}-")));
            OllamaStatus { installed: true, running: true, models, has_model }
        }
        None => OllamaStatus { installed, ..Default::default() },
    }
}

/// Ask Ollama to download a model, reporting progress as `capri:pull`.
pub fn ollama_pull(model: &str, events: &EventBus) -> Result<(), String> {
    let mut last = 0u64;
    crate::web::post_lines(&format!("{OLLAMA}/api/pull"), &serde_json::json!({ "model": model, "stream": true }), 6 * 3600, |line| {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            let done = v["completed"].as_u64().unwrap_or(0);
            if done.saturating_sub(last) > 2_000_000 || v["status"] == "success" {
                last = done;
                events.emit("capri:pull", serde_json::json!({ "status": v["status"], "done": done, "total": v["total"] }));
            }
        }
    })
    .map_err(|e| if e.contains("couldn't reach") || e.contains("onnect") { "Ollama isn't running: install it from ollama.com and open it.".to_string() } else { e })
}

#[cfg(windows)]
mod imp {
    use std::path::{Path, PathBuf};

    /// Capri's install, from Windows' list of installed apps (else the usual folders).
    pub fn find() -> Option<(PathBuf, Option<String>)> {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
        use winreg::RegKey;
        const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
        for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            let Ok(list) = RegKey::predef(root).open_subkey_with_flags(UNINSTALL, KEY_READ) else { continue };
            for k in list.enum_keys().flatten() {
                let Ok(app) = list.open_subkey_with_flags(&k, KEY_READ) else { continue };
                let name: String = app.get_value("DisplayName").unwrap_or_default();
                if !(name == "Capri" || name.starts_with("Capri ")) {
                    continue;
                }
                let version: Option<String> = app.get_value("DisplayVersion").ok();
                let dir: String = app.get_value("InstallLocation").unwrap_or_default();
                let icon: String = app.get_value("DisplayIcon").unwrap_or_default();
                let exe = [Path::new(dir.trim_matches('"')).join(super::PROCESS), PathBuf::from(icon.split(',').next().unwrap_or("").trim_matches('"'))].into_iter().find(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")));
                if let Some(exe) = exe {
                    return Some((exe, version));
                }
            }
        }
        let local = dirs::data_local_dir()?;
        [local.join(r"Programs\Capri").join(super::PROCESS), local.join("Capri").join(super::PROCESS)].into_iter().find(|p| p.is_file()).map(|p| (p, None))
    }

    pub fn run(setup: &Path) -> Result<(), String> {
        std::process::Command::new(setup).spawn().map(|_| ()).map_err(|e| format!("Couldn't start the installer: {e}"))
    }

    pub fn ollama_installed() -> bool {
        dirs::data_local_dir().is_some_and(|l| l.join(r"Programs\Ollama\ollama.exe").is_file())
    }
}

#[cfg(not(windows))]
mod imp {
    use std::path::{Path, PathBuf};
    pub fn find() -> Option<(PathBuf, Option<String>)> {
        None
    }
    pub fn run(_setup: &Path) -> Result<(), String> {
        Err("Capri Desktop's installer is for Windows.".into())
    }
    pub fn ollama_installed() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_latest_release() {
        let v = serde_json::json!({
            "tag_name": "v0.8.1",
            "body": "Fixes",
            "assets": [
                { "name": "Capri-mac-intel.zip", "browser_download_url": "https://github.com/lachydotmcg/capri-desktop/releases/download/v0.8.1/Capri-mac-intel.zip", "size": 1 },
                { "name": "Capri-Setup.exe", "browser_download_url": "https://github.com/lachydotmcg/capri-desktop/releases/download/v0.8.1/Capri-Setup.exe", "size": 9000, "digest": "sha256:ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef0123456789" }
            ]
        });
        let l = parse_latest(&v).unwrap();
        assert_eq!((l.version.as_str(), l.size), ("0.8.1", 9000));
        assert_eq!(l.sha256.as_deref(), Some("abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"));
        let mut bad = v.clone();
        bad["assets"][1]["browser_download_url"] = serde_json::json!("https://evil.example/Capri-Setup.exe");
        assert!(parse_latest(&bad).is_err(), "only from Capri's own releases");
        assert!(parse_latest(&serde_json::json!({ "tag_name": "v1", "assets": [] })).is_err());
    }

    #[test]
    fn not_installed_here() {
        let t = tempfile::tempdir().unwrap();
        let c = CapriApp::new(t.path(), t.path(), EventBus::new());
        let s = c.status(false);
        assert!(!s.installed);
        c.save(CapriSettings { start_with_omnihub: true }).unwrap();
        assert!(CapriApp::new(t.path(), t.path(), EventBus::new()).settings().start_with_omnihub);
    }
}
