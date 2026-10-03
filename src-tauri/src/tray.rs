//! The notification-area icon and its menu.

use std::sync::Arc;

use omnihub_core::core::AppCore;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

const TRAY_ID: &str = "omnihub";

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn navigate(app: &AppHandle, route: &str) {
    show_main(app);
    let _ = app.emit_to("main", "app:navigate", route);
}

fn build_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let core = app.state::<Arc<AppCore>>().inner().clone();
    let s = core.settings.get();
    let remote_on = core.remote.is_running();
    let vault_open = core.vault.is_unlocked();
    let open = MenuItem::with_id(app, "open", "Open OmniHub", true, None::<&str>)?;
    let shot = MenuItem::with_id(app, "screenshot", format!("Screenshot region\t{}", s.screenshots.hotkey_region), true, None::<&str>)?;
    let scan = MenuItem::with_id(app, "scan", "Analyse storage", true, None::<&str>)?;
    let idea = MenuItem::with_id(app, "idea", "New idea for Claude", true, None::<&str>)?;
    let remote = CheckMenuItem::with_id(app, "remote", "Phone companion", true, remote_on, None::<&str>)?;
    let pair = MenuItem::with_id(app, "pair", "Pair a phone…", remote_on, None::<&str>)?;
    let lock = MenuItem::with_id(app, "lock", "Lock vault", vault_open, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit OmniHub", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    Menu::with_items(app, &[&open, &sep1, &shot, &scan, &idea, &remote, &pair, &lock, &sep2, &quit])
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let icon = app.default_window_icon().cloned().expect("bundled window icon");
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("OmniHub")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let core = app.state::<Arc<AppCore>>().inner().clone();
            match event.id().as_ref() {
                "open" => show_main(app),
                "screenshot" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = crate::overlay::begin(app.clone(), false).await {
                            crate::shortcuts::notify(&app, "Screenshot failed", &e);
                        }
                    });
                }
                "scan" => navigate(app, "storage"),
                "idea" => navigate(app, "notes?new=idea"),
                "pair" => navigate(app, "phone?pair=1"),
                "remote" => {
                    let app = app.clone();
                    std::thread::spawn(move || {
                        let on = !core.remote.is_running();
                        let res = core.update_settings(&serde_json::json!({ "remote": { "enabled": on } }));
                        if on && res.is_ok() && !core.remote.is_running() {
                            let _ = core.remote.start(core.clone());
                        }
                        if !on {
                            core.remote.stop();
                        }
                        if let Err(e) = res {
                            crate::shortcuts::notify(&app, "Phone companion", &e.to_string());
                        }
                        refresh(&app);
                    });
                }
                "lock" => {
                    core.vault.lock();
                    refresh(app);
                }
                "quit" => {
                    core.remote.stop();
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                let app = tray.app_handle();
                match app.get_webview_window("main") {
                    Some(w) if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) => {
                        let _ = w.hide();
                    }
                    _ => show_main(app),
                }
            }
        })
        .build(app)?;
    Ok(())
}

/// Rebuild the menu so check marks and enabled states match reality.
pub fn refresh(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), build_menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
}
