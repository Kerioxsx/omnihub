//! What a game folder says about how the game can be modded: its engine,
//! the mod loaders already in it, its anti-cheat, and where its mod folders
//! are. Only file and folder names are read (a few levels deep), never the
//! game's own files' contents.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Engine {
    /// Unity with its C# code as .NET assemblies (BepInEx 5, MelonLoader).
    UnityMono,
    /// Unity compiled to native code (BepInEx 6, MelonLoader).
    UnityIl2cpp,
    Unreal,
    /// LÖVE (Lua): Balatro and others (Lovely).
    Love,
    /// Rockstar's RAGE: GTA V (Script Hook V).
    Rage,
    Source,
    Source2,
    Godot,
    GameMaker,
    RenPy,
    RpgMaker,
    /// Bethesda's Creation engine (Skyrim, Fallout, Starfield): .esp/.esm plugins.
    Creation,
    #[default]
    Unknown,
}

impl Engine {
    pub fn label(self) -> &'static str {
        match self {
            Engine::UnityMono => "Unity",
            Engine::UnityIl2cpp => "Unity (IL2CPP)",
            Engine::Unreal => "Unreal Engine",
            Engine::Love => "LÖVE",
            Engine::Rage => "RAGE",
            Engine::Source => "Source",
            Engine::Source2 => "Source 2",
            Engine::Godot => "Godot",
            Engine::GameMaker => "GameMaker",
            Engine::RenPy => "Ren'Py",
            Engine::RpgMaker => "RPG Maker",
            Engine::Creation => "Creation Engine",
            Engine::Unknown => "Unknown engine",
        }
    }
}

/// Mod loaders OmniHub knows how to recognise (and most, to install).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum LoaderKind {
    /// BepInEx 5 for Mono Unity games.
    BepInEx,
    /// BepInEx 6 (bleeding edge) for IL2CPP Unity games.
    BepInExIl2cpp,
    MelonLoader,
    /// Lovely injector (LÖVE games), with Steamodded for Balatro.
    Lovely,
    /// Alexander Blade's Script Hook V (GTA V): downloaded by the user from dev-c.com.
    ScriptHookV,
    /// Script Hook V .NET: C# and VB scripts in `scripts\`.
    ScriptHookVDotNet,
    /// UE4SS: Lua mods for Unreal games.
    Ue4ss,
}

impl LoaderKind {
    pub fn label(self) -> &'static str {
        match self {
            LoaderKind::BepInEx => "BepInEx 5",
            LoaderKind::BepInExIl2cpp => "BepInEx 6 (IL2CPP)",
            LoaderKind::MelonLoader => "MelonLoader",
            LoaderKind::Lovely => "Lovely",
            LoaderKind::ScriptHookV => "Script Hook V",
            LoaderKind::ScriptHookVDotNet => "Script Hook V .NET",
            LoaderKind::Ue4ss => "UE4SS",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Traits {
    pub engine: Engine,
    /// Loaders found in the folder.
    pub loaders: Vec<LoaderKind>,
    /// Anti-cheat found in the folder ("Easy Anti-Cheat", "BattlEye"…).
    pub anti_cheat: Vec<String>,
    /// The main program, relative to the game folder.
    pub exe: Option<String>,
    /// Unreal: the folder holding the game's .pak files, relative.
    pub paks: Option<String>,
    /// Unreal: the folder of the Shipping program (where UE4SS goes), relative.
    pub binaries: Option<String>,
}

fn has(dir: &Path, name: &str) -> bool {
    dir.join(name).exists()
}

/// Names in `dir` (lower case), files and folders, one level.
fn names(dir: &Path) -> Vec<(String, bool)> {
    std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| (e.file_name().to_string_lossy().to_lowercase(), e.file_type().map(|t| t.is_dir()).unwrap_or(false))).collect())
        .unwrap_or_default()
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).to_string_lossy().replace('\\', "/")
}

/// The folder holding `Content/Paks`, up to three levels down.
fn find_paks(root: &Path) -> Option<PathBuf> {
    let mut stack = vec![(root.to_path_buf(), 0)];
    while let Some((d, depth)) = stack.pop() {
        let paks = d.join("Content").join("Paks");
        if paks.is_dir() && d.file_name().is_some_and(|n| n != "Engine") {
            return Some(paks);
        }
        if depth < 3 {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let n = e.file_name().to_string_lossy().to_lowercase();
                if e.file_type().is_ok_and(|t| t.is_dir()) && !n.starts_with('.') && n != "engine" {
                    stack.push((e.path(), depth + 1));
                }
            }
        }
    }
    None
}

/// The Unreal Shipping program's folder (`<Project>/Binaries/Win64`).
fn find_shipping(paks: &Path) -> Option<PathBuf> {
    let project = paks.parent()?.parent()?;
    for sub in ["Win64", "WinGDK"] {
        let d = project.join("Binaries").join(sub);
        if names(&d).iter().any(|(n, d)| !d && n.ends_with("-shipping.exe")) {
            return Some(d);
        }
    }
    None
}

/// The biggest program in the folder itself (not crash reporters or anti-cheat).
fn main_exe(dir: &Path) -> Option<String> {
    const SKIP: &[&str] = &["crash", "unins", "setup", "redist", "easyanticheat", "battleye", "beservice", "launcherhelper", "updater", "start_protected_game", "unitycrashhandler"];
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            let l = n.to_lowercase();
            (l.ends_with(".exe") && !SKIP.iter().any(|s| l.contains(s))).then(|| (e.metadata().map(|m| m.len()).unwrap_or(0), n))
        })
        .max()
        .map(|(_, n)| n)
}

/// Read a game folder.
pub fn detect(dir: &Path) -> Traits {
    let top = names(dir);
    let is = |n: &str| top.iter().any(|(x, _)| x == n);
    let is_dir = |n: &str| top.iter().any(|(x, d)| *d && x == n);
    let mut t = Traits { exe: main_exe(dir), ..Default::default() };

    // Engine.
    let data = top.iter().find(|(n, d)| *d && n.ends_with("_data")).map(|(n, _)| n.clone());
    if is("gta5.exe") || is("gta5_enhanced.exe") || is("playgtav.exe") || is("rdr2.exe") {
        t.engine = Engine::Rage;
        t.exe = ["GTA5_Enhanced.exe", "GTA5.exe", "RDR2.exe"].iter().find(|e| top.iter().any(|(x, _)| x == &e.to_lowercase())).map(|s| s.to_string()).or(t.exe);
    } else if is("unityplayer.dll") || data.as_ref().is_some_and(|d| has(&dir.join(d), "globalgamemanagers") || has(&dir.join(d), "data.unity3d")) {
        t.engine = if is("gameassembly.dll") { Engine::UnityIl2cpp } else { Engine::UnityMono };
    } else if is("love.dll") || (is_dir("lovely") && is("lua51.dll")) {
        t.engine = Engine::Love;
    } else if is("data.win") {
        t.engine = Engine::GameMaker;
    } else if is_dir("renpy") && is_dir("game") {
        t.engine = Engine::RenPy;
    } else if has(&dir.join("www").join("js"), "plugins.js") || has(&dir.join("js"), "plugins.js") {
        t.engine = Engine::RpgMaker;
    } else if names(&dir.join("Data")).iter().any(|(n, _)| n.ends_with(".esm")) {
        t.engine = Engine::Creation;
    } else if has(&dir.join("game").join("bin"), "win64") && names(&dir.join("game")).iter().any(|(n, d)| *d && has(&dir.join("game").join(n), "gameinfo.gi")) {
        t.engine = Engine::Source2;
    } else if top.iter().any(|(n, d)| *d && has(&dir.join(n), "gameinfo.txt")) && is_dir("bin") {
        t.engine = Engine::Source;
    } else if top.iter().any(|(n, d)| !d && n.ends_with(".pck")) {
        t.engine = Engine::Godot;
    } else if let Some(paks) = find_paks(dir) {
        t.engine = Engine::Unreal;
        t.binaries = find_shipping(&paks).map(|b| rel(dir, &b));
        t.paks = Some(rel(dir, &paks));
    }
    if t.engine == Engine::Unknown && is_dir("engine") {
        if let Some(paks) = find_paks(dir) {
            t.engine = Engine::Unreal;
            t.binaries = find_shipping(&paks).map(|b| rel(dir, &b));
            t.paks = Some(rel(dir, &paks));
        }
    }

    // Loaders.
    if is_dir("bepinex") && has(&dir.join("BepInEx"), "core") {
        let core = names(&dir.join("BepInEx").join("core"));
        t.loaders.push(if core.iter().any(|(n, _)| n == "bepinex.unity.il2cpp.dll" || n == "bepinex.core.dll") { LoaderKind::BepInExIl2cpp } else { LoaderKind::BepInEx });
    }
    if is_dir("melonloader") {
        t.loaders.push(LoaderKind::MelonLoader);
    }
    if t.engine == Engine::Love && (is("winmm.dll") || is("version.dll")) {
        t.loaders.push(LoaderKind::Lovely);
    }
    if is("scripthookv.dll") {
        t.loaders.push(LoaderKind::ScriptHookV);
    }
    if is("scripthookvdotnet.asi") {
        t.loaders.push(LoaderKind::ScriptHookVDotNet);
    }
    if let Some(b) = &t.binaries {
        let bin = dir.join(b);
        if has(&bin, "ue4ss") || has(&bin, "UE4SS.dll") {
            t.loaders.push(LoaderKind::Ue4ss);
        }
    }

    // Anti-cheat (folders and files a game ships with it).
    let mut ac = Vec::new();
    for (n, _) in &top {
        let name = if n.starts_with("easyanticheat") || n == "start_protected_game.exe" {
            "Easy Anti-Cheat"
        } else if n.starts_with("battleye") || n.starts_with("beservice") || n.starts_with("beclient") {
            "BattlEye"
        } else if n.starts_with("eaanticheat") {
            "EA Anticheat"
        } else if n.contains("xigncode") {
            "XIGNCODE3"
        } else if n == "gameguard" || n.contains("nprotect") {
            "nProtect GameGuard"
        } else if n.starts_with("mhyprot") || n == "mhypbase.dll" {
            "HoYoverse anti-cheat"
        } else if n.starts_with("equ8") {
            "EQU8"
        } else {
            continue;
        };
        if !ac.contains(&name) {
            ac.push(name);
        }
    }
    t.anti_cheat = ac.into_iter().map(String::from).collect();
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(p: &Path) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"x").unwrap();
    }

    #[test]
    fn engines_loaders_and_anti_cheat_are_recognised() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();

        let balatro = r.join("Balatro");
        for f in ["Balatro.exe", "love.dll", "lua51.dll", "winmm.dll"] {
            touch(&balatro.join(f));
        }
        let b = detect(&balatro);
        assert_eq!((b.engine, b.loaders.clone(), b.exe.as_deref()), (Engine::Love, vec![LoaderKind::Lovely], Some("Balatro.exe")));

        let gta = r.join("GTAV");
        for f in ["GTA5_Enhanced.exe", "PlayGTAV.exe", "ScriptHookV.dll", "dinput8.dll", "BattlEye/BEClient_x64.dll"] {
            touch(&gta.join(f));
        }
        let g = detect(&gta);
        assert_eq!((g.engine, g.loaders.clone(), g.anti_cheat.clone(), g.exe.as_deref()), (Engine::Rage, vec![LoaderKind::ScriptHookV], vec!["BattlEye".to_string()], Some("GTA5_Enhanced.exe")));

        let mono = r.join("Lethal");
        for f in ["Lethal Company.exe", "UnityPlayer.dll", "UnityCrashHandler64.exe", "Lethal Company_Data/globalgamemanagers", "BepInEx/core/BepInEx.dll", "winhttp.dll"] {
            touch(&mono.join(f));
        }
        let m = detect(&mono);
        assert_eq!((m.engine, m.loaders.clone(), m.exe.as_deref()), (Engine::UnityMono, vec![LoaderKind::BepInEx], Some("Lethal Company.exe")));

        let il = r.join("Schedule");
        for f in ["Schedule I.exe", "UnityPlayer.dll", "GameAssembly.dll", "MelonLoader/net6/MelonLoader.dll", "version.dll", "EasyAntiCheat/Settings.json"] {
            touch(&il.join(f));
        }
        let i = detect(&il);
        assert_eq!((i.engine, i.loaders.clone(), i.anti_cheat.clone()), (Engine::UnityIl2cpp, vec![LoaderKind::MelonLoader], vec!["Easy Anti-Cheat".to_string()]));

        let ue = r.join("Palworld");
        for f in ["Palworld.exe", "Engine/Binaries/ThirdParty/x.dll", "Pal/Content/Paks/Pal-Windows.pak", "Pal/Binaries/Win64/Palworld-Win64-Shipping.exe", "Pal/Binaries/Win64/ue4ss/UE4SS.dll"] {
            touch(&ue.join(f));
        }
        let u = detect(&ue);
        assert_eq!((u.engine, u.paks.as_deref(), u.binaries.as_deref(), u.loaders.clone()), (Engine::Unreal, Some("Pal/Content/Paks"), Some("Pal/Binaries/Win64"), vec![LoaderKind::Ue4ss]));

        let gm = r.join("GM");
        touch(&gm.join("data.win"));
        touch(&gm.join("Game.exe"));
        assert_eq!(detect(&gm).engine, Engine::GameMaker);
        let sky = r.join("Skyrim");
        touch(&sky.join("Data/Skyrim.esm"));
        touch(&sky.join("SkyrimSE.exe"));
        assert_eq!(detect(&sky).engine, Engine::Creation);
        assert_eq!(detect(&r.join("missing")).engine, Engine::Unknown);
    }
}
