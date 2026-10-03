//! The region-capture overlay: a borderless, always-on-top window covering
//! the monitor under the pointer, showing the frozen screen to drag a
//! rectangle on (UI in src/desktop, route `#/overlay`).

use std::sync::Arc;
use std::time::Duration;

use omnihub_core::capture::screenshots::PendingRegion;
use omnihub_core::core::AppCore;
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

/// The frozen screen waiting for a selection, and whether the main window
/// was hidden for the capture (so it can come back afterwards).
static PENDING: Mutex<Option<(PendingRegion, bool)>> = Mutex::new(None);

pub fn pending(_core: &AppCore) -> Option<PendingRegion> {
    PENDING.lock().as_ref().map(|p| p.0.clone())
}

/// Freeze the screen and open the overlay. `from_app` hides the main window
/// first so it does not end up in the capture.
pub async fn begin(app: AppHandle, from_app: bool) -> Result<(), String> {
    let core = app.state::<Arc<AppCore>>().inner().clone();
    let main = app.get_webview_window("main");
    let hide_main = from_app && main.as_ref().and_then(|w| w.is_visible().ok()).unwrap_or(false);
    if hide_main {
        if let Some(w) = &main {
            let _ = w.hide();
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let region = tauri::async_runtime::spawn_blocking(move || core.screenshots.begin_region())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string());
    let region = match region {
        Ok(r) => r,
        Err(e) => {
            if hide_main {
                if let Some(w) = &main {
                    let _ = w.show();
                }
            }
            return Err(e);
        }
    };
    *PENDING.lock() = Some((region.clone(), hide_main));

    let scale = region.scale.max(0.5) as f64;
    let (x, y) = (region.x as f64 / scale, region.y as f64 / scale);
    let (w, h) = (region.width as f64 / scale, region.height as f64 / scale);
    let window = match app.get_webview_window("overlay") {
        Some(win) => win,
        None => WebviewWindowBuilder::new(&app, "overlay", WebviewUrl::App("index.html#/overlay".into()))
            .title("OmniHub capture")
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .shadow(false)
            .visible(false)
            .position(x, y)
            .inner_size(w, h)
            .build()
            .map_err(|e| e.to_string())?,
    };
    let _ = window.set_position(tauri::LogicalPosition::new(x, y));
    let _ = window.set_size(tauri::LogicalSize::new(w, h));
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit_to("overlay", "region:pending", &region);
    Ok(())
}

/// Close the overlay and bring the main window back if we hid it.
pub fn finish(app: &AppHandle) {
    let restore = PENDING.lock().take().map(|p| p.1).unwrap_or(false);
    if let Some(w) = app.get_webview_window("overlay") {
        let _ = w.hide();
        let _ = w.close();
    }
    if restore {
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}
