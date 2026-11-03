//! Emulators: find the ones installed (RetroArch, Dolphin, PCSX2, PPSSPP,
//! DuckStation, mGBA, melonDS, Azahar/Citra, Ryujinx, Cemu, RPCS3…) or add
//! one by hand, scan your ROM folders, show box art (libretro's thumbnail
//! collection, matched by the ROM's name), start a game in the emulator for
//! its platform, and find its save next to it. Kept in `emulators.json`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// (name, program names, platforms, arguments with {rom}).
const KNOWN: &[(&str, &[&str], &[&str], &str)] = &[
    ("RetroArch", &["retroarch.exe"], &["gba", "gb", "gbc", "nes", "snes", "n64", "nds", "genesis", "psx"], "{core} \"{rom}\""),
    ("mGBA", &["mgba.exe"], &["gba", "gb", "gbc"], "\"{rom}\""),
    ("VisualBoyAdvance-M", &["visualboyadvance-m.exe"], &["gba", "gb", "gbc"], "\"{rom}\""),
    ("melonDS", &["melonds.exe"], &["nds"], "\"{rom}\""),
    ("DeSmuME", &["desmume.exe", "desmume_x64.exe"], &["nds"], "\"{rom}\""),
    ("Azahar", &["azahar.exe", "lime3ds.exe", "lime3ds-gui.exe", "citra-qt.exe"], &["3ds"], "\"{rom}\""),
    ("Dolphin", &["dolphin.exe"], &["gc", "wii"], "-e \"{rom}\""),
    ("Cemu", &["cemu.exe"], &["wiiu"], "-g \"{rom}\""),
    ("Ryujinx", &["ryujinx.exe"], &["switch"], "\"{rom}\""),
    ("Snes9x", &["snes9x-x64.exe", "snes9x.exe"], &["snes"], "\"{rom}\""),
    ("Mesen", &["mesen.exe"], &["nes", "snes", "gb", "gbc"], "\"{rom}\""),
    ("Project64", &["project64.exe"], &["n64"], "\"{rom}\""),
    ("DuckStation", &["duckstation-qt-x64-releaseltcg.exe", "duckstation-qt.exe"], &["psx"], "\"{rom}\""),
    ("PCSX2", &["pcsx2-qt.exe", "pcsx2.exe"], &["ps2"], "\"{rom}\""),
    ("PPSSPP", &["ppssppwindows64.exe", "ppssppwindows.exe"], &["psp"], "\"{rom}\""),
    ("RPCS3", &["rpcs3.exe"], &["ps3"], "\"{rom}\""),
];

/// RetroArch's core for each platform.
const CORES: &[(&str, &str)] = &[("gba", "mgba"), ("gb", "gambatte"), ("gbc", "gambatte"), ("nes", "mesen"), ("snes", "snes9x"), ("n64", "mupen64plus_next"), ("nds", "melonds"), ("genesis", "genesis_plus_gx"), ("psx", "swanstation")];

pub const PLATFORMS: &[(&str, &str, &str)] = &[
    ("gba", "Game Boy Advance", "Nintendo_-_Game_Boy_Advance"),
    ("gb", "Game Boy", "Nintendo_-_Game_Boy"),
    ("gbc", "Game Boy Color", "Nintendo_-_Game_Boy_Color"),
    ("nds", "Nintendo DS", "Nintendo_-_Nintendo_DS"),
    ("3ds", "Nintendo 3DS", "Nintendo_-_Nintendo_3DS"),
    ("nes", "NES", "Nintendo_-_Nintendo_Entertainment_System"),
    ("snes", "SNES", "Nintendo_-_Super_Nintendo_Entertainment_System"),
    ("n64", "Nintendo 64", "Nintendo_-_Nintendo_64"),
    ("gc", "GameCube", "Nintendo_-_GameCube"),
    ("wii", "Wii", "Nintendo_-_Wii"),
    ("wiiu", "Wii U", ""),
    ("switch", "Switch", ""),
    ("genesis", "Mega Drive / Genesis", "Sega_-_Mega_Drive_-_Genesis"),
    ("psx", "PlayStation", "Sony_-_PlayStation"),
    ("ps2", "PlayStation 2", "Sony_-_PlayStation_2"),
    ("psp", "PlayStation Portable", "Sony_-_PlayStation_Portable"),
    ("ps3", "PlayStation 3", ""),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Emulator {
    pub id: String,
    pub name: String,
    pub exe: String,
    pub platforms: Vec<String>,
    /// With `{rom}` (and `{core}` for RetroArch).
    pub args: String,
    /// Found by itself (not added by hand).
    pub found: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rom {
    pub id: String,
    pub path: String,
    pub name: String,
    pub platform: String,
    pub cover: Option<String>,
    /// The save file next to it (or RetroArch's), if there is one.
    pub save: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct EmuState {
    pub emulators: Vec<Emulator>,
    pub rom_dirs: Vec<String>,
    /// Which emulator plays each platform (else the first that can).
    pub chosen: HashMap<String, String>,
    #[serde(skip_deserializing)]
    pub roms: Vec<Rom>,
}

pub struct Emulators {
    file: PathBuf,
    state: Mutex<EmuState>,
}

/// A platform from a ROM's extension (and its folder's name for discs).
pub fn platform_of(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    Some(match ext.as_str() {
        "gba" => "gba",
        "gb" => "gb",
        "gbc" => "gbc",
        "nds" => "nds",
        "3ds" | "cci" | "cxi" => "3ds",
        "nes" => "nes",
        "sfc" | "smc" => "snes",
        "n64" | "z64" | "v64" => "n64",
        "gcm" | "rvz" | "gcz" => "gc",
        "wbfs" | "wad" => "wii",
        "wua" | "wud" | "wux" => "wiiu",
        "xci" | "nsp" => "switch",
        "md" | "gen" | "smd" => "genesis",
        "pbp" | "cso" => "psp",
        "iso" | "chd" | "cue" | "bin" => {
            let folder = path.parent().map(|p| p.to_string_lossy().to_lowercase()).unwrap_or_default();
            if folder.contains("ps2") {
                "ps2"
            } else if folder.contains("psp") {
                "psp"
            } else if folder.contains("gamecube") || folder.contains("gcn") {
                "gc"
            } else if folder.contains("wii") {
                "wii"
            } else if folder.contains("psx") || folder.contains("ps1") || folder.contains("playstation") {
                "psx"
            } else {
                return None;
            }
        }
        _ => return None,
    })
}

/// libretro's box art for a ROM named the No-Intro way ("Pokemon - Emerald Version (USA, Europe)").
pub fn cover_url(platform: &str, name: &str) -> Option<String> {
    let repo = PLATFORMS.iter().find(|p| p.0 == platform).map(|p| p.2).filter(|r| !r.is_empty())?;
    let file: String = name.chars().map(|c| if "&*/:`<>?\\|\"".contains(c) { '_' } else { c }).collect();
    let enc = percent_encoding::utf8_percent_encode(&file, percent_encoding::NON_ALPHANUMERIC);
    Some(format!("https://raw.githubusercontent.com/libretro-thumbnails/{repo}/master/Named_Boxarts/{enc}.png"))
}

fn exe_lower(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Look for known emulators in the usual places (two folders deep).
pub fn find_installed(roots: &[PathBuf]) -> Vec<Emulator> {
    let mut out: Vec<Emulator> = Vec::new();
    for root in roots {
        for e in walkdir::WalkDir::new(root).max_depth(3).into_iter().filter_entry(|e| {
            let n = e.file_name().to_string_lossy().to_lowercase();
            e.depth() == 0 || !(n == "windows" || n == "node_modules" || n.starts_with('.') || n == "steamapps" || n == "common")
        }) {
            let Ok(e) = e else { continue };
            if !e.file_type().is_file() {
                continue;
            }
            let n = exe_lower(e.path());
            if let Some((name, _, plats, args)) = KNOWN.iter().find(|k| k.1.contains(&n.as_str())) {
                if out.iter().any(|o| o.name == *name) {
                    continue;
                }
                out.push(Emulator { id: name.to_lowercase().replace(' ', "-"), name: name.to_string(), exe: e.path().to_string_lossy().into_owned(), platforms: plats.iter().map(|s| s.to_string()).collect(), args: args.to_string(), found: true });
            }
        }
    }
    out
}

/// Where emulators usually are on Windows.
pub fn default_roots() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = ["ProgramFiles", "ProgramFiles(x86)"].iter().filter_map(|k| std::env::var_os(k).map(PathBuf::from)).collect();
    if let Some(local) = dirs::data_local_dir() {
        v.push(local.join("Programs"));
        v.push(local.clone());
    }
    if let Some(roaming) = dirs::data_dir() {
        v.push(roaming);
    }
    if let Some(home) = dirs::home_dir() {
        v.push(home.join("scoop").join("apps"));
        v.push(home.join("Emulators"));
        v.push(home.join("Desktop"));
        v.push(home.join("Downloads"));
    }
    v.push(PathBuf::from(r"C:\Emulators"));
    v.push(PathBuf::from(r"C:\RetroArch-Win64"));
    v.into_iter().filter(|p| p.is_dir()).collect()
}

/// The save next to a ROM (.sav/.srm/.dsv), or RetroArch's.
fn save_for(rom: &Path, retroarch: Option<&Path>) -> Option<String> {
    let stem = rom.file_stem()?.to_string_lossy().into_owned();
    let dir = rom.parent()?;
    let mut candidates: Vec<PathBuf> = ["sav", "srm", "dsv"].iter().map(|x| dir.join(format!("{stem}.{x}"))).collect();
    if let Some(ra) = retroarch.and_then(Path::parent) {
        candidates.push(ra.join("saves").join(format!("{stem}.srm")));
        candidates.push(ra.join("saves").join(format!("{stem}.sav")));
    }
    candidates.into_iter().find(|p| p.is_file()).map(|p| p.to_string_lossy().into_owned())
}

pub fn scan_roms(dirs: &[String], retroarch: Option<&Path>) -> Vec<Rom> {
    let mut out = Vec::new();
    for d in dirs {
        for e in walkdir::WalkDir::new(d).max_depth(4).into_iter().flatten() {
            if !e.file_type().is_file() {
                continue;
            }
            let Some(platform) = platform_of(e.path()) else { continue };
            // A .bin with a .cue beside it is one game: list the .cue.
            if e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("bin")) && e.path().with_extension("cue").is_file() {
                continue;
            }
            let name = e.path().file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            let path = e.path().to_string_lossy().into_owned();
            out.push(Rom { id: blake3::hash(path.as_bytes()).to_hex()[..16].to_string(), cover: cover_url(platform, &name), save: save_for(e.path(), retroarch), name, platform: platform.into(), path });
            if out.len() >= 5000 {
                return out;
            }
        }
    }
    out.sort_by_key(|r| (r.platform.clone(), r.name.to_lowercase()));
    out
}

impl Emulators {
    pub fn new(data: &Path) -> Emulators {
        let file = data.join("emulators.json");
        let state = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Emulators { file, state: Mutex::new(state) }
    }

    fn save(&self, s: &EmuState) -> Result<(), String> {
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    pub fn state(&self) -> EmuState {
        self.state.lock().clone()
    }

    /// Look for emulators again and rescan the ROM folders.
    pub fn refresh(&self, roots: &[PathBuf]) -> Result<EmuState, String> {
        let found = find_installed(roots);
        let mut s = self.state.lock();
        s.emulators.retain(|e| !e.found || Path::new(&e.exe).is_file());
        for f in found {
            if !s.emulators.iter().any(|e| e.exe.eq_ignore_ascii_case(&f.exe) || (e.found && e.name == f.name)) {
                s.emulators.push(f);
            }
        }
        let ra = s.emulators.iter().find(|e| e.name == "RetroArch").map(|e| PathBuf::from(&e.exe));
        s.roms = scan_roms(&s.rom_dirs, ra.as_deref());
        self.save(&s)?;
        Ok(s.clone())
    }

    /// Add an emulator by its program (known ones get their platforms and arguments).
    pub fn add(&self, exe: &str, platforms: Vec<String>) -> Result<EmuState, String> {
        let p = Path::new(exe);
        if !p.is_file() || !exe_lower(p).ends_with(".exe") {
            return Err("Pick the emulator's .exe".into());
        }
        let n = exe_lower(p);
        let (name, plats, args) = match KNOWN.iter().find(|k| k.1.contains(&n.as_str())) {
            Some((name, _, plats, args)) => (name.to_string(), plats.iter().map(|s| s.to_string()).collect(), args.to_string()),
            None => (p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(), platforms, "\"{rom}\"".to_string()),
        };
        let mut s = self.state.lock();
        if !s.emulators.iter().any(|e| e.exe.eq_ignore_ascii_case(exe)) {
            s.emulators.push(Emulator { id: format!("{}-{}", name.to_lowercase().replace(' ', "-"), &blake3::hash(exe.as_bytes()).to_hex()[..6]), name, exe: exe.into(), platforms: plats, args, found: false });
        }
        self.save(&s)?;
        Ok(s.clone())
    }

    pub fn remove(&self, id: &str) -> Result<EmuState, String> {
        let mut s = self.state.lock();
        s.emulators.retain(|e| e.id != id);
        s.chosen.retain(|_, v| v != id);
        self.save(&s)?;
        Ok(s.clone())
    }

    pub fn set_dirs(&self, dirs: Vec<String>) -> Result<EmuState, String> {
        let mut s = self.state.lock();
        s.rom_dirs = dirs.into_iter().filter(|d| Path::new(d).is_dir()).take(20).collect();
        let ra = s.emulators.iter().find(|e| e.name == "RetroArch").map(|e| PathBuf::from(&e.exe));
        s.roms = scan_roms(&s.rom_dirs, ra.as_deref());
        self.save(&s)?;
        Ok(s.clone())
    }

    pub fn choose(&self, platform: &str, emulator: &str) -> Result<EmuState, String> {
        let mut s = self.state.lock();
        if emulator.is_empty() {
            s.chosen.remove(platform);
        } else {
            s.chosen.insert(platform.into(), emulator.into());
        }
        self.save(&s)?;
        Ok(s.clone())
    }

    /// The program and arguments that start a ROM.
    pub fn command(&self, rom_id: &str) -> Result<(String, Vec<String>), String> {
        let s = self.state.lock();
        let rom = s.roms.iter().find(|r| r.id == rom_id).ok_or("That game isn't in your ROM folders any more")?;
        let emu = s
            .chosen
            .get(&rom.platform)
            .and_then(|id| s.emulators.iter().find(|e| &e.id == id))
            .or_else(|| s.emulators.iter().find(|e| e.platforms.contains(&rom.platform)))
            .ok_or_else(|| format!("No emulator for {} yet: add one", PLATFORMS.iter().find(|p| p.0 == rom.platform).map_or(rom.platform.as_str(), |p| p.1)))?;
        let core = if emu.name == "RetroArch" {
            let core = CORES.iter().find(|c| c.0 == rom.platform).map(|c| c.1).unwrap_or("mgba");
            let dll = Path::new(&emu.exe).parent().map(|d| d.join("cores").join(format!("{core}_libretro.dll"))).filter(|p| p.is_file()).ok_or_else(|| format!("RetroArch needs its {core} core: install it from RetroArch's Online Updater"))?;
            format!("-L \"{}\"", dll.display())
        } else {
            String::new()
        };
        let line = emu.args.replace("{core}", &core).replace("{rom}", &rom.path);
        Ok((emu.exe.clone(), super::split_args(&line)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platforms_covers_and_scan() {
        assert_eq!(platform_of(Path::new("x/Pokemon Emerald.gba")), Some("gba"));
        assert_eq!(platform_of(Path::new("roms/ps2/God of War.iso")), Some("ps2"));
        assert_eq!(platform_of(Path::new("roms/misc/thing.iso")), None);
        assert_eq!(
            cover_url("gba", "Pokemon - Emerald Version (USA, Europe)").unwrap(),
            "https://raw.githubusercontent.com/libretro-thumbnails/Nintendo_-_Game_Boy_Advance/master/Named_Boxarts/Pokemon%20%2D%20Emerald%20Version%20%28USA%2C%20Europe%29.png"
        );
        assert!(cover_url("switch", "x").is_none());
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Pokemon Emerald.gba"), b"x").unwrap();
        std::fs::write(dir.path().join("Pokemon Emerald.sav"), b"x").unwrap();
        std::fs::write(dir.path().join("Game.cue"), b"x").unwrap();
        std::fs::write(dir.path().join("Game.bin"), b"x").unwrap();
        std::fs::write(dir.path().join("readme.txt"), b"x").unwrap();
        let roms = scan_roms(&[dir.path().to_string_lossy().into_owned()], None);
        assert_eq!(roms.len(), 1, "{roms:?}");
        assert!(roms[0].save.as_deref().unwrap().ends_with("Pokemon Emerald.sav"));
    }

    #[test]
    fn finds_adds_and_starts() {
        let dir = tempfile::tempdir().unwrap();
        let mgba = dir.path().join("Programs").join("mGBA");
        std::fs::create_dir_all(&mgba).unwrap();
        std::fs::write(mgba.join("mGBA.exe"), b"x").unwrap();
        let found = find_installed(&[dir.path().to_path_buf()]);
        assert_eq!(found.len(), 1);
        assert!(found[0].platforms.contains(&"gba".to_string()));
        let roms = dir.path().join("roms");
        std::fs::create_dir_all(&roms).unwrap();
        std::fs::write(roms.join("Emerald.gba"), b"x").unwrap();
        let e = Emulators::new(dir.path());
        e.refresh(&[dir.path().join("Programs")]).unwrap();
        let s = e.set_dirs(vec![roms.to_string_lossy().into_owned()]).unwrap();
        let (exe, args) = e.command(&s.roms[0].id).unwrap();
        assert!(exe.ends_with("mGBA.exe"));
        assert!(args[0].ends_with("Emerald.gba"));
        assert!(e.add("nope.txt", vec![]).is_err());
    }
}
