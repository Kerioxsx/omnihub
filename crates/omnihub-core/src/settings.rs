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
    pub apps: AppsSettings,
    pub media: MediaSettings,
    pub updates: UpdateSettings,
    pub visuals: VisualSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateSettings {
    /// Look for a new version at start and every few hours.
    pub check: bool,
    /// Install it without asking (when nothing important is going on).
    pub auto_install: bool,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        UpdateSettings { check: true, auto_install: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct MediaSettings {
    /// Paired phones may see what is playing and control it.
    pub allow_phone: bool,
    /// Look lyrics up online (LRCLIB).
    pub lyrics_online: bool,
    /// Bass and treble through Equalizer APO.
    pub eq_enabled: bool,
    pub bass_db: f32,
    pub treble_db: f32,
    /// A folder of your own .lrc files, checked before looking online.
    pub lrc_folder: Option<String>,
    /// Sharper covers from the iTunes catalogue (the player's are small).
    pub hires_art: bool,
}

impl Default for MediaSettings {
    fn default() -> Self {
        MediaSettings { allow_phone: true, lyrics_online: true, eq_enabled: false, bass_db: 0.0, treble_db: 0.0, lrc_folder: None, hires_art: true }
    }
}

/// How the music player shows a song.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum PlayerView {
    /// Light around the screen, a music-reactive scene and floating lyrics.
    #[default]
    Aurora,
    /// The cover and controls with scrolling lyrics (Apple Music style).
    Lyrics,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ColorMode {
    /// From the cover art (the saved colours when there is none).
    #[default]
    Album,
    /// Two colours.
    Duo,
    /// Several colours.
    Multi,
    /// One colour.
    Single,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ColorSettings {
    pub mode: ColorMode,
    pub primary: String,
    pub secondary: String,
    /// For the multi-colour gradient.
    pub colors: Vec<String>,
    /// 0–1.5: saturation of everything drawn.
    pub vividness: f32,
}

impl Default for ColorSettings {
    fn default() -> Self {
        ColorSettings { mode: ColorMode::Album, primary: "#a855f7".into(), secondary: "#ec4899".into(), colors: ["#8b5cf6", "#ec4899", "#3b82f6", "#22d3ee"].map(String::from).to_vec(), vividness: 1.0 }
    }
}

/// How the light around the screen moves.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum GlowAnimation {
    /// With the music (bass, beats, highs).
    #[default]
    Music,
    /// Colours drift slowly, whatever plays.
    Idle,
    /// Still.
    None,
}

/// The light around the screen: a crisp edge, an inner highlight, a glow
/// and a wide bloom, each with its own strength and size.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct GlowSettings {
    pub enabled: bool,
    pub animation: GlowAnimation,
    /// Edge width, pixels.
    pub thickness: f32,
    /// 0–1.5 overall.
    pub intensity: f32,
    pub edge: f32,
    pub highlight: f32,
    pub glow: f32,
    /// Glow reach, pixels.
    pub glow_size: f32,
    pub bloom: f32,
    pub bloom_size: f32,
    /// Corner rounding, pixels.
    pub radius: f32,
}

impl Default for GlowSettings {
    fn default() -> Self {
        GlowSettings { enabled: true, animation: GlowAnimation::Music, thickness: 2.5, intensity: 1.0, edge: 1.0, highlight: 0.6, glow: 0.85, glow_size: 12.0, bloom: 0.3, bloom_size: 48.0, radius: 12.0 }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LyricTiming {
    /// Word by word when the lyrics time each word, else line by line.
    #[default]
    Auto,
    Line,
}

/// Stack: what is being sung, huge, one word at a time when the lyrics time
/// words (else one line), the previous above and the next below, small.
/// Lines: the current line with the next lines under it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LyricLayout {
    #[default]
    Stack,
    Lines,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LyricEmphasis {
    /// White with a glow in the accent colour.
    #[default]
    Glow,
    /// White on a box of the accent colour.
    Box,
    /// In the accent colour.
    Color,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LyricPlace {
    #[default]
    Center,
    Upper,
    Lower,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AuroraLyricsSettings {
    pub layout: LyricLayout,
    /// How the word (or line) being sung stands out.
    pub emphasis: LyricEmphasis,
    /// "display", "condensed", "heavy", "serif" or "impact".
    pub font: String,
    pub visible: bool,
    /// Show by themselves when a song has lyrics.
    pub auto_show: bool,
    /// Text size, × the default.
    pub size: f32,
    pub weight: u16,
    /// Light up the word being sung.
    pub word_highlight: bool,
    /// Lyrics that only time lines (most): spread the words over the line by
    /// their syllables, so the highlight moves word by word (close, not exact).
    pub estimate_words: bool,
    pub timing: LyricTiming,
    pub place: LyricPlace,
    /// Moved by hand, percent of the width/height.
    pub offset_x: f32,
    pub offset_y: f32,
    /// Lines shown: the current one and those after it.
    pub lines: u8,
    /// 0–1: dark backing behind the text.
    pub backing: f32,
    /// None: from the palette.
    pub highlight_color: Option<String>,
    pub glow: f32,
    /// × the default speed of line changes.
    pub transition: f32,
    /// 0–1.5: how much the lyrics move with the music (bounce, sway, pop).
    pub motion: f32,
}

impl Default for AuroraLyricsSettings {
    fn default() -> Self {
        AuroraLyricsSettings { layout: LyricLayout::Stack, emphasis: LyricEmphasis::Glow, font: "display".into(), visible: true, auto_show: true, size: 1.0, weight: 800, word_highlight: true, estimate_words: true, timing: LyricTiming::Auto, place: LyricPlace::Center, offset_x: 0.0, offset_y: 0.0, lines: 3, backing: 0.0, highlight_color: None, glow: 0.6, transition: 1.0, motion: 0.6 }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum SceneStyle {
    /// The cover art, big, glitching and pulsing with the music.
    #[default]
    Visual,
    /// The cover art through a lens that swells with the bass.
    Fisheye,
    /// Both.
    FisheyeVisual,
    /// Mostly dark: the cover small and dim, slow light.
    Minimal,
    /// Flowing colour fields from the palette, no cover.
    Ambient,
}

/// How strongly each effect plays on the cover, 0–1 (0 = off).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct EffectSettings {
    /// Swells with the bass and beats.
    pub zoom: f32,
    /// Rows slipping and blocks jumping on beats.
    pub glitch: f32,
    /// Colour channels splitting apart.
    pub split: f32,
    pub fisheye: f32,
    /// Rings rippling out from the middle.
    pub ripple: f32,
    pub kaleidoscope: f32,
    /// Printed dots.
    pub halftone: f32,
    /// Blocks on beats.
    pub pixelate: f32,
    pub shake: f32,
    /// A larger ghost of the cover behind it.
    pub echo: f32,
    pub scanlines: f32,
    /// The cover recoloured in the palette.
    pub duotone: f32,
    /// A swirl from the middle.
    pub twist: f32,
    /// Bright parts glowing.
    pub bloom: f32,
}

impl Default for EffectSettings {
    fn default() -> Self {
        EffectSettings { zoom: 0.5, glitch: 0.3, split: 0.35, fisheye: 0.0, ripple: 0.25, kaleidoscope: 0.0, halftone: 0.0, pixelate: 0.0, shake: 0.15, echo: 0.4, scanlines: 0.0, duotone: 0.0, twist: 0.15, bloom: 0.4 }
    }
}

/// The music-reactive scene behind the lyrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SceneSettings {
    pub enabled: bool,
    pub style: SceneStyle,
    pub intensity: f32,
    pub speed: f32,
    /// 0–1: softness of the light shapes.
    pub blur: f32,
    pub saturation: f32,
    /// 0–1: how much of the scene shows over the dark background.
    pub opacity: f32,
    /// A blurred copy of the cover art underneath.
    pub artwork: bool,
    /// A ring that follows the spectrum.
    pub waveform: bool,
    pub effects: EffectSettings,
}

impl Default for SceneSettings {
    fn default() -> Self {
        SceneSettings { enabled: true, style: SceneStyle::Visual, intensity: 0.8, speed: 1.0, blur: 0.1, saturation: 1.1, opacity: 0.9, artwork: true, waveform: false, effects: EffectSettings::default() }
    }
}

/// The glow around a monitor, over every app (a click-through window).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DesktopGlowSettings {
    pub enabled: bool,
    /// Floating lyrics too.
    pub lyrics: bool,
    /// "primary", "all", or a monitor's name.
    pub display: String,
    /// Stay above the taskbar.
    pub clear_taskbar: bool,
    /// Step aside for full-screen apps and games (and boosted games).
    pub hide_fullscreen: bool,
    /// Turn the desktop glow on when OmniHub starts.
    pub start_with_app: bool,
}

impl Default for DesktopGlowSettings {
    fn default() -> Self {
        DesktopGlowSettings { enabled: false, lyrics: false, display: "primary".into(), clear_taskbar: true, hide_fullscreen: true, start_with_app: false }
    }
}

/// Aurora: the music-reactive light, scene and lyrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct VisualSettings {
    pub view: PlayerView,
    /// Everything Aurora draws (the lyrics stay).
    pub enabled: bool,
    /// Listen to what the PC plays so the light follows the music.
    pub audio_reactive: bool,
    /// 0.2–3.
    pub sensitivity: f32,
    /// 0–2.
    pub bass: f32,
    /// 0–1.
    pub smoothing: f32,
    pub reduced_motion: bool,
    /// No bright pulses on beats.
    pub no_flashes: bool,
    /// × animation speed.
    pub speed: f32,
    /// Keep the screen on while the player shows Aurora and music plays.
    pub keep_awake: bool,
    pub color: ColorSettings,
    pub glow: GlowSettings,
    pub lyrics: AuroraLyricsSettings,
    pub scene: SceneSettings,
    pub desktop: DesktopGlowSettings,
}

impl Default for VisualSettings {
    fn default() -> Self {
        VisualSettings {
            view: PlayerView::Aurora,
            enabled: true,
            audio_reactive: true,
            sensitivity: 1.0,
            bass: 1.0,
            smoothing: 0.5,
            reduced_motion: false,
            no_flashes: false,
            speed: 1.0,
            keep_awake: true,
            color: ColorSettings::default(),
            glow: GlowSettings::default(),
            lyrics: AuroraLyricsSettings::default(),
            scene: SceneSettings::default(),
            desktop: DesktopGlowSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AppsSettings {
    /// IDs of apps pinned as favourites.
    pub favorites: Vec<String>,
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
    /// Name used in greetings; empty = the Windows account's first name.
    pub display_name: String,
    /// Interface size in percent (80–150).
    pub ui_scale: u16,
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
            display_name: String::new(),
            ui_scale: 100,
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
    /// How the Explorer tab shows a folder.
    pub explorer_view: ExplorerView,
    /// Tile size of the grid view: "sm", "md" or "lg".
    pub grid_size: String,
    /// Show previews of images in the grid view.
    pub grid_previews: bool,
    /// Warn when a drive runs low on space.
    pub low_space_alert: bool,
    /// "Low" means less than this share of the drive free.
    pub low_space_percent: u8,
}

impl Default for StorageSettings {
    fn default() -> Self {
        StorageSettings {
            default_mode: ScanMode::Fast,
            exclude: Vec::new(),
            cleanup: CleanupOptions::default(),
            show_hidden: true,
            size_metric: SizeMetric::Size,
            explorer_view: ExplorerView::Split,
            grid_size: "md".into(),
            grid_previews: true,
            low_space_alert: true,
            low_space_percent: 10,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ExplorerView {
    /// Treemap next to the details list.
    #[default]
    Split,
    /// Details list across the whole width.
    List,
    /// Tiles with icons or image previews.
    Grid,
    /// The treemap alone.
    Treemap,
    /// Rings: each folder's share of its parent.
    Sunburst,
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
    /// Let the OmniHub browser extension (Brave, Chrome, Edge) fill logins.
    pub browser_autofill: bool,
    /// The extension offers to save logins typed on websites.
    pub browser_offer_save: bool,
}

impl Default for VaultSettings {
    fn default() -> Self {
        VaultSettings { auto_lock_minutes: 5, clipboard_clear_seconds: 20, lock_on_session_lock: true, allow_phone: false, hello_enabled: false, browser_autofill: false, browser_offer_save: true }
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
    /// Paired phones may put text on this PC's clipboard.
    pub allow_clipboard: bool,
    /// Paired phones may see what's running and end tasks.
    pub allow_tasks: bool,
    /// "Send to → OmniHub (phone)" in Explorer's right-click menu.
    pub send_to_menu: bool,
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
            allow_clipboard: true,
            allow_tasks: true,
            send_to_menu: true,
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
    /// iPhone → PC mirroring (AirPlay receiver).
    pub airplay: crate::capture::airplay::AirPlayOptions,
    /// Keep the iPhone window above other windows.
    pub airplay_keep_on_top: bool,
    /// Put the iPhone window small in the bottom-right corner when it opens.
    pub airplay_pip: bool,
    /// Start the receiver whenever OmniHub starts.
    pub airplay_auto_start: bool,
    /// Use this uxplay instead of the add-on.
    pub uxplay_path: Option<String>,
}

impl Default for ScreenShareSettings {
    fn default() -> Self {
        ScreenShareSettings {
            preset: "balanced".into(),
            max_fps: 60,
            scrcpy_path: None,
            sunshine_path: None,
            airplay: crate::capture::airplay::AirPlayOptions { name: format!("{} (OmniHub)", gethostname::gethostname().to_string_lossy()), ..Default::default() },
            airplay_keep_on_top: false,
            airplay_pip: false,
            airplay_auto_start: false,
            uxplay_path: None,
        }
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

    #[test]
    fn aurora_settings_persist_and_reset() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let store = SettingsStore::load(&path);
        let d = Settings::default().visuals;
        assert_eq!((d.view, d.scene.style, d.lyrics.layout, d.glow.animation), (PlayerView::Aurora, SceneStyle::Visual, LyricLayout::Stack, GlowAnimation::Music));
        let s = store
            .update(&serde_json::json!({ "visuals": { "view": "lyrics", "sensitivity": 2.0, "color": { "mode": "duo", "primary": "#ff0000", "colors": ["#000000"] }, "glow": { "thickness": 6.0, "animation": "idle" }, "lyrics": { "font": "condensed", "emphasis": "box", "highlightColor": "#00ff00", "place": "lower" }, "scene": { "style": "fisheyeVisual" }, "desktop": { "enabled": true, "display": "all" } } }))
            .unwrap();
        // After a restart.
        let again = SettingsStore::load(&path).get().visuals;
        assert_eq!(again, s.visuals);
        assert_eq!((again.view, again.color.mode, again.color.colors.len(), again.scene.style, again.lyrics.highlight_color.as_deref()), (PlayerView::Lyrics, ColorMode::Duo, 1, SceneStyle::FisheyeVisual, Some("#00ff00")));
        assert!((again.sensitivity - 2.0).abs() < 1e-6 && again.desktop.enabled);
        // "Reset to defaults" sends every default (null clears the highlight colour).
        let mut reset = serde_json::to_value(Settings::default().visuals).unwrap();
        reset["view"] = serde_json::json!("lyrics");
        reset["lyrics"]["highlightColor"] = serde_json::Value::Null;
        let back = store.update(&serde_json::json!({ "visuals": reset })).unwrap().visuals;
        assert_eq!(back, VisualSettings { view: PlayerView::Lyrics, ..Default::default() });
    }
}
