//! OmniHub desktop shell: window, tray, hotkeys and the command bridge to
//! `omnihub-core`.

mod commands;
mod overlay;
mod shortcuts;
mod tray;

use std::sync::Arc;
use std::time::Duration;

use omnihub_core::core::{AppCore, CoreOptions};
use omnihub_core::paths::AppPaths;
use tauri::{Emitter, Manager, WindowEvent};

fn init_logging(paths: &AppPaths) {
    use tracing_subscriber::prelude::*;
    let file = std::fs::OpenOptions::new().create(true).append(true).open(paths.logs.join("omnihub.log")).ok();
    let filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tao=warn,wry=warn".into());
    let registry = tracing_subscriber::registry().with(filter);
    match file {
        Some(f) => {
            let _ = registry.with(tracing_subscriber::fmt::layer().with_ansi(false).with_writer(std::sync::Mutex::new(f))).try_init();
        }
        None => {
            let _ = registry.with(tracing_subscriber::fmt::layer()).try_init();
        }
    }
}

/// Topics worth a desktop notification when the window is hidden.
fn notification_for(topic: &str, payload: &serde_json::Value) -> Option<(String, String)> {
    match topic {
        "remote:paired" => Some(("Phone paired".into(), format!("{} can now connect to this PC.", payload["name"].as_str().unwrap_or("A phone")))),
        "transfer:done" if payload["direction"] == "upload" => {
            Some((format!("Received from {}", payload["device"].as_str().unwrap_or("phone")), payload["name"].as_str().unwrap_or("file").to_string()))
        }
        "power:pending" => {
            let by = payload["requestedBy"].as_str().unwrap_or("");
            by.starts_with("phone:").then(|| (format!("{} requested by {}", payload["label"].as_str().unwrap_or("Power action"), by.trim_start_matches("phone:")), "Open OmniHub to cancel.".into()))
        }
        "screen:viewers" => payload.as_array().filter(|v| !v.is_empty()).map(|v| ("Screen is being shared".into(), format!("{} viewing your screen.", v[0]["device"].as_str().unwrap_or("A phone")))),
        _ => None,
    }
}

pub fn run(args: Vec<String>) {
    // An elevated instance started by "Restart as administrator" waits for
    // the old one to exit so the single-instance lock is free.
    if let Some(pos) = args.iter().position(|a| a == "--replace") {
        if let Some(pid) = args.get(pos + 1).and_then(|p| p.parse::<u32>().ok()) {
            omnihub_core::system::elevation::wait_for_pid_exit(pid, Duration::from_secs(10));
        }
    }
    let start_hidden = args.iter().any(|a| a == "--minimized");

    let paths = AppPaths::default_for_user().expect("cannot create the OmniHub data folder");
    init_logging(&paths);
    let core = AppCore::new(paths, CoreOptions::default()).expect("cannot open the OmniHub database");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| tray::show_main(app)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::default().with_denylist(&["overlay"]).build())
        .manage(core.clone())
        .setup(move |app| {
            let handle = app.handle().clone();

            // Forward core events to the windows (and notify when hidden).
            let mut rx = core.events.subscribe();
            let h = handle.clone();
            std::thread::Builder::new()
                .name("event-forwarder".into())
                .spawn(move || loop {
                    match rx.blocking_recv() {
                        Ok(ev) => {
                            let _ = h.emit(&ev.topic, &ev.payload);
                            let hidden = h.get_webview_window("main").and_then(|w| w.is_visible().ok()).is_none_or(|v| !v);
                            if hidden {
                                if let Some((title, body)) = notification_for(&ev.topic, &ev.payload) {
                                    shortcuts::notify(&h, &title, &body);
                                }
                            }
                            if matches!(ev.topic.as_str(), "vault:locked" | "vault:unlocked" | "remote:status") {
                                tray::refresh(&h);
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                })?;

            tray::create(&handle)?;
            shortcuts::register(&handle);
            core.start_background();

            // Autostart passes --minimized; "Start minimized" decides whether
            // that start stays in the tray. A normal launch always shows.
            let settings = core.settings.get();
            if let Some(w) = app.get_webview_window("main") {
                if !(start_hidden && settings.general.start_minimized) {
                    let _ = w.show();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if let WindowEvent::CloseRequested { api, .. } = event {
                let core = window.state::<Arc<AppCore>>();
                if core.settings.get().general.close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    core.remote.stop();
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::system_stats,
            commands::settings_get,
            commands::settings_update,
            commands::open_path,
            commands::reveal_path,
            commands::open_url,
            commands::restart_elevated,
            commands::audit_list,
            commands::audit_clear,
            commands::pick_folder,
            commands::pick_files,
            commands::pick_save_file,
            commands::set_autostart,
            commands::hide_window,
            commands::storage_volumes,
            commands::storage_scan,
            commands::storage_progress,
            commands::storage_cancel,
            commands::storage_scans,
            commands::storage_summary,
            commands::storage_open_cached,
            commands::storage_children,
            commands::storage_treemap,
            commands::storage_top_files,
            commands::storage_extensions,
            commands::storage_search,
            commands::storage_path,
            commands::storage_cleanup,
            commands::storage_delete,
            commands::storage_empty_recycle_bin,
            commands::storage_duplicates_start,
            commands::storage_duplicates_progress,
            commands::storage_duplicates_result,
            commands::storage_duplicates_cancel,
            commands::apps_list,
            commands::apps_icon,
            commands::apps_launch,
            commands::apps_uninstall,
            commands::apps_screenshots,
            commands::shots_list,
            commands::shots_capture,
            commands::shots_region_begin,
            commands::shots_region_pending,
            commands::shots_region_commit,
            commands::shots_region_cancel,
            commands::shots_thumb,
            commands::shots_image,
            commands::shots_update,
            commands::shots_delete,
            commands::shots_copy,
            commands::shots_sync,
            commands::notes_list,
            commands::notes_get,
            commands::notes_save,
            commands::notes_delete,
            commands::notes_tags,
            commands::notes_export,
            commands::notes_folder_files,
            commands::notes_folder_read,
            commands::vault_status,
            commands::vault_create,
            commands::vault_unlock,
            commands::vault_lock,
            commands::vault_touch,
            commands::vault_list,
            commands::vault_get,
            commands::vault_save,
            commands::vault_delete,
            commands::vault_copy,
            commands::vault_generate,
            commands::vault_strength,
            commands::vault_change_password,
            commands::vault_hello_enable,
            commands::vault_hello_disable,
            commands::vault_hello_unlock,
            commands::vault_export,
            commands::vault_import,
            commands::remote_status,
            commands::remote_start,
            commands::remote_stop,
            commands::remote_pair_begin,
            commands::remote_pair_cancel,
            commands::remote_devices,
            commands::remote_device_rename,
            commands::remote_device_revoke,
            commands::remote_send,
            commands::remote_inbox,
            commands::remote_stop_viewer,
            commands::remote_stop_all_viewers,
            commands::power_schedule,
            commands::power_cancel,
            commands::power_pending,
            commands::screen_monitors,
            commands::screen_presets,
            commands::scrcpy_status,
            commands::scrcpy_launch,
            commands::scrcpy_stop,
            commands::scrcpy_wireless,
            commands::scrcpy_connect,
            commands::scrcpy_pair,
            commands::sunshine_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OmniHub");
}
