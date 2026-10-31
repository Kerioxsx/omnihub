//! File tools: check a download's checksum, shred a file for good, rename
//! many files by a pattern (with a preview), and the files Windows lists as
//! recently used.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use sha2::Digest;

// ---------- checksums ----------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum HashKind {
    #[default]
    Sha256,
    Sha1,
    Sha512,
}

/// The file's checksum (lower-case hex); `progress` gets (done, total) bytes.
pub fn hash_file(path: &Path, kind: HashKind, progress: &mut dyn FnMut(u64, u64), cancel: &AtomicBool) -> Result<String, String> {
    let total = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 1 << 20];
    let mut done = 0u64;
    macro_rules! run {
        ($h:expr) => {{
            let mut h = $h;
            loop {
                if cancel.load(Ordering::Relaxed) {
                    return Err("Stopped.".into());
                }
                let n = f.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
                done += n as u64;
                progress(done, total);
            }
            hex::encode(h.finalize())
        }};
    }
    Ok(match kind {
        HashKind::Sha256 => run!(sha2::Sha256::new()),
        HashKind::Sha512 => run!(sha2::Sha512::new()),
        HashKind::Sha1 => run!(sha1::Sha1::new()),
    })
}

/// A pasted checksum, cleaned: "SHA256: AB CD…", "sha256:abcd" → "abcd…".
pub fn clean_checksum(s: &str) -> String {
    let s = s.trim();
    let s = s.rsplit_once(':').map_or(s, |(k, v)| if k.len() <= 8 { v } else { s });
    s.chars().filter(|c| c.is_ascii_hexdigit()).collect::<String>().to_lowercase()
}

/// Which kind a pasted checksum is, by its length.
pub fn kind_of(checksum: &str) -> Option<HashKind> {
    match clean_checksum(checksum).len() {
        64 => Some(HashKind::Sha256),
        40 => Some(HashKind::Sha1),
        128 => Some(HashKind::Sha512),
        _ => None,
    }
}

// ---------- shredding ----------

/// Overwrite a file with random bytes, give it a random name, empty it and
/// delete it; a folder, everything in it. Returns how many files went.
///
/// On a hard drive this leaves nothing to recover. SSDs (and their
/// wear-levelling) and copies elsewhere (backups, cloud folders, restore
/// points) are beyond any program's reach, which the page says.
pub fn shred(path: &Path, cancel: &AtomicBool) -> Result<usize, String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if meta.is_dir() {
        let mut n = 0;
        for e in walkdir::WalkDir::new(path).contents_first(true) {
            let e = e.map_err(|e| e.to_string())?;
            if cancel.load(Ordering::Relaxed) {
                return Err("Stopped.".into());
            }
            if e.file_type().is_dir() {
                std::fs::remove_dir(e.path()).map_err(|err| format!("{}: {err}", e.path().display()))?;
            } else {
                shred_file(e.path(), cancel)?;
                n += 1;
            }
        }
        Ok(n)
    } else {
        shred_file(path, cancel).map(|_| 1)
    }
}

fn shred_file(path: &Path, cancel: &AtomicBool) -> Result<(), String> {
    let why = |e: std::io::Error| format!("{}: {e}", path.display());
    let len = std::fs::metadata(path).map_err(why)?.len();
    // Read-only files are made writable first (they're being destroyed anyway).
    if let Ok(m) = std::fs::metadata(path) {
        let mut p = m.permissions();
        if p.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            p.set_readonly(false);
            let _ = std::fs::set_permissions(path, p);
        }
    }
    {
        let mut f = std::fs::OpenOptions::new().write(true).open(path).map_err(why)?;
        let mut buf = vec![0u8; 1 << 20];
        let mut left = len;
        while left > 0 {
            if cancel.load(Ordering::Relaxed) {
                return Err("Stopped.".into());
            }
            let n = left.min(buf.len() as u64) as usize;
            rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut buf[..n]);
            f.write_all(&buf[..n]).map_err(why)?;
            left -= n as u64;
        }
        f.sync_all().map_err(why)?;
        f.set_len(0).map_err(why)?;
        f.sync_all().map_err(why)?;
    }
    // A random name, so the old one isn't left in the folder's records either.
    let gone = path.with_file_name(uuid::Uuid::new_v4().simple().to_string());
    let target = if std::fs::rename(path, &gone).is_ok() { gone } else { path.to_path_buf() };
    std::fs::remove_file(&target).map_err(why)
}

// ---------- bulk rename ----------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Case {
    #[default]
    Keep,
    Lower,
    Upper,
    Title,
}

/// How to rename. `pattern` uses `{name}` (the old name without its
/// extension), `{n}` (a counter; `{n:3}` pads it to 3 digits), `{date}` and
/// `{time}` (when the file last changed) and `{ext}`; the extension is kept
/// unless the pattern has `{ext}` or a dot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RenameRule {
    pub pattern: String,
    pub find: String,
    pub replace: String,
    pub case: Case,
    pub start: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RenameItem {
    pub from: String,
    pub to: String,
    /// Why it can't be renamed ("" when it can).
    pub problem: String,
}

fn bad_name(n: &str) -> Option<&'static str> {
    if n.trim().is_empty() || n == "." || n == ".." {
        Some("empty name")
    } else if n.chars().any(|c| c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')) {
        Some("has a character Windows doesn't allow")
    } else if n.ends_with(' ') || n.ends_with('.') {
        Some("ends with a space or a dot")
    } else {
        None
    }
}

fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut up = true;
    for c in s.chars() {
        if up {
            out.extend(c.to_uppercase());
        } else {
            out.extend(c.to_lowercase());
        }
        up = !c.is_alphanumeric();
    }
    out
}

/// The new name of one file.
pub fn new_name(old: &str, rule: &RenameRule, index: u32, modified: Option<chrono::NaiveDateTime>) -> String {
    let p = Path::new(old);
    let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = p.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default();
    let mut base = if rule.pattern.trim().is_empty() { stem.clone() } else { rule.pattern.clone() };
    base = base.replace("{name}", &stem);
    let n = rule.start + index;
    // {n:3} → zero-padded.
    while let Some(i) = base.find("{n:") {
        let Some(j) = base[i..].find('}') else { break };
        let width: usize = base[i + 3..i + j].parse().unwrap_or(1).min(9);
        base.replace_range(i..i + j + 1, &format!("{n:0width$}"));
    }
    base = base.replace("{n}", &n.to_string());
    let when = modified.unwrap_or_default();
    base = base.replace("{date}", &when.format("%Y-%m-%d").to_string()).replace("{time}", &when.format("%H-%M-%S").to_string());
    let explicit_ext = base.contains("{ext}");
    base = base.replace("{ext}", &ext);
    if !rule.find.is_empty() {
        base = base.replace(&rule.find, &rule.replace);
    }
    base = match rule.case {
        Case::Keep => base,
        Case::Lower => base.to_lowercase(),
        Case::Upper => base.to_uppercase(),
        Case::Title => title_case(&base),
    };
    let pattern_has_dot = rule.pattern.contains('.');
    if explicit_ext || pattern_has_dot || ext.is_empty() {
        base
    } else {
        format!("{base}.{ext}")
    }
}

fn modified(path: &Path) -> Option<chrono::NaiveDateTime> {
    let t = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(chrono::DateTime::<chrono::Local>::from(t).naive_local())
}

/// What each file would be called, with any problem (a clash, a bad name).
pub fn rename_plan(files: &[String], rule: &RenameRule) -> Vec<RenameItem> {
    let mut plan: Vec<RenameItem> = files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let p = Path::new(f);
            let old = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let to = new_name(&old, rule, i as u32, modified(p));
            RenameItem { from: f.clone(), to, problem: String::new() }
        })
        .collect();
    let lower = |s: &str| s.to_lowercase();
    let targets: Vec<String> = plan.iter().map(|it| Path::new(&it.from).with_file_name(&it.to).to_string_lossy().into_owned()).collect();
    let sources: std::collections::HashSet<String> = files.iter().map(|f| lower(f)).collect();
    for i in 0..plan.len() {
        if let Some(why) = bad_name(&plan[i].to) {
            plan[i].problem = why.into();
        } else if targets.iter().enumerate().any(|(j, t)| j != i && lower(t) == lower(&targets[i])) {
            plan[i].problem = "two files would get this name".into();
        } else if Path::new(&targets[i]).exists() && !sources.contains(&lower(&targets[i])) {
            plan[i].problem = "a file with this name is already there".into();
        }
    }
    plan
}

/// Rename by the plan (only if nothing in it has a problem). Two steps, so
/// files can swap names. Returns how many were renamed.
pub fn rename_apply(files: &[String], rule: &RenameRule) -> Result<usize, String> {
    let plan = rename_plan(files, rule);
    if let Some(bad) = plan.iter().find(|p| !p.problem.is_empty()) {
        return Err(format!("{} → {}: {}", Path::new(&bad.from).file_name().unwrap_or_default().to_string_lossy(), bad.to, bad.problem));
    }
    let moves: Vec<(PathBuf, PathBuf, PathBuf)> = plan
        .iter()
        .filter(|p| Path::new(&p.from).file_name().map(|n| n.to_string_lossy() != p.to.as_str()).unwrap_or(true))
        .map(|p| {
            let from = PathBuf::from(&p.from);
            let tmp = from.with_file_name(format!(".omnihub-rename-{}", uuid::Uuid::new_v4().simple()));
            let to = from.with_file_name(&p.to);
            (from, tmp, to)
        })
        .collect();
    let mut staged: Vec<&(PathBuf, PathBuf, PathBuf)> = Vec::new();
    for m in &moves {
        if let Err(e) = std::fs::rename(&m.0, &m.1) {
            // Put back what already moved.
            for s in staged {
                let _ = std::fs::rename(&s.1, &s.0);
            }
            return Err(format!("{}: {e}", m.0.display()));
        }
        staged.push(m);
    }
    for m in &moves {
        std::fs::rename(&m.1, &m.2).map_err(|e| format!("{}: {e}", m.2.display()))?;
    }
    Ok(moves.len())
}

// ---------- recent files ----------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecentFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    /// When it was last opened (Unix seconds).
    pub used: i64,
}

/// The target of a Windows shortcut (`.lnk`), from its LinkInfo.
pub fn lnk_target(data: &[u8]) -> Option<String> {
    let u32_at = |o: usize| data.get(o..o + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()) as usize);
    let u16_at = |o: usize| data.get(o..o + 2).map(|b| u16::from_le_bytes(b.try_into().unwrap()) as usize);
    if u32_at(0)? != 0x4C {
        return None;
    }
    let flags = u32_at(0x14)?;
    let mut pos = 0x4C;
    if flags & 0x1 != 0 {
        pos += 2 + u16_at(pos)?;
    }
    if flags & 0x2 == 0 {
        return None;
    }
    let li = pos;
    let header = u32_at(li + 4)?;
    let li_flags = u32_at(li + 8)?;
    if li_flags & 0x1 == 0 {
        return None; // a network path only
    }
    let ansi = |o: usize| -> Option<String> {
        let start = li + o;
        let end = data.get(start..)?.iter().position(|&b| b == 0)? + start;
        Some(String::from_utf8_lossy(&data[start..end]).into_owned())
    };
    let wide = |o: usize| -> Option<String> {
        let start = li + o;
        let units: Vec<u16> = data.get(start..)?.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
        Some(String::from_utf16_lossy(&units))
    };
    let (base, suffix) = if header >= 0x24 {
        (wide(u32_at(li + 0x1C)?)?, wide(u32_at(li + 0x20)?).unwrap_or_default())
    } else {
        (ansi(u32_at(li + 0x10)?)?, ansi(u32_at(li + 0x18)?).unwrap_or_default())
    };
    let path = format!("{base}{suffix}");
    (!path.is_empty()).then_some(path)
}

/// Files (not folders) Windows lists as recently used, newest first.
pub fn recent_files(limit: usize) -> Vec<RecentFile> {
    let Some(dir) = dirs::data_dir().map(|d| d.join("Microsoft").join("Windows").join("Recent")) else { return Vec::new() };
    recent_in(&dir, limit)
}

fn recent_in(dir: &Path, limit: usize) -> Vec<RecentFile> {
    let mut links: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk"))).filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path()))).collect())
        .unwrap_or_default();
    links.sort_by_key(|(t, _)| std::cmp::Reverse(*t));
    let mut seen = std::collections::HashSet::new();
    links
        .into_iter()
        .filter_map(|(t, p)| {
            let target = lnk_target(&std::fs::read(&p).ok()?)?;
            let meta = std::fs::metadata(&target).ok().filter(|m| m.is_file())?;
            seen.insert(target.to_lowercase()).then(|| RecentFile {
                name: Path::new(&target).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                path: target,
                size: meta.len(),
                used: t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0),
            })
        })
        .take(limit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksums() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.bin");
        std::fs::write(&f, b"hello").unwrap();
        let no = AtomicBool::new(false);
        assert_eq!(hash_file(&f, HashKind::Sha256, &mut |_, _| {}, &no).unwrap(), "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
        assert_eq!(hash_file(&f, HashKind::Sha1, &mut |_, _| {}, &no).unwrap(), "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d");
        assert_eq!(clean_checksum("SHA256: 2CF2 4DBA"), "2cf24dba");
        assert_eq!(kind_of(&"a".repeat(64)), Some(HashKind::Sha256));
        assert_eq!(kind_of("abc"), None);
    }

    #[test]
    fn shredding() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("secret.txt");
        std::fs::write(&f, b"top secret").unwrap();
        let sub = dir.path().join("folder");
        std::fs::create_dir_all(sub.join("deeper")).unwrap();
        std::fs::write(sub.join("x"), b"1").unwrap();
        std::fs::write(sub.join("deeper").join("y"), b"2").unwrap();
        let no = AtomicBool::new(false);
        assert_eq!(shred(&f, &no).unwrap(), 1);
        assert!(!f.exists());
        assert_eq!(shred(&sub, &no).unwrap(), 2);
        assert!(!sub.exists());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0, "no renamed leftovers");
    }

    #[test]
    fn rename_patterns_and_plan() {
        let when = chrono::NaiveDate::from_ymd_opt(2026, 10, 30).unwrap().and_hms_opt(9, 5, 7);
        let r = |pattern: &str| RenameRule { pattern: pattern.into(), start: 1, ..Default::default() };
        assert_eq!(new_name("IMG_001.JPG", &r("Holiday {n:3}"), 0, when), "Holiday 001.JPG");
        assert_eq!(new_name("a.txt", &r("{date} {name}"), 1, when), "2026-10-30 a.txt");
        assert_eq!(new_name("a.txt", &r("{name}.md"), 0, when), "a.md", "a dot in the pattern sets the extension");
        assert_eq!(new_name("My File.PDF", &RenameRule { find: " ".into(), replace: "_".into(), case: Case::Lower, ..Default::default() }, 0, when), "my_file.PDF", "case changes the name, not the extension");
        assert_eq!(new_name("hello world.txt", &RenameRule { case: Case::Title, ..Default::default() }, 0, when), "Hello World.txt");

        let dir = tempfile::tempdir().unwrap();
        let files: Vec<String> = ["b.txt", "a.txt", "keep.txt"].iter().map(|n| {
            let p = dir.path().join(n);
            std::fs::write(&p, n).unwrap();
            p.to_string_lossy().into_owned()
        }).collect();
        // Swapping names works (two steps).
        let swap = RenameRule { pattern: "{name}".into(), find: "".into(), ..Default::default() };
        assert!(rename_plan(&files[..2], &swap).iter().all(|p| p.problem.is_empty()));
        let clash = rename_plan(&files[..2], &r("same"));
        assert!(clash.iter().all(|p| p.problem.contains("two files")));
        let taken = rename_plan(&files[..1], &r("keep"));
        assert!(taken[0].problem.contains("already there"));
        assert!(rename_plan(&files[..1], &r("bad?name"))[0].problem.contains("character"));
        assert!(rename_apply(&files[..2], &r("same")).is_err());
        assert_eq!(rename_apply(&files[..2], &r("file {n}")).unwrap(), 2);
        assert_eq!(std::fs::read_to_string(dir.path().join("file 1.txt")).unwrap(), "b.txt");
        assert_eq!(std::fs::read_to_string(dir.path().join("file 2.txt")).unwrap(), "a.txt");
    }

    /// A shortcut as Windows writes one (header + LinkInfo with a Unicode path).
    fn lnk(path: &str) -> Vec<u8> {
        let mut d = vec![0u8; 0x4C];
        d[..4].copy_from_slice(&0x4Cu32.to_le_bytes());
        d[0x14..0x18].copy_from_slice(&0x2u32.to_le_bytes()); // HasLinkInfo
        let wide: Vec<u8> = path.encode_utf16().chain([0]).flat_map(|u| u.to_le_bytes()).collect();
        let header = 0x24u32;
        let base_off = header as usize + 1; // an empty ANSI path first
        let wide_off = base_off + 1;
        let suffix_off = wide_off + wide.len();
        let mut li = vec![0u8; header as usize];
        li[4..8].copy_from_slice(&header.to_le_bytes());
        li[8..12].copy_from_slice(&1u32.to_le_bytes());
        li[0x10..0x14].copy_from_slice(&(base_off as u32).to_le_bytes());
        li[0x1C..0x20].copy_from_slice(&(wide_off as u32).to_le_bytes());
        li[0x20..0x24].copy_from_slice(&(suffix_off as u32).to_le_bytes());
        li.extend([0, 0]);
        li.extend(&wide);
        li.extend([0, 0]);
        let size = li.len() as u32;
        li[..4].copy_from_slice(&size.to_le_bytes());
        d.extend(li);
        d
    }

    #[test]
    fn recent_files_from_shortcuts() {
        let dir = tempfile::tempdir().unwrap();
        let doc = dir.path().join("Résumé.pdf");
        std::fs::write(&doc, b"pdf").unwrap();
        let recent = dir.path().join("Recent");
        std::fs::create_dir_all(&recent).unwrap();
        std::fs::write(recent.join("Résumé.pdf.lnk"), lnk(&doc.to_string_lossy())).unwrap();
        std::fs::write(recent.join("gone.lnk"), lnk("C:\\nowhere\\gone.txt")).unwrap();
        std::fs::write(recent.join("junk.lnk"), b"not a shortcut").unwrap();
        assert_eq!(lnk_target(&lnk("C:\\a\\b.txt")).as_deref(), Some("C:\\a\\b.txt"));
        let found = recent_in(&recent, 10);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Résumé.pdf");
    }
}
