//! OmniHub Client: Minecraft 1.8.9 for PvP, set up in the official
//! Minecraft Launcher (which signs in with your Microsoft account, so it
//! works on Hypixel and every other server).
//!
//! It installs Forge 1.8.9 (the version file and its libraries, the same
//! way Forge's installer does), a profile "OmniHub Client 1.8.9" with its
//! own game folder, the open-source mods you pick from a curated list
//! (from Modrinth, hash-checked), OmniHub's own kill-effects mod (built
//! with this release, checksum built in), and PvP-friendly video settings
//! the first time.
//!
//! Every download is checked: Forge's files against the SHA-1s Forge
//! publishes, mods against Modrinth's hashes, the kill-effects mod against
//! the release's SHA-256.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::minecraft::{Mixer, PackFile};

pub const MC: &str = "1.8.9";
pub const FORGE: &str = "1.8.9-11.15.1.2318-1.8.9";
pub const VERSION_ID: &str = "1.8.9-forge1.8.9-11.15.1.2318-1.8.9";
pub const PROFILE_KEY: &str = "omnihub-client-1.8.9";
pub const PROFILE_NAME: &str = "OmniHub Client 1.8.9";
pub const KILLFX_ASSET: &str = "OmniHub-KillFx-1.8.9.jar";
const KILLFX_FILE: &str = "OmniHub-KillFx.jar";
const MANIFEST: &str = "omnihub-client.json";
const FORGE_MAVEN: &str = "https://maven.minecraftforge.net/";
const MOJANG_LIBS: &str = "https://libraries.minecraft.net/";
const MAVEN_CENTRAL: &str = "https://repo1.maven.org/maven2/";

/// A mod in the curated list.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Curated {
    /// Modrinth project slug.
    pub slug: &'static str,
    pub name: &'static str,
    pub what: &'static str,
    /// "visuals", "hud", "performance", "packs", "movement"
    pub group: &'static str,
    pub default_on: bool,
    pub license: &'static str,
}

/// The best open-source 1.8.9 Forge mods for each job (all on Modrinth;
/// the Polyfrost ones load their settings library, OneConfig, themselves).
pub const CURATED: &[Curated] = &[
    Curated { slug: "patcher", name: "PolyPatcher", what: "Hundreds of fixes and FPS improvements for 1.8.9: faster chunk loading, entity culling, smoother fonts, less memory.", group: "performance", default_on: true, license: "CC-BY-NC-SA-4.0" },
    Curated { slug: "animations", name: "OverflowAnimations", what: "The 1.7 animations: blocking, sword swing, eating, bow pull, armour flashing red when hit.", group: "visuals", default_on: true, license: "GPL-3.0" },
    Curated { slug: "hitbox", name: "PolyHitbox", what: "Hitboxes you can colour and style per mob and player, with reach-line and on-hit colour.", group: "visuals", default_on: true, license: "GPL-3.0" },
    Curated { slug: "crosshair", name: "PolyCrosshair", what: "Draw your own crosshair: shape, size, colour, outline, and a colour change when aiming at a player.", group: "visuals", default_on: true, license: "GPL-3.0" },
    Curated { slug: "damagetint", name: "DamageTint", what: "The red flash when something is hit, in any colour and strength you like.", group: "visuals", default_on: true, license: "LGPL-3.0" },
    Curated { slug: "overflowparticles", name: "OverflowParticles", what: "Particles your way: more crit and sharpness particles on every hit, sizes, fades.", group: "visuals", default_on: true, license: "GPL-3.0" },
    Curated { slug: "resourcify", name: "Resourcify", what: "Browse and download resource packs from Modrinth and CurseForge inside Minecraft's own Resource Packs screen.", group: "packs", default_on: true, license: "LGPL-3.0" },
    Curated { slug: "evergreenhud", name: "EvergreenHUD", what: "FPS, CPS, ping, coordinates, keystrokes, armour and more on screen, each placed where you want.", group: "hud", default_on: true, license: "GPL-3.0" },
    Curated { slug: "polysprint", name: "PolySprint", what: "Toggle sprint and sneak, with a small on-screen note.", group: "movement", default_on: true, license: "AGPL-3.0" },
    Curated { slug: "freelook-oneconfig", name: "Freelook", what: "Hold a key to look around without turning.", group: "movement", default_on: false, license: "LGPL-3.0" },
];

/// What the kill-effects mod does (config/omnihub-killfx.properties).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct KillFx {
    pub enabled: bool,
    /// lightning, blood, flames, firework, souls, hearts, explosion, random
    pub effect: String,
    /// default, none, levelup, orb, anvil, explode, thunder, wither, ding
    pub sound: String,
    /// 1 subtle … 3 big
    pub intensity: u8,
    /// "#RRGGBB"
    pub color: String,
    pub players_only: bool,
    pub kill_mark: bool,
}

impl Default for KillFx {
    fn default() -> Self {
        KillFx { enabled: true, effect: "blood".into(), sound: "default".into(), intensity: 2, color: "#FF3B3B".into(), players_only: false, kill_mark: true }
    }
}

pub const EFFECTS: &[&str] = &["lightning", "blood", "flames", "firework", "souls", "hearts", "explosion", "random"];
pub const SOUNDS: &[&str] = &["default", "none", "levelup", "orb", "anvil", "explode", "thunder", "wither", "ding"];

impl KillFx {
    fn clean(mut self) -> KillFx {
        if !EFFECTS.contains(&self.effect.as_str()) {
            self.effect = "blood".into();
        }
        if !SOUNDS.contains(&self.sound.as_str()) {
            self.sound = "default".into();
        }
        self.intensity = self.intensity.clamp(1, 3);
        let hex = self.color.trim_start_matches('#');
        self.color = if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) { format!("#{}", hex.to_ascii_uppercase()) } else { "#FF3B3B".into() };
        self
    }

    /// The properties file the mod reads.
    pub fn to_properties(&self) -> String {
        let k = self.clone().clean();
        format!(
            "# OmniHub Kill Effects (set these in OmniHub: Games > Minecraft client)\nenabled={}\neffect={}\nsound={}\nintensity={}\ncolor={}\nplayers_only={}\nkill_mark={}\n",
            k.enabled, k.effect, k.sound, k.intensity, k.color, k.players_only, k.kill_mark
        )
    }

    pub fn from_properties(text: &str) -> KillFx {
        let mut k = KillFx::default();
        for line in text.lines() {
            let Some((key, v)) = line.split_once('=') else { continue };
            let v = v.trim();
            match key.trim() {
                "enabled" => k.enabled = v.eq_ignore_ascii_case("true"),
                "effect" => k.effect = v.to_ascii_lowercase(),
                "sound" => k.sound = v.to_ascii_lowercase(),
                "intensity" => k.intensity = v.parse().unwrap_or(2),
                // Java's Properties writes "#" as "\#".
                "color" => k.color = v.replace('\\', ""),
                "players_only" => k.players_only = v.eq_ignore_ascii_case("true"),
                "kill_mark" => k.kill_mark = v.eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
        k.clean()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
struct Manifest {
    /// slug → file name in mods/.
    mods: BTreeMap<String, String>,
    killfx: bool,
    at: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModState {
    #[serde(flatten)]
    pub info: Curated,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// The official launcher's .minecraft folder was found.
    pub minecraft_found: bool,
    pub installed: bool,
    pub dir: String,
    pub mods: Vec<ModState>,
    pub killfx_installed: bool,
    pub killfx: KillFx,
    pub packs: Vec<String>,
    pub installed_at: i64,
}

fn game_dir(mc: &Path) -> PathBuf {
    mc.join("omnihub").join("client-1.8.9")
}

fn read_manifest(dir: &Path) -> Option<Manifest> {
    std::fs::read(dir.join(MANIFEST)).ok().and_then(|b| serde_json::from_slice(&b).ok())
}

pub fn status(mixer: &Mixer) -> Status {
    let mc = mixer.minecraft_dir().map(Path::to_path_buf);
    let dir = mc.as_deref().map(game_dir);
    let man = dir.as_deref().and_then(read_manifest);
    let installed = man.is_some() && mc.as_deref().is_some_and(|m| m.join("versions").join(VERSION_ID).join(format!("{VERSION_ID}.json")).is_file());
    let man = man.unwrap_or_default();
    let killfx = dir.as_deref().and_then(|d| std::fs::read_to_string(d.join("config").join("omnihub-killfx.properties")).ok()).map(|t| KillFx::from_properties(&t)).unwrap_or_default();
    let packs = dir
        .as_deref()
        .and_then(|d| std::fs::read_dir(d.join("resourcepacks")).ok())
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.to_ascii_lowercase().ends_with(".zip"))
        .collect();
    Status {
        minecraft_found: mc.as_deref().is_some_and(Path::is_dir),
        installed,
        dir: dir.map(|d| d.to_string_lossy().into_owned()).unwrap_or_default(),
        mods: CURATED.iter().map(|c| ModState { info: c.clone(), installed: man.mods.contains_key(c.slug) }).collect(),
        killfx_installed: man.killfx,
        killfx,
        packs,
        installed_at: man.at,
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(20))).timeout_global(Some(Duration::from_secs(300))).http_status_as_error(false).build().into()
}

const UA: &str = concat!("Kerioxsx/omnihub/", env!("CARGO_PKG_VERSION"), " (OmniHub Minecraft client)");

fn fetch(agent: &ureq::Agent, url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let ok_host = [FORGE_MAVEN, MOJANG_LIBS, MAVEN_CENTRAL, "https://github.com/"].iter().any(|h| url.starts_with(h)) || url.starts_with(&crate::capture::airplay::releases_url());
    if !ok_host {
        return Err(format!("{url} isn't a place OmniHub downloads from."));
    }
    let mut res = agent.get(url).header("User-Agent", UA).call().map_err(|e| format!("Couldn't download {url}: {e}"))?;
    let status = res.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!("{url} answered {status}."));
    }
    let mut out = Vec::new();
    res.body_mut().as_reader().take(limit).read_to_end(&mut out).map_err(|e| format!("The download stopped: {e}"))?;
    Ok(out)
}

fn sha1_hex(b: &[u8]) -> String {
    use sha1::Digest as _;
    hex::encode(sha1::Sha1::digest(b))
}

fn sha256_hex(b: &[u8]) -> String {
    use sha2::Digest as _;
    hex::encode(sha2::Sha256::digest(b))
}

/// "group:artifact:version" → "group/path/artifact/version/artifact-version.jar"
pub fn maven_path(name: &str) -> Option<String> {
    let mut it = name.split(':');
    let (g, a, v) = (it.next()?, it.next()?, it.next()?);
    let classifier = it.next();
    let ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "._-+".contains(c)) && !s.contains("..");
    if !(ok(g) && ok(a) && ok(v) && classifier.is_none_or(ok)) {
        return None;
    }
    let file = match classifier {
        Some(c) => format!("{a}-{v}-{c}.jar"),
        None => format!("{a}-{v}.jar"),
    };
    Some(format!("{}/{a}/{v}/{file}", g.replace('.', "/")))
}

/// One library to fetch: where it goes under libraries/, where from, and
/// the SHA-1s it may have (empty: check the server's .sha1 file).
#[derive(Debug, Clone, PartialEq)]
pub struct Lib {
    pub path: String,
    pub urls: Vec<String>,
    pub sha1: Vec<String>,
}

/// Forge's version file for the launcher (server-only libraries left out)
/// and the libraries to fetch, from the installer's install_profile.json.
pub fn plan(profile: &Value) -> Result<(Value, Vec<Lib>), String> {
    let mut version = profile["versionInfo"].clone();
    if version["id"].as_str() != Some(VERSION_ID) {
        return Err("Forge's installer isn't the expected version.".into());
    }
    let mut libs = Vec::new();
    let mut keep = Vec::new();
    for l in version["libraries"].as_array().cloned().unwrap_or_default() {
        if l["clientreq"].as_bool() == Some(false) {
            continue;
        }
        let name = l["name"].as_str().unwrap_or_default();
        let path = maven_path(name).ok_or_else(|| format!("A library name isn't valid: {name}"))?;
        keep.push(l.clone());
        if name.starts_with("net.minecraftforge:forge:") {
            continue; // comes from the installer itself
        }
        let mut bases: Vec<String> = Vec::new();
        if let Some(u) = l["url"].as_str() {
            let u = u.replace("http://files.minecraftforge.net/maven/", FORGE_MAVEN).replace("https://files.minecraftforge.net/maven/", FORGE_MAVEN);
            bases.push(if u.ends_with('/') { u } else { format!("{u}/") });
        }
        bases.push(MOJANG_LIBS.into());
        bases.push(MAVEN_CENTRAL.into());
        bases.dedup();
        let sha1 = l["checksums"].as_array().into_iter().flatten().filter_map(|c| c.as_str()).map(str::to_ascii_lowercase).collect();
        libs.push(Lib { urls: bases.iter().map(|b| format!("{b}{path}")).collect(), path, sha1 });
    }
    version["libraries"] = Value::Array(keep);
    Ok((version, libs))
}

/// PvP-friendly video settings (only when the folder has no options.txt yet).
pub const OPTIONS: &str = "maxFps:260\nenableVsync:false\nfancyGraphics:false\nao:0\nrenderClouds:false\nrenderDistance:8\nuseVbo:true\nfboEnable:true\nmipmapLevels:0\nentityShadows:false\nparticles:0\nguiScale:2\nbobView:false\n";

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().ok_or("No folder.")?).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("omnihub-part");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn killfx_sha256(agent: &ureq::Agent) -> Result<String, String> {
    if let Some(s) = option_env!("OMNIHUB_KILLFX_SHA256").filter(|s| s.len() == 64) {
        return Ok(s.to_ascii_lowercase());
    }
    let sums = String::from_utf8_lossy(&fetch(agent, &format!("{}/v{}/SHA256SUMS.txt", crate::capture::airplay::releases_url(), env!("CARGO_PKG_VERSION")), 1 << 20)?).into_owned();
    crate::capture::airplay::checksum_for(&sums, KILLFX_ASSET).ok_or_else(|| "This version's release doesn't list the kill-effects mod.".to_string())
}

/// Install or update the client with these mods (Modrinth slugs from
/// CURATED) and, when `killfx` is set, the kill-effects mod.
pub fn install(mixer: &Mixer, chosen: &[String], killfx: bool, memory_gb: u32, progress: &mut dyn FnMut(usize, usize, &str)) -> Result<Status, String> {
    let mc = mixer.minecraft_dir().map(Path::to_path_buf).filter(|p| p.is_dir()).ok_or("Minecraft (Java Edition) isn't installed: open the Minecraft Launcher once, then try again.")?;
    let chosen: Vec<&Curated> = CURATED.iter().filter(|c| chosen.iter().any(|s| s == c.slug)).collect();
    let agent = agent();
    let dir = game_dir(&mc);
    let total = 3 + chosen.len() + usize::from(killfx);
    let mut step = 0;

    // 1. Forge 1.8.9.
    progress(step, total, "Forge 1.8.9");
    let vfile = mc.join("versions").join(VERSION_ID).join(format!("{VERSION_ID}.json"));
    let forge_jar = mc.join("libraries").join(maven_path(&format!("net.minecraftforge:forge:{FORGE}")).unwrap_or_default());
    if !vfile.is_file() || !forge_jar.is_file() {
        let url = format!("{FORGE_MAVEN}net/minecraftforge/forge/{FORGE}/forge-{FORGE}-installer.jar");
        let want = String::from_utf8_lossy(&fetch(&agent, &format!("{url}.sha1"), 1024)?).trim().to_ascii_lowercase();
        let jar = fetch(&agent, &url, 64 << 20)?;
        if want.len() != 40 || sha1_hex(&jar) != want {
            return Err("Forge's installer didn't match its published checksum.".into());
        }
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(jar)).map_err(|_| "Forge's installer couldn't be opened.".to_string())?;
        let profile: Value = {
            let mut f = z.by_name("install_profile.json").map_err(|_| "Forge's installer has no install profile.".to_string())?;
            let mut b = Vec::new();
            f.read_to_end(&mut b).map_err(|e| e.to_string())?;
            serde_json::from_slice(&b).map_err(|e| e.to_string())?
        };
        let (version, libs) = plan(&profile)?;
        let universal = profile["install"]["filePath"].as_str().unwrap_or_default().to_string();
        let mut b = Vec::new();
        z.by_name(&universal).map_err(|_| "Forge's installer has no Forge jar.".to_string())?.read_to_end(&mut b).map_err(|e| e.to_string())?;
        write_atomic(&forge_jar, &b)?;
        step += 1;
        for (i, l) in libs.iter().enumerate() {
            let out = mc.join("libraries").join(&l.path);
            progress(step, total, &format!("Forge libraries ({}/{})", i + 1, libs.len()));
            if out.is_file() {
                continue;
            }
            let mut last = String::new();
            let mut done = false;
            for u in &l.urls {
                let bytes = match fetch(&agent, u, 64 << 20) {
                    Ok(b) => b,
                    Err(e) => {
                        last = e;
                        continue;
                    }
                };
                let sums = if l.sha1.is_empty() { fetch(&agent, &format!("{u}.sha1"), 1024).map(|s| vec![String::from_utf8_lossy(&s).split_whitespace().next().unwrap_or_default().to_ascii_lowercase()]).unwrap_or_default() } else { l.sha1.clone() };
                if sums.is_empty() || !sums.contains(&sha1_hex(&bytes)) {
                    last = format!("{u} didn't match its checksum.");
                    continue;
                }
                write_atomic(&out, &bytes)?;
                done = true;
                break;
            }
            if !done {
                return Err(if last.is_empty() { format!("Couldn't download {}.", l.path) } else { last });
            }
        }
        write_atomic(&vfile, &serde_json::to_vec_pretty(&version).unwrap_or_default())?;
    } else {
        step += 1;
    }
    step += 1;

    // 2. The mods.
    std::fs::create_dir_all(dir.join("mods")).map_err(|e| e.to_string())?;
    let mut man = read_manifest(&dir).unwrap_or_default();
    // Take out the ones no longer wanted (only files OmniHub put there).
    for (slug, file) in man.mods.clone() {
        if !chosen.iter().any(|c| c.slug == slug) {
            let _ = std::fs::remove_file(dir.join("mods").join(&file));
            man.mods.remove(&slug);
        }
    }
    for c in &chosen {
        progress(step, total, c.name);
        let rel = mixer.api.newest(c.slug, &["forge"], MC, "mods")?.ok_or_else(|| format!("{} has no 1.8.9 Forge version on Modrinth any more.", c.name))?;
        let PackFile { path, urls, sha1, sha512, .. } = rel.file;
        let name = path.rsplit('/').next().unwrap_or_default().to_string();
        if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
            return Err(format!("{}'s file name isn't safe.", c.name));
        }
        if man.mods.get(c.slug) != Some(&name) || !dir.join("mods").join(&name).is_file() {
            let bytes = mixer.api.download(&urls, &sha1, &sha512)?;
            if let Some(old) = man.mods.get(c.slug) {
                let _ = std::fs::remove_file(dir.join("mods").join(old));
            }
            write_atomic(&dir.join("mods").join(&name), &bytes)?;
            man.mods.insert(c.slug.to_string(), name);
        }
        step += 1;
    }

    // 3. Kill effects.
    let kpath = dir.join("mods").join(KILLFX_FILE);
    if killfx {
        progress(step, total, "Kill effects");
        let want = killfx_sha256(&agent)?;
        let have = std::fs::read(&kpath).ok().map(|b| sha256_hex(&b));
        if have.as_deref() != Some(want.as_str()) {
            let bytes = fetch(&agent, &format!("{}/v{}/{KILLFX_ASSET}", crate::capture::airplay::releases_url(), env!("CARGO_PKG_VERSION")), 16 << 20)?;
            if sha256_hex(&bytes) != want {
                return Err("The kill-effects mod didn't match its checksum.".into());
            }
            write_atomic(&kpath, &bytes)?;
        }
        let cfg = dir.join("config").join("omnihub-killfx.properties");
        if !cfg.is_file() {
            write_atomic(&cfg, KillFx::default().to_properties().as_bytes())?;
        }
        step += 1;
    } else {
        let _ = std::fs::remove_file(&kpath);
    }
    man.killfx = killfx;

    // 4. Video settings and the launcher profile.
    progress(step, total, "Launcher profile");
    if !dir.join("options.txt").is_file() {
        write_atomic(&dir.join("options.txt"), OPTIONS.as_bytes())?;
    }
    for d in ["resourcepacks", "screenshots"] {
        let _ = std::fs::create_dir_all(dir.join(d));
    }
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let mem = memory_gb.clamp(2, 4);
    let java = format!("-Xmx{mem}G -Xms{mem}G -XX:+UseG1GC -XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=16M -Dfml.ignorePatchDiscrepancies=true -Dfml.ignoreInvalidMinecraftCertificates=true");
    let entry = json!({ "name": PROFILE_NAME, "type": "custom", "created": now, "lastUsed": now, "lastVersionId": VERSION_ID, "gameDir": dir.to_string_lossy(), "icon": "TNT", "javaArgs": java });
    super::minecraft::write_launcher_profile(&mc, PROFILE_KEY, &entry)?;
    man.at = crate::db::now();
    write_atomic(&dir.join(MANIFEST), &serde_json::to_vec_pretty(&man).unwrap_or_default())?;
    progress(total, total, "");
    Ok(status(mixer))
}

/// Save the kill-effects settings (the mod picks them up in a couple of
/// seconds, even mid-game).
pub fn set_killfx(mixer: &Mixer, k: &KillFx) -> Result<KillFx, String> {
    let mc = mixer.minecraft_dir().ok_or("Minecraft's folder wasn't found.")?;
    let k = k.clone().clean();
    write_atomic(&game_dir(mc).join("config").join("omnihub-killfx.properties"), k.to_properties().as_bytes())?;
    Ok(k)
}

/// Download a resource pack (Modrinth project) into the client's folder.
pub fn add_pack(mixer: &Mixer, project: &str) -> Result<String, String> {
    let mc = mixer.minecraft_dir().ok_or("Minecraft's folder wasn't found.")?;
    let rel = mixer.api.newest(project, &["minecraft"], MC, "resourcepacks")?.ok_or("That pack has no 1.8.9 version.")?;
    let name = rel.file.path.rsplit('/').next().unwrap_or_default().to_string();
    if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
        return Err("The pack's file name isn't safe.".into());
    }
    let bytes = mixer.api.download(&rel.file.urls, &rel.file.sha1, &rel.file.sha512)?;
    write_atomic(&game_dir(mc).join("resourcepacks").join(&name), &bytes)?;
    Ok(name)
}

/// Remove a resource pack from the client's folder.
pub fn remove_pack(mixer: &Mixer, name: &str) -> Result<(), String> {
    let mc = mixer.minecraft_dir().ok_or("Minecraft's folder wasn't found.")?;
    if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
        return Err("That isn't a pack name.".into());
    }
    std::fs::remove_file(game_dir(mc).join("resourcepacks").join(name)).map_err(|e| e.to_string())
}

/// Take the client out: its launcher profile and folder (Forge's shared
/// version files stay, like the official installer leaves them).
pub fn uninstall(mixer: &Mixer) -> Result<(), String> {
    let mc = mixer.minecraft_dir().ok_or("Minecraft's folder wasn't found.")?;
    super::minecraft::remove_launcher_profile(mc, PROFILE_KEY);
    let dir = game_dir(mc);
    if dir.is_dir() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("Couldn't remove its folder (is Minecraft running?): {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_paths() {
        assert_eq!(maven_path("org.scala-lang:scala-library:2.11.1").as_deref(), Some("org/scala-lang/scala-library/2.11.1/scala-library-2.11.1.jar"));
        assert_eq!(maven_path("net.minecraftforge:forge:1.8.9-11.15.1.2318-1.8.9").as_deref(), Some("net/minecraftforge/forge/1.8.9-11.15.1.2318-1.8.9/forge-1.8.9-11.15.1.2318-1.8.9.jar"));
        assert_eq!(maven_path("a:b:1:natives-windows").as_deref(), Some("a/b/1/b-1-natives-windows.jar"));
        assert!(maven_path("a:../b:1").is_none());
        assert!(maven_path("a:b").is_none());
    }

    #[test]
    fn plan_leaves_server_libraries_out() {
        let profile = json!({
            "install": { "filePath": "forge-universal.jar" },
            "versionInfo": { "id": VERSION_ID, "inheritsFrom": "1.8.9", "libraries": [
                { "name": format!("net.minecraftforge:forge:{FORGE}"), "url": "https://maven.minecraftforge.net/" },
                { "name": "jline:jline:2.13", "url": "https://maven.minecraftforge.net/", "clientreq": false, "serverreq": true },
                { "name": "org.scala-lang:scala-library:2.11.1", "url": "http://files.minecraftforge.net/maven/", "checksums": ["0E11DA23", "1e4d"], "clientreq": true },
                { "name": "net.sf.trove4j:trove4j:3.0.3", "clientreq": true }
            ]}
        });
        let (v, libs) = plan(&profile).unwrap();
        let names: Vec<&str> = v["libraries"].as_array().unwrap().iter().map(|l| l["name"].as_str().unwrap()).collect();
        assert!(!names.contains(&"jline:jline:2.13"));
        assert_eq!(names.len(), 3);
        assert_eq!(libs.len(), 2, "forge itself comes from the installer");
        assert_eq!(libs[0].urls[0], "https://maven.minecraftforge.net/org/scala-lang/scala-library/2.11.1/scala-library-2.11.1.jar");
        assert_eq!(libs[0].sha1, vec!["0e11da23".to_string(), "1e4d".to_string()]);
        assert_eq!(libs[1].urls[0], "https://libraries.minecraft.net/net/sf/trove4j/trove4j/3.0.3/trove4j-3.0.3.jar");
        assert!(libs[1].sha1.is_empty());
        assert!(plan(&json!({ "versionInfo": { "id": "other" } })).is_err());
    }

    #[test]
    fn killfx_settings_round_trip() {
        let k = KillFx { effect: "souls".into(), sound: "orb".into(), intensity: 3, color: "#00ffaa".into(), players_only: true, kill_mark: false, enabled: true };
        let back = KillFx::from_properties(&k.to_properties());
        assert_eq!(back, KillFx { color: "#00FFAA".into(), ..k });
        // What Java's Properties.store writes.
        let java = "#OmniHub\nenabled=false\neffect=nope\ncolor=\\#12AB34\nintensity=9\n";
        let j = KillFx::from_properties(java);
        assert!(!j.enabled);
        assert_eq!(j.effect, "blood");
        assert_eq!(j.color, "#12AB34");
        assert_eq!(j.intensity, 3);
    }

    #[test]
    fn curated_list_is_sane() {
        let mut slugs: Vec<&str> = CURATED.iter().map(|c| c.slug).collect();
        slugs.sort();
        slugs.dedup();
        assert_eq!(slugs.len(), CURATED.len());
        assert!(OPTIONS.lines().all(|l| l.contains(':')));
    }
}
