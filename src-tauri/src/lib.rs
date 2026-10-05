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

/// Panics go to the log file (there is no console), with where they happened.
fn log_panics() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("panic: {info}\n{}", std::backtrace::Backtrace::force_capture());
        default(info);
    }));
}

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
        "notes:reminder" => Some((
            format!("Reminder: {}", payload["title"].as_str().filter(|t| !t.is_empty()).unwrap_or("Untitled note")),
            payload["snippet"].as_str().filter(|s| !s.is_empty()).unwrap_or("Open OmniHub to see the note.").to_string(),
        )),
        "storage:low-space" => Some((
            format!("{} is almost full", payload["root"].as_str().unwrap_or("A drive").trim_end_matches('\\')),
            format!("Only {} left. Open OmniHub to see what takes the space.", omnihub_core::storage::format_bytes(payload["free"].as_u64().unwrap_or(0))),
        )),
        "screen:viewers" => payload.as_array().filter(|v| !v.is_empty()).map(|v| ("Screen is being shared".into(), format!("{} viewing your screen.", v[0]["device"].as_str().unwrap_or("A phone")))),
        _ => None,
    }
}

/// Explorer's "Send to → OmniHub (phone)": show the window with the files
/// ready to send. Returns whether the arguments were a send request.
fn handle_send_to(app: &tauri::AppHandle, argv: &[String]) -> bool {
    let Some(paths) = omnihub_core::system::shell::send_to_paths(argv) else { return false };
    if !paths.is_empty() {
        app.state::<commands::PendingSend>().0.lock().extend(paths.iter().cloned());
        tray::show_main(app);
        let _ = app.emit("send:files", &paths);
    }
    true
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
    let launch_args = args.clone();

    let paths = AppPaths::default_for_user().expect("cannot create the OmniHub data folder");
    init_logging(&paths);
    log_panics();
    let logs_dir = paths.logs.clone();
    let last_crash = omnihub_core::crashlog::begin_session(&logs_dir);
    if last_crash.is_some() {
        tracing::warn!("the previous run did not end normally; details in logs/last-crash.txt");
    }
    let core = AppCore::new(paths, CoreOptions::default()).expect("cannot open the OmniHub database");
    #[cfg(windows)]
    let session_logs = logs_dir.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if !handle_send_to(app, &argv) {
                tray::show_main(app);
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::default().with_denylist(&["overlay"]).build())
        .manage(core.clone())
        .manage(commands::PendingSend::default())
        .manage(commands::LastCrash(parking_lot::Mutex::new(last_crash)))
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
                            // Reminders are due now: always a Windows notification.
                            if hidden || ev.topic == "notes:reminder" {
                                if let Some((title, body)) = notification_for(&ev.topic, &ev.payload) {
                                    shortcuts::notify(&h, &title, &body);
                                }
                            }
                            if matches!(ev.topic.as_str(), "vault:locked" | "vault:unlocked" | "remote:status") {
                                tray::refresh(&h);
                            }
                            // The browser extension waits on the user: bring the window up.
                            if matches!(ev.topic.as_str(), "browser:pair-request" | "browser:unlock-request") {
                                tray::show_main(&h);
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                })?;

            tray::create(&handle)?;
            shortcuts::register(&handle);
            // The installer closes and reopens OmniHub; leave cleanly first.
            let exit_handle = handle.clone();
            core.updater.set_exit(move || exit_handle.exit(0));
            // Automatic updates wait while the window is open (it would close on you).
            let window_handle = handle.clone();
            core.updater.set_in_use(move || window_handle.get_webview_window("main").is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false)));
            core.start_background();

            // Autostart passes --minimized; "Start minimized" decides whether
            // that start stays in the tray. A normal launch always shows.
            let settings = core.settings.get();
            // "Launch at login" starts this copy, also after OmniHub moved
            // folders (say from Program Files to the user's own).
            if settings.general.launch_at_login {
                use tauri_plugin_autostart::ManagerExt;
                let _ = app.autolaunch().enable();
            }
            if let Some(w) = app.get_webview_window("main") {
                if !(start_hidden && settings.general.start_minimized) {
                    let _ = w.show();
                }
                // Windows signing out, or an installer closing OmniHub to
                // replace it: leave (closing would only hide to the tray and
                // keep omnihub.exe locked).
                #[cfg(windows)]
                if let Ok(hwnd) = w.hwnd() {
                    let (h, c) = (handle.clone(), core.clone());
                    omnihub_core::system::session_end::watch(hwnd.0 as isize, move || {
                        tracing::info!("Windows asked OmniHub to close");
                        c.remote.stop();
                        omnihub_core::crashlog::end_session(&session_logs);
                        h.exit(0);
                    });
                }
            }
            handle_send_to(&handle, &launch_args);
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
            commands::storage_thumb,
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
            commands::remote_send_text,
            commands::remote_inbox_remove,
            commands::clipboard_text,
            commands::take_pending_send,
            commands::remote_inbox,
            commands::remote_stop_viewer,
            commands::remote_stop_all_viewers,
            commands::remote_diagnostics,
            commands::remote_fix_firewall,
            commands::network_make_private,
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
            commands::airplay_status,
            commands::airplay_install,
            commands::airplay_uninstall,
            commands::airplay_start,
            commands::airplay_stop,
            commands::airplay_keep_on_top,
            commands::airplay_place,
            commands::airplay_firewall,
            commands::airplay_fix_firewall,
            commands::storage_growth,
            commands::storage_export_csv,
            commands::media_state,
            commands::media_control,
            commands::media_lyrics,
            commands::media_art,
            commands::media_audio,
            commands::media_set_volume,
            commands::settings_export,
            commands::settings_import,
            commands::screen_share_state,
            commands::screen_windows,
            commands::screen_set_paused,
            commands::screen_set_window,
            commands::vault_health,
            commands::vault_breach_check,
            commands::notes_remind,
            commands::shots_text,
            commands::shots_save_edit,
            commands::startup_list,
            commands::startup_set,
            commands::system_processes,
            commands::system_end_process,
            commands::system_usage,
            commands::system_set_priority,
            commands::storage_scan_task_status,
            commands::storage_scan_task_set,
            commands::update_state,
            commands::update_check,
            commands::update_install,
            commands::games_list,
            commands::games_create,
            commands::games_save,
            commands::games_delete,
            commands::games_state,
            commands::games_play,
            commands::games_stop,
            commands::games_ping,
            commands::games_ping_targets,
            commands::games_roblox_status,
            commands::games_roblox_write,
            commands::games_roblox_preview,
            commands::browser_status,
            commands::browser_repair,
            commands::browser_pair_respond,
            commands::browser_revoke,
            commands::vault_totp,
            commands::app_last_crash,
            commands::app_log_error,
        ])
        .build(tauri::generate_context!())
        .expect("error while starting OmniHub")
        .run(move |_app, event| {
            // A normal exit (tray Quit, an update): next start is not a crash.
            if let tauri::RunEvent::Exit = event {
                omnihub_core::crashlog::end_session(&logs_dir);
            }
        });
}
