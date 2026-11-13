//! Any Minecraft version, plain or with a loader (Forge, NeoForge, Fabric,
//! Quilt or Legacy Fabric), as a profile in the official Minecraft Launcher,
//! each in its own folder (`.minecraft/omnihub/instances/<loader>-<version>`)
//! so mods for one version never mix with another's. Mods for it come from
//! Modrinth, checked against Modrinth's hashes. Signing in and starting the
//! game stay with the Minecraft Launcher.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{json, Value};

use super::minecraft::{version_cmp, Mixer, ProjectHit};
use crate::mcserver::install::{Flavor, Web};

pub const LEGACY_FABRIC_META: &str = "https://meta.legacyfabric.net";
pub const LOADERS: &[&str] = &["vanilla", "fabric", "forge", "neoforge", "quilt", "legacyfabric"];

/// What an install made.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Made {
    /// The launcher profile's name ("Fabric 1.21.1 (OmniHub)").
    pub profile: String,
    /// The version the launcher starts.
    pub version_id: String,
    /// The loader's own version ("0.16.9"), empty for plain Minecraft.
    pub loader_version: String,
    /// Its own game folder, with mods/ inside.
    pub game_dir: String,
}

pub fn label(loader: &str) -> &'static str {
    match loader {
        "fabric" => "Fabric",
        "forge" => "Forge",
        "neoforge" => "NeoForge",
        "quilt" => "Quilt",
        "legacyfabric" => "Legacy Fabric",
        _ => "Minecraft",
    }
}

fn flavor(loader: &str) -> Option<Flavor> {
    match loader {
        "fabric" => Some(Flavor::Fabric),
        "quilt" => Some(Flavor::Quilt),
        "forge" => Some(Flavor::Forge),
        "neoforge" => Some(Flavor::NeoForge),
        _ => None,
    }
}

fn ok_text(v: &str) -> bool {
    !v.is_empty() && v.len() < 64 && v.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_' | ' '))
}

/// Where a profile's game folder goes.
pub fn instance_dir(mc_dir: &Path, loader: &str, mc: &str) -> PathBuf {
    mc_dir.join("omnihub").join("instances").join(format!("{loader}-{mc}"))
}

/// Minecraft versions (releases) a loader has builds for, newest first.
pub fn versions(web: &Web, loader: &str) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = match loader {
        "vanilla" => web.mc_versions()?.into_iter().filter(|v| v.kind == "release").map(|v| v.id).collect(),
        "legacyfabric" => web
            .get_json(&format!("{LEGACY_FABRIC_META}/v2/versions/game"))?
            .as_array()
            .into_iter()
            .flatten()
            .filter(|v| v["stable"].as_bool().unwrap_or(true))
            .filter_map(|v| v["version"].as_str().map(String::from))
            .collect(),
        l => web.supported(flavor(l).ok_or("Unknown loader.")?)?,
    };
    out.retain(|v| ok_text(v) && !v.contains(' '));
    out.sort_by(|a, b| version_cmp(b, a));
    out.dedup();
    Ok(out)
}

/// The newest stable Legacy Fabric loader for a Minecraft version.
fn legacy_loader(web: &Web, mc: &str) -> Result<String, String> {
    let v = web.get_json(&format!("{LEGACY_FABRIC_META}/v2/versions/loader/{mc}"))?;
    let list = v.as_array().ok_or("No loader list.")?;
    let pick = list.iter().find(|x| x["loader"]["stable"].as_bool() == Some(true)).or(list.first()).ok_or_else(|| format!("Legacy Fabric has no loader for Minecraft {mc}."))?;
    pick["loader"]["version"].as_str().map(String::from).ok_or_else(|| "No loader version.".into())
}

/// Writes a Fabric-style version file (Fabric, Quilt, Legacy Fabric) into the launcher's folder.
fn write_version(mc_dir: &Path, profile: &Value) -> Result<String, String> {
    let id = profile["id"].as_str().filter(|s| ok_text(s) && !s.contains(' ')).ok_or("The loader's profile has no valid id.")?.to_string();
    let dir = mc_dir.join("versions").join(&id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{id}.json")), serde_json::to_vec_pretty(profile).unwrap_or_default()).map_err(|e| e.to_string())?;
    Ok(id)
}

/// Installs `loader` for Minecraft `mc` and adds a launcher profile for it.
/// `forge_client` installs Forge or NeoForge with their own installer (it
/// needs Java; the server hub brings its own).
pub fn install(web: &Web, mc_dir: &Path, loader: &str, mc: &str, forge_client: &dyn Fn(&str, &str, &str) -> Result<String, String>) -> Result<Made, String> {
    if !LOADERS.contains(&loader) || !ok_text(mc) || mc.contains(' ') {
        return Err("That loader or version isn't valid.".into());
    }
    if !mc_dir.join("launcher_profiles.json").is_file() {
        return Err("Open the Minecraft Launcher once first (it makes its profile list), then install again.".into());
    }
    let (version_id, loader_version) = match loader {
        "vanilla" => (mc.to_string(), String::new()),
        "fabric" | "quilt" => {
            let f = flavor(loader).unwrap();
            let lv = web.latest_loader(f, mc)?;
            let url = if loader == "fabric" {
                format!("{}/v2/versions/loader/{mc}/{lv}/profile/json", web.ep.fabric_meta)
            } else {
                format!("{}/v3/versions/loader/{mc}/{lv}/profile/json", web.ep.quilt_meta)
            };
            (write_version(mc_dir, &web.get_json(&url)?)?, lv)
        }
        "legacyfabric" => {
            let lv = legacy_loader(web, mc)?;
            (write_version(mc_dir, &web.get_json(&format!("{LEGACY_FABRIC_META}/v2/versions/loader/{mc}/{lv}/profile/json"))?)?, lv)
        }
        _ => {
            if loader == "forge" && version_cmp(mc, "1.12.2") == std::cmp::Ordering::Less {
                return Err(format!(
                    "Forge for Minecraft {mc} only installs with its own window: download the installer from files.minecraftforge.net and choose Install client.{}",
                    if mc == "1.8.9" { " For 1.8.9, the Client tab installs Forge with Vesper and PvP mods in one click." } else { "" }
                ));
            }
            let lv = web.latest_loader(flavor(loader).unwrap(), mc)?;
            (forge_client(loader, mc, &lv)?, lv)
        }
    };
    let game = instance_dir(mc_dir, loader, mc);
    std::fs::create_dir_all(game.join("mods")).map_err(|e| e.to_string())?;
    let name = if loader == "vanilla" { format!("Minecraft {mc} (OmniHub)") } else { format!("{} {mc} (OmniHub)", label(loader)) };
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let entry = json!({
        "name": name,
        "type": "custom",
        "lastVersionId": version_id,
        "gameDir": game.to_string_lossy(),
        "icon": if loader == "vanilla" { "Grass" } else { "Furnace" },
        "created": now,
        "lastUsed": now,
    });
    super::minecraft::write_launcher_profile(mc_dir, &format!("omnihub-{loader}-{mc}"), &entry)?;
    Ok(Made { profile: name, version_id, loader_version, game_dir: game.to_string_lossy().into_owned() })
}

/// Modrinth's category name for a loader.
fn category(loader: &str) -> &str {
    if loader == "legacyfabric" {
        "legacy-fabric"
    } else {
        loader
    }
}

/// Mods for a loader and version on Modrinth.
pub fn search_mods(mixer: &Mixer, loader: &str, mc: &str, query: &str) -> Result<Vec<ProjectHit>, String> {
    if loader == "vanilla" {
        return Err("Plain Minecraft has no mods: pick a loader.".into());
    }
    let facets = vec![vec![format!("categories:{}", category(loader))], vec![format!("versions:{mc}")]];
    Ok(mixer.api.search_projects("mod", query, &facets, 0)?.0)
}

/// Downloads a mod (and the mods it requires) into that version's mods folder.
pub fn install_mod(mixer: &Mixer, mc_dir: &Path, loader: &str, mc: &str, project: &str) -> Result<Vec<String>, String> {
    if !project.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') || project.is_empty() || project.len() > 64 {
        return Err("That isn't a Modrinth project.".into());
    }
    let mods = instance_dir(mc_dir, loader, mc).join("mods");
    if !mods.parent().is_some_and(|p| p.is_dir()) {
        return Err(format!("Install {} {mc} first.", label(loader)));
    }
    std::fs::create_dir_all(&mods).map_err(|e| e.to_string())?;
    let loaders: Vec<&str> = if loader == "quilt" { vec!["quilt", "fabric"] } else { vec![category(loader)] };
    let mut done = Vec::new();
    let mut queue = vec![project.to_string()];
    let mut seen = std::collections::HashSet::new();
    while let Some(p) = queue.pop() {
        if !seen.insert(p.clone()) || seen.len() > 12 {
            continue;
        }
        let Some(r) = mixer.api.newest(&p, &loaders, mc, "mods")? else {
            if p == project {
                return Err(format!("It has no file for {} {mc}.", label(loader)));
            }
            continue;
        };
        let name = Path::new(&r.file.path).file_name().map(|n| n.to_string_lossy().into_owned()).filter(|n| n.ends_with(".jar") && !n.contains("..")).ok_or("Modrinth's file isn't a .jar.")?;
        let dest = mods.join(&name);
        if !dest.is_file() {
            let bytes = mixer.api.download(&r.file.urls, &r.file.sha1, &r.file.sha512)?;
            let tmp = dest.with_extension("jar.part");
            std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
            std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
        }
        done.push(name);
        queue.extend(r.requires);
    }
    Ok(done)
}

/// Mods in that version's folder.
pub fn mods_in(mc_dir: &Path, loader: &str, mc: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(instance_dir(mc_dir, loader, mc).join("mods"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".jar"))
        .collect();
    v.sort();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_and_labels() {
        let d = instance_dir(Path::new("/mc"), "fabric", "1.21.1");
        assert!(d.ends_with("omnihub/instances/fabric-1.21.1"));
        assert_eq!(label("legacyfabric"), "Legacy Fabric");
        assert_eq!(category("legacyfabric"), "legacy-fabric");
        assert!(ok_text("1.20.1") && !ok_text("../x") && !ok_text(""));
    }

    #[test]
    fn writes_a_fabric_style_version() {
        let t = tempfile::tempdir().unwrap();
        let id = write_version(t.path(), &json!({"id": "fabric-loader-0.16.9-1.21.1", "inheritsFrom": "1.21.1"})).unwrap();
        assert!(t.path().join("versions").join(&id).join(format!("{id}.json")).is_file());
        assert!(write_version(t.path(), &json!({"id": "../evil"})).is_err());
    }
}
