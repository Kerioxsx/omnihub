//! HTTP and WebSocket API of the companion server.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::{Body, Bytes};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, DefaultBodyLimit, Path as UrlPath, Query, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Extension, Json, Router};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::auth::{Device, PairCheck, TicketPurpose};
use super::{net, transfer, VaultSession, Viewer, ViewerInfo};
use crate::capture::stream::{self, StreamControl, ViewerMessage};
use crate::core::AppCore;
use crate::settings::BrowseScope;
use crate::system::power::PowerAction;

type Ctx = Arc<AppCore>;

#[derive(Debug)]
pub struct ApiError(StatusCode, String);

impl ApiError {
    fn new(code: StatusCode, msg: impl Into<String>) -> Self {
        ApiError(code, msg.into())
    }
    fn bad(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, msg)
    }
    fn forbidden(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, msg)
    }
    fn not_found(msg: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, msg)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl<E: std::fmt::Display> From<E> for ApiError {
    fn from(e: E) -> Self {
        ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    }
}

type ApiResult<T> = Result<T, ApiError>;

pub fn router(core: Ctx) -> Router {
    let authed = Router::new()
        .route("/api/me", get(me))
        .route("/api/unpair", post(unpair))
        .route("/api/ticket", post(ticket))
        .route("/api/status", get(status))
        .route("/api/fs/roots", get(fs_roots))
        .route("/api/fs/list", get(fs_list))
        .route("/api/fs/thumb", get(fs_thumb))
        .route("/api/files/ticket", post(file_ticket))
        .route("/api/upload", post(upload_start))
        .route("/api/upload/{id}", get(upload_status).delete(upload_cancel))
        .route("/api/upload/{id}/chunk", put(upload_chunk).layer(DefaultBodyLimit::max(transfer::MAX_CHUNK + 1024)))
        .route("/api/upload/{id}/finish", post(upload_finish))
        .route("/api/inbox", get(inbox_list))
        .route("/api/inbox/{id}/ticket", post(inbox_ticket))
        .route("/api/inbox/{id}", axum::routing::delete(inbox_dismiss))
        .route("/api/clipboard", post(clipboard_set))
        .route("/api/media", get(media_get))
        .route("/api/media/art", get(media_art))
        .route("/api/media/control", post(media_control))
        .route("/api/media/lyrics", get(media_lyrics))
        .route("/api/media/audio", post(media_audio))
        .route("/api/sound", get(sound_get))
        .route("/api/sound/master", post(sound_master))
        .route("/api/sound/app", post(sound_app))
        .route("/api/sound/mic", post(sound_mic))
        .route("/api/games", get(games_list))
        .route("/api/games/stop", post(games_stop))
        .route("/api/games/ping", post(games_ping))
        .route("/api/games/{id}/play", post(games_play))
        .route("/api/tasks", get(tasks_list))
        .route("/api/tasks/end", post(tasks_end))
        .route("/api/tasks/priority", post(tasks_priority))
        .route("/api/open-apps", get(open_apps_list))
        .route("/api/open-apps/close", post(open_apps_close))
        .route("/api/open-apps/icon", get(open_apps_icon))
        .route("/api/power", get(power_info).post(power_request))
        .route("/api/power/cancel", post(power_cancel))
        .route("/api/apps", get(apps_list))
        .route("/api/apps/{id}/icon", get(app_icon))
        .route("/api/apps/{id}/launch", post(app_launch))
        .route("/api/notes", get(notes_list).post(notes_create))
        .route("/api/notes/{id}", get(notes_get))
        .route("/api/chats", get(chats_list))
        .route("/api/chats/new", post(chats_new))
        .route("/api/chats/folders", get(chats_folders))
        .route("/api/chats/usage", get(chats_usage))
        .route("/api/chats/{source}/{id}", get(chats_view))
        .route("/api/chats/{source}/{id}/send", post(chats_send))
        .route("/api/chats/{source}/{id}/stop", post(chats_stop))
        .route("/api/ideas", get(ideas_list).post(ideas_save))
        .route("/api/servers", get(servers_list))
        .route("/api/servers/{id}/start", post(servers_start))
        .route("/api/servers/{id}/stop", post(servers_stop))
        .route("/api/servers/{id}/restart", post(servers_restart))
        .route("/api/servers/{id}/players", get(servers_players))
        .route("/api/servers/{id}/player/{name}", get(servers_player))
        .route("/api/servers/{id}/act", post(servers_act))
        .route("/api/servers/{id}/whitelist", post(servers_whitelist))
        .route("/api/servers/{id}/max-players", post(servers_max))
        .route("/api/servers/{id}/join", post(servers_join))
        .route("/api/servers/{id}/console", get(servers_console))
        .route("/api/servers/{id}/command", post(servers_command))
        .route("/api/servers/{id}/map", get(servers_map))
        .route("/api/servers/{id}/map/tile", get(servers_tile))
        .route("/api/screen/info", get(screen_info))
        .route("/api/vault/session", post(vault_session).delete(vault_end))
        .route("/api/vault/entries", get(vault_entries))
        .route("/api/vault/reveal", post(vault_reveal))
        .route_layer(middleware::from_fn_with_state(core.clone(), require_device));

    Router::new()
        .route("/api/info", get(info))
        .route("/api/pair", post(pair))
        .route("/api/ws", get(events_socket))
        .route("/api/screen/ws", get(screen_socket))
        .route("/dl/{ticket}", get(download))
        .merge(authed)
        .fallback(super::assets::serve)
        .layer(DefaultBodyLimit::max(1 << 20))
        .layer(middleware::from_fn_with_state(core.clone(), guard))
        .with_state(core)
}

/// Every request: private-network peers only, Host header sanity, and
/// security headers on the way out.
async fn guard(State(core): State<Ctx>, ConnectInfo(peer): ConnectInfo<SocketAddr>, req: Request, next: Next) -> Response {
    let s = core.settings.get().remote;
    let allowed = net::is_allowed_peer(peer.ip(), s.allow_tailscale);
    if !peer.ip().is_loopback() {
        let ua = req.headers().get(header::USER_AGENT).and_then(|h| h.to_str().ok()).unwrap_or("");
        core.remote.record_visit(&peer.ip().to_canonical().to_string(), ua, allowed);
    }
    if !allowed {
        return ApiError::forbidden("only devices on your local network can connect").into_response();
    }
    let host = req.headers().get(header::HOST).and_then(|h| h.to_str().ok()).unwrap_or("");
    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    if !host.is_empty() && !net::host_allowed(host, &hostname, s.allow_tailscale) {
        return ApiError::new(StatusCode::MISDIRECTED_REQUEST, "unexpected host name").into_response();
    }
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    h.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    h.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    h.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'; img-src 'self' data: blob:; media-src 'self' blob:; connect-src 'self' ws: wss:; style-src 'self' 'unsafe-inline'; script-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"),
    );
    res
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers.get(header::AUTHORIZATION)?.to_str().ok()?.strip_prefix("Bearer ")
}

async fn require_device(State(core): State<Ctx>, ConnectInfo(peer): ConnectInfo<SocketAddr>, mut req: Request, next: Next) -> Response {
    let Some(token) = bearer(req.headers()) else {
        return ApiError::new(StatusCode::UNAUTHORIZED, "pair this phone first").into_response();
    };
    match core.remote.devices.authenticate(token, &peer.ip().to_string()) {
        Some(dev) => {
            req.extensions_mut().insert(dev);
            next.run(req).await
        }
        None => ApiError::new(StatusCode::UNAUTHORIZED, "this phone is no longer paired").into_response(),
    }
}

fn actor(d: &Device) -> String {
    format!("phone:{}", d.name)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Features {
    uploads: bool,
    power: bool,
    screen: bool,
    control: bool,
    apps: bool,
    notes: bool,
    vault: bool,
    clipboard: bool,
    media: bool,
    tasks: bool,
    games: bool,
    chats: bool,
    chat_agents: bool,
    servers: bool,
}

fn features(core: &AppCore, tls: bool) -> Features {
    let s = core.settings.get();
    Features {
        uploads: s.remote.allow_uploads,
        power: s.remote.allow_power,
        screen: s.remote.allow_screen,
        control: s.remote.allow_control,
        apps: s.remote.allow_app_launch,
        notes: s.remote.allow_notes,
        vault: s.vault.allow_phone && tls && core.vault.exists(),
        clipboard: s.remote.allow_clipboard,
        tasks: s.remote.allow_tasks,
        games: s.remote.allow_app_launch,
        media: s.media.allow_phone,
        chats: s.chats.allow_phone,
        chat_agents: s.chats.allow_phone && s.chats.phone_agents,
        servers: s.remote.allow_servers,
    }
}

async fn info(State(core): State<Ctx>, ConnectInfo(peer): ConnectInfo<SocketAddr>, headers: HeaderMap) -> Json<serde_json::Value> {
    let paired = bearer(&headers).and_then(|t| core.remote.devices.authenticate(t, &peer.ip().to_string())).is_some();
    let tls = core.remote.status().tls;
    let s = core.settings.get();
    Json(json!({
        "name": s.remote.device_name,
        "app": "OmniHub",
        "version": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS,
        "tls": tls,
        "paired": paired,
        "pairingOpen": core.remote.pairing.is_open(),
        "features": features(&core, tls),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairReq {
    secret: Option<String>,
    pin: Option<String>,
    device_name: String,
}

async fn pair(State(core): State<Ctx>, ConnectInfo(peer): ConnectInfo<SocketAddr>, headers: HeaderMap, Json(req): Json<PairReq>) -> ApiResult<Json<serde_json::Value>> {
    if !core.remote.pair_limiter.allow(peer.ip()) {
        return Err(ApiError::new(StatusCode::TOO_MANY_REQUESTS, "too many attempts; wait a minute"));
    }
    match core.remote.pairing.check(req.secret.as_deref(), req.pin.as_deref()) {
        PairCheck::Ok => {}
        PairCheck::Closed => return Err(ApiError::forbidden("pairing is not open; click \"Pair a phone\" in OmniHub on the PC")),
        PairCheck::Wrong => {
            core.audit.record(&format!("ip:{}", peer.ip()), "remote.pair", "wrong code", false);
            return Err(ApiError::forbidden("wrong code"));
        }
    }
    let ua = headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).unwrap_or("");
    let (dev, token) = core.remote.devices.create(&req.device_name, ua, &peer.ip().to_string())?;
    core.audit.record(&actor(&dev), "remote.pair", &format!("from {}", peer.ip()), true);
    core.events.emit("remote:paired", &dev);
    core.events.emit("remote:devices", core.remote.devices.list());
    Ok(Json(json!({ "token": token, "deviceId": dev.id, "serverName": core.settings.get().remote.device_name })))
}

async fn me(Extension(dev): Extension<Device>) -> Json<Device> {
    Json(dev)
}

async fn unpair(State(core): State<Ctx>, Extension(dev): Extension<Device>) -> ApiResult<StatusCode> {
    core.remote.revoke_device(&dev.id);
    core.audit.record(&actor(&dev), "remote.unpair", "", true);
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct TicketReq {
    purpose: String,
}

async fn ticket(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<TicketReq>) -> ApiResult<Json<serde_json::Value>> {
    let purpose = match req.purpose.as_str() {
        "socket" => TicketPurpose::Socket,
        "screen" => {
            if !core.settings.get().remote.allow_screen {
                return Err(ApiError::forbidden("screen sharing is turned off on the PC"));
            }
            TicketPurpose::Screen
        }
        _ => return Err(ApiError::bad("unknown ticket purpose")),
    };
    Ok(Json(json!({ "ticket": core.remote.tickets.issue(&dev.id, purpose, Duration::from_secs(30)) })))
}

static SYS: std::sync::LazyLock<parking_lot::Mutex<sysinfo::System>> = std::sync::LazyLock::new(|| parking_lot::Mutex::new(sysinfo::System::new()));

async fn status(State(core): State<Ctx>) -> Json<serde_json::Value> {
    let (cpu, used, total) = {
        let mut sys = SYS.lock();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        (sys.global_cpu_usage(), sys.used_memory(), sys.total_memory())
    };
    let disks: Vec<_> = crate::storage::volumes::list().into_iter().filter(|v| v.total > 0).map(|v| json!({ "root": v.root, "label": v.label, "total": v.total, "free": v.free })).collect();
    Json(json!({
        "hostname": gethostname::gethostname().to_string_lossy(),
        "os": sysinfo::System::long_os_version().unwrap_or_default(),
        "uptime": sysinfo::System::uptime(),
        "cpu": cpu,
        "memory": { "used": used, "total": total },
        "disks": disks,
        "pendingPower": core.power.pending(),
        "vaultUnlocked": core.vault.is_unlocked(),
    }))
}

// ---------- files ----------

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Root {
    name: String,
    path: String,
}

fn roots(core: &AppCore) -> Vec<Root> {
    let s = core.settings.get().remote;
    let mut v: Vec<Root> = match s.browse_scope {
        BrowseScope::UserFolders => crate::system::shell::user_folders().into_iter().map(|(n, p)| Root { name: n, path: p.to_string_lossy().to_string() }).collect(),
        BrowseScope::AllDrives => crate::storage::volumes::list()
            .into_iter()
            .map(|v| Root { name: if v.label.is_empty() { v.root.clone() } else { format!("{} ({})", v.label, v.root.trim_end_matches('\\')) }, path: v.root })
            .collect(),
        BrowseScope::Custom => s.custom_roots.iter().map(|p| Root { name: Path::new(p).file_name().map_or(p.clone(), |n| n.to_string_lossy().to_string()), path: p.clone() }).collect(),
    };
    let incoming = core.incoming_dir();
    if s.browse_scope != BrowseScope::AllDrives && !v.iter().any(|r| Path::new(&r.path) == incoming) {
        let _ = std::fs::create_dir_all(&incoming);
        v.push(Root { name: "From phone".into(), path: incoming.to_string_lossy().to_string() });
    }
    v
}

/// Resolve `path` and make sure it is inside one of the allowed roots.
fn allowed_path(core: &AppCore, path: &str) -> ApiResult<PathBuf> {
    let p = std::fs::canonicalize(path).map_err(|_| ApiError::not_found("no such file or folder"))?;
    for r in roots(core) {
        if let Ok(root) = std::fs::canonicalize(&r.path) {
            if p.starts_with(&root) {
                return Ok(p);
            }
        }
    }
    Err(ApiError::forbidden("that location is not shared with the phone (Settings → Phone → Browsing)"))
}

fn display_path(p: &Path) -> String {
    let s = p.to_string_lossy();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

fn kind_of(name: &str) -> &'static str {
    match crate::storage::tree::extension_of(name).as_deref() {
        Some("jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp" | "heic" | "svg") => "image",
        Some("mp4" | "mkv" | "mov" | "avi" | "webm" | "wmv") => "video",
        Some("mp3" | "flac" | "wav" | "ogg" | "m4a" | "aac") => "audio",
        Some("pdf" | "doc" | "docx" | "txt" | "md" | "xls" | "xlsx" | "ppt" | "pptx" | "odt") => "document",
        Some("zip" | "7z" | "rar" | "tar" | "gz" | "iso") => "archive",
        Some("exe" | "msi" | "apk") => "app",
        _ => "file",
    }
}

async fn fs_roots(State(core): State<Ctx>) -> Json<serde_json::Value> {
    Json(json!({ "roots": roots(&core) }))
}

#[derive(Deserialize)]
struct PathQuery {
    path: String,
    size: Option<u32>,
}

async fn fs_list(State(core): State<Ctx>, Query(q): Query<PathQuery>) -> ApiResult<Json<serde_json::Value>> {
    let dir = allowed_path(&core, &q.path)?;
    let core2 = core.clone();
    let (entries, parent) = tokio::task::spawn_blocking(move || -> ApiResult<_> {
        let rd = std::fs::read_dir(&dir).map_err(|e| ApiError::forbidden(format!("cannot open folder: {e}")))?;
        let mut entries = Vec::new();
        for e in rd.flatten() {
            let Ok(meta) = e.metadata() else { continue };
            let name = e.file_name().to_string_lossy().to_string();
            let path = display_path(&e.path());
            let is_dir = meta.is_dir();
            let modified = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
            #[cfg(windows)]
            let hidden = {
                use std::os::windows::fs::MetadataExt;
                meta.file_attributes() & 0x6 != 0
            };
            #[cfg(not(windows))]
            let hidden = name.starts_with('.');
            entries.push(json!({ "name": name, "path": path, "isDir": is_dir, "size": if is_dir { None } else { Some(meta.len()) }, "modified": modified, "kind": if is_dir { "folder" } else { kind_of(&name) }, "hidden": hidden }));
        }
        if entries.len() <= 400 {
            for e in entries.iter_mut().filter(|e| e["isDir"] == true) {
                if let Some((size, _)) = core2.storage.size_of_path(e["path"].as_str().unwrap_or("")) {
                    e["size"] = json!(size);
                }
            }
        }
        entries.sort_by(|a, b| {
            b["isDir"].as_bool().cmp(&a["isDir"].as_bool()).then_with(|| crate::storage::tree::natural_cmp(a["name"].as_str().unwrap_or(""), b["name"].as_str().unwrap_or("")))
        });
        // Only offer "up" while still inside a shared root.
        let parent = dir.parent().filter(|p| roots(&core2).iter().any(|r| std::fs::canonicalize(&r.path).is_ok_and(|root| p.starts_with(&root)))).map(display_path);
        Ok((entries, parent))
    })
    .await??;
    Ok(Json(json!({ "path": display_path(&std::fs::canonicalize(&q.path)?), "parent": parent, "entries": entries })))
}

async fn fs_thumb(State(core): State<Ctx>, Query(q): Query<PathQuery>) -> ApiResult<Response> {
    let p = allowed_path(&core, &q.path)?;
    if !crate::thumbs::is_previewable(&p) {
        return Err(ApiError::bad("not a previewable image"));
    }
    let size = q.size.unwrap_or(256).clamp(64, 1024);
    let c = core.clone();
    let bytes = tokio::task::spawn_blocking(move || c.thumbs.jpeg(&p, size)).await?.map_err(|e| ApiError::bad(e.to_string()))?;
    Ok(([(header::CONTENT_TYPE, "image/jpeg"), (header::CACHE_CONTROL, "private, max-age=600")], (*bytes).clone()).into_response())
}

#[derive(Deserialize)]
struct FileTicketReq {
    path: String,
}

async fn file_ticket(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<FileTicketReq>) -> ApiResult<Json<serde_json::Value>> {
    let p = allowed_path(&core, &req.path)?;
    let meta = std::fs::metadata(&p)?;
    if !meta.is_file() {
        return Err(ApiError::bad("only files can be downloaded"));
    }
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let t = core.remote.tickets.issue(&dev.id, TicketPurpose::Download(p.clone()), Duration::from_secs(15 * 60));
    core.audit.record(&actor(&dev), "files.download", &display_path(&p), true);
    Ok(Json(json!({ "url": format!("/dl/{t}"), "name": name, "size": meta.len() })))
}

fn content_disposition(name: &str, inline: bool) -> String {
    let ascii: String = name.chars().map(|c| if c.is_ascii_graphic() && c != '"' && c != '\\' || c == ' ' { c } else { '_' }).collect();
    let encoded = percent_encoding::utf8_percent_encode(name, percent_encoding::NON_ALPHANUMERIC).to_string();
    format!("{}; filename=\"{ascii}\"; filename*=UTF-8''{encoded}", if inline { "inline" } else { "attachment" })
}

#[derive(Deserialize)]
struct DlQuery {
    inline: Option<u8>,
}

async fn download(State(core): State<Ctx>, UrlPath(ticket): UrlPath<String>, Query(q): Query<DlQuery>, req: Request) -> ApiResult<Response> {
    let t = core.remote.tickets.redeem(&ticket, false).ok_or_else(|| ApiError::not_found("this download link has expired"))?;
    let TicketPurpose::Download(path) = t.purpose else { return Err(ApiError::not_found("not a download")) };
    // Ranges, conditional requests and content length are handled here.
    let mut res = tower::ServiceExt::oneshot(tower_http::services::ServeFile::new(&path), req).await.map_err(|e| ApiError::from(e.to_string()))?.map(Body::new);
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if let Ok(v) = HeaderValue::from_str(&content_disposition(&name, q.inline == Some(1))) {
        res.headers_mut().insert(header::CONTENT_DISPOSITION, v);
    }
    res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    Ok(res)
}

// ---------- uploads ----------

#[derive(Deserialize)]
struct UploadReq {
    name: String,
    size: u64,
    dir: Option<String>,
}

async fn upload_start(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<UploadReq>) -> ApiResult<Json<transfer::UploadStatus>> {
    if !core.settings.get().remote.allow_uploads {
        return Err(ApiError::forbidden("sending files to this PC is turned off"));
    }
    let dest = match req.dir.as_deref().filter(|d| !d.is_empty()) {
        Some(d) => allowed_path(&core, d)?,
        None => core.incoming_dir(),
    };
    if let Some((_, free)) = crate::storage::volumes::space_of(&dest.to_string_lossy()) {
        if free > 0 && req.size > free.saturating_sub(256 << 20) {
            return Err(ApiError::new(StatusCode::INSUFFICIENT_STORAGE, "not enough free space on the PC"));
        }
    }
    let st = core.remote.uploads(&core).start(&req.name, req.size, &dest, &dev.id, &dev.name).map_err(|e| ApiError::bad(e.to_string()))?;
    Ok(Json(st))
}

fn transfer_err(e: transfer::TransferError) -> ApiError {
    use transfer::TransferError::*;
    match e {
        Unknown => ApiError::not_found(e.to_string()),
        Forbidden => ApiError::forbidden(e.to_string()),
        WrongOffset(_) | Incomplete(..) => ApiError::new(StatusCode::CONFLICT, e.to_string()),
        TooLarge => ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, e.to_string()),
        Checksum | BadName => ApiError::bad(e.to_string()),
        Io(e) => ApiError::from(e),
    }
}

async fn upload_status(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>) -> ApiResult<Json<transfer::UploadStatus>> {
    core.remote.uploads(&core).status(&id, &dev.id).map(Json).map_err(transfer_err)
}

#[derive(Deserialize)]
struct ChunkQuery {
    offset: u64,
}

async fn upload_chunk(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>, Query(q): Query<ChunkQuery>, headers: HeaderMap, body: Bytes) -> ApiResult<Json<serde_json::Value>> {
    let crc = headers.get("x-chunk-crc32").and_then(|v| v.to_str().ok()).and_then(|v| u32::from_str_radix(v.trim_start_matches("0x"), 16).ok());
    let uploads = core.remote.uploads(&core);
    let dev_id = dev.id.clone();
    let offset = tokio::task::spawn_blocking(move || uploads.write_chunk(&id, &dev_id, q.offset, &body, crc)).await?.map_err(transfer_err)?;
    Ok(Json(json!({ "offset": offset })))
}

async fn upload_finish(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>) -> ApiResult<Json<transfer::FinishedUpload>> {
    let uploads = core.remote.uploads(&core);
    let dev_id = dev.id.clone();
    let done = tokio::task::spawn_blocking(move || uploads.finish(&id, &dev_id)).await?.map_err(transfer_err)?;
    core.audit.record(&actor(&dev), "files.upload", &format!("{} ({} bytes)", done.path, done.size), true);
    Ok(Json(done))
}

async fn upload_cancel(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>) -> ApiResult<StatusCode> {
    core.remote.uploads(&core).cancel(&id, &dev.id).map_err(transfer_err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn inbox_list(State(core): State<Ctx>, Extension(dev): Extension<Device>) -> Json<serde_json::Value> {
    Json(json!({ "items": core.remote.inbox.for_device(&dev.id) }))
}

async fn inbox_ticket(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>) -> ApiResult<Json<serde_json::Value>> {
    let item = core.remote.inbox.get(&id, &dev.id).ok_or_else(|| ApiError::not_found("no longer offered"))?;
    if item.kind == transfer::InboxKind::Text {
        return Err(ApiError::bad("text items have nothing to download"));
    }
    if !item.path.is_file() {
        return Err(ApiError::not_found("the file was moved or deleted on the PC"));
    }
    let t = core.remote.tickets.issue(&dev.id, TicketPurpose::Download(item.path.clone()), Duration::from_secs(15 * 60));
    core.audit.record(&actor(&dev), "files.receive", &display_path(&item.path), true);
    Ok(Json(json!({ "url": format!("/dl/{t}"), "name": item.name, "size": item.size })))
}

#[derive(Deserialize)]
struct ClipboardText {
    text: String,
}

// ---------- music: what's playing, lyrics, volume, bass ----------

fn media_allowed(core: &AppCore) -> ApiResult<()> {
    if core.settings.get().media.allow_phone {
        Ok(())
    } else {
        Err(ApiError::forbidden("music control from the phone is turned off on the PC"))
    }
}

/// Volume and equaliser state (blocking: Core Audio and the registry).
fn audio_json(core: &AppCore) -> serde_json::Value {
    let m = core.settings.get().media;
    json!({
        "volume": crate::media::audio::get(),
        "eq": { "status": crate::media::eq::status(), "enabled": m.eq_enabled, "bass": m.bass_db, "treble": m.treble_db, "maxDb": crate::media::eq::MAX_DB },
    })
}

async fn media_get(State(core): State<Ctx>) -> ApiResult<Json<serde_json::Value>> {
    media_allowed(&core)?;
    let c = core.clone();
    let audio = tokio::task::spawn_blocking(move || audio_json(&c)).await?;
    Ok(Json(json!({ "state": core.media.state(), "nowMs": crate::media::now_ms(), "audio": audio })))
}

#[derive(Deserialize)]
struct ArtQuery {
    id: String,
}

async fn media_art(State(core): State<Ctx>, Query(q): Query<ArtQuery>) -> ApiResult<Response> {
    media_allowed(&core)?;
    let (bytes, mime) = core.media.art(&q.id).ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "no artwork"))?;
    Ok(([(header::CONTENT_TYPE, mime), (header::CACHE_CONTROL, "private, max-age=3600".to_string())], (*bytes).clone()).into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaControl {
    action: crate::media::Action,
    #[serde(default)]
    position_ms: u64,
}

async fn media_control(State(core): State<Ctx>, Json(c): Json<MediaControl>) -> ApiResult<Json<serde_json::Value>> {
    media_allowed(&core)?;
    let c2 = core.clone();
    tokio::task::spawn_blocking(move || c2.media.control(c.action, c.position_ms)).await?.map_err(ApiError::bad)?;
    Ok(Json(json!({ "ok": true })))
}

async fn media_lyrics(State(core): State<Ctx>) -> ApiResult<Json<serde_json::Value>> {
    media_allowed(&core)?;
    Ok(Json(json!({ "key": core.media.state().map(|s| s.key), "lyrics": core.media.lyrics() })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaAudio {
    level: Option<f32>,
    muted: Option<bool>,
    bass: Option<f32>,
    treble: Option<f32>,
    eq_enabled: Option<bool>,
}

async fn media_audio(State(core): State<Ctx>, Json(a): Json<MediaAudio>) -> ApiResult<Json<serde_json::Value>> {
    media_allowed(&core)?;
    let c = core.clone();
    let out = tokio::task::spawn_blocking(move || -> Result<serde_json::Value, String> {
        if a.level.is_some() || a.muted.is_some() {
            crate::media::audio::set(a.level, a.muted)?;
        }
        let mut patch = serde_json::Map::new();
        let clamp = |v: f32| v.clamp(-crate::media::eq::MAX_DB, crate::media::eq::MAX_DB);
        if let Some(b) = a.bass.filter(|v| v.is_finite()) {
            patch.insert("bassDb".into(), json!(clamp(b)));
        }
        if let Some(t) = a.treble.filter(|v| v.is_finite()) {
            patch.insert("trebleDb".into(), json!(clamp(t)));
        }
        if let Some(e) = a.eq_enabled {
            patch.insert("eqEnabled".into(), json!(e));
        }
        if !patch.is_empty() {
            c.update_settings(&json!({ "media": patch })).map_err(|e| e.to_string())?;
        }
        Ok(audio_json(&c))
    })
    .await?
    .map_err(ApiError::bad)?;
    Ok(Json(out))
}

// ---------- sound: volume mixer, microphone and calls ----------

async fn sound_get(State(core): State<Ctx>) -> ApiResult<Json<crate::media::mixer::Mixer>> {
    media_allowed(&core)?;
    let fake = core.media.is_fake();
    Ok(Json(tokio::task::spawn_blocking(move || crate::media::mixer::get(fake)).await?))
}

#[derive(Deserialize)]
struct SoundReq {
    #[serde(default)]
    key: String,
    level: Option<f32>,
    muted: Option<bool>,
}

async fn sound_master(State(core): State<Ctx>, Json(req): Json<SoundReq>) -> ApiResult<Json<crate::media::mixer::Mixer>> {
    media_allowed(&core)?;
    let fake = core.media.is_fake();
    Ok(Json(tokio::task::spawn_blocking(move || crate::media::mixer::set_master(fake, req.level, req.muted)).await?.map_err(ApiError::bad)?))
}

async fn sound_app(State(core): State<Ctx>, Json(req): Json<SoundReq>) -> ApiResult<Json<crate::media::mixer::Mixer>> {
    media_allowed(&core)?;
    let fake = core.media.is_fake();
    Ok(Json(tokio::task::spawn_blocking(move || crate::media::mixer::set_app(fake, &req.key, req.level, req.muted)).await?.map_err(ApiError::bad)?))
}

async fn sound_mic(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<SoundReq>) -> ApiResult<Json<crate::media::mixer::Mixer>> {
    media_allowed(&core)?;
    let muted = req.muted.ok_or_else(|| ApiError::bad("say whether to mute"))?;
    let fake = core.media.is_fake();
    let r = tokio::task::spawn_blocking(move || crate::media::mixer::set_mic(fake, muted)).await?;
    core.audit.record(&actor(&dev), if muted { "microphone.mute" } else { "microphone.unmute" }, "all recording devices", r.is_ok());
    Ok(Json(r.map_err(ApiError::bad)?))
}

/// Put text from the phone on this PC's clipboard.
async fn clipboard_set(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(c): Json<ClipboardText>) -> ApiResult<StatusCode> {
    if !core.settings.get().remote.allow_clipboard {
        return Err(ApiError::forbidden("the PC does not accept clipboard text from phones"));
    }
    let text = c.text;
    if text.is_empty() || text.len() > transfer::MAX_TEXT {
        return Err(ApiError::bad("send between 1 character and 100 KB of text"));
    }
    let chars = text.chars().count();
    tokio::task::spawn_blocking(move || crate::system::clipboard::copy_text(&text)).await??;
    core.audit.record(&actor(&dev), "clipboard.from-phone", &format!("{chars} characters"), true);
    core.events.emit("clipboard:from-phone", json!({ "device": dev.name, "chars": chars }));
    Ok(StatusCode::NO_CONTENT)
}

async fn inbox_dismiss(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>) -> ApiResult<StatusCode> {
    if core.remote.inbox.get(&id, &dev.id).is_some() {
        core.remote.inbox.remove(&id);
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------- power ----------

async fn power_info(State(core): State<Ctx>) -> Json<serde_json::Value> {
    let s = core.settings.get().remote;
    let actions: Vec<_> = PowerAction::all().iter().map(|a| json!({ "action": a, "label": a.label(), "destructive": a.destructive() })).collect();
    Json(json!({ "enabled": s.allow_power, "actions": actions, "pending": core.power.pending(), "delaySeconds": s.power_delay_seconds }))
}

#[derive(Deserialize)]
struct PowerReq {
    action: PowerAction,
    #[serde(default)]
    confirm: bool,
}

async fn power_request(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<PowerReq>) -> ApiResult<Json<serde_json::Value>> {
    let s = core.settings.get().remote;
    if !s.allow_power {
        return Err(ApiError::forbidden("power control from the phone is turned off"));
    }
    if req.action.destructive() && !req.confirm {
        return Err(ApiError::bad("confirm the action first"));
    }
    let delay = if req.action.destructive() { s.power_delay_seconds.clamp(0, 600) } else { 0 };
    let pending = core.power.schedule(req.action, delay, &actor(&dev));
    Ok(Json(json!({ "pending": pending })))
}

async fn power_cancel(State(core): State<Ctx>, Extension(dev): Extension<Device>) -> Json<serde_json::Value> {
    Json(json!({ "cancelled": core.power.cancel(&actor(&dev)) }))
}

// ---------- tasks: what's using the PC ----------

fn tasks_allowed(core: &AppCore) -> ApiResult<()> {
    if core.settings.get().remote.allow_tasks {
        Ok(())
    } else {
        Err(ApiError::forbidden("tasks from the phone are turned off on the PC"))
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct TasksQuery {
    sort: crate::system::procs::ProcessSort,
    limit: Option<usize>,
}

async fn tasks_list(State(core): State<Ctx>, Query(q): Query<TasksQuery>) -> ApiResult<Json<crate::system::procs::Usage>> {
    tasks_allowed(&core)?;
    let c = core.clone();
    Ok(Json(tokio::task::spawn_blocking(move || c.procs.usage(q.sort, q.limit.unwrap_or(60).clamp(1, 500), true)).await?))
}

#[derive(Deserialize)]
struct TaskReq {
    name: String,
    priority: Option<crate::system::procs::Priority>,
}

async fn tasks_end(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<TaskReq>) -> ApiResult<Json<serde_json::Value>> {
    tasks_allowed(&core)?;
    let c = core.clone();
    let name = req.name.clone();
    let r = tokio::task::spawn_blocking(move || c.procs.end(&name)).await?;
    core.audit.record(&actor(&dev), "process.end", &req.name, r.is_ok());
    let ended = r.map_err(ApiError::bad)?;
    Ok(Json(json!({ "ended": ended })))
}

async fn tasks_priority(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<TaskReq>) -> ApiResult<Json<serde_json::Value>> {
    tasks_allowed(&core)?;
    let priority = req.priority.ok_or_else(|| ApiError::bad("missing priority"))?;
    let c = core.clone();
    let name = req.name.clone();
    let r = tokio::task::spawn_blocking(move || c.procs.set_priority(&name, priority)).await?;
    core.audit.record(&actor(&dev), "process.priority", &format!("{} → {priority:?}", req.name), r.is_ok());
    let changed = r.map_err(ApiError::bad)?;
    Ok(Json(json!({ "changed": changed })))
}

// ---------- open apps: close them from the phone ----------

async fn open_apps_list(State(core): State<Ctx>) -> ApiResult<Json<serde_json::Value>> {
    tasks_allowed(&core)?;
    let fake = core.media.is_fake();
    let apps = tokio::task::spawn_blocking(move || crate::system::open_apps::list(fake)).await?;
    Ok(Json(json!({ "apps": apps })))
}

#[derive(Deserialize)]
struct OpenAppReq {
    key: String,
}

/// Ask an app's windows to close, like clicking ×.
async fn open_apps_close(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(req): Json<OpenAppReq>) -> ApiResult<Json<serde_json::Value>> {
    tasks_allowed(&core)?;
    let fake = core.media.is_fake();
    let key = req.key.clone();
    let r = tokio::task::spawn_blocking(move || crate::system::open_apps::close(fake, &key)).await?;
    core.audit.record(&actor(&dev), "app.close", &req.key, r.is_ok());
    let windows = r.map_err(ApiError::bad)?;
    Ok(Json(json!({ "windows": windows })))
}

async fn open_apps_icon(State(core): State<Ctx>, Query(req): Query<OpenAppReq>) -> ApiResult<Json<serde_json::Value>> {
    tasks_allowed(&core)?;
    let fake = core.media.is_fake();
    let icon = tokio::task::spawn_blocking(move || crate::system::open_apps::path_of(fake, &req.key.to_ascii_lowercase()).and_then(|p| crate::apps::exe_icon_data_url(&p))).await?;
    Ok(Json(json!({ "icon": icon })))
}

// ---------- games: boost and launch from the phone ----------

fn games_allowed(core: &AppCore) -> ApiResult<()> {
    if core.settings.get().remote.allow_app_launch {
        Ok(())
    } else {
        Err(ApiError::forbidden("launching apps from the phone is turned off"))
    }
}

async fn games_list(State(core): State<Ctx>) -> ApiResult<Json<serde_json::Value>> {
    games_allowed(&core)?;
    let profiles: Vec<_> = core
        .games
        .profiles()
        .into_iter()
        .map(|p| json!({ "id": p.id, "name": p.name, "kind": p.kind, "lastPlayed": p.last_played, "canLaunch": !matches!(p.launch, crate::games::Launch::None), "process": p.process }))
        .collect();
    Ok(Json(json!({ "profiles": profiles, "session": core.games.session(), "fps": core.games.fps_live() })))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PlayReq {
    /// false: boost only.
    launch: Option<bool>,
}

async fn games_play(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>, body: Option<Json<PlayReq>>) -> ApiResult<Json<serde_json::Value>> {
    games_allowed(&core)?;
    let launch = body.and_then(|b| b.0.launch).unwrap_or(true);
    let r = core.games.play(&id, launch);
    core.audit.record(&actor(&dev), if launch { "game.play" } else { "game.boost" }, &id, r.is_ok());
    let session = r.map_err(ApiError::bad)?;
    Ok(Json(json!({ "session": session })))
}

async fn games_stop(State(core): State<Ctx>) -> ApiResult<Json<serde_json::Value>> {
    games_allowed(&core)?;
    Ok(Json(json!({ "stopped": core.games.stop() })))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PingReq {
    id: Option<String>,
}

async fn games_ping(State(core): State<Ctx>, Json(req): Json<PingReq>) -> ApiResult<Json<serde_json::Value>> {
    games_allowed(&core)?;
    let c = core.clone();
    let results = tokio::task::spawn_blocking(move || c.games.ping(req.id.as_deref(), None)).await?;
    Ok(Json(json!({ "results": results })))
}

// ---------- apps ----------

async fn apps_list(State(core): State<Ctx>) -> ApiResult<Json<serde_json::Value>> {
    if !core.settings.get().remote.allow_app_launch {
        return Err(ApiError::forbidden("launching apps from the phone is turned off"));
    }
    let c = core.clone();
    let apps = tokio::task::spawn_blocking(move || c.apps.list(&|p| c.storage.size_of_path(p).map(|s| s.0))).await?;
    let list: Vec<_> = apps.into_iter().filter(|a| a.launchable).map(|a| json!({ "id": a.id, "name": a.name, "publisher": a.publisher, "source": a.source })).collect();
    Ok(Json(json!({ "apps": list })))
}

async fn app_icon(State(core): State<Ctx>, UrlPath(id): UrlPath<String>) -> ApiResult<Json<serde_json::Value>> {
    let c = core.clone();
    let icon = tokio::task::spawn_blocking(move || c.apps.icon_data_url(&id)).await?;
    Ok(Json(json!({ "icon": icon })))
}

async fn app_launch(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>) -> ApiResult<Json<serde_json::Value>> {
    if !core.settings.get().remote.allow_app_launch {
        return Err(ApiError::forbidden("launching apps from the phone is turned off"));
    }
    let name = core.apps.launch(&id).map_err(|e| ApiError::bad(e.to_string()))?;
    core.audit.record(&actor(&dev), "apps.launch", &name, true);
    Ok(Json(json!({ "launched": name })))
}

// ---------- notes ----------

#[derive(Deserialize, Default)]
#[serde(default)]
struct NotesQuery {
    kind: Option<crate::notes::NoteKind>,
    q: String,
}

async fn notes_list(State(core): State<Ctx>, Query(q): Query<NotesQuery>) -> ApiResult<Json<serde_json::Value>> {
    if !core.settings.get().remote.allow_notes {
        return Err(ApiError::forbidden("notes are not shared with the phone"));
    }
    let notes = core.notes.list(&crate::notes::NoteFilter { kind: q.kind, query: q.q, tag: None, limit: Some(200) })?;
    Ok(Json(json!({ "notes": notes, "claudeFolder": core.settings.get().notes.claude_folder.is_some() })))
}

async fn notes_get(State(core): State<Ctx>, UrlPath(id): UrlPath<String>) -> ApiResult<Json<crate::notes::Note>> {
    if !core.settings.get().remote.allow_notes {
        return Err(ApiError::forbidden("notes are not shared with the phone"));
    }
    core.notes.get(&id).map(Json).map_err(|e| ApiError::not_found(e.to_string()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewNote {
    #[serde(default)]
    title: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    kind: crate::notes::NoteKind,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    send_to_claude: bool,
}

async fn notes_create(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(n): Json<NewNote>) -> ApiResult<Json<crate::notes::Note>> {
    if !core.settings.get().remote.allow_notes {
        return Err(ApiError::forbidden("notes are not shared with the phone"));
    }
    if n.title.trim().is_empty() && n.body.trim().is_empty() {
        return Err(ApiError::bad("the note is empty"));
    }
    let mut tags = n.tags;
    if !tags.iter().any(|t| t == "from-phone") {
        tags.push("from-phone".into());
    }
    let note = core.notes.save(crate::notes::NoteInput { id: None, kind: n.kind, title: n.title, body: n.body, tags, pinned: false, color: None })?;
    let s = core.settings.get().notes;
    let export = n.send_to_claude || (note.kind == crate::notes::NoteKind::Idea && s.auto_export_ideas);
    let note = match (export, s.claude_folder) {
        (true, Some(folder)) => core.notes.export(&note.id, Path::new(&folder), &core.export_options())?,
        _ => note,
    };
    core.audit.record(&actor(&dev), "notes.create", &note.title, true);
    Ok(Json(note))
}

// ---------- Minecraft servers ----------

fn servers_allowed(core: &AppCore) -> ApiResult<()> {
    if core.settings.get().remote.allow_servers {
        Ok(())
    } else {
        Err(ApiError::forbidden("Minecraft servers aren't shared with the phone (Phone → Permissions on the PC)"))
    }
}

async fn servers_list(State(core): State<Ctx>) -> ApiResult<Json<Vec<crate::mcserver::View>>> {
    servers_allowed(&core)?;
    let c = core.clone();
    Ok(Json(tokio::task::spawn_blocking(move || c.mcserver.list()).await?))
}

async fn servers_run(core: Ctx, dev: &Device, id: String, what: &'static str) -> ApiResult<Json<crate::mcserver::View>> {
    servers_allowed(&core)?;
    let c = core.clone();
    let sid = id.clone();
    let r = tokio::task::spawn_blocking(move || match what {
        "start" => c.mcserver.start(&sid),
        "stop" => c.mcserver.stop(&sid),
        _ => c.mcserver.restart(&sid),
    })
    .await?;
    core.audit.record(&actor(dev), &format!("mcserver.{what}"), &id, r.is_ok());
    r.map(Json).map_err(ApiError::bad)
}

async fn servers_start(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>) -> ApiResult<Json<crate::mcserver::View>> {
    servers_run(core, &dev, id, "start").await
}

async fn servers_stop(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>) -> ApiResult<Json<crate::mcserver::View>> {
    servers_run(core, &dev, id, "stop").await
}

async fn servers_restart(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>) -> ApiResult<Json<crate::mcserver::View>> {
    servers_run(core, &dev, id, "restart").await
}

async fn servers_players(State(core): State<Ctx>, UrlPath(id): UrlPath<String>) -> ApiResult<Json<crate::mcserver::PlayersView>> {
    servers_allowed(&core)?;
    let c = core.clone();
    // A phone looking at the players keeps their positions coming.
    tokio::task::spawn_blocking(move || {
        let r = c.mcserver.players(&id);
        if r.is_ok() {
            c.mcserver.watch(&id);
        }
        r
    })
    .await?
    .map(Json)
    .map_err(ApiError::not_found)
}

async fn servers_player(State(core): State<Ctx>, UrlPath((id, name)): UrlPath<(String, String)>) -> ApiResult<Json<crate::mcserver::PlayerView>> {
    servers_allowed(&core)?;
    let c = core.clone();
    tokio::task::spawn_blocking(move || c.mcserver.player(&id, &name)).await?.map(Json).map_err(ApiError::bad)
}

#[derive(Deserialize)]
struct ServerAct {
    name: String,
    action: crate::mcserver::Action,
}

async fn servers_act(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>, Json(req): Json<ServerAct>) -> ApiResult<Json<serde_json::Value>> {
    servers_allowed(&core)?;
    let what: String = format!("{}: {:?}", req.name, req.action).chars().take(160).collect();
    let c = core.clone();
    let r = tokio::task::spawn_blocking(move || c.mcserver.act(&id, &req.name, req.action)).await?;
    core.audit.record(&actor(&dev), "mcserver.player", &what, r.is_ok());
    Ok(Json(json!({ "reply": r.map_err(ApiError::bad)? })))
}

#[derive(Deserialize)]
struct ServerFlag {
    on: bool,
}

async fn servers_whitelist(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>, Json(req): Json<ServerFlag>) -> ApiResult<Json<crate::mcserver::View>> {
    servers_allowed(&core)?;
    let c = core.clone();
    let sid = id.clone();
    let r = tokio::task::spawn_blocking(move || c.mcserver.set_whitelist(&sid, req.on)).await?;
    core.audit.record(&actor(&dev), "mcserver.whitelist", &format!("{id}: {}", if req.on { "on" } else { "off" }), r.is_ok());
    r.map(Json).map_err(ApiError::bad)
}

#[derive(Deserialize)]
struct ServerMax {
    players: u32,
}

async fn servers_max(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>, Json(req): Json<ServerMax>) -> ApiResult<Json<crate::mcserver::View>> {
    servers_allowed(&core)?;
    let c = core.clone();
    let sid = id.clone();
    let r = tokio::task::spawn_blocking(move || c.mcserver.set_properties(&sid, &[("max-players".into(), req.players.to_string())])).await?;
    core.audit.record(&actor(&dev), "mcserver.settings", &format!("{id}: max-players {}", req.players), r.is_ok());
    r.map(Json).map_err(ApiError::bad)
}

#[derive(Deserialize)]
struct ServerJoin {
    /// Players without a paid Minecraft account can join too.
    everyone: bool,
}

async fn servers_join(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>, Json(req): Json<ServerJoin>) -> ApiResult<Json<crate::mcserver::View>> {
    servers_allowed(&core)?;
    let c = core.clone();
    let sid = id.clone();
    let r = tokio::task::spawn_blocking(move || c.mcserver.set_properties(&sid, &[("online-mode".into(), (!req.everyone).to_string())])).await?;
    core.audit.record(&actor(&dev), "mcserver.settings", &format!("{id}: {}", if req.everyone { "everyone can join" } else { "paid accounts only" }), r.is_ok());
    r.map(Json).map_err(ApiError::bad)
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ConsoleQuery {
    after: Option<u64>,
}

async fn servers_console(State(core): State<Ctx>, UrlPath(id): UrlPath<String>, Query(q): Query<ConsoleQuery>) -> ApiResult<Json<Vec<crate::mcserver::process::Line>>> {
    servers_allowed(&core)?;
    let c = core.clone();
    let lines = tokio::task::spawn_blocking(move || c.mcserver.console(&id, q.after)).await?;
    // The newest 400 at most (phones only show the end).
    let skip = lines.len().saturating_sub(400);
    Ok(Json(lines.into_iter().skip(skip).collect()))
}

#[derive(Deserialize)]
struct ServerCommand {
    text: String,
}

async fn servers_command(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(id): UrlPath<String>, Json(req): Json<ServerCommand>) -> ApiResult<Json<serde_json::Value>> {
    servers_allowed(&core)?;
    let c = core.clone();
    let text = req.text.clone();
    let r = tokio::task::spawn_blocking(move || c.mcserver.command_line(&id, &text)).await?;
    core.audit.record(&actor(&dev), "mcserver.command", &req.text.chars().take(120).collect::<String>(), r.is_ok());
    r.map_err(ApiError::bad)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct MapQuery {
    dim: Option<String>,
    x: i32,
    z: i32,
}

async fn servers_map(State(core): State<Ctx>, UrlPath(id): UrlPath<String>, Query(q): Query<MapQuery>) -> ApiResult<Json<crate::mcserver::MapInfo>> {
    servers_allowed(&core)?;
    let c = core.clone();
    tokio::task::spawn_blocking(move || c.mcserver.map_info(&id, q.dim.as_deref().unwrap_or("overworld"))).await?.map(Json).map_err(ApiError::bad)
}

async fn servers_tile(State(core): State<Ctx>, UrlPath(id): UrlPath<String>, Query(q): Query<MapQuery>) -> ApiResult<Json<serde_json::Value>> {
    servers_allowed(&core)?;
    let c = core.clone();
    let url = tokio::task::spawn_blocking(move || c.mcserver.map_tile(&id, q.dim.as_deref().unwrap_or("overworld"), q.x, q.z)).await?.map_err(ApiError::not_found)?;
    Ok(Json(json!({ "url": url })))
}

// ---------- chats ----------

fn chats_allowed(core: &AppCore) -> ApiResult<()> {
    if !core.settings.get().chats.allow_phone {
        return Err(ApiError::forbidden("chats are not shared with the phone (Settings → Chats)"));
    }
    Ok(())
}

fn chat_source(s: &str) -> ApiResult<crate::chats::Source> {
    crate::chats::Source::parse(s).ok_or_else(|| ApiError::not_found("no such chat"))
}

/// Running Claude Code or Codex from a phone: allowed in Settings, and
/// only in folders they already work in.
fn agents_allowed(core: &AppCore) -> ApiResult<()> {
    let s = core.settings.get().chats;
    if !s.phone_agents {
        return Err(ApiError::forbidden("running Claude Code and Codex from the phone is turned off (Settings → Chats)"));
    }
    Ok(())
}

async fn chats_list(State(core): State<Ctx>) -> ApiResult<Json<serde_json::Value>> {
    chats_allowed(&core)?;
    let c = core.clone();
    let (list, tools) = tokio::task::spawn_blocking(move || (c.chats.list(), c.chats.tools())).await?;
    let s = core.settings.get().chats;
    Ok(Json(json!({ "chats": list, "tools": tools, "agents": s.phone_agents, "claudeMode": s.claude_mode, "codexSandbox": s.codex_sandbox })))
}

async fn chats_view(State(core): State<Ctx>, UrlPath((source, id)): UrlPath<(String, String)>) -> ApiResult<Json<crate::chats::ChatView>> {
    chats_allowed(&core)?;
    let source = chat_source(&source)?;
    let c = core.clone();
    tokio::task::spawn_blocking(move || c.chats.view(source, &id)).await?.map(Json).map_err(ApiError::not_found)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatSend {
    text: String,
    #[serde(default)]
    mode: String,
}

async fn chats_send(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath((source, id)): UrlPath<(String, String)>, Json(m): Json<ChatSend>) -> ApiResult<Json<serde_json::Value>> {
    chats_allowed(&core)?;
    let source = chat_source(&source)?;
    if source != crate::chats::Source::Omni {
        agents_allowed(&core)?;
    }
    let c = core.clone();
    let mode = m.mode.clone();
    let r = tokio::task::spawn_blocking(move || c.chats.send(source, &id, &m.text, &m.mode)).await?;
    if source != crate::chats::Source::Omni {
        core.audit.record(&actor(&dev), "chats.agent", &format!("{} {mode}", source.label()), r.is_ok());
    }
    r.map_err(ApiError::bad)?;
    Ok(Json(json!({ "ok": true })))
}

async fn chats_stop(State(core): State<Ctx>, UrlPath((source, id)): UrlPath<(String, String)>) -> ApiResult<Json<serde_json::Value>> {
    chats_allowed(&core)?;
    core.chats.stop(chat_source(&source)?, &id);
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatNew {
    kind: crate::chats::NewKind,
    #[serde(default)]
    folder: Option<String>,
    text: String,
    #[serde(default)]
    mode: String,
}

async fn chats_new(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(n): Json<ChatNew>) -> ApiResult<Json<crate::chats::ChatSummary>> {
    chats_allowed(&core)?;
    let agent = matches!(n.kind, crate::chats::NewKind::ClaudeCode | crate::chats::NewKind::Codex);
    if agent {
        agents_allowed(&core)?;
    }
    let c = core.clone();
    let folder = n.folder.clone();
    let r = tokio::task::spawn_blocking(move || {
        if agent && !n.folder.as_deref().is_some_and(|f| c.chats.known_folder(f)) {
            return Err("From the phone, pick one of the folders Claude Code or Codex already work in.".to_string());
        }
        c.chats.create(n.kind, n.folder.as_deref(), &n.text, &n.mode)
    })
    .await?;
    if agent {
        core.audit.record(&actor(&dev), "chats.agent", &format!("new in {}", folder.unwrap_or_default()), r.is_ok());
    }
    r.map(Json).map_err(ApiError::bad)
}

async fn chats_folders(State(core): State<Ctx>) -> ApiResult<Json<Vec<crate::chats::Folder>>> {
    chats_allowed(&core)?;
    let c = core.clone();
    Ok(Json(tokio::task::spawn_blocking(move || c.chats.folders()).await?))
}

async fn chats_usage(State(core): State<Ctx>) -> ApiResult<Json<crate::chats::usage::Report>> {
    chats_allowed(&core)?;
    let c = core.clone();
    Ok(Json(tokio::task::spawn_blocking(move || c.chats.usage_report(crate::chats::ChatHub::maker_tokens(&c.mods.maker.projects(None)))).await?))
}

async fn ideas_list(State(core): State<Ctx>) -> ApiResult<Json<Vec<crate::notes::Note>>> {
    chats_allowed(&core)?;
    Ok(Json(core.notes.list(&crate::notes::NoteFilter { kind: Some(crate::notes::NoteKind::Idea), limit: Some(300), ..Default::default() })?))
}

#[derive(Deserialize)]
struct IdeaNew {
    text: String,
    #[serde(default)]
    from: Option<String>,
}

async fn ideas_save(State(core): State<Ctx>, Extension(dev): Extension<Device>, Json(i): Json<IdeaNew>) -> ApiResult<Json<crate::notes::Note>> {
    chats_allowed(&core)?;
    let n = crate::chats::save_idea(&core, &i.text, i.from.as_deref()).map_err(ApiError::bad)?;
    core.audit.record(&actor(&dev), "ideas.save", &n.title, true);
    Ok(Json(n))
}

// ---------- vault ----------

fn vault_allowed(core: &AppCore) -> ApiResult<()> {
    if !core.settings.get().vault.allow_phone {
        return Err(ApiError::forbidden("vault access from phones is turned off (Settings → Vault)"));
    }
    if !core.remote.status().tls {
        return Err(ApiError::forbidden("vault access needs the secure (HTTPS) connection"));
    }
    Ok(())
}

#[derive(Deserialize)]
struct VaultUnlock {
    password: String,
}

async fn vault_session(State(core): State<Ctx>, Extension(dev): Extension<Device>, ConnectInfo(peer): ConnectInfo<SocketAddr>, Json(req): Json<VaultUnlock>) -> ApiResult<Json<serde_json::Value>> {
    vault_allowed(&core)?;
    if !core.remote.vault_limiter.allow(peer.ip()) {
        return Err(ApiError::new(StatusCode::TOO_MANY_REQUESTS, "too many attempts; wait a minute"));
    }
    let c = core.clone();
    let password = zeroize::Zeroizing::new(req.password);
    let res = tokio::task::spawn_blocking(move || c.vault.open_snapshot(&password)).await?;
    match res {
        Ok(entries) => {
            let token = super::auth::random_token(32);
            core.remote.vault_sessions.lock().insert(token.clone(), VaultSession { device_id: dev.id.clone(), entries, last_used: Instant::now() });
            core.audit.record(&actor(&dev), "vault.phone-unlock", "", true);
            Ok(Json(json!({ "session": token, "idleSeconds": super::VAULT_SESSION_IDLE.as_secs() })))
        }
        Err(e) => {
            core.audit.record(&actor(&dev), "vault.phone-unlock", &e.to_string(), false);
            Err(ApiError::forbidden(e.to_string()))
        }
    }
}

fn with_session<T>(core: &AppCore, dev: &Device, headers: &HeaderMap, f: impl FnOnce(&mut VaultSession) -> ApiResult<T>) -> ApiResult<T> {
    vault_allowed(core)?;
    // 423 Locked, not 401: a locked vault must not look like an unpaired phone.
    let token = headers.get("x-vault-session").and_then(|v| v.to_str().ok()).ok_or_else(|| ApiError::new(StatusCode::LOCKED, "unlock the vault first"))?;
    let mut sessions = core.remote.vault_sessions.lock();
    let s = sessions.get_mut(token).ok_or_else(|| ApiError::new(StatusCode::LOCKED, "the vault locked; unlock it again"))?;
    if s.device_id != dev.id || s.last_used.elapsed() > super::VAULT_SESSION_IDLE {
        sessions.remove(token);
        return Err(ApiError::new(StatusCode::LOCKED, "the vault locked; unlock it again"));
    }
    s.last_used = Instant::now();
    f(s)
}

async fn vault_entries(State(core): State<Ctx>, Extension(dev): Extension<Device>, headers: HeaderMap) -> ApiResult<Json<serde_json::Value>> {
    let list = with_session(&core, &dev, &headers, |s| Ok(s.entries.iter().map(crate::vault::Vault::summarize).collect::<Vec<_>>()))?;
    Ok(Json(json!({ "entries": list })))
}

#[derive(Deserialize)]
struct RevealReq {
    id: String,
    field: String,
}

async fn vault_reveal(State(core): State<Ctx>, Extension(dev): Extension<Device>, headers: HeaderMap, Json(req): Json<RevealReq>) -> ApiResult<Json<serde_json::Value>> {
    let (title, value) = with_session(&core, &dev, &headers, |s| {
        let e = s.entries.iter().find(|e| e.id == req.id).ok_or_else(|| ApiError::not_found("entry not found"))?;
        let v = match req.field.as_str() {
            "password" => e.password.clone(),
            "notes" => e.notes.clone(),
            "username" => e.username.clone(),
            "email" => e.email.clone(),
            _ => return Err(ApiError::bad("unknown field")),
        };
        Ok((e.title.clone(), v))
    })?;
    core.audit.record(&actor(&dev), "vault.phone-reveal", &format!("{title} ({})", req.field), true);
    Ok(Json(json!({ "value": value })))
}

async fn vault_end(State(core): State<Ctx>, headers: HeaderMap) -> StatusCode {
    if let Some(t) = headers.get("x-vault-session").and_then(|v| v.to_str().ok()) {
        core.remote.vault_sessions.lock().remove(t);
    }
    StatusCode::NO_CONTENT
}

// ---------- sockets ----------

#[derive(Deserialize)]
struct SocketQuery {
    ticket: String,
    monitor: Option<usize>,
    preset: Option<String>,
}

/// Whether an event goes to a given phone. Transfers and inbox offers are
/// private to the device they concern; other phones' names and file names
/// are never sent.
fn for_phone(ev: &crate::events::Event, device_id: &str) -> bool {
    let owner = || ev.payload.get("deviceId").and_then(|d| d.as_str());
    if ev.topic.starts_with("transfer:") {
        return owner() == Some(device_id);
    }
    if ev.topic == "inbox:new" {
        return owner().is_none_or(|d| d == device_id);
    }
    if ev.topic == "inbox:removed" {
        return true;
    }
    ["power:", "notes:", "media:", "games:", "chats:"].iter().any(|p| ev.topic.starts_with(p)) || matches!(ev.topic.as_str(), "mcserver:changed" | "mcserver:players" | "mcserver:guard")
}

async fn events_socket(State(core): State<Ctx>, Query(q): Query<SocketQuery>, ws: WebSocketUpgrade) -> ApiResult<Response> {
    let t = core.remote.tickets.redeem(&q.ticket, true).ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "expired ticket"))?;
    if t.purpose != TicketPurpose::Socket {
        return Err(ApiError::forbidden("wrong ticket"));
    }
    let device_id = t.device_id;
    Ok(ws.on_upgrade(move |socket| async move {
        let (mut tx, mut rx) = socket.split();
        let mut events = core.events.subscribe();
        let mut ping = tokio::time::interval(Duration::from_secs(25));
        let mut revoked_check = tokio::time::interval(Duration::from_secs(5));
        loop {
            tokio::select! {
                ev = events.recv() => {
                    let Ok(ev) = ev else { continue };
                    if !for_phone(&ev, &device_id) || (ev.topic.starts_with("chats:") && !core.settings.get().chats.allow_phone) || (ev.topic.starts_with("mcserver:") && !core.settings.get().remote.allow_servers) {
                        continue;
                    }
                    let text = serde_json::to_string(&ev).unwrap_or_default();
                    if tx.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                msg = rx.next() => {
                    match msg {
                        Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                        _ => {}
                    }
                }
                _ = ping.tick() => {
                    if tx.send(Message::Ping(Bytes::new())).await.is_err() {
                        break;
                    }
                }
                _ = revoked_check.tick() => {
                    // A revoked device is disconnected within seconds.
                    if core.remote.devices.get(&device_id).ok().flatten().is_none_or(|d| d.revoked) {
                        break;
                    }
                }
            }
        }
    }))
}

async fn screen_info(State(core): State<Ctx>) -> ApiResult<Json<serde_json::Value>> {
    let s = core.settings.get().remote;
    if !s.allow_screen {
        return Err(ApiError::forbidden("screen sharing is turned off on the PC"));
    }
    let monitors = tokio::task::spawn_blocking(stream::monitors).await?;
    Ok(Json(json!({ "monitors": monitors, "presets": stream::presets(), "allowControl": s.allow_control, "defaultPreset": core.settings.get().screen.preset })))
}

async fn screen_socket(State(core): State<Ctx>, ConnectInfo(_peer): ConnectInfo<SocketAddr>, Query(q): Query<SocketQuery>, ws: WebSocketUpgrade) -> ApiResult<Response> {
    let t = core.remote.tickets.redeem(&q.ticket, true).ok_or_else(|| ApiError::new(StatusCode::UNAUTHORIZED, "expired ticket"))?;
    if t.purpose != TicketPurpose::Screen || !core.settings.get().remote.allow_screen {
        return Err(ApiError::forbidden("screen sharing is not allowed"));
    }
    let dev = core.remote.devices.get(&t.device_id)?.ok_or_else(|| ApiError::forbidden("unknown device"))?;
    let monitor = q.monitor.unwrap_or(0);
    let preset = stream::preset(q.preset.as_deref().unwrap_or(&core.settings.get().screen.preset));
    Ok(ws.max_message_size(64 * 1024).on_upgrade(move |socket| screen_session(core, dev, monitor, preset, socket)))
}

async fn screen_session(core: Ctx, dev: Device, monitor: usize, preset: stream::Preset, socket: WebSocket) {
    let ctl = StreamControl::new(&preset);
    let viewer_id = uuid::Uuid::new_v4().to_string();
    core.remote.viewers.lock().insert(
        viewer_id.clone(),
        Viewer { info: ViewerInfo { id: viewer_id.clone(), device: dev.name.clone(), monitor: None, since: crate::db::now(), controlling: false }, ctl: ctl.clone() },
    );
    core.audit.record(&actor(&dev), "screen.start", &format!("display {}", monitor + 1), true);
    core.events.emit("screen:viewers", core.remote.status().viewers);

    let (frames_tx, mut frames_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(2);
    let cap_ctl = ctl.clone();
    let capture = std::thread::Builder::new().name("screen-capture".into()).spawn(move || {
        if let Err(e) = stream::run_capture(monitor, cap_ctl, frames_tx) {
            tracing::warn!("screen capture ended: {e}");
        }
    });

    let (mut tx, mut rx) = socket.split();
    let mut stats = tokio::time::interval(Duration::from_secs(1));
    let mut last_frames = 0u32;
    let mut last_bytes = 0u64;
    let mut controlled = false;
    loop {
        tokio::select! {
            frame = frames_rx.recv() => {
                let Some(frame) = frame else { break };
                if tx.send(Message::Binary(frame.into())).await.is_err() {
                    break;
                }
            }
            msg = rx.next() => {
                let text = match msg {
                    Some(Ok(Message::Text(t))) => t,
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => continue,
                };
                let Ok(m) = serde_json::from_str::<ViewerMessage>(&text) else { continue };
                match m {
                    ViewerMessage::Ack { ts, .. } => ctl.ack(ts),
                    ViewerMessage::Preset { id } => {
                        let p = stream::preset(&id);
                        *ctl.params.lock() = (&p).into();
                    }
                    other => {
                        if !core.settings.get().remote.allow_control || stream::paused() {
                            continue;
                        }
                        if !controlled {
                            controlled = true;
                            core.audit.record(&actor(&dev), "screen.control", "remote input started", true);
                            if let Some(v) = core.remote.viewers.lock().get_mut(&viewer_id) {
                                v.info.controlling = true;
                            }
                            core.events.emit("screen:viewers", core.remote.status().viewers);
                        }
                        let mon = ctl.monitor.lock().clone();
                        if let Some(mon) = mon {
                            match other {
                                ViewerMessage::Pointer { x, y, kind, button } => stream::input::pointer(&mon, x, y, kind, button),
                                ViewerMessage::Wheel { dy, dx } => stream::input::wheel(dy, dx),
                                ViewerMessage::Key { key } => { stream::input::chord(&key); }
                                ViewerMessage::Text { text } => stream::input::text(&text),
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ = stats.tick() => {
                if ctl.stop.load(Ordering::Relaxed) {
                    break;
                }
                // A device revoked on the PC loses its stream right away.
                if core.remote.devices.get(&dev.id).ok().flatten().is_none_or(|d| d.revoked) {
                    break;
                }
                let frames = ctl.sent_frames.load(Ordering::Relaxed);
                let bytes = ctl.sent_bytes.load(Ordering::Relaxed);
                let p = ctl.params.lock().clone();
                let monitor = ctl.monitor.lock().clone();
                if let Some(m) = &monitor {
                    if let Some(v) = core.remote.viewers.lock().get_mut(&viewer_id) {
                        v.info.monitor.get_or_insert_with(|| m.clone());
                    }
                }
                let s = json!({
                    "t": "stats",
                    "fps": frames - last_frames,
                    "kbps": (bytes - last_bytes) * 8 / 1000,
                    "rtt": ctl.rtt_ms.load(Ordering::Relaxed),
                    "encodeMs": ctl.encode_ms.load(Ordering::Relaxed),
                    "quality": p.quality,
                    "maxWidth": p.max_width,
                    "monitor": monitor,
                    "control": core.settings.get().remote.allow_control,
                    "paused": stream::paused(),
                    "window": stream::shared_window().is_some(),
                });
                last_frames = frames;
                last_bytes = bytes;
                if tx.send(Message::Text(s.to_string().into())).await.is_err() {
                    break;
                }
            }
        }
    }
    ctl.stop.store(true, Ordering::Relaxed);
    drop(frames_rx);
    if let Ok(h) = capture {
        let _ = tokio::task::spawn_blocking(move || h.join()).await;
    }
    core.remote.viewers.lock().remove(&viewer_id);
    core.audit.record(&actor(&dev), "screen.stop", "", true);
    core.events.emit("screen:viewers", core.remote.status().viewers);
}
