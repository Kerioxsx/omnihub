//! Modpacks other launchers already installed on this PC: Prism Launcher (and
//! PolyMC and MultiMC, which it grew from), CurseForge, the Modrinth App and
//! the Minecraft Launcher's own folder; what's in each (mods from their jars'
//! own descriptions, single-player worlds from their level.dat). Their
//! folders are only read, never changed; the Modrinth App's database is
//! opened read-only.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Launcher {
    Prism,
    Curseforge,
    Modrinth,
    /// Mojang's Minecraft Launcher (.minecraft).
    Minecraft,
}

/// A modpack (an instance) another launcher installed.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    /// The same for the same folder (a hash of where it is).
    pub id: String,
    pub launcher: Launcher,
    /// "Prism Launcher", "PolyMC", "MultiMC", "CurseForge" or "Modrinth App".
    pub launcher_name: String,
    pub name: String,
    /// The instance's own folder.
    pub path: String,
    /// The game's folder, with mods/ and config/ (`.minecraft` inside Prism's).
    pub game_dir: String,
    pub mc: String,
    /// "fabric", "quilt", "forge" or "neoforge"; empty for plain Minecraft.
    pub loader: String,
    pub loader_version: String,
    pub mods: usize,
    /// Turned off in the launcher (`.jar.disabled`).
    pub disabled: usize,
    /// The modpack it was installed from, with its version.
    pub pack: Option<String>,
    /// Unix seconds.
    pub last_played: Option<i64>,
    pub has_icon: bool,
    #[serde(skip)]
    pub icon: Option<PathBuf>,
}

/// Where to look.
#[derive(Debug, Clone, Default)]
pub struct Places {
    /// %AppData% (Roaming): PrismLauncher, PolyMC, ModrinthApp.
    pub roaming: Option<PathBuf>,
    /// The user's folder: curseforge\minecraft\Instances, Documents\Curse.
    pub home: Option<PathBuf>,
    /// Folders the user added: a launcher's folder, its instances folder or one instance.
    pub extra: Vec<PathBuf>,
}

const PRISM_CFG: &[(&str, &str)] = &[("prismlauncher.cfg", "Prism Launcher"), ("polymc.cfg", "PolyMC"), ("multimc.cfg", "MultiMC")];

/// Every installed modpack found, most recently played first.
pub fn find(p: &Places) -> Vec<Instance> {
    let mut out = Vec::new();
    if let Some(r) = &p.roaming {
        prism_data(&r.join("PrismLauncher"), "Prism Launcher", &mut out);
        prism_data(&r.join("PolyMC"), "PolyMC", &mut out);
        minecraft_launcher(&r.join(".minecraft"), &mut out);
        let app = r.join("ModrinthApp");
        if app.join("app.db").is_file() {
            modrinth_app(&app, &mut out);
        } else {
            modrinth_legacy(&r.join("com.modrinth.theseus").join("profiles"), &mut out);
        }
    }
    if let Some(h) = &p.home {
        scan(&h.join("curseforge").join("minecraft").join("Instances"), "", None, &mut out);
        for docs in [h.join("Documents"), h.join("OneDrive").join("Documents")] {
            scan(&docs.join("Curse").join("Minecraft").join("Instances"), "", None, &mut out);
        }
    }
    for e in &p.extra {
        added(e, &mut out);
    }
    // One entry per game folder, also when it's found two ways.
    let mut seen = HashSet::new();
    out.retain(|i| seen.insert(i.id.clone()));
    out.sort_by(|a, b| b.last_played.unwrap_or(0).cmp(&a.last_played.unwrap_or(0)).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    out
}

/// The instance's picture (at most 8 MB, and one that can be shown).
pub fn icon(i: &Instance) -> Option<Vec<u8>> {
    let b = read_small(i.icon.as_deref()?, 8 << 20)?;
    super::cover::decode(&b)?;
    Some(b)
}

/// A folder the user added: one instance, a launcher's folder, or an instances folder.
fn added(d: &Path, out: &mut Vec<Instance>) {
    if d.join("instance.cfg").is_file() {
        let icons = d.parent().and_then(Path::parent).map(|p| p.join("icons"));
        out.extend(prism_instance(d, prism_label(d.parent().and_then(Path::parent)), icons.as_deref()));
    } else if d.join("minecraftinstance.json").is_file() {
        out.extend(curseforge_instance(d));
    } else if d.join("app.db").is_file() {
        modrinth_app(d, out);
    } else if PRISM_CFG.iter().any(|(c, _)| d.join(c).is_file()) {
        prism_data(d, prism_label(Some(d)), out);
    } else if let Some(inner) = ["instances", "Instances"].iter().map(|n| d.join(n)).find(|p| p.is_dir()) {
        // A launcher's folder (Prism's portable one, CurseForge's minecraft folder).
        scan(&inner, prism_label(Some(d)), Some(&d.join("icons")), out);
    } else if d.join("profiles").is_dir() {
        modrinth_legacy(&d.join("profiles"), out);
    } else {
        scan(d, prism_label(d.parent()), d.parent().map(|p| p.join("icons")).as_deref(), out);
    }
}

fn prism_label(data: Option<&Path>) -> &'static str {
    data.and_then(|d| PRISM_CFG.iter().find(|(c, _)| d.join(c).is_file())).map(|(_, l)| *l).unwrap_or("Prism Launcher")
}

/// Prism Launcher's (or PolyMC's) own folder: its instances where its settings say.
fn prism_data(d: &Path, label: &str, out: &mut Vec<Instance>) {
    if !d.is_dir() {
        return;
    }
    let cfg = PRISM_CFG.iter().find_map(|(c, _)| std::fs::read_to_string(d.join(c)).ok()).map(|t| ini(&t)).unwrap_or_default();
    let at = |key: &str, default: &str| {
        let v = cfg.get(key).filter(|v| !v.is_empty()).map(String::as_str).unwrap_or(default);
        let p = PathBuf::from(v);
        if p.is_absolute() {
            p
        } else {
            d.join(p)
        }
    };
    scan(&at("InstanceDir", "instances"), label, Some(&at("IconsDir", "icons")), out);
}

/// An instances folder: each instance in it, Prism's or CurseForge's.
fn scan(d: &Path, prism: &str, icons: Option<&Path>, out: &mut Vec<Instance>) {
    for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        if p.join("instance.cfg").is_file() {
            out.extend(prism_instance(&p, if prism.is_empty() { "Prism Launcher" } else { prism }, icons));
        } else if p.join("minecraftinstance.json").is_file() {
            out.extend(curseforge_instance(&p));
        }
    }
}

/// `key=value` lines of a Qt settings file (Prism's instance.cfg and prismlauncher.cfg).
fn ini(text: &str) -> HashMap<String, String> {
    text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim().to_string(), unquote(v.trim()))).collect()
}

fn unquote(v: &str) -> String {
    let Some(inner) = v.strip_prefix('"').and_then(|v| v.strip_suffix('"')) else { return v.to_string() };
    let mut s = String::new();
    let mut it = inner.chars();
    while let Some(c) = it.next() {
        match (c, c == '\\') {
            (_, true) => match it.next() {
                Some('n') => s.push('\n'),
                Some('t') => s.push('\t'),
                Some(o) => s.push(o),
                None => {}
            },
            (c, false) => s.push(c),
        }
    }
    s
}

fn prism_instance(dir: &Path, label: &str, icons: Option<&Path>) -> Option<Instance> {
    let cfg = ini(&std::fs::read_to_string(dir.join("instance.cfg")).ok()?);
    let pack: Value = std::fs::read(dir.join("mmc-pack.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    let (mut mc, mut loader, mut lv) = (String::new(), String::new(), String::new());
    for c in pack["components"].as_array().into_iter().flatten() {
        let v = c["version"].as_str().or(c["cachedVersion"].as_str()).unwrap_or_default().to_string();
        let l = match c["uid"].as_str().unwrap_or_default() {
            "net.minecraft" => {
                mc = v;
                continue;
            }
            "net.fabricmc.fabric-loader" => "fabric",
            "org.quiltmc.quilt-loader" => "quilt",
            "net.minecraftforge" => "forge",
            "net.neoforged" => "neoforge",
            _ => continue,
        };
        (loader, lv) = (l.to_string(), v);
    }
    let game = [".minecraft", "minecraft"].iter().map(|n| dir.join(n)).find(|p| p.is_dir()).unwrap_or_else(|| dir.join(".minecraft"));
    let managed = cfg.get("ManagedPack").is_some_and(|v| v == "true");
    let pack = managed.then(|| both(cfg.get("ManagedPackName"), cfg.get("ManagedPackVersionName"))).flatten();
    let last = cfg.get("lastLaunchTime").and_then(|t| t.parse::<i64>().ok()).filter(|t| *t > 0).map(|t| t / 1000);
    let icon = cfg.get("iconKey").filter(|k| safe_key(k)).and_then(|k| icons.and_then(|d| picture_named(d, k)));
    let name = cfg.get("name").cloned().unwrap_or_default();
    Some(make(Launcher::Prism, label, name, dir, &game, mc, loader, lv, pack, last, icon))
}

fn curseforge_instance(dir: &Path) -> Option<Instance> {
    let v: Value = serde_json::from_slice(&read_small(&dir.join("minecraftinstance.json"), 64 << 20)?).ok()?;
    let ml = &v["baseModLoader"];
    let mc = v["gameVersion"].as_str().or(ml["minecraftVersion"].as_str()).unwrap_or_default().to_string();
    // "forge-47.2.0", "neoforge-21.1.77", "fabric-0.15.11-1.20.1"
    let lname = ml["name"].as_str().unwrap_or_default();
    let by_name = ["neoforge", "forge", "fabric", "quilt"].into_iter().find(|l| lname.starts_with(&format!("{l}-")));
    let by_type = match ml["type"].as_i64() {
        Some(1) => Some("forge"),
        Some(4) => Some("fabric"),
        Some(5) => Some("quilt"),
        Some(6) => Some("neoforge"),
        _ => None,
    };
    let loader = by_name.or(by_type).unwrap_or_default().to_string();
    let named = by_name.map(|l| lname[l.len() + 1..].to_string()).map(|r| r.strip_suffix(&format!("-{mc}")).map(String::from).unwrap_or(r));
    let lv = named.or_else(|| ml["forgeVersion"].as_str().map(String::from)).unwrap_or_default();
    let pack = both(v["installedModpack"]["name"].as_str().or(v["manifest"]["name"].as_str()).map(String::from).as_ref(), v["manifest"]["version"].as_str().map(String::from).as_ref());
    let last = v["lastPlayed"].as_str().and_then(when);
    let icon = v["profileImagePath"].as_str().filter(|p| !p.is_empty()).map(PathBuf::from).filter(|p| p.is_file());
    let name = v["name"].as_str().unwrap_or_default().to_string();
    Some(make(Launcher::Curseforge, "CurseForge", name, dir, dir, mc, loader, lv, pack, last, icon))
}

/// CurseForge's times: "2024-05-01T18:22:31.1234567Z", sometimes with no zone; year 1 for never.
fn when(t: &str) -> Option<i64> {
    let s = chrono::DateTime::parse_from_rfc3339(t).map(|d| d.timestamp()).ok().or_else(|| chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S%.f").ok().map(|d| d.and_utc().timestamp()))?;
    (s > 0).then_some(s)
}

struct MrRow {
    path: String,
    name: String,
    icon: Option<String>,
    last: Option<i64>,
    mc: String,
    loader: String,
    lv: Option<String>,
    stage: String,
    pack: Option<String>,
}

/// The Modrinth App: its instances are listed in app.db (read-only); their
/// folders are in profiles\, where its settings say.
fn modrinth_app(settings: &Path, out: &mut Vec<Instance>) {
    let Some(db) = open_db(&settings.join("app.db")) else { return };
    let custom: Option<String> = db.query_row("SELECT custom_dir FROM settings LIMIT 1", [], |r| r.get(0)).ok().flatten();
    let root = custom.map(PathBuf::from).filter(|p| p.is_dir()).unwrap_or_else(|| settings.to_path_buf()).join("profiles");
    let table = |t: &str| db.query_row("SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1", [t], |r| r.get::<_, i64>(0)).unwrap_or(0) > 0;
    let rows = if table("instances") {
        // Since mid-2026: instances, with their version and loader in the content set they use.
        let skip = if table("instance_quarantines") { " WHERE i.id NOT IN (SELECT instance_id FROM instance_quarantines)" } else { "" };
        let q = format!("SELECT i.id, i.path, i.name, i.icon_path, i.last_played, c.game_version, c.loader, c.loader_version, i.install_stage FROM instances i LEFT JOIN instance_content_sets c ON c.id = i.applied_content_set_id{skip}");
        let mut rows = query(&db, &q, |r| Ok((r.get::<_, String>(0)?, row(r, 1)?)));
        // The pack it was imported from, where the app keeps it.
        if let Ok(mut st) = db.prepare("SELECT imported_name, imported_version_number FROM instance_links WHERE instance_id = ?1") {
            for (id, r) in &mut rows {
                r.pack = st.query_row([&*id], |x| Ok(both(x.get::<_, Option<String>>(0)?.as_ref(), x.get::<_, Option<String>>(1)?.as_ref()))).ok().flatten();
            }
        }
        rows.into_iter().map(|(_, r)| r).collect()
    } else {
        query(&db, "SELECT path, name, icon_path, last_played, game_version, mod_loader, mod_loader_version, install_stage FROM profiles", |r| row(r, 0))
    };
    for r in rows.into_iter().filter(|r| !r.stage.contains("installing") && !r.path.is_empty()) {
        let Some(rel) = super::install::safe_rel(&r.path) else { continue };
        let dir = root.join(rel);
        if !dir.is_dir() {
            continue;
        }
        let loader = if r.loader == "vanilla" { String::new() } else { r.loader };
        out.push(make(Launcher::Modrinth, "Modrinth App", r.name, &dir, &dir, r.mc, loader, r.lv.unwrap_or_default(), r.pack, r.last.filter(|t| *t > 0), r.icon.map(PathBuf::from)));
    }
}

fn row(r: &rusqlite::Row, at: usize) -> rusqlite::Result<MrRow> {
    Ok(MrRow {
        path: r.get(at)?,
        name: r.get(at + 1)?,
        icon: r.get(at + 2)?,
        last: r.get(at + 3)?,
        mc: r.get::<_, Option<String>>(at + 4)?.unwrap_or_default(),
        loader: r.get::<_, Option<String>>(at + 5)?.unwrap_or_default(),
        lv: r.get(at + 6)?,
        stage: r.get::<_, Option<String>>(at + 7)?.unwrap_or_default(),
        pack: None,
    })
}

fn query<T>(db: &rusqlite::Connection, q: &str, f: impl FnMut(&rusqlite::Row) -> rusqlite::Result<T>) -> Vec<T> {
    let Ok(mut st) = db.prepare(q) else { return vec![] };
    st.query_map([], f).map(|it| it.flatten().collect()).unwrap_or_default()
}

/// Opened without changing anything beside it: while the Modrinth App runs
/// (its -wal and -shm files are there) as one more reader; when it doesn't,
/// as an unchanging file, so SQLite makes no -wal or -shm files of its own.
fn open_db(p: &Path) -> Option<rusqlite::Connection> {
    use rusqlite::OpenFlags;
    let beside = |x: &str| {
        let mut s = p.as_os_str().to_owned();
        s.push(x);
        Path::new(&s).is_file()
    };
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let running = beside("-wal") && beside("-shm");
    let db = if running { None } else { rusqlite::Connection::open_with_flags(format!("{}?immutable=1", file_uri(p)), flags | OpenFlags::SQLITE_OPEN_URI).ok() };
    let db = match db {
        Some(db) => db,
        None => rusqlite::Connection::open_with_flags(p, flags).ok()?,
    };
    let _ = db.busy_timeout(std::time::Duration::from_secs(2));
    db.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0)).ok()?;
    Some(db)
}

/// `file:///C:/Users/…/app.db`, with what a URI can't hold escaped.
fn file_uri(p: &Path) -> String {
    use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
    const PATH: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'#').add(b'%').add(b'?').add(b'<').add(b'>').add(b'`').add(b'{').add(b'}');
    let s = p.to_string_lossy().replace('\\', "/");
    let lead = if s.starts_with('/') { "" } else { "/" };
    format!("file://{lead}{}", utf8_percent_encode(&s, PATH))
}

/// The Modrinth App before 2024 (com.modrinth.theseus): a profile.json in each profile.
fn modrinth_legacy(profiles: &Path, out: &mut Vec<Instance>) {
    for e in std::fs::read_dir(profiles).into_iter().flatten().flatten() {
        let dir = e.path();
        let Some(v) = read_small(&dir.join("profile.json"), 16 << 20).and_then(|b| serde_json::from_slice::<Value>(&b).ok()) else { continue };
        let m = &v["metadata"];
        let loader = m["loader"].as_str().filter(|l| *l != "vanilla").unwrap_or_default().to_string();
        let last = m["last_played"].as_str().and_then(when);
        let icon = m["icon"].as_str().or(v["icon_path"].as_str()).map(PathBuf::from);
        out.push(make(Launcher::Modrinth, "Modrinth App", m["name"].as_str().unwrap_or_default().to_string(), &dir, &dir, m["game_version"].as_str().unwrap_or_default().to_string(), loader, m["loader_version"]["id"].as_str().unwrap_or_default().to_string(), None, last, icon));
    }
}

/// The Minecraft Launcher's own folder, when it has worlds or mods: its
/// version and loader from the profile played last that uses that folder.
fn minecraft_launcher(dir: &Path, out: &mut Vec<Instance>) {
    let has_worlds = std::fs::read_dir(dir.join("saves")).into_iter().flatten().flatten().any(|e| e.path().join("level.dat").is_file());
    if !has_worlds && count_mods(&dir.join("mods")).0 == 0 {
        return;
    }
    let profiles: Value = read_small(&dir.join("launcher_profiles.json"), 16 << 20).and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(Value::Null);
    let last = profiles["profiles"].as_object().into_iter().flat_map(|m| m.values()).filter(|p| p["gameDir"].as_str().is_none_or(|g| g.is_empty() || Path::new(g) == dir)).filter_map(|p| Some((when(p["lastUsed"].as_str()?)?, p["lastVersionId"].as_str()?.to_string()))).max_by_key(|(t, _)| *t);
    let (mc, loader, lv) = last.as_ref().map(|(_, v)| version_parts(v)).unwrap_or_default();
    out.push(make(Launcher::Minecraft, "Minecraft Launcher", "Minecraft Launcher".into(), dir, dir, mc, loader, lv, None, last.map(|(t, _)| t), None));
}

/// A launcher version id as (Minecraft, loader, loader version):
/// "1.21.4", "fabric-loader-0.16.9-1.21.4", "quilt-loader-0.26.0-1.20.4",
/// "1.20.1-forge-47.2.0", "neoforge-21.1.77" (Minecraft 1.21.1).
fn version_parts(v: &str) -> (String, String, String) {
    let three = |a: &str, b: &str, c: &str| (a.to_string(), b.to_string(), c.to_string());
    for (prefix, loader) in [("fabric-loader-", "fabric"), ("quilt-loader-", "quilt")] {
        if let Some((lv, mc)) = v.strip_prefix(prefix).and_then(|r| r.split_once('-')) {
            return three(mc, loader, lv);
        }
    }
    if let Some((mc, lv)) = v.split_once("-forge-") {
        return three(mc, "forge", lv);
    }
    if let Some(lv) = v.strip_prefix("neoforge-") {
        // NeoForge 21.1.x is for Minecraft 1.21.1; 26.1.2.x for 26.1.2.
        let parts: Vec<&str> = lv.split('.').collect();
        let mc = match parts.as_slice() {
            [major, minor, ..] if major.parse::<u32>().is_ok_and(|m| m < 25) => if *minor == "0" { format!("1.{major}") } else { format!("1.{major}.{minor}") },
            [a, b, c, _, ..] => format!("{a}.{b}.{c}"),
            _ => String::new(),
        };
        return three(&mc, "neoforge", lv);
    }
    if v.starts_with("latest-") || !v.starts_with(|c: char| c.is_ascii_digit()) {
        return Default::default();
    }
    three(v, "", "")
}

/// A single-player world in an instance's saves folder.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct World {
    /// Its folder in saves\.
    pub folder: String,
    pub name: String,
    /// The Minecraft version it was last played in.
    pub version: String,
    /// Unix seconds.
    pub last_played: Option<i64>,
    /// "survival", "creative", "adventure" or "spectator".
    pub game_mode: String,
    pub hardcore: bool,
    /// "peaceful", "easy", "normal" or "hard"; empty when the world doesn't say.
    pub difficulty: String,
    pub cheats: bool,
    pub size: u64,
    /// It keeps the single player's things (inventory, place), which a server can take over.
    pub has_player: bool,
    pub has_icon: bool,
}

/// The worlds in a game folder's saves, most recently played first.
pub fn worlds(game: &Path) -> Vec<World> {
    let mut v: Vec<World> = std::fs::read_dir(game.join("saves")).into_iter().flatten().flatten().filter_map(|e| world(&e.path())).collect();
    v.sort_by(|a, b| b.last_played.unwrap_or(0).cmp(&a.last_played.unwrap_or(0)).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    v
}

pub fn world(dir: &Path) -> Option<World> {
    use crate::mcserver::nbt;
    let folder = dir.file_name()?.to_string_lossy().into_owned();
    let level = nbt::read(&read_small(&dir.join("level.dat"), 64 << 20)?).ok()?;
    let d = level.get("Data")?;
    let int = |k: &str| d.get(k).and_then(nbt::Tag::int);
    let game_mode = ["survival", "creative", "adventure", "spectator"].get(int("GameType").unwrap_or(0).clamp(0, 3) as usize).copied().unwrap_or("survival");
    let difficulty = int("Difficulty").and_then(|x| ["peaceful", "easy", "normal", "hard"].get(x.clamp(0, 3) as usize).copied()).unwrap_or_default();
    let name = d.get("LevelName").and_then(nbt::Tag::str).filter(|n| !n.trim().is_empty()).map(String::from).unwrap_or_else(|| folder.clone());
    Some(World {
        name,
        version: d.get("Version").and_then(|v| v.get("Name")).and_then(nbt::Tag::str).unwrap_or_default().to_string(),
        last_played: int("LastPlayed").filter(|t| *t > 0).map(|t| t / 1000),
        game_mode: game_mode.into(),
        hardcore: int("hardcore").unwrap_or(0) != 0,
        difficulty: difficulty.into(),
        cheats: int("allowCommands").unwrap_or(0) != 0,
        size: walkdir::WalkDir::new(dir).follow_links(false).into_iter().flatten().filter_map(|e| e.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum(),
        has_player: d.get("Player").is_some(),
        has_icon: dir.join("icon.png").is_file(),
        folder,
    })
}

/// A world folder in an instance, by its name in saves (one plain name, no path).
pub fn world_dir(game: &Path, folder: &str) -> Option<PathBuf> {
    let ok = !folder.is_empty() && super::install::safe_rel(folder).is_some_and(|r| r == folder && !r.contains('/'));
    Some(game.join("saves").join(folder)).filter(|d| ok && d.join("level.dat").is_file())
}

/// A mod in an instance, as its jar describes itself.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModInfo {
    pub file: String,
    pub id: String,
    pub name: String,
    pub version: String,
    /// "client" (only in the game itself), "server", or empty (both, or it doesn't say).
    pub side: String,
    /// Off in the launcher (.jar.disabled).
    pub disabled: bool,
    pub size: u64,
}

/// The mods in a game folder, by name.
pub fn mods_in(game: &Path) -> Vec<ModInfo> {
    let mut v: Vec<ModInfo> = std::fs::read_dir(game.join("mods"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let file = e.file_name().to_string_lossy().into_owned();
            let low = file.to_lowercase();
            let disabled = low.ends_with(".jar.disabled");
            if !disabled && !low.ends_with(".jar") {
                return None;
            }
            let size = e.metadata().ok().filter(|m| m.is_file())?.len();
            let (id, name, version, side) = jar_meta(&e.path()).unwrap_or_default();
            let fallback = file.trim_end_matches(".disabled").trim_end_matches(".jar").to_string();
            Some(ModInfo { name: if name.is_empty() { fallback } else { name }, file, id, version, side, disabled, size })
        })
        .collect();
    v.sort_by_key(|a| a.name.to_lowercase());
    v
}

/// (id, name, version, side) from fabric.mod.json, quilt.mod.json or
/// (neoforge.)mods.toml, with "${file.jarVersion}" taken from the manifest.
fn jar_meta(jar: &Path) -> Option<(String, String, String, String)> {
    let mut z = zip::ZipArchive::new(std::fs::File::open(jar).ok()?).ok()?;
    let mut text = |name: &str| -> Option<String> {
        let mut f = z.by_name(name).ok()?;
        let mut s = String::new();
        std::io::Read::read_to_string(&mut std::io::Read::take(&mut f, 1 << 20), &mut s).ok()?;
        Some(s)
    };
    let side = |env: &str| match env {
        "client" => "client",
        "server" | "dedicated_server" => "server",
        _ => "",
    };
    if let Some(v) = text("fabric.mod.json").and_then(|t| serde_json::from_str::<Value>(t.trim_start_matches('\u{feff}')).ok()) {
        let s = |k: &str| v[k].as_str().unwrap_or_default().to_string();
        return Some((s("id"), s("name"), s("version"), side(v["environment"].as_str().unwrap_or_default()).into()));
    }
    if let Some(v) = text("quilt.mod.json").and_then(|t| serde_json::from_str::<Value>(&t).ok()) {
        let q = &v["quilt_loader"];
        let s = |x: &Value| x.as_str().unwrap_or_default().to_string();
        return Some((s(&q["id"]), s(&q["metadata"]["name"]), s(&q["version"]), side(v["minecraft"]["environment"].as_str().unwrap_or_default()).into()));
    }
    let toml = text("META-INF/neoforge.mods.toml").or_else(|| text("META-INF/mods.toml"))?;
    let first = toml_first_mod(&toml);
    let get = |k: &str| first.get(k).cloned().unwrap_or_default();
    let mut version = get("version");
    if version.contains("${") {
        version = text("META-INF/MANIFEST.MF").and_then(|m| m.lines().find_map(|l| l.strip_prefix("Implementation-Version:").map(|v| v.trim().to_string()))).unwrap_or_default();
    }
    // Forge mods that only run in the game say so with `clientSideOnly = true`.
    let client = toml.lines().any(|l| l.split('#').next().unwrap_or("").replace(' ', "") == "clientSideOnly=true");
    Some((get("modId"), get("displayName"), version, if client { "client".into() } else { String::new() }))
}

/// The keys of the first `[[mods]]` table in a mods.toml (plain `key = "value"` lines).
fn toml_first_mod(t: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut inside = false;
    for l in t.lines().map(str::trim) {
        if l.starts_with('[') {
            if inside {
                break;
            }
            inside = l == "[[mods]]";
            continue;
        }
        if !inside {
            continue;
        }
        let Some((k, v)) = l.split_once('=') else { continue };
        let v = v.trim();
        let v = v.strip_prefix('"').and_then(|x| x.split_once('"')).map(|x| x.0).or_else(|| v.strip_prefix('\'').and_then(|x| x.split_once('\'')).map(|x| x.0));
        if let Some(v) = v {
            out.insert(k.trim().to_string(), v.to_string());
        }
    }
    out
}

/// How much room a folder takes.
pub fn size_of(dir: &Path) -> u64 {
    walkdir::WalkDir::new(dir).follow_links(false).into_iter().flatten().filter_map(|e| e.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum()
}

#[allow(clippy::too_many_arguments)]
fn make(launcher: Launcher, label: &str, name: String, dir: &Path, game: &Path, mc: String, loader: String, loader_version: String, pack: Option<String>, last_played: Option<i64>, icon: Option<PathBuf>) -> Instance {
    let (mods, disabled) = count_mods(&game.join("mods"));
    let icon = icon.filter(|p| p.is_file());
    let name = if name.trim().is_empty() { dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Instance".into()) } else { name.trim().to_string() };
    Instance { id: id_of(game), launcher, launcher_name: label.into(), name, path: dir.to_string_lossy().into_owned(), game_dir: game.to_string_lossy().into_owned(), mc, loader, loader_version, mods, disabled, pack, last_played, has_icon: icon.is_some(), icon }
}

fn id_of(game: &Path) -> String {
    use sha1::Digest as _;
    // Windows paths don't care about case or which slash.
    let key = game.to_string_lossy().replace('\\', "/").trim_end_matches('/').to_lowercase();
    hex::encode(sha1::Sha1::digest(key.as_bytes()))[..16].to_string()
}

fn count_mods(mods: &Path) -> (usize, usize) {
    let mut n = (0, 0);
    for e in std::fs::read_dir(mods).into_iter().flatten().flatten() {
        let f = e.file_name().to_string_lossy().to_lowercase();
        if f.ends_with(".jar") {
            n.0 += 1;
        } else if f.ends_with(".jar.disabled") {
            n.1 += 1;
        }
    }
    n
}

/// "Name 1.2" from a name and a version that may be missing.
fn both(name: Option<&String>, version: Option<&String>) -> Option<String> {
    let name = name.map(|n| n.trim()).filter(|n| !n.is_empty())?;
    Some(match version.map(|v| v.trim()).filter(|v| !v.is_empty() && !name.contains(*v)) {
        Some(v) => format!("{name} {v}"),
        None => name.to_string(),
    })
}

fn safe_key(k: &str) -> bool {
    !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')) && !k.contains("..")
}

/// Prism's icon `key` in its icons folder (the built-in ones have no file).
fn picture_named(icons: &Path, key: &str) -> Option<PathBuf> {
    ["png", "jpg", "jpeg", "gif", "webp", "bmp"].iter().map(|x| icons.join(format!("{key}.{x}"))).chain([icons.join(key)]).find(|p| p.is_file())
}

fn read_small(p: &Path, max: u64) -> Option<Vec<u8>> {
    std::fs::metadata(p).ok().filter(|m| m.is_file() && m.len() <= max)?;
    std::fs::read(p).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(p: &Path, b: impl AsRef<[u8]>) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b).unwrap();
    }

    fn png() -> Vec<u8> {
        super::super::cover::png(&image::RgbaImage::from_pixel(4, 4, image::Rgba([200, 40, 40, 255])))
    }

    /// Prism (instances with .minecraft and minecraft folders, a managed pack,
    /// an icon), CurseForge (Forge and Fabric, the old Curse folder), the
    /// Modrinth App (its current database and the one before, with a moved
    /// folder) and folders the user adds.
    #[test]
    fn finds_what_other_launchers_installed() {
        let t = tempfile::tempdir().unwrap();
        let roaming = t.path().join("Roaming");
        let home = t.path().join("home");

        // Prism Launcher, with its instances moved to another folder.
        let prism = roaming.join("PrismLauncher");
        let insts = t.path().join("Games").join("Prism");
        write(&prism.join("prismlauncher.cfg"), format!("[General]\nInstanceDir={}\nIconsDir=icons\n", insts.display()));
        write(&prism.join("icons").join("atm9.png"), png());
        let atm = insts.join("ATM9");
        write(&atm.join("instance.cfg"), "[General]\nInstanceType=OneSix\nname=\"All the Mods 9, ATM9\"\niconKey=atm9\nlastLaunchTime=1760000000000\nManagedPack=true\nManagedPackName=All the Mods 9\nManagedPackVersionName=0.2.60\nManagedPackType=flame\n");
        write(&atm.join("mmc-pack.json"), r#"{"components":[{"uid":"net.minecraft","version":"1.20.1"},{"uid":"net.minecraftforge","version":"47.2.0"}],"formatVersion":1}"#);
        for m in ["a.jar", "b.jar", "c.jar.disabled"] {
            write(&atm.join(".minecraft").join("mods").join(m), b"jar");
        }
        let fo = insts.join("FO");
        write(&fo.join("instance.cfg"), "name=Fabulously Optimized\niconKey=default\nlastLaunchTime=0\n");
        write(&fo.join("mmc-pack.json"), r#"{"components":[{"uid":"org.lwjgl3","version":"3.3.3"},{"uid":"net.minecraft","version":"1.21.4"},{"uid":"net.fabricmc.intermediary","version":"1.21.4"},{"uid":"net.fabricmc.fabric-loader","version":"0.16.10"}]}"#);
        write(&fo.join("minecraft").join("mods").join("sodium.jar"), b"jar");
        std::fs::create_dir_all(insts.join("_LAUNCHER_TEMP")).unwrap();

        // CurseForge.
        let cf = home.join("curseforge").join("minecraft").join("Instances");
        let bm = cf.join("Better MC");
        write(&bm.join("minecraftinstance.json"), r#"{"name":"Better MC [FORGE]","gameVersion":"1.20.1","baseModLoader":{"name":"forge-47.3.0","forgeVersion":"47.3.0","minecraftVersion":"1.20.1","type":1},"lastPlayed":"2025-09-01T10:00:00.1234567Z","installedModpack":{"name":"Better MC [FORGE] - BMC4"},"manifest":{"name":"Better MC","version":"v32"}}"#);
        write(&bm.join("mods").join("x.jar"), b"jar");
        let fab = cf.join("My Fabric");
        write(&fab.join("minecraftinstance.json"), r#"{"name":"My Fabric","gameVersion":"1.21.1","baseModLoader":{"name":"fabric-0.16.5-1.21.1","forgeVersion":"0.16.5","minecraftVersion":"1.21.1","type":4},"lastPlayed":"0001-01-01T00:00:00"}"#);
        let old = home.join("Documents").join("Curse").join("Minecraft").join("Instances").join("Old");
        write(&old.join("minecraftinstance.json"), r#"{"name":"Old one","gameVersion":"1.12.2","baseModLoader":{"name":"forge-14.23.5.2860","type":1},"lastPlayed":"2019-01-01T00:00:00"}"#);

        // The Modrinth App, with its folder moved and the current database.
        let app = roaming.join("ModrinthApp");
        let moved = t.path().join("D").join("Modrinth");
        std::fs::create_dir_all(&app).unwrap();
        let mr_icon = app.join("caches").join("icons").join("i.png");
        write(&mr_icon, png());
        // Kept open while OmniHub reads, as when the Modrinth App is running.
        let running = rusqlite::Connection::open(app.join("app.db")).unwrap();
        running
            .execute_batch(&format!(
                "PRAGMA journal_mode=WAL;
                 CREATE TABLE settings (id INTEGER, custom_dir TEXT NULL);
                 INSERT INTO settings VALUES (0, '{moved}');
                 CREATE TABLE instances (id TEXT, path TEXT, applied_content_set_id TEXT, install_stage TEXT, name TEXT, icon_path TEXT, last_played INTEGER);
                 CREATE TABLE instance_content_sets (id TEXT, instance_id TEXT, game_version TEXT, loader TEXT, loader_version TEXT);
                 CREATE TABLE instance_links (instance_id TEXT, link_kind TEXT, imported_name TEXT, imported_version_number TEXT);
                 CREATE TABLE instance_quarantines (instance_id TEXT);
                 INSERT INTO instances VALUES ('i1', 'Cobblemon', 'c1', 'installed', 'Cobblemon Official', '{icon}', 1770000000);
                 INSERT INTO instance_content_sets VALUES ('c1', 'i1', '1.21.1', 'fabric', '0.16.9');
                 INSERT INTO instance_links VALUES ('i1', 'imported_modpack', 'Cobblemon Official Modpack', '1.6.1');
                 INSERT INTO instances VALUES ('i2', 'Plain', 'c2', 'installed', 'Plain', NULL, NULL);
                 INSERT INTO instance_content_sets VALUES ('c2', 'i2', '1.21.10', 'vanilla', NULL);
                 INSERT INTO instances VALUES ('i3', 'Half', 'c3', 'pack_installing', 'Half', NULL, NULL);
                 INSERT INTO instances VALUES ('i4', 'Bad', 'c4', 'installed', 'Bad', NULL, NULL);
                 INSERT INTO instance_quarantines VALUES ('i4');
                 INSERT INTO instances VALUES ('i5', '../../outside', 'c5', 'installed', 'Outside', NULL, NULL);",
                moved = moved.display(),
                icon = mr_icon.display()
            ))
            .unwrap();
        for p in ["Cobblemon", "Plain", "Half", "Bad"] {
            std::fs::create_dir_all(moved.join("profiles").join(p).join("mods")).unwrap();
        }
        write(&moved.join("profiles").join("Cobblemon").join("mods").join("cobblemon.jar"), b"jar");

        let found = find(&Places { roaming: Some(roaming.clone()), home: Some(home.clone()), extra: vec![] });
        let by = |n: &str| found.iter().find(|i| i.name == n).unwrap_or_else(|| panic!("{n} wasn't found in {:#?}", found.iter().map(|i| &i.name).collect::<Vec<_>>()));
        assert_eq!(found.len(), 7, "{found:#?}");
        // Most recently played first.
        assert_eq!(found[0].name, "Cobblemon Official");
        assert_eq!(found[1].name, "All the Mods 9, ATM9");

        let a = by("All the Mods 9, ATM9");
        assert_eq!((a.launcher, a.launcher_name.as_str(), a.mc.as_str(), a.loader.as_str(), a.loader_version.as_str()), (Launcher::Prism, "Prism Launcher", "1.20.1", "forge", "47.2.0"));
        assert_eq!((a.mods, a.disabled, a.pack.as_deref(), a.last_played), (2, 1, Some("All the Mods 9 0.2.60"), Some(1_760_000_000)));
        assert_eq!(Path::new(&a.game_dir), atm.join(".minecraft"));
        assert_eq!(icon(a), Some(png()));
        let f = by("Fabulously Optimized");
        assert_eq!((f.mc.as_str(), f.loader.as_str(), f.loader_version.as_str(), f.mods, f.last_played, f.has_icon, f.pack.as_deref()), ("1.21.4", "fabric", "0.16.10", 1, None, false, None));
        assert_eq!(Path::new(&f.game_dir), fo.join("minecraft"));

        let b = by("Better MC [FORGE]");
        assert_eq!((b.launcher, b.mc.as_str(), b.loader.as_str(), b.loader_version.as_str(), b.mods), (Launcher::Curseforge, "1.20.1", "forge", "47.3.0", 1));
        assert_eq!((b.pack.as_deref(), b.last_played), (Some("Better MC [FORGE] - BMC4 v32"), Some(1_756_720_800)));
        assert_eq!(Path::new(&b.game_dir), bm);
        let mf = by("My Fabric");
        assert_eq!((mf.loader.as_str(), mf.loader_version.as_str(), mf.last_played), ("fabric", "0.16.5", None));
        assert_eq!(by("Old one").loader_version, "14.23.5.2860");

        let c = by("Cobblemon Official");
        assert_eq!((c.launcher, c.mc.as_str(), c.loader.as_str(), c.loader_version.as_str(), c.mods), (Launcher::Modrinth, "1.21.1", "fabric", "0.16.9", 1));
        assert_eq!((c.pack.as_deref(), c.last_played, c.has_icon), (Some("Cobblemon Official Modpack 1.6.1"), Some(1_770_000_000), true));
        assert_eq!(Path::new(&c.path), moved.join("profiles").join("Cobblemon"));
        let p = by("Plain");
        assert_eq!((p.mc.as_str(), p.loader.as_str()), ("1.21.10", ""), "vanilla has no loader");
        assert!(found.iter().all(|i| !["Half", "Bad", "Outside"].contains(&i.name.as_str())), "still installing, quarantined, or outside its folder");
        drop(running);
        // With the app closed, reading leaves nothing beside its database.
        assert_eq!(find(&Places { roaming: Some(roaming.clone()), home: Some(home.clone()), extra: vec![] }), found);
        assert!(!app.join("app.db-wal").exists() && !app.join("app.db-shm").exists());

        // Ids stay the same, and say nothing about the path.
        let again = find(&Places { roaming: Some(roaming.clone()), home: Some(home.clone()), extra: vec![] });
        assert_eq!(found, again);
        assert!(found.iter().all(|i| i.id.len() == 16 && i.id.bytes().all(|b| b.is_ascii_hexdigit())));

        // Added by hand: a portable MultiMC, one instance folder, and Prism's own
        // folder again (not listed twice).
        let mmc = t.path().join("MultiMC");
        write(&mmc.join("multimc.cfg"), "InstanceDir=instances\n");
        write(&mmc.join("instances").join("Vanilla+").join("instance.cfg"), "name=Vanilla+\n");
        let one = t.path().join("elsewhere").join("Lone");
        write(&one.join("minecraftinstance.json"), r#"{"name":"Lone","gameVersion":"1.19.2","baseModLoader":{"name":"neoforge-47.1.84","type":6}}"#);
        let found = find(&Places { roaming: Some(roaming), home: Some(home), extra: vec![mmc, one, prism] });
        assert_eq!(found.len(), 9);
        let v = found.iter().find(|i| i.name == "Vanilla+").unwrap();
        assert_eq!(v.launcher_name, "MultiMC");
        let l = found.iter().find(|i| i.name == "Lone").unwrap();
        assert_eq!((l.loader.as_str(), l.loader_version.as_str()), ("neoforge", "47.1.84"));
    }

    /// The Modrinth App's database before mid-2026 (one profiles table), and
    /// before that its profile.json files.
    #[test]
    fn older_modrinth_app_layouts() {
        let t = tempfile::tempdir().unwrap();
        let roaming = t.path().join("Roaming");
        let app = roaming.join("ModrinthApp");
        std::fs::create_dir_all(app.join("profiles").join("Fabric 1").join("mods")).unwrap();
        write(&app.join("profiles").join("Fabric 1").join("mods").join("a.jar"), b"jar");
        {
            let db = rusqlite::Connection::open(app.join("app.db")).unwrap();
            db.execute_batch(
                "CREATE TABLE settings (id INTEGER, custom_dir TEXT NULL);
                 INSERT INTO settings VALUES (0, NULL);
                 CREATE TABLE profiles (path TEXT, install_stage TEXT, name TEXT, icon_path TEXT, game_version TEXT, mod_loader TEXT, mod_loader_version TEXT, last_played INTEGER);
                 INSERT INTO profiles VALUES ('Fabric 1', 'installed', 'Fabric One', NULL, '1.20.4', 'quilt', '0.26.0', 1700000000);",
            )
            .unwrap();
        }
        let found = find(&Places { roaming: Some(roaming.clone()), ..Default::default() });
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].name.as_str(), found[0].mc.as_str(), found[0].loader.as_str(), found[0].loader_version.as_str(), found[0].mods), ("Fabric One", "1.20.4", "quilt", "0.26.0", 1));
        let mut beside: Vec<String> = std::fs::read_dir(&app).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        beside.sort();
        assert_eq!(beside, ["app.db", "profiles"], "nothing new beside the database");

        std::fs::remove_dir_all(&app).unwrap();
        let old = roaming.join("com.modrinth.theseus").join("profiles").join("Legacy");
        write(&old.join("profile.json"), r#"{"path":"Legacy","install_stage":"installed","metadata":{"name":"Legacy pack","game_version":"1.19.4","loader":"forge","loader_version":{"id":"45.1.0"},"last_played":"2023-06-01T12:00:00Z"},"projects":{}}"#);
        let found = find(&Places { roaming: Some(roaming), ..Default::default() });
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].name.as_str(), found[0].loader.as_str(), found[0].loader_version.as_str(), found[0].last_played), ("Legacy pack", "forge", "45.1.0", Some(1_685_620_800)));
    }

    fn level_dat(name: &str, version: &str, mode: i32, player: bool) -> Vec<u8> {
        use crate::mcserver::nbt::{gzip, write, Tag};
        let mut data = vec![
            ("LevelName".to_string(), Tag::Str(name.into())),
            ("Version".into(), Tag::Compound(vec![("Name".into(), Tag::Str(version.into()))])),
            ("LastPlayed".into(), Tag::Long(1_760_000_000_000)),
            ("GameType".into(), Tag::Int(mode)),
            ("hardcore".into(), Tag::Byte(0)),
            ("Difficulty".into(), Tag::Byte(3)),
            ("allowCommands".into(), Tag::Byte(1)),
        ];
        if player {
            data.push(("Player".into(), Tag::Compound(vec![("XpLevel".into(), Tag::Int(30))])));
        }
        gzip(&write(&Tag::Compound(vec![("Data".into(), Tag::Compound(data))])))
    }

    /// Worlds from their level.dat, mods from their jars' own descriptions,
    /// and the Minecraft Launcher's folder with its last-played version.
    #[test]
    fn worlds_and_mods_inside() {
        use crate::mods::tests::zip_of;
        let t = tempfile::tempdir().unwrap();
        let game = t.path().join("Roaming").join(".minecraft");
        write(&game.join("saves/New World/level.dat"), level_dat("My Base", "1.21.1", 1, true));
        write(&game.join("saves/New World/region/r.0.0.mca"), vec![0u8; 4096]);
        write(&game.join("saves/New World/icon.png"), png());
        write(&game.join("saves/Old/level.dat"), level_dat("", "1.12.2", 7, false));
        std::fs::create_dir_all(game.join("saves/not a world")).unwrap();
        let ws = worlds(&game);
        assert_eq!(ws.len(), 2);
        let w = &ws.iter().find(|w| w.folder == "New World").unwrap();
        assert_eq!((w.name.as_str(), w.version.as_str(), w.game_mode.as_str(), w.difficulty.as_str(), w.cheats, w.hardcore, w.has_player, w.has_icon, w.last_played), ("My Base", "1.21.1", "creative", "hard", true, false, true, true, Some(1_760_000_000)));
        assert!(w.size > 4096);
        let o = ws.iter().find(|w| w.folder == "Old").unwrap();
        assert_eq!((o.name.as_str(), o.game_mode.as_str(), o.has_player), ("Old", "spectator", false), "an unnamed world goes by its folder");
        for bad in ["", "..", "../New World", "saves/New World", "not a world"] {
            assert!(world_dir(&game, bad).is_none(), "{bad}");
        }
        assert!(world_dir(&game, "New World").is_some());

        let m = game.join("mods");
        write(&m.join("sodium.jar"), zip_of(&[("fabric.mod.json", br#"{"id":"sodium","name":"Sodium","version":"0.6.0","environment":"client"}"#)]));
        write(&m.join("qsl.jar"), zip_of(&[("quilt.mod.json", br#"{"quilt_loader":{"id":"qsl","version":"10.0","metadata":{"name":"Quilt Standard Libraries"}},"minecraft":{"environment":"*"}}"#)]));
        write(&m.join("create.jar"), zip_of(&[("META-INF/mods.toml", b"modLoader=\"javafml\"\n[[mods]]\nmodId=\"create\"\nversion=\"${file.jarVersion}\"\ndisplayName=\"Create\" # the name\n[[dependencies.create]]\nmodId=\"forge\"\n"), ("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\r\nImplementation-Version: 6.0.4\r\n")]));
        write(&m.join("emi.jar"), zip_of(&[("META-INF/neoforge.mods.toml", b"[[mods]]\nmodId='emi'\nversion='1.1.18'\ndisplayName='EMI'\nclientSideOnly = true\n")]));
        write(&m.join("mystery.jar"), b"not a zip");
        write(&m.join("off.jar.disabled"), zip_of(&[("fabric.mod.json", br#"{"id":"off","name":"Turned Off","version":"1"}"#)]));
        write(&m.join("notes.txt"), b"x");
        let mods = mods_in(&game);
        let row = |f: &str| mods.iter().find(|x| x.file == f).map(|x| (x.id.as_str(), x.name.as_str(), x.version.as_str(), x.side.as_str(), x.disabled)).unwrap();
        assert_eq!(mods.len(), 6);
        assert_eq!(row("sodium.jar"), ("sodium", "Sodium", "0.6.0", "client", false));
        assert_eq!(row("qsl.jar"), ("qsl", "Quilt Standard Libraries", "10.0", "", false));
        assert_eq!(row("create.jar"), ("create", "Create", "6.0.4", "", false));
        assert_eq!(row("emi.jar"), ("emi", "EMI", "1.1.18", "client", false));
        assert_eq!(row("mystery.jar"), ("", "mystery", "", "", false));
        assert_eq!(row("off.jar.disabled"), ("off", "Turned Off", "1", "", true));

        // The Minecraft Launcher's own folder: the profile played last that uses it.
        write(&game.join("launcher_profiles.json"), r#"{"profiles":{
            "a":{"lastUsed":"2026-09-01T10:00:00.000Z","lastVersionId":"fabric-loader-0.16.9-1.21.1"},
            "b":{"lastUsed":"2025-01-01T10:00:00.000Z","lastVersionId":"1.20.1"},
            "c":{"lastUsed":"2026-10-01T10:00:00.000Z","lastVersionId":"1.19","gameDir":"D:\\Elsewhere"}}}"#);
        let found = find(&Places { roaming: Some(t.path().join("Roaming")), ..Default::default() });
        let l = found.iter().find(|i| i.launcher == Launcher::Minecraft).unwrap();
        assert_eq!((l.name.as_str(), l.mc.as_str(), l.loader.as_str(), l.loader_version.as_str(), l.mods, l.disabled), ("Minecraft Launcher", "1.21.1", "fabric", "0.16.9", 5, 1));
        assert_eq!(version_parts("1.20.1-forge-47.2.0"), ("1.20.1".into(), "forge".into(), "47.2.0".into()));
        assert_eq!(version_parts("neoforge-21.1.77"), ("1.21.1".into(), "neoforge".into(), "21.1.77".into()));
        assert_eq!(version_parts("neoforge-21.0.167"), ("1.21".into(), "neoforge".into(), "21.0.167".into()));
        assert_eq!(version_parts("neoforge-26.1.2.122"), ("26.1.2".into(), "neoforge".into(), "26.1.2.122".into()));
        assert_eq!(version_parts("quilt-loader-0.26.0-1.20.4"), ("1.20.4".into(), "quilt".into(), "0.26.0".into()));
        assert_eq!(version_parts("latest-release"), Default::default());
        assert_eq!(version_parts("26.3"), ("26.3".into(), String::new(), String::new()));
    }

    #[test]
    fn database_addresses() {
        assert_eq!(file_uri(Path::new("/home/a b/#1%/app.db")), "file:///home/a%20b/%231%25/app.db");
        assert_eq!(file_uri(Path::new("C:\\Users\\Ké\\AppData\\Roaming\\ModrinthApp\\app.db")), "file:///C:/Users/K%C3%A9/AppData/Roaming/ModrinthApp/app.db");
    }

    #[test]
    fn qt_settings_values() {
        let m = ini("[General]\nname=\"A \\\"quoted\\\" name, with a comma\"\nplain=x=y\n");
        assert_eq!(m["name"], "A \"quoted\" name, with a comma");
        assert_eq!(m["plain"], "x=y");
        assert!(!safe_key("../x") && !safe_key("") && safe_key("modrinth_fabulously-optimized"));
    }
}
