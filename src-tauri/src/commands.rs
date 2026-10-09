//! Tauri commands: thin wrappers that hand UI requests to `AppCore`.
//!
//! Every command is `async` so it runs on the async runtime rather than the
//! UI thread; anything that may take a while (disk, PowerShell, hashing,
//! Argon2) runs on the blocking pool via `blocking`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use omnihub_core::apps::AppInfo;
use omnihub_core::audit::AuditEntry;
use omnihub_core::browser::register::Registration;
use omnihub_core::browser::{BrowserClient, PairRequest};
use omnihub_core::capture::airplay::AirPlayStatus;
use omnihub_core::capture::bridges::{ScrcpyOptions, ScrcpyStatus, SunshineStatus};
use omnihub_core::capture::screenshots::{CaptureKind, PendingRegion, Rect, Screenshot, ShotFilter};
use omnihub_core::capture::stream::{self, MonitorInfo, Preset};
use omnihub_core::core::AppCore;
use omnihub_core::notes::{FolderFile, Note, NoteFilter, NoteInput};
use omnihub_core::remote::auth::{Device, PairingInfo};
use omnihub_core::remote::net::LanAddress;
use omnihub_core::remote::transfer::InboxItem;
use omnihub_core::remote::{ServerStatus, Visit};
use omnihub_core::system::firewall::{self, FirewallReport, Proto};
use omnihub_core::settings::Settings;
use omnihub_core::storage::cleanup::{DeleteResult, Suggestion};
use omnihub_core::storage::dupes::{DupeGroup, DupeOptions};
use omnihub_core::storage::engine::{ChildrenPage, DupeJobProgress, JobProgress, PathedNode, ScanRequest, ScanSummary, SearchResult};
use omnihub_core::storage::tree::{ExtensionStat, SearchQuery, SortKey, TreemapItem};
use omnihub_core::storage::volumes::VolumeInfo;
use omnihub_core::system::power::{PendingPower, PowerAction};
use omnihub_core::vault::generator::{self, GeneratorOptions, Strength};
use omnihub_core::vault::{Entry, EntryInput, EntrySummary, VaultStatus};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

type Core<'a> = State<'a, Arc<AppCore>>;
type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Run blocking work off the async runtime.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Res<T> + Send + 'static) -> Res<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(err)?
}

// ---------- app ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfoDetails {
    version: String,
    elevated: bool,
    platform: String,
    hostname: String,
    dpapi: bool,
    data_dir: String,
    config_dir: String,
    cache_dir: String,
    screenshot_dir: String,
    incoming_dir: String,
    /// First name of the signed-in user, for greetings.
    user_name: String,
}

#[tauri::command]
pub async fn app_info(core: Core<'_>) -> Res<AppInfoDetails> {
    Ok(AppInfoDetails {
        version: env!("CARGO_PKG_VERSION").into(),
        elevated: omnihub_core::system::elevation::is_elevated(),
        platform: std::env::consts::OS.into(),
        hostname: sysinfo::System::host_name().unwrap_or_default(),
        dpapi: omnihub_core::system::dpapi::available(),
        data_dir: core.paths.data.to_string_lossy().into(),
        config_dir: core.paths.config.to_string_lossy().into(),
        cache_dir: core.paths.cache.to_string_lossy().into(),
        screenshot_dir: core.screenshot_dir().to_string_lossy().into(),
        incoming_dir: core.incoming_dir().to_string_lossy().into(),
        user_name: omnihub_core::system::user_display_name(),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    used: u64,
    total: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    cpu: f32,
    memory: Memory,
    uptime: u64,
    os: String,
}

static SYS: std::sync::LazyLock<parking_lot::Mutex<sysinfo::System>> = std::sync::LazyLock::new(|| parking_lot::Mutex::new(sysinfo::System::new()));

#[tauri::command]
pub async fn system_stats() -> Res<SystemStats> {
    let mut sys = SYS.lock();
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    Ok(SystemStats {
        cpu: sys.global_cpu_usage(),
        memory: Memory { used: sys.used_memory(), total: sys.total_memory() },
        uptime: sysinfo::System::uptime(),
        os: sysinfo::System::long_os_version().unwrap_or_default(),
    })
}

#[tauri::command]
pub async fn system_processes(core: Core<'_>, sort: omnihub_core::system::procs::ProcessSort, limit: Option<usize>) -> Res<Vec<omnihub_core::system::procs::ProcessGroup>> {
    let core = core.inner().clone();
    blocking(move || Ok(core.procs.top(sort, limit.unwrap_or(8)))).await
}

/// The whole PC (CPU, memory, GPUs) and its busiest programs.
#[tauri::command]
pub async fn system_usage(core: Core<'_>, sort: omnihub_core::system::procs::ProcessSort, limit: Option<usize>) -> Res<omnihub_core::system::procs::Usage> {
    let core = core.inner().clone();
    blocking(move || Ok(core.procs.usage(sort, limit.unwrap_or(200), true))).await
}

#[tauri::command]
pub async fn system_set_priority(core: Core<'_>, name: String, priority: omnihub_core::system::procs::Priority) -> Res<usize> {
    let core = core.inner().clone();
    blocking(move || {
        let r = core.procs.set_priority(&name, priority);
        core.audit.record("desktop", "process.priority", &format!("{name} → {priority:?}"), r.is_ok());
        r
    })
    .await
}

/// End every process with this name ("End task").
#[tauri::command]
pub async fn system_end_process(core: Core<'_>, name: String) -> Res<usize> {
    let core = core.inner().clone();
    blocking(move || {
        let r = core.procs.end(&name);
        core.audit.record("desktop", "process.end", &name, r.is_ok());
        r
    })
    .await
}

#[tauri::command]
pub async fn vault_health(core: Core<'_>) -> Res<omnihub_core::vault::health::HealthReport> {
    let core = core.inner().clone();
    blocking(move || core.vault.health().map_err(err)).await
}

/// Check the vault's passwords against Have I Been Pwned (k-anonymity:
/// only 5 characters of each SHA-1 hash are sent).
#[tauri::command]
pub async fn vault_breach_check(core: Core<'_>) -> Res<Vec<omnihub_core::vault::health::HealthItem>> {
    let core = core.inner().clone();
    blocking(move || {
        let r = core.vault.breach_check(&omnihub_core::vault::health::fetch_range);
        core.audit.record("desktop", "vault.breach-check", &r.as_ref().map(|v| format!("{} found", v.len())).unwrap_or_default(), r.is_ok());
        r.map_err(err)
    })
    .await
}

/// Set (Unix seconds) or clear (null) a note's reminder.
#[tauri::command]
pub async fn notes_remind(core: Core<'_>, id: String, at: Option<i64>) -> Res<Note> {
    let core = core.inner().clone();
    blocking(move || core.notes.set_reminder(&id, at).map_err(err)).await
}

#[tauri::command]
pub async fn startup_list() -> Res<Vec<omnihub_core::startup::StartupItem>> {
    blocking(|| Ok(omnihub_core::startup::list())).await
}

/// Switch a startup entry on or off (all-users entries ask for approval).
#[tauri::command]
pub async fn startup_set(core: Core<'_>, id: String, enabled: bool) -> Res<Vec<omnihub_core::startup::StartupItem>> {
    let core = core.inner().clone();
    blocking(move || {
        let r = omnihub_core::startup::set_enabled(&id, enabled);
        core.audit.record("desktop", if enabled { "startup.enable" } else { "startup.disable" }, id.split_once(':').map_or(id.as_str(), |(_, n)| n), r.is_ok());
        r.map_err(err)?;
        Ok(omnihub_core::startup::list())
    })
    .await
}

#[tauri::command]
pub async fn settings_get(core: Core<'_>) -> Res<Settings> {
    Ok(core.settings.get())
}

#[tauri::command]
pub async fn settings_update(app: AppHandle, core: Core<'_>, patch: serde_json::Value) -> Res<Settings> {
    let core = core.inner().clone();
    let before = core.settings.get();
    let after = blocking(move || core.update_settings(&patch).map_err(err)).await?;
    if before.screenshots != after.screenshots {
        crate::shortcuts::register(&app);
    }
    crate::tray::refresh(&app);
    Ok(after)
}

/// Save all settings as JSON (no secrets are kept in settings).
#[tauri::command]
pub async fn settings_export(core: Core<'_>, path: String) -> Res<()> {
    let s = core.settings.get();
    let json = serde_json::to_vec_pretty(&serde_json::json!({ "omnihubSettings": 1, "version": env!("CARGO_PKG_VERSION"), "settings": s })).map_err(err)?;
    omnihub_core::settings::write_atomic(Path::new(&path), &json).map_err(err)
}

/// Restore settings saved with [`settings_export`]; unknown or missing
/// values keep their current setting.
#[tauri::command]
pub async fn settings_import(app: AppHandle, core: Core<'_>, path: String) -> Res<Settings> {
    let core = core.inner().clone();
    let after = blocking(move || {
        let bytes = std::fs::read(&path).map_err(err)?;
        let v: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| "That file is not an OmniHub settings backup.".to_string())?;
        let s = v.get("settings").filter(|_| v.get("omnihubSettings").is_some()).ok_or("That file is not an OmniHub settings backup.")?;
        // Validate by reading it as settings, then apply as a patch (side effects run).
        let parsed: Settings = serde_json::from_value(s.clone()).map_err(|e| format!("The backup could not be read: {e}"))?;
        core.update_settings(&serde_json::to_value(parsed).map_err(err)?).map_err(err)
    })
    .await?;
    crate::shortcuts::register(&app);
    crate::tray::refresh(&app);
    Ok(after)
}

#[tauri::command]
pub async fn open_path(path: String) -> Res<()> {
    omnihub_core::system::shell::open_path(Path::new(&path)).map_err(err)
}

#[tauri::command]
pub async fn reveal_path(path: String) -> Res<()> {
    omnihub_core::system::shell::reveal(Path::new(&path)).map_err(err)
}

#[tauri::command]
pub async fn open_url(url: String) -> Res<()> {
    omnihub_core::system::shell::open_url(&url).map_err(err)
}

#[tauri::command]
pub async fn restart_elevated(app: AppHandle) -> Res<()> {
    let pid = std::process::id().to_string();
    omnihub_core::system::elevation::relaunch_elevated(&["--replace".into(), pid]).map_err(err)?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub async fn audit_list(core: Core<'_>, limit: usize, offset: usize) -> Res<Vec<AuditEntry>> {
    Ok(core.audit.list(limit.min(1000), offset))
}

#[tauri::command]
pub async fn audit_clear(core: Core<'_>) -> Res<()> {
    core.audit.clear();
    Ok(())
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle, title: Option<String>) -> Res<Option<String>> {
    blocking(move || {
        let mut d = app.dialog().file();
        if let Some(t) = title {
            d = d.set_title(t);
        }
        Ok(d.blocking_pick_folder().and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().to_string()))
    })
    .await
}

#[tauri::command]
pub async fn pick_files(app: AppHandle, title: Option<String>) -> Res<Vec<String>> {
    blocking(move || {
        let mut d = app.dialog().file();
        if let Some(t) = title {
            d = d.set_title(t);
        }
        Ok(d.blocking_pick_files()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|p| p.into_path().ok())
            .map(|p| p.to_string_lossy().to_string())
            .collect())
    })
    .await
}

#[tauri::command]
pub async fn pick_save_file(app: AppHandle, title: String, default_name: String) -> Res<Option<String>> {
    blocking(move || {
        Ok(app
            .dialog()
            .file()
            .set_title(title)
            .set_file_name(default_name)
            .blocking_save_file()
            .and_then(|p| p.into_path().ok())
            .map(|p| p.to_string_lossy().to_string()))
    })
    .await
}

#[tauri::command]
pub async fn set_autostart(app: AppHandle, core: Core<'_>, enabled: bool) -> Res<()> {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    if enabled { launcher.enable() } else { launcher.disable() }.map_err(err)?;
    let core = core.inner().clone();
    blocking(move || core.update_settings(&serde_json::json!({ "general": { "launchAtLogin": enabled } })).map(|_| ()).map_err(err)).await
}

#[tauri::command]
pub async fn hide_window(app: AppHandle) -> Res<()> {
    if let Some(w) = app.get_webview_window("main") {
        w.hide().map_err(err)?;
    }
    Ok(())
}

// ---------- storage ----------

#[tauri::command]
pub async fn storage_volumes(core: Core<'_>) -> Res<Vec<VolumeInfo>> {
    let core = core.inner().clone();
    blocking(move || Ok(core.storage.volumes())).await
}

#[tauri::command]
pub async fn storage_scan(core: Core<'_>, mut request: ScanRequest) -> Res<String> {
    if request.exclude.is_empty() {
        request.exclude = core.settings.get().storage.exclude;
    }
    Ok(core.storage.start_scan(request))
}

#[tauri::command]
pub async fn storage_progress(core: Core<'_>, job_id: String) -> Res<Option<JobProgress>> {
    Ok(core.storage.progress(&job_id))
}

#[tauri::command]
pub async fn storage_cancel(core: Core<'_>, job_id: String) -> Res<()> {
    core.storage.cancel(&job_id);
    Ok(())
}

#[tauri::command]
pub async fn storage_scans(core: Core<'_>) -> Res<Vec<ScanSummary>> {
    Ok(core.storage.scans())
}

#[tauri::command]
pub async fn storage_summary(core: Core<'_>, scan_id: String) -> Res<ScanSummary> {
    core.storage.summary(&scan_id).map_err(err)
}

#[tauri::command]
pub async fn storage_open_cached(core: Core<'_>, root: String) -> Res<Option<ScanSummary>> {
    let core = core.inner().clone();
    blocking(move || Ok(core.storage.open_cached(&root))).await
}

#[tauri::command]
pub async fn storage_children(core: Core<'_>, scan_id: String, node: u32, sort: SortKey, descending: bool, offset: usize, limit: usize) -> Res<ChildrenPage> {
    let include_hidden = core.settings.get().storage.show_hidden;
    core.storage.children(&scan_id, node, sort, descending, offset, limit.min(5000), include_hidden).map_err(err)
}

#[tauri::command]
pub async fn storage_treemap(core: Core<'_>, scan_id: String, node: u32, depth: u32, max_items: usize) -> Res<TreemapItem> {
    core.storage.treemap(&scan_id, node, depth, max_items).map_err(err)
}

#[tauri::command]
pub async fn storage_growth(core: Core<'_>, scan_id: String, limit: Option<usize>) -> Res<omnihub_core::storage::growth::GrowthReport> {
    let core = core.inner().clone();
    blocking(move || core.storage.growth(&scan_id, limit.unwrap_or(15)).map_err(err)).await
}

/// `kind`: "children" (the folder's contents) or "largest" (its largest files).
#[tauri::command]
pub async fn storage_export_csv(core: Core<'_>, scan_id: String, node: u32, kind: String, path: String) -> Res<usize> {
    let core = core.inner().clone();
    blocking(move || {
        let hidden = core.settings.get().storage.show_hidden;
        core.storage.export_csv(&scan_id, node, &kind, Path::new(&path), hidden).map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn storage_top_files(core: Core<'_>, scan_id: String, node: u32, n: usize) -> Res<Vec<PathedNode>> {
    let core = core.inner().clone();
    blocking(move || {
        let mut files = core.storage.top_files(&scan_id, node, n).map_err(err)?;
        if !core.settings.get().storage.show_hidden {
            files.retain(|f| !f.node.hidden && !f.node.system);
        }
        Ok(files)
    })
    .await
}

#[tauri::command]
pub async fn storage_extensions(core: Core<'_>, scan_id: String, node: u32) -> Res<Vec<ExtensionStat>> {
    let core = core.inner().clone();
    blocking(move || core.storage.extensions(&scan_id, node).map_err(err)).await
}

#[tauri::command]
pub async fn storage_search(core: Core<'_>, scan_id: String, mut query: SearchQuery) -> Res<SearchResult> {
    let core = core.inner().clone();
    query.exclude_hidden |= !core.settings.get().storage.show_hidden;
    blocking(move || core.storage.search(&scan_id, &query).map_err(err)).await
}

#[tauri::command]
pub async fn storage_path(core: Core<'_>, scan_id: String, node: u32) -> Res<String> {
    core.storage.path_of(&scan_id, node).map_err(err)
}

#[tauri::command]
pub async fn storage_cleanup(core: Core<'_>, scan_id: String) -> Res<Vec<Suggestion>> {
    let core = core.inner().clone();
    blocking(move || {
        let opts = core.settings.get().storage.cleanup;
        core.storage.cleanup_suggestions(&scan_id, &opts).map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn storage_delete(core: Core<'_>, paths: Vec<String>, permanent: bool) -> Res<Vec<DeleteResult>> {
    let core = core.inner().clone();
    blocking(move || {
        let results = core.storage.delete(&paths, permanent);
        let ok = results.iter().filter(|r| r.ok).count();
        core.audit.record("desktop", if permanent { "storage.delete-permanent" } else { "storage.recycle" }, &format!("{ok} of {} item(s)", results.len()), ok == results.len());
        Ok(results)
    })
    .await
}

#[tauri::command]
pub async fn storage_empty_recycle_bin(core: Core<'_>) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || {
        core.storage.empty_recycle_bin()?;
        core.audit.record("desktop", "storage.empty-recycle-bin", "", true);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn storage_duplicates_start(core: Core<'_>, scan_id: String, options: DupeOptions) -> Res<String> {
    core.storage.start_duplicates(&scan_id, options).map_err(err)
}

#[tauri::command]
pub async fn storage_duplicates_progress(core: Core<'_>, job_id: String) -> Res<Option<DupeJobProgress>> {
    Ok(core.storage.duplicates_progress(&job_id))
}

#[tauri::command]
pub async fn storage_duplicates_result(core: Core<'_>, job_id: String) -> Res<Option<Vec<DupeGroup>>> {
    Ok(core.storage.duplicates_result(&job_id))
}

#[tauri::command]
pub async fn storage_duplicates_cancel(core: Core<'_>, job_id: String) -> Res<()> {
    core.storage.cancel_duplicates(&job_id);
    Ok(())
}

// ---------- apps ----------

#[tauri::command]
pub async fn apps_list(core: Core<'_>, refresh: bool) -> Res<Vec<AppInfo>> {
    let core = core.inner().clone();
    blocking(move || {
        if refresh {
            core.apps.refresh();
        }
        Ok(core.apps.list(&|p| core.storage.size_of_path(p).map(|s| s.0)))
    })
    .await
}

#[tauri::command]
pub async fn apps_icon(core: Core<'_>, id: String) -> Res<Option<String>> {
    let core = core.inner().clone();
    blocking(move || Ok(core.apps.icon_data_url(&id))).await
}

#[tauri::command]
pub async fn apps_launch(core: Core<'_>, id: String) -> Res<()> {
    let name = core.apps.launch(&id).map_err(err)?;
    core.audit.record("desktop", "apps.launch", &name, true);
    Ok(())
}

#[tauri::command]
pub async fn apps_uninstall(core: Core<'_>, id: String) -> Res<()> {
    let name = core.apps.uninstall(&id).map_err(err)?;
    core.audit.record("desktop", "apps.uninstall", &name, true);
    Ok(())
}

#[tauri::command]
pub async fn apps_screenshots(core: Core<'_>, id: String) -> Res<Vec<Screenshot>> {
    let app = core.apps.get(&id).ok_or("unknown app")?;
    core.screenshots.for_app(&[], app.install_location.as_deref()).map_err(err)
}

// ---------- screenshots ----------

#[tauri::command]
pub async fn shots_list(core: Core<'_>, filter: ShotFilter) -> Res<Vec<Screenshot>> {
    core.screenshots.list(&filter).map_err(err)
}

#[tauri::command]
pub async fn shots_capture(app: AppHandle, core: Core<'_>, kind: CaptureKind, delay_seconds: u32) -> Res<Screenshot> {
    let core = core.inner().clone();
    // Get our own window out of the picture first.
    let main = app.get_webview_window("main");
    let was_visible = main.as_ref().and_then(|w| w.is_visible().ok()).unwrap_or(false);
    if let Some(w) = &main {
        let _ = w.hide();
    }
    tokio::time::sleep(Duration::from_millis(250 + delay_seconds.min(30) as u64 * 1000)).await;
    let shot = blocking(move || {
        let s = core.settings.get().screenshots;
        let dir = core.screenshot_dir();
        let shot = core.screenshots.capture(kind, &dir, s.format).map_err(err)?;
        if s.copy_to_clipboard {
            let _ = core.screenshots.copy_to_clipboard(&shot.id);
        }
        Ok(shot)
    })
    .await;
    if was_visible {
        if let Some(w) = &main {
            let _ = w.show();
        }
    }
    shot
}

#[tauri::command]
pub async fn shots_region_begin(app: AppHandle) -> Res<()> {
    crate::overlay::begin(app, true).await
}

#[tauri::command]
pub async fn shots_region_pending(core: Core<'_>) -> Res<Option<PendingRegion>> {
    Ok(crate::overlay::pending(&core))
}

#[tauri::command]
pub async fn shots_region_commit(app: AppHandle, core: Core<'_>, id: String, rect: Rect) -> Res<Screenshot> {
    let core2 = core.inner().clone();
    let shot = blocking(move || {
        let s = core2.settings.get().screenshots;
        let dir = core2.screenshot_dir();
        let shot = core2.screenshots.commit_region(&id, rect, &dir, s.format).map_err(err)?;
        if s.copy_to_clipboard {
            let _ = core2.screenshots.copy_to_clipboard(&shot.id);
        }
        Ok(shot)
    })
    .await;
    crate::overlay::finish(&app);
    shot
}

#[tauri::command]
pub async fn shots_region_cancel(app: AppHandle, core: Core<'_>) -> Res<()> {
    core.screenshots.cancel_region();
    crate::overlay::finish(&app);
    Ok(())
}

#[tauri::command]
pub async fn shots_thumb(core: Core<'_>, id: String) -> Res<String> {
    let core = core.inner().clone();
    blocking(move || core.screenshots.thumbnail(&id).map_err(err)).await
}

#[tauri::command]
pub async fn shots_image(core: Core<'_>, id: String) -> Res<String> {
    let core = core.inner().clone();
    blocking(move || core.screenshots.full_image(&id).map_err(err)).await
}

#[tauri::command]
pub async fn shots_update(core: Core<'_>, id: String, tags: Option<Vec<String>>, note: Option<String>, favorite: Option<bool>) -> Res<Screenshot> {
    core.screenshots.update(&id, tags, note, favorite).map_err(err)
}

#[tauri::command]
pub async fn shots_delete(core: Core<'_>, id: String, trash: bool) -> Res<()> {
    core.screenshots.delete(&id, trash).map_err(err)
}

#[tauri::command]
pub async fn shots_copy(core: Core<'_>, id: String) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || core.screenshots.copy_to_clipboard(&id).map_err(err)).await
}

/// The text in a screenshot (Windows OCR; kept for search).
#[tauri::command]
pub async fn shots_text(core: Core<'_>, id: String) -> Res<String> {
    let core = core.inner().clone();
    blocking(move || core.screenshots.text(&id).map_err(err)).await
}

/// Save the markup editor's result (a PNG data URL) as a new screenshot.
#[tauri::command]
pub async fn shots_save_edit(core: Core<'_>, id: String, png: String) -> Res<Screenshot> {
    let core = core.inner().clone();
    blocking(move || {
        let shot = core.screenshots.store_edited_data_url(&id, &png, &core.screenshot_dir()).map_err(err)?;
        core.audit.record("desktop", "screenshot.edit", &shot.path, true);
        Ok(shot)
    })
    .await
}

#[tauri::command]
pub async fn shots_sync(core: Core<'_>) -> Res<usize> {
    let core = core.inner().clone();
    blocking(move || core.screenshots.sync_folder(&core.screenshot_dir()).map_err(err)).await
}

// ---------- notes ----------

#[tauri::command]
pub async fn notes_list(core: Core<'_>, filter: NoteFilter) -> Res<Vec<Note>> {
    core.notes.list(&filter).map_err(err)
}

#[tauri::command]
pub async fn notes_get(core: Core<'_>, id: String) -> Res<Note> {
    core.notes.get(&id).map_err(err)
}

#[tauri::command]
pub async fn notes_save(core: Core<'_>, input: NoteInput) -> Res<Note> {
    let note = core.notes.save(input).map_err(err)?;
    let s = core.settings.get().notes;
    if note.kind == omnihub_core::notes::NoteKind::Idea && s.auto_export_ideas {
        if let Some(folder) = s.claude_folder {
            return core.notes.export(&note.id, Path::new(&folder), &core.export_options()).map_err(err);
        }
    }
    Ok(note)
}

#[tauri::command]
pub async fn notes_delete(core: Core<'_>, id: String) -> Res<()> {
    core.notes.delete(&id).map_err(err)
}

#[tauri::command]
pub async fn notes_tags(core: Core<'_>) -> Res<Vec<(String, usize)>> {
    core.notes.tags().map_err(err)
}

fn claude_folder(core: &AppCore) -> Res<PathBuf> {
    core.settings.get().notes.claude_folder.map(PathBuf::from).ok_or_else(|| "Choose a Claude folder in Settings → Notes first".to_string())
}

#[tauri::command]
pub async fn notes_export(core: Core<'_>, id: String) -> Res<Note> {
    let folder = claude_folder(&core)?;
    let note = core.notes.export(&id, &folder, &core.export_options()).map_err(err)?;
    core.audit.record("desktop", "notes.export", &note.title, true);
    Ok(note)
}

#[tauri::command]
pub async fn notes_folder_files(core: Core<'_>) -> Res<Vec<FolderFile>> {
    let folder = claude_folder(&core)?;
    std::fs::create_dir_all(&folder).map_err(err)?;
    core.notes.folder_files(&folder).map_err(err)
}

#[tauri::command]
pub async fn notes_folder_read(core: Core<'_>, path: String) -> Res<String> {
    let folder = claude_folder(&core)?;
    core.notes.read_folder_file(&folder, Path::new(&path)).map_err(err)
}

// ---------- vault ----------

#[tauri::command]
pub async fn vault_status(core: Core<'_>) -> Res<VaultStatus> {
    Ok(core.vault.status())
}

#[tauri::command]
pub async fn vault_create(core: Core<'_>, password: String) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || {
        core.vault.create(&password).map_err(err)?;
        core.audit.record("desktop", "vault.create", "", true);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn vault_unlock(core: Core<'_>, password: String) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || {
        let r = core.vault.unlock(&password);
        core.audit.record("desktop", "vault.unlock", r.as_ref().err().map(|e| e.to_string()).as_deref().unwrap_or(""), r.is_ok());
        r.map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn vault_lock(core: Core<'_>) -> Res<()> {
    core.vault.lock();
    Ok(())
}

#[tauri::command]
pub async fn vault_touch(core: Core<'_>) -> Res<()> {
    core.vault.touch();
    Ok(())
}

#[tauri::command]
pub async fn vault_list(core: Core<'_>) -> Res<Vec<EntrySummary>> {
    core.vault.list().map_err(err)
}

#[tauri::command]
pub async fn vault_get(core: Core<'_>, id: String) -> Res<Entry> {
    core.vault.get(&id).map_err(err)
}

#[tauri::command]
pub async fn vault_save(core: Core<'_>, input: EntryInput) -> Res<EntrySummary> {
    core.vault.save(input).map_err(err)
}

#[tauri::command]
pub async fn vault_delete(core: Core<'_>, id: String) -> Res<()> {
    core.vault.delete(&id).map_err(err)
}

#[tauri::command]
pub async fn vault_copy(core: Core<'_>, id: String, field: String) -> Res<()> {
    let secs = core.settings.get().vault.clipboard_clear_seconds;
    core.vault.copy_field(&id, &field, Duration::from_secs(secs as u64)).map_err(err)?;
    if field == "password" {
        core.audit.record("desktop", "vault.copy", "password", true);
    }
    Ok(())
}

#[tauri::command]
pub async fn vault_generate(options: GeneratorOptions) -> Res<String> {
    Ok(generator::generate(&options))
}

#[tauri::command]
pub async fn vault_strength(password: String) -> Res<Strength> {
    Ok(generator::strength(&password))
}

#[tauri::command]
pub async fn vault_change_password(core: Core<'_>, old_password: String, new_password: String) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || {
        core.vault.change_password(&old_password, &new_password).map_err(err)?;
        core.audit.record("desktop", "vault.change-password", "", true);
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn vault_hello_enable(core: Core<'_>) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || {
        core.vault.enable_hello().map_err(err)?;
        core.update_settings(&serde_json::json!({ "vault": { "helloEnabled": true } })).map_err(err)?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn vault_hello_disable(core: Core<'_>) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || {
        core.vault.disable_hello();
        core.update_settings(&serde_json::json!({ "vault": { "helloEnabled": false } })).map_err(err)?;
        Ok(())
    })
    .await
}

#[tauri::command]
pub async fn vault_hello_unlock(core: Core<'_>) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || {
        let r = core.vault.unlock_with_hello();
        core.audit.record("desktop", "vault.unlock-hello", "", r.is_ok());
        r.map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn vault_export(core: Core<'_>, path: String) -> Res<()> {
    core.vault.export_backup(Path::new(&path)).map_err(err)?;
    core.audit.record("desktop", "vault.export", &path, true);
    Ok(())
}

#[tauri::command]
pub async fn vault_import(core: Core<'_>, path: String, password: String) -> Res<usize> {
    let core = core.inner().clone();
    blocking(move || core.vault.import_backup(Path::new(&path), &password).map_err(err)).await
}

// ---------- phone companion & power ----------

#[tauri::command]
pub async fn remote_status(core: Core<'_>) -> Res<ServerStatus> {
    Ok(core.remote.status())
}

#[tauri::command]
pub async fn remote_start(app: AppHandle, core: Core<'_>) -> Res<ServerStatus> {
    let core = core.inner().clone();
    let status = blocking(move || {
        core.update_settings(&serde_json::json!({ "remote": { "enabled": true } })).map_err(err)?;
        if !core.remote.is_running() {
            core.remote.start(core.clone()).map_err(err)?;
        }
        Ok(core.remote.status())
    })
    .await?;
    crate::tray::refresh(&app);
    Ok(status)
}

#[tauri::command]
pub async fn remote_stop(app: AppHandle, core: Core<'_>) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || {
        core.update_settings(&serde_json::json!({ "remote": { "enabled": false } })).map_err(err)?;
        core.remote.stop();
        Ok(())
    })
    .await?;
    crate::tray::refresh(&app);
    Ok(())
}

#[tauri::command]
pub async fn remote_pair_begin(core: Core<'_>) -> Res<PairingInfo> {
    let info = core.remote.begin_pairing().map_err(err)?;
    core.audit.record("desktop", "remote.pairing-open", "", true);
    Ok(info)
}

#[tauri::command]
pub async fn remote_pair_cancel(core: Core<'_>) -> Res<()> {
    core.remote.cancel_pairing();
    Ok(())
}

#[tauri::command]
pub async fn remote_devices(core: Core<'_>) -> Res<Vec<Device>> {
    Ok(core.remote.devices.list())
}

#[tauri::command]
pub async fn remote_device_rename(core: Core<'_>, id: String, name: String) -> Res<()> {
    if !core.remote.devices.rename(&id, &name) {
        return Err("could not rename the device".into());
    }
    core.events.emit("remote:devices", core.remote.devices.list());
    Ok(())
}

#[tauri::command]
pub async fn remote_device_revoke(core: Core<'_>, id: String) -> Res<()> {
    core.remote.revoke_device(&id);
    core.audit.record("desktop", "remote.revoke", &id, true);
    Ok(())
}

#[tauri::command]
pub async fn remote_send(core: Core<'_>, paths: Vec<String>, device_id: Option<String>) -> Res<Vec<InboxItem>> {
    let core = core.inner().clone();
    blocking(move || {
        // Folders are zipped first, which can take a moment.
        let items = core.remote.send_to_phone(&paths, device_id, &core.outbox_dir()).map_err(err)?;
        core.audit.record("desktop", "files.offer", &format!("{} item(s)", items.len()), true);
        Ok(items)
    })
    .await
}

#[tauri::command]
pub async fn remote_send_text(core: Core<'_>, text: String, device_id: Option<String>) -> Res<InboxItem> {
    let item = core.remote.send_text(&text, device_id).map_err(err)?;
    core.audit.record("desktop", "text.offer", &format!("{} characters", text.chars().count()), true);
    Ok(item)
}

#[tauri::command]
pub async fn remote_inbox_remove(core: Core<'_>, id: String) -> Res<()> {
    core.remote.unsend(&id);
    Ok(())
}

/// A small preview of an image file as a data URL (`None` for other files).
#[tauri::command]
pub async fn storage_thumb(core: Core<'_>, path: String, size: u32) -> Res<Option<String>> {
    let core = core.inner().clone();
    blocking(move || {
        let p = Path::new(&path);
        if !omnihub_core::thumbs::is_previewable(p) {
            return Ok(None);
        }
        Ok(core.thumbs.data_url(p, size).ok())
    })
    .await
}

/// Text currently on the PC clipboard (to send to a phone).
#[tauri::command]
pub async fn clipboard_text() -> Res<Option<String>> {
    blocking(|| Ok(omnihub_core::system::clipboard::get_text())).await
}

/// Paths from Explorer's "Send to → OmniHub (phone)" waiting for the UI.
#[derive(Default)]
pub struct PendingSend(pub parking_lot::Mutex<Vec<String>>);

/// The report about the previous run, when it did not end normally (shown once).
pub struct LastCrash(pub parking_lot::Mutex<Option<String>>);

#[tauri::command]
pub fn app_last_crash(state: tauri::State<'_, LastCrash>) -> Option<String> {
    state.0.lock().take()
}

/// Errors the window could not recover from, for the log file.
#[tauri::command]
pub fn app_log_error(message: String) {
    let m: String = message.chars().take(4000).collect();
    tracing::error!("window: {m}");
}

#[tauri::command]
pub async fn take_pending_send(pending: State<'_, PendingSend>) -> Res<Vec<String>> {
    Ok(std::mem::take(&mut *pending.0.lock()))
}

#[tauri::command]
pub async fn remote_inbox(core: Core<'_>) -> Res<Vec<InboxItem>> {
    Ok(core.remote.inbox.all())
}

#[tauri::command]
pub async fn remote_stop_viewer(core: Core<'_>, id: String) -> Res<()> {
    core.remote.stop_viewer(&id);
    Ok(())
}

#[tauri::command]
pub async fn remote_stop_all_viewers(core: Core<'_>) -> Res<()> {
    core.remote.stop_all_viewers();
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteDiagnostics {
    running: bool,
    port: u16,
    tls: bool,
    addresses: Vec<LanAddress>,
    firewall: FirewallReport,
    visitors: Vec<Visit>,
    /// This PC connecting to itself on each address (empty when off).
    self_test: Vec<omnihub_core::remote::net::SelfTest>,
}

/// Why a phone might not reach the companion: addresses, Windows Firewall
/// and network type, and which devices did get through.
#[tauri::command]
pub async fn remote_diagnostics(core: Core<'_>) -> Res<RemoteDiagnostics> {
    let core = core.inner().clone();
    blocking(move || {
        let status = core.remote.status();
        let port = if status.running { status.port } else { core.settings.get().remote.port };
        let exe = std::env::current_exe().map_err(err)?;
        let addresses = omnihub_core::remote::net::lan_addresses();
        let self_test = if status.running && status.bind == omnihub_core::settings::Bind::Lan { omnihub_core::remote::net::self_test(&addresses, port) } else { Vec::new() };
        Ok(RemoteDiagnostics {
            running: status.running,
            port,
            tls: status.tls,
            addresses,
            firewall: firewall::check(&exe, Some(port), Proto::Tcp, "OmniHub"),
            visitors: core.remote.visitors(),
            self_test,
        })
    })
    .await
}

/// Allow OmniHub through Windows Firewall (asks for administrator approval).
#[tauri::command]
pub async fn remote_fix_firewall(core: Core<'_>, include_public: bool) -> Res<FirewallReport> {
    let core = core.inner().clone();
    blocking(move || {
        let exe = std::env::current_exe().map_err(err)?;
        let mut mask = firewall::PROFILE_PRIVATE | firewall::PROFILE_DOMAIN;
        if include_public {
            mask |= firewall::PROFILE_PUBLIC;
        }
        let res = firewall::allow_program(&exe, "OmniHub phone companion", mask);
        core.audit.record("desktop", "firewall.allow", &firewall::profile_names(mask).join(", "), res.is_ok());
        res.map_err(err)?;
        let port = core.settings.get().remote.port;
        Ok(firewall::check(&exe, Some(port), Proto::Tcp, "OmniHub"))
    })
    .await
}

/// Mark a connected network as Private (asks for administrator approval).
#[tauri::command]
pub async fn network_make_private(core: Core<'_>, id: String) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || {
        let res = firewall::make_network_private(&id);
        core.audit.record("desktop", "network.private", &id, res.is_ok());
        res.map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn power_schedule(core: Core<'_>, action: PowerAction, delay_seconds: u32) -> Res<PendingPower> {
    Ok(core.power.schedule(action, delay_seconds.min(24 * 3600), "desktop"))
}

#[tauri::command]
pub async fn power_cancel(core: Core<'_>) -> Res<bool> {
    Ok(core.power.cancel("desktop"))
}

#[tauri::command]
pub async fn power_pending(core: Core<'_>) -> Res<Option<PendingPower>> {
    Ok(core.power.pending())
}

// ---------- screen sharing ----------

#[tauri::command]
pub async fn screen_monitors() -> Res<Vec<MonitorInfo>> {
    blocking(|| Ok(stream::monitors())).await
}

#[tauri::command]
pub async fn screen_presets() -> Res<Vec<Preset>> {
    Ok(stream::presets())
}

#[tauri::command]
pub async fn scrcpy_status(core: Core<'_>) -> Res<ScrcpyStatus> {
    let core = core.inner().clone();
    blocking(move || {
        let path = core.settings.get().screen.scrcpy_path;
        Ok(core.bridges.scrcpy_status(path.as_deref()))
    })
    .await
}

#[tauri::command]
pub async fn scrcpy_launch(core: Core<'_>, options: ScrcpyOptions) -> Res<()> {
    let path = core.settings.get().screen.scrcpy_path;
    core.bridges.launch_scrcpy(path.as_deref(), &options).map_err(err)?;
    core.audit.record("desktop", "screen.scrcpy", options.serial.as_deref().unwrap_or("default device"), true);
    Ok(())
}

#[tauri::command]
pub async fn scrcpy_stop(core: Core<'_>) -> Res<()> {
    core.bridges.stop_scrcpy();
    Ok(())
}

#[tauri::command]
pub async fn scrcpy_wireless(core: Core<'_>, serial: String) -> Res<String> {
    let core = core.inner().clone();
    blocking(move || {
        let path = core.settings.get().screen.scrcpy_path;
        core.bridges.enable_wireless(path.as_deref(), &serial).map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn scrcpy_connect(core: Core<'_>, addr: String) -> Res<String> {
    let core = core.inner().clone();
    blocking(move || {
        let path = core.settings.get().screen.scrcpy_path;
        core.bridges.connect(path.as_deref(), &addr).map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn scrcpy_pair(core: Core<'_>, addr: String, code: String) -> Res<String> {
    let core = core.inner().clone();
    blocking(move || {
        let path = core.settings.get().screen.scrcpy_path;
        core.bridges.pair(path.as_deref(), &addr, &code).map_err(err)
    })
    .await
}

#[tauri::command]
pub async fn sunshine_status(core: Core<'_>) -> Res<SunshineStatus> {
    let core = core.inner().clone();
    blocking(move || {
        let path = core.settings.get().screen.sunshine_path;
        Ok(core.bridges.sunshine_status(path.as_deref()))
    })
    .await
}

// ---------- music ----------

#[tauri::command]
pub async fn media_state(core: Core<'_>) -> Res<serde_json::Value> {
    Ok(serde_json::json!({ "state": core.media.state(), "nowMs": omnihub_core::media::now_ms() }))
}

#[tauri::command]
pub async fn media_control(core: Core<'_>, action: omnihub_core::media::Action, position_ms: Option<u64>) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || core.media.control(action, position_ms.unwrap_or(0))).await
}

#[tauri::command]
pub async fn media_lyrics(core: Core<'_>) -> Res<serde_json::Value> {
    Ok(serde_json::json!({ "key": core.media.state().map(|s| s.key), "lyrics": core.media.lyrics() }))
}

/// Cover art as a data URL.
#[tauri::command]
pub async fn media_art(core: Core<'_>, id: String) -> Res<Option<String>> {
    Ok(core.media.art_data_url(&id))
}

#[tauri::command]
pub async fn media_audio() -> Res<serde_json::Value> {
    blocking(|| Ok(serde_json::json!({ "volume": omnihub_core::media::audio::get(), "eq": omnihub_core::media::eq::status() }))).await
}

#[tauri::command]
pub async fn media_set_volume(level: Option<f32>, muted: Option<bool>) -> Res<omnihub_core::media::audio::Volume> {
    blocking(move || omnihub_core::media::audio::set(level, muted)).await
}

// ---------- fast scans without a prompt ----------

#[tauri::command]
pub async fn storage_scan_task_status() -> Res<bool> {
    blocking(|| Ok(omnihub_core::storage::scan_task::enabled())).await
}

/// Create (or remove) the scheduled task that runs fast scans with
/// administrator rights, after one UAC approval.
#[tauri::command]
pub async fn storage_scan_task_set(core: Core<'_>, on: bool) -> Res<bool> {
    let core = core.inner().clone();
    blocking(move || {
        let r = omnihub_core::storage::scan_task::set_enabled(on, &core.paths.scans());
        core.audit.record("desktop", if on { "scan-task.on" } else { "scan-task.off" }, "", r.is_ok());
        r?;
        Ok(omnihub_core::storage::scan_task::enabled())
    })
    .await
}

// ---------- updates ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    current: String,
    state: omnihub_core::update::UpdateState,
    /// The version that ran before, on the first start after an update.
    updated_from: Option<String>,
}

#[tauri::command]
pub async fn update_state(core: Core<'_>) -> Res<UpdateInfo> {
    Ok(UpdateInfo { current: core.updater.current_version().to_string(), state: core.updater.state(), updated_from: core.updater.updated_from().map(str::to_string) })
}

#[tauri::command]
pub async fn update_check(core: Core<'_>) -> Res<UpdateInfo> {
    let core = core.inner().clone();
    blocking(move || Ok(UpdateInfo { current: core.updater.current_version().to_string(), state: core.updater.check(), updated_from: core.updater.updated_from().map(str::to_string) })).await
}

/// Download, verify and run the installer for the available version; the
/// app closes and the installer reopens the new one.
#[tauri::command]
pub async fn update_install(core: Core<'_>) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || {
        let release = match core.updater.state() {
            omnihub_core::update::UpdateState::Available { release } => release,
            _ => match core.updater.check() {
                omnihub_core::update::UpdateState::Available { release } => release,
                _ => return Err("OmniHub is up to date.".to_string()),
            },
        };
        core.audit.record("desktop", "update.install", &release.version, true);
        core.updater.install(&release)
    })
    .await
}

// ---------- games ----------

use omnihub_core::games::{self, GameKind, GameProfile};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GamesOverview {
    profiles: Vec<GameProfile>,
    session: Option<games::Session>,
}

#[tauri::command]
pub async fn games_list(core: Core<'_>) -> Res<GamesOverview> {
    Ok(GamesOverview { profiles: core.games.profiles(), session: core.games.session() })
}

#[tauri::command]
pub async fn games_create(core: Core<'_>, kind: GameKind) -> Res<GameProfile> {
    let core = core.inner().clone();
    blocking(move || Ok(core.games.create(kind))).await
}

#[tauri::command]
pub async fn games_save(core: Core<'_>, profile: GameProfile) -> Res<games::SaveResult> {
    let core = core.inner().clone();
    blocking(move || core.games.save(profile)).await
}

#[tauri::command]
pub async fn games_delete(core: Core<'_>, id: String) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || core.games.delete(&id)).await
}

#[tauri::command]
pub async fn games_state(core: Core<'_>, id: String) -> Res<Option<games::GameState>> {
    let core = core.inner().clone();
    blocking(move || Ok(core.games.state(&id))).await
}

#[tauri::command]
pub async fn games_play(core: Core<'_>, id: String, launch: bool) -> Res<games::Session> {
    let core = core.inner().clone();
    let r = core.games.play(&id, launch);
    core.audit.record("desktop", if launch { "game.play" } else { "game.boost" }, &id, r.is_ok());
    r
}

#[tauri::command]
pub async fn games_stop(core: Core<'_>) -> Res<bool> {
    Ok(core.games.stop())
}

#[tauri::command]
pub async fn games_ping(core: Core<'_>, id: Option<String>, host: Option<String>) -> Res<Vec<games::ping::PingResult>> {
    let core = core.inner().clone();
    blocking(move || Ok(core.games.ping(id.as_deref(), host.as_deref()))).await
}

#[tauri::command]
pub async fn games_ping_targets(core: Core<'_>, id: Option<String>) -> Res<Vec<games::ping::PingTarget>> {
    Ok(core.games.ping_targets(id.as_deref()))
}

#[tauri::command]
pub async fn games_roblox_status(core: Core<'_>) -> Res<games::roblox::RobloxInstall> {
    let core = core.inner().clone();
    blocking(move || Ok(core.games.roblox_status())).await
}

#[tauri::command]
pub async fn games_roblox_write(core: Core<'_>, flags: games::roblox::RobloxFlags) -> Res<Vec<String>> {
    let core = core.inner().clone();
    blocking(move || core.games.write_roblox(&flags)).await
}

/// Flags in a list Roblox ignores (outside its allowlist).
#[tauri::command]
pub async fn games_roblox_preview(flags: games::roblox::RobloxFlags) -> Res<serde_json::Value> {
    let map = flags.to_flags();
    let ignored = games::roblox::ignored(&map);
    Ok(serde_json::json!({ "flags": map, "ignored": ignored }))
}

// ---------- game settings, installed games, PC tweaks ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameConfigs {
    options: games::configs::ConfigOptions,
    games: Vec<games::configs::ConfigStatus>,
}

fn game_configs(core: &AppCore) -> GameConfigs {
    GameConfigs { options: core.games.config_options(), games: games::configs::ConfigGame::all().into_iter().map(|g| core.games.config_status(g)).collect() }
}

#[tauri::command]
pub async fn games_configs(core: Core<'_>) -> Res<GameConfigs> {
    let core = core.inner().clone();
    blocking(move || Ok(game_configs(&core))).await
}

#[tauri::command]
pub async fn games_config_options(core: Core<'_>, options: games::configs::ConfigOptions) -> Res<GameConfigs> {
    let core = core.inner().clone();
    blocking(move || {
        core.games.set_config_options(options);
        Ok(game_configs(&core))
    })
    .await
}

#[tauri::command]
pub async fn games_config_apply(core: Core<'_>, game: games::configs::ConfigGame) -> Res<GameConfigs> {
    let core = core.inner().clone();
    blocking(move || {
        let r = core.games.config_apply(game);
        core.audit.record("desktop", "game.settings", &format!("{} ({} changes)", game.label(), r.as_ref().map(Vec::len).unwrap_or(0)), r.is_ok());
        r?;
        Ok(game_configs(&core))
    })
    .await
}

#[tauri::command]
pub async fn games_config_restore(core: Core<'_>, game: games::configs::ConfigGame) -> Res<GameConfigs> {
    let core = core.inner().clone();
    blocking(move || {
        let r = core.games.config_restore(game);
        core.audit.record("desktop", "game.settings-restore", game.label(), r.is_ok());
        r?;
        Ok(game_configs(&core))
    })
    .await
}

#[tauri::command]
pub async fn games_library(core: Core<'_>) -> Res<Vec<games::library::InstalledGame>> {
    let core = core.inner().clone();
    blocking(move || Ok(core.games.library())).await
}

#[tauri::command]
pub async fn games_add_installed(core: Core<'_>, key: String) -> Res<GameProfile> {
    let core = core.inner().clone();
    blocking(move || core.games.add_installed(&key)).await
}

#[tauri::command]
pub async fn games_load_test(core: Core<'_>, id: Option<String>) -> Res<games::ping::LoadTest> {
    let core = core.inner().clone();
    blocking(move || Ok(core.games.load_test(id.as_deref()))).await
}

// ---------- FPS meter ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FpsOverview {
    status: games::fps::FpsStatus,
    live: Option<games::fps::Live>,
    history: Vec<games::fps::FpsRecord>,
}

fn fps_overview(core: &AppCore) -> FpsOverview {
    FpsOverview { status: core.games.fps_status(), live: core.games.fps_live(), history: core.games.fps_history(None) }
}

#[tauri::command]
pub async fn games_fps(core: Core<'_>) -> Res<FpsOverview> {
    let core = core.inner().clone();
    blocking(move || Ok(fps_overview(&core))).await
}

#[tauri::command]
pub async fn games_fps_install(core: Core<'_>) -> Res<FpsOverview> {
    let core = core.inner().clone();
    blocking(move || {
        let r = core.games.fps_install();
        core.audit.record("desktop", "game.fps-install", "PresentMon", r.is_ok());
        r?;
        Ok(fps_overview(&core))
    })
    .await
}

#[tauri::command]
pub async fn games_fps_allow(core: Core<'_>) -> Res<FpsOverview> {
    let core = core.inner().clone();
    blocking(move || {
        let r = core.games.fps_allow();
        core.audit.record("desktop", "game.fps-allow", "Performance Log Users", r.is_ok());
        r?;
        Ok(fps_overview(&core))
    })
    .await
}

#[tauri::command]
pub async fn pc_status(core: Core<'_>) -> Res<games::pc::PcStatus> {
    let core = core.inner().clone();
    blocking(move || Ok(core.games.pc_status())).await
}

#[tauri::command]
pub async fn pc_set(core: Core<'_>, id: games::pc::TweakId, on: bool) -> Res<games::pc::PcStatus> {
    let core = core.inner().clone();
    blocking(move || {
        let r = core.games.pc_set(id, on);
        core.audit.record("desktop", if on { "pc.optimize" } else { "pc.undo" }, &format!("{id:?}"), r.is_ok());
        r
    })
    .await
}

// ---------- screen sharing privacy ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareState {
    paused: bool,
    window_id: Option<u32>,
    /// The shared window, if it still exists.
    window: Option<stream::WindowInfo>,
}

fn share_state() -> ShareState {
    let window_id = stream::shared_window();
    let window = window_id.and_then(|id| stream::windows().into_iter().find(|w| w.id == id));
    ShareState { paused: stream::paused(), window_id, window }
}

#[tauri::command]
pub async fn screen_share_state() -> Res<ShareState> {
    blocking(|| Ok(share_state())).await
}

#[tauri::command]
pub async fn screen_windows() -> Res<Vec<stream::WindowInfo>> {
    blocking(|| Ok(stream::windows())).await
}

/// Pause (or resume) every screen share: nothing is captured while paused.
#[tauri::command]
pub async fn screen_set_paused(core: Core<'_>, on: bool) -> Res<ShareState> {
    stream::set_paused(on);
    core.audit.record("desktop", if on { "screen.pause" } else { "screen.resume" }, "", true);
    let st = blocking(|| Ok(share_state())).await?;
    core.events.emit("screen:share-state", &st);
    Ok(st)
}

/// Share only one window (null = the whole display the viewer picks).
#[tauri::command]
pub async fn screen_set_window(core: Core<'_>, id: Option<u32>) -> Res<ShareState> {
    stream::set_shared_window(id);
    let st = blocking(|| Ok(share_state())).await?;
    core.audit.record("desktop", "screen.window", st.window.as_ref().map_or("whole display", |w| w.title.as_str()), true);
    core.events.emit("screen:share-state", &st);
    Ok(st)
}

// ---------- iPhone mirroring (AirPlay) ----------

#[tauri::command]
pub async fn airplay_status(core: Core<'_>) -> Res<AirPlayStatus> {
    Ok(core.airplay.status(core.settings.get().screen.uxplay_path.as_deref()))
}

/// Download and install the AirPlay add-on in the background
/// (progress arrives as `airplay:install` events).
#[tauri::command]
pub async fn airplay_install(core: Core<'_>) -> Res<()> {
    let core = core.inner().clone();
    std::thread::spawn(move || {
        let url = core.airplay.status(None).download_url;
        let res = core.airplay.install_addon(&url, omnihub_core::capture::airplay::pinned_sha256());
        core.audit.record("desktop", "airplay.install", &url, res.is_ok());
        if let Err(e) = res {
            tracing::warn!("AirPlay add-on install failed: {e}");
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn airplay_uninstall(core: Core<'_>) -> Res<()> {
    core.airplay.uninstall_addon().map_err(err)
}

/// Start the receiver with the saved options; returns the PIN if one is needed.
#[tauri::command]
pub async fn airplay_start(core: Core<'_>) -> Res<Option<String>> {
    let core = core.inner().clone();
    blocking(move || {
        let s = core.settings.get().screen;
        let pin = core.airplay.start(s.uxplay_path.as_deref(), &s.airplay, s.airplay_keep_on_top, s.airplay_pip).map_err(err)?;
        core.audit.record("desktop", "airplay.start", &s.airplay.name, true);
        Ok(pin)
    })
    .await
}

#[tauri::command]
pub async fn airplay_stop(core: Core<'_>) -> Res<()> {
    core.airplay.stop();
    Ok(())
}

#[tauri::command]
pub async fn airplay_keep_on_top(core: Core<'_>, on: bool) -> Res<()> {
    core.airplay.set_keep_on_top(on);
    core.update_settings(&serde_json::json!({ "screen": { "airplayKeepOnTop": on } })).map_err(err)?;
    Ok(())
}

/// "pip" (small, bottom-right) or "center".
#[tauri::command]
pub async fn airplay_place(core: Core<'_>, how: String) -> Res<()> {
    core.airplay.place_window(&how);
    Ok(())
}

fn uxplay_exe(core: &AppCore) -> Res<PathBuf> {
    core.airplay.find(core.settings.get().screen.uxplay_path.as_deref()).map(|(p, _)| p).ok_or_else(|| "the AirPlay receiver is not installed".to_string())
}

#[tauri::command]
pub async fn airplay_firewall(core: Core<'_>) -> Res<FirewallReport> {
    let core = core.inner().clone();
    blocking(move || {
        let exe = uxplay_exe(&core)?;
        Ok(firewall::check(&exe, None, Proto::Any, "the AirPlay receiver"))
    })
    .await
}

#[tauri::command]
pub async fn airplay_fix_firewall(core: Core<'_>, include_public: bool) -> Res<FirewallReport> {
    let core = core.inner().clone();
    blocking(move || {
        let exe = uxplay_exe(&core)?;
        let mut mask = firewall::PROFILE_PRIVATE | firewall::PROFILE_DOMAIN;
        if include_public {
            mask |= firewall::PROFILE_PUBLIC;
        }
        let res = firewall::allow_program(&exe, "OmniHub AirPlay receiver", mask);
        core.audit.record("desktop", "firewall.allow", &format!("AirPlay receiver ({})", firewall::profile_names(mask).join(", ")), res.is_ok());
        res.map_err(err)?;
        Ok(firewall::check(&exe, None, Proto::Any, "the AirPlay receiver"))
    })
    .await
}

// ---------- browser autofill ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserStatus {
    enabled: bool,
    listening: bool,
    /// What the browsers start (this executable or the stand-alone host).
    host: Option<String>,
    browsers: Vec<Registration>,
    clients: Vec<BrowserClient>,
    pending: Option<PairRequest>,
    /// Folder to "Load unpacked" in brave://extensions.
    extension_dir: Option<String>,
    extension_id: String,
}

/// The extension folder shipped with the app (the source folder in development).
fn extension_dir(app: &AppHandle) -> Option<PathBuf> {
    let bundled = app.path().resource_dir().ok().map(|d| d.join("browser-extension"));
    let dev = cfg!(debug_assertions).then(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../browser-extension"));
    [bundled, dev].into_iter().flatten().find(|d| d.join("manifest.json").is_file()).map(|d| dunce_path(&d))
}

/// Explorer and Brave both choke on `\\?\` paths; show the plain form.
fn dunce_path(p: &Path) -> PathBuf {
    let p = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    let s = p.to_string_lossy();
    PathBuf::from(s.strip_prefix(r"\\?\").unwrap_or(&s))
}

#[tauri::command]
pub async fn browser_status(app: AppHandle, core: Core<'_>) -> Res<BrowserStatus> {
    let core = core.inner().clone();
    let dir = extension_dir(&app);
    blocking(move || {
        Ok(BrowserStatus {
            enabled: core.settings.get().vault.browser_autofill,
            listening: core.browser.is_listening(),
            host: omnihub_core::browser::register::host_path(&core.paths.data).map(|p| p.to_string_lossy().into_owned()),
            browsers: omnihub_core::browser::register::status(),
            clients: core.browser.clients(),
            pending: core.browser.pending(),
            extension_dir: dir.map(|d| d.to_string_lossy().into_owned()),
            extension_id: omnihub_core::browser::EXTENSION_ID.to_string(),
        })
    })
    .await
}

/// Register the host with the browsers again (after moving OmniHub, or if
/// a browser was installed after autofill was turned on).
#[tauri::command]
pub async fn browser_repair(core: Core<'_>) -> Res<Vec<Registration>> {
    let core = core.inner().clone();
    blocking(move || omnihub_core::browser::register::register(&core.paths.data).map_err(err)).await
}

#[tauri::command]
pub async fn browser_pair_respond(core: Core<'_>, id: String, allow: bool) -> Res<bool> {
    Ok(core.browser.respond(&id, allow))
}

#[tauri::command]
pub async fn browser_revoke(core: Core<'_>, id: String) -> Res<bool> {
    let name = core.browser.clients().into_iter().find(|c| c.id == id).map(|c| c.name).unwrap_or_default();
    let done = core.browser.revoke(&id);
    core.audit.record("desktop", "browser.revoke", &name, done);
    Ok(done)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotpCode {
    code: String,
    /// Seconds until the code changes.
    remaining: u64,
}

#[tauri::command]
pub async fn vault_totp(core: Core<'_>, id: String) -> Res<TotpCode> {
    let (code, remaining) = core.vault.totp_code(&id).map_err(err)?;
    Ok(TotpCode { code, remaining })
}
