//! Mirroring the music app's own moving picture: Spotify's Canvas (the short
//! loop it shows beside a song) or the music video it plays, and the same in
//! Apple Music. Neither lets other apps read those, so Aurora watches the
//! app's window the way a screen recorder does (the user's own screen, on
//! this PC, never saved or sent), finds the part that keeps moving, and
//! shows it behind the lyrics, in step with what the app plays.
//!
//! A window covered completely or minimised may stop being drawn (Spotify,
//! like Chrome, saves work then); when the picture stops moving while the
//! song plays, the mirror says so and Aurora goes back to the cover.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
#[cfg(windows)]
use std::time::Duration;
use std::time::Instant;

use parking_lot::Mutex;
use serde::Serialize;

/// Width of the small grey copy the motion is looked for in.
#[cfg(windows)]
const SMALL_W: usize = 160;

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MirrorStatus {
    /// "off", "looking" (for the app's window or something moving in it),
    /// "mirroring", "hidden" (the app stopped drawing), "none" (nothing
    /// moving in it: no Canvas or video showing), "unavailable".
    pub state: String,
    /// "canvas" (portrait, a loop) or "video" (landscape).
    pub kind: Option<String>,
    /// The app being mirrored, for the status line.
    pub app: Option<String>,
    pub width: u32,
    pub height: u32,
}

/// Where things move, in a small grey copy of the window.
pub struct Motion {
    pub w: usize,
    pub h: usize,
    prev: Vec<u8>,
    activity: Vec<f32>,
    frames: u32,
}

impl Motion {
    pub fn new(w: usize, h: usize) -> Motion {
        Motion { w, h, prev: Vec::new(), activity: vec![0.0; w * h], frames: 0 }
    }

    /// One frame of the small copy. Each pixel's activity is the share of
    /// recent frames (about two seconds) in which it changed.
    pub fn feed(&mut self, small: &[u8]) {
        if small.len() != self.w * self.h {
            return;
        }
        if self.prev.len() == small.len() {
            for ((a, s), p) in self.activity.iter_mut().zip(small).zip(&self.prev) {
                let changed = (*s as i16 - *p as i16).abs() > 6;
                *a = *a * 0.92 + if changed { 0.08 } else { 0.0 };
            }
            self.frames += 1;
        }
        self.prev.clear();
        self.prev.extend_from_slice(small);
    }

    /// How much moved inside `r` lately (0–1).
    pub fn activity_in(&self, r: (usize, usize, usize, usize)) -> f32 {
        let (x, y, w, h) = r;
        let mut sum = 0.0;
        for yy in y..(y + h).min(self.h) {
            for xx in x..(x + w).min(self.w) {
                sum += self.activity[yy * self.w + xx];
            }
        }
        sum / (w * h).max(1) as f32
    }

    /// The moving picture: the largest block of pixels that keep changing,
    /// (x, y, w, h) in the small copy. Scattered changes (scrolling lyrics,
    /// a progress bar, a clock) are left out.
    pub fn region(&self) -> Option<(usize, usize, usize, usize)> {
        if self.frames < 8 {
            return None;
        }
        let (w, h) = (self.w, self.h);
        let active: Vec<bool> = self.activity.iter().map(|a| *a > 0.25).collect();
        // Only pixels with mostly active neighbours: blocks, not lines of text.
        let mut dense = vec![false; w * h];
        for y in 1..h.saturating_sub(1) {
            for x in 1..w.saturating_sub(1) {
                let mut n = 0;
                for dy in 0..3 {
                    for dx in 0..3 {
                        n += active[(y + dy - 1) * w + x + dx - 1] as u32;
                    }
                }
                dense[y * w + x] = n >= 7;
            }
        }
        // The largest connected block.
        let mut seen = vec![false; w * h];
        let mut best: Option<(usize, (usize, usize, usize, usize))> = None;
        let mut stack = Vec::new();
        for start in 0..w * h {
            if !dense[start] || seen[start] {
                continue;
            }
            let (mut x0, mut y0, mut x1, mut y1, mut count) = (w, h, 0, 0, 0usize);
            stack.push(start);
            seen[start] = true;
            while let Some(i) = stack.pop() {
                let (x, y) = (i % w, i / w);
                count += 1;
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
                let mut push = |j: usize| {
                    if dense[j] && !seen[j] {
                        seen[j] = true;
                        stack.push(j);
                    }
                };
                if x > 0 {
                    push(i - 1);
                }
                if x + 1 < w {
                    push(i + 1);
                }
                if y > 0 {
                    push(i - w);
                }
                if y + 1 < h {
                    push(i + w);
                }
            }
            if best.is_none_or(|b| count > b.0) {
                // The dense pixels sit one in from the block's edge.
                let r = (x0.saturating_sub(1), y0.saturating_sub(1), (x1 + 2).min(w) - x0.saturating_sub(1), (y1 + 2).min(h) - y0.saturating_sub(1));
                best = Some((count, r));
            }
        }
        let (count, r) = best?;
        let area = r.2 * r.3;
        let aspect = r.2 as f32 / r.3 as f32;
        // Big enough to be a picture, mostly filled, and picture-shaped.
        (area * 100 >= w * h * 3 && count * 10 >= area * 5 && (0.3..=3.2).contains(&aspect)).then_some(r)
    }
}

/// A grey copy of an RGBA picture, `sw`×`sh`, by averaging blocks.
pub fn downsample(rgba: &[u8], w: usize, h: usize, sw: usize, sh: usize) -> Vec<u8> {
    let mut out = vec![0u8; sw * sh];
    if w == 0 || h == 0 {
        return out;
    }
    for sy in 0..sh {
        let (y0, y1) = (sy * h / sh, ((sy + 1) * h / sh).max(sy * h / sh + 1).min(h));
        for sx in 0..sw {
            let (x0, x1) = (sx * w / sw, ((sx + 1) * w / sw).max(sx * w / sw + 1).min(w));
            let (mut sum, mut n) = (0u32, 0u32);
            // Every other row and column is plenty.
            for y in (y0..y1).step_by(2) {
                for x in (x0..x1).step_by(2) {
                    let p = (y * w + x) * 4;
                    sum += (rgba[p] as u32 * 3 + rgba[p + 1] as u32 * 6 + rgba[p + 2] as u32) / 10;
                    n += 1;
                }
            }
            out[sy * sw + sx] = (sum / n.max(1)) as u8;
        }
    }
    out
}

/// Whether two regions are about the same (overlap over union).
pub fn same_region(a: (usize, usize, usize, usize), b: (usize, usize, usize, usize)) -> bool {
    let ix = (a.0 + a.2).min(b.0 + b.2).saturating_sub(a.0.max(b.0));
    let iy = (a.1 + a.3).min(b.1 + b.3).saturating_sub(a.1.max(b.1));
    let inter = (ix * iy) as f32;
    let union = (a.2 * a.3 + b.2 * b.3) as f32 - inter;
    union > 0.0 && inter / union > 0.8
}

/// The window-name token for a media app id ("Spotify.exe",
/// "SpotifyAB.SpotifyMusic_…!Spotify", "AppleInc.AppleMusicWin_…").
pub fn app_token(app: &str) -> Option<&'static str> {
    let a = app.to_ascii_lowercase();
    if a.contains("spotify") {
        Some("spotify")
    } else if a.contains("applemusic") || a.contains("apple music") {
        Some("music")
    } else {
        None
    }
}

pub struct Mirror {
    running: AtomicBool,
    asked: Mutex<(Instant, String, bool)>,
    status: Mutex<MirrorStatus>,
    frame: Mutex<(u32, Vec<u8>)>,
    fake: bool,
}

impl Mirror {
    pub fn new(fake: bool) -> Arc<Mirror> {
        Arc::new(Mirror {
            running: AtomicBool::new(false),
            asked: Mutex::new((Instant::now(), String::new(), false)),
            status: Mutex::new(MirrorStatus { state: "off".into(), ..Default::default() }),
            frame: Mutex::new((0, Vec::new())),
            fake,
        })
    }

    /// Aurora asks every second or two while it wants the mirror of `app`
    /// (the playing app's id); it stops a few seconds after the last ask.
    pub fn hold(self: &Arc<Self>, app: &str, playing: bool) -> MirrorStatus {
        let Some(token) = app_token(app) else {
            return MirrorStatus { state: "unavailable".into(), ..Default::default() };
        };
        if self.fake || !cfg!(windows) {
            return MirrorStatus { state: "unavailable".into(), ..Default::default() };
        }
        *self.asked.lock() = (Instant::now(), token.to_string(), playing);
        if !self.running.swap(true, Ordering::SeqCst) {
            *self.status.lock() = MirrorStatus { state: "looking".into(), app: Some(token.into()), ..Default::default() };
            let me = self.clone();
            std::thread::Builder::new()
                .name("app-mirror".into())
                .spawn(move || {
                    me.run();
                    me.running.store(false, Ordering::SeqCst);
                    *me.status.lock() = MirrorStatus { state: "off".into(), ..Default::default() };
                })
                .ok();
        }
        self.status.lock().clone()
    }

    pub fn status(&self) -> MirrorStatus {
        self.status.lock().clone()
    }

    /// The newest picture (JPEG) if it is newer than `after`.
    pub fn frame(&self, after: u32) -> Option<(u32, Vec<u8>)> {
        let f = self.frame.lock();
        (f.0 != after && !f.1.is_empty() && self.status.lock().state == "mirroring").then(|| (f.0, f.1.clone()))
    }

    #[cfg(not(windows))]
    fn run(&self) {}

    #[cfg(windows)]
    fn run(&self) {
        let me = std::process::id();
        let mut window: Option<xcap::Window> = None;
        let mut found_at = Instant::now() - Duration::from_secs(10);
        let mut motion: Option<(u32, u32, Motion)> = None;
        let mut region: Option<(usize, usize, usize, usize)> = None;
        let mut candidate: Option<((usize, usize, usize, usize), u32)> = None;
        let mut still_since: Option<Instant> = None;
        let mut nothing_since = Instant::now();
        let mut encoder = crate::capture::stream::JpegEncoder::default();
        let mut seq = 0u32;
        let period = Duration::from_millis(80);
        loop {
            let started = Instant::now();
            let (asked, token, playing) = self.asked.lock().clone();
            if asked.elapsed() > Duration::from_secs(5) {
                return;
            }
            // The app's biggest visible window, looked up again now and then.
            if window.is_none() || found_at.elapsed() > Duration::from_secs(3) {
                found_at = Instant::now();
                window = xcap::Window::all().unwrap_or_default().into_iter().filter(|w| w.pid().ok() != Some(me) && !w.is_minimized().unwrap_or(true) && w.app_name().is_ok_and(|n| n.to_ascii_lowercase().contains(&token))).max_by_key(|w| w.width().unwrap_or(0) as u64 * w.height().unwrap_or(0) as u64);
            }
            let Some(win) = window.as_ref() else {
                self.set("looking", None, &token, 0, 0);
                std::thread::sleep(Duration::from_millis(500));
                continue;
            };
            let Ok(img) = win.capture_image() else {
                window = None;
                continue;
            };
            let (w, h) = img.dimensions();
            if w < 200 || h < 150 {
                std::thread::sleep(Duration::from_millis(500));
                continue;
            }
            let sh = ((h as usize * SMALL_W) / w as usize).max(16);
            if motion.as_ref().is_none_or(|m| (m.0, m.1) != (w, h)) {
                motion = Some((w, h, Motion::new(SMALL_W, sh)));
                region = None;
            }
            let (_, _, m) = motion.as_mut().unwrap();
            let raw = img.as_raw();
            m.feed(&downsample(raw, w as usize, h as usize, SMALL_W, sh));
            // The region settles: a new one has to be seen three times in a row.
            if m.frames.is_multiple_of(4) {
                match m.region() {
                    Some(r) if region.is_some_and(|cur| same_region(cur, r)) => candidate = None,
                    Some(r) => {
                        let n = candidate.filter(|c| same_region(c.0, r)).map_or(1, |c| c.1 + 1);
                        if n >= 3 || region.is_none() && n >= 2 {
                            region = Some(r);
                            candidate = None;
                        } else {
                            candidate = Some((r, n));
                        }
                    }
                    None => {}
                }
            }
            let Some(r) = region else {
                if nothing_since.elapsed() > Duration::from_secs(6) {
                    self.set("none", None, &token, 0, 0);
                } else {
                    self.set("looking", None, &token, 0, 0);
                }
                std::thread::sleep(period.saturating_sub(started.elapsed()));
                continue;
            };
            nothing_since = Instant::now();
            // Still while the song plays: the app stopped drawing (covered or minimised).
            if playing && m.activity_in(r) < 0.02 {
                let since = *still_since.get_or_insert_with(Instant::now);
                if since.elapsed() > Duration::from_secs(3) {
                    self.set("hidden", None, &token, 0, 0);
                    std::thread::sleep(Duration::from_millis(250));
                    continue;
                }
            } else {
                still_since = None;
            }
            // The region in the full picture, a pixel in from its edges.
            let (fx, fy) = (w as f32 / m.w as f32, h as f32 / m.h as f32);
            let x0 = ((r.0 as f32 * fx) as u32 + 1).min(w - 2);
            let y0 = ((r.1 as f32 * fy) as u32 + 1).min(h - 2);
            let cw = ((r.2 as f32 * fx) as u32).saturating_sub(2).clamp(1, w - x0);
            let ch = ((r.3 as f32 * fy) as u32).saturating_sub(2).clamp(1, h - y0);
            let mut data = Vec::with_capacity((cw * ch * 4) as usize);
            for y in y0..y0 + ch {
                let row = ((y * w + x0) * 4) as usize;
                data.extend_from_slice(&raw[row..row + (cw * 4) as usize]);
            }
            for px in data.as_chunks_mut::<4>().0 {
                px.swap(0, 2); // RGBA -> BGRA, as the encoder takes it
            }
            let frame = crate::capture::stream::Frame { width: cw, height: ch, stride: cw as usize * 4, data, cursor: None };
            if encoder.encode(&frame, 1280, 82).is_ok() {
                seq = seq.wrapping_add(1).max(1);
                *self.frame.lock() = (seq, encoder.out.clone());
                let kind = if cw * 10 < ch * 8 { "canvas" } else { "video" };
                self.set("mirroring", Some(kind), &token, cw, ch);
            }
            std::thread::sleep(period.saturating_sub(started.elapsed()));
        }
    }

    #[cfg(windows)]
    fn set(&self, state: &str, kind: Option<&str>, app: &str, width: u32, height: u32) {
        *self.status.lock() = MirrorStatus { state: state.into(), kind: kind.map(Into::into), app: Some(app.into()), width, height };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 160×100 "window": static UI, a moving picture at (100, 20, 40×70),
    /// and a line of text that changes now and then.
    fn frame(t: u32) -> Vec<u8> {
        let (w, h) = (160usize, 100usize);
        let mut f = vec![40u8; w * h];
        for y in 20..90 {
            for x in 100..140 {
                f[y * w + x] = ((x as u32 * 7 + y as u32 * 13 + t * 37) % 251) as u8;
            }
        }
        if t.is_multiple_of(3) {
            for x in 10..80 {
                f[60 * w + x] = if (x + t as usize).is_multiple_of(2) { 200 } else { 40 };
            }
        }
        f
    }

    #[test]
    fn finds_the_moving_picture_and_ignores_text() {
        let mut m = Motion::new(160, 100);
        assert_eq!(m.region(), None);
        for t in 0..40 {
            m.feed(&frame(t));
        }
        let (x, y, w, h) = m.region().expect("the picture");
        assert!(x.abs_diff(100) <= 1 && y.abs_diff(20) <= 1 && w.abs_diff(40) <= 2 && h.abs_diff(70) <= 2, "{:?}", (x, y, w, h));
        assert!(m.activity_in((x, y, w, h)) > 0.3);
        // It stops moving (the app stopped drawing): activity fades.
        let still = frame(40);
        for _ in 0..60 {
            m.feed(&still);
        }
        assert!(m.activity_in((x, y, w, h)) < 0.02);
    }

    #[test]
    fn nothing_moving_is_no_region() {
        let mut m = Motion::new(160, 100);
        let f = vec![90u8; 160 * 100];
        for _ in 0..30 {
            m.feed(&f);
        }
        assert_eq!(m.region(), None);
    }

    #[test]
    fn downsamples_and_compares_regions() {
        let (w, h) = (320usize, 200usize);
        let mut rgba = vec![0u8; w * h * 4];
        for y in 0..h {
            for x in 160..w {
                rgba[(y * w + x) * 4..(y * w + x) * 4 + 3].copy_from_slice(&[255, 255, 255]);
            }
        }
        let s = downsample(&rgba, w, h, 160, 100);
        assert_eq!((s[50 * 160 + 10], s[50 * 160 + 150]), (0, 255));
        assert!(same_region((10, 10, 50, 50), (11, 10, 50, 51)));
        assert!(!same_region((10, 10, 50, 50), (40, 10, 50, 50)));
        assert_eq!(app_token("Spotify.exe"), Some("spotify"));
        assert_eq!(app_token("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"), Some("spotify"));
        assert_eq!(app_token("AppleInc.AppleMusicWin_nzyj5cx40ttqa!App"), Some("music"));
        assert_eq!(app_token("chrome.exe"), None);
    }
}
