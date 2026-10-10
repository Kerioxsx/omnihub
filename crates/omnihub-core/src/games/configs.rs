//! A game's own settings, set for frames per second and low delay:
//! Fortnite's GameUserSettings.ini, Minecraft's options.txt, VALORANT's
//! GameUserSettings.ini (one per account), Counter-Strike 2's cs2_video.txt
//! (one per Steam account), Apex Legends' videoconfig.txt, Overwatch 2's
//! Settings_v0.ini and Roblox's GlobalBasicSettings. OmniHub writes the same
//! values the game's settings menu would, keeps a copy of each file from
//! before its first change, and can put it back.
//!
//! Only settings a game is known to read are added when missing (Fortnite's
//! standard Unreal Engine ones); everything else changes only when the game
//! already wrote it, so a game update that renames something is left alone
//! rather than guessed at. Games rewrite these files when they close, so
//! nothing is written while the game runs.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::ini::{Doc, ANY};
use super::GameKind;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ConfigGame {
    Fortnite,
    Minecraft,
    Valorant,
    Cs2,
    Apex,
    Overwatch,
    Roblox,
}

impl ConfigGame {
    pub fn all() -> [ConfigGame; 7] {
        use ConfigGame::*;
        [Fortnite, Valorant, Cs2, Apex, Overwatch, Roblox, Minecraft]
    }

    pub fn label(self) -> &'static str {
        match self {
            ConfigGame::Fortnite => "Fortnite",
            ConfigGame::Minecraft => "Minecraft",
            ConfigGame::Valorant => "VALORANT",
            ConfigGame::Cs2 => "Counter-Strike 2",
            ConfigGame::Apex => "Apex Legends",
            ConfigGame::Overwatch => "Overwatch 2",
            ConfigGame::Roblox => "Roblox",
        }
    }

    pub fn kind(self) -> GameKind {
        match self {
            ConfigGame::Fortnite => GameKind::Fortnite,
            ConfigGame::Minecraft => GameKind::Minecraft,
            ConfigGame::Valorant => GameKind::Valorant,
            ConfigGame::Cs2 => GameKind::Cs2,
            ConfigGame::Apex => GameKind::Apex,
            ConfigGame::Overwatch => GameKind::Overwatch,
            ConfigGame::Roblox => GameKind::Roblox,
        }
    }

    /// The program that rewrites the file when it closes.
    pub fn process(self) -> &'static str {
        match self {
            ConfigGame::Fortnite => "FortniteClient-Win64-Shipping.exe",
            ConfigGame::Minecraft => "javaw.exe",
            ConfigGame::Valorant => "VALORANT-Win64-Shipping.exe",
            ConfigGame::Cs2 => "cs2.exe",
            ConfigGame::Apex => "r5apex.exe",
            ConfigGame::Overwatch => "Overwatch.exe",
            ConfigGame::Roblox => super::roblox::PLAYER_EXE,
        }
    }

    /// What to know that OmniHub can't change for you.
    pub fn note(self) -> Option<&'static str> {
        Some(match self {
            ConfigGame::Cs2 => "CS2 keeps its frame limit in the console: type fps_max 0 for unlimited (or fps_max 540).",
            ConfigGame::Apex => "Apex's frame limit is a launch option: add +fps_max 0 in Steam (Properties → Launch options) or the EA app.",
            ConfigGame::Roblox => "Roblox's own menu goes up to 240 FPS; the rest of its speed comes from the Fast Flags below.",
            ConfigGame::Valorant => "Turn on NVIDIA Reflex (On + Boost) in VALORANT's video settings if you have it.",
            ConfigGame::Overwatch => "Turn on NVIDIA Reflex (Enabled + Boost) in Overwatch's video settings if you have it.",
            ConfigGame::Fortnite => "Turn on NVIDIA Reflex (On + Boost) in Fortnite's graphics settings if you have it.",
            ConfigGame::Minecraft => return None,
        })
    }

    pub fn slug(self) -> &'static str {
        match self {
            ConfigGame::Fortnite => "fortnite",
            ConfigGame::Minecraft => "minecraft",
            ConfigGame::Valorant => "valorant",
            ConfigGame::Cs2 => "cs2",
            ConfigGame::Apex => "apex",
            ConfigGame::Overwatch => "overwatch",
            ConfigGame::Roblox => "roblox",
        }
    }

    fn read(self, bytes: &[u8]) -> Doc {
        match self {
            ConfigGame::Fortnite | ConfigGame::Valorant | ConfigGame::Overwatch => Doc::ini(bytes),
            ConfigGame::Minecraft => Doc::colon(bytes),
            ConfigGame::Cs2 | ConfigGame::Apex => Doc::key_values(bytes),
            ConfigGame::Roblox => Doc::xml(bytes),
        }
    }

    /// The game's settings files for this Windows user (some keep one per
    /// account).
    pub fn locate(self) -> Vec<PathBuf> {
        let local = dirs::data_local_dir();
        let one = |p: Option<PathBuf>| p.filter(|p| p.is_file()).into_iter().collect::<Vec<_>>();
        match self {
            ConfigGame::Fortnite => one(local.map(|l| l.join(r"FortniteGame\Saved\Config\WindowsClient\GameUserSettings.ini"))),
            ConfigGame::Minecraft => one(dirs::config_dir().map(|c| c.join(".minecraft").join("options.txt"))),
            ConfigGame::Valorant => local.map(|l| find_named(&l.join(r"VALORANT\Saved\Config"), "GameUserSettings.ini", 3)).unwrap_or_default(),
            ConfigGame::Cs2 => super::library::steam_root()
                .map(|s| {
                    let mut out = Vec::new();
                    for e in std::fs::read_dir(s.join("userdata")).into_iter().flatten().flatten() {
                        let f = e.path().join(r"730\local\cfg\cs2_video.txt");
                        if f.is_file() {
                            out.push(f);
                        }
                    }
                    out
                })
                .unwrap_or_default(),
            ConfigGame::Apex => one(dirs::home_dir().map(|h| h.join(r"Saved Games\Respawn\Apex\local\videoconfig.txt"))),
            ConfigGame::Overwatch => one(dirs::document_dir().map(|d| d.join(r"Overwatch\Settings\Settings_v0.ini"))),
            ConfigGame::Roblox => one(local.map(|l| l.join(r"Roblox\GlobalBasicSettings_13.xml"))),
        }
    }
}

/// Files named `name` up to `depth` folders below `dir`.
fn find_named(dir: &Path, name: &str, depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), 0)];
    while let Some((d, n)) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                if n < depth {
                    stack.push((p, n + 1));
                }
            } else if e.file_name().to_string_lossy().eq_ignore_ascii_case(name) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
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
    /// Chunks the world updates around you; None keeps yours.
    pub simulation_distance: Option<u8>,
}

impl Default for MinecraftOptions {
    fn default() -> Self {
        MinecraftOptions { unlimited_fps: true, fast_graphics: true, minimal_particles: true, no_clouds: true, render_distance: None, simulation_distance: None }
    }
}

/// The switches for games without options of their own.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProOptions {
    /// No frame cap the file controls, VSync off.
    pub uncapped: bool,
    /// The lowest quality where it costs frames (shadows, effects, AO…).
    pub lowest_quality: bool,
    /// The game's low-latency options (NVIDIA Reflex where it is a setting).
    pub low_latency: bool,
}

impl Default for ProOptions {
    fn default() -> Self {
        ProOptions { uncapped: true, lowest_quality: true, low_latency: true }
    }
}

/// The options for every supported game.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ConfigOptions {
    pub fortnite: FortniteOptions,
    pub minecraft: MinecraftOptions,
    pub valorant: ProOptions,
    pub cs2: ProOptions,
    pub apex: ProOptions,
    pub overwatch: ProOptions,
    pub roblox: ProOptions,
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
    pub label: String,
    /// The settings files (several for games with one per account).
    pub paths: Vec<String>,
    /// A settings file exists (the game has run on this PC).
    pub found: bool,
    pub running: bool,
    pub settings: Vec<Setting>,
    /// What Optimize would change; empty when already optimized.
    pub pending: Vec<Change>,
    /// When the copy of your own settings was taken (Unix seconds).
    pub backup_at: Option<i64>,
    pub note: Option<String>,
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

/// Unreal Engine's scalability groups, lowest first.
const UE_GROUPS: &[(&str, &str)] = &[
    ("sg.AntiAliasingQuality", "Anti-aliasing"),
    ("sg.ShadowQuality", "Shadows"),
    ("sg.GlobalIlluminationQuality", "Global illumination"),
    ("sg.ReflectionQuality", "Reflections"),
    ("sg.PostProcessQuality", "Post-processing"),
    ("sg.TextureQuality", "Textures"),
    ("sg.EffectsQuality", "Effects"),
    ("sg.FoliageQuality", "Foliage"),
    ("sg.ShadingQuality", "Shading"),
];

fn fortnite_edits(o: &FortniteOptions) -> Vec<Edit> {
    let s = Some(FN_SECTION);
    let mut e = vec![
        edit(s, "FrameRateLimit", format!("{}.000000", o.frame_limit), true, if o.frame_limit == 0 { "Frame rate limit: unlimited".to_string() } else { format!("Frame rate limit: {} FPS", o.frame_limit) }),
        edit(s, "bUseVSync", "False", true, "VSync off"),
        edit(s, "bUseDynamicResolution", "False", false, "Dynamic resolution off"),
        edit(s, "bMotionBlur", "False", false, "Motion blur off"),
        edit(s, "bShowFPS", if o.show_fps { "True" } else { "False" }, true, if o.show_fps { "FPS counter on" } else { "FPS counter off" }),
        edit(s, "bDisableMouseAcceleration", "True", false, "Mouse acceleration off"),
        // Spreads rendering over more cores (Fortnite's "Multithreaded Rendering").
        edit(s, "bAllowMultithreadedRendering", "True", false, "Multithreaded rendering on"),
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
        for (key, what) in UE_GROUPS {
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
        // Chunk updates on a separate thread rather than blocking a frame.
        e.push(edit(None, "prioritizeChunkUpdates", "0", false, "Chunk builder: threaded"));
    }
    if o.minimal_particles {
        e.push(edit(None, "particles", "2", false, "Particles: minimal"));
    }
    if o.no_clouds {
        e.push(edit(None, "renderClouds", "false", false, "Clouds off"));
    }
    if let Some(d) = o.render_distance {
        e.push(edit(None, "renderDistance", d.clamp(2, 32).to_string(), false, format!("Render distance: {} chunks", d.clamp(2, 32))));
    }
    if let Some(d) = o.simulation_distance {
        e.push(edit(None, "simulationDistance", d.clamp(5, 32).to_string(), false, format!("Simulation distance: {} chunks", d.clamp(5, 32))));
    }
    e
}

fn valorant_edits(o: &ProOptions) -> Vec<Edit> {
    let a = Some(ANY);
    let mut e = Vec::new();
    if o.uncapped {
        e.push(edit(a, "bUseVSync", "False", false, "VSync off"));
        e.push(edit(a, "FrameRateLimit", "0.000000", false, "Frame rate limit: unlimited"));
        e.push(edit(a, "bUseDynamicResolution", "False", false, "Dynamic resolution off"));
    }
    if o.lowest_quality {
        for (key, what) in UE_GROUPS {
            e.push(edit(a, key, "0", false, format!("{what}: low")));
        }
    }
    e
}

fn cs2_edits(o: &ProOptions) -> Vec<Edit> {
    let mut e = Vec::new();
    if o.uncapped {
        e.push(edit(None, "setting.mat_vsync", "0", false, "VSync off"));
    }
    if o.low_latency {
        // NVIDIA Reflex: 0 off, 1 on, 2 on + boost.
        e.push(edit(None, "setting.r_low_latency", "1", false, "NVIDIA Reflex: on"));
    }
    if o.lowest_quality {
        for (key, label) in [
            ("setting.shaderquality", "Shader detail: low"),
            ("setting.videocfg_shadow_quality", "Shadows: low"),
            ("setting.msaa_samples", "Multisampling anti-aliasing: off"),
            ("setting.r_csgo_cmaa_enable", "CMAA2 anti-aliasing: off"),
            ("setting.videocfg_texture_detail", "Textures: low"),
            ("setting.videocfg_particle_detail", "Particles: low"),
            ("setting.videocfg_ao_detail", "Ambient occlusion: off"),
        ] {
            e.push(edit(None, key, "0", false, label));
        }
    }
    e
}

fn apex_edits(o: &ProOptions) -> Vec<Edit> {
    let mut e = Vec::new();
    if o.uncapped {
        e.push(edit(None, "setting.mat_vsync_mode", "0", false, "VSync off"));
        e.push(edit(None, "setting.dvs_enable", "0", false, "Adaptive resolution off"));
    }
    if o.lowest_quality {
        for (key, label) in [
            ("setting.mat_antialias_mode", "Anti-aliasing: off"),
            ("setting.csm_coverage", "Sun shadow coverage: low"),
            ("setting.shadow_enable", "Spot shadows: off"),
            ("setting.ssao_enabled", "Ambient occlusion: off"),
            ("setting.volumetric_lighting", "Volumetric lighting: off"),
            ("setting.particle_cpu_level", "Effects: low"),
            ("setting.mat_depthfeather_enable", "Depth feathering: off"),
        ] {
            e.push(edit(None, key, "0", false, label));
        }
    }
    e
}

fn overwatch_edits(o: &ProOptions) -> Vec<Edit> {
    let a = Some(ANY);
    let mut e = Vec::new();
    if o.uncapped {
        // 600 is Overwatch's highest frame limit.
        e.push(edit(a, "FrameRateCap", "600", false, "Frame rate limit: 600"));
        e.push(edit(a, "VerticalSyncEnabled", "0", false, "VSync off"));
        e.push(edit(a, "TripleBufferingEnabled", "0", false, "Triple buffering off"));
    }
    if o.low_latency {
        e.push(edit(a, "ReduceBuffering", "1", false, "Reduce buffering: on"));
    }
    e
}

fn roblox_edits(o: &ProOptions) -> Vec<Edit> {
    let mut e = Vec::new();
    if o.uncapped {
        e.push(edit(None, "FramerateCap", "240", false, "Maximum frame rate: 240"));
    }
    e
}

fn edits(game: ConfigGame, opts: &ConfigOptions, doc: &Doc) -> Vec<Edit> {
    match game {
        ConfigGame::Fortnite => fortnite_edits(&opts.fortnite),
        ConfigGame::Minecraft => minecraft_edits(&opts.minecraft, doc),
        ConfigGame::Valorant => valorant_edits(&opts.valorant),
        ConfigGame::Cs2 => cs2_edits(&opts.cs2),
        ConfigGame::Apex => apex_edits(&opts.apex),
        ConfigGame::Overwatch => overwatch_edits(&opts.overwatch),
        ConfigGame::Roblox => roblox_edits(&opts.roblox),
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

fn on_off(on: bool) -> String {
    if on { "On" } else { "Off" }.into()
}

/// The settings a player cares about, as the file has them now.
pub fn settings(game: ConfigGame, doc: &Doc) -> Vec<Setting> {
    let row = |label: &str, value: String, good: bool| Setting { label: label.into(), value, good };
    let num = |section: Option<&str>, key: &str| doc.get(section, key).and_then(|v| v.parse::<f64>().ok());
    let level = |v: Option<f64>| -> (String, bool) {
        match v {
            Some(x) if x <= 0.0 => ("Low".into(), true),
            Some(x) if x <= 1.0 => ("Medium".into(), false),
            Some(_) => ("High".into(), false),
            None => ("Game's choice".into(), false),
        }
    };
    let mut rows = match game {
        ConfigGame::Fortnite => {
            let s = Some(FN_SECTION);
            let limit = num(s, "FrameRateLimit");
            let fl = doc.get(Some(FN_RHI), "PreferredFeatureLevel").unwrap_or_default();
            let rhi = doc.get(Some(FN_RHI), "PreferredRHI").unwrap_or_default();
            let perf = fl.eq_ignore_ascii_case("es31");
            let on = |k: &str| doc.get(s, k).is_some_and(|v| v.eq_ignore_ascii_case("true"));
            let shadows = doc.get(Some(FN_SCALABILITY), "sg.ShadowQuality").and_then(|v| v.parse::<i32>().ok());
            let mode = doc.get(s, "FullscreenMode").and_then(|v| v.parse::<i32>().ok());
            vec![
                row("Frame rate limit", match limit { Some(l) if l <= 0.0 => "Unlimited".into(), Some(l) => format!("{l:.0} FPS"), None => "Not set".into() }, limit.is_some_and(|l| l <= 0.0 || l >= 240.0)),
                row("Rendering mode", if perf { "Performance".into() } else if rhi.eq_ignore_ascii_case("dx11") { "DirectX 11".into() } else if rhi.eq_ignore_ascii_case("dx12") { "DirectX 12".into() } else { "Game's choice".into() }, perf),
                row("VSync", on_off(on("bUseVSync")), !on("bUseVSync")),
                row("Shadows", match shadows { Some(0) => "Off".into(), Some(n) => ["Off", "Medium", "High", "Epic", "Cinematic"].get(n as usize).copied().unwrap_or("High").into(), None => "Game's choice".into() }, shadows == Some(0)),
                row("Window mode", match mode { Some(0) => "Fullscreen".into(), Some(1) => "Windowed fullscreen".into(), Some(2) => "Windowed".into(), _ => "Game's choice".into() }, mode == Some(0)),
                row("FPS counter", on_off(on("bShowFPS")), on("bShowFPS")),
            ]
        }
        ConfigGame::Minecraft => {
            let fps = doc.get(None, "maxFps").and_then(|v| v.parse::<u32>().ok());
            let vsync = doc.get(None, "enableVsync").is_some_and(|v| v == "true");
            let graphics = doc.get(None, "graphicsMode").and_then(|v| v.parse::<u32>().ok());
            let distance = doc.get(None, "renderDistance").unwrap_or_else(|| "?".into());
            vec![
                row("Max frame rate", match fps { Some(f) if f >= 260 => "Unlimited".into(), Some(f) => format!("{f} FPS"), None => "Not set".into() }, fps.is_some_and(|f| f >= 260)),
                row("VSync", on_off(vsync), !vsync),
                row("Graphics", match graphics { Some(0) => "Fast".into(), Some(1) => "Fancy".into(), Some(2) => "Fabulous".into(), _ => "Game's choice".into() }, graphics == Some(0)),
                row("Render distance", format!("{distance} chunks"), true),
            ]
        }
        ConfigGame::Valorant => {
            let vsync = doc.get(Some(ANY), "bUseVSync").map(|v| v.eq_ignore_ascii_case("true"));
            let (shadows, sg) = level(num(Some(ANY), "sg.ShadowQuality"));
            let (fx, fg) = level(num(Some(ANY), "sg.EffectsQuality"));
            vec![row("VSync", vsync.map(on_off).unwrap_or_else(|| "Game's choice".into()), vsync == Some(false)), row("Shadows", shadows, sg), row("Effects", fx, fg)]
        }
        ConfigGame::Cs2 => {
            let vsync = num(None, "setting.mat_vsync");
            let reflex = num(None, "setting.r_low_latency");
            let (shaders, sg) = level(num(None, "setting.shaderquality"));
            let (shadows, shg) = level(num(None, "setting.videocfg_shadow_quality"));
            vec![
                row("VSync", vsync.map(|v| on_off(v > 0.0)).unwrap_or_else(|| "Game's choice".into()), vsync == Some(0.0)),
                row("NVIDIA Reflex", match reflex { Some(r) if r >= 2.0 => "On + Boost".into(), Some(r) if r >= 1.0 => "On".into(), Some(_) => "Off".into(), None => "Game's choice".into() }, reflex.is_some_and(|r| r >= 1.0)),
                row("Shader detail", shaders, sg),
                row("Shadows", shadows, shg),
            ]
        }
        ConfigGame::Apex => {
            let vsync = num(None, "setting.mat_vsync_mode");
            let aa = num(None, "setting.mat_antialias_mode");
            let ao = num(None, "setting.ssao_enabled");
            let vol = num(None, "setting.volumetric_lighting");
            let flag = |v: Option<f64>| v.map(|x| on_off(x > 0.0)).unwrap_or_else(|| "Game's choice".into());
            vec![row("VSync", flag(vsync), vsync == Some(0.0)), row("Anti-aliasing", flag(aa), aa == Some(0.0)), row("Ambient occlusion", flag(ao), ao == Some(0.0)), row("Volumetric lighting", flag(vol), vol == Some(0.0))]
        }
        ConfigGame::Overwatch => {
            let cap = num(Some(ANY), "FrameRateCap");
            let vsync = num(Some(ANY), "VerticalSyncEnabled");
            let reduce = num(Some(ANY), "ReduceBuffering");
            vec![
                row("Frame rate limit", cap.map(|c| format!("{c:.0} FPS")).unwrap_or_else(|| "Game's choice".into()), cap.is_some_and(|c| c >= 400.0)),
                row("VSync", vsync.map(|v| on_off(v > 0.0)).unwrap_or_else(|| "Game's choice".into()), vsync == Some(0.0)),
                row("Reduce buffering", reduce.map(|v| on_off(v > 0.0)).unwrap_or_else(|| "Game's choice".into()), reduce == Some(1.0)),
            ]
        }
        ConfigGame::Roblox => {
            let cap = num(None, "FramerateCap");
            vec![row("Maximum frame rate", match cap { Some(c) if c <= 0.0 => "Default".into(), Some(c) => format!("{c:.0} FPS"), None => "Game's choice".into() }, cap.is_some_and(|c| c >= 240.0))]
        }
    };
    rows.retain(|r| !r.label.is_empty());
    rows
}

/// Where OmniHub keeps the copy of your own settings for one file.
fn backup_path(game: ConfigGame, path: &Path, backups: &Path) -> PathBuf {
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|| "txt".into());
    // Games with one file per account get one copy per file.
    let id = path.to_string_lossy().bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3));
    backups.join(format!("{}-{:08x}-yours.{ext}", game.slug(), id as u32))
}

fn mtime(p: &Path) -> Option<i64> {
    let t = std::fs::metadata(p).ok()?.modified().ok()?;
    Some(t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64)
}

pub fn status(game: ConfigGame, opts: &ConfigOptions, paths: &[PathBuf], backups: &Path, running: bool) -> ConfigStatus {
    let docs: Vec<Doc> = paths.iter().filter_map(|p| std::fs::read(p).ok()).map(|b| game.read(&b)).collect();
    let mut pending_all: Vec<Change> = Vec::new();
    for d in &docs {
        for c in pending(game, opts, d) {
            if !pending_all.iter().any(|x| x.key == c.key) {
                pending_all.push(c);
            }
        }
    }
    ConfigStatus {
        game,
        label: game.label().into(),
        paths: paths.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        found: !docs.is_empty(),
        running,
        settings: docs.first().map(|d| settings(game, d)).unwrap_or_default(),
        pending: pending_all,
        backup_at: paths.iter().filter_map(|p| mtime(&backup_path(game, p, backups))).min(),
        note: game.note().map(str::to_string),
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

/// Apply `opts` to each of the game's files, copying your own settings away
/// first (only the first time, so Restore always brings back yours).
pub fn apply(game: ConfigGame, opts: &ConfigOptions, paths: &[PathBuf], backups: &Path) -> Result<Vec<Change>, String> {
    if paths.is_empty() {
        return Err(format!("{} has no settings file yet. Start it once, close it, then try again.", game.label()));
    }
    let mut all: Vec<Change> = Vec::new();
    for path in paths {
        let bytes = std::fs::read(path).map_err(|_| format!("{} has no settings file yet. Start it once, close it, then try again.", game.label()))?;
        let mut doc = game.read(&bytes);
        let changes = pending(game, opts, &doc);
        if changes.is_empty() {
            continue;
        }
        std::fs::create_dir_all(backups).map_err(|e| e.to_string())?;
        let backup = backup_path(game, path, backups);
        if !backup.exists() {
            std::fs::write(&backup, &bytes).map_err(|e| format!("Could not keep a copy of your settings: {e}"))?;
        }
        apply_to(game, opts, &mut doc);
        write_keeping_readonly(path, &doc.to_bytes()).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
        for c in changes {
            if !all.iter().any(|x| x.key == c.key) {
                all.push(c);
            }
        }
    }
    Ok(all)
}

/// Put your own settings back (the copies from before OmniHub's first change).
pub fn restore(game: ConfigGame, paths: &[PathBuf], backups: &Path) -> Result<(), String> {
    let mut restored = 0;
    for path in paths {
        let backup = backup_path(game, path, backups);
        let Ok(bytes) = std::fs::read(&backup) else { continue };
        write_keeping_readonly(path, &bytes).or_else(|_| std::fs::write(path, &bytes)).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
        let _ = std::fs::remove_file(&backup);
        restored += 1;
    }
    if restored == 0 {
        return Err("There is no copy of your own settings to restore.".into());
    }
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
        let paths = vec![path.clone()];
        let backups = dir.path().join("backups");
        std::fs::write(&path, FORTNITE).unwrap();
        let opts = ConfigOptions::default();

        let before = status(ConfigGame::Fortnite, &opts, &paths, &backups, false);
        assert!(before.found);
        assert_eq!(before.settings[0].value, "144 FPS");
        assert_eq!(before.settings[1].value, "DirectX 12");
        assert!(before.pending.iter().any(|c| c.key == "FrameRateLimit" && c.from.as_deref() == Some("144.000000")));
        assert!(!before.pending.iter().any(|c| c.key == "bUseDynamicResolution"), "already off");
        assert!(!before.pending.iter().any(|c| c.key == "MeshQuality"), "absent section-only key is not invented");

        let changed = apply(ConfigGame::Fortnite, &opts, &paths, &backups).unwrap();
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

        let after = status(ConfigGame::Fortnite, &opts, &paths, &backups, false);
        assert!(after.pending.is_empty(), "{:?}", after.pending);
        assert_eq!(after.settings[0].value, "Unlimited");
        assert_eq!(after.settings[1].value, "Performance");
        assert!(after.backup_at.is_some());

        // A second optimize with other options keeps the first copy (yours).
        let capped = ConfigOptions { fortnite: FortniteOptions { frame_limit: 540, ..Default::default() }, ..Default::default() };
        apply(ConfigGame::Fortnite, &capped, &paths, &backups).unwrap();
        assert_eq!(Doc::ini(&std::fs::read(&path).unwrap()).get(Some(FN_SECTION), "FrameRateLimit").as_deref(), Some("540.000000"));
        restore(ConfigGame::Fortnite, &paths, &backups).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), FORTNITE);
        assert!(restore(ConfigGame::Fortnite, &paths, &backups).is_err(), "nothing left to restore");
    }

    #[test]
    fn missing_file_and_readonly() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("GameUserSettings.ini");
        let s = status(ConfigGame::Fortnite, &ConfigOptions::default(), &[], dir.path(), false);
        assert!(!s.found && s.pending.is_empty());
        assert!(apply(ConfigGame::Fortnite, &ConfigOptions::default(), &[], dir.path()).unwrap_err().contains("Start it once"));

        std::fs::write(&path, FORTNITE).unwrap();
        let mut p = std::fs::metadata(&path).unwrap().permissions();
        p.set_readonly(true);
        std::fs::set_permissions(&path, p).unwrap();
        apply(ConfigGame::Fortnite, &ConfigOptions::default(), std::slice::from_ref(&path), &dir.path().join("b")).unwrap();
        assert!(std::fs::metadata(&path).unwrap().permissions().readonly(), "read-only mark kept");
        assert!(status(ConfigGame::Fortnite, &ConfigOptions::default(), &[path], dir.path(), false).pending.is_empty());
    }

    #[test]
    fn minecraft_follows_the_files_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("options.txt");
        std::fs::write(&path, "version:3953\nao:true\nmaxFps:120\nenableVsync:true\ngraphicsMode:1\nrenderClouds:\"true\"\nparticles:0\nrenderDistance:12\nentityShadows:true\n").unwrap();
        let opts = ConfigOptions::default();
        let s = status(ConfigGame::Minecraft, &opts, std::slice::from_ref(&path), dir.path(), false);
        assert_eq!(s.settings[0].value, "120 FPS");
        assert!(!s.pending.iter().any(|c| c.key == "fancyGraphics" || c.key == "mipmapLevels"), "keys this version lacks stay out");
        apply(ConfigGame::Minecraft, &opts, std::slice::from_ref(&path), &dir.path().join("b")).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "version:3953\nao:false\nmaxFps:260\nenableVsync:false\ngraphicsMode:0\nrenderClouds:\"false\"\nparticles:2\nrenderDistance:12\nentityShadows:false\n");
        // Older files spell smooth lighting as a number.
        let mut old = Doc::colon(b"ao:2\n");
        apply_to(ConfigGame::Minecraft, &opts, &mut old);
        assert_eq!(old.get(None, "ao").as_deref(), Some("0"));
    }

    #[test]
    fn counter_strike_for_every_steam_account() {
        let dir = tempfile::tempdir().unwrap();
        let src = "\"video.cfg\"\n{\n\t\"Version\"\t\t\"14\"\n\t\"setting.mat_vsync\"\t\t\"1\"\n\t\"setting.r_low_latency\"\t\t\"0\"\n\t\"setting.shaderquality\"\t\t\"1\"\n\t\"setting.videocfg_shadow_quality\"\t\t\"2\"\n}\n";
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, src).unwrap();
        std::fs::write(&b, src.replace("\"setting.mat_vsync\"\t\t\"1\"", "\"setting.mat_vsync\"\t\t\"0\"")).unwrap();
        let paths = vec![a.clone(), b.clone()];
        let backups = dir.path().join("bk");
        let opts = ConfigOptions::default();
        let s = status(ConfigGame::Cs2, &opts, &paths, &backups, false);
        assert_eq!(s.settings[0].value, "On");
        assert_eq!(s.pending.len(), 4, "{:?}", s.pending);
        assert!(s.note.as_deref().is_some_and(|n| n.contains("fps_max")));
        apply(ConfigGame::Cs2, &opts, &paths, &backups).unwrap();
        for p in [&a, &b] {
            let d = Doc::key_values(&std::fs::read(p).unwrap());
            assert_eq!(d.get(None, "setting.mat_vsync").as_deref(), Some("0"));
            assert_eq!(d.get(None, "setting.r_low_latency").as_deref(), Some("1"));
            assert_eq!(d.get(None, "setting.videocfg_shadow_quality").as_deref(), Some("0"));
            assert!(d.get(None, "setting.msaa_samples").is_none(), "nothing added");
        }
        assert!(status(ConfigGame::Cs2, &opts, &paths, &backups, false).pending.is_empty());
        restore(ConfigGame::Cs2, &paths, &backups).unwrap();
        assert_eq!(std::fs::read_to_string(&a).unwrap(), src);
    }

    #[test]
    fn apex_overwatch_valorant_roblox() {
        let dir = tempfile::tempdir().unwrap();
        let opts = ConfigOptions::default();
        let bk = dir.path().join("bk");
        let file = |name: &str, body: &str| {
            let p = dir.path().join(name);
            std::fs::write(&p, body).unwrap();
            vec![p]
        };
        let apex = file("videoconfig.txt", "\"VideoConfig\"\n{\n\t\"setting.mat_vsync_mode\"\t\t\"1\"\n\t\"setting.ssao_enabled\"\t\t\"1\"\n\t\"setting.volumetric_lighting\"\t\t\"1\"\n}\n");
        apply(ConfigGame::Apex, &opts, &apex, &bk).unwrap();
        let d = Doc::key_values(&std::fs::read(&apex[0]).unwrap());
        assert_eq!((d.get(None, "setting.mat_vsync_mode").as_deref(), d.get(None, "setting.ssao_enabled").as_deref()), (Some("0"), Some("0")));

        let ow = file("Settings_v0.ini", "[Render.13]\nFrameRateCap = \"300.000000\"\nVerticalSyncEnabled = \"1\"\nReduceBuffering = \"0\"\n");
        assert_eq!(status(ConfigGame::Overwatch, &opts, &ow, &bk, false).settings[0].value, "300 FPS");
        apply(ConfigGame::Overwatch, &opts, &ow, &bk).unwrap();
        assert_eq!(std::fs::read_to_string(&ow[0]).unwrap(), "[Render.13]\nFrameRateCap = \"600\"\nVerticalSyncEnabled = \"0\"\nReduceBuffering = \"1\"\n");

        let val = file("GameUserSettings.ini", "[/Script/ShooterGame.ShooterGameUserSettings]\nbUseVSync=True\n[ScalabilityGroups]\nsg.ShadowQuality=3\n");
        apply(ConfigGame::Valorant, &opts, &val, &bk).unwrap();
        assert_eq!(std::fs::read_to_string(&val[0]).unwrap(), "[/Script/ShooterGame.ShooterGameUserSettings]\nbUseVSync=False\n[ScalabilityGroups]\nsg.ShadowQuality=0\n", "only keys VALORANT wrote");

        let rb = file("GlobalBasicSettings_13.xml", "<roblox>\n<Properties>\n\t<int name=\"FramerateCap\">60</int>\n</Properties>\n</roblox>\n");
        apply(ConfigGame::Roblox, &opts, &rb, &bk).unwrap();
        assert!(std::fs::read_to_string(&rb[0]).unwrap().contains("<int name=\"FramerateCap\">240</int>"));
        assert_eq!(status(ConfigGame::Roblox, &opts, &rb, &bk, false).settings[0].value, "240 FPS");
    }

    #[test]
    fn finds_files_per_account() {
        let dir = tempfile::tempdir().unwrap();
        for acct in ["one-eu", "two-na"] {
            let p = dir.path().join(acct).join("Windows");
            std::fs::create_dir_all(&p).unwrap();
            std::fs::write(p.join("GameUserSettings.ini"), "").unwrap();
        }
        assert_eq!(find_named(dir.path(), "GameUserSettings.ini", 3).len(), 2);
    }
}
