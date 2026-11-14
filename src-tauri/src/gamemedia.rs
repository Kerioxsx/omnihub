//! `gamemedia://localhost/<token>`: files the game library lists (artwork,
//! screenshots, clips, soundtrack tracks), by the token it handed out only.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use omnihub_core::core::AppCore;
use omnihub_core::rig::screens;

fn mime(p: &Path) -> &'static str {
    match p.extension().map(|e| e.to_string_lossy().to_lowercase()).as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("mp4") => "video/mp4",
        Some("mkv") => "video/x-matroska",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        Some("flac") => "audio/flac",
        Some("ogg" | "opus") => "audio/ogg",
        Some("wav") => "audio/wav",
        Some("m4a" | "aac") => "audio/mp4",
        _ => "application/octet-stream",
    }
}

pub fn serve(core: &AppCore, request: &tauri::http::Request<Vec<u8>>) -> tauri::http::Response<Vec<u8>> {
    use tauri::http::{Response, StatusCode};
    let fail = |code: StatusCode| Response::builder().status(code).body(Vec::new()).unwrap_or_default();
    let token = request.uri().path().trim_start_matches('/');
    let Some(path) = core.shelf.media_path(token) else { return fail(StatusCode::NOT_FOUND) };
    let Ok(mut f) = std::fs::File::open(&path) else { return fail(StatusCode::NOT_FOUND) };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let kind = mime(&path);
    let asked = request.headers().get("range").and_then(|v| v.to_str().ok());
    // Pictures whole; sound and video 4 MB at a time.
    let chunk = if kind.starts_with("image/") { screens::PICTURE_MAX } else { 4 << 20 };
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
        .header("Content-Type", kind)
        .header("Accept-Ranges", "bytes")
        .header("Content-Length", body.len().to_string())
        .header("Content-Range", format!("bytes {start}-{end}/{len}"))
        .header("Cache-Control", "max-age=3600")
        .body(body)
        .unwrap_or_default()
}
