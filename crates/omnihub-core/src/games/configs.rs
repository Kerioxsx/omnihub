//! A game's own settings, set for frames per second: Fortnite's
//! GameUserSettings.ini and Minecraft's options.txt. OmniHub writes the
//! same values the game's settings menu would, keeps a copy of the file
//! from before its first change, and can put it back.
//!
//! Only settings the game is known to read are added when missing; the
//! rest are changed only when the game already wrote them, so a game
//! update that renames something is left alone rather than guessed at.
//! Games rewrite these files when they close, so nothing is written while
//! the game runs.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::ini::Doc;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ConfigGame {
    Fortnite,
    Minecraft,
}

impl ConfigGame {
    pub fn all() -> [ConfigGame; 2] {
        [ConfigGame::Fortnite, ConfigGame::Minecraft]
    }

    pub fn label(self) -> &'static str {
        match self {
            ConfigGame::Fortnite => "Fortnite",
            ConfigGame::Minecraft => "Minecraft",
        }
    }

    /// The program that rewrites the file when it closes.
    pub fn process(self) -> &'static str {
        match self {
            ConfigGame::Fortnite => "FortniteClient-Win64-Shipping.exe",
            ConfigGame::Minecraft => "javaw.exe",
        }
    }

    /// Where the game keeps the file for this Windows user.
    pub fn default_path(self) -> Option<PathBuf> {
        Some(match self {
            ConfigGame::Fortnite => dirs::data_local_dir()?.join("FortniteGame").join("Saved").join("Config").join("WindowsClient").join("GameUserSettings.ini"),
            // %APPDATA%\.minecraft on Windows.
            ConfigGame::Minecraft => dirs::config_dir()?.join(".minecraft").join("options.txt"),
        })
    }

    fn slug(self) -> &'static str {
        match self {
            ConfigGame::Fortnite => "fortnite",
            ConfigGame::Minecraft => "minecraft",
        }
    }

    fn read(self, bytes: &[u8]) -> Doc {
        match self {
            ConfigGame::Fortnite => Doc::ini(bytes),
            ConfigGame::Minecraft => Doc::colon(bytes),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FortniteOptions {
    /// Frames per second the game may draw; 0 = unlimited.
    pub frame_limit: u32,
    /// Fortnite's "Performance" rendering mode (the fastest by far).
    pub performance_mode: bool,
    /// Every quality setting at its lowest (no shadows, effects, grass…).
    pub lowest_quality: bool,
    /// 0 near, 1 medium, 2 far, 3 epic.
    pub view_distance: u8,
    /// 3D resolution, percent of the screen (lower is faster and blurrier).
    pub resolution_scale: u8,
    /// Fortnite's own FPS counter in the corner.
    pub show_fps: bool,
    /// Fullscreen rather than windowed (lowest input delay).
    pub fullscreen: bool,
}

impl Default for FortniteOptions {
    fn default() -> Self {
        FortniteOptions { frame_limit: 0, performance_mode: true, lowest_quality: true, view_distance: 1, resolution_scale: 100, show_fps: true, fullscreen: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct MinecraftOptions {
    pub unlimited_fps: bool,
    /// "Fast" graphics, no smooth lighting, shadows or mipmaps.
    pub fast_graphics: bool,
    pub minimal_particles: bool,
    pub no_clouds: bool,
    /// Chunks; None keeps yours.
    pub render_distance: Option<u8>,
}

impl Default for MinecraftOptions {
    fn default() -> Self {
        MinecraftOptions { unlimited_fps: true, fast_graphics: true, minimal_particles: true, no_clouds: true, render_distance: None }
    }
}

/// The options for every supported game.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfigOptions {
    pub fortnite: FortniteOptions,
    pub minecraft: MinecraftOptions,
}

/// One value Optimize sets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub label: String,
    pub key: String,
    pub from: Option<String>,
    pub to: String,
}

/// A current setting, for display.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Setting {
    pub label: String,
    pub value: String,
    /// Already the fast choice.
    pub good: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigStatus {
    pub game: ConfigGame,
    pub path: Option<String>,
    /// The settings file exists (the game has run on this PC).
    pub found: bool,
    pub running: bool,
    pub settings: Vec<Setting>,
    /// What Optimize would change; empty when already optimized.
    pub pending: Vec<Change>,
    /// When the copy of your own settings was taken (Unix seconds).
    pub backup_at: Option<i64>,
}

struct Edit {
    section: Option<&'static str>,
    key: &'static str,
    value: String,
    /// Add it when the file doesn't have it yet.
    add: bool,
    label: String,
}

const FN_SECTION: &str = "/Script/FortniteGame.FortGameUserSettings";
const FN_SCALABILITY: &str = "ScalabilityGroups";
const FN_RHI: &str = "D3DRHIPreference";

fn edit(section: Option<&'static str>, key: &'static str, value: impl Into<String>, add: bool, label: impl Into<String>) -> Edit {
    Edit { section, key, value: value.into(), add, label: label.into() }
}

fn fortnite_edits(o: &FortniteOptions) -> Vec<Edit> {
    let s = Some(FN_SECTION);
    let mut e = vec![
        edit(s, "FrameRateLimit", format!("{}.000000", o.frame_limit), true, if o.frame_limit == 0 { "Frame rate limit: unlimited".to_string() } else { format!("Frame rate limit: {} FPS", o.frame_limit) }),
        edit(s, "bUseVSync", "False", true, "VSync off"),
        edit(s, "bUseDynamicResolution", "False", false, "Dynamic resolution off"),
        edit(s, "bMotionBlur", "False", false, "Motion blur off"),
        edit(s, "bShowFPS", if o.show_fps { "True" } else { "False" }, true, if o.show_fps { "FPS counter on" } else { "FPS counter off" }),
        edit(s, "bDisableMouseAcceleration", "True", false, "Mouse acceleration off"),
    ];
    if o.fullscreen {
        for key in ["FullscreenMode", "LastConfirmedFullscreenMode", "PreferredFullscreenMode"] {
            e.push(edit(s, key, "0", false, "Window mode: fullscreen"));
        }
    }
    if o.performance_mode {
        // The feature level Epic's own "-FeatureLevelES31" switch selects.
        e.push(edit(Some(FN_RHI), "PreferredFeatureLevel", "es31", true, "Rendering mode: Performance"));
        e.push(edit(Some("PerformanceMode"), "MeshQuality", "0", false, "Performance-mode meshes: low"));
    }
    if o.performance_mode || o.lowest_quality {
        e.push(edit(s, "bRayTracing", "False", false, "Ray tracing off"));
    }
    let g = Some(FN_SCALABILITY);
    e.push(edit(g, "sg.ResolutionQuality", format!("{}.000000", o.resolution_scale.clamp(25, 100)), true, format!("3D resolution: {}%", o.resolution_scale.clamp(25, 100))));
    if o.lowest_quality {
        e.push(edit(s, "bShowGrass", "False", false, "Grass off"));
        e.push(edit(g, "sg.ViewDistanceQuality", o.view_distance.min(3).to_string(), true, format!("View distance: {}", ["near", "medium", "far", "epic"][o.view_distance.min(3) as usize])));
        for (key, what) in [
            ("sg.AntiAliasingQuality", "Anti-aliasing"),
            ("sg.ShadowQuality", "Shadows"),
            ("sg.GlobalIlluminationQuality", "Global illumination"),
            ("sg.ReflectionQuality", "Reflections"),
            ("sg.PostProcessQuality", "Post-processing"),
            ("sg.TextureQuality", "Textures"),
            ("sg.EffectsQuality", "Effects"),
            ("sg.FoliageQuality", "Foliage"),
            ("sg.ShadingQuality", "Shading"),
        ] {
            e.push(edit(g, key, "0", true, format!("{what}: low")));
        }
    }
    e
}

fn minecraft_edits(o: &MinecraftOptions, doc: &Doc) -> Vec<Edit> {
    // Minecraft has spelled values differently over the years; follow the file.
    let boolish = |key: &str, on: bool| -> String {
        match doc.get(None, key).as_deref() {
            Some(v) if v.parse::<i64>().is_ok() => (on as i32).to_string(),
            _ => on.to_string(),
        }
    };
    let mut e = Vec::new();
    if o.unlimited_fps {
        // 260 is the slider's "Unlimited".
        e.push(edit(None, "maxFps", "260", false, "Max frame rate: unlimited"));
        e.push(edit(None, "enableVsync", "false", false, "VSync off"));
    }
    if o.fast_graphics {
        e.push(edit(None, "graphicsMode", "0", false, "Graphics: fast"));
        e.push(edit(None, "fancyGraphics", "false", false, "Graphics: fast"));
        e.push(edit(None, "ao", boolish("ao", false), false, "Smooth lighting off"));
        e.push(edit(None, "entityShadows", "false", false, "Entity shadows off"));
        e.push(edit(None, "mipmapLevels", "0", false, "Mipmap levels: off"));
        e.push(edit(None, "biomeBlendRadius", "0", false, "Biome blend: off"));
    }
    if o.minimal_particles {
        e.push(edit(None, "particles", "2", false, "Particles: minimal"));
    }
    if o.no_clouds {
        let quoted = doc.get(None, "renderClouds").is_some_and(|v| v.starts_with('"'));
        e.push(edit(None, "renderClouds", if quoted { "\"false\"" } else { "false" }, false, "Clouds off"));
    }
    if let Some(d) = o.render_distance {
        e.push(edit(None, "renderDistance", d.clamp(2, 32).to_string(), false, format!("Render distance: {} chunks", d.clamp(2, 32))));
    }
    e
}

fn edits(game: ConfigGame, opts: &ConfigOptions, doc: &Doc) -> Vec<Edit> {
    match game {
        ConfigGame::Fortnite => fortnite_edits(&opts.fortnite),
        ConfigGame::Minecraft => minecraft_edits(&opts.minecraft, doc),
    }
}

/// "144.000000" and "144" are the same setting.
fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b) || matches!((a.parse::<f64>(), b.parse::<f64>()), (Ok(x), Ok(y)) if x == y)
}

/// What applying `opts` would change in `doc`.
pub fn pending(game: ConfigGame, opts: &ConfigOptions, doc: &Doc) -> Vec<Change> {
    let mut out: Vec<Change> = Vec::new();
    for e in edits(game, opts, doc) {
        let current = doc.get(e.section, e.key);
        let needed = match &current {
            Some(v) => !same(v, &e.value),
            None => e.add,
        };
        if needed {
            out.push(Change { label: e.label, key: e.key.to_string(), from: current, to: e.value });
        }
    }
    out
}

/// Apply `opts` to `doc`; returns how many values changed.
pub fn apply_to(game: ConfigGame, opts: &ConfigOptions, doc: &mut Doc) -> usize {
    let list = edits(game, opts, doc);
    let mut n = 0;
    for e in list {
        if doc.get(e.section, e.key).is_some_and(|v| same(&v, &e.value)) {
            continue;
        }
        if doc.set(e.section, e.key, &e.value, e.add) {
            n += 1;
        }
    }
    n
}

/// The settings a player cares about, as the file has them now.
pub fn settings(game: ConfigGame, doc: &Doc) -> Vec<Setting> {
    let row = |label: &str, value: String, good: bool| Setting { label: label.into(), value, good };
    match game {
        ConfigGame::Fortnite => {
            let s = Some(FN_SECTION);
            let limit = doc.get(s, "FrameRateLimit").and_then(|v| v.parse::<f64>().ok());
            let level = doc.get(Some(FN_RHI), "PreferredFeatureLevel").unwrap_or_default();
            let rhi = doc.get(Some(FN_RHI), "PreferredRHI").unwrap_or_default();
            let perf = level.eq_ignore_ascii_case("es31");
            let on = |k: &str| doc.get(s, k).is_some_and(|v| v.eq_ignore_ascii_case("true"));
            let shadows = doc.get(Some(FN_SCALABILITY), "sg.ShadowQuality").and_then(|v| v.parse::<i32>().ok());
            let mode = doc.get(s, "FullscreenMode").and_then(|v| v.parse::<i32>().ok());
            vec![
                row("Frame rate limit", match limit { Some(l) if l <= 0.0 => "Unlimited".into(), Some(l) => format!("{l:.0} FPS"), None => "Not set".into() }, limit.is_some_and(|l| l <= 0.0 || l >= 240.0)),
                row("Rendering mode", if perf { "Performance".into() } else if rhi.eq_ignore_ascii_case("dx11") { "DirectX 11".into() } else if rhi.eq_ignore_ascii_case("dx12") { "DirectX 12".into() } else { "Game's choice".into() }, perf),
                row("VSync", if on("bUseVSync") { "On".into() } else { "Off".into() }, !on("bUseVSync")),
                row("Shadows", match shadows { Some(0) => "Off".into(), Some(n) => ["Off", "Medium", "High", "Epic", "Cinematic"].get(n as usize).copied().unwrap_or("High").into(), None => "Game's choice".into() }, shadows == Some(0)),
                row("Window mode", match mode { Some(0) => "Fullscreen".into(), Some(1) => "Windowed fullscreen".into(), Some(2) => "Windowed".into(), _ => "Game's choice".into() }, mode == Some(0)),
                row("FPS counter", if on("bShowFPS") { "On".into() } else { "Off".into() }, on("bShowFPS")),
            ]
        }
        ConfigGame::Minecraft => {
            let fps = doc.get(None, "maxFps").and_then(|v| v.parse::<u32>().ok());
            let vsync = doc.get(None, "enableVsync").is_some_and(|v| v == "true");
            let graphics = doc.get(None, "graphicsMode").and_then(|v| v.parse::<u32>().ok());
            let distance = doc.get(None, "renderDistance").unwrap_or_else(|| "?".into());
            vec![
                row("Max frame rate", match fps { Some(f) if f >= 260 => "Unlimited".into(), Some(f) => format!("{f} FPS"), None => "Not set".into() }, fps.is_some_and(|f| f >= 260)),
                row("VSync", if vsync { "On".into() } else { "Off".into() }, !vsync),
                row("Graphics", match graphics { Some(0) => "Fast".into(), Some(1) => "Fancy".into(), Some(2) => "Fabulous".into(), _ => "Game's choice".into() }, graphics == Some(0)),
                row("Render distance", format!("{distance} chunks"), true),
            ]
        }
    }
}

/// Where OmniHub keeps the copy of your own settings.
fn backup_path(game: ConfigGame, path: &Path, backups: &Path) -> PathBuf {
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|| "txt".into());
    backups.join(format!("{}-yours.{ext}", game.slug()))
}

fn mtime(p: &Path) -> Option<i64> {
    let t = std::fs::metadata(p).ok()?.modified().ok()?;
    Some(t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64)
}

pub fn status(game: ConfigGame, opts: &ConfigOptions, path: Option<&Path>, backups: &Path, running: bool) -> ConfigStatus {
    let doc = path.and_then(|p| std::fs::read(p).ok()).map(|b| game.read(&b));
    ConfigStatus {
        game,
        path: path.map(|p| p.to_string_lossy().into_owned()),
        found: doc.is_some(),
        running,
        settings: doc.as_ref().map(|d| settings(game, d)).unwrap_or_default(),
        pending: doc.as_ref().map(|d| pending(game, opts, d)).unwrap_or_default(),
        backup_at: path.map(|p| backup_path(game, p, backups)).and_then(|b| mtime(&b)),
    }
}

/// Write a file the game may have marked read-only, keeping that mark.
fn write_keeping_readonly(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let perms = std::fs::metadata(path)?.permissions();
    let readonly = perms.readonly();
    if readonly {
        let mut p = perms.clone();
        #[allow(clippy::permissions_set_readonly_false)]
        p.set_readonly(false);
        std::fs::set_permissions(path, p)?;
    }
    let tmp = path.with_extension("omnihub-tmp");
    let r = std::fs::write(&tmp, bytes).and_then(|_| std::fs::rename(&tmp, path));
    if r.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    if readonly {
        let mut p = std::fs::metadata(path)?.permissions();
        p.set_readonly(true);
        std::fs::set_permissions(path, p)?;
    }
    r
}

/// Apply `opts` to the game's file, copying your own settings away first
/// (only the first time, so Restore always brings back yours).
pub fn apply(game: ConfigGame, opts: &ConfigOptions, path: &Path, backups: &Path) -> Result<Vec<Change>, String> {
    let bytes = std::fs::read(path).map_err(|_| format!("{} has no settings file yet. Start it once, close it, then try again.", game.label()))?;
    let mut doc = game.read(&bytes);
    let changes = pending(game, opts, &doc);
    if changes.is_empty() {
        return Ok(changes);
    }
    std::fs::create_dir_all(backups).map_err(|e| e.to_string())?;
    let backup = backup_path(game, path, backups);
    if !backup.exists() {
        std::fs::write(&backup, &bytes).map_err(|e| format!("Could not keep a copy of your settings: {e}"))?;
    }
    apply_to(game, opts, &mut doc);
    write_keeping_readonly(path, &doc.to_bytes()).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(changes)
}

/// Put your own settings back (the copy from before OmniHub's first change).
pub fn restore(game: ConfigGame, path: &Path, backups: &Path) -> Result<(), String> {
    let backup = backup_path(game, path, backups);
    let bytes = std::fs::read(&backup).map_err(|_| "There is no copy of your own settings to restore.".to_string())?;
    write_keeping_readonly(path, &bytes).or_else(|_| std::fs::write(path, &bytes)).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    let _ = std::fs::remove_file(&backup);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FORTNITE: &str = "[/Script/FortniteGame.FortGameUserSettings]\r\nbUseVSync=True\r\nbMotionBlur=True\r\nbShowGrass=True\r\nbShowFPS=False\r\nFullscreenMode=1\r\nLastConfirmedFullscreenMode=1\r\nPreferredFullscreenMode=1\r\nFrameRateLimit=144.000000\r\nbUseDynamicResolution=False\r\n\r\n[ScalabilityGroups]\r\nsg.ResolutionQuality=100.000000\r\nsg.ViewDistanceQuality=3\r\nsg.ShadowQuality=3\r\nsg.TextureQuality=3\r\n\r\n[D3DRHIPreference]\r\nPreferredRHI=dx12\r\nPreferredFeatureLevel=sm6\r\n";

    #[test]
    fn fortnite_goes_fast_and_comes_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("GameUserSettings.ini");
        let backups = dir.path().join("backups");
        std::fs::write(&path, FORTNITE).unwrap();
        let opts = ConfigOptions::default();

        let before = status(ConfigGame::Fortnite, &opts, Some(&path), &backups, false);
        assert!(before.found);
        assert_eq!(before.settings[0].value, "144 FPS");
        assert_eq!(before.settings[1].value, "DirectX 12");
        assert!(before.pending.iter().any(|c| c.key == "FrameRateLimit" && c.from.as_deref() == Some("144.000000")));
        assert!(!before.pending.iter().any(|c| c.key == "bUseDynamicResolution"), "already off");
        assert!(!before.pending.iter().any(|c| c.key == "MeshQuality"), "absent section-only key is not invented");

        let changed = apply(ConfigGame::Fortnite, &opts, &path, &backups).unwrap();
        assert!(changed.len() > 10, "{changed:?}");
        let doc = Doc::ini(&std::fs::read(&path).unwrap());
        assert_eq!(doc.get(Some(FN_SECTION), "FrameRateLimit").as_deref(), Some("0.000000"));
        assert_eq!(doc.get(Some(FN_SECTION), "bUseVSync").as_deref(), Some("False"));
        assert_eq!(doc.get(Some(FN_SECTION), "bShowFPS").as_deref(), Some("True"));
        assert_eq!(doc.get(Some(FN_SECTION), "FullscreenMode").as_deref(), Some("0"));
        assert_eq!(doc.get(Some(FN_RHI), "PreferredFeatureLevel").as_deref(), Some("es31"));
        assert_eq!(doc.get(Some(FN_RHI), "PreferredRHI").as_deref(), Some("dx12"), "the API is left to Fortnite");
        assert_eq!(doc.get(Some(FN_SCALABILITY), "sg.ShadowQuality").as_deref(), Some("0"));
        assert_eq!(doc.get(Some(FN_SCALABILITY), "sg.EffectsQuality").as_deref(), Some("0"), "known scalability keys are added");
        assert!(String::from_utf8(std::fs::read(&path).unwrap()).unwrap().contains("\r\n"), "line endings kept");

        let after = status(ConfigGame::Fortnite, &opts, Some(&path), &backups, false);
        assert!(after.pending.is_empty(), "{:?}", after.pending);
        assert_eq!(after.settings[0].value, "Unlimited");
        assert_eq!(after.settings[1].value, "Performance");
        assert!(after.backup_at.is_some());

        // A second optimize with other options keeps the first copy (yours).
        let capped = ConfigOptions { fortnite: FortniteOptions { frame_limit: 540, ..Default::default() }, ..Default::default() };
        apply(ConfigGame::Fortnite, &capped, &path, &backups).unwrap();
        assert_eq!(Doc::ini(&std::fs::read(&path).unwrap()).get(Some(FN_SECTION), "FrameRateLimit").as_deref(), Some("540.000000"));
        restore(ConfigGame::Fortnite, &path, &backups).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), FORTNITE);
        assert!(restore(ConfigGame::Fortnite, &path, &backups).is_err(), "nothing left to restore");
    }

    #[test]
    fn missing_file_and_readonly() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("GameUserSettings.ini");
        let s = status(ConfigGame::Fortnite, &ConfigOptions::default(), Some(&path), dir.path(), false);
        assert!(!s.found && s.pending.is_empty());
        assert!(apply(ConfigGame::Fortnite, &ConfigOptions::default(), &path, dir.path()).unwrap_err().contains("Start it once"));

        std::fs::write(&path, FORTNITE).unwrap();
        let mut p = std::fs::metadata(&path).unwrap().permissions();
        p.set_readonly(true);
        std::fs::set_permissions(&path, p).unwrap();
        apply(ConfigGame::Fortnite, &ConfigOptions::default(), &path, &dir.path().join("b")).unwrap();
        assert!(std::fs::metadata(&path).unwrap().permissions().readonly(), "read-only mark kept");
        assert!(status(ConfigGame::Fortnite, &ConfigOptions::default(), Some(&path), dir.path(), false).pending.is_empty());
    }

    #[test]
    fn minecraft_follows_the_files_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("options.txt");
        std::fs::write(&path, "version:3953\nao:true\nmaxFps:120\nenableVsync:true\ngraphicsMode:1\nrenderClouds:\"true\"\nparticles:0\nrenderDistance:12\nentityShadows:true\n").unwrap();
        let opts = ConfigOptions::default();
        let s = status(ConfigGame::Minecraft, &opts, Some(&path), dir.path(), false);
        assert_eq!(s.settings[0].value, "120 FPS");
        assert!(!s.pending.iter().any(|c| c.key == "fancyGraphics" || c.key == "mipmapLevels"), "keys this version lacks stay out");
        apply(ConfigGame::Minecraft, &opts, &path, &dir.path().join("b")).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "version:3953\nao:false\nmaxFps:260\nenableVsync:false\ngraphicsMode:0\nrenderClouds:\"false\"\nparticles:2\nrenderDistance:12\nentityShadows:false\n");
        // Older files spell smooth lighting as a number.
        let mut old = Doc::colon(b"ao:2\n");
        apply_to(ConfigGame::Minecraft, &opts, &mut old);
        assert_eq!(old.get(None, "ao").as_deref(), Some("0"));
    }
}
