//! Roblox: find the installed player, write Fast Flags (graphics options
//! Roblox reads from `ClientSettings\ClientAppSettings.json` at start), and
//! launch it.
//!
//! Since 2025 Roblox only honours flags on its allowlist; the presets use
//! only those, and custom flags outside it are reported as ignored.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Flags Roblox still reads from the local file.
pub const ALLOWLIST: &[&str] = &[
    "DFIntCSGLevelOfDetailSwitchingDistance",
    "DFIntCSGLevelOfDetailSwitchingDistanceL12",
    "DFIntCSGLevelOfDetailSwitchingDistanceL23",
    "DFIntCSGLevelOfDetailSwitchingDistanceL34",
    "FFlagHandleAltEnterFullscreenManually",
    "DFFlagTextureQualityOverrideEnabled",
    "DFIntTextureQualityOverride",
    "FIntDebugForceMSAASamples",
    "DFFlagDisableDPIScale",
    "FFlagDebugGraphicsPreferD3D11",
    "FFlagDebugGraphicsPreferVulkan",
    "FFlagDebugGraphicsPreferOpenGL",
    "FFlagDebugSkyGray",
    "DFFlagDebugPauseVoxelizer",
    "DFIntDebugFRMQualityLevelOverride",
    "FIntFRMMaxGrassDistance",
    "FIntFRMMinGrassDistance",
    "FIntGrassMovementReducedMotionFactor",
];

pub const PLAYER_EXE: &str = "RobloxPlayerBeta.exe";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Preset {
    /// Lowest settings everywhere: the most frames per second.
    MaxFps,
    #[default]
    Balanced,
    Quality,
    /// Whatever the individual options say.
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Renderer {
    #[default]
    Auto,
    D3d11,
    Vulkan,
    OpenGl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RobloxFlags {
    /// Write the flags before each launch.
    pub enabled: bool,
    pub preset: Preset,
    pub renderer: Renderer,
    /// Anti-aliasing samples (0 = off); None leaves Roblox's choice.
    pub msaa: Option<u8>,
    /// 0 (lowest) – 3; None leaves Roblox's choice.
    pub texture_quality: Option<u8>,
    pub no_grass: bool,
    pub gray_sky: bool,
    /// Simpler models in the distance.
    pub low_detail_distance: bool,
    /// Forces Roblox's graphics level, 1–21.
    pub quality_level: Option<u8>,
    /// Alt+Enter switches to exclusive fullscreen.
    pub exclusive_fullscreen: bool,
    /// Render at full resolution on scaled displays (sharper, slower).
    pub ignore_display_scaling: bool,
    /// Extra flags, name → value.
    pub custom: Map<String, Value>,
}

impl Default for RobloxFlags {
    fn default() -> Self {
        RobloxFlags {
            enabled: true,
            preset: Preset::Balanced,
            renderer: Renderer::Auto,
            msaa: None,
            texture_quality: None,
            no_grass: false,
            gray_sky: false,
            low_detail_distance: false,
            quality_level: None,
            exclusive_fullscreen: false,
            ignore_display_scaling: false,
            custom: Map::new(),
        }
    }
}

impl RobloxFlags {
    /// The options a preset stands for (Custom keeps the current ones).
    pub fn with_preset(&self, preset: Preset) -> RobloxFlags {
        let base = RobloxFlags { enabled: self.enabled, preset, custom: self.custom.clone(), ..Default::default() };
        match preset {
            Preset::MaxFps => RobloxFlags { renderer: Renderer::D3d11, msaa: Some(0), texture_quality: Some(0), no_grass: true, low_detail_distance: true, quality_level: Some(1), exclusive_fullscreen: true, ..base },
            Preset::Balanced => RobloxFlags { renderer: Renderer::D3d11, no_grass: true, exclusive_fullscreen: true, ..base },
            Preset::Quality => RobloxFlags { msaa: Some(4), texture_quality: Some(3), ..base },
            Preset::Custom => RobloxFlags { preset, ..self.clone() },
        }
    }

    /// The flags to write, as Roblox spells them (string values, the way
    /// Roblox's own tools write them).
    pub fn to_flags(&self) -> Map<String, Value> {
        let f = if self.preset == Preset::Custom { self.clone() } else { self.with_preset(self.preset) };
        let mut m = Map::new();
        let mut set = |k: &str, v: String| {
            m.insert(k.to_string(), Value::String(v));
        };
        match f.renderer {
            Renderer::Auto => {}
            Renderer::D3d11 => set("FFlagDebugGraphicsPreferD3D11", "True".into()),
            Renderer::Vulkan => set("FFlagDebugGraphicsPreferVulkan", "True".into()),
            Renderer::OpenGl => set("FFlagDebugGraphicsPreferOpenGL", "True".into()),
        }
        if let Some(s) = f.msaa {
            set("FIntDebugForceMSAASamples", s.min(8).to_string());
        }
        if let Some(q) = f.texture_quality {
            set("DFFlagTextureQualityOverrideEnabled", "True".into());
            set("DFIntTextureQualityOverride", q.min(3).to_string());
        }
        if f.no_grass {
            set("FIntFRMMinGrassDistance", "0".into());
            set("FIntFRMMaxGrassDistance", "0".into());
        }
        if f.gray_sky {
            set("FFlagDebugSkyGray", "True".into());
        }
        if f.low_detail_distance {
            for k in ["DFIntCSGLevelOfDetailSwitchingDistance", "DFIntCSGLevelOfDetailSwitchingDistanceL12", "DFIntCSGLevelOfDetailSwitchingDistanceL23", "DFIntCSGLevelOfDetailSwitchingDistanceL34"] {
                set(k, "0".into());
            }
        }
        if let Some(l) = f.quality_level {
            set("DFIntDebugFRMQualityLevelOverride", l.clamp(1, 21).to_string());
        }
        if f.exclusive_fullscreen {
            set("FFlagHandleAltEnterFullscreenManually", "False".into());
        }
        if f.ignore_display_scaling {
            set("DFFlagDisableDPIScale", "True".into());
        }
        for (k, v) in &self.custom {
            m.insert(k.clone(), v.clone());
        }
        m
    }
}

/// Custom flag names Roblox will ignore.
pub fn ignored(flags: &Map<String, Value>) -> Vec<String> {
    flags.keys().filter(|k| !ALLOWLIST.contains(&k.as_str())).cloned().collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RobloxInstall {
    pub found: bool,
    /// Newest RobloxPlayerBeta.exe.
    pub player: Option<String>,
    pub version: Option<String>,
    /// Every version folder flags are written to.
    pub version_dirs: Vec<String>,
    /// "Bloxstrap" / "Fishstrap" when one manages Roblox.
    pub bootstrapper: Option<String>,
    /// Whether Roblox is running (flags apply from its next start).
    pub running: bool,
}

/// Where Roblox lives: the per-user install, an all-users install, and the
/// folders of the popular bootstrappers.
pub fn roots() -> Vec<(PathBuf, Option<&'static str>)> {
    let mut v = Vec::new();
    if let Some(local) = dirs::data_local_dir() {
        v.push((local.join("Roblox"), None));
        v.push((local.join("Bloxstrap"), Some("Bloxstrap")));
        v.push((local.join("Fishstrap"), Some("Fishstrap")));
    }
    for var in ["ProgramFiles(x86)", "ProgramFiles"] {
        if let Some(p) = std::env::var_os(var) {
            v.push((PathBuf::from(p).join("Roblox"), None));
        }
    }
    v
}

pub fn detect_in(roots: &[(PathBuf, Option<&'static str>)]) -> RobloxInstall {
    let mut dirs: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    let mut bootstrapper = None;
    for (root, boot) in roots {
        let Ok(rd) = std::fs::read_dir(root.join("Versions")) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.join(PLAYER_EXE).is_file() {
                let t = std::fs::metadata(p.join(PLAYER_EXE)).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                dirs.push((t, p));
                if boot.is_some() {
                    bootstrapper = *boot;
                }
            }
        }
    }
    dirs.sort_by_key(|d| std::cmp::Reverse(d.0));
    let newest = dirs.first().map(|(_, p)| p.clone());
    RobloxInstall {
        found: newest.is_some(),
        player: newest.as_ref().map(|p| p.join(PLAYER_EXE).to_string_lossy().into_owned()),
        version: newest.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()),
        version_dirs: dirs.iter().map(|(_, p)| p.to_string_lossy().into_owned()).collect(),
        bootstrapper: bootstrapper.map(str::to_string),
        running: false,
    }
}

pub fn detect() -> RobloxInstall {
    detect_in(&roots())
}

/// Write `flags` into every version folder (and a bootstrapper's
/// Modifications folder, which it copies over at launch). Keys OmniHub
/// wrote before (`managed`) are replaced; anything else already in the
/// file is kept. Returns the files written.
pub fn write_flags(roots: &[(PathBuf, Option<&'static str>)], flags: &Map<String, Value>, managed: &[String]) -> std::io::Result<Vec<PathBuf>> {
    let install = detect_in(roots);
    let mut files: Vec<PathBuf> = install.version_dirs.iter().map(|d| Path::new(d).join("ClientSettings").join("ClientAppSettings.json")).collect();
    for (root, boot) in roots {
        if boot.is_some() && root.join("Versions").is_dir() {
            files.push(root.join("Modifications").join("ClientSettings").join("ClientAppSettings.json"));
        }
    }
    if files.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Roblox is not installed (or has never been started) on this PC."));
    }
    for f in &files {
        let mut current: Map<String, Value> = std::fs::read(f).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        for k in managed {
            current.remove(k);
        }
        for (k, v) in flags {
            current.insert(k.clone(), v.clone());
        }
        if let Some(dir) = f.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = f.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&current).unwrap_or_default())?;
        std::fs::rename(&tmp, f)?;
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_use_allowed_flags_only() {
        for p in [Preset::MaxFps, Preset::Balanced, Preset::Quality] {
            let f = RobloxFlags::default().with_preset(p).to_flags();
            assert!(ignored(&f).is_empty(), "{p:?}: {:?}", ignored(&f));
        }
        let max = RobloxFlags { preset: Preset::MaxFps, ..Default::default() }.to_flags();
        assert_eq!(max["FIntDebugForceMSAASamples"], "0");
        assert_eq!(max["DFIntTextureQualityOverride"], "0");
        assert_eq!(max["FFlagDebugGraphicsPreferD3D11"], "True");
        assert_eq!(max["DFIntDebugFRMQualityLevelOverride"], "1");
        let custom = RobloxFlags { preset: Preset::Custom, renderer: Renderer::Vulkan, custom: serde_json::from_str(r#"{"DFIntTaskSchedulerTargetFps":"9999"}"#).unwrap(), ..Default::default() }.to_flags();
        assert_eq!(custom["FFlagDebugGraphicsPreferVulkan"], "True");
        assert_eq!(ignored(&custom), vec!["DFIntTaskSchedulerTargetFps".to_string()]);
    }

    #[test]
    fn finds_versions_and_merges_flags() {
        let dir = tempfile::tempdir().unwrap();
        let roblox = dir.path().join("Roblox");
        let boot = dir.path().join("Bloxstrap");
        for (root, v) in [(&roblox, "version-aaa"), (&roblox, "version-bbb"), (&boot, "version-ccc")] {
            std::fs::create_dir_all(root.join("Versions").join(v)).unwrap();
            std::fs::write(root.join("Versions").join(v).join(PLAYER_EXE), b"MZ").unwrap();
        }
        // A folder without the player is not a version.
        std::fs::create_dir_all(roblox.join("Versions/version-empty")).unwrap();
        let roots = vec![(roblox.clone(), None), (boot.clone(), Some("Bloxstrap")), (dir.path().join("missing"), None)];
        let inst = detect_in(&roots);
        assert!(inst.found);
        assert_eq!(inst.version_dirs.len(), 3);
        assert_eq!(inst.bootstrapper.as_deref(), Some("Bloxstrap"));

        // The user's own flag survives; ours are replaced on the next write.
        let user_file = roblox.join("Versions/version-aaa/ClientSettings/ClientAppSettings.json");
        std::fs::create_dir_all(user_file.parent().unwrap()).unwrap();
        std::fs::write(&user_file, r#"{"FFlagUserThing":"True"}"#).unwrap();
        let first = RobloxFlags { preset: Preset::MaxFps, ..Default::default() }.to_flags();
        let files = write_flags(&roots, &first, &[]).unwrap();
        assert_eq!(files.len(), 4, "three versions + Bloxstrap's Modifications");
        let managed: Vec<String> = first.keys().cloned().collect();
        let second = RobloxFlags { preset: Preset::Quality, ..Default::default() }.to_flags();
        write_flags(&roots, &second, &managed).unwrap();
        let v: Map<String, Value> = serde_json::from_slice(&std::fs::read(&user_file).unwrap()).unwrap();
        assert_eq!(v["FFlagUserThing"], "True");
        assert_eq!(v["FIntDebugForceMSAASamples"], "4");
        assert!(!v.contains_key("FFlagDebugGraphicsPreferD3D11"), "old preset's flags are gone");
        assert!(boot.join("Modifications/ClientSettings/ClientAppSettings.json").is_file());

        assert!(write_flags(&[(dir.path().join("none"), None)], &second, &[]).is_err());
    }
}
