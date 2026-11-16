//! Games from the other stores, next to Steam and Epic: GOG (its own
//! registry records), Ubisoft Connect (its install records), EA and
//! Battle.net (their games' Windows uninstall entries) and Xbox / Game Pass
//! (the `XboxGames` folders on each drive, with the app's package from
//! Windows' app records so it starts the way the Xbox app does).

use std::path::{Path, PathBuf};

use super::library::{guess_exe, InstalledGame};
use super::{GameKind, Launch};

/// A game folder's start program: a "<name> Launcher.exe" first (Blizzard
/// games start through it), else the biggest program.
fn start_exe(dir: &Path) -> Option<PathBuf> {
    let launcher = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| p.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase().ends_with(" launcher.exe")));
    launcher.or_else(|| guess_exe(dir))
}

fn exe_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn game(key: String, name: String, source: &str, launch: Launch, exe: Option<PathBuf>, dir: Option<String>) -> InstalledGame {
    InstalledGame { key, name, source: source.into(), kind: GameKind::Custom, launch, process: exe.as_deref().map(exe_name).unwrap_or_default(), exe_path: exe.map(|p| p.to_string_lossy().into_owned()), install_dir: dir, profile_id: None }
}

/// One uninstall entry, as read from the registry (or a test).
#[derive(Debug, Clone, Default)]
pub struct UninstallEntry {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub location: String,
}

/// EA and Battle.net games from uninstall entries (not the launchers themselves).
pub fn from_uninstall(entries: &[UninstallEntry]) -> Vec<InstalledGame> {
    let mut out = Vec::new();
    for e in entries {
        let p = e.publisher.to_lowercase();
        let n = e.name.to_lowercase();
        let source = if p.contains("electronic arts") && !n.starts_with("ea app") && !n.contains("origin") {
            "ea"
        } else if p.contains("blizzard") && !n.starts_with("battle.net") {
            "battlenet"
        } else {
            continue;
        };
        let dir = Path::new(&e.location);
        if e.location.is_empty() || !dir.is_dir() {
            continue;
        }
        let exe = start_exe(dir);
        let launch = exe.as_ref().map(|x| Launch::Exe { path: x.to_string_lossy().into_owned(), args: String::new() }).unwrap_or_default();
        let id: String = e.id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').take(60).collect();
        out.push(game(format!("{source}:{id}"), e.name.clone(), source, launch, exe, Some(e.location.clone())));
    }
    out
}

/// An Xbox app's `MicrosoftGame.config`: (identity name, display name, executable, app id).
pub fn parse_ms_game_config(xml: &str) -> Option<(String, String, String, String)> {
    let attr = |tag: &str, name: &str| -> Option<String> {
        let at = xml.find(&format!("<{tag} "))?;
        let rest = &xml[at..];
        let end = rest.find('>')?;
        let el = &rest[..end];
        let k = el.find(&format!(" {name}=\""))? + name.len() + 3;
        Some(el[k..].split('"').next()?.to_string())
    };
    let identity = attr("Identity", "Name")?;
    let display = attr("ShellVisuals", "DefaultDisplayName").filter(|d| !d.starts_with("ms-resource")).unwrap_or_else(|| identity.clone());
    let exe = attr("Executable", "Name")?;
    let app = attr("Executable", "Id").unwrap_or_else(|| "Game".into());
    Some((identity, display, exe, app))
}

/// Xbox / Game Pass games under `<drive>\XboxGames`, started through their
/// app package (`packages` are Windows' installed package full names).
pub fn xbox(roots: &[PathBuf], packages: &[String]) -> Vec<InstalledGame> {
    let mut out = Vec::new();
    for root in roots {
        let Ok(rd) = std::fs::read_dir(root) else { continue };
        for e in rd.flatten() {
            let content = e.path().join("Content");
            let Ok(xml) = std::fs::read_to_string(content.join("MicrosoftGame.config")) else { continue };
            let Some((identity, name, exe, app)) = parse_ms_game_config(&xml) else { continue };
            // "Name_1.0.0.0_x64__8wekyb3d8bbwe" → family "Name_8wekyb3d8bbwe".
            let family = packages.iter().find(|p| p.starts_with(&format!("{identity}_"))).and_then(|p| p.rsplit_once("__").map(|(_, publisher)| format!("{identity}_{publisher}")));
            let exe_path = content.join(&exe);
            let ok = |s: &str| s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
            let launch = match family {
                Some(f) if ok(&f) && ok(&app) => Launch::Url { url: format!("shell:AppsFolder\\{f}!{app}") },
                _ if exe_path.is_file() => Launch::Exe { path: exe_path.to_string_lossy().into_owned(), args: String::new() },
                _ => Launch::None,
            };
            out.push(game(format!("xbox:{identity}"), name, "xbox", launch, Some(exe_path), Some(content.to_string_lossy().into_owned())));
        }
    }
    out
}

/// GOG games: (id, name, exe, folder) from GOG's records.
pub fn gog(records: &[(String, String, String, String)]) -> Vec<InstalledGame> {
    records
        .iter()
        .filter(|(id, name, _, dir)| !id.is_empty() && !name.is_empty() && Path::new(dir).is_dir())
        .map(|(id, name, exe, dir)| {
            let exe = Some(PathBuf::from(exe)).filter(|p| p.is_file()).or_else(|| guess_exe(Path::new(dir)));
            let launch = exe.as_ref().map(|x| Launch::Exe { path: x.to_string_lossy().into_owned(), args: String::new() }).unwrap_or_default();
            game(format!("gog:{id}"), name.clone(), "gog", launch, exe, Some(dir.clone()))
        })
        .collect()
}

/// Ubisoft Connect games: (id, folder).
pub fn ubisoft(records: &[(String, String)]) -> Vec<InstalledGame> {
    records
        .iter()
        .filter(|(id, dir)| id.chars().all(|c| c.is_ascii_digit()) && !id.is_empty() && Path::new(dir).is_dir())
        .map(|(id, dir)| {
            let name = Path::new(dir.trim_end_matches(['\\', '/'])).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| format!("Ubisoft game {id}"));
            game(format!("ubisoft:{id}"), name, "ubisoft", Launch::Url { url: format!("uplay://launch/{id}/0") }, guess_exe(Path::new(dir)), Some(dir.clone()))
        })
        .collect()
}

/// Every game from these stores on this PC.
pub fn scan() -> Vec<InstalledGame> {
    let mut out = Vec::new();
    out.extend(gog(&imp::gog_records()));
    out.extend(ubisoft(&imp::ubisoft_records()));
    out.extend(from_uninstall(&imp::uninstall_entries()));
    let drives: Vec<PathBuf> = (b'C'..=b'Z').map(|d| PathBuf::from(format!("{}:\\XboxGames", d as char))).filter(|p| p.is_dir()).collect();
    if !drives.is_empty() {
        out.extend(xbox(&drives, &imp::packages()));
    }
    out
}

#[cfg(windows)]
mod imp {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY};
    use winreg::RegKey;

    use super::UninstallEntry;

    pub fn gog_records() -> Vec<(String, String, String, String)> {
        let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(r"SOFTWARE\GOG.com\Games", KEY_READ | KEY_WOW64_32KEY) else { return Vec::new() };
        k.enum_keys()
            .flatten()
            .filter_map(|id| {
                let g = k.open_subkey(&id).ok()?;
                let s = |n: &str| g.get_value::<String, _>(n).unwrap_or_default();
                Some((id.clone(), s("gameName"), s("exe"), s("path")))
            })
            .collect()
    }

    pub fn ubisoft_records() -> Vec<(String, String)> {
        let Ok(k) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(r"SOFTWARE\Ubisoft\Launcher\Installs", KEY_READ | KEY_WOW64_32KEY) else { return Vec::new() };
        k.enum_keys().flatten().filter_map(|id| Some((id.clone(), k.open_subkey(&id).ok()?.get_value::<String, _>("InstallDir").ok()?))).collect()
    }

    pub fn uninstall_entries() -> Vec<UninstallEntry> {
        let mut out = Vec::new();
        const KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
        for (root, flags) in [(HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY), (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY), (HKEY_CURRENT_USER, 0)] {
            let Ok(k) = RegKey::predef(root).open_subkey_with_flags(KEY, KEY_READ | flags) else { continue };
            for id in k.enum_keys().flatten() {
                let Ok(e) = k.open_subkey(&id) else { continue };
                let s = |n: &str| e.get_value::<String, _>(n).unwrap_or_default();
                let publisher = s("Publisher");
                if publisher.is_empty() {
                    continue;
                }
                out.push(UninstallEntry { id: id.clone(), name: s("DisplayName"), publisher, location: s("InstallLocation").trim_matches('"').to_string() });
            }
        }
        out
    }

    pub fn packages() -> Vec<String> {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\Repository\Packages")
            .map(|k| k.enum_keys().flatten().collect())
            .unwrap_or_default()
    }
}

#[cfg(not(windows))]
mod imp {
    use super::UninstallEntry;
    pub fn gog_records() -> Vec<(String, String, String, String)> {
        Vec::new()
    }
    pub fn ubisoft_records() -> Vec<(String, String)> {
        Vec::new()
    }
    pub fn uninstall_entries() -> Vec<UninstallEntry> {
        Vec::new()
    }
    pub fn packages() -> Vec<String> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_other_stores() {
        let t = tempfile::tempdir().unwrap();
        let mk = |rel: &str, exe: &str, size: usize| {
            let d = t.path().join(rel);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join(exe), vec![0u8; size]).unwrap();
            d
        };
        // GOG
        let witcher = mk("GOG/Witcher", "witcher3.exe", 10);
        let g = gog(&[("1207664663".into(), "The Witcher 3".into(), witcher.join("witcher3.exe").to_string_lossy().into(), witcher.to_string_lossy().into())]);
        assert_eq!((g[0].key.as_str(), g[0].process.as_str()), ("gog:1207664663", "witcher3.exe"));
        // EA and Battle.net through uninstall entries; the launchers themselves are skipped.
        let fc = mk("EA/FC", "FC25.exe", 50);
        let ow = mk("Blizzard/Overwatch", "Overwatch Launcher.exe", 5);
        std::fs::write(ow.join("Overwatch.exe"), vec![0u8; 500]).unwrap();
        let e = |id: &str, n: &str, p: &str, l: &Path| UninstallEntry { id: id.into(), name: n.into(), publisher: p.into(), location: l.to_string_lossy().into() };
        let found = from_uninstall(&[e("{FC}", "EA SPORTS FC 25", "Electronic Arts", &fc), e("Overwatch", "Overwatch", "Blizzard Entertainment", &ow), e("Battle.net", "Battle.net", "Blizzard Entertainment", &ow), e("x", "Notepad++", "Don Ho", &fc)]);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(found[1].process, "Overwatch Launcher.exe", "Blizzard games start through their launcher");
        // Xbox
        let content = t.path().join("XboxGames/Forza/Content");
        std::fs::create_dir_all(&content).unwrap();
        std::fs::write(content.join("MicrosoftGame.config"), r#"<?xml version="1.0"?><Game configVersion="1"><Identity Name="Microsoft.SunriseBaseGame" Publisher="CN=Microsoft" Version="1.0.0.0"/><ExecutableList><Executable Name="ForzaHorizon5.exe" Id="Game" /></ExecutableList><ShellVisuals DefaultDisplayName="Forza Horizon 5" /></Game>"#).unwrap();
        let x = xbox(&[t.path().join("XboxGames")], &["Microsoft.SunriseBaseGame_3.1.2.0_x64__8wekyb3d8bbwe".into()]);
        assert_eq!(x[0].name, "Forza Horizon 5");
        assert_eq!(x[0].launch, Launch::Url { url: r"shell:AppsFolder\Microsoft.SunriseBaseGame_8wekyb3d8bbwe!Game".into() });
        assert!(super::super::launch_target(&x[0].launch).is_ok());
        // Ubisoft
        let ac = mk("Ubisoft/Assassin's Creed Mirage", "ACMirage.exe", 9);
        let u = ubisoft(&[("5595".into(), ac.to_string_lossy().into()), ("bad id".into(), ac.to_string_lossy().into())]);
        assert_eq!((u.len(), u[0].name.as_str()), (1, "Assassin's Creed Mirage"));
    }
}
