//! ROM organizer: check your ROMs against a DAT file you provide (No-Intro
//! or Redump, XML or ClrMamePro text), rename them to the DAT's names, and
//! find duplicates. Files are matched by CRC32 (for a .zip, the CRC stored
//! inside it, so nothing is unpacked). Renames stay in the same folder,
//! never replace a file, and take the save file with them.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
pub struct DatRom {
    pub game: String,
    pub name: String,
    pub size: u64,
    pub crc: u32,
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

fn attr(tag: &str, key: &str) -> Option<String> {
    let at = tag.find(&format!(" {key}=\""))? + key.len() + 3;
    let end = tag[at..].find('"')? + at;
    Some(unescape(&tag[at..end]))
}

/// Read a DAT (Logiqx XML, or ClrMamePro's text format).
pub fn parse(text: &str) -> Vec<DatRom> {
    let mut out = Vec::new();
    if text.trim_start().starts_with('<') {
        let mut game = String::new();
        for piece in text.split('<').skip(1) {
            let tag = piece.split('>').next().unwrap_or("");
            if tag.starts_with("game ") || tag.starts_with("machine ") {
                game = attr(tag, "name").unwrap_or_default();
            } else if tag.starts_with("rom ") {
                let (Some(name), Some(crc)) = (attr(tag, "name"), attr(tag, "crc").and_then(|c| u32::from_str_radix(c.trim(), 16).ok())) else { continue };
                out.push(DatRom { game: game.clone(), name, size: attr(tag, "size").and_then(|s| s.parse().ok()).unwrap_or(0), crc });
            }
        }
    } else {
        let mut game = String::new();
        for line in text.lines().map(str::trim) {
            if let Some(rest) = line.strip_prefix("name \"") {
                game = rest.trim_end_matches('"').to_string();
            } else if let Some(rest) = line.strip_prefix("rom (") {
                let name = rest.split("name \"").nth(1).and_then(|r| r.split('"').next()).map(String::from);
                let field = |k: &str| rest.split(&format!(" {k} ")).nth(1).and_then(|r| r.split_whitespace().next()).map(String::from);
                let (Some(name), Some(crc)) = (name, field("crc").and_then(|c| u32::from_str_radix(&c, 16).ok())) else { continue };
                out.push(DatRom { game: game.clone(), name, size: field("size").and_then(|s| s.parse().ok()).unwrap_or(0), crc });
            }
        }
    }
    out
}

/// A ROM's CRC32: inside a one-file .zip, the stored one; else the file's.
pub fn crc_of(path: &Path) -> Option<(u32, u64)> {
    if path.extension().is_some_and(|x| x.eq_ignore_ascii_case("zip")) {
        let mut z = zip::ZipArchive::new(std::fs::File::open(path).ok()?).ok()?;
        if z.len() != 1 {
            return None;
        }
        let f = z.by_index_raw(0).ok()?;
        return Some((f.crc32(), f.size()));
    }
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() > 512 << 20 {
        return None;
    }
    let mut h = crc32fast::Hasher::new();
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Some((h.finalize(), meta.len()))
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RomCheck {
    pub path: String,
    /// "ok" (already the DAT's name), "rename", or "unknown" (not in the DAT).
    pub status: String,
    /// The name it should have (with the file's own extension for a .zip).
    pub proper: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DatReport {
    pub dat_roms: usize,
    pub checked: Vec<RomCheck>,
    /// Files with the same contents.
    pub duplicates: Vec<Vec<String>>,
}

pub fn check(dat: &[DatRom], files: &[PathBuf]) -> DatReport {
    let by_crc: HashMap<u32, &DatRom> = dat.iter().map(|r| (r.crc, r)).collect();
    let mut seen: HashMap<u32, Vec<String>> = HashMap::new();
    let mut checked = Vec::new();
    for p in files.iter().take(5000) {
        let Some((crc, _)) = crc_of(p) else { continue };
        let path = p.display().to_string();
        seen.entry(crc).or_default().push(path.clone());
        let file = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let zip = p.extension().is_some_and(|x| x.eq_ignore_ascii_case("zip"));
        let (status, proper) = match by_crc.get(&crc) {
            Some(r) => {
                let want = if zip { format!("{}.zip", r.game) } else { r.name.clone() };
                (if want == file { "ok" } else { "rename" }, Some(want))
            }
            None => ("unknown", None),
        };
        checked.push(RomCheck { path, status: status.into(), proper });
    }
    let mut duplicates: Vec<Vec<String>> = seen.into_values().filter(|v| v.len() > 1).collect();
    duplicates.sort();
    DatReport { dat_roms: dat.len(), checked, duplicates }
}

/// Rename the files the report says to; returns how many were renamed.
pub fn apply(report: &DatReport) -> (usize, Vec<String>) {
    let mut done = 0;
    let mut skipped = Vec::new();
    for c in report.checked.iter().filter(|c| c.status == "rename") {
        let Some(proper) = &c.proper else { continue };
        let from = Path::new(&c.path);
        if proper.contains(['/', '\\', ':']) || proper.starts_with('.') {
            skipped.push(format!("{proper}: not a plain file name"));
            continue;
        }
        let to = from.with_file_name(proper);
        if to.exists() {
            skipped.push(format!("{proper} is already there"));
            continue;
        }
        if std::fs::rename(from, &to).is_err() {
            skipped.push(format!("{} couldn't be renamed (open in an emulator?)", from.display()));
            continue;
        }
        done += 1;
        // Saves are found by the ROM's name, so they move with it.
        for ext in ["sav", "srm"] {
            let old = from.with_extension(ext);
            let new = to.with_extension(ext);
            if old.is_file() && !new.exists() {
                let _ = std::fs::rename(old, new);
            }
        }
    }
    (done, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_dats_and_renames() {
        let xml = r#"<?xml version="1.0"?><datafile><game name="Tiny &amp; Co (World)"><rom name="Tiny &amp; Co (World).gba" size="5" crc="3610a686" md5="x"/></game></datafile>"#;
        let d = parse(xml);
        assert_eq!(d, vec![DatRom { game: "Tiny & Co (World)".into(), name: "Tiny & Co (World).gba".into(), size: 5, crc: 0x3610a686 }]);
        let cmp = "clrmamepro (\n\tname \"Test\"\n)\ngame (\n\tname \"Tiny (World)\"\n\trom ( name \"Tiny (World).gba\" size 5 crc 3610A686 md5 aa )\n)\n";
        assert_eq!(parse(cmp)[0].crc, 0x3610a686);
        let t = tempfile::tempdir().unwrap();
        // crc32("hello") = 3610a686
        let a = t.path().join("tiny.gba");
        std::fs::write(&a, b"hello").unwrap();
        std::fs::write(t.path().join("tiny.sav"), b"save").unwrap();
        let b = t.path().join("copy.gba");
        std::fs::write(&b, b"hello").unwrap();
        let c = t.path().join("other.gba");
        std::fs::write(&c, b"other").unwrap();
        let r = check(&d, &[a.clone(), b.clone(), c.clone()]);
        assert_eq!(r.checked.iter().map(|x| x.status.as_str()).collect::<Vec<_>>(), vec!["rename", "rename", "unknown"]);
        assert_eq!(r.duplicates.len(), 1);
        let (n, skipped) = apply(&r);
        // The second copy can't take the same name.
        assert_eq!((n, skipped.len()), (1, 1));
        assert!(t.path().join("Tiny & Co (World).gba").is_file() && t.path().join("Tiny & Co (World).sav").is_file());
    }
}
