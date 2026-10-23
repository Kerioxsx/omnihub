//! Minecraft (Java) modpacks, mixed: take two or more packs (from Modrinth,
//! a .mrpack file, or an installed instance's folder), choose one Minecraft
//! version and mod loader, and get one pack with every mod once — each at a
//! version made for that Minecraft and loader (found on Modrinth by the
//! file's hash), with the dependencies they need and with mods that refuse
//! to run together sorted out. Configs from the packs are merged (the first
//! pack wins a clash). The mix is saved as a .mrpack (Modrinth's format,
//! which Modrinth App, Prism Launcher and others open), or, for Fabric and
//! Quilt, installed straight into the official Minecraft Launcher as a new
//! profile with its own folder.
//!
//! Every file downloaded is checked against the hashes Modrinth publishes.

use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::launchers;

pub const MODRINTH: &str = "https://api.modrinth.com";
pub const FABRIC_META: &str = "https://meta.fabricmc.net";
pub const QUILT_META: &str = "https://meta.quiltmc.org";

fn yes() -> bool {
    true
}

/// Memory to give a modded Minecraft (GB): 4, 6 or 8 by the PC's RAM.
pub fn memory_for_pc() -> u32 {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let gb = sys.total_memory() / (1 << 30);
    if gb >= 24 {
        8
    } else if gb >= 12 {
        6
    } else {
        4
    }
}

/// Hosts a .mrpack may download from (Modrinth's own list).
const HOSTS: &[&str] = &["https://cdn.modrinth.com/", "https://github.com/", "https://raw.githubusercontent.com/", "https://gitlab.com/"];

/// Minecraft versions compared part by part ("1.21.10" > "1.21.9", "26.1" > "1.21.4").
pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let p = |s: &str| s.split(['.', '-', '+']).map(|x| x.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
    p(a).cmp(&p(b))
}

fn loader_key(loader: &str) -> Option<&'static str> {
    Some(match loader {
        "fabric" => "fabric-loader",
        "quilt" => "quilt-loader",
        "forge" => "forge",
        "neoforge" => "neoforge",
        _ => return None,
    })
}

/// One file of a pack.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackFile {
    /// "mods/sodium.jar"
    pub path: String,
    pub sha1: String,
    pub sha512: String,
    pub size: u64,
    pub urls: Vec<String>,
    /// The client needs it (false: server-only).
    pub client: bool,
    /// Servers use it (false: client-only, left off servers).
    #[serde(default = "yes")]
    pub server: bool,
    /// A file on this PC (from an instance folder) rather than a download.
    #[serde(default)]
    pub local: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SourceKind {
    Modrinth { project: String, version: String },
    File { path: String },
    Folder { path: String },
    /// A modpack another launcher installed ("Prism Launcher", its instance folder).
    Launcher { launcher: String, path: String },
}

/// A pack added to a mix.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub name: String,
    pub kind: SourceKind,
    pub icon: Option<String>,
    pub mc: String,
    pub loader: String,
    pub loader_version: String,
    pub files: Vec<PackFile>,
    /// How many config and other files it brings.
    pub overrides: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EntryStatus {
    /// Already made for the chosen version and loader.
    Kept,
    /// Moved to the version made for the chosen Minecraft and loader.
    Updated,
    /// Needed by another mod.
    Dependency,
    /// Kept as it is: not on Modrinth, but the pack it's from uses the same version.
    AsIs,
    /// Left out: no version for the chosen Minecraft and loader.
    Missing,
    /// Left out: another mod in the mix refuses to run with it.
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub project: Option<String>,
    pub title: String,
    pub icon: Option<String>,
    /// mods, resourcepacks, shaderpacks…
    pub folder: String,
    pub version: String,
    pub file: Option<PackFile>,
    /// The packs it came from.
    pub from: Vec<String>,
    pub status: EntryStatus,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Mix {
    pub id: String,
    pub name: String,
    pub mc: String,
    pub loader: String,
    pub loader_version: String,
    pub sources: Vec<String>,
    pub entries: Vec<Entry>,
    /// Config files two packs both had ("config/x.toml: kept A's").
    pub clashes: Vec<String>,
    pub problems: Vec<String>,
    /// It can go straight into the Minecraft Launcher (Fabric, Quilt).
    pub installable: bool,
    /// The packs' ids, in the same order as `sources`.
    #[serde(default)]
    pub source_ids: Vec<String>,
    /// How its cover was made: `blend` (here) or `ai` (Pollinations).
    #[serde(default)]
    pub cover: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackHit {
    pub project: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon: Option<String>,
    pub downloads: u64,
    pub versions: Vec<String>,
    pub loaders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackVersion {
    pub id: String,
    pub name: String,
    pub number: String,
    pub mc: Vec<String>,
    pub loaders: Vec<String>,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    pub name: String,
    pub profile: String,
    pub dir: String,
    pub version_id: String,
    pub mc: String,
    pub loader: String,
    pub mods: usize,
    pub at: i64,
}

/// A Modrinth version, the parts used here.
#[derive(Debug, Clone, Deserialize)]
struct MrVersion {
    #[serde(default)]
    project_id: String,
    #[serde(default)]
    version_number: String,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    loaders: Vec<String>,
    #[serde(default)]
    date_published: String,
    #[serde(default)]
    files: Vec<MrFile>,
    #[serde(default)]
    dependencies: Vec<MrDep>,
}

#[derive(Debug, Clone, Deserialize)]
struct MrFile {
    #[serde(default)]
    hashes: HashMap<String, String>,
    url: String,
    filename: String,
    #[serde(default)]
    primary: bool,
    #[serde(default)]
    size: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct MrDep {
    project_id: Option<String>,
    dependency_type: String,
}

impl MrVersion {
    fn primary(&self) -> Option<&MrFile> {
        self.files.iter().find(|f| f.primary).or(self.files.first())
    }

    fn fits(&self, mc: &str, loader: &str, folder: &str) -> bool {
        self.game_versions.iter().any(|v| v == mc) && (folder != "mods" || self.loaders.iter().any(|l| l == loader))
    }

    fn file(&self, folder: &str, client: bool) -> Option<PackFile> {
        let f = self.primary()?;
        Some(PackFile { path: format!("{folder}/{}", f.filename), sha1: f.hashes.get("sha1").cloned().unwrap_or_default(), sha512: f.hashes.get("sha512").cloned().unwrap_or_default(), size: f.size, urls: vec![f.url.clone()], client, server: true, local: None })
    }
}

pub struct Modrinth {
    base: String,
    fabric: String,
    quilt: String,
    /// NeoForge's Maven and Forge's file site, for their newest builds.
    neo: String,
    forge: String,
    agent: ureq::Agent,
}

fn agent_for(base: &str) -> ureq::Agent {
    let mut cfg = ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(20))).timeout_global(Some(Duration::from_secs(300))).http_status_as_error(false);
    if base.starts_with("http://127.0.0.1") {
        cfg = cfg.proxy(None);
    }
    cfg.build().into()
}

impl Modrinth {
    pub fn new(base: &str, fabric: &str, quilt: &str) -> Modrinth {
        Modrinth { base: base.trim_end_matches('/').into(), fabric: fabric.trim_end_matches('/').into(), quilt: quilt.trim_end_matches('/').into(), neo: crate::mcserver::install::NEO_MAVEN.into(), forge: crate::mcserver::install::FORGE_FILES.into(), agent: agent_for(base) }
    }

    /// Where NeoForge and Forge builds are looked up (tests use a pretend site).
    pub fn with_forge_sites(self, neo: &str, forge: &str) -> Modrinth {
        Modrinth { neo: neo.trim_end_matches('/').into(), forge: forge.trim_end_matches('/').into(), ..self }
    }

    fn ua() -> &'static str {
        concat!("Kerioxsx/omnihub/", env!("CARGO_PKG_VERSION"), " (OmniHub mod manager)")
    }

    fn read(mut res: ureq::http::Response<ureq::Body>, limit: u64) -> Result<Vec<u8>, String> {
        let status = res.status().as_u16();
        let mut out = Vec::new();
        res.body_mut().as_reader().take(limit).read_to_end(&mut out).map_err(|e| format!("The download stopped: {e}"))?;
        if status == 429 {
            return Err("Modrinth asked OmniHub to slow down. Try again in a minute.".into());
        }
        if !(200..300).contains(&status) {
            return Err(format!("Modrinth answered {status}."));
        }
        Ok(out)
    }

    fn get(&self, url: &str) -> Result<Value, String> {
        let res = self.agent.get(url).header("User-Agent", Self::ua()).call().map_err(|e| format!("Couldn't reach Modrinth: {e}"))?;
        serde_json::from_slice(&Self::read(res, 64 << 20)?).map_err(|e| format!("Modrinth sent something unreadable: {e}"))
    }

    fn post(&self, path: &str, body: &Value) -> Result<Value, String> {
        let res = self.agent.post(&format!("{}{path}", self.base)).header("User-Agent", Self::ua()).header("Content-Type", "application/json").send(body.to_string()).map_err(|e| format!("Couldn't reach Modrinth: {e}"))?;
        serde_json::from_slice(&Self::read(res, 64 << 20)?).map_err(|e| format!("Modrinth sent something unreadable: {e}"))
    }

    fn q(v: &Value) -> String {
        percent_encoding::utf8_percent_encode(&v.to_string(), percent_encoding::NON_ALPHANUMERIC).to_string()
    }

    pub fn search(&self, query: &str, offset: usize) -> Result<(Vec<PackHit>, u64), String> {
        let facets = json!([["project_type:modpack"]]);
        let v = self.get(&format!("{}/v2/search?query={}&facets={}&index={}&limit=20&offset={offset}", self.base, percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC), Self::q(&facets), if query.trim().is_empty() { "downloads" } else { "relevance" }))?;
        const LOADERS: &[&str] = &["fabric", "quilt", "forge", "neoforge"];
        let hits = v["hits"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|h| {
                let s = |k: &str| h[k].as_str().unwrap_or_default().to_string();
                let list = |k: &str| h[k].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect::<Vec<_>>();
                PackHit { project: s("project_id"), slug: s("slug"), title: s("title"), description: s("description"), icon: h["icon_url"].as_str().filter(|u| !u.is_empty()).map(String::from), downloads: h["downloads"].as_u64().unwrap_or(0), versions: list("versions"), loaders: list("categories").into_iter().filter(|c| LOADERS.contains(&c.as_str())).collect() }
            })
            .collect();
        Ok((hits, v["total_hits"].as_u64().unwrap_or(0)))
    }

    fn ok_id(id: &str) -> Result<(), String> {
        if id.is_empty() || id.len() > 64 || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
            return Err("Not a Modrinth project.".into());
        }
        Ok(())
    }

    pub fn pack_versions(&self, project: &str) -> Result<Vec<PackVersion>, String> {
        Self::ok_id(project)?;
        let v = self.get(&format!("{}/v2/project/{project}/version", self.base))?;
        Ok(v.as_array()
            .into_iter()
            .flatten()
            .map(|x| {
                let list = |k: &str| x[k].as_array().into_iter().flatten().filter_map(|y| y.as_str().map(String::from)).collect::<Vec<_>>();
                PackVersion { id: x["id"].as_str().unwrap_or_default().into(), name: x["name"].as_str().unwrap_or_default().into(), number: x["version_number"].as_str().unwrap_or_default().into(), mc: list("game_versions"), loaders: list("loaders"), date: x["date_published"].as_str().unwrap_or_default().into() }
            })
            .collect())
    }

    /// A modpack version's .mrpack, its project's title and icon.
    fn pack_file(&self, project: &str, version: &str) -> Result<(Vec<u8>, String, Option<String>), String> {
        Self::ok_id(project)?;
        Self::ok_id(version)?;
        let v = self.get(&format!("{}/v2/version/{version}", self.base))?;
        if v["project_id"].as_str() != Some(project) {
            return Err("That version belongs to another pack.".into());
        }
        let ver: MrVersion = serde_json::from_value(v).map_err(|e| e.to_string())?;
        let f = ver.files.iter().find(|f| f.filename.ends_with(".mrpack")).or(ver.primary()).ok_or("That version has no .mrpack.")?;
        let p = self.get(&format!("{}/v2/project/{project}", self.base))?;
        let bytes = self.download(std::slice::from_ref(&f.url), f.hashes.get("sha1").map(String::as_str).unwrap_or_default(), f.hashes.get("sha512").map(String::as_str).unwrap_or_default())?;
        Ok((bytes, p["title"].as_str().unwrap_or(project).to_string(), p["icon_url"].as_str().filter(|u| !u.is_empty()).map(String::from)))
    }

    fn by_hash(&self, hashes: &[String]) -> Result<HashMap<String, MrVersion>, String> {
        let mut out = HashMap::new();
        for chunk in hashes.chunks(500) {
            let v = self.post("/v2/version_files", &json!({ "hashes": chunk, "algorithm": "sha1" }))?;
            out.extend(serde_json::from_value::<HashMap<String, MrVersion>>(v).unwrap_or_default());
        }
        Ok(out)
    }

    fn update_by_hash(&self, hashes: &[String], mc: &str, loaders: &[&str]) -> Result<HashMap<String, MrVersion>, String> {
        let mut out = HashMap::new();
        for chunk in hashes.chunks(500) {
            let v = self.post("/v2/version_files/update", &json!({ "hashes": chunk, "algorithm": "sha1", "loaders": loaders, "game_versions": [mc] }))?;
            out.extend(serde_json::from_value::<HashMap<String, MrVersion>>(v).unwrap_or_default());
        }
        Ok(out)
    }

    fn versions_for(&self, project: &str, mc: &str, loaders: &[&str]) -> Result<Vec<MrVersion>, String> {
        Self::ok_id(project)?;
        let v = self.get(&format!("{}/v2/project/{project}/version?loaders={}&game_versions={}", self.base, Self::q(&json!(loaders)), Self::q(&json!([mc]))))?;
        Ok(serde_json::from_value(v).unwrap_or_default())
    }

    fn projects(&self, ids: &[String]) -> Result<HashMap<String, (String, Option<String>)>, String> {
        let mut out = HashMap::new();
        for chunk in ids.chunks(200) {
            let v = self.get(&format!("{}/v2/projects?ids={}", self.base, Self::q(&json!(chunk))))?;
            for p in v.as_array().into_iter().flatten() {
                if let Some(id) = p["id"].as_str() {
                    out.insert(id.to_string(), (p["title"].as_str().unwrap_or(id).to_string(), p["icon_url"].as_str().filter(|u| !u.is_empty()).map(String::from)));
                }
            }
        }
        Ok(out)
    }

    /// Download from the first address that works; kept only if a hash matches.
    pub fn download(&self, urls: &[String], sha1: &str, sha512: &str) -> Result<Vec<u8>, String> {
        let mut last = "No download address.".to_string();
        for u in urls {
            if !(HOSTS.iter().any(|h| u.starts_with(h)) || u.starts_with(&self.base)) {
                last = format!("{u} isn't on a host modpacks may use.");
                continue;
            }
            let res = match self.agent.get(u).header("User-Agent", Self::ua()).call() {
                Ok(r) => r,
                Err(e) => {
                    last = format!("Couldn't download {u}: {e}");
                    continue;
                }
            };
            let bytes = match Self::read(res, 1 << 30) {
                Ok(b) => b,
                Err(e) => {
                    last = e;
                    continue;
                }
            };
            use sha1::Digest as _;
            let ok = if !sha512.is_empty() {
                hex::encode(sha2::Sha512::digest(&bytes)).eq_ignore_ascii_case(sha512)
            } else if !sha1.is_empty() {
                hex::encode(sha1::Sha1::digest(&bytes)).eq_ignore_ascii_case(sha1)
            } else {
                false
            };
            if ok {
                return Ok(bytes);
            }
            last = format!("{u} didn't match its published checksum.");
        }
        Err(last)
    }

    /// A pack's picture from Modrinth's CDN (at most 8 MB).
    pub fn picture(&self, url: &str) -> Result<Vec<u8>, String> {
        if !(url.starts_with("https://cdn.modrinth.com/") || url.starts_with(&format!("{}/", self.base))) {
            return Err("Pictures only come from Modrinth.".into());
        }
        let res = self.agent.get(url).header("User-Agent", Self::ua()).call().map_err(|e| format!("Couldn't reach Modrinth: {e}"))?;
        Self::read(res, 8 << 20)
    }

    /// The newest stable Fabric or Quilt loader for a Minecraft version.
    pub fn latest_loader(&self, loader: &str, mc: &str) -> Result<String, String> {
        let url = match loader {
            "fabric" => format!("{}/v2/versions/loader/{mc}", self.fabric),
            "quilt" => format!("{}/v3/versions/loader/{mc}", self.quilt),
            "neoforge" => {
                // NeoForge 21.1.x is for Minecraft 1.21.1, 26.1.2.x for 26.1.2.
                let v = self.get(&format!("{}/api/maven/versions/releases/net/neoforged/neoforge", self.neo)).map_err(|e| e.replace("Modrinth", "NeoForge"))?;
                let mut all: Vec<String> = v["versions"].as_array().into_iter().flatten().filter_map(|x| x.as_str()).filter(|x| crate::mcserver::install::neo_mc(x).as_deref() == Some(mc)).map(String::from).collect();
                all.sort_by(|a, b| version_cmp(b, a));
                return all.iter().find(|x| !x.contains("beta") && !x.contains("alpha")).or(all.first()).cloned().ok_or_else(|| format!("NeoForge has no build for Minecraft {mc} yet."));
            }
            "forge" => {
                let v = self.get(&format!("{}/net/minecraftforge/forge/promotions_slim.json", self.forge)).map_err(|e| e.replace("Modrinth", "Forge"))?;
                let p = &v["promos"];
                return p[format!("{mc}-recommended")].as_str().or(p[format!("{mc}-latest")].as_str()).map(String::from).ok_or_else(|| format!("Forge has no build for Minecraft {mc}."));
            }
            _ => return Err(format!("OmniHub can't look up {loader} versions.")),
        };
        let v = self.get(&url)?;
        let list = v.as_array().ok_or("No loader list.")?;
        let pick = list.iter().find(|x| x["loader"]["stable"].as_bool() == Some(true) || !x["loader"]["version"].as_str().unwrap_or_default().contains("beta")).or(list.first()).ok_or_else(|| format!("{loader} has no loader for Minecraft {mc}."))?;
        pick["loader"]["version"].as_str().map(String::from).ok_or_else(|| "No loader version.".into())
    }

    /// The launcher profile (version JSON) for Fabric or Quilt.
    fn profile_json(&self, loader: &str, mc: &str, version: &str) -> Result<Value, String> {
        let url = match loader {
            "fabric" => format!("{}/v2/versions/loader/{mc}/{version}/profile/json", self.fabric),
            "quilt" => format!("{}/v3/versions/loader/{mc}/{version}/profile/json", self.quilt),
            _ => return Err("Only Fabric and Quilt mixes go straight into the Minecraft Launcher.".into()),
        };
        self.get(&url)
    }
}

/// A Modrinth project as a search lists it (mods, plugins, data packs).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectHit {
    pub project: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub icon: Option<String>,
    pub downloads: u64,
    pub categories: Vec<String>,
    /// required, optional, unsupported, unknown
    pub server_side: String,
    pub client_side: String,
}

/// A project's version picked for a server: its file and the projects it needs.
#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub project: String,
    pub number: String,
    pub file: PackFile,
    pub requires: Vec<String>,
}

/// A project's title, icon and where it runs.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectMeta {
    pub title: String,
    pub icon: Option<String>,
    pub server_side: String,
}

impl Modrinth {
    /// Search one kind of project (mod, plugin, datapack) with extra facets.
    pub fn search_projects(&self, project_type: &str, query: &str, facets: &[Vec<String>], offset: usize) -> Result<(Vec<ProjectHit>, u64), String> {
        let mut all = vec![vec![format!("project_type:{project_type}")]];
        all.extend(facets.iter().filter(|f| !f.is_empty()).cloned());
        let v = self.get(&format!("{}/v2/search?query={}&facets={}&index={}&limit=20&offset={offset}", self.base, percent_encoding::utf8_percent_encode(query, percent_encoding::NON_ALPHANUMERIC), Self::q(&json!(all)), if query.trim().is_empty() { "downloads" } else { "relevance" }))?;
        let hits = v["hits"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|h| {
                let s = |k: &str| h[k].as_str().unwrap_or_default().to_string();
                ProjectHit { project: s("project_id"), slug: s("slug"), title: s("title"), description: s("description"), author: s("author"), icon: h["icon_url"].as_str().filter(|u| !u.is_empty()).map(String::from), downloads: h["downloads"].as_u64().unwrap_or(0), categories: h["categories"].as_array().into_iter().flatten().filter_map(|c| c.as_str().map(String::from)).collect(), server_side: s("server_side"), client_side: s("client_side") }
            })
            .collect();
        Ok((hits, v["total_hits"].as_u64().unwrap_or(0)))
    }

    /// The newest version of a project for these loaders and Minecraft version.
    pub fn newest(&self, project: &str, loaders: &[&str], mc: &str, folder: &str) -> Result<Option<Release>, String> {
        let list = self.versions_for(project, mc, loaders)?;
        let Some(v) = list.into_iter().find(|v| v.game_versions.iter().any(|g| g == mc)) else { return Ok(None) };
        let Some(file) = v.file(folder, true) else { return Ok(None) };
        let requires = v.dependencies.iter().filter(|d| d.dependency_type == "required").filter_map(|d| d.project_id.clone()).collect();
        Ok(Some(Release { project: v.project_id.clone(), number: v.version_number.clone(), file, requires }))
    }

    /// Projects' titles, icons and whether they run on servers.
    pub fn project_meta(&self, ids: &[String]) -> Result<HashMap<String, ProjectMeta>, String> {
        let mut out = HashMap::new();
        for chunk in ids.chunks(200) {
            let v = self.get(&format!("{}/v2/projects?ids={}", self.base, Self::q(&json!(chunk))))?;
            for p in v.as_array().into_iter().flatten() {
                if let Some(id) = p["id"].as_str() {
                    out.insert(id.to_string(), ProjectMeta { title: p["title"].as_str().unwrap_or(id).to_string(), icon: p["icon_url"].as_str().filter(|u| !u.is_empty()).map(String::from), server_side: p["server_side"].as_str().unwrap_or("unknown").to_string() });
                }
            }
        }
        Ok(out)
    }

    /// Which project and version files are, by their SHA-1.
    pub fn identify(&self, sha1s: &[String]) -> Result<HashMap<String, (String, String)>, String> {
        Ok(self.by_hash(sha1s)?.into_iter().map(|(h, v)| (h, (v.project_id, v.version_number))).collect())
    }

    /// A modpack version's .mrpack, its title and icon.
    pub fn mrpack(&self, project: &str, version: &str) -> Result<(Vec<u8>, String, Option<String>), String> {
        self.pack_file(project, version)
    }
}

/// Read a .mrpack: its index and how many override files it has.
/// The picture an instance folder keeps: Prism Launcher's icon (named in
/// instance.cfg), Modrinth App's (in profile.json), or an icon.png beside it.
fn folder_icon(path: &Path) -> Option<Vec<u8>> {
    let small = |p: &Path| std::fs::metadata(p).ok().filter(|m| m.is_file() && m.len() <= 8 << 20).and_then(|_| std::fs::read(p).ok());
    let roots: Vec<PathBuf> = [Some(path.to_path_buf()), path.parent().map(Path::to_path_buf)].into_iter().flatten().collect();
    for r in &roots {
        for n in ["icon.png", "pack.png"] {
            if let Some(b) = small(&r.join(n)) {
                return Some(b);
            }
        }
        if let Ok(cfg) = std::fs::read_to_string(r.join("instance.cfg")) {
            let key = cfg.lines().find_map(|l| l.strip_prefix("iconKey=")).map(str::trim).filter(|k| !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')));
            if let (Some(key), Some(icons)) = (key, r.parent().and_then(Path::parent).map(|p| p.join("icons"))) {
                for ext in ["png", "jpg", "jpeg", "gif", "webp"] {
                    if let Some(b) = small(&icons.join(format!("{key}.{ext}"))) {
                        return Some(b);
                    }
                }
            }
        }
        if let Some(icon) = std::fs::read(r.join("profile.json")).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()).and_then(|v| v["icon_path"].as_str().map(PathBuf::from)) {
            if let Some(b) = small(&icon) {
                return Some(b);
            }
        }
    }
    None
}

pub fn read_mrpack(bytes: &[u8]) -> Result<(Value, usize), String> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|_| "That isn't a .mrpack (zip) file.".to_string())?;
    let mut idx = String::new();
    z.by_name("modrinth.index.json").map_err(|_| "That zip has no modrinth.index.json, so it isn't a Modrinth pack.".to_string())?.take(32 << 20).read_to_string(&mut idx).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&idx).map_err(|e| format!("Its modrinth.index.json can't be read: {e}"))?;
    if v["game"] != "minecraft" {
        return Err("That pack isn't for Minecraft.".into());
    }
    let n = z.file_names().filter(|n| (n.starts_with("overrides/") || n.starts_with("client-overrides/")) && !n.ends_with('/')).count();
    Ok((v, n))
}

fn pack_files(index: &Value) -> Vec<PackFile> {
    index["files"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| {
            let path = super::install::safe_rel(f["path"].as_str()?)?;
            Some(PackFile {
                path,
                sha1: f["hashes"]["sha1"].as_str().unwrap_or_default().to_string(),
                sha512: f["hashes"]["sha512"].as_str().unwrap_or_default().to_string(),
                size: f["fileSize"].as_u64().unwrap_or(0),
                urls: f["downloads"].as_array().into_iter().flatten().filter_map(|u| u.as_str().map(String::from)).collect(),
                client: f["env"]["client"].as_str() != Some("unsupported"),
                server: f["env"]["server"].as_str() != Some("unsupported"),
                local: None,
            })
        })
        .collect()
}

/// The pack's Minecraft version, loader and loader version.
fn pack_target(index: &Value) -> (String, String, String) {
    let deps = &index["dependencies"];
    let mc = deps["minecraft"].as_str().unwrap_or_default().to_string();
    for (key, name) in [("fabric-loader", "fabric"), ("quilt-loader", "quilt"), ("neoforge", "neoforge"), ("forge", "forge")] {
        if let Some(v) = deps[key].as_str() {
            return (mc, name.into(), v.into());
        }
    }
    (mc, "vanilla".into(), String::new())
}

/// The .jar files in a folder (not the turned-off .jar.disabled ones).
fn jars_in(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)?.flatten().map(|e| e.path()).filter(|p| p.is_file() && p.extension().is_some_and(|x| x.eq_ignore_ascii_case("jar"))).collect();
    v.sort();
    Ok(v)
}

fn sha_pair(b: &[u8]) -> (String, String) {
    use sha1::Digest as _;
    (hex::encode(sha1::Sha1::digest(b)), hex::encode(sha2::Sha512::digest(b)))
}

/// Folders of an instance that are carried into a mix.
/// Files beside the mods (configs and the like): where they go, and what's in them.
pub type Extras = Vec<(String, Vec<u8>)>;

/// What's in an installed modpack.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherDetails {
    pub mods: Vec<launchers::ModInfo>,
    pub worlds: Vec<launchers::World>,
    /// The instance folder's size, worlds included.
    pub size: u64,
}

/// The kind of file a pack for friends is saved as.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PackFormat {
    /// Modrinth's .mrpack (the Modrinth App, Prism Launcher, ATLauncher…).
    Mrpack,
    /// CurseForge's modpack zip (manifest.json and overrides).
    Curseforge,
}

/// A server to put in a pack's server list, so friends find it in Multiplayer.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ServerEntry {
    pub name: String,
    pub address: String,
}

/// What went into a pack for friends.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Shared {
    pub mods: usize,
    /// Mods the friends' launcher downloads from Modrinth.
    pub from_modrinth: usize,
    /// Mods carried inside the file.
    pub included: usize,
    pub bytes: u64,
}

/// servers.dat with one server in it (uncompressed NBT, as Minecraft writes it).
pub fn servers_dat(server: &ServerEntry) -> Result<Vec<u8>, String> {
    use crate::mcserver::nbt::Tag;
    let address = server.address.trim();
    let ok = (1..=255).contains(&address.len()) && address.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']' | '_'));
    if !ok {
        return Err("That server address can't be used (letters, digits, dots, dashes and a :port).".into());
    }
    let name: String = server.name.chars().filter(|c| !c.is_control()).take(64).collect();
    let entry = Tag::Compound(vec![("name".into(), Tag::Str(if name.trim().is_empty() { "Server".into() } else { name.trim().to_string() })), ("ip".into(), Tag::Str(address.into()))]);
    Ok(crate::mcserver::nbt::write(&Tag::Compound(vec![("servers".into(), Tag::List(vec![entry]))])))
}

/// Folders a pack for friends carries along (not your keys, video settings or servers).
const SHARE: &[&str] = &["config", "defaultconfigs", "kubejs", "resourcepacks", "shaderpacks", "global_packs"];

const KEEP: &[&str] = &["config", "defaultconfigs", "kubejs", "resourcepacks", "shaderpacks", "global_packs", "options.txt", "servers.dat"];

pub struct Mixer {
    dir: PathBuf,
    pub api: Modrinth,
    minecraft: Option<PathBuf>,
    /// Where other launchers keep their modpacks.
    places: launchers::Places,
    /// What was found there the last time.
    found: parking_lot::Mutex<Vec<launchers::Instance>>,
}

impl Mixer {
    pub fn new(dir: &Path, api: Modrinth, minecraft: Option<PathBuf>) -> Mixer {
        Mixer { dir: dir.to_path_buf(), api, minecraft, places: launchers::Places::default(), found: Default::default() }
    }

    /// Look for other launchers' modpacks in these places.
    pub fn looking_in(self, places: launchers::Places) -> Mixer {
        Mixer { places, ..self }
    }

    pub fn minecraft_dir(&self) -> Option<&Path> {
        self.minecraft.as_deref()
    }

    fn sdir(&self, id: &str) -> Result<PathBuf, String> {
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err("No such pack.".into());
        }
        Ok(self.dir.join("sources").join(id))
    }

    fn mdir(&self, id: &str) -> Result<PathBuf, String> {
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric()) {
            return Err("No such mix.".into());
        }
        Ok(self.dir.join("mixes").join(id))
    }

    pub fn sources(&self) -> Vec<Source> {
        let mut v: Vec<Source> = std::fs::read_dir(self.dir.join("sources")).into_iter().flatten().flatten().filter_map(|e| std::fs::read(e.path().join("source.json")).ok()).filter_map(|b| serde_json::from_slice(&b).ok()).collect();
        v.sort_by_key(|s| s.name.to_lowercase());
        v
    }

    pub fn source(&self, id: &str) -> Result<Source, String> {
        std::fs::read(self.sdir(id)?.join("source.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).ok_or_else(|| "That pack was removed.".into())
    }

    pub fn remove_source(&self, id: &str) -> Result<(), String> {
        let d = self.sdir(id)?;
        if d.is_dir() {
            std::fs::remove_dir_all(d).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn keep_source(&self, s: &Source) -> Result<(), String> {
        let d = self.sdir(&s.id)?;
        std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
        std::fs::write(d.join("source.json"), serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    /// Unpack a .mrpack's overrides into the source's folder.
    fn add_mrpack(&self, bytes: &[u8], name: Option<String>, icon: Option<String>, kind: SourceKind) -> Result<Source, String> {
        let (index, n) = read_mrpack(bytes)?;
        let id = uuid::Uuid::new_v4().simple().to_string();
        let d = self.sdir(&id)?;
        let over = d.join("overrides");
        std::fs::create_dir_all(&over).map_err(|e| e.to_string())?;
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
        for i in 0..z.len() {
            let mut f = z.by_index(i).map_err(|e| e.to_string())?;
            let name = f.name().to_string();
            let rest = name.strip_prefix("overrides/").or_else(|| name.strip_prefix("client-overrides/"));
            let Some(rest) = rest.filter(|_| !f.is_dir()) else { continue };
            let Some(rel) = super::install::safe_rel(rest) else { return Err(format!("The pack has a file that would land outside its folder: {name}")) };
            let out = over.join(&rel);
            std::fs::create_dir_all(out.parent().unwrap_or(&over)).map_err(|e| e.to_string())?;
            let mut w = std::fs::File::create(&out).map_err(|e| e.to_string())?;
            std::io::copy(&mut (&mut f).take(1 << 30), &mut w).map_err(|e| e.to_string())?;
        }
        let (mc, loader, loader_version) = pack_target(&index);
        let s = Source { id, name: name.unwrap_or_else(|| index["name"].as_str().unwrap_or("Modpack").to_string()), kind, icon, mc, loader, loader_version, files: pack_files(&index), overrides: n };
        self.keep_source(&s)?;
        Ok(s)
    }

    /// Add a Modrinth modpack (a version of it) to the mixer.
    pub fn add_modrinth(&self, project: &str, version: &str) -> Result<Source, String> {
        let (bytes, title, icon) = self.api.pack_file(project, version)?;
        self.add_mrpack(&bytes, Some(title), icon, SourceKind::Modrinth { project: project.into(), version: version.into() })
    }

    /// Add a .mrpack file from this PC.
    pub fn add_file(&self, path: &str) -> Result<Source, String> {
        let b = std::fs::read(path).map_err(|e| format!("Couldn't open it: {e}"))?;
        self.add_mrpack(&b, None, None, SourceKind::File { path: path.into() })
    }

    /// Add an installed instance (Prism, CurseForge, Modrinth App, the launcher's
    /// own folder…): its mods are looked up on Modrinth by hash, and its
    /// version and loader worked out from what they're made for.
    pub fn add_folder(&self, path: &str) -> Result<Source, String> {
        let root = Path::new(path);
        let root = if root.join(".minecraft").is_dir() { root.join(".minecraft") } else if root.join("minecraft").join("mods").is_dir() { root.join("minecraft") } else { root.to_path_buf() };
        let name = Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "Instance".into());
        self.add_dir(&root, name, SourceKind::Folder { path: path.into() }, None)
    }

    /// A game folder's mods and settings as a pack. With `target` (version,
    /// loader, loader version) known, Modrinth is only asked for its copies of
    /// the mods, and the pack is added even when it can't be reached.
    fn add_dir(&self, root: &Path, name: String, kind: SourceKind, target: Option<(String, String, String)>) -> Result<Source, String> {
        let mods = if root.join("mods").is_dir() { root.join("mods") } else { root.to_path_buf() };
        let jars = jars_in(&mods).map_err(|e| format!("Couldn't open it: {e}"))?;
        if jars.is_empty() {
            return Err("There are no mods (.jar files) in that folder or its mods folder.".into());
        }
        let mut files = Vec::new();
        for j in &jars {
            let b = std::fs::read(j).map_err(|e| e.to_string())?;
            let (sha1, sha512) = sha_pair(&b);
            let name = j.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            files.push(PackFile { path: format!("mods/{name}"), sha1, sha512, size: b.len() as u64, urls: vec![], client: true, server: true, local: Some(j.to_string_lossy().into_owned()) });
        }
        let known = match (self.api.by_hash(&files.iter().map(|f| f.sha1.clone()).collect::<Vec<_>>()), &target) {
            (Ok(k), _) => k,
            (Err(_), Some(_)) => HashMap::new(),
            (Err(e), None) => return Err(e),
        };
        // Version and loader: what most of its mods are made for.
        let mut votes: HashMap<(String, String), usize> = HashMap::new();
        for f in &mut files {
            if let Some(v) = known.get(&f.sha1) {
                // The download of this very jar: a version can hold several
                // (a Forge and a NeoForge jar, sources), and its main one
                // may be another, which wouldn't match this jar's hashes.
                if let Some(p) = v.files.iter().find(|x| x.hashes.get("sha1") == Some(&f.sha1)) {
                    f.urls = vec![p.url.clone()];
                }
                for mc in &v.game_versions {
                    for l in v.loaders.iter().filter(|l| loader_key(l).is_some()) {
                        *votes.entry((mc.clone(), l.clone())).or_default() += 1;
                    }
                }
            }
        }
        let (mc, loader, loader_version) = match target {
            Some(t) => t,
            None => {
                let ((mc, loader), _) = votes.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(version_cmp(&a.0 .0, &b.0 .0))).ok_or("None of its mods are on Modrinth, so its version can't be worked out.")?;
                (mc, loader, String::new())
            }
        };
        let id = uuid::Uuid::new_v4().simple().to_string();
        let over = self.sdir(&id)?.join("overrides");
        std::fs::create_dir_all(&over).map_err(|e| e.to_string())?;
        let mut n = 0;
        for k in KEEP {
            let p = root.join(k);
            if p.is_file() {
                std::fs::copy(&p, over.join(k)).map_err(|e| e.to_string())?;
                n += 1;
            } else if p.is_dir() {
                for e in walkdir::WalkDir::new(&p).follow_links(false).into_iter().flatten().filter(|e| e.file_type().is_file()) {
                    let rel = e.path().strip_prefix(root).unwrap_or(e.path());
                    let out = over.join(rel);
                    std::fs::create_dir_all(out.parent().unwrap_or(&over)).map_err(|e| e.to_string())?;
                    std::fs::copy(e.path(), &out).map_err(|e| e.to_string())?;
                    n += 1;
                }
            }
        }
        let s = Source { id, name, kind, icon: None, mc, loader, loader_version, files, overrides: n };
        self.keep_source(&s)?;
        Ok(s)
    }

    /// Folders the user added for other launchers' modpacks.
    pub fn launcher_places(&self) -> Vec<String> {
        std::fs::read(self.dir.join("places.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    fn keep_places(&self, v: &[String]) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        std::fs::write(self.dir.join("places.json"), serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    /// Modpacks Prism Launcher, CurseForge and the Modrinth App installed, looked for again.
    pub fn launcher_packs(&self) -> Vec<launchers::Instance> {
        let places = launchers::Places { extra: self.launcher_places().into_iter().map(PathBuf::from).collect(), ..self.places.clone() };
        let v = launchers::find(&places);
        *self.found.lock() = v.clone();
        v
    }

    pub fn launcher_pack(&self, id: &str) -> Result<launchers::Instance, String> {
        if let Some(i) = self.found.lock().iter().find(|i| i.id == id) {
            return Ok(i.clone());
        }
        self.launcher_packs().into_iter().find(|i| i.id == id).ok_or_else(|| "That modpack isn't there any more.".into())
    }

    /// The modpack's picture as a PNG, at most 128 pixels across.
    pub fn launcher_icon(&self, id: &str) -> Option<Vec<u8>> {
        let img = super::cover::decode(&launchers::icon(&self.launcher_pack(id).ok()?)?)?;
        let img = if img.width().max(img.height()) > 128 { image::imageops::resize(&img, 128, 128, image::imageops::FilterType::Triangle) } else { img };
        Some(super::cover::png(&img))
    }

    /// Also look in this folder (a launcher's folder, its instances folder or one instance).
    pub fn add_launcher_place(&self, path: &str) -> Result<Vec<launchers::Instance>, String> {
        let p = PathBuf::from(path);
        if launchers::find(&launchers::Places { extra: vec![p], ..Default::default() }).is_empty() {
            return Err("There are no Prism Launcher, CurseForge or Modrinth App modpacks in that folder.".into());
        }
        let mut v = self.launcher_places();
        if !v.iter().any(|x| x == path) {
            v.push(path.into());
            self.keep_places(&v)?;
        }
        Ok(self.launcher_packs())
    }

    pub fn remove_launcher_place(&self, path: &str) -> Result<Vec<launchers::Instance>, String> {
        let mut v = self.launcher_places();
        v.retain(|x| x != path);
        self.keep_places(&v)?;
        Ok(self.launcher_packs())
    }

    /// Add a modpack another launcher installed, with the version and loader it says it uses.
    pub fn add_launcher_pack(&self, id: &str) -> Result<Source, String> {
        let i = self.launcher_pack(id)?;
        let target = (!i.mc.is_empty() && !i.loader.is_empty()).then(|| (i.mc.clone(), i.loader.clone(), i.loader_version.clone()));
        let s = self.add_dir(Path::new(&i.game_dir), i.name.clone(), SourceKind::Launcher { launcher: i.launcher_name.clone(), path: i.path.clone() }, target)?;
        if let Some(b) = launchers::icon(&i) {
            let _ = std::fs::write(self.sdir(&s.id)?.join("icon"), b);
        }
        Ok(s)
    }

    /// An installed modpack's mods (its files on this PC, those Modrinth says
    /// don't run on servers marked so) and its server-side settings.
    pub fn launcher_server_parts(&self, i: &launchers::Instance) -> Result<(Vec<PackFile>, Extras), String> {
        let game = Path::new(&i.game_dir);
        let mut files = Vec::new();
        for j in jars_in(&game.join("mods")).unwrap_or_default() {
            let b = std::fs::read(&j).map_err(|e| e.to_string())?;
            let (sha1, sha512) = sha_pair(&b);
            let name = j.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            files.push(PackFile { path: format!("mods/{name}"), sha1, sha512, size: b.len() as u64, urls: vec![], client: true, server: true, local: Some(j.to_string_lossy().into_owned()) });
        }
        if let Ok(known) = self.api.identify(&files.iter().map(|f| f.sha1.clone()).collect::<Vec<_>>()) {
            let ids: Vec<String> = known.values().map(|(p, _)| p.clone()).collect::<HashSet<_>>().into_iter().collect();
            let meta = if ids.is_empty() { HashMap::new() } else { self.api.project_meta(&ids).unwrap_or_default() };
            for f in &mut files {
                if known.get(&f.sha1).and_then(|(p, _)| meta.get(p)).is_some_and(|m| m.server_side == "unsupported") {
                    f.server = false;
                }
            }
        }
        let mut over = Vec::new();
        for k in ["config", "defaultconfigs", "kubejs", "global_packs"] {
            let d = game.join(k);
            for rel in super::install::files_in(&d) {
                if let Ok(b) = std::fs::read(d.join(&rel)) {
                    over.push((format!("{k}/{rel}"), b));
                }
            }
        }
        Ok((files, over))
    }

    /// What's in an installed modpack: its mods (as their jars describe
    /// themselves), its single-player worlds and its size.
    pub fn launcher_details(&self, id: &str) -> Result<LauncherDetails, String> {
        let i = self.launcher_pack(id)?;
        let game = Path::new(&i.game_dir);
        Ok(LauncherDetails { mods: launchers::mods_in(game), worlds: launchers::worlds(game), size: launchers::size_of(Path::new(&i.path)) })
    }

    /// A world's icon.png (at most 1 MB, and one that can be shown).
    pub fn launcher_world_icon(&self, id: &str, folder: &str) -> Option<Vec<u8>> {
        let i = self.launcher_pack(id).ok()?;
        let p = launchers::world_dir(Path::new(&i.game_dir), folder)?.join("icon.png");
        std::fs::metadata(&p).ok().filter(|m| m.len() <= 1 << 20)?;
        let b = std::fs::read(p).ok()?;
        super::cover::decode(&b)?;
        Some(b)
    }

    /// Mods with a newer version on Modrinth for the pack's Minecraft and
    /// loader: file name → that version. Nothing in the pack changes.
    pub fn launcher_updates(&self, id: &str) -> Result<HashMap<String, String>, String> {
        let i = self.launcher_pack(id)?;
        if i.mc.is_empty() || loader_key(&i.loader).is_none() {
            return Err(format!("{} doesn't say which Minecraft version and mod loader it uses.", i.launcher_name));
        }
        let mut by_sha = HashMap::new();
        for j in jars_in(&Path::new(&i.game_dir).join("mods")).unwrap_or_default() {
            let b = std::fs::read(&j).map_err(|e| e.to_string())?;
            by_sha.insert(sha_pair(&b).0, j.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default());
        }
        let found = self.api.update_by_hash(&by_sha.keys().cloned().collect::<Vec<_>>(), &i.mc, &[i.loader.as_str()])?;
        Ok(found
            .into_iter()
            .filter(|(sha, v)| !v.files.iter().any(|f| f.hashes.get("sha1") == Some(sha)))
            .filter_map(|(sha, v)| Some((by_sha.get(&sha)?.clone(), v.version_number)))
            .collect())
    }

    /// Save an installed modpack for friends: the same mods (those on
    /// Modrinth as downloads in a .mrpack, the rest carried inside), its
    /// configs, resource and shader packs, and optionally your server in its
    /// server list. Your own keys, video settings and server list stay out.
    pub fn share_launcher_pack(&self, id: &str, dest: &Path, format: PackFormat, server: Option<&ServerEntry>) -> Result<Shared, String> {
        let i = self.launcher_pack(id)?;
        if i.mc.is_empty() {
            return Err(format!("{} doesn't say which Minecraft version {} is for.", i.launcher_name, i.name));
        }
        if !i.loader.is_empty() && i.loader_version.is_empty() {
            return Err(format!("{} doesn't say which {} version {} uses.", i.launcher_name, i.loader, i.name));
        }
        let servers = server.map(servers_dat).transpose()?;
        let game = Path::new(&i.game_dir);
        let jars = jars_in(&game.join("mods")).unwrap_or_default();
        // Each jar: its path in the pack, hashes and size.
        let mut mods = Vec::new();
        for j in &jars {
            let b = std::fs::read(j).map_err(|e| e.to_string())?;
            let (sha1, sha512) = sha_pair(&b);
            let name = j.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            mods.push((format!("mods/{name}"), j.clone(), sha1, sha512, b.len() as u64));
        }
        let mut files = Vec::new();
        let mut inside: Vec<(String, PathBuf)> = Vec::new();
        if format == PackFormat::Mrpack {
            let known = self.api.by_hash(&mods.iter().map(|m| m.2.clone()).collect::<Vec<_>>()).unwrap_or_default();
            let ids: Vec<String> = known.values().map(|v| v.project_id.clone()).collect::<HashSet<_>>().into_iter().collect();
            let meta = if ids.is_empty() { HashMap::new() } else { self.api.project_meta(&ids).unwrap_or_default() };
            for (path, src, sha1, sha512, size) in &mods {
                let v = known.get(sha1);
                let url = v.and_then(|v| v.files.iter().find(|f| f.hashes.get("sha1") == Some(sha1))).map(|f| f.url.clone()).filter(|u| HOSTS.iter().any(|h| u.starts_with(h)) || u.starts_with(&format!("{}/", self.api.base)));
                match url {
                    Some(url) => {
                        let server_side = v.and_then(|v| meta.get(&v.project_id)).is_some_and(|m| m.server_side == "unsupported");
                        files.push(json!({ "path": path, "hashes": { "sha1": sha1, "sha512": sha512 }, "env": { "client": "required", "server": if server_side { "unsupported" } else { "required" } }, "downloads": [url], "fileSize": size }));
                    }
                    None => inside.push((path.clone(), src.clone())),
                }
            }
        } else {
            inside = mods.iter().map(|m| (m.0.clone(), m.1.clone())).collect();
        }
        let summary = format!("Shared with OmniHub from {} ({}).", i.name, i.launcher_name);
        let (index_name, index) = match format {
            PackFormat::Mrpack => {
                let mut deps = serde_json::Map::new();
                deps.insert("minecraft".into(), json!(i.mc));
                if let Some(k) = loader_key(&i.loader) {
                    deps.insert(k.into(), json!(i.loader_version));
                }
                ("modrinth.index.json", json!({ "formatVersion": 1, "game": "minecraft", "versionId": "1.0.0", "name": i.name, "summary": summary, "files": files, "dependencies": deps }))
            }
            PackFormat::Curseforge => {
                let loaders: Vec<Value> = if i.loader.is_empty() { vec![] } else { vec![json!({ "id": format!("{}-{}", i.loader, i.loader_version), "primary": true })] };
                ("manifest.json", json!({ "minecraft": { "version": i.mc, "modLoaders": loaders }, "manifestType": "minecraftModpack", "manifestVersion": 1, "name": i.name, "version": "1.0.0", "author": "OmniHub", "files": [], "overrides": "overrides" }))
            }
        };
        let tmp = dest.with_extension("part");
        let written = (|| -> Result<(), String> {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&tmp).map_err(|e| e.to_string())?);
            let o = zip::write::SimpleFileOptions::default().large_file(true);
            z.start_file(index_name, o).map_err(|e| e.to_string())?;
            z.write_all(serde_json::to_string_pretty(&index).unwrap_or_default().as_bytes()).map_err(|e| e.to_string())?;
            let copy = |z: &mut zip::ZipWriter<std::fs::File>, rel: &str, src: &Path| -> Result<(), String> {
                z.start_file(format!("overrides/{rel}"), o).map_err(|e| e.to_string())?;
                std::io::copy(&mut std::fs::File::open(src).map_err(|e| format!("{}: {e}", src.display()))?, z).map_err(|e| e.to_string())?;
                Ok(())
            };
            for (rel, src) in &inside {
                copy(&mut z, rel, src)?;
            }
            for k in SHARE {
                let d = game.join(k);
                for rel in super::install::files_in(&d) {
                    copy(&mut z, &format!("{k}/{rel}"), &d.join(&rel))?;
                }
            }
            if let Some(b) = &servers {
                z.start_file("overrides/servers.dat", o).map_err(|e| e.to_string())?;
                z.write_all(b).map_err(|e| e.to_string())?;
            }
            z.finish().map_err(|e| e.to_string())?;
            Ok(())
        })();
        if let Err(e) = written {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        std::fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
        Ok(Shared { mods: mods.len(), from_modrinth: files.len(), included: inside.len(), bytes: std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0) })
    }

    /// Work out the mix of `sources` (in priority order) for `mc` and `loader`.
    pub fn plan(&self, name: &str, ids: &[String], mc: &str, loader: &str) -> Result<Mix, String> {
        if ids.is_empty() {
            return Err("Add at least one pack.".into());
        }
        if loader_key(loader).is_none() {
            return Err("Pick Fabric, Quilt, Forge or NeoForge.".into());
        }
        let sources: Vec<Source> = ids.iter().map(|i| self.source(i)).collect::<Result<_, _>>()?;
        let mut problems = Vec::new();
        // Loader version: the newest a pack with the same target uses, or the newest stable one.
        let loader_version = sources
            .iter()
            .filter(|s| s.mc == mc && s.loader == loader && !s.loader_version.is_empty())
            .map(|s| s.loader_version.clone())
            .max_by(|a, b| version_cmp(a, b))
            .map(Ok)
            .unwrap_or_else(|| self.api.latest_loader(loader, mc));
        let loader_version = match loader_version {
            Ok(v) => v,
            Err(e) => {
                problems.push(e);
                String::new()
            }
        };

        // Every file once by hash, remembering which packs brought it.
        let mut order: Vec<(PackFile, Vec<String>, bool)> = Vec::new();
        for s in &sources {
            let same = s.mc == mc && s.loader == loader;
            for f in &s.files {
                if !f.client {
                    continue;
                }
                match order.iter_mut().find(|(g, _, _)| g.sha1 == f.sha1 && !f.sha1.is_empty()) {
                    Some((_, from, _)) => from.push(s.name.clone()),
                    None => order.push((f.clone(), vec![s.name.clone()], same)),
                }
            }
        }
        let hashes: Vec<String> = order.iter().map(|(f, _, _)| f.sha1.clone()).filter(|h| !h.is_empty()).collect();
        let known = self.api.by_hash(&hashes)?;
        let folder_of = |p: &str| p.split('/').next().unwrap_or("mods").to_string();

        // Group by Modrinth project; the first pack's copy decides the order.
        struct Pick {
            project: Option<String>,
            folder: String,
            files: Vec<(PackFile, Option<MrVersion>, bool)>,
            from: Vec<String>,
        }
        let mut picks: Vec<Pick> = Vec::new();
        for (f, from, same) in order {
            let v = known.get(&f.sha1).cloned();
            let project = v.as_ref().map(|v| v.project_id.clone());
            let folder = folder_of(&f.path);
            match picks.iter_mut().find(|p| p.project.is_some() && p.project == project) {
                Some(p) => {
                    for n in from {
                        if !p.from.contains(&n) {
                            p.from.push(n);
                        }
                    }
                    p.files.push((f, v, same));
                }
                None => picks.push(Pick { project, folder, from, files: vec![(f, v, same)] }),
            }
        }
        let loaders: Vec<&str> = if loader == "quilt" { vec!["quilt", "fabric"] } else { vec![loader] };
        // One lookup for every project that has no copy made for the target.
        let need: Vec<String> = picks.iter().filter(|p| p.project.is_some() && p.folder == "mods" && !p.files.iter().any(|(_, v, _)| v.as_ref().is_some_and(|v| loaders.iter().any(|l| v.fits(mc, l, "mods"))))).map(|p| p.files[0].0.sha1.clone()).collect();
        let updates = if need.is_empty() { HashMap::new() } else { self.api.update_by_hash(&need, mc, &loaders)? };

        let mut entries: Vec<(Entry, Option<MrVersion>)> = Vec::new();
        for p in picks {
            let first = &p.files[0].0;
            let base_title = first.path.rsplit('/').next().unwrap_or(&first.path).trim_end_matches(".jar").to_string();
            let entry = |status, file: Option<PackFile>, version: String, note: Option<String>| Entry { project: p.project.clone(), title: base_title.clone(), icon: None, folder: p.folder.clone(), version, file, from: p.from.clone(), status, note };
            if p.project.is_none() {
                let same = p.files[0].2;
                if same || p.folder != "mods" {
                    entries.push((entry(EntryStatus::AsIs, Some(first.clone()), String::new(), Some("Not on Modrinth: kept as the pack had it.".into())), None));
                } else {
                    entries.push((entry(EntryStatus::Missing, None, String::new(), Some(format!("Not on Modrinth, so it can't be moved to Minecraft {mc}."))), None));
                }
                continue;
            }
            // Resource and shader packs aren't tied to a version.
            if p.folder != "mods" {
                let (f, v, _) = p.files.last().cloned().unwrap_or_else(|| p.files[0].clone());
                entries.push((entry(EntryStatus::Kept, Some(f), v.as_ref().map(|v| v.version_number.clone()).unwrap_or_default(), None), v));
                continue;
            }
            let fitting = p.files.iter().filter(|(_, v, _)| v.as_ref().is_some_and(|v| loaders.iter().any(|l| v.fits(mc, l, "mods")))).max_by(|a, b| a.1.as_ref().map(|v| v.date_published.clone()).cmp(&b.1.as_ref().map(|v| v.date_published.clone())));
            if let Some((f, v, _)) = fitting {
                entries.push((entry(EntryStatus::Kept, Some(f.clone()), v.as_ref().map(|v| v.version_number.clone()).unwrap_or_default(), None), v.clone()));
            } else if let Some(u) = updates.get(&first.sha1).filter(|u| loaders.iter().any(|l| u.fits(mc, l, "mods"))) {
                let from_version = p.files[0].1.as_ref().map(|v| v.version_number.clone()).unwrap_or_default();
                entries.push((entry(EntryStatus::Updated, u.file("mods", true), u.version_number.clone(), Some(format!("{from_version} → {}", u.version_number))), Some(u.clone())));
            } else {
                entries.push((entry(EntryStatus::Missing, None, String::new(), Some(format!("No version for Minecraft {mc} with {loader}."))), None));
            }
        }

        // Required dependencies that no pack brought.
        for _ in 0..4 {
            let have: HashSet<String> = entries.iter().filter(|(e, _)| e.file.is_some()).filter_map(|(e, _)| e.project.clone()).collect();
            let missing: Vec<(String, String)> = entries
                .iter()
                .filter(|(e, _)| e.file.is_some())
                .flat_map(|(e, v)| v.iter().flat_map(|v| v.dependencies.iter()).filter(|d| d.dependency_type == "required").filter_map(|d| d.project_id.clone()).map(|d| (d, e.project.clone().unwrap_or_else(|| e.title.clone()))).collect::<Vec<_>>())
                .filter(|(d, _)| !have.contains(d))
                .collect();
            if missing.is_empty() {
                break;
            }
            let mut added = HashSet::new();
            for (dep, by) in missing {
                if !added.insert(dep.clone()) {
                    continue;
                }
                let found = self.api.versions_for(&dep, mc, &loaders)?.into_iter().next();
                match found {
                    Some(v) => entries.push((Entry { project: Some(dep.clone()), title: dep.clone(), icon: None, folder: "mods".into(), version: v.version_number.clone(), file: v.file("mods", true), from: vec![], status: EntryStatus::Dependency, note: Some(format!("Needed by {by}.")) }, Some(v))),
                    None => problems.push(format!("{by} needs a mod ({dep}) that has no version for Minecraft {mc} with {loader}.")),
                }
            }
        }

        // Mods that refuse to run with each other (either side saying so):
        // the one from the earlier pack stays.
        let mut dropped: HashMap<usize, String> = HashMap::new();
        for i in 0..entries.len() {
            let bad: Vec<String> = entries[i].1.iter().flat_map(|v| v.dependencies.iter()).filter(|d| d.dependency_type == "incompatible").filter_map(|d| d.project_id.clone()).collect();
            for j in 0..entries.len() {
                if i == j || entries[j].0.file.is_none() || entries[i].0.file.is_none() || !entries[j].0.project.as_ref().is_some_and(|p| bad.contains(p)) {
                    continue;
                }
                let (keep, lose) = (i.min(j), i.max(j));
                if dropped.contains_key(&keep) || dropped.contains_key(&lose) {
                    continue;
                }
                dropped.insert(lose, entries[keep].0.project.clone().unwrap_or_else(|| entries[keep].0.title.clone()));
            }
        }
        for (j, by) in &dropped {
            let e = &mut entries[*j].0;
            e.status = EntryStatus::Conflict;
            e.note = Some(format!("Doesn't run alongside {by}, which stays."));
            e.file = None;
        }

        // Names and icons.
        let ids: Vec<String> = entries.iter().filter_map(|(e, _)| e.project.clone()).collect::<HashSet<_>>().into_iter().collect();
        if let Ok(info) = self.api.projects(&ids) {
            for (e, _) in &mut entries {
                if let Some((t, i)) = e.project.as_ref().and_then(|p| info.get(p)) {
                    e.title = t.clone();
                    e.icon = i.clone();
                }
            }
            // Notes and problems named projects by id before names were known.
            let named = |text: &str| info.iter().fold(text.to_string(), |acc, (id, (t, _))| acc.replace(id.as_str(), t));
            for (e, _) in &mut entries {
                e.note = e.note.as_deref().map(named);
            }
            for p in &mut problems {
                *p = named(p);
            }
        }

        // Configs and other files: merged, the first pack winning.
        let id = uuid::Uuid::new_v4().simple().to_string();
        let md = self.mdir(&id)?;
        let over = md.join("overrides");
        std::fs::create_dir_all(&over).map_err(|e| e.to_string())?;
        let mut owner: HashMap<String, String> = HashMap::new();
        let mut clashes = Vec::new();
        let mut inside: Vec<Entry> = Vec::new();
        for s in &sources {
            let sd = self.sdir(&s.id)?.join("overrides");
            for f in super::install::files_in(&sd) {
                let key = f.to_lowercase();
                // A mod jar inside a pack's own files can't be moved to another version.
                if key.starts_with("mods/") && key.ends_with(".jar") {
                    let same = s.mc == mc && s.loader == loader;
                    let title = f.rsplit('/').next().unwrap_or(&f).trim_end_matches(".jar").to_string();
                    let status = if same { EntryStatus::AsIs } else { EntryStatus::Missing };
                    let note = if same { format!("Inside {}'s own files: kept as it is.", s.name) } else { format!("Inside {}'s own files and not on Modrinth, so it can't be moved to Minecraft {mc}.", s.name) };
                    inside.push(Entry { project: None, title, icon: None, folder: "mods".into(), version: String::new(), file: None, from: vec![s.name.clone()], status, note: Some(note) });
                    if !same {
                        continue;
                    }
                }
                if let Some(first) = owner.get(&key) {
                    if std::fs::read(sd.join(&f)).ok() != std::fs::read(over.join(&f)).ok() {
                        clashes.push(format!("{f}: kept {first}'s, not {}'s", s.name));
                    }
                    continue;
                }
                let out = over.join(&f);
                std::fs::create_dir_all(out.parent().unwrap_or(&over)).map_err(|e| e.to_string())?;
                std::fs::copy(sd.join(&f), &out).map_err(|e| e.to_string())?;
                owner.insert(key, s.name.clone());
            }
        }
        // A mod in the overrides that the mix also lists would load twice.
        for (e, _) in &entries {
            if let Some(f) = &e.file {
                let _ = std::fs::remove_file(over.join(&f.path));
            }
        }
        let mut entries: Vec<Entry> = entries.into_iter().map(|(e, _)| e).chain(inside).collect();
        entries.sort_by_key(|e| (matches!(e.status, EntryStatus::Missing | EntryStatus::Conflict), e.folder != "mods", e.title.to_lowercase()));
        let mix = Mix {
            id,
            name: if name.trim().is_empty() { sources.iter().map(|s| s.name.clone()).collect::<Vec<_>>().join(" + ") } else { name.trim().chars().take(60).collect() },
            mc: mc.into(),
            loader: loader.into(),
            loader_version,
            sources: sources.iter().map(|s| s.name.clone()).collect(),
            entries,
            clashes,
            problems,
            installable: matches!(loader, "fabric" | "quilt"),
            source_ids: sources.iter().map(|s| s.id.clone()).collect(),
            cover: None,
        };
        std::fs::write(md.join("mix.json"), serde_json::to_vec_pretty(&mix).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        Ok(mix)
    }

    fn keep_mix(&self, m: &Mix) -> Result<(), String> {
        std::fs::write(self.mdir(&m.id)?.join("mix.json"), serde_json::to_vec_pretty(m).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    /// A pack's own picture: Modrinth's icon, or one an instance folder has.
    fn icon_of(&self, s: &Source) -> Option<Vec<u8>> {
        let cache = self.sdir(&s.id).ok()?.join("icon");
        if let Ok(b) = std::fs::read(&cache) {
            return Some(b).filter(|b| super::cover::decode(b).is_some());
        }
        let bytes = match (&s.icon, &s.kind) {
            (Some(url), _) => self.api.picture(url).ok(),
            (None, SourceKind::Folder { path } | SourceKind::Launcher { path, .. }) => folder_icon(Path::new(path)),
            _ => None,
        }?;
        super::cover::decode(&bytes)?;
        let _ = std::fs::write(&cache, &bytes);
        Some(bytes)
    }

    /// Each pack of a mix as a cover tile, in order.
    fn tiles(&self, m: &Mix) -> Vec<image::RgbaImage> {
        let all = self.sources();
        let list: Vec<Source> = if m.source_ids.is_empty() {
            m.sources.iter().filter_map(|n| all.iter().find(|s| &s.name == n).cloned()).collect()
        } else {
            m.source_ids.iter().filter_map(|id| all.iter().find(|s| &s.id == id).cloned()).collect()
        };
        let mut out: Vec<image::RgbaImage> = list.iter().map(|s| self.icon_of(s).and_then(|b| super::cover::decode(&b)).map(|i| super::cover::tile(&i)).unwrap_or_else(|| super::cover::placeholder(&s.name))).collect();
        if out.is_empty() {
            out = m.sources.iter().map(|n| super::cover::placeholder(n)).collect();
        }
        out
    }

    /// The mix's cover (PNG), made from its packs' pictures the first time.
    pub fn cover(&self, id: &str) -> Result<Vec<u8>, String> {
        match std::fs::read(self.mdir(id)?.join("cover.png")) {
            Ok(b) => Ok(b),
            Err(_) => self.remake_cover(id),
        }
    }

    /// Blend the packs' pictures into the cover again, here.
    pub fn remake_cover(&self, id: &str) -> Result<Vec<u8>, String> {
        let mut m = self.mix(id)?;
        let png = super::cover::png(&super::cover::blend(&self.tiles(&m)));
        std::fs::write(self.mdir(id)?.join("cover.png"), &png).map_err(|e| e.to_string())?;
        m.cover = Some("blend".into());
        self.keep_mix(&m)?;
        Ok(png)
    }

    /// Have Pollinations paint one new cover from the packs' pictures, with
    /// the user's own account.
    pub fn ai_cover(&self, id: &str, p: &super::cover::Pollinations) -> Result<Vec<u8>, String> {
        let mut m = self.mix(id)?;
        let tiles: Vec<Vec<u8>> = self.tiles(&m).iter().map(|t| super::cover::png(&image::imageops::resize(t, 256, 256, image::imageops::FilterType::Triangle))).collect();
        let png = p.blend(&tiles)?;
        std::fs::write(self.mdir(id)?.join("cover.png"), &png).map_err(|e| e.to_string())?;
        m.cover = Some("ai".into());
        self.keep_mix(&m)?;
        Ok(png)
    }

    /// A mix's merged config files (overrides/).
    pub fn overrides_dir(&self, id: &str) -> Result<PathBuf, String> {
        Ok(self.mdir(id)?.join("overrides"))
    }

    /// Mixes worked out so far, newest first (the newest of each name).
    pub fn mixes(&self) -> Vec<Mix> {
        let mut v: Vec<(std::time::SystemTime, Mix)> = std::fs::read_dir(self.dir.join("mixes"))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let p = e.path().join("mix.json");
                let at = std::fs::metadata(&p).and_then(|m| m.modified()).ok()?;
                Some((at, serde_json::from_slice::<Mix>(&std::fs::read(&p).ok()?).ok()?))
            })
            .collect();
        v.sort_by_key(|x| std::cmp::Reverse(x.0));
        let mut seen = HashSet::new();
        v.into_iter().map(|(_, m)| m).filter(|m| seen.insert(m.name.to_lowercase())).collect()
    }

    pub fn mix(&self, id: &str) -> Result<Mix, String> {
        std::fs::read(self.mdir(id)?.join("mix.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).ok_or_else(|| "That mix is gone; work it out again.".into())
    }

    /// Save a mix as a .mrpack at `dest`.
    pub fn export(&self, id: &str, dest: &Path, server: Option<&ServerEntry>) -> Result<(), String> {
        let servers = server.map(servers_dat).transpose()?;
        // The cover rides along as icon.png beside the index (launchers skip
        // files there); OmniHub on a friend's PC uses it for the mix again.
        let cover = self.cover(id).ok();
        let m = self.mix(id)?;
        let key = loader_key(&m.loader).ok_or("No loader.")?;
        if m.loader_version.is_empty() {
            return Err(m.problems.first().cloned().unwrap_or_else(|| "The loader version is unknown.".into()));
        }
        let over = self.mdir(id)?.join("overrides");
        let mut files = Vec::new();
        let mut local = Vec::new();
        for e in &m.entries {
            let Some(f) = &e.file else { continue };
            match &f.local {
                Some(p) if f.urls.is_empty() => local.push((f.path.clone(), PathBuf::from(p))),
                _ => files.push(json!({ "path": f.path, "hashes": { "sha1": f.sha1, "sha512": f.sha512 }, "env": { "client": "required", "server": "required" }, "downloads": f.urls, "fileSize": f.size })),
            }
        }
        let index = json!({
            "formatVersion": 1,
            "game": "minecraft",
            "versionId": "1.0.0",
            "name": m.name,
            "summary": format!("Mixed with OmniHub from {}.", m.sources.join(", ")),
            "files": files,
            "dependencies": { "minecraft": m.mc, key: m.loader_version },
        });
        let tmp = dest.with_extension("mrpack.part");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&tmp).map_err(|e| e.to_string())?);
            let o = zip::write::SimpleFileOptions::default();
            z.start_file("modrinth.index.json", o).map_err(|e| e.to_string())?;
            z.write_all(serde_json::to_string_pretty(&index).unwrap_or_default().as_bytes()).map_err(|e| e.to_string())?;
            for f in super::install::files_in(&over) {
                z.start_file(format!("overrides/{f}"), o).map_err(|e| e.to_string())?;
                z.write_all(&std::fs::read(over.join(&f)).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            }
            for (path, p) in local {
                z.start_file(format!("overrides/{path}"), o).map_err(|e| e.to_string())?;
                z.write_all(&std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?).map_err(|e| e.to_string())?;
            }
            if let Some(b) = &servers {
                z.start_file("overrides/servers.dat", o).map_err(|e| e.to_string())?;
                z.write_all(b).map_err(|e| e.to_string())?;
            }
            if let Some(png) = &cover {
                z.start_file("icon.png", o).map_err(|e| e.to_string())?;
                z.write_all(png).map_err(|e| e.to_string())?;
            }
            z.finish().map_err(|e| e.to_string())?;
        }
        std::fs::rename(&tmp, dest).map_err(|e| e.to_string())
    }

    /// A pack a friend shared (a .mrpack, from OmniHub or anywhere): added as
    /// a pack and made into a mix of its own, ready to install, with the
    /// friend's cover when the file has one.
    pub fn import_shared(&self, path: &str) -> Result<Mix, String> {
        let b = std::fs::read(path).map_err(|e| format!("Couldn't open it: {e}"))?;
        let s = self.add_mrpack(&b, None, None, SourceKind::File { path: path.into() })?;
        let picture = zip::ZipArchive::new(std::io::Cursor::new(&b)).ok().and_then(|mut z| {
            let mut f = z.by_name("icon.png").ok()?;
            let mut v = Vec::new();
            (&mut f).take(8 << 20).read_to_end(&mut v).ok()?;
            super::cover::decode(&v)
        });
        if let Some(img) = &picture {
            let _ = std::fs::write(self.sdir(&s.id)?.join("icon"), super::cover::png(img));
        }
        let m = self.plan(&s.name, std::slice::from_ref(&s.id), &s.mc, &s.loader)?;
        if let Some(img) = &picture {
            std::fs::write(self.mdir(&m.id)?.join("cover.png"), super::cover::png(img)).map_err(|e| e.to_string())?;
            let mut m = self.mix(&m.id)?;
            m.cover = Some("shared".into());
            self.keep_mix(&m)?;
        }
        self.mix(&m.id)
    }

    fn installs_file(&self) -> PathBuf {
        self.dir.join("installed.json")
    }

    pub fn installed(&self) -> Vec<Installed> {
        std::fs::read(self.installs_file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    /// Install a Fabric or Quilt mix into the Minecraft Launcher: its own
    /// folder under .minecraft\omnihub, the loader's version file, and a profile.
    pub fn install(&self, id: &str, progress: &mut dyn FnMut(usize, usize, &str), memory_gb: u32) -> Result<Installed, String> {
        let m = self.mix(id)?;
        if !m.installable {
            return Err("Only Fabric and Quilt mixes go straight into the Minecraft Launcher. Save it as a .mrpack and open it in Modrinth App or Prism Launcher instead.".into());
        }
        if m.loader_version.is_empty() {
            return Err(m.problems.first().cloned().unwrap_or_else(|| "The loader version is unknown.".into()));
        }
        let mc_dir = self.minecraft.clone().ok_or("Minecraft (Java Edition) isn't installed: its .minecraft folder wasn't found.")?;
        let profile = self.api.profile_json(&m.loader, &m.mc, &m.loader_version)?;
        let version_id = profile["id"].as_str().filter(|s| super::install::safe_rel(s).is_some_and(|r| !r.contains('/'))).ok_or("The loader's profile has no id.")?.to_string();
        let slug = super::slug(&m.name);
        let mut dir = mc_dir.join("omnihub").join(&slug);
        let mut n = 2;
        while dir.exists() {
            dir = mc_dir.join("omnihub").join(format!("{slug}_{n}"));
            n += 1;
        }
        let tmp = dir.with_extension("part");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        let res = (|| -> Result<usize, String> {
            let files: Vec<&PackFile> = m.entries.iter().filter_map(|e| e.file.as_ref()).collect();
            let total = files.len();
            for (i, f) in files.iter().enumerate() {
                progress(i, total, f.path.rsplit('/').next().unwrap_or(&f.path));
                let bytes = match (&f.local, f.urls.is_empty()) {
                    (Some(p), true) => std::fs::read(p).map_err(|e| e.to_string())?,
                    _ => self.api.download(&f.urls, &f.sha1, &f.sha512)?,
                };
                let out = tmp.join(super::install::safe_rel(&f.path).ok_or("A file path in the mix isn't safe.")?);
                std::fs::create_dir_all(out.parent().unwrap_or(&tmp)).map_err(|e| e.to_string())?;
                std::fs::write(out, bytes).map_err(|e| e.to_string())?;
            }
            let over = self.mdir(id)?.join("overrides");
            if over.is_dir() {
                super::install::unpack(&over, &tmp)?;
            }
            progress(total, total, "");
            Ok(files.iter().filter(|f| f.path.starts_with("mods/")).count())
        })();
        let mods = match res {
            Ok(n) => n,
            Err(e) => {
                let _ = std::fs::remove_dir_all(&tmp);
                return Err(e);
            }
        };
        std::fs::rename(&tmp, &dir).map_err(|e| e.to_string())?;
        // The loader's version file (shared by every profile that uses it).
        let vdir = mc_dir.join("versions").join(&version_id);
        if !vdir.join(format!("{version_id}.json")).is_file() {
            std::fs::create_dir_all(&vdir).map_err(|e| e.to_string())?;
            std::fs::write(vdir.join(format!("{version_id}.json")), serde_json::to_vec_pretty(&profile).unwrap_or_default()).map_err(|e| e.to_string())?;
        }
        let key = format!("omnihub-{}", dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or(slug));
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let java = format!("-Xmx{memory_gb}G -XX:+UnlockExperimentalVMOptions -XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M");
        let icon = self.cover(id).ok().and_then(|c| super::cover::launcher_icon(&c)).unwrap_or_else(|| "Crafting_Table".into());
        let entry = json!({ "name": m.name, "type": "custom", "created": now, "lastUsed": now, "lastVersionId": version_id, "gameDir": dir.to_string_lossy(), "icon": icon, "javaArgs": java });
        let mut wrote = false;
        for file in ["launcher_profiles.json", "launcher_profiles_microsoft_store.json"] {
            let p = mc_dir.join(file);
            if !p.is_file() && file != "launcher_profiles.json" {
                continue;
            }
            let mut v: Value = std::fs::read(&p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_else(|| json!({ "profiles": {}, "settings": {}, "version": 3 }));
            if p.is_file() {
                let backup = mc_dir.join(format!("{file}.omnihub-backup"));
                if !backup.exists() {
                    let _ = std::fs::copy(&p, &backup);
                }
            }
            if !v["profiles"].is_object() {
                v["profiles"] = json!({});
            }
            v["profiles"][&key] = entry.clone();
            let t = p.with_extension("json.omnihub");
            std::fs::write(&t, serde_json::to_vec_pretty(&v).unwrap_or_default()).map_err(|e| e.to_string())?;
            std::fs::rename(&t, &p).map_err(|e| e.to_string())?;
            wrote = true;
        }
        if !wrote {
            return Err("The Minecraft Launcher's profile list couldn't be written.".into());
        }
        let inst = Installed { name: m.name.clone(), profile: key, dir: dir.to_string_lossy().into_owned(), version_id, mc: m.mc.clone(), loader: m.loader.clone(), mods, at: crate::db::now() };
        let mut all = self.installed();
        all.push(inst.clone());
        let _ = std::fs::write(self.installs_file(), serde_json::to_vec_pretty(&all).unwrap_or_default());
        Ok(inst)
    }

    /// Remove an installed mix: its folder and its launcher profile.
    pub fn uninstall(&self, profile: &str) -> Result<(), String> {
        let mut all = self.installed();
        let i = all.iter().position(|x| x.profile == profile).ok_or("That mix isn't installed any more.")?;
        let inst = all.remove(i);
        let mc_dir = self.minecraft.clone().ok_or("Minecraft's folder wasn't found.")?;
        let dir = PathBuf::from(&inst.dir);
        if dir.starts_with(mc_dir.join("omnihub")) && dir.is_dir() {
            std::fs::remove_dir_all(&dir).map_err(|e| format!("Couldn't remove its folder (is Minecraft running?): {e}"))?;
        }
        for file in ["launcher_profiles.json", "launcher_profiles_microsoft_store.json"] {
            let p = mc_dir.join(file);
            if let Some(mut v) = std::fs::read(&p).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok()) {
                if let Some(o) = v["profiles"].as_object_mut() {
                    o.remove(profile);
                }
                let _ = std::fs::write(&p, serde_json::to_vec_pretty(&v).unwrap_or_default());
            }
        }
        std::fs::write(self.installs_file(), serde_json::to_vec_pretty(&all).unwrap_or_default()).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::tests::{serve, zip_of, Seen};
    use std::sync::Arc;

    #[allow(clippy::too_many_arguments)]
    fn ver(project: &str, number: &str, mc: &[&str], loaders: &[&str], file: (&str, &[u8]), base: &str, deps: Value, date: &str) -> Value {
        let (sha1, sha512) = sha_pair(file.1);
        json!({ "project_id": project, "version_number": number, "game_versions": mc, "loaders": loaders, "date_published": date,
            "files": [{ "hashes": { "sha1": sha1, "sha512": sha512 }, "url": format!("{base}/cdn/{}", file.0), "filename": file.0, "primary": true, "size": file.1.len() }],
            "dependencies": deps })
    }

    fn mrpack(name: &str, mc: &str, loader: (&str, &str), files: &[(&str, &[u8])], base: &str, overrides: &[(&str, &[u8])]) -> Vec<u8> {
        let list: Vec<Value> = files.iter().map(|(n, b)| {
            let (s1, s5) = sha_pair(b);
            json!({ "path": format!("mods/{n}"), "hashes": { "sha1": s1, "sha512": s5 }, "env": { "client": "required", "server": "required" }, "downloads": [format!("{base}/cdn/{n}")], "fileSize": b.len() })
        }).collect();
        let index = json!({ "formatVersion": 1, "game": "minecraft", "versionId": "1", "name": name, "files": list, "dependencies": { "minecraft": mc, loader.0: loader.1 } });
        let mut all: Vec<(String, Vec<u8>)> = vec![("modrinth.index.json".into(), index.to_string().into_bytes())];
        for (p, b) in overrides {
            all.push((format!("overrides/{p}"), b.to_vec()));
        }
        let refs: Vec<(&str, &[u8])> = all.iter().map(|(a, b)| (a.as_str(), b.as_slice())).collect();
        zip_of(&refs)
    }

    #[test]
    fn two_packs_mix_into_one_for_a_chosen_version() {
        // Pack A (1.21.1 Fabric): Sodium, Iris, Lithium. Pack B (1.20.1 Fabric): Sodium (old), Create, Embeddium (refuses Sodium).
        let sodium_new: &[u8] = b"sodium 0.6 for 1.21.1";
        let sodium_old: &[u8] = b"sodium 0.5 for 1.20.1";
        let iris: &[u8] = b"iris 1.8";
        let lithium: &[u8] = b"lithium";
        let create_old: &[u8] = b"create for 1.20.1";
        let create_new: &[u8] = b"create for 1.21.1";
        let embeddium: &[u8] = b"embeddium";
        let fapi: &[u8] = b"fabric api";
        let base_cell: Arc<parking_lot::Mutex<String>> = Arc::default();
        let bc = base_cell.clone();
        let files: Vec<(&'static str, &'static [u8])> = vec![("sodium-0.6.jar", sodium_new), ("sodium-0.5.jar", sodium_old), ("iris.jar", iris), ("lithium.jar", lithium), ("create-1.20.1.jar", create_old), ("create-1.21.1.jar", create_new), ("embeddium.jar", embeddium), ("fabric-api.jar", fapi)];
        let s = serve(Box::new(move |r: &Seen| {
            let base = bc.lock().clone();
            let body: Value = serde_json::from_slice(&r.body).unwrap_or(Value::Null);
            let by_sha = |b: &[u8]| sha_pair(b).0;
            let v = |p, n, mc: &[&str], f: (&str, &[u8]), deps: Value, date| ver(p, n, mc, &["fabric"], f, &base, deps, date);
            let known: Vec<(String, Value)> = vec![
                (by_sha(sodium_new), v("AANobbMI", "0.6.0", &["1.21.1"], ("sodium-0.6.jar", sodium_new), json!([]), "2026-05-01")),
                (by_sha(sodium_old), v("AANobbMI", "0.5.8", &["1.20.1"], ("sodium-0.5.jar", sodium_old), json!([]), "2024-01-01")),
                (by_sha(iris), v("YL57xq9U", "1.8.0", &["1.21.1"], ("iris.jar", iris), json!([{ "project_id": "AANobbMI", "dependency_type": "required" }]), "2026-05-02")),
                (by_sha(lithium), v("gvQqBUqZ", "0.14", &["1.21.1"], ("lithium.jar", lithium), json!([]), "2026-05-03")),
                (by_sha(create_old), v("LNytGWDc", "6.0.1", &["1.20.1"], ("create-1.20.1.jar", create_old), json!([]), "2025-01-01")),
                (by_sha(embeddium), v("sk9rgfiA", "1.0", &["1.20.1"], ("embeddium.jar", embeddium), json!([{ "project_id": "AANobbMI", "dependency_type": "incompatible" }]), "2025-01-01")),
            ];
            let p = r.path.as_str();
            let ok = |v: Value| Some((200, "application/json", v.to_string().into_bytes()));
            if p == "/v2/version_files" {
                let want: Vec<String> = serde_json::from_value(body["hashes"].clone()).unwrap();
                return ok(Value::Object(known.iter().filter(|(h, _)| want.contains(h)).map(|(h, v)| (h.clone(), v.clone())).collect()));
            }
            if p == "/v2/version_files/update" {
                let want: Vec<String> = serde_json::from_value(body["hashes"].clone()).unwrap();
                let mut out = serde_json::Map::new();
                if body["game_versions"] == json!(["1.21.1"]) {
                    for h in want {
                        if h == by_sha(create_old) {
                            out.insert(h, v("LNytGWDc", "6.0.6", &["1.21.1"], ("create-1.21.1.jar", create_new), json!([{ "project_id": "P7dR8mSH", "dependency_type": "required" }]), "2026-06-01"));
                        } else if h == by_sha(embeddium) {
                            out.insert(h, v("sk9rgfiA", "1.1", &["1.21.1"], ("embeddium.jar", embeddium), json!([{ "project_id": "AANobbMI", "dependency_type": "incompatible" }]), "2026-06-01"));
                        }
                    }
                }
                return ok(Value::Object(out));
            }
            if p.starts_with("/v2/project/P7dR8mSH/version") {
                return ok(json!([v("P7dR8mSH", "0.110.0", &["1.21.1"], ("fabric-api.jar", fapi), json!([]), "2026-06-02")]));
            }
            if p.starts_with("/v2/projects?") {
                return ok(json!([{ "id": "AANobbMI", "title": "Sodium" }, { "id": "YL57xq9U", "title": "Iris" }, { "id": "gvQqBUqZ", "title": "Lithium" }, { "id": "LNytGWDc", "title": "Create" }, { "id": "sk9rgfiA", "title": "Embeddium" }, { "id": "P7dR8mSH", "title": "Fabric API" }]));
            }
            if p == "/v2/versions/loader/1.21.1/0.16.9/profile/json" {
                return ok(json!({ "id": "fabric-loader-0.16.9-1.21.1", "inheritsFrom": "1.21.1", "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient" }));
            }
            if let Some(name) = p.strip_prefix("/cdn/") {
                return files.iter().find(|(n, _)| *n == name).map(|(_, b)| (200, "application/java-archive", b.to_vec()));
            }
            None
        }));
        *base_cell.lock() = s.base.clone();
        let base = s.base.clone();
        let t = tempfile::tempdir().unwrap();
        let mc_dir = t.path().join(".minecraft");
        std::fs::create_dir_all(&mc_dir).unwrap();
        std::fs::write(mc_dir.join("launcher_profiles.json"), r#"{"profiles":{"abc":{"name":"Latest release","type":"latest-release"}},"settings":{},"version":3}"#).unwrap();
        let mixer = Mixer::new(&t.path().join("mc"), Modrinth::new(&base, &base, &base), Some(mc_dir.clone()));

        let a = t.path().join("a.mrpack");
        std::fs::write(&a, mrpack("Speed", "1.21.1", ("fabric-loader", "0.16.9"), &[("sodium-0.6.jar", sodium_new), ("iris.jar", iris), ("lithium.jar", lithium)], &base, &[("config/sodium.json", b"{\"a\":1}"), ("options.txt", b"fov:90")])).unwrap();
        let b = t.path().join("b.mrpack");
        std::fs::write(&b, mrpack("Create Plus", "1.20.1", ("fabric-loader", "0.15.0"), &[("sodium-0.5.jar", sodium_old), ("create-1.20.1.jar", create_old), ("embeddium.jar", embeddium)], &base, &[("config/sodium.json", b"{\"a\":2}"), ("config/create.toml", b"x=1"), ("mods/extra.jar", b"stray")])).unwrap();
        let sa = mixer.add_file(a.to_str().unwrap()).unwrap();
        let sb = mixer.add_file(b.to_str().unwrap()).unwrap();
        assert_eq!((sa.mc.as_str(), sa.loader.as_str(), sa.loader_version.as_str(), sa.files.len(), sa.overrides), ("1.21.1", "fabric", "0.16.9", 3, 2));
        assert_eq!(mixer.sources().len(), 2);

        let mix = mixer.plan("", &[sa.id.clone(), sb.id.clone()], "1.21.1", "fabric").unwrap();
        assert_eq!(mix.name, "Speed + Create Plus");
        assert_eq!(mix.loader_version, "0.16.9");
        let st = |title: &str| mix.entries.iter().find(|e| e.title == title).map(|e| (e.status, e.version.clone(), e.from.clone())).unwrap();
        assert_eq!(st("Sodium"), (EntryStatus::Kept, "0.6.0".into(), vec!["Speed".to_string(), "Create Plus".to_string()]), "once, the copy for 1.21.1");
        assert_eq!(st("Create"), (EntryStatus::Updated, "6.0.6".into(), vec!["Create Plus".to_string()]));
        assert_eq!(st("Fabric API").0, EntryStatus::Dependency);
        assert_eq!(st("Embeddium").0, EntryStatus::Conflict, "it refuses Sodium, which came first");
        assert_eq!(st("extra").0, EntryStatus::Missing, "a jar inside a 1.20.1 pack's own files can't move");
        assert_eq!(mix.entries.iter().filter(|e| e.file.is_some()).count(), 5);
        assert_eq!(mix.entries.iter().find(|e| e.title == "Fabric API").unwrap().note.as_deref(), Some("Needed by Create."));
        assert_eq!(mix.clashes, ["config/sodium.json: kept Speed's, not Create Plus's"]);
        assert!(mix.installable);

        // As a .mrpack.
        let out = t.path().join("mix.mrpack");
        mixer.export(&mix.id, &out, None).unwrap();
        let (idx, n) = read_mrpack(&std::fs::read(&out).unwrap()).unwrap();
        assert_eq!(idx["dependencies"], json!({ "minecraft": "1.21.1", "fabric-loader": "0.16.9" }));
        assert_eq!(idx["files"].as_array().unwrap().len(), 5);
        assert_eq!(n, 3, "Speed's sodium.json, create.toml and options.txt");

        // Its cover: the two packs (no icons in .mrpack files) blended, kept with the mix.
        let cover = mixer.cover(&mix.id).unwrap();
        assert_eq!(crate::mods::cover::decode(&cover).unwrap().dimensions(), (crate::mods::cover::SIZE, crate::mods::cover::SIZE));
        assert_eq!(mixer.mix(&mix.id).unwrap().cover.as_deref(), Some("blend"));
        assert_eq!(mixer.mix(&mix.id).unwrap().source_ids, [sa.id.clone(), sb.id.clone()]);
        assert_eq!(mixer.cover(&mix.id).unwrap(), cover, "made once, then kept");

        // Shared with a friend: the file carries the cover, and their OmniHub
        // makes the same mix with the same picture.
        let shared = t.path().join("for-a-friend.mrpack");
        mixer.export(&mix.id, &shared, Some(&ServerEntry { name: "Our server".into(), address: "play.example.net".into() })).unwrap();
        let mut z = zip::ZipArchive::new(std::fs::File::open(&shared).unwrap()).unwrap();
        let mut inside = Vec::new();
        z.by_name("icon.png").unwrap().read_to_end(&mut inside).unwrap();
        assert_eq!(inside, cover);
        assert!(z.by_name("overrides/servers.dat").is_ok());
        let friend = Mixer::new(&t.path().join("friend"), Modrinth::new(&base, &base, &base), Some(t.path().join("friend-mc")));
        let theirs = friend.import_shared(shared.to_str().unwrap()).unwrap();
        assert_eq!((theirs.mc.as_str(), theirs.loader.as_str(), theirs.cover.as_deref()), ("1.21.1", "fabric", Some("shared")));
        assert_eq!(theirs.entries.iter().filter(|e| e.file.is_some()).count(), 5);
        assert_eq!(crate::mods::cover::decode(&friend.cover(&theirs.id).unwrap()).unwrap(), crate::mods::cover::decode(&cover).unwrap());

        // Into the Minecraft Launcher.
        let mut steps = 0;
        let inst = mixer.install(&mix.id, &mut |_, _, _| steps += 1, 6).unwrap();
        let dir = PathBuf::from(&inst.dir);
        assert!(dir.join("mods/sodium-0.6.jar").is_file() && dir.join("mods/create-1.21.1.jar").is_file() && dir.join("mods/fabric-api.jar").is_file());
        assert!(!dir.join("mods/embeddium.jar").exists());
        assert_eq!(std::fs::read(dir.join("config/sodium.json")).unwrap(), b"{\"a\":1}");
        assert!(mc_dir.join("versions/fabric-loader-0.16.9-1.21.1/fabric-loader-0.16.9-1.21.1.json").is_file());
        let lp: Value = serde_json::from_slice(&std::fs::read(mc_dir.join("launcher_profiles.json")).unwrap()).unwrap();
        let p = &lp["profiles"][&inst.profile];
        assert_eq!((p["lastVersionId"].as_str(), p["type"].as_str()), (Some("fabric-loader-0.16.9-1.21.1"), Some("custom")));
        assert!(p["javaArgs"].as_str().unwrap().starts_with("-Xmx6G"));
        assert!(p["icon"].as_str().unwrap().starts_with("data:image/png;base64,"), "the mix's cover, not the crafting table");
        assert!(lp["profiles"]["abc"].is_object(), "the user's own profiles stay");
        assert!(mc_dir.join("launcher_profiles.json.omnihub-backup").is_file());
        assert!(steps > 5);
        mixer.uninstall(&inst.profile).unwrap();
        assert!(!dir.exists());
        let lp: Value = serde_json::from_slice(&std::fs::read(mc_dir.join("launcher_profiles.json")).unwrap()).unwrap();
        assert!(lp["profiles"].get(&inst.profile).is_none() && lp["profiles"]["abc"].is_object());

        // Moving everything to 1.20.1 instead: Iris and Lithium have no copy for it.
        let back = mixer.plan("Old", &[sa.id.clone(), sb.id.clone()], "1.20.1", "fabric").unwrap();
        let bst = |title: &str| back.entries.iter().find(|e| e.title == title).map(|e| e.status).unwrap();
        assert_eq!((bst("Sodium"), bst("Create"), bst("Iris"), bst("Lithium"), bst("Embeddium")), (EntryStatus::Kept, EntryStatus::Kept, EntryStatus::Missing, EntryStatus::Missing, EntryStatus::Conflict));
        assert_eq!(back.loader_version, "0.15.0");

        // A tampered download is refused.
        assert!(mixer.api.download(&[format!("{base}/cdn/iris.jar")], "", &"0".repeat(128)).unwrap_err().contains("checksum"));
        assert!(mixer.api.download(&["https://evil.example/x.jar".into()], "", "").unwrap_err().contains("host"));
        assert_eq!(version_cmp("26.1", "1.21.10"), std::cmp::Ordering::Greater);
        assert_eq!(version_cmp("1.21.10", "1.21.9"), std::cmp::Ordering::Greater);
    }

    #[test]
    fn pack_pictures_come_from_modrinth_or_the_instance_folder() {
        let t = tempfile::tempdir().unwrap();
        let pic = crate::mods::cover::png(&image::RgbaImage::from_pixel(64, 64, image::Rgba([10, 200, 30, 255])));
        // Prism Launcher: instances/<name>/.minecraft, its icon named in instance.cfg.
        let inst = t.path().join("PrismLauncher/instances/Skyblock");
        std::fs::create_dir_all(inst.join(".minecraft/mods")).unwrap();
        std::fs::create_dir_all(t.path().join("PrismLauncher/icons")).unwrap();
        std::fs::write(inst.join("instance.cfg"), "[General]\niconKey=sky_icon\nname=Skyblock\n").unwrap();
        std::fs::write(t.path().join("PrismLauncher/icons/sky_icon.png"), &pic).unwrap();
        assert_eq!(folder_icon(&inst).as_deref(), Some(pic.as_slice()));
        assert_eq!(folder_icon(&inst.join(".minecraft")).as_deref(), Some(pic.as_slice()), "from inside its .minecraft too");
        // An icon key that tries to leave the icons folder is ignored.
        std::fs::write(inst.join("instance.cfg"), "iconKey=../../secret\n").unwrap();
        assert!(folder_icon(&inst).is_none());
        // Modrinth App: profile.json's icon_path.
        let mra = t.path().join("ModrinthApp/profiles/Vanilla+");
        std::fs::create_dir_all(&mra).unwrap();
        std::fs::write(t.path().join("cached.png"), &pic).unwrap();
        std::fs::write(mra.join("profile.json"), json!({ "icon_path": t.path().join("cached.png") }).to_string()).unwrap();
        assert!(folder_icon(&mra).is_some());

        let m = Modrinth::new("http://127.0.0.1:9", "http://127.0.0.1:9", "http://127.0.0.1:9");
        assert!(m.picture("https://evil.example/icon.png").unwrap_err().contains("only come from Modrinth"));
    }

    /// A pack for friends: the same mods (Modrinth's as downloads, the rest
    /// inside), its configs and packs, not your own settings or server list,
    /// and your server in its list; also as a CurseForge zip.
    #[test]
    fn friends_get_the_same_pack() {
        let t = tempfile::tempdir().unwrap();
        let roaming = t.path().join("Roaming");
        let inst = roaming.join("PrismLauncher/instances/Pals");
        let game = inst.join(".minecraft");
        for d in ["mods", "config", "resourcepacks"] {
            std::fs::create_dir_all(game.join(d)).unwrap();
        }
        std::fs::write(inst.join("instance.cfg"), "name=Pals\n").unwrap();
        std::fs::write(inst.join("mmc-pack.json"), r#"{"components":[{"uid":"net.minecraft","version":"1.21.1"},{"uid":"net.fabricmc.fabric-loader","version":"0.16.9"}]}"#).unwrap();
        let sodium = zip_of(&[("fabric.mod.json", br#"{"id":"sodium","name":"Sodium","version":"0.6.0","environment":"client"}"#)]);
        let secret: &[u8] = b"a mod made by a friend";
        std::fs::write(game.join("mods/sodium.jar"), &sodium).unwrap();
        std::fs::write(game.join("mods/secret.jar"), secret).unwrap();
        std::fs::write(game.join("mods/old.jar.disabled"), b"off").unwrap();
        std::fs::write(game.join("config/sodium.json"), b"{}").unwrap();
        std::fs::write(game.join("resourcepacks/x.zip"), b"pack").unwrap();
        std::fs::write(game.join("options.txt"), b"key_key.jump:key.keyboard.space").unwrap();
        std::fs::write(game.join("servers.dat"), b"my own list").unwrap();
        let (sha, newer) = (sha_pair(&sodium).0, b"sodium 0.7".as_slice());
        let base: Arc<parking_lot::Mutex<String>> = Arc::default();
        let b2 = base.clone();
        let (sha2, sodium2) = (sha.clone(), sodium.clone());
        let s = serve(Box::new(move |r: &Seen| {
            let b = b2.lock().clone();
            let ok = |v: Value| Some((200, "application/json", v.to_string().into_bytes()));
            let body: Value = serde_json::from_slice(&r.body).unwrap_or(Value::Null);
            let asked = |h: &str| body["hashes"].as_array().is_some_and(|a| a.iter().any(|x| x == h));
            match r.path.as_str() {
                "/v2/version_files" if asked(&sha2) => ok(json!({ &sha2: ver("AANobbMI", "0.6.0", &["1.21.1"], &["fabric"], ("sodium.jar", &sodium2), &b, json!([]), "2026-05-01") })),
                "/v2/version_files/update" if asked(&sha2) => ok(json!({ &sha2: ver("AANobbMI", "0.7.0", &["1.21.1"], &["fabric"], ("sodium-0.7.jar", newer), &b, json!([]), "2026-09-01") })),
                "/v2/version_files" | "/v2/version_files/update" => ok(json!({})),
                p if p.starts_with("/v2/projects?") => ok(json!([{ "id": "AANobbMI", "title": "Sodium", "server_side": "unsupported" }])),
                _ => None,
            }
        }));
        *base.lock() = s.base.clone();
        let mixer = Mixer::new(&t.path().join("mc"), Modrinth::new(&s.base, &s.base, &s.base), None).looking_in(launchers::Places { roaming: Some(roaming), ..Default::default() });
        let id = mixer.launcher_packs()[0].id.clone();

        let d = mixer.launcher_details(&id).unwrap();
        assert_eq!(d.mods.iter().map(|m| (m.name.as_str(), m.version.as_str(), m.side.as_str(), m.disabled)).collect::<Vec<_>>(), [("old", "", "", true), ("secret", "", "", false), ("Sodium", "0.6.0", "client", false)]);
        assert!(d.worlds.is_empty() && d.size > 0);
        assert_eq!(mixer.launcher_updates(&id).unwrap(), HashMap::from([("sodium.jar".to_string(), "0.7.0".to_string())]));

        let pals = ServerEntry { name: "Pals".into(), address: "203.0.113.5:25565".into() };
        let out = t.path().join("Pals.mrpack");
        let shared = mixer.share_launcher_pack(&id, &out, PackFormat::Mrpack, Some(&pals)).unwrap();
        assert_eq!((shared.mods, shared.from_modrinth, shared.included), (2, 1, 1));
        let mut z = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
        let names: HashSet<String> = z.file_names().map(String::from).collect();
        for n in ["modrinth.index.json", "overrides/mods/secret.jar", "overrides/config/sodium.json", "overrides/resourcepacks/x.zip", "overrides/servers.dat"] {
            assert!(names.contains(n), "{n} is in it: {names:?}");
        }
        for n in ["overrides/mods/sodium.jar", "overrides/mods/old.jar.disabled", "overrides/options.txt"] {
            assert!(!names.contains(n), "{n} stays out");
        }
        let index: Value = serde_json::from_reader(z.by_name("modrinth.index.json").unwrap()).unwrap();
        assert_eq!(index["dependencies"], json!({ "minecraft": "1.21.1", "fabric-loader": "0.16.9" }));
        assert_eq!(index["files"][0]["downloads"], json!([format!("{}/cdn/sodium.jar", s.base)]));
        assert_eq!(index["files"][0]["env"], json!({ "client": "required", "server": "unsupported" }));
        let mut dat = Vec::new();
        z.by_name("overrides/servers.dat").unwrap().read_to_end(&mut dat).unwrap();
        let list = crate::mcserver::nbt::read(&dat).unwrap();
        let first = &list.get("servers").unwrap().list()[0];
        assert_eq!((first.get("name").and_then(|t| t.str()), first.get("ip").and_then(|t| t.str())), (Some("Pals"), Some("203.0.113.5:25565")));

        let cf = t.path().join("Pals.zip");
        let shared = mixer.share_launcher_pack(&id, &cf, PackFormat::Curseforge, None).unwrap();
        assert_eq!((shared.from_modrinth, shared.included), (0, 2));
        let mut z = zip::ZipArchive::new(std::fs::File::open(&cf).unwrap()).unwrap();
        let manifest: Value = serde_json::from_reader(z.by_name("manifest.json").unwrap()).unwrap();
        assert_eq!(manifest["minecraft"], json!({ "version": "1.21.1", "modLoaders": [{ "id": "fabric-0.16.9", "primary": true }] }));
        assert!(z.by_name("overrides/mods/sodium.jar").is_ok() && z.by_name("overrides/servers.dat").is_err());

        let bad = ServerEntry { name: "x".into(), address: "evil server; rm".into() };
        assert!(mixer.share_launcher_pack(&id, &t.path().join("bad.mrpack"), PackFormat::Mrpack, Some(&bad)).unwrap_err().contains("can't be used"));
        assert!(!t.path().join("bad.mrpack").exists() && !t.path().join("bad.part").exists());
        // The instance itself is as it was.
        assert_eq!(std::fs::read(game.join("servers.dat")).unwrap(), b"my own list");
    }

    /// NeoForge and Forge mixes get a loader build even when no pack names
    /// one, so they can be saved as .mrpack; and a mod's download is its own
    /// jar, not another file of the same Modrinth version.
    #[test]
    fn neoforge_mixes_get_a_build_and_the_right_jars() {
        let t = tempfile::tempdir().unwrap();
        let neo_jar: &[u8] = b"the NeoForge jar";
        let forge_jar: &[u8] = b"the Forge jar";
        let (neo_sha, forge_sha) = (sha_pair(neo_jar), sha_pair(forge_jar));
        let base: Arc<parking_lot::Mutex<String>> = Arc::default();
        let b2 = base.clone();
        let (ns, fs) = (neo_sha.clone(), forge_sha.clone());
        let s = serve(Box::new(move |r: &Seen| {
            let b = b2.lock().clone();
            let ok = |v: Value| Some((200, "application/json", v.to_string().into_bytes()));
            match r.path.as_str() {
                "/api/maven/versions/releases/net/neoforged/neoforge" => ok(json!({ "versions": ["21.1.70", "21.1.77", "21.1.80-beta", "21.0.167", "26.1.2.100", "26.1.2.122"] })),
                "/net/minecraftforge/forge/promotions_slim.json" => ok(json!({ "promos": { "1.20.1-latest": "47.4.2", "1.20.1-recommended": "47.4.0", "1.21.4-latest": "54.1.0" } })),
                // One version with both jars, the Forge one first and main.
                "/v2/version_files" => ok(json!({ &ns.0: { "project_id": "multi", "version_number": "2.0", "game_versions": ["1.21.1"], "loaders": ["forge", "neoforge"],
                    "files": [{ "hashes": { "sha1": fs.0, "sha512": fs.1 }, "url": format!("{b}/cdn/thing-forge.jar"), "filename": "thing-forge.jar", "primary": true, "size": 13 },
                              { "hashes": { "sha1": ns.0, "sha512": ns.1 }, "url": format!("{b}/cdn/thing-neoforge.jar"), "filename": "thing-neoforge.jar", "primary": false, "size": 16 }] } })),
                "/v2/version_files/update" => ok(json!({})),
                p if p.starts_with("/v2/projects?") => ok(json!([])),
                _ => None,
            }
        }));
        *base.lock() = s.base.clone();
        let api = Modrinth::new(&s.base, &s.base, &s.base).with_forge_sites(&s.base, &s.base);
        assert_eq!(api.latest_loader("neoforge", "1.21.1").unwrap(), "21.1.77", "the newest that isn't a beta");
        assert_eq!(api.latest_loader("neoforge", "1.21").unwrap(), "21.0.167");
        assert_eq!(api.latest_loader("neoforge", "26.1.2").unwrap(), "26.1.2.122");
        assert!(api.latest_loader("neoforge", "1.12.2").unwrap_err().contains("no build"));
        assert_eq!(api.latest_loader("forge", "1.20.1").unwrap(), "47.4.0", "recommended first");
        assert_eq!(api.latest_loader("forge", "1.21.4").unwrap(), "54.1.0");

        let mixer = Mixer::new(&t.path().join("mc"), api, None);
        let inst = t.path().join("Thing");
        std::fs::create_dir_all(inst.join("mods")).unwrap();
        std::fs::write(inst.join("mods").join("thing.jar"), neo_jar).unwrap();
        let src = mixer.add_folder(inst.to_str().unwrap()).unwrap();
        assert_eq!(src.files[0].urls, [format!("{}/cdn/thing-neoforge.jar", s.base)], "its own jar, not the main file");
        // No pack names a NeoForge build for the folder: the mix gets the newest.
        let mix = mixer.plan("Neo", std::slice::from_ref(&src.id), "1.21.1", "neoforge").unwrap();
        assert_eq!(mix.loader_version, "21.1.77");
        assert!(mix.problems.is_empty(), "{:?}", mix.problems);
        let out = t.path().join("neo.mrpack");
        mixer.export(&mix.id, &out, None).unwrap();
        let (idx, _) = read_mrpack(&std::fs::read(&out).unwrap()).unwrap();
        assert_eq!(idx["dependencies"], json!({ "minecraft": "1.21.1", "neoforge": "21.1.77" }));
        let f = &idx["files"][0];
        assert_eq!((f["hashes"]["sha1"].as_str(), f["downloads"][0].as_str()), (Some(neo_sha.0.as_str()), Some(format!("{}/cdn/thing-neoforge.jar", s.base).as_str())));
    }

    #[test]
    fn modpacks_other_launchers_installed_join_mixes() {
        let t = tempfile::tempdir().unwrap();
        let roaming = t.path().join("Roaming");
        let inst = roaming.join("PrismLauncher/instances/Create");
        let pic = crate::mods::cover::png(&image::RgbaImage::from_pixel(300, 300, image::Rgba([10, 200, 30, 255])));
        std::fs::create_dir_all(inst.join(".minecraft/mods")).unwrap();
        std::fs::create_dir_all(inst.join(".minecraft/config")).unwrap();
        std::fs::create_dir_all(roaming.join("PrismLauncher/icons")).unwrap();
        std::fs::write(roaming.join("PrismLauncher/icons/create.png"), &pic).unwrap();
        std::fs::write(inst.join("instance.cfg"), "name=Create Above\niconKey=create\n").unwrap();
        std::fs::write(inst.join("mmc-pack.json"), r#"{"components":[{"uid":"net.minecraft","version":"1.20.1"},{"uid":"net.minecraftforge","version":"47.2.0"}]}"#).unwrap();
        std::fs::write(inst.join(".minecraft/mods/create.jar"), b"create").unwrap();
        std::fs::write(inst.join(".minecraft/mods/off.jar.disabled"), b"off").unwrap();
        std::fs::write(inst.join(".minecraft/config/create.toml"), b"x=1").unwrap();
        std::fs::write(inst.join(".minecraft/options.txt"), b"fov:1").unwrap();
        // Modrinth can't be reached: the pack still comes in, as its launcher says it is.
        let dead = "http://127.0.0.1:9";
        let mixer = Mixer::new(&t.path().join("mc"), Modrinth::new(dead, dead, dead), None).looking_in(launchers::Places { roaming: Some(roaming), ..Default::default() });
        let found = mixer.launcher_packs();
        assert_eq!(found.len(), 1);
        let i = &found[0];
        assert_eq!((i.mods, i.disabled), (1, 1));
        let png = mixer.launcher_icon(&i.id).unwrap();
        assert_eq!(crate::mods::cover::decode(&png).unwrap().width(), 128);
        let s = mixer.add_launcher_pack(&i.id).unwrap();
        assert_eq!((s.name.as_str(), s.mc.as_str(), s.loader.as_str(), s.loader_version.as_str(), s.files.len(), s.overrides), ("Create Above", "1.20.1", "forge", "47.2.0", 1, 2));
        assert_eq!(s.kind, SourceKind::Launcher { launcher: "Prism Launcher".into(), path: i.path.clone() });
        // Compared as paths: Windows writes either slash.
        assert_eq!(s.files[0].local.as_deref().map(Path::new), Some(inst.join(".minecraft").join("mods").join("create.jar").as_path()));
        assert!(mixer.icon_of(&s).is_some(), "its picture is kept for the mix's cover");
        // A folder added by hand (no version known) still needs Modrinth.
        assert!(mixer.add_folder(inst.to_str().unwrap()).is_err());
        // For a server: its mods and settings (not options.txt).
        let (files, over) = mixer.launcher_server_parts(i).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(over.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>(), ["config/create.toml"]);
        assert!(mixer.launcher_pack("0000000000000000").unwrap_err().contains("isn't there"));

        // Other folders to look in.
        let other = t.path().join("Elsewhere/CurseForge/Instances/Pack");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("minecraftinstance.json"), r#"{"name":"Pack","gameVersion":"1.21.1","baseModLoader":{"name":"neoforge-21.1.77","type":6}}"#).unwrap();
        assert!(mixer.add_launcher_place(t.path().to_str().unwrap()).unwrap_err().contains("no Prism"));
        let dir = t.path().join("Elsewhere/CurseForge").to_string_lossy().into_owned();
        assert_eq!(mixer.add_launcher_place(&dir).unwrap().len(), 2);
        assert_eq!(mixer.add_launcher_place(&dir).unwrap().len(), 2, "added once");
        assert_eq!(mixer.launcher_places(), [dir.as_str()]);
        assert_eq!(mixer.remove_launcher_place(&dir).unwrap().len(), 1);
        assert!(mixer.launcher_places().is_empty());
    }
}
