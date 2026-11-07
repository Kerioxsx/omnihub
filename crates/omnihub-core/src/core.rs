//! `AppCore` owns every service. The desktop shell and the headless server
//! both create one and talk to it; the companion server holds a reference.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::apps::AppLibrary;
use crate::audit::Audit;
use crate::capture::bridges::Bridges;
use crate::capture::screenshots::{self, ScreenshotLibrary};
use crate::db::Db;
use crate::events::EventBus;
use crate::notes::{ExportOptions, Notes};
use crate::paths::AppPaths;
use crate::remote::RemoteServer;
use crate::settings::{Settings, SettingsStore};
use crate::storage::engine::{ElevatedRunner, SelfElevated, StorageEngine};
use crate::system::power::PowerScheduler;
use crate::vault::crypto::KdfParams;
use crate::vault::Vault;

pub struct CoreOptions {
    /// Log power actions instead of performing them.
    pub dry_run_power: bool,
    pub runner: Arc<dyn ElevatedRunner>,
    /// Override vault key-derivation cost (tests).
    pub vault_kdf: Option<KdfParams>,
    /// Override DPAPI use for the vault (tests on Windows).
    pub vault_dpapi: Option<bool>,
    /// Register with the browsers and listen for the extension when browser
    /// autofill is on (tests turn this off: no real browser config is touched).
    pub browser_integration: bool,
    /// Use the pretend music player (tests; also `OMNIHUB_FAKE_MEDIA=1`).
    pub fake_media: bool,
}

impl Default for CoreOptions {
    fn default() -> Self {
        CoreOptions { dry_run_power: false, runner: Arc::new(SelfElevated), vault_kdf: None, vault_dpapi: None, browser_integration: true, fake_media: false }
    }
}

pub struct AppCore {
    pub paths: AppPaths,
    pub settings: SettingsStore,
    pub events: EventBus,
    pub db: Arc<Db>,
    pub audit: Audit,
    pub storage: Arc<StorageEngine>,
    pub notes: Notes,
    pub vault: Vault,
    pub screenshots: ScreenshotLibrary,
    pub apps: AppLibrary,
    pub power: PowerScheduler,
    pub bridges: Bridges,
    pub remote: RemoteServer,
    pub thumbs: crate::thumbs::Thumbs,
    pub airplay: crate::capture::airplay::AirPlay,
    pub browser: crate::browser::BrowserBridge,
    pub procs: Arc<crate::system::procs::ProcessMonitor>,
    pub games: Arc<crate::games::GameHub>,
    /// Capri and your games: reactions, voice commands, instant replay.
    pub play: Arc<crate::play::PlayHub>,
    /// Saved window arrangements ("Work", "Gaming").
    pub layouts: crate::system::layouts::Layouts,
    /// Folder rules (tidy Downloads and the like).
    pub rules: crate::rules::Rules,
    /// The focus timer and the time tracker.
    pub focus: crate::focus::Focus,
    /// The Pokémon save being edited.
    pub pkm: crate::pkm::Editor,
    pub counters: crate::counters::Counters,
    /// The game library's notes, sessions and save backups.
    pub shelf: crate::games::shelf::Shelf,
    /// Your own pictures behind the Bleach themes.
    pub theme_art: crate::theme_art::ThemeArt,
    /// Quick sites (Netflix, YouTube…) for Big Picture and the phone.
    pub sites: crate::sites::Sites,
    /// Game art addresses asked of Steam's store.
    pub store_art: Arc<crate::games::storeart::StoreArt>,
    pub emulators: crate::games::emulators::Emulators,
    /// The OBS page for counters, while on.
    pub counters_overlay: parking_lot::Mutex<Option<crate::counters_overlay::OverlayServer>>,
    pub autoclick: crate::autoclick::AutoClicker,
    /// Clipboard history, the quick-launch wheel and the macro deck.
    pub quick: Arc<crate::quick::Quick>,
    /// The PC's, controllers' and Bluetooth devices' batteries.
    pub batteries: crate::system::battery::Batteries,
    /// The library game running now: (name, started, unix ms).
    pub now_playing: parking_lot::Mutex<Option<(String, i64)>>,
    /// The Discord status ("Playing …").
    pub presence: crate::presence::Presence,
    /// Network speed and data used per day.
    pub netmon: crate::system::netmon::NetMonitor,
    /// Folder backups to another drive.
    pub backups: Arc<crate::backup::Backups>,
    /// Screens off and fans at their lowest (from the phone, the deck…).
    pub quiet: crate::quiet::Quiet,
    /// Capri Desktop (the fan-made companion app) and Ollama.
    pub capri_app: crate::capri_app::CapriApp,
    pub heat: crate::heat::Heat,
    pub app_updates: crate::app_updates::AutoUpdates,
    pub party: crate::party::Party,
    pub screentime: crate::screentime::ScreenTime,
    /// The Fortnite item shop and price watches.
    pub watchers: crate::watchers::Watchers,
    /// Compress, convert, cut and take the sound out of videos (FFmpeg).
    pub video: crate::videotools::VideoTools,
    /// Links downloaded for a phone (or the PC).
    pub downloads: Arc<crate::downloads::Downloads>,
    pub updater: Arc<crate::update::Updater>,
    pub media: Arc<crate::media::MediaHub>,
    /// Listens to the PC's sound for the music visuals (only while one is shown).
    pub visual: Arc<crate::media::visual::VisualHub>,
    /// Lighting, fans, screens and specs ("My PC").
    pub rig: Arc<crate::rig::RigHub>,
    /// Govee lights on the network (and through Govee's cloud with a key).
    pub govee: Arc<crate::govee::GoveeHub>,
    /// Games' mods, Claude's mod maker and Minecraft modpacks.
    pub mods: Arc<crate::mods::ModHub>,
    /// Chats with Claude and ChatGPT's models, and Claude Code and Codex conversations.
    pub chats: Arc<crate::chats::ChatHub>,
    /// Minecraft servers hosted on this PC.
    pub mcserver: Arc<crate::mcserver::ServerHub>,
    browser_integration: bool,
    /// Drives already warned about (root → when), so each warns once a day.
    low_space_warned: parking_lot::Mutex<std::collections::HashMap<String, i64>>,
}

impl AppCore {
    pub fn new(paths: AppPaths, opts: CoreOptions) -> anyhow::Result<Arc<Self>> {
        let events = EventBus::new();
        let settings = SettingsStore::load(&paths.settings_file());
        let db = Arc::new(Db::open(&paths.database())?);
        let audit = Audit::new(db.clone(), events.clone());
        let storage = StorageEngine::new(events.clone(), paths.scans(), opts.runner.clone());
        let vault = Vault::with_options(
            &paths.vault_file(),
            events.clone(),
            opts.vault_kdf.unwrap_or_default(),
            opts.vault_dpapi.unwrap_or(crate::system::dpapi::available()),
        );
        let s = settings.get();
        vault.set_auto_lock(s.vault.auto_lock_minutes);
        // Copies Open left behind last time (an app still had them open).
        vault.hidden.clean_opened();
        let procs = Arc::new(crate::system::procs::ProcessMonitor::new());
        let media = crate::media::MediaHub::new(&paths.data, events.clone(), opts.fake_media);
        let games = crate::games::GameHub::new(&paths.data, events.clone(), procs.clone());
        let play = crate::play::PlayHub::new(&paths.data, events.clone());
        let video = crate::videotools::VideoTools::new(play.ffmpeg_path(), events.clone());
        let downloads = Arc::new(crate::downloads::Downloads::new(media.clone(), play.ffmpeg_path(), events.clone()));
        let mods = crate::mods::ModHub::new(&paths.data, events.clone(), games.clone(), procs.clone());
        let chats = crate::chats::ChatHub::new(&paths.data, events.clone(), mods.claude.clone());
        let mcserver = crate::mcserver::ServerHub::new(&paths.data, events.clone(), mods.clone());
        mods.maker.set_servers(mcserver.clone());
        let visual = crate::media::visual::VisualHub::new(events.clone(), opts.fake_media || std::env::var("OMNIHUB_FAKE_MEDIA").is_ok_and(|v| v == "1"));
        let core = Arc::new(AppCore {
            notes: Notes::new(db.clone(), events.clone()),
            screenshots: ScreenshotLibrary::new(db.clone(), events.clone(), &paths.cache),
            apps: AppLibrary::new(&paths.cache),
            power: PowerScheduler::new(events.clone(), audit.clone(), opts.dry_run_power),
            bridges: Bridges::new(),
            remote: RemoteServer::new(db.clone(), events.clone()),
            thumbs: crate::thumbs::Thumbs::default(),
            browser: crate::browser::BrowserBridge::new(db.clone(), events.clone()),
            browser_integration: opts.browser_integration,
            games,
            play,
            mods,
            chats,
            mcserver,
            updater: Arc::new(crate::update::Updater::new(&paths.cache, &paths.data, events.clone())),
            procs,
            media: media.clone(),
            visual: visual.clone(),
            rig: crate::rig::RigHub::new(&paths.data, events.clone(), visual.clone()),
            govee: crate::govee::GoveeHub::new(&paths.data, events.clone(), visual, media),
            low_space_warned: Default::default(),
            layouts: crate::system::layouts::Layouts::new(&paths.data),
            rules: crate::rules::Rules::new(&paths.data),
            focus: crate::focus::Focus::new(db.clone()),
            pkm: crate::pkm::Editor::default(),
            counters: crate::counters::Counters::new(&paths.data, events.clone()),
            shelf: crate::games::shelf::Shelf::new(&paths.data, events.clone()),
            theme_art: crate::theme_art::ThemeArt::new(&paths.data),
            sites: crate::sites::Sites::new(&paths.data),
            store_art: Arc::new(crate::games::storeart::StoreArt::new(&paths.cache)),
            emulators: crate::games::emulators::Emulators::new(&paths.data),
            counters_overlay: parking_lot::Mutex::new(None),
            autoclick: crate::autoclick::AutoClicker::new(events.clone()),
            quick: Arc::new(crate::quick::Quick::new(&paths.data, events.clone())),
            batteries: crate::system::battery::Batteries::new(events.clone()),
            now_playing: parking_lot::Mutex::new(None),
            presence: crate::presence::Presence::new(&paths.data),
            netmon: crate::system::netmon::NetMonitor::new(&paths.data),
            backups: Arc::new(crate::backup::Backups::new(&paths.data, events.clone())),
            quiet: crate::quiet::Quiet::new(&paths.data),
            capri_app: crate::capri_app::CapriApp::new(&paths.data, &paths.cache, events.clone()),
            heat: crate::heat::Heat::new(&paths.data),
            app_updates: crate::app_updates::AutoUpdates::new(&paths.data),
            party: crate::party::Party::default(),
            screentime: crate::screentime::ScreenTime::new(&paths.data),
            watchers: crate::watchers::Watchers::new(&paths.data),
            video,
            downloads,
            airplay: crate::capture::airplay::AirPlay::new(&paths.data.join("addons"), &paths.data, events.clone()),
            paths,
            settings,
            events,
            db,
            audit,
            storage,
            vault,
        });
        core.apply_visuals(&s.visuals);
        core.media.set_lrc_folder(s.media.lrc_folder.as_ref().map(std::path::PathBuf::from));
        core.media.set_hires_art(s.media.hires_art);
        Ok(core)
    }

    /// Serve (or stop) the counters' OBS page as the counters' settings say.
    pub fn apply_counters_overlay(self: &Arc<Self>) -> Result<(), String> {
        let s = self.counters.state();
        let mut cur = self.counters_overlay.lock();
        if !s.overlay_on {
            *cur = None;
            return Ok(());
        }
        if cur.as_ref().is_some_and(|o| o.port == s.overlay_port) {
            return Ok(());
        }
        *cur = None;
        *cur = Some(crate::counters_overlay::start(s.overlay_port, Arc::downgrade(self))?);
        Ok(())
    }

    /// Watch the keys counters and the auto clicker need (and no others).
    pub fn refresh_keys(&self) {
        let mut keys = self.counters.watched_keys();
        keys.extend(self.autoclick.watched_keys());
        keys.sort_unstable();
        keys.dedup();
        crate::system::keys::watch(&keys);
    }

    /// Start timers, watchers and (if enabled) the companion server.
    pub fn start_background(self: &Arc<Self>) {
        self.play.start(self);
        let weak = Arc::downgrade(self);
        crate::system::keys::on_key(move |vk| {
            let Some(core) = weak.upgrade() else { return };
            let clicker = core.autoclick.on_key(vk);
            core.counters.on_key(vk);
            if clicker {
                core.refresh_keys();
            }
        });
        self.refresh_keys();
        self.start_game_tracking();
        if let Err(e) = self.apply_counters_overlay() {
            tracing::warn!("counters overlay: {e}");
        }
        crate::quick::watch(self.quick.clone());
        // Network speed every two seconds; data used per day saved each minute.
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("network-use".into())
            .spawn(move || {
                let mut n = 0u32;
                loop {
                    let Some(core) = weak.upgrade() else { return };
                    match crate::system::netmon::totals() {
                        Some((rx, tx)) => core.netmon.take(chrono::Utc::now().timestamp_millis(), &chrono::Local::now().format("%Y-%m-%d").to_string(), rx, tx),
                        None => return,
                    }
                    n += 1;
                    if n.is_multiple_of(30) {
                        core.netmon.save();
                    }
                    drop(core);
                    std::thread::sleep(Duration::from_secs(2));
                }
            })
            .ok();
        self.start_extras();
        // Quiet mode ends when someone uses the PC's mouse or keyboard.
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("quiet-mode".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(2));
                let Some(core) = weak.upgrade() else { return };
                core.quiet_check();
            })
            .ok();
        // Backups that are due (daily, weekly, or a drive just plugged in).
        let backups = self.backups.clone();
        std::thread::Builder::new()
            .name("backups".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(5 * 60));
                backups.run_due(crate::db::now());
            })
            .ok();
        // The Discord status: the game running and the shiny hunt going on.
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("discord-status".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(15));
                let Some(core) = weak.upgrade() else { return };
                let s = core.presence.settings();
                if !s.enabled {
                    core.presence.update(None);
                    continue;
                }
                let now = crate::db::now() * 1000;
                let game = core.now_playing.lock().clone();
                let hunt = core
                    .counters
                    .state()
                    .counters
                    .into_iter()
                    .filter(|c| c.hunt.is_some() && (c.timer_since.is_some() || now - c.updated < 10 * 60 * 1000))
                    .max_by_key(|c| c.updated)
                    .and_then(|c| {
                        let h = c.hunt?;
                        let odds = (1.0 / h.chance().max(1e-9)).round() as u64;
                        Some((h.name, c.count, format!("1/{odds}")))
                    });
                core.presence.update(crate::presence::activity(&s, game, hunt));
            })
            .ok();
        // Read the text in screenshots a few at a time, so search finds them
        // by what they show (Windows' own OCR, on this PC).
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("screenshot-text".into())
            .spawn(move || {
                std::thread::sleep(Duration::from_secs(90));
                loop {
                    let Some(core) = weak.upgrade() else { return };
                    let n = if core.games.session().is_some_and(|s| s.active()) { 0 } else { core.screenshots.index_text(12, crate::capture::ocr::recognize) };
                    drop(core);
                    std::thread::sleep(Duration::from_secs(if n > 0 { 20 } else { 300 }));
                }
            })
            .ok();
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("batteries".into())
            .spawn(move || loop {
                let Some(core) = weak.upgrade() else { return };
                core.batteries.sample(crate::db::now() * 1000);
                drop(core);
                std::thread::sleep(Duration::from_secs(120));
            })
            .ok();
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("vault-timer".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(3));
                let Some(core) = weak.upgrade() else { return };
                core.vault.tick();
                if core.vault.is_unlocked() && core.settings.get().vault.lock_on_session_lock && crate::vault::session_locked() {
                    core.vault.lock_with_reason("session-locked");
                }
                core.remote.expire_vault_sessions();
            })
            .ok();
        self.rig.start();
        self.govee.start();
        self.mcserver.start_background();
        let s = self.settings.get();
        self.notes.watch_folder(s.notes.claude_folder.as_deref().map(std::path::Path::new));
        let core = self.clone();
        std::thread::spawn(move || {
            let dir = core.screenshot_dir();
            let _ = core.screenshots.sync_folder(&dir);
        });
        if s.remote.enabled && !self.remote.is_running() {
            if let Err(e) = self.remote.start(self.clone()) {
                tracing::error!("companion server failed to start: {e}");
            }
        }
        if s.screen.airplay_auto_start && self.airplay.find(s.screen.uxplay_path.as_deref()).is_some() {
            if let Err(e) = self.airplay.start(s.screen.uxplay_path.as_deref(), &s.screen.airplay, s.screen.airplay_keep_on_top, s.screen.airplay_pip) {
                tracing::warn!("AirPlay receiver failed to start: {e}");
            }
        }
        if s.vault.browser_autofill {
            self.enable_browser_autofill(true);
        }
        self.start_watchers();
        self.start_productivity();
        let games = self.games.clone();
        std::thread::spawn(move || games.recover());
        self.start_update_checks();
        let weak = Arc::downgrade(self);
        self.media.start(move || weak.upgrade().is_some_and(|c| c.settings.get().media.lyrics_online));
        let send_to = s.remote.send_to_menu;
        let core = self.clone();
        std::thread::spawn(move || {
            if let Err(e) = crate::system::shell::set_send_to_shortcut(send_to) {
                tracing::warn!("could not update the Send to shortcut: {e}");
            }
            core.clean_outbox();
        });
    }

    /// Register the native-messaging host and listen for the extension (or stop).
    pub fn enable_browser_autofill(self: &Arc<Self>, on: bool) {
        if !self.browser_integration {
            return;
        }
        if on {
            if let Err(e) = crate::browser::register::register(&self.paths.data, &self.settings.get().vault.browsers) {
                tracing::warn!("could not register the browser host: {e}");
            }
        } else {
            crate::browser::register::unregister();
        }
        if let Err(e) = self.browser.set_listening(self, on) {
            tracing::warn!("browser autofill pipe: {e}");
        }
    }

    /// Half a minute after start, then every six hours: look for a new
    /// version, and install it when that's allowed and nothing is going on.
    fn start_update_checks(self: &Arc<Self>) {
        self.updater.clean();
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("updates".into())
            .spawn(move || {
                std::thread::sleep(Duration::from_secs(30));
                loop {
                    let Some(core) = weak.upgrade() else { return };
                    let s = core.settings.get().updates;
                    let mut pending = None;
                    if s.check {
                        if let crate::update::UpdateState::Available { release } = core.updater.check() {
                            // No surprise Windows prompt from the tray: an
                            // install that needs approval waits for a click.
                            // Nor again by itself after that version didn't finish installing.
                            if s.auto_install && !core.updater.needs_approval() && core.updater.failed_version() != Some(release.version.as_str()) {
                                pending = Some(release);
                            }
                        }
                    }
                    drop(core);
                    // An automatic update closes and reopens the app, so it
                    // waits until OmniHub is in the tray and nothing would be
                    // interrupted. Meanwhile the sidebar offers it in one click.
                    let next_check = std::time::Instant::now() + Duration::from_secs(6 * 3600);
                    while std::time::Instant::now() < next_check {
                        if let Some(release) = &pending {
                            let Some(core) = weak.upgrade() else { return };
                            let still = matches!(core.updater.state(), crate::update::UpdateState::Available { .. });
                            if !still {
                                pending = None;
                            } else if core.idle_for_update() {
                                tracing::info!("installing OmniHub {}", release.version);
                                core.audit.record("app", "update.install", &release.version, true);
                                let _ = core.updater.install(release);
                                pending = None;
                            }
                        }
                        std::thread::sleep(Duration::from_secs(if pending.is_some() { 60 } else { 600 }));
                    }
                }
            })
            .ok();
    }

    /// Nothing an automatic update would interrupt: the window is not open
    /// in front of someone, no game boost, nobody watching the screen, no
    /// iPhone mirroring receiver, no phone transfer.
    pub fn idle_for_update(&self) -> bool {
        let boosting = self.games.session().is_some_and(|s| s.active());
        let watching = !self.remote.status().viewers.is_empty();
        !self.updater.app_in_use() && !boosting && !watching && !self.airplay.is_running() && !self.remote.transfers_active()
    }

    /// Every 20 seconds: due note reminders; every five minutes: drive space.
    fn start_watchers(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("watchers".into())
            .spawn(move || {
                let mut tick: u64 = 0;
                loop {
                    let Some(core) = weak.upgrade() else { return };
                    match core.notes.take_due_reminders(crate::db::now()) {
                        Ok(due) => {
                            for n in due {
                                let snippet: String = n.body.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#')).unwrap_or_default().chars().take(140).collect();
                                core.events.emit("notes:reminder", serde_json::json!({ "id": n.id, "title": n.title, "snippet": snippet, "kind": n.kind }));
                            }
                        }
                        Err(e) => tracing::warn!("reminders: {e}"),
                    }
                    if tick.is_multiple_of(15) {
                        core.check_low_space();
                    }
                    drop(core);
                    tick += 1;
                    std::thread::sleep(Duration::from_secs(20));
                }
            })
            .ok();
    }

    /// Emit `storage:low-space` for drives that just ran low.
    pub fn check_low_space(&self) {
        let s = self.settings.get().storage;
        if !s.low_space_alert {
            return;
        }
        let vols = crate::storage::volumes::list();
        let due = low_space_due(&vols, s.low_space_percent, &mut self.low_space_warned.lock(), crate::db::now());
        for v in due {
            self.events.emit("storage:low-space", serde_json::json!({ "root": v.root, "label": v.label, "free": v.free, "total": v.total }));
        }
    }

    /// Where zips of folders sent to phones are made.
    pub fn outbox_dir(&self) -> PathBuf {
        self.paths.cache.join(crate::remote::transfer::OUTBOX_DIR)
    }

    /// Delete zips no offer refers to any more.
    fn clean_outbox(&self) {
        let used: std::collections::HashSet<PathBuf> = self.remote.inbox.all().into_iter().map(|i| i.path).collect();
        let Ok(rd) = std::fs::read_dir(self.outbox_dir()) else { return };
        for e in rd.flatten() {
            if !used.contains(&e.path()) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }

    pub fn screenshot_dir(&self) -> PathBuf {
        self.settings.get().screenshots.dir.map(PathBuf::from).unwrap_or_else(screenshots::default_dir)
    }

    /// Where pages scanned with the phone go.
    pub fn scans_dir(&self) -> PathBuf {
        self.settings.get().remote.scans_dir.filter(|d| !d.trim().is_empty()).map(PathBuf::from).unwrap_or_else(|| dirs::document_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Documents")).join("OmniHub Scans"))
    }

    /// Pages from the phone's scanner: saved (PDF, or the picture for one
    /// page), read with Windows' text recognition, a receipt or card read into
    /// fields, and a note made when asked.
    pub fn scan(&self, kind: crate::scan::Kind, title: &str, target: crate::scan::Target, pages: Vec<Vec<u8>>, from: &str) -> Result<crate::scan::Scanned, String> {
        use crate::scan::{self, Kind, Target};
        if pages.is_empty() || pages.len() > 60 {
            return Err("Send between 1 and 60 pages.".into());
        }
        if pages.iter().any(|p| scan::jpeg_size(p).is_none()) {
            return Err("A page isn't a JPEG picture.".into());
        }
        let name = scan::file_title(title, kind);
        let file = scan::save_pages(&self.scans_dir(), &name, &pages)?;
        let mut text = String::new();
        for p in &pages {
            if let Ok(t) = crate::capture::ocr::recognize_bytes(p.clone()) {
                if !text.is_empty() && !t.trim().is_empty() {
                    text.push_str("\n\n");
                }
                text.push_str(t.trim());
            }
        }
        let fields = match kind {
            Kind::Receipt => scan::parse_receipt(&text),
            Kind::Card => scan::parse_card(&text),
            Kind::Document => Vec::new(),
        };
        let mut files = vec![file.display().to_string()];
        if target == Target::Folder && !text.trim().is_empty() {
            let t = file.with_extension("txt");
            std::fs::write(&t, &text).map_err(|e| e.to_string())?;
            files.push(t.display().to_string());
        }
        if kind == Kind::Card && !fields.is_empty() {
            let v = file.with_extension("vcf");
            std::fs::write(&v, scan::vcard(&fields)).map_err(|e| e.to_string())?;
            files.push(v.display().to_string());
        }
        let field = |k: &str| fields.iter().find(|(f, _)| f == k).map(|(_, v)| v.clone());
        let note_id = if target == Target::Notes {
            let shown = match (title.trim(), kind) {
                (t, _) if !t.is_empty() => t.to_string(),
                (_, Kind::Receipt) => format!("Receipt{}{}", field("Shop").map(|s| format!(" – {s}")).unwrap_or_default(), field("Total").map(|t| format!(" ({t})")).unwrap_or_default()),
                (_, Kind::Card) => field("Name").unwrap_or_else(|| "Business card".into()),
                _ => name.clone(),
            };
            let tag = match kind {
                Kind::Document => "scan",
                Kind::Receipt => "receipt",
                Kind::Card => "card",
            };
            let note = self
                .notes
                .save(crate::notes::NoteInput { id: None, kind: Default::default(), title: shown, body: scan::note_body(kind, &fields, &text, &file), tags: vec![tag.into()], pinned: false, color: None, locked: false })
                .map_err(|e| e.to_string())?;
            Some(note.id)
        } else {
            None
        };
        self.audit.record(from, "scan", &file.display().to_string(), true);
        self.events.emit("scan:saved", serde_json::json!({ "path": file.display().to_string(), "from": from }));
        Ok(scan::Scanned { title: name, files, text, fields, note_id })
    }

    /// Approve on your phone: waits for a paired phone's yes when this kind
    /// of action is chosen in Settings. Nothing is asked with no phone paired
    /// or the phone connection off.
    pub fn approve(&self, kind: crate::remote::approvals::Kind, detail: &str) -> Result<(), String> {
        use crate::remote::approvals::{Kind, WAIT};
        let s = self.settings.get();
        let a = &s.approvals;
        let wanted = a.enabled
            && match kind {
                Kind::VaultReveal | Kind::VaultReset => a.vault,
                Kind::Admin => a.admin,
                Kind::Shred => a.shred,
                Kind::Environment => a.environment,
                Kind::Settings => true,
            };
        if !wanted || !s.remote.enabled || !self.remote.devices.list().iter().any(|d| !d.revoked) || self.remote.approvals.recently(kind, detail) {
            return Ok(());
        }
        let events = self.events.clone();
        let mut id = String::new();
        let r = self.remote.approvals.ask(kind, detail, WAIT, |req| {
            id = req.id.clone();
            events.emit("approval:request", req);
        });
        self.events.emit("approval:done", serde_json::json!({ "id": id, "approved": r.is_ok() }));
        let device = r?;
        self.audit.record(&format!("phone:{device}"), "approved", &format!("{} ({detail})", kind.label()), true);
        Ok(())
    }

    pub fn incoming_dir(&self) -> PathBuf {
        self.settings
            .get()
            .remote
            .incoming_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs::download_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default()).join("OmniHub"))
    }

    pub fn export_options(&self) -> ExportOptions {
        let n = self.settings.get().notes;
        ExportOptions { sidecar_json: n.sidecar_json, index_file: n.index_file }
    }

    /// Audio capture on or off, and how the analysis reacts.
    fn apply_visuals(&self, v: &crate::settings::VisualSettings) {
        self.visual.set_enabled(v.enabled && v.audio_reactive);
        self.visual.set_tuning(crate::media::visual::analyzer::Tuning { sensitivity: v.sensitivity, bass: v.bass, smoothing: v.smoothing });
    }

    /// Settings (and window layouts) as a file to keep or take to another
    /// PC. Keys, passwords and paired phones aren't in it: they never are in
    /// the settings, and stay protected on this PC.
    pub fn export_settings(&self, path: &std::path::Path) -> Result<(), String> {
        let doc = serde_json::json!({
            "omnihubSettings": 1,
            "version": env!("CARGO_PKG_VERSION"),
            "exported": chrono::Utc::now().to_rfc3339(),
            "settings": self.settings.get(),
            "layouts": self.layouts.list(),
        });
        let bytes = serde_json::to_vec_pretty(&doc).map_err(|e| e.to_string())?;
        crate::settings::write_atomic(path, &bytes).map_err(|e| e.to_string())
    }

    /// Bring in settings saved with `export_settings`. This PC's own name
    /// and phone connection stay as they are.
    pub fn import_settings(self: &Arc<Self>, path: &std::path::Path) -> Result<Settings, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let doc: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| "That file is not an OmniHub settings backup.".to_string())?;
        if doc.get("omnihubSettings").is_none() || !doc["settings"].is_object() {
            return Err("That file is not an OmniHub settings backup.".into());
        }
        // Read it as settings first, so a damaged file changes nothing.
        let parsed: Settings = serde_json::from_value(doc["settings"].clone()).map_err(|e| format!("The backup could not be read: {e}"))?;
        let mut patch = serde_json::to_value(parsed).map_err(|e| e.to_string())?;
        if let Some(remote) = patch.get_mut("remote").and_then(|r| r.as_object_mut()) {
            for k in ["deviceName", "enabled", "port", "bind", "tls"] {
                remote.remove(k);
            }
        }
        // A backup can't switch approve-on-phone off behind its back.
        if self.settings.get().approvals.enabled {
            if let Some(o) = patch.as_object_mut() {
                o.remove("approvals");
            }
        }
        let after = self.update_settings(&patch).map_err(|e| e.to_string())?;
        if let Ok(layouts) = serde_json::from_value::<Vec<crate::system::layouts::Layout>>(doc["layouts"].clone()) {
            self.layouts.import(layouts);
        }
        Ok(after)
    }

    /// Folder rules every half minute; the time tracker and the focus timer every few seconds.
    fn start_productivity(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("folder-rules".into())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_secs(30));
                let Some(core) = weak.upgrade() else { return };
                let done = core.rules.run_once(std::time::SystemTime::now());
                if !done.is_empty() {
                    core.events.emit("rules:done", &done);
                }
            })
            .ok();
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("focus".into())
            .spawn(move || loop {
                std::thread::sleep(crate::focus::SAMPLE);
                let Some(core) = weak.upgrade() else { return };
                let day = crate::focus::day_of(chrono::Local::now());
                if core.settings.get().focus.track_time {
                    if let Some(app) = crate::focus::foreground() {
                        core.focus.sample(&app, crate::focus::SAMPLE.as_secs(), &day);
                    }
                }
                let now_ms = chrono::Utc::now().timestamp_millis();
                if let Some(phase) = core.focus.tick(&day, now_ms) {
                    core.events.emit("focus:phase", serde_json::json!({ "phase": phase }));
                    core.events.emit("focus:changed", serde_json::json!({}));
                }
                if core.focus.state().is_some() {
                    let fake = core.media.is_fake();
                    let open: Vec<String> = crate::system::open_apps::list(fake).into_iter().map(|a| a.key).collect();
                    for key in core.focus.to_close(&open) {
                        let _ = crate::system::open_apps::close(fake, &key);
                    }
                }
            })
            .ok();
    }

    /// The panic button: lock the vault (closing open hidden files), cut
    /// every phone off, stop screen sharing and lock the PC. The desktop
    /// app also hides its window on "app:panic". Phone access stays off
    /// until you turn it on again in Phone.
    pub fn panic(&self, by: &str) {
        self.vault.lock();
        self.remote.stop();
        // Off for good, not just until OmniHub restarts.
        if let Ok(after) = self.settings.update(&serde_json::json!({ "remote": { "enabled": false } })) {
            self.events.emit("settings:changed", &after);
        }
        self.audit.record(by, "panic", "vault locked, phone access stopped, PC locked", true);
        self.events.emit("app:panic", serde_json::json!({ "by": by }));
        self.power.schedule(crate::system::power::PowerAction::Lock, 0, by);
    }

    /// Apply a settings patch and react to what changed.
    pub fn update_settings(self: &Arc<Self>, patch: &serde_json::Value) -> anyhow::Result<Settings> {
        let before = self.settings.get();
        let after = self.settings.update(patch)?;
        if before.vault.auto_lock_minutes != after.vault.auto_lock_minutes {
            self.vault.set_auto_lock(after.vault.auto_lock_minutes);
        }
        if before.notes.claude_folder != after.notes.claude_folder {
            self.notes.watch_folder(after.notes.claude_folder.as_deref().map(std::path::Path::new));
        }
        if (before.screen.airplay != after.screen.airplay || before.screen.uxplay_path != after.screen.uxplay_path) && self.airplay.is_running() {
            let sc = &after.screen;
            if let Err(e) = self.airplay.start(sc.uxplay_path.as_deref(), &sc.airplay, sc.airplay_keep_on_top, sc.airplay_pip) {
                tracing::warn!("could not restart the AirPlay receiver: {e}");
            }
        }
        if (before.media.eq_enabled, before.media.bass_db, before.media.treble_db) != (after.media.eq_enabled, after.media.bass_db, after.media.treble_db) {
            let m = after.media.clone();
            std::thread::spawn(move || {
                if let Err(e) = crate::media::eq::apply(m.bass_db, m.treble_db, m.eq_enabled) {
                    tracing::warn!("equaliser: {e}");
                }
            });
        }
        if before.media.hires_art != after.media.hires_art {
            self.media.set_hires_art(after.media.hires_art);
        }
        if before.media.lrc_folder != after.media.lrc_folder {
            self.media.set_lrc_folder(after.media.lrc_folder.as_ref().map(std::path::PathBuf::from));
        }
        if before.visuals != after.visuals {
            self.apply_visuals(&after.visuals);
        }
        if before.vault.browser_autofill != after.vault.browser_autofill || (after.vault.browser_autofill && before.vault.browsers != after.vault.browsers) {
            self.enable_browser_autofill(after.vault.browser_autofill);
        }
        let r0 = &before.remote;
        let r1 = &after.remote;
        if r0.send_to_menu != r1.send_to_menu {
            if let Err(e) = crate::system::shell::set_send_to_shortcut(r1.send_to_menu) {
                tracing::warn!("could not update the Send to shortcut: {e}");
            }
        }
        let needs_restart = r0.port != r1.port || r0.bind != r1.bind || r0.tls != r1.tls || r0.enabled != r1.enabled;
        if needs_restart {
            self.remote.stop();
            if r1.enabled {
                self.remote.start(self.clone())?;
            }
        }
        self.events.emit("settings:changed", &after);
        Ok(after)
    }
}

/// Fixed drives below `percent` free that were not warned about in the last
/// day; drives that recovered are forgotten, so they warn again next time.
pub fn low_space_due(vols: &[crate::storage::volumes::VolumeInfo], percent: u8, warned: &mut std::collections::HashMap<String, i64>, now: i64) -> Vec<crate::storage::volumes::VolumeInfo> {
    let mut due = Vec::new();
    for v in vols.iter().filter(|v| v.kind == crate::storage::volumes::DriveKind::Fixed && v.total > 0) {
        let low = (v.free as u128) * 100 < (v.total as u128) * percent as u128;
        if !low {
            warned.remove(&v.root);
            continue;
        }
        if warned.get(&v.root).is_none_or(|t| now - t >= 24 * 3600) {
            warned.insert(v.root.clone(), now);
            due.push(v.clone());
        }
    }
    due
}

impl AppCore {
    /// Background checks for the 0.30 features: overheat alerts, screen
    /// time, the party queue, scheduled app updates, the item shop and prices,
    /// and starting Capri Desktop with OmniHub.
    fn start_extras(self: &Arc<Self>) {
        if self.capri_app.settings().start_with_omnihub && self.procs.pids_named(crate::capri_app::PROCESS).is_empty() {
            let _ = self.capri_app.open();
        }
        // Every 5 s: temperatures, the party queue; every 30 s: screen time.
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("watchers-fast".into())
            .spawn(move || {
                let mut n = 0u64;
                let mut has_temps = false;
                loop {
                    std::thread::sleep(Duration::from_secs(5));
                    let Some(core) = weak.upgrade() else { return };
                    n += 1;
                    let now = chrono::Utc::now().timestamp_millis();
                    if core.heat.settings().enabled {
                        // Keep the readings fresh; without sensors, look again every 5 minutes.
                        if n % 4 == 1 && (has_temps || n % 60 == 1) {
                            core.rig.hold();
                        }
                        let st = core.rig.status();
                        has_temps = st.cpu_temp.is_some() || st.gpu_temp.is_some();
                        for a in core.heat.check(st.cpu_temp, st.gpu_temp, now) {
                            core.events.emit("alert:hot", &a);
                        }
                    }
                    if let Some(m) = core.media.state() {
                        if core.party.song_ended(&m.title, &m.artist, m.playing, m.position_ms, m.duration_ms) {
                            let _ = core.party_play_next();
                        }
                    }
                    if n.is_multiple_of(6) {
                        let fg = crate::capture::screenshots::foreground_app();
                        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                        for act in core.screentime.tick(fg.exe.as_deref(), fg.title.as_deref(), 30, &today) {
                            match act {
                                crate::screentime::Act::Warn { name, left } => core.events.emit("screentime:limit", serde_json::json!({ "name": name, "left": left })),
                                crate::screentime::Act::CloseApp { exe, name } => {
                                    let _ = core.procs.end(&exe);
                                    core.events.emit("screentime:limit", serde_json::json!({ "name": name, "left": 0 }));
                                }
                                crate::screentime::Act::HideSite { name } => {
                                    crate::screentime::minimize_foreground();
                                    core.events.emit("screentime:limit", serde_json::json!({ "name": name, "left": 0 }));
                                }
                            }
                        }
                    }
                }
            })
            .ok();
        // Every 10 minutes: app updates due, the item shop, prices.
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("watchers-slow".into())
            .spawn(move || {
                std::thread::sleep(Duration::from_secs(60));
                let mut shop_at: Option<String> = None;
                let mut n = 0u64;
                loop {
                    let Some(core) = weak.upgrade() else { return };
                    let local = chrono::Local::now();
                    let minute = local.format("%H").to_string().parse::<u32>().unwrap_or(0) * 60 + local.format("%M").to_string().parse::<u32>().unwrap_or(0);
                    let now = crate::db::now();
                    let today_start = now - (minute as i64 * 60) - local.format("%S").to_string().parse::<i64>().unwrap_or(0);
                    let boosting = core.games.session().is_some_and(|s| s.active());
                    if !boosting && crate::app_updates::is_due(&core.app_updates.settings(), minute, today_start, now) {
                        if let Ok((done, failed)) = core.app_updates.run(now) {
                            if !done.is_empty() || !failed.is_empty() {
                                core.events.emit("apps:auto-updated", serde_json::json!({ "updated": done, "failed": failed }));
                            }
                        }
                    }
                    // The shop changes at midnight UTC.
                    let utc_day = chrono::Utc::now().format("%Y-%m-%d").to_string();
                    let w = core.watchers.get();
                    if w.shop_alerts && !w.shop_wanted.is_empty() && shop_at.as_deref() != Some(utc_day.as_str()) {
                        if let Ok(hits) = core.watchers.check_shop(&utc_day) {
                            shop_at = Some(utc_day.clone());
                            for h in hits {
                                core.events.emit("watch:shop", serde_json::json!({ "item": h }));
                            }
                        }
                    }
                    // Prices every 6 hours.
                    if n.is_multiple_of(36) {
                        for p in core.watchers.get().prices {
                            let read = crate::watchers::fetch_price(&p.url);
                            if let Some(d) = core.watchers.record(&p.id, read, now) {
                                core.events.emit("watch:price", &d);
                            }
                        }
                    }
                    n += 1;
                    drop(core);
                    std::thread::sleep(Duration::from_secs(600));
                }
            })
            .ok();
    }

    /// Hand the next party song to the music app.
    pub fn party_play_next(&self) -> Result<Option<crate::party::PartySong>, String> {
        let Some(song) = self.party.take_next() else { return Ok(None) };
        let item = crate::media::catalog::Item::Track { title: song.title.clone(), artist: song.artist.clone(), album: song.album.clone() };
        let app = self.media.state().map(|m| m.app_name);
        crate::media::catalog::play(&item, crate::media::catalog::Source::Auto, app.as_deref())?;
        self.events.emit("party:changed", self.party.state());
        Ok(Some(song))
    }
}

#[cfg(test)]
mod low_space_tests {
    use super::*;
    use crate::storage::volumes::{DriveKind, VolumeInfo};

    fn vol(root: &str, free: u64, kind: DriveKind) -> VolumeInfo {
        VolumeInfo { root: root.into(), label: String::new(), file_system: "NTFS".into(), kind, total: 1000, free, cluster_size: 4096, serial: None, mft_capable: false }
    }

    #[test]
    fn warns_once_a_day_and_again_after_recovering() {
        let mut warned = std::collections::HashMap::new();
        let vols = vec![vol("C:\\", 50, DriveKind::Fixed), vol("D:\\", 500, DriveKind::Fixed), vol("E:\\", 10, DriveKind::Removable)];
        let due = low_space_due(&vols, 10, &mut warned, 1000);
        assert_eq!(due.iter().map(|v| v.root.as_str()).collect::<Vec<_>>(), ["C:\\"]);
        assert!(low_space_due(&vols, 10, &mut warned, 1000 + 3600).is_empty());
        assert_eq!(low_space_due(&vols, 10, &mut warned, 1000 + 25 * 3600).len(), 1);
        // Freed up, then low again: warns at once.
        low_space_due(&[vol("C:\\", 400, DriveKind::Fixed)], 10, &mut warned, 1000 + 26 * 3600);
        assert_eq!(low_space_due(&vols, 10, &mut warned, 1000 + 26 * 3600 + 60).len(), 1);
    }
}
