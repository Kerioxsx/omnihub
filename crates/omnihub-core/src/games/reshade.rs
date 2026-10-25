//! Roblox shaders, the way RoShade and Bloxshade do it: the official ReShade
//! (the build Roblox allows; never the add-on one) next to the Roblox player
//! as `dxgi.dll`, effects from ReShade's own packages (SweetFX, qUINT) and
//! OmniHub's presets. In the game, Home opens ReShade's menu and End turns
//! the effects off.
//!
//! Everything lives in OmniHub's folder; each Roblox version folder gets the
//! DLL, a `ReShade.ini` pointing back at it, and a marker file so OmniHub
//! only ever removes its own copy (another tool's ReShade is left alone).

use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const SITE: &str = "https://reshade.me";
pub const RAW: &str = "https://raw.githubusercontent.com";
/// Beside `dxgi.dll` when the copy is OmniHub's.
pub const MARKER: &str = "OmniHub.reshade";

/// The effect files (path under `RAW` → file name). The `.fxh` files are
/// what the effects include.
const FILES: &[(&str, &str)] = &[
    ("crosire/reshade-shaders/slim/Shaders/ReShade.fxh", "ReShade.fxh"),
    ("crosire/reshade-shaders/slim/Shaders/ReShadeUI.fxh", "ReShadeUI.fxh"),
    ("CeeJayDK/SweetFX/master/Shaders/SweetFX/Vibrance.fx", "Vibrance.fx"),
    ("CeeJayDK/SweetFX/master/Shaders/SweetFX/LumaSharpen.fx", "LumaSharpen.fx"),
    ("CeeJayDK/SweetFX/master/Shaders/SweetFX/Tonemap.fx", "Tonemap.fx"),
    ("CeeJayDK/SweetFX/master/Shaders/SweetFX/Vignette.fx", "Vignette.fx"),
    ("martymcmodding/qUINT/master/Shaders/qUINT_common.fxh", "qUINT_common.fxh"),
    ("martymcmodding/qUINT/master/Shaders/qUINT_bloom.fx", "qUINT_bloom.fx"),
    ("martymcmodding/qUINT/master/Shaders/qUINT_lightroom.fx", "qUINT_lightroom.fx"),
    ("martymcmodding/qUINT/master/Shaders/qUINT_mxao.fx", "qUINT_mxao.fx"),
    ("martymcmodding/qUINT/master/Shaders/qUINT_sharp.fx", "qUINT_sharp.fx"),
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ShaderPreset {
    /// Shadows in corners (ambient occlusion), soft bloom, a little contrast.
    #[default]
    Realistic,
    /// Livelier colours and a crisper image.
    Vivid,
    /// Warmer, contrasty, bloom and a vignette, like a film.
    Cinematic,
    /// Dreamy glow and gentle colours.
    Soft,
    /// Only sharper and a touch more colour: the lightest on frame rate.
    Sharp,
    /// What you tune in ReShade's menu in the game (kept as you leave it).
    Custom,
}

impl ShaderPreset {
    pub const ALL: [ShaderPreset; 6] = [ShaderPreset::Realistic, ShaderPreset::Vivid, ShaderPreset::Cinematic, ShaderPreset::Soft, ShaderPreset::Sharp, ShaderPreset::Custom];

    fn file(self) -> &'static str {
        match self {
            ShaderPreset::Realistic => "Realistic.ini",
            ShaderPreset::Vivid => "Vivid.ini",
            ShaderPreset::Cinematic => "Cinematic.ini",
            ShaderPreset::Soft => "Soft.ini",
            ShaderPreset::Sharp => "Sharp.ini",
            ShaderPreset::Custom => "Custom.ini",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ShaderSettings {
    pub enabled: bool,
    pub preset: ShaderPreset,
    /// How strong the effects are, 25–150 %.
    pub strength: u8,
}

impl Default for ShaderSettings {
    fn default() -> Self {
        ShaderSettings { enabled: false, preset: ShaderPreset::Realistic, strength: 100 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ShaderStatus {
    /// ReShade's version once downloaded.
    pub version: Option<String>,
    /// Roblox folders with OmniHub's ReShade in place.
    pub applied: usize,
    /// Roblox folders with another tool's `dxgi.dll` (RoShade, Bloxshade…),
    /// left as they are.
    pub foreign: Vec<String>,
}

pub struct Shaders {
    pub dir: PathBuf,
    pub site: String,
    pub raw: String,
}

/// The newest standard (not add-on) setup on ReShade's page.
pub fn latest_version(page: &str) -> Option<String> {
    const KEY: &str = "downloads/ReShade_Setup_";
    let mut best: Option<String> = None;
    for (i, _) in page.match_indices(KEY) {
        let rest = &page[i + KEY.len()..];
        let ver: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
        let ver = ver.trim_end_matches('.').to_string();
        if ver.is_empty() || !rest[ver.len()..].starts_with(".exe") {
            continue;
        }
        let key = |v: &str| v.split('.').map(|p| p.parse::<u32>().unwrap_or(0)).collect::<Vec<_>>();
        if best.as_deref().is_none_or(|b| key(&ver) > key(b)) {
            best = Some(ver);
        }
    }
    best
}

/// ReShade64.dll out of the setup (a zip archive rides at its end).
pub fn extract_dll(setup: &[u8]) -> Result<Vec<u8>, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(setup)).map_err(|e| format!("ReShade's setup couldn't be opened: {e}"))?;
    let mut f = zip.by_name("ReShade64.dll").map_err(|_| "ReShade's setup has no ReShade64.dll.".to_string())?;
    let mut out = Vec::new();
    f.by_ref().take(64 << 20).read_to_end(&mut out).map_err(|e| e.to_string())?;
    if out.len() < 100_000 || !out.starts_with(b"MZ") {
        return Err("ReShade64.dll in the setup doesn't look right.".into());
    }
    Ok(out)
}

/// One effect in a preset: technique, file, its values.
type Section = (&'static str, &'static str, Vec<(&'static str, String)>);

fn f(v: f32) -> String {
    format!("{v:.6}")
}

/// A preset file, the way ReShade writes them, at `strength` (1 = as made).
pub fn preset_text(preset: ShaderPreset, strength: f32) -> String {
    let k = strength.clamp(0.25, 1.5);
    let mut sections: Vec<Section> = Vec::new();
    let mxao = |amount: f32| ("MXAO", "qUINT_mxao.fx", vec![("MXAO_GLOBAL_SAMPLE_QUALITY_PRESET", "2".to_string()), ("MXAO_SAMPLE_RADIUS", f(2.5)), ("MXAO_SSAO_AMOUNT", f(amount * k)), ("MXAO_SSIL_AMOUNT", f(0.0))]);
    let lightroom = |contrast: f32, vibrance: f32, saturation: f32, blacks: f32, temperature: f32| {
        ("Lightroom", "qUINT_lightroom.fx", vec![("LIGHTROOM_GLOBAL_CONTRAST", f(contrast * k)), ("LIGHTROOM_GLOBAL_VIBRANCE", f(vibrance * k)), ("LIGHTROOM_GLOBAL_SATURATION", f(saturation * k)), ("LIGHTROOM_GLOBAL_BLACKS_CURVE", f(blacks * k)), ("LIGHTROOM_GLOBAL_TEMPERATURE", f(temperature * k))])
    };
    let bloom = |intensity: f32, curve: f32, sat: f32| ("Bloom", "qUINT_bloom.fx", vec![("BLOOM_INTENSITY", f(intensity * k)), ("BLOOM_CURVE", f(curve)), ("BLOOM_SAT", f(sat))]);
    let luma = |s: f32| ("LumaSharpen", "LumaSharpen.fx", vec![("sharp_strength", f((s * k).clamp(0.1, 3.0))), ("sharp_clamp", f(0.035))]);
    let vibrance = |v: f32| ("Vibrance", "Vibrance.fx", vec![("Vibrance", f((v * k).clamp(-1.0, 1.0))), ("VibranceRGBBalance", "1.000000,1.000000,1.000000".to_string())]);
    let vignette = |amount: f32, radius: f32| ("Vignette", "Vignette.fx", vec![("Type", "0".to_string()), ("Amount", f((amount * k).clamp(-2.0, 1.0))), ("Radius", f(radius)), ("Slope", "2".to_string())]);
    match preset {
        ShaderPreset::Realistic | ShaderPreset::Custom => {
            sections.push(mxao(1.0));
            sections.push(lightroom(0.12, 0.15, 0.0, -0.08, 0.03));
            sections.push(bloom(0.6, 2.0, 1.2));
            sections.push(luma(0.5));
            sections.push(vignette(-0.35, 1.6));
        }
        ShaderPreset::Vivid => {
            sections.push(vibrance(0.35));
            sections.push(lightroom(0.1, 0.1, 0.12, -0.05, 0.0));
            sections.push(bloom(0.4, 2.0, 1.5));
            sections.push(luma(0.6));
        }
        ShaderPreset::Cinematic => {
            sections.push(mxao(1.2));
            sections.push(lightroom(0.2, 0.05, -0.05, -0.12, 0.08));
            sections.push(bloom(1.0, 1.8, 1.0));
            sections.push(("Tonemap", "Tonemap.fx", vec![("Gamma", f(1.0)), ("Exposure", f(0.0)), ("Saturation", f(-0.05 * k)), ("Bleach", f((0.1 * k).min(1.0))), ("Defog", f(0.0))]));
            sections.push(vignette(-0.6, 1.3));
            sections.push(("DELC_Sharpen", "qUINT_sharp.fx", vec![("SHARP_STRENGTH", f((0.5 * k).min(1.0)))]));
        }
        ShaderPreset::Soft => {
            sections.push(bloom(1.4, 1.4, 1.0));
            sections.push(lightroom(-0.05, 0.1, 0.0, 0.0, 0.05));
            sections.push(vibrance(0.15));
        }
        ShaderPreset::Sharp => {
            sections.push(("DELC_Sharpen", "qUINT_sharp.fx", vec![("SHARP_STRENGTH", f((0.7 * k).min(1.0)))]));
            sections.push(vibrance(0.2));
        }
    }
    let list = sections.iter().map(|(t, file, _)| format!("{t}@{file}")).collect::<Vec<_>>().join(",");
    let mut out = format!("PreprocessorDefinitions=\nTechniques={list}\nTechniqueSorting={list}\n");
    for (_, file, values) in &sections {
        out.push_str(&format!("\n[{file}]\n"));
        for (k, v) in values {
            out.push_str(&format!("{k}={v}\n"));
        }
    }
    out
}

impl Shaders {
    pub fn new(dir: PathBuf) -> Shaders {
        Shaders { dir, site: SITE.into(), raw: RAW.into() }
    }

    fn shaders_dir(&self) -> PathBuf {
        self.dir.join("Shaders")
    }

    fn dll(&self) -> PathBuf {
        self.dir.join("ReShade64.dll")
    }

    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(90))).build().into();
        let mut r = agent.get(url).header("User-Agent", "OmniHub").call().map_err(|e| format!("{url}: {e}"))?;
        r.body_mut().with_config().limit(64 << 20).read_to_vec().map_err(|e| format!("{url}: {e}"))
    }

    /// The ReShade version downloaded, if all its files are here.
    pub fn version(&self) -> Option<String> {
        let ok = self.dll().is_file() && FILES.iter().all(|(_, n)| self.shaders_dir().join(n).is_file());
        ok.then(|| std::fs::read_to_string(self.dir.join("version.txt")).ok()).flatten().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }

    /// Download ReShade (when there's a newer one) and the effects.
    pub fn download(&self, progress: &mut dyn FnMut(&str)) -> Result<String, String> {
        progress("Looking for the newest ReShade");
        let page = String::from_utf8_lossy(&self.get(&format!("{}/", self.site))?).into_owned();
        let ver = match latest_version(&page) {
            Some(v) => v,
            None => return self.version().ok_or_else(|| "ReShade's site didn't list a download.".to_string()),
        };
        if self.version().as_deref() == Some(ver.as_str()) {
            return Ok(ver);
        }
        std::fs::create_dir_all(self.shaders_dir()).map_err(|e| e.to_string())?;
        progress(&format!("Downloading ReShade {ver}"));
        let dll = extract_dll(&self.get(&format!("{}/downloads/ReShade_Setup_{ver}.exe", self.site))?)?;
        write_atomic(&self.dll(), &dll)?;
        for (path, name) in FILES {
            progress(&format!("Downloading {name}"));
            let bytes = self.get(&format!("{}/{path}", self.raw))?;
            if bytes.len() > 2 << 20 || bytes.starts_with(b"<!") {
                return Err(format!("{name} didn't download properly."));
            }
            write_atomic(&self.shaders_dir().join(name), &bytes)?;
        }
        std::fs::create_dir_all(self.dir.join("Textures")).map_err(|e| e.to_string())?;
        write_atomic(&self.dir.join("version.txt"), ver.as_bytes())?;
        Ok(ver)
    }

    /// The presets at `strength` (the Custom one only the first time, then
    /// it keeps what was tuned in the game).
    fn write_presets(&self, strength: u8) -> Result<(), String> {
        let dir = self.dir.join("presets");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        for p in ShaderPreset::ALL {
            let path = dir.join(p.file());
            if p == ShaderPreset::Custom && path.is_file() {
                continue;
            }
            write_atomic(&path, preset_text(p, strength as f32 / 100.0).as_bytes())?;
        }
        Ok(())
    }

    /// The `ReShade.ini` for a Roblox folder.
    pub fn ini(&self, preset: ShaderPreset) -> String {
        let shots = dirs::picture_dir().map(|p| p.join("OmniHub")).unwrap_or_else(|| self.dir.join("Screenshots"));
        format!(
            "[GENERAL]\nEffectSearchPaths={}\nTextureSearchPaths={}\nPresetPath={}\nPerformanceMode=1\nSkipLoadingDisabledEffects=1\nCheckForUpdates=0\n\n[INPUT]\nKeyOverlay=36,0,0,0\nKeyEffects=35,0,0,0\n\n[OVERLAY]\nTutorialProgress=4\n\n[SCREENSHOT]\nSavePath={}\n",
            self.shaders_dir().display(),
            self.dir.join("Textures").display(),
            self.dir.join("presets").join(preset.file()).display(),
            shots.display()
        )
    }

    /// Put ReShade in (or take OmniHub's out of) each Roblox folder.
    pub fn apply(&self, s: &ShaderSettings, folders: &[PathBuf]) -> Result<ShaderStatus, String> {
        let mut st = ShaderStatus { version: self.version(), ..Default::default() };
        if s.enabled {
            if st.version.is_none() {
                return Err("ReShade isn't downloaded yet.".into());
            }
            self.write_presets(s.strength.clamp(25, 150))?;
        }
        let dll = if s.enabled { Some(std::fs::read(self.dll()).map_err(|e| e.to_string())?) } else { None };
        for d in folders {
            let target = d.join("dxgi.dll");
            let ours = d.join(MARKER).is_file();
            if target.is_file() && !ours {
                st.foreign.push(d.to_string_lossy().into_owned());
                continue;
            }
            match &dll {
                Some(bytes) => {
                    std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
                    if std::fs::metadata(&target).map(|m| m.len()).ok() != Some(bytes.len() as u64) {
                        write_atomic(&target, bytes)?;
                    }
                    write_atomic(&d.join("ReShade.ini"), self.ini(s.preset).as_bytes())?;
                    write_atomic(&d.join(MARKER), b"OmniHub put ReShade here; it removes it when shaders are turned off.\n")?;
                    st.applied += 1;
                }
                None if ours => {
                    for name in ["dxgi.dll", "ReShade.ini", "ReShade.log", MARKER] {
                        let _ = std::fs::remove_file(d.join(name));
                    }
                }
                None => {}
            }
        }
        Ok(st)
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("omnihub-part");
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn finds_the_standard_setup_on_the_page() {
        let page = r#"<a href="/downloads/ReShade_Setup_6.7.3.exe">x</a> <a href="/downloads/ReShade_Setup_6.8.0_Addon.exe">y</a> <a href="/downloads/ReShade_Setup_6.8.0.exe">z</a>"#;
        assert_eq!(latest_version(page).as_deref(), Some("6.8.0"));
        assert_eq!(latest_version("<html>nothing</html>"), None);
    }

    fn setup_with(dll: &[u8]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("ReShade64.dll", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(dll).unwrap();
        let archive = zip.finish().unwrap().into_inner();
        // The setup program comes first, the archive after it.
        let mut exe = b"MZ pretend setup program".repeat(100);
        exe.extend_from_slice(&archive);
        exe
    }

    #[test]
    fn takes_the_dll_out_of_the_setup() {
        let mut dll = b"MZ".to_vec();
        dll.extend(std::iter::repeat_n(7u8, 200_000));
        assert_eq!(extract_dll(&setup_with(&dll)).unwrap(), dll);
        assert!(extract_dll(&setup_with(b"not a dll")).is_err());
    }

    #[test]
    fn presets_name_their_techniques_and_scale() {
        let p = preset_text(ShaderPreset::Realistic, 1.0);
        assert!(p.contains("Techniques=MXAO@qUINT_mxao.fx,Lightroom@qUINT_lightroom.fx,Bloom@qUINT_bloom.fx,LumaSharpen@LumaSharpen.fx,Vignette@Vignette.fx\n"));
        assert!(p.contains("[qUINT_bloom.fx]\nBLOOM_INTENSITY=0.600000\n"));
        assert!(preset_text(ShaderPreset::Realistic, 0.5).contains("BLOOM_INTENSITY=0.300000"));
        // Every effect a preset uses is one OmniHub downloads.
        for preset in ShaderPreset::ALL {
            let text = preset_text(preset, 1.5);
            let line = text.lines().find(|l| l.starts_with("Techniques=")).unwrap();
            for t in line["Techniques=".len()..].split(',') {
                let file = t.split('@').nth(1).unwrap();
                assert!(FILES.iter().any(|(_, n)| *n == file), "{file}");
            }
        }
    }

    /// Against the real sites (run with --ignored).
    #[test]
    #[ignore]
    fn downloads_reshade_and_the_effects() {
        let tmp = tempfile::tempdir().unwrap();
        let sh = Shaders::new(tmp.path().join("reshade"));
        let v = sh.download(&mut |_| {}).unwrap();
        assert_eq!(sh.version().as_deref(), Some(v.as_str()));
        assert!(std::fs::metadata(sh.dll()).unwrap().len() > 1_000_000);
        assert!(std::fs::read_to_string(sh.shaders_dir().join("qUINT_bloom.fx")).unwrap().contains("technique"));
    }

    #[test]
    fn puts_reshade_in_and_takes_only_its_own_out() {
        let tmp = tempfile::tempdir().unwrap();
        let sh = Shaders::new(tmp.path().join("reshade"));
        std::fs::create_dir_all(sh.shaders_dir()).unwrap();
        std::fs::write(sh.dll(), b"MZ our reshade").unwrap();
        for (_, n) in FILES {
            std::fs::write(sh.shaders_dir().join(n), "// fx").unwrap();
        }
        std::fs::write(sh.dir.join("version.txt"), "6.8.0").unwrap();
        let (mine, theirs) = (tmp.path().join("version-a"), tmp.path().join("version-b"));
        std::fs::create_dir_all(&theirs).unwrap();
        std::fs::write(theirs.join("dxgi.dll"), b"MZ someone else's").unwrap();
        let on = ShaderSettings { enabled: true, preset: ShaderPreset::Cinematic, strength: 100 };
        let st = sh.apply(&on, &[mine.clone(), theirs.clone()]).unwrap();
        assert_eq!((st.version.as_deref(), st.applied, st.foreign.len()), (Some("6.8.0"), 1, 1));
        assert_eq!(std::fs::read(mine.join("dxgi.dll")).unwrap(), b"MZ our reshade");
        let ini = std::fs::read_to_string(mine.join("ReShade.ini")).unwrap();
        assert!(ini.contains("Cinematic.ini") && ini.contains("KeyOverlay=36,0,0,0"));
        assert!(sh.dir.join("presets").join("Cinematic.ini").is_file());
        // Custom is kept as tuned in the game.
        std::fs::write(sh.dir.join("presets").join("Custom.ini"), "Techniques=Mine@Vibrance.fx\n").unwrap();
        sh.apply(&on, std::slice::from_ref(&mine)).unwrap();
        assert!(std::fs::read_to_string(sh.dir.join("presets").join("Custom.ini")).unwrap().contains("Mine@"));
        // Off: OmniHub's copy goes, the other tool's stays.
        sh.apply(&ShaderSettings::default(), &[mine.clone(), theirs.clone()]).unwrap();
        assert!(!mine.join("dxgi.dll").exists() && !mine.join(MARKER).exists());
        assert!(theirs.join("dxgi.dll").exists());
    }
}
