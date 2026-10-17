//! Bass and treble for everything the PC plays, through Equalizer APO
//! (free, open source; Windows has no system-wide equaliser of its own).
//!
//! OmniHub writes its own `omnihub.txt` (a low shelf for bass, a high
//! shelf for treble, and a preamp so boosts do not clip) and adds one
//! `Include: omnihub.txt` line to Equalizer APO's `config.txt`. Equalizer APO
//! applies changes the moment the file is saved. Where the config folder
//! needs administrator rights, the elevated helper job `eq-write` writes it.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const INCLUDE_FILE: &str = "omnihub.txt";
const INCLUDE_LINE: &str = "Include: omnihub.txt";
/// Limits for the sliders, in dB.
pub const MAX_DB: f32 = 12.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EqStatus {
    /// Equalizer APO is installed.
    pub available: bool,
    /// Our include line is in its config.
    pub hooked: bool,
    pub config_dir: Option<String>,
}

/// Equalizer APO's config folder, if it is installed.
pub fn config_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_64KEY};
        let from_reg = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey_with_flags(r"SOFTWARE\EqualizerAPO", KEY_READ | KEY_WOW64_64KEY)
            .and_then(|k| k.get_value::<String, _>("ConfigPath").or_else(|_| k.get_value::<String, _>("InstallPath").map(|p| format!(r"{p}\config"))))
            .ok()
            .map(PathBuf::from);
        let default = std::env::var_os("ProgramFiles").map(|p| PathBuf::from(p).join(r"EqualizerAPO\config"));
        [from_reg, default].into_iter().flatten().find(|d| d.join("config.txt").is_file())
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("OMNIHUB_EQ_CONFIG_DIR").map(PathBuf::from).filter(|d| d.join("config.txt").is_file())
    }
}

pub fn status() -> EqStatus {
    match config_dir() {
        Some(d) => EqStatus { available: true, hooked: is_hooked(&d), config_dir: Some(d.to_string_lossy().into_owned()) },
        None => EqStatus { available: false, hooked: false, config_dir: None },
    }
}

fn is_hooked(dir: &Path) -> bool {
    std::fs::read_to_string(dir.join("config.txt")).is_ok_and(|c| c.lines().any(|l| l.trim().eq_ignore_ascii_case(INCLUDE_LINE)))
}

/// The filters for a bass and treble setting (dB), or a neutral file when off.
pub fn render(bass: f32, treble: f32, enabled: bool) -> String {
    let clamp = |v: f32| (v.clamp(-MAX_DB, MAX_DB) * 10.0).round() / 10.0;
    let (bass, treble) = (clamp(bass), clamp(treble));
    let mut s = String::from("# Written by OmniHub (bass and treble from the phone or Settings). Edits here are overwritten.\n");
    if !enabled || (bass == 0.0 && treble == 0.0) {
        s.push_str("Preamp: 0 dB\n");
        return s;
    }
    // Headroom for the largest boost, so loud parts do not clip.
    let boost = bass.max(treble).max(0.0);
    s.push_str(&format!("Preamp: {:.1} dB\n", -boost));
    if bass != 0.0 {
        s.push_str(&format!("Filter: ON LSC Fc 105 Hz Gain {bass:.1} dB\n"));
    }
    if treble != 0.0 {
        s.push_str(&format!("Filter: ON HSC Fc 7500 Hz Gain {treble:.1} dB\n"));
    }
    s
}

/// Write our filters and make sure Equalizer APO includes them.
pub fn write(dir: &Path, bass: f32, treble: f32, enabled: bool) -> std::io::Result<()> {
    crate::settings::write_atomic(&dir.join(INCLUDE_FILE), render(bass, treble, enabled).as_bytes())?;
    if !is_hooked(dir) {
        let cfg = dir.join("config.txt");
        let mut text = std::fs::read_to_string(&cfg)?;
        if !text.ends_with('\n') && !text.is_empty() {
            text.push('\n');
        }
        text.push_str(INCLUDE_LINE);
        text.push('\n');
        crate::settings::write_atomic(&cfg, text.as_bytes())?;
    }
    Ok(())
}

/// Apply a setting, asking for administrator approval if the folder needs it.
pub fn apply(bass: f32, treble: f32, enabled: bool) -> Result<EqStatus, String> {
    let dir = config_dir().ok_or("Equalizer APO is not installed on this PC.")?;
    match write(&dir, bass, treble, enabled) {
        Ok(()) => Ok(status()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let args = vec![crate::helper::HELPER_FLAG.to_string(), "eq-write".into(), format!("{bass:.1}"), format!("{treble:.1}"), if enabled { "1" } else { "0" }.into()];
            match crate::system::elevation::run_elevated_and_wait(&exe, &args, false).map_err(|e| e.to_string())? {
                0 => Ok(status()),
                code => Err(format!("Equalizer APO's settings were not changed (code {code}).")),
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Helper job (elevated): validated numbers only.
pub fn helper_write(bass: &str, treble: &str, enabled: &str) -> i32 {
    let (Ok(b), Ok(t)) = (bass.parse::<f32>(), treble.parse::<f32>()) else { return 64 };
    if !b.is_finite() || !t.is_finite() || b.abs() > MAX_DB || t.abs() > MAX_DB || !matches!(enabled, "0" | "1") {
        return 64;
    }
    let Some(dir) = config_dir() else { return 1 };
    match write(&dir, b, t, enabled == "1") {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_and_hook() {
        let r = render(6.0, -3.0, true);
        assert!(r.contains("Preamp: -6.0 dB") && r.contains("Filter: ON LSC Fc 105 Hz Gain 6.0 dB") && r.contains("Filter: ON HSC Fc 7500 Hz Gain -3.0 dB"), "{r}");
        assert!(render(30.0, 0.0, true).contains("Gain 12.0 dB"));
        assert!(!render(6.0, 3.0, false).contains("Filter"));
        assert!(!render(0.0, 0.0, true).contains("Filter"));

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("config.txt"), "Preamp: -2 dB\nInclude: example.txt").unwrap();
        write(dir.path(), 4.0, 0.0, true).unwrap();
        write(dir.path(), 5.0, 0.0, true).unwrap();
        let cfg = std::fs::read_to_string(dir.path().join("config.txt")).unwrap();
        // The user's own lines stay; ours is added once.
        assert_eq!(cfg, "Preamp: -2 dB\nInclude: example.txt\nInclude: omnihub.txt\n");
        assert!(std::fs::read_to_string(dir.path().join(INCLUDE_FILE)).unwrap().contains("Gain 5.0 dB"));
        assert_eq!(helper_write("99", "0", "1"), 64);
        assert_eq!(helper_write("nan", "0", "1"), 64);
        assert_eq!(helper_write("1", "0", "yes"), 64);
    }
}
