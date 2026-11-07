//! Your own picture behind a Bleach theme: a character or anything else you
//! choose, copied into OmniHub's data folder on this PC and shown faded
//! behind the cards. OmniHub ships no artwork from the series; this is only
//! ever a file you pick.

use std::path::{Path, PathBuf};

pub const THEMES: &[&str] = &["bankai", "hollow", "mugetsu", "hakka"];
const EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];
const MAX_BYTES: u64 = 25 << 20;

pub struct ThemeArt {
    dir: PathBuf,
}

/// The file's own first bytes say it's a picture (not just its name).
pub fn looks_like_image(head: &[u8]) -> bool {
    head.starts_with(&[0x89, b'P', b'N', b'G']) || head.starts_with(&[0xFF, 0xD8, 0xFF]) || head.starts_with(b"GIF8") || (head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP")
}

impl ThemeArt {
    pub fn new(data: &Path) -> ThemeArt {
        ThemeArt { dir: data.join("theme-art") }
    }

    fn check(theme: &str) -> Result<(), String> {
        if THEMES.contains(&theme) {
            Ok(())
        } else {
            Err("That theme doesn't take a picture".into())
        }
    }

    /// The picture set for a theme, if any.
    pub fn path(&self, theme: &str) -> Option<PathBuf> {
        Self::check(theme).ok()?;
        EXTS.iter().map(|e| self.dir.join(format!("{theme}.{e}"))).find(|p| p.is_file())
    }

    /// Use `src` for `theme` (copied here), or remove the picture with None.
    pub fn set(&self, theme: &str, src: Option<&Path>) -> Result<Option<PathBuf>, String> {
        Self::check(theme)?;
        let Some(src) = src else {
            for e in EXTS {
                let _ = std::fs::remove_file(self.dir.join(format!("{theme}.{e}")));
            }
            return Ok(None);
        };
        let ext = src.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if !EXTS.contains(&ext.as_str()) {
            return Err("Pick a PNG, JPG, WebP or GIF picture".into());
        }
        let meta = std::fs::metadata(src).map_err(|e| format!("Couldn't read that file: {e}"))?;
        if meta.len() > MAX_BYTES {
            return Err("That picture is over 25 MB; pick a smaller one".into());
        }
        let bytes = std::fs::read(src).map_err(|e| format!("Couldn't read that file: {e}"))?;
        if !looks_like_image(&bytes[..bytes.len().min(16)]) {
            return Err("That file isn't a picture".into());
        }
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        for e in EXTS {
            let _ = std::fs::remove_file(self.dir.join(format!("{theme}.{e}")));
        }
        let ext = if ext == "jpeg" { "jpg".to_string() } else { ext };
        let dest = self.dir.join(format!("{theme}.{ext}"));
        crate::settings::write_atomic(&dest, &bytes).map_err(|e| e.to_string())?;
        Ok(Some(dest))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_one_picture_per_theme() {
        let t = tempfile::tempdir().unwrap();
        let art = ThemeArt::new(t.path());
        let png = t.path().join("me.png");
        std::fs::write(&png, [0x89, b'P', b'N', b'G', 13, 10, 26, 10, 0, 0]).unwrap();
        let fake = t.path().join("fake.jpg");
        std::fs::write(&fake, b"not a picture").unwrap();
        assert!(art.set("bankai", Some(&fake)).is_err(), "checked by content");
        assert!(art.set("classic", Some(&png)).is_err());
        let got = art.set("bankai", Some(&png)).unwrap().unwrap();
        assert!(got.ends_with("bankai.png"));
        assert_eq!(art.path("bankai"), Some(got));
        assert!(art.path("hakka").is_none());
        art.set("bankai", None).unwrap();
        assert!(art.path("bankai").is_none());
        assert!(looks_like_image(b"RIFF\0\0\0\0WEBPVP8 "));
    }
}
