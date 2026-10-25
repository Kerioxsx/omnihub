//! Windows nobody can see rest. A window that is hidden (in the tray),
//! minimised, or sits behind a full-screen game or video on its monitor
//! has its web view marked invisible (its pages stop drawing and their
//! timers pause), asked to use as little memory as it can (WebView2's low
//! memory target), and suspended like a sleeping browser tab.
//!
//! Events for a resting window are held and handed over, in order, when it
//! wakes; the fast streams (audio, light frames, fps) are skipped for it.
//! A window that missed too much reloads instead.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde_json::Value;
use tauri::{AppHandle, Emitter, EventTarget, Manager, WebviewWindow};

/// Streams only worth seeing live.
const LIVE_ONLY: &[&str] = &["audio:frame", "govee:frame", "games:fps"];
/// Topics that each describe the whole state: only the newest is kept.
const LATEST_ONLY: &[&str] = &[
    "media:state",
    "rig:status",
    "govee:status",
    "audio:status",
    "remote:status",
    "remote:devices",
    "screen:viewers",
    "screen:share-state",
    "settings:changed",
    "update:state",
    "games:session",
    "mcserver:players",
    "mcserver:work",
    "browser:clients",
    "airplay:changed",
];
/// Held events per window; past this it reloads when it wakes.
const HOLD_MAX: usize = 1500;
/// How long a full-screen app must cover a window before it rests.
const COVERED_FOR: Duration = Duration::from_secs(3);

#[derive(Default)]
struct Held {
    events: Vec<(String, Value)>,
    overflow: bool,
}

static RESTING: LazyLock<Mutex<HashMap<String, Held>>> = LazyLock::new(Default::default);
static MAIN_SHOWN: AtomicBool = AtomicBool::new(false);

/// Whether the main window is on screen (not hidden in the tray).
pub fn main_shown() -> bool {
    MAIN_SHOWN.load(Ordering::Relaxed)
}

/// Send a core event to the windows that are awake; hold it for the resting ones.
pub fn deliver(app: &AppHandle, topic: &str, payload: &Value) {
    let mut resting = RESTING.lock();
    if resting.is_empty() {
        drop(resting);
        let _ = app.emit(topic, payload);
        return;
    }
    if !LIVE_ONLY.contains(&topic) {
        let latest = LATEST_ONLY.contains(&topic) || topic.ends_with(":progress");
        for held in resting.values_mut() {
            if held.overflow {
                continue;
            }
            if latest {
                held.events.retain(|(t, _)| t != topic);
            }
            if held.events.len() >= HOLD_MAX {
                held.overflow = true;
                held.events = Vec::new();
            } else {
                held.events.push((topic.to_string(), payload.clone()));
            }
        }
    }
    let _ = app.emit_filter(topic, payload, |t| match t {
        EventTarget::AnyLabel { label } | EventTarget::Window { label } | EventTarget::Webview { label } | EventTarget::WebviewWindow { label } => !resting.contains_key(label),
        _ => true,
    });
}

/// Show a window, awake (use instead of `show()`).
pub fn show(w: &WebviewWindow) {
    wake(w);
    let _ = w.show();
}

/// Bring a resting window back: drawing again, then what it missed.
pub fn wake(w: &WebviewWindow) {
    let label = w.label().to_string();
    // Locked while replaying, so newer events can't overtake the held ones.
    let mut resting = RESTING.lock();
    let Some(held) = resting.remove(&label) else { return };
    set_awake(w, true);
    if held.overflow {
        let _ = w.reload();
        return;
    }
    for (topic, payload) in held.events {
        let _ = w.emit_to(EventTarget::WebviewWindow { label: label.clone() }, &topic, payload);
    }
}

fn rest(w: &WebviewWindow) {
    let mut resting = RESTING.lock();
    if resting.contains_key(w.label()) {
        return;
    }
    resting.insert(w.label().to_string(), Held::default());
    set_awake(w, false);
}

pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("window-rest".into())
        .spawn(move || {
            // Let the first window finish opening.
            std::thread::sleep(Duration::from_millis(1500));
            let mut covered_since: HashMap<String, Instant> = HashMap::new();
            loop {
                let windows = app.webview_windows();
                RESTING.lock().retain(|label, _| windows.contains_key(label));
                covered_since.retain(|label, _| windows.contains_key(label));
                for (label, w) in &windows {
                    // The capture overlay lives for a moment and is always looked at.
                    if label == "overlay" {
                        continue;
                    }
                    let shown = w.is_visible().unwrap_or(true);
                    if label == "main" {
                        MAIN_SHOWN.store(shown, Ordering::Relaxed);
                    }
                    let covered = label == "main" && shown && covered_by_fullscreen(w);
                    let since = covered.then(|| *covered_since.entry(label.clone()).or_insert_with(Instant::now));
                    if !covered {
                        covered_since.remove(label);
                    }
                    let unseen = !shown || w.is_minimized().unwrap_or(false) || since.is_some_and(|t| t.elapsed() >= COVERED_FOR);
                    let resting = RESTING.lock().contains_key(label.as_str());
                    if unseen && !resting {
                        rest(w);
                    } else if !unseen && resting && !covered {
                        wake(w);
                    }
                }
                std::thread::sleep(Duration::from_millis(300));
            }
        })
        .ok();
}

#[cfg(windows)]
fn set_awake(w: &WebviewWindow, awake: bool) {
    let _ = w.with_webview(move |view| {
        use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2_19, ICoreWebView2_3, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL};
        use webview2_com::TrySuspendCompletedHandler;
        use windows::core::Interface;
        // SAFETY: COM calls on the web view's own thread (with_webview runs there).
        unsafe {
            let controller = view.controller();
            let Ok(core) = controller.CoreWebView2() else { return };
            let v3 = core.cast::<ICoreWebView2_3>().ok();
            if awake {
                if let Some(v3) = &v3 {
                    let _ = v3.Resume();
                }
                let _ = controller.SetIsVisible(true);
            } else {
                // Invisible first: suspending needs it, and it stops drawing either way.
                let _ = controller.SetIsVisible(false);
            }
            if let Ok(v19) = core.cast::<ICoreWebView2_19>() {
                let _ = v19.SetMemoryUsageTargetLevel(if awake { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL } else { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW });
            }
            if !awake {
                if let Some(v3) = &v3 {
                    // Best effort: a page playing sound stays awake.
                    let _ = v3.TrySuspend(&TrySuspendCompletedHandler::create(Box::new(|_, _| Ok(()))));
                }
            }
        }
    });
}

#[cfg(not(windows))]
fn set_awake(_w: &WebviewWindow, _awake: bool) {}

/// Whether another program fills the whole monitor this window is on, in
/// front of it (a full-screen game or video).
#[cfg(windows)]
fn covered_by_fullscreen(w: &WebviewWindow) -> bool {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONULL};
    use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId};
    let Ok(ours) = w.hwnd() else { return false };
    let ours = HWND(ours.0 as _);
    // SAFETY: plain queries on window handles; buffers are local.
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() || fg == ours {
            return false;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(fg, Some(&mut pid));
        if pid == std::process::id() {
            return false;
        }
        // The desktop and taskbar cover the monitor too, but sit behind us.
        let mut class = [0u16; 32];
        let n = GetClassNameW(fg, &mut class).max(0) as usize;
        if matches!(String::from_utf16_lossy(&class[..n]).as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
            return false;
        }
        let monitor = MonitorFromWindow(fg, MONITOR_DEFAULTTONULL);
        if monitor.is_invalid() || monitor != MonitorFromWindow(ours, MONITOR_DEFAULTTONULL) {
            return false;
        }
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let mut r = RECT::default();
        if !GetMonitorInfoW(monitor, &mut info).as_bool() || GetWindowRect(fg, &mut r).is_err() {
            return false;
        }
        let m = info.rcMonitor;
        r.left <= m.left && r.top <= m.top && r.right >= m.right && r.bottom >= m.bottom
    }
}

#[cfg(not(windows))]
fn covered_by_fullscreen(_w: &WebviewWindow) -> bool {
    false
}
