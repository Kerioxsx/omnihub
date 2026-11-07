//! Mods: every game in the Steam library (and Epic, and any folder the user
//! adds), how each is modded, and a mod manager for them. Mods come from
//! Thunderstore (with their dependencies and the game's mod loader), from a
//! file or folder the user picks, or from Claude (`maker`). Every file a mod
//! puts in place is recorded, with any game file it replaced backed up, so
//! a mod can be switched off (originals back) or removed cleanly.
//!
//! Online competitive games are left alone: changing their files breaks
//! their anti-cheat rules. Games with anti-cheat get a plain warning.

pub mod cover;
pub mod detect;
pub mod install;
pub mod launchers;
pub mod maker;
pub mod minecraft;
pub mod thunderstore;
pub mod workshop;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;
use crate::games::GameHub;
use crate::system::procs::ProcessMonitor;
use detect::{Engine, LoaderKind, Traits};
use install::{Deployed, Dirs, Layout, Root};
use thunderstore::{LoaderPackages, Package, Sort, TsGame};

const GITHUB: &str = "https://api.github.com";
const STEAM_API: &str = "https://api.steampowered.com";
const SCHEMA_TTL: i64 = 24 * 3600 * 1000;
const LISTING_TTL: i64 = 3 * 3600 * 1000;

/// How OmniHub puts mods in place for a game.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Method {
    BepInEx,
    BepInExIl2cpp,
    /// MelonLoader 0.7+, a folder per mod.
    Melon,
    /// MelonLoader on games that keep mods flat in `Mods/`.
    MelonLegacy,
    Lovely,
    Gta,
    Unreal,
    /// Files into the game folder, as the mod has them.
    Files,
    /// Not offered (online competitive games).
    None,
}

impl Method {
    fn layout(self, t: &Traits) -> Option<Layout> {
        Some(match self {
            Method::BepInEx | Method::BepInExIl2cpp => Layout::BepInEx,
            Method::Melon => Layout::Melon { recursive: true },
            Method::MelonLegacy => Layout::Melon { recursive: false },
            Method::Lovely => Layout::Lovely,
            Method::Gta => Layout::Gta,
            Method::Unreal => Layout::Unreal { paks: t.paks.clone()?, binaries: t.binaries.clone() },
            Method::Files => Layout::GameRoot,
            Method::None => return None,
        })
    }

    fn label(self) -> &'static str {
        match self {
            Method::BepInEx => "BepInEx plugins",
            Method::BepInExIl2cpp => "BepInEx 6 plugins",
            Method::Melon | Method::MelonLegacy => "MelonLoader mods",
            Method::Lovely => "Lovely mods",
            Method::Gta => "Script Hook V mods",
            Method::Unreal => ".pak and UE4SS mods",
            Method::Files => "Files in the game folder",
            Method::None => "No mods",
        }
    }

    /// The loader(s) mods need, in install order.
    fn loaders(self) -> Vec<LoaderKind> {
        match self {
            Method::BepInEx => vec![LoaderKind::BepInEx],
            Method::BepInExIl2cpp => vec![LoaderKind::BepInExIl2cpp],
            Method::Melon | Method::MelonLegacy => vec![LoaderKind::MelonLoader],
            Method::Lovely => vec![LoaderKind::Lovely],
            Method::Gta => vec![LoaderKind::ScriptHookV, LoaderKind::ScriptHookVDotNet],
            _ => vec![],
        }
    }
}

/// What Claude can make for a game.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MakerKind {
    /// Balatro: a Steamodded mod (Lua).
    Steamodded,
    /// Other LÖVE games: Lovely patches and Lua.
    Lovely,
    /// GTA V: a Script Hook V .NET script (C#, compiled by the loader).
    ShvdnScript,
    /// Unreal: a UE4SS Lua mod.
    Ue4ssLua,
    /// Unity: a BepInEx plugin (C#, built with the .NET SDK).
    BepInExPlugin,
    /// Unity: a MelonLoader mod (C#, built with the .NET SDK).
    MelonMod,
    /// Config, data or text files in the game folder.
    Files,
    /// Minecraft servers: a data pack (functions, recipes, loot, world generation).
    Datapack,
    /// Paper servers: a plugin (Java, compiled with the JDK).
    PaperPlugin,
}

impl MakerKind {
    pub fn label(self) -> &'static str {
        match self {
            MakerKind::Steamodded => "a Steamodded mod (Lua)",
            MakerKind::Lovely => "a Lovely mod (Lua)",
            MakerKind::ShvdnScript => "a Script Hook V .NET script (C#)",
            MakerKind::Ue4ssLua => "a UE4SS Lua mod",
            MakerKind::BepInExPlugin => "a BepInEx plugin (C#)",
            MakerKind::MelonMod => "a MelonLoader mod (C#)",
            MakerKind::Files => "game files (config, data, text)",
            MakerKind::Datapack => "a data pack",
            MakerKind::PaperPlugin => "a Paper plugin (Java)",
        }
    }

    /// Needs building with the .NET SDK.
    pub fn compiled(self) -> bool {
        matches!(self, MakerKind::BepInExPlugin | MakerKind::MelonMod)
    }

    /// Needs building before it's installed (.NET or Java).
    pub fn built(self) -> bool {
        self.compiled() || self == MakerKind::PaperPlugin
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    /// info, warn, bad
    pub tone: String,
    pub text: String,
    pub link: Option<String>,
}

fn note(tone: &str, text: impl Into<String>, link: Option<&str>) -> Note {
    Note { tone: tone.into(), text: text.into(), link: link.map(String::from) }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoaderInfo {
    pub kind: LoaderKind,
    pub label: String,
    pub installed: bool,
    /// OmniHub put it there (and can switch it off or remove it).
    pub managed: bool,
    /// OmniHub can install it.
    pub auto: bool,
    /// Where to get it when OmniHub can't.
    pub link: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MakerInfo {
    pub available: bool,
    pub kind: Option<MakerKind>,
    pub label: String,
    pub why: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModGame {
    pub key: String,
    pub name: String,
    /// steam, epic, folder
    pub source: String,
    pub app_id: Option<u32>,
    pub dir: String,
    pub exe: Option<String>,
    pub engine: Engine,
    pub engine_label: String,
    pub anti_cheat: Vec<String>,
    pub online_only: bool,
    pub method: Method,
    pub method_label: String,
    pub loaders: Vec<LoaderInfo>,
    /// Its Thunderstore community, when it has one.
    pub thunderstore: Option<String>,
    /// Thunderstore's loader name for it (bepinex, lovely, shimloader…).
    pub ts_loader: Option<String>,
    /// OmniHub installs its Thunderstore mods (otherwise they open in r2modman).
    pub ts_installs: bool,
    pub data_dir: Option<String>,
    pub mods: usize,
    pub enabled_mods: usize,
    pub image: Option<String>,
    pub notes: Vec<Note>,
    pub maker: MakerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ModSource {
    Thunderstore { full_name: String, url: String },
    File { name: String },
    Claude { project: String },
    Github { repo: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMod {
    pub id: String,
    pub game: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub icon: String,
    pub source: ModSource,
    pub enabled: bool,
    /// This is a mod loader (BepInEx, Lovely…).
    pub loader: Option<LoaderKind>,
    /// Installed because another mod needed it.
    #[serde(default)]
    pub dependency: bool,
    pub installed_at: i64,
    pub game_dir: String,
    pub data_dir: Option<String>,
    pub files: Vec<Deployed>,
    pub created: Vec<(Root, String)>,
    /// Files the install moved out of the way (an older loader), put back on removal.
    #[serde(default)]
    pub moved: Vec<(Root, String)>,
    /// A newer version on Thunderstore.
    #[serde(default, skip_deserializing)]
    pub update: Option<String>,
}

impl InstalledMod {
    fn dirs(&self) -> Dirs {
        Dirs { game: PathBuf::from(&self.game_dir), data: self.data_dir.as_ref().map(PathBuf::from) }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomGame {
    pub key: String,
    pub name: String,
    pub dir: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct State {
    mods: Vec<InstalledMod>,
    folders: Vec<CustomGame>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Browse {
    pub packages: Vec<BrowsePackage>,
    pub total: usize,
    pub loading: bool,
    pub progress: Option<(usize, usize)>,
    pub error: Option<String>,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BrowsePackage {
    #[serde(flatten)]
    pub package: Package,
    /// The installed version, if any.
    pub installed: Option<String>,
    /// The mod loader itself.
    pub is_loader: bool,
    /// r2modman's install link, for games OmniHub doesn't install for.
    pub manager_link: String,
}

#[derive(Default)]
struct Listing {
    fetched: i64,
    packages: Arc<Vec<Package>>,
    loading: bool,
    progress: Option<(usize, usize)>,
    error: Option<String>,
}

/// Steam games whose mods are cheating: online competitive play with anti-cheat.
const ONLINE_ONLY: &[u32] = &[
    730, 570, 1_172_470, 2_357_570, 1_938_090, 252_950, 578_080, 359_550, 2_767_030, 2_073_850, 1_422_450, 1_203_220, 1_085_660, 381_210, 252_490, 2_507_950, 230_410, 1_599_340, 236_390, 444_090, 386_360, 1_665_460, 1_240_440,
];

fn online_only(g: &crate::games::library::InstalledGame) -> bool {
    use crate::games::GameKind;
    matches!(g.kind, GameKind::Fortnite | GameKind::Valorant | GameKind::Cs2 | GameKind::Apex | GameKind::RocketLeague | GameKind::CallOfDuty | GameKind::League | GameKind::Overwatch | GameKind::Roblox)
        || matches!(g.launch, crate::games::Launch::Steam { app_id } if ONLINE_ONLY.contains(&app_id))
}

const BALATRO: u32 = 2_379_780;
const GTA_ENHANCED: u32 = 3_240_220;
const GTA_LEGACY: u32 = 271_590;
pub const SCRIPT_HOOK_V: &str = "https://www.dev-c.com/gtav/scripthookv/";

fn now() -> i64 {
    crate::db::now()
}

pub(crate) fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if (c == ' ' || c == '-' || c == '_' || c == '.') && !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    let out = out.trim_end_matches('_').chars().take(48).collect::<String>();
    if out.is_empty() {
        "mod".into()
    } else {
        out
    }
}

/// Progress of a long change, for the window.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    game: &'a str,
    text: String,
    done: usize,
    total: usize,
}

/// Every game, described, with what was read from its folder.
type Described = Vec<(ModGame, Traits)>;

pub struct ModHub {
    dir: PathBuf,
    events: EventBus,
    games: Arc<GameHub>,
    procs: Arc<ProcessMonitor>,
    pub claude: Arc<crate::claude::Claude>,
    state: Mutex<State>,
    ts: thunderstore::Client,
    github: String,
    steam_api: String,
    roaming: Option<PathBuf>,
    schema: Mutex<Option<(i64, Vec<TsGame>, LoaderPackages)>>,
    schema_loading: Mutex<bool>,
    listings: Mutex<HashMap<String, Listing>>,
    cache: Mutex<Option<(Instant, Described)>>,
    busy: Mutex<HashSet<String>>,
    pub maker: maker::Maker,
    /// Minecraft modpacks, mixed.
    pub minecraft: minecraft::Mixer,
    /// The user's own Pollinations account, for painted mix covers.
    pub pollinations: cover::Pollinations,
}

/// Where the hub talks to (real sites, or pretend ones in tests).
#[derive(Debug, Clone)]
pub struct Endpoints {
    pub thunderstore: String,
    pub github: String,
    pub steam_api: String,
    pub claude: String,
    pub modrinth: String,
    pub fabric_meta: String,
    pub quilt_meta: String,
    pub pollinations_gen: String,
    pub pollinations_enter: String,
    /// %AppData% (Roaming), for games that keep mods there (and .minecraft).
    pub roaming: Option<PathBuf>,
    /// The user's folder (CurseForge keeps its instances there).
    pub home: Option<PathBuf>,
}

impl Default for Endpoints {
    fn default() -> Self {
        Endpoints {
            thunderstore: thunderstore::BASE.into(),
            github: GITHUB.into(),
            steam_api: STEAM_API.into(),
            claude: crate::claude::api::BASE.into(),
            modrinth: minecraft::MODRINTH.into(),
            fabric_meta: minecraft::FABRIC_META.into(),
            quilt_meta: minecraft::QUILT_META.into(),
            pollinations_gen: cover::GEN.into(),
            pollinations_enter: cover::ENTER.into(),
            roaming: dirs::config_dir(),
            home: dirs::home_dir(),
        }
    }
}

impl ModHub {
    pub fn new(data_dir: &Path, events: EventBus, games: Arc<GameHub>, procs: Arc<ProcessMonitor>) -> Arc<ModHub> {
        ModHub::with(data_dir, events, games, procs, Endpoints::default())
    }

    pub fn with(data_dir: &Path, events: EventBus, games: Arc<GameHub>, procs: Arc<ProcessMonitor>, ep: Endpoints) -> Arc<ModHub> {
        let dir = data_dir.join("mods");
        let state: State = std::fs::read(dir.join("state.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let schema = std::fs::read(dir.join("cache").join("thunderstore-games.json")).ok().and_then(|b| serde_json::from_slice::<(i64, Vec<TsGame>, LoaderPackages)>(&b).ok());
        let claude = Arc::new(crate::claude::Claude::with_base(data_dir, &ep.claude));
        let mc_dir = ep.roaming.as_ref().map(|r| r.join(".minecraft"));
        Arc::new(ModHub {
            maker: maker::Maker::new(&dir.join("maker"), events.clone(), claude.clone()),
            minecraft: minecraft::Mixer::new(&dir.join("minecraft"), minecraft::Modrinth::new(&ep.modrinth, &ep.fabric_meta, &ep.quilt_meta), mc_dir).looking_in(launchers::Places { roaming: ep.roaming.clone(), home: ep.home.clone(), extra: vec![] }),
            pollinations: cover::Pollinations::with(data_dir, &ep.pollinations_gen, &ep.pollinations_enter),
            dir,
            events,
            games,
            procs,
            claude,
            state: Mutex::new(state),
            ts: thunderstore::Client::new(&ep.thunderstore),
            github: ep.github.trim_end_matches('/').to_string(),
            steam_api: ep.steam_api,
            roaming: ep.roaming,
            schema: Mutex::new(schema),
            schema_loading: Mutex::new(false),
            listings: Mutex::new(HashMap::new()),
            cache: Mutex::new(None),
            busy: Mutex::new(HashSet::new()),
        })
    }

    fn save(&self, s: &State) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let tmp = self.dir.join("state.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, self.dir.join("state.json")).map_err(|e| e.to_string())
    }

    fn changed(&self, game: &str) {
        *self.cache.lock() = None;
        self.events.emit("mods:changed", serde_json::json!({ "game": game }));
    }

    fn progress(&self, game: &str, text: impl Into<String>, done: usize, total: usize) {
        self.events.emit("mods:progress", Progress { game, text: text.into(), done, total });
    }

    // ---------- Thunderstore's game list ----------

    fn ts_game(&self, app: Option<u32>, exe: Option<&str>) -> Option<TsGame> {
        let s = self.schema.lock();
        let (_, games, _) = s.as_ref()?;
        if let Some(a) = app {
            if let Some(g) = games.iter().find(|g| g.steam.contains(&a)) {
                return Some(g.clone());
            }
        }
        let exe = exe?.to_lowercase();
        games.iter().find(|g| g.steam.is_empty() && g.exe_names.iter().any(|e| e.to_lowercase() == exe)).cloned()
    }

    fn loader_packages(&self) -> LoaderPackages {
        self.schema.lock().as_ref().map(|s| s.2.clone()).unwrap_or_default()
    }

    /// Fetch Thunderstore's game list now if it's missing or a day old.
    pub fn refresh_schema(&self) -> Result<(), String> {
        let fresh = self.schema.lock().as_ref().is_some_and(|(at, _, _)| now() - at < SCHEMA_TTL);
        if fresh {
            return Ok(());
        }
        let (games, loaders) = self.ts.schema()?;
        let v = (now(), games, loaders);
        let cache = self.dir.join("cache");
        let _ = std::fs::create_dir_all(&cache);
        let _ = std::fs::write(cache.join("thunderstore-games.json"), serde_json::to_vec(&v).unwrap_or_default());
        *self.schema.lock() = Some(v);
        *self.cache.lock() = None;
        Ok(())
    }

    fn refresh_schema_later(self: &Arc<Self>) {
        let stale = self.schema.lock().as_ref().is_none_or(|(at, _, _)| now() - at >= SCHEMA_TTL);
        if !stale || std::mem::replace(&mut *self.schema_loading.lock(), true) {
            return;
        }
        let me = self.clone();
        std::thread::Builder::new()
            .name("mods-schema".into())
            .spawn(move || {
                let ok = me.refresh_schema().is_ok();
                *me.schema_loading.lock() = false;
                if ok {
                    me.changed("");
                }
            })
            .ok();
    }

    // ---------- the games ----------

    fn data_dir_for(&self, engine: Engine, app: Option<u32>, exe: Option<&str>) -> Option<PathBuf> {
        let roaming = self.roaming.as_ref()?;
        if app == Some(BALATRO) {
            return Some(roaming.join("Balatro"));
        }
        (engine == Engine::Love).then(|| exe.map(|e| roaming.join(e.trim_end_matches(".exe").trim_end_matches(".EXE"))))?
    }

    #[allow(clippy::too_many_arguments)]
    fn describe(&self, key: &str, name: &str, source: &str, app: Option<u32>, dir: &Path, blocked: bool, state: &State) -> (ModGame, Traits) {
        let t = detect::detect(dir);
        let ts = self.ts_game(app, t.exe.as_deref());
        let gta = matches!(app, Some(GTA_ENHANCED) | Some(GTA_LEGACY)) || t.engine == Engine::Rage;
        let method = if blocked {
            Method::None
        } else if let Some(g) = ts.as_ref().filter(|g| matches!(g.loader.as_str(), "bepinex" | "melonloader" | "recursive-melonloader" | "lovely")) {
            match g.loader.as_str() {
                "bepinex" if t.engine == Engine::UnityIl2cpp => Method::BepInExIl2cpp,
                "bepinex" => Method::BepInEx,
                "melonloader" => Method::MelonLegacy,
                "recursive-melonloader" => Method::Melon,
                _ => Method::Lovely,
            }
        } else if gta {
            Method::Gta
        } else if t.loaders.contains(&LoaderKind::MelonLoader) {
            Method::Melon
        } else {
            match t.engine {
                Engine::Love => Method::Lovely,
                Engine::UnityMono => Method::BepInEx,
                Engine::UnityIl2cpp => Method::BepInExIl2cpp,
                Engine::Unreal if t.paks.is_some() => Method::Unreal,
                _ => Method::Files,
            }
        };
        let mods: Vec<&InstalledMod> = state.mods.iter().filter(|m| m.game == key).collect();
        let mut loaders: Vec<LoaderInfo> = method
            .loaders()
            .into_iter()
            .map(|k| {
                let managed = mods.iter().any(|m| m.loader == Some(k));
                LoaderInfo {
                    kind: k,
                    label: k.label().into(),
                    installed: t.loaders.contains(&k) || mods.iter().any(|m| m.loader == Some(k) && m.enabled),
                    managed,
                    auto: k != LoaderKind::ScriptHookV,
                    link: (k == LoaderKind::ScriptHookV).then(|| SCRIPT_HOOK_V.to_string()),
                }
            })
            .collect();
        if method == Method::Unreal && t.binaries.is_some() {
            let k = LoaderKind::Ue4ss;
            loaders.push(LoaderInfo { kind: k, label: "UE4SS (for Lua mods)".into(), installed: t.loaders.contains(&k), managed: mods.iter().any(|m| m.loader == Some(k)), auto: true, link: None });
        }
        let ts_installs = ts.is_some() && matches!(method, Method::BepInEx | Method::BepInExIl2cpp | Method::Melon | Method::MelonLegacy | Method::Lovely);
        let data_dir = self.data_dir_for(t.engine, app, t.exe.as_deref());

        let mut notes = Vec::new();
        if blocked {
            notes.push(note("bad", "An online competitive game: changing its files breaks its anti-cheat rules and can get the account banned, so OmniHub doesn't offer mods for it.", None));
        } else if gta {
            notes.push(note("warn", "Mods work in Story Mode only, with BattlEye off: in the Rockstar Games Launcher, Settings → this game → uncheck BattlEye (or add -nobattleye to the game's Steam launch options). GTA Online won't start that way: switch every mod off here and BattlEye back on to play online.", None));
            if !t.loaders.contains(&LoaderKind::ScriptHookV) && !mods.iter().any(|m| m.loader == Some(LoaderKind::ScriptHookV)) {
                notes.push(note("info", "Most mods need Script Hook V by Alexander Blade. Its licence doesn't let others hand it out: download it from his page (pick the version for your game build), then add the zip here.", Some(SCRIPT_HOOK_V)));
            }
        } else if !t.anti_cheat.is_empty() {
            notes.push(note("warn", format!("It uses {}. Keep mods to offline or single-player play; modded online play can get the account banned.", t.anti_cheat.join(" and ")), None));
        }
        if !blocked {
            match method {
                Method::Lovely if app == Some(BALATRO) => notes.push(note("info", "Mods live in %AppData%\\Balatro\\Mods. Most need Steamodded; OmniHub installs it (and Lovely) with the first mod that does.", None)),
                Method::Lovely => notes.push(note("info", "Mods for LÖVE games load through Lovely, from the game's folder in %AppData%.", None)),
                Method::BepInEx | Method::BepInExIl2cpp if !ts_installs => notes.push(note("info", "Mods for this game are BepInEx plugins (.dll). Get them from its mod pages (Nexus Mods, GitHub…) and add the file here; OmniHub installs BepInEx with the first one.", None)),
                Method::Unreal => notes.push(note("info", ".pak mods go into ~mods; Lua mods need UE4SS, which OmniHub can install. Some Unreal games check their files and refuse mods.", None)),
                Method::Files => notes.push(note("info", "OmniHub doesn't know a mod loader for this game. Mods that are files for the game folder can still be added and removed cleanly; follow the mod's own instructions.", None)),
                _ => {}
            }
            if let Some(g) = ts.as_ref().filter(|_| !ts_installs) {
                notes.push(note("info", format!("Thunderstore has mods for it that use {}. Their Install buttons open r2modman or the Thunderstore Mod Manager.", g.loader), Some("https://r2modman.com/")));
            }
        }

        let maker_kind = match method {
            Method::Lovely if app == Some(BALATRO) => Some(MakerKind::Steamodded),
            Method::Lovely => Some(MakerKind::Lovely),
            Method::Gta => Some(MakerKind::ShvdnScript),
            Method::Unreal if t.binaries.is_some() => Some(MakerKind::Ue4ssLua),
            Method::BepInEx | Method::BepInExIl2cpp => Some(MakerKind::BepInExPlugin),
            Method::Melon | Method::MelonLegacy => Some(MakerKind::MelonMod),
            Method::Files | Method::Unreal => Some(MakerKind::Files),
            Method::None => None,
        };
        let maker = match maker_kind {
            Some(k) => MakerInfo { available: true, kind: Some(k), label: k.label().into(), why: None },
            None => MakerInfo { available: false, kind: None, label: String::new(), why: Some("Claude doesn't make mods for online competitive games: they'd count as cheating.".into()) },
        };
        let g = ModGame {
            key: key.into(),
            name: name.into(),
            source: source.into(),
            app_id: app,
            dir: dir.to_string_lossy().into_owned(),
            exe: t.exe.clone(),
            engine: t.engine,
            engine_label: t.engine.label().into(),
            anti_cheat: t.anti_cheat.clone(),
            online_only: blocked,
            method,
            method_label: method.label().into(),
            loaders,
            thunderstore: ts.as_ref().map(|g| g.community.clone()),
            ts_loader: ts.as_ref().map(|g| g.loader.clone()),
            ts_installs,
            data_dir: data_dir.map(|d| d.to_string_lossy().into_owned()),
            mods: mods.iter().filter(|m| m.loader.is_none()).count(),
            enabled_mods: mods.iter().filter(|m| m.loader.is_none() && m.enabled).count(),
            image: app.map(|a| format!("https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/{a}/header.jpg")),
            notes,
            maker,
        };
        (g, t)
    }

    fn all(self: &Arc<Self>) -> Vec<(ModGame, Traits)> {
        if let Some((at, v)) = self.cache.lock().as_ref() {
            if at.elapsed() < Duration::from_secs(60) {
                return v.clone();
            }
        }
        self.refresh_schema_later();
        let state = self.state.lock().clone();
        let mut out = Vec::new();
        for g in self.games.library() {
            if !matches!(g.source.as_str(), "steam" | "epic") {
                continue;
            }
            let Some(dir) = g.install_dir.clone().filter(|d| Path::new(d).is_dir()) else { continue };
            let app = match g.launch {
                crate::games::Launch::Steam { app_id } => Some(app_id),
                _ => None,
            };
            out.push(self.describe(&g.key, &g.name, &g.source, app, Path::new(&dir), online_only(&g), &state));
        }
        for c in &state.folders {
            if Path::new(&c.dir).is_dir() {
                out.push(self.describe(&c.key, &c.name, "folder", None, Path::new(&c.dir), false, &state));
            }
        }
        out.sort_by(|a, b| b.0.mods.cmp(&a.0.mods).then(a.0.name.to_lowercase().cmp(&b.0.name.to_lowercase())));
        *self.cache.lock() = Some((Instant::now(), out.clone()));
        out
    }

    /// Every game, the ones with mods first.
    pub fn games(self: &Arc<Self>) -> Vec<ModGame> {
        self.all().into_iter().map(|(g, _)| g).collect()
    }

    pub fn game(self: &Arc<Self>, key: &str) -> Result<(ModGame, Traits), String> {
        self.all().into_iter().find(|(g, _)| g.key == key).ok_or_else(|| "That game is no longer installed.".into())
    }

    /// The game as its folder is now (before changing it).
    fn game_now(self: &Arc<Self>, key: &str) -> Result<(ModGame, Traits), String> {
        *self.cache.lock() = None;
        self.game(key)
    }

    /// Add a game that isn't in Steam or Epic, by its folder.
    pub fn add_folder(self: &Arc<Self>, name: &str, dir: &str) -> Result<ModGame, String> {
        let p = Path::new(dir);
        if !p.is_dir() {
            return Err("That folder doesn't exist.".into());
        }
        let name = name.trim().chars().take(60).collect::<String>();
        let name = if name.is_empty() { p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Game".into()) } else { name };
        let key = {
            let mut s = self.state.lock();
            if let Some(c) = s.folders.iter().find(|c| Path::new(&c.dir) == p) {
                c.key.clone()
            } else {
                let key = format!("folder:{}", uuid::Uuid::new_v4().simple());
                s.folders.push(CustomGame { key: key.clone(), name, dir: dir.into() });
                self.save(&s)?;
                key
            }
        };
        self.changed(&key);
        self.game(&key).map(|(g, _)| g)
    }

    pub fn remove_folder(self: &Arc<Self>, key: &str) -> Result<(), String> {
        let mut s = self.state.lock();
        if s.mods.iter().any(|m| m.game == key) {
            return Err("Remove its mods first.".into());
        }
        s.folders.retain(|c| c.key != key);
        self.save(&s)?;
        drop(s);
        self.changed(key);
        Ok(())
    }

    /// The mods installed for a game, loaders first, each with any newer version.
    pub fn mods(self: &Arc<Self>, key: &str) -> Vec<InstalledMod> {
        let ts = self.game(key).ok().and_then(|(g, _)| g.thunderstore);
        let latest: HashMap<String, String> = ts.and_then(|c| self.listings.lock().get(&c).map(|l| l.packages.clone())).map(|p| p.iter().map(|p| (p.full_name.clone(), p.version.clone())).collect()).unwrap_or_default();
        let mut v: Vec<InstalledMod> = self.state.lock().mods.iter().filter(|m| m.game == key).cloned().collect();
        for m in &mut v {
            if let ModSource::Thunderstore { full_name, .. } = &m.source {
                m.update = latest.get(full_name).filter(|l| newer(l, &m.version)).cloned();
            }
        }
        v.sort_by(|a, b| b.loader.is_some().cmp(&a.loader.is_some()).then(b.installed_at.cmp(&a.installed_at)));
        v
    }

    // ---------- Thunderstore listings ----------

    fn listing_cache(&self, community: &str) -> PathBuf {
        self.dir.join("cache").join(format!("ts-{community}.json.gz"))
    }

    /// Load a community's mods now (blocking).
    pub fn load_listing(&self, community: &str) -> Result<Arc<Vec<Package>>, String> {
        {
            let l = self.listings.lock();
            if let Some(x) = l.get(community).filter(|x| !x.packages.is_empty() && now() - x.fetched < LISTING_TTL) {
                return Ok(x.packages.clone());
            }
        }
        // From disk if it's recent.
        if let Ok(b) = std::fs::read(self.listing_cache(community)) {
            if let Ok((at, pk)) = thunderstore::ungzip(&b).and_then(|p| serde_json::from_slice::<(i64, Vec<Package>)>(&p).map_err(|e| e.to_string())) {
                if now() - at < LISTING_TTL {
                    let arc = Arc::new(pk);
                    self.listings.lock().insert(community.into(), Listing { fetched: at, packages: arc.clone(), ..Default::default() });
                    return Ok(arc);
                }
            }
        }
        let c = community.to_string();
        let res = self.ts.listing(community, &mut |done, total| {
            if let Some(l) = self.listings.lock().get_mut(&c) {
                l.progress = Some((done, total));
            }
            self.events.emit("mods:listing", serde_json::json!({ "community": c, "done": done, "total": total }));
        });
        match res {
            Ok(pk) => {
                let at = now();
                if let Ok(json) = serde_json::to_vec(&(at, &pk)) {
                    use std::io::Write;
                    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
                    if e.write_all(&json).is_ok() {
                        if let Ok(gz) = e.finish() {
                            let _ = std::fs::create_dir_all(self.dir.join("cache"));
                            let _ = std::fs::write(self.listing_cache(community), gz);
                        }
                    }
                }
                let arc = Arc::new(pk);
                self.listings.lock().insert(community.into(), Listing { fetched: at, packages: arc.clone(), ..Default::default() });
                Ok(arc)
            }
            Err(e) => {
                let mut l = self.listings.lock();
                let x = l.entry(community.into()).or_default();
                x.loading = false;
                x.error = Some(e.clone());
                Err(e)
            }
        }
    }

    /// Search a game's Thunderstore mods. While its list is still coming,
    /// says so (and `mods:listing` events follow, then `mods:changed`).
    pub fn browse(self: &Arc<Self>, key: &str, query: &str, category: Option<&str>, sort: Sort, offset: usize, limit: usize) -> Result<Browse, String> {
        let (g, _) = self.game(key)?;
        let Some(community) = g.thunderstore.clone() else { return Err("Thunderstore has no mods for this game.".into()) };
        let packages = {
            let mut l = self.listings.lock();
            let x = l.entry(community.clone()).or_default();
            if x.packages.is_empty() || now() - x.fetched >= LISTING_TTL {
                if !x.loading {
                    x.loading = true;
                    x.error = None;
                    let me = self.clone();
                    let (c, k) = (community.clone(), key.to_string());
                    std::thread::Builder::new()
                        .name("mods-listing".into())
                        .spawn(move || {
                            let _ = me.load_listing(&c);
                            if let Some(x) = me.listings.lock().get_mut(&c) {
                                x.loading = false;
                            }
                            me.changed(&k);
                        })
                        .ok();
                }
                if x.packages.is_empty() {
                    return Ok(Browse { packages: vec![], total: 0, loading: x.loading, progress: x.progress, error: x.error.clone(), categories: vec![] });
                }
            }
            x.packages.clone()
        };
        let installed: HashMap<String, String> = self.state.lock().mods.iter().filter(|m| m.game == key).filter_map(|m| match &m.source {
            ModSource::Thunderstore { full_name, .. } => Some((full_name.clone(), m.version.clone())),
            _ => None,
        }).collect();
        let loaders = self.loader_packages();
        let hits = thunderstore::search(&packages, query, category, sort, false);
        let mut cats: Vec<String> = packages.iter().flat_map(|p| p.categories.iter().cloned()).collect::<HashSet<_>>().into_iter().collect();
        cats.sort();
        Ok(Browse {
            total: hits.len(),
            packages: hits
                .into_iter()
                .skip(offset)
                .take(limit.clamp(1, 100))
                .map(|p| BrowsePackage { installed: installed.get(&p.full_name).cloned(), is_loader: loaders.contains_key(&p.full_name), manager_link: thunderstore::manager_link(p), package: p.clone() })
                .collect(),
            loading: false,
            progress: None,
            error: None,
            categories: cats,
        })
    }

    // ---------- changing things ----------

    fn lock_game(&self, key: &str) -> Result<GameLock<'_>, String> {
        if !self.busy.lock().insert(key.to_string()) {
            return Err("OmniHub is already changing this game's mods.".into());
        }
        Ok(GameLock { hub: self, key: key.into() })
    }

    fn not_running(&self, g: &ModGame) -> Result<(), String> {
        if let Some(exe) = &g.exe {
            if !self.procs.pids_named(exe).is_empty() {
                return Err(format!("Close {} first.", g.name));
            }
        }
        Ok(())
    }

    fn owner_of(&self, game: &str, except: Option<&str>) -> impl Fn(Root, &str) -> Option<String> {
        let s = self.state.lock();
        let map: HashMap<(Root, String), String> = s
            .mods
            .iter()
            .filter(|m| m.game == game && m.enabled && Some(m.id.as_str()) != except)
            .flat_map(|m| m.files.iter().map(move |f| ((f.root, f.path.to_lowercase()), m.name.clone())))
            .collect();
        move |r: Root, p: &str| map.get(&(r, p.to_lowercase())).cloned()
    }

    fn tmp(&self) -> PathBuf {
        self.dir.join("tmp").join(uuid::Uuid::new_v4().simple().to_string())
    }

    /// Unpack, place and record one mod. `staging` is removed afterwards.
    fn put(&self, g: &ModGame, staging: &Path, layout: &Layout, folder: &str, mut m: InstalledMod, moved_out: &[String]) -> Result<InstalledMod, String> {
        let files = install::files_in(staging);
        let planned = install::plan(&files, layout, folder)?;
        if planned.iter().any(|p| p.root == Root::Data) && g.data_dir.is_none() {
            return Err("This game has no mod folder for that kind of mod.".into());
        }
        let dirs = Dirs { game: PathBuf::from(&g.dir), data: g.data_dir.as_ref().map(PathBuf::from) };
        let backup = self.dir.join("backups").join(&m.id);
        // An older loader file that would load twice: out of the way, into the backup.
        let mut moved = Vec::new();
        for rel in moved_out {
            let p = dirs.game.join(rel);
            if p.is_file() {
                let to = backup.join("moved").join(rel);
                std::fs::create_dir_all(to.parent().unwrap_or(&backup)).map_err(|e| e.to_string())?;
                std::fs::rename(&p, &to).or_else(|_| std::fs::copy(&p, &to).and_then(|_| std::fs::remove_file(&p))).map_err(|e| format!("Couldn't move the old {rel} aside: {e}"))?;
                moved.push((Root::Game, rel.clone()));
            }
        }
        let owner = self.owner_of(&g.key, None);
        let res = install::deploy(staging, &planned, &dirs, &backup, &owner);
        let _ = std::fs::remove_dir_all(staging);
        let (deployed, created) = match res {
            Ok(x) => x,
            Err(e) => {
                restore_moved(&moved, &dirs, &backup);
                return Err(e);
            }
        };
        m.files = deployed;
        m.created = created;
        m.moved = moved;
        m.game_dir = g.dir.clone();
        m.data_dir = g.data_dir.clone();
        m.enabled = true;
        m.installed_at = now();
        let mut s = self.state.lock();
        s.mods.push(m.clone());
        self.save(&s)?;
        Ok(m)
    }

    fn new_record(&self, g: &ModGame, name: &str, version: &str, source: ModSource) -> InstalledMod {
        InstalledMod {
            id: uuid::Uuid::new_v4().simple().to_string(),
            game: g.key.clone(),
            name: name.into(),
            version: version.into(),
            author: String::new(),
            description: String::new(),
            icon: String::new(),
            source,
            enabled: true,
            loader: None,
            dependency: false,
            installed_at: now(),
            game_dir: g.dir.clone(),
            data_dir: g.data_dir.clone(),
            files: vec![],
            created: vec![],
            moved: vec![],
            update: None,
        }
    }

    fn put_package(&self, g: &ModGame, t: &Traits, p: &Package, loader: Option<(LoaderKind, Layout)>, dependency: bool) -> Result<InstalledMod, String> {
        let bytes = self.ts.download(p)?;
        let tmp = self.tmp();
        let zip = tmp.with_extension("zip");
        std::fs::create_dir_all(tmp.parent().unwrap_or(&self.dir)).map_err(|e| e.to_string())?;
        std::fs::write(&zip, &bytes).map_err(|e| e.to_string())?;
        let r = install::unpack(&zip, &tmp);
        let _ = std::fs::remove_file(&zip);
        r?;
        let mut m = self.new_record(g, &p.name.replace('_', " "), &p.version, ModSource::Thunderstore { full_name: p.full_name.clone(), url: p.url.clone() });
        m.author = p.owner.clone();
        m.description = p.description.clone();
        m.icon = p.icon.clone();
        m.dependency = dependency;
        let (layout, moved) = match loader {
            Some((k, l)) => {
                m.loader = Some(k);
                let moved = if k == LoaderKind::Lovely { vec!["version.dll".to_string()] } else { vec![] };
                (l, moved)
            }
            None => (g.method.layout(t).ok_or("OmniHub doesn't install mods for this game.")?, vec![]),
        };
        self.put(g, &tmp, &layout, &p.full_name, m, &moved)
    }

    /// The loader package for a Thunderstore loader kind, and where its files sit.
    fn loader_package(&self, g: &ModGame, kind: LoaderKind) -> Result<(Package, Layout), String> {
        let (owner, name, folder, only) = match kind {
            LoaderKind::BepInEx => ("BepInEx", "BepInExPack", "BepInExPack", None),
            LoaderKind::BepInExIl2cpp => ("BepInEx", "BepInExPack_IL2CPP", "BepInExPack", None),
            LoaderKind::MelonLoader => ("LavaGang", "MelonLoader", "", None),
            LoaderKind::Lovely => ("Thunderstore", "lovely", "", Some(vec![".dll".to_string()])),
            _ => return Err("Not a Thunderstore loader.".into()),
        };
        // The game's own pack when its community has one (BepInExPack_Valheim…).
        if let (Some(c), LoaderKind::BepInEx | LoaderKind::BepInExIl2cpp) = (&g.thunderstore, kind) {
            let loaders = self.loader_packages();
            if let Ok(all) = self.load_listing(c) {
                let best = all.iter().filter(|p| loaders.get(&p.full_name).is_some_and(|(l, _)| l == "bepinex")).filter(|p| (kind == LoaderKind::BepInExIl2cpp) == p.name.to_lowercase().contains("il2cpp") || all.iter().filter(|q| loaders.contains_key(&q.full_name)).count() == 1).max_by_key(|p| p.downloads);
                if let Some(p) = best {
                    let folder = loaders.get(&p.full_name).map(|(_, f)| f.clone()).unwrap_or_default();
                    return Ok((p.clone(), Layout::Loader { folder, only: None }));
                }
            }
        }
        Ok((self.ts.latest(owner, name)?, Layout::Loader { folder: folder.into(), only }))
    }

    /// A GitHub project's latest release zip, checked against GitHub's
    /// published SHA-256 when it gives one.
    fn github_zip(&self, repo: &str, pick: &dyn Fn(&str) -> bool) -> Result<(String, String, Vec<u8>), String> {
        let v: serde_json::Value = serde_json::from_slice(&self.ts.fetch(&format!("{}/repos/{repo}/releases/latest", self.github), 4 << 20)?).map_err(|e| e.to_string())?;
        let tag = v["tag_name"].as_str().unwrap_or("latest").to_string();
        let asset = v["assets"].as_array().into_iter().flatten().find(|a| a["name"].as_str().is_some_and(|n| n.to_lowercase().ends_with(".zip") && pick(n))).ok_or_else(|| format!("{repo}'s latest release has no zip OmniHub knows."))?;
        let url = asset["browser_download_url"].as_str().unwrap_or_default();
        if !(url.starts_with("https://github.com/") || url.starts_with(&self.github)) {
            return Err("That download isn't on GitHub.".into());
        }
        let bytes = self.ts.fetch(url, 512 << 20)?;
        if let Some(want) = asset["digest"].as_str().and_then(|d| d.strip_prefix("sha256:")) {
            use sha2::Digest;
            let got = hex::encode(sha2::Sha256::digest(&bytes));
            if !got.eq_ignore_ascii_case(want) {
                return Err("The download didn't match GitHub's checksum, so it wasn't used.".into());
            }
        }
        Ok((tag, asset["name"].as_str().unwrap_or_default().to_string(), bytes))
    }

    fn install_loader_now(&self, g: &ModGame, t: &Traits, kind: LoaderKind) -> Result<InstalledMod, String> {
        self.progress(&g.key, format!("Installing {}…", kind.label()), 0, 1);
        match kind {
            LoaderKind::BepInEx | LoaderKind::BepInExIl2cpp | LoaderKind::MelonLoader | LoaderKind::Lovely => {
                let (p, layout) = self.loader_package(g, kind)?;
                self.put_package(g, t, &p, Some((kind, layout)), false)
            }
            LoaderKind::ScriptHookVDotNet | LoaderKind::Ue4ss => {
                let (repo, layout): (&str, Layout) = if kind == LoaderKind::Ue4ss {
                    ("UE4SS-RE/RE-UE4SS", Layout::GameSub(t.binaries.clone().ok_or("This game's program folder wasn't found.")?))
                } else {
                    ("scripthookvdotnet/scripthookvdotnet-nightly", Layout::Gta)
                };
                let pick = |n: &str| if kind == LoaderKind::Ue4ss { n.to_lowercase().starts_with("ue4ss_v") } else { true };
                let (tag, _, bytes) = self.github_zip(repo, &pick)?;
                let tmp = self.tmp();
                let zip = tmp.with_extension("zip");
                std::fs::create_dir_all(tmp.parent().unwrap_or(&self.dir)).map_err(|e| e.to_string())?;
                std::fs::write(&zip, &bytes).map_err(|e| e.to_string())?;
                let r = install::unpack(&zip, &tmp);
                let _ = std::fs::remove_file(&zip);
                r?;
                let mut m = self.new_record(g, kind.label(), tag.trim_start_matches('v'), ModSource::Github { repo: repo.into() });
                m.loader = Some(kind);
                m.author = repo.split('/').next().unwrap_or_default().into();
                self.put(g, &tmp, &layout, kind.label(), m, &[])
            }
            LoaderKind::ScriptHookV => Err("Script Hook V has to come from its author's page: download it there, then add the zip with “Add a file”.".into()),
        }
    }

    /// Install the loader mods need (when it isn't there yet).
    pub fn install_loader(self: &Arc<Self>, key: &str, kind: LoaderKind) -> Result<InstalledMod, String> {
        let (g, t) = self.game_now(key)?;
        let _lock = self.lock_game(key)?;
        self.not_running(&g)?;
        let _ = self.refresh_schema();
        let r = self.install_loader_now(&g, &t, kind);
        self.progress(key, "", 1, 1);
        self.changed(key);
        r
    }

    fn ensure_loaders(&self, g: &ModGame, t: &Traits) -> Result<(), String> {
        for l in &g.loaders {
            if !l.installed && l.auto && l.kind != LoaderKind::Ue4ss {
                self.install_loader_now(g, t, l.kind)?;
            }
        }
        Ok(())
    }

    /// Install a Thunderstore mod with what it needs (its dependencies, and
    /// the game's loader). Returns what was installed.
    pub fn install_package(self: &Arc<Self>, key: &str, full_name: &str) -> Result<Vec<InstalledMod>, String> {
        let (g, t) = self.game_now(key)?;
        if !g.ts_installs {
            return Err("OmniHub doesn't install this game's Thunderstore mods. Use r2modman instead.".into());
        }
        let _lock = self.lock_game(key)?;
        self.not_running(&g)?;
        let _ = self.refresh_schema();
        let community = g.thunderstore.clone().unwrap_or_default();
        let all = self.load_listing(&community)?;
        let map: HashMap<String, Package> = all.iter().map(|p| (p.full_name.clone(), p.clone())).collect();
        let mut skip: HashSet<String> = self.loader_packages().into_keys().collect();
        let installed: Vec<String> = self.state.lock().mods.iter().filter(|m| m.game == key).filter_map(|m| match &m.source {
            ModSource::Thunderstore { full_name, .. } => Some(full_name.clone()),
            _ => None,
        }).collect();
        if installed.iter().any(|n| n == full_name) {
            return Err("It's already installed.".into());
        }
        skip.extend(installed);
        if self.loader_packages().contains_key(full_name) {
            return Err("That's the mod loader itself: OmniHub installs it with the first mod.".into());
        }
        let order = thunderstore::resolve(full_name, &map, &skip)?;
        self.ensure_loaders(&g, &t)?;
        let mut done = Vec::new();
        let total = order.len();
        for (i, p) in order.iter().enumerate() {
            self.progress(key, format!("Installing {} ({} of {total})…", p.name.replace('_', " "), i + 1), i, total);
            match self.put_package(&g, &t, p, None, p.full_name != full_name) {
                Ok(m) => done.push(m),
                Err(e) => {
                    self.progress(key, "", total, total);
                    self.changed(key);
                    return Err(if done.is_empty() { e } else { format!("{e} ({} installed before that.)", done.iter().map(|m| m.name.clone()).collect::<Vec<_>>().join(", ")) });
                }
            }
        }
        self.progress(key, "", total, total);
        self.changed(key);
        Ok(done)
    }

    /// Install a mod from a file or folder the user picked.
    pub fn install_file(self: &Arc<Self>, key: &str, path: &str) -> Result<InstalledMod, String> {
        let (g, t) = self.game_now(key)?;
        let layout = g.method.layout(&t).ok_or("OmniHub doesn't install mods for this game.")?;
        let _lock = self.lock_game(key)?;
        self.not_running(&g)?;
        let src = Path::new(path);
        let tmp = self.tmp();
        self.progress(key, "Unpacking…", 0, 1);
        if let Err(e) = install::unpack(src, &tmp) {
            let _ = std::fs::remove_dir_all(&tmp);
            self.progress(key, "", 1, 1);
            return Err(e);
        }
        let stem = src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Mod".into());
        let mut m = self.new_record(&g, &stem.replace(['_', '-'], " "), "", ModSource::File { name: src.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default() });
        let files = install::files_in(&tmp);
        if g.method == Method::Gta && files.iter().any(|f| f.to_lowercase().ends_with("scripthookv.dll")) {
            m.loader = Some(LoaderKind::ScriptHookV);
            m.name = "Script Hook V".into();
        }
        let need_loaders = m.loader.is_none();
        if need_loaders {
            if let Err(e) = self.ensure_loaders(&g, &t) {
                let _ = std::fs::remove_dir_all(&tmp);
                self.progress(key, "", 1, 1);
                return Err(e);
            }
        }
        let r = self.put(&g, &tmp, &layout, &slug(&stem), m, &[]);
        self.progress(key, "", 1, 1);
        self.changed(key);
        r
    }

    /// Install (or update) files Claude made. `files` is the mod's folder.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn install_made(self: &Arc<Self>, key: &str, folder: &Path, name: &str, project: &str, version: &str, previous: Option<&str>, layout: Option<Layout>) -> Result<InstalledMod, String> {
        let (g, t) = self.game(key)?;
        let layout = match layout {
            Some(l) => l,
            None => g.method.layout(&t).ok_or("OmniHub doesn't install mods for this game.")?,
        };
        if let Some(id) = previous {
            if self.state.lock().mods.iter().any(|m| m.id == id) {
                self.uninstall(id)?;
            }
        }
        let (g, t) = self.game_now(key)?;
        let _lock = self.lock_game(key)?;
        self.not_running(&g)?;
        let _ = self.refresh_schema();
        self.ensure_loaders(&g, &t)?;
        // Balatro mods need Steamodded too (unless it's there already, from here or by hand).
        let mods_dir = Path::new(g.data_dir.as_deref().unwrap_or("")).join("Mods");
        let by_hand = std::fs::read_dir(&mods_dir).into_iter().flatten().flatten().any(|e| matches!(e.file_name().to_string_lossy().to_lowercase().as_str(), "smods" | "steamodded" | "smods-main"));
        if g.app_id == Some(BALATRO) && !by_hand && !self.state.lock().mods.iter().any(|m| m.game == key && matches!(&m.source, ModSource::Thunderstore { full_name, .. } if full_name == "Steamodded-Steamodded")) {
            if let Ok(all) = self.load_listing("balatro") {
                if let Some(p) = all.iter().find(|p| p.full_name == "Steamodded-Steamodded") {
                    self.put_package(&g, &t, p, None, true)?;
                }
            }
        }
        let tmp = self.tmp();
        install::unpack(folder, &tmp)?;
        let mut m = self.new_record(&g, name, version, ModSource::Claude { project: project.into() });
        m.author = "Claude".into();
        let r = self.put(&g, &tmp, &layout, &slug(name), m, &[]);
        self.changed(key);
        r
    }

    fn find(&self, id: &str) -> Result<InstalledMod, String> {
        self.state.lock().mods.iter().find(|m| m.id == id).cloned().ok_or_else(|| "That mod isn't installed any more.".into())
    }

    /// Switch a mod off (its files aside, originals back) or on again.
    pub fn set_enabled(self: &Arc<Self>, id: &str, on: bool) -> Result<(), String> {
        let m = self.find(id)?;
        if m.enabled == on {
            return Ok(());
        }
        if let Ok((g, _)) = self.game(&m.game) {
            self.not_running(&g)?;
        }
        let _lock = self.lock_game(&m.game)?;
        let dirs = m.dirs();
        let backup = self.dir.join("backups").join(&m.id);
        let aside = self.dir.join("off").join(&m.id);
        let mut files = m.files.clone();
        if on {
            let owner = self.owner_of(&m.game, Some(id));
            move_out(&m.moved, &dirs, &backup)?;
            install::enable(&mut files, &dirs, &backup, &aside, &owner)?;
        } else {
            install::disable(&files, &dirs, &backup, &aside)?;
            restore_moved(&m.moved, &dirs, &backup);
        }
        let mut s = self.state.lock();
        if let Some(x) = s.mods.iter_mut().find(|x| x.id == id) {
            x.enabled = on;
            x.files = files;
        }
        self.save(&s)?;
        drop(s);
        self.changed(&m.game);
        Ok(())
    }

    /// Switch every mod of a game off (to play online) or on.
    pub fn set_all(self: &Arc<Self>, key: &str, on: bool) -> Result<usize, String> {
        let ids: Vec<String> = self.state.lock().mods.iter().filter(|m| m.game == key && m.enabled != on).map(|m| m.id.clone()).collect();
        // Loaders go off last and come back on first.
        let mut ordered: Vec<(bool, String)> = ids.iter().map(|id| (self.find(id).map(|m| m.loader.is_some()).unwrap_or(false), id.clone())).collect();
        ordered.sort_by_key(|(l, _)| if on { !*l } else { *l });
        for (_, id) in &ordered {
            self.set_enabled(id, on)?;
        }
        Ok(ordered.len())
    }

    /// Remove a mod: its files go, the game's originals come back.
    pub fn uninstall(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let m = self.find(id)?;
        if let Ok((g, _)) = self.game(&m.game) {
            self.not_running(&g)?;
        }
        let _lock = self.lock_game(&m.game)?;
        let dirs = m.dirs();
        let backup = self.dir.join("backups").join(&m.id);
        let aside = self.dir.join("off").join(&m.id);
        if m.enabled {
            restore_moved(&m.moved, &dirs, &backup);
        }
        install::remove(&m.files, &m.created, &dirs, &backup, m.enabled)?;
        let _ = std::fs::remove_dir_all(&aside);
        let mut s = self.state.lock();
        s.mods.retain(|x| x.id != id);
        self.save(&s)?;
        drop(s);
        self.changed(&m.game);
        Ok(())
    }

    /// Update a Thunderstore mod to its newest version.
    pub fn update(self: &Arc<Self>, id: &str) -> Result<InstalledMod, String> {
        let m = self.find(id)?;
        let ModSource::Thunderstore { full_name, .. } = m.source.clone() else { return Err("Only Thunderstore mods update here.".into()) };
        let (g, t) = self.game_now(&m.game)?;
        let all = self.load_listing(g.thunderstore.as_deref().unwrap_or_default())?;
        let p = all.iter().find(|p| p.full_name == full_name).cloned().ok_or("Thunderstore no longer has it.")?;
        if !newer(&p.version, &m.version) {
            return Ok(m);
        }
        let was_on = m.enabled;
        self.uninstall(id)?;
        let _lock = self.lock_game(&m.game)?;
        let loader = m.loader.map(|k| self.loader_package(&g, k).map(|(_, l)| (k, l))).transpose()?;
        let mut n = self.put_package(&g, &t, &p, loader, m.dependency)?;
        drop(_lock);
        if !was_on {
            self.set_enabled(&n.id, false)?;
            n.enabled = false;
        }
        self.changed(&m.game);
        Ok(n)
    }

    /// The Steam Workshop items this game has (names from Steam, cached).
    pub fn workshop(self: &Arc<Self>, key: &str) -> Result<Vec<workshop::WorkshopItem>, String> {
        let (g, _) = self.game(key)?;
        let Some(app) = g.app_id else { return Ok(vec![]) };
        // <library>/steamapps/common/<game>
        let Some(steamapps) = Path::new(&g.dir).parent().and_then(Path::parent) else { return Ok(vec![]) };
        let mut items = workshop::installed(steamapps, app);
        if items.is_empty() {
            return Ok(items);
        }
        let cache_file = self.dir.join("cache").join("workshop.json");
        let mut cache: HashMap<String, (String, String)> = std::fs::read(&cache_file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let missing: Vec<usize> = items.iter().enumerate().filter(|(_, i)| !cache.contains_key(&i.id)).map(|(n, _)| n).collect();
        if !missing.is_empty() {
            let mut ask: Vec<workshop::WorkshopItem> = missing.iter().map(|&n| items[n].clone()).collect();
            if workshop::fill_details(&mut ask, &self.steam_api).is_ok() {
                for it in &ask {
                    cache.insert(it.id.clone(), (it.title.clone(), it.image.clone()));
                }
                let _ = std::fs::create_dir_all(self.dir.join("cache"));
                let _ = std::fs::write(&cache_file, serde_json::to_vec(&cache).unwrap_or_default());
            }
        }
        for it in &mut items {
            match cache.get(&it.id) {
                Some((t, img)) => {
                    it.title = t.clone();
                    it.image = img.clone();
                }
                None => it.title = format!("Workshop item {}", it.id),
            }
        }
        Ok(items)
    }

    /// For the mod maker: a game's details and the paths Claude may read.
    pub(crate) fn maker_context(self: &Arc<Self>, key: &str) -> Result<(ModGame, Traits), String> {
        self.game(key)
    }
}

struct GameLock<'a> {
    hub: &'a ModHub,
    key: String,
}

impl Drop for GameLock<'_> {
    fn drop(&mut self) {
        self.hub.busy.lock().remove(&self.key);
    }
}

/// Put back files an install moved aside.
fn restore_moved(moved: &[(Root, String)], dirs: &Dirs, backup: &Path) {
    for (root, rel) in moved {
        let Ok(base) = dirs.root(*root) else { continue };
        let from = backup.join("moved").join(rel);
        let to = base.join(rel);
        if from.is_file() && !to.exists() {
            let _ = std::fs::copy(&from, &to);
        }
    }
}

/// Move them aside again (a mod switched back on).
fn move_out(moved: &[(Root, String)], dirs: &Dirs, backup: &Path) -> Result<(), String> {
    for (root, rel) in moved {
        let p = dirs.root(*root)?.join(rel);
        if p.is_file() {
            let to = backup.join("moved").join(rel);
            std::fs::create_dir_all(to.parent().unwrap_or(backup)).map_err(|e| e.to_string())?;
            std::fs::copy(&p, &to).and_then(|_| std::fs::remove_file(&p)).map_err(|e| format!("Couldn't move {rel} aside: {e}"))?;
        }
    }
    Ok(())
}

/// Is version `a` newer than `b` ("1.10.0" > "1.9.2")?
pub fn newer(a: &str, b: &str) -> bool {
    let parts = |s: &str| s.split(['.', '-', '+']).map(|p| p.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    parts(a) > parts(b)
}

#[cfg(test)]
pub mod tests;
