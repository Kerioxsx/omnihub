//! Phone routes added in 0.30: file search, the gaming dashboard, voice
//! commands, the quiet schedule and sleep timer, photo import, Capri
//! Desktop, and the party queue's guest pages (no pairing, the party code
//! only).

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Path as UrlPath, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use serde::Deserialize;
use serde_json::json;

use super::{actor, display_path, kind_of, roots, ApiError, ApiResult, Ctx};
use crate::remote::auth::Device;

pub(super) fn authed() -> Router<Ctx> {
    Router::new()
        .route("/api/fs/search", get(fs_search))
        .route("/api/live", get(live))
        .route("/api/voice", post(voice))
        .route("/api/quiet/timer", post(quiet_timer))
        .route("/api/quiet/schedule", post(quiet_schedule))
        .route("/api/photos/check", post(photos_check))
        .route("/api/capri-desktop", get(capri_get).post(capri_set))
        .route("/api/party", get(party_get))
        .route("/api/sites", get(sites_list))
        .route("/api/sites/open", post(sites_open))
        .route("/api/sound/outputs", get(outputs_list).post(outputs_set))
        .route("/api/bluetooth", get(bt_list))
        .route("/api/bluetooth/connect", post(bt_connect))
        .route("/api/bluetooth/forget", post(bt_forget))
}

pub(super) fn public() -> Router<Ctx> {
    Router::new()
        .route("/party.js", get(party_js))
        .route("/party/{code}", get(party_page))
        .route("/party/{code}/queue", get(party_queue))
        .route("/party/{code}/search", get(party_search))
        .route("/party/{code}/add", post(party_add))
}

// ---------- file search ----------

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

/// Folders not worth walking (and slow): system, programs, caches.
const SKIP: &[&str] = &["windows", "program files", "program files (x86)", "programdata", "appdata", "$recycle.bin", "system volume information", "node_modules", ".git", "target", "__pycache__", "steamapps", "winsxs"];
const MAX_RESULTS: usize = 100;
const MAX_VISITED: usize = 60_000;
const MAX_DEPTH: usize = 8;

/// Files and folders under `roots` whose names hold every word of `q`.
pub fn search_names(roots: &[PathBuf], q: &str, budget: Duration) -> (Vec<(PathBuf, bool, u64)>, bool) {
    let words: Vec<String> = q.to_lowercase().split_whitespace().map(String::from).collect();
    let mut out = Vec::new();
    if words.is_empty() {
        return (out, false);
    }
    let start = Instant::now();
    let mut visited = 0;
    let mut stack: Vec<(PathBuf, usize)> = roots.iter().map(|r| (r.clone(), 0)).collect();
    let mut cut = false;
    while let Some((dir, depth)) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            visited += 1;
            if visited > MAX_VISITED || out.len() >= MAX_RESULTS || start.elapsed() > budget {
                cut = true;
                break;
            }
            let name = e.file_name().to_string_lossy().to_lowercase();
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            if words.iter().all(|w| name.contains(w.as_str())) {
                let size = if ft.is_file() { e.metadata().map(|m| m.len()).unwrap_or(0) } else { 0 };
                out.push((e.path(), ft.is_dir(), size));
            }
            if ft.is_dir() && depth < MAX_DEPTH && !name.starts_with('.') && !SKIP.contains(&name.as_str()) {
                stack.push((e.path(), depth + 1));
            }
        }
        if cut {
            break;
        }
    }
    (out, cut)
}

async fn fs_search(State(core): State<Ctx>, Query(q): Query<SearchQuery>) -> ApiResult<Json<serde_json::Value>> {
    let query = q.q.trim().to_string();
    if query.chars().count() < 2 {
        return Err(ApiError::bad("type at least two letters"));
    }
    let roots: Vec<PathBuf> = roots(&core).into_iter().map(|r| PathBuf::from(r.path)).collect();
    let (found, cut) = tokio::task::spawn_blocking(move || search_names(&roots, &query, Duration::from_secs(5))).await?;
    let items: Vec<_> = found
        .into_iter()
        .map(|(p, dir, size)| {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            json!({ "name": name, "path": display_path(&p), "dir": dir, "size": size, "kind": if dir { "folder" } else { kind_of(&name) } })
        })
        .collect();
    Ok(Json(json!({ "items": items, "more": cut })))
}

// ---------- gaming dashboard ----------

async fn live(State(core): State<Ctx>) -> ApiResult<Json<serde_json::Value>> {
    let c = core.clone();
    let v = tokio::task::spawn_blocking(move || {
        c.rig.hold();
        let rig = c.rig.status();
        let usage = c.procs.usage(crate::system::procs::ProcessSort::Cpu, 0, true);
        let gpu = usage.gpus.iter().map(|g| g.percent).fold(None, |a: Option<f32>, u| Some(a.map_or(u, |a| a.max(u))));
        let session = c.games.session();
        json!({
            "fps": c.games.fps_live(),
            "cpuTemp": rig.cpu_temp,
            "gpuTemp": rig.gpu_temp,
            "cpu": usage.cpu,
            "gpu": gpu,
            "memoryUsed": usage.memory_used,
            "memoryTotal": usage.memory_total,
            "game": session.map(|s| s.name),
        })
    })
    .await?;
    Ok(Json(v))
}

// ---------- voice ----------

#[derive(Deserialize)]
struct VoiceReq {
    text: String,
}

async fn voice(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<VoiceReq>) -> ApiResult<Json<serde_json::Value>> {
    let s = core.settings.get().remote;
    if !s.allow_app_launch {
        return Err(ApiError::forbidden("Voice commands need \"Launch apps\" on the PC (OmniHub → Phone → Permissions)"));
    }
    let text: String = req.text.chars().take(300).collect();
    let c = core.clone();
    let t = text.clone();
    let r = tokio::task::spawn_blocking(move || c.voice_text(&t, s.allow_power)).await?;
    core.audit.record(&actor(&dev), "voice", &text, r.is_ok());
    Ok(Json(json!({ "said": r.map_err(ApiError::bad)? })))
}

// ---------- quiet schedule + sleep timer ----------

fn power_ok(core: &crate::core::AppCore) -> ApiResult<()> {
    if core.settings.get().remote.allow_power {
        Ok(())
    } else {
        Err(ApiError::forbidden("Power actions are off on the PC (OmniHub → Phone → Permissions)"))
    }
}

#[derive(Deserialize)]
struct TimerReq {
    minutes: Option<u32>,
}

async fn quiet_timer(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<TimerReq>) -> ApiResult<Json<crate::quiet::QuietStatus>> {
    power_ok(&core)?;
    core.quiet.set_timer(req.minutes);
    core.audit.record(&actor(&dev), "quiet.timer", &req.minutes.map_or("off".into(), |m| format!("{m} min")), true);
    let s = core.quiet_status();
    core.events.emit("quiet:changed", &s);
    Ok(Json(s))
}

async fn quiet_schedule(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<crate::quiet::QuietSchedule>) -> ApiResult<Json<crate::quiet::QuietStatus>> {
    power_ok(&core)?;
    let detail = format!("{} {}–{}", if req.enabled { "on" } else { "off" }, req.start, req.end);
    core.quiet.set_schedule(req).map_err(ApiError::bad)?;
    core.audit.record(&actor(&dev), "quiet.schedule", &detail, true);
    let s = core.quiet_status();
    core.events.emit("quiet:changed", &s);
    Ok(Json(s))
}

// ---------- photo import ----------

/// Where photos from phones go: Pictures\Phone\<year-month>.
pub fn photos_dir() -> PathBuf {
    let pics = dirs::picture_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Pictures"));
    pics.join("Phone").join(chrono::Local::now().format("%Y-%m").to_string())
}

/// Whether a photo with this name and size is already in Pictures\Phone.
pub fn photo_imported(base: &Path, name: &str, size: u64) -> bool {
    let Ok(months) = std::fs::read_dir(base) else { return false };
    months.flatten().any(|m| std::fs::metadata(m.path().join(name)).is_ok_and(|md| md.len() == size))
}

#[derive(Deserialize)]
struct PhotoFile {
    name: String,
    size: u64,
}

#[derive(Deserialize)]
struct PhotosReq {
    files: Vec<PhotoFile>,
}

async fn photos_check(State(core): State<Ctx>, Json(req): Json<PhotosReq>) -> ApiResult<Json<serde_json::Value>> {
    if !core.settings.get().remote.allow_uploads {
        return Err(ApiError::forbidden("sending files to this PC is turned off"));
    }
    let dest = photos_dir();
    let base = dest.parent().map(Path::to_path_buf).unwrap_or_default();
    let have = tokio::task::spawn_blocking(move || req.files.iter().take(5000).map(|f| photo_imported(&base, &f.name, f.size)).collect::<Vec<bool>>()).await?;
    Ok(Json(json!({ "have": have, "dest": display_path(&dest) })))
}

// ---------- Capri Desktop ----------

async fn capri_get(State(core): State<Ctx>) -> ApiResult<Json<serde_json::Value>> {
    let c = core.clone();
    let (installed, running) = tokio::task::spawn_blocking(move || {
        let st = c.capri_app.status(false);
        (st.installed, !c.procs.pids_named(crate::capri_app::PROCESS).is_empty())
    })
    .await?;
    Ok(Json(json!({ "installed": installed, "running": running })))
}

#[derive(Deserialize)]
struct CapriReq {
    open: bool,
}

async fn capri_set(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<CapriReq>) -> ApiResult<Json<serde_json::Value>> {
    if !core.settings.get().remote.allow_app_launch {
        return Err(ApiError::forbidden("launching apps from the phone is turned off"));
    }
    let r = if req.open { core.capri_app.open() } else { core.procs.end(crate::capri_app::PROCESS).map(|_| ()) };
    core.audit.record(&actor(&dev), if req.open { "capri.open" } else { "capri.close" }, "Capri Desktop", r.is_ok());
    r.map_err(ApiError::bad)?;
    Ok(Json(json!({ "running": req.open })))
}

// ---------- quick sites ----------

async fn sites_list(State(core): State<Ctx>) -> Json<serde_json::Value> {
    Json(json!({ "sites": core.sites.list() }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiteReq {
    id: String,
    #[serde(default = "yes")]
    full_screen: bool,
}

fn yes() -> bool {
    true
}

async fn sites_open(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<SiteReq>) -> ApiResult<StatusCode> {
    if !core.settings.get().remote.allow_app_launch {
        return Err(ApiError::forbidden("opening things on the PC from the phone is turned off"));
    }
    // Only sites on the list (not any address the phone sends).
    let site = core.sites.list().into_iter().find(|s| s.id == req.id).ok_or_else(|| ApiError::not_found("that site isn't on the list any more"))?;
    let (url, full) = (site.url.clone(), req.full_screen);
    crate::system::awake::wake_screen();
    let r = tokio::task::spawn_blocking(move || crate::sites::open(&url, full)).await?;
    core.audit.record(&actor(&dev), "site.open", &site.name, r.is_ok());
    r.map_err(ApiError::bad)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------- where the sound comes out ----------

fn sound_ok(core: &crate::core::AppCore) -> ApiResult<()> {
    if core.settings.get().media.allow_phone {
        Ok(())
    } else {
        Err(ApiError::forbidden("music control from the phone is turned off on the PC"))
    }
}

async fn outputs_list(State(core): State<Ctx>) -> ApiResult<Json<serde_json::Value>> {
    sound_ok(&core)?;
    let list = tokio::task::spawn_blocking(crate::media::outputs::list).await?.map_err(ApiError::bad)?;
    Ok(Json(json!({ "outputs": list })))
}

#[derive(Deserialize)]
struct OutputReq {
    id: String,
}

async fn outputs_set(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<OutputReq>) -> ApiResult<Json<serde_json::Value>> {
    sound_ok(&core)?;
    let id = req.id.clone();
    let list = tokio::task::spawn_blocking(move || {
        crate::media::outputs::set_default(&id)?;
        crate::media::outputs::list()
    })
    .await?;
    core.audit.record(&actor(&dev), "sound.output", &req.id.chars().take(60).collect::<String>(), list.is_ok());
    Ok(Json(json!({ "outputs": list.map_err(ApiError::bad)? })))
}

// ---------- Bluetooth ----------

async fn bt_list(State(core): State<Ctx>) -> ApiResult<Json<serde_json::Value>> {
    sound_ok(&core)?;
    let list = tokio::task::spawn_blocking(crate::system::bt::list).await?.map_err(ApiError::bad)?;
    Ok(Json(json!({ "devices": list })))
}

#[derive(Deserialize)]
struct BtReq {
    address: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    on: bool,
}

async fn bt_connect(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<BtReq>) -> ApiResult<Json<serde_json::Value>> {
    sound_ok(&core)?;
    let (a, n, on) = (req.address.clone(), req.name.clone(), req.on);
    let r = tokio::task::spawn_blocking(move || {
        crate::system::bt::connect(&a, &n, on)?;
        std::thread::sleep(Duration::from_millis(1200));
        crate::system::bt::list()
    })
    .await?;
    core.audit.record(&actor(&dev), if req.on { "bluetooth.connect" } else { "bluetooth.disconnect" }, &req.name, r.is_ok());
    Ok(Json(json!({ "devices": r.map_err(ApiError::bad)? })))
}

async fn bt_forget(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<BtReq>) -> ApiResult<Json<serde_json::Value>> {
    sound_ok(&core)?;
    let a = req.address.clone();
    let r = tokio::task::spawn_blocking(move || {
        crate::system::bt::forget(&a)?;
        crate::system::bt::list()
    })
    .await?;
    core.audit.record(&actor(&dev), "bluetooth.forget", &req.name, r.is_ok());
    Ok(Json(json!({ "devices": r.map_err(ApiError::bad)? })))
}

// ---------- party queue ----------

async fn party_get(State(core): State<Ctx>) -> Json<serde_json::Value> {
    let st = core.party.state();
    Json(json!({ "code": st.code, "queue": st.queue.len(), "playing": st.playing }))
}

/// A wrong code counts against the address (guessing is slow).
fn party_ok(core: &crate::core::AppCore, code: &str, peer: &SocketAddr) -> ApiResult<()> {
    if core.party.code_ok(code) {
        return Ok(());
    }
    if !core.remote.guest_limiter.allow(peer.ip()) {
        return Err(ApiError::new(StatusCode::TOO_MANY_REQUESTS, "too many tries; wait a minute"));
    }
    Err(ApiError::new(StatusCode::GONE, "This party has ended."))
}

async fn party_js() -> Response {
    ([(header::CONTENT_TYPE, "text/javascript; charset=utf-8"), (header::CACHE_CONTROL, "no-store")], crate::party::PAGE_JS).into_response()
}

async fn party_page() -> Response {
    ([(header::CONTENT_TYPE, "text/html; charset=utf-8"), (header::CACHE_CONTROL, "no-store")], crate::party::PAGE).into_response()
}

async fn party_queue(State(core): State<Ctx>, ConnectInfo(peer): ConnectInfo<SocketAddr>, UrlPath(code): UrlPath<String>) -> ApiResult<Json<serde_json::Value>> {
    party_ok(&core, &code, &peer)?;
    let st = core.party.state();
    Ok(Json(json!({ "queue": st.queue, "playing": st.playing })))
}

async fn party_search(State(core): State<Ctx>, ConnectInfo(peer): ConnectInfo<SocketAddr>, UrlPath(code): UrlPath<String>, Query(q): Query<SearchQuery>) -> ApiResult<Json<serde_json::Value>> {
    party_ok(&core, &code, &peer)?;
    let query: String = q.q.trim().chars().take(100).collect();
    if query.chars().count() < 2 {
        return Ok(Json(json!({ "tracks": [] })));
    }
    let r = tokio::task::spawn_blocking(move || crate::media::catalog::search(&query, &crate::media::catalog::http_get)).await?.map_err(ApiError::bad)?;
    let tracks: Vec<_> = r.tracks.into_iter().take(10).map(|t| json!({ "title": t.title, "artist": t.artist, "album": t.album, "cover": t.cover })).collect();
    Ok(Json(json!({ "tracks": tracks })))
}

async fn party_add(State(core): State<Ctx>, ConnectInfo(peer): ConnectInfo<SocketAddr>, UrlPath(code): UrlPath<String>, Json(song): Json<crate::party::PartySong>) -> ApiResult<Json<serde_json::Value>> {
    party_ok(&core, &code, &peer)?;
    let mut song = song;
    // Only cover art from the catalogue's own image hosts.
    if !song.cover.starts_with("https://cdn-images.dzcdn.net/") && !song.cover.starts_with("https://e-cdns-images.dzcdn.net/") {
        song.cover.clear();
    }
    let n = core.party.add(&code, song, crate::db::now()).map_err(ApiError::bad)?;
    core.events.emit("party:changed", core.party.state());
    Ok(Json(json!({ "position": n })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_files_by_every_word_and_skips_system_folders() {
        let t = tempfile::tempdir().unwrap();
        let r = t.path();
        std::fs::create_dir_all(r.join("Docs/Trip")).unwrap();
        std::fs::create_dir_all(r.join("node_modules/x")).unwrap();
        std::fs::write(r.join("Docs/Trip/Beach Photos 2026.zip"), b"12345").unwrap();
        std::fs::write(r.join("Docs/beach-notes.txt"), b"1").unwrap();
        std::fs::write(r.join("node_modules/x/beach photos.js"), b"1").unwrap();
        let (found, cut) = search_names(&[r.to_path_buf()], "beach PHOTOS", Duration::from_secs(5));
        assert!(!cut);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].2, 5);
        assert!(search_names(&[r.to_path_buf()], "  ", Duration::from_secs(5)).0.is_empty());
    }

    #[test]
    fn knows_photos_already_imported() {
        let t = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(t.path().join("2026-10")).unwrap();
        std::fs::write(t.path().join("2026-10/IMG_1.jpg"), b"abc").unwrap();
        assert!(photo_imported(t.path(), "IMG_1.jpg", 3));
        assert!(!photo_imported(t.path(), "IMG_1.jpg", 4), "a different photo with the same name");
        assert!(!photo_imported(t.path(), "IMG_2.jpg", 3));
        assert!(photos_dir().ends_with(chrono::Local::now().format("%Y-%m").to_string()));
    }
}
