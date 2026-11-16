//! "My PC" in the shell: the commands for lighting, fans, screens and
//! specs, the window that fills a case screen (a monitor inside the case,
//! route `#/casescreen`), and `rigmedia://`, which hands that window the
//! library's pictures and videos by id (with byte ranges, so videos seek).

use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use std::time::Duration;

use omnihub_core::core::AppCore;
use omnihub_core::rig::lighting::LightProfile;
use omnihub_core::rig::screens::{self, Media};
use omnihub_core::rig::specs::Specs;
use omnihub_core::rig::{RigSettings, RigStatus, ScreenShow};
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

type Core<'a> = State<'a, Arc<AppCore>>;
type Res<T> = Result<T, String>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Res<T> + Send + 'static) -> Res<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

const LABEL: &str = "casescreen";

#[tauri::command]
pub async fn rig_specs(core: Core<'_>, refresh: bool) -> Res<Specs> {
    let core = core.inner().clone();
    blocking(move || Ok(core.rig.specs(refresh))).await
}

#[tauri::command]
pub async fn rig_settings(core: Core<'_>) -> Res<RigSettings> {
    Ok(core.rig.settings())
}

#[tauri::command]
pub async fn rig_update(core: Core<'_>, patch: serde_json::Value) -> Res<RigSettings> {
    let core = core.inner().clone();
    blocking(move || core.rig.update(&patch)).await
}

/// The page is open (asked every few seconds): readings keep coming.
#[tauri::command]
pub async fn rig_hold(core: Core<'_>) -> Res<RigStatus> {
    Ok(core.rig.hold())
}

#[tauri::command]
pub async fn rig_profiles(core: Core<'_>) -> Res<Vec<LightProfile>> {
    Ok(core.rig.profiles())
}

#[tauri::command]
pub async fn rig_save_profile(core: Core<'_>, profile: LightProfile) -> Res<LightProfile> {
    let core = core.inner().clone();
    blocking(move || core.rig.save_profile(profile)).await
}

#[tauri::command]
pub async fn rig_delete_profile(core: Core<'_>, id: String) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || core.rig.delete_profile(&id)).await
}

#[tauri::command]
pub async fn rig_lhm_login(core: Core<'_>, user: String, password: String) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || core.rig.set_lhm_login(&user, &password)).await
}

/// Start OpenRGB with its SDK server (it stays in the tray).
#[tauri::command]
pub async fn rig_start_openrgb(core: Core<'_>) -> Res<()> {
    let exe = omnihub_core::rig::openrgb_installed().ok_or("OpenRGB isn't installed. Get it from openrgb.org.")?;
    let mut c = std::process::Command::new(&exe);
    c.args(["--server", "--startminimized"]);
    if let Some(dir) = exe.parent() {
        c.current_dir(dir);
    }
    c.spawn().map_err(|e| format!("OpenRGB did not start: {e}"))?;
    core.audit.record("desktop", "rig.openrgb", "start", true);
    Ok(())
}

#[tauri::command]
pub async fn rig_media(core: Core<'_>) -> Res<Vec<Media>> {
    Ok(core.rig.library.list())
}

/// Pick a picture, GIF or video on this PC for the screens.
#[tauri::command]
pub async fn rig_media_add(app: AppHandle, core: Core<'_>) -> Res<Option<Media>> {
    let core = core.inner().clone();
    blocking(move || {
        let picked = app.dialog().file().set_title("Choose a picture, GIF or video for your PC's screen").add_filter("Pictures and videos", &["png", "jpg", "jpeg", "gif", "webp", "mp4", "webm"]).blocking_pick_file();
        let Some(path) = picked.and_then(|p| p.into_path().ok()) else { return Ok(None) };
        let m = core.rig.library.import(&path)?;
        core.audit.record("desktop", "rig.media", &m.name, true);
        Ok(Some(m))
    })
    .await
}

#[tauri::command]
pub async fn rig_media_remove(core: Core<'_>, id: String) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || {
        let s = core.rig.settings();
        let mut patch = serde_json::Map::new();
        let off = serde_json::json!({ "kind": "off" });
        if s.screens.monitor_show == (ScreenShow::Media { id: id.clone() }) {
            patch.insert("monitorShow".into(), off.clone());
        }
        if s.screens.lcd_show == (ScreenShow::Media { id: id.clone() }) {
            patch.insert("lcdShow".into(), serde_json::json!({ "kind": "liquid" }));
        }
        if !patch.is_empty() {
            core.rig.update(&serde_json::json!({ "screens": patch }))?;
        }
        core.rig.library.remove(&id)
    })
    .await
}

/// The monitors, to pick the one inside the case.
#[tauri::command]
pub async fn rig_displays(app: AppHandle) -> Res<Vec<crate::ambient::AmbientDisplay>> {
    Ok(crate::ambient::displays(&app))
}

/// `rigmedia://localhost/<id>`: a library file, by id only.
pub fn serve(core: &AppCore, request: &tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::{Response, StatusCode};
    let fail = |code: StatusCode| Response::builder().status(code).body(Vec::new()).unwrap_or_default();
    let id = request.uri().path().trim_start_matches('/');
    let Some(path) = core.rig.library.path(id) else { return fail(StatusCode::NOT_FOUND) };
    let Ok(mut f) = std::fs::File::open(&path) else { return fail(StatusCode::NOT_FOUND) };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let asked = request.headers().get("range").and_then(|v| v.to_str().ok());
    // Pictures whole; videos 4 MB at a time.
    let chunk = if screens::mime_of(id).starts_with("video/") { 4 << 20 } else { screens::PICTURE_MAX };
    let Some((start, end)) = screens::range(asked, len, chunk) else {
        return Response::builder().status(StatusCode::RANGE_NOT_SATISFIABLE).header("Content-Range", format!("bytes */{len}")).body(Vec::new()).unwrap_or_default();
    };
    let mut body = vec![0u8; (end - start + 1) as usize];
    if f.seek(SeekFrom::Start(start)).and_then(|_| f.read_exact(&mut body)).is_err() {
        return fail(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let partial = asked.is_some() || end + 1 < len;
    Response::builder()
        .status(if partial { StatusCode::PARTIAL_CONTENT } else { StatusCode::OK })
        .header("Content-Type", screens::mime_of(id))
        .header("Accept-Ranges", "bytes")
        .header("Content-Length", body.len().to_string())
        .header("Content-Range", format!("bytes {start}-{end}/{len}"))
        .header("Cache-Control", "no-store")
        .body(body)
        .unwrap_or_default()
}

/// Keep the case-screen window on the chosen monitor, or closed.
pub fn start(app: AppHandle) {
    let core = app.state::<Arc<AppCore>>().inner().clone();
    std::thread::Builder::new()
        .name("case-screen".into())
        .spawn(move || {
            let mut placed: Option<(tauri::PhysicalPosition<i32>, tauri::PhysicalSize<u32>)> = None;
            loop {
                let s = core.rig.settings().screens;
                let target = if s.monitor.is_empty() || s.monitor_show == ScreenShow::Off {
                    None
                } else {
                    app.available_monitors().unwrap_or_default().into_iter().find(|m| m.name().is_some_and(|n| n == &s.monitor)).map(|m| (*m.position(), *m.size()))
                };
                let window = app.get_webview_window(LABEL);
                match (target, window) {
                    (None, Some(w)) => {
                        let _ = w.destroy();
                        placed = None;
                    }
                    (Some((pos, size)), None) => {
                        let built = WebviewWindowBuilder::new(&app, LABEL, WebviewUrl::App("index.html#/casescreen".into()))
                            .title("OmniHub case screen")
                            .decorations(false)
                            .skip_taskbar(true)
                            .resizable(false)
                            .shadow(false)
                            .focused(false)
                            .focusable(false)
                            .background_color(tauri::webview::Color(0, 0, 0, 255))
                            .visible(false)
                            .build();
                        if let Ok(w) = built {
                            let _ = w.set_position(pos);
                            let _ = w.set_size(size);
                            let _ = w.show();
                            placed = Some((pos, size));
                        }
                    }
                    (Some((pos, size)), Some(w)) if placed != Some((pos, size)) => {
                        let _ = w.set_position(pos);
                        let _ = w.set_size(size);
                        placed = Some((pos, size));
                    }
                    _ => {}
                }
                std::thread::sleep(Duration::from_secs(3));
            }
        })
        .ok();
}
