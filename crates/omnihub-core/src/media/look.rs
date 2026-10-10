//! The user's own look for Aurora's lyrics: a font file and a zip of item
//! textures (a game's icons, say) that stand in for the emojis. The user picks
//! the files on their PC; they are copied into OmniHub's data folder and used
//! only on that PC. OmniHub ships none of them.
//!
//! Both are checked before they are kept: the font must be a TrueType,
//! OpenType or WOFF file, and only PNG pictures are taken out of the zip
//! (by their file name alone, flattened, size-limited), so a zip can never
//! write outside the textures folder or fill the disk.

use base64::Engine as _;
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Fonts are small (a few hundred KB at most); anything bigger is not one.
const MAX_FONT: u64 = 20 << 20;
/// A texture is a small icon.
const MAX_TEXTURE: u64 = 2 << 20;
/// All of them together.
const MAX_TEXTURES_TOTAL: u64 = 300 << 20;
const MAX_ENTRIES: usize = 50_000;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LookStatus {
    /// The chosen font's name (its file name), if any.
    pub font: Option<String>,
    /// The texture names (file names without ".png"), sorted.
    pub textures: Vec<String>,
}

pub struct LookFiles {
    dir: PathBuf,
}

/// The font's kind from its first bytes: (file extension, MIME type).
pub fn font_kind(head: &[u8]) -> Option<(&'static str, &'static str)> {
    match head.get(..4)? {
        [0, 1, 0, 0] | b"true" => Some(("ttf", "font/ttf")),
        b"OTTO" => Some(("otf", "font/otf")),
        b"wOFF" => Some(("woff", "font/woff")),
        b"wOF2" => Some(("woff2", "font/woff2")),
        _ => None,
    }
}

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

/// The name a zip entry is kept under: its file name only, lower case,
/// letters, digits and "_" (others become "_"); None when it is not a PNG.
pub fn texture_name(entry: &str) -> Option<String> {
    let file = entry.rsplit(['/', '\\']).next()?;
    let stem = file.strip_suffix(".png").or_else(|| file.strip_suffix(".PNG"))?;
    let name: String = stem.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect();
    (!name.is_empty() && name.len() <= 120 && name.chars().any(|c| c.is_ascii_alphanumeric())).then_some(name)
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 120 && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl LookFiles {
    pub fn new(dir: PathBuf) -> Self {
        LookFiles { dir }
    }

    fn textures_dir(&self) -> PathBuf {
        self.dir.join("textures")
    }

    /// The font file kept now, with its MIME type.
    fn font_file(&self) -> Option<(PathBuf, &'static str)> {
        for (ext, mime) in [("ttf", "font/ttf"), ("otf", "font/otf"), ("woff", "font/woff"), ("woff2", "font/woff2")] {
            let p = self.dir.join(format!("font.{ext}"));
            if p.is_file() {
                return Some((p, mime));
            }
        }
        None
    }

    pub fn status(&self) -> LookStatus {
        let font = self.font_file().map(|_| std::fs::read_to_string(self.dir.join("font-name.txt")).unwrap_or_else(|_| "Your font".into()));
        let mut textures: Vec<String> = std::fs::read_dir(self.textures_dir())
            .map(|rd| rd.filter_map(|e| e.ok()).filter_map(|e| e.file_name().to_str()?.strip_suffix(".png").map(str::to_string)).filter(|n| valid_name(n)).collect())
            .unwrap_or_default();
        textures.sort();
        LookStatus { font, textures }
    }

    /// Keep a copy of the font at `src` (replacing the last one).
    pub fn import_font(&self, src: &Path) -> Result<String, String> {
        let size = std::fs::metadata(src).map_err(|e| format!("Could not read the font: {e}"))?.len();
        if size > MAX_FONT {
            return Err("That file is too big to be a font.".into());
        }
        let bytes = std::fs::read(src).map_err(|e| format!("Could not read the font: {e}"))?;
        let (ext, _) = font_kind(&bytes).ok_or("That is not a font file (TrueType .ttf, OpenType .otf or .woff).")?;
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let tmp = self.dir.join("font.part");
        std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
        self.forget_font();
        std::fs::rename(&tmp, self.dir.join(format!("font.{ext}"))).map_err(|e| e.to_string())?;
        let name = src.file_stem().and_then(|s| s.to_str()).unwrap_or("Your font").chars().take(60).collect::<String>();
        let _ = std::fs::write(self.dir.join("font-name.txt"), &name);
        Ok(name)
    }

    pub fn forget_font(&self) {
        for ext in ["ttf", "otf", "woff", "woff2"] {
            let _ = std::fs::remove_file(self.dir.join(format!("font.{ext}")));
        }
        let _ = std::fs::remove_file(self.dir.join("font-name.txt"));
    }

    /// The font as a data URL, for the lyrics' @font-face.
    pub fn font_data_url(&self) -> Option<String> {
        let (path, mime) = self.font_file()?;
        let bytes = std::fs::read(path).ok().filter(|b| b.len() as u64 <= MAX_FONT && font_kind(b).is_some())?;
        Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
    }

    /// Take the PNG pictures out of the zip at `src` (replacing the last set).
    /// Returns how many there are.
    pub fn import_textures(&self, src: &Path) -> Result<usize, String> {
        let f = std::fs::File::open(src).map_err(|e| format!("Could not open the zip: {e}"))?;
        let mut z = zip::ZipArchive::new(std::io::BufReader::new(f)).map_err(|_| "That is not a zip file.".to_string())?;
        if z.len() > MAX_ENTRIES {
            return Err("That zip has too many files.".into());
        }
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let tmp = self.dir.join("textures.part");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        let result = (|| {
            // Item pictures win over block pictures of the same name (resource packs have both).
            let mut order: Vec<usize> = (0..z.len()).collect();
            let names: Vec<String> = (0..z.len()).map(|i| z.name_for_index(i).unwrap_or_default().to_string()).collect();
            order.sort_by_key(|&i| (!names[i].contains("/item/"), names[i].clone()));
            let (mut count, mut total) = (0usize, 0u64);
            for i in order {
                let Some(name) = texture_name(&names[i]) else { continue };
                let dest = tmp.join(format!("{name}.png"));
                if dest.exists() {
                    continue;
                }
                let mut entry = z.by_index(i).map_err(|e| e.to_string())?;
                if !entry.is_file() || entry.size() > MAX_TEXTURE {
                    continue;
                }
                let mut bytes = Vec::with_capacity(entry.size() as usize);
                (&mut entry).take(MAX_TEXTURE + 1).read_to_end(&mut bytes).map_err(|e| format!("The zip is damaged: {e}"))?;
                if bytes.len() as u64 > MAX_TEXTURE || !bytes.starts_with(PNG) {
                    continue;
                }
                total += bytes.len() as u64;
                if total > MAX_TEXTURES_TOTAL {
                    return Err("The pictures in that zip add up to too much.".to_string());
                }
                std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
                count += 1;
            }
            if count == 0 {
                return Err("No PNG pictures in that zip.".to_string());
            }
            Ok(count)
        })();
        match result {
            Ok(count) => {
                let _ = std::fs::remove_dir_all(self.textures_dir());
                std::fs::rename(&tmp, self.textures_dir()).map_err(|e| e.to_string())?;
                Ok(count)
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&tmp);
                Err(e)
            }
        }
    }

    pub fn forget_textures(&self) {
        let _ = std::fs::remove_dir_all(self.textures_dir());
    }

    /// One texture as a data URL (None when there is no such picture).
    pub fn texture_data_url(&self, name: &str) -> Option<String> {
        if !valid_name(name) {
            return None;
        }
        let bytes = std::fs::read(self.textures_dir().join(format!("{name}.png"))).ok().filter(|b| b.len() as u64 <= MAX_TEXTURE && b.starts_with(PNG))?;
        Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn png(extra: &[u8]) -> Vec<u8> {
        [PNG, extra].concat()
    }

    #[test]
    fn knows_fonts_by_their_first_bytes() {
        assert_eq!(font_kind(&[0, 1, 0, 0, 9]), Some(("ttf", "font/ttf")));
        assert_eq!(font_kind(b"OTTO...."), Some(("otf", "font/otf")));
        assert_eq!(font_kind(b"wOF2...."), Some(("woff2", "font/woff2")));
        assert_eq!(font_kind(b"PK\x03\x04"), None);
        assert_eq!(font_kind(b"ab"), None);
    }

    #[test]
    fn keeps_only_the_file_name_of_pngs() {
        assert_eq!(texture_name("apple.png").as_deref(), Some("apple"));
        assert_eq!(texture_name("assets/minecraft/textures/item/Golden_Apple.png").as_deref(), Some("golden_apple"));
        assert_eq!(texture_name("..\\..\\evil name.png").as_deref(), Some("evil_name"));
        assert_eq!(texture_name("../../x.txt"), None);
        assert_eq!(texture_name("dir/.png"), None);
        assert_eq!(texture_name("item/"), None);
    }

    #[test]
    fn imports_a_font_and_a_texture_zip_and_forgets_them() {
        let dir = tempfile::tempdir().unwrap();
        let look = LookFiles::new(dir.path().join("look"));
        assert_eq!(look.status(), LookStatus { font: None, textures: vec![] });

        let font = dir.path().join("minecraft.ttf");
        std::fs::write(&font, [&[0u8, 1, 0, 0][..], &[7; 60]].concat()).unwrap();
        assert_eq!(look.import_font(&font).unwrap(), "minecraft");
        assert!(look.font_data_url().unwrap().starts_with("data:font/ttf;base64,AAEAAA"));
        let not_font = dir.path().join("x.ttf");
        std::fs::write(&not_font, b"hello").unwrap();
        assert!(look.import_font(&not_font).is_err());
        assert_eq!(look.status().font.as_deref(), Some("minecraft"), "a bad file leaves the last font");

        let zip_path = dir.path().join("items.zip");
        let mut w = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        for (name, bytes) in [
            ("apple.png", png(b"item")),
            ("textures/block/diamond.png", png(b"block")),
            ("textures/item/diamond.png", png(b"item")),
            ("../../escape.png", png(b"x")),
            ("notes.txt", b"hi".to_vec()),
            ("fake.png", b"not a png".to_vec()),
        ] {
            w.start_file(name, opts).unwrap();
            w.write_all(&bytes).unwrap();
        }
        w.finish().unwrap();
        assert_eq!(look.import_textures(&zip_path).unwrap(), 3);
        assert_eq!(look.status().textures, vec!["apple", "diamond", "escape"]);
        assert!(!dir.path().join("escape.png").exists() && !dir.path().join("look").join("escape.png").exists());
        let diamond = look.texture_data_url("diamond").unwrap();
        let b = base64::engine::general_purpose::STANDARD.decode(diamond.trim_start_matches("data:image/png;base64,")).unwrap();
        assert!(b.ends_with(b"item"), "the item picture wins over the block one");
        assert_eq!(look.texture_data_url("../apple"), None);
        assert_eq!(look.texture_data_url("pear"), None);

        // A zip without pictures keeps the last set.
        let empty = dir.path().join("empty.zip");
        let mut w = zip::ZipWriter::new(std::fs::File::create(&empty).unwrap());
        w.start_file("readme.txt", opts).unwrap();
        w.write_all(b"hi").unwrap();
        w.finish().unwrap();
        assert!(look.import_textures(&empty).is_err());
        assert_eq!(look.status().textures.len(), 3);

        look.forget_font();
        look.forget_textures();
        assert_eq!(look.status(), LookStatus { font: None, textures: vec![] });
    }
}
