//! The games installed on this PC, read from the launchers' own records:
//! Epic Games (install manifests), Steam (library folders and app
//! manifests), Riot (product settings), Roblox and Minecraft (Java). Each
//! comes with what a profile needs: how to start it and which program to
//! follow.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{GameKind, Launch};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledGame {
    /// Stable id: "<source>:<launcher's id>".
    pub key: String,
    pub name: String,
    /// epic, steam, riot, roblox, minecraft
    pub source: String,
    pub kind: GameKind,
    pub launch: Launch,
    /// The program to follow; empty when unknown.
    pub process: String,
    pub exe_path: Option<String>,
    pub install_dir: Option<String>,
    /// The profile already set up for it.
    pub profile_id: Option<String>,
}

/// Where to look (the real locations, or folders in tests).
#[derive(Debug, Clone, Default)]
pub struct Roots {
    pub epic_manifests: Option<PathBuf>,
    pub steam: Option<PathBuf>,
    pub riot_metadata: Option<PathBuf>,
    pub minecraft: Option<PathBuf>,
}

impl Roots {
    pub fn system() -> Roots {
        let program_data = std::env::var_os("ProgramData").map(PathBuf::from);
        Roots {
            epic_manifests: program_data.as_ref().map(|p| p.join(r"Epic\EpicGamesLauncher\Data\Manifests")),
            steam: steam_root(),
            riot_metadata: program_data.as_ref().map(|p| p.join(r"Riot Games\Metadata")),
            minecraft: dirs::config_dir().map(|d| d.join(".minecraft")),
        }
    }
}

#[cfg(windows)]
pub(crate) fn steam_root() -> Option<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    let p: String = winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey(r"Software\Valve\Steam").ok()?.get_value("SteamPath").ok()?;
    let p = PathBuf::from(p.replace('/', "\\"));
    p.is_dir().then_some(p)
}

#[cfg(not(windows))]
pub(crate) fn steam_root() -> Option<PathBuf> {
    None
}

/// Known games by Steam app id: (kind, program).
fn steam_known(app_id: u32) -> Option<(GameKind, &'static str)> {
    Some(match app_id {
        730 => (GameKind::Cs2, "cs2.exe"),
        1_172_470 => (GameKind::Apex, "r5apex.exe"),
        271_590 => (GameKind::Gta5, "GTA5.exe"),
        1_938_090 => (GameKind::CallOfDuty, "cod.exe"),
        252_950 => (GameKind::RocketLeague, "RocketLeague.exe"),
        2_357_570 => (GameKind::Overwatch, "Overwatch.exe"),
        _ => return None,
    })
}

/// Steam entries that are tools, not games.
fn steam_tool(app_id: u32, name: &str) -> bool {
    matches!(app_id, 228_980 | 1_070_560 | 1_391_110 | 1_628_350 | 250_820 | 1_826_330)
        || ["Redistributable", "Steamworks", "Proton", "Steam Linux Runtime", "SteamVR", "Dedicated Server", "Soundtrack", " SDK"].iter().any(|w| name.contains(w))
}

/// Every quoted "key" "value" pair in a Valve KeyValues file, in order.
fn vdf_pairs(text: &str) -> Vec<(String, String)> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut s = String::new();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => {
                        if let Some(n) = chars.next() {
                            s.push(n);
                        }
                    }
                    '"' => break,
                    c => s.push(c),
                }
            }
            tokens.push(Some(s));
        } else if c == '{' || c == '}' {
            tokens.push(None);
        }
    }
    // A string followed by a string is a pair; one followed by a brace names a block.
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        match (&tokens[i], tokens.get(i + 1)) {
            (Some(k), Some(Some(v))) => {
                out.push((k.clone(), v.clone()));
                i += 2;
            }
            _ => i += 1,
        }
    }
    out
}

/// A game's main program, guessed from its folder: the biggest .exe that
/// isn't a crash reporter, installer or anti-cheat service.
pub fn guess_exe(dir: &Path) -> Option<PathBuf> {
    const SKIP: &[&str] = &["crash", "unins", "setup", "redist", "vcredist", "dxsetup", "directx", "easyanticheat", "battleye", "beservice", "launcherhelper", "report", "updater", "dotnet", "vc_", "ue4prereq", "uninstall", "install", "helper", "cefprocess", "webhelper"];
    let mut best: Option<(u64, PathBuf)> = None;
    let mut stack = vec![(dir.to_path_buf(), 0)];
    while let Some((d, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let Ok(t) = e.file_type() else { continue };
            if t.is_dir() {
                // Unreal games keep it in Binaries\Win64; don't wander far.
                if depth < 4 {
                    stack.push((p, depth + 1));
                }
                continue;
            }
            let name = e.file_name().to_string_lossy().to_lowercase();
            if !name.ends_with(".exe") || SKIP.iter().any(|s| name.contains(s)) {
                continue;
            }
            let len = e.metadata().map(|m| m.len()).unwrap_or(0);
            if best.as_ref().is_none_or(|(l, _)| len > *l) {
                best = Some((len, p));
            }
        }
    }
    best.map(|(_, p)| p)
}

fn exe_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn epic(dir: &Path) -> Vec<InstalledGame> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        if e.path().extension().is_none_or(|x| x != "item") {
            continue;
        }
        let Ok(v) = std::fs::read(e.path()).map_err(|_| ()).and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).map_err(|_| ())) else { continue };
        let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_string();
        if v.get("bIsIncompleteInstall").and_then(|x| x.as_bool()) == Some(true) {
            continue;
        }
        let cats: Vec<String> = v.get("AppCategories").and_then(|c| c.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_lowercase)).collect()).unwrap_or_default();
        if !cats.is_empty() && !cats.iter().any(|c| c == "games") {
            continue;
        }
        let (app, name, dir) = (s("AppName"), s("DisplayName"), s("InstallLocation"));
        if app.is_empty() || name.is_empty() {
            continue;
        }
        let (ns, item) = (s("CatalogNamespace"), s("CatalogItemId"));
        let alnum = |x: &str| !x.is_empty() && x.chars().all(|c| c.is_ascii_alphanumeric());
        let launch_id = if alnum(&ns) && alnum(&item) && alnum(&app) { format!("{ns}%3A{item}%3A{app}") } else { app.clone() };
        let (kind, process, exe) = match app.as_str() {
            "Fortnite" => (GameKind::Fortnite, "FortniteClient-Win64-Shipping.exe".to_string(), Some(Path::new(&dir).join(r"FortniteGame\Binaries\Win64\FortniteClient-Win64-Shipping.exe"))),
            "Sugar" => (GameKind::RocketLeague, "RocketLeague.exe".to_string(), Some(Path::new(&dir).join(r"Binaries\Win64\RocketLeague.exe"))),
            _ => {
                let exe = Some(s("LaunchExecutable")).filter(|l| !l.is_empty()).map(|l| Path::new(&dir).join(l));
                (GameKind::Custom, exe.as_deref().map(exe_name).unwrap_or_default(), exe)
            }
        };
        let launch = if alnum(&launch_id.replace("%3A", "")) { Launch::Epic { app: launch_id } } else { Launch::None };
        out.push(InstalledGame { key: format!("epic:{app}"), name, source: "epic".into(), kind, launch, process, exe_path: exe.map(|p| p.to_string_lossy().into_owned()), install_dir: Some(dir), profile_id: None });
    }
    out
}

fn steam(root: &Path) -> Vec<InstalledGame> {
    let mut libraries = vec![root.to_path_buf()];
    if let Ok(t) = std::fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")) {
        for (k, v) in vdf_pairs(&t) {
            if k == "path" {
                let p = PathBuf::from(&v);
                if !libraries.contains(&p) {
                    libraries.push(p);
                }
            }
        }
    }
    let mut out = Vec::new();
    for lib in libraries {
        let apps = lib.join("steamapps");
        let Ok(rd) = std::fs::read_dir(&apps) else { continue };
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if !(n.starts_with("appmanifest_") && n.ends_with(".acf")) {
                continue;
            }
            let Ok(t) = std::fs::read_to_string(e.path()) else { continue };
            let pairs = vdf_pairs(&t);
            let get = |k: &str| pairs.iter().find(|(a, _)| a.eq_ignore_ascii_case(k)).map(|(_, v)| v.clone());
            let (Some(id), Some(name), Some(dir)) = (get("appid").and_then(|v| v.parse::<u32>().ok()), get("name"), get("installdir")) else { continue };
            if steam_tool(id, &name) {
                continue;
            }
            let install = apps.join("common").join(&dir);
            if !install.is_dir() {
                continue;
            }
            let (kind, process, exe) = match steam_known(id) {
                Some((k, p)) => (k, p.to_string(), None),
                None => {
                    let exe = guess_exe(&install);
                    (GameKind::Custom, exe.as_deref().map(exe_name).unwrap_or_default(), exe)
                }
            };
            out.push(InstalledGame { key: format!("steam:{id}"), name, source: "steam".into(), kind, launch: Launch::Steam { app_id: id }, process, exe_path: exe.map(|p| p.to_string_lossy().into_owned()), install_dir: Some(install.to_string_lossy().into_owned()), profile_id: None });
        }
    }
    out
}

fn riot(metadata: &Path) -> Vec<InstalledGame> {
    let mut out = Vec::new();
    for (product, kind, name, process) in [("valorant", GameKind::Valorant, "VALORANT", "VALORANT-Win64-Shipping.exe"), ("league_of_legends", GameKind::League, "League of Legends", "League of Legends.exe")] {
        let file = metadata.join(format!("{product}.live")).join(format!("{product}.live.product_settings.yaml"));
        let Ok(t) = std::fs::read_to_string(&file) else { continue };
        let dir = t.lines().find_map(|l| l.trim().strip_prefix("product_install_full_path:")).map(|v| v.trim().trim_matches('"').replace('/', std::path::MAIN_SEPARATOR_STR));
        if dir.as_deref().is_some_and(|d| !Path::new(d).is_dir()) {
            continue;
        }
        out.push(InstalledGame { key: format!("riot:{product}"), name: name.into(), source: "riot".into(), kind, launch: Launch::Riot { product: product.into() }, process: process.into(), exe_path: None, install_dir: dir, profile_id: None });
    }
    out
}

fn minecraft(dir: &Path) -> Vec<InstalledGame> {
    if !(dir.join("options.txt").is_file() || dir.join("launcher_profiles.json").is_file()) {
        return Vec::new();
    }
    let launcher = [r"C:\Program Files (x86)\Minecraft Launcher\MinecraftLauncher.exe", r"C:\Program Files\Minecraft Launcher\MinecraftLauncher.exe"].iter().map(PathBuf::from).find(|p| p.is_file());
    let launch = launcher.map(|p| Launch::Exe { path: p.to_string_lossy().into_owned(), args: String::new() }).unwrap_or_default();
    vec![InstalledGame { key: "minecraft:java".into(), name: "Minecraft".into(), source: "minecraft".into(), kind: GameKind::Minecraft, launch, process: "javaw.exe".into(), exe_path: None, install_dir: Some(dir.to_string_lossy().into_owned()), profile_id: None }]
}

/// Every game found, sorted by name.
pub fn scan(roots: &Roots, roblox_player: Option<String>) -> Vec<InstalledGame> {
    let mut out = Vec::new();
    if let Some(d) = &roots.epic_manifests {
        out.extend(epic(d));
    }
    if let Some(d) = &roots.steam {
        out.extend(steam(d));
    }
    if let Some(d) = &roots.riot_metadata {
        out.extend(riot(d));
    }
    if let Some(d) = &roots.minecraft {
        out.extend(minecraft(d));
    }
    if let Some(p) = roblox_player {
        out.push(InstalledGame { key: "roblox:player".into(), name: "Roblox".into(), source: "roblox".into(), kind: GameKind::Roblox, launch: Launch::Roblox { place_id: None }, process: super::roblox::PLAYER_EXE.into(), exe_path: Some(p), install_dir: None, profile_id: None });
    }
    out.sort_by_key(|g| g.name.to_lowercase());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, s: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, s).unwrap();
    }

    #[test]
    fn finds_games_in_launcher_records() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        // Epic: Fortnite, another game, and the Unreal editor (not a game).
        let epic = root.join("epic");
        let fn_dir = root.join("Games").join("Fortnite");
        write(&epic.join("a.item"), &serde_json::json!({"AppName": "Fortnite", "DisplayName": "Fortnite", "CatalogNamespace": "fn", "CatalogItemId": "4fe75bbc5a674f4f9b356b5c90567da5", "InstallLocation": fn_dir, "LaunchExecutable": "FortniteGame/Binaries/Win64/FortniteLauncher.exe", "AppCategories": ["public", "games", "applications"]}).to_string());
        write(&epic.join("b.item"), &serde_json::json!({"AppName": "Kinglet", "DisplayName": "Skate Park", "CatalogNamespace": "ns1", "CatalogItemId": "abc", "InstallLocation": root.join("Games").join("Skate"), "LaunchExecutable": "Skate.exe", "AppCategories": ["games"]}).to_string());
        write(&epic.join("c.item"), &serde_json::json!({"AppName": "UE_5.4", "DisplayName": "Unreal Engine 5.4", "InstallLocation": "x", "AppCategories": ["engines"]}).to_string());
        write(&epic.join("d.item"), &serde_json::json!({"AppName": "Half", "DisplayName": "Half", "bIsIncompleteInstall": true}).to_string());
        // Steam: two libraries, CS2 in the second, a tool and a game with an unknown program.
        let steam = root.join("Steam");
        let lib2 = root.join("SteamLibrary");
        write(&steam.join("steamapps").join("libraryfolders.vdf"), &format!("\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t\t\"apps\" {{ \"730\" \"123\" }}\n\t}}\n}}\n", steam.display().to_string().replace('\\', "\\\\"), lib2.display().to_string().replace('\\', "\\\\")));
        write(&lib2.join("steamapps").join("appmanifest_730.acf"), "\"AppState\"\n{\n\t\"appid\"\t\t\"730\"\n\t\"name\"\t\t\"Counter-Strike 2\"\n\t\"installdir\"\t\t\"Counter-Strike Global Offensive\"\n}\n");
        std::fs::create_dir_all(lib2.join("steamapps").join("common").join("Counter-Strike Global Offensive")).unwrap();
        write(&steam.join("steamapps").join("appmanifest_228980.acf"), "\"AppState\" { \"appid\" \"228980\" \"name\" \"Steamworks Common Redistributables\" \"installdir\" \"Steamworks Shared\" }");
        std::fs::create_dir_all(steam.join("steamapps").join("common").join("Steamworks Shared")).unwrap();
        write(&steam.join("steamapps").join("appmanifest_999.acf"), "\"AppState\" { \"appid\" \"999\" \"name\" \"Indie \\\"Quoted\\\" Game\" \"installdir\" \"Indie\" }");
        let indie = steam.join("steamapps").join("common").join("Indie");
        write(&indie.join("UnityCrashHandler64.exe"), &"x".repeat(5000));
        write(&indie.join("Indie.exe"), &"x".repeat(900));
        write(&indie.join("Bin").join("Engine.exe"), &"x".repeat(2000));
        // Riot and Minecraft.
        let riot = root.join("Riot");
        let val = root.join("Riot Games").join("VALORANT").join("live");
        std::fs::create_dir_all(&val).unwrap();
        write(&riot.join("valorant.live").join("valorant.live.product_settings.yaml"), &format!("product_install_full_path: \"{}\"\nother: 1\n", val.display().to_string().replace('\\', "/")));
        let mc = root.join(".minecraft");
        write(&mc.join("options.txt"), "maxFps:120\n");

        let games = scan(&Roots { epic_manifests: Some(epic), steam: Some(steam), riot_metadata: Some(riot), minecraft: Some(mc) }, Some("C:/Roblox/RobloxPlayerBeta.exe".into()));
        let names: Vec<&str> = games.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, vec!["Counter-Strike 2", "Fortnite", "Indie \"Quoted\" Game", "Minecraft", "Roblox", "Skate Park", "VALORANT"]);
        let f = games.iter().find(|g| g.kind == GameKind::Fortnite).unwrap();
        assert_eq!(f.launch, Launch::Epic { app: "fn%3A4fe75bbc5a674f4f9b356b5c90567da5%3AFortnite".into() });
        assert_eq!(f.process, "FortniteClient-Win64-Shipping.exe");
        let skate = games.iter().find(|g| g.key == "epic:Kinglet").unwrap();
        assert_eq!((skate.kind, skate.process.as_str()), (GameKind::Custom, "Skate.exe"));
        let cs = games.iter().find(|g| g.key == "steam:730").unwrap();
        assert_eq!((cs.kind, cs.process.as_str()), (GameKind::Cs2, "cs2.exe"));
        let indie_game = games.iter().find(|g| g.key == "steam:999").unwrap();
        assert_eq!(indie_game.process, "Engine.exe", "biggest program, crash handler skipped");
        assert_eq!(games.iter().find(|g| g.key == "riot:valorant").unwrap().launch, Launch::Riot { product: "valorant".into() });
        assert_eq!(games.iter().find(|g| g.kind == GameKind::Minecraft).unwrap().process, "javaw.exe");
    }
}
