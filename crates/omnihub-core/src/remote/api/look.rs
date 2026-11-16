//! The phone's looks: the PC's theme (so the phone can match it) and the
//! photo wallpapers (the PC's, or one just for the phone).

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path as UrlPath, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Extension, Json, Router};
use serde_json::json;

use super::{actor, ApiError, ApiResult, Ctx};
use crate::remote::auth::Device;

pub(super) fn authed() -> Router<Ctx> {
    Router::new()
        .route("/api/look", get(look))
        .route("/api/wallpaper/{slot}", get(wallpaper_get).post(wallpaper_set).delete(wallpaper_clear).layer(DefaultBodyLimit::max(41 << 20)))
}

/// The PC's theme and the wallpapers there are.
async fn look(State(core): State<Ctx>) -> ApiResult<Json<serde_json::Value>> {
    let g = core.settings.get().general;
    Ok(Json(json!({
        "look": g.look,
        "accent": g.accent,
        "theme": g.theme,
        "glass": g.glass,
        "wallpapers": { "pc": core.wallpapers.info("pc"), "phone": core.wallpapers.info("phone") },
    })))
}

/// The phone-sized copy of a wallpaper.
async fn wallpaper_get(State(core): State<Ctx>, UrlPath(slot): UrlPath<String>) -> ApiResult<Response> {
    let path = core.wallpapers.file(&slot, true).ok_or_else(|| ApiError::not_found("no wallpaper"))?;
    let bytes = tokio::fs::read(path).await?;
    Ok(([(header::CONTENT_TYPE, "image/jpeg"), (header::CACHE_CONTROL, "private, max-age=31536000, immutable")], bytes).into_response())
}

/// A photo from the phone, as the phone's own wallpaper.
async fn wallpaper_set(State(core): State<Ctx>, Extension(dev): Extension<Device>, UrlPath(slot): UrlPath<String>, body: Bytes) -> ApiResult<Json<crate::wallpaper::WallInfo>> {
    if slot != "phone" {
        return Err(ApiError::forbidden("the PC's wallpaper is changed on the PC"));
    }
    let c = core.clone();
    let r = tokio::task::spawn_blocking(move || c.wallpapers.set_bytes("phone", &body)).await?;
    core.audit.record(&actor(&dev), "wallpaper.set", "phone", r.is_ok());
    Ok(Json(r.map_err(ApiError::bad)?))
}

async fn wallpaper_clear(State(core): State<Ctx>, UrlPath(slot): UrlPath<String>) -> ApiResult<StatusCode> {
    if slot != "phone" {
        return Err(ApiError::forbidden("the PC's wallpaper is changed on the PC"));
    }
    core.wallpapers.clear("phone").map_err(ApiError::bad)?;
    Ok(StatusCode::NO_CONTENT)
}
