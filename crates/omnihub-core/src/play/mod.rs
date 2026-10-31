//! Capri and your games: she reacts to them, hears "Capri, clip that", and
//! the last minutes of the screen are kept so a clip can be saved.
//!
//! Nothing here runs until one of its switches is on (Games → Clips &
//! Capri), and the heavier parts only while a game runs: a light check every
//! two seconds notices a game starting (any game OmniHub knows: your
//! profiles, the games it found installed, the popular ones). Then:
//!
//! * **Instant replay** ([`clips`]) records on the graphics card into a
//!   rolling buffer; "clip that" joins its newest seconds into an MP4.
//! * **Voice** ([`voice`]) listens with Windows' offline recognizer for the
//!   sentences starting with "Capri".
//! * **Capri** says things through her local bridge ([`crate::companion::say`]):
//!   the game starting and ending, clips saved, and, where the game tells
//!   OmniHub what happens, the match itself: Counter-Strike 2 ([`cs2`], its
//!   official game-state integration) and League of Legends ([`lol`], Riot's
//!   local live-data service). Other games have no such channel.

pub mod clips;
pub mod cs2;
mod lines;
pub mod lol;
pub mod voice;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::core::AppCore;
use crate::events::EventBus;
use crate::settings::PlaySettings;
use voice::Command;

/// Popular games OmniHub notices even without a profile: (program, name).
const KNOWN: &[(&str, &str)] = &[
    ("FortniteClient-Win64-Shipping.exe", "Fortnite"),
    ("RobloxPlayerBeta.exe", "Roblox"),
    ("VALORANT-Win64-Shipping.exe", "VALORANT"),
    ("cs2.exe", "Counter-Strike 2"),
    (lol::PROCESS, "League of Legends"),
    ("r5apex.exe", "Apex Legends"),
    ("r5apex_dx12.exe", "Apex Legends"),
    ("RocketLeague.exe", "Rocket League"),
    ("GTA5.exe", "GTA V"),
    ("GTA5_Enhanced.exe", "GTA V"),
    ("Overwatch.exe", "Overwatch 2"),
    ("cod.exe", "Call of Duty"),
    ("Minecraft.Windows.exe", "Minecraft"),
    ("RainbowSix.exe", "Rainbow Six Siege"),
    ("eldenring.exe", "Elden Ring"),
    ("Marvel-Win64-Shipping.exe", "Marvel Rivals"),
    ("dota2.exe", "Dota 2"),
];

/// Reactions this close together are skipped (answers to you never are).
const REACT_GAP: Duration = Duration::from_secs(6);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PlayStatus {
    pub supported: bool,
    /// The game being played.
    pub game: Option<String>,
    pub since: Option<i64>,
    pub recorder_installed: bool,
    pub installing: bool,
    pub replaying: bool,
    /// Seconds kept so far.
    pub buffered: u32,
    pub recording: bool,
    pub encoder: Option<String>,
    pub replay_error: Option<String>,
    pub listening: bool,
    pub voice_error: Option<String>,
    pub capri_running: bool,
    pub folder: String,
    /// The last thing heard, for the page.
    pub heard: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub created: i64,
}

struct Playing {
    name: String,
    process: String,
    since: Instant,
    started_at: i64,
    clips: u32,
}

/// What reaches the worker that acts (so listeners return at once).
enum Msg {
    #[cfg_attr(not(windows), allow(dead_code))]
    Voice(Command),
    Cs2(cs2::Event),
    Lol(lol::Event),
    /// Look at the automatic clip's deadline again.
    Wake,
}

#[derive(Default)]
struct State {
    game: Option<Playing>,
    recorder: Option<Arc<clips::Recorder>>,
    recorder_opts: Option<(clips::Options, u32)>,
    replay_error: Option<String>,
    listener: Option<Box<dyn std::any::Any + Send>>,
    listener_phrases: Vec<String>,
    voice_error: Option<String>,
    cs2: Option<cs2::Receiver>,
    lol: Option<lol::Watcher>,
    encoder: Option<clips::Encoder>,
    /// No encoder worked (tried again with the next game).
    detect_failed: bool,
    last_react: Option<Instant>,
    heard: Option<String>,
    fps: VecDeque<f32>,
    fps_low: u32,
    fps_warned: Option<Instant>,
    /// An automatic clip waiting for the moment to finish: (when, seconds, what).
    auto_clip: Option<(Instant, u32)>,
    recording_since: Option<Instant>,
    watch: Vec<(String, String)>,
    watch_at: Option<Instant>,
}

pub struct PlayHub {
    tools: PathBuf,
    buffer_dir: PathBuf,
    events: EventBus,
    state: Mutex<State>,
    installing: AtomicBool,
    apps_loading: AtomicBool,
    tx: Mutex<Option<mpsc::Sender<Msg>>>,
}

impl PlayHub {
    pub fn new(data: &std::path::Path, events: EventBus) -> Arc<Self> {
        Arc::new(PlayHub { tools: data.join("tools"), buffer_dir: data.join("replay"), events, state: Mutex::new(State::default()), installing: AtomicBool::new(false), apps_loading: AtomicBool::new(false), tx: Mutex::new(None) })
    }

    fn ffmpeg(&self) -> PathBuf {
        self.tools.join(if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" })
    }

    /// Where clips go.
    pub fn folder(s: &PlaySettings) -> PathBuf {
        s.folder.as_ref().filter(|f| !f.trim().is_empty()).map(PathBuf::from).unwrap_or_else(|| dirs::video_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Videos")).join("OmniHub Clips"))
    }

    pub fn status(&self, s: &PlaySettings) -> PlayStatus {
        let st = self.state.lock();
        PlayStatus {
            supported: cfg!(windows),
            game: st.game.as_ref().map(|g| g.name.clone()),
            since: st.game.as_ref().map(|g| g.started_at),
            recorder_installed: self.ffmpeg().is_file(),
            installing: self.installing.load(Ordering::Relaxed),
            replaying: st.recorder.is_some(),
            buffered: st.recorder.as_ref().map_or(0, |r| r.buffered()),
            recording: st.recorder.as_ref().is_some_and(|r| r.recording()),
            encoder: st.encoder.map(|e| e.label().to_string()),
            replay_error: st.replay_error.clone(),
            listening: st.listener.is_some(),
            voice_error: st.voice_error.clone(),
            capri_running: crate::companion::listening(),
            folder: Self::folder(s).display().to_string(),
            heard: st.heard.clone(),
        }
    }

    fn changed(&self) {
        self.events.emit("play:changed", serde_json::json!({}));
    }

    /// Download FFmpeg from this release and check its published checksum.
    pub fn install_recorder(&self) -> Result<(), String> {
        if self.installing.swap(true, Ordering::SeqCst) {
            return Err("The recorder is already downloading.".into());
        }
        self.changed();
        let r = (|| -> Result<(), String> {
            use crate::capture::airplay;
            let url = format!("{}/v{}/{}", airplay::releases_url(), env!("CARGO_PKG_VERSION"), clips::ASSET);
            std::fs::create_dir_all(&self.tools).map_err(|e| e.to_string())?;
            let part = self.tools.join("ffmpeg.exe.part");
            let hash = airplay::download(&url, &part, &mut |done, total| self.events.emit("play:install", serde_json::json!({ "done": done, "total": total }))).map_err(|e| format!("Couldn't download the recorder: {e}"))?;
            let expected = match clips::pinned_sha256() {
                Some(h) => h.to_lowercase(),
                None => {
                    let sums = airplay::fetch_text(&format!("{}/v{}/SHA256SUMS.txt", airplay::releases_url(), env!("CARGO_PKG_VERSION"))).map_err(|e| e.to_string())?;
                    airplay::checksum_for(&sums, clips::ASSET).ok_or("This release lists no checksum for the recorder.")?
                }
            };
            if hash != expected {
                let _ = std::fs::remove_file(&part);
                return Err("The download didn't match its published checksum, so it wasn't kept.".into());
            }
            std::fs::rename(&part, self.ffmpeg()).map_err(|e| e.to_string())
        })();
        self.installing.store(false, Ordering::SeqCst);
        self.changed();
        r
    }

    /// The clips saved so far, newest first.
    pub fn clips(&self, s: &PlaySettings) -> Vec<Clip> {
        let mut v: Vec<Clip> = std::fs::read_dir(Self::folder(s))
            .map(|rd| {
                rd.flatten()
                    .filter_map(|e| {
                        let p = e.path();
                        if p.extension().is_none_or(|x| !x.eq_ignore_ascii_case("mp4")) {
                            return None;
                        }
                        let m = e.metadata().ok()?;
                        let created = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64;
                        Some(Clip { path: p.display().to_string(), name: p.file_stem()?.to_string_lossy().into_owned(), size: m.len(), created })
                    })
                    .collect()
            })
            .unwrap_or_default();
        v.sort_by_key(|c| std::cmp::Reverse(c.created));
        v
    }

    /// Move a clip to the Recycle Bin (only a file in the clips folder).
    pub fn delete_clip(&self, s: &PlaySettings, path: &std::path::Path) -> Result<(), String> {
        let folder = crate::paths::canonical(&Self::folder(s)).map_err(|e| e.to_string())?;
        let inside = path.parent().and_then(|d| crate::paths::canonical(d).ok()).is_some_and(|d| d == folder);
        if !inside || !path.is_file() {
            return Err("That file isn't in the clips folder.".into());
        }
        trash::delete(path).map_err(|e| e.to_string())
    }

    /// Start the light game check and the worker (once, with OmniHub).
    pub fn start(self: &Arc<Self>, core: &Arc<AppCore>) {
        let (tx, rx) = mpsc::channel::<Msg>();
        *self.tx.lock() = Some(tx);
        let weak = Arc::downgrade(core);
        let hub = self.clone();
        let _ = std::thread::Builder::new().name("play-watch".into()).spawn({
            let weak = weak.clone();
            let hub = hub.clone();
            move || loop {
                std::thread::sleep(Duration::from_secs(2));
                let Some(core) = weak.upgrade() else { return };
                hub.tick(&core);
            }
        });
        let _ = std::thread::Builder::new().name("play-act".into()).spawn(move || loop {
            let due = hub.state.lock().auto_clip.map(|(at, _)| at);
            let wait = due.map_or(Duration::from_secs(3600), |d| d.saturating_duration_since(Instant::now()));
            let msg = rx.recv_timeout(wait);
            let Some(core) = weak.upgrade() else { return };
            match msg {
                Ok(m) => hub.act(&core, m),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let pending = hub.state.lock().auto_clip.take_if(|(at, _)| *at <= Instant::now());
                    if let Some((_, secs)) = pending {
                        hub.save_clip(&core, secs, true);
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        });
    }

    fn send(&self, m: Msg) {
        if let Some(tx) = self.tx.lock().as_ref() {
            let _ = tx.send(m);
        }
    }

    /// The games to notice: (program, name).
    fn watch_list(&self, core: &AppCore) -> Vec<(String, String)> {
        let mut st = self.state.lock();
        if st.watch_at.is_none_or(|t| t.elapsed() > Duration::from_secs(300)) {
            let mut v: Vec<(String, String)> = core.games.profiles().into_iter().filter(|p| !p.process.is_empty()).map(|p| (p.process, p.name)).collect();
            drop(st);
            v.extend(core.games.library().into_iter().filter(|g| !g.process.is_empty()).map(|g| (g.process, g.name)));
            v.extend(KNOWN.iter().map(|(p, n)| (p.to_string(), n.to_string())));
            let mut seen = std::collections::HashSet::new();
            v.retain(|(p, _)| seen.insert(p.to_lowercase()));
            st = self.state.lock();
            st.watch = v;
            st.watch_at = Some(Instant::now());
        }
        st.watch.clone()
    }

    /// Every two seconds: notice games, and keep the replay and the listener as the settings want.
    fn tick(self: &Arc<Self>, core: &Arc<AppCore>) {
        let s = core.settings.get().play;
        let busy = self.state.lock().game.is_some();
        if !(s.capri_reacts || s.voice || s.replay) && !busy {
            return;
        }
        // The game being played (the one already followed while it still runs).
        let current = self.state.lock().game.as_ref().map(|g| g.process.clone());
        let running = match current.filter(|p| !core.procs.pids_named(p).is_empty()) {
            Some(p) => Some(p),
            None => self.watch_list(core).into_iter().find(|(p, _)| !core.procs.pids_named(p).is_empty()).map(|(p, n)| {
                self.game_started(core, &s, p.clone(), n);
                p
            }),
        };
        if running.is_none() && busy {
            self.game_ended(core, &s);
        }
        self.reconcile(core, &s);
        self.watch_game(core, &s);
    }

    fn game_started(self: &Arc<Self>, core: &Arc<AppCore>, s: &PlaySettings, process: String, name: String) {
        self.state.lock().game = Some(Playing { name: name.clone(), process: process.clone(), since: Instant::now(), started_at: crate::db::now(), clips: 0 });
        if process.eq_ignore_ascii_case("cs2.exe") {
            self.start_cs2(core);
        }
        if process.eq_ignore_ascii_case(lol::PROCESS) {
            self.state.lock().lol = Some(lol::Watcher::default());
        }
        let mut line = lines::pick(lines::GAME_START, &self.fill(s, &[]));
        if s.replay && self.ffmpeg().is_file() {
            line = format!("{line} {}", lines::pick(lines::GAME_START_REPLAY, &self.fill(s, &[("minutes", s.replay_minutes.clamp(1, 20).to_string())])));
        }
        self.react(s, &line);
        self.changed();
    }

    fn game_ended(&self, core: &AppCore, s: &PlaySettings) {
        // A recording still going is saved, not lost.
        let recording = self.state.lock().recorder.as_ref().is_some_and(|r| r.recording());
        if recording {
            self.stop_recording(core);
        }
        let mut st = self.state.lock();
        let game = st.game.take();
        st.cs2 = None;
        st.lol = None;
        st.fps.clear();
        st.auto_clip = None;
        drop(st);
        if let Some(g) = game {
            let mut fill = vec![("duration", lines::duration(g.since.elapsed().as_secs())), ("game", g.name.clone())];
            let line = if g.clips > 0 {
                fill.push(("clips", if g.clips == 1 { "1 clip".into() } else { format!("{} clips", g.clips) }));
                lines::pick(lines::GAME_END_CLIPS, &self.fill(s, &fill))
            } else {
                lines::pick(lines::GAME_END, &self.fill(s, &fill))
            };
            self.react(s, &line);
        }
        self.changed();
    }

    /// Start or stop the replay and the listener to match the settings.
    fn reconcile(self: &Arc<Self>, core: &Arc<AppCore>, s: &PlaySettings) {
        let playing = self.state.lock().game.is_some();
        // Instant replay: while a game runs (and while a recording goes).
        let keep = s.replay_minutes.clamp(1, 20) * 60;
        let recording = self.state.lock().recorder.as_ref().is_some_and(|r| r.recording());
        let want_replay = cfg!(windows) && s.replay && (playing || recording) && self.ffmpeg().is_file();
        let opts = |enc| clips::Options { video: clips::Video::Desktop { monitor: s.monitor }, fps: if s.fps >= 60 { 60 } else { 30 }, quality: s.quality, encoder: enc, audio: s.game_audio };
        if want_replay {
            let enc = self.state.lock().encoder;
            let enc = match enc {
                Some(e) => e,
                None if self.state.lock().detect_failed => return self.reconcile_voice(core, s, playing),
                None => match clips::detect(&self.ffmpeg(), s.monitor) {
                    Some(e) => {
                        self.state.lock().encoder = Some(e);
                        e
                    }
                    None => {
                        let mut st = self.state.lock();
                        st.detect_failed = true;
                        st.replay_error = Some("No video encoder worked with screen capture on this PC (graphics driver too old?).".into());
                        drop(st);
                        self.changed();
                        return self.reconcile_voice(core, s, playing);
                    }
                },
            };
            let wanted = opts(enc);
            let mut st = self.state.lock();
            if st.recorder_opts.as_ref().is_some_and(|(o, _)| *o == wanted) {
                if let Some(r) = st.recorder.clone() {
                    r.set_keep(keep);
                    if let Err(e) = r.alive() {
                        st.recorder = None;
                        st.replay_error = Some(format!("The recorder stopped: {e}"));
                        // After a good while (a screen mode change ends the capture): start again.
                        if r.age() > Duration::from_secs(30) {
                            st.recorder_opts = None;
                        }
                        drop(st);
                        self.changed();
                    }
                }
                // Failed to start with these options: wait for a change (or the next game).
            } else {
                st.recorder = None;
                let sound = if wanted.audio { clips::windows_sound() } else { None };
                match clips::Recorder::start(&self.ffmpeg(), &self.buffer_dir, &wanted, keep, sound) {
                    Ok(r) => {
                        st.recorder = Some(Arc::new(r));
                        st.replay_error = None;
                    }
                    Err(e) => st.replay_error = Some(e),
                }
                st.recorder_opts = Some((wanted, keep));
                drop(st);
                self.changed();
            }
        } else {
            let mut st = self.state.lock();
            st.recorder_opts = None;
            st.detect_failed = false;
            if st.recorder.take().is_some() {
                drop(st);
                self.changed();
            }
            if !s.replay {
                self.state.lock().replay_error = None;
            }
        }
        self.reconcile_voice(core, s, playing);
    }

    fn reconcile_voice(self: &Arc<Self>, _core: &Arc<AppCore>, s: &PlaySettings, playing: bool) {
        let want = cfg!(windows) && s.voice && (playing || !s.voice_only_in_games);
        let apps = self.voice_apps(_core, want);
        let phrases: Vec<String> = voice::phrases(&s.nicknames, &apps).into_iter().map(|(p, _)| p).collect();
        let mut st = self.state.lock();
        let running = st.listener.is_some();
        if want && (!running || st.listener_phrases != phrases) && (st.voice_error.is_none() || st.listener_phrases != phrases) {
            st.listener = None;
            st.listener_phrases = phrases.clone();
            drop(st);
            let r = self.start_listener(phrases, s.nicknames.clone(), apps);
            let mut st = self.state.lock();
            match r {
                Ok(l) => {
                    st.listener = Some(l);
                    st.voice_error = None;
                }
                Err(e) => st.voice_error = Some(e),
            }
            drop(st);
            self.changed();
        } else if !want && running {
            st.listener = None;
            drop(st);
            self.changed();
        } else if !s.voice {
            st.voice_error = None;
        }
    }

    /// Start menu programs for "Capri, open …": the cached list, read once
    /// in the background the first time (the listener restarts with them).
    fn voice_apps(self: &Arc<Self>, core: &Arc<AppCore>, want: bool) -> Vec<String> {
        if let Some(names) = core.apps.cached_launchable() {
            return names;
        }
        if want && !self.apps_loading.swap(true, Ordering::Relaxed) {
            let core = core.clone();
            let hub = Arc::downgrade(self);
            std::thread::spawn(move || {
                core.apps.list(&|_| None);
                if let Some(hub) = hub.upgrade() {
                    hub.send(Msg::Wake);
                }
            });
        }
        Vec::new()
    }

    #[cfg(windows)]
    fn start_listener(self: &Arc<Self>, phrases: Vec<String>, nicknames: Vec<String>, apps: Vec<String>) -> Result<Box<dyn std::any::Any + Send>, String> {
        let hub = Arc::downgrade(self);
        let l = voice::Listener::start(phrases, move |text| {
            let Some(hub) = hub.upgrade() else { return };
            hub.state.lock().heard = Some(text.clone());
            if let Some(cmd) = voice::parse(&text, &nicknames, &apps) {
                hub.send(Msg::Voice(cmd));
            }
            hub.events.emit("play:heard", serde_json::json!({ "text": text }));
        })?;
        Ok(Box::new(l))
    }

    #[cfg(not(windows))]
    fn start_listener(self: &Arc<Self>, _phrases: Vec<String>, _nicknames: Vec<String>, _apps: Vec<String>) -> Result<Box<dyn std::any::Any + Send>, String> {
        Err("Voice commands use Windows speech recognition.".into())
    }

    fn start_cs2(self: &Arc<Self>, core: &AppCore) {
        // Its config, written where cs2.exe is (read by CS2 when it starts).
        let token = cs2_token(&self.tools);
        let exe = core.procs.exe_of("cs2.exe").map(PathBuf::from).or_else(|| core.games.library().into_iter().find(|g| g.process.eq_ignore_ascii_case("cs2.exe")).and_then(|g| g.exe_path.map(PathBuf::from)));
        if let Some(dir) = exe.as_deref().and_then(cs2::cfg_dir) {
            if let Ok(true) = cs2::install(&dir, &token) {
                tracing::info!("CS2 game-state integration set up in {}", dir.display());
            }
        }
        let hub = Arc::downgrade(self);
        match cs2::Receiver::start(token, move |e| {
            if let Some(h) = hub.upgrade() {
                h.send(Msg::Cs2(e));
            }
        }) {
            Ok(r) => self.state.lock().cs2 = Some(r),
            Err(e) => tracing::warn!("CS2 events: {e}"),
        }
    }

    /// While a game runs: League's events, the FPS, a recorder that stopped.
    fn watch_game(&self, core: &AppCore, s: &PlaySettings) {
        // Asked without holding the state (a request can take a moment).
        let watcher = self.state.lock().lol.take();
        if let Some(mut w) = watcher {
            let events = w.poll();
            let mut st = self.state.lock();
            if st.game.as_ref().is_some_and(|g| g.process.eq_ignore_ascii_case(lol::PROCESS)) && st.lol.is_none() {
                st.lol = Some(w);
            }
            drop(st);
            for e in events {
                self.send(Msg::Lol(e));
            }
        }
        if !s.capri_reacts || self.state.lock().game.is_none() {
            return;
        }
        let Some(live) = core.games.fps_live() else { return };
        let mut st = self.state.lock();
        if st.fps.len() >= 30 {
            st.fps.pop_front();
        }
        st.fps.push_back(live.fps);
        let avg = st.fps.iter().sum::<f32>() / st.fps.len() as f32;
        // Two checks in a row well below the minute's average, and not too often.
        st.fps_low = if st.fps.len() >= 10 && avg > 45.0 && live.fps < avg * 0.55 { st.fps_low + 1 } else { 0 };
        if st.fps_low >= 2 && st.fps_warned.is_none_or(|t| t.elapsed() > Duration::from_secs(180)) {
            st.fps_warned = Some(Instant::now());
            st.fps_low = 0;
            drop(st);
            self.react(s, &lines::pick(lines::FPS_DROP, &self.fill(s, &[("fps", format!("{:.0}", live.fps))])));
        }
    }

    fn fill(&self, s: &PlaySettings, extra: &[(&'static str, String)]) -> Vec<(&'static str, String)> {
        let name = if s.nickname.trim().is_empty() { "champ".to_string() } else { s.nickname.trim().to_string() };
        let game = self.state.lock().game.as_ref().map(|g| g.name.clone()).unwrap_or_else(|| "the game".into());
        let mut v = vec![("name", name), ("game", game)];
        v.extend(extra.iter().cloned());
        v
    }

    fn title(&self) -> String {
        self.state.lock().game.as_ref().map(|g| g.name.clone()).unwrap_or_else(|| "OmniHub".into())
    }

    /// Capri reacting to the game (only with reactions on, and not too often).
    fn react(&self, s: &PlaySettings, text: &str) {
        if !s.capri_reacts {
            return;
        }
        {
            let mut st = self.state.lock();
            if st.last_react.is_some_and(|t| t.elapsed() < REACT_GAP) {
                return;
            }
            st.last_react = Some(Instant::now());
        }
        self.say(text);
    }

    /// Capri answering you (always, when she's running).
    fn say(&self, text: &str) {
        let title = self.title();
        let _ = crate::companion::say(&title, text);
        self.events.emit("play:said", serde_json::json!({ "text": text }));
    }

    /// Have Capri say hello (the page's "Test" button).
    pub fn hello(&self, core: &AppCore) -> Result<(), String> {
        let s = core.settings.get().play;
        crate::companion::say(&self.title(), &lines::pick(lines::HELLO, &self.fill(&s, &[])))
    }

    fn act(self: &Arc<Self>, core: &Arc<AppCore>, m: Msg) {
        let s = core.settings.get().play;
        let f = |extra: &[(&'static str, String)]| self.fill(&s, extra);
        match m {
            Msg::Voice(Command::Clip) => self.save_clip(core, s.clip_seconds.clamp(5, 300), false),
            Msg::Voice(Command::SaveLast(secs)) => self.save_clip(core, secs, false),
            Msg::Voice(Command::StartRecording) => self.start_recording(&s),
            Msg::Voice(Command::StopRecording) => self.stop_recording(core),
            Msg::Voice(Command::CallMe(name)) => {
                let _ = core.update_settings(&serde_json::json!({ "play": { "nickname": name } }));
                self.say(&lines::pick(lines::CALL_ME, &[("name", name)]));
            }
            Msg::Voice(Command::Fps) => match core.games.fps_live().filter(|_| self.state.lock().game.is_some()) {
                Some(l) => self.say(&lines::pick(lines::FPS, &f(&[("fps", format!("{:.0}", l.fps)), ("low", format!("{:.0}", l.low1))]))),
                None => self.say(&lines::pick(lines::NO_FPS, &f(&[]))),
            },
            Msg::Voice(Command::Screenshot) => {
                let shots = core.settings.get().screenshots;
                match core.screenshots.capture(crate::capture::screenshots::CaptureKind::Screen, &core.screenshot_dir(), shots.format) {
                    Ok(_) => self.say(&lines::pick(lines::SCREENSHOT, &f(&[]))),
                    Err(e) => self.say(&format!("The screenshot didn't work: {e}")),
                }
            }
            Msg::Voice(Command::Hello) => self.say(&lines::pick(lines::HELLO, &f(&[]))),
            Msg::Voice(Command::LockPc) => {
                self.say("Locking your PC.");
                core.power.schedule(crate::system::power::PowerAction::Lock, 0, "voice");
            }
            Msg::Voice(Command::Panic) => {
                core.panic("voice");
            }
            Msg::Voice(Command::OpenApp(name)) => match core.apps.find_spoken(&name) {
                Some(id) => match core.apps.launch(&id) {
                    Ok(app) => self.say(&format!("Opening {app}.")),
                    Err(e) => self.say(&format!("I couldn't open {name}: {e}")),
                },
                None => self.say(&format!("I can't find {name}.")),
            },
            Msg::Voice(Command::Music(what)) => {
                let action = match what.as_str() {
                    "play" => crate::media::Action::Play,
                    "pause" => crate::media::Action::Pause,
                    "next" => crate::media::Action::Next,
                    _ => crate::media::Action::Previous,
                };
                if let Err(e) = core.media.control(action, 0) {
                    self.say(&format!("The music didn't listen: {e}"));
                }
            }
            Msg::Voice(Command::Mute(on)) => {
                let _ = crate::media::mixer::set_master(core.media.is_fake(), None, Some(on));
            }
            Msg::Cs2(e) => self.on_cs2(&s, e),
            Msg::Lol(e) => self.on_lol(&s, e),
            Msg::Wake => {}
        }
    }

    fn on_cs2(&self, s: &PlaySettings, e: cs2::Event) {
        let f = |extra: &[(&'static str, String)]| self.fill(s, extra);
        match e {
            cs2::Event::Kill { round_kills } if round_kills >= 3 => {
                let set = match round_kills {
                    3 => lines::CS_TRIPLE,
                    4 => lines::CS_QUAD,
                    _ => lines::CS_ACE,
                };
                self.big_moment(s, set, 25);
            }
            cs2::Event::Kill { .. } => {}
            cs2::Event::RoundWon { ours, theirs } => self.react(s, &lines::pick(lines::CS_ROUND_WON, &f(&[("ours", ours.to_string()), ("theirs", theirs.to_string())]))),
            cs2::Event::RoundLost { ours, theirs } => self.react(s, &lines::pick(lines::CS_ROUND_LOST, &f(&[("ours", ours.to_string()), ("theirs", theirs.to_string())]))),
            cs2::Event::Mvp => self.react(s, &lines::pick(lines::CS_MVP, &f(&[]))),
            cs2::Event::BombPlanted => self.react(s, &lines::pick(lines::CS_BOMB, &f(&[]))),
            cs2::Event::MatchOver { won, ours, theirs } => {
                // The match result always gets through.
                self.state.lock().last_react = None;
                self.react(s, &lines::pick(if won { lines::CS_WON } else { lines::CS_LOST }, &f(&[("ours", ours.to_string()), ("theirs", theirs.to_string())])));
            }
        }
    }

    fn on_lol(&self, s: &PlaySettings, e: lol::Event) {
        let f = |extra: &[(&'static str, String)]| self.fill(s, extra);
        match e {
            lol::Event::Kill => self.react(s, &lines::pick(lines::LOL_KILL, &f(&[]))),
            lol::Event::Multikill(2) => self.react(s, &lines::pick(lines::LOL_DOUBLE, &f(&[]))),
            lol::Event::Multikill(n) => self.big_moment(s, if n >= 5 { lines::LOL_PENTA } else if n == 4 { lines::LOL_QUADRA } else { lines::LOL_TRIPLE }, 30),
            lol::Event::Died => self.react(s, &lines::pick(lines::LOL_DIED, &f(&[]))),
            lol::Event::Dragon { ours } => self.react(s, &lines::pick(if ours { lines::LOL_DRAGON } else { lines::LOL_DRAGON_LOST }, &f(&[]))),
            lol::Event::Baron { ours } => self.react(s, &lines::pick(if ours { lines::LOL_BARON } else { lines::LOL_BARON_LOST }, &f(&[]))),
            lol::Event::Ace { ours: true } => self.react(s, &lines::pick(lines::LOL_ACE, &f(&[]))),
            lol::Event::Ace { ours: false } => {}
            lol::Event::Result { won } => {
                self.state.lock().last_react = None;
                self.react(s, &lines::pick(if won { lines::LOL_WIN } else { lines::LOL_LOSS }, &f(&[])));
            }
        }
    }

    /// A multi-kill: Capri cheers, and (if on) a clip is saved a few seconds
    /// later, so a fourth or fifth kill still makes it in.
    fn big_moment(&self, s: &PlaySettings, set: &[&str], secs: u32) {
        self.state.lock().last_react = None;
        self.react(s, &lines::pick(set, &self.fill(s, &[])));
        if s.auto_clip && s.replay {
            self.state.lock().auto_clip = Some((Instant::now() + Duration::from_secs(6), secs));
            self.send_wake();
        }
    }

    /// Wake the worker so it sees a new automatic clip's deadline.
    fn send_wake(&self) {
        self.send(Msg::Wake);
    }

    fn clip_path(&self, s: &PlaySettings, what: &str) -> PathBuf {
        let game = self.title().replace(|c: char| !(c.is_alphanumeric() || c == ' ' || c == '-'), "");
        let stamp = chrono::Local::now().format("%Y-%m-%d %H-%M-%S");
        Self::folder(s).join(format!("{game} {what}{stamp}.mp4"))
    }

    /// Save the last `secs` seconds (from a voice command, the page, or a big moment).
    pub fn save_clip(&self, core: &AppCore, secs: u32, auto: bool) {
        let _ = self.clip(core, secs, auto);
    }

    /// Like [`PlayHub::save_clip`], returning where the clip went.
    pub fn clip(&self, core: &AppCore, secs: u32, auto: bool) -> Result<PathBuf, String> {
        let s = core.settings.get().play;
        let f = |extra: &[(&'static str, String)]| self.fill(&s, extra);
        let keep = s.replay_minutes.clamp(1, 20) * 60;
        let rec = self.state.lock().recorder.clone();
        let Some(r) = rec else {
            let line = if !s.replay { lines::NO_REPLAY } else { lines::NO_GAME };
            let msg = lines::pick(line, &f(&[]));
            if !auto {
                self.say(&msg);
            }
            return Err(msg);
        };
        let dest = self.clip_path(&s, if auto { "highlight " } else { "" });
        let want = secs.min(keep);
        match r.save(want, &dest) {
            Ok(()) => {
                if let Some(g) = self.state.lock().game.as_mut() {
                    g.clips += 1;
                }
                if !auto {
                    let line = if secs > keep {
                        lines::pick(lines::ONLY_KEEP, &f(&[("minutes", (keep / 60).to_string())]))
                    } else if secs >= 60 {
                        let what = if secs.is_multiple_of(60) { format!("{} minute{}", secs / 60, if secs == 60 { "" } else { "s" }) } else { format!("{secs} seconds") };
                        lines::pick(lines::LONG_SAVED, &f(&[("what", what)]))
                    } else {
                        lines::pick(lines::CLIP_SAVED, &f(&[("secs", secs.to_string())]))
                    };
                    self.say(&line);
                }
                self.events.emit("play:clip", serde_json::json!({ "path": dest.display().to_string(), "seconds": want, "auto": auto }));
                Ok(dest)
            }
            Err(e) => {
                if !auto {
                    self.say(&lines::pick(lines::CLIP_FAILED, &f(&[("why", e.clone())])));
                }
                Err(e)
            }
        }
    }

    pub fn start_recording(&self, s: &PlaySettings) {
        let rec = self.state.lock().recorder.clone();
        match rec {
            Some(r) => {
                r.start_recording();
                self.state.lock().recording_since = Some(Instant::now());
                self.say(&lines::pick(lines::RECORDING, &self.fill(s, &[])));
            }
            None => self.say(&lines::pick(lines::NO_REPLAY, &self.fill(s, &[]))),
        }
        self.changed();
    }

    pub fn stop_recording(&self, core: &AppCore) {
        let s = core.settings.get().play;
        let dest = self.clip_path(&s, "recording ");
        let rec = self.state.lock().recorder.clone();
        let r = match rec {
            Some(r) if r.recording() => r.stop_recording(&dest),
            _ => Err(String::new()),
        };
        let since = self.state.lock().recording_since.take();
        match r {
            Ok(()) => {
                self.say(&lines::pick(lines::RECORDING_SAVED, &self.fill(&s, &[("duration", lines::duration(since.map_or(0, |t| t.elapsed().as_secs())))])));
                self.events.emit("play:clip", serde_json::json!({ "path": dest.display().to_string(), "recording": true }));
            }
            Err(e) if e.is_empty() => self.say(&lines::pick(lines::NOT_RECORDING, &self.fill(&s, &[]))),
            Err(e) => self.say(&lines::pick(lines::CLIP_FAILED, &self.fill(&s, &[("why", e)]))),
        }
        self.changed();
    }

    /// Stop everything (OmniHub is closing).
    pub fn shutdown(&self) {
        let mut st = self.state.lock();
        st.recorder = None;
        st.listener = None;
        st.cs2 = None;
    }
}

/// The token CS2 sends back (kept, so its config stays valid).
fn cs2_token(tools: &std::path::Path) -> String {
    let file = tools.join("cs2-gsi-token.txt");
    if let Ok(t) = std::fs::read_to_string(&file) {
        if t.trim().len() >= 16 {
            return t.trim().to_string();
        }
    }
    let t = uuid::Uuid::new_v4().simple().to_string();
    let _ = std::fs::create_dir_all(tools);
    let _ = std::fs::write(&file, &t);
    t
}

