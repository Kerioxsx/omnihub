//! Thunderstore (thunderstore.io), the mod site r2modman uses: which games
//! it has (with their Steam app ids and mod loaders, from its ecosystem
//! schema), each game's mods (its package listing), and dependencies.
//!
//! The schema is fetched at most once a day and kept in a small form. A
//! game's listing is a list of gzip chunks; each chunk is read as it arrives
//! and only what the Mods page shows is kept (the latest version, and the
//! downloads of all versions added up), so even games with tens of thousands
//! of mods fit in a few megabytes.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::time::Duration;

use serde::de::{SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

pub const BASE: &str = "https://thunderstore.io";

/// A game on Thunderstore.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TsGame {
    /// The community's id in its URLs ("lethal-company").
    pub community: String,
    pub name: String,
    pub steam: Vec<u32>,
    /// r2modman's loader name: bepinex, melonloader, recursive-melonloader, lovely, shimloader…
    pub loader: String,
    pub exe_names: Vec<String>,
}

/// Loader packages, by full name ("BepInEx-BepInExPack") → (loader, folder in the zip).
pub type LoaderPackages = HashMap<String, (String, String)>;

/// The games, and the loader packages, from the ecosystem schema.
pub fn parse_schema(v: &Value) -> (Vec<TsGame>, LoaderPackages) {
    let mut games = Vec::new();
    if let Some(m) = v["games"].as_object() {
        for g in m.values() {
            let Some(r) = g["r2modman"].as_array().and_then(|a| a.first()) else { continue };
            let index = r["packageIndex"].as_str().unwrap_or_default();
            let Some(community) = index.split("/c/").nth(1).and_then(|s| s.split('/').next()).filter(|s| !s.is_empty()) else { continue };
            let steam: Vec<u32> = g["distributions"].as_array().into_iter().flatten().filter(|d| d["platform"] == "steam").filter_map(|d| d["identifier"].as_str()?.parse().ok()).collect();
            let name = g["meta"]["displayName"].as_str().or(r["meta"]["displayName"].as_str()).unwrap_or(community).to_string();
            games.push(TsGame {
                community: community.to_string(),
                name,
                steam,
                loader: r["packageLoader"].as_str().unwrap_or("none").to_string(),
                exe_names: r["exeNames"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect(),
            });
        }
    }
    games.sort_by(|a, b| a.community.cmp(&b.community));
    let loaders = v["modloaderPackages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| Some((p["packageId"].as_str()?.to_string(), (p["loader"].as_str()?.to_string(), p["rootFolder"].as_str().unwrap_or_default().to_string()))))
        .collect();
    (games, loaders)
}

/// A mod (package) on Thunderstore, as the Mods page shows it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    /// "Owner-Name".
    pub full_name: String,
    pub name: String,
    pub owner: String,
    pub description: String,
    pub icon: String,
    pub version: String,
    pub download_url: String,
    /// "Owner-Name-1.2.3".
    pub dependencies: Vec<String>,
    /// All versions' downloads together.
    pub downloads: u64,
    pub rating: u32,
    pub categories: Vec<String>,
    pub updated: String,
    pub deprecated: bool,
    pub nsfw: bool,
    pub pinned: bool,
    pub website: String,
    pub size: u64,
    pub url: String,
}

#[derive(Deserialize, Default)]
struct RawVersion {
    #[serde(default)]
    description: String,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    version_number: String,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    download_url: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    website_url: String,
    #[serde(default)]
    file_size: u64,
}

#[derive(Deserialize)]
struct Downloads {
    #[serde(default)]
    downloads: u64,
}

/// The newest version (listed first) in full, and the others' downloads.
struct Versions(RawVersion, u64);

impl<'de> Deserialize<'de> for Versions {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Versions;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a list of versions")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Versions, A::Error> {
                let first: RawVersion = seq.next_element()?.unwrap_or_default();
                let mut total = first.downloads;
                while let Some(x) = seq.next_element::<Downloads>()? {
                    total += x.downloads;
                }
                Ok(Versions(first, total))
            }
        }
        d.deserialize_seq(V)
    }
}

#[derive(Deserialize)]
struct RawPackage {
    #[serde(default)]
    name: String,
    #[serde(default)]
    full_name: String,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    package_url: String,
    #[serde(default)]
    date_updated: String,
    #[serde(default)]
    rating_score: u32,
    #[serde(default)]
    is_pinned: bool,
    #[serde(default)]
    is_deprecated: bool,
    #[serde(default)]
    has_nsfw_content: bool,
    #[serde(default)]
    categories: Vec<String>,
    versions: Versions,
}

/// One chunk of a game's listing (gzip or plain JSON).
pub fn parse_chunk(bytes: &[u8]) -> Result<Vec<Package>, String> {
    let plain = ungzip(bytes)?;
    let raw: Vec<RawPackage> = serde_json::from_slice(&plain).map_err(|e| format!("Thunderstore sent a list OmniHub can't read: {e}"))?;
    Ok(raw
        .into_iter()
        .filter(|p| !p.versions.0.version_number.is_empty())
        .map(|p| {
            let Versions(v, total) = p.versions;
            Package {
                full_name: p.full_name,
                name: p.name,
                owner: p.owner,
                description: v.description,
                icon: v.icon,
                version: v.version_number,
                download_url: v.download_url,
                dependencies: v.dependencies,
                downloads: total,
                rating: p.rating_score,
                categories: p.categories,
                updated: p.date_updated,
                deprecated: p.is_deprecated,
                nsfw: p.has_nsfw_content,
                pinned: p.is_pinned,
                website: v.website_url,
                size: v.file_size,
                url: p.package_url,
            }
        })
        .collect())
}

/// Gzip data unpacked; anything else as it is.
pub fn ungzip(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.starts_with(&[0x1f, 0x8b]) {
        let mut out = Vec::new();
        flate2::read::GzDecoder::new(bytes).take(1 << 30).read_to_end(&mut out).map_err(|e| format!("Couldn't unpack Thunderstore's list: {e}"))?;
        Ok(out)
    } else {
        Ok(bytes.to_vec())
    }
}

/// "Owner-Name-1.2.3" → ("Owner-Name", "1.2.3").
pub fn split_dependency(dep: &str) -> Option<(&str, &str)> {
    let (name, version) = dep.rsplit_once('-')?;
    (version.split('.').count() == 3 && version.split('.').all(|p| p.parse::<u32>().is_ok()) && name.contains('-')).then_some((name, version))
}

/// The packages to install for `full_name`, dependencies first (each once),
/// leaving out `skip` (already installed, or the loader). Missing
/// dependencies are named.
pub fn resolve<'a>(full_name: &str, all: &'a HashMap<String, Package>, skip: &HashSet<String>) -> Result<Vec<&'a Package>, String> {
    fn visit<'a>(name: &str, all: &'a HashMap<String, Package>, skip: &HashSet<String>, seen: &mut HashSet<String>, out: &mut Vec<&'a Package>, missing: &mut Vec<String>) {
        if skip.contains(name) || !seen.insert(name.to_string()) {
            return;
        }
        let Some(p) = all.get(name) else {
            missing.push(name.to_string());
            return;
        };
        for d in &p.dependencies {
            if let Some((dep, _)) = split_dependency(d) {
                visit(dep, all, skip, seen, out, missing);
            }
        }
        out.push(p);
    }
    let (mut seen, mut out, mut missing) = (HashSet::new(), Vec::new(), Vec::new());
    visit(full_name, all, skip, &mut seen, &mut out, &mut missing);
    if !missing.is_empty() {
        return Err(format!("Thunderstore no longer has {} (needed by this mod).", missing.join(", ")));
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Sort {
    #[default]
    Downloads,
    Rating,
    Updated,
    Name,
}

/// Mods matching `query` (name, author, description, category), safe for
/// work, newest-first or most popular first.
pub fn search<'a>(all: &'a [Package], query: &str, category: Option<&str>, sort: Sort, show_deprecated: bool) -> Vec<&'a Package> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut hits: Vec<&Package> = all
        .iter()
        .filter(|p| !p.nsfw && (show_deprecated || !p.deprecated))
        .filter(|p| category.is_none_or(|c| p.categories.iter().any(|x| x.eq_ignore_ascii_case(c))))
        .filter(|p| {
            if words.is_empty() {
                return true;
            }
            let hay = format!("{} {} {} {}", p.name.replace('_', " "), p.owner, p.description, p.categories.join(" ")).to_lowercase();
            words.iter().all(|w| hay.contains(w))
        })
        .collect();
    match sort {
        Sort::Downloads => hits.sort_by(|a, b| b.pinned.cmp(&a.pinned).then(b.downloads.cmp(&a.downloads))),
        Sort::Rating => hits.sort_by(|a, b| b.rating.cmp(&a.rating).then(b.downloads.cmp(&a.downloads))),
        Sort::Updated => hits.sort_by(|a, b| b.updated.cmp(&a.updated)),
        Sort::Name => hits.sort_by_key(|p| p.name.to_lowercase()),
    }
    if !words.is_empty() {
        // An exact name match first.
        let q = words.join("").replace(['_', '-'], "");
        hits.sort_by_key(|p| p.name.to_lowercase().replace(['_', '-'], "") != q);
    }
    hits
}

/// Talks to Thunderstore (or a pretend one in tests).
pub struct Client {
    base: String,
    agent: ureq::Agent,
}

impl Client {
    pub fn new(base: &str) -> Client {
        let mut cfg = ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(20))).timeout_global(Some(Duration::from_secs(600))).http_status_as_error(false);
        if base.starts_with("http://127.0.0.1") {
            cfg = cfg.proxy(None);
        }
        Client { base: base.trim_end_matches('/').to_string(), agent: cfg.build().into() }
    }

    /// GET a URL (relative ones on Thunderstore), up to `limit` bytes.
    pub fn fetch(&self, url: &str, limit: u64) -> Result<Vec<u8>, String> {
        let url = if url.starts_with('/') { format!("{}{url}", self.base) } else { url.to_string() };
        let mut res = self.agent.get(&url).header("User-Agent", concat!("OmniHub/", env!("CARGO_PKG_VERSION"))).call().map_err(|e| format!("Couldn't connect: {e}"))?;
        let status = res.status().as_u16();
        if status == 404 {
            return Err("Not found (404).".into());
        }
        if !(200..300).contains(&status) {
            return Err(format!("The site answered {status}. Try again in a moment."));
        }
        let mut out = Vec::new();
        res.body_mut().as_reader().take(limit).read_to_end(&mut out).map_err(|e| format!("The download stopped: {e}"))?;
        Ok(out)
    }

    /// Games and loader packages.
    pub fn schema(&self) -> Result<(Vec<TsGame>, LoaderPackages), String> {
        let bytes = self.fetch("/api/experimental/schema/dev/latest/", 64 << 20)?;
        let v: Value = serde_json::from_slice(&ungzip(&bytes)?).map_err(|e| format!("Thunderstore's game list can't be read: {e}"))?;
        Ok(parse_schema(&v))
    }

    /// Every mod for a community, chunk by chunk (`progress(done, chunks)`).
    pub fn listing(&self, community: &str, progress: &mut dyn FnMut(usize, usize)) -> Result<Vec<Package>, String> {
        if !community.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') || community.is_empty() {
            return Err("Not a Thunderstore community.".into());
        }
        let index = self.fetch(&format!("/c/{community}/api/v1/package-listing-index/"), 1 << 20)?;
        let urls: Vec<String> = serde_json::from_slice(&ungzip(&index)?).map_err(|e| format!("Thunderstore's list index can't be read: {e}"))?;
        let mut out = Vec::new();
        for (i, u) in urls.iter().enumerate() {
            progress(i, urls.len());
            if !(u.starts_with("https://") || u.starts_with(&self.base)) {
                continue;
            }
            out.extend(parse_chunk(&self.fetch(u, 256 << 20)?)?);
        }
        progress(urls.len(), urls.len());
        Ok(out)
    }

    /// The latest version of one package (for loaders: BepInExPack, MelonLoader, lovely).
    pub fn latest(&self, owner: &str, name: &str) -> Result<Package, String> {
        let ok = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
        if !ok(owner) || !ok(name) {
            return Err("Not a Thunderstore package.".into());
        }
        let v: Value = serde_json::from_slice(&self.fetch(&format!("/api/experimental/package/{owner}/{name}/"), 4 << 20)?).map_err(|e| e.to_string())?;
        let l = &v["latest"];
        let s = |x: &Value| x.as_str().unwrap_or_default().to_string();
        let version = s(&l["version_number"]);
        if version.is_empty() {
            return Err("Thunderstore lists no version of it.".into());
        }
        Ok(Package {
            full_name: format!("{owner}-{name}"),
            name: name.into(),
            owner: owner.into(),
            description: s(&l["description"]),
            icon: s(&l["icon"]),
            version,
            download_url: s(&l["download_url"]),
            dependencies: l["dependencies"].as_array().into_iter().flatten().filter_map(|d| d.as_str().map(String::from)).collect(),
            downloads: v["total_downloads"].as_u64().unwrap_or(0),
            website: s(&l["website_url"]),
            url: s(&v["package_url"]),
            ..Default::default()
        })
    }

    /// A package's zip, size-checked when Thunderstore gave a size.
    pub fn download(&self, p: &Package) -> Result<Vec<u8>, String> {
        let ok = p.download_url.starts_with("https://thunderstore.io/") || p.download_url.starts_with("https://gcdn.thunderstore.io/") || p.download_url.starts_with(&self.base);
        if !ok {
            return Err("That download isn't on Thunderstore.".into());
        }
        let bytes = self.fetch(&p.download_url, 2 << 30)?;
        if p.size > 0 && bytes.len() as u64 != p.size {
            return Err(format!("{} arrived incomplete ({} of {} bytes). Try again.", p.name, bytes.len(), p.size));
        }
        if !bytes.starts_with(b"PK") {
            return Err(format!("{} isn't a zip file.", p.name));
        }
        Ok(bytes)
    }
}

/// The address r2modman and the Thunderstore Mod Manager open to install a
/// package themselves (for games OmniHub doesn't install mods for).
pub fn manager_link(p: &Package) -> String {
    format!("ror2mm://v1/install/thunderstore.io/{}/{}/{}/", p.owner, p.name, p.version)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::Write;

    pub fn gz(s: &str) -> Vec<u8> {
        let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        e.write_all(s.as_bytes()).unwrap();
        e.finish().unwrap()
    }

    pub const SCHEMA: &str = r#"{"schemaVersion":"0.3.0","games":{
      "balatro":{"meta":{"displayName":"Balatro"},"distributions":[{"platform":"steam","identifier":"2379780"}],
        "r2modman":[{"packageIndex":"https://thunderstore.io/c/balatro/api/v1/package-listing-index/","packageLoader":"lovely","exeNames":["Balatro.exe"]}]},
      "lethal-company":{"meta":{"displayName":"Lethal Company"},"distributions":[{"platform":"steam","identifier":"1966720"}],
        "r2modman":[{"packageIndex":"https://thunderstore.io/c/lethal-company/api/v1/package-listing-index/","packageLoader":"bepinex","exeNames":["Lethal Company.exe"]}]},
      "no-manager":{"meta":{"displayName":"X"},"distributions":[],"r2modman":null}
    },"modloaderPackages":[{"packageId":"BepInEx-BepInExPack","rootFolder":"BepInExPack","loader":"bepinex"},{"packageId":"Thunderstore-lovely","rootFolder":"","loader":"lovely"}]}"#;

    pub fn chunk() -> String {
        serde_json::json!([
            {"name":"Steamodded","full_name":"Steamodded-Steamodded","owner":"Steamodded","package_url":"https://thunderstore.io/c/balatro/p/Steamodded/Steamodded/","date_updated":"2026-10-02T20:51:59Z","rating_score":7,"is_pinned":true,"is_deprecated":false,"has_nsfw_content":false,"categories":["Mods","Libraries"],
             "versions":[{"name":"Steamodded","full_name":"Steamodded-Steamodded-26.1002.0","description":"A Balatro Modding Framework","icon":"https://gcdn.thunderstore.io/i.png","version_number":"26.1002.0","dependencies":["Thunderstore-lovely-0.9.0"],"download_url":"https://thunderstore.io/package/download/Steamodded/Steamodded/26.1002.0/","downloads":8970,"website_url":"https://smods.dev","file_size":100,"extra":{"ignored":[1,2]}},
                         {"version_number":"1.0.0","downloads":30}]},
            {"name":"Cryptid","full_name":"spectralpack-Cryptid","owner":"spectralpack","date_updated":"2026-09-01T00:00:00Z","rating_score":2,"categories":["Mods","Content Mods"],
             "versions":[{"version_number":"0.5.17","description":"Absurd jokers","dependencies":["just_frostice482-Amulet-3.5.4","Steamodded-Steamodded-1.1814.0"],"download_url":"https://thunderstore.io/package/download/spectralpack/Cryptid/0.5.17/","downloads":6109}]},
            {"name":"Amulet","full_name":"just_frostice482-Amulet","owner":"just_frostice482","date_updated":"2026-08-01T00:00:00Z","categories":["Libraries"],
             "versions":[{"version_number":"3.5.4","description":"Big numbers","dependencies":["Steamodded-Steamodded-1.0.0"],"download_url":"https://thunderstore.io/package/download/just_frostice482/Amulet/3.5.4/","downloads":5000}]},
            {"name":"Spicy","full_name":"x-Spicy","owner":"x","has_nsfw_content":true,"versions":[{"version_number":"1.0.0","downloads":99999}]},
            {"name":"Old","full_name":"y-Old","owner":"y","is_deprecated":true,"versions":[{"version_number":"1.0.0","description":"old jokers","downloads":5}]},
            {"name":"Empty","full_name":"z-Empty","owner":"z","versions":[]}
        ])
        .to_string()
    }

    #[test]
    fn schema_listing_search_and_dependencies() {
        let (games, loaders) = parse_schema(&serde_json::from_str(SCHEMA).unwrap());
        assert_eq!(games.iter().map(|g| (g.community.as_str(), g.steam.clone(), g.loader.as_str())).collect::<Vec<_>>(), vec![("balatro", vec![2379780], "lovely"), ("lethal-company", vec![1966720], "bepinex")]);
        assert_eq!(loaders["BepInEx-BepInExPack"], ("bepinex".to_string(), "BepInExPack".to_string()));

        let all = parse_chunk(&gz(&chunk())).unwrap();
        assert_eq!(all.len(), 5, "the package with no versions is left out");
        let s = &all[0];
        assert_eq!((s.version.as_str(), s.downloads, s.dependencies.clone(), s.size), ("26.1002.0", 9000, vec!["Thunderstore-lovely-0.9.0".to_string()], 100));
        assert_eq!(parse_chunk(chunk().as_bytes()).unwrap().len(), 5, "plain JSON works too");

        let names = |v: Vec<&Package>| v.into_iter().map(|p| p.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(search(&all, "", None, Sort::Downloads, false)), ["Steamodded", "Cryptid", "Amulet"], "pinned first, no NSFW or deprecated");
        assert_eq!(names(search(&all, "jokers", None, Sort::Downloads, true)), ["Cryptid", "Old"]);
        assert_eq!(names(search(&all, "", Some("libraries"), Sort::Name, false)), ["Amulet", "Steamodded"]);
        assert_eq!(names(search(&all, "amulet", None, Sort::Downloads, false)), ["Amulet"]);

        let map: HashMap<String, Package> = all.iter().map(|p| (p.full_name.clone(), p.clone())).collect();
        let skip: HashSet<String> = ["Thunderstore-lovely".to_string()].into();
        let order = resolve("spectralpack-Cryptid", &map, &skip).unwrap();
        assert_eq!(order.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["Steamodded", "Amulet", "Cryptid"], "dependencies first, each once, the loader left out");
        let err = resolve("spectralpack-Cryptid", &map, &HashSet::new()).unwrap_err();
        assert!(err.contains("Thunderstore-lovely"), "{err}");
        assert_eq!(split_dependency("just_frostice482-Amulet-3.5.4"), Some(("just_frostice482-Amulet", "3.5.4")));
        assert_eq!(split_dependency("nonsense"), None);
        assert_eq!(manager_link(&all[0]), "ror2mm://v1/install/thunderstore.io/Steamodded/Steamodded/26.1002.0/");
    }
}
