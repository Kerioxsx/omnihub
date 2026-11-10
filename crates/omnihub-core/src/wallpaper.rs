//! Your own photo as the wallpaper behind the whole app, on the PC and on
//! the phone. The photo is turned the right way up (its EXIF orientation),
//! sized so it covers a 4K screen (smaller photos are enlarged with Lanczos
//! and sharpened; bigger ones are brought down to 4K), and saved as a JPEG
//! in OmniHub's data folder with a lighter copy for phones. Colours are read
//! from it so the app's theme can match.
//!
//! Enlarging can't invent detail a small photo doesn't have; Lanczos and a
//! light sharpen keep edges clean instead of blurry.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::{DynamicImage, ImageDecoder, RgbImage};
use serde::{Deserialize, Serialize};

/// "pc": behind OmniHub on the PC (the phone can use it too). "phone": a
/// photo just for the phone.
pub const SLOTS: &[&str] = &["pc", "phone"];
const MAX_BYTES: usize = 40 << 20;
/// Pictures bigger than this (in pixels) are refused before decoding.
const MAX_PIXELS: u64 = 120_000_000;

/// The colours read from a photo.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Palette {
    /// The main accent ("#7c9cff"), bright enough to read on the theme.
    pub accent: String,
    pub accent2: String,
    /// The photo is bright: use the light palette.
    pub light: bool,
    /// A dark (or, when light, pale) colour from the photo for backgrounds.
    pub tint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WallInfo {
    pub slot: String,
    /// Changes with every new photo (unix ms), for caches.
    pub version: i64,
    pub width: u32,
    pub height: u32,
    pub source_width: u32,
    pub source_height: u32,
    /// The photo was enlarged to reach 4K.
    pub upscaled: bool,
    pub palette: Palette,
    /// The gallery photo it came from ("" for a photo set directly).
    #[serde(default)]
    pub source: String,
}

/// How many photos the gallery keeps.
pub const GALLERY_MAX: usize = 24;

pub struct Wallpapers {
    dir: PathBuf,
}

impl Wallpapers {
    pub fn new(data: &Path) -> Wallpapers {
        Wallpapers { dir: data.join("wallpaper") }
    }

    fn check(slot: &str) -> Result<(), String> {
        if SLOTS.contains(&slot) {
            Ok(())
        } else {
            Err("Unknown wallpaper".into())
        }
    }

    /// The saved picture: the 4K one, or the lighter phone copy.
    pub fn file(&self, slot: &str, phone: bool) -> Option<PathBuf> {
        Self::check(slot).ok()?;
        let p = self.dir.join(if phone { format!("{slot}-phone.jpg") } else { format!("{slot}.jpg") });
        p.is_file().then_some(p)
    }

    pub fn info(&self, slot: &str) -> Option<WallInfo> {
        Self::check(slot).ok()?;
        let b = std::fs::read(self.dir.join(format!("{slot}.json"))).ok()?;
        let info: WallInfo = serde_json::from_slice(&b).ok()?;
        self.file(slot, true).map(|_| info)
    }

    pub fn clear(&self, slot: &str) -> Result<(), String> {
        Self::check(slot)?;
        for f in [format!("{slot}.jpg"), format!("{slot}-phone.jpg"), format!("{slot}.json")] {
            let _ = std::fs::remove_file(self.dir.join(f));
        }
        Ok(())
    }

    pub fn set_file(&self, slot: &str, src: &Path) -> Result<WallInfo, String> {
        let meta = std::fs::metadata(src).map_err(|e| format!("Couldn't read that file: {e}"))?;
        if meta.len() as usize > MAX_BYTES {
            return Err("That picture is over 40 MB; pick a smaller one".into());
        }
        let bytes = std::fs::read(src).map_err(|e| format!("Couldn't read that file: {e}"))?;
        self.set_bytes(slot, &bytes)
    }

    /// Use these picture bytes as the wallpaper for `slot`.
    pub fn set_bytes(&self, slot: &str, bytes: &[u8]) -> Result<WallInfo, String> {
        Self::check(slot)?;
        self.write(&self.dir, slot, slot, "", bytes)
    }

    fn gallery_dir(&self) -> PathBuf {
        self.dir.join("gallery")
    }

    fn gallery_id(id: &str) -> Result<(), String> {
        if id.len() > 1 && id.len() < 24 && id.starts_with('g') && id[1..].bytes().all(|b| b.is_ascii_digit()) {
            Ok(())
        } else {
            Err("Unknown photo".into())
        }
    }

    /// Your photos, newest first.
    pub fn gallery(&self) -> Vec<WallInfo> {
        let mut v: Vec<WallInfo> = std::fs::read_dir(self.gallery_dir())
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok())
            .collect();
        v.sort_by_key(|w| std::cmp::Reverse(w.version));
        v
    }

    /// A gallery photo's picture (the phone-sized one is its thumbnail).
    pub fn gallery_file(&self, id: &str, phone: bool) -> Option<PathBuf> {
        Self::gallery_id(id).ok()?;
        let p = self.gallery_dir().join(if phone { format!("{id}-phone.jpg") } else { format!("{id}.jpg") });
        p.is_file().then_some(p)
    }

    /// Add a photo to the gallery (prepared for 4K like any wallpaper).
    pub fn add(&self, src: &Path) -> Result<WallInfo, String> {
        if self.gallery().len() >= GALLERY_MAX {
            return Err(format!("The gallery holds {GALLERY_MAX} photos; remove one first"));
        }
        let meta = std::fs::metadata(src).map_err(|e| format!("Couldn't read that file: {e}"))?;
        if meta.len() as usize > MAX_BYTES {
            return Err("That picture is over 40 MB; pick a smaller one".into());
        }
        let bytes = std::fs::read(src).map_err(|e| format!("Couldn't read that file: {e}"))?;
        let mut id = chrono::Utc::now().timestamp_millis();
        while self.gallery_dir().join(format!("g{id}.json")).exists() {
            id += 1;
        }
        let id = format!("g{id}");
        self.write(&self.gallery_dir(), &id, "pc", &id, &bytes)
    }

    /// Make a gallery photo the PC's wallpaper.
    pub fn select(&self, id: &str) -> Result<WallInfo, String> {
        Self::gallery_id(id)?;
        let g = self.gallery_dir();
        let mut info: WallInfo = serde_json::from_slice(&std::fs::read(g.join(format!("{id}.json"))).map_err(|_| "That photo isn't in the gallery any more".to_string())?).map_err(|e| e.to_string())?;
        for (from, to) in [(format!("{id}.jpg"), "pc.jpg"), (format!("{id}-phone.jpg"), "pc-phone.jpg")] {
            let b = std::fs::read(g.join(from)).map_err(|e| format!("Couldn't read the photo: {e}"))?;
            crate::settings::write_atomic(&self.dir.join(to), &b).map_err(|e| e.to_string())?;
        }
        info.slot = "pc".into();
        info.source = id.into();
        info.version = chrono::Utc::now().timestamp_millis();
        crate::settings::write_atomic(&self.dir.join("pc.json"), &serde_json::to_vec_pretty(&info).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        Ok(info)
    }

    pub fn remove(&self, id: &str) -> Result<(), String> {
        Self::gallery_id(id)?;
        for f in [format!("{id}.jpg"), format!("{id}-phone.jpg"), format!("{id}.json")] {
            let _ = std::fs::remove_file(self.gallery_dir().join(f));
        }
        Ok(())
    }

    /// Prepare `bytes` and save them as `name`.* in `dir`.
    fn write(&self, dir: &Path, name: &str, slot: &str, source: &str, bytes: &[u8]) -> Result<WallInfo, String> {
        if bytes.len() > MAX_BYTES {
            return Err("That picture is over 40 MB; pick a smaller one".into());
        }
        if !crate::theme_art::looks_like_image(&bytes[..bytes.len().min(16)]) {
            return Err("That file isn't a picture (PNG, JPG, WebP or GIF)".into());
        }
        let img = decode(bytes)?;
        let (sw, sh) = (img.width(), img.height());
        if sw < 16 || sh < 16 {
            return Err("That picture is too small".into());
        }
        let rgb = img.to_rgb8();
        drop(img);
        let palette = palette(&rgb);
        // The PC copy covers 3840 × 2160; the phone copy covers 1440 × 3120.
        let (big, upscaled) = fit(&rgb, (3840, 2160), 6144);
        let (small, _) = fit(&rgb, (1440, 3120), 3600);
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        crate::settings::write_atomic(&dir.join(format!("{name}.jpg")), &jpeg(&big, 92)?).map_err(|e| e.to_string())?;
        crate::settings::write_atomic(&dir.join(format!("{name}-phone.jpg")), &jpeg(&small, 88)?).map_err(|e| e.to_string())?;
        let info = WallInfo { slot: slot.into(), version: chrono::Utc::now().timestamp_millis(), width: big.width(), height: big.height(), source_width: sw, source_height: sh, upscaled, palette, source: source.into() };
        crate::settings::write_atomic(&dir.join(format!("{name}.json")), &serde_json::to_vec_pretty(&info).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        Ok(info)
    }
}

/// Decode, turned the way the camera meant (EXIF orientation).
fn decode(bytes: &[u8]) -> Result<DynamicImage, String> {
    let reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format().map_err(|e| format!("Couldn't read that picture: {e}"))?;
    let mut dec = reader.into_decoder().map_err(|e| format!("Couldn't read that picture: {e}"))?;
    let (w, h) = dec.dimensions();
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err("That picture is too big (over 120 megapixels)".into());
    }
    let orient = dec.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let mut img = DynamicImage::from_decoder(dec).map_err(|e| format!("Couldn't read that picture: {e}"))?;
    img.apply_orientation(orient);
    Ok(img)
}

/// The picture scaled so it covers `target` (w, h), at most 4× larger and
/// with its long side at most `max_long`. Enlarged pictures are sharpened.
pub fn fit(img: &RgbImage, target: (u32, u32), max_long: u32) -> (RgbImage, bool) {
    let (w, h) = (img.width() as f64, img.height() as f64);
    let mut s = (target.0 as f64 / w).max(target.1 as f64 / h).min(4.0);
    let long = w.max(h) * s;
    if long > max_long as f64 {
        s *= max_long as f64 / long;
    }
    let (nw, nh) = (((w * s).round() as u32).max(1), ((h * s).round() as u32).max(1));
    if (nw, nh) == (img.width(), img.height()) {
        return (img.clone(), false);
    }
    let out = image::imageops::resize(img, nw, nh, image::imageops::FilterType::Lanczos3);
    if s > 1.05 {
        // A light unsharp mask, stronger the more it was enlarged.
        let sigma = (0.6 + (s as f32 - 1.0) * 0.25).min(1.4);
        (image::imageops::unsharpen(&out, sigma, 2), true)
    } else {
        (out, false)
    }
}

fn jpeg(img: &RgbImage, quality: u8) -> Result<Vec<u8>, String> {
    let (w, h) = (u16::try_from(img.width()).map_err(|_| "too wide")?, u16::try_from(img.height()).map_err(|_| "too tall")?);
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, quality).encode(img.as_raw(), w, h, jpeg_encoder::ColorType::Rgb).map_err(|e| format!("Couldn't save the picture: {e}"))?;
    Ok(out)
}

fn hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if max == 0.0 { 0.0 } else { d / max }, max)
}

fn hsl_hex(h: f32, s: f32, l: f32) -> String {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h.rem_euclid(360.0) / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let to = |v: f32| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", to(r), to(g), to(b))
}

/// The photo's colours: the strongest hue as the accent, a second hue, how
/// bright it is overall, and a background tint.
pub fn palette(img: &RgbImage) -> Palette {
    let small = image::imageops::thumbnail(img, 64, 64);
    let mut bins = [(0f32, 0f32, 0f32); 12]; // (weight, hue x, hue y) per 30°
    let (mut lum, mut n) = (0f32, 0f32);
    let (mut hx, mut hy, mut sat_sum) = (0f32, 0f32, 0f32);
    for p in small.pixels() {
        let [r, g, b] = p.0.map(|v| v as f32 / 255.0);
        lum += 0.2126 * r + 0.7152 * g + 0.0722 * b;
        n += 1.0;
        let (h, s, v) = hsv(r, g, b);
        let (x, y) = (h.to_radians().cos(), h.to_radians().sin());
        hx += x * s;
        hy += y * s;
        sat_sum += s;
        if s > 0.18 && v > 0.22 {
            let w = s * s * v;
            let bin = &mut bins[((h / 30.0) as usize).min(11)];
            bin.0 += w;
            bin.1 += x * w;
            bin.2 += y * w;
        }
    }
    let light = n > 0.0 && lum / n > 0.62;
    let hue_of = |b: (f32, f32, f32)| b.2.atan2(b.1).to_degrees().rem_euclid(360.0);
    let mut order: Vec<usize> = (0..12).collect();
    order.sort_by(|a, b| bins[*b].0.total_cmp(&bins[*a].0));
    let total: f32 = bins.iter().map(|b| b.0).sum();
    let l = if light { 0.42 } else { 0.66 };
    let (accent, accent2) = if total <= 0.5 {
        // A black-and-white (or very muted) photo: calm, cool accents.
        (hsl_hex(230.0, 0.55, l), hsl_hex(200.0, 0.4, l))
    } else {
        let h1 = hue_of(bins[order[0]]);
        let second = order.iter().skip(1).find(|i| {
            let d = (**i as i32 - order[0] as i32).rem_euclid(12);
            d.min(12 - d) >= 2 && bins[**i].0 > total * 0.06
        });
        let h2 = second.map(|i| hue_of(bins[*i])).unwrap_or((h1 + 40.0) % 360.0);
        (hsl_hex(h1, 0.72, l), hsl_hex(h2, 0.62, l))
    };
    let mean_hue = hy.atan2(hx).to_degrees().rem_euclid(360.0);
    let mean_sat = if n > 0.0 { (sat_sum / n).min(0.5) } else { 0.0 };
    let tint = if light { hsl_hex(mean_hue, mean_sat * 0.6, 0.95) } else { hsl_hex(mean_hue, mean_sat * 0.8, 0.09) };
    Palette { accent, accent2, light, tint }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
        let img = RgbImage::from_fn(w, h, |x, y| image::Rgb(f(x, y)));
        let mut out = Vec::new();
        DynamicImage::ImageRgb8(img).write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    }

    fn hue(hex: &str) -> f32 {
        let n = u32::from_str_radix(&hex[1..], 16).unwrap();
        let [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map(|v| v as f32 / 255.0);
        hsv(r, g, b).0
    }

    #[test]
    fn small_photo_is_enlarged_to_cover_4k_with_matching_colours() {
        let t = tempfile::tempdir().unwrap();
        let w = Wallpapers::new(t.path());
        // A mostly red-orange picture with some blue.
        let bytes = png(320, 180, |x, _| if x < 240 { [220, 70, 30] } else { [30, 60, 200] });
        let info = w.set_bytes("pc", &bytes).unwrap();
        assert!(info.upscaled);
        assert!(info.width >= 1280 && info.height >= 720, "at most 4× bigger: {}×{}", info.width, info.height);
        let h = hue(&info.palette.accent);
        assert!(!(40.0..=340.0).contains(&h), "accent follows the red-orange: {h}");
        let h2 = hue(&info.palette.accent2);
        assert!((180.0..=260.0).contains(&h2), "second colour is the blue: {h2}");
        assert!(!info.palette.light);
        assert!(w.file("pc", false).is_some() && w.file("pc", true).is_some());
        assert_eq!(w.info("pc").unwrap(), info);
        w.clear("pc").unwrap();
        assert!(w.info("pc").is_none() && w.file("pc", false).is_none());
    }

    #[test]
    fn big_photo_comes_down_to_4k_and_bright_photo_is_light() {
        let img = RgbImage::from_pixel(5000, 3000, image::Rgb([235, 238, 245]));
        let (out, up) = fit(&img, (3840, 2160), 6144);
        assert!(!up);
        assert_eq!((out.width(), out.height()), (3840, 2304));
        assert!(palette(&img).light);
        let (phone, _) = fit(&img, (1440, 3120), 3600);
        assert!(phone.height() >= 3120 || phone.width() == 3600, "{}×{}", phone.width(), phone.height());
    }

    #[test]
    fn gallery_adds_selects_and_removes() {
        let t = tempfile::tempdir().unwrap();
        let w = Wallpapers::new(t.path());
        let a = t.path().join("a.png");
        std::fs::write(&a, png(64, 36, |_, _| [200, 60, 40])).unwrap();
        let b = t.path().join("b.png");
        std::fs::write(&b, png(64, 36, |_, _| [40, 60, 200])).unwrap();
        let ia = w.add(&a).unwrap();
        let ib = w.add(&b).unwrap();
        assert_ne!(ia.source, ib.source);
        assert_eq!(w.gallery().len(), 2);
        assert!(w.info("pc").is_none(), "adding doesn't change the wallpaper");
        let now = w.select(&ia.source).unwrap();
        assert_eq!((now.slot.as_str(), now.source.as_str()), ("pc", ia.source.as_str()));
        assert_eq!(w.info("pc").unwrap().palette, ia.palette);
        assert!(w.gallery_file(&ib.source, true).is_some());
        w.remove(&ib.source).unwrap();
        assert_eq!(w.gallery().len(), 1);
        assert!(w.select(&ib.source).is_err() && w.select("../pc").is_err() && w.gallery_file("g1/../x", false).is_none());
    }

    #[test]
    fn refuses_what_isnt_a_picture() {
        let t = tempfile::tempdir().unwrap();
        let w = Wallpapers::new(t.path());
        assert!(w.set_bytes("pc", b"hello, not a picture").is_err());
        assert!(w.set_bytes("desk", &png(32, 32, |_, _| [0, 0, 0])).is_err());
        // Grey picture: calm default accents, not an error.
        let info = w.set_bytes("phone", &png(64, 64, |_, _| [90, 90, 90])).unwrap();
        assert!(info.palette.accent.starts_with('#'));
    }
}
