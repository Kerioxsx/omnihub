//! While OmniHub sits in the tray (or is minimised), its window's web view
//! is asked to use as little memory as it can: WebView2's low memory target
//! drops caches and trims its processes. Back to normal when it shows.
//! Timers in the window pause by themselves while it is hidden.

use std::time::Duration;

use tauri::{AppHandle, Manager};

pub fn start(app: AppHandle) {
    std::thread::Builder::new()
        .name("window-rest".into())
        .spawn(move || {
            let mut resting: Option<bool> = None;
            loop {
                if let Some(w) = app.get_webview_window("main") {
                    let hidden = !w.is_visible().unwrap_or(true) || w.is_minimized().unwrap_or(false);
                    if resting != Some(hidden) {
                        set_low_memory(&w, hidden);
                        resting = Some(hidden);
                    }
                }
                std::thread::sleep(Duration::from_secs(2));
            }
        })
        .ok();
}

#[cfg(windows)]
fn set_low_memory(w: &tauri::WebviewWindow, low: bool) {
    let _ = w.with_webview(move |view| {
        use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL};
        use windows::core::Interface;
        // SAFETY: COM calls on the web view's own thread (with_webview runs there).
        unsafe {
            let Ok(core) = view.controller().CoreWebView2() else { return };
            if let Ok(v19) = core.cast::<ICoreWebView2_19>() {
                let _ = v19.SetMemoryUsageTargetLevel(if low { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW } else { COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL });
            }
        }
    });
}

#[cfg(not(windows))]
fn set_low_memory(_w: &tauri::WebviewWindow, _low: bool) {}
