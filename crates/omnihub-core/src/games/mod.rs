//! Game profiles: one click gets the PC out of a game's way (power plan,
//! background apps, notifications, GPU choice, Wi-Fi scanning…), starts the
//! game, and puts everything back when it closes. Plus Roblox flags and a
//! ping helper.
//!
//! Nothing here can make a server closer: the ping helper measures the
//! connection and removes the things on this PC that add lag and spikes.

pub mod configs;
pub mod ini;
pub mod library;
pub mod pc;
pub mod ping;
pub mod roblox;
pub mod tweaks;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;
use configs::{ConfigGame, ConfigOptions, ConfigStatus};
use library::InstalledGame;
use pc::{PcStatus, TweakId};
use crate::system::procs::{Priority, ProcessMonitor};
use ping::{PingResult, PingTarget};
use roblox::RobloxFlags;
use tweaks::{AdminOp, PowerPlan};

pub const FORTNITE_APP: &str = "fn%3A4fe75bbc5a674f4f9b356b5c90567da5%3AFortnite";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum GameKind {
    Fortnite,
    Roblox,
    Valorant,
    Cs2,
    Apex,
    RocketLeague,
    Gta5,
    CallOfDuty,
    League,
    Minecraft,
    #[default]
    Custom,
}

impl GameKind {
    pub fn all() -> [GameKind; 11] {
        use GameKind::*;
        [Fortnite, Roblox, Valorant, Cs2, Apex, RocketLeague, Gta5, CallOfDuty, League, Minecraft, Custom]
    }

    /// The game whose own settings file OmniHub can optimize.
    pub fn config_game(self) -> Option<ConfigGame> {
        match self {
            GameKind::Fortnite => Some(ConfigGame::Fortnite),
            GameKind::Minecraft => Some(ConfigGame::Minecraft),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Launch {
    /// Boost only; start the game yourself.
    #[default]
    None,
    Exe {
        path: String,
        #[serde(default)]
        args: String,
    },
    /// steam://, com.epicgames.launcher://, roblox://, https://
    Url { url: String },
    Steam { app_id: u32 },
    Epic { app: String },
    Riot { product: String },
    Roblox {
        #[serde(default)]
        place_id: Option<u64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Boost {
    pub power_plan: PowerPlan,
    /// Priority given to the game once it runs.
    pub priority: Option<Priority>,
    /// Programs closed for the session (executable names).
    pub close_apps: Vec<String>,
    /// Start them again afterwards.
    pub reopen_apps: bool,
    pub silence_notifications: bool,
    pub game_mode: bool,
    /// Per game: use the high-performance GPU.
    pub gpu_high_performance: bool,
    /// Per game: disable fullscreen optimizations.
    pub fullscreen_optimizations_off: bool,
    /// Session: no Wi-Fi background scans, low-latency mode.
    pub wifi_low_latency: bool,
    /// Per game, needs admin once: mark its traffic as high priority (DSCP 46).
    pub network_priority: bool,
    /// Per game, needs admin once: Windows starts it at High priority (works
    /// with anti-cheat, which can block changing it later).
    pub start_high_priority: bool,
    /// Fortnite, Minecraft: write the fastest in-game settings before each
    /// launch (see `configs`).
    pub game_settings: bool,
}

impl Default for Boost {
    fn default() -> Self {
        Boost {
            power_plan: PowerPlan::Ultimate,
            priority: Some(Priority::High),
            close_apps: Vec::new(),
            reopen_apps: true,
            silence_notifications: true,
            game_mode: true,
            gpu_high_performance: true,
            fullscreen_optimizations_off: false,
            wifi_low_latency: true,
            network_priority: false,
            start_high_priority: false,
            game_settings: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GameProfile {
    pub id: String,
    pub name: String,
    pub kind: GameKind,
    pub launch: Launch,
    /// The game's executable name, watched to know when it closes.
    pub process: String,
    /// Its full path when known (needed for the per-game Windows settings).
    pub exe_path: Option<String>,
    pub boost: Boost,
    /// Custom ping host; otherwise the game's regions or general targets.
    pub ping_host: Option<String>,
    pub roblox: RobloxFlags,
    pub last_played: Option<i64>,
}

/// A ready-made profile for a game.
pub fn template(kind: GameKind) -> GameProfile {
    let (name, launch, process) = match kind {
        GameKind::Fortnite => ("Fortnite", Launch::Epic { app: FORTNITE_APP.into() }, "FortniteClient-Win64-Shipping.exe"),
        GameKind::Roblox => ("Roblox", Launch::Roblox { place_id: None }, roblox::PLAYER_EXE),
        GameKind::Valorant => ("VALORANT", Launch::Riot { product: "valorant".into() }, "VALORANT-Win64-Shipping.exe"),
        GameKind::Cs2 => ("Counter-Strike 2", Launch::Steam { app_id: 730 }, "cs2.exe"),
        GameKind::Apex => ("Apex Legends", Launch::Steam { app_id: 1_172_470 }, "r5apex.exe"),
        GameKind::RocketLeague => ("Rocket League", Launch::Epic { app: "Sugar".into() }, "RocketLeague.exe"),
        GameKind::Gta5 => ("GTA V", Launch::Steam { app_id: 271_590 }, "GTA5.exe"),
        GameKind::CallOfDuty => ("Call of Duty", Launch::Steam { app_id: 1_938_090 }, "cod.exe"),
        GameKind::League => ("League of Legends", Launch::Riot { product: "league_of_legends".into() }, "League of Legends.exe"),
        // Java edition; the launcher starts the game.
        GameKind::Minecraft => ("Minecraft", Launch::None, "javaw.exe"),
        GameKind::Custom => ("My game", Launch::None, ""),
    };
    GameProfile { id: String::new(), name: name.into(), kind, launch, process: process.into(), exe_path: None, boost: Boost::default(), ping_host: None, roblox: RobloxFlags::default(), last_played: None }
}

/// Where a known game's executable is, when it can be found.
fn locate(kind: GameKind) -> Option<String> {
    let p = match kind {
        GameKind::Fortnite => tweaks::epic_install("Fortnite")?.join(r"FortniteGame\Binaries\Win64\FortniteClient-Win64-Shipping.exe"),
        GameKind::RocketLeague => tweaks::epic_install("Sugar")?.join(r"Binaries\Win64\RocketLeague.exe"),
        GameKind::Roblox => PathBuf::from(roblox::detect().player?),
        _ => return None,
    };
    p.is_file().then(|| p.to_string_lossy().into_owned())
}

/// Programs a launch needs running (never closed by a boost).
fn needed_by(launch: &Launch) -> &'static [&'static str] {
    match launch {
        Launch::Epic { .. } => &["EpicGamesLauncher.exe", "EpicWebHelper.exe"],
        Launch::Steam { .. } => &["steam.exe", "steamwebhelper.exe", "steamservice.exe"],
        Launch::Riot { .. } => &["RiotClientServices.exe", "Riot Client.exe", "RiotClientUx.exe", "RiotClientCrashHandler.exe"],
        _ => &[],
    }
}

fn url_allowed(url: &str) -> bool {
    ["steam://", "com.epicgames.launcher://", "roblox://", "roblox-player:", "https://", "http://", "battlenet://", "uplay://", "origin2://", "minecraft://"].iter().any(|p| url.starts_with(p)) && !url.chars().any(|c| c.is_whitespace() || c == '"')
}

/// The URL (or program) that starts a profile's game.
pub fn launch_target(launch: &Launch) -> Result<Option<(String, String)>, String> {
    Ok(Some(match launch {
        Launch::None => return Ok(None),
        Launch::Exe { path, args } => {
            if !Path::new(path).is_file() {
                return Err(format!("{path} was not found."));
            }
            (path.clone(), args.clone())
        }
        Launch::Url { url } => {
            if !url_allowed(url) {
                return Err("That link type can't be opened from a game profile.".into());
            }
            (url.clone(), String::new())
        }
        Launch::Steam { app_id } => (format!("steam://rungameid/{app_id}"), String::new()),
        Launch::Epic { app } => {
            if !app.chars().all(|c| c.is_ascii_alphanumeric() || c == '%') {
                return Err("That Epic game id is not valid.".into());
            }
            (format!("com.epicgames.launcher://apps/{app}?action=launch&silent=true"), String::new())
        }
        Launch::Riot { product } => {
            if !product.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err("That Riot product is not valid.".into());
            }
            let client = tweaks::riot_client().ok_or("Riot Client is not installed.")?;
            (client.to_string_lossy().into_owned(), format!("--launch-product={product} --launch-patchline=live"))
        }
        Launch::Roblox { place_id: Some(id) } => (format!("roblox://experiences/start?placeId={id}"), String::new()),
        Launch::Roblox { place_id: None } => match roblox::detect().player {
            Some(p) => (p, String::new()),
            None => ("roblox://".into(), String::new()),
        },
    }))
}

fn start(target: &str, args: &str) -> std::io::Result<()> {
    let path = Path::new(target);
    if path.is_file() {
        let mut c = std::process::Command::new(path);
        c.args(split_args(args));
        if let Some(dir) = path.parent() {
            c.current_dir(dir);
        }
        c.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        let mut child = c.spawn()?;
        // Reap it when it exits (no zombie left looking like a running game).
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        return Ok(());
    }
    crate::system::shell::shell_execute(target, args)
}

/// Split a command line on spaces, keeping "quoted parts" together.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for c in s.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

// ---------- session ----------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Starting,
    /// Launched; waiting for the game's window.
    Waiting,
    Playing,
    /// Boost without a game to watch: until stopped.
    Boosted,
    Restoring,
    Ended,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StepStatus {
    Done,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub id: String,
    pub label: String,
    pub status: StepStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub profile_id: String,
    pub name: String,
    pub phase: Phase,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub launched: bool,
    pub steps: Vec<Step>,
    /// Restore steps, once the session ends.
    pub restored: Vec<Step>,
    pub message: Option<String>,
}

impl Session {
    pub fn active(&self) -> bool {
        self.phase != Phase::Ended
    }
}

/// What a session changed, on disk so a crash or power cut doesn't leave
/// the PC boosted.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
struct Journal {
    profile_id: String,
    plan_before: Option<String>,
    toasts_before: Option<Option<u32>>,
    /// (name, executable) of closed programs.
    closed: Vec<(String, Option<String>)>,
    reopen: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Store {
    profiles: Vec<GameProfile>,
    /// The "Ultimate Performance" plan OmniHub created, reused next time.
    ultimate_plan: Option<String>,
    /// Roblox flags OmniHub wrote last, replaced on the next write.
    roblox_managed: Vec<String>,
    /// How Fortnite and Minecraft are set up by "Optimize".
    config_options: ConfigOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SaveResult {
    pub profile: GameProfile,
    pub warnings: Vec<String>,
}

/// Per-game Windows settings currently in place.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub network_priority: bool,
    pub start_high_priority: bool,
    pub on_wifi: Option<bool>,
    pub running: bool,
}

struct Inner {
    store: Store,
    session: Option<Session>,
    stop: Option<Arc<AtomicBool>>,
}

pub struct GameHub {
    store_path: PathBuf,
    journal_path: PathBuf,
    /// Copies of the games' own settings from before OmniHub's first change.
    backups_dir: PathBuf,
    pc_journal: PathBuf,
    config_paths: Mutex<std::collections::HashMap<ConfigGame, PathBuf>>,
    library_roots: Mutex<library::Roots>,
    events: EventBus,
    procs: Arc<ProcessMonitor>,
    inner: Mutex<Inner>,
    roblox_roots: Mutex<Vec<(PathBuf, Option<&'static str>)>>,
    /// Poll interval while watching a game (shorter in tests).
    poll: Duration,
    /// How long to wait for a launched game to show up.
    wait_limit: Duration,
}

fn write_json(path: &Path, v: &impl Serialize) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v).map_err(std::io::Error::other)?)?;
    std::fs::rename(&tmp, path)
}

fn step(id: &str, label: &str, r: Result<String, String>) -> Step {
    let (status, detail) = match r {
        Ok(d) if d.starts_with("skip:") => (StepStatus::Skipped, d[5..].trim().to_string()),
        Err(e) if e == "Windows only" => (StepStatus::Skipped, e),
        Ok(d) => (StepStatus::Done, d),
        Err(e) => (StepStatus::Failed, e),
    };
    Step { id: id.into(), label: label.into(), status, detail }
}

fn short(name: &str) -> &str {
    name.strip_suffix(".exe").or_else(|| name.strip_suffix(".EXE")).unwrap_or(name)
}

impl GameHub {
    pub fn new(data_dir: &Path, events: EventBus, procs: Arc<ProcessMonitor>) -> Arc<Self> {
        let store_path = data_dir.join("games.json");
        let store = std::fs::read(&store_path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Arc::new(GameHub {
            journal_path: data_dir.join("games-session.json"),
            backups_dir: data_dir.join("game-settings-backup"),
            pc_journal: data_dir.join("pc-tweaks.json"),
            config_paths: Mutex::new(ConfigGame::all().into_iter().filter_map(|g| Some((g, g.default_path()?))).collect()),
            library_roots: Mutex::new(library::Roots::system()),
            store_path,
            events,
            procs,
            inner: Mutex::new(Inner { store, session: None, stop: None }),
            roblox_roots: Mutex::new(roblox::roots()),
            poll: Duration::from_secs(2),
            wait_limit: Duration::from_secs(600),
        })
    }

    /// For tests: quicker polling and Roblox in a temporary folder.
    pub fn with_test_timing(data_dir: &Path, events: EventBus, procs: Arc<ProcessMonitor>, poll: Duration, wait_limit: Duration) -> Arc<Self> {
        let hub = Self::new(data_dir, events, procs);
        let mut h = Arc::try_unwrap(hub).unwrap_or_else(|_| unreachable!());
        h.poll = poll;
        h.wait_limit = wait_limit;
        Arc::new(h)
    }

    pub fn set_roblox_roots(&self, roots: Vec<(PathBuf, Option<&'static str>)>) {
        *self.roblox_roots.lock() = roots;
    }

    fn save_store(&self, store: &Store) {
        if let Err(e) = write_json(&self.store_path, store) {
            tracing::warn!("games: could not save profiles: {e}");
        }
    }

    pub fn profiles(&self) -> Vec<GameProfile> {
        self.inner.lock().store.profiles.clone()
    }

    pub fn profile(&self, id: &str) -> Option<GameProfile> {
        self.inner.lock().store.profiles.iter().find(|p| p.id == id).cloned()
    }

    pub fn session(&self) -> Option<Session> {
        self.inner.lock().session.clone()
    }

    /// Add a profile from a template, filling in where the game is installed.
    pub fn create(&self, kind: GameKind) -> GameProfile {
        let mut p = template(kind);
        p.id = uuid::Uuid::new_v4().to_string();
        if kind == GameKind::Roblox {
            p.exe_path = roblox::detect_in(&self.roblox_roots.lock()).player;
        } else {
            p.exe_path = locate(kind);
        }
        let mut inner = self.inner.lock();
        // "Fortnite 2" when there is one already.
        let n = inner.store.profiles.iter().filter(|q| q.kind == kind && kind != GameKind::Custom).count();
        if n > 0 {
            p.name = format!("{} {}", p.name, n + 1);
        }
        inner.store.profiles.push(p.clone());
        self.save_store(&inner.store);
        p
    }

    pub fn save(&self, mut p: GameProfile) -> Result<SaveResult, String> {
        p.name = p.name.trim().chars().take(60).collect();
        if p.name.is_empty() {
            return Err("Give the profile a name.".into());
        }
        p.process = p.process.trim().to_string();
        if let (true, Launch::Exe { path, .. }) = (p.process.is_empty(), &p.launch) {
            p.process = Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        }
        if !p.process.is_empty() && !tweaks::valid_exe_name(&p.process) {
            return Err(format!("“{}” is not a program name like Game.exe.", p.process));
        }
        if let Launch::Exe { path, .. } = &p.launch {
            if p.exe_path.is_none() {
                p.exe_path = Some(path.clone());
            }
        }
        if let Launch::Url { url } = &p.launch {
            if !url_allowed(url) {
                return Err("Use a steam://, com.epicgames.launcher://, roblox:// or https:// link.".into());
            }
        }
        if let Some(h) = &p.ping_host {
            if h.trim().is_empty() {
                p.ping_host = None;
            } else if !ping::valid_host(h.trim()) {
                return Err("The ping host should be a name like eu.example.com or an IP address.".into());
            }
        }
        p.boost.close_apps.retain(|a| tweaks::valid_exe_name(a) && !crate::system::procs::is_protected(a));
        let old = self.profile(&p.id).ok_or("That profile no longer exists.")?;
        let mut warnings = Vec::new();
        // Per-game settings turned off: take them away now.
        if let Some(exe) = old.exe_path.as_deref() {
            if old.boost.gpu_high_performance && !p.boost.gpu_high_performance {
                let _ = tweaks::set_gpu_high_performance(exe, false);
            }
            if old.boost.fullscreen_optimizations_off && !p.boost.fullscreen_optimizations_off {
                let _ = tweaks::set_fullscreen_optimizations_off(exe, false);
            }
        }
        let mut ops = Vec::new();
        if !old.process.is_empty() {
            if old.boost.network_priority && !p.boost.network_priority && tweaks::qos_enabled(&old.process) {
                ops.push(AdminOp::QosOff(old.process.clone()));
            }
            if old.boost.start_high_priority && !p.boost.start_high_priority && tweaks::start_high_enabled(&old.process) {
                ops.push(AdminOp::StartHighOff(old.process.clone()));
            }
        }
        if let Err(e) = tweaks::run_admin(&ops) {
            warnings.push(format!("Saved, but the network/start priority is still on: {e}"));
        }
        let mut inner = self.inner.lock();
        if let Some(slot) = inner.store.profiles.iter_mut().find(|q| q.id == p.id) {
            p.last_played = slot.last_played;
            *slot = p.clone();
        }
        self.save_store(&inner.store);
        Ok(SaveResult { profile: p, warnings })
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        if self.session().is_some_and(|s| s.active() && s.profile_id == id) {
            return Err("Stop the boost first.".into());
        }
        let p = self.profile(id).ok_or("That profile no longer exists.")?;
        if let Some(exe) = p.exe_path.as_deref() {
            let _ = tweaks::set_gpu_high_performance(exe, false);
            let _ = tweaks::set_fullscreen_optimizations_off(exe, false);
        }
        let mut ops = Vec::new();
        if !p.process.is_empty() && tweaks::qos_enabled(&p.process) {
            ops.push(AdminOp::QosOff(p.process.clone()));
        }
        if !p.process.is_empty() && tweaks::start_high_enabled(&p.process) {
            ops.push(AdminOp::StartHighOff(p.process.clone()));
        }
        let _ = tweaks::run_admin(&ops);
        let mut inner = self.inner.lock();
        inner.store.profiles.retain(|q| q.id != id);
        self.save_store(&inner.store);
        Ok(())
    }

    /// Per-game settings already in place, and whether the game runs.
    pub fn state(&self, id: &str) -> Option<GameState> {
        let p = self.profile(id)?;
        let has = !p.process.is_empty();
        Some(GameState {
            network_priority: has && tweaks::qos_enabled(&p.process),
            start_high_priority: has && tweaks::start_high_enabled(&p.process),
            on_wifi: tweaks::on_wifi(),
            running: has && !self.procs.pids_named(&p.process).is_empty(),
        })
    }

    pub fn roblox_status(&self) -> roblox::RobloxInstall {
        let mut r = roblox::detect_in(&self.roblox_roots.lock());
        r.running = !self.procs.pids_named(roblox::PLAYER_EXE).is_empty();
        r
    }

    /// Write a profile's Roblox flags now. Returns the files written.
    pub fn write_roblox(&self, flags: &RobloxFlags) -> Result<Vec<String>, String> {
        let map = if flags.enabled { flags.to_flags() } else { serde_json::Map::new() };
        let managed = self.inner.lock().store.roblox_managed.clone();
        let files = roblox::write_flags(&self.roblox_roots.lock(), &map, &managed).map_err(|e| e.to_string())?;
        let mut inner = self.inner.lock();
        inner.store.roblox_managed = map.keys().cloned().collect();
        self.save_store(&inner.store);
        Ok(files.into_iter().map(|f| f.to_string_lossy().into_owned()).collect())
    }

    /// The ping targets for a profile: its custom host, or its game's regions.
    pub fn ping_targets(&self, id: Option<&str>) -> Vec<PingTarget> {
        let p = id.and_then(|i| self.profile(i));
        match p {
            Some(GameProfile { ping_host: Some(h), .. }) => vec![PingTarget { id: "custom".into(), label: h.clone(), host: h }],
            Some(GameProfile { kind: GameKind::Fortnite, .. }) => ping::fortnite_regions(),
            _ => ping::general_targets(),
        }
    }

    pub fn ping(&self, id: Option<&str>, host: Option<&str>) -> Vec<PingResult> {
        let targets = match host.map(str::trim).filter(|h| !h.is_empty()) {
            Some(h) => vec![PingTarget { id: "custom".into(), label: h.into(), host: h.into() }],
            None => self.ping_targets(id),
        };
        ping::measure_all(&targets, 10, Duration::from_millis(150))
    }

    // ---------- the games' own settings ----------

    /// For tests: the games' settings files and the launchers' records in
    /// temporary folders.
    pub fn set_test_paths(&self, configs: Vec<(ConfigGame, PathBuf)>, roots: library::Roots) {
        *self.config_paths.lock() = configs.into_iter().collect();
        *self.library_roots.lock() = roots;
    }

    fn config_path(&self, game: ConfigGame) -> Option<PathBuf> {
        self.config_paths.lock().get(&game).cloned()
    }

    pub fn config_options(&self) -> ConfigOptions {
        self.inner.lock().store.config_options.clone()
    }

    pub fn config_status(&self, game: ConfigGame) -> ConfigStatus {
        let running = !self.procs.pids_named(game.process()).is_empty();
        configs::status(game, &self.config_options(), self.config_path(game).as_deref(), &self.backups_dir, running)
    }

    pub fn set_config_options(&self, opts: ConfigOptions) {
        let mut inner = self.inner.lock();
        inner.store.config_options = opts;
        self.save_store(&inner.store);
    }

    /// Write the fastest settings into the game's file (not while it runs:
    /// it would overwrite them when it closes).
    pub fn config_apply(&self, game: ConfigGame) -> Result<Vec<configs::Change>, String> {
        if !self.procs.pids_named(game.process()).is_empty() {
            return Err(format!("Close {} first — it saves its own settings when it closes and would undo these.", game.label()));
        }
        let path = self.config_path(game).ok_or("Can't tell where the game keeps its settings.")?;
        configs::apply(game, &self.config_options(), &path, &self.backups_dir)
    }

    pub fn config_restore(&self, game: ConfigGame) -> Result<(), String> {
        if !self.procs.pids_named(game.process()).is_empty() {
            return Err(format!("Close {} first.", game.label()));
        }
        let path = self.config_path(game).ok_or("Can't tell where the game keeps its settings.")?;
        configs::restore(game, &path, &self.backups_dir)
    }

    // ---------- installed games ----------

    /// Games installed on this PC, each with the profile already made for it.
    pub fn library(&self) -> Vec<InstalledGame> {
        let roots = self.library_roots.lock().clone();
        let roblox = roblox::detect_in(&self.roblox_roots.lock()).player;
        let mut games = library::scan(&roots, roblox);
        let profiles = self.profiles();
        for g in &mut games {
            g.profile_id = profiles
                .iter()
                .find(|p| (g.kind != GameKind::Custom && p.kind == g.kind) || (!g.process.is_empty() && p.process.eq_ignore_ascii_case(&g.process)) || (p.launch != Launch::None && p.launch == g.launch))
                .map(|p| p.id.clone());
        }
        games
    }

    /// Make a profile for an installed game.
    pub fn add_installed(&self, key: &str) -> Result<GameProfile, String> {
        let g = self.library().into_iter().find(|g| g.key == key).ok_or("That game is no longer installed.")?;
        if let Some(id) = &g.profile_id {
            return self.profile(id).ok_or_else(|| "That profile no longer exists.".into());
        }
        let mut p = self.create(g.kind);
        if g.kind == GameKind::Custom || p.exe_path.is_none() {
            p.name = g.name.chars().take(60).collect();
            p.launch = g.launch.clone();
            if !g.process.is_empty() && tweaks::valid_exe_name(&g.process) {
                p.process = g.process.clone();
            }
            p.exe_path = g.exe_path.clone().or(p.exe_path);
            let mut inner = self.inner.lock();
            if let Some(slot) = inner.store.profiles.iter_mut().find(|q| q.id == p.id) {
                *slot = p.clone();
            }
            self.save_store(&inner.store);
        }
        Ok(p)
    }

    // ---------- PC-wide settings ----------

    pub fn pc_status(&self) -> PcStatus {
        let plan = self.inner.lock().store.ultimate_plan.clone();
        pc::status(&self.pc_journal, plan.as_deref())
    }

    pub fn pc_set(&self, id: TweakId, on: bool) -> Result<PcStatus, String> {
        let plan = self.inner.lock().store.ultimate_plan.clone();
        if let Some(created) = pc::set(id, on, &self.pc_journal, plan.as_deref())? {
            let mut inner = self.inner.lock();
            inner.store.ultimate_plan = Some(created);
            self.save_store(&inner.store);
        }
        Ok(self.pc_status())
    }

    fn emit(&self) {
        let s = self.inner.lock().session.clone();
        self.events.emit("games:session", &s);
    }

    fn update(&self, f: impl FnOnce(&mut Session)) {
        if let Some(s) = self.inner.lock().session.as_mut() {
            f(s);
        }
        self.emit();
    }

    /// Boost the PC for a profile and (with `launch`) start its game. The
    /// boost ends, and everything is put back, when the game closes or on
    /// `stop()`.
    pub fn play(self: &Arc<Self>, id: &str, launch: bool) -> Result<Session, String> {
        let p = self.profile(id).ok_or("That profile no longer exists.")?;
        let stop = Arc::new(AtomicBool::new(false));
        {
            let mut inner = self.inner.lock();
            if let Some(s) = inner.session.as_ref().filter(|s| s.active()) {
                return Err(format!("{} is already boosted — stop that first.", s.name));
            }
            inner.session = Some(Session { profile_id: p.id.clone(), name: p.name.clone(), phase: Phase::Starting, started_at: crate::db::now(), ended_at: None, launched: false, steps: Vec::new(), restored: Vec::new(), message: None });
            inner.stop = Some(stop.clone());
        }
        self.emit();
        let hub = self.clone();
        std::thread::Builder::new().name("game-boost".into()).spawn(move || hub.run(p, launch, stop)).map_err(|e| e.to_string())?;
        Ok(self.session().unwrap_or_else(|| unreachable!()))
    }

    /// End the boost (the game keeps running).
    pub fn stop(&self) -> bool {
        match self.inner.lock().stop.as_ref() {
            Some(s) => {
                s.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }

    /// Add a step (or update it, when it runs again — e.g. once the game's
    /// location is known).
    fn push(&self, s: Step) {
        self.update(|x| match x.steps.iter_mut().find(|o| o.id == s.id) {
            Some(o) => *o = s,
            None => x.steps.push(s),
        });
    }

    fn run(self: Arc<Self>, mut p: GameProfile, launch: bool, stop: Arc<AtomicBool>) {
        let b = p.boost.clone();
        let mut j = Journal { profile_id: p.id.clone(), reopen: b.reopen_apps, ..Default::default() };
        let save_journal = |j: &Journal| {
            if let Err(e) = write_json(&self.journal_path, j) {
                tracing::warn!("games: journal: {e}");
            }
        };
        save_journal(&j);

        // 1. Background programs.
        if !b.close_apps.is_empty() {
            let keep: Vec<&str> = needed_by(&p.launch).to_vec();
            let running = self.procs.usage(crate::system::procs::ProcessSort::Name, 10_000, false).processes;
            let mut closed = Vec::new();
            let mut failed = Vec::new();
            for name in &b.close_apps {
                if keep.iter().any(|k| k.eq_ignore_ascii_case(name)) || name.eq_ignore_ascii_case(&p.process) {
                    continue;
                }
                let Some(g) = running.iter().find(|g| g.name.eq_ignore_ascii_case(name) && g.can_end) else { continue };
                match self.procs.end(&g.name) {
                    Ok(_) => {
                        closed.push(short(&g.name).to_string());
                        j.closed.push((g.name.clone(), g.exe.clone()));
                    }
                    Err(_) => failed.push(short(&g.name).to_string()),
                }
            }
            save_journal(&j);
            let r = match (closed.is_empty(), failed.is_empty()) {
                (true, true) => Ok("skip: none of them were running".to_string()),
                (false, true) => Ok(format!("Closed {}", closed.join(", "))),
                (_, false) => Err(format!("Could not close {}{}", failed.join(", "), if closed.is_empty() { String::new() } else { format!(" (closed {})", closed.join(", ")) })),
            };
            self.push(step("apps", "Background apps", r));
        }

        // 2. Power plan.
        if b.power_plan != PowerPlan::Keep {
            let before = tweaks::active_plan();
            let remembered = self.inner.lock().store.ultimate_plan.clone();
            let r = tweaks::activate_plan(b.power_plan, remembered.as_deref()).map(|(guid, label, created)| {
                if let Some(c) = created {
                    let mut inner = self.inner.lock();
                    inner.store.ultimate_plan = Some(c);
                    self.save_store(&inner.store);
                }
                if before.as_deref() != Some(guid.as_str()) {
                    j.plan_before = before.clone();
                }
                label.to_string()
            });
            save_journal(&j);
            self.push(step("power", "Power plan", r));
        }

        // 3. Notifications.
        if b.silence_notifications {
            let r = tweaks::silence_notifications().map(|before| {
                j.toasts_before = Some(before);
                "Pop-ups paused".to_string()
            });
            save_journal(&j);
            self.push(step("notifications", "Notifications", r));
        }

        // 4. Game Mode.
        if b.game_mode {
            self.push(step("gamemode", "Game Mode", tweaks::ensure_game_mode().map(|changed| if changed { "Turned on".into() } else { "skip: already on".into() })));
        }

        // 5. Per-game GPU and fullscreen settings.
        self.per_exe_settings(&p);

        // 6. Admin-only per-game settings (one prompt, only when missing).
        let mut ops = Vec::new();
        if !p.process.is_empty() {
            if b.network_priority && !tweaks::qos_enabled(&p.process) {
                ops.push(AdminOp::QosOn(p.process.clone()));
            }
            if b.start_high_priority && !tweaks::start_high_enabled(&p.process) {
                ops.push(AdminOp::StartHighOn(p.process.clone()));
            }
        }
        if !ops.is_empty() {
            let label = ops.iter().map(|o| if matches!(o, AdminOp::QosOn(_)) { "network priority" } else { "start at High priority" }).collect::<Vec<_>>().join(" + ");
            self.push(step("admin", "Network & start priority", tweaks::run_admin(&ops).map(|_| format!("Turned on {label} (kept for next time)"))));
        } else if b.network_priority || b.start_high_priority {
            self.push(step("admin", "Network & start priority", Ok("skip: already on".into())));
        }

        // 7. Wi-Fi low-latency mode, held for the session.
        let mut wifi = None;
        if b.wifi_low_latency {
            let r = tweaks::WifiLowLatency::start().map(|(guard, names)| {
                wifi = Some(guard);
                format!("Low-latency mode on {names}")
            });
            let r = match r {
                Err(e) if e.contains("nothing to do") || e.contains("no Wi-Fi") => Ok(format!("skip: {e}")),
                r => r,
            };
            self.push(step("wifi", "Wi-Fi", r));
        }

        // 8. Roblox flags.
        if p.kind == GameKind::Roblox && p.roblox.enabled {
            let r = self.write_roblox(&p.roblox).map(|files| format!("{} flags written ({} file{})", p.roblox.to_flags().len(), files.len(), if files.len() == 1 { "" } else { "s" }));
            self.push(step("roblox", "Roblox flags", r));
        }

        // 8b. The game's own settings (only while it is closed).
        if let (Some(game), true) = (p.kind.config_game(), b.game_settings) {
            let r = if !self.procs.pids_named(game.process()).is_empty() {
                Ok(format!("skip: {} is already running", game.label()))
            } else {
                self.config_apply(game).map(|c| if c.is_empty() { "skip: already set for the most FPS".to_string() } else { format!("{} settings set for the most FPS", c.len()) })
            };
            let r = match r {
                Err(e) if e.contains("Start it once") => Ok(format!("skip: {e}")),
                r => r,
            };
            self.push(step("settings", "Game settings", r));
        }

        // 9. Launch.
        let mut launched = false;
        if launch {
            let r = launch_target(&p.launch).and_then(|t| match t {
                None => Ok("skip: no launcher set — start the game yourself".to_string()),
                Some((target, args)) => start(&target, &args).map(|_| format!("Started {}", p.name)).map_err(|e| e.to_string()),
            });
            launched = matches!(&r, Ok(d) if !d.starts_with("skip:"));
            self.push(step("launch", "Launch", r));
            let stamp = crate::db::now();
            let mut inner = self.inner.lock();
            if let Some(q) = inner.store.profiles.iter_mut().find(|q| q.id == p.id) {
                q.last_played = Some(stamp);
            }
            self.save_store(&inner.store);
        }
        self.update(|s| s.launched = launched);

        // 10. Watch the game (boost only: until it has been played and closed).
        if !p.process.is_empty() {
            self.update(|s| s.phase = Phase::Waiting);
            let t0 = Instant::now();
            let mut seen = false;
            let mut missing = 0;
            loop {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let running = !self.procs.pids_named(&p.process).is_empty();
                if running && !seen {
                    seen = true;
                    missing = 0;
                    self.update(|s| s.phase = Phase::Playing);
                    self.on_game_started(&mut p);
                } else if !running && seen {
                    // Launchers sometimes restart the game; allow a moment.
                    missing += 1;
                    if missing >= 2 {
                        self.update(|s| s.message = Some(format!("{} closed", p.name)));
                        break;
                    }
                } else if running {
                    missing = 0;
                } else if launched && t0.elapsed() > self.wait_limit {
                    self.update(|s| s.message = Some(format!("{} didn't start, so the boost ended.", p.name)));
                    break;
                }
                std::thread::sleep(self.poll);
            }
        } else {
            self.update(|s| s.phase = Phase::Boosted);
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(self.poll.min(Duration::from_millis(500)));
            }
        }

        // Put everything back.
        self.update(|s| s.phase = Phase::Restoring);
        let mut restored = Vec::new();
        // Closing the handle hands Wi-Fi back to Windows.
        if wifi.take().is_some() {
            restored.push(step("wifi", "Wi-Fi", Ok("Back to normal".into())));
        }
        restored.extend(self.restore(&j, true));
        let _ = std::fs::remove_file(&self.journal_path);
        {
            let mut inner = self.inner.lock();
            inner.stop = None;
            if let Some(s) = inner.session.as_mut() {
                s.restored = restored;
                s.phase = Phase::Ended;
                s.ended_at = Some(crate::db::now());
            }
        }
        self.emit();
    }

    fn per_exe_settings(&self, p: &GameProfile) {
        let b = &p.boost;
        if !(b.gpu_high_performance || b.fullscreen_optimizations_off) {
            return;
        }
        let Some(exe) = p.exe_path.as_deref() else {
            self.push(step("pergame", "GPU & fullscreen", Ok("skip: set once the game has run".into())));
            return;
        };
        let mut done = Vec::new();
        let mut err = None;
        if b.gpu_high_performance {
            match tweaks::set_gpu_high_performance(exe, true) {
                Ok(_) => done.push("high-performance GPU"),
                Err(e) => err = Some(e),
            }
        }
        if b.fullscreen_optimizations_off {
            match tweaks::set_fullscreen_optimizations_off(exe, true) {
                Ok(_) => done.push("fullscreen optimizations off"),
                Err(e) => err = Some(e),
            }
        }
        let r = match err {
            Some(e) if done.is_empty() => Err(e),
            _ => Ok(done.join(", ")),
        };
        self.push(step("pergame", "GPU & fullscreen", r));
    }

    /// The game is up: learn where it lives and give it priority.
    fn on_game_started(&self, p: &mut GameProfile) {
        if p.exe_path.is_none() {
            if let Some(exe) = self.procs.exe_of(&p.process) {
                p.exe_path = Some(exe.clone());
                let mut inner = self.inner.lock();
                if let Some(q) = inner.store.profiles.iter_mut().find(|q| q.id == p.id) {
                    q.exe_path = Some(exe);
                }
                self.save_store(&inner.store);
                drop(inner);
                // Takes effect from the next start.
                self.per_exe_settings(p);
            }
        }
        if let Some(pr) = p.boost.priority {
            let r = self.procs.set_priority(&p.process, pr).map(|_| format!("{pr:?} priority")).map_err(|e| {
                if p.boost.start_high_priority {
                    format!("{e} — Windows already starts it at High priority")
                } else {
                    format!("{e} (anti-cheat often blocks this; try “Always start at High priority”)")
                }
            });
            let r = match r {
                Err(e) if e.contains("Windows feature") => Ok("skip: Windows only".to_string()),
                r => r,
            };
            self.push(step("priority", "Game priority", r));
        }
    }

    /// Undo a session's changes. `reopen` starts closed apps again.
    fn restore(&self, j: &Journal, reopen: bool) -> Vec<Step> {
        let mut out = Vec::new();
        if let Some(plan) = &j.plan_before {
            out.push(step("power", "Power plan", tweaks::set_plan(plan).map(|_| "Back to the previous plan".into())));
        }
        if let Some(before) = j.toasts_before {
            out.push(step("notifications", "Notifications", tweaks::restore_notifications(before).map(|_| "Pop-ups back on".into())));
        }
        if reopen && j.reopen && !j.closed.is_empty() {
            let mut started = Vec::new();
            for (name, exe) in &j.closed {
                if !self.procs.pids_named(name).is_empty() {
                    continue;
                }
                if let Some(exe) = exe.as_deref().filter(|e| Path::new(e).is_file()) {
                    if start(exe, "").is_ok() {
                        started.push(short(name).to_string());
                    }
                }
            }
            out.push(step("apps", "Background apps", Ok(if started.is_empty() { "skip: nothing to reopen".into() } else { format!("Reopened {}", started.join(", ")) })));
        }
        out
    }

    /// After a crash or power cut during a boost: put the PC back.
    pub fn recover(&self) {
        let Some(j) = std::fs::read(&self.journal_path).ok().and_then(|b| serde_json::from_slice::<Journal>(&b).ok()) else { return };
        tracing::info!("games: restoring settings from an unfinished boost");
        // Don't relaunch apps hours later.
        let _ = self.restore(&j, false);
        let _ = std::fs::remove_file(&self.journal_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_and_launch_targets() {
        for k in GameKind::all() {
            let t = template(k);
            assert!(!t.name.is_empty());
            assert!(t.process.is_empty() || tweaks::valid_exe_name(&t.process), "{k:?}");
        }
        assert_eq!(launch_target(&template(GameKind::Fortnite).launch).unwrap().unwrap().0, format!("com.epicgames.launcher://apps/{FORTNITE_APP}?action=launch&silent=true"));
        assert_eq!(launch_target(&Launch::Steam { app_id: 730 }).unwrap().unwrap().0, "steam://rungameid/730");
        assert_eq!(launch_target(&Launch::Roblox { place_id: Some(920_587_237) }).unwrap().unwrap().0, "roblox://experiences/start?placeId=920587237");
        assert!(launch_target(&Launch::Url { url: "file:///c:/windows/system32/cmd.exe".into() }).is_err());
        assert!(launch_target(&Launch::Url { url: "steam://run/730 \"x".into() }).is_err());
        assert!(launch_target(&Launch::Epic { app: "a&b".into() }).is_err());
        assert!(launch_target(&Launch::None).unwrap().is_none());
        assert_eq!(split_args(r#"-a "two words" -b"#), vec!["-a", "two words", "-b"]);
        // The JSON shape the UI sends.
        let l: Launch = serde_json::from_str(r#"{"type":"steam","appId":730}"#).unwrap();
        assert_eq!(l, Launch::Steam { app_id: 730 });
        let l: Launch = serde_json::from_str(r#"{"type":"roblox","placeId":null}"#).unwrap();
        assert_eq!(l, Launch::Roblox { place_id: None });
    }

    #[test]
    fn steps_read_naturally() {
        assert_eq!(step("x", "X", Ok("skip: not needed".into())).status, StepStatus::Skipped);
        assert_eq!(step("x", "X", Ok("skip: not needed".into())).detail, "not needed");
        assert_eq!(step("x", "X", Err("nope".into())).status, StepStatus::Failed);
    }
}
