//! OmniHub desktop shell: window, tray, hotkeys and the command bridge to
//! `omnihub-core`.

mod commands;
mod ambient;
mod govee;
mod chats;
mod mcserver;
mod mods;
mod overlay;
mod rest;
mod rig;
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
        "mcserver:guard" => Some((
            format!("{} wants in on {}", payload["name"].as_str().unwrap_or("Someone"), payload["server"].as_str().unwrap_or("your server")),
            "That name is an operator and joined from a new place, so it was kept out. Open OmniHub (Servers → Players) to allow it.".into(),
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
    let rig_hub = core.rig.clone();
    let govee_hub = core.govee.clone();
    let server_hub = core.mcserver.clone();
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
        // Size, place and maximised are remembered; full screen is not (closed or
        // updated during full-screen Aurora, it would come back full screen
        // with no player open to leave it).
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(tauri_plugin_window_state::StateFlags::all() & !tauri_plugin_window_state::StateFlags::FULLSCREEN)
                .with_denylist(&["overlay"])
                .with_filter(|label| !label.starts_with("ambient-"))
                .build(),
        )
        .manage(core.clone())
        .register_asynchronous_uri_scheme_protocol("rigmedia", {
            let core = core.clone();
            move |_ctx, request, responder| {
                let core = core.clone();
                std::thread::spawn(move || responder.respond(rig::serve(&core, &request)));
            }
        })
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
                            // Resting windows get theirs when they wake.
                            rest::deliver(&h, &ev.topic, &ev.payload);
                            let hidden = !rest::main_shown();
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
            let servers = core.mcserver.clone();
            core.updater.set_exit(move || {
                // Servers running now come back after the update.
                servers.remember_running();
                exit_handle.exit(0)
            });
            // Automatic updates wait while the window is open (it would close on you).
            let window_handle = handle.clone();
            core.updater.set_in_use(move || window_handle.get_webview_window("main").is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false)));
            core.start_background();
            ambient::start(handle.clone());
            rig::start(handle.clone());
            rest::start(handle.clone());
            // Capri starts with OmniHub while she is switched on.
            if core.settings.get().chats.capri {
                let c = core.clone();
                std::thread::spawn(move || {
                    if let Err(e) = omnihub_core::companion::start(&c.procs) {
                        tracing::warn!("capri: {e}");
                    }
                });
            }

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
                    rest::show(&w);
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
                        // Minecraft servers save their worlds first (Windows waits a few seconds).
                        c.mcserver.shutdown(Duration::from_secs(8));
                        omnihub_core::crashlog::end_session(&session_logs);
                        h.exit(0);
                    });
                }
            }
            handle_send_to(&handle, &launch_args);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Clicked back to: awake at once, not at the next check.
            if let WindowEvent::Focused(true) = event {
                if let Some(w) = window.app_handle().get_webview_window(window.label()) {
                    rest::wake(&w);
                }
            }
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
            commands::media_video,
            commands::media_video_stream,
            commands::media_video_sync,
            commands::media_mirror,
            commands::media_mirror_frame,
            commands::media_motion,
            commands::aurora_look,
            commands::aurora_choose_font,
            commands::aurora_choose_emoji_pack,
            commands::aurora_emoji_font,
            commands::aurora_forget_look,
            commands::aurora_font,
            commands::aurora_texture,
            commands::media_stream_status,
            commands::media_art,
            commands::media_audio,
            commands::media_set_volume,
            commands::visual_hold,
            commands::visual_release,
            commands::visual_status,
            commands::visual_keep_awake,
            commands::ambient_displays,
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
            commands::games_roblox_shaders,
            commands::games_roblox_shaders_apply,
            commands::games_roblox_flag_list,
            commands::games_configs,
            commands::games_config_options,
            commands::games_config_apply,
            commands::games_config_restore,
            commands::games_fortnite_clean,
            commands::displays_list,
            commands::displays_set,
            commands::shots_pin,
            commands::vault_wifi_qr,
            commands::vault_import_file,
            commands::apps_updates,
            commands::apps_update,
            commands::capri_status,
            commands::capri_set,
            commands::capri_update,
            commands::games_load_test,
            commands::games_fps,
            commands::games_fps_install,
            commands::games_fps_allow,
            commands::games_library,
            commands::games_add_installed,
            commands::pc_status,
            commands::pc_set,
            commands::browser_status,
            commands::browser_repair,
            commands::browser_pair_respond,
            commands::browser_revoke,
            commands::vault_totp,
            commands::app_last_crash,
            commands::app_log_error,
            rig::rig_specs,
            rig::rig_settings,
            rig::rig_update,
            rig::rig_hold,
            rig::rig_profiles,
            rig::rig_save_profile,
            rig::rig_delete_profile,
            rig::rig_lhm_login,
            rig::rig_start_openrgb,
            rig::rig_media,
            rig::rig_media_add,
            rig::rig_media_remove,
            rig::rig_displays,
            govee::govee_settings,
            govee::govee_update,
            govee::govee_update_light,
            govee::govee_hold,
            govee::govee_scan,
            govee::govee_set_key,
            govee::govee_control,
            govee::govee_identify,
            govee::govee_presets,
            govee::govee_save_preset,
            govee::govee_delete_preset,
            govee::govee_apply_preset,
            govee::govee_save_timer,
            govee::govee_delete_timer,
            govee::govee_sleep,
            govee::govee_monitors,
            govee::govee_firewall,
            govee::govee_fix_firewall,
            chats::chats_list,
            chats::chats_view,
            chats::chats_send,
            chats::chats_create,
            chats::chats_stop,
            chats::chats_rename,
            chats::chats_delete,
            chats::chats_folders,
            chats::chats_tools,
            chats::chats_usage,
            chats::ideas_list,
            chats::ideas_save,
            chats::openai_settings,
            chats::openai_update,
            chats::openai_set_key,
            chats::openai_models,
            mods::mods_games,
            mods::mods_list,
            mods::mods_browse,
            mods::mods_install_package,
            mods::mods_install_file,
            mods::mods_install_loader,
            mods::mods_set_enabled,
            mods::mods_set_all,
            mods::mods_uninstall,
            mods::mods_update,
            mods::mods_workshop,
            mods::mods_add_folder,
            mods::mods_remove_folder,
            mods::claude_settings,
            mods::claude_update,
            mods::claude_set_key,
            mods::claude_test,
            mods::maker_projects,
            mods::maker_create,
            mods::maker_send,
            mods::maker_stop,
            mods::maker_approve,
            mods::maker_view,
            mods::maker_file,
            mods::maker_delete,
            mods::maker_rename,
            mods::maker_install,
            mods::mc_status,
            mods::mc_search,
            mods::mc_versions,
            mods::mc_add_modrinth,
            mods::mc_add_file,
            mods::mc_add_folder,
            mods::mc_launcher_packs,
            mods::mc_launcher_icon,
            mods::mc_add_launcher_pack,
            mods::mc_add_launcher_place,
            mods::mc_remove_launcher_place,
            mods::mc_import_shared,
            mods::mc_launcher_details,
            mods::mc_launcher_world_icon,
            mods::mc_launcher_updates,
            mods::mc_share_launcher_pack,
            mods::mc_remove_source,
            mods::mc_plan,
            mods::mc_export,
            mods::mc_install,
            mods::mc_uninstall,
            mods::mc_mixes,
            mods::mc_cover,
            mods::mc_remake_cover,
            mods::mc_ai_cover,
            mods::mc_save_cover,
            mods::pollinations_status,
            mods::pollinations_sign_in,
            mods::pollinations_check,
            mods::pollinations_set_key,
            mods::pollinations_forget,
            mcserver::srv_list,
            mcserver::srv_get,
            mcserver::srv_versions,
            mcserver::srv_create,
            mcserver::srv_adopt,
            mcserver::srv_update,
            mcserver::srv_delete,
            mcserver::srv_start,
            mcserver::srv_stop,
            mcserver::srv_restart,
            mcserver::srv_kill,
            mcserver::srv_console,
            mcserver::srv_command,
            mcserver::srv_properties,
            mcserver::srv_set_properties,
            mcserver::srv_set_icon,
            mcserver::srv_players,
            mcserver::srv_player,
            mcserver::srv_act,
            mcserver::srv_whitelist,
            mcserver::srv_watch,
            mcserver::srv_map,
            mcserver::srv_map_tile,
            mcserver::srv_content,
            mcserver::srv_identify,
            mcserver::srv_search,
            mcserver::srv_install,
            mcserver::srv_add_file,
            mcserver::srv_set_content,
            mcserver::srv_remove_content,
            mcserver::srv_backups,
            mcserver::srv_backup,
            mcserver::srv_restore,
            mcserver::srv_delete_backup,
            mcserver::srv_network,
            mcserver::srv_set_upnp,
            mcserver::srv_fix_firewall,
        ])
        .build(tauri::generate_context!())
        .expect("error while starting OmniHub")
        .run(move |_app, event| {
            // A normal exit (tray Quit, an update): next start is not a crash.
            if let tauri::RunEvent::Exit = event {
                // Fans OmniHub drives go back to the hardware's own control.
                rig_hub.shutdown();
                // Streaming Govee lights go back to their own modes.
                govee_hub.shutdown();
                // Minecraft servers save their worlds and stop.
                server_hub.shutdown(Duration::from_secs(25));
                omnihub_core::crashlog::end_session(&logs_dir);
            }
        });
}
