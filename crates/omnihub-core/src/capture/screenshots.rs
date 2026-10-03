//! Screenshot capture and the screenshot library.
//!
//! Captures are saved as files in a normal Pictures folder (so they stay
//! useful without OmniHub), and indexed in SQLite with the app that was in
//! front, tags and a note. Thumbnails live in the cache directory.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::{now, Db};
use crate::events::EventBus;
use crate::settings::ImageFormat;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub id: String,
    pub path: String,
    pub created: i64,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub app_exe: Option<String>,
    pub app_title: Option<String>,
    pub tags: Vec<String>,
    pub note: String,
    pub favorite: bool,
    pub exists: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureKind {
    /// The monitor under the mouse pointer.
    Screen,
    /// Every monitor stitched together.
    AllScreens,
    /// The window in front.
    Window,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ShotFilter {
    pub query: String,
    pub tag: Option<String>,
    pub app: Option<String>,
    pub favorites: bool,
    pub limit: Option<usize>,
}

/// A capture waiting for the user to pick a region on the overlay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PendingRegion {
    pub id: String,
    /// Monitor position and size in physical pixels.
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f32,
    /// JPEG preview of the frozen screen as a data URL for the overlay background.
    pub image: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Default)]
pub struct ForegroundApp {
    pub exe: Option<String>,
    pub title: Option<String>,
    /// Window rectangle in virtual-screen coordinates.
    pub rect: Option<(i32, i32, i32, i32)>,
}

#[derive(Debug, thiserror::Error)]
pub enum ShotError {
    #[error("screen capture failed: {0}")]
    Capture(String),
    #[error("screenshot not found")]
    NotFound,
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct ScreenshotLibrary {
    db: Arc<Db>,
    events: EventBus,
    thumbs: PathBuf,
    pending: Mutex<Option<(String, image::RgbaImage, ForegroundApp)>>,
}

fn sanitize(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    s.chars().take(40).collect::<String>().trim_end_matches('.').trim().to_string()
}

pub fn default_dir() -> PathBuf {
    dirs::picture_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default()).join("OmniHub Screenshots")
}

impl ScreenshotLibrary {
    pub fn new(db: Arc<Db>, events: EventBus, cache: &Path) -> Self {
        let thumbs = cache.join("thumbs");
        let _ = std::fs::create_dir_all(&thumbs);
        ScreenshotLibrary { db, events, thumbs, pending: Mutex::new(None) }
    }

    fn row(r: &rusqlite::Row) -> rusqlite::Result<Screenshot> {
        let path: String = r.get(1)?;
        let tags: String = r.get(8)?;
        Ok(Screenshot {
            id: r.get(0)?,
            exists: Path::new(&path).exists(),
            path,
            created: r.get(2)?,
            width: r.get(3)?,
            height: r.get(4)?,
            bytes: r.get::<_, i64>(5)? as u64,
            app_exe: r.get(6)?,
            app_title: r.get(7)?,
            tags: serde_json::from_str(&tags).unwrap_or_default(),
            note: r.get(9)?,
            favorite: r.get(10)?,
        })
    }

    const COLS: &'static str = "id, path, created, width, height, bytes, app_exe, app_title, tags, note, favorite";

    pub fn list(&self, f: &ShotFilter) -> Result<Vec<Screenshot>, ShotError> {
        let all = self.db.with(|c| {
            let mut st = c.prepare(&format!("SELECT {} FROM screenshots ORDER BY created DESC", Self::COLS))?;
            let rows = st.query_map([], Self::row)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;
        let q = f.query.to_lowercase();
        Ok(all
            .into_iter()
            .filter(|s| !f.favorites || s.favorite)
            .filter(|s| f.tag.as_ref().is_none_or(|t| s.tags.iter().any(|x| x.eq_ignore_ascii_case(t))))
            .filter(|s| f.app.as_ref().is_none_or(|a| s.app_exe.as_ref().is_some_and(|e| e.eq_ignore_ascii_case(a))))
            .filter(|s| {
                q.is_empty()
                    || s.note.to_lowercase().contains(&q)
                    || s.app_title.as_ref().is_some_and(|t| t.to_lowercase().contains(&q))
                    || s.app_exe.as_ref().is_some_and(|t| t.to_lowercase().contains(&q))
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
                    || s.path.to_lowercase().contains(&q)
            })
            .take(f.limit.unwrap_or(usize::MAX))
            .collect())
    }

    pub fn get(&self, id: &str) -> Result<Screenshot, ShotError> {
        self.db
            .with(|c| c.query_row(&format!("SELECT {} FROM screenshots WHERE id=?1", Self::COLS), params![id], Self::row).optional())?
            .ok_or(ShotError::NotFound)
    }

    /// Screenshots taken while an app (identified by its exe) was in front.
    pub fn for_app(&self, exe_names: &[String], install_dir: Option<&str>) -> Result<Vec<Screenshot>, ShotError> {
        let dir = install_dir.map(|d| d.to_lowercase());
        Ok(self
            .list(&ShotFilter::default())?
            .into_iter()
            .filter(|s| {
                s.app_exe.as_ref().is_some_and(|e| {
                    let e = e.to_lowercase();
                    exe_names.iter().any(|n| e.ends_with(&n.to_lowercase())) || dir.as_ref().is_some_and(|d| e.starts_with(d.as_str()))
                })
            })
            .collect())
    }

    /// Save an image, index it and make a thumbnail.
    pub fn store(&self, img: &image::RgbaImage, app: &ForegroundApp, dir: &Path, format: ImageFormat) -> Result<Screenshot, ShotError> {
        let t = chrono::Local::now();
        let folder = dir.join(t.format("%Y-%m").to_string());
        std::fs::create_dir_all(&folder)?;
        // Split on both separators: the path is a Windows path even when
        // this code runs elsewhere (tests).
        let app_name = app
            .exe
            .as_ref()
            .and_then(|e| e.rsplit(['\\', '/']).next())
            .map(|f| f.rsplit_once('.').map_or(f, |(stem, _)| stem))
            .map(sanitize)
            .filter(|n| !n.is_empty());
        let ext = match format {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
        };
        let base = format!("Screenshot {}{}", t.format("%Y-%m-%d %H.%M.%S"), app_name.map(|n| format!(" - {n}")).unwrap_or_default());
        let mut path = folder.join(format!("{base}.{ext}"));
        let mut i = 2;
        while path.exists() {
            path = folder.join(format!("{base} ({i}).{ext}"));
            i += 1;
        }
        match format {
            ImageFormat::Png => img.save_with_format(&path, image::ImageFormat::Png)?,
            ImageFormat::Jpeg => {
                let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
                let mut f = std::io::BufWriter::new(std::fs::File::create(&path)?);
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut f, 92).encode_image(&rgb)?;
            }
        }
        let bytes = std::fs::metadata(&path)?.len();
        let id = uuid::Uuid::new_v4().to_string();
        let created = now();
        let path_s = path.to_string_lossy().to_string();
        self.db.with(|c| {
            c.execute(
                "INSERT INTO screenshots (id, path, created, width, height, bytes, app_exe, app_title) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![id, path_s, created, img.width(), img.height(), bytes as i64, app.exe, app.title],
            )
        })?;
        self.make_thumb(&id, img);
        let shot = self.get(&id)?;
        self.events.emit("screenshots:new", &shot);
        Ok(shot)
    }

    fn make_thumb(&self, id: &str, img: &image::RgbaImage) {
        let w = 360u32.min(img.width().max(1));
        let h = ((img.height() as f64) * (w as f64 / img.width().max(1) as f64)).round().max(1.0) as u32;
        let thumb = image::imageops::thumbnail(img, w, h);
        let rgb = image::DynamicImage::ImageRgba8(thumb).to_rgb8();
        if let Ok(f) = std::fs::File::create(self.thumbs.join(format!("{id}.jpg"))) {
            let mut f = std::io::BufWriter::new(f);
            let _ = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut f, 80).encode_image(&rgb);
        }
    }

    /// Thumbnail as a data URL (created on demand for imported files).
    pub fn thumbnail(&self, id: &str) -> Result<String, ShotError> {
        use base64::Engine;
        let file = self.thumbs.join(format!("{id}.jpg"));
        if !file.exists() {
            let shot = self.get(id)?;
            let img = image::open(&shot.path)?.to_rgba8();
            self.make_thumb(id, &img);
        }
        let bytes = std::fs::read(&file)?;
        Ok(format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
    }

    /// Full image as a data URL (for the viewer).
    pub fn full_image(&self, id: &str) -> Result<String, ShotError> {
        use base64::Engine;
        let shot = self.get(id)?;
        let bytes = std::fs::read(&shot.path)?;
        let mime = if shot.path.to_lowercase().ends_with(".png") { "image/png" } else { "image/jpeg" };
        Ok(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
    }

    pub fn update(&self, id: &str, tags: Option<Vec<String>>, note: Option<String>, favorite: Option<bool>) -> Result<Screenshot, ShotError> {
        let cur = self.get(id)?;
        let tags = tags.unwrap_or(cur.tags);
        let tags: Vec<String> = tags.into_iter().map(|t| t.trim().trim_start_matches('#').to_string()).filter(|t| !t.is_empty()).collect();
        self.db.with(|c| {
            c.execute(
                "UPDATE screenshots SET tags=?2, note=?3, favorite=?4 WHERE id=?1",
                params![id, serde_json::to_string(&tags).unwrap(), note.unwrap_or(cur.note), favorite.unwrap_or(cur.favorite)],
            )
        })?;
        self.get(id)
    }

    /// Remove from the library; the file goes to the Recycle Bin when `trash_file`.
    pub fn delete(&self, id: &str, trash_file: bool) -> Result<(), ShotError> {
        let shot = self.get(id)?;
        if trash_file && Path::new(&shot.path).exists() {
            trash::delete(&shot.path).map_err(|e| std::io::Error::other(e.to_string()))?;
        }
        self.db.with(|c| c.execute("DELETE FROM screenshots WHERE id=?1", params![id]))?;
        let _ = std::fs::remove_file(self.thumbs.join(format!("{id}.jpg")));
        self.events.emit("screenshots:deleted", serde_json::json!({ "id": id }));
        Ok(())
    }

    /// Index image files in `dir` that are not in the library yet (and drop
    /// entries whose files are gone). Returns how many were added.
    pub fn sync_folder(&self, dir: &Path) -> Result<usize, ShotError> {
        let known: std::collections::HashSet<String> = self.list(&ShotFilter::default())?.into_iter().map(|s| s.path.to_lowercase()).collect();
        let mut added = 0;
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp") {
                    continue;
                }
                let ps = p.to_string_lossy().to_string();
                if known.contains(&ps.to_lowercase()) {
                    continue;
                }
                let Ok((w, h)) = image::image_dimensions(&p) else { continue };
                let meta = e.metadata()?;
                let created = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(now(), |d| d.as_secs() as i64);
                let id = uuid::Uuid::new_v4().to_string();
                self.db.with(|c| {
                    c.execute(
                        "INSERT OR IGNORE INTO screenshots (id, path, created, width, height, bytes) VALUES (?1,?2,?3,?4,?5,?6)",
                        params![id, ps, created, w, h, meta.len() as i64],
                    )
                })?;
                added += 1;
            }
        }
        let gone: Vec<String> = self.list(&ShotFilter::default())?.into_iter().filter(|s| !s.exists).map(|s| s.id).collect();
        for id in gone {
            let _ = self.db.with(|c| c.execute("DELETE FROM screenshots WHERE id=?1", params![id]));
        }
        if added > 0 {
            self.events.emit("screenshots:synced", serde_json::json!({ "added": added }));
        }
        Ok(added)
    }

    pub fn capture(&self, kind: CaptureKind, dir: &Path, format: ImageFormat) -> Result<Screenshot, ShotError> {
        let app = foreground_app();
        let img = grab(kind, &app)?;
        self.store(&img, &app, dir, format)
    }

    /// Freeze the screen under the pointer for region selection.
    pub fn begin_region(&self) -> Result<PendingRegion, ShotError> {
        use base64::Engine;
        let app = foreground_app();
        let (img, x, y, scale) = grab_monitor_under_cursor()?;
        // A JPEG preview keeps the hand-off to the overlay window fast even
        // for 4K screens; the crop is taken from the lossless capture.
        let mut preview = std::io::Cursor::new(Vec::new());
        let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut preview, 88).encode_image(&rgb)?;
        let id = uuid::Uuid::new_v4().to_string();
        let pending = PendingRegion {
            id: id.clone(),
            x,
            y,
            width: img.width(),
            height: img.height(),
            scale,
            image: format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(preview.into_inner())),
        };
        *self.pending.lock() = Some((id, img, app));
        Ok(pending)
    }

    /// Crop the frozen screen to `rect` (image pixels) and save it.
    pub fn commit_region(&self, id: &str, rect: Rect, dir: &Path, format: ImageFormat) -> Result<Screenshot, ShotError> {
        let (pid, img, app) = self.pending.lock().take().ok_or(ShotError::NotFound)?;
        if pid != id {
            return Err(ShotError::NotFound);
        }
        let x = rect.x.min(img.width().saturating_sub(1));
        let y = rect.y.min(img.height().saturating_sub(1));
        let w = rect.width.clamp(1, img.width() - x);
        let h = rect.height.clamp(1, img.height() - y);
        let crop = image::imageops::crop_imm(&img, x, y, w, h).to_image();
        self.store(&crop, &app, dir, format)
    }

    pub fn pending_region_id(&self) -> Option<String> {
        self.pending.lock().as_ref().map(|p| p.0.clone())
    }

    pub fn cancel_region(&self) {
        *self.pending.lock() = None;
    }

    pub fn copy_to_clipboard(&self, id: &str) -> Result<(), ShotError> {
        let shot = self.get(id)?;
        let img = image::open(&shot.path)?.to_rgba8();
        crate::system::clipboard::copy_image(img.width() as usize, img.height() as usize, img.as_raw())?;
        Ok(())
    }
}

#[cfg(windows)]
pub fn foreground_app() -> ForegroundApp {
    use windows::Win32::Foundation::{CloseHandle, RECT};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return ForegroundApp::default();
        }
        let mut title = [0u16; 512];
        let n = GetWindowTextW(hwnd, &mut title);
        let title = (n > 0).then(|| String::from_utf16_lossy(&title[..n as usize]));
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let mut exe = None;
        if let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            if QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut len).is_ok() {
                exe = Some(String::from_utf16_lossy(&buf[..len as usize]));
            }
            let _ = CloseHandle(h);
        }
        let mut r = RECT::default();
        let rect = DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut _ as *mut _, std::mem::size_of::<RECT>() as u32)
            .ok()
            .map(|_| (r.left, r.top, r.right, r.bottom));
        ForegroundApp { exe, title, rect }
    }
}

#[cfg(not(windows))]
pub fn foreground_app() -> ForegroundApp {
    ForegroundApp::default()
}

#[cfg(windows)]
fn cursor_pos() -> (i32, i32) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut p = POINT::default();
    let _ = unsafe { GetCursorPos(&mut p) };
    (p.x, p.y)
}

#[cfg(windows)]
fn grab_monitor_under_cursor() -> Result<(image::RgbaImage, i32, i32, f32), ShotError> {
    let (cx, cy) = cursor_pos();
    let m = match xcap::Monitor::from_point(cx, cy) {
        Ok(m) => m,
        Err(_) => xcap::Monitor::all()
            .map_err(|e| ShotError::Capture(e.to_string()))?
            .into_iter()
            .next()
            .ok_or_else(|| ShotError::Capture("no monitor found".into()))?,
    };
    let img = m.capture_image().map_err(|e| ShotError::Capture(e.to_string()))?;
    Ok((img, m.x().unwrap_or(0), m.y().unwrap_or(0), m.scale_factor().unwrap_or(1.0)))
}

#[cfg(not(windows))]
fn grab_monitor_under_cursor() -> Result<(image::RgbaImage, i32, i32, f32), ShotError> {
    Err(ShotError::Capture("screen capture is only implemented on Windows".into()))
}

#[cfg(windows)]
fn grab(kind: CaptureKind, app: &ForegroundApp) -> Result<image::RgbaImage, ShotError> {
    let cap = |e: xcap::XCapError| ShotError::Capture(e.to_string());
    match kind {
        CaptureKind::Screen => Ok(grab_monitor_under_cursor()?.0),
        CaptureKind::AllScreens => {
            let monitors = xcap::Monitor::all().map_err(cap)?;
            let shots: Vec<(i32, i32, image::RgbaImage)> = monitors
                .iter()
                .filter_map(|m| Some((m.x().ok()?, m.y().ok()?, m.capture_image().ok()?)))
                .collect();
            if shots.is_empty() {
                return Err(ShotError::Capture("no monitor could be captured".into()));
            }
            let min_x = shots.iter().map(|s| s.0).min().unwrap();
            let min_y = shots.iter().map(|s| s.1).min().unwrap();
            let max_x = shots.iter().map(|s| s.0 + s.2.width() as i32).max().unwrap();
            let max_y = shots.iter().map(|s| s.1 + s.2.height() as i32).max().unwrap();
            let mut canvas = image::RgbaImage::new((max_x - min_x) as u32, (max_y - min_y) as u32);
            for (x, y, img) in &shots {
                image::imageops::overlay(&mut canvas, img, (*x - min_x) as i64, (*y - min_y) as i64);
            }
            Ok(canvas)
        }
        CaptureKind::Window => {
            let Some((l, t, r, b)) = app.rect else { return Ok(grab_monitor_under_cursor()?.0) };
            let (cx, cy) = ((l + r) / 2, (t + b) / 2);
            let m = xcap::Monitor::from_point(cx, cy).map_err(cap)?;
            let img = m.capture_image().map_err(cap)?;
            let (mx, my) = (m.x().unwrap_or(0), m.y().unwrap_or(0));
            let x0 = (l - mx).clamp(0, img.width() as i32) as u32;
            let y0 = (t - my).clamp(0, img.height() as i32) as u32;
            let x1 = (r - mx).clamp(0, img.width() as i32) as u32;
            let y1 = (b - my).clamp(0, img.height() as i32) as u32;
            if x1 <= x0 || y1 <= y0 {
                return Ok(img);
            }
            Ok(image::imageops::crop_imm(&img, x0, y0, x1 - x0, y1 - y0).to_image())
        }
    }
}

#[cfg(not(windows))]
fn grab(_kind: CaptureKind, _app: &ForegroundApp) -> Result<image::RgbaImage, ShotError> {
    Err(ShotError::Capture("screen capture is only implemented on Windows".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib(dir: &Path) -> ScreenshotLibrary {
        ScreenshotLibrary::new(Arc::new(Db::in_memory().unwrap()), EventBus::new(), dir)
    }

    #[test]
    fn store_list_update_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let l = lib(tmp.path());
        let img = image::RgbaImage::from_fn(800, 600, |x, y| image::Rgba([(x % 256) as u8, (y % 256) as u8, 90, 255]));
        let app = ForegroundApp { exe: Some("C:\\Games\\Foo\\foo.exe".into()), title: Some("Foo - level 3".into()), rect: None };
        let shots = tmp.path().join("shots");
        let s = l.store(&img, &app, &shots, ImageFormat::Png).unwrap();
        assert!(Path::new(&s.path).exists());
        assert!(s.path.ends_with(" - foo.png"));
        assert_eq!((s.width, s.height), (800, 600));
        let s2 = l.store(&img, &ForegroundApp::default(), &shots, ImageFormat::Jpeg).unwrap();
        assert!(s2.path.ends_with(".jpg"));
        assert!(l.thumbnail(&s.id).unwrap().starts_with("data:image/jpeg;base64,"));

        l.update(&s.id, Some(vec!["#boss".into(), "".into()]), Some("beat it".into()), Some(true)).unwrap();
        assert_eq!(l.list(&ShotFilter { favorites: true, ..Default::default() }).unwrap().len(), 1);
        assert_eq!(l.list(&ShotFilter { tag: Some("BOSS".into()), ..Default::default() }).unwrap()[0].note, "beat it");
        assert_eq!(l.list(&ShotFilter { query: "level 3".into(), ..Default::default() }).unwrap().len(), 1);
        assert_eq!(l.for_app(&["foo.exe".into()], None).unwrap().len(), 1);
        assert_eq!(l.for_app(&[], Some("C:\\Games\\Foo")).unwrap().len(), 1);

        l.delete(&s2.id, false).unwrap();
        assert_eq!(l.list(&ShotFilter::default()).unwrap().len(), 1);
        // The file stays (it was not trashed) and is picked up again by sync.
        assert_eq!(l.sync_folder(&shots).unwrap(), 1);
        assert_eq!(l.sync_folder(&shots).unwrap(), 0);
        std::fs::remove_file(&s.path).unwrap();
        l.sync_folder(&shots).unwrap();
        assert_eq!(l.list(&ShotFilter::default()).unwrap().len(), 1, "missing file dropped");
    }

    #[test]
    fn names_are_sanitised() {
        assert_eq!(sanitize("a<b>c:d\"e/f\\g|h?i*j"), "a b c d e f g h i j");
        assert_eq!(sanitize("  dots... "), "dots");
    }
}
