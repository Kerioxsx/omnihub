//! Small JPEG previews of image files (the Storage grid view and the phone
//! file browser), cached in memory.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;

/// Larger images are not decoded for a preview.
pub const MAX_SOURCE: u64 = 60 << 20;
const CACHE_ENTRIES: usize = 400;

/// Image types the `image` crate decodes.
pub fn is_previewable(path: &Path) -> bool {
    matches!(crate::storage::tree::extension_of(&path.to_string_lossy()).as_deref(), Some("jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp"))
}

type Key = (PathBuf, i64, u64, u32);

#[derive(Default)]
pub struct Thumbs {
    cache: Mutex<VecDeque<(Key, Arc<Vec<u8>>)>>,
}

impl Thumbs {
    /// A JPEG at most `size` pixels on its longer side.
    pub fn jpeg(&self, path: &Path, size: u32) -> std::io::Result<Arc<Vec<u8>>> {
        let meta = std::fs::metadata(path)?;
        if !meta.is_file() || !is_previewable(path) || meta.len() > MAX_SOURCE {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a previewable image"));
        }
        let mtime = meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64);
        let key: Key = (path.to_path_buf(), mtime, meta.len(), size.clamp(48, 1024));
        {
            let mut c = self.cache.lock();
            if let Some(i) = c.iter().position(|(k, _)| *k == key) {
                let hit = c.remove(i).expect("index from position");
                let bytes = hit.1.clone();
                c.push_back(hit);
                return Ok(bytes);
            }
        }
        let img = image::open(path).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        let t = img.thumbnail(key.3, key.3).to_rgb8();
        let mut out = std::io::Cursor::new(Vec::new());
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 78).encode_image(&t).map_err(std::io::Error::other)?;
        let bytes = Arc::new(out.into_inner());
        let mut c = self.cache.lock();
        c.push_back((key, bytes.clone()));
        while c.len() > CACHE_ENTRIES {
            c.pop_front();
        }
        Ok(bytes)
    }

    /// The preview as a `data:` URL for the desktop UI.
    pub fn data_url(&self, path: &Path, size: u32) -> std::io::Result<String> {
        use base64::Engine;
        let bytes = self.jpeg(path, size)?;
        Ok(format!("data:image/jpeg;base64,{}", base64::engine::general_purpose::STANDARD.encode(&*bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_are_cached_and_refreshed() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("pic.png");
        image::RgbImage::from_pixel(800, 400, image::Rgb([200, 30, 30])).save(&p).unwrap();
        let t = Thumbs::default();
        let a = t.jpeg(&p, 128).unwrap();
        let img = image::load_from_memory(&a).unwrap();
        assert_eq!((img.width(), img.height()), (128, 64));
        assert!(Arc::ptr_eq(&a, &t.jpeg(&p, 128).unwrap()), "second call is served from the cache");
        assert!(t.data_url(&p, 64).unwrap().starts_with("data:image/jpeg;base64,"));
        std::fs::write(dir.path().join("notes.txt"), "x").unwrap();
        assert!(t.jpeg(&dir.path().join("notes.txt"), 64).is_err());
    }
}
