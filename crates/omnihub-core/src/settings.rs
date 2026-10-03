//! User settings, stored as JSON in the config directory.
//!
//! The UI updates settings with a JSON merge patch, so adding a field never
//! needs a new command, and unknown or missing fields fall back to defaults.

use std::path::{Path, PathBuf};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::storage::cleanup::CleanupOptions;
use crate::storage::engine::ScanMode;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
#[derive(Default)]
pub struct Settings {
    pub general: GeneralSettings,
    pub storage: StorageSettings,
    pub notes: NotesSettings,
    pub vault: VaultSettings,
    pub remote: RemoteSettings,
    pub screenshots: ScreenshotSettings,
    pub screen: ScreenShareSettings,
}


#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct GeneralSettings {
    pub theme: Theme,
    pub accent: String,
    pub reduced_motion: bool,
    pub launch_at_login: bool,
    pub start_minimized: bool,
    pub close_to_tray: bool,
    pub onboarded: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        GeneralSettings {
            theme: Theme::System,
            accent: "violet".into(),
            reduced_motion: false,
            launch_at_login: false,
            start_minimized: false,
            close_to_tray: true,
            onboarded: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct StorageSettings {
    pub default_mode: ScanMode,
    pub exclude: Vec<String>,
    pub cleanup: CleanupOptions,
    pub show_hidden: bool,
    pub size_metric: SizeMetric,
}

impl Default for StorageSettings {
    fn default() -> Self {
        StorageSettings {
            default_mode: ScanMode::Fast,
            exclude: Vec::new(),
            cleanup: CleanupOptions::default(),
            show_hidden: true,
            size_metric: SizeMetric::Size,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SizeMetric {
    #[default]
    Size,
    Alloc,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NotesSettings {
    /// Folder that "Send to Claude" writes into (watched for replies).
    pub claude_folder: Option<String>,
    /// Also write a `.json` file with the same data next to each `.md`.
    pub sidecar_json: bool,
    /// Export ideas automatically every time they are saved.
    pub auto_export_ideas: bool,
    /// Keep an `INDEX.md` listing every exported idea.
    pub index_file: bool,
}

impl Default for NotesSettings {
    fn default() -> Self {
        NotesSettings { claude_folder: None, sidecar_json: false, auto_export_ideas: false, index_file: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct VaultSettings {
    pub auto_lock_minutes: u32,
    pub clipboard_clear_seconds: u32,
    pub lock_on_session_lock: bool,
    /// Let paired phones list and reveal entries (needs HTTPS and the master password).
    pub allow_phone: bool,
    pub hello_enabled: bool,
}

impl Default for VaultSettings {
    fn default() -> Self {
        VaultSettings { auto_lock_minutes: 5, clipboard_clear_seconds: 20, lock_on_session_lock: true, allow_phone: false, hello_enabled: false }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Bind {
    /// Reachable from phones on the same network (private addresses only).
    #[default]
    Lan,
    /// Only this PC (useful with a USB tether or a reverse tunnel).
    Localhost,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum BrowseScope {
    /// Desktop, Documents, Downloads, Pictures, Videos, Music.
    #[default]
    UserFolders,
    /// Every drive.
    AllDrives,
    /// Only `custom_roots`.
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RemoteSettings {
    pub enabled: bool,
    pub port: u16,
    pub bind: Bind,
    pub tls: bool,
    /// Also accept Tailscale/CGNAT addresses (100.64.0.0/10).
    pub allow_tailscale: bool,
    pub browse_scope: BrowseScope,
    pub custom_roots: Vec<String>,
    pub allow_uploads: bool,
    pub allow_power: bool,
    pub power_delay_seconds: u32,
    pub allow_screen: bool,
    pub allow_control: bool,
    pub allow_app_launch: bool,
    pub allow_notes: bool,
    /// Where files sent from phones land (default: Downloads\OmniHub).
    pub incoming_dir: Option<String>,
    pub device_name: String,
}

impl Default for RemoteSettings {
    fn default() -> Self {
        RemoteSettings {
            enabled: false,
            port: 47800,
            bind: Bind::Lan,
            tls: true,
            allow_tailscale: false,
            browse_scope: BrowseScope::UserFolders,
            custom_roots: Vec::new(),
            allow_uploads: true,
            allow_power: true,
            power_delay_seconds: 10,
            allow_screen: true,
            allow_control: false,
            allow_app_launch: true,
            allow_notes: true,
            incoming_dir: None,
            device_name: gethostname::gethostname().to_string_lossy().to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ImageFormat {
    #[default]
    Png,
    Jpeg,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenshotSettings {
    pub dir: Option<String>,
    pub format: ImageFormat,
    pub hotkey_region: String,
    pub hotkey_full: String,
    pub hotkey_window: String,
    pub copy_to_clipboard: bool,
}

impl Default for ScreenshotSettings {
    fn default() -> Self {
        ScreenshotSettings {
            dir: None,
            format: ImageFormat::Png,
            hotkey_region: "Alt+Shift+S".into(),
            hotkey_full: "Alt+Shift+A".into(),
            hotkey_window: "Alt+Shift+W".into(),
            copy_to_clipboard: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenShareSettings {
    pub preset: String,
    pub max_fps: u32,
    pub scrcpy_path: Option<String>,
    pub sunshine_path: Option<String>,
}

impl Default for ScreenShareSettings {
    fn default() -> Self {
        ScreenShareSettings { preset: "balanced".into(), max_fps: 60, scrcpy_path: None, sunshine_path: None }
    }
}

pub struct SettingsStore {
    path: PathBuf,
    current: RwLock<Settings>,
}

/// RFC 7396 JSON merge patch.
pub fn merge_patch(target: &mut serde_json::Value, patch: &serde_json::Value) {
    match (target, patch) {
        (serde_json::Value::Object(t), serde_json::Value::Object(p)) => {
            for (k, v) in p {
                if v.is_null() {
                    t.remove(k);
                } else {
                    merge_patch(t.entry(k.clone()).or_insert(serde_json::Value::Null), v);
                }
            }
        }
        (t, p) => *t = p.clone(),
    }
}

impl SettingsStore {
    pub fn load(path: &Path) -> Self {
        let current = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Settings>(&b).ok())
            .unwrap_or_default();
        SettingsStore { path: path.to_path_buf(), current: RwLock::new(current) }
    }

    pub fn get(&self) -> Settings {
        self.current.read().clone()
    }

    pub fn update(&self, patch: &serde_json::Value) -> std::io::Result<Settings> {
        let mut cur = self.current.write();
        let mut value = serde_json::to_value(&*cur)?;
        merge_patch(&mut value, patch);
        let next: Settings = serde_json::from_value(value).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        write_atomic(&self.path, &serde_json::to_vec_pretty(&next)?)?;
        *cur = next.clone();
        Ok(next)
    }

    pub fn replace(&self, next: Settings) -> std::io::Result<()> {
        write_atomic(&self.path, &serde_json::to_vec_pretty(&next)?)?;
        *self.current.write() = next;
        Ok(())
    }
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension(format!("{}.tmp", path.extension().and_then(|e| e.to_str()).unwrap_or("")));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe() {
        let s = Settings::default();
        assert!(!s.remote.enabled, "companion server must be off by default");
        assert!(!s.remote.allow_control, "remote control must be opt-in");
        assert!(!s.vault.allow_phone, "phone vault access must be opt-in");
        assert!(s.remote.tls);
        assert_eq!(s.remote.browse_scope, BrowseScope::UserFolders);
    }

    #[test]
    fn patch_and_persist() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = SettingsStore::load(&path);
        let s = store.update(&serde_json::json!({ "remote": { "enabled": true, "port": 50000 }, "general": { "theme": "dark" } })).unwrap();
        assert!(s.remote.enabled);
        assert_eq!(s.remote.port, 50000);
        assert!(s.remote.tls, "untouched fields keep their value");
        let again = SettingsStore::load(&path).get();
        assert_eq!(again, s);
        assert!(store.update(&serde_json::json!({ "remote": { "port": "nope" } })).is_err());
        assert_eq!(store.get().remote.port, 50000);
        // Unknown/old files still load.
        std::fs::write(&path, br#"{"general":{"theme":"light","removedField":1}}"#).unwrap();
        assert_eq!(SettingsStore::load(&path).get().general.theme, Theme::Light);
    }
}
