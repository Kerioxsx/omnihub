//! Minecraft servers hosted on this PC, kept up around the clock: made
//! (vanilla, Paper, Fabric, Quilt, Forge or NeoForge, or from a modpack or
//! one of the user's mixes) with the Java they need, started with OmniHub,
//! restarted if they crash, backed up on a schedule, and stopped cleanly
//! when OmniHub quits. Players (whitelist, operators, bans, where they are,
//! what they carry; players without a paid account too, with operators
//! guarded), a live map, the server's settings, its mods, plugins
//! and data packs (also ones Claude makes), and the way in for friends
//! (firewall, router port, public address) are all handled here.

pub mod backup;
pub mod content;
pub mod guard;
pub mod install;
pub mod map;
pub mod nbt;
pub mod net;
pub mod players;
pub mod process;
pub mod props;

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use base64::Engine as _;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::events::EventBus;
use crate::mods::ModHub;
pub use content::{Content, Kind};
pub use install::{Flavor, Launch};
use players::{Details, List, Listed};
use process::{Line, Proc, Said};
use props::Props;

pub const MOJANG_API: &str = "https://api.mojang.com";

fn now() -> i64 {
    crate::db::now()
}

fn yes() -> bool {
    true
}

/// One server, as kept in `servers.json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub flavor: Flavor,
    pub mc: String,
    /// Paper build, or the Fabric, Quilt, Forge or NeoForge version.
    pub loader: String,
    pub dir: String,
    /// None until its program is installed.
    pub launch: Option<Launch>,
    pub java_major: u32,
    /// A Java the user picked; None: OmniHub's own.
    pub java_path: Option<String>,
    pub flags: Vec<String>,
    pub memory_mb: u32,
    pub auto_start: bool,
    pub auto_restart: bool,
    pub keep_awake: bool,
    /// Hours between scheduled backups (0: none).
    pub backup_hours: u32,
    pub backup_keep: u32,
    pub last_backup: i64,
    /// Open its port on the router (UPnP) while it runs.
    pub upnp: bool,
    /// The modpack or mix it was made from.
    pub from: Option<String>,
    pub created: i64,
    /// When the user accepted the Minecraft EULA for it.
    pub eula_at: Option<i64>,
    /// OmniHub made its folder (and may delete it).
    pub owned: bool,
}

impl Default for Server {
    fn default() -> Self {
        Server {
            id: String::new(),
            name: String::new(),
            flavor: Flavor::Vanilla,
            mc: String::new(),
            loader: String::new(),
            dir: String::new(),
            launch: None,
            java_major: 21,
            java_path: None,
            flags: Vec::new(),
            memory_mb: 2048,
            auto_start: false,
            auto_restart: true,
            keep_awake: true,
            backup_hours: 24,
            backup_keep: 7,
            last_backup: 0,
            upnp: false,
            from: None,
            created: 0,
            eula_at: None,
            owned: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum State {
    #[default]
    Stopped,
    Installing,
    Starting,
    Running,
    Stopping,
    Crashed,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Online {
    pub name: String,
    pub uuid: Option<String>,
    pub since: i64,
    pub pos: Option<[f64; 3]>,
    pub dimension: Option<String>,
}

/// Something long under way (installing, backing up).
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Work {
    pub text: String,
    pub done: u64,
    pub total: u64,
}

/// A server for the window: its record and how it's doing.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    #[serde(flatten)]
    pub server: Server,
    pub state: State,
    pub since: i64,
    pub online: Vec<Online>,
    pub max_players: u32,
    pub port: u16,
    pub motd: String,
    pub whitelist: bool,
    pub online_mode: bool,
    pub problem: Option<String>,
    pub work: Option<Work>,
    pub memory_used_mb: u64,
    pub cpu: f32,
    pub restart_needed: bool,
    pub icon: Option<String>,
    /// This PC's address on the home network, with the port.
    pub lan: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum FromPack {
    Modrinth { project: String, version: String },
    Mix { id: String },
    File { path: String },
    /// A modpack Prism Launcher, CurseForge or the Modrinth App installed.
    Launcher { id: String },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewServer {
    pub name: String,
    pub flavor: Flavor,
    #[serde(default)]
    pub mc: String,
    #[serde(default)]
    pub loader: String,
    #[serde(default)]
    pub memory_mb: Option<u32>,
    #[serde(default)]
    pub port: Option<u16>,
    pub eula: bool,
    #[serde(default = "yes")]
    pub online_mode: bool,
    #[serde(default)]
    pub max_players: Option<u32>,
    #[serde(default)]
    pub motd: Option<String>,
    #[serde(default)]
    pub whitelist: bool,
    #[serde(default)]
    pub seed: Option<String>,
    #[serde(default)]
    pub gamemode: Option<String>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub from: Option<FromPack>,
    #[serde(default)]
    pub start: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Patch {
    pub name: Option<String>,
    pub memory_mb: Option<u32>,
    /// "" for OmniHub's own Java.
    pub java_path: Option<String>,
    pub flags: Option<Vec<String>>,
    pub auto_start: Option<bool>,
    pub auto_restart: Option<bool>,
    pub keep_awake: Option<bool>,
    pub backup_hours: Option<u32>,
    pub backup_keep: Option<u32>,
    pub upnp: Option<bool>,
    pub eula: Option<bool>,
}

/// Something to do to a player.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Action {
    Give { item: String, count: u32 },
    Clear { item: Option<String>, count: Option<u32> },
    ClearSlot { slot: i32 },
    Teleport { x: f64, y: f64, z: f64, dimension: Option<String> },
    TeleportTo { player: String },
    Spawn,
    Gamemode { mode: String },
    Heal,
    Feed,
    Message { text: String },
    Kick { reason: Option<String> },
    Ban { reason: Option<String> },
    Pardon,
    Op,
    Deop,
    Whitelist { on: bool },
    /// An operator's name may join from this place (offline mode).
    Allow { place: String },
    /// It wasn't them: forget the attempt.
    Refuse { place: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Known {
    pub name: String,
    pub uuid: String,
    pub online: bool,
    pub op: bool,
    pub whitelisted: bool,
    pub banned: bool,
    pub ban_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayersView {
    pub online: Vec<Online>,
    pub known: Vec<Known>,
    pub whitelist_on: bool,
    pub enforce: bool,
    pub max_players: u32,
    pub online_mode: bool,
    /// Operator names kept out because they joined from a new place.
    pub waiting: Vec<guard::Waiting>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerView {
    pub name: String,
    pub uuid: Option<String>,
    pub online: bool,
    /// Read now from the server (true) or from the player's saved file.
    pub live: bool,
    #[serde(flatten)]
    pub details: Details,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapInfo {
    pub regions: Vec<(i32, i32)>,
    pub spawn: Option<[i32; 3]>,
    pub players: Vec<Online>,
    /// Drawn worlds need Minecraft 1.18 or newer.
    pub supported: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    #[serde(flatten)]
    pub hit: crate::mods::minecraft::ProjectHit,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetView {
    pub port: u16,
    pub lan: Vec<String>,
    pub public_ip: Option<String>,
    pub router: Option<String>,
    pub router_ip: Option<String>,
    pub upnp: bool,
    pub port_open: bool,
    pub upnp_error: Option<String>,
    /// The provider shares one public address between homes: a port can't be opened.
    pub cgnat: bool,
    pub java: Option<String>,
    pub firewall: Option<crate::system::firewall::FirewallReport>,
}

#[derive(Default)]
struct Live {
    state: State,
    proc: Option<Arc<Proc>>,
    console: Arc<Mutex<VecDeque<Line>>>,
    next: Arc<AtomicU64>,
    online: Vec<Online>,
    max: Option<u32>,
    since: i64,
    problem: Option<String>,
    crashes: Vec<i64>,
    restart_at: Option<Instant>,
    work: Option<Work>,
    busy: Option<String>,
    watch_until: Option<Instant>,
    polling: bool,
    console_dirty: bool,
    restart_needed: bool,
    port_open: bool,
    port_renewed: Option<Instant>,
    upnp_error: Option<String>,
    /// Names aren't checked (offline mode): OmniHub holds the operators.
    guarded: bool,
    /// Guarded operators made operators for this visit (lower-case).
    opped: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct Endpoints {
    pub web: install::Endpoints,
    pub mojang_api: String,
    pub ipify: String,
    pub ssdp: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Endpoints { web: install::Endpoints::default(), mojang_api: MOJANG_API.into(), ipify: net::IPIFY.into(), ssdp: net::SSDP.into() }
    }
}

/// Builds a server's start command (replaced in tests).
pub type Launcher = Arc<dyn Fn(&Server, &Path) -> Command + Send + Sync>;

pub struct ServerHub {
    dir: PathBuf,
    events: EventBus,
    servers: Mutex<Vec<Server>>,
    live: Mutex<HashMap<String, Live>>,
    pub web: install::Web,
    pub runtimes: install::Runtimes,
    mods: Arc<ModHub>,
    agent: ureq::Agent,
    ep: Endpoints,
    versions: Mutex<HashMap<Flavor, (Instant, Vec<String>)>>,
    gateway: Mutex<Option<(Instant, Result<net::Gateway, String>)>>,
    public_ip: Mutex<Option<(Instant, Option<String>)>>,
    pub launcher: Mutex<Option<Launcher>>,
    sys: Mutex<sysinfo::System>,
    awake: AtomicBool,
    quitting: AtomicBool,
}

fn level_of(dir: &Path) -> String {
    let p = Props::load(&dir.join("server.properties"));
    let l = p.get_or("level-name", "world").trim().to_string();
    crate::mods::install::safe_rel(&l).filter(|r| !r.contains('/')).unwrap_or_else(|| "world".into())
}

/// Start-up flags a user may add (no programs, no argument files).
fn flag_ok(f: &str) -> bool {
    // JVM flags that run a program or load more options or classes from a file.
    const NOT: [&str; 5] = ["-XX:OnOutOfMemoryError", "-XX:OnError", "-XX:VMOptionsFile", "-XX:Flags=", "-Xbootclasspath"];
    !f.is_empty() && f.len() < 200 && !f.contains(char::is_whitespace) && !f.contains('@') && (f.starts_with("-X") || f.starts_with("-D") || f.starts_with("--add-") || f.starts_with("--enable-")) && !f.starts_with("-jar") && !NOT.iter().any(|n| f.starts_with(n))
}

/// Text a command may carry (one line, no section signs that break formatting).
fn clean_text(s: &str, max: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(max).collect::<String>().trim().to_string()
}

impl ServerHub {
    pub fn new(data_dir: &Path, events: EventBus, mods: Arc<ModHub>) -> Arc<ServerHub> {
        ServerHub::with(data_dir, events, mods, Endpoints::default())
    }

    pub fn with(data_dir: &Path, events: EventBus, mods: Arc<ModHub>, ep: Endpoints) -> Arc<ServerHub> {
        let dir = data_dir.join("mcserver");
        let servers: Vec<Server> = std::fs::read(dir.join("servers.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let local = ep.mojang_api.starts_with("http://127.0.0.1");
        let mut cfg = ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(10))).timeout_global(Some(Duration::from_secs(30))).http_status_as_error(false);
        if local {
            cfg = cfg.proxy(None);
        }
        Arc::new(ServerHub {
            runtimes: install::Runtimes { dir: dir.join("java") },
            web: install::Web::new(ep.web.clone()),
            dir,
            events,
            servers: Mutex::new(servers),
            live: Mutex::new(HashMap::new()),
            mods,
            agent: cfg.build().into(),
            ep,
            versions: Mutex::new(HashMap::new()),
            gateway: Mutex::new(None),
            public_ip: Mutex::new(None),
            launcher: Mutex::new(None),
            sys: Mutex::new(sysinfo::System::new()),
            awake: AtomicBool::new(false),
            quitting: AtomicBool::new(false),
        })
    }

    // ---------- records ----------

    fn save(&self) -> Result<(), String> {
        let list = self.servers.lock().clone();
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let tmp = self.dir.join("servers.json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&list).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, self.dir.join("servers.json")).map_err(|e| e.to_string())
    }

    pub fn server(&self, id: &str) -> Result<Server, String> {
        self.servers.lock().iter().find(|s| s.id == id).cloned().ok_or_else(|| "That server is gone.".into())
    }

    fn edit(&self, id: &str, f: impl FnOnce(&mut Server)) -> Result<Server, String> {
        let s = {
            let mut list = self.servers.lock();
            let s = list.iter_mut().find(|s| s.id == id).ok_or("That server is gone.")?;
            f(s);
            s.clone()
        };
        self.save()?;
        Ok(s)
    }

    fn changed(&self, id: &str) {
        self.events.emit("mcserver:changed", json!({ "id": id }));
    }

    fn with_live<T>(&self, id: &str, f: impl FnOnce(&mut Live) -> T) -> T {
        let mut live = self.live.lock();
        f(live.entry(id.to_string()).or_default())
    }

    fn set_work(&self, id: &str, text: &str, done: u64, total: u64) {
        self.with_live(id, |l| l.work = Some(Work { text: text.into(), done, total }));
        self.events.emit("mcserver:work", json!({ "id": id, "text": text, "done": done, "total": total }));
    }

    /// A note from OmniHub in the server's console.
    fn note(&self, id: &str, text: &str, level: &str) {
        let (console, next) = self.with_live(id, |l| {
            l.console_dirty = true;
            (l.console.clone(), l.next.clone())
        });
        let mut c = console.lock();
        c.push_back(Line { n: next.fetch_add(1, Ordering::SeqCst), at: now(), text: format!("[OmniHub] {text}"), level: level.into() });
        while c.len() > 3000 {
            c.pop_front();
        }
    }

    fn proc_of(&self, id: &str) -> Option<Arc<Proc>> {
        self.live.lock().get(id).and_then(|l| l.proc.clone()).filter(|p| p.alive())
    }

    fn running(&self, id: &str) -> Result<Arc<Proc>, String> {
        let p = self.proc_of(id).ok_or("The server isn't running.")?;
        if self.live.lock().get(id).map(|l| l.state) != Some(State::Running) {
            return Err("Wait until the server has started.".into());
        }
        Ok(p)
    }

    // ---------- the list ----------

    fn icon_of(dir: &Path) -> Option<String> {
        let b = std::fs::read(dir.join("server-icon.png")).ok().filter(|b| b.len() < 1 << 20)?;
        Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(b)))
    }

    pub fn view(&self, id: &str) -> Result<View, String> {
        let s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        let p = Props::load(&dir.join("server.properties"));
        let port: u16 = p.get_or("server-port", "25565").parse().unwrap_or(25565);
        let (state, since, online, max, problem, work, restart_needed, pid) = {
            let live = self.live.lock();
            match live.get(id) {
                Some(l) => (l.state, l.since, l.online.clone(), l.max, l.problem.clone(), l.work.clone(), l.restart_needed, l.proc.as_ref().map(|p| p.pid)),
                None => (State::Stopped, 0, vec![], None, None, None, false, None),
            }
        };
        let (mut mem, mut cpu) = (0, 0.0);
        if let Some(pid) = pid {
            let mut sys = self.sys.lock();
            let pid = sysinfo::Pid::from_u32(pid);
            sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
            if let Some(pr) = sys.process(pid) {
                mem = pr.memory() / (1 << 20);
                // Of the whole PC (each core counts as 100% in sysinfo).
                cpu = pr.cpu_usage() / std::thread::available_parallelism().map_or(1, |n| n.get()) as f32;
            }
        }
        let lan = net::lan_ips().into_iter().find(|a| a.is_private()).map(|a| format!("{a}:{port}"));
        Ok(View {
            max_players: max.unwrap_or_else(|| p.get_or("max-players", "20").parse().unwrap_or(20)),
            motd: p.get_or("motd", "A Minecraft Server").to_string(),
            whitelist: p.get_or("white-list", "false") == "true",
            online_mode: p.get_or("online-mode", "true") != "false",
            icon: Self::icon_of(&dir),
            server: s,
            state,
            since,
            online,
            port,
            problem,
            work,
            memory_used_mb: mem,
            cpu,
            restart_needed,
            lan,
        })
    }

    pub fn list(&self) -> Vec<View> {
        let ids: Vec<String> = self.servers.lock().iter().map(|s| s.id.clone()).collect();
        ids.iter().filter_map(|id| self.view(id).ok()).collect()
    }

    /// The Minecraft versions a server type runs (kept for an hour).
    pub fn versions(&self, flavor: Flavor) -> Result<Vec<String>, String> {
        if let Some((at, v)) = self.versions.lock().get(&flavor) {
            if at.elapsed() < Duration::from_secs(3600) {
                return Ok(v.clone());
            }
        }
        let v = self.web.supported(flavor)?;
        self.versions.lock().insert(flavor, (Instant::now(), v.clone()));
        Ok(v)
    }

    // ---------- making one ----------

    fn free_port(&self) -> u16 {
        let used: Vec<u16> = self.servers.lock().iter().map(|s| Props::load(&Path::new(&s.dir).join("server.properties")).get_or("server-port", "25565").parse().unwrap_or(25565)).collect();
        (25565..26000).find(|p| !used.contains(p)).unwrap_or(25565)
    }

    fn default_memory(flavor: Flavor) -> u32 {
        let gb = crate::mods::minecraft::memory_for_pc();
        if flavor.mods() {
            gb * 1024
        } else {
            (gb / 2).max(2) * 1024
        }
    }

    /// Make a server: its folder and settings now, its program (and Java) in the background.
    pub fn create(self: &Arc<Self>, req: NewServer) -> Result<View, String> {
        let name: String = clean_text(&req.name, 60);
        if name.is_empty() {
            return Err("Give the server a name.".into());
        }
        if !req.eula {
            return Err("A Minecraft server only runs once you accept Minecraft's EULA (aka.ms/MinecraftEULA).".into());
        }
        let (mut flavor, mut mc, mut loader) = (req.flavor, req.mc.trim().to_string(), req.loader.trim().to_string());
        let mut from_label = None;
        match &req.from {
            Some(FromPack::Mix { id }) => {
                let m = self.mods.minecraft.mix(id)?;
                flavor = Flavor::from_loader(&m.loader).ok_or("That mix's loader can't run as a server.")?;
                mc = m.mc.clone();
                loader = m.loader_version.clone();
                from_label = Some(format!("Mix: {}", m.name));
            }
            Some(FromPack::File { path }) => {
                let b = std::fs::read(path).map_err(|e| format!("Couldn't open it: {e}"))?;
                let (idx, _) = crate::mods::minecraft::read_mrpack(&b)?;
                let (m, l, v) = pack_target(&idx);
                flavor = Flavor::from_loader(&l).ok_or("That pack's loader can't run as a server.")?;
                mc = m;
                loader = v;
                from_label = Some(format!("Modpack: {}", idx["name"].as_str().unwrap_or("pack")));
            }
            Some(FromPack::Launcher { id }) => {
                let i = self.mods.minecraft.launcher_pack(id)?;
                flavor = Flavor::from_loader(&i.loader).ok_or("That modpack's loader can't run as a server.")?;
                if i.mc.is_empty() {
                    return Err(format!("{} doesn't say which Minecraft version that modpack is for.", i.launcher_name));
                }
                mc = i.mc.clone();
                loader = i.loader_version.clone();
                from_label = Some(format!("{}: {}", i.launcher_name, i.name));
            }
            Some(FromPack::Modrinth { .. }) => from_label = Some("Modpack".into()),
            None => {}
        }
        if mc.is_empty() {
            return Err("Pick a Minecraft version.".into());
        }
        let id = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
        let slug = crate::mods::slug(&name).to_lowercase();
        let base = self.dir.join("servers");
        let mut dir = base.join(&slug);
        let mut n = 2;
        while dir.exists() {
            dir = base.join(format!("{slug}_{n}"));
            n += 1;
        }
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let port = req.port.filter(|p| *p >= 1024).unwrap_or_else(|| self.free_port());
        let mut p = Props::default();
        p.set("server-port", &port.to_string());
        p.set("motd", &clean_text(req.motd.as_deref().unwrap_or(&name), 59));
        p.set("max-players", &req.max_players.unwrap_or(10).clamp(1, 1000).to_string());
        p.set("online-mode", if req.online_mode { "true" } else { "false" });
        p.set("white-list", if req.whitelist { "true" } else { "false" });
        p.set("enforce-whitelist", if req.whitelist { "true" } else { "false" });
        p.set("enable-rcon", "false");
        if let Some(seed) = req.seed.as_deref().map(|s| clean_text(s, 64)).filter(|s| !s.is_empty()) {
            p.set("level-seed", &seed);
        }
        if let Some(g) = req.gamemode.as_deref().filter(|g| ["survival", "creative", "adventure", "spectator"].contains(g)) {
            p.set("gamemode", g);
        }
        if let Some(d) = req.difficulty.as_deref().filter(|d| ["peaceful", "easy", "normal", "hard"].contains(d)) {
            p.set("difficulty", d);
        }
        p.save(&dir.join("server.properties"))?;
        std::fs::write(dir.join("eula.txt"), format!("# Accepted in OmniHub on {} (https://aka.ms/MinecraftEULA)\neula=true\n", chrono::Local::now().format("%Y-%m-%d %H:%M"))).map_err(|e| e.to_string())?;
        let s = Server {
            id: id.clone(),
            name,
            flavor,
            mc,
            loader,
            dir: dir.to_string_lossy().into_owned(),
            memory_mb: req.memory_mb.unwrap_or_else(|| Self::default_memory(flavor)).clamp(512, 65536),
            from: from_label,
            created: now(),
            eula_at: Some(now()),
            owned: true,
            ..Default::default()
        };
        self.servers.lock().push(s);
        self.save()?;
        self.with_live(&id, |l| {
            l.state = State::Installing;
            l.since = now();
        });
        let me = self.clone();
        let (from, start) = (req.from.clone(), req.start);
        let tid = id.clone();
        std::thread::Builder::new()
            .name("mc-install".into())
            .spawn(move || {
                let r = me.install(&tid, from.as_ref());
                me.with_live(&tid, |l| {
                    l.state = State::Stopped;
                    l.work = None;
                    l.problem = r.as_ref().err().cloned();
                });
                match &r {
                    Ok(()) => me.note(&tid, "Ready. Press Start.", "info"),
                    Err(e) => me.note(&tid, &format!("Setting up failed: {e}"), "error"),
                }
                me.changed(&tid);
                if r.is_ok() && start {
                    let _ = me.start(&tid);
                }
            })
            .map_err(|e| e.to_string())?;
        self.changed(&id);
        self.view(&id)
    }

    /// Get Java and the server's program, then a pack's mods if it's made from one.
    fn install(self: &Arc<Self>, id: &str, from: Option<&FromPack>) -> Result<(), String> {
        let mut s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        // The pack's server files, its configs, its name and where it's from.
        let mut pack: Option<(Vec<crate::mods::minecraft::PackFile>, content::Overrides, String, String)> = None;
        let mut icon: Option<Vec<u8>> = None;
        match from {
            Some(FromPack::Modrinth { project, version }) => {
                self.set_work(id, "Downloading the modpack", 0, 0);
                let (b, title, icon_url) = self.mods.minecraft.api.mrpack(project, version)?;
                let (idx, files, over) = content::mrpack_parts(&b)?;
                let (mc, l, v) = pack_target(&idx);
                let flavor = Flavor::from_loader(&l).ok_or("That pack's loader can't run as a server.")?;
                s = self.edit(id, |x| {
                    x.flavor = flavor;
                    x.mc = mc;
                    x.loader = v;
                    x.from = Some(format!("Modpack: {title}"));
                    x.memory_mb = x.memory_mb.max(Self::default_memory(flavor));
                })?;
                icon = icon_url.and_then(|u| self.mods.minecraft.api.picture(&u).ok());
                pack = Some((files, over, title, "pack".into()));
            }
            Some(FromPack::File { path }) => {
                let b = std::fs::read(path).map_err(|e| e.to_string())?;
                let (idx, files, over) = content::mrpack_parts(&b)?;
                pack = Some((files, over, idx["name"].as_str().unwrap_or("pack").to_string(), "pack".into()));
            }
            Some(FromPack::Mix { id: mix }) => {
                let m = self.mods.minecraft.mix(mix)?;
                let mut files: Vec<_> = m.entries.iter().filter_map(|e| e.file.clone().map(|f| (e.project.clone(), f))).collect();
                // Mods that say they don't run on servers stay off.
                let ids: Vec<String> = files.iter().filter_map(|(p, _)| p.clone()).collect();
                let meta = if ids.is_empty() { HashMap::new() } else { self.mods.minecraft.api.project_meta(&ids).unwrap_or_default() };
                for (p, f) in files.iter_mut() {
                    if p.as_ref().and_then(|p| meta.get(p)).is_some_and(|m| m.server_side == "unsupported") {
                        f.server = false;
                    }
                }
                let over = content::folder_files(&self.mods.minecraft.overrides_dir(mix)?);
                icon = self.mods.minecraft.cover(mix).ok();
                pack = Some((files.into_iter().map(|(_, f)| f).collect(), over, m.name.clone(), "mix".into()));
            }
            Some(FromPack::Launcher { id: from }) => {
                self.set_work(id, "Reading the modpack's mods", 0, 0);
                let i = self.mods.minecraft.launcher_pack(from)?;
                let (files, over) = self.mods.minecraft.launcher_server_parts(&i)?;
                icon = crate::mods::launchers::icon(&i);
                pack = (!files.is_empty()).then(|| (files, over, i.name.clone(), "pack".into()));
            }
            None => {}
        }
        let (java_major, _) = self.web.java_for(s.flavor, &s.mc)?;
        let java = self.java_for(&Server { java_major, ..s.clone() }, id)?;
        let me = self.clone();
        let tid = id.to_string();
        let mut progress = move |done: u64, total: u64, text: &str| me.set_work(&tid, text, done, total);
        let inst = self.web.install(s.flavor, &s.mc, &s.loader, &dir, &java, &mut progress)?;
        self.edit(id, |x| {
            x.launch = Some(inst.launch.clone());
            x.loader = inst.loader_version.clone();
            x.java_major = inst.java;
            x.flags = inst.flags.clone();
        })?;
        if let Some((files, over, title, source)) = pack {
            let me = self.clone();
            let tid = id.to_string();
            let (added, skipped) = content::install_pack(&dir, &self.mods.minecraft.api, &files, &over, &title, &source, &mut |i, n, what| me.set_work(&tid, &format!("Adding {what}"), i as u64, n as u64))?;
            self.note(id, &format!("Added {added} mods from {title}; {skipped} that only run in the game itself were left out."), "info");
        }
        if let Some(b) = icon {
            let _ = Self::write_icon(&dir, &b);
        }
        Ok(())
    }

    /// `server-icon.png`: the picture in the server list (64×64).
    fn write_icon(dir: &Path, picture: &[u8]) -> Result<(), String> {
        let img = crate::mods::cover::decode(picture).ok_or("That picture can't be read.")?;
        let small = image::imageops::resize(&img, 64, 64, image::imageops::FilterType::Triangle);
        std::fs::write(dir.join("server-icon.png"), crate::mods::cover::png(&small)).map_err(|e| e.to_string())
    }

    pub fn set_icon(&self, id: &str, path: &str) -> Result<View, String> {
        let s = self.server(id)?;
        let b = std::fs::read(path).map_err(|e| e.to_string())?;
        Self::write_icon(Path::new(&s.dir), &b)?;
        self.with_live(id, |l| l.restart_needed |= l.state == State::Running);
        self.changed(id);
        self.view(id)
    }

    /// Take over a server folder made elsewhere (its type worked out from its files).
    pub fn adopt(&self, path: &str) -> Result<View, String> {
        let dir = PathBuf::from(path);
        if !dir.join("server.properties").is_file() && !dir.join("eula.txt").is_file() {
            return Err("That folder doesn't look like a Minecraft server (no server.properties).".into());
        }
        if self.servers.lock().iter().any(|s| Path::new(&s.dir) == dir) {
            return Err("OmniHub already has that server.".into());
        }
        let (flavor, mc, loader, launch) = detect(&dir).ok_or("OmniHub couldn't tell how this server starts (no server jar it knows).")?;
        let java_major = self.web.vanilla(&mc).map(|v| v.2).unwrap_or(21);
        let eula = std::fs::read_to_string(dir.join("eula.txt")).map(|t| t.contains("eula=true")).unwrap_or(false);
        let id = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
        let name = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Server".into());
        let s = Server { id: id.clone(), name, flavor, mc, loader, dir: dir.to_string_lossy().into_owned(), launch: Some(launch), java_major, memory_mb: Self::default_memory(flavor), created: now(), eula_at: eula.then(now), owned: false, backup_hours: 0, ..Default::default() };
        self.servers.lock().push(s);
        self.save()?;
        self.changed(&id);
        self.view(&id)
    }

    pub fn update(&self, id: &str, p: Patch) -> Result<View, String> {
        if let Some(f) = &p.flags {
            if let Some(bad) = f.iter().find(|f| !flag_ok(f)) {
                return Err(format!("“{bad}” isn't a start-up flag OmniHub passes on (use -X…, -XX:… or -D… flags)."));
            }
        }
        if let Some(j) = p.java_path.as_deref().filter(|j| !j.is_empty()) {
            let v = install::java_version(Path::new(j)).ok_or("That isn't a Java program OmniHub can run.")?;
            let need = self.server(id)?.java_major;
            if v < need {
                return Err(format!("That's Java {v}; this server needs Java {need} or newer."));
            }
        }
        let s = self.server(id)?;
        if p.eula == Some(true) {
            std::fs::write(Path::new(&s.dir).join("eula.txt"), "eula=true\n").map_err(|e| e.to_string())?;
        }
        let upnp_off = p.upnp == Some(false) && s.upnp;
        let restart = p.memory_mb.is_some() || p.java_path.is_some();
        self.edit(id, |s| {
            if let Some(n) = p.name.as_deref().map(|n| clean_text(n, 60)).filter(|n| !n.is_empty()) {
                s.name = n;
            }
            if let Some(m) = p.memory_mb {
                s.memory_mb = m.clamp(512, 65536);
            }
            if let Some(j) = p.java_path {
                s.java_path = Some(j).filter(|j| !j.is_empty());
            }
            if let Some(f) = p.flags {
                s.flags = f;
            }
            if let Some(v) = p.auto_start {
                s.auto_start = v;
            }
            if let Some(v) = p.auto_restart {
                s.auto_restart = v;
            }
            if let Some(v) = p.keep_awake {
                s.keep_awake = v;
            }
            if let Some(v) = p.backup_hours {
                s.backup_hours = v.min(24 * 30);
            }
            if let Some(v) = p.backup_keep {
                s.backup_keep = v.clamp(1, 100);
            }
            if let Some(v) = p.upnp {
                s.upnp = v;
            }
            if p.eula == Some(true) {
                s.eula_at = Some(now());
            }
        })?;
        if upnp_off {
            self.close_port(id);
        }
        if restart {
            self.with_live(id, |l| l.restart_needed |= l.state == State::Running);
        }
        self.changed(id);
        self.view(id)
    }

    /// Forget a server; its folder is deleted too if OmniHub made it and `files` is set.
    pub fn delete(&self, id: &str, files: bool) -> Result<(), String> {
        let s = self.server(id)?;
        if self.proc_of(id).is_some() {
            return Err("Stop the server first.".into());
        }
        if self.live.lock().get(id).is_some_and(|l| l.state == State::Installing) {
            return Err("Wait until it's set up.".into());
        }
        if files && s.owned && Path::new(&s.dir).starts_with(self.dir.join("servers")) {
            std::fs::remove_dir_all(&s.dir).map_err(|e| format!("Couldn't delete its folder: {e}"))?;
        }
        self.servers.lock().retain(|x| x.id != id);
        self.live.lock().remove(id);
        self.save()?;
        self.changed(id);
        Ok(())
    }

    // ---------- Java ----------

    /// The Java a server runs with: the one picked, OmniHub's (downloaded once), or the PC's.
    fn java_for(self: &Arc<Self>, s: &Server, id: &str) -> Result<PathBuf, String> {
        if let Some(p) = s.java_path.as_ref().filter(|p| !p.is_empty()) {
            let p = PathBuf::from(p);
            return if p.is_file() { Ok(p) } else { Err("The Java picked in this server's settings isn't there any more.".into()) };
        }
        if let Some(p) = self.runtimes.find(s.java_major, false) {
            return Ok(p);
        }
        let me = self.clone();
        let tid = id.to_string();
        let mut progress = move |done: u64, total: u64, text: &str| me.set_work(&tid, text, done, total);
        match self.runtimes.ensure(&self.web, s.java_major, false, &mut progress) {
            Ok(p) => Ok(p),
            Err(e) => match install::java_version(Path::new("java")) {
                Some(v) if v >= s.java_major => Ok(PathBuf::from("java")),
                _ => Err(e),
            },
        }
    }

    /// The Java program a server uses now (for the firewall), if it's there.
    pub fn java_exe(&self, id: &str) -> Option<PathBuf> {
        let s = self.server(id).ok()?;
        s.java_path.map(PathBuf::from).filter(|p| p.is_file()).or_else(|| self.runtimes.find(s.java_major, false))
    }

    // ---------- running ----------

    fn command(&self, s: &Server, java: &Path) -> Result<Command, String> {
        if let Some(l) = self.launcher.lock().clone() {
            return Ok(l(s, java));
        }
        let launch = s.launch.clone().ok_or("The server isn't set up yet.")?;
        let mut c = Command::new(java);
        let mem = s.memory_mb.max(512);
        c.arg(format!("-Xms{}M", mem.min(1024))).arg(format!("-Xmx{mem}M"));
        c.args(s.flags.iter().filter(|f| flag_ok(f)));
        c.args(["-Dstdout.encoding=UTF-8", "-Dstderr.encoding=UTF-8", "-Dfile.encoding=UTF-8", "-Dlog4j2.formatMsgNoLookups=true"]);
        match launch {
            Launch::Jar { jar } => {
                c.arg("-jar").arg(jar);
            }
            Launch::Args { args } => {
                if Path::new(&s.dir).join("user_jvm_args.txt").is_file() {
                    c.arg("@user_jvm_args.txt");
                }
                c.arg(format!("@{args}"));
            }
        }
        c.arg("nogui").current_dir(&s.dir);
        Ok(c)
    }

    fn running_file(&self) -> PathBuf {
        self.dir.join("running.json")
    }

    fn note_running(&self, id: &str, pid: Option<u32>) {
        let mut m: HashMap<String, u32> = std::fs::read(self.running_file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        match pid {
            Some(p) => {
                m.insert(id.into(), p);
            }
            None => {
                m.remove(id);
            }
        }
        let _ = std::fs::create_dir_all(&self.dir);
        let _ = std::fs::write(self.running_file(), serde_json::to_vec(&m).unwrap_or_default());
    }

    /// Start the server (Java is fetched first if it's missing).
    pub fn start(self: &Arc<Self>, id: &str) -> Result<View, String> {
        let s = self.server(id)?;
        if s.launch.is_none() {
            return Err("The server isn't set up yet.".into());
        }
        if s.eula_at.is_none() && !std::fs::read_to_string(Path::new(&s.dir).join("eula.txt")).is_ok_and(|t| t.contains("eula=true")) {
            return Err("Accept Minecraft's EULA for this server first (Settings).".into());
        }
        let port = Props::load(&Path::new(&s.dir).join("server.properties")).get_or("server-port", "25565").to_string();
        for other in self.servers.lock().iter().filter(|o| o.id != id) {
            let theirs = Props::load(&Path::new(&other.dir).join("server.properties")).get_or("server-port", "25565").to_string();
            if theirs == port && self.live.lock().get(&other.id).is_some_and(|l| l.proc.is_some()) {
                return Err(format!("{} is running on the same port ({port}). Stop it, or change this server's port in Settings.", other.name));
            }
        }
        self.with_live(id, |l| {
            if matches!(l.state, State::Installing | State::Starting | State::Running | State::Stopping) || l.proc.is_some() {
                return Err(if l.state == State::Installing { "It's still being set up." } else { "It's already running." }.to_string());
            }
            if let Some(b) = &l.busy {
                return Err(format!("Wait: {b}"));
            }
            l.state = State::Starting;
            l.since = now();
            l.problem = None;
            l.online.clear();
            l.restart_at = None;
            l.restart_needed = false;
            Ok(())
        })?;
        self.changed(id);
        let me = self.clone();
        let tid = id.to_string();
        std::thread::Builder::new()
            .name("mc-start".into())
            .spawn(move || {
                if let Err(e) = me.spawn_server(&tid) {
                    me.with_live(&tid, |l| {
                        l.state = State::Stopped;
                        l.problem = Some(e.clone());
                        l.work = None;
                    });
                    me.note(&tid, &e, "error");
                    me.changed(&tid);
                }
            })
            .map_err(|e| e.to_string())?;
        self.view(id)
    }

    fn spawn_server(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let s = self.server(id)?;
        let java = self.java_for(&s, id)?;
        self.with_live(id, |l| l.work = None);
        self.prepare_join(&s, id);
        let cmd = self.command(&s, &java)?;
        let (console, next) = self.with_live(id, |l| (l.console.clone(), l.next.clone()));
        self.note(id, &format!("Starting {} {} with {} MB of memory…", s.flavor.label(), s.mc, s.memory_mb), "info");
        let weak: Weak<ServerHub> = Arc::downgrade(self);
        let lid = id.to_string();
        let on_line: process::OnLine = Arc::new(move |_line: &Line, said: &Said| {
            if let Some(h) = weak.upgrade() {
                h.heard(&lid, said);
            }
        });
        let weak: Weak<ServerHub> = Arc::downgrade(self);
        let eid = id.to_string();
        let on_exit = Box::new(move |code: Option<i32>| {
            if let Some(h) = weak.upgrade() {
                h.exited(&eid, code);
            }
        });
        let p = Proc::spawn(cmd, console, next, on_line, on_exit)?;
        self.note_running(id, Some(p.pid));
        self.with_live(id, |l| l.proc = Some(p));
        self.changed(id);
        Ok(())
    }

    /// A console line that matters.
    fn heard(self: &Arc<Self>, id: &str, said: &Said) {
        let mut changed = true;
        let mut started = false;
        let mut login: Option<(String, String, bool)> = None;
        let mut deop: Option<String> = None;
        self.with_live(id, |l| {
            l.console_dirty = true;
            match said {
                Said::Started => {
                    l.state = State::Running;
                    l.since = now();
                    started = true;
                }
                Said::Joined(n) => {
                    if !l.online.iter().any(|o| &o.name == n) {
                        l.online.push(Online { name: n.clone(), uuid: None, since: now(), pos: None, dimension: None });
                    }
                }
                Said::LoggedIn(n, place) => {
                    login = Some((n.clone(), place.clone(), l.guarded));
                    changed = false;
                }
                Said::Left(n) => {
                    l.online.retain(|o| &o.name != n);
                    // A guarded operator is one only while they're on.
                    if l.opped.remove(&n.to_lowercase()) {
                        deop = Some(n.clone());
                    }
                }
                Said::List(names, max) => {
                    let mut next: Vec<Online> = names.iter().map(|(n, u)| l.online.iter().find(|o| &o.name == n).cloned().map(|mut o| {
                        if u.is_some() {
                            o.uuid = u.clone();
                        }
                        o
                    }).unwrap_or(Online { name: n.clone(), uuid: u.clone(), since: now(), pos: None, dimension: None })).collect();
                    next.sort_by_key(|o| o.since);
                    changed = next != l.online || *max != l.max;
                    l.online = next;
                    if max.is_some() {
                        l.max = *max;
                    }
                }
                Said::Problem(p) => l.problem = Some(p.clone()),
                Said::Chat(..) | Said::Nothing => changed = false,
            }
        });
        if let (Some(n), Some(p)) = (deop, self.proc_of(id)) {
            let _ = p.send(&format!("deop {n}"));
        }
        if let Said::Left(n) = said {
            self.absorb_ops(id, Some(n));
        }
        if let Some((n, place, guarded)) = login {
            self.on_login(id, &n, &place, guarded);
        }
        if started {
            let me = self.clone();
            let tid = id.to_string();
            std::thread::spawn(move || {
                if let Some(p) = me.proc_of(&tid) {
                    quiet_list(&p);
                }
                me.open_port_if_wanted(&tid);
            });
        }
        if changed {
            self.changed(id);
        }
    }

    fn exited(self: &Arc<Self>, id: &str, code: Option<i32>) {
        self.note_running(id, None);
        let auto = self.server(id).map(|s| s.auto_restart).unwrap_or(false);
        let quitting = self.quitting.load(Ordering::SeqCst);
        let mut msg = String::new();
        self.with_live(id, |l| {
            let wanted = l.proc.as_ref().is_some_and(|p| p.stopping.load(Ordering::SeqCst));
            // A server that never finished starting would only fail the same way again.
            let while_starting = l.state == State::Starting;
            l.proc = None;
            l.online.clear();
            l.opped.clear();
            l.since = now();
            if wanted || code == Some(0) {
                l.state = State::Stopped;
                msg = "The server stopped.".into();
            } else {
                l.state = State::Crashed;
                let t = now();
                l.crashes.retain(|c| t - c < 10 * 60);
                l.crashes.push(t);
                if l.problem.is_none() {
                    l.problem = Some(format!("The server stopped unexpectedly (exit code {}).", code.map(|c| c.to_string()).unwrap_or_else(|| "none".into())));
                }
                if while_starting {
                    msg = "The server stopped while starting up. The console shows why.".into();
                    if l.problem.is_none() {
                        l.problem = Some(msg.clone());
                    }
                } else if auto && !quitting && l.crashes.len() < 3 {
                    l.restart_at = Some(Instant::now() + Duration::from_secs(5));
                    msg = "The server stopped unexpectedly; OmniHub restarts it in 5 seconds.".into();
                } else if auto && l.crashes.len() >= 3 {
                    msg = "The server crashed 3 times in 10 minutes, so OmniHub stopped restarting it. The console shows why.".into();
                    l.problem = Some(msg.clone());
                } else {
                    msg = "The server stopped unexpectedly.".into();
                }
            }
        });
        self.note(id, &msg, if msg.starts_with("The server stopped.") { "info" } else { "error" });
        self.close_port(id);
        self.changed(id);
    }

    /// Save and stop (in the background; ended after 90 seconds if it hangs).
    pub fn stop(self: &Arc<Self>, id: &str) -> Result<View, String> {
        let p = self.proc_of(id).ok_or("The server isn't running.")?;
        self.with_live(id, |l| {
            l.state = State::Stopping;
            l.restart_at = None;
        });
        self.changed(id);
        let me = self.clone();
        let tid = id.to_string();
        std::thread::spawn(move || {
            if !p.stop(Duration::from_secs(90)) {
                me.note(&tid, "The server didn't stop within 90 seconds, so OmniHub ended it.", "warn");
                p.kill();
            }
        });
        self.view(id)
    }

    /// Stop, then start again.
    pub fn restart(self: &Arc<Self>, id: &str) -> Result<View, String> {
        let Some(p) = self.proc_of(id) else { return self.start(id) };
        self.with_live(id, |l| l.state = State::Stopping);
        self.changed(id);
        let me = self.clone();
        let tid = id.to_string();
        std::thread::spawn(move || {
            if !p.stop(Duration::from_secs(90)) {
                p.kill();
            }
            for _ in 0..100 {
                if me.proc_of(&tid).is_none() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            if let Err(e) = me.start(&tid) {
                me.note(&tid, &e, "error");
            }
        });
        self.view(id)
    }

    /// End it at once (anything not saved yet is lost).
    pub fn kill(&self, id: &str) -> Result<(), String> {
        let p = self.proc_of(id).ok_or("The server isn't running.")?;
        p.kill();
        Ok(())
    }

    pub fn console(&self, id: &str, after: Option<u64>) -> Vec<Line> {
        let c = self.with_live(id, |l| l.console.clone());
        let c = c.lock();
        c.iter().filter(|l| after.is_none_or(|a| l.n > a)).cloned().collect()
    }

    /// Type a command into the server.
    pub fn command_line(&self, id: &str, text: &str) -> Result<(), String> {
        let p = self.proc_of(id).ok_or("The server isn't running.")?;
        let t = clean_text(text, 1000);
        if t.is_empty() {
            return Ok(());
        }
        let (console, next) = self.with_live(id, |l| {
            l.console_dirty = true;
            (l.console.clone(), l.next.clone())
        });
        console.lock().push_back(Line { n: next.fetch_add(1, Ordering::SeqCst), at: now(), text: format!("> {}", t.trim_start_matches('/')), level: "cmd".into() });
        // While names aren't checked, operators typed here go through the guard too.
        if let ["op" | "deop", name] = t.trim_start_matches('/').split_whitespace().collect::<Vec<_>>()[..] {
            let dir = PathBuf::from(&self.server(id)?.dir);
            if players::valid_name(name) && self.guarded(id, &dir) {
                let reply = self.guarded_op(id, &dir, name, t.trim_start_matches('/').starts_with("op "))?;
                self.note(id, &reply, "info");
                return Ok(());
            }
        }
        p.send(&t)
    }

    // ---------- settings ----------

    pub fn properties(&self, id: &str) -> Result<Vec<(String, String)>, String> {
        let s = self.server(id)?;
        Ok(Props::load(&Path::new(&s.dir).join("server.properties")).pairs())
    }

    /// Change settings; ones the server can take while running are applied now.
    pub fn set_properties(&self, id: &str, changes: &[(String, String)]) -> Result<View, String> {
        let s = self.server(id)?;
        let path = Path::new(&s.dir).join("server.properties");
        let mut p = Props::load(&path);
        let mut live_cmds = Vec::new();
        let mut needs_restart = false;
        for (k, v) in changes {
            let k = k.trim();
            if k.is_empty() || k.len() > 80 || !k.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_')) {
                return Err(format!("“{k}” isn't a setting name."));
            }
            let v = v.chars().filter(|c| *c != '\n' && *c != '\r').collect::<String>();
            match k {
                "server-port" | "query.port" | "rcon.port" => {
                    let n: u32 = v.trim().parse().map_err(|_| "Ports are numbers.".to_string())?;
                    if !(1024..=65535).contains(&n) {
                        return Err("Pick a port from 1024 to 65535.".into());
                    }
                }
                "max-players" => {
                    let n: u32 = v.trim().parse().map_err(|_| "The most players is a number.".to_string())?;
                    if !(1..=10_000).contains(&n) {
                        return Err("Between 1 and 10,000 players.".into());
                    }
                }
                "view-distance" | "simulation-distance" => {
                    let n: u32 = v.trim().parse().map_err(|_| "Distances are numbers.".to_string())?;
                    if !(2..=32).contains(&n) {
                        return Err("Distances go from 2 to 32 chunks.".into());
                    }
                }
                "enable-rcon" if v.trim() == "true" && p.get("rcon.password").is_none_or(|x| x.len() < 12) => {
                    return Err("Set an rcon.password of 12 or more characters before turning RCON on.".into());
                }
                _ => {}
            }
            if p.get(k) == Some(v.as_str()) {
                continue;
            }
            p.set(k, &v);
            match k {
                "difficulty" => live_cmds.push(format!("difficulty {}", v.trim())),
                "white-list" => live_cmds.push(format!("whitelist {}", if v.trim() == "true" { "on" } else { "off" })),
                "gamemode" => live_cmds.push(format!("defaultgamemode {}", v.trim())),
                _ => needs_restart = true,
            }
        }
        p.save(&path)?;
        if let Some(proc) = self.proc_of(id) {
            for c in live_cmds.iter().filter(|c| c.split_whitespace().nth(1).is_some_and(|w| w.chars().all(|ch| ch.is_ascii_alphanumeric()))) {
                let _ = proc.send(c);
            }
            if needs_restart {
                self.with_live(id, |l| l.restart_needed = true);
            }
        }
        self.changed(id);
        self.view(id)
    }

    // ---------- players ----------

    pub fn players(&self, id: &str) -> Result<PlayersView, String> {
        let s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        let p = Props::load(&dir.join("server.properties"));
        let online = self.live.lock().get(id).map(|l| l.online.clone()).unwrap_or_default();
        let g = guard::load(&dir);
        let (wl, mut ops, bans) = (players::read_list(&dir, List::Whitelist), players::read_list(&dir, List::Ops), players::read_list(&dir, List::Bans));
        ops.extend(g.ops.iter().cloned());
        let mut known: Vec<Known> = Vec::new();
        let mut add = |name: &str, uuid: &str| {
            if !known.iter().any(|k| k.name.eq_ignore_ascii_case(name)) {
                let ban = bans.iter().find(|b| b.name.eq_ignore_ascii_case(name));
                known.push(Known {
                    name: name.into(),
                    uuid: uuid.into(),
                    online: online.iter().any(|o| o.name.eq_ignore_ascii_case(name)),
                    op: ops.iter().any(|o| o.name.eq_ignore_ascii_case(name)),
                    whitelisted: wl.iter().any(|o| o.name.eq_ignore_ascii_case(name)),
                    banned: ban.is_some(),
                    ban_reason: ban.and_then(|b| b.reason.clone()),
                });
            }
        };
        for o in &online {
            add(&o.name, o.uuid.as_deref().unwrap_or(""));
        }
        for x in wl.iter().chain(ops.iter()).chain(bans.iter()) {
            add(&x.name, &x.uuid);
        }
        for c in players::seen(&dir) {
            add(&c.name, &c.uuid);
        }
        known.sort_by(|a, b| b.online.cmp(&a.online).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        Ok(PlayersView {
            // The setting (the running server's own number can be older until a restart).
            max_players: p.get_or("max-players", "20").parse().unwrap_or(20),
            whitelist_on: p.get_or("white-list", "false") == "true",
            enforce: p.get_or("enforce-whitelist", "false") == "true",
            online_mode: p.get_or("online-mode", "true") != "false",
            waiting: if p.get_or("online-mode", "true") == "false" { g.waiting } else { vec![] },
            online,
            known,
        })
    }

    /// A player's name as the server knows it (online, or seen before), for commands.
    fn player_name(&self, id: &str, name: &str) -> Result<String, String> {
        let name = name.trim();
        if let Some(o) = self.live.lock().get(id).and_then(|l| l.online.iter().find(|o| o.name.eq_ignore_ascii_case(name)).map(|o| o.name.clone())) {
            return Ok(o);
        }
        if players::valid_name(name) {
            Ok(name.to_string())
        } else {
            Err("That isn't a Minecraft name.".into())
        }
    }

    /// Where a player is and what they carry: asked of the server if they're online, else from their saved file.
    pub fn player(&self, id: &str, name: &str) -> Result<PlayerView, String> {
        let s = self.server(id)?;
        let name = self.player_name(id, name)?;
        let online = self.live.lock().get(id).and_then(|l| l.online.iter().find(|o| o.name == name).cloned());
        if let (Some(o), Ok(p)) = (&online, self.running(id)) {
            let prefix = format!("{name} has the following entity data: ");
            let pre = prefix.clone();
            let lines = p.ask(&[format!("data get entity {name}")], move |m| m.starts_with(&pre) || m == "No entity was found", |m| m.contains(" has the following entity data: ") || m == "No entity was found", Duration::from_secs(4))?;
            if let Some(t) = lines.iter().find_map(|l| l.strip_prefix(&prefix)).and_then(|t| nbt::snbt(t).ok()) {
                return Ok(PlayerView { name, uuid: o.uuid.clone(), online: true, live: true, details: players::details(&t) });
            }
        }
        let dir = PathBuf::from(&s.dir);
        let uuid = online.as_ref().and_then(|o| o.uuid.clone()).or_else(|| players::seen(&dir).into_iter().find(|c| c.name.eq_ignore_ascii_case(&name)).map(|c| c.uuid)).or_else(|| {
            [List::Whitelist, List::Ops, List::Bans].iter().flat_map(|l| players::read_list(&dir, *l)).find(|x| x.name.eq_ignore_ascii_case(&name)).map(|x| x.uuid)
        });
        let details = uuid.as_deref().and_then(|u| players::saved(&dir.join(level_of(&dir)), u)).unwrap_or_default();
        Ok(PlayerView { name, uuid, online: online.is_some(), live: false, details })
    }

    /// Who a name is: from Mojang (online-mode servers) or worked out (offline mode).
    fn identity(&self, dir: &Path, name: &str) -> Result<Listed, String> {
        let offline = Props::load(&dir.join("server.properties")).get_or("online-mode", "true") == "false";
        let seen = players::seen(dir).into_iter().find(|c| c.name.eq_ignore_ascii_case(name));
        if let Some(c) = seen.as_ref().filter(|c| guard::is_offline_uuid(&c.uuid) == offline) {
            return Ok(Listed::plain(&c.uuid, &c.name));
        }
        if offline {
            // Made from the name exactly as the player types it (as the server last saw it, if it did).
            let name = seen.map_or_else(|| name.to_string(), |c| c.name);
            return Ok(Listed::plain(&players::offline_uuid(&name), &name));
        }
        match players::mojang_uuid(&self.agent, &self.ep.mojang_api, name)? {
            Some((u, n)) => Ok(Listed::plain(&u, &n)),
            None => Err(format!("There's no Minecraft account called {name}.")),
        }
    }

    /// Do something to a player; returns what the server said.
    pub fn act(&self, id: &str, name: &str, a: Action) -> Result<String, String> {
        let s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        let name = self.player_name(id, name)?;
        let running = self.running(id).ok();
        let reason = |r: &Option<String>| r.as_deref().map(|r| clean_text(r, 200)).filter(|r| !r.is_empty());
        // Lists work while stopped too, through their files.
        let file_change = |l: List, add: bool, why: Option<String>| -> Result<String, String> {
            if add {
                let who = self.identity(&dir, &name)?;
                players::add_to(&dir, l, who, why.as_deref())?;
            } else {
                players::remove_from(&dir, l, &name)?;
            }
            Ok("Done (it applies when the server starts).".into())
        };
        // Allowed places, and operators while names aren't checked, go through OmniHub's guard.
        match &a {
            Action::Allow { place } | Action::Refuse { place } => {
                let mut g = guard::load(&dir);
                let allow = matches!(a, Action::Allow { .. });
                if allow {
                    g.allow(&name, place);
                } else {
                    g.waiting.retain(|w| !(w.name.eq_ignore_ascii_case(&name) && &w.place == place));
                }
                guard::save(&dir, &g)?;
                self.changed(id);
                return Ok(if allow { format!("{name} can join from {place} now. Ask them to join again.") } else { "Kept out.".into() });
            }
            Action::Op | Action::Deop if self.guarded(id, &dir) => return self.guarded_op(id, &dir, &name, a == Action::Op),
            _ => {}
        }
        let Some(p) = running else {
            return match &a {
                Action::Whitelist { on } => file_change(List::Whitelist, *on, None),
                Action::Op => file_change(List::Ops, true, None),
                Action::Deop => file_change(List::Ops, false, None),
                Action::Ban { reason: r } => file_change(List::Bans, true, reason(r)),
                Action::Pardon => file_change(List::Bans, false, None),
                _ => Err("Start the server to do that (the player must be online).".into()),
            };
        };
        let cmd = match &a {
            Action::Give { item, count } => {
                if !players::valid_item(item) {
                    return Err("That isn't an item id (like minecraft:diamond).".into());
                }
                format!("give {name} {item} {}", (*count).clamp(1, 6400))
            }
            Action::Clear { item: Some(item), count } => {
                if !players::valid_item(item) {
                    return Err("That isn't an item id.".into());
                }
                match count {
                    Some(c) => format!("clear {name} {item} {}", (*c).min(100_000)),
                    None => format!("clear {name} {item}"),
                }
            }
            Action::Clear { item: None, .. } => format!("clear {name}"),
            Action::ClearSlot { slot } => format!("item replace entity {name} {} with minecraft:air", players::slot_name(*slot).ok_or("No such slot.")?),
            Action::Teleport { x, y, z, dimension } => {
                if ![x, y, z].iter().all(|v| v.is_finite() && v.abs() < 30_000_000.0) {
                    return Err("Those coordinates are outside the world.".into());
                }
                let dim = dimension.as_deref().unwrap_or("minecraft:overworld");
                if !players::valid_item(dim) {
                    return Err("That isn't a dimension.".into());
                }
                format!("execute in {dim} run tp {name} {x} {y} {z}")
            }
            Action::TeleportTo { player } => format!("tp {name} {}", self.player_name(id, player)?),
            Action::Spawn => {
                let sp = map::spawn(&dir, &level_of(&dir)).unwrap_or([0, 100, 0]);
                format!("execute in minecraft:overworld run tp {name} {} {} {}", sp[0], sp[1], sp[2])
            }
            Action::Gamemode { mode } => {
                if !["survival", "creative", "adventure", "spectator"].contains(&mode.as_str()) {
                    return Err("Survival, creative, adventure or spectator.".into());
                }
                format!("gamemode {mode} {name}")
            }
            Action::Heal => format!("effect give {name} minecraft:instant_health 1 3 true"),
            Action::Feed => format!("effect give {name} minecraft:saturation 1 20 true"),
            Action::Message { text } => {
                let t = clean_text(text, 250);
                if t.is_empty() {
                    return Err("Write a message.".into());
                }
                format!("tell {name} {t}")
            }
            Action::Kick { reason: r } => format!("kick {name} {}", reason(r).unwrap_or_else(|| "Kicked by an operator.".into())),
            Action::Ban { reason: r } => format!("ban {name} {}", reason(r).unwrap_or_else(|| "Banned by an operator.".into())),
            Action::Pardon => format!("pardon {name}"),
            Action::Op => format!("op {name}"),
            Action::Deop => format!("deop {name}"),
            Action::Whitelist { on } => format!("whitelist {} {name}", if *on { "add" } else { "remove" }),
            Action::Allow { .. } | Action::Refuse { .. } => unreachable!("handled above"),
        };
        // The server's answer is the next line it prints that isn't someone else's news.
        let lines = p.ask(std::slice::from_ref(&cmd), reply_like, reply_like, Duration::from_secs(3)).unwrap_or_default();
        Ok(lines.into_iter().next().unwrap_or_else(|| "Sent.".into()))
    }

    /// Names aren't checked: the running server's mode, or the settings' while it's stopped.
    fn guarded(&self, id: &str, dir: &Path) -> bool {
        match self.live.lock().get(id).filter(|l| l.proc.is_some()) {
            Some(l) => l.guarded,
            None => Props::load(&dir.join("server.properties")).get_or("online-mode", "true") == "false",
        }
    }

    /// Operators while names aren't checked: held by OmniHub, and made
    /// operators only when they join from a place the owner allowed.
    fn guarded_op(&self, id: &str, dir: &Path, name: &str, on: bool) -> Result<String, String> {
        let mut g = guard::load(dir);
        let p = self.running(id).ok();
        let playing = self.live.lock().get(id).is_some_and(|l| l.online.iter().any(|o| o.name.eq_ignore_ascii_case(name)));
        let reply = if on {
            g.add_op_from_last(self.identity(dir, name)?);
            let from = g.last.get(&name.to_lowercase()).cloned();
            match (&p, playing) {
                (Some(p), true) => {
                    self.with_live(id, |l| l.opped.insert(name.to_lowercase()));
                    let _ = p.send(&format!("op {name}"));
                    format!("{name} is an operator now, when joining from {}.", from.unwrap_or_else(|| "where they play now".into()))
                }
                _ => match from {
                    Some(f) => format!("{name} becomes an operator when they join from {f} (where they last played). Anywhere else asks you first."),
                    None => format!("{name} becomes an operator once you allow the place they join from."),
                },
            }
        } else {
            g.remove_op(name);
            players::remove_from(dir, List::Ops, name)?;
            if let Some(p) = &p {
                self.with_live(id, |l| l.opped.remove(&name.to_lowercase()));
                let _ = p.send(&format!("deop {name}"));
            }
            format!("{name} is no longer an operator.")
        };
        guard::save(dir, &g)?;
        self.changed(id);
        Ok(reply)
    }

    /// Someone logged in: remember from where, and guard operator names
    /// while names aren't checked.
    fn on_login(&self, id: &str, name: &str, place: &str, guarded: bool) {
        let Ok(s) = self.server(id) else { return };
        let dir = PathBuf::from(&s.dir);
        if guarded {
            self.absorb_ops(id, None);
        }
        let mut g = guard::load(&dir);
        let mut send: Vec<String> = Vec::new();
        let (mut kept_out, mut news) = (false, false);
        if guarded && g.is_op(name) {
            let key = name.to_lowercase();
            if g.may_join(name, place) {
                self.with_live(id, |l| l.opped.insert(key));
                send.push(format!("op {name}"));
            } else {
                // Logging in again from elsewhere replaces a session that was an operator.
                if self.with_live(id, |l| l.opped.remove(&key)) {
                    send.push(format!("deop {name}"));
                }
                send.push(format!("kick {name} {}", guard::KICK));
                news = g.wait(name, place, now());
                kept_out = true;
            }
        }
        // Where they play from, unless it was kept out.
        if !kept_out {
            g.last.insert(name.to_lowercase(), place.to_string());
        }
        if let Err(e) = guard::save(&dir, &g) {
            tracing::warn!("couldn't save the operator guard for {id}: {e}");
        }
        if let Some(p) = self.proc_of(id) {
            for c in &send {
                let _ = p.send(c);
            }
        }
        if news {
            self.note(id, &format!("Kept {name} out: that name is an operator here and joined from a new place ({place}). If it's really them, allow it in Players."), "warn");
            self.events.emit("mcserver:guard", json!({ "id": id, "server": s.name, "name": name, "place": place }));
            self.changed(id);
        }
    }

    /// Operators made in the game (an operator typed /op) join the guard too:
    /// an operator only while they're on, and from where they joined.
    fn absorb_ops(&self, id: &str, leaving: Option<&str>) {
        let Ok(s) = self.server(id) else { return };
        let dir = PathBuf::from(&s.dir);
        let Some((opped, online)) = self.live.lock().get(id).filter(|l| l.guarded).map(|l| (l.opped.clone(), l.online.iter().map(|o| o.name.to_lowercase()).collect::<HashSet<_>>())) else { return };
        let extra: Vec<Listed> = players::read_list(&dir, List::Ops).into_iter().filter(|o| !opped.contains(&o.name.to_lowercase())).collect();
        if extra.is_empty() {
            return;
        }
        let mut g = guard::load(&dir);
        let mut send = Vec::new();
        for o in extra {
            let key = o.name.to_lowercase();
            let on = online.contains(&key);
            let left = leaving.is_some_and(|n| n.eq_ignore_ascii_case(&o.name));
            if !on {
                send.push(format!("deop {}", o.name));
            }
            // Already held (a deop still on its way): nothing new is allowed.
            if g.is_op(&o.name) {
                continue;
            }
            if on {
                // Made an operator while playing: where they play from now is theirs.
                self.with_live(id, |l| l.opped.insert(key));
            }
            if on || left {
                g.add_op_from_last(o);
            } else {
                g.add_op(o);
            }
        }
        let _ = guard::save(&dir, &g);
        if let Some(p) = self.proc_of(id) {
            for c in &send {
                let _ = p.send(c);
            }
        }
    }

    /// Before starting: the lists and saved players use the kind of UUID the
    /// server will use, and OmniHub holds the operators while names aren't checked.
    fn prepare_join(&self, s: &Server, id: &str) {
        let dir = PathBuf::from(&s.dir);
        let offline = Props::load(&dir.join("server.properties")).get_or("online-mode", "true") == "false";
        let held = match guard::hold_or_return_ops(&dir, offline) {
            Ok(g) => g.ops.len(),
            Err(e) => {
                self.note(id, &format!("Couldn't sort out the operators: {e}"), "warn");
                0
            }
        };
        let (agent, api) = (self.agent.clone(), self.ep.mojang_api.clone());
        let moved = guard::match_accounts(&dir, &dir.join(level_of(&dir)), offline, &mut |n| players::mojang_uuid(&agent, &api, n).ok().flatten());
        if !moved.is_empty() {
            let kind = if offline { "name-only (no paid account needed)" } else { "Minecraft account" };
            self.note(id, &format!("Moved {} to {kind} UUIDs: whitelist, operators, bans and saved things.", moved.join(", ")), "info");
        }
        if offline {
            self.note(id, &format!("Players without a paid Minecraft account can join. Names aren't checked, so OmniHub guards the operators{}: an operator's name from a new place waits for your OK.", if held > 0 { format!(" ({held})") } else { String::new() }), "info");
        }
        self.with_live(id, |l| {
            l.guarded = offline;
            l.opped.clear();
        });
    }

    /// Whitelist on or off (and enforced, so players not on it are removed).
    pub fn set_whitelist(&self, id: &str, on: bool) -> Result<View, String> {
        let v = if on { "true" } else { "false" };
        self.set_properties(id, &[("white-list".into(), v.into()), ("enforce-whitelist".into(), v.into())])
    }

    /// Players' positions now; asking keeps them coming for a while (the map and player list).
    pub fn watch(&self, id: &str) -> Vec<Online> {
        self.with_live(id, |l| {
            l.watch_until = Some(Instant::now() + Duration::from_secs(15));
            l.online.clone()
        })
    }

    fn poll_positions(self: &Arc<Self>, id: &str) {
        let Ok(p) = self.running(id) else { return };
        let names: Vec<String> = self.live.lock().get(id).map(|l| l.online.iter().map(|o| o.name.clone()).collect()).unwrap_or_default();
        if names.is_empty() {
            return;
        }
        let lines = p.ask(&["execute as @a run data get entity @s Pos".into(), "execute as @a run data get entity @s Dimension".into(), "list uuids".into()], |m| m.contains(" has the following entity data: "), |m| m.starts_with("There are "), Duration::from_secs(4));
        let Ok(lines) = lines else { return };
        let mut pos: HashMap<String, [f64; 3]> = HashMap::new();
        let mut dims: HashMap<String, String> = HashMap::new();
        for l in &lines {
            let Some((who, data)) = l.split_once(" has the following entity data: ") else { continue };
            // A team prefix can come before the name.
            let Some(name) = names.iter().find(|n| who == n.as_str() || who.ends_with(&format!(" {n}"))) else { continue };
            match nbt::snbt(data) {
                Ok(nbt::Tag::List(v)) => {
                    let n: Vec<f64> = v.iter().filter_map(nbt::Tag::num).collect();
                    if let [x, y, z] = n[..] {
                        pos.insert(name.clone(), [x, y, z]);
                    }
                }
                Ok(nbt::Tag::Str(d)) => {
                    dims.insert(name.clone(), d);
                }
                _ => {}
            }
        }
        let changed = self.with_live(id, |l| {
            let before = l.online.clone();
            for o in l.online.iter_mut() {
                if let Some(p) = pos.get(&o.name) {
                    o.pos = Some(*p);
                }
                if let Some(d) = dims.get(&o.name) {
                    o.dimension = Some(d.clone());
                }
            }
            before != l.online
        });
        if changed {
            let online = self.live.lock().get(id).map(|l| l.online.clone()).unwrap_or_default();
            self.events.emit("mcserver:players", json!({ "id": id, "online": online }));
        }
    }

    // ---------- the map ----------

    pub fn map_info(&self, id: &str, dim: &str) -> Result<MapInfo, String> {
        let s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        let level = level_of(&dir);
        let d = map::Dim::from_id(dim).ok_or("No such dimension.")?;
        let supported = crate::mods::minecraft::version_cmp(&s.mc, "1.18") != std::cmp::Ordering::Less;
        let regions = map::region_dir(&dir, &level, d).map(|r| map::regions(&r)).unwrap_or_default();
        let players = self.watch(id).into_iter().filter(|o| o.dimension.as_deref().and_then(map::Dim::from_id).unwrap_or(map::Dim::Overworld) == d).collect();
        Ok(MapInfo { regions, spawn: if d == map::Dim::Overworld { map::spawn(&dir, &level) } else { None }, players, supported })
    }

    /// One region's picture (PNG, as a data URL).
    pub fn map_tile(&self, id: &str, dim: &str, x: i32, z: i32) -> Result<String, String> {
        let s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        let level = level_of(&dir);
        let d = map::Dim::from_id(dim).ok_or("No such dimension.")?;
        let rd = map::region_dir(&dir, &level, d).ok_or("That part of the world hasn't been explored.")?;
        let png = map::tile(&rd, &dir.join(".omnihub").join("map"), x, z, d)?;
        Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)))
    }

    // ---------- content ----------

    pub fn content(&self, id: &str) -> Result<Vec<Content>, String> {
        let s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        Ok(content::list(&dir, s.flavor, &level_of(&dir)))
    }

    /// Look up unknown jars on Modrinth (in the background of the Content tab).
    pub fn identify_content(&self, id: &str) -> Result<Vec<Content>, String> {
        let s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        content::identify(&dir, s.flavor, &level_of(&dir), &self.mods.minecraft.api)?;
        self.content(id)
    }

    pub fn search(&self, id: &str, kind: Kind, query: &str, offset: usize) -> Result<(Vec<Hit>, u64), String> {
        let s = self.server(id)?;
        if !content::allowed(s.flavor, kind) {
            return Ok((vec![], 0));
        }
        let mut facets = vec![vec![format!("versions:{}", s.mc)]];
        match kind {
            Kind::Mod => {
                facets.push(s.flavor.loaders().iter().map(|l| format!("categories:{l}")).collect());
                facets.push(vec!["server_side:required".into(), "server_side:optional".into()]);
            }
            Kind::Plugin => facets.push(s.flavor.loaders().iter().map(|l| format!("categories:{l}")).collect()),
            Kind::Datapack => {}
        }
        let (hits, total) = self.mods.minecraft.api.search_projects(kind.project_type(), query, &facets, offset)?;
        let have: Vec<String> = self.content(id)?.into_iter().filter_map(|c| c.project).collect();
        Ok((hits.into_iter().map(|h| Hit { installed: have.contains(&h.project), hit: h }).collect(), total))
    }

    fn content_changed(&self, id: &str, kind: Kind) {
        if let Some(p) = self.proc_of(id) {
            if kind.live() {
                let _ = p.send("reload");
            } else {
                self.with_live(id, |l| l.restart_needed = true);
            }
        }
        self.events.emit("mcserver:content", json!({ "id": id }));
        self.changed(id);
    }

    pub fn install_content(&self, id: &str, kind: Kind, project: &str) -> Result<Vec<Content>, String> {
        let s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        let got = content::install(&dir, s.flavor, &s.mc, &level_of(&dir), &self.mods.minecraft.api, kind, project, &mut |m| self.set_work(id, m, 0, 0))?;
        self.with_live(id, |l| l.work = None);
        self.content_changed(id, kind);
        Ok(got)
    }

    pub fn add_content_file(&self, id: &str, path: &str) -> Result<Content, String> {
        let s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        let c = content::add_file(&dir, s.flavor, &level_of(&dir), Path::new(path))?;
        self.content_changed(id, c.kind);
        Ok(c)
    }

    pub fn set_content(&self, id: &str, file: &str, on: bool) -> Result<(), String> {
        let s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        let kind = content::list(&dir, s.flavor, &level_of(&dir)).into_iter().find(|c| c.file == file).map(|c| c.kind).ok_or("That's gone.")?;
        if kind != Kind::Datapack && self.proc_of(id).is_some() {
            // Running servers keep their jars open on Windows.
            return Err("Stop the server to switch mods and plugins on or off.".into());
        }
        content::set_enabled(&dir, file, on)?;
        self.content_changed(id, kind);
        Ok(())
    }

    pub fn remove_content(&self, id: &str, file: &str) -> Result<(), String> {
        let s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        let kind = content::list(&dir, s.flavor, &level_of(&dir)).into_iter().find(|c| c.file == file).map(|c| c.kind).ok_or("That's gone.")?;
        if kind != Kind::Datapack && self.proc_of(id).is_some() {
            return Err("Stop the server to remove mods and plugins.".into());
        }
        content::remove(&dir, file)?;
        self.content_changed(id, kind);
        Ok(())
    }

    /// Put something Claude made on a server (a data pack folder or a plugin jar).
    pub fn add_made(&self, id: &str, kind: Kind, src: &Path, name: &str, project: &str) -> Result<Content, String> {
        let s = self.server(id)?;
        if !content::allowed(s.flavor, kind) {
            return Err(format!("A {} server can't run that.", s.flavor.label()));
        }
        let dir = PathBuf::from(&s.dir);
        if kind == Kind::Plugin && self.proc_of(id).is_some() {
            // The jar can be replaced only while Paper isn't using it.
            let target = dir.join("plugins").join(format!("omnihub-{}.jar", crate::mods::slug(name).to_lowercase()));
            if target.exists() {
                return Err("Stop the server to update the plugin (Paper keeps it open while running).".into());
            }
        }
        let c = content::add_made(&dir, &level_of(&dir), kind, src, name, project)?;
        self.content_changed(id, kind);
        Ok(c)
    }

    // ---------- backups ----------

    pub fn backups(&self, id: &str) -> Result<Vec<backup::Backup>, String> {
        Ok(backup::list(Path::new(&self.server(id)?.dir)))
    }

    /// Back up now; a running server saves first and pauses saving while it's copied.
    pub fn backup_now(&self, id: &str, auto: bool) -> Result<backup::Backup, String> {
        let s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        self.with_live(id, |l| {
            if l.busy.is_some() {
                return Err("A backup is already being made.".to_string());
            }
            l.busy = Some("a backup is being made.".into());
            Ok(())
        })?;
        self.set_work(id, "Backing up the world", 0, 0);
        let p = self.running(id).ok();
        if let Some(p) = &p {
            let _ = p.ask(&["save-off".into(), "save-all flush".into()], |m| m.starts_with("Saved the game") || m.contains("Automatic saving is now disabled"), |m| m.starts_with("Saved the game"), Duration::from_secs(120));
        }
        let r = backup::make(&dir, &level_of(&dir), auto);
        if let Some(p) = &p {
            let _ = p.send("save-on");
        }
        if r.is_ok() {
            backup::prune(&dir, s.backup_keep as usize);
            let _ = self.edit(id, |x| x.last_backup = now());
        }
        self.with_live(id, |l| {
            l.busy = None;
            l.work = None;
        });
        match &r {
            Ok(b) => self.note(id, &format!("Backed up the world ({}).", crate::storage::format_bytes(b.size)), "info"),
            Err(e) => self.note(id, e, "error"),
        }
        self.changed(id);
        r
    }

    pub fn restore(&self, id: &str, name: &str) -> Result<String, String> {
        let s = self.server(id)?;
        if self.proc_of(id).is_some() {
            return Err("Stop the server before putting a backup back.".into());
        }
        let dir = PathBuf::from(&s.dir);
        let kept = backup::restore(&dir, &level_of(&dir), name)?;
        self.note(id, &format!("Put back {name}. The world it replaced is kept in the folder {kept}."), "info");
        self.changed(id);
        Ok(kept)
    }

    pub fn delete_backup(&self, id: &str, name: &str) -> Result<(), String> {
        backup::delete(Path::new(&self.server(id)?.dir), name)
    }

    // ---------- the way in ----------

    fn gateway(&self, fresh: bool) -> Result<net::Gateway, String> {
        if !fresh {
            if let Some((at, g)) = self.gateway.lock().as_ref() {
                if at.elapsed() < Duration::from_secs(600) {
                    return g.clone();
                }
            }
        }
        let g = net::discover(&self.agent, &self.ep.ssdp, Duration::from_secs(3));
        *self.gateway.lock() = Some((Instant::now(), g.clone()));
        g
    }

    fn port_of(&self, id: &str) -> u16 {
        self.server(id).map(|s| Props::load(&Path::new(&s.dir).join("server.properties")).get_or("server-port", "25565").parse().unwrap_or(25565)).unwrap_or(25565)
    }

    fn open_port_if_wanted(&self, id: &str) {
        if !self.server(id).map(|s| s.upnp).unwrap_or(false) {
            return;
        }
        let port = self.port_of(id);
        let r = self.gateway(false).and_then(|g| g.open(&self.agent, port, "OmniHub Minecraft"));
        self.with_live(id, |l| {
            l.port_open = r.is_ok();
            l.port_renewed = Some(Instant::now());
            l.upnp_error = r.as_ref().err().cloned();
        });
        match r {
            Ok(()) => self.note(id, &format!("Opened port {port} on the router for friends outside your network."), "info"),
            Err(e) => self.note(id, &format!("Couldn't open the port on the router: {e}"), "warn"),
        }
    }

    fn close_port(&self, id: &str) {
        let was = self.with_live(id, |l| std::mem::replace(&mut l.port_open, false));
        if !was {
            return;
        }
        let port = self.port_of(id);
        if let Ok(g) = self.gateway(false) {
            let _ = g.close(&self.agent, port);
        }
    }

    /// How friends get in: addresses, the router and the firewall.
    pub fn network(&self, id: &str) -> Result<NetView, String> {
        let s = self.server(id)?;
        let port = self.port_of(id);
        let lan = net::lan_ips().into_iter().map(|a| format!("{a}:{port}")).collect();
        let public_ip = {
            let cached = self.public_ip.lock().as_ref().filter(|(at, _)| at.elapsed() < Duration::from_secs(600)).map(|(_, ip)| ip.clone());
            match cached {
                Some(ip) => ip,
                None => {
                    let ip = net::public_ip(&self.agent, &self.ep.ipify).ok().map(|a| a.to_string());
                    *self.public_ip.lock() = Some((Instant::now(), ip.clone()));
                    ip
                }
            }
        };
        let g = self.gateway(false);
        let router_ip = g.as_ref().ok().and_then(|g| g.external_ip(&self.agent).ok());
        let cgnat = router_ip.is_some_and(net::not_public) || (router_ip.is_some() && public_ip.is_some() && router_ip.map(|r| r.to_string()) != public_ip);
        let (port_open, upnp_error) = self.live.lock().get(id).map(|l| (l.port_open, l.upnp_error.clone())).unwrap_or((false, None));
        let java = self.java_exe(id);
        let firewall = java.as_ref().map(|j| crate::system::firewall::check(j, Some(port), crate::system::firewall::Proto::Tcp, "the Minecraft server"));
        Ok(NetView {
            port,
            lan,
            public_ip,
            router: g.as_ref().ok().map(|g| g.name.clone()),
            router_ip: router_ip.map(|r| r.to_string()),
            upnp: s.upnp,
            port_open,
            upnp_error: upnp_error.or_else(|| g.err()),
            cgnat,
            java: java.map(|j| j.to_string_lossy().into_owned()),
            firewall,
        })
    }

    /// Turn the router port on or off now (and remember it).
    pub fn set_upnp(&self, id: &str, on: bool) -> Result<NetView, String> {
        self.edit(id, |s| s.upnp = on)?;
        if on {
            let _ = self.gateway(true);
            if self.proc_of(id).is_some() {
                self.open_port_if_wanted(id);
            }
        } else {
            self.close_port(id);
        }
        self.network(id)
    }

    // ---------- around the clock ----------

    /// After OmniHub starts: end servers left behind by a crash, start the
    /// ones set to start with OmniHub, and keep everything going.
    pub fn start_background(self: &Arc<Self>) {
        let leftovers: HashMap<String, u32> = std::fs::read(self.running_file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        if !leftovers.is_empty() {
            let mut sys = sysinfo::System::new();
            sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            for pid in leftovers.values() {
                if let Some(p) = sys.process(sysinfo::Pid::from_u32(*pid)) {
                    if p.name().to_string_lossy().to_lowercase().contains("java") {
                        tracing::warn!("ending a Minecraft server left running by an earlier OmniHub (pid {pid})");
                        p.kill();
                    }
                }
            }
            let _ = std::fs::remove_file(self.running_file());
        }
        // Servers set to start with OmniHub, and ones that ran before an update restarted it.
        let resume: Vec<String> = std::fs::read(self.dir.join("resume.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let _ = std::fs::remove_file(self.dir.join("resume.json"));
        let ids: Vec<String> = self.servers.lock().iter().filter(|s| (s.auto_start || resume.contains(&s.id)) && s.launch.is_some()).map(|s| s.id.clone()).collect();
        for id in ids {
            if let Err(e) = self.start(&id) {
                tracing::warn!("couldn't start Minecraft server {id}: {e}");
            }
        }
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("mc-servers".into())
            .spawn(move || {
                let mut n: u64 = 0;
                loop {
                    std::thread::sleep(Duration::from_millis(250));
                    let Some(h) = weak.upgrade() else { return };
                    if h.quitting.load(Ordering::SeqCst) {
                        return;
                    }
                    h.tick(n);
                    n += 1;
                }
            })
            .ok();
    }

    fn tick(self: &Arc<Self>, n: u64) {
        // Console news, four times a second at most.
        let dirty: Vec<String> = self.live.lock().iter_mut().filter_map(|(id, l)| std::mem::replace(&mut l.console_dirty, false).then(|| id.clone())).collect();
        for id in dirty {
            self.events.emit("mcserver:console", json!({ "id": id }));
        }
        if !n.is_multiple_of(4) {
            return;
        }
        let now_i = Instant::now();
        let servers = self.servers.lock().clone();
        let mut awake = false;
        for s in &servers {
            let (state, restart_at, watched, polling, renewed, has_proc) = {
                let live = self.live.lock();
                match live.get(&s.id) {
                    Some(l) => (l.state, l.restart_at, l.watch_until.is_some_and(|w| w > now_i), l.polling, l.port_renewed, l.proc.is_some()),
                    None => (State::Stopped, None, false, false, None, false),
                }
            };
            if has_proc && s.keep_awake {
                awake = true;
            }
            // Restart after a crash.
            if state == State::Crashed && restart_at.is_some_and(|t| t <= now_i) {
                self.with_live(&s.id, |l| l.restart_at = None);
                if let Err(e) = self.start(&s.id) {
                    self.note(&s.id, &e, "error");
                }
                continue;
            }
            if state != State::Running {
                continue;
            }
            // Where players are, every 2 seconds while someone looks.
            if watched && !polling && n.is_multiple_of(8) {
                self.with_live(&s.id, |l| l.polling = true);
                let me = self.clone();
                let id = s.id.clone();
                std::thread::spawn(move || {
                    me.poll_positions(&id);
                    me.with_live(&id, |l| l.polling = false);
                });
            }
            // Who's online, once a minute (in case a line was missed), and operators made in the game.
            if n.is_multiple_of(240) {
                if let Some(p) = self.proc_of(&s.id) {
                    std::thread::spawn(move || quiet_list(&p));
                    self.absorb_ops(&s.id, None);
                }
            }
            // Scheduled backups, while it runs (a stopped world doesn't change).
            if n.is_multiple_of(240) && has_proc && s.backup_hours > 0 && now() - s.last_backup >= s.backup_hours as i64 * 3600 && self.live.lock().get(&s.id).is_some_and(|l| l.busy.is_none()) {
                let me = self.clone();
                let id = s.id.clone();
                std::thread::spawn(move || {
                    let _ = me.backup_now(&id, true);
                });
            }
            // The router port, renewed twice a day.
            if s.upnp && renewed.is_none_or(|r| now_i.duration_since(r) > Duration::from_secs(12 * 3600)) && n.is_multiple_of(240) {
                let me = self.clone();
                let id = s.id.clone();
                std::thread::spawn(move || me.open_port_if_wanted(&id));
            }
        }
        if awake != self.awake.swap(awake, Ordering::SeqCst) {
            crate::system::awake::keep_system_on(awake);
        }
    }

    /// OmniHub is quitting: save and stop every server (waits up to `wait`).
    pub fn shutdown(&self, wait: Duration) {
        self.quitting.store(true, Ordering::SeqCst);
        let procs: Vec<(String, Arc<Proc>)> = self.live.lock().iter().filter_map(|(id, l)| l.proc.clone().map(|p| (id.clone(), p))).collect();
        let handles: Vec<_> = procs
            .into_iter()
            .map(|(id, p)| {
                std::thread::spawn(move || {
                    if !p.stop(wait) {
                        tracing::warn!("Minecraft server {id} didn't stop in time; ending it");
                        p.kill();
                    }
                })
            })
            .collect();
        for h in handles {
            let _ = h.join();
        }
        let ids: Vec<String> = self.live.lock().keys().cloned().collect();
        for id in ids {
            self.close_port(&id);
        }
        if self.awake.swap(false, Ordering::SeqCst) {
            crate::system::awake::keep_system_on(false);
        }
    }

    /// Before OmniHub restarts for an update: the servers running now start again after it.
    pub fn remember_running(&self) {
        let ids: Vec<String> = self.live.lock().iter().filter(|(_, l)| l.proc.is_some()).map(|(id, _)| id.clone()).collect();
        let _ = std::fs::create_dir_all(&self.dir);
        let _ = std::fs::write(self.dir.join("resume.json"), serde_json::to_vec(&ids).unwrap_or_default());
    }

    /// Servers running now.
    pub fn running_count(&self) -> usize {
        self.live.lock().values().filter(|l| l.proc.is_some()).count()
    }

    /// What Claude needs to know to make something for a server.
    pub fn maker_info(&self, id: &str) -> Result<Value, String> {
        let s = self.server(id)?;
        let dir = PathBuf::from(&s.dir);
        Ok(json!({ "name": s.name, "dir": s.dir, "flavor": s.flavor.label(), "plugins": s.flavor.plugins(), "mc": s.mc, "level": level_of(&dir), "packFormat": pack_format(&dir), "java": s.java_major }))
    }

    /// The Java compiler (a JDK of the server's Java version, fetched once) and
    /// the jars a plugin compiles against (Paper's own, after it ran once).
    pub fn javac(self: &Arc<Self>, id: &str) -> Result<(PathBuf, Vec<PathBuf>), String> {
        let s = self.server(id)?;
        if s.flavor != Flavor::Paper {
            return Err("Plugins are for Paper servers.".into());
        }
        let dir = PathBuf::from(&s.dir);
        let libs: Vec<PathBuf> = walkdir::WalkDir::new(dir.join("libraries")).into_iter().flatten().filter(|e| e.file_type().is_file() && e.file_name().to_string_lossy().ends_with(".jar")).map(|e| e.into_path()).collect();
        if !libs.iter().any(|p| p.to_string_lossy().contains("paper-api")) {
            return Err("Start the server once first: Paper unpacks the plugin API (paper-api) when it first runs.".into());
        }
        let javac = match self.runtimes.javac(s.java_major) {
            Some(j) => j,
            None => {
                let me = self.clone();
                let tid = id.to_string();
                let mut progress = move |done: u64, total: u64, text: &str| me.set_work(&tid, text, done, total);
                self.runtimes.ensure(&self.web, s.java_major, true, &mut progress)?;
                self.with_live(id, |l| l.work = None);
                self.runtimes.javac(s.java_major).ok_or("The Java compiler wasn't in the JDK download.")?
            }
        };
        Ok((javac, libs))
    }
}

/// A console line that could be a command's answer (not chat, joins or the server's own news).
fn reply_like(m: &str) -> bool {
    !(m.starts_with('<') || m.starts_with("[Not Secure]") || m.contains(" logged in with entity id ") || m.ends_with(" joined the game") || m.ends_with(" left the game") || m.contains(" lost connection: ") || m.starts_with("Saving") || m.starts_with("ThreadedAnvilChunkStorage") || m.starts_with("[ChunkHolderManager]") || m.contains("has made the advancement") || m.contains("has completed the challenge") || m.contains("has reached the goal") || m.starts_with("Can't keep up!"))
}

/// A .mrpack's Minecraft version, loader and loader version.
fn pack_target(index: &Value) -> (String, String, String) {
    let deps = &index["dependencies"];
    let mc = deps["minecraft"].as_str().unwrap_or_default().to_string();
    for (key, name) in [("fabric-loader", "fabric"), ("quilt-loader", "quilt"), ("neoforge", "neoforge"), ("forge", "forge")] {
        if let Some(v) = deps[key].as_str() {
            return (mc, name.into(), v.into());
        }
    }
    (mc, "vanilla".into(), String::new())
}

/// Read `version.json` from inside a jar.
fn jar_version(jar: &Path) -> Option<Value> {
    let f = std::fs::File::open(jar).ok()?;
    let mut z = zip::ZipArchive::new(f).ok()?;
    let mut s = String::new();
    std::io::Read::read_to_string(&mut std::io::Read::take(z.by_name("version.json").ok()?, 1 << 20), &mut s).ok()?;
    serde_json::from_str(&s).ok()
}

/// The jars of a server that carry Mojang's version.json.
fn version_jars(dir: &Path) -> Vec<PathBuf> {
    let mut v = vec![dir.join("server.jar")];
    for sub in ["versions", ".fabric/server", "libraries/net/minecraft/server"] {
        v.extend(walkdir::WalkDir::new(dir.join(sub)).max_depth(3).into_iter().flatten().filter(|e| e.file_type().is_file() && e.file_name().to_string_lossy().ends_with(".jar")).map(|e| e.into_path()));
    }
    v
}

/// The data pack format this server's Minecraft uses ("121.0" or "48").
pub fn pack_format(dir: &Path) -> Option<String> {
    for j in version_jars(dir) {
        let Some(v) = jar_version(&j) else { continue };
        let pv = &v["pack_version"];
        if let Some(n) = pv.as_u64() {
            return Some(n.to_string());
        }
        if let Some(maj) = pv["data_major"].as_u64() {
            return Some(format!("{maj}.{}", pv["data_minor"].as_u64().unwrap_or(0)));
        }
        if let Some(d) = pv["data"].as_u64() {
            return Some(d.to_string());
        }
    }
    None
}

/// Work out an existing server's type, version and how it starts.
/// Ask who's online without filling the console (the answer still updates the list).
fn quiet_list(p: &Proc) {
    let _ = p.ask(&["list uuids".into()], |m| m.starts_with("There are "), |m| m.starts_with("There are "), Duration::from_secs(3));
}

pub fn detect(dir: &Path) -> Option<(Flavor, String, String, Launch)> {
    let names: Vec<String> = std::fs::read_dir(dir).ok()?.flatten().filter_map(|e| e.file_name().to_str().map(String::from)).collect();
    let mc_from_jars = || version_jars(dir).iter().find_map(|j| jar_version(j)).and_then(|v| v["id"].as_str().or(v["name"].as_str()).map(String::from));
    // NeoForge and Forge (1.17+): an arguments file under libraries/.
    for (flavor, sub) in [(Flavor::NeoForge, "libraries/net/neoforged/neoforge"), (Flavor::Forge, "libraries/net/minecraftforge/forge")] {
        let mut vs: Vec<String> = std::fs::read_dir(dir.join(sub)).into_iter().flatten().flatten().filter_map(|e| e.file_name().to_str().map(String::from)).collect();
        vs.sort_by(|a, b| crate::mods::minecraft::version_cmp(b, a));
        if let Some(v) = vs.first() {
            let (mc, loader) = if flavor == Flavor::Forge { v.split_once('-').map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or_default() } else { (install::neo_mc(v).unwrap_or_default(), v.clone()) };
            if let Some(l) = install::forge_launch(dir, flavor, &mc, &loader) {
                return Some((flavor, mc, loader, l));
            }
        }
    }
    if names.iter().any(|n| n == "fabric-server-launch.jar") {
        let mc = std::fs::read_dir(dir.join(".fabric/server")).into_iter().flatten().flatten().filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix("-server.jar")).map(String::from)).next().or_else(mc_from_jars).unwrap_or_default();
        return Some((Flavor::Fabric, mc, String::new(), Launch::Jar { jar: "fabric-server-launch.jar".into() }));
    }
    if names.iter().any(|n| n == "quilt-server-launch.jar") {
        return Some((Flavor::Quilt, mc_from_jars().unwrap_or_default(), String::new(), Launch::Jar { jar: "quilt-server-launch.jar".into() }));
    }
    if let Some(p) = names.iter().find(|n| (n.starts_with("paper") || n.starts_with("purpur")) && n.ends_with(".jar")) {
        // paper-1.21.4-232.jar, or versions/<mc>/ after the first run.
        let mc = p.trim_end_matches(".jar").split('-').nth(1).filter(|v| v.contains('.')).map(String::from).or_else(|| std::fs::read_dir(dir.join("versions")).into_iter().flatten().flatten().filter_map(|e| e.file_name().to_str().map(String::from)).next()).unwrap_or_default();
        return Some((Flavor::Paper, mc, String::new(), Launch::Jar { jar: p.clone() }));
    }
    let jar = if names.iter().any(|n| n == "server.jar") { Some("server.jar".to_string()) } else { names.iter().find(|n| n.starts_with("minecraft_server") && n.ends_with(".jar")).cloned() };
    let jar = jar?;
    let mc = jar_version(&dir.join(&jar)).and_then(|v| v["id"].as_str().map(String::from)).or_else(|| jar.strip_prefix("minecraft_server.").and_then(|r| r.strip_suffix(".jar")).map(String::from)).unwrap_or_default();
    Some((Flavor::Vanilla, mc, String::new(), Launch::Jar { jar }))
}

#[cfg(test)]
mod tests;
