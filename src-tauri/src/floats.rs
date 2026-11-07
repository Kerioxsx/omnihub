//! Small windows of their own: the speedrun timer above the game, a
//! crosshair over everything (click-through), desktop widgets sitting on the
//! desktop under other windows, and a live wallpaper behind the desktop
//! icons. Which widgets are open (and where) is remembered and they come
//! back when OmniHub starts.

use std::sync::Arc;

use omnihub_core::core::AppCore;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

type Res<T> = Result<T, String>;
type Core<'a> = tauri::State<'a, Arc<AppCore>>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Widget {
    pub id: String,
    /// "temps", "music", "counter", "battery", "network", "clock"
    pub kind: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Floats {
    pub widgets: Vec<Widget>,
    pub crosshair: bool,
    pub wallpaper: bool,
}

const KINDS: &[&str] = &["temps", "music", "counter", "battery", "network", "clock"];

fn file(core: &AppCore) -> std::path::PathBuf {
    core.paths.data.join("floats.json")
}

fn load(core: &AppCore) -> Floats {
    std::fs::read(file(core)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn store(core: &AppCore, f: &Floats) {
    if let Ok(j) = serde_json::to_vec_pretty(f) {
        let _ = omnihub_core::settings::write_atomic(&file(core), &j);
    }
}

// ---------- speedrun ----------

#[tauri::command]
pub async fn speedrun_state(core: Core<'_>) -> Res<omnihub_core::speedrun::SpeedrunState> {
    Ok(core.speedrun.state())
}

#[tauri::command]
pub async fn speedrun_save(core: Core<'_>, settings: omnihub_core::speedrun::SpeedrunSettings) -> Res<omnihub_core::speedrun::SpeedrunState> {
    let st = core.speedrun.save(settings)?;
    core.refresh_keys();
    Ok(st)
}

/// "split", "reset", "pause" or "undo" (the buttons in the window).
#[tauri::command]
pub async fn speedrun_press(core: Core<'_>, what: String) -> Res<omnihub_core::speedrun::SpeedrunState> {
    use omnihub_core::speedrun::Press;
    let p = match what.as_str() {
        "split" => Press::Split,
        "reset" => Press::Reset,
        "pause" => Press::Pause,
        "undo" => Press::Undo,
        _ => return Err("Unknown button".into()),
    };
    core.speedrun.press(p, chrono_now());
    Ok(core.speedrun.state())
}

fn chrono_now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

/// Open or close the timer window; its keys work only while it's open.
#[tauri::command]
pub async fn speedrun_window(app: AppHandle, core: Core<'_>, open: bool) -> Res<()> {
    let keys = |on: bool, core: &AppCore| {
        core.speedrun_keys.store(on, std::sync::atomic::Ordering::Relaxed);
        core.refresh_keys();
    };
    if !open {
        if let Some(w) = app.get_webview_window("speedrun") {
            let _ = w.close();
        }
        keys(false, &core);
        return Ok(());
    }
    if let Some(w) = app.get_webview_window("speedrun") {
        let _ = w.show();
        let _ = w.set_focus();
        return Ok(());
    }
    let w = WebviewWindowBuilder::new(&app, "speedrun", WebviewUrl::App("index.html#/speedrun".into()))
        .title("Speedrun timer")
        .decorations(false)
        .transparent(true)
        .background_color(tauri::webview::Color(0, 0, 0, 0))
        .always_on_top(true)
        .resizable(true)
        .skip_taskbar(true)
        .shadow(false)
        .inner_size(300.0, 420.0)
        .min_inner_size(220.0, 180.0)
        .build()
        .map_err(|e| e.to_string())?;
    keys(true, &core);
    let c = core.inner().clone();
    w.on_window_event(move |e| {
        if matches!(e, tauri::WindowEvent::Destroyed) {
            c.speedrun_keys.store(false, std::sync::atomic::Ordering::Relaxed);
            c.refresh_keys();
        }
    });
    Ok(())
}

// ---------- crosshair ----------

fn open_crosshair(app: &AppHandle) -> Res<()> {
    if app.get_webview_window("crosshair").is_some() {
        return Ok(());
    }
    let m = app.primary_monitor().ok().flatten().ok_or("No screen found")?;
    let w = WebviewWindowBuilder::new(app, "crosshair", WebviewUrl::App("index.html#/crosshair".into()))
        .title("OmniHub crosshair")
        .decorations(false)
        .transparent(true)
        .background_color(tauri::webview::Color(0, 0, 0, 0))
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .focusable(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    let _ = w.set_position(*m.position());
    let _ = w.set_size(*m.size());
    let _ = w.set_ignore_cursor_events(true);
    let _ = w.show();
    Ok(())
}

#[tauri::command]
pub async fn crosshair_set(app: AppHandle, core: Core<'_>, on: bool) -> Res<bool> {
    let mut f = load(&core);
    f.crosshair = on;
    store(&core, &f);
    if on {
        open_crosshair(&app)?;
    } else if let Some(w) = app.get_webview_window("crosshair") {
        let _ = w.close();
    }
    Ok(on)
}

// ---------- desktop widgets ----------

fn widget_label(id: &str) -> String {
    format!("widget-{}", id.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>())
}

fn open_widget(app: &AppHandle, core: Arc<AppCore>, wd: &Widget) -> Res<()> {
    let label = widget_label(&wd.id);
    if app.get_webview_window(&label).is_some() {
        return Ok(());
    }
    let url = format!("index.html#/widget?kind={}&id={}", wd.kind, wd.id.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>());
    let w = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(url.into()))
        .title("OmniHub widget")
        .decorations(false)
        .transparent(true)
        .background_color(tauri::webview::Color(0, 0, 0, 0))
        .always_on_bottom(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .inner_size(250.0, if wd.kind == "music" { 120.0 } else { 140.0 })
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    if wd.x != 0 || wd.y != 0 {
        let _ = w.set_position(PhysicalPosition::new(wd.x, wd.y));
    }
    let _ = w.show();
    // Remember where it was dragged.
    let id = wd.id.clone();
    w.on_window_event(move |e| {
        if let tauri::WindowEvent::Moved(p) = e {
            let mut f = load(&core);
            if let Some(x) = f.widgets.iter_mut().find(|x| x.id == id) {
                x.x = p.x;
                x.y = p.y;
                store(&core, &f);
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn floats_get(core: Core<'_>) -> Res<Floats> {
    Ok(load(&core))
}

#[tauri::command]
pub async fn widget_add(app: AppHandle, core: Core<'_>, kind: String) -> Res<Floats> {
    if !KINDS.contains(&kind.as_str()) {
        return Err("Unknown widget".into());
    }
    let mut f = load(&core);
    if f.widgets.len() >= 12 {
        return Err("Twelve widgets is the most".into());
    }
    let wd = Widget { id: format!("{:x}", chrono_now()), kind, x: 0, y: 0 };
    f.widgets.push(wd.clone());
    store(&core, &f);
    open_widget(&app, core.inner().clone(), &wd)?;
    Ok(f)
}

#[tauri::command]
pub async fn widget_remove(app: AppHandle, core: Core<'_>, id: String) -> Res<Floats> {
    let mut f = load(&core);
    f.widgets.retain(|w| w.id != id);
    store(&core, &f);
    if let Some(w) = app.get_webview_window(&widget_label(&id)) {
        let _ = w.close();
    }
    Ok(f)
}

// ---------- live wallpaper ----------

#[tauri::command]
pub async fn wallpaper_set(app: AppHandle, core: Core<'_>, on: bool) -> Res<bool> {
    let mut f = load(&core);
    f.wallpaper = on;
    store(&core, &f);
    if on {
        open_wallpaper(&app)?;
    } else if let Some(w) = app.get_webview_window("wallpaper") {
        let _ = w.close();
    }
    Ok(on)
}

fn open_wallpaper(app: &AppHandle) -> Res<()> {
    if app.get_webview_window("wallpaper").is_some() {
        return Ok(());
    }
    let m = app.primary_monitor().ok().flatten().ok_or("No screen found")?;
    let w = WebviewWindowBuilder::new(app, "wallpaper", WebviewUrl::App("index.html#/wallpaper".into()))
        .title("OmniHub wallpaper")
        .decorations(false)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    let _ = w.set_position(*m.position());
    let _ = w.set_size(*m.size());
    behind_icons(&w)?;
    let _ = w.show();
    Ok(())
}

/// Put a window behind the desktop icons (in the desktop's own WorkerW
/// window, the way wallpaper apps do).
#[cfg(windows)]
fn behind_icons(w: &tauri::WebviewWindow) -> Res<()> {
    use windows::core::{w, BOOL, PCWSTR};
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, FindWindowExW, FindWindowW, SendMessageTimeoutW, SetParent, SMTO_NORMAL};
    let ours = HWND(w.hwnd().map_err(|e| e.to_string())?.0);
    unsafe {
        let progman = FindWindowW(w!("Progman"), PCWSTR::null()).map_err(|_| "The desktop wasn't found".to_string())?;
        // Ask the desktop for its WorkerW layer behind the icons.
        let _ = SendMessageTimeoutW(progman, 0x052C, WPARAM(0xD), LPARAM(0x1), SMTO_NORMAL, 1000, None);
        // Windows 11 24H2 and later keep it inside Progman.
        let mut target = FindWindowExW(Some(progman), None, w!("WorkerW"), PCWSTR::null()).ok();
        if target.is_none() {
            unsafe extern "system" fn each(top: HWND, out: LPARAM) -> BOOL {
                if FindWindowExW(Some(top), None, w!("SHELLDLL_DefView"), PCWSTR::null()).is_ok() {
                    if let Ok(worker) = FindWindowExW(None, Some(top), w!("WorkerW"), PCWSTR::null()) {
                        *(out.0 as *mut HWND) = worker;
                    }
                }
                BOOL(1)
            }
            let mut found = HWND::default();
            let _ = EnumWindows(Some(each), LPARAM(&mut found as *mut HWND as isize));
            target = (!found.is_invalid()).then_some(found);
        }
        let target = target.ok_or("Windows didn't make room behind the icons")?;
        SetParent(ours, Some(target)).map_err(|e| format!("Couldn't put it behind the icons: {e}"))?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn behind_icons(_w: &tauri::WebviewWindow) -> Res<()> {
    Ok(())
}

/// At start: the widgets, crosshair and wallpaper that were on.
pub fn restore(app: &AppHandle, core: Arc<AppCore>) {
    let f = load(&core);
    for wd in &f.widgets {
        if let Err(e) = open_widget(app, core.clone(), wd) {
            tracing::warn!("widget: {e}");
        }
    }
    if f.crosshair {
        let _ = open_crosshair(app);
    }
    if f.wallpaper {
        let _ = open_wallpaper(app);
    }
}
