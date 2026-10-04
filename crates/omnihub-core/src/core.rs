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
    pub procs: crate::system::procs::ProcessMonitor,
    pub media: Arc<crate::media::MediaHub>,
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
            procs: crate::system::procs::ProcessMonitor::new(),
            media: crate::media::MediaHub::new(&paths.data, events.clone(), opts.fake_media),
            low_space_warned: Default::default(),
            airplay: crate::capture::airplay::AirPlay::new(&paths.data.join("addons"), &paths.data, events.clone()),
            paths,
            settings,
            events,
            db,
            audit,
            storage,
            vault,
        });
        Ok(core)
    }

    /// Start timers, watchers and (if enabled) the companion server.
    pub fn start_background(self: &Arc<Self>) {
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
            if let Err(e) = crate::browser::register::register(&self.paths.data) {
                tracing::warn!("could not register the browser host: {e}");
            }
        } else {
            crate::browser::register::unregister();
        }
        if let Err(e) = self.browser.set_listening(self, on) {
            tracing::warn!("browser autofill pipe: {e}");
        }
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
        if before.vault.browser_autofill != after.vault.browser_autofill {
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
