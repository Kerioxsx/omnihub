//! Image tools: take out the location and camera details before sharing,
//! shrink, compress, and turn iPhone HEIC photos into JPG. Everything runs on
//! this PC, and the original is never changed: each result is a new file next
//! to it.

use std::path::Path;

use image::{DynamicImage, ImageDecoder, ImageReader};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Job {
    /// Take out location, camera, date and comments (the picture is untouched).
    Clean,
    /// Make the longest side at most this many pixels.
    #[serde(rename_all = "camelCase")]
    Resize { max_side: u32 },
    /// Smaller file at this JPG quality (keeps PNGs with see-through parts as PNG).
    Compress { quality: u8 },
    /// HEIC, PNG, WebP... to JPG.
    ToJpg,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Done {
    pub source: String,
    pub output: String,
    pub before: u64,
    pub after: u64,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    pub path: String,
    pub bytes: u64,
    pub width: u32,
    pub height: u32,
    /// The photo says where it was taken.
    pub has_location: bool,
    /// "2024-05-01 13:22:10", from the camera.
    pub taken: Option<String>,
    pub camera: Option<String>,
}

/// What the file's EXIF block says.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Exif {
    pub orientation: u16,
    pub gps: bool,
    pub taken: Option<String>,
    pub make: Option<String>,
    pub model: Option<String>,
}

pub const EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "heic", "heif", "avif", "bmp", "gif", "tif", "tiff"];

fn ext_of(p: &Path) -> String {
    p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

pub fn is_image(p: &Path) -> bool {
    EXTENSIONS.contains(&ext_of(p).as_str())
}

pub fn inspect(path: &Path) -> Result<Info, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
    let exif = exif_block(&bytes).and_then(parse_exif).unwrap_or_default();
    let (width, height) = image::ImageReader::new(std::io::Cursor::new(&bytes)).with_guessed_format().ok().and_then(|r| r.into_dimensions().ok()).unwrap_or((0, 0));
    let camera = match (&exif.make, &exif.model) {
        (Some(m), Some(n)) if n.starts_with(m.as_str()) => Some(n.clone()),
        (Some(m), Some(n)) => Some(format!("{m} {n}")),
        (a, b) => a.clone().or(b.clone()),
    };
    Ok(Info { path: path.display().to_string(), bytes: bytes.len() as u64, width, height, has_location: exif.gps, taken: exif.taken, camera })
}

/// When the photo was taken, from its EXIF (for sorting a camera's card by day).
pub fn taken(path: &Path) -> Option<String> {
    let mut head = vec![0u8; 256 * 1024];
    let n = std::io::Read::read(&mut std::fs::File::open(path).ok()?, &mut head).ok()?;
    head.truncate(n);
    exif_block(&head).and_then(parse_exif)?.taken
}

pub fn run(path: &Path, job: Job) -> Result<Done, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
    let ext = ext_of(path);
    let dir = path.parent().unwrap_or(Path::new("."));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
    let (out, note): (Vec<u8>, Option<String>);
    let out_ext: &str;
    match job {
        Job::Clean if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp") => {
            let cleaned = strip(&bytes).ok_or("This file isn't a picture OmniHub can read.")?;
            note = (cleaned.len() == bytes.len()).then(|| "There was nothing to take out.".to_string());
            out = cleaned;
            out_ext = if ext == "jpeg" { "jpg" } else { ext.as_str() };
        }
        // Other formats: a fresh JPG has nothing in it.
        Job::Clean | Job::ToJpg => {
            let img = decode(&bytes)?;
            out = jpeg(&img, 92)?;
            note = None;
            out_ext = "jpg";
        }
        Job::Resize { max_side } => {
            let img = decode(&bytes)?;
            let max_side = max_side.clamp(16, 16384);
            let img = if img.width().max(img.height()) > max_side { resize(&img, max_side)? } else { img };
            let keep = matches!(ext.as_str(), "png" | "webp" | "gif" | "bmp") && has_alpha(&img);
            (out, out_ext) = if keep || ext == "png" { (png(&img)?, "png") } else { (jpeg(&img, 90)?, "jpg") };
            note = None;
        }
        Job::Compress { quality } => {
            let img = decode(&bytes)?;
            let alpha = has_alpha(&img);
            let (o, e) = if alpha { (png(&img)?, "png") } else { (jpeg(&img, quality.clamp(20, 95))?, "jpg") };
            if o.len() as u64 >= bytes.len() as u64 && !matches!(ext.as_str(), "heic" | "heif" | "avif" | "tif" | "tiff" | "bmp") {
                return Ok(Done { source: path.display().to_string(), output: path.display().to_string(), before: bytes.len() as u64, after: bytes.len() as u64, note: Some("Already as small as it gets; left as it was.".into()) });
            }
            (out, out_ext) = (o, e);
            note = alpha.then(|| "Kept as PNG so the see-through parts stay.".to_string());
        }
    }
    let suffix = match job {
        Job::Clean => " (no details)".to_string(),
        Job::Resize { max_side } => format!(" ({max_side}px)"),
        Job::Compress { .. } => " (small)".to_string(),
        Job::ToJpg => String::new(),
    };
    let dest = crate::remote::transfer::unique_path(dir, &format!("{stem}{suffix}.{out_ext}"));
    std::fs::write(&dest, &out).map_err(|e| format!("Couldn't save {}: {e}", dest.display()))?;
    Ok(Done { source: path.display().to_string(), output: dest.display().to_string(), before: bytes.len() as u64, after: out.len() as u64, note })
}

/// The picture, turned the right way up. HEIC and the like go through the
/// decoders built into Windows.
pub fn decode(bytes: &[u8]) -> Result<DynamicImage, String> {
    let ours = (|| -> image::ImageResult<DynamicImage> {
        let mut dec = ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?.into_decoder()?;
        let o = dec.orientation()?;
        let mut img = DynamicImage::from_decoder(dec)?;
        img.apply_orientation(o);
        Ok(img)
    })();
    match ours {
        Ok(img) => Ok(img),
        #[cfg(windows)]
        Err(_) => {
            let owned = bytes.to_vec();
            let (w, h, px) = std::thread::spawn(move || win::decode(&owned)).join().map_err(|_| "Reading the picture crashed.".to_string())??;
            image::RgbaImage::from_raw(w, h, px).map(DynamicImage::ImageRgba8).ok_or_else(|| "Windows returned a broken picture.".to_string())
        }
        #[cfg(not(windows))]
        Err(e) => Err(format!("Couldn't read the picture: {e}")),
    }
}

fn has_alpha(img: &DynamicImage) -> bool {
    img.color().has_alpha() && img.to_rgba8().pixels().any(|p| p[3] < 255)
}

fn resize(img: &DynamicImage, max_side: u32) -> Result<DynamicImage, String> {
    use fast_image_resize::images::Image;
    use fast_image_resize::{PixelType, ResizeAlg, ResizeOptions, Resizer};
    let (w, h) = (img.width(), img.height());
    let k = max_side as f64 / w.max(h) as f64;
    let (tw, th) = (((w as f64 * k).round() as u32).max(1), ((h as f64 * k).round() as u32).max(1));
    let src = Image::from_vec_u8(w, h, img.to_rgba8().into_raw(), PixelType::U8x4).map_err(|e| e.to_string())?;
    let mut dst = Image::new(tw, th, PixelType::U8x4);
    Resizer::new().resize(&src, &mut dst, &ResizeOptions::new().resize_alg(ResizeAlg::Convolution(fast_image_resize::FilterType::Lanczos3))).map_err(|e| e.to_string())?;
    image::RgbaImage::from_raw(tw, th, dst.into_vec()).map(DynamicImage::ImageRgba8).ok_or_else(|| "Resizing failed.".into())
}

/// JPG (see-through parts on white).
pub fn jpeg(img: &DynamicImage, quality: u8) -> Result<Vec<u8>, String> {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    if w > 65_535 || h > 65_535 {
        return Err("The picture is too big for a JPG.".into());
    }
    let mut rgb = Vec::with_capacity((w * h * 3) as usize);
    for p in rgba.pixels() {
        let a = p[3] as u32;
        for c in &p.0[..3] {
            rgb.push(((*c as u32 * a + 255 * (255 - a)) / 255) as u8);
        }
    }
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, quality).encode(&rgb, w as u16, h as u16, jpeg_encoder::ColorType::Rgb).map_err(|e| e.to_string())?;
    Ok(out)
}

fn png(img: &DynamicImage) -> Result<Vec<u8>, String> {
    use image::codecs::png::{CompressionType, FilterType, PngEncoder};
    let mut out = Vec::new();
    let img = if has_alpha(img) { DynamicImage::ImageRgba8(img.to_rgba8()) } else { DynamicImage::ImageRgb8(img.to_rgb8()) };
    img.write_with_encoder(PngEncoder::new_with_quality(&mut out, CompressionType::Best, FilterType::Adaptive)).map_err(|e| e.to_string())?;
    Ok(out)
}

// ---- Metadata --------------------------------------------------------------

/// The same picture without EXIF, XMP, IPTC and comments (JPG, PNG, WebP).
/// A photo shot sideways stays the right way up (only the turn is kept).
pub fn strip(bytes: &[u8]) -> Option<Vec<u8>> {
    if bytes.starts_with(&[0xFF, 0xD8]) {
        strip_jpeg(bytes)
    } else if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        strip_png(bytes)
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        strip_webp(bytes)
    } else {
        None
    }
}

fn strip_jpeg(b: &[u8]) -> Option<Vec<u8>> {
    let orientation = exif_block(b).and_then(parse_exif).map_or(1, |e| e.orientation);
    let mut out = Vec::with_capacity(b.len());
    out.extend_from_slice(&[0xFF, 0xD8]);
    let mut wrote_turn = orientation <= 1 || orientation > 8;
    let mut i = 2;
    loop {
        // A marker: 0xFF (any fill bytes), then its code.
        if *b.get(i)? != 0xFF {
            return None;
        }
        while b.get(i) == Some(&0xFF) {
            i += 1;
        }
        let m = *b.get(i)?;
        i += 1;
        match m {
            0xD9 => {
                // End of the picture; anything after (previews, gain maps) carries its own details.
                out.extend_from_slice(&[0xFF, 0xD9]);
                return Some(out);
            }
            0xD0..=0xD7 | 0x01 => out.extend_from_slice(&[0xFF, m]),
            _ => {
                let len = u16::from_be_bytes([*b.get(i)?, *b.get(i + 1)?]) as usize;
                let seg = b.get(i..i + len)?;
                let keep = match m {
                    0xE0 | 0xEE => true,                     // JFIF, Adobe (colour transform)
                    0xE2 => seg.get(2..14) == Some(b"ICC_PROFILE\0"), // colour profile
                    0xE1 | 0xE3..=0xED | 0xEF | 0xFE => false, // EXIF/XMP, IPTC, vendor blocks, comments
                    _ => true,
                };
                if !wrote_turn && m != 0xE0 {
                    out.extend_from_slice(&orientation_segment(orientation));
                    wrote_turn = true;
                }
                if keep {
                    out.extend_from_slice(&[0xFF, m]);
                    out.extend_from_slice(seg);
                }
                i += len;
                if m == 0xDA {
                    // The picture data runs to the next real marker.
                    let start = i;
                    while i + 1 < b.len() && !(b[i] == 0xFF && b[i + 1] != 0 && !(0xD0..=0xD7).contains(&b[i + 1])) {
                        i += 1;
                    }
                    if i + 1 >= b.len() {
                        return None;
                    }
                    out.extend_from_slice(&b[start..i]);
                }
            }
        }
    }
}

/// A minimal EXIF block holding only the orientation.
fn orientation_segment(o: u16) -> Vec<u8> {
    let mut s = vec![0xFF, 0xE1, 0, 34];
    s.extend_from_slice(b"Exif\0\0MM\0\x2A\0\0\0\x08\0\x01\x01\x12\0\x03\0\0\0\x01");
    s.extend_from_slice(&o.to_be_bytes());
    s.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    s
}

fn strip_png(b: &[u8]) -> Option<Vec<u8>> {
    let mut out = b[..8].to_vec();
    let mut i = 8;
    while i < b.len() {
        let len = u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?) as usize;
        let kind = b.get(i + 4..i + 8)?;
        let end = i.checked_add(12 + len)?;
        let chunk = b.get(i..end)?;
        if !matches!(kind, b"tEXt" | b"zTXt" | b"iTXt" | b"eXIf" | b"tIME") {
            out.extend_from_slice(chunk);
        }
        i = end;
        if kind == b"IEND" {
            break;
        }
    }
    Some(out)
}

fn strip_webp(b: &[u8]) -> Option<Vec<u8>> {
    let mut body = b"WEBP".to_vec();
    let mut i = 12;
    while i + 8 <= b.len() {
        let kind = &b[i..i + 4];
        let len = u32::from_le_bytes(b[i + 4..i + 8].try_into().ok()?) as usize;
        let end = i.checked_add(8 + len + (len & 1))?.min(b.len());
        let chunk = b.get(i..end)?;
        match kind {
            b"EXIF" | b"XMP " => {}
            b"VP8X" => {
                let mut c = chunk.to_vec();
                if let Some(flags) = c.get_mut(8) {
                    *flags &= !0x0C;
                }
                body.extend_from_slice(&c);
            }
            _ => body.extend_from_slice(chunk),
        }
        i = end;
    }
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&body);
    Some(out)
}

/// The TIFF-shaped EXIF data inside a JPG, PNG or WebP.
pub fn exif_block(b: &[u8]) -> Option<&[u8]> {
    if b.starts_with(&[0xFF, 0xD8]) {
        let mut i = 2;
        while i + 4 <= b.len() && b[i] == 0xFF {
            let m = b[i + 1];
            if m == 0xDA || m == 0xD9 {
                return None;
            }
            let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
            let seg = b.get(i + 4..i + 2 + len)?;
            if m == 0xE1 && seg.starts_with(b"Exif\0\0") {
                return Some(&seg[6..]);
            }
            i += 2 + len;
        }
        None
    } else if b.starts_with(b"\x89PNG") {
        let mut i = 8;
        while i + 8 <= b.len() {
            let len = u32::from_be_bytes(b[i..i + 4].try_into().ok()?) as usize;
            if &b[i + 4..i + 8] == b"eXIf" {
                return b.get(i + 8..i + 8 + len);
            }
            i = i.checked_add(12 + len)?;
        }
        None
    } else if b.len() > 12 && &b[..4] == b"RIFF" {
        let mut i = 12;
        while i + 8 <= b.len() {
            let len = u32::from_le_bytes(b[i + 4..i + 8].try_into().ok()?) as usize;
            if &b[i..i + 4] == b"EXIF" {
                let d = b.get(i + 8..i + 8 + len)?;
                return Some(d.strip_prefix(b"Exif\0\0").unwrap_or(d));
            }
            i = i.checked_add(8 + len + (len & 1))?;
        }
        None
    } else {
        None
    }
}

struct Tiff<'a> {
    b: &'a [u8],
    le: bool,
}

impl Tiff<'_> {
    fn u16(&self, o: usize) -> Option<u16> {
        let s: [u8; 2] = self.b.get(o..o + 2)?.try_into().ok()?;
        Some(if self.le { u16::from_le_bytes(s) } else { u16::from_be_bytes(s) })
    }
    fn u32(&self, o: usize) -> Option<u32> {
        let s: [u8; 4] = self.b.get(o..o + 4)?.try_into().ok()?;
        Some(if self.le { u32::from_le_bytes(s) } else { u32::from_be_bytes(s) })
    }
    /// (tag, type, count, where the value is) for each entry of the IFD at `at`.
    fn entries(&self, at: usize) -> Vec<(u16, u16, u32, usize)> {
        let n = self.u16(at).unwrap_or(0).min(512) as usize;
        (0..n)
            .filter_map(|k| {
                let e = at + 2 + k * 12;
                let (tag, typ, count) = (self.u16(e)?, self.u16(e + 2)?, self.u32(e + 4)?);
                let size = [0, 1, 1, 2, 4, 8, 1, 1, 2, 4, 8, 4, 8].get(typ as usize).copied().unwrap_or(1) * count as usize;
                let at = if size <= 4 { e + 8 } else { self.u32(e + 8)? as usize };
                Some((tag, typ, count, at))
            })
            .collect()
    }
    fn text(&self, at: usize, count: u32) -> Option<String> {
        let s = String::from_utf8_lossy(self.b.get(at..at + count as usize)?).trim_end_matches(['\0', ' ']).trim().to_string();
        (!s.is_empty()).then_some(s)
    }
}

pub fn parse_exif(b: &[u8]) -> Option<Exif> {
    let le = match b.get(..4)? {
        b"II*\0" => true,
        b"MM\0*" => false,
        _ => return None,
    };
    let t = Tiff { b, le };
    let ifd0 = t.u32(4)? as usize;
    let mut e = Exif { orientation: 1, ..Default::default() };
    let mut date = None;
    for (tag, typ, count, at) in t.entries(ifd0) {
        match tag {
            0x0112 if typ == 3 => e.orientation = t.u16(at).unwrap_or(1),
            0x010F => e.make = t.text(at, count),
            0x0110 => e.model = t.text(at, count),
            0x0132 => date = t.text(at, count),
            0x8825 => e.gps = t.u32(at).is_some_and(|gps| t.entries(gps as usize).iter().any(|x| matches!(x.0, 2 | 4))),
            0x8769 => {
                if let Some(sub) = t.u32(at) {
                    e.taken = t.entries(sub as usize).iter().find(|x| x.0 == 0x9003).and_then(|x| t.text(x.3, x.2));
                }
            }
            _ => {}
        }
    }
    // "2024:05:01 13:22:10" → "2024-05-01 13:22:10"
    e.taken = e.taken.or(date).filter(|d| d.len() >= 10 && !d.starts_with("0000")).map(|d| d.replacen(':', "-", 2));
    Some(e)
}

#[cfg(windows)]
mod win {
    use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapDecoder, BitmapPixelFormat, BitmapTransform, ColorManagementMode, ExifOrientationMode};
    use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    fn err(e: windows::core::Error) -> String {
        e.message().to_string()
    }

    /// RGBA pixels, turned the right way up.
    pub fn decode(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
        // SAFETY: initialises WinRT for this (new) thread.
        let _ = unsafe { RoInitialize(RO_INIT_MULTITHREADED) };
        let stream = InMemoryRandomAccessStream::new().map_err(err)?;
        let writer = DataWriter::CreateDataWriter(&stream).map_err(err)?;
        writer.WriteBytes(bytes).map_err(err)?;
        writer.StoreAsync().map_err(err)?.join().map_err(err)?;
        writer.FlushAsync().map_err(err)?.join().map_err(err)?;
        writer.DetachStream().map_err(err)?;
        stream.Seek(0).map_err(err)?;
        let decoder = BitmapDecoder::CreateAsync(&stream).map_err(err)?.join().map_err(|_| {
            "Windows can't read this picture. For iPhone HEIC photos, install the free “HEIF Image Extensions” and “HEVC Video Extensions” from the Microsoft Store.".to_string()
        })?;
        let transform = BitmapTransform::new().map_err(err)?;
        let px = decoder
            .GetPixelDataTransformedAsync(BitmapPixelFormat::Rgba8, BitmapAlphaMode::Straight, &transform, ExifOrientationMode::RespectExifOrientation, ColorManagementMode::ColorManageToSRgb)
            .map_err(err)?
            .join()
            .map_err(err)?;
        let data = px.DetachPixelData().map_err(err)?;
        let (w, h) = (decoder.OrientedPixelWidth().map_err(err)?, decoder.OrientedPixelHeight().map_err(err)?);
        if data.len() != (w as usize) * (h as usize) * 4 {
            return Err("Windows returned a broken picture.".into());
        }
        Ok((w, h, data.to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A JPG with EXIF: orientation 6 (shot sideways), a camera, a date and GPS.
    fn photo(w: u32, h: u32) -> Vec<u8> {
        let img = DynamicImage::ImageRgb8(image::RgbImage::from_fn(w, h, |x, y| image::Rgb([(x * 7) as u8, (y * 5) as u8, 90])));
        let plain = jpeg(&img, 90).unwrap();
        // TIFF (big-endian): IFD0 at 8 with Make, Orientation, ExifIFD, GPS.
        let mut t = b"MM\0\x2A\0\0\0\x08".to_vec();
        let ifd0_len = 2 + 4 * 12 + 4;
        let make_at = 8 + ifd0_len;
        let exif_at = make_at + 8;
        let gps_at = exif_at + 2 + 12 + 4 + 20;
        let entry = |t: &mut Vec<u8>, tag: u16, typ: u16, count: u32, val: u32| {
            t.extend_from_slice(&tag.to_be_bytes());
            t.extend_from_slice(&typ.to_be_bytes());
            t.extend_from_slice(&count.to_be_bytes());
            t.extend_from_slice(&val.to_be_bytes());
        };
        t.extend_from_slice(&4u16.to_be_bytes());
        entry(&mut t, 0x010F, 2, 6, make_at as u32);
        entry(&mut t, 0x0112, 3, 1, 6 << 16);
        entry(&mut t, 0x8769, 4, 1, exif_at as u32);
        entry(&mut t, 0x8825, 4, 1, gps_at as u32);
        t.extend_from_slice(&[0; 4]);
        t.extend_from_slice(b"Pixel\0\0\0");
        t.extend_from_slice(&1u16.to_be_bytes());
        entry(&mut t, 0x9003, 2, 20, (exif_at + 2 + 12 + 4) as u32);
        t.extend_from_slice(&[0; 4]);
        t.extend_from_slice(b"2024:05:01 13:22:10\0");
        assert_eq!(t.len(), gps_at);
        t.extend_from_slice(&1u16.to_be_bytes());
        entry(&mut t, 0x0002, 5, 3, 0);
        t.extend_from_slice(&[0; 4]);
        let mut seg = b"Exif\0\0".to_vec();
        seg.extend_from_slice(&t);
        let mut out = plain[..2].to_vec();
        out.extend_from_slice(&[0xFF, 0xE1]);
        out.extend_from_slice(&((seg.len() + 2) as u16).to_be_bytes());
        out.extend_from_slice(&seg);
        out.extend_from_slice(b"\xFF\xFE\0\x07hello");
        out.extend_from_slice(&plain[2..]);
        out
    }

    #[test]
    fn reads_and_strips_jpeg_details() {
        let p = photo(40, 20);
        let e = parse_exif(exif_block(&p).unwrap()).unwrap();
        assert_eq!(e, Exif { orientation: 6, gps: true, taken: Some("2024-05-01 13:22:10".into()), make: Some("Pixel".into()), model: None });
        let clean = strip(&p).unwrap();
        assert!(clean.len() < p.len());
        assert!(!clean.windows(5).any(|w| w == b"Pixel") && !clean.windows(5).any(|w| w == b"hello"));
        // Only the turn is left, so it still shows upright.
        let e2 = parse_exif(exif_block(&clean).unwrap()).unwrap();
        assert_eq!(e2, Exif { orientation: 6, ..Default::default() });
        let img = decode(&clean).unwrap();
        assert_eq!((img.width(), img.height()), (20, 40));
        // Junk after the end (a gain map with its own EXIF) goes too.
        let mut tail = p.clone();
        tail.extend_from_slice(&p);
        assert_eq!(strip(&tail).unwrap(), clean);
        assert!(strip(b"\xFF\xD8\xFF\xE1\xFF\xFF").is_none());
    }

    #[test]
    fn strips_png_and_webp() {
        let img = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 128])));
        let p = png(&img).unwrap();
        // Insert a tEXt chunk after IHDR.
        let ihdr_end = 8 + 12 + 13;
        let mut with = p[..ihdr_end].to_vec();
        let data = b"Comment\0secret";
        with.extend_from_slice(&(data.len() as u32).to_be_bytes());
        with.extend_from_slice(b"tEXt");
        with.extend_from_slice(data);
        with.extend_from_slice(&crc32fast::hash(&[b"tEXt".as_slice(), data].concat()).to_be_bytes());
        with.extend_from_slice(&p[ihdr_end..]);
        assert_eq!(strip(&with).unwrap(), p);

        let mut w = Vec::new();
        img.write_with_encoder(image::codecs::webp::WebPEncoder::new_lossless(&mut w)).unwrap();
        // RIFF(WEBP, VP8X with the EXIF flag, VP8L, EXIF).
        let vp8l = &w[12..];
        let mut body = b"WEBP".to_vec();
        body.extend_from_slice(b"VP8X\x0a\0\0\0\x18\0\0\0\x07\0\0\x07\0\0");
        body.extend_from_slice(vp8l);
        body.extend_from_slice(b"EXIF\x05\0\0\0abcde\0");
        let mut webp = b"RIFF".to_vec();
        webp.extend_from_slice(&(body.len() as u32).to_le_bytes());
        webp.extend_from_slice(&body);
        assert_eq!(exif_block(&webp), Some(b"abcde".as_slice()));
        let clean = strip(&webp).unwrap();
        assert!(exif_block(&clean).is_none());
        assert_eq!(clean[20], 0x10); // only the alpha flag left
        assert_eq!(u32::from_le_bytes(clean[4..8].try_into().unwrap()) as usize, clean.len() - 8);
        assert_eq!(decode(&clean).unwrap().width(), 8);
    }

    #[test]
    fn jobs_from_the_page() {
        let j: Job = serde_json::from_str(r#"{"kind":"resize","maxSide":1600}"#).unwrap();
        assert_eq!(j, Job::Resize { max_side: 1600 });
        let v: crate::videotools::Job = serde_json::from_str(r#"{"kind":"compress","size":"small","maxHeight":null,"targetMb":10}"#).unwrap();
        assert_eq!(v, crate::videotools::Job::Compress { size: crate::videotools::Size::Small, max_height: None, target_mb: Some(10.0) });
    }

    #[test]
    fn jobs_write_new_files() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("trip.jpg");
        std::fs::write(&src, photo(400, 200)).unwrap();
        let info = inspect(&src).unwrap();
        assert!(info.has_location);
        assert_eq!(info.camera.as_deref(), Some("Pixel"));

        let d = run(&src, Job::Clean).unwrap();
        assert!(d.output.ends_with("trip (no details).jpg"));
        assert!(!inspect(Path::new(&d.output)).unwrap().has_location);

        let d = run(&src, Job::Resize { max_side: 100 }).unwrap();
        assert!(d.output.ends_with("trip (100px).jpg"));
        let img = decode(&std::fs::read(&d.output).unwrap()).unwrap();
        assert_eq!((img.width(), img.height()), (50, 100)); // turned upright, then shrunk

        let d = run(&src, Job::Compress { quality: 40 }).unwrap();
        assert!(d.after < d.before && d.output.ends_with("trip (small).jpg"));
        let again = run(Path::new(&d.output), Job::Compress { quality: 95 }).unwrap();
        assert!(again.note.unwrap().contains("Already"));

        let pngp = dir.path().join("logo.png");
        std::fs::write(&pngp, png(&DynamicImage::ImageRgb8(image::RgbImage::from_pixel(30, 30, image::Rgb([200, 10, 10])))).unwrap()).unwrap();
        let d = run(&pngp, Job::ToJpg).unwrap();
        assert!(d.output.ends_with("logo.jpg"));
        assert_eq!(std::fs::read(&src).unwrap(), photo(400, 200), "the original is never touched");
        assert_eq!(taken(&src).as_deref(), Some("2024-05-01 13:22:10"));
    }
}
