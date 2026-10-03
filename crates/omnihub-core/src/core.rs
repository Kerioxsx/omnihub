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
}

impl Default for CoreOptions {
    fn default() -> Self {
        CoreOptions { dry_run_power: false, runner: Arc::new(SelfElevated), vault_kdf: None, vault_dpapi: None }
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
        if s.remote.enabled {
            if let Err(e) = self.remote.start(self.clone()) {
                tracing::error!("companion server failed to start: {e}");
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
        let r0 = &before.remote;
        let r1 = &after.remote;
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
