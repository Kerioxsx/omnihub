//! Sharper cover art. Windows hands apps the player's thumbnail, often
//! 300×300 or smaller, and Aurora draws the cover across the screen. When
//! allowed, OmniHub asks the iTunes Search API (Apple's public catalogue
//! search: no account, no key) for the same song and uses its 1200×1200
//! artwork. Only the title, artist and album are sent. A result is used only
//! when the song matches (title, artist, length) and the picture looks like
//! the player's own thumbnail, so a different song's cover never shows up.
//! Covers are cached on disk, one small file per song.

use std::path::PathBuf;

use serde::Deserialize;

use super::lyrics::clean_query;

/// Edge of the picture asked for.
const SIZE: u32 = 1200;
/// Covers bigger than this are not downloaded.
const MAX_BYTES: u64 = 6 << 20;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Result_ {
    #[serde(default)]
    track_name: String,
    #[serde(default)]
    artist_name: String,
    #[serde(default)]
    collection_name: String,
    #[serde(default)]
    track_time_millis: u64,
    #[serde(default)]
    artwork_url100: String,
}

#[derive(Deserialize)]
struct Response {
    #[serde(default)]
    results: Vec<Result_>,
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The URL of a large cover for this song, if the catalogue has the song.
/// `get(url)` returns the body (`Ok(None)` for "not found").
pub fn find(title: &str, artist: &str, album: &str, duration_ms: u64, get: &dyn Fn(&str) -> Result<Option<String>, String>) -> Result<Option<String>, String> {
    let (t, a) = clean_query(title, artist);
    if t.is_empty() {
        return Ok(None);
    }
    let (nt, na, nal) = (norm(&t), norm(&a), norm(&clean_query(album, "").0));
    let url = format!("https://itunes.apple.com/search?term={}&media=music&entity=song&limit=15", enc(&format!("{a} {t}")));
    let Some(body) = get(&url)? else { return Ok(None) };
    let res: Response = serde_json::from_str(&body).map_err(|e| format!("unexpected catalogue answer: {e}"))?;
    let mut best: Option<(u32, &Result_)> = None;
    for r in &res.results {
        if r.artwork_url100.is_empty() {
            continue;
        }
        let rt = norm(&clean_query(&r.track_name, "").0);
        let ra = norm(&r.artist_name);
        let title_ok = rt == nt || (nt.len() >= 4 && (rt.starts_with(&nt) || nt.starts_with(&rt)) && rt.len().abs_diff(nt.len()) <= 12);
        let artist_ok = na.is_empty() || ra.contains(&na) || na.contains(&ra);
        if !title_ok || !artist_ok {
            continue;
        }
        let length_ok = duration_ms == 0 || r.track_time_millis == 0 || r.track_time_millis.abs_diff(duration_ms) <= 8_000;
        if !length_ok {
            continue;
        }
        let score = u32::from(rt == nt) * 4 + u32::from(!nal.is_empty() && norm(&clean_query(&r.collection_name, "").0) == nal) * 3 + u32::from(r.track_time_millis.abs_diff(duration_ms) <= 2_000) * 2;
        if best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, r));
        }
    }
    Ok(best.map(|(_, r)| r.artwork_url100.replace("100x100bb", &format!("{SIZE}x{SIZE}bb"))))
}

/// How alike two pictures are, 0 (same) to 1: their 16×16 thumbnails compared.
pub fn difference(a: &[u8], b: &[u8]) -> Option<f32> {
    let small = |bytes: &[u8]| -> Option<Vec<f32>> {
        let img = image::load_from_memory(bytes).ok()?.resize_exact(16, 16, image::imageops::FilterType::Triangle).to_rgb8();
        Some(img.pixels().flat_map(|p| p.0.map(|c| c as f32 / 255.0)).collect())
    };
    let (x, y) = (small(a)?, small(b)?);
    Some(x.iter().zip(&y).map(|(p, q)| (p - q).abs()).sum::<f32>() / x.len() as f32)
}

/// The picture's type from its first bytes (only JPEG and PNG are used).
pub fn mime_of(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if bytes.starts_with(b"\x89PNG") {
        Some("image/png")
    } else {
        None
    }
}

/// Download a cover (at most 6 MB).
pub fn download(url: &str) -> Result<Vec<u8>, String> {
    if !url.starts_with("https://") {
        return Err("only https covers are downloaded".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(15))).build().into();
    let mut res = agent
        .get(url)
        .header("User-Agent", concat!("OmniHub/", env!("CARGO_PKG_VERSION"), " (https://github.com/Kerioxsx/omnihub)"))
        .call()
        .map_err(|e| format!("could not download the cover: {e}"))?;
    res.body_mut().with_config().limit(MAX_BYTES).read_to_vec().map_err(|e| e.to_string())
}

/// Large covers kept on disk, so a song is looked up once.
pub struct ArtCache {
    dir: PathBuf,
}

impl ArtCache {
    pub fn new(dir: PathBuf) -> Self {
        ArtCache { dir }
    }

    fn file(&self, key: &str) -> PathBuf {
        use sha2::Digest;
        self.dir.join(format!("{}.bin", hex::encode(&sha2::Sha256::digest(key.as_bytes())[..12])))
    }

    /// `Some(None)`: looked up before, nothing better found.
    pub fn get(&self, key: &str) -> Option<Option<Vec<u8>>> {
        let bytes = std::fs::read(self.file(key)).ok()?;
        Some((!bytes.is_empty()).then_some(bytes))
    }

    pub fn put(&self, key: &str, bytes: Option<&[u8]>) {
        let _ = std::fs::create_dir_all(&self.dir);
        let _ = std::fs::write(self.file(key), bytes.unwrap_or_default());
        // Keep the folder small: the oldest go past 400 covers.
        if let Ok(rd) = std::fs::read_dir(&self.dir) {
            let mut files: Vec<_> = rd.flatten().filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path()))).collect();
            if files.len() > 400 {
                files.sort();
                for (_, p) in files.iter().take(files.len() - 400) {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(color: [u8; 3], size: u32) -> Vec<u8> {
        let img = image::RgbImage::from_fn(size, size, |x, y| if (x / (size / 4) + y / (size / 4)).is_multiple_of(2) { image::Rgb(color) } else { image::Rgb([10, 10, 10]) });
        let mut out = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    }

    #[test]
    fn finds_the_right_song_and_asks_for_a_big_cover() {
        let calls = std::cell::RefCell::new(Vec::new());
        let get = |url: &str| -> Result<Option<String>, String> {
            calls.borrow_mut().push(url.to_string());
            Ok(Some(
                r#"{"results":[
                {"trackName":"Waves (Live)","artistName":"Joey Bada$$","collectionName":"Live","trackTimeMillis":300000,"artworkUrl100":"https://is1.example/live/100x100bb.jpg"},
                {"trackName":"Waves","artistName":"Someone Else","collectionName":"Other","trackTimeMillis":187000,"artworkUrl100":"https://is1.example/other/100x100bb.jpg"},
                {"trackName":"Waves","artistName":"Joey Bada$$","collectionName":"1999","trackTimeMillis":187500,"artworkUrl100":"https://is1.example/1999/100x100bb.jpg"}
                ]}"#
                .into(),
            ))
        };
        let url = find("Waves", "Joey Bada$$", "1999", 187_000, &get).unwrap().unwrap();
        assert_eq!(url, "https://is1.example/1999/1200x1200bb.jpg");
        assert!(calls.borrow()[0].starts_with("https://itunes.apple.com/search?term=Joey+Bada%24%24+Waves&media=music&entity=song"));
        // A different length (another version) or no match: nothing.
        assert!(find("Waves", "Joey Bada$$", "1999", 240_000, &get).unwrap().is_none());
        assert!(find("Some Other Song", "Joey Bada$$", "", 0, &get).unwrap().is_none());
    }

    #[test]
    fn pictures_are_compared_and_typed() {
        let a = png([255, 120, 40], 64);
        let big = png([255, 120, 40], 512);
        let other = png([40, 90, 255], 64);
        assert!(difference(&a, &big).unwrap() < 0.05);
        assert!(difference(&a, &other).unwrap() > 0.15);
        assert_eq!(mime_of(&a), Some("image/png"));
        assert_eq!(mime_of(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(mime_of(b"<html>"), None);
    }

    #[test]
    fn cache_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let c = ArtCache::new(dir.path().join("covers"));
        assert!(c.get("k").is_none());
        c.put("k", None);
        assert_eq!(c.get("k"), Some(None));
        c.put("k", Some(b"jpeg"));
        assert_eq!(c.get("k"), Some(Some(b"jpeg".to_vec())));
    }
}
