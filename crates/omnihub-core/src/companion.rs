//! Capri (github.com/mochiiwuff/capri-desktop): a desktop companion who
//! narrates what Claude Code, Codex or Antigravity are doing. OmniHub does
//! not ship her (her code is free for non-commercial use; her artwork
//! belongs to its rights holders): switching her on installs her official
//! release from her GitHub page the first time, starts her, and has OmniHub
//! start her with itself. Switching her off closes her and takes her out of
//! Windows' startup list, so she stays off.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::system::procs::ProcessMonitor;

const RELEASES: &str = "https://api.github.com/repos/mochiiwuff/capri-desktop/releases/latest";
pub const EXE: &str = "Capri.exe";
/// Her installer is about 100 MB; anything far bigger isn't it.
const MAX_SETUP: u64 = 400 << 20;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapriStatus {
    pub installed: bool,
    pub version: Option<String>,
    pub running: bool,
    /// The newest release on her page, when asked for.
    pub latest: Option<String>,
}

/// The newest release: its version and its Windows installer.
#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub version: String,
    pub url: String,
    pub size: u64,
    /// "sha256:…" as GitHub publishes it, when it does.
    pub digest: Option<String>,
}

/// The installer in a GitHub "latest release" answer.
pub fn parse_release(json: &serde_json::Value) -> Option<Release> {
    let version = json["tag_name"].as_str()?.trim_start_matches('v').to_string();
    let asset = json["assets"].as_array()?.iter().find(|a| a["name"].as_str().is_some_and(|n| n.to_ascii_lowercase().starts_with("capri-setup") && n.to_ascii_lowercase().ends_with(".exe")))?;
    let url = asset["browser_download_url"].as_str()?.to_string();
    if !url.starts_with("https://github.com/mochiiwuff/capri-desktop/releases/download/") {
        return None;
    }
    Some(Release { version, url, size: asset["size"].as_u64().unwrap_or(0), digest: asset["digest"].as_str().map(str::to_string) })
}

/// Newer than `installed`, comparing the numbers in "0.4.10" and "0.4.9".
pub fn newer(latest: &str, installed: &str) -> bool {
    let nums = |v: &str| v.trim_start_matches('v').split(['.', '-']).map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    nums(latest) > nums(installed)
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(600))).build().into()
}

pub fn latest_release() -> Result<Release, String> {
    let mut res = agent().get(RELEASES).header("User-Agent", "OmniHub").header("Accept", "application/vnd.github+json").call().map_err(|e| format!("Couldn't reach Capri's page on GitHub: {e}"))?;
    let body = res.body_mut().with_config().limit(1 << 20).read_to_string().map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    parse_release(&json).ok_or_else(|| "Capri's newest release has no Windows installer.".into())
}

pub fn status(procs: &ProcessMonitor, check: bool) -> CapriStatus {
    let found = imp::find();
    CapriStatus {
        installed: found.is_some(),
        version: found.and_then(|f| f.1),
        running: !procs.pids_named(EXE).is_empty(),
        latest: if check { latest_release().ok().map(|r| r.version) } else { None },
    }
}

/// Download her installer into `dir` (checked against GitHub's checksum)
/// and run it quietly; she installs for this Windows user only.
pub fn install(dir: &Path, progress: &mut dyn FnMut(&str)) -> Result<String, String> {
    progress("Looking for Capri's newest release");
    let rel = latest_release()?;
    if rel.size > MAX_SETUP {
        return Err("Capri's installer is unexpectedly large; not downloading it.".into());
    }
    progress(&format!("Downloading Capri {} (about {} MB)", rel.version, (rel.size >> 20).max(1)));
    let mut res = agent().get(&rel.url).header("User-Agent", "OmniHub").call().map_err(|e| format!("Capri's download failed: {e}"))?;
    let bytes = res.body_mut().with_config().limit(MAX_SETUP).read_to_vec().map_err(|e| format!("Capri's download failed: {e}"))?;
    if !bytes.starts_with(b"MZ") {
        return Err("Capri's download isn't a Windows program.".into());
    }
    if let Some(want) = rel.digest.as_deref().and_then(|d| d.strip_prefix("sha256:")) {
        use sha2::Digest;
        let got = sha2::Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect::<String>();
        if !got.eq_ignore_ascii_case(want) {
            return Err("Capri's download doesn't match the checksum on her page; not installing it.".into());
        }
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let setup = dir.join(format!("Capri-Setup-{}.exe", rel.version.replace(|c: char| !(c.is_ascii_alphanumeric() || c == '.'), "")));
    std::fs::write(&setup, &bytes).map_err(|e| e.to_string())?;
    progress(&format!("Installing Capri {}", rel.version));
    let r = imp::run_setup(&setup);
    let _ = std::fs::remove_file(&setup);
    r?;
    // Her installer may start her already; either way she starts below.
    std::thread::sleep(Duration::from_millis(500));
    imp::find().ok_or("Capri's installer finished, but she isn't where Windows says she is.")?;
    Ok(rel.version)
}

/// Start her if she is installed and not running.
pub fn start(procs: &ProcessMonitor) -> Result<(), String> {
    let (exe, _) = imp::find().ok_or("Capri isn't installed.")?;
    if !procs.pids_named(EXE).is_empty() {
        return Ok(());
    }
    let mut c = std::process::Command::new(&exe);
    if let Some(d) = exe.parent() {
        c.current_dir(d);
    }
    c.spawn().map(|_| ()).map_err(|e| format!("Capri didn't start: {e}"))
}

/// Close her and keep her from starting with Windows by herself.
pub fn stop(procs: &ProcessMonitor) -> Result<(), String> {
    imp::remove_autostart();
    if procs.pids_named(EXE).is_empty() {
        return Ok(());
    }
    procs.end(EXE).map(|_| ())
}

/// Switch her on (installing her the first time) or off.
pub fn set(procs: &ProcessMonitor, dir: &Path, on: bool, progress: &mut dyn FnMut(&str)) -> Result<CapriStatus, String> {
    if on {
        if imp::find().is_none() {
            install(dir, progress)?;
        }
        progress("Starting Capri");
        start(procs)?;
    } else {
        stop(procs)?;
    }
    Ok(status(procs, false))
}

/// Install her newest release over the one there (she is closed for it and started again).
pub fn update(procs: &ProcessMonitor, dir: &Path, progress: &mut dyn FnMut(&str)) -> Result<CapriStatus, String> {
    let was_running = !procs.pids_named(EXE).is_empty();
    if was_running {
        let _ = procs.end(EXE);
        std::thread::sleep(Duration::from_secs(1));
    }
    install(dir, progress)?;
    if was_running {
        start(procs)?;
    }
    Ok(status(procs, false))
}

/// The port of her local bridge (where Claude Code's hooks reach her).
fn bridge_port() -> u16 {
    std::env::var("CAPRI_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(47_823)
}

/// Is she running and listening?
pub fn listening() -> bool {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_millis(600))).build().into();
    agent.get(format!("http://127.0.0.1:{}/health", bridge_port())).call().is_ok()
}

/// What her settings say about hearing others: (follows Claude Code, her assistant).
fn her_settings() -> (bool, String) {
    // Electron keeps it under her app's name in %APPDATA% (her package or product name).
    let cfg: serde_json::Value = dirs::config_dir()
        .into_iter()
        .flat_map(|d| ["capri-desktop", "Capri"].map(|n| d.join(n).join("config.json")))
        .find_map(|f| std::fs::read_to_string(f).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    (cfg["claude"].as_bool() != Some(false), cfg["assistant"].as_str().unwrap_or("auto").to_string())
}

/// Make Capri say `text` (shown in her panel, tagged `title`), through the
/// same local bridge Claude Code's hooks use: a notification she shows as is.
pub fn say(title: &str, text: &str) -> Result<(), String> {
    let (watching, assistant) = her_settings();
    if !watching {
        return Err("Capri only listens while she follows your coding assistant: turn that on in her menu.".into());
    }
    // She hears only the assistant she watches; its events are marked like this.
    let session = match assistant.as_str() {
        "codex" => "codex:omnihub-play",
        "antigravity" | "gemini" | "agy" => "agy:omnihub-play",
        _ => "omnihub-play",
    };
    let body = serde_json::json!({
        "hook_event_name": "Notification",
        "session_id": session,
        "session_title": title,
        "cwd": "OmniHub",
        "notification_type": "omnihub",
        "message": text,
    });
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_millis(800))).build().into();
    agent.post(format!("http://127.0.0.1:{}/event", bridge_port())).header("content-type", "application/json").send(body.to_string()).map(|_| ()).map_err(|_| "Capri isn't running.".to_string())
}

#[cfg(windows)]
mod imp {
    use std::os::windows::process::CommandExt;

    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
    use winreg::RegKey;

    use super::*;

    const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

    /// Her program and version, from her uninstall entry (her installer is
    /// per-user), else where her installer puts her.
    pub fn find() -> Option<(PathBuf, Option<String>)> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(root) = hkcu.open_subkey_with_flags(UNINSTALL, KEY_READ) {
            for name in root.enum_keys().flatten() {
                let Ok(k) = root.open_subkey_with_flags(&name, KEY_READ) else { continue };
                let display: String = k.get_value("DisplayName").unwrap_or_default();
                if !(display == "Capri" || display.starts_with("Capri ")) {
                    continue;
                }
                let location: String = k.get_value("InstallLocation").unwrap_or_default();
                let exe = PathBuf::from(location.trim_matches('"')).join(EXE);
                if exe.is_file() {
                    return Some((exe, k.get_value::<String, _>("DisplayVersion").ok()));
                }
            }
        }
        let local = dirs::data_local_dir()?;
        ["capri-desktop", "Capri"].iter().map(|d| local.join("Programs").join(d).join(EXE)).find(|p| p.is_file()).map(|p| (p, None))
    }

    pub fn run_setup(setup: &Path) -> Result<(), String> {
        // `/S`: her installer's quiet mode, for this user only (no administrator).
        let status = std::process::Command::new(setup).arg("/S").creation_flags(0x0800_0000).status().map_err(|e| format!("Capri's installer didn't start: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("Capri's installer stopped with code {}.", status.code().unwrap_or(-1)))
        }
    }

    /// Her "Start with Windows" entry, if she made one.
    pub fn remove_autostart() {
        let Ok(run) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN, KEY_READ | KEY_SET_VALUE) else { return };
        let ours: Vec<String> = run.enum_values().flatten().filter(|(_, v)| v.to_string().to_ascii_lowercase().contains(&EXE.to_ascii_lowercase())).map(|(n, _)| n).collect();
        for name in ours {
            let _ = run.delete_value(name);
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn find() -> Option<(PathBuf, Option<String>)> {
        None
    }

    pub fn run_setup(_setup: &Path) -> Result<(), String> {
        Err("Capri's installer is for Windows.".into())
    }

    pub fn remove_autostart() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_her_installer() {
        let json = serde_json::json!({
            "tag_name": "v0.4.4",
            "assets": [
                { "name": "latest.yml", "browser_download_url": "https://github.com/mochiiwuff/capri-desktop/releases/download/v0.4.4/latest.yml", "size": 300 },
                { "name": "Capri-Setup-0.4.4.exe", "browser_download_url": "https://github.com/mochiiwuff/capri-desktop/releases/download/v0.4.4/Capri-Setup-0.4.4.exe", "size": 104_857_600, "digest": "sha256:ab" }
            ]
        });
        let r = parse_release(&json).unwrap();
        assert_eq!((r.version.as_str(), r.size, r.digest.as_deref()), ("0.4.4", 104_857_600, Some("sha256:ab")));
        assert!(r.url.ends_with("Capri-Setup-0.4.4.exe"));
        // Only from her own page.
        let elsewhere = serde_json::json!({ "tag_name": "1.0", "assets": [{ "name": "Capri-Setup-1.0.exe", "browser_download_url": "https://example.com/Capri-Setup-1.0.exe" }] });
        assert!(parse_release(&elsewhere).is_none());
        assert!(parse_release(&serde_json::json!({ "tag_name": "1.0", "assets": [] })).is_none());
    }

    #[test]
    fn versions_compare_by_number() {
        assert!(newer("0.4.10", "0.4.9"));
        assert!(newer("v0.5.0", "0.4.4"));
        assert!(!newer("0.4.4", "0.4.4"));
        assert!(!newer("0.4.3", "0.4.4"));
    }
}
