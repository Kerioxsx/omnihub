//! Putting a mod's files where its game loads them, and taking them away
//! again: unpacking (zip, 7z, a folder or a single file), working out each
//! file's place for the game's mod loader, and copying them in with every
//! game file they replace backed up first. Switching a mod off moves its
//! files aside and puts the originals back; removing it does the same and
//! forgets it. Nothing outside the game's folder (and its own mod folder,
//! such as Balatro's in AppData) is ever written.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Which folder a file is relative to.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Root {
    /// The game's install folder.
    Game,
    /// The game's own data folder for mods (Balatro: %AppData%\Balatro).
    Data,
}

/// Where a mod's files go, by the game's loader.
#[derive(Debug, Clone, PartialEq)]
pub enum Layout {
    /// Into the game folder as the archive has them.
    GameRoot,
    /// Into a folder inside the game folder.
    GameSub(String),
    /// Into the game folder exactly as given (no wrapping folder dropped).
    Exact,
    /// BepInEx's rules (as r2modman installs Thunderstore packages).
    BepInEx,
    /// MelonLoader: `Mods/<mod>/` (or flat `Mods/` for older games), `UserData/` kept.
    Melon { recursive: bool },
    /// Lovely (Balatro): `<data>/Mods/<mod>/`.
    Lovely,
    /// GTA V: ASI plugins and Script Hook files at the top, scripts in `scripts/`.
    Gta,
    /// Unreal: .pak files into `<paks>/~mods/<mod>/`, UE4SS Lua mods into `<binaries>/ue4ss/Mods/<mod>/`.
    Unreal { paks: String, binaries: Option<String> },
    /// A mod loader's own package: what's inside `folder` into the game folder,
    /// only files with these endings at the top when `only` is set.
    Loader { folder: String, only: Option<Vec<String>> },
}

/// One file to copy: from the unpacked mod (`src`, empty = an empty file) to `dest` under `root`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    pub src: String,
    pub root: Root,
    pub dest: String,
}

/// Folders a mod's install made (removed again with it).
pub type Created = Vec<(Root, String)>;

/// One file a mod put in place.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Deployed {
    pub root: Root,
    pub path: String,
    pub size: u64,
    /// A file was there before; it's kept in the mod's backup.
    #[serde(default)]
    pub replaced: bool,
}

/// The folders a game's mods go into.
#[derive(Debug, Clone, PartialEq)]
pub struct Dirs {
    pub game: PathBuf,
    pub data: Option<PathBuf>,
}

impl Dirs {
    pub fn root(&self, r: Root) -> Result<&Path, String> {
        match r {
            Root::Game => Ok(&self.game),
            Root::Data => self.data.as_deref().ok_or_else(|| "This game has no mod data folder.".to_string()),
        }
    }
}

const MAX_FILES: usize = 200_000;
const MAX_BYTES: u64 = 40 << 30;

/// A path inside a mod as plain parts, or None if it would leave its folder
/// (`..`, a drive, a device name) or is empty.
pub fn safe_rel(p: &str) -> Option<String> {
    const DEVICES: &[&str] = &["con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9"];
    let mut parts = Vec::new();
    for part in p.split(['/', '\\']) {
        match part {
            "" | "." => continue,
            ".." => return None,
            _ => {}
        }
        if part.contains(':') || part.chars().any(|c| c.is_control() || matches!(c, '<' | '>' | '"' | '|' | '?' | '*')) || part.ends_with(' ') || part.ends_with('.') {
            return None;
        }
        let stem = part.split('.').next().unwrap_or(part).to_ascii_lowercase();
        if DEVICES.contains(&stem.as_str()) {
            return None;
        }
        parts.push(part);
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// What kind of file it is, from its first bytes.
fn kind_of(path: &Path) -> std::io::Result<&'static str> {
    let mut head = [0u8; 8];
    let n = std::fs::File::open(path)?.read(&mut head)?;
    Ok(match &head[..n] {
        [b'P', b'K', 3, 4, ..] | [b'P', b'K', 5, 6, ..] => "zip",
        [b'7', b'z', 0xBC, 0xAF, 0x27, 0x1C, ..] => "7z",
        [b'R', b'a', b'r', b'!', ..] => "rar",
        _ => "file",
    })
}

/// Unpack `src` (a zip, a 7z, a folder, or one file) into the empty folder `dest`.
pub fn unpack(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    if src.is_dir() {
        return copy_tree(src, dest);
    }
    let mut count = 0usize;
    let mut bytes = 0u64;
    let mut room = |size: u64| -> Result<(), String> {
        count += 1;
        bytes += size;
        if count > MAX_FILES || bytes > MAX_BYTES {
            return Err("That archive is far too big to be a mod.".into());
        }
        Ok(())
    };
    match kind_of(src).map_err(|e| format!("Couldn't open it: {e}"))? {
        "zip" => {
            let mut z = zip::ZipArchive::new(std::fs::File::open(src).map_err(|e| e.to_string())?).map_err(|e| format!("That zip can't be read: {e}"))?;
            for i in 0..z.len() {
                let mut f = z.by_index(i).map_err(|e| format!("That zip can't be read: {e}"))?;
                if f.is_dir() || f.is_symlink() {
                    continue;
                }
                let Some(rel) = safe_rel(&f.name().replace('\\', "/")) else { return Err(format!("The archive has a file that would land outside the mod: {}", f.name())) };
                room(f.size())?;
                let out = dest.join(&rel);
                std::fs::create_dir_all(out.parent().unwrap_or(dest)).map_err(|e| e.to_string())?;
                let mut w = std::fs::File::create(&out).map_err(|e| e.to_string())?;
                std::io::copy(&mut (&mut f).take(MAX_BYTES), &mut w).map_err(|e| format!("Couldn't unpack {rel}: {e}"))?;
            }
            Ok(())
        }
        "7z" => {
            let mut r = sevenz_rust2::ArchiveReader::open(src, sevenz_rust2::Password::empty()).map_err(|e| format!("That 7z can't be read: {e}"))?;
            let mut problem: Option<String> = None;
            r.for_each_entries(|entry, reader| {
                if entry.is_directory() {
                    return Ok(true);
                }
                let Some(rel) = safe_rel(entry.name()) else {
                    problem = Some(format!("The archive has a file that would land outside the mod: {}", entry.name()));
                    return Ok(false);
                };
                if let Err(e) = room(entry.size()) {
                    problem = Some(e);
                    return Ok(false);
                }
                let out = dest.join(&rel);
                std::fs::create_dir_all(out.parent().unwrap_or(dest))?;
                let mut w = std::fs::File::create(&out)?;
                std::io::copy(&mut reader.take(MAX_BYTES), &mut w)?;
                Ok(true)
            })
            .map_err(|e| format!("That 7z can't be unpacked: {e}"))?;
            problem.map_or(Ok(()), Err)
        }
        "rar" => Err("RAR archives can't be opened by OmniHub. Extract it first (right-click → Extract All with 7-Zip or WinRAR), then add the folder.".into()),
        _ => {
            let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let rel = safe_rel(&name).ok_or("That file name can't be used.")?;
            std::fs::copy(src, dest.join(rel)).map(|_| ()).map_err(|e| e.to_string())
        }
    }
}

fn copy_tree(src: &Path, dest: &Path) -> Result<(), String> {
    let mut count = 0usize;
    for e in walkdir::WalkDir::new(src).follow_links(false) {
        let e = e.map_err(|e| e.to_string())?;
        if !e.file_type().is_file() {
            continue;
        }
        count += 1;
        if count > MAX_FILES {
            return Err("That folder is far too big to be a mod.".into());
        }
        let rel = e.path().strip_prefix(src).map_err(|e| e.to_string())?;
        let out = dest.join(rel);
        std::fs::create_dir_all(out.parent().unwrap_or(dest)).map_err(|e| e.to_string())?;
        std::fs::copy(e.path(), &out).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Every file under `dir`, as "a/b.txt", sorted.
pub fn files_in(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.path().strip_prefix(dir).ok().map(|p| p.to_string_lossy().replace('\\', "/")))
        .collect();
    v.sort();
    v
}

fn lower(s: &str) -> String {
    s.to_ascii_lowercase()
}

fn file_name(p: &str) -> &str {
    p.rsplit('/').next().unwrap_or(p)
}

/// Thunderstore's own files at the top of a package.
fn is_meta(p: &str) -> bool {
    !p.contains('/') && matches!(lower(p).as_str(), "manifest.json" | "icon.png" | "readme.md" | "changelog.md" | "license" | "license.md" | "license.txt")
}

/// Read-me files at the top of an archive.
fn is_readme(p: &str) -> bool {
    let l = lower(p);
    !p.contains('/') && (is_meta(p) || l.starts_with("readme") || l.ends_with(".url") || l == "credits.txt")
}

/// The one folder everything sits in ("MyMod-main/"), unless it's a folder
/// the game needs by name.
fn wrapper(files: &[&str], keep: &[&str]) -> Option<String> {
    let mut first: Option<&str> = None;
    for f in files {
        let (top, rest) = f.split_once('/')?;
        if rest.is_empty() {
            return None;
        }
        match first {
            None => first = Some(top),
            Some(t) if t.eq_ignore_ascii_case(top) => {}
            _ => return None,
        }
    }
    let top = first?;
    (!keep.iter().any(|k| k.eq_ignore_ascii_case(top))).then(|| format!("{top}/"))
}

fn strip<'a>(f: &'a str, prefix: &Option<String>) -> &'a str {
    match prefix {
        Some(p) if f.len() > p.len() && f[..p.len()].eq_ignore_ascii_case(p) => &f[p.len()..],
        _ => f,
    }
}

/// Folder names that mean something to some loader, never dropped as a wrapper.
const NAMED: &[&str] = &["bepinex", "plugins", "patchers", "core", "config", "monomod", "mods", "userdata", "userlibs", "scripts", "data", "content", "ue4ss", "melonloader", "lovely", "bin", "binaries", "win64"];

/// Where each of `files` (from the unpacked mod) goes. `name` is the mod's
/// folder name where the loader wants one.
pub fn plan(files: &[String], layout: &Layout, name: &str) -> Result<Vec<Planned>, String> {
    let name = safe_rel(name).filter(|n| !n.contains('/')).unwrap_or_else(|| "mod".into());
    let all: Vec<&str> = files.iter().map(String::as_str).collect();
    let mut out = Vec::new();
    let mut put = |src: &str, root: Root, dest: String| -> Result<(), String> {
        let dest = safe_rel(&dest).ok_or_else(|| format!("{src} can't be placed safely."))?;
        out.push(Planned { src: src.to_string(), root, dest });
        Ok(())
    };
    match layout {
        Layout::GameRoot | Layout::GameSub(_) => {
            let body: Vec<&str> = all.iter().copied().filter(|f| !is_readme(f)).collect();
            let w = wrapper(&body, NAMED);
            let sub = match layout {
                Layout::GameSub(s) => format!("{s}/"),
                _ => String::new(),
            };
            for f in body {
                put(f, Root::Game, format!("{sub}{}", strip(f, &w)))?;
            }
        }
        Layout::Exact => {
            for f in &all {
                put(f, Root::Game, f.to_string())?;
            }
        }
        Layout::BepInEx => {
            const OVERRIDE: &[&str] = &["plugins", "patchers", "core", "config", "monomod"];
            for f in &all {
                let parts: Vec<&str> = f.split('/').collect();
                let at = parts[..parts.len() - 1].iter().position(|p| OVERRIDE.iter().any(|o| o.eq_ignore_ascii_case(p)));
                let dest = match at {
                    Some(i) => {
                        let folder = lower(parts[i]);
                        let rest = parts[i + 1..].join("/");
                        if folder == "config" {
                            format!("BepInEx/config/{rest}")
                        } else {
                            format!("BepInEx/{folder}/{name}/{rest}")
                        }
                    }
                    None if lower(f).ends_with(".mm.dll") => format!("BepInEx/monomod/{name}/{}", file_name(f)),
                    None => format!("BepInEx/plugins/{name}/{}", file_name(f)),
                };
                put(f, Root::Game, dest)?;
            }
        }
        Layout::Melon { recursive } => {
            for f in &all {
                let l = lower(f);
                let dest = if let Some(rest) = l.strip_prefix("userdata/").map(|_| &f["userdata/".len()..]) {
                    format!("UserData/{rest}")
                } else if *recursive {
                    let rest = if l.starts_with("mods/") { &f[5..] } else { f };
                    format!("Mods/{name}/{rest}")
                } else if l.starts_with("plugins/") || l.starts_with("userlibs/") || l.starts_with("mods/") {
                    f.to_string()
                } else if l.ends_with(".dll") {
                    format!("Mods/{}", file_name(f))
                } else {
                    format!("Mods/{name}/{f}")
                };
                put(f, Root::Game, dest)?;
            }
        }
        Layout::Lovely => {
            if all.iter().any(|f| lower(f).starts_with("mods/")) {
                for f in all.iter().filter(|f| lower(f).starts_with("mods/")) {
                    put(f, Root::Data, format!("Mods/{}", &f[5..]))?;
                }
            } else {
                let body: Vec<&str> = all.iter().copied().filter(|f| !(lower(f).ends_with(".url"))).collect();
                let inner: Vec<&str> = body.iter().copied().filter(|f| !is_meta(f)).collect();
                let w = wrapper(&inner, &["lovely", "assets", "localization"]);
                for f in body {
                    put(f, Root::Data, format!("Mods/{name}/{}", strip(f, &w)))?;
                }
            }
        }
        Layout::Gta => {
            let body: Vec<&str> = all.iter().copied().filter(|f| !is_readme(f)).collect();
            let w = if body.iter().any(|f| lower(f) == "bin/scripthookv.dll") { Some("bin/".to_string()) } else { wrapper(&body, &["scripts", "mods", "bin", "plugins"]) };
            let inner: Vec<&str> = body.iter().map(|f| strip(f, &w)).collect();
            let asi_at_top = inner.iter().any(|f| !f.contains('/') && lower(f).ends_with(".asi"));
            const TOP_DLLS: &[&str] = &["scripthookv.dll", "dinput8.dll", "scripthookvdotnet2.dll", "scripthookvdotnet3.dll", "version.dll", "dsound.dll", "winmm.dll", "xinput1_4.dll"];
            for (f, i) in body.iter().zip(inner) {
                let l = lower(i);
                if w.as_deref() == Some("bin/") && !lower(f).starts_with("bin/") {
                    continue;
                }
                let script_file = [".cs", ".vb", ".dll", ".pdb", ".ini", ".xml", ".json"].iter().any(|x| l.ends_with(x));
                let dest = if !i.contains('/') && !asi_at_top && script_file && !TOP_DLLS.contains(&l.as_str()) && !l.starts_with("scripthookvdotnet") { format!("scripts/{i}") } else { i.to_string() };
                put(f, Root::Game, dest)?;
            }
        }
        Layout::Unreal { paks, binaries } => {
            let body: Vec<&str> = all.iter().copied().filter(|f| !is_readme(f)).collect();
            let lua_mod = body.iter().find_map(|f| {
                let l = lower(f);
                let i = l.find("scripts/main.lua")?;
                Some(f[..i].to_string())
            });
            for f in &body {
                let l = lower(f);
                if let (Some(prefix), Some(bin)) = (&lua_mod, binaries) {
                    if l.starts_with(&lower(prefix)) {
                        let rest = &f[prefix.len()..];
                        let folder = prefix.trim_end_matches('/').rsplit('/').next().filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("mods")).unwrap_or(&name).to_string();
                        put(f, Root::Game, format!("{bin}/ue4ss/Mods/{folder}/{rest}"))?;
                        continue;
                    }
                }
                if let Some(i) = l.find("content/paks/") {
                    put(f, Root::Game, format!("{paks}/{}", &f[i + "content/paks/".len()..]))?;
                } else if [".pak", ".ucas", ".utoc", ".sig"].iter().any(|x| l.ends_with(x)) {
                    put(f, Root::Game, format!("{paks}/~mods/{name}/{}", file_name(f)))?;
                } else if lua_mod.is_none() {
                    put(f, Root::Game, strip(f, &wrapper(&body, NAMED)).to_string())?;
                }
            }
            if let (Some(prefix), Some(bin)) = (&lua_mod, binaries) {
                let folder = prefix.trim_end_matches('/').rsplit('/').next().filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("mods")).unwrap_or(&name).to_string();
                if !body.iter().any(|f| lower(f) == lower(&format!("{prefix}enabled.txt"))) {
                    put("", Root::Game, format!("{bin}/ue4ss/Mods/{folder}/enabled.txt"))?;
                }
            }
        }
        Layout::Loader { folder, only } => {
            let prefix = if folder.is_empty() { String::new() } else { format!("{folder}/") };
            for f in &all {
                let Some(rest) = (if prefix.is_empty() { Some(*f) } else { f.get(prefix.len()..).filter(|_| lower(f).starts_with(&lower(&prefix))) }) else { continue };
                if is_meta(rest) || (prefix.is_empty() && is_meta(f)) {
                    continue;
                }
                if let Some(exts) = only {
                    if rest.contains('/') || !exts.iter().any(|x| lower(rest).ends_with(x.as_str())) {
                        continue;
                    }
                }
                put(f, Root::Game, rest.to_string())?;
            }
        }
    }
    if out.is_empty() {
        return Err("There's nothing in it to install.".into());
    }
    let mut seen = HashSet::new();
    if let Some(dup) = out.iter().find(|p| !seen.insert((p.root, lower(&p.dest)))) {
        return Err(format!("Two files in it would both become {}.", dup.dest));
    }
    Ok(out)
}

/// A file's place on disk.
fn at(dirs: &Dirs, root: Root, rel: &str) -> Result<PathBuf, String> {
    let rel = safe_rel(rel).ok_or("A path in this mod isn't safe.")?;
    Ok(dirs.root(root)?.join(rel))
}

fn store_path(dir: &Path, root: Root, rel: &str) -> PathBuf {
    dir.join(match root {
        Root::Game => "game",
        Root::Data => "data",
    })
    .join(rel)
}

fn busy(e: &std::io::Error, what: &str) -> String {
    if e.kind() == std::io::ErrorKind::PermissionDenied || e.raw_os_error() == Some(32) {
        format!("{what} is in use or protected. Close the game and try again.")
    } else {
        format!("{what}: {e}")
    }
}

/// Move a file (copying when it's on another drive).
fn move_file(from: &Path, to: &Path) -> std::io::Result<()> {
    if let Some(p) = to.parent() {
        std::fs::create_dir_all(p)?;
    }
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)?;
    std::fs::remove_file(from)
}

/// Folders that `path`'s parent needs that aren't there yet, top first, as `root`-relative.
fn missing_dirs(root_dir: &Path, rel: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = Path::new(rel).parent();
    while let Some(p) = cur.filter(|p| !p.as_os_str().is_empty()) {
        if root_dir.join(p).is_dir() {
            break;
        }
        out.push(p.to_string_lossy().replace('\\', "/"));
        cur = p.parent();
    }
    out.reverse();
    out
}

/// Copy the planned files in. A file already there is backed up in
/// `backup` first, unless `owner(root, path)` says another mod put it there
/// (then nothing changes and that mod is named). On any failure, everything
/// done so far is undone.
pub fn deploy(staging: &Path, planned: &[Planned], dirs: &Dirs, backup: &Path, owner: &dyn Fn(Root, &str) -> Option<String>) -> Result<(Vec<Deployed>, Created), String> {
    for p in planned {
        if let Some(other) = owner(p.root, &p.dest) {
            return Err(format!("{} already comes from {other}. Remove or switch off {other} first.", p.dest));
        }
    }
    let mut done: Vec<Deployed> = Vec::new();
    let mut created: Vec<(Root, String)> = Vec::new();
    let res = (|| -> Result<(), String> {
        for p in planned {
            let dest = at(dirs, p.root, &p.dest)?;
            for d in missing_dirs(dirs.root(p.root)?, &p.dest) {
                created.push((p.root, d));
            }
            let replaced = dest.is_file();
            if replaced {
                let b = store_path(backup, p.root, &p.dest);
                std::fs::create_dir_all(b.parent().unwrap_or(backup)).map_err(|e| e.to_string())?;
                std::fs::copy(&dest, &b).map_err(|e| busy(&e, &p.dest))?;
            }
            std::fs::create_dir_all(dest.parent().unwrap_or(&dest)).map_err(|e| busy(&e, &p.dest))?;
            let tmp = dest.with_file_name(format!("{}.omnihub-new", file_name(&p.dest)));
            if p.src.is_empty() {
                std::fs::write(&tmp, b"").map_err(|e| busy(&e, &p.dest))?;
            } else {
                std::fs::copy(staging.join(&p.src), &tmp).map_err(|e| busy(&e, &p.dest))?;
            }
            if let Err(e) = std::fs::rename(&tmp, &dest) {
                let _ = std::fs::remove_file(&tmp);
                return Err(busy(&e, &p.dest));
            }
            let size = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
            done.push(Deployed { root: p.root, path: p.dest.clone(), size, replaced });
        }
        Ok(())
    })();
    match res {
        Ok(()) => Ok((done, created)),
        Err(e) => {
            let _ = remove(&done, &created, dirs, backup, true);
            Err(e)
        }
    }
}

/// Take a mod's files away, putting back what they replaced. `enabled`: the
/// files are in place (otherwise they're in the switched-off store already
/// cleared by the caller).
pub fn remove(files: &[Deployed], created: &[(Root, String)], dirs: &Dirs, backup: &Path, enabled: bool) -> Result<(), String> {
    let mut first_err = None;
    if enabled {
        for f in files.iter().rev() {
            let Ok(dest) = at(dirs, f.root, &f.path) else { continue };
            if dest.is_file() {
                if let Err(e) = std::fs::remove_file(&dest) {
                    first_err.get_or_insert(busy(&e, &f.path));
                    continue;
                }
            }
            if f.replaced {
                let b = store_path(backup, f.root, &f.path);
                if b.is_file() {
                    if let Err(e) = std::fs::copy(&b, &dest) {
                        first_err.get_or_insert(busy(&e, &f.path));
                    }
                }
            }
        }
    }
    for (root, d) in created.iter().rev() {
        if let Ok(p) = at(dirs, *root, d) {
            let _ = std::fs::remove_dir(p);
        }
    }
    match first_err {
        Some(e) => Err(e),
        None => {
            let _ = std::fs::remove_dir_all(backup);
            Ok(())
        }
    }
}

/// Switch a mod off: its files go into `aside`, the originals come back.
pub fn disable(files: &[Deployed], dirs: &Dirs, backup: &Path, aside: &Path) -> Result<(), String> {
    for f in files {
        let dest = at(dirs, f.root, &f.path)?;
        if dest.is_file() {
            move_file(&dest, &store_path(aside, f.root, &f.path)).map_err(|e| busy(&e, &f.path))?;
        }
        if f.replaced {
            let b = store_path(backup, f.root, &f.path);
            if b.is_file() {
                std::fs::copy(&b, &dest).map_err(|e| busy(&e, &f.path))?;
            }
        }
    }
    Ok(())
}

/// Switch a mod back on. A file that appeared in its place meanwhile is
/// backed up first (and the list says so).
pub fn enable(files: &mut [Deployed], dirs: &Dirs, backup: &Path, aside: &Path, owner: &dyn Fn(Root, &str) -> Option<String>) -> Result<(), String> {
    for f in files.iter() {
        if let Some(other) = owner(f.root, &f.path) {
            return Err(format!("{} now comes from {other}. Switch off {other} first.", f.path));
        }
    }
    for f in files.iter_mut() {
        let dest = at(dirs, f.root, &f.path)?;
        let kept = store_path(aside, f.root, &f.path);
        if !kept.is_file() {
            continue;
        }
        if dest.is_file() && !f.replaced {
            let b = store_path(backup, f.root, &f.path);
            std::fs::create_dir_all(b.parent().unwrap_or(backup)).map_err(|e| e.to_string())?;
            std::fs::copy(&dest, &b).map_err(|e| busy(&e, &f.path))?;
            f.replaced = true;
        }
        move_file(&kept, &dest).map_err(|e| busy(&e, &f.path))?;
    }
    let _ = std::fs::remove_dir_all(aside);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn plan_of(files: &[&str], layout: &Layout, name: &str) -> Vec<(Root, String)> {
        let v: Vec<String> = files.iter().map(|s| s.to_string()).collect();
        plan(&v, layout, name).unwrap().into_iter().map(|p| (p.root, p.dest)).collect()
    }

    fn dests(v: Vec<(Root, String)>) -> Vec<String> {
        v.into_iter().map(|(_, d)| d).collect()
    }

    #[test]
    fn paths_stay_inside() {
        assert_eq!(safe_rel("a\\b/./c.txt").as_deref(), Some("a/b/c.txt"));
        for bad in ["../x", "a/../../x", "C:/Windows/x", "a/CON.txt", "a/nul", "", "/", "a/b.", "x|y"] {
            assert_eq!(safe_rel(bad), None, "{bad}");
        }
    }

    #[test]
    fn layouts_follow_each_loader() {
        // BepInEx, as r2modman does it.
        let b = dests(plan_of(&["manifest.json", "icon.png", "MoreCompany.dll", "lib/sub/Helper.dll", "x.mm.dll", "BepInEx/plugins/Assets/a.bundle", "config/More.cfg", "patchers/P.dll"], &Layout::BepInEx, "notnotnotswipez-MoreCompany"));
        assert_eq!(
            b,
            [
                "BepInEx/plugins/notnotnotswipez-MoreCompany/manifest.json",
                "BepInEx/plugins/notnotnotswipez-MoreCompany/icon.png",
                "BepInEx/plugins/notnotnotswipez-MoreCompany/MoreCompany.dll",
                "BepInEx/plugins/notnotnotswipez-MoreCompany/Helper.dll",
                "BepInEx/monomod/notnotnotswipez-MoreCompany/x.mm.dll",
                "BepInEx/plugins/notnotnotswipez-MoreCompany/Assets/a.bundle",
                "BepInEx/config/More.cfg",
                "BepInEx/patchers/notnotnotswipez-MoreCompany/P.dll"
            ]
        );
        // MelonLoader 0.7 (folders per mod) and older games (flat).
        assert_eq!(dests(plan_of(&["Mods/Cool.dll", "UserData/Cool.cfg", "Plugins/P.dll"], &Layout::Melon { recursive: true }, "a-Cool")), ["Mods/a-Cool/Cool.dll", "UserData/Cool.cfg", "Mods/a-Cool/Plugins/P.dll"]);
        assert_eq!(dests(plan_of(&["Cool.dll", "UserLibs/L.dll"], &Layout::Melon { recursive: false }, "a-Cool")), ["Mods/Cool.dll", "UserLibs/L.dll"]);
        // Lovely: into the data folder, the archive's own folder dropped, patches kept.
        let l = plan_of(&["manifest.json", "Cryptid-main/Cryptid.lua", "Cryptid-main/lovely/patch.toml", "Cryptid-main/assets/1x/j.png"], &Layout::Lovely, "Cryptid");
        assert!(l.iter().all(|(r, _)| *r == Root::Data));
        assert_eq!(dests(l), ["Mods/Cryptid/manifest.json", "Mods/Cryptid/Cryptid.lua", "Mods/Cryptid/lovely/patch.toml", "Mods/Cryptid/assets/1x/j.png"]);
        assert_eq!(dests(plan_of(&["Mods/Talisman/talisman.lua", "readme.txt"], &Layout::Lovely, "x")), ["Mods/Talisman/talisman.lua"]);
        // GTA V: Script Hook V's own zip, an ASI pack, a script mod, and loose scripts.
        assert_eq!(dests(plan_of(&["bin/ScriptHookV.dll", "bin/dinput8.dll", "bin/NativeTrainer.asi", "readme.txt"], &Layout::Gta, "shv")), ["ScriptHookV.dll", "dinput8.dll", "NativeTrainer.asi"]);
        assert_eq!(dests(plan_of(&["Trainer/Trainer.asi", "Trainer/Trainer.ini"], &Layout::Gta, "t")), ["Trainer.asi", "Trainer.ini"]);
        assert_eq!(dests(plan_of(&["scripts/Speedo.dll", "scripts/Speedo.ini", "Readme.txt"], &Layout::Gta, "s")), ["scripts/Speedo.dll", "scripts/Speedo.ini"]);
        assert_eq!(dests(plan_of(&["SlowMo.cs", "SlowMo.ini", "ScriptHookVDotNet3.dll"], &Layout::Gta, "s")), ["scripts/SlowMo.cs", "scripts/SlowMo.ini", "ScriptHookVDotNet3.dll"]);
        // Unreal: pak mods into ~mods, a UE4SS Lua mod into ue4ss/Mods (switched on).
        let u = Layout::Unreal { paks: "Pal/Content/Paks".into(), binaries: Some("Pal/Binaries/Win64".into()) };
        assert_eq!(dests(plan_of(&["Better_P.pak", "Better_P.utoc", "Better_P.ucas"], &u, "Better")), ["Pal/Content/Paks/~mods/Better/Better_P.pak", "Pal/Content/Paks/~mods/Better/Better_P.utoc", "Pal/Content/Paks/~mods/Better/Better_P.ucas"]);
        assert_eq!(dests(plan_of(&["FastTravel/Scripts/main.lua", "FastTravel/config.lua"], &u, "x")), ["Pal/Binaries/Win64/ue4ss/Mods/FastTravel/Scripts/main.lua", "Pal/Binaries/Win64/ue4ss/Mods/FastTravel/config.lua", "Pal/Binaries/Win64/ue4ss/Mods/FastTravel/enabled.txt"]);
        assert_eq!(dests(plan_of(&["Pal/Content/Paks/LogicMods/L.pak"], &u, "x")), ["Pal/Content/Paks/LogicMods/L.pak"]);
        // Loaders' own packages.
        assert_eq!(dests(plan_of(&["manifest.json", "icon.png", "BepInExPack/winhttp.dll", "BepInExPack/BepInEx/core/BepInEx.dll", "BepInExPack/doorstop_config.ini"], &Layout::Loader { folder: "BepInExPack".into(), only: None }, "x")), ["winhttp.dll", "BepInEx/core/BepInEx.dll", "doorstop_config.ini"]);
        assert_eq!(dests(plan_of(&["winmm.dll", "lovely/config.toml", "README.md", "CHANGELOG.md"], &Layout::Loader { folder: String::new(), only: Some(vec![".dll".into()]) }, "x")), ["winmm.dll"]);
        // Plain files into the game folder, or a folder in it.
        assert_eq!(dests(plan_of(&["Better Textures v2/Data/a.dds", "Better Textures v2/readme.txt"], &Layout::GameRoot, "x")), ["Data/a.dds", "readme.txt"]);
        assert_eq!(dests(plan_of(&["a.pck"], &Layout::GameSub("mods".into()), "x")), ["mods/a.pck"]);
        // Clashes and empty mods are refused.
        assert!(plan(&["A.dll".into(), "sub/a.dll".into()], &Layout::BepInEx, "m").unwrap_err().contains("both become"));
        assert!(plan(&["readme.txt".into()], &Layout::GameRoot, "m").is_err());
    }

    #[test]
    fn deploy_switch_off_on_and_remove_keep_the_game_intact() {
        let t = tempfile::tempdir().unwrap();
        let game = t.path().join("game");
        let data = t.path().join("data");
        std::fs::create_dir_all(game.join("BepInEx")).unwrap();
        std::fs::write(game.join("winhttp.dll"), b"original").unwrap();
        let staging = t.path().join("staging");
        std::fs::create_dir_all(staging.join("p")).unwrap();
        std::fs::write(staging.join("winhttp.dll"), b"modded").unwrap();
        std::fs::write(staging.join("p/Mod.dll"), b"mod").unwrap();
        let dirs = Dirs { game: game.clone(), data: Some(data.clone()) };
        let planned = vec![
            Planned { src: "winhttp.dll".into(), root: Root::Game, dest: "winhttp.dll".into() },
            Planned { src: "p/Mod.dll".into(), root: Root::Game, dest: "BepInEx/plugins/x/Mod.dll".into() },
            Planned { src: String::new(), root: Root::Data, dest: "Mods/x/enabled.txt".into() },
        ];
        let backup = t.path().join("backup");
        let aside = t.path().join("aside");
        let nobody = |_: Root, _: &str| None;
        let (mut files, created) = deploy(&staging, &planned, &dirs, &backup, &nobody).unwrap();
        assert_eq!(std::fs::read(game.join("winhttp.dll")).unwrap(), b"modded");
        assert!(files[0].replaced && !files[1].replaced);
        assert_eq!(created, [(Root::Game, "BepInEx/plugins".to_string()), (Root::Game, "BepInEx/plugins/x".to_string()), (Root::Data, "Mods".to_string()), (Root::Data, "Mods/x".to_string())]);
        assert!(data.join("Mods/x/enabled.txt").is_file());

        disable(&files, &dirs, &backup, &aside).unwrap();
        assert_eq!(std::fs::read(game.join("winhttp.dll")).unwrap(), b"original", "the original is back while it's off");
        assert!(!game.join("BepInEx/plugins/x/Mod.dll").exists());
        enable(&mut files, &dirs, &backup, &aside, &nobody).unwrap();
        assert_eq!(std::fs::read(game.join("BepInEx/plugins/x/Mod.dll")).unwrap(), b"mod");
        assert_eq!(std::fs::read(game.join("winhttp.dll")).unwrap(), b"modded");

        // Another mod's file is never overwritten.
        let other = |_: Root, p: &str| (p == "winhttp.dll").then(|| "BepInExPack".to_string());
        assert!(deploy(&staging, &planned[..1], &dirs, &t.path().join("b2"), &other).unwrap_err().contains("BepInExPack"));

        remove(&files, &created, &dirs, &backup, true).unwrap();
        assert_eq!(std::fs::read(game.join("winhttp.dll")).unwrap(), b"original");
        assert!(!game.join("BepInEx/plugins").exists(), "folders it made are gone");
        assert!(game.join("BepInEx").is_dir(), "folders that were there stay");
        assert!(!data.join("Mods").exists());
        assert!(!backup.exists());

        // A failure half-way undoes the files already copied.
        let broken = vec![planned[0].clone(), Planned { src: "missing.dll".into(), root: Root::Game, dest: "x.dll".into() }];
        assert!(deploy(&staging, &broken, &dirs, &backup, &nobody).is_err());
        assert_eq!(std::fs::read(game.join("winhttp.dll")).unwrap(), b"original");
    }

    #[test]
    fn unpacks_zip_7z_folders_and_single_files() {
        let t = tempfile::tempdir().unwrap();
        let zp = t.path().join("m.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&zp).unwrap());
            let o = zip::write::SimpleFileOptions::default();
            z.add_directory("Mod/", o).unwrap();
            z.start_file("Mod/a.lua", o).unwrap();
            z.write_all(b"print(1)").unwrap();
            z.start_file("Mod\\sub\\b.txt", o).unwrap();
            z.write_all(b"b").unwrap();
            z.finish().unwrap();
        }
        let out = t.path().join("out");
        unpack(&zp, &out).unwrap();
        assert_eq!(files_in(&out), ["Mod/a.lua", "Mod/sub/b.txt"]);

        let evil = t.path().join("evil.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&evil).unwrap());
            z.start_file("../../escape.txt", zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(b"x").unwrap();
            z.finish().unwrap();
        }
        assert!(unpack(&evil, &t.path().join("e")).unwrap_err().contains("outside"));
        assert!(!t.path().join("escape.txt").exists());

        let src = t.path().join("src7");
        std::fs::create_dir_all(src.join("pak")).unwrap();
        std::fs::write(src.join("pak/x.pak"), vec![7u8; 5000]).unwrap();
        let sz = t.path().join("m.7z");
        sevenz_rust2::compress_to_path(&src, &sz).unwrap();
        let o7 = t.path().join("o7");
        unpack(&sz, &o7).unwrap();
        assert_eq!(files_in(&o7), ["pak/x.pak"]);
        assert_eq!(std::fs::read(o7.join("pak/x.pak")).unwrap(), vec![7u8; 5000]);

        let rar = t.path().join("m.rar");
        std::fs::write(&rar, b"Rar!\x1a\x07\x01\x00junk").unwrap();
        assert!(unpack(&rar, &t.path().join("r")).unwrap_err().contains("RAR"));

        let one = t.path().join("Speedo.asi");
        std::fs::write(&one, b"MZ").unwrap();
        let o1 = t.path().join("o1");
        unpack(&one, &o1).unwrap();
        assert_eq!(files_in(&o1), ["Speedo.asi"]);
        let of = t.path().join("of");
        unpack(&src, &of).unwrap();
        assert_eq!(files_in(&of), ["pak/x.pak"]);
    }
}
