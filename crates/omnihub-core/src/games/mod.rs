//! Game profiles: one click gets the PC out of a game's way (power plan,
//! background apps, notifications, GPU choice, Wi-Fi scanning…), starts the
//! game, and puts everything back when it closes. Plus Roblox flags and a
//! ping helper.
//!
//! Nothing here can make a server closer: the ping helper measures the
//! connection and removes the things on this PC that add lag and spikes.

pub mod configs;
pub mod fps;
pub mod ini;
pub mod junk;
pub mod library;
pub mod nvidia;
pub mod pc;
pub mod ping;
pub mod reshade;
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
use fps::{FpsRecord, FpsStatus};
use library::InstalledGame;
use pc::{PcStatus, TweakId};
use crate::system::procs::{Priority, ProcessMonitor};
use ping::{PingResult, PingTarget};
use roblox::RobloxFlags;
use tweaks::{AdminOp, PowerPlan};

/// How long a launched game may take to appear before OmniHub says so.
const SLOW_START: Duration = Duration::from_secs(45);

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
    Overwatch,
    Cyberpunk,
    #[default]
    Custom,
}

impl GameKind {
    pub fn all() -> [GameKind; 13] {
        use GameKind::*;
        [Fortnite, Roblox, Valorant, Cs2, Apex, Overwatch, RocketLeague, Gta5, CallOfDuty, League, Minecraft, Cyberpunk, Custom]
    }

    /// Competitive games go for frames; the rest keep their picture.
    pub fn default_mode(self) -> BoostMode {
        match self {
            GameKind::Gta5 | GameKind::Cyberpunk | GameKind::Custom => BoostMode::Quality,
            _ => BoostMode::Competitive,
        }
    }

    /// The game whose own settings file OmniHub can optimize.
    pub fn config_game(self) -> Option<ConfigGame> {
        match self {
            GameKind::Fortnite => Some(ConfigGame::Fortnite),
            GameKind::Minecraft => Some(ConfigGame::Minecraft),
            GameKind::Valorant => Some(ConfigGame::Valorant),
            GameKind::Cs2 => Some(ConfigGame::Cs2),
            GameKind::Apex => Some(ConfigGame::Apex),
            GameKind::Overwatch => Some(ConfigGame::Overwatch),
            GameKind::Roblox => Some(ConfigGame::Roblox),
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

/// How far a boost goes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum BoostMode {
    /// The most frames and the least delay: also sets the game's own
    /// graphics to the fastest (where OmniHub knows them).
    Competitive,
    /// Everything that doesn't change the picture: the game's graphics are
    /// never touched (4K Ultra stays 4K Ultra).
    Quality,
    /// Each switch as you set it.
    #[default]
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Boost {
    pub mode: BoostMode,
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
    /// Games OmniHub knows the settings of: write the fastest in-game
    /// settings before each launch (see `configs`).
    pub game_settings: bool,
    /// Close cloud sync, Windows extras and updaters (`junk::JUNK`) for the
    /// session.
    pub close_junk: bool,
    /// Browsers, launchers' web views and sync apps at "Below normal"
    /// priority while playing.
    pub lower_background: bool,
    /// Before the game starts, background programs hand the memory they
    /// aren't using back to Windows, so the game starts with the most free RAM.
    pub free_memory: bool,
    /// Windows' finest timer (0.5 ms) while playing.
    pub precise_timer: bool,
    /// Windows never slows the game down to save power.
    pub full_speed: bool,
    /// Measure frames per second while playing (PresentMon).
    pub fps_meter: bool,
    /// Per game, needs admin once: the game's NVIDIA driver profile set for
    /// speed (full clocks, low latency, fast texture filtering, no forced
    /// VSync).
    pub nvidia_profile: bool,
}

impl Boost {
    /// The switches a mode stands for. Close lists and the per-game
    /// admin settings stay as they are.
    pub fn with_mode(&self, mode: BoostMode) -> Boost {
        let mut b = self.clone();
        b.mode = mode;
        if mode == BoostMode::Custom {
            return b;
        }
        let competitive = mode == BoostMode::Competitive;
        b.power_plan = PowerPlan::Ultimate;
        b.priority = Some(if competitive { Priority::High } else { Priority::AboveNormal });
        b.reopen_apps = true;
        b.silence_notifications = true;
        b.game_mode = true;
        b.gpu_high_performance = true;
        b.wifi_low_latency = true;
        b.game_settings = competitive;
        b.close_junk = true;
        b.lower_background = true;
        b.free_memory = true;
        b.precise_timer = true;
        b.full_speed = true;
        // Texture filtering's fast setting is a (small) picture trade-off.
        b.nvidia_profile = competitive;
        b
    }
}

impl Default for Boost {
    fn default() -> Self {
        Boost {
            mode: BoostMode::Custom,
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
            close_junk: true,
            lower_background: true,
            free_memory: true,
            precise_timer: true,
            full_speed: true,
            fps_meter: true,
            nvidia_profile: false,
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
        // Battle.net's link for Overwatch 2 (its product code is "Pro").
        GameKind::Overwatch => ("Overwatch 2", Launch::Url { url: "battlenet://Pro".into() }, "Overwatch.exe"),
        GameKind::Cyberpunk => ("Cyberpunk 2077", Launch::Steam { app_id: 1_091_500 }, "Cyberpunk2077.exe"),
        // Java edition; the launcher starts the game.
        GameKind::Minecraft => ("Minecraft", Launch::None, "javaw.exe"),
        GameKind::Custom => ("My game", Launch::None, ""),
    };
    GameProfile { id: String::new(), name: name.into(), kind, launch, process: process.into(), exe_path: None, boost: Boost::default().with_mode(kind.default_mode()), ping_host: None, roblox: RobloxFlags::default(), last_played: None }
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
        Launch::Url { url } if url.starts_with("battlenet://") => &["Battle.net.exe", "Agent.exe"],
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
        // Roblox opens its app only when asked to (`--app`); through its
        // link when one is set up, so its bootstrapper updates it first.
        Launch::Roblox { place_id: None } => match roblox::detect().player {
            Some(_) if tweaks::link_handler_installed("roblox-player") => ("roblox-player:1+launchmode:app".into(), String::new()),
            Some(p) => (p, "--app".into()),
            None => ("roblox-player:1+launchmode:app".into(), String::new()),
        },
    }))
}

/// The scheme of a game link and the app that opens it.
fn link_app(target: &str) -> Option<(&str, &'static str)> {
    let scheme = target.split_once(':')?.0;
    if scheme.len() < 2 || matches!(scheme, "http" | "https") || !scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+')) {
        return None;
    }
    let app = match scheme {
        "com.epicgames.launcher" => "the Epic Games Launcher",
        "steam" => "Steam",
        "roblox" | "roblox-player" => "Roblox",
        "battlenet" => "Battle.net",
        "uplay" => "Ubisoft Connect",
        "origin2" => "the EA app",
        "minecraft" => "the Minecraft Launcher",
        _ => "the game's launcher",
    };
    Some((scheme, app))
}

fn start(target: &str, args: &str) -> std::io::Result<()> {
    if let Some((scheme, app)) = link_app(target) {
        if !tweaks::link_handler_installed(scheme) {
            return Err(std::io::Error::other(format!("Nothing on this PC opens {scheme}: links. Install {app} (or open it once), then press Play again.")));
        }
    }
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
    /// Still going (an administrator prompt not answered yet).
    Waiting,
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
    pub mode: BoostMode,
    pub phase: Phase,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub launched: bool,
    pub steps: Vec<Step>,
    /// Restore steps, once the session ends.
    pub restored: Vec<Step>,
    pub message: Option<String>,
    /// Frames per second over the session, when the FPS meter ran.
    pub fps: Option<fps::Summary>,
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
    /// Programs set to "Below normal" for the session.
    lowered: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Store {
    profiles: Vec<GameProfile>,
    /// The "Ultimate Performance" plan OmniHub created, reused next time.
    ultimate_plan: Option<String>,
    /// Roblox flags OmniHub wrote last, replaced on the next write.
    roblox_managed: Vec<String>,
    /// How each game is set up by "Optimize".
    config_options: ConfigOptions,
    /// FPS meter results, newest last.
    fps_records: Vec<FpsRecord>,
}

/// Results kept (all games together).
const FPS_RECORDS: usize = 60;

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
    /// The game's NVIDIA profile is set; `None` without an NVIDIA driver.
    pub nvidia_profile: Option<bool>,
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
    /// For tests: the games' settings files; otherwise they are looked up.
    config_paths: Mutex<Option<std::collections::HashMap<ConfigGame, Vec<PathBuf>>>>,
    library_roots: Mutex<library::Roots>,
    /// Downloaded tools (PresentMon).
    tools_dir: PathBuf,
    /// Written once Windows was asked to let OmniHub read frame timings.
    fps_marker: PathBuf,
    fps_installing: AtomicBool,
    live_fps: Mutex<Option<fps::Live>>,
    /// For tests: a stand-in PresentMon, allowed to run.
    fps_test_tool: Mutex<Option<PathBuf>>,
    events: EventBus,
    procs: Arc<ProcessMonitor>,
    inner: Mutex<Inner>,
    roblox_roots: Mutex<Vec<(PathBuf, Option<&'static str>)>>,
    /// Poll interval while watching a game (shorter in tests).
    poll: Duration,
    /// How long to wait for a launched game to show up.
    wait_limit: Duration,
    /// How long a boost waits for Windows' administrator prompt.
    admin_wait: Duration,
    /// Runs the administrator changes (a stand-in in tests).
    run_admin: AdminRunner,
}

type AdminRunner = Arc<dyn Fn(&[AdminOp]) -> Result<(), String> + Send + Sync>;

fn write_json(path: &Path, v: &impl Serialize) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v).map_err(std::io::Error::other)?)?;
    std::fs::rename(&tmp, path)
}

fn step(id: &str, label: &str, r: Result<String, String>) -> Step {
    let (status, detail) = match r {
        Ok(d) if d.starts_with("skip:") => (StepStatus::Skipped, d[5..].trim().to_string()),
        Ok(d) if d.starts_with("wait:") => (StepStatus::Waiting, d[5..].trim().to_string()),
        Err(e) if e == "Windows only" => (StepStatus::Skipped, e),
        Ok(d) => (StepStatus::Done, d),
        Err(e) => (StepStatus::Failed, e),
    };
    Step { id: id.into(), label: label.into(), status, detail }
}

/// Whether an NVIDIA driver answers (asked once; drivers don't come and go).
fn nvidia_available() -> bool {
    static AVAILABLE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *AVAILABLE.get_or_init(nvidia::available)
}

fn nvidia_on(exe: &str) -> bool {
    nvidia_available() && nvidia::state(exe).is_ok_and(|s| s.on)
}

fn short(name: &str) -> &str {
    name.strip_suffix(".exe").or_else(|| name.strip_suffix(".EXE")).unwrap_or(name)
}

impl GameHub {
    pub fn new(data_dir: &Path, events: EventBus, procs: Arc<ProcessMonitor>) -> Arc<Self> {
        let store_path = data_dir.join("games.json");
        let mut store: Store = std::fs::read(&store_path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        // A mode decides its switches, including ones added since the
        // profile was saved.
        for p in &mut store.profiles {
            p.boost = p.boost.with_mode(p.boost.mode);
        }
        Arc::new(GameHub {
            journal_path: data_dir.join("games-session.json"),
            backups_dir: data_dir.join("game-settings-backup"),
            pc_journal: data_dir.join("pc-tweaks.json"),
            config_paths: Mutex::new(None),
            library_roots: Mutex::new(library::Roots::system()),
            tools_dir: data_dir.join("tools"),
            fps_marker: data_dir.join("fps-access.json"),
            fps_installing: AtomicBool::new(false),
            live_fps: Mutex::new(None),
            fps_test_tool: Mutex::new(None),
            store_path,
            events,
            procs,
            inner: Mutex::new(Inner { store, session: None, stop: None }),
            roblox_roots: Mutex::new(roblox::roots()),
            poll: Duration::from_secs(2),
            wait_limit: Duration::from_secs(600),
            admin_wait: Duration::from_secs(25),
            run_admin: Arc::new(tweaks::run_admin),
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

    /// For tests: a stand-in for the administrator step and its wait.
    pub fn with_admin(self: Arc<Self>, wait: Duration, run: impl Fn(&[AdminOp]) -> Result<(), String> + Send + Sync + 'static) -> Arc<Self> {
        let mut h = Arc::try_unwrap(self).unwrap_or_else(|_| unreachable!());
        h.admin_wait = wait;
        h.run_admin = Arc::new(run);
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
        p.boost = p.boost.with_mode(p.boost.mode);
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
            if old.boost.nvidia_profile && !p.boost.nvidia_profile && nvidia_on(&old.process) {
                ops.push(AdminOp::NvidiaOff(old.process.clone()));
            }
        }
        if let Err(e) = tweaks::run_admin(&ops) {
            warnings.push(format!("Saved, but the network/start priority or NVIDIA settings are still on: {e}"));
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
        if !p.process.is_empty() && p.boost.nvidia_profile && nvidia_on(&p.process) {
            ops.push(AdminOp::NvidiaOff(p.process.clone()));
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
            nvidia_profile: nvidia_available().then(|| has && nvidia::state(&p.process).is_ok_and(|s| s.on)),
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

    fn shaders(&self) -> reshade::Shaders {
        reshade::Shaders::new(self.tools_dir.join("reshade"))
    }

    /// Roblox shaders now: ReShade's version and where it is in place.
    pub fn roblox_shaders(&self) -> reshade::ShaderStatus {
        let mut st = reshade::ShaderStatus { version: self.shaders().version(), ..Default::default() };
        for d in roblox::player_dirs(&self.roblox_roots.lock()) {
            match (d.join("dxgi.dll").is_file(), d.join(reshade::MARKER).is_file()) {
                (true, true) => st.applied += 1,
                (true, false) => st.foreign.push(d.to_string_lossy().into_owned()),
                _ => {}
            }
        }
        st
    }

    /// Roblox's anti-tamper now stops Roblox from starting with ReShade in
    /// its folder ("Failed to load library: d3d11.dll"; Bloxshade is down for
    /// the same reason), so shaders are only ever taken out.
    pub fn apply_roblox_shaders(&self, s: &reshade::ShaderSettings) -> Result<reshade::ShaderStatus, String> {
        let st = self.remove_roblox_shaders()?;
        if s.enabled {
            return Err(reshade::BLOCKED.into());
        }
        Ok(st)
    }

    /// Take OmniHub's ReShade out of every Roblox folder (another tool's stays).
    pub fn remove_roblox_shaders(&self) -> Result<reshade::ShaderStatus, String> {
        let off = reshade::ShaderSettings { enabled: false, ..Default::default() };
        self.shaders().apply(&off, &roblox::player_dirs(&self.roblox_roots.lock()))
    }

    /// Every flag Roblox's client has (from a community tracker, cached a
    /// day), allowlisted ones first.
    pub fn roblox_flag_list(&self, refresh: bool) -> Result<Vec<roblox::KnownFlag>, String> {
        let cache = self.tools_dir.join("roblox-flags.json");
        let fresh = std::fs::metadata(&cache).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age < Duration::from_secs(86_400));
        if fresh && !refresh {
            if let Some(list) = std::fs::read(&cache).ok().and_then(|b| serde_json::from_slice(&b).ok()) {
                return Ok(list);
            }
        }
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(60))).build().into();
        let get = |url: &str| -> Result<Vec<u8>, String> { agent.get(url).call().map_err(|e| format!("The flag list didn't download: {e}"))?.body_mut().with_config().limit(32 << 20).read_to_vec().map_err(|e| e.to_string()) };
        let names = String::from_utf8_lossy(&get(roblox::FLAG_NAMES)?).into_owned();
        let values: serde_json::Value = serde_json::from_slice(&get(roblox::FLAG_VALUES)?).unwrap_or_default();
        let list = roblox::known_flags(&names, &values);
        if list.is_empty() {
            return Err("The flag list came back empty.".into());
        }
        let _ = std::fs::create_dir_all(&self.tools_dir);
        let _ = std::fs::write(&cache, serde_json::to_vec(&list).unwrap_or_default());
        Ok(list)
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

    /// Lag under load against the profile's closest server (or the nearest
    /// Cloudflare/Google one). Progress goes out as `games:loadTest`.
    pub fn load_test(&self, id: Option<&str>) -> ping::LoadTest {
        let targets = self.ping_targets(id);
        let quick = ping::measure_all(&targets, 4, Duration::from_millis(100));
        let best = quick.iter().filter(|r| r.avg_ms.is_some()).min_by(|a, b| a.avg_ms.unwrap_or(f32::MAX).total_cmp(&b.avg_ms.unwrap_or(f32::MAX))).and_then(|r| targets.iter().find(|t| t.id == r.id)).or(targets.first()).cloned();
        let Some(target) = best else { return ping::LoadTest { error: Some("Nothing to ping.".into()), ..Default::default() } };
        ping::load_test(&target, &ping::LoadUrls::default(), Duration::from_secs(8), tweaks::on_wifi(), &mut |phase, progress, ms| {
            self.events.emit("games:loadTest", serde_json::json!({ "phase": phase, "progress": progress, "ms": ms }));
        })
    }

    // ---------- the games' own settings ----------

    /// For tests: the games' settings files and the launchers' records in
    /// temporary folders.
    pub fn set_test_paths(&self, configs: Vec<(ConfigGame, PathBuf)>, roots: library::Roots) {
        let mut map = std::collections::HashMap::<ConfigGame, Vec<PathBuf>>::new();
        for (g, p) in configs {
            map.entry(g).or_default().push(p);
        }
        *self.config_paths.lock() = Some(map);
        *self.library_roots.lock() = roots;
    }

    /// The game's settings files (looked up each time: a game makes its
    /// file the first time it runs).
    fn config_files(&self, game: ConfigGame) -> Vec<PathBuf> {
        match self.config_paths.lock().as_ref() {
            Some(map) => map.get(&game).map(|v| v.iter().filter(|p| p.is_file()).cloned().collect()).unwrap_or_default(),
            None => game.locate(),
        }
    }

    pub fn config_options(&self) -> ConfigOptions {
        self.inner.lock().store.config_options.clone()
    }

    pub fn config_status(&self, game: ConfigGame) -> ConfigStatus {
        let running = !self.procs.pids_named(game.process()).is_empty();
        configs::status(game, &self.config_options(), &self.config_files(game), &self.backups_dir, running)
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
        configs::apply(game, &self.config_options(), &self.config_files(game), &self.backups_dir)
    }

    /// Empty Fortnite's logs, crash reports and web cache, and the graphics
    /// driver's shader caches (not while Fortnite runs; the first match
    /// after rebuilds its shaders).
    pub fn fortnite_clean_caches(&self) -> Result<CacheClean, String> {
        if !self.procs.pids_named(ConfigGame::Fortnite.process()).is_empty() {
            return Err("Close Fortnite first.".into());
        }
        let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).ok_or("Windows didn’t say where your app data is.")?;
        Ok(clean_folders(&fortnite_cache_dirs(&local)))
    }

    pub fn config_restore(&self, game: ConfigGame) -> Result<(), String> {
        if !self.procs.pids_named(game.process()).is_empty() {
            return Err(format!("Close {} first.", game.label()));
        }
        configs::restore(game, &self.config_files(game), &self.backups_dir)
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

    // ---------- FPS meter ----------

    /// For tests: a stand-in for PresentMon that is allowed to run.
    pub fn set_fps_test_tool(&self, exe: PathBuf) {
        *self.fps_test_tool.lock() = Some(exe);
    }

    fn fps_exe(&self) -> PathBuf {
        self.fps_test_tool.lock().clone().unwrap_or_else(|| self.tools_dir.join("PresentMon.exe"))
    }

    pub fn fps_status(&self) -> FpsStatus {
        let test = self.fps_test_tool.lock().is_some();
        let allowed = test || fps::allowed();
        FpsStatus { installed: self.fps_exe().is_file(), allowed, sign_out_needed: !allowed && self.fps_marker.exists(), supported: test || cfg!(windows) }
    }

    /// The live figures while a game is measured.
    pub fn fps_live(&self) -> Option<fps::Live> {
        self.live_fps.lock().clone()
    }

    /// Download PresentMon from this release and check it against its
    /// published checksum. Progress goes out as `games:fpsInstall`.
    pub fn fps_install(&self) -> Result<FpsStatus, String> {
        if self.fps_installing.swap(true, Ordering::SeqCst) {
            return Err("The FPS meter is already downloading.".into());
        }
        let r = (|| -> Result<(), String> {
            use crate::capture::airplay;
            let url = format!("{}/v{}/{}", airplay::releases_url(), env!("CARGO_PKG_VERSION"), fps::ASSET);
            std::fs::create_dir_all(&self.tools_dir).map_err(|e| e.to_string())?;
            let part = self.tools_dir.join("PresentMon.exe.part");
            let hash = airplay::download(&url, &part, &mut |done, total| self.events.emit("games:fpsInstall", serde_json::json!({ "done": done, "total": total }))).map_err(|e| format!("Could not download the FPS meter: {e}"))?;
            let expected = match fps::pinned_sha256() {
                Some(h) => h.to_lowercase(),
                None => {
                    let sums = airplay::fetch_text(&format!("{}/v{}/SHA256SUMS.txt", airplay::releases_url(), env!("CARGO_PKG_VERSION"))).map_err(|e| e.to_string())?;
                    airplay::checksum_for(&sums, fps::ASSET).ok_or("This release lists no checksum for the FPS meter.")?
                }
            };
            if hash != expected {
                let _ = std::fs::remove_file(&part);
                return Err("The download did not match its published checksum, so it was not kept.".into());
            }
            std::fs::rename(&part, self.tools_dir.join("PresentMon.exe")).map_err(|e| e.to_string())
        })();
        self.fps_installing.store(false, Ordering::SeqCst);
        r.map(|_| self.fps_status())
    }

    /// One administrator prompt: Windows adds you to "Performance Log
    /// Users" (it counts from your next sign-in).
    pub fn fps_allow(&self) -> Result<FpsStatus, String> {
        if !self.fps_status().allowed {
            fps::request_access()?;
            let _ = write_json(&self.fps_marker, &serde_json::json!({ "at": crate::db::now() }));
        }
        Ok(self.fps_status())
    }

    /// Past results, newest first (one game's, or all).
    pub fn fps_history(&self, profile_id: Option<&str>) -> Vec<FpsRecord> {
        self.inner.lock().store.fps_records.iter().rev().filter(|r| profile_id.is_none_or(|id| r.profile_id == id)).cloned().collect()
    }

    /// Start measuring the game, or say why not.
    fn start_meter(self: &Arc<Self>, p: &GameProfile, stats: &Arc<Mutex<fps::Stats>>) -> Option<fps::Meter> {
        let st = self.fps_status();
        let why = if !st.supported {
            Some("Windows only".to_string())
        } else if !st.installed {
            Some("skip: get the FPS meter in Games → FPS first".to_string())
        } else if !st.allowed {
            Some(if st.sign_out_needed { "skip: sign out of Windows and back in once to finish turning on the FPS meter".to_string() } else { "skip: allow the FPS meter once in Games → FPS".to_string() })
        } else {
            None
        };
        if let Some(w) = why {
            self.push(step("fps", "FPS meter", if w == "Windows only" { Err(w) } else { Ok(w) }));
            return None;
        }
        let pid = *self.procs.pids_named(&p.process).first()?;
        let hub = self.clone();
        match fps::Meter::start(&self.fps_exe(), pid, stats.clone(), move |l| {
            *hub.live_fps.lock() = Some(l.clone());
            hub.events.emit("games:fps", &l);
        }) {
            Ok(m) => {
                self.push(step("fps", "FPS meter", Ok("Measuring every frame (PresentMon)".into())));
                Some(m)
            }
            Err(e) => {
                self.push(step("fps", "FPS meter", Err(e)));
                None
            }
        }
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
                // Still waiting for this game: Play again starts it again.
                if launch && s.profile_id == p.id && s.phase == Phase::Waiting {
                    drop(inner);
                    if let Some((target, args)) = launch_target(&p.launch)? {
                        start(&target, &args).map_err(|e| e.to_string())?;
                    }
                    self.update(|s| s.message = Some(format!("Started {} again", p.name)));
                    return self.session().ok_or_else(|| "The boost ended.".into());
                }
                return Err(format!("{} is already boosted — stop that first.", s.name));
            }
            inner.session = Some(Session { profile_id: p.id.clone(), name: p.name.clone(), mode: p.boost.mode, phase: Phase::Starting, started_at: crate::db::now(), ended_at: None, launched: false, steps: Vec::new(), restored: Vec::new(), message: None, fps: None });
            inner.stop = Some(stop.clone());
        }
        self.emit();
        let hub = self.clone();
        std::thread::Builder::new()
            .name("game-boost".into())
            .spawn(move || {
                let run = hub.clone();
                if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || run.run(p, launch, stop))) {
                    let why = e.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| e.downcast_ref::<String>().cloned()).unwrap_or_default();
                    hub.crashed(&why);
                }
            })
            .map_err(|e| e.to_string())?;
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

        // 1. Background programs: the profile's list, plus the junk nobody
        // needs mid-game.
        let mut to_close: Vec<String> = b.close_apps.clone();
        if b.close_junk {
            for x in junk::JUNK {
                if !to_close.iter().any(|c| c.eq_ignore_ascii_case(x.exe)) {
                    to_close.push(x.exe.to_string());
                }
            }
        }
        let running = if !to_close.is_empty() || b.lower_background { self.procs.usage(crate::system::procs::ProcessSort::Name, 10_000, false).processes } else { Vec::new() };
        if !to_close.is_empty() {
            let keep: Vec<&str> = needed_by(&p.launch).to_vec();
            let mut closed: Vec<String> = Vec::new();
            let mut failed = Vec::new();
            for name in &to_close {
                if keep.iter().any(|k| k.eq_ignore_ascii_case(name)) || name.eq_ignore_ascii_case(&p.process) {
                    continue;
                }
                let Some(g) = running.iter().find(|g| g.name.eq_ignore_ascii_case(name) && g.can_end) else { continue };
                let known = junk::find(&g.name).filter(|_| !b.close_apps.iter().any(|c| c.eq_ignore_ascii_case(&g.name)));
                // Junk Windows restarts by itself is not started again.
                let exe = if known.is_some_and(|k| k.reopen.is_none()) { None } else { g.exe.clone() };
                let label = known.map_or_else(|| short(&g.name).to_string(), |k| k.label.to_string());
                match self.procs.end(&g.name) {
                    Ok(_) => {
                        if !closed.contains(&label) {
                            closed.push(label);
                        }
                        j.closed.push((g.name.clone(), exe));
                    }
                    Err(_) => failed.push(label),
                }
            }
            save_journal(&j);
            let r = match (closed.is_empty(), failed.is_empty()) {
                (true, true) => Ok("skip: nothing to close was running".to_string()),
                (false, true) => Ok(format!("Closed {}", closed.join(", "))),
                (_, false) => Err(format!("Could not close {}{}", failed.join(", "), if closed.is_empty() { String::new() } else { format!(" (closed {})", closed.join(", ")) })),
            };
            self.push(step("apps", "Background apps", r));
        }

        // 1b. What stays open gets the processor after the game.
        if b.lower_background {
            let mut lowered = Vec::new();
            for g in running.iter().filter(|g| junk::lowered(&g.name) && !g.name.eq_ignore_ascii_case(&p.process) && g.priority == Some(Priority::Normal)) {
                if j.closed.iter().any(|(n, _)| n.eq_ignore_ascii_case(&g.name)) {
                    continue;
                }
                if self.procs.set_priority(&g.name, Priority::BelowNormal).is_ok() {
                    lowered.push(short(&g.name).to_string());
                    j.lowered.push(g.name.clone());
                }
            }
            save_journal(&j);
            let r = if lowered.is_empty() { Ok("skip: no browser, launcher or sync app was running".to_string()) } else { Ok(format!("{} at Below normal priority", lowered.join(", "))) };
            let r = if cfg!(windows) { r } else { Err("Windows only".into()) };
            self.push(step("background", "Background priority", r));
        }

        // 1c. Free memory for the game.
        if b.free_memory {
            let mut keep: Vec<&str> = needed_by(&p.launch).to_vec();
            keep.push(p.process.as_str());
            let r = tweaks::free_memory(&keep).map(|freed| if freed >= 64 << 20 { format!("{} MB of memory handed back to Windows", freed >> 20) } else { "skip: background programs were already lean".into() });
            self.push(step("memory", "Free memory", r));
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
            if b.nvidia_profile && nvidia_available() && !nvidia_on(&p.process) {
                ops.push(AdminOp::NvidiaOn(p.process.clone()));
            }
        }
        if !ops.is_empty() {
            self.admin_step(ops, &stop);
        } else if b.network_priority || b.start_high_priority || (b.nvidia_profile && nvidia_available()) {
            self.push(step("admin", "Network & start priority", Ok("skip: already on".into())));
        }

        // 6b. Windows' finest timer, held for the session.
        let mut timer = None;
        if b.precise_timer {
            let r = tweaks::timer_reaches_games().map_err(|e| format!("skip: {e}")).and_then(|_| {
                tweaks::TimerResolution::start().map(|(guard, ms)| {
                    timer = Some(guard);
                    format!("{ms:.1} ms (Windows' default is up to 15.6 ms)")
                })
            });
            let r = match r {
                Err(e) if e.starts_with("skip:") => Ok(e),
                r => r,
            };
            self.push(step("timer", "Precise timer", r));
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
        // Roblox no longer starts with ReShade in its folder: take ours out.
        if p.kind == GameKind::Roblox && p.roblox.shaders.enabled {
            let r = self.remove_roblox_shaders().map(|_| format!("skip: {}", reshade::BLOCKED));
            self.push(step("shaders", "Roblox shaders", r));
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
        let fps_stats = Arc::new(Mutex::new(fps::Stats::default()));
        let mut meter: Option<fps::Meter> = None;
        let mut meter_starts = 0;
        if !p.process.is_empty() {
            self.update(|s| s.phase = Phase::Waiting);
            let t0 = Instant::now();
            let mut seen = false;
            let mut hinted = false;
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
                    if b.fps_meter {
                        meter = self.start_meter(&p, &fps_stats);
                        meter_starts = 1;
                    }
                } else if running && meter.as_mut().is_some_and(|m| !m.running()) && meter_starts < 3 {
                    // The launcher restarted the game: follow the new one.
                    if let Some(m) = meter.take() {
                        m.stop();
                    }
                    meter = self.start_meter(&p, &fps_stats);
                    meter_starts += 1;
                } else if !running && seen {
                    // Launchers sometimes restart the game; allow a moment.
                    missing += 1;
                    if missing >= 2 {
                        self.update(|s| s.message = Some(format!("{} closed", p.name)));
                        break;
                    }
                } else if running {
                    missing = 0;
                } else if launched && !hinted && t0.elapsed() > SLOW_START {
                    hinted = true;
                    let hint = match &p.launch {
                        Launch::Epic { .. } => format!("{} hasn't opened yet — look at the Epic Games Launcher: it may be updating the game or asking you to sign in. Press Play again to retry.", p.name),
                        Launch::Steam { .. } => format!("{} hasn't opened yet — look at Steam: it may be updating the game. Press Play again to retry.", p.name),
                        _ => format!("{} hasn't opened yet. Press Play again to retry.", p.name),
                    };
                    self.update(|s| s.message = Some(hint));
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

        if let Some(m) = meter.take() {
            m.stop();
        }
        *self.live_fps.lock() = None;
        let summary = fps_stats.lock().summary();
        if let Some(sum) = summary.clone().filter(|s| s.seconds >= 10.0) {
            let mut inner = self.inner.lock();
            inner.store.fps_records.push(FpsRecord { profile_id: p.id.clone(), name: p.name.clone(), mode: b.mode, at: crate::db::now(), summary: sum });
            let extra = inner.store.fps_records.len().saturating_sub(FPS_RECORDS);
            inner.store.fps_records.drain(..extra);
            self.save_store(&inner.store);
        }
        self.update(|s| s.fps = summary);

        // Put everything back.
        self.update(|s| s.phase = Phase::Restoring);
        let mut restored = Vec::new();
        // Closing the handle hands Wi-Fi back to Windows.
        if wifi.take().is_some() {
            restored.push(step("wifi", "Wi-Fi", Ok("Back to normal".into())));
        }
        if timer.take().is_some() {
            restored.push(step("timer", "Precise timer", Ok("Back to Windows' timer".into())));
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

    /// The one administrator change of a boost. Windows' prompt is answered
    /// by a person, so the game is never held behind it for long: after
    /// `admin_wait` the boost goes on, and the answer still lands in the
    /// step when it comes.
    fn admin_step(self: &Arc<Self>, ops: Vec<AdminOp>, stop: &AtomicBool) {
        const ID: &str = "admin";
        const LABEL: &str = "Network & start priority";
        let what = ops
            .iter()
            .map(|o| match o {
                AdminOp::QosOn(_) => "network priority",
                AdminOp::NvidiaOn(_) => "the game's NVIDIA settings",
                _ => "start at High priority",
            })
            .collect::<Vec<_>>()
            .join(" + ");
        self.push(step(ID, LABEL, Ok("wait: answer Windows' administrator prompt (the shield in the taskbar if it isn't in front)".into())));
        let (tx, rx) = crossbeam_channel::bounded::<Result<String, String>>(1);
        let hub = self.clone();
        let run_admin = self.run_admin.clone();
        let spawned = std::thread::Builder::new().name("game-admin".into()).spawn(move || {
            let r = run_admin(&ops).map(|_| format!("Turned on {what} (kept for next time)"));
            // Nobody waits any more: report the answer by itself.
            if let Err(crossbeam_channel::SendError(r)) = tx.send(r) {
                hub.push(step(ID, LABEL, r));
            }
        });
        if let Err(e) = spawned {
            self.push(step(ID, LABEL, Err(e.to_string())));
            return;
        }
        let t0 = Instant::now();
        loop {
            match rx.recv_timeout(Duration::from_millis(250)) {
                Ok(r) => return self.push(step(ID, LABEL, r)),
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return self.push(step(ID, LABEL, Err("The administrator step stopped unexpectedly.".into()))),
                Err(crossbeam_channel::RecvTimeoutError::Timeout) if stop.load(Ordering::Relaxed) || t0.elapsed() >= self.admin_wait => {
                    self.push(step(ID, LABEL, Ok("wait: still waiting for Windows' administrator prompt — the boost went on without it".into())));
                    return;
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            }
        }
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
        if p.boost.full_speed {
            let pids = self.procs.pids_named(&p.process);
            let ok = pids.iter().filter(|&&pid| tweaks::set_power_throttling(Some(pid), true).is_ok()).count();
            let r = if !cfg!(windows) {
                Err("Windows only".to_string())
            } else if ok > 0 {
                Ok("Never slowed down to save power".to_string())
            } else {
                Ok("skip: the game's anti-cheat doesn't allow it (Game Mode does the same for the game in front)".to_string())
            };
            self.push(step("fullspeed", "Full speed", r));
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
        if !j.lowered.is_empty() {
            for name in &j.lowered {
                let _ = self.procs.set_priority(name, Priority::Normal);
            }
            out.push(step("background", "Background priority", Ok("Back to Normal".into())));
        }
        if reopen && j.reopen && !j.closed.is_empty() {
            let mut started = Vec::new();
            for (name, exe) in &j.closed {
                if !self.procs.pids_named(name).is_empty() {
                    continue;
                }
                if let Some(exe) = exe.as_deref().filter(|e| Path::new(e).is_file()) {
                    if start(exe, junk::find(name).and_then(|k| k.reopen).unwrap_or("")).is_ok() {
                        started.push(short(name).to_string());
                    }
                }
            }
            out.push(step("apps", "Background apps", Ok(if started.is_empty() { "skip: nothing to reopen".into() } else { format!("Reopened {}", started.join(", ")) })));
        }
        out
    }

    /// After a crash or power cut during a boost: put the PC back.
    /// A boost that failed half-way: put back what it changed and end the
    /// session, so Play and Boost only work again.
    fn crashed(&self, why: &str) {
        tracing::error!("games: the boost failed: {why}");
        let restored = match std::fs::read(&self.journal_path).ok().and_then(|b| serde_json::from_slice::<Journal>(&b).ok()) {
            Some(j) => self.restore(&j, true),
            None => Vec::new(),
        };
        let _ = std::fs::remove_file(&self.journal_path);
        *self.live_fps.lock() = None;
        {
            let mut inner = self.inner.lock();
            inner.stop = None;
            if let Some(s) = inner.session.as_mut() {
                s.restored = restored;
                s.phase = Phase::Ended;
                s.ended_at = Some(crate::db::now());
                s.message = Some(format!("The boost stopped on an error{}; what it changed was put back.", if why.is_empty() { String::new() } else { format!(" ({why})") }));
            }
        }
        self.emit();
    }

    pub fn recover(&self) {
        // Shaders an earlier OmniHub put in Roblox stop it from starting.
        if let Err(e) = self.remove_roblox_shaders() {
            tracing::warn!("games: could not take ReShade out of Roblox: {e}");
        }
        let Some(j) = std::fs::read(&self.journal_path).ok().and_then(|b| serde_json::from_slice::<Journal>(&b).ok()) else { return };
        tracing::info!("games: restoring settings from an unfinished boost");
        // Don't relaunch apps hours later.
        let _ = self.restore(&j, false);
        let _ = std::fs::remove_file(&self.journal_path);
    }
}

/// What emptying cache folders freed.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CacheClean {
    pub freed_bytes: u64,
    pub files: u64,
    /// Files in use (left alone).
    pub skipped: u64,
}

/// Fortnite's throwaway folders and the GPU drivers' shader caches.
fn fortnite_cache_dirs(local: &Path) -> Vec<PathBuf> {
    let saved = local.join("FortniteGame").join("Saved");
    let mut dirs = vec![saved.join("Logs"), saved.join("Crashes")];
    if let Ok(rd) = std::fs::read_dir(&saved) {
        dirs.extend(rd.flatten().filter(|e| e.file_name().to_string_lossy().to_ascii_lowercase().starts_with("webcache")).map(|e| e.path()));
    }
    for d in ["D3DSCache", "NVIDIA/DXCache", "NVIDIA/GLCache", "AMD/DxCache", "AMD/DxcCache", "AMD/VkCache", "Intel/ShaderCache"] {
        dirs.push(local.join(d));
    }
    dirs
}

/// Delete what is inside each folder (the folders themselves stay).
fn clean_folders(dirs: &[PathBuf]) -> CacheClean {
    let mut out = CacheClean::default();
    for dir in dirs {
        for entry in walkdir::WalkDir::new(dir).min_depth(1).contents_first(true).into_iter().flatten() {
            let path = entry.path();
            if entry.file_type().is_dir() {
                let _ = std::fs::remove_dir(path);
                continue;
            }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if std::fs::remove_file(path).is_ok() {
                out.freed_bytes += size;
                out.files += 1;
            } else {
                out.skipped += 1;
            }
        }
    }
    out
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

    #[test]
    fn fortnite_caches_are_emptied() {
        let dir = tempfile::tempdir().unwrap();
        let saved = dir.path().join("FortniteGame/Saved");
        for (f, n) in [("Logs/FortniteGame.log", 100), ("Crashes/x/dump.dmp", 50), ("webcache_4430/a", 10), ("Config/WindowsClient/GameUserSettings.ini", 5)] {
            let p = saved.join(f);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, vec![0u8; n]).unwrap();
        }
        std::fs::create_dir_all(dir.path().join("NVIDIA/DXCache")).unwrap();
        std::fs::write(dir.path().join("NVIDIA/DXCache/s.bin"), [1u8; 40]).unwrap();
        let r = clean_folders(&fortnite_cache_dirs(dir.path()));
        assert_eq!((r.freed_bytes, r.files, r.skipped), (200, 4, 0));
        assert!(saved.join("Logs").is_dir() && !saved.join("Crashes/x").exists());
        assert!(saved.join("Config/WindowsClient/GameUserSettings.ini").is_file(), "settings stay");
    }
}
