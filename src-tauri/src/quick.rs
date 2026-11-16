//! Clipboard history, the quick-launch wheel, the macro deck, batteries and
//! the PC health score: commands, hotkeys and the two pop-up windows.

use std::str::FromStr;
use std::sync::Arc;

use omnihub_core::core::AppCore;
use omnihub_core::quick::{ClipContent, ClipItem, QuickAction, QuickState};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_global_shortcut::{GlobalShortcut, Shortcut, ShortcutState};

type Core<'a> = State<'a, Arc<AppCore>>;
type Res<T> = Result<T, String>;

const WHEEL: &str = "wheel";
const CLIP: &str = "clipboard";

/// Register the wheel's and the clipboard's hotkeys (after `unregister_all`).
pub fn register_hotkeys(core: &AppCore, gs: &GlobalShortcut<tauri::Wry>) {
    let s = core.quick.state();
    for (on, spec, label) in [(s.wheel_on, s.wheel_hotkey, WHEEL), (s.clip_on, s.clip_hotkey, CLIP)] {
        if !on || spec.trim().is_empty() {
            continue;
        }
        let Ok(shortcut) = Shortcut::from_str(&spec) else {
            tracing::warn!("invalid hotkey {spec:?}");
            continue;
        };
        let res = gs.on_shortcut(shortcut, move |app, _sc, event| {
            if event.state() == ShortcutState::Pressed {
                let _ = popup(app, label);
            }
        });
        if let Err(e) = res {
            tracing::warn!("could not register hotkey {spec}: {e}");
        }
    }
}

/// Show the wheel or the clipboard list at the mouse (made once, then hidden
/// and shown so it opens at once).
fn popup(app: &AppHandle, label: &str) -> Res<()> {
    let (w, h) = if label == WHEEL { (440.0, 440.0) } else { (380.0, 520.0) };
    let win = match app.get_webview_window(label) {
        Some(win) => win,
        None => WebviewWindowBuilder::new(app, label, WebviewUrl::App(format!("index.html#/{label}").into()))
            .title(if label == WHEEL { "Quick launch" } else { "Clipboard" })
            .decorations(false)
            .transparent(label == WHEEL)
            .shadow(label != WHEEL)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(false)
            .inner_size(w, h)
            .build()
            .map_err(|e| e.to_string())?,
    };
    // Centre the wheel on the mouse; put the list beside it, kept on screen.
    if let Ok(p) = app.cursor_position() {
        let scale = app.monitor_from_point(p.x, p.y).ok().flatten().map(|m| (m.scale_factor(), *m.position(), *m.size()));
        let (sf, mp, ms) = scale.map_or((1.0, tauri::PhysicalPosition::new(0, 0), tauri::PhysicalSize::new(1920, 1080)), |s| s);
        let (pw, ph) = (w * sf, h * sf);
        let (mut x, mut y) = if label == WHEEL { (p.x - pw / 2.0, p.y - ph / 2.0) } else { (p.x + 12.0, p.y - 40.0) };
        x = x.clamp(mp.x as f64, (mp.x as f64 + ms.width as f64 - pw).max(mp.x as f64));
        y = y.clamp(mp.y as f64, (mp.y as f64 + ms.height as f64 - ph).max(mp.y as f64));
        let _ = win.set_position(tauri::PhysicalPosition::new(x.round() as i32, y.round() as i32));
    }
    let _ = win.show();
    let _ = win.set_focus();
    let _ = tauri::Emitter::emit(&win, "popup:shown", ());
    Ok(())
}

fn hide(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.hide();
    }
}

#[tauri::command]
pub async fn quick_state(core: Core<'_>) -> Res<QuickState> {
    Ok(core.quick.state())
}

#[tauri::command]
pub async fn quick_save(app: AppHandle, core: Core<'_>, state: QuickState) -> Res<QuickState> {
    let saved = core.quick.save(state)?;
    crate::shortcuts::register(&app);
    Ok(saved)
}

/// Run an action (from the wheel, the deck page or a test button).
#[tauri::command]
pub async fn quick_run(app: AppHandle, core: Core<'_>, action: QuickAction, notify: Option<bool>) -> Res<String> {
    hide(&app, WHEEL);
    let r = run_action(&app, core.inner().clone(), action).await;
    // The wheel has no room for messages: say what went wrong in a notification.
    if let (Err(e), Some(true)) = (&r, notify) {
        crate::shortcuts::notify(&app, "Quick launch", e);
    }
    r
}

async fn run_action(app: &AppHandle, core: Arc<AppCore>, action: QuickAction) -> Res<String> {
    if action == QuickAction::Clipboard {
        popup(app, CLIP)?;
        return Ok(String::new());
    }
    action.check()?;
    // Let the window under the wheel get the keyboard back before keys are pressed.
    if matches!(action, QuickAction::Keys { .. } | QuickAction::Text { .. }) {
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    }
    tauri::async_runtime::spawn_blocking(move || core.run_quick(&action)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn quick_hide(app: AppHandle, which: String) -> Res<()> {
    hide(&app, if which == WHEEL { WHEEL } else { CLIP });
    Ok(())
}

#[tauri::command]
pub async fn quick_open(app: AppHandle, which: String) -> Res<()> {
    popup(&app, if which == WHEEL { WHEEL } else { CLIP })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsInfo {
    has_password: bool,
    scenes: Vec<String>,
    inputs: Vec<String>,
}

/// Save (or forget, with "") OBS's WebSocket password.
#[tauri::command]
pub async fn quick_obs_password(core: Core<'_>, password: String) -> Res<()> {
    core.quick.set_obs_password(password.trim())
}

/// Whether a password is saved, and (when `connect`) OBS's scenes and sources.
#[tauri::command]
pub async fn quick_obs_info(core: Core<'_>, connect: bool) -> Res<ObsInfo> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let has_password = core.quick.has_obs_password();
        if !connect {
            return Ok(ObsInfo { has_password, scenes: Vec::new(), inputs: Vec::new() });
        }
        let (scenes, inputs) = omnihub_core::obs::lists(&core.quick.state().obs_url, &core.quick.obs_password())?;
        Ok(ObsInfo { has_password, scenes, inputs })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipList {
    items: Vec<ClipItem>,
    paused: bool,
}

#[tauri::command]
pub async fn clipboard_list(core: Core<'_>) -> Res<ClipList> {
    Ok(ClipList { items: core.quick.items(), paused: core.quick.paused() })
}

fn put_back(core: &AppCore, id: &str) -> Res<()> {
    match core.quick.content(id)? {
        ClipContent::Text(t) => omnihub_core::system::clipboard::copy_text(&t),
        ClipContent::Image { width, height, rgba } => omnihub_core::system::clipboard::copy_image(width, height, &rgba),
    }
    .map_err(|e| e.to_string())
}

/// Put an item back on the clipboard.
#[tauri::command]
pub async fn clipboard_copy(core: Core<'_>, id: String) -> Res<()> {
    put_back(&core, &id)
}

/// Put an item back and paste it into the app that was in front.
#[tauri::command]
pub async fn clipboard_paste(app: AppHandle, core: Core<'_>, id: String) -> Res<()> {
    if let Err(e) = put_back(&core, &id) {
        crate::shortcuts::notify(&app, "Couldn't paste", &e);
        return Err(e);
    }
    hide(&app, CLIP);
    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    omnihub_core::capture::stream::input::chord("Ctrl+V");
    Ok(())
}

#[tauri::command]
pub async fn clipboard_pin(core: Core<'_>, id: String, on: bool) -> Res<()> {
    core.quick.pin(&id, on)
}

#[tauri::command]
pub async fn clipboard_delete(core: Core<'_>, id: String) -> Res<()> {
    core.quick.delete(&id)
}

#[tauri::command]
pub async fn clipboard_clear(core: Core<'_>) -> Res<()> {
    core.quick.clear();
    Ok(())
}

#[tauri::command]
pub async fn clipboard_pause(core: Core<'_>, on: bool) -> Res<()> {
    core.quick.set_paused(on);
    Ok(())
}

#[tauri::command]
pub async fn batteries(core: Core<'_>, fresh: bool) -> Res<Vec<omnihub_core::system::battery::Battery>> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let now = omnihub_core::db::now() * 1000;
        if fresh {
            core.batteries.sample(now)
        } else {
            core.batteries.list(now)
        }
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn pc_health() -> Res<omnihub_core::health::Health> {
    tauri::async_runtime::spawn_blocking(|| omnihub_core::health::score(&omnihub_core::health::gather(), omnihub_core::db::now() * 1000)).await.map_err(|e| e.to_string())
}

/// Add the "Tidy Downloads" folder rules.
#[tauri::command]
pub async fn rules_tidy_downloads(core: Core<'_>, old_installers: Option<u32>) -> Res<Vec<omnihub_core::rules::Rule>> {
    let dl = omnihub_core::system::shell::user_folders().into_iter().find(|f| f.0 == "Downloads").map(|f| f.1).ok_or("Couldn't find your Downloads folder.")?;
    core.rules.tidy(&dl, old_installers)
}

#[tauri::command]
pub async fn presence_status(core: Core<'_>) -> Res<omnihub_core::presence::PresenceStatus> {
    Ok(core.presence.status())
}

#[tauri::command]
pub async fn presence_save(core: Core<'_>, settings: omnihub_core::presence::PresenceSettings) -> Res<omnihub_core::presence::PresenceStatus> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let st = core.presence.save(settings)?;
        if !st.settings.enabled {
            core.presence.update(None);
        }
        Ok(st)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Network speed, data used per day, and programs with connections open.
#[tauri::command]
pub async fn network_status(core: Core<'_>) -> Res<omnihub_core::system::netmon::NetStatus> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let apps = omnihub_core::system::netmon::group_connections(&omnihub_core::system::ports::list());
        core.netmon.status(apps)
    })
    .await
    .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupList {
    jobs: Vec<omnihub_core::backup::BackupJob>,
    running: Option<String>,
}

#[tauri::command]
pub async fn backups_list(core: Core<'_>) -> Res<BackupList> {
    Ok(BackupList { jobs: core.backups.list(), running: core.backups.running() })
}

#[tauri::command]
pub async fn backups_save(core: Core<'_>, job: omnihub_core::backup::BackupJob) -> Res<omnihub_core::backup::BackupJob> {
    core.backups.save(job)
}

#[tauri::command]
pub async fn backups_delete(core: Core<'_>, id: String) -> Res<()> {
    core.backups.delete(&id)
}

/// Run a backup now (in the background; `backup:done` says how it went).
#[tauri::command]
pub async fn backups_run(core: Core<'_>, id: String) -> Res<()> {
    if core.backups.running().is_some() {
        return Err("A backup is already running".into());
    }
    let backups = core.backups.clone();
    std::thread::spawn(move || {
        let _ = backups.run(&id, omnihub_core::db::now());
    });
    Ok(())
}

#[tauri::command]
pub async fn backups_cancel(core: Core<'_>) -> Res<()> {
    core.backups.cancel();
    Ok(())
}

/// The MAC and broadcast addresses a phone's Wake-on-LAN app needs.
#[tauri::command]
pub async fn wake_targets() -> Res<Vec<omnihub_core::system::wol::WakeTarget>> {
    tauri::async_runtime::spawn_blocking(omnihub_core::system::wol::targets).await.map_err(|e| e.to_string())
}

/// Device Manager, where a network card's "wake this computer" settings are.
#[tauri::command]
pub async fn open_device_manager() -> Res<()> {
    omnihub_core::system::shell::shell_execute("devmgmt.msc", "").map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn quiet_status(core: Core<'_>) -> Res<omnihub_core::quiet::QuietStatus> {
    Ok(core.quiet_status())
}

/// Quiet mode on (screens off, fans at their lowest) or off.
#[tauri::command]
pub async fn quiet_set(core: Core<'_>, on: bool) -> Res<omnihub_core::quiet::QuietStatus> {
    let core = core.inner().clone();
    tauri::async_runtime::spawn_blocking(move || if on { core.quiet_on() } else { core.quiet_off() }).await.map_err(|e| e.to_string())?
}
