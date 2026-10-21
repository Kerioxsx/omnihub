//! What a server runs beyond Minecraft itself: mods (Fabric, Quilt, Forge,
//! NeoForge), plugins (Paper) and data packs (any server), from Modrinth
//! (checked against Modrinth's hashes, with the projects they need), from
//! a modpack or one of the user's mixes (only the parts servers use), from
//! a file, or made by Claude. Switching one off moves it aside, so the
//! server doesn't load it, and back.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::install::Flavor;
use crate::mods::install::safe_rel;
use crate::mods::minecraft::{Modrinth, PackFile};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Mod,
    Plugin,
    Datapack,
}

impl Kind {
    /// Modrinth's project type.
    pub fn project_type(self) -> &'static str {
        match self {
            Kind::Mod => "mod",
            Kind::Plugin => "plugin",
            Kind::Datapack => "datapack",
        }
    }

    /// The folder it goes in, inside the server's folder.
    pub fn folder(self, level: &str) -> String {
        match self {
            Kind::Mod => "mods".into(),
            Kind::Plugin => "plugins".into(),
            Kind::Datapack => format!("{level}/datapacks"),
        }
    }

    /// Takes effect without a restart (`reload`).
    pub fn live(self) -> bool {
        self == Kind::Datapack
    }
}

/// What a server can take of each kind.
pub fn allowed(flavor: Flavor, k: Kind) -> bool {
    match k {
        Kind::Mod => flavor.mods(),
        Kind::Plugin => flavor.plugins(),
        Kind::Datapack => true,
    }
}

/// One thing a server runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    /// Its path in the server's folder ("mods/sodium.jar").
    pub file: String,
    pub kind: Kind,
    pub title: String,
    pub version: String,
    pub icon: Option<String>,
    pub project: Option<String>,
    /// modrinth, pack, mix, claude, file
    pub source: String,
    /// The pack, mix or Claude project it came from.
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub dependency: bool,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub sha1: String,
}

fn yes() -> bool {
    true
}

/// OmniHub's notes on a server's content: `.omnihub/content.json`.
fn record_file(dir: &Path) -> PathBuf {
    dir.join(".omnihub").join("content.json")
}

fn records(dir: &Path) -> Vec<Content> {
    std::fs::read(record_file(dir)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn keep(dir: &Path, v: &[Content]) -> Result<(), String> {
    let p = record_file(dir);
    std::fs::create_dir_all(p.parent().unwrap_or(dir)).map_err(|e| e.to_string())?;
    std::fs::write(&p, serde_json::to_vec_pretty(v).unwrap_or_default()).map_err(|e| e.to_string())
}

/// Where switched-off content waits.
fn aside(dir: &Path, file: &str) -> PathBuf {
    dir.join(".omnihub").join("disabled").join(file)
}

fn sha1_of(p: &Path) -> String {
    use sha1::Digest as _;
    let mut h = sha1::Sha1::new();
    if let Ok(mut f) = std::fs::File::open(p) {
        let mut buf = vec![0u8; 1 << 16];
        while let Ok(n) = f.read(&mut buf) {
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
        }
    }
    hex::encode(h.finalize())
}

fn size_of(p: &Path) -> u64 {
    if p.is_dir() {
        walkdir::WalkDir::new(p).into_iter().flatten().filter_map(|e| e.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum()
    } else {
        std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
    }
}

/// Everything the server runs (or has switched off), with what OmniHub knows of each.
pub fn list(dir: &Path, flavor: Flavor, level: &str) -> Vec<Content> {
    let recs = records(dir);
    let mut out: Vec<Content> = Vec::new();
    let mut seen = HashSet::new();
    for k in [Kind::Mod, Kind::Plugin, Kind::Datapack] {
        if !allowed(flavor, k) && k != Kind::Datapack {
            continue;
        }
        let folder = k.folder(level);
        for (base, enabled) in [(dir.join(&folder), true), (aside(dir, &folder), false)] {
            let Ok(rd) = std::fs::read_dir(&base) else { continue };
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                let p = e.path();
                let ok = match k {
                    Kind::Mod | Kind::Plugin => p.is_file() && name.to_lowercase().ends_with(".jar"),
                    Kind::Datapack => (p.is_file() && name.to_lowercase().ends_with(".zip")) || (p.is_dir() && p.join("pack.mcmeta").is_file()),
                };
                if !ok {
                    continue;
                }
                let file = format!("{folder}/{name}");
                if !seen.insert(file.clone()) {
                    continue;
                }
                let mut c = recs.iter().find(|r| r.file == file).cloned().unwrap_or_else(|| Content {
                    file: file.clone(),
                    kind: k,
                    title: name.trim_end_matches(".jar").trim_end_matches(".zip").to_string(),
                    version: String::new(),
                    icon: None,
                    project: None,
                    source: "file".into(),
                    from: None,
                    dependency: false,
                    enabled,
                    size: 0,
                    sha1: String::new(),
                });
                c.enabled = enabled;
                c.size = size_of(&p);
                out.push(c);
            }
        }
    }
    out.sort_by_key(|c| (c.kind as u8, c.title.to_lowercase()));
    out
}

/// Look up files OmniHub doesn't know on Modrinth (by hash), and remember what they are.
pub fn identify(dir: &Path, flavor: Flavor, level: &str, api: &Modrinth) -> Result<usize, String> {
    let all = list(dir, flavor, level);
    let unknown: Vec<(Content, String)> = all
        .into_iter()
        .filter(|c| c.project.is_none() && (c.kind != Kind::Datapack || c.file.ends_with(".zip")))
        .map(|c| {
            let p = if c.enabled { dir.join(&c.file) } else { aside(dir, &c.file) };
            let h = sha1_of(&p);
            (c, h)
        })
        .collect();
    if unknown.is_empty() {
        return Ok(0);
    }
    let found = api.identify(&unknown.iter().map(|(_, h)| h.clone()).collect::<Vec<_>>())?;
    let ids: Vec<String> = found.values().map(|(p, _)| p.clone()).collect::<HashSet<_>>().into_iter().collect();
    let meta = if ids.is_empty() { HashMap::new() } else { api.project_meta(&ids)? };
    let mut recs = records(dir);
    let mut n = 0;
    for (mut c, h) in unknown {
        let Some((project, number)) = found.get(&h) else { continue };
        let m = meta.get(project);
        c.project = Some(project.clone());
        c.version = number.clone();
        c.sha1 = h;
        if let Some(m) = m {
            c.title = m.title.clone();
            c.icon = m.icon.clone();
        }
        if c.source == "file" {
            c.source = "modrinth".into();
        }
        recs.retain(|r| r.file != c.file);
        recs.push(c);
        n += 1;
    }
    keep(dir, &recs)?;
    Ok(n)
}

/// Put a downloaded or made file in place and note it.
fn place(dir: &Path, c: &Content, bytes: Option<&[u8]>, from_path: Option<&Path>) -> Result<(), String> {
    let rel = safe_rel(&c.file).ok_or("That file name can't be used.")?;
    let out = dir.join(&rel);
    std::fs::create_dir_all(out.parent().unwrap_or(dir)).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(aside(dir, &rel));
    match (bytes, from_path) {
        (Some(b), _) => std::fs::write(&out, b).map_err(|e| e.to_string())?,
        (None, Some(src)) if src.is_dir() => {
            if out.exists() {
                std::fs::remove_dir_all(&out).map_err(|e| e.to_string())?;
            }
            crate::mods::install::unpack(src, &out)?;
        }
        (None, Some(src)) => {
            std::fs::copy(src, &out).map_err(|e| e.to_string())?;
        }
        (None, None) => return Err("Nothing to put in place.".into()),
    }
    let mut recs = records(dir);
    recs.retain(|r| r.file != c.file && !(c.project.is_some() && r.project == c.project && r.kind == c.kind));
    recs.push(c.clone());
    keep(dir, &recs)
}

/// Install a Modrinth project (and the projects it needs) on a server.
#[allow(clippy::too_many_arguments)]
pub fn install(dir: &Path, flavor: Flavor, mc: &str, level: &str, api: &Modrinth, kind: Kind, project: &str, progress: &mut dyn FnMut(&str)) -> Result<Vec<Content>, String> {
    if !allowed(flavor, kind) {
        return Err(match kind {
            Kind::Mod => format!("A {} server doesn't run mods. Make a Fabric, Forge or NeoForge server for mods.", flavor.label()),
            Kind::Plugin => format!("A {} server doesn't run plugins. Paper servers do.", flavor.label()),
            Kind::Datapack => unreachable!(),
        });
    }
    let loaders: Vec<&str> = match kind {
        Kind::Datapack => vec!["datapack"],
        _ => flavor.loaders().to_vec(),
    };
    let folder = kind.folder(level);
    let have: HashSet<String> = list(dir, flavor, level).into_iter().filter_map(|c| c.project).collect();
    let mut queue = vec![(project.to_string(), false)];
    let mut done: Vec<Content> = Vec::new();
    let mut visited = HashSet::new();
    while let Some((p, dep)) = queue.pop() {
        if !visited.insert(p.clone()) || visited.len() > 40 {
            continue;
        }
        if dep && have.contains(&p) {
            continue;
        }
        let Some(r) = api.newest(&p, &loaders, mc, &folder)? else {
            if dep {
                continue;
            }
            return Err(format!("It has no version for Minecraft {mc} on {}.", if kind == Kind::Datapack { "data packs".to_string() } else { flavor.label().to_string() }));
        };
        let meta = api.project_meta(std::slice::from_ref(&r.project))?.remove(&r.project);
        if kind == Kind::Mod && meta.as_ref().is_some_and(|m| m.server_side == "unsupported") {
            if dep {
                continue;
            }
            return Err("That mod only works in the game itself (it says servers don't use it).".into());
        }
        let title = meta.as_ref().map(|m| m.title.clone()).unwrap_or_else(|| p.clone());
        progress(&format!("Downloading {title} {}", r.number));
        let bytes = api.download(&r.file.urls, &r.file.sha1, &r.file.sha512)?;
        // A newer copy replaces the old file of the same project.
        for old in list(dir, flavor, level).into_iter().filter(|c| c.project.as_deref() == Some(r.project.as_str()) && c.file != r.file.path) {
            let _ = remove(dir, &old.file);
        }
        let c = Content { file: r.file.path.clone(), kind, title, version: r.number.clone(), icon: meta.and_then(|m| m.icon), project: Some(r.project.clone()), source: "modrinth".into(), from: None, dependency: dep, enabled: true, size: bytes.len() as u64, sha1: r.file.sha1.clone() };
        place(dir, &c, Some(&bytes), None)?;
        done.push(c);
        if kind != Kind::Datapack {
            for need in r.requires {
                queue.push((need, true));
            }
        }
    }
    Ok(done)
}

/// Add a file from this PC (a mod or plugin jar, or a data pack zip or folder).
pub fn add_file(dir: &Path, flavor: Flavor, level: &str, path: &Path) -> Result<Content, String> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("That has no name.")?;
    let low = name.to_lowercase();
    let kind = if path.is_dir() || low.ends_with(".zip") {
        let ok = if path.is_dir() { path.join("pack.mcmeta").is_file() } else { zip_has(path, "pack.mcmeta") };
        if !ok {
            return Err("That isn't a data pack (it has no pack.mcmeta).".into());
        }
        Kind::Datapack
    } else if low.ends_with(".jar") {
        if zip_has(path, "plugin.yml") || zip_has(path, "paper-plugin.yml") {
            Kind::Plugin
        } else {
            Kind::Mod
        }
    } else {
        return Err("Add a mod or plugin (.jar) or a data pack (.zip or folder).".into());
    };
    if !allowed(flavor, kind) {
        return Err(format!("A {} server can't use {}.", flavor.label(), match kind { Kind::Mod => "mods", Kind::Plugin => "plugins", Kind::Datapack => "data packs" }));
    }
    let c = Content { file: format!("{}/{name}", kind.folder(level)), kind, title: name.trim_end_matches(".jar").trim_end_matches(".zip").to_string(), version: String::new(), icon: None, project: None, source: "file".into(), from: None, dependency: false, enabled: true, size: size_of(path), sha1: if path.is_file() { sha1_of(path) } else { String::new() } };
    place(dir, &c, None, Some(path))?;
    Ok(c)
}

/// Put made content (Claude's data pack folder or plugin jar) on the server.
pub fn add_made(dir: &Path, level: &str, kind: Kind, src: &Path, name: &str, project: &str) -> Result<Content, String> {
    let slug = crate::mods::slug(name).to_lowercase();
    let file = match kind {
        Kind::Plugin => format!("plugins/omnihub-{slug}.jar"),
        _ => format!("{}/omnihub-{slug}", kind.folder(level)),
    };
    let c = Content { file, kind, title: name.to_string(), version: String::new(), icon: None, project: None, source: "claude".into(), from: Some(project.to_string()), dependency: false, enabled: true, size: size_of(src), sha1: String::new() };
    place(dir, &c, None, Some(src))?;
    Ok(c)
}

fn zip_has(p: &Path, name: &str) -> bool {
    std::fs::File::open(p).ok().and_then(|f| zip::ZipArchive::new(f).ok()).is_some_and(|mut z| z.by_name(name).is_ok())
}

/// A Fabric or Quilt mod that says it only runs in the game (not on servers).
pub fn client_only(jar: &[u8]) -> bool {
    let Ok(mut z) = zip::ZipArchive::new(std::io::Cursor::new(jar)) else { return false };
    let mut read = |n: &str| -> Option<Value> {
        let mut s = String::new();
        z.by_name(n).ok()?.take(1 << 20).read_to_string(&mut s).ok()?;
        serde_json::from_str(&s).ok()
    };
    if let Some(v) = read("fabric.mod.json") {
        return v["environment"] == "client";
    }
    if let Some(v) = read("quilt.mod.json") {
        return v["minecraft"]["environment"] == "client";
    }
    false
}

/// Switch content off (moved aside) or back on.
pub fn set_enabled(dir: &Path, file: &str, on: bool) -> Result<(), String> {
    let rel = safe_rel(file).ok_or("No such file.")?;
    let (from, to) = if on { (aside(dir, &rel), dir.join(&rel)) } else { (dir.join(&rel), aside(dir, &rel)) };
    if !from.exists() {
        return if to.exists() { Ok(()) } else { Err("That file is gone.".into()) };
    }
    std::fs::create_dir_all(to.parent().unwrap_or(dir)).map_err(|e| e.to_string())?;
    std::fs::rename(&from, &to).map_err(|e| format!("Couldn't move it (is the server using it?): {e}"))
}

/// Remove content for good.
pub fn remove(dir: &Path, file: &str) -> Result<(), String> {
    let rel = safe_rel(file).ok_or("No such file.")?;
    if !(rel.starts_with("mods/") || rel.starts_with("plugins/") || rel.contains("/datapacks/")) {
        return Err("Only mods, plugins and data packs can be removed here.".into());
    }
    for p in [dir.join(&rel), aside(dir, &rel)] {
        if p.is_dir() {
            std::fs::remove_dir_all(&p).map_err(|e| e.to_string())?;
        } else if p.is_file() {
            std::fs::remove_file(&p).map_err(|e| format!("Couldn't remove it (is the server using it?): {e}"))?;
        }
    }
    let mut recs = records(dir);
    recs.retain(|r| r.file != rel);
    keep(dir, &recs)
}

/// A pack's own files (configs and the like): path and bytes.
pub type Overrides = Vec<(String, Vec<u8>)>;

/// Folders of a pack that servers use (not shaders, resource packs or the game's options).
fn server_override(rel: &str) -> bool {
    let top = rel.split('/').next().unwrap_or(rel);
    !matches!(top, "resourcepacks" | "shaderpacks" | "screenshots" | "saves" | "options.txt" | "optionsof.txt" | "optionsshaders.txt" | "servers.dat" | "servers.dat_old" | "journeymap" | "xaero" | "XaeroWaypoints" | "emi.json") && !rel.ends_with(".mrpack")
}

/// Install pack files (the server-side ones) and overrides onto a server.
pub fn install_pack(dir: &Path, api: &Modrinth, files: &[PackFile], overrides: &[(String, Vec<u8>)], from: &str, source: &str, progress: &mut dyn FnMut(usize, usize, &str)) -> Result<(usize, usize), String> {
    let wanted: Vec<&PackFile> = files.iter().filter(|f| f.server && safe_rel(&f.path).is_some()).collect();
    let total = wanted.len();
    let mut skipped = files.len() - total;
    let mut recs = records(dir);
    let mut added = 0;
    for (i, f) in wanted.iter().enumerate() {
        let name = f.path.rsplit('/').next().unwrap_or(&f.path).to_string();
        progress(i, total, &name);
        let bytes = match (&f.local, f.urls.is_empty()) {
            (Some(p), true) => std::fs::read(p).map_err(|e| e.to_string())?,
            _ => api.download(&f.urls, &f.sha1, &f.sha512)?,
        };
        if f.path.starts_with("mods/") && client_only(&bytes) {
            skipped += 1;
            continue;
        }
        let rel = safe_rel(&f.path).unwrap_or_default();
        let out = dir.join(&rel);
        std::fs::create_dir_all(out.parent().unwrap_or(dir)).map_err(|e| e.to_string())?;
        std::fs::write(&out, &bytes).map_err(|e| e.to_string())?;
        let kind = if rel.starts_with("plugins/") { Kind::Plugin } else if rel.contains("datapacks/") { Kind::Datapack } else { Kind::Mod };
        recs.retain(|r| r.file != rel);
        recs.push(Content { file: rel, kind, title: name.trim_end_matches(".jar").to_string(), version: String::new(), icon: None, project: None, source: source.into(), from: Some(from.into()), dependency: false, enabled: true, size: bytes.len() as u64, sha1: f.sha1.clone() });
        added += 1;
    }
    for (rel, b) in overrides {
        let Some(rel) = safe_rel(rel).filter(|r| server_override(r)) else { continue };
        let out = dir.join(&rel);
        std::fs::create_dir_all(out.parent().unwrap_or(dir)).map_err(|e| e.to_string())?;
        std::fs::write(&out, b).map_err(|e| e.to_string())?;
    }
    keep(dir, &recs)?;
    progress(total, total, "");
    Ok((added, skipped))
}

/// A .mrpack's server-side files and overrides (overrides/ then server-overrides/, which win).
pub fn mrpack_parts(bytes: &[u8]) -> Result<(Value, Vec<PackFile>, Overrides), String> {
    let (index, _) = crate::mods::minecraft::read_mrpack(bytes)?;
    let files: Vec<PackFile> = serde_json::from_value::<Vec<Value>>(index["files"].clone())
        .unwrap_or_default()
        .iter()
        .filter_map(|f| {
            Some(PackFile {
                path: safe_rel(f["path"].as_str()?)?,
                sha1: f["hashes"]["sha1"].as_str().unwrap_or_default().into(),
                sha512: f["hashes"]["sha512"].as_str().unwrap_or_default().into(),
                size: f["fileSize"].as_u64().unwrap_or(0),
                urls: f["downloads"].as_array().into_iter().flatten().filter_map(|u| u.as_str().map(String::from)).collect(),
                client: f["env"]["client"].as_str() != Some("unsupported"),
                server: f["env"]["server"].as_str() != Some("unsupported"),
                local: None,
            })
        })
        .collect();
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut over: Overrides = Vec::new();
    let mut server_over: Overrides = Vec::new();
    let mut total = 0u64;
    for i in 0..z.len() {
        let mut f = z.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        let name = f.name().to_string();
        let (list, rest) = if let Some(r) = name.strip_prefix("overrides/") {
            (&mut over, r.to_string())
        } else if let Some(r) = name.strip_prefix("server-overrides/") {
            (&mut server_over, r.to_string())
        } else {
            continue;
        };
        let mut b = Vec::new();
        (&mut f).take(256 << 20).read_to_end(&mut b).map_err(|e| e.to_string())?;
        total += b.len() as u64;
        if total > 4 << 30 {
            return Err("That pack's files are far too big.".into());
        }
        list.push((rest, b));
    }
    over.extend(server_over);
    Ok((index, files, over))
}

/// Every file of a folder as (relative path, bytes), for overrides from a mix.
pub fn folder_files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    crate::mods::install::files_in(dir).into_iter().filter_map(|f| std::fs::read(dir.join(&f)).ok().map(|b| (f, b))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::tests::{serve, zip_of, Seen};
    use serde_json::json;

    fn sha(b: &[u8]) -> (String, String) {
        use sha1::Digest as _;
        (hex::encode(sha1::Sha1::digest(b)), hex::encode(sha2::Sha512::digest(b)))
    }

    #[test]
    fn mods_plugins_and_data_packs_from_modrinth() {
        let lithium = zip_of(&[("fabric.mod.json", br#"{"id":"lithium","environment":"*"}"#)]);
        let fapi = zip_of(&[("fabric.mod.json", br#"{"id":"fabric-api"}"#)]);
        let pack = zip_of(&[("pack.mcmeta", br#"{"pack":{"description":"x"}}"#)]);
        let base: std::sync::Arc<parking_lot::Mutex<String>> = Default::default();
        let b2 = base.clone();
        let (l2, f2, p2) = (lithium.clone(), fapi.clone(), pack.clone());
        let s = serve(Box::new(move |r: &Seen| {
            let b = b2.lock().clone();
            let ok = |v: Value| Some((200, "application/json", v.to_string().into_bytes()));
            let ver = |project: &str, num: &str, name: &str, bytes: &[u8], loaders: Value, deps: Value| {
                let (s1, s5) = sha(bytes);
                json!({ "project_id": project, "version_number": num, "game_versions": ["26.3"], "loaders": loaders, "files": [{ "hashes": { "sha1": s1, "sha512": s5 }, "url": format!("{b}/cdn/{name}"), "filename": name, "primary": true, "size": bytes.len() }], "dependencies": deps })
            };
            let decoded = percent_encoding::percent_decode_str(&r.path).decode_utf8_lossy().to_string();
            let p = decoded.as_str();
            if p.starts_with("/v2/project/gvQqBUqZ/version") {
                assert!(p.contains("fabric") && p.contains("26.3"), "{p}");
                return ok(json!([ver("gvQqBUqZ", "0.15.0", "lithium.jar", &l2, json!(["fabric"]), json!([{ "project_id": "P7dR8mSH", "dependency_type": "required" }, { "project_id": "opt", "dependency_type": "optional" }]))]));
            }
            if p.starts_with("/v2/project/P7dR8mSH/version") {
                return ok(json!([ver("P7dR8mSH", "0.130.0", "fabric-api.jar", &f2, json!(["fabric"]), json!([]))]));
            }
            if p.starts_with("/v2/project/clientonly/version") {
                return ok(json!([ver("clientonly", "1", "zoom.jar", b"zoom", json!(["fabric"]), json!([]))]));
            }
            if p.starts_with("/v2/project/terra/version") {
                assert!(p.contains("datapack"), "{p}");
                return ok(json!([ver("terra", "2.0", "terralith.zip", &p2, json!(["datapack"]), json!([]))]));
            }
            if p.starts_with("/v2/projects?") {
                return ok(json!([{ "id": "gvQqBUqZ", "title": "Lithium", "server_side": "optional" }, { "id": "P7dR8mSH", "title": "Fabric API", "server_side": "required" }, { "id": "clientonly", "title": "Zoom", "server_side": "unsupported" }, { "id": "terra", "title": "Terralith", "server_side": "required" }]));
            }
            if p == "/v2/version_files" {
                let (s1, _) = sha(&l2);
                return ok(json!({ s1: ver("gvQqBUqZ", "0.15.0", "lithium.jar", &l2, json!(["fabric"]), json!([])) }));
            }
            if let Some(n) = p.strip_prefix("/cdn/") {
                return Some((200, "application/java-archive", match n { "lithium.jar" => l2.clone(), "fabric-api.jar" => f2.clone(), "terralith.zip" => p2.clone(), _ => b"zoom".to_vec() }));
            }
            None
        }));
        *base.lock() = s.base.clone();
        let api = Modrinth::new(&s.base, &s.base, &s.base);
        let t = tempfile::tempdir().unwrap();
        let dir = t.path();
        let mut log = Vec::new();
        let got = install(dir, Flavor::Fabric, "26.3", "world", &api, Kind::Mod, "gvQqBUqZ", &mut |m| log.push(m.to_string())).unwrap();
        assert_eq!(got.iter().map(|c| (c.title.as_str(), c.dependency)).collect::<Vec<_>>(), [("Lithium", false), ("Fabric API", true)], "with what it needs, not the optional one");
        assert!(dir.join("mods/lithium.jar").is_file() && dir.join("mods/fabric-api.jar").is_file());
        assert!(install(dir, Flavor::Fabric, "26.3", "world", &api, Kind::Mod, "clientonly", &mut |_| {}).unwrap_err().contains("only works in the game"));
        assert!(install(dir, Flavor::Paper, "26.3", "world", &api, Kind::Mod, "gvQqBUqZ", &mut |_| {}).unwrap_err().contains("doesn't run mods"));
        install(dir, Flavor::Fabric, "26.3", "world", &api, Kind::Datapack, "terra", &mut |_| {}).unwrap();
        assert!(dir.join("world/datapacks/terralith.zip").is_file());
        let l = list(dir, Flavor::Fabric, "world");
        assert_eq!(l.len(), 3);
        assert!(l.iter().all(|c| c.enabled && c.source == "modrinth"));
        // Off and on again; removed.
        set_enabled(dir, "mods/lithium.jar", false).unwrap();
        assert!(!dir.join("mods/lithium.jar").exists());
        assert!(!list(dir, Flavor::Fabric, "world").iter().find(|c| c.file == "mods/lithium.jar").unwrap().enabled);
        set_enabled(dir, "mods/lithium.jar", true).unwrap();
        assert!(dir.join("mods/lithium.jar").is_file());
        remove(dir, "world/datapacks/terralith.zip").unwrap();
        assert!(!dir.join("world/datapacks/terralith.zip").exists());
        assert!(remove(dir, "server.properties").is_err());
        assert!(set_enabled(dir, "../outside.jar", false).is_err());
        // A jar dropped in by hand is looked up by its hash.
        std::fs::remove_file(record_file(dir)).unwrap();
        assert_eq!(identify(dir, Flavor::Fabric, "world", &api).unwrap(), 1);
        assert_eq!(list(dir, Flavor::Fabric, "world").iter().find(|c| c.file == "mods/lithium.jar").unwrap().title, "Lithium");
        // A plugin jar from a file goes in plugins/ on Paper only.
        let plug = t.path().join("Hello.jar");
        std::fs::write(&plug, zip_of(&[("plugin.yml", b"name: Hello\nmain: a.B\nversion: 1\n")])).unwrap();
        assert!(add_file(dir, Flavor::Fabric, "world", &plug).is_err());
        let c = add_file(dir, Flavor::Paper, "world", &plug).unwrap();
        assert_eq!((c.kind, c.file.as_str()), (Kind::Plugin, "plugins/Hello.jar"));
    }

    #[test]
    fn modpacks_bring_only_what_servers_use() {
        let shader = zip_of(&[("fabric.mod.json", br#"{"id":"iris","environment":"client"}"#)]);
        let common = zip_of(&[("fabric.mod.json", br#"{"id":"create"}"#)]);
        let (s1, s5) = sha(&common);
        let (i1, i5) = sha(&shader);
        let index = json!({ "formatVersion": 1, "game": "minecraft", "versionId": "1", "name": "Pack", "dependencies": { "minecraft": "26.3", "fabric-loader": "0.19.5" }, "files": [
            { "path": "mods/create.jar", "hashes": { "sha1": s1, "sha512": s5 }, "env": { "client": "required", "server": "required" }, "downloads": ["https://cdn.modrinth.com/create.jar"], "fileSize": 1 },
            { "path": "mods/menu.jar", "hashes": { "sha1": "x", "sha512": "y" }, "env": { "client": "required", "server": "unsupported" }, "downloads": ["https://cdn.modrinth.com/menu.jar"], "fileSize": 1 },
            { "path": "mods/iris.jar", "hashes": { "sha1": i1, "sha512": i5 }, "env": { "client": "required", "server": "required" }, "downloads": [], "fileSize": 1 },
        ] });
        let mr = zip_of(&[("modrinth.index.json", index.to_string().as_bytes()), ("overrides/config/create.toml", b"a=1"), ("overrides/options.txt", b"fov:90"), ("overrides/shaderpacks/x.zip", b"z"), ("server-overrides/config/create.toml", b"a=2")]);
        let (idx, mut files, over) = mrpack_parts(&mr).unwrap();
        assert_eq!(idx["dependencies"]["fabric-loader"], "0.19.5");
        assert_eq!(files.iter().filter(|f| f.server).count(), 2);
        // Pretend the downloads are local (no network in this test).
        let t = tempfile::tempdir().unwrap();
        for f in files.iter_mut() {
            let p = t.path().join(f.path.replace('/', "_"));
            std::fs::write(&p, if f.path.contains("iris") { &shader } else { &common }).unwrap();
            f.local = Some(p.to_string_lossy().into_owned());
            f.urls.clear();
        }
        let api = Modrinth::new("http://127.0.0.1:9", "http://127.0.0.1:9", "http://127.0.0.1:9");
        let dir = t.path().join("srv");
        let (added, skipped) = install_pack(&dir, &api, &files, &over, "Pack", "pack", &mut |_, _, _| {}).unwrap();
        assert_eq!((added, skipped), (1, 2), "the client-only menu and shader mods stay off the server");
        assert!(dir.join("mods/create.jar").is_file() && !dir.join("mods/iris.jar").exists());
        assert_eq!(std::fs::read(dir.join("config/create.toml")).unwrap(), b"a=2", "server-overrides win");
        assert!(!dir.join("options.txt").exists() && !dir.join("shaderpacks").exists());
        assert_eq!(list(&dir, Flavor::Fabric, "world")[0].from.as_deref(), Some("Pack"));
    }
}
