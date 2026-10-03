//! System-wide screenshot hotkeys (configurable in Settings → Screenshots).

use std::str::FromStr;
use std::sync::Arc;

use omnihub_core::capture::screenshots::CaptureKind;
use omnihub_core::core::AppCore;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_notification::NotificationExt;

/// (Re)register the hotkeys from the current settings.
pub fn register(app: &AppHandle) {
    let core = app.state::<Arc<AppCore>>().inner().clone();
    let s = core.settings.get().screenshots;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    for (spec, action) in [(s.hotkey_region, Action::Region), (s.hotkey_full, Action::Capture(CaptureKind::Screen)), (s.hotkey_window, Action::Capture(CaptureKind::Window))] {
        let Ok(shortcut) = Shortcut::from_str(&spec) else {
            tracing::warn!("invalid hotkey {spec:?}");
            continue;
        };
        let res = gs.on_shortcut(shortcut, move |app, _sc, event| {
            if event.state() != ShortcutState::Pressed {
                return;
            }
            run(app.clone(), action);
        });
        if let Err(e) = res {
            tracing::warn!("could not register hotkey {spec}: {e}");
        }
    }
}

#[derive(Clone, Copy)]
enum Action {
    Region,
    Capture(CaptureKind),
}

fn run(app: AppHandle, action: Action) {
    match action {
        Action::Region => {
            tauri::async_runtime::spawn(async move {
                if let Err(e) = crate::overlay::begin(app.clone(), false).await {
                    notify(&app, "Screenshot failed", &e);
                }
            });
        }
        Action::Capture(kind) => {
            let core = app.state::<Arc<AppCore>>().inner().clone();
            std::thread::spawn(move || {
                let s = core.settings.get().screenshots;
                match core.screenshots.capture(kind, &core.screenshot_dir(), s.format) {
                    Ok(shot) => {
                        if s.copy_to_clipboard {
                            let _ = core.screenshots.copy_to_clipboard(&shot.id);
                        }
                        notify(&app, "Screenshot saved", &shot.path);
                    }
                    Err(e) => notify(&app, "Screenshot failed", &e.to_string()),
                }
            });
        }
    }
}

pub fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}
