//! Pictures, GIFs and videos for the PC's own screens: a case display or
//! sensor panel that Windows sees as a monitor (OmniHub fills it with a
//! window), or a cooler's LCD (sent through liquidctl). Files are copied
//! into OmniHub's data folder after their first bytes say what they are,
//! and are served to the window only by their id.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const PICTURE_MAX: u64 = 40 << 20;
pub const VIDEO_MAX: u64 = 1 << 30;
pub const ITEMS_MAX: usize = 200;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Picture,
    Gif,
    Video,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    /// "<uuid>.<ext>": the file's name in the library.
    pub id: String,
    pub name: String,
    pub kind: MediaKind,
    pub mime: String,
    pub bytes: u64,
}

/// What a file is, from its first bytes: (extension, MIME type, kind).
pub fn sniff(head: &[u8]) -> Option<(&'static str, &'static str, MediaKind)> {
    if let Some((ext, mime)) = crate::media::look::picture_kind("", head) {
        let kind = if ext == "gif" { MediaKind::Gif } else { MediaKind::Picture };
        return Some((ext, mime, kind));
    }
    if head.len() >= 12 && &head[4..8] == b"ftyp" {
        return Some(("mp4", "video/mp4", MediaKind::Video));
    }
    if head.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return Some(("webm", "video/webm", MediaKind::Video));
    }
    None
}

/// A library id: our own "<uuid>.<ext>" and nothing that could leave the folder.
pub fn valid_id(id: &str) -> bool {
    let Some((stem, ext)) = id.split_once('.') else { return false };
    stem.len() == 36 && stem.chars().all(|c| c.is_ascii_hexdigit() || c == '-') && matches!(ext, "png" | "jpg" | "gif" | "webp" | "mp4" | "webm")
}

pub fn mime_of(id: &str) -> &'static str {
    match id.rsplit('.').next() {
        Some("png") => "image/png",
        Some("jpg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        _ => "application/octet-stream",
    }
}

pub struct Library {
    pub dir: PathBuf,
}

impl Library {
    pub fn new(dir: PathBuf) -> Self {
        Library { dir }
    }

    fn index_path(&self) -> PathBuf {
        self.dir.join("index.json")
    }

    pub fn list(&self) -> Vec<Media> {
        std::fs::read(self.index_path()).ok().and_then(|b| serde_json::from_slice::<Vec<Media>>(&b).ok()).unwrap_or_default().into_iter().filter(|m| valid_id(&m.id) && self.dir.join(&m.id).is_file()).collect()
    }

    fn save(&self, list: &[Media]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        std::fs::write(self.index_path(), serde_json::to_vec_pretty(list).map_err(std::io::Error::other)?)
    }

    /// Copy a picture, GIF or video in. The window never passes a path:
    /// this is called with the file the user picked in Windows' dialog.
    pub fn import(&self, src: &Path) -> Result<Media, String> {
        use std::io::Read;
        let meta = std::fs::metadata(src).map_err(|e| e.to_string())?;
        if !meta.is_file() {
            return Err("That isn't a file.".into());
        }
        let mut head = [0u8; 64];
        let n = std::fs::File::open(src).and_then(|mut f| f.read(&mut head)).map_err(|e| e.to_string())?;
        let (ext, mime, kind) = sniff(&head[..n]).ok_or("Use a PNG, JPEG, GIF or WebP picture, or an MP4 or WebM video.")?;
        let max = if kind == MediaKind::Video { VIDEO_MAX } else { PICTURE_MAX };
        if meta.len() > max {
            return Err(format!("That file is over {} MB.", max >> 20));
        }
        let mut list = self.list();
        if list.len() >= ITEMS_MAX {
            return Err("The library is full; remove something first.".into());
        }
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let id = format!("{}.{ext}", uuid::Uuid::new_v4());
        std::fs::copy(src, self.dir.join(&id)).map_err(|e| e.to_string())?;
        let name: String = src.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default().chars().take(80).collect();
        let m = Media { id, name, kind, mime: mime.into(), bytes: meta.len() };
        list.push(m.clone());
        self.save(&list).map_err(|e| e.to_string())?;
        Ok(m)
    }

    pub fn remove(&self, id: &str) -> Result<(), String> {
        if !valid_id(id) {
            return Err("Not in the library.".into());
        }
        let list: Vec<Media> = self.list().into_iter().filter(|m| m.id != id).collect();
        let _ = std::fs::remove_file(self.dir.join(id));
        self.save(&list).map_err(|e| e.to_string())
    }

    /// The file behind an id, if it is in the library.
    pub fn path(&self, id: &str) -> Option<PathBuf> {
        if !valid_id(id) {
            return None;
        }
        let p = self.dir.join(id);
        p.is_file().then_some(p)
    }

    pub fn get(&self, id: &str) -> Option<Media> {
        self.list().into_iter().find(|m| m.id == id)
    }
}

/// A byte range from an HTTP `Range` header ("bytes=0-", "bytes=100-199"),
/// clamped to the file and to `chunk` bytes.
pub fn range(header: Option<&str>, len: u64, chunk: u64) -> Option<(u64, u64)> {
    let len = len.max(1);
    let (start, end) = match header.and_then(|h| h.trim().strip_prefix("bytes=")) {
        Some(r) => {
            let (a, b) = r.split(',').next()?.split_once('-')?;
            match (a.trim().parse::<u64>().ok(), b.trim().parse::<u64>().ok()) {
                (Some(a), Some(b)) => (a, b.min(len - 1)),
                (Some(a), None) => (a, len - 1),
                // The last n bytes.
                (None, Some(n)) => (len.saturating_sub(n), len - 1),
                (None, None) => return None,
            }
        }
        None => (0, len - 1),
    };
    if start > end || start >= len {
        return None;
    }
    Some((start, end.min(start + chunk - 1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_pictures_and_videos_by_their_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let lib = Library::new(dir.path().join("lib"));
        let png = dir.path().join("Kraken art.png");
        std::fs::write(&png, b"\x89PNG\r\n\x1a\n0000").unwrap();
        let mp4 = dir.path().join("loop.mp4");
        let mut v = vec![0, 0, 0, 0x20];
        v.extend_from_slice(b"ftypisom0000");
        std::fs::write(&mp4, &v).unwrap();
        let fake = dir.path().join("fake.png");
        std::fs::write(&fake, b"MZ not a picture").unwrap();

        let a = lib.import(&png).unwrap();
        assert_eq!((a.kind, a.name.as_str(), a.mime.as_str()), (MediaKind::Picture, "Kraken art", "image/png"));
        assert!(a.id.ends_with(".png") && valid_id(&a.id));
        let b = lib.import(&mp4).unwrap();
        assert_eq!(b.kind, MediaKind::Video);
        assert!(lib.import(&fake).is_err());
        assert_eq!(lib.list().len(), 2);
        assert!(lib.path(&a.id).is_some());
        assert!(lib.path("../settings.json").is_none());
        assert!(lib.path("..\\x.png").is_none());
        lib.remove(&a.id).unwrap();
        assert_eq!(lib.list(), vec![b]);
        assert_eq!(sniff(b"GIF89a....").map(|s| s.2), Some(MediaKind::Gif));
    }

    #[test]
    fn serves_byte_ranges() {
        assert_eq!(range(None, 1000, 4096), Some((0, 999)));
        assert_eq!(range(Some("bytes=0-"), 10_000, 4096), Some((0, 4095)));
        assert_eq!(range(Some("bytes=100-199"), 1000, 4096), Some((100, 199)));
        assert_eq!(range(Some("bytes=-100"), 1000, 4096), Some((900, 999)));
        assert_eq!(range(Some("bytes=5000-"), 1000, 4096), None);
        assert_eq!(range(Some("bytes=900-5000"), 1000, 4096), Some((900, 999)));
    }
}
