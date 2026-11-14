//! Download from your phone: share or paste a link on the phone (or the PC)
//! and the PC downloads it into Downloads\OmniHub. A link to a file downloads
//! as it is; a page with a video (YouTube, TikTok, X, Instagram…) goes through
//! yt-dlp, the helper music videos already use.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;
use crate::media::MediaHub;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// A file as it is, or a page's video.
    #[default]
    Auto,
    /// Only the sound of a page's video.
    Audio,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Download {
    pub id: String,
    pub url: String,
    /// The file's name once known.
    pub name: Option<String>,
    pub path: Option<String>,
    pub state: State,
    /// 0–100, when the size is known.
    pub percent: Option<f32>,
    pub bytes: u64,
    pub error: Option<String>,
    /// The phone that asked (or "This PC").
    pub from: String,
    pub started_at: i64,
}

pub struct Downloads {
    media: Arc<MediaHub>,
    ffmpeg: PathBuf,
    events: EventBus,
    list: Arc<Mutex<Vec<Download>>>,
    stops: Arc<Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>>,
}

pub fn folder() -> PathBuf {
    dirs::download_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Downloads")).join("OmniHub")
}

/// http(s) only; the link from text a phone shared ("Look at this https://…").
pub fn clean_url(text: &str) -> Result<String, String> {
    let u = text.split_whitespace().find(|w| w.starts_with("https://") || w.starts_with("http://")).ok_or("That isn't a web link (it must start with https://).")?;
    let u = u.trim_end_matches(['"', '\'', ')', ']', '>', ',']);
    if u.len() > 4096 || u.chars().any(char::is_control) {
        return Err("That link is too long.".into());
    }
    Ok(u.to_string())
}

/// The file name for a direct download.
pub fn name_for(url: &str, disposition: Option<&str>) -> String {
    let from_header = disposition.and_then(|d| {
        // filename*=UTF-8''na%20me.zip wins over filename="name.zip".
        let star = d.split(';').map(str::trim).find_map(|p| p.strip_prefix("filename*=")).and_then(|v| v.rsplit("''").next()).map(|v| percent_encoding::percent_decode_str(v.trim_matches('"')).decode_utf8_lossy().into_owned());
        star.or_else(|| d.split(';').map(str::trim).find_map(|p| p.strip_prefix("filename=")).map(|v| v.trim_matches('"').to_string()))
    });
    let from_url = || {
        let path = url.split(['?', '#']).next().unwrap_or("");
        let last = path.trim_end_matches('/').rsplit('/').next().unwrap_or("");
        let after_host = path.splitn(4, '/').nth(3).is_some_and(|p| !p.trim_matches('/').is_empty());
        (after_host && !last.is_empty()).then(|| percent_encoding::percent_decode_str(last).decode_utf8_lossy().into_owned())
    };
    let name = from_header.filter(|n| !n.trim().is_empty()).or_else(from_url).unwrap_or_else(|| "download".into());
    crate::remote::guest::safe_name(&name)
}

impl Downloads {
    pub fn new(media: Arc<MediaHub>, ffmpeg: PathBuf, events: EventBus) -> Self {
        Downloads { media, ffmpeg, events, list: Default::default(), stops: Default::default() }
    }

    pub fn list(&self) -> Vec<Download> {
        self.list.lock().iter().rev().cloned().collect()
    }

    pub fn clear(&self) {
        self.list.lock().retain(|d| d.state == State::Running);
        self.events.emit("download:changed", serde_json::json!({}));
    }

    pub fn cancel(&self, id: &str) {
        if let Some(s) = self.stops.lock().get(id) {
            s.store(true, Ordering::SeqCst);
        }
    }

    /// Start downloading; returns the id. Progress comes as `download:changed`.
    pub fn start(self: &Arc<Self>, text: &str, mode: Mode, from: &str) -> Result<String, String> {
        let url = clean_url(text)?;
        if self.list.lock().iter().filter(|d| d.state == State::Running).count() >= 5 {
            return Err("Five downloads are already running; wait for one to finish.".into());
        }
        let id = crate::remote::auth::random_token(10);
        let d = Download { id: id.clone(), url: url.clone(), name: None, path: None, state: State::Running, percent: None, bytes: 0, error: None, from: from.chars().take(60).collect(), started_at: chrono::Utc::now().timestamp() };
        self.list.lock().push(d.clone());
        let stop = Arc::new(AtomicBool::new(false));
        self.stops.lock().insert(id.clone(), stop.clone());
        self.events.emit("download:changed", &d);
        let me = self.clone();
        let task = id.clone();
        std::thread::spawn(move || {
            let id = task;
            let r = me.fetch(&id, &url, mode, &stop);
            me.stops.lock().remove(&id);
            let cancelled = stop.load(Ordering::SeqCst);
            me.update(&id, |d| match &r {
                Ok(p) => {
                    d.state = State::Done;
                    d.percent = Some(100.0);
                    d.path = Some(p.display().to_string());
                    d.name = p.file_name().map(|n| n.to_string_lossy().into_owned());
                    d.bytes = std::fs::metadata(p).map(|m| m.len()).unwrap_or(d.bytes);
                }
                Err(e) => {
                    d.state = if cancelled { State::Cancelled } else { State::Failed };
                    d.error = (!cancelled).then(|| e.clone());
                }
            });
            if let Ok(p) = r {
                me.events.emit("download:done", serde_json::json!({ "name": p.file_name().map(|n| n.to_string_lossy().into_owned()), "path": p.display().to_string() }));
            }
        });
        Ok(id)
    }

    fn update(&self, id: &str, f: impl FnOnce(&mut Download)) {
        let mut l = self.list.lock();
        if let Some(d) = l.iter_mut().find(|d| d.id == id) {
            f(d);
            self.events.emit("download:changed", &*d);
        }
    }

    fn fetch(&self, id: &str, url: &str, mode: Mode, stop: &AtomicBool) -> Result<PathBuf, String> {
        let dir = folder();
        std::fs::create_dir_all(&dir).map_err(|e| format!("Couldn't make {}: {e}", dir.display()))?;
        let mut res = None;
        if mode == Mode::Auto {
            let r = crate::capture::airplay::agent().get(url).header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) OmniHub").call().map_err(|e| format!("Couldn't open the link: {e}"))?;
            let html = r.headers().get("content-type").and_then(|v| v.to_str().ok()).is_some_and(|t| t.starts_with("text/html"));
            if !html {
                res = Some(r);
            }
        }
        match res {
            Some(r) => self.direct(id, url, r, &dir, stop),
            None => self.page(id, url, mode == Mode::Audio, &dir, stop),
        }
    }

    fn direct(&self, id: &str, url: &str, mut res: ureq::http::Response<ureq::Body>, dir: &Path, stop: &AtomicBool) -> Result<PathBuf, String> {
        let disposition = res.headers().get("content-disposition").and_then(|v| v.to_str().ok()).map(String::from);
        let name = name_for(url, disposition.as_deref());
        let total = res.body().content_length();
        self.update(id, |d| d.name = Some(name.clone()));
        let dest = crate::remote::transfer::unique_path(dir, &name);
        let part = dest.with_file_name(format!(".{}.omnihub-part", uuid::Uuid::new_v4()));
        let r = (|| -> Result<(), String> {
            let mut file = std::io::BufWriter::new(std::fs::File::create(&part).map_err(|e| e.to_string())?);
            let mut reader = res.body_mut().with_config().limit(u64::MAX).reader();
            let mut buf = vec![0u8; 256 * 1024];
            let (mut done, mut last) = (0u64, Instant::now());
            loop {
                if stop.load(Ordering::SeqCst) {
                    return Err("Cancelled.".into());
                }
                let n = reader.read(&mut buf).map_err(|e| format!("The download stopped: {e}"))?;
                if n == 0 {
                    break;
                }
                file.write_all(&buf[..n]).map_err(|e| format!("Couldn't save: {e}"))?;
                done += n as u64;
                if last.elapsed().as_millis() >= 300 {
                    last = Instant::now();
                    self.update(id, |d| {
                        d.bytes = done;
                        d.percent = total.filter(|t| *t > 0).map(|t| (done as f64 / t as f64 * 100.0).min(99.0) as f32);
                    });
                }
            }
            file.flush().map_err(|e| e.to_string())?;
            if total.is_some_and(|t| t != done) {
                return Err("The download ended early.".into());
            }
            Ok(())
        })();
        match r.and_then(|_| std::fs::rename(&part, &dest).map_err(|e| e.to_string())) {
            Ok(()) => Ok(dest),
            Err(e) => {
                let _ = std::fs::remove_file(&part);
                Err(e)
            }
        }
    }

    fn page(&self, id: &str, url: &str, audio: bool, dir: &Path, stop: &AtomicBool) -> Result<PathBuf, String> {
        let tool = self.media.stream_tool();
        if !tool.status().installed {
            self.update(id, |d| d.name = Some("Getting the video helper (once)…".into()));
            tool.install(&mut |_, _| {})?;
        }
        self.update(id, |d| d.name = None);
        let ffmpeg = self.ffmpeg.is_file().then_some(self.ffmpeg.as_path());
        tool.download(url, dir, audio, ffmpeg, stop, &mut |p| self.update(id, |d| d.percent = Some(p.min(99.0))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_and_names() {
        assert_eq!(clean_url("look at this https://youtu.be/abc?si=x, cool").unwrap(), "https://youtu.be/abc?si=x");
        assert!(clean_url("file:///C:/Windows").is_err() && clean_url("javascript:alert(1)").is_err());
        assert_eq!(name_for("https://x.com/files/My%20Song.mp3?dl=1", None), "My Song.mp3");
        assert_eq!(name_for("https://x.com/", None), "download");
        assert_eq!(name_for("https://x.com/get", Some(r#"attachment; filename="report.pdf""#)), "report.pdf");
        assert_eq!(name_for("https://x.com/get", Some("attachment; filename=\"a.zip\"; filename*=UTF-8''r%C3%A9sum%C3%A9.zip")), "résumé.zip");
        assert_eq!(name_for("https://x.com/get", Some(r#"attachment; filename="..\..\evil.bat""#)), "evil.bat");
        assert_eq!(crate::media::stream::percent_in("[download]  42.3% of   10.00MiB at  1.2MiB/s ETA 00:05"), Some(42.3));
        assert_eq!(crate::media::stream::percent_in("[Merger] Merging formats"), None);
    }
}
