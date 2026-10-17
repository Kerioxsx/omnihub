//! The glow around the screen: a transparent, click-through, always-on-top
//! window per monitor, over every app (UI in src/desktop, route `#/ambient`).
//! A watcher keeps the windows in line with the settings (on/off, which
//! displays, above the taskbar or not), the monitors (added, removed,
//! rearranged, rescaled) and what is on screen: the windows step aside while
//! a game, video or presentation fills the screen and during game boosts.
//!
//! Limits: Windows draws exclusive full-screen games above every window, so
//! the glow can't show over those (it hides instead); and it never takes
//! focus or clicks.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use omnihub_core::core::AppCore;
use serde::Serialize;
use tauri::{AppHandle, Manager, Monitor, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

const PREFIX: &str = "ambient-";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AmbientDisplay {
    pub name: String,
    pub label: String,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub primary: bool,
}

fn monitors(app: &AppHandle) -> (Vec<Monitor>, Option<String>) {
    let list = app.available_monitors().unwrap_or_default();
    let primary = app.primary_monitor().ok().flatten().and_then(|m| m.name().cloned());
    (list, primary)
}

/// The monitors, for the display picker.
pub fn displays(app: &AppHandle) -> Vec<AmbientDisplay> {
    let (list, primary) = monitors(app);
    list.iter()
        .enumerate()
        .map(|(i, m)| {
            let name = m.name().cloned().unwrap_or_else(|| format!("Display {}", i + 1));
            let is_primary = primary.as_deref() == Some(name.as_str());
            let s = m.size();
            AmbientDisplay { label: format!("Display {} · {}×{}{}", i + 1, s.width, s.height, if is_primary { " (main)" } else { "" }), name, width: s.width, height: s.height, scale: m.scale_factor(), primary: is_primary }
        })
        .collect()
}

/// A window label from a monitor name ("\\.\DISPLAY1" → "ambient-DISPLAY1").
fn label_for(name: &str, i: usize) -> String {
    let clean: String = name.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    format!("{PREFIX}{}", if clean.is_empty() { format!("display{i}") } else { clean })
}

/// Where each glow window goes: label → (position, size), physical pixels.
fn targets(app: &AppHandle, display: &str, clear_taskbar: bool) -> HashMap<String, (PhysicalPosition<i32>, PhysicalSize<u32>)> {
    let (list, primary) = monitors(app);
    let mut out = HashMap::new();
    for (i, m) in list.iter().enumerate() {
        let name = m.name().cloned().unwrap_or_default();
        let wanted = match display {
            "all" => true,
            "primary" | "" => primary.as_deref().map_or(i == 0, |p| p == name),
            other => other == name,
        };
        if !wanted {
            continue;
        }
        let (pos, size) = if clear_taskbar {
            let wa = m.work_area();
            (wa.position, wa.size)
        } else {
            (*m.position(), *m.size())
        };
        out.insert(label_for(&name, i), (pos, size));
    }
    // A monitor that was chosen by name but is unplugged: fall back to the main one.
    if out.is_empty() && display != "all" && !list.is_empty() {
        return targets(app, "primary", clear_taskbar);
    }
    out
}

fn open(app: &AppHandle, label: &str, pos: PhysicalPosition<i32>, size: PhysicalSize<u32>) -> Option<tauri::WebviewWindow> {
    let w = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html#/ambient".into()))
        .title("OmniHub glow")
        .transparent(true)
        .background_color(tauri::webview::Color(0, 0, 0, 0))
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .focusable(false)
        .visible(false)
        .build();
    match w {
        Ok(w) => {
            let _ = w.set_position(pos);
            let _ = w.set_size(size);
            let _ = w.set_ignore_cursor_events(true);
            Some(w)
        }
        Err(e) => {
            tracing::warn!("could not open the screen glow: {e}");
            None
        }
    }
}

/// Follow the settings, the monitors and full-screen apps (runs for the app's life).
pub fn start(app: AppHandle) {
    let core = app.state::<Arc<AppCore>>().inner().clone();
    // "Turn on when OmniHub starts" off: the glow starts off.
    let v = core.settings.get().visuals;
    if v.desktop.enabled && !v.desktop.start_with_app {
        let _ = core.update_settings(&serde_json::json!({ "visuals": { "desktop": { "enabled": false } } }));
    }
    std::thread::Builder::new()
        .name("screen-glow".into())
        .spawn(move || {
            let mut placed: HashMap<String, (PhysicalPosition<i32>, PhysicalSize<u32>)> = HashMap::new();
            let mut hidden = true;
            loop {
                let v = core.settings.get().visuals;
                let d = &v.desktop;
                let want = if v.enabled && d.enabled { targets(&app, &d.display, d.clear_taskbar) } else { HashMap::new() };
                // Close what is no longer wanted.
                for (label, w) in app.webview_windows() {
                    if label.starts_with(PREFIX) && !want.contains_key(&label) {
                        let _ = w.destroy();
                        placed.remove(&label);
                    }
                }
                let step_aside = d.hide_fullscreen && (omnihub_core::system::awake::fullscreen_app_running() || core.games.session().is_some_and(|s| s.active()));
                for (label, (pos, size)) in &want {
                    match app.get_webview_window(label) {
                        None => {
                            if open(&app, label, *pos, *size).is_some() {
                                placed.insert(label.clone(), (*pos, *size));
                                hidden = true;
                            }
                        }
                        Some(w) if placed.get(label) != Some(&(*pos, *size)) => {
                            let _ = w.set_position(*pos);
                            let _ = w.set_size(*size);
                            placed.insert(label.clone(), (*pos, *size));
                        }
                        _ => {}
                    }
                }
                let show = !want.is_empty() && !step_aside;
                if show == hidden {
                    for (label, w) in app.webview_windows() {
                        if label.starts_with(PREFIX) {
                            let _ = if show { w.show() } else { w.hide() };
                            if show {
                                // Showing can bring back the cursor; keep clicks going through.
                                let _ = w.set_ignore_cursor_events(true);
                            }
                        }
                    }
                    hidden = !show;
                }
                std::thread::sleep(Duration::from_millis(if want.is_empty() { 2000 } else { 1000 }));
            }
        })
        .ok();
}
