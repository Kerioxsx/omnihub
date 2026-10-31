//! The user's own look for Aurora's lyrics: a font file, and an emoji pack
//! that stands in for the emojis. The pack is either a zip of pictures (any
//! emoji set: Twemoji, Noto, OpenMoji, Discord emotes, a game's item
//! textures…; PNG, GIF, WebP, JPEG or SVG, named by code point, by name or by
//! the emoji itself) or an emoji font. The user picks the files on their PC;
//! they are copied into OmniHub's data folder and used only on that PC.
//! OmniHub ships none of them.
//!
//! Everything is checked before it is kept: fonts by their first bytes
//! (TrueType, OpenType, collections, WOFF), pictures by theirs; pictures are
//! taken out of the zip by their file name alone (flattened, size-limited),
//! so a zip can never write outside the pack's folder or fill the disk.

use base64::Engine as _;
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Fonts are small (a few hundred KB at most); anything bigger is not one.
const MAX_FONT: u64 = 20 << 20;
/// A picture is a small icon (an animated GIF can be a few MB).
const MAX_TEXTURE: u64 = 4 << 20;
/// All of them together.
const MAX_TEXTURES_TOTAL: u64 = 400 << 20;
const MAX_ENTRIES: usize = 60_000;
/// Picture kinds, by preference when a pack has the same name twice
/// (SVG scales best; then the bigger file, usually the sharper one).
const PICTURES: [&str; 5] = ["svg", "png", "webp", "gif", "jpg"];

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LookStatus {
    /// The chosen lyrics font's name (its file name), if any.
    pub font: Option<String>,
    /// The emoji pack's picture names (file names without the extension), sorted.
    pub textures: Vec<String>,
    /// The emoji pack, when it is a font: its name.
    pub emoji_font: Option<String>,
}

/// What an emoji pack turned out to be.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum EmojiPack {
    Pictures { count: usize },
    Font { name: String },
}

pub struct LookFiles {
    dir: PathBuf,
}

/// The picture kept for a name: which zip entry, its kind, and how it ranks
/// (an item picture, an SVG, its size).
struct Choice {
    index: usize,
    ext: &'static str,
    rank: (bool, bool, u64),
}

/// The font's kind from its first bytes: (file extension, MIME type).
pub fn font_kind(head: &[u8]) -> Option<(&'static str, &'static str)> {
    match head.get(..4)? {
        [0, 1, 0, 0] | b"true" => Some(("ttf", "font/ttf")),
        b"OTTO" => Some(("otf", "font/otf")),
        b"wOFF" => Some(("woff", "font/woff")),
        b"wOF2" => Some(("woff2", "font/woff2")),
        b"ttcf" => Some(("ttc", "font/collection")),
        _ => None,
    }
}

const FONT_KINDS: [(&str, &str); 5] = [("ttf", "font/ttf"), ("otf", "font/otf"), ("woff", "font/woff"), ("woff2", "font/woff2"), ("ttc", "font/collection")];

/// A picture's kind from its first bytes (and its file extension, for SVG,
/// which is text): (extension, MIME type).
pub fn picture_kind(ext: &str, head: &[u8]) -> Option<(&'static str, &'static str)> {
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("png", "image/png"))
    } else if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        Some(("gif", "image/gif"))
    } else if head.len() >= 12 && &head[..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        Some(("webp", "image/webp"))
    } else if head.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(("jpg", "image/jpeg"))
    } else if ext == "svg" {
        // Shown only as an <img> (where SVG runs no script); it must be an SVG.
        let text = String::from_utf8_lossy(&head[..head.len().min(4096)]).to_ascii_lowercase();
        let t = text.trim_start_matches('\u{feff}').trim_start();
        (t.starts_with('<') && t.contains("<svg")).then_some(("svg", "image/svg+xml"))
    } else {
        None
    }
}

/// The name a picture is kept and looked up under: its file name only,
/// without the extension, lower case, words joined by single "_"; characters
/// beyond ASCII (an emoji in the file name) become their code points, so
/// "🔥.png", "1F525.svg" and "1f525.png" are all "1f525", and "Red Heart.png"
/// is "red_heart". None when it is not a picture's name.
pub fn texture_name(entry: &str) -> Option<(String, &'static str)> {
    let file = entry.rsplit(['/', '\\']).next()?;
    let (stem, ext) = file.rsplit_once('.')?;
    let ext = match ext.to_ascii_lowercase().as_str() {
        "png" => "png",
        "gif" => "gif",
        "webp" => "webp",
        "jpg" | "jpeg" => "jpg",
        "svg" => "svg",
        _ => return None,
    };
    let mut words: Vec<String> = Vec::new();
    let mut cur = String::new();
    for c in stem.chars() {
        if c.is_ascii_alphanumeric() {
            cur.push(c.to_ascii_lowercase());
        } else {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            if !c.is_ascii() {
                words.push(format!("{:x}", c as u32));
            }
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    let name = words.join("_");
    (!name.is_empty() && name.len() <= 120).then_some((name, ext))
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

    /// The font file kept under `slot` ("font", "emoji-font"), with its MIME type.
    fn font_file(&self, slot: &str) -> Option<(PathBuf, &'static str)> {
        FONT_KINDS.iter().map(|(ext, mime)| (self.dir.join(format!("{slot}.{ext}")), *mime)).find(|(p, _)| p.is_file())
    }

    fn font_name(&self, slot: &str) -> Option<String> {
        self.font_file(slot).map(|_| std::fs::read_to_string(self.dir.join(format!("{slot}-name.txt"))).unwrap_or_else(|_| "Your font".into()))
    }

    pub fn status(&self) -> LookStatus {
        let mut textures: Vec<String> = std::fs::read_dir(self.textures_dir())
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter_map(|e| {
                        let f = e.file_name().to_str()?.to_string();
                        let (stem, ext) = f.rsplit_once('.')?;
                        (PICTURES.contains(&ext) && valid_name(stem)).then(|| stem.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default();
        textures.sort();
        textures.dedup();
        LookStatus { font: self.font_name("font"), textures, emoji_font: self.font_name("emoji-font") }
    }

    /// Keep a copy of the font at `src` under `slot` (replacing the last one).
    fn import_font_as(&self, src: &Path, slot: &str) -> Result<String, String> {
        let size = std::fs::metadata(src).map_err(|e| format!("Could not read the font: {e}"))?.len();
        if size > MAX_FONT {
            return Err("That file is too big to be a font.".into());
        }
        let bytes = std::fs::read(src).map_err(|e| format!("Could not read the font: {e}"))?;
        let (ext, _) = font_kind(&bytes).ok_or("That is not a font file (TrueType .ttf, OpenType .otf, .ttc or .woff).")?;
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let tmp = self.dir.join(format!("{slot}.part"));
        std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
        self.forget_font_slot(slot);
        std::fs::rename(&tmp, self.dir.join(format!("{slot}.{ext}"))).map_err(|e| e.to_string())?;
        let name = src.file_stem().and_then(|s| s.to_str()).unwrap_or("Your font").chars().take(60).collect::<String>();
        let _ = std::fs::write(self.dir.join(format!("{slot}-name.txt")), &name);
        Ok(name)
    }

    fn forget_font_slot(&self, slot: &str) {
        for (ext, _) in FONT_KINDS {
            let _ = std::fs::remove_file(self.dir.join(format!("{slot}.{ext}")));
        }
        let _ = std::fs::remove_file(self.dir.join(format!("{slot}-name.txt")));
    }

    fn font_url(&self, slot: &str) -> Option<String> {
        let (path, mime) = self.font_file(slot)?;
        let bytes = std::fs::read(path).ok().filter(|b| b.len() as u64 <= MAX_FONT && font_kind(b).is_some())?;
        Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
    }

    /// Keep a copy of the lyrics font at `src` (replacing the last one).
    pub fn import_font(&self, src: &Path) -> Result<String, String> {
        self.import_font_as(src, "font")
    }

    pub fn forget_font(&self) {
        self.forget_font_slot("font");
    }

    /// The lyrics font as a data URL, for its @font-face.
    pub fn font_data_url(&self) -> Option<String> {
        self.font_url("font")
    }

    /// The emoji pack's font as a data URL, when the pack is a font.
    pub fn emoji_font_data_url(&self) -> Option<String> {
        self.font_url("emoji-font")
    }

    /// Use the file at `src` as the emoji pack: a zip of pictures, or an
    /// emoji font (told apart by their first bytes). Replaces the last pack.
    pub fn import_emoji_pack(&self, src: &Path) -> Result<EmojiPack, String> {
        let mut head = [0u8; 4];
        let n = std::fs::File::open(src).and_then(|mut f| f.read(&mut head)).map_err(|e| format!("Could not read that file: {e}"))?;
        if head[..n].starts_with(b"PK") {
            let count = self.import_textures(src)?;
            self.forget_font_slot("emoji-font");
            Ok(EmojiPack::Pictures { count })
        } else if font_kind(&head[..n]).is_some() {
            let name = self.import_font_as(src, "emoji-font")?;
            self.forget_textures();
            Ok(EmojiPack::Font { name })
        } else {
            Err("That is neither a zip of pictures nor a font.".into())
        }
    }

    /// Take the pictures out of the zip at `src` (replacing the last set).
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
            // One picture per name. When a pack has a name twice (sizes,
            // styles, a resource pack's item and block pictures), the item
            // picture wins, then SVG, then the bigger file.
            let mut best: std::collections::HashMap<String, Choice> = std::collections::HashMap::new();
            for i in 0..z.len() {
                let Some(path) = z.name_for_index(i) else { continue };
                let Some((name, ext)) = texture_name(path) else { continue };
                let item = path.contains("/item/") || path.contains("/items/");
                let entry = z.by_index_raw(i).map_err(|e| e.to_string())?;
                if !entry.is_file() || entry.size() > MAX_TEXTURE || entry.size() == 0 {
                    continue;
                }
                let rank = (item, ext == "svg", entry.size());
                if best.get(&name).is_none_or(|b| rank > b.rank) {
                    best.insert(name, Choice { index: i, ext, rank });
                }
            }
            let (mut count, mut total) = (0usize, 0u64);
            let mut chosen: Vec<(String, Choice)> = best.into_iter().collect();
            chosen.sort_by_key(|(_, c)| c.index);
            for (name, Choice { index: i, ext, .. }) in chosen {
                let mut entry = z.by_index(i).map_err(|e| e.to_string())?;
                let mut bytes = Vec::with_capacity(entry.size() as usize);
                (&mut entry).take(MAX_TEXTURE + 1).read_to_end(&mut bytes).map_err(|e| format!("The zip is damaged: {e}"))?;
                if bytes.len() as u64 > MAX_TEXTURE {
                    continue;
                }
                let Some((kind, _)) = picture_kind(ext, &bytes) else { continue };
                total += bytes.len() as u64;
                if total > MAX_TEXTURES_TOTAL {
                    return Err("The pictures in that zip add up to too much.".to_string());
                }
                std::fs::write(tmp.join(format!("{name}.{kind}")), &bytes).map_err(|e| e.to_string())?;
                count += 1;
            }
            if count == 0 {
                return Err("No pictures in that zip (PNG, GIF, WebP, JPEG or SVG).".to_string());
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

    /// No emoji pack: neither pictures nor an emoji font.
    pub fn forget_emoji_pack(&self) {
        self.forget_textures();
        self.forget_font_slot("emoji-font");
    }

    /// One picture as a data URL (None when there is no such picture).
    pub fn texture_data_url(&self, name: &str) -> Option<String> {
        if !valid_name(name) {
            return None;
        }
        PICTURES.iter().find_map(|ext| {
            let bytes = std::fs::read(self.textures_dir().join(format!("{name}.{ext}"))).ok().filter(|b| b.len() as u64 <= MAX_TEXTURE)?;
            let (_, mime) = picture_kind(ext, &bytes)?;
            Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";

    fn png(extra: &[u8]) -> Vec<u8> {
        [PNG, extra].concat()
    }

    fn zip_of(path: &Path, files: &[(&str, Vec<u8>)]) {
        let mut w = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        for (name, bytes) in files {
            w.start_file(*name, opts).unwrap();
            w.write_all(bytes).unwrap();
        }
        w.finish().unwrap();
    }

    fn decode(url: &str) -> Vec<u8> {
        base64::engine::general_purpose::STANDARD.decode(url.split_once(',').unwrap().1).unwrap()
    }

    #[test]
    fn knows_fonts_and_pictures_by_their_first_bytes() {
        assert_eq!(font_kind(&[0, 1, 0, 0, 9]), Some(("ttf", "font/ttf")));
        assert_eq!(font_kind(b"OTTO...."), Some(("otf", "font/otf")));
        assert_eq!(font_kind(b"wOF2...."), Some(("woff2", "font/woff2")));
        assert_eq!(font_kind(b"ttcf...."), Some(("ttc", "font/collection")));
        assert_eq!(font_kind(b"PK\x03\x04"), None);
        assert_eq!(font_kind(b"ab"), None);
        assert_eq!(picture_kind("png", &png(b"")), Some(("png", "image/png")));
        assert_eq!(picture_kind("gif", b"GIF89a.."), Some(("gif", "image/gif")));
        assert_eq!(picture_kind("webp", b"RIFF\0\0\0\0WEBPVP8 "), Some(("webp", "image/webp")));
        assert_eq!(picture_kind("jpg", &[0xff, 0xd8, 0xff, 0xe0]), Some(("jpg", "image/jpeg")));
        assert_eq!(picture_kind("svg", b"\xef\xbb\xbf <?xml version='1.0'?><svg xmlns=''/>"), Some(("svg", "image/svg+xml")));
        assert_eq!(picture_kind("svg", b"<html><script>alert(1)</script>"), None);
        assert_eq!(picture_kind("png", b"not a png"), None);
        // The content decides, not the name.
        assert_eq!(picture_kind("png", b"GIF87a.."), Some(("gif", "image/gif")));
    }

    #[test]
    fn names_pictures_the_way_packs_do() {
        let n = |e: &str| texture_name(e).map(|(n, _)| n);
        assert_eq!(n("apple.png").as_deref(), Some("apple"));
        assert_eq!(n("assets/minecraft/textures/item/Golden_Apple.png").as_deref(), Some("golden_apple"));
        assert_eq!(n("..\\..\\evil name.png").as_deref(), Some("evil_name"));
        // Code points, however they are written, and the emoji itself.
        assert_eq!(n("72x72/1F525.png").as_deref(), Some("1f525"));
        assert_eq!(n("emoji_u1f525.png").as_deref(), Some("emoji_u1f525"));
        assert_eq!(n("U+1F525.svg").as_deref(), Some("u_1f525"));
        assert_eq!(n("1f469-200d-1f4bb.svg").as_deref(), Some("1f469_200d_1f4bb"));
        assert_eq!(n("🔥.gif").as_deref(), Some("1f525"));
        assert_eq!(n("❤️.webp").as_deref(), Some("2764_fe0f"));
        assert_eq!(n("Red Heart (3D).JPEG").as_deref(), Some("red_heart_3d"));
        assert_eq!(texture_name("a.jpeg").map(|(_, e)| e), Some("jpg"));
        assert_eq!(n("../../x.txt"), None);
        assert_eq!(n("dir/.png"), None);
        assert_eq!(n("item/"), None);
        assert_eq!(n("noext"), None);
    }

    #[test]
    fn imports_a_font_and_a_texture_zip_and_forgets_them() {
        let dir = tempfile::tempdir().unwrap();
        let look = LookFiles::new(dir.path().join("look"));
        let empty = LookStatus { font: None, textures: vec![], emoji_font: None };
        assert_eq!(look.status(), empty);

        let font = dir.path().join("minecraft.ttf");
        std::fs::write(&font, [&[0u8, 1, 0, 0][..], &[7; 60]].concat()).unwrap();
        assert_eq!(look.import_font(&font).unwrap(), "minecraft");
        assert!(look.font_data_url().unwrap().starts_with("data:font/ttf;base64,AAEAAA"));
        let not_font = dir.path().join("x.ttf");
        std::fs::write(&not_font, b"hello").unwrap();
        assert!(look.import_font(&not_font).is_err());
        assert_eq!(look.status().font.as_deref(), Some("minecraft"), "a bad file leaves the last font");

        let zip_path = dir.path().join("items.zip");
        zip_of(
            &zip_path,
            &[
                ("apple.png", png(b"item")),
                ("textures/block/diamond.png", png(b"block, a much bigger picture")),
                ("textures/item/diamond.png", png(b"item")),
                ("../../escape.png", png(b"x")),
                ("notes.txt", b"hi".to_vec()),
                ("fake.png", b"not a png".to_vec()),
            ],
        );
        assert_eq!(look.import_textures(&zip_path).unwrap(), 3);
        assert_eq!(look.status().textures, vec!["apple", "diamond", "escape"]);
        assert!(!dir.path().join("escape.png").exists() && !dir.path().join("look").join("escape.png").exists());
        assert!(decode(&look.texture_data_url("diamond").unwrap()).ends_with(b"item"), "the item picture wins over the block one");
        assert_eq!(look.texture_data_url("../apple"), None);
        assert_eq!(look.texture_data_url("pear"), None);

        // A zip without pictures keeps the last set.
        let none = dir.path().join("none.zip");
        zip_of(&none, &[("readme.txt", b"hi".to_vec())]);
        assert!(look.import_textures(&none).is_err());
        assert_eq!(look.status().textures.len(), 3);

        look.forget_font();
        look.forget_textures();
        assert_eq!(look.status(), empty);
    }

    #[test]
    fn takes_any_emoji_pack_pictures_or_a_font() {
        let dir = tempfile::tempdir().unwrap();
        let look = LookFiles::new(dir.path().join("look"));
        // Twemoji-like: the same emoji as PNG at two sizes and as SVG; SVG wins.
        let svg = b"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 36 36'/>".to_vec();
        let pack = dir.path().join("emoji.zip");
        zip_of(
            &pack,
            &[
                ("twemoji/72x72/1f525.png", png(b"72")),
                ("twemoji/svg/1f525.svg", svg.clone()),
                ("twemoji/72x72/2764.png", png(b"heart")),
                ("emotes/joy.gif", b"GIF89a-animated".to_vec()),
                ("emotes/Pepe Cry.webp", b"RIFF\0\0\0\0WEBPVP8 ".to_vec()),
                ("photos/sun.jpeg", vec![0xff, 0xd8, 0xff, 0xe0, 1, 2]),
                ("bad.svg", b"<script>alert(1)</script>".to_vec()),
            ],
        );
        assert_eq!(look.import_emoji_pack(&pack).unwrap(), EmojiPack::Pictures { count: 5 });
        assert_eq!(look.status().textures, vec!["1f525", "2764", "joy", "pepe_cry", "sun"]);
        assert_eq!(look.texture_data_url("1f525").unwrap(), format!("data:image/svg+xml;base64,{}", base64::engine::general_purpose::STANDARD.encode(&svg)));
        assert!(look.texture_data_url("joy").unwrap().starts_with("data:image/gif;base64,"));
        assert!(look.texture_data_url("pepe_cry").unwrap().starts_with("data:image/webp;base64,"));
        assert!(look.texture_data_url("sun").unwrap().starts_with("data:image/jpeg;base64,"));
        assert_eq!(look.texture_data_url("bad"), None);
        assert_eq!(look.status().emoji_font, None);

        // An emoji font instead: the pictures go.
        let font = dir.path().join("NotoColorEmoji.ttf");
        std::fs::write(&font, [&[0u8, 1, 0, 0][..], &[1; 40]].concat()).unwrap();
        assert_eq!(look.import_emoji_pack(&font).unwrap(), EmojiPack::Font { name: "NotoColorEmoji".into() });
        let st = look.status();
        assert_eq!((st.emoji_font.as_deref(), st.textures.len(), st.font.as_deref()), (Some("NotoColorEmoji"), 0, None));
        assert!(look.emoji_font_data_url().unwrap().starts_with("data:font/ttf;base64,"));
        // And pictures again: the font goes.
        assert_eq!(look.import_emoji_pack(&pack).unwrap(), EmojiPack::Pictures { count: 5 });
        assert_eq!(look.status().emoji_font, None);

        let neither = dir.path().join("x.rar");
        std::fs::write(&neither, b"Rar!\x1a\x07").unwrap();
        assert!(look.import_emoji_pack(&neither).is_err());
        assert_eq!(look.status().textures.len(), 5, "a file that is neither leaves the pack");
    }
}
