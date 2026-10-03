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
use omnihub_core::capture::bridges::{ScrcpyOptions, ScrcpyStatus, SunshineStatus};
use omnihub_core::capture::screenshots::{CaptureKind, PendingRegion, Rect, Screenshot, ShotFilter};
use omnihub_core::capture::stream::{self, MonitorInfo, Preset};
use omnihub_core::core::AppCore;
use omnihub_core::notes::{FolderFile, Note, NoteFilter, NoteInput};
use omnihub_core::remote::auth::{Device, PairingInfo};
use omnihub_core::remote::transfer::InboxItem;
use omnihub_core::remote::ServerStatus;
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
    let items = core.remote.send_to_phone(&paths, device_id).map_err(err)?;
    core.audit.record("desktop", "files.offer", &format!("{} file(s)", items.len()), true);
    Ok(items)
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
