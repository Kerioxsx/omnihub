//! A picture for a Minecraft mix. OmniHub blends the packs' own icons into
//! one itself, with no account and nothing sent anywhere: a soft diagonal
//! for two packs, wedges for more, pixel art kept crisp. With the user's own
//! Pollinations account it can also ask an image model to paint one new
//! picture from those icons. That key is kept like the other keys: encrypted
//! for this Windows account, never shown to the window, sent only to
//! Pollinations.

use std::f32::consts::PI;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Covers are this many pixels square.
pub const SIZE: u32 = 512;
/// The Minecraft Launcher shows profile icons this big.
pub const ICON: u32 = 128;
pub const GEN: &str = "https://gen.pollinations.ai";
pub const ENTER: &str = "https://enter.pollinations.ai";
/// Takes several reference pictures, is open to free accounts, and costs
/// about 0.005 Pollen a picture.
pub const MODEL: &str = "black-forest-labs/flux.2-klein-4b";
const SECRET_ENTROPY: &[u8] = b"omnihub-pollinations";
const MAX_PICTURE: usize = 16 << 20;
/// At most this many packs show in one cover.
const MOST: usize = 6;

// ---------- pictures ----------

/// Decode a PNG, JPEG, GIF, WebP or BMP, refusing very big ones.
pub fn decode(bytes: &[u8]) -> Option<RgbaImage> {
    if bytes.is_empty() || bytes.len() > MAX_PICTURE {
        return None;
    }
    let mut r = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format().ok()?;
    let mut lim = image::Limits::default();
    lim.max_image_width = Some(4096);
    lim.max_image_height = Some(4096);
    lim.max_alloc = Some(256 << 20);
    r.limits(lim);
    Some(r.decode().ok()?.to_rgba8())
}

pub fn png(img: &RgbaImage) -> Vec<u8> {
    let mut out = Vec::new();
    let _ = DynamicImage::ImageRgba8(img.clone()).write_to(&mut Cursor::new(&mut out), ImageFormat::Png);
    out
}

/// A PNG as a `data:` address the window can show.
pub fn data_url(png: &[u8]) -> String {
    use base64::Engine;
    format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png))
}

/// A cover as the Minecraft Launcher's profile icon (`data:image/png;base64,…`).
pub fn launcher_icon(cover_png: &[u8]) -> Option<String> {
    use base64::Engine;
    let img = decode(cover_png)?;
    let small = image::imageops::resize(&img, ICON, ICON, FilterType::CatmullRom);
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png(&small))))
}

fn hash(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3))
}

fn hsl(h: f32, s: f32, l: f32) -> [u8; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h.rem_euclid(360.0)) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [((r + m) * 255.0).round() as u8, ((g + m) * 255.0).round() as u8, ((b + m) * 255.0).round() as u8]
}

/// A pack without an icon gets a block-like tile in a colour of its own.
pub fn placeholder(name: &str) -> RgbaImage {
    let h = hash(name);
    let hue = (h % 360) as f32;
    let mut small = RgbaImage::new(8, 8);
    for (x, y, p) in small.enumerate_pixels_mut() {
        let n = hash(&format!("{h}:{x}:{y}")) % 1000;
        let l = 0.30 + (n as f32 / 1000.0) * 0.16 + if y < 2 { 0.08 } else { 0.0 };
        let [r, g, b] = hsl(hue, 0.55, l);
        *p = Rgba([r, g, b, 255]);
    }
    image::imageops::resize(&small, SIZE, SIZE, FilterType::Nearest)
}

fn average(img: &RgbaImage) -> [f32; 3] {
    let (mut sum, mut n) = ([0f32; 3], 0f32);
    for p in img.pixels().step_by(7) {
        let a = p[3] as f32 / 255.0;
        if a > 0.5 {
            for i in 0..3 {
                sum[i] += p[i] as f32;
            }
            n += 1.0;
        }
    }
    if n == 0.0 {
        return [40.0, 40.0, 48.0];
    }
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

/// Small pictures with few colours, or mostly runs of the same colour, are
/// pixel art: they're enlarged block by block so they stay crisp.
fn pixel_art(img: &RgbaImage) -> bool {
    let (w, h) = img.dimensions();
    if w.max(h) > 160 {
        return false;
    }
    let mut colours = std::collections::HashSet::new();
    let (mut same, mut pairs) = (0u32, 0u32);
    for y in 0..h {
        for x in 0..w {
            let p = img.get_pixel(x, y);
            if colours.len() <= 256 {
                colours.insert(p.0);
            }
            if x + 1 < w {
                pairs += 1;
                same += (img.get_pixel(x + 1, y) == p) as u32;
            }
        }
    }
    colours.len() <= 256 || (pairs > 0 && same * 100 / pairs >= 45)
}

/// An icon as a full square tile: the middle of it, scaled (pixel art stays
/// crisp), with see-through parts on a dark tone of its own colour.
pub fn tile(icon: &RgbaImage) -> RgbaImage {
    let (w, h) = icon.dimensions();
    let side = w.min(h).max(1);
    let sq = image::imageops::crop_imm(icon, (w - side) / 2, (h - side) / 2, side, side).to_image();
    let filter = if pixel_art(&sq) { FilterType::Nearest } else { FilterType::Lanczos3 };
    let big = image::imageops::resize(&sq, SIZE, SIZE, filter);
    let avg = average(&big);
    let bg = [avg[0] * 0.32, avg[1] * 0.32, avg[2] * 0.32];
    let mut out = RgbaImage::new(SIZE, SIZE);
    for (x, y, p) in big.enumerate_pixels() {
        let a = p[3] as f32 / 255.0;
        let px = |i: usize| (p[i] as f32 * a + bg[i] * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
        out.put_pixel(x, y, Rgba([px(0), px(1), px(2), 255]));
    }
    out
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Nearest-neighbour enlarged pixel art: almost every pixel matches its neighbour.
fn blocky(img: &RgbaImage) -> bool {
    let (w, h) = img.dimensions();
    let (mut same, mut pairs) = (0u32, 0u32);
    for y in (0..h).step_by(3) {
        for x in (0..w.saturating_sub(1)).step_by(3) {
            pairs += 1;
            same += (img.get_pixel(x, y) == img.get_pixel(x + 1, y)) as u32;
        }
    }
    pairs > 0 && same * 100 / pairs >= 80
}

/// Distance from (px, py) to a rounded square (centre c, half side h, corner r); negative inside.
fn rounded(px: f32, py: f32, c: (f32, f32), h: f32, r: f32) -> f32 {
    let (qx, qy) = ((px - c.0).abs() - h + r, (py - c.1).abs() - h + r);
    (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r
}

/// Where each pack's part is: (its weight at a pixel, for `n` packs).
fn weights(n: usize, fx: f32, fy: f32, s: f32, w: &mut [f32]) {
    let c = (s - 1.0) / 2.0;
    let feather = 0.045 * s;
    match n {
        0 => {}
        1 => w[0] = 1.0,
        2 => {
            // Across the line from the top right corner to the bottom left one.
            let d = (fx + fy - (s - 1.0)) / std::f32::consts::SQRT_2;
            let b = smoothstep(-feather, feather, d);
            w[0] = 1.0 - b;
            w[1] = b;
        }
        _ => {
            let (dx, dy) = (fx - c, fy - c);
            let r = (dx * dx + dy * dy).sqrt().max(1.0);
            // Clockwise from the left, so four packs sit in the four corners.
            let a = (dy.atan2(dx) + PI).rem_euclid(2.0 * PI);
            let width = 2.0 * PI / n as f32;
            let fr = (feather / r).min(width / 2.0);
            for (k, wk) in w.iter_mut().enumerate().take(n) {
                let mid = (k as f32 + 0.5) * width;
                let d = ((a - mid + PI).rem_euclid(2.0 * PI) - PI).abs();
                *wk = 1.0 - smoothstep(width / 2.0 - fr, width / 2.0 + fr, d);
            }
        }
    }
}

/// Where each pack's icon goes (centre) and how big it is (side), for `n` packs.
fn spots(n: usize, s: f32) -> Vec<((f32, f32), f32)> {
    let c = (s - 1.0) / 2.0;
    match n {
        0 => vec![],
        1 => vec![((c, c), s * 0.84)],
        2 => vec![((s / 3.0, s / 3.0), s * 0.6), ((s * 2.0 / 3.0, s * 2.0 / 3.0), s * 0.6)],
        _ => {
            let (side, dist) = match n {
                3 => (0.46, 0.27),
                4 => (0.38, 0.3),
                _ => (0.32, 0.31),
            };
            let width = 2.0 * PI / n as f32;
            (0..n)
                .map(|k| {
                    let a = (k as f32 + 0.5) * width - PI;
                    ((c + s * dist * a.cos(), c + s * dist * a.sin()), s * side)
                })
                .collect()
        }
    }
}

/// Blend tiles into one cover. Each pack gets its own part (a soft diagonal
/// for two, wedges for three to six; the first at the top left, then
/// clockwise), filled with a blurred wash of its colours, and its icon whole
/// in the middle of that part as a rounded tile with a soft shadow. Packs
/// meet on soft seams, and a light vignette ties it together.
pub fn blend(tiles: &[RgbaImage]) -> RgbaImage {
    let tiles: Vec<&RgbaImage> = tiles.iter().take(MOST).collect();
    let n = tiles.len();
    let s = SIZE as f32;
    let c = (s - 1.0) / 2.0;
    let spots = spots(n, s);
    // Each pack's wash, and its icon at the size it's shown.
    let washes: Vec<RgbaImage> = tiles
        .iter()
        .map(|t| {
            let small = image::imageops::resize(*t, 48, 48, FilterType::Triangle);
            let soft = image::imageops::blur(&small, 5.0);
            image::imageops::resize(&soft, SIZE, SIZE, FilterType::Triangle)
        })
        .collect();
    let icons: Vec<RgbaImage> = tiles
        .iter()
        .zip(&spots)
        .map(|(t, (_, side))| {
            let px = side.round().max(1.0) as u32;
            image::imageops::resize(*t, px, px, if blocky(t) { FilterType::Nearest } else { FilterType::Lanczos3 })
        })
        .collect();
    let mut out = RgbaImage::new(SIZE, SIZE);
    let mut w = vec![0f32; n.max(1)];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (fx, fy) = (x as f32, y as f32);
            weights(n, fx, fy, s, &mut w);
            let total: f32 = w.iter().take(n).sum::<f32>().max(1e-6);
            let mut rgb = if n == 0 { [36.0, 36.0, 44.0] } else { [0f32; 3] };
            for k in 0..n {
                if w[k] <= 0.0 {
                    continue;
                }
                let ((cx, cy), side) = spots[k];
                let half = side / 2.0;
                let corner = side * 0.14;
                let bg = washes[k].get_pixel(x, y);
                // The wash, darker, and darker still under the icon's shadow.
                let shadow = 1.0 - 0.5 * (1.0 - smoothstep(0.0, side * 0.09, rounded(fx - side * 0.02, fy - side * 0.035, (cx, cy), half, corner)));
                let mut p = [bg[0] as f32 * 0.5 * shadow, bg[1] as f32 * 0.5 * shadow, bg[2] as f32 * 0.5 * shadow];
                let d = rounded(fx, fy, (cx, cy), half, corner);
                if d < 1.0 {
                    let a = 1.0 - smoothstep(-1.0, 1.0, d);
                    let ix = (fx - (cx - half)).floor();
                    let iy = (fy - (cy - half)).floor();
                    let icon = &icons[k];
                    if ix >= 0.0 && iy >= 0.0 && (ix as u32) < icon.width() && (iy as u32) < icon.height() {
                        let q = icon.get_pixel(ix as u32, iy as u32);
                        for i in 0..3 {
                            p[i] = p[i] * (1.0 - a) + q[i] as f32 * a;
                        }
                    }
                }
                for i in 0..3 {
                    rgb[i] += p[i] * w[k] / total;
                }
            }
            let dist = ((fx - c).powi(2) + (fy - c).powi(2)).sqrt() / (s * std::f32::consts::FRAC_1_SQRT_2);
            let v = 1.0 - 0.22 * smoothstep(0.62, 1.0, dist);
            out.put_pixel(x, y, Rgba([(rgb[0] * v).round().clamp(0.0, 255.0) as u8, (rgb[1] * v).round().clamp(0.0, 255.0) as u8, (rgb[2] * v).round().clamp(0.0, 255.0) as u8, 255]));
        }
    }
    out
}

// ---------- Pollinations ----------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
struct Stored {
    /// The key, DPAPI-encrypted and base64'd.
    api_secret: String,
    /// The Pollinations user name it belongs to, when known.
    user: Option<String>,
    /// `signIn` (approved on Pollinations' site) or `key` (pasted).
    via: String,
    connected_at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PollStatus {
    pub connected: bool,
    pub user: Option<String>,
    pub via: Option<String>,
    pub connected_at: Option<i64>,
    /// A sign-in is waiting for the user to approve it.
    pub waiting: bool,
}

/// What the user sees to approve OmniHub on Pollinations' site.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignIn {
    pub user_code: String,
    /// Pollinations' approval page, with the code filled in.
    pub url: String,
    pub expires_in: u64,
    pub interval: u64,
}

struct Pending {
    device_code: zeroize::Zeroizing<String>,
    until: std::time::Instant,
}

pub struct Pollinations {
    file: PathBuf,
    stored: Mutex<Stored>,
    pending: Mutex<Option<Pending>>,
    gen: String,
    enter: String,
    agent: ureq::Agent,
    #[cfg(test)]
    test_key: Mutex<Option<String>>,
}

fn key_ok(key: &str) -> bool {
    key.starts_with("sk_") && (20..=400).contains(&key.len()) && key.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

fn plain_name(name: &str) -> String {
    let kept: String = name.chars().map(|c| if c.is_control() { ' ' } else { c }).filter(|c| !matches!(c, '"' | '\\' | '<' | '>')).take(60).collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// What the image model is asked for.
pub fn prompt(name: &str, n: usize) -> String {
    let name = plain_name(name);
    let named = if name.is_empty() { String::new() } else { format!(" called \"{name}\"") };
    format!(
        "Make one new square cover picture for a Minecraft modpack{named} by blending these {n} modpack icons into a single picture, as if the packs were merged into one. \
         Combine their main subjects, colours and mood into one scene. Keep the art style of the icons: pixel art stays pixel art. \
         Bold and easy to read when small, centered. No text, letters, numbers, logos, watermarks or borders."
    )
}

impl Pollinations {
    pub fn new(data_dir: &Path) -> Pollinations {
        Pollinations::with(data_dir, GEN, ENTER)
    }

    pub fn with(data_dir: &Path, gen: &str, enter: &str) -> Pollinations {
        let file = data_dir.join("pollinations.json");
        let stored: Stored = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let mut cfg = ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(20))).timeout_global(Some(Duration::from_secs(240))).http_status_as_error(false);
        if gen.starts_with("http://127.0.0.1") {
            cfg = cfg.proxy(None);
        }
        Pollinations {
            file,
            stored: Mutex::new(stored),
            pending: Mutex::new(None),
            gen: gen.trim_end_matches('/').into(),
            enter: enter.trim_end_matches('/').into(),
            agent: cfg.build().into(),
            #[cfg(test)]
            test_key: Mutex::new(None),
        }
    }

    fn ua() -> &'static str {
        concat!("Kerioxsx/omnihub/", env!("CARGO_PKG_VERSION"), " (OmniHub)")
    }

    pub fn status(&self) -> PollStatus {
        let s = self.stored.lock().clone();
        let connected = self.has_key();
        let waiting = self.pending.lock().as_ref().is_some_and(|p| p.until > std::time::Instant::now());
        PollStatus { connected, user: s.user.filter(|_| connected), via: Some(s.via).filter(|v| connected && !v.is_empty()), connected_at: connected.then_some(s.connected_at), waiting }
    }

    pub fn has_key(&self) -> bool {
        #[cfg(test)]
        if self.test_key.lock().is_some() {
            return true;
        }
        !self.stored.lock().api_secret.is_empty()
    }

    fn key(&self) -> Option<zeroize::Zeroizing<String>> {
        use base64::Engine;
        #[cfg(test)]
        if let Some(k) = self.test_key.lock().clone() {
            return Some(zeroize::Zeroizing::new(k));
        }
        let secret = self.stored.lock().api_secret.clone();
        if secret.is_empty() {
            return None;
        }
        let bytes = base64::engine::general_purpose::STANDARD.decode(secret).ok()?;
        let plain = crate::system::dpapi::unprotect(&bytes, SECRET_ENTROPY).ok()?;
        Some(zeroize::Zeroizing::new(String::from_utf8_lossy(&plain).into_owned()))
    }

    fn save(&self, s: &Stored) -> Result<(), String> {
        if let Some(p) = self.file.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let tmp = self.file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.file).map_err(|e| e.to_string())
    }

    fn keep(&self, key: &str, user: Option<String>, via: &str) -> Result<(), String> {
        #[cfg(test)]
        {
            *self.test_key.lock() = Some(key.to_string());
            let s = Stored { api_secret: String::new(), user, via: via.into(), connected_at: chrono::Utc::now().timestamp() };
            self.save(&s)?;
            *self.stored.lock() = s;
            Ok(())
        }
        #[cfg(not(test))]
        {
            use base64::Engine;
            let secret = base64::engine::general_purpose::STANDARD.encode(crate::system::dpapi::protect(key.as_bytes(), SECRET_ENTROPY).map_err(|e| e.to_string())?);
            let s = Stored { api_secret: secret, user, via: via.into(), connected_at: chrono::Utc::now().timestamp() };
            self.save(&s)?;
            *self.stored.lock() = s;
            Ok(())
        }
    }

    /// Use the user's own Pollinations secret key (`sk_…`); empty forgets it.
    pub fn set_key(&self, key: &str) -> Result<(), String> {
        let key = key.trim();
        if key.is_empty() {
            return self.forget();
        }
        if !key_ok(key) {
            return Err("That doesn't look like a Pollinations secret key (they start with sk_). Get one at enter.pollinations.ai, or sign in instead.".into());
        }
        let user = self.whoami(key).ok().flatten();
        self.keep(key, user, "key")
    }

    pub fn forget(&self) -> Result<(), String> {
        #[cfg(test)]
        {
            *self.test_key.lock() = None;
        }
        *self.pending.lock() = None;
        let s = Stored::default();
        self.save(&s)?;
        *self.stored.lock() = s;
        Ok(())
    }

    fn post(&self, url: &str, body: &Value, key: Option<&str>) -> Result<(u16, Value), String> {
        let mut req = self.agent.post(url).header("User-Agent", Self::ua()).header("Content-Type", "application/json");
        if let Some(k) = key {
            req = req.header("Authorization", &format!("Bearer {k}"));
        }
        let mut res = req.send(body.to_string()).map_err(|e| format!("Couldn't reach Pollinations: {e}"))?;
        let status = res.status().as_u16();
        let mut out = Vec::new();
        res.body_mut().as_reader().take(48 << 20).read_to_end(&mut out).map_err(|e| format!("Pollinations' answer stopped: {e}"))?;
        Ok((status, serde_json::from_slice(&out).unwrap_or(Value::Null)))
    }

    fn whoami(&self, key: &str) -> Result<Option<String>, String> {
        let mut res = self.agent.get(&format!("{}/api/device/userinfo", self.enter)).header("User-Agent", Self::ua()).header("Authorization", &format!("Bearer {key}")).call().map_err(|e| e.to_string())?;
        if res.status().as_u16() != 200 {
            return Ok(None);
        }
        let mut out = Vec::new();
        res.body_mut().as_reader().take(1 << 20).read_to_end(&mut out).map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_slice(&out).unwrap_or(Value::Null);
        Ok(v["preferred_username"].as_str().or(v["name"].as_str()).map(|s| s.chars().take(60).collect()))
    }

    /// Start signing in: the user approves OmniHub (and sets its spending
    /// cap) on Pollinations' site; `check_sign_in` then picks up the key.
    pub fn sign_in(&self) -> Result<SignIn, String> {
        let (status, v) = self.post(&format!("{}/api/device/code", self.enter), &json!({ "models": "image", "budget": 2, "expiry": 30, "scope": "profile" }), None)?;
        if status != 200 {
            return Err(format!("Pollinations couldn't start the sign-in ({status})."));
        }
        let code = v["device_code"].as_str().filter(|c| !c.is_empty()).ok_or("Pollinations didn't send a sign-in code.")?;
        let user_code = v["user_code"].as_str().filter(|c| !c.is_empty() && c.len() <= 32 && c.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')).ok_or("Pollinations sent an odd sign-in code.")?;
        // Only ever open Pollinations' own page.
        let page = format!("{}/device?user_code={user_code}", self.enter);
        let url = v["verification_uri_complete"].as_str().filter(|u| u.starts_with(&format!("{}/", self.enter))).unwrap_or(&page).to_string();
        let expires_in = v["expires_in"].as_u64().unwrap_or(1800).clamp(60, 3600);
        let interval = v["interval"].as_u64().unwrap_or(5).clamp(2, 30);
        *self.pending.lock() = Some(Pending { device_code: zeroize::Zeroizing::new(code.to_string()), until: std::time::Instant::now() + Duration::from_secs(expires_in) });
        Ok(SignIn { user_code: user_code.into(), url, expires_in, interval })
    }

    /// Whether the user has approved the sign-in yet (Ok(true) once connected).
    pub fn check_sign_in(&self) -> Result<bool, String> {
        let code = {
            let p = self.pending.lock();
            match p.as_ref() {
                Some(p) if p.until > std::time::Instant::now() => p.device_code.clone(),
                Some(_) => return Err("The sign-in code ran out. Start again.".into()),
                None => return Err("No sign-in is waiting.".into()),
            }
        };
        let (status, v) = self.post(&format!("{}/api/device/token", self.enter), &json!({ "device_code": code.as_str() }), None)?;
        match v["error"].as_str() {
            Some("authorization_pending") | Some("slow_down") => return Ok(false),
            Some("expired_token") => {
                *self.pending.lock() = None;
                return Err("The sign-in code ran out. Start again.".into());
            }
            Some("access_denied") => {
                *self.pending.lock() = None;
                return Err("The sign-in was declined on Pollinations.".into());
            }
            _ => {}
        }
        let key = v["access_token"].as_str().filter(|k| key_ok(k)).ok_or_else(|| format!("Pollinations didn't finish the sign-in ({status})."))?;
        let user = self.whoami(key).ok().flatten();
        self.keep(key, user, "signIn")?;
        *self.pending.lock() = None;
        Ok(true)
    }

    /// One new picture painted from the packs' tiles (PNGs), as a SIZE×SIZE PNG.
    pub fn blend(&self, tiles: &[Vec<u8>], name: &str) -> Result<Vec<u8>, String> {
        use base64::Engine;
        let key = self.key().ok_or("Connect your Pollinations account first.")?;
        if tiles.is_empty() {
            return Err("There are no pack pictures to blend.".into());
        }
        let images: Vec<Value> = tiles.iter().take(MOST).map(|t| json!({ "image_url": format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(t)) })).collect();
        let seed = (chrono::Utc::now().timestamp_subsec_nanos() % 2_000_000_000) as i64;
        let body = json!({ "model": MODEL, "prompt": prompt(name, images.len()), "image": images, "size": "1024x1024", "n": 1, "response_format": "b64_json", "seed": seed });
        let (status, v) = self.post(&format!("{}/v1/images/edits", self.gen), &body, Some(key.as_str()))?;
        if status != 200 {
            let said = v["error"]["message"].as_str().or(v["error"].as_str()).or(v["message"].as_str()).unwrap_or("").chars().take(200).collect::<String>();
            return Err(match status {
                401 | 403 => "Pollinations didn't accept your sign-in. Connect your account again.".to_string(),
                402 => "Your Pollinations account has no Pollen left, or reached the cap you set for OmniHub. Top up or raise it at enter.pollinations.ai.".into(),
                429 => "Pollinations is busy right now. Try again in a minute.".into(),
                500..=599 => "Pollinations had a problem making the picture. Try again.".into(),
                _ if !said.is_empty() => format!("Pollinations said: {said}"),
                _ => format!("Pollinations answered {status}."),
            });
        }
        let b64 = v["data"][0]["b64_json"].as_str().ok_or("Pollinations didn't send a picture.")?;
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|_| "Pollinations sent a picture that couldn't be read.")?;
        let img = decode(&bytes).ok_or("Pollinations sent a picture that couldn't be read.")?;
        let sq = tile(&img);
        Ok(png(&sq))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::tests::{serve, Seen};

    fn solid(c: [u8; 3]) -> RgbaImage {
        RgbaImage::from_pixel(64, 64, Rgba([c[0], c[1], c[2], 255]))
    }

    fn near(a: &Rgba<u8>, b: [u8; 3], tol: i32) -> bool {
        (0..3).all(|i| (a[i] as i32 - b[i] as i32).abs() <= tol)
    }

    #[test]
    fn two_packs_split_on_a_soft_diagonal() {
        let (red, blue) = ([220, 40, 40], [40, 60, 220]);
        let c = blend(&[tile(&solid(red)), tile(&solid(blue))]);
        assert_eq!(c.dimensions(), (SIZE, SIZE));
        assert!(near(c.get_pixel(150, 200), red, 4), "the first pack at the top left: {:?}", c.get_pixel(150, 200));
        assert!(near(c.get_pixel(360, 320), blue, 4), "the second at the bottom right: {:?}", c.get_pixel(360, 320));
        let mid = c.get_pixel(255, 256);
        assert!(mid[0] > 90 && mid[2] > 90, "the seam blends both: {mid:?}");
        // The corners are a little darker than the middle of each side.
        assert!(c.get_pixel(2, 2)[0] < c.get_pixel(150, 200)[0]);
    }

    #[test]
    fn more_packs_become_wedges_and_pixel_art_stays_crisp() {
        let cols = [[220, 40, 40], [40, 200, 60], [40, 60, 220]];
        let c = blend(&cols.iter().map(|&k| tile(&solid(k))).collect::<Vec<_>>());
        // Each icon whole in its own part, clockwise from the left: red (top left), green (right), blue (bottom left).
        for (k, ((x, y), _)) in spots(3, SIZE as f32).into_iter().enumerate() {
            let p = c.get_pixel(x as u32, y as u32);
            assert!(near(p, cols[k], 4), "pack {k} at ({x}, {y}): {p:?}");
        }
        let (tl, r) = (spots(3, SIZE as f32)[0].0, spots(3, SIZE as f32)[1].0);
        assert!(tl.0 < 256.0 && tl.1 < 256.0 && r.0 > 300.0, "{tl:?} {r:?}");
        // Around an icon, its part is a darker wash of the same colour.
        let edge = c.get_pixel(30, 250);
        assert!(edge[0] > edge[1] && edge[0] < 150, "{edge:?}");

        // A 16-pixel icon: only its own two colours inside each cell.
        let mut px = RgbaImage::new(16, 16);
        for (x, y, p) in px.enumerate_pixels_mut() {
            *p = if (x + y) % 2 == 0 { Rgba([250, 250, 250, 255]) } else { Rgba([10, 10, 10, 255]) };
        }
        let t = tile(&px);
        let cell = SIZE / 16;
        assert_eq!(*t.get_pixel(cell / 2, cell / 2), Rgba([250, 250, 250, 255]));
        assert_eq!(*t.get_pixel(cell + cell / 2, cell / 2), Rgba([10, 10, 10, 255]));
        assert_eq!(*t.get_pixel(cell - 1, cell / 2), Rgba([250, 250, 250, 255]), "no smudged edges");

        // A small smooth picture (a 96-pixel thumbnail) is smoothed, not blocky.
        let smooth = RgbaImage::from_fn(96, 96, |x, y| Rgba([(x * 255 / 95) as u8, (y * 255 / 95) as u8, ((x * y) % 251) as u8, 255]));
        assert!(!pixel_art(&smooth) && pixel_art(&px));
        let t = tile(&smooth);
        assert_ne!(t.get_pixel(2, 100), t.get_pixel(3, 100), "no blocks");

        // See-through icons sit on a dark tone of their own colour.
        let mut clear = RgbaImage::from_pixel(32, 32, Rgba([0, 0, 0, 0]));
        for x in 8..24 {
            for y in 8..24 {
                clear.put_pixel(x, y, Rgba([200, 100, 0, 255]));
            }
        }
        let t = tile(&clear);
        let bg = t.get_pixel(4, 4);
        assert!(bg[3] == 255 && bg[0] > bg[2] && bg[0] < 100, "{bg:?}");
    }

    #[test]
    fn packs_without_icons_get_their_own_tile_and_covers_become_launcher_icons() {
        let a = placeholder("Create Plus");
        assert_eq!(a, placeholder("Create Plus"));
        assert_ne!(a.get_pixel(100, 300), placeholder("Speed").get_pixel(100, 300));
        let cover = png(&blend(&[a, placeholder("Speed")]));
        let icon = launcher_icon(&cover).unwrap();
        assert!(icon.starts_with("data:image/png;base64,"));
        use base64::Engine;
        let back = decode(&base64::engine::general_purpose::STANDARD.decode(&icon[22..]).unwrap()).unwrap();
        assert_eq!(back.dimensions(), (ICON, ICON));
        assert!(decode(b"not a picture").is_none());
        assert!(decode(&vec![0u8; MAX_PICTURE + 1]).is_none());
    }

    /// Writes sample covers from real icons: OMNIHUB_COVER_SAMPLES=dir with icon files.
    #[test]
    #[ignore]
    fn sample_covers() {
        let Ok(dir) = std::env::var("OMNIHUB_COVER_SAMPLES") else { return };
        let dir = std::path::PathBuf::from(dir);
        let mut icons: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "webp" || x == "png") && !p.file_name().unwrap().to_string_lossy().starts_with("cover")).collect();
        icons.sort();
        let tiles: Vec<RgbaImage> = icons.iter().map(|p| tile(&decode(&std::fs::read(p).unwrap()).unwrap())).collect();
        for n in 1..=tiles.len() {
            std::fs::write(dir.join(format!("cover-{n}.png")), png(&blend(&tiles[..n]))).unwrap();
        }
        std::fs::write(dir.join("cover-placeholder.png"), png(&blend(&[placeholder("Create Plus"), placeholder("Speed")]))).unwrap();
    }

    #[test]
    fn prompt_keeps_the_name_plain() {
        let p = prompt("Speed \"+\" Create\n<b>", 2);
        assert!(p.contains("called \"Speed + Create b\"") && p.contains("2 modpack icons") && p.contains("No text"), "{p}");
    }

    #[test]
    fn signs_in_and_blends_with_the_users_own_account() {
        use base64::Engine;
        let polls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let p2 = polls.clone();
        let made = png(&RgbaImage::from_pixel(1024, 1024, Rgba([30, 160, 90, 255])));
        let made_b64 = base64::engine::general_purpose::STANDARD.encode(&made);
        let s = serve(Box::new(move |r: &Seen| {
            let ok = |v: Value| Some((200, "application/json", v.to_string().into_bytes()));
            let auth = r.headers.iter().find(|(k, _)| k == "authorization").map(|(_, v)| v.clone());
            match r.path.as_str() {
                "/api/device/code" => ok(json!({ "device_code": "dev-123", "user_code": "ABCD-1234", "verification_uri": "https://evil.example/device", "verification_uri_complete": "https://evil.example/device?user_code=ABCD-1234", "expires_in": 1800, "interval": 5 })),
                "/api/device/token" => {
                    let body: Value = serde_json::from_slice(&r.body).unwrap();
                    assert_eq!(body["device_code"], "dev-123");
                    if p2.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                        Some((400, "application/json", br#"{"error":"authorization_pending"}"#.to_vec()))
                    } else {
                        ok(json!({ "access_token": "sk_userkey1234567890abcdef", "token_type": "bearer" }))
                    }
                }
                "/api/device/userinfo" if auth.as_deref() == Some("Bearer sk_userkey1234567890abcdef") => ok(json!({ "sub": "u1", "preferred_username": "kerio" })),
                "/v1/images/edits" => {
                    if auth.as_deref() != Some("Bearer sk_userkey1234567890abcdef") {
                        return Some((401, "application/json", br#"{"error":{"message":"bad key"}}"#.to_vec()));
                    }
                    let body: Value = serde_json::from_slice(&r.body).unwrap();
                    assert_eq!(body["model"], MODEL);
                    assert_eq!(body["image"].as_array().unwrap().len(), 2);
                    assert!(body["image"][0]["image_url"].as_str().unwrap().starts_with("data:image/png;base64,"));
                    assert!(body["prompt"].as_str().unwrap().contains("called \"Speed + Create"));
                    if body["prompt"].as_str().unwrap().contains("Broke") {
                        return Some((402, "application/json", br#"{"error":{"message":"insufficient pollen"}}"#.to_vec()));
                    }
                    ok(json!({ "created": 1, "data": [{ "b64_json": made_b64 }] }))
                }
                _ => None,
            }
        }));
        let t = tempfile::tempdir().unwrap();
        let p = Pollinations::with(t.path(), &s.base, &s.base);
        assert!(!p.status().connected);
        assert_eq!(p.blend(&[vec![1]], "x").unwrap_err(), "Connect your Pollinations account first.");

        let si = p.sign_in().unwrap();
        assert_eq!(si.user_code, "ABCD-1234");
        assert_eq!(si.url, format!("{}/device?user_code=ABCD-1234", s.base), "only Pollinations' own page is opened");
        assert!(p.status().waiting);
        assert!(!p.check_sign_in().unwrap(), "not approved yet");
        assert!(p.check_sign_in().unwrap());
        let st = p.status();
        assert_eq!((st.connected, st.user.as_deref(), st.via.as_deref(), st.waiting), (true, Some("kerio"), Some("signIn"), false));
        let saved = std::fs::read_to_string(t.path().join("pollinations.json")).unwrap();
        assert!(!saved.contains("sk_userkey"), "the key is never written in the clear");

        let tiles = vec![png(&tile(&solid([200, 40, 40]))), png(&tile(&solid([40, 40, 200])))];
        let out = decode(&p.blend(&tiles, "Speed + Create").unwrap()).unwrap();
        assert_eq!(out.dimensions(), (SIZE, SIZE));
        assert!(near(out.get_pixel(200, 200), [30, 160, 90], 3));
        assert!(p.blend(&tiles, "Speed + Create Broke").unwrap_err().contains("no Pollen left"));

        assert!(p.set_key("sk-ant-api03-not-pollinations").unwrap_err().contains("sk_"));
        assert!(p.set_key("sk_has spaces in it oh no").is_err());
        p.forget().unwrap();
        assert!(!p.status().connected);
        assert!(p.check_sign_in().unwrap_err().contains("No sign-in"));
    }
}
