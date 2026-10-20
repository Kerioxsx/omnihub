//! Mods in the shell: the Mods page's commands (games, the mod manager,
//! Thunderstore, Workshop items), Claude's settings and the mod maker, and
//! the Minecraft modpack mixer.

use std::sync::Arc;

use omnihub_core::claude::ClaudeSettings;
use omnihub_core::core::AppCore;
use omnihub_core::mods::detect::LoaderKind;
use omnihub_core::mods::maker::{Project, View};
use omnihub_core::mods::minecraft::{Installed, Mix, PackHit, PackVersion, Source};
use omnihub_core::mods::thunderstore::Sort;
use omnihub_core::mods::workshop::WorkshopItem;
use omnihub_core::mods::{Browse, InstalledMod, ModGame};
use serde::Serialize;
use tauri::State;

type Core<'a> = State<'a, Arc<AppCore>>;
type Res<T> = Result<T, String>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Res<T> + Send + 'static) -> Res<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

macro_rules! run {
    ($core:expr, |$c:ident| $body:expr) => {{
        let $c = $core.inner().clone();
        blocking(move || $body).await
    }};
}

// ---------- games and their mods ----------

#[tauri::command]
pub async fn mods_games(core: Core<'_>) -> Res<Vec<ModGame>> {
    run!(core, |c| Ok(c.mods.games()))
}

#[tauri::command]
pub async fn mods_list(core: Core<'_>, key: String) -> Res<Vec<InstalledMod>> {
    run!(core, |c| Ok(c.mods.mods(&key)))
}

#[tauri::command]
pub async fn mods_browse(core: Core<'_>, key: String, query: String, category: Option<String>, sort: Sort, offset: usize, limit: usize) -> Res<Browse> {
    run!(core, |c| c.mods.browse(&key, &query, category.as_deref(), sort, offset, limit))
}

#[tauri::command]
pub async fn mods_install_package(core: Core<'_>, key: String, full_name: String) -> Res<Vec<InstalledMod>> {
    run!(core, |c| {
        let r = c.mods.install_package(&key, &full_name);
        c.audit.record("desktop", "mods.install", &full_name, r.is_ok());
        r
    })
}

#[tauri::command]
pub async fn mods_install_file(core: Core<'_>, key: String, path: String) -> Res<InstalledMod> {
    run!(core, |c| {
        let r = c.mods.install_file(&key, &path);
        c.audit.record("desktop", "mods.install_file", &path, r.is_ok());
        r
    })
}

#[tauri::command]
pub async fn mods_install_loader(core: Core<'_>, key: String, kind: LoaderKind) -> Res<InstalledMod> {
    run!(core, |c| c.mods.install_loader(&key, kind))
}

#[tauri::command]
pub async fn mods_set_enabled(core: Core<'_>, id: String, on: bool) -> Res<()> {
    run!(core, |c| c.mods.set_enabled(&id, on))
}

#[tauri::command]
pub async fn mods_set_all(core: Core<'_>, key: String, on: bool) -> Res<usize> {
    run!(core, |c| c.mods.set_all(&key, on))
}

#[tauri::command]
pub async fn mods_uninstall(core: Core<'_>, id: String) -> Res<()> {
    run!(core, |c| {
        c.mods.uninstall(&id)?;
        c.mods.maker.forget_install(&id);
        Ok(())
    })
}

#[tauri::command]
pub async fn mods_update(core: Core<'_>, id: String) -> Res<InstalledMod> {
    run!(core, |c| c.mods.update(&id))
}

#[tauri::command]
pub async fn mods_workshop(core: Core<'_>, key: String) -> Res<Vec<WorkshopItem>> {
    run!(core, |c| c.mods.workshop(&key))
}

#[tauri::command]
pub async fn mods_add_folder(core: Core<'_>, name: String, dir: String) -> Res<ModGame> {
    run!(core, |c| c.mods.add_folder(&name, &dir))
}

#[tauri::command]
pub async fn mods_remove_folder(core: Core<'_>, key: String) -> Res<()> {
    run!(core, |c| c.mods.remove_folder(&key))
}

// ---------- Claude ----------

#[tauri::command]
pub async fn claude_settings(core: Core<'_>) -> Res<ClaudeSettings> {
    Ok(core.mods.claude.settings())
}

#[tauri::command]
pub async fn claude_update(core: Core<'_>, patch: serde_json::Value) -> Res<ClaudeSettings> {
    core.mods.claude.update(&patch)
}

/// The Anthropic API key: kept encrypted for this Windows account; empty removes it.
#[tauri::command]
pub async fn claude_set_key(core: Core<'_>, key: String) -> Res<()> {
    run!(core, |c| {
        let had = c.mods.claude.has_key();
        let r = c.mods.claude.set_key(&key);
        c.audit.record("desktop", "claude.key", if key.trim().is_empty() { "removed" } else if had { "replaced" } else { "added" }, r.is_ok());
        r
    })
}

#[tauri::command]
pub async fn claude_test(core: Core<'_>) -> Res<()> {
    run!(core, |c| c.mods.claude.test())
}

// ---------- the mod maker ----------

#[tauri::command]
pub async fn maker_projects(core: Core<'_>, game: Option<String>) -> Res<Vec<Project>> {
    run!(core, |c| Ok(c.mods.maker.projects(game.as_deref())))
}

#[tauri::command]
pub async fn maker_create(core: Core<'_>, game: String, text: String) -> Res<Project> {
    run!(core, |c| c.mods.maker.create(&c.mods, &game, &text))
}

#[tauri::command]
pub async fn maker_send(core: Core<'_>, id: String, text: String) -> Res<()> {
    run!(core, |c| c.mods.maker.send(&c.mods, &id, &text))
}

#[tauri::command]
pub async fn maker_stop(core: Core<'_>, id: String) -> Res<()> {
    core.mods.maker.stop(&id);
    Ok(())
}

#[tauri::command]
pub async fn maker_approve(core: Core<'_>, id: String, yes: bool) -> Res<()> {
    core.mods.maker.approve(&id, yes)
}

#[tauri::command]
pub async fn maker_view(core: Core<'_>, id: String) -> Res<View> {
    run!(core, |c| c.mods.maker.view(&id))
}

#[tauri::command]
pub async fn maker_file(core: Core<'_>, id: String, path: String) -> Res<String> {
    run!(core, |c| c.mods.maker.file(&id, &path))
}

#[tauri::command]
pub async fn maker_delete(core: Core<'_>, id: String) -> Res<()> {
    run!(core, |c| c.mods.maker.delete(&id))
}

#[tauri::command]
pub async fn maker_rename(core: Core<'_>, id: String, name: String) -> Res<Project> {
    run!(core, |c| c.mods.maker.rename(&id, &name))
}

#[tauri::command]
pub async fn maker_install(core: Core<'_>, id: String) -> Res<Project> {
    run!(core, |c| {
        let r = c.mods.maker.install(&c.mods, &id);
        c.audit.record("desktop", "mods.install_made", &id, r.is_ok());
        r
    })
}

// ---------- Minecraft ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McStatus {
    pub found: bool,
    pub dir: Option<String>,
    pub sources: Vec<Source>,
    pub installed: Vec<Installed>,
}

#[tauri::command]
pub async fn mc_status(core: Core<'_>) -> Res<McStatus> {
    run!(core, |c| {
        let m = &c.mods.minecraft;
        let dir = m.minecraft_dir().map(|d| d.to_path_buf());
        Ok(McStatus { found: dir.as_ref().is_some_and(|d| d.is_dir()), dir: dir.map(|d| d.to_string_lossy().into_owned()), sources: m.sources(), installed: m.installed() })
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McSearch {
    pub hits: Vec<PackHit>,
    pub total: u64,
}

#[tauri::command]
pub async fn mc_search(core: Core<'_>, query: String, offset: usize) -> Res<McSearch> {
    run!(core, |c| c.mods.minecraft.api.search(&query, offset).map(|(hits, total)| McSearch { hits, total }))
}

#[tauri::command]
pub async fn mc_versions(core: Core<'_>, project: String) -> Res<Vec<PackVersion>> {
    run!(core, |c| c.mods.minecraft.api.pack_versions(&project))
}

#[tauri::command]
pub async fn mc_add_modrinth(core: Core<'_>, project: String, version: String) -> Res<Source> {
    run!(core, |c| c.mods.minecraft.add_modrinth(&project, &version))
}

#[tauri::command]
pub async fn mc_add_file(core: Core<'_>, path: String) -> Res<Source> {
    run!(core, |c| c.mods.minecraft.add_file(&path))
}

#[tauri::command]
pub async fn mc_add_folder(core: Core<'_>, path: String) -> Res<Source> {
    run!(core, |c| c.mods.minecraft.add_folder(&path))
}

#[tauri::command]
pub async fn mc_remove_source(core: Core<'_>, id: String) -> Res<()> {
    run!(core, |c| c.mods.minecraft.remove_source(&id))
}

#[tauri::command]
pub async fn mc_plan(core: Core<'_>, name: String, ids: Vec<String>, mc: String, loader: String) -> Res<Mix> {
    run!(core, |c| c.mods.minecraft.plan(&name, &ids, &mc, &loader))
}

#[tauri::command]
pub async fn mc_export(core: Core<'_>, id: String, path: String) -> Res<()> {
    run!(core, |c| c.mods.minecraft.export(&id, std::path::Path::new(&path)))
}

#[tauri::command]
pub async fn mc_install(core: Core<'_>, id: String) -> Res<Installed> {
    run!(core, |c| {
        for exe in ["MinecraftLauncher.exe", "Minecraft.exe"] {
            if !c.procs.pids_named(exe).is_empty() {
                return Err("Close the Minecraft Launcher first (it rewrites its profile list when it closes).".into());
            }
        }
        let memory = omnihub_core::mods::minecraft::memory_for_pc();
        let ev = c.events.clone();
        let r = c.mods.minecraft.install(&id, &mut |done, total, file| ev.emit("mods:progress", serde_json::json!({ "game": "minecraft", "text": if file.is_empty() { String::new() } else { format!("Downloading {file}…") }, "done": done, "total": total })), memory);
        c.audit.record("desktop", "minecraft.install", &id, r.is_ok());
        r
    })
}

#[tauri::command]
pub async fn mc_uninstall(core: Core<'_>, profile: String) -> Res<()> {
    run!(core, |c| c.mods.minecraft.uninstall(&profile))
}
