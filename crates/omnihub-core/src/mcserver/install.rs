//! Getting a server ready: the Java it needs (Eclipse Temurin from
//! Adoptium) and its server program — Mojang's own, Paper, or Mojang's with
//! Fabric, Quilt, Forge or NeoForge. Everything downloaded is checked
//! against the checksum its publisher lists (Fabric's tiny launcher, which
//! has none, comes over HTTPS from Fabric and fetches Mojang's jar that
//! OmniHub has already checked). Forge, NeoForge and Quilt set themselves
//! up with their own installers, run once in the server's folder.

use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PISTON: &str = "https://piston-meta.mojang.com";
pub const FILL: &str = "https://fill.papermc.io";
pub const FORGE_MAVEN: &str = "https://maven.minecraftforge.net";
pub const FORGE_FILES: &str = "https://files.minecraftforge.net";
pub const NEO_MAVEN: &str = "https://maven.neoforged.net";
pub const QUILT_MAVEN: &str = "https://maven.quiltmc.org";
pub const ADOPTIUM: &str = "https://api.adoptium.net";

/// Which server program.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Flavor {
    Vanilla,
    Paper,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl Flavor {
    pub fn label(self) -> &'static str {
        match self {
            Flavor::Vanilla => "Vanilla",
            Flavor::Paper => "Paper",
            Flavor::Fabric => "Fabric",
            Flavor::Quilt => "Quilt",
            Flavor::Forge => "Forge",
            Flavor::NeoForge => "NeoForge",
        }
    }

    /// Takes mods (from `mods/`).
    pub fn mods(self) -> bool {
        matches!(self, Flavor::Fabric | Flavor::Quilt | Flavor::Forge | Flavor::NeoForge)
    }

    /// Takes plugins (from `plugins/`).
    pub fn plugins(self) -> bool {
        self == Flavor::Paper
    }

    /// Modrinth's loader names for what this server runs.
    pub fn loaders(self) -> &'static [&'static str] {
        match self {
            Flavor::Vanilla => &[],
            Flavor::Paper => &["paper", "spigot", "bukkit", "purpur", "folia"],
            Flavor::Fabric => &["fabric"],
            Flavor::Quilt => &["quilt", "fabric"],
            Flavor::Forge => &["forge"],
            Flavor::NeoForge => &["neoforge"],
        }
    }

    /// From a modpack's loader name.
    pub fn from_loader(l: &str) -> Option<Flavor> {
        Some(match l {
            "fabric" => Flavor::Fabric,
            "quilt" => Flavor::Quilt,
            "forge" => Flavor::Forge,
            "neoforge" => Flavor::NeoForge,
            "vanilla" | "" => Flavor::Vanilla,
            _ => return None,
        })
    }
}

/// How the server starts (paths inside its folder).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Launch {
    /// `java -jar <jar> nogui`
    Jar { jar: String },
    /// `java @user_jvm_args.txt @<args> nogui` (Forge and NeoForge since 1.17).
    Args { args: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct McVersion {
    pub id: String,
    /// release, snapshot
    pub kind: String,
    pub released: String,
}

/// What an install produced.
#[derive(Debug, Clone, PartialEq)]
pub struct Installed {
    pub launch: Launch,
    pub loader_version: String,
    pub java: u32,
    /// Extra start-up flags the publisher recommends (Paper's).
    pub flags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Endpoints {
    pub piston: String,
    pub fill: String,
    pub fabric_meta: String,
    pub quilt_meta: String,
    pub quilt_maven: String,
    pub forge_maven: String,
    pub forge_files: String,
    pub neo_maven: String,
    pub adoptium: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Endpoints {
            piston: PISTON.into(),
            fill: FILL.into(),
            fabric_meta: crate::mods::minecraft::FABRIC_META.into(),
            quilt_meta: crate::mods::minecraft::QUILT_META.into(),
            quilt_maven: QUILT_MAVEN.into(),
            forge_maven: FORGE_MAVEN.into(),
            forge_files: FORGE_FILES.into(),
            neo_maven: NEO_MAVEN.into(),
            adoptium: ADOPTIUM.into(),
        }
    }
}

/// A checksum to check a download against.
#[derive(Debug, Clone, PartialEq)]
pub enum Sum {
    Sha1(String),
    Sha256(String),
    Sha512(String),
    /// Nothing published: HTTPS only.
    None,
}

enum Hasher {
    Sha1(sha1::Sha1),
    Sha256(sha2::Sha256),
    Sha512(sha2::Sha512),
    None,
}

impl Hasher {
    fn new(s: &Sum) -> Hasher {
        use sha1::Digest as _;
        match s {
            Sum::Sha1(_) => Hasher::Sha1(sha1::Sha1::new()),
            Sum::Sha256(_) => Hasher::Sha256(sha2::Sha256::new()),
            Sum::Sha512(_) => Hasher::Sha512(sha2::Sha512::new()),
            Sum::None => Hasher::None,
        }
    }

    fn update(&mut self, b: &[u8]) {
        use sha1::Digest as _;
        match self {
            Hasher::Sha1(h) => h.update(b),
            Hasher::Sha256(h) => h.update(b),
            Hasher::Sha512(h) => h.update(b),
            Hasher::None => {}
        }
    }

    fn matches(self, s: &Sum) -> bool {
        use sha1::Digest as _;
        match (self, s) {
            (Hasher::Sha1(h), Sum::Sha1(want)) => hex::encode(h.finalize()).eq_ignore_ascii_case(want.trim()),
            (Hasher::Sha256(h), Sum::Sha256(want)) => hex::encode(h.finalize()).eq_ignore_ascii_case(want.trim()),
            (Hasher::Sha512(h), Sum::Sha512(want)) => hex::encode(h.finalize()).eq_ignore_ascii_case(want.trim()),
            (Hasher::None, Sum::None) => true,
            _ => false,
        }
    }
}

/// Progress: (bytes or steps done, total, what).
pub type Progress<'a> = &'a mut dyn FnMut(u64, u64, &str);

pub struct Web {
    pub ep: Endpoints,
    agent: ureq::Agent,
    local: bool,
}

pub fn ua() -> &'static str {
    concat!("Kerioxsx/omnihub/", env!("CARGO_PKG_VERSION"), " (OmniHub server host)")
}

impl Web {
    pub fn new(ep: Endpoints) -> Web {
        let local = ep.piston.starts_with("http://127.0.0.1");
        let mut cfg = ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(20))).timeout_global(Some(Duration::from_secs(1800))).http_status_as_error(false);
        if local {
            cfg = cfg.proxy(None);
        }
        Web { ep, agent: cfg.build().into(), local }
    }

    /// Addresses from publishers' lists are only followed over HTTPS.
    fn ok_url(&self, u: &str) -> Result<(), String> {
        if u.starts_with("https://") || (self.local && u.starts_with("http://127.0.0.1")) {
            Ok(())
        } else {
            Err(format!("{u} isn't an HTTPS address."))
        }
    }

    fn get_raw(&self, url: &str, limit: u64) -> Result<Vec<u8>, String> {
        self.ok_url(url)?;
        let host = url.split('/').nth(2).unwrap_or(url).to_string();
        let mut res = self.agent.get(url).header("User-Agent", ua()).call().map_err(|e| format!("Couldn't reach {host}: {e}"))?;
        let status = res.status().as_u16();
        let mut out = Vec::new();
        res.body_mut().as_reader().take(limit).read_to_end(&mut out).map_err(|e| format!("The download from {host} stopped: {e}"))?;
        match status {
            200..=299 => Ok(out),
            404 => Err(format!("{host} doesn't have that ({url}).")),
            429 => Err(format!("{host} asked OmniHub to slow down. Try again in a minute.")),
            s => Err(format!("{host} answered {s}.")),
        }
    }

    pub fn get_json(&self, url: &str) -> Result<Value, String> {
        let b = self.get_raw(url, 32 << 20)?;
        serde_json::from_slice(&b).map_err(|e| format!("{} sent something unreadable: {e}", url.split('/').nth(2).unwrap_or(url)))
    }

    pub fn get_text(&self, url: &str) -> Result<String, String> {
        Ok(String::from_utf8_lossy(&self.get_raw(url, 4 << 20)?).into_owned())
    }

    /// Download to `dest`, checked against `sum`; nothing is left behind if it fails.
    pub fn fetch(&self, url: &str, sum: &Sum, dest: &Path, progress: Progress) -> Result<(), String> {
        self.ok_url(url)?;
        let host = url.split('/').nth(2).unwrap_or(url).to_string();
        let name = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let mut res = self.agent.get(url).header("User-Agent", ua()).call().map_err(|e| format!("Couldn't reach {host}: {e}"))?;
        let status = res.status().as_u16();
        if !(200..300).contains(&status) {
            return Err(format!("{host} answered {status} for {name}."));
        }
        let total = res.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()).unwrap_or(0);
        if let Some(p) = dest.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let part = dest.with_extension("part");
        let r = (|| -> Result<(), String> {
            let mut f = std::fs::File::create(&part).map_err(|e| e.to_string())?;
            let mut h = Hasher::new(sum);
            let mut rd = res.body_mut().as_reader().take(2 << 30);
            let mut buf = vec![0u8; 256 << 10];
            let mut done = 0u64;
            let mut shown = 0u64;
            loop {
                let n = rd.read(&mut buf).map_err(|e| format!("The download of {name} stopped: {e}"))?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
                f.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                done += n as u64;
                if done - shown >= 1 << 20 {
                    shown = done;
                    progress(done, total, &name);
                }
            }
            f.flush().map_err(|e| e.to_string())?;
            if !h.matches(sum) {
                return Err(format!("{name} from {host} didn't match its published checksum, so it wasn't used."));
            }
            Ok(())
        })();
        match r {
            Ok(()) => std::fs::rename(&part, dest).map_err(|e| e.to_string()),
            Err(e) => {
                let _ = std::fs::remove_file(&part);
                Err(e)
            }
        }
    }

    // ---------- versions ----------

    /// Minecraft versions, newest first.
    pub fn mc_versions(&self) -> Result<Vec<McVersion>, String> {
        let v = self.get_json(&format!("{}/mc/game/version_manifest_v2.json", self.ep.piston))?;
        Ok(v["versions"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|x| Some(McVersion { id: x["id"].as_str()?.to_string(), kind: x["type"].as_str().unwrap_or("release").to_string(), released: x["releaseTime"].as_str().unwrap_or_default().to_string() }))
            .collect())
    }

    /// Mojang's version file for one Minecraft version.
    fn mojang_version(&self, mc: &str) -> Result<Value, String> {
        let v = self.get_json(&format!("{}/mc/game/version_manifest_v2.json", self.ep.piston))?;
        let url = v["versions"].as_array().into_iter().flatten().find(|x| x["id"] == mc).and_then(|x| x["url"].as_str()).ok_or_else(|| format!("Mojang has no Minecraft {mc}."))?.to_string();
        self.get_json(&url)
    }

    /// Mojang's server jar for a version (address, SHA-1) and the Java it needs.
    pub fn vanilla(&self, mc: &str) -> Result<(String, String, u32), String> {
        let v = self.mojang_version(mc)?;
        let s = &v["downloads"]["server"];
        let url = s["url"].as_str().ok_or_else(|| format!("Minecraft {mc} has no server download."))?;
        let java = v["javaVersion"]["majorVersion"].as_u64().unwrap_or(8) as u32;
        Ok((url.to_string(), s["sha1"].as_str().unwrap_or_default().to_string(), java))
    }

    /// The Minecraft versions a server type can run, newest first.
    pub fn supported(&self, flavor: Flavor) -> Result<Vec<String>, String> {
        use crate::mods::minecraft::version_cmp;
        let mut out: Vec<String> = match flavor {
            Flavor::Vanilla => return Ok(self.mc_versions()?.into_iter().filter(|v| v.kind == "release").map(|v| v.id).collect()),
            Flavor::Paper => {
                let v = self.get_json(&format!("{}/v3/projects/paper", self.ep.fill))?;
                v["versions"].as_object().into_iter().flat_map(|m| m.values()).flat_map(|l| l.as_array().into_iter().flatten()).filter_map(|x| x.as_str()).filter(|s| !s.contains('-')).map(String::from).collect()
            }
            Flavor::Fabric | Flavor::Quilt => {
                let url = if flavor == Flavor::Fabric { format!("{}/v2/versions/game", self.ep.fabric_meta) } else { format!("{}/v3/versions/game", self.ep.quilt_meta) };
                let v = self.get_json(&url)?;
                v.as_array().into_iter().flatten().filter(|x| x["stable"].as_bool() == Some(true)).filter_map(|x| x["version"].as_str().map(String::from)).collect()
            }
            Flavor::Forge => {
                let v = self.get_json(&format!("{}/net/minecraftforge/forge/promotions_slim.json", self.ep.forge_files))?;
                let mut s: Vec<String> = v["promos"].as_object().into_iter().flat_map(|m| m.keys()).filter_map(|k| k.rsplit_once('-').map(|(mc, _)| mc.to_string())).collect();
                s.dedup();
                s
            }
            Flavor::NeoForge => self.neo_versions()?.into_iter().filter_map(|v| neo_mc(&v)).collect(),
        };
        out.sort_by(|a, b| version_cmp(b, a));
        out.dedup();
        Ok(out)
    }

    fn neo_versions(&self) -> Result<Vec<String>, String> {
        let v = self.get_json(&format!("{}/api/maven/versions/releases/net/neoforged/neoforge", self.ep.neo_maven))?;
        Ok(v["versions"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)).collect())
    }

    /// The newest loader (or build) for a server type and Minecraft version.
    pub fn latest_loader(&self, flavor: Flavor, mc: &str) -> Result<String, String> {
        use crate::mods::minecraft::version_cmp;
        match flavor {
            Flavor::Vanilla => Ok(String::new()),
            Flavor::Paper => self.paper_build(mc).map(|(b, ..)| b),
            Flavor::Fabric | Flavor::Quilt => {
                let url = if flavor == Flavor::Fabric { format!("{}/v2/versions/loader/{mc}", self.ep.fabric_meta) } else { format!("{}/v3/versions/loader/{mc}", self.ep.quilt_meta) };
                let v = self.get_json(&url)?;
                let list = v.as_array().ok_or("No loader list.")?;
                let stable = |x: &&Value| x["loader"]["stable"].as_bool() != Some(false) && !x["loader"]["version"].as_str().unwrap_or_default().contains("beta");
                let pick = list.iter().find(stable).or(list.first()).ok_or_else(|| format!("{} has no loader for Minecraft {mc}.", flavor.label()))?;
                pick["loader"]["version"].as_str().map(String::from).ok_or_else(|| "No loader version.".into())
            }
            Flavor::Forge => {
                let v = self.get_json(&format!("{}/net/minecraftforge/forge/promotions_slim.json", self.ep.forge_files))?;
                let p = &v["promos"];
                p[format!("{mc}-recommended")].as_str().or(p[format!("{mc}-latest")].as_str()).map(String::from).ok_or_else(|| format!("Forge has no build for Minecraft {mc}."))
            }
            Flavor::NeoForge => {
                let mut all: Vec<String> = self.neo_versions()?.into_iter().filter(|v| neo_mc(v).as_deref() == Some(mc)).collect();
                all.sort_by(|a, b| version_cmp(b, a));
                all.iter().find(|v| !v.contains("beta") && !v.contains("alpha")).or(all.first()).cloned().ok_or_else(|| format!("NeoForge has no build for Minecraft {mc}."))
            }
        }
    }

    /// Paper's newest build for a version: (build, address, SHA-256).
    fn paper_build(&self, mc: &str) -> Result<(String, String, String), String> {
        let v = self.get_json(&format!("{}/v3/projects/paper/versions/{mc}/builds", self.ep.fill))?;
        let list = v.as_array().ok_or_else(|| format!("Paper has no builds for {mc}."))?;
        let b = list.iter().find(|b| b["channel"] == "STABLE").or(list.first()).ok_or_else(|| format!("Paper has no builds for {mc}."))?;
        let d = &b["downloads"]["server:default"];
        Ok((b["id"].as_u64().map(|n| n.to_string()).unwrap_or_default(), d["url"].as_str().unwrap_or_default().to_string(), d["checksums"]["sha256"].as_str().unwrap_or_default().to_string()))
    }

    /// Forge or NeoForge for the Minecraft Launcher in `mc_dir`, the way their
    /// own installers do it (`--installClient`). Returns the version the
    /// launcher starts; nothing runs when it is there already.
    pub fn install_client(&self, flavor: Flavor, mc: &str, loader: &str, mc_dir: &Path, java: &Path, progress: Progress) -> Result<String, String> {
        let ok_ver = |v: &str| !v.is_empty() && v.len() < 64 && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'));
        if !ok_ver(mc) || !ok_ver(loader) {
            return Err("That loader version isn't valid.".into());
        }
        let (base, name, id) = match flavor {
            Flavor::Forge => (format!("{}/net/minecraftforge/forge/{mc}-{loader}/forge-{mc}-{loader}-installer.jar", self.ep.forge_maven), format!("Forge {loader}"), format!("{mc}-forge-{loader}")),
            Flavor::NeoForge => (format!("{}/releases/net/neoforged/neoforge/{loader}/neoforge-{loader}-installer.jar", self.ep.neo_maven), format!("NeoForge {loader}"), format!("neoforge-{loader}")),
            _ => return Err("Only Forge and NeoForge have their own installer.".into()),
        };
        let made = |id: &str| mc_dir.join("versions").join(id).join(format!("{id}.json")).is_file();
        if made(&id) {
            return Ok(id);
        }
        if !mc_dir.join("launcher_profiles.json").is_file() {
            return Err("Open the Minecraft Launcher once first (it makes its profile list), then install again.".into());
        }
        let work = mc_dir.join("omnihub").join(".installer");
        std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
        let res = (|| {
            let sum = self.maven_sum(&base)?;
            progress(0, 0, &format!("Downloading {name}'s installer"));
            self.fetch(&base, &sum, &work.join("installer.jar"), progress)?;
            progress(0, 0, &format!("Installing {name} in the Minecraft Launcher (about a minute)"));
            run_installer(java, &work, &["-jar", "installer.jar", "--installClient", &mc_dir.to_string_lossy()], progress)
        })();
        let _ = std::fs::remove_dir_all(&work);
        res?;
        if made(&id) {
            Ok(id)
        } else {
            Err(format!("{name}'s installer finished, but its version wasn't found in the Minecraft Launcher's folder."))
        }
    }

    /// The Java a server needs and the start flags its publisher recommends.
    pub fn java_for(&self, flavor: Flavor, mc: &str) -> Result<(u32, Vec<String>), String> {
        if flavor == Flavor::Paper {
            if let Ok(v) = self.get_json(&format!("{}/v3/projects/paper/versions/{mc}", self.ep.fill)) {
                let j = &v["version"]["java"];
                if let Some(min) = j["version"]["minimum"].as_u64() {
                    let flags = j["flags"]["recommended"].as_array().into_iter().flatten().filter_map(|f| f.as_str()).filter(|f| f.starts_with("-XX:")).map(String::from).collect();
                    return Ok((min as u32, flags));
                }
            }
        }
        Ok((self.vanilla(mc)?.2, Vec::new()))
    }

    // ---------- installing ----------

    /// Put the server program for `flavor` into `dir`, with `java` for the
    /// installers that need running.
    pub fn install(&self, flavor: Flavor, mc: &str, loader: &str, dir: &Path, java: &Path, progress: Progress) -> Result<Installed, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let (java_major, flags) = self.java_for(flavor, mc)?;
        let loader = if loader.trim().is_empty() { self.latest_loader(flavor, mc)? } else { loader.trim().to_string() };
        let ok_ver = |v: &str| !v.is_empty() && v.len() < 64 && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'));
        if !loader.is_empty() && !ok_ver(&loader) {
            return Err("That loader version isn't valid.".into());
        }
        if !ok_ver(mc) {
            return Err("That Minecraft version isn't valid.".into());
        }
        let launch = match flavor {
            Flavor::Vanilla => {
                let (url, sha1, _) = self.vanilla(mc)?;
                progress(0, 0, &format!("Downloading Minecraft {mc}"));
                self.fetch(&url, &Sum::Sha1(sha1), &dir.join("server.jar"), progress)?;
                Launch::Jar { jar: "server.jar".into() }
            }
            Flavor::Paper => {
                let (_, url, sha256) = self.paper_build(mc)?;
                progress(0, 0, &format!("Downloading Paper {mc} build {loader}"));
                self.fetch(&url, &Sum::Sha256(sha256), &dir.join("paper.jar"), progress)?;
                Launch::Jar { jar: "paper.jar".into() }
            }
            Flavor::Fabric => {
                // Mojang's jar where Fabric's launcher looks for it (so it doesn't fetch it unchecked).
                let (url, sha1, _) = self.vanilla(mc)?;
                progress(0, 0, &format!("Downloading Minecraft {mc}"));
                self.fetch(&url, &Sum::Sha1(sha1), &dir.join(".fabric").join("server").join(format!("{mc}-server.jar")), progress)?;
                let inst = self.get_json(&format!("{}/v2/versions/installer", self.ep.fabric_meta))?;
                let installer = inst.as_array().into_iter().flatten().find(|x| x["stable"].as_bool() == Some(true)).or(inst.as_array().and_then(|a| a.first())).and_then(|x| x["version"].as_str()).ok_or("Fabric has no installer listed.")?.to_string();
                if !ok_ver(&installer) {
                    return Err("Fabric listed an odd installer version.".into());
                }
                progress(0, 0, &format!("Downloading Fabric Loader {loader}"));
                self.fetch(&format!("{}/v2/versions/loader/{mc}/{loader}/{installer}/server/jar", self.ep.fabric_meta), &Sum::None, &dir.join("fabric-server-launch.jar"), progress)?;
                Launch::Jar { jar: "fabric-server-launch.jar".into() }
            }
            Flavor::Quilt => {
                let meta = self.get_text(&format!("{}/repository/release/org/quiltmc/quilt-installer/maven-metadata.xml", self.ep.quilt_maven))?;
                let ver = tag(&meta, "release").or_else(|| tag(&meta, "latest")).filter(|v| ok_ver(v)).ok_or("Quilt's installer isn't listed.")?;
                let base = format!("{}/repository/release/org/quiltmc/quilt-installer/{ver}/quilt-installer-{ver}.jar", self.ep.quilt_maven);
                let sum = self.maven_sum(&base)?;
                progress(0, 0, "Downloading Quilt's installer");
                let jar = dir.join("quilt-installer.jar");
                self.fetch(&base, &sum, &jar, progress)?;
                run_installer(java, dir, &["-jar", "quilt-installer.jar", "install", "server", mc, &loader, "--download-server", "--install-dir=."], progress)?;
                let _ = std::fs::remove_file(&jar);
                if !dir.join("quilt-server-launch.jar").is_file() {
                    return Err("Quilt's installer finished without making quilt-server-launch.jar.".into());
                }
                Launch::Jar { jar: "quilt-server-launch.jar".into() }
            }
            Flavor::Forge | Flavor::NeoForge => {
                let (base, name) = if flavor == Flavor::Forge {
                    (format!("{}/net/minecraftforge/forge/{mc}-{loader}/forge-{mc}-{loader}-installer.jar", self.ep.forge_maven), format!("Forge {loader}"))
                } else {
                    (format!("{}/releases/net/neoforged/neoforge/{loader}/neoforge-{loader}-installer.jar", self.ep.neo_maven), format!("NeoForge {loader}"))
                };
                let sum = self.maven_sum(&base)?;
                progress(0, 0, &format!("Downloading {name}'s installer"));
                let jar = dir.join("installer.jar");
                self.fetch(&base, &sum, &jar, progress)?;
                run_installer(java, dir, &["-jar", "installer.jar", "--installServer"], progress)?;
                let _ = std::fs::remove_file(&jar);
                let _ = std::fs::remove_file(dir.join("installer.jar.log"));
                forge_launch(dir, flavor, mc, &loader).ok_or_else(|| format!("{name}'s installer finished, but the server it made wasn't found."))?
            }
        };
        Ok(Installed { launch, loader_version: loader, java: java_major, flags })
    }

    /// The checksum Maven publishes beside a file (SHA-256 if there is one, else SHA-1).
    fn maven_sum(&self, url: &str) -> Result<Sum, String> {
        let clean = |s: String| s.split_whitespace().next().unwrap_or_default().to_string();
        if let Ok(s) = self.get_text(&format!("{url}.sha256")) {
            let s = clean(s);
            if s.len() == 64 {
                return Ok(Sum::Sha256(s));
            }
        }
        let s = clean(self.get_text(&format!("{url}.sha1"))?);
        if s.len() == 40 {
            Ok(Sum::Sha1(s))
        } else {
            Err("The installer has no checksum to check it with.".into())
        }
    }
}

/// `<tag>value</tag>` from Maven's metadata.
fn tag(xml: &str, name: &str) -> Option<String> {
    let start = xml.find(&format!("<{name}>"))? + name.len() + 2;
    let end = xml[start..].find(&format!("</{name}>"))? + start;
    Some(xml[start..end].trim().to_string())
}

/// The Minecraft version a NeoForge version is for ("21.1.77" → 1.21.1, "26.3.0.95" → 26.3).
pub fn neo_mc(v: &str) -> Option<String> {
    let core = v.split(['-', '+']).next()?;
    let p: Vec<&str> = core.split('.').collect();
    let major: u32 = p.first()?.parse().ok()?;
    if major < 26 {
        let minor = p.get(1)?;
        Some(if *minor == "0" { format!("1.{major}") } else { format!("1.{major}.{minor}") })
    } else {
        let (a, b, c) = (p.first()?, p.get(1)?, p.get(2).copied().unwrap_or("0"));
        Some(if c == "0" { format!("{a}.{b}") } else { format!("{a}.{b}.{c}") })
    }
}

/// How a Forge or NeoForge install starts: its arguments file (1.17+) or its jar.
pub fn forge_launch(dir: &Path, flavor: Flavor, mc: &str, loader: &str) -> Option<Launch> {
    let file = if cfg!(windows) { "win_args.txt" } else { "unix_args.txt" };
    let rel = if flavor == Flavor::Forge { format!("libraries/net/minecraftforge/forge/{mc}-{loader}/{file}") } else { format!("libraries/net/neoforged/neoforge/{loader}/{file}") };
    if dir.join(&rel).is_file() {
        return Some(Launch::Args { args: rel });
    }
    // Older Forge: forge-<mc>-<v>.jar (or -universal.jar) beside the installer.
    let mut jars: Vec<String> = std::fs::read_dir(dir).ok()?.flatten().filter_map(|e| e.file_name().to_str().map(String::from)).filter(|n| n.starts_with("forge-") && n.ends_with(".jar") && !n.contains("installer")).collect();
    jars.sort();
    jars.pop().map(|jar| Launch::Jar { jar })
}

/// Run a loader's installer in the server's folder, 15 minutes at most.
fn run_installer(java: &Path, dir: &Path, args: &[&str], progress: Progress) -> Result<(), String> {
    let mut c = Command::new(java);
    no_window(&mut c);
    c.args(args).current_dir(dir).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = c.spawn().map_err(|e| format!("Java couldn't start the installer: {e}"))?;
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    for s in [child.stdout.take().map(|s| Box::new(s) as Box<dyn Read + Send>), child.stderr.take().map(|s| Box::new(s) as Box<dyn Read + Send>)].into_iter().flatten() {
        let tx = tx.clone();
        std::thread::spawn(move || {
            for l in std::io::BufReader::new(s).lines().map_while(Result::ok) {
                let _ = tx.send(l);
            }
        });
    }
    drop(tx);
    let start = std::time::Instant::now();
    let mut tail: Vec<String> = Vec::new();
    loop {
        while let Ok(l) = rx.try_recv() {
            let l = l.trim().to_string();
            if !l.is_empty() {
                progress(0, 0, &l.chars().take(140).collect::<String>());
                tail.push(l);
                if tail.len() > 30 {
                    tail.remove(0);
                }
            }
        }
        if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
            while let Ok(l) = rx.recv_timeout(Duration::from_millis(200)) {
                tail.push(l);
            }
            return if st.success() { Ok(()) } else { Err(format!("The installer failed:\n{}", tail.join("\n"))) };
        }
        if start.elapsed() > Duration::from_secs(900) {
            let _ = child.kill();
            return Err("The installer took over 15 minutes and was stopped.".into());
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

/// Programs OmniHub starts get no console window.
pub fn no_window(c: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    #[cfg(not(windows))]
    let _ = c;
}

// ---------- Java ----------

/// The Java versions Adoptium offers (16 is gone; 17 runs what needed it).
pub fn java_line(major: u32) -> u32 {
    match major {
        0..=8 => 8,
        9..=11 => 11,
        12..=17 => 17,
        18..=21 => 21,
        n => n,
    }
}

/// The main version of a Java program (`java -version`), e.g. 21 or 8.
pub fn java_version(java: &Path) -> Option<u32> {
    let mut c = Command::new(java);
    no_window(&mut c);
    let out = c.arg("-version").env_remove("JAVA_TOOL_OPTIONS").stdin(Stdio::null()).output().ok()?;
    let text = format!("{}{}", String::from_utf8_lossy(&out.stderr), String::from_utf8_lossy(&out.stdout));
    parse_java_version(&text)
}

pub fn parse_java_version(text: &str) -> Option<u32> {
    let v = text.lines().find(|l| l.contains(" version \""))?.split('"').nth(1)?;
    let mut parts = v.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// Java runtimes OmniHub keeps (in its own folder) for servers.
pub struct Runtimes {
    pub dir: PathBuf,
}

impl Runtimes {
    fn folder(&self, major: u32, jdk: bool) -> PathBuf {
        self.dir.join(format!("{major}-{}", if jdk { "jdk" } else { "jre" }))
    }

    fn exe(dir: &Path, name: &str) -> PathBuf {
        dir.join("bin").join(if cfg!(windows) { format!("{name}.exe") } else { name.to_string() })
    }

    /// A kept Java of exactly this version (a JDK also counts as a JRE).
    pub fn find(&self, major: u32, jdk: bool) -> Option<PathBuf> {
        let line = java_line(major);
        let mut dirs = vec![self.folder(line, true)];
        if !jdk {
            dirs.push(self.folder(line, false));
        }
        dirs.into_iter().map(|d| Self::exe(&d, "java")).find(|p| p.is_file())
    }

    /// The Java compiler beside a kept JDK.
    pub fn javac(&self, major: u32) -> Option<PathBuf> {
        let p = Self::exe(&self.folder(java_line(major), true), "javac");
        p.is_file().then_some(p)
    }

    /// Download Eclipse Temurin from Adoptium if it isn't here yet (Windows).
    pub fn ensure(&self, web: &Web, major: u32, jdk: bool, progress: Progress) -> Result<PathBuf, String> {
        if let Some(p) = self.find(major, jdk) {
            return Ok(p);
        }
        let line = java_line(major);
        let os = if cfg!(windows) { "windows" } else if cfg!(target_os = "macos") { "mac" } else { "linux" };
        let kind = if jdk { "jdk" } else { "jre" };
        let v = web.get_json(&format!("{}/v3/assets/latest/{line}/hotspot?architecture=x64&image_type={kind}&os={os}&vendor=eclipse", web.ep.adoptium))?;
        let pkg = v.as_array().and_then(|a| a.first()).map(|a| &a["binary"]["package"]).ok_or_else(|| format!("Adoptium has no Java {line} for this PC."))?;
        let (link, sum, name) = (pkg["link"].as_str().unwrap_or_default(), pkg["checksum"].as_str().unwrap_or_default(), pkg["name"].as_str().unwrap_or_default());
        if !name.ends_with(".zip") {
            return Err(format!("OmniHub downloads Java only on Windows. Install Java {line} yourself and choose it in the server's settings."));
        }
        if sum.len() != 64 {
            return Err("Adoptium listed no checksum for that Java.".into());
        }
        let zip = self.dir.join(format!("{line}-{kind}.zip"));
        progress(0, pkg["size"].as_u64().unwrap_or(0), &format!("Downloading Java {line}"));
        web.fetch(link, &Sum::Sha256(sum.into()), &zip, progress)?;
        progress(0, 0, &format!("Unpacking Java {line}"));
        let stage = self.dir.join(format!("{line}-{kind}.part"));
        let _ = std::fs::remove_dir_all(&stage);
        let r = crate::mods::install::unpack(&zip, &stage);
        let _ = std::fs::remove_file(&zip);
        r?;
        // The zip holds one folder (jdk-21.0.5+11-jre/); that's the runtime.
        let inner = std::fs::read_dir(&stage).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).find(|p| Self::exe(p, "java").is_file()).ok_or("That Java download has no java program in it.")?;
        let dest = self.folder(line, jdk);
        let _ = std::fs::remove_dir_all(&dest);
        std::fs::rename(&inner, &dest).map_err(|e| e.to_string())?;
        let _ = std::fs::remove_dir_all(&stage);
        Ok(Self::exe(&dest, "java"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_names() {
        assert_eq!(neo_mc("21.1.77").as_deref(), Some("1.21.1"));
        assert_eq!(neo_mc("21.0.167").as_deref(), Some("1.21"));
        assert_eq!(neo_mc("20.4.237").as_deref(), Some("1.20.4"));
        assert_eq!(neo_mc("21.10.5-beta").as_deref(), Some("1.21.10"));
        assert_eq!(neo_mc("26.3.0.95").as_deref(), Some("26.3"));
        assert_eq!(neo_mc("26.1.2.17").as_deref(), Some("26.1.2"));
        assert_eq!(parse_java_version("openjdk version \"25.0.4.1\" 2026-08-18 LTS"), Some(25));
        assert_eq!(parse_java_version("java version \"1.8.0_392\"\nJava(TM) SE Runtime"), Some(8));
        assert_eq!(parse_java_version("Picked up JAVA_TOOL_OPTIONS: x\nopenjdk version \"21.0.2\" 2024-01-16"), Some(21));
        assert_eq!((java_line(16), java_line(25), java_line(8)), (17, 25, 8));
        assert_eq!(tag("<metadata><versioning><latest>0.12.1</latest><release>0.12.0</release>", "release").as_deref(), Some("0.12.0"));
        assert_eq!(Flavor::from_loader("neoforge"), Some(Flavor::NeoForge));
    }
}

