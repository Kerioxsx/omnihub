//! Minecraft servers in the shell: the Server page's commands (making,
//! running, the console, settings, players, the map, content, backups and
//! the way in for friends).

use std::sync::Arc;

use omnihub_core::core::AppCore;
use omnihub_core::mcserver::backup::Backup;
use omnihub_core::mcserver::process::Line;
use omnihub_core::mcserver::{Action, Content, Flavor, Hit, Kind, MapInfo, NetView, NewServer, Online, Patch, PlayerView, PlayersView, View};
use omnihub_core::system::firewall;
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

/// Run and note it in the activity log.
fn audited<T>(c: &AppCore, action: &str, detail: &str, r: Res<T>) -> Res<T> {
    c.audit.record("desktop", action, detail, r.is_ok());
    r
}

#[tauri::command]
pub async fn srv_list(core: Core<'_>) -> Res<Vec<View>> {
    run!(core, |c| Ok(c.mcserver.list()))
}

#[tauri::command]
pub async fn srv_get(core: Core<'_>, id: String) -> Res<View> {
    run!(core, |c| c.mcserver.view(&id))
}

#[tauri::command]
pub async fn srv_versions(core: Core<'_>, flavor: Flavor) -> Res<Vec<String>> {
    run!(core, |c| c.mcserver.versions(flavor))
}

#[tauri::command]
pub async fn srv_create(core: Core<'_>, server: NewServer) -> Res<View> {
    run!(core, |c| {
        let name = server.name.clone();
        audited(&c, "mcserver.create", &name, c.mcserver.create(server))
    })
}

#[tauri::command]
pub async fn srv_adopt(core: Core<'_>, path: String) -> Res<View> {
    run!(core, |c| audited(&c, "mcserver.adopt", &path, c.mcserver.adopt(&path)))
}

#[tauri::command]
pub async fn srv_update(core: Core<'_>, id: String, patch: Patch) -> Res<View> {
    run!(core, |c| c.mcserver.update(&id, patch))
}

#[tauri::command]
pub async fn srv_delete(core: Core<'_>, id: String, files: bool) -> Res<()> {
    run!(core, |c| audited(&c, "mcserver.delete", &id, c.mcserver.delete(&id, files)))
}

#[tauri::command]
pub async fn srv_start(core: Core<'_>, id: String) -> Res<View> {
    run!(core, |c| audited(&c, "mcserver.start", &id, c.mcserver.start(&id)))
}

#[tauri::command]
pub async fn srv_stop(core: Core<'_>, id: String) -> Res<View> {
    run!(core, |c| audited(&c, "mcserver.stop", &id, c.mcserver.stop(&id)))
}

#[tauri::command]
pub async fn srv_restart(core: Core<'_>, id: String) -> Res<View> {
    run!(core, |c| audited(&c, "mcserver.restart", &id, c.mcserver.restart(&id)))
}

#[tauri::command]
pub async fn srv_kill(core: Core<'_>, id: String) -> Res<()> {
    run!(core, |c| audited(&c, "mcserver.kill", &id, c.mcserver.kill(&id)))
}

#[tauri::command]
pub async fn srv_console(core: Core<'_>, id: String, after: Option<u64>) -> Res<Vec<Line>> {
    run!(core, |c| Ok(c.mcserver.console(&id, after)))
}

#[tauri::command]
pub async fn srv_command(core: Core<'_>, id: String, text: String) -> Res<()> {
    run!(core, |c| audited(&c, "mcserver.command", &text.chars().take(120).collect::<String>(), c.mcserver.command_line(&id, &text)))
}

#[tauri::command]
pub async fn srv_properties(core: Core<'_>, id: String) -> Res<Vec<(String, String)>> {
    run!(core, |c| c.mcserver.properties(&id))
}

#[tauri::command]
pub async fn srv_set_properties(core: Core<'_>, id: String, changes: Vec<(String, String)>) -> Res<View> {
    run!(core, |c| c.mcserver.set_properties(&id, &changes))
}

#[tauri::command]
pub async fn srv_set_icon(core: Core<'_>, id: String, path: String) -> Res<View> {
    run!(core, |c| c.mcserver.set_icon(&id, &path))
}

#[tauri::command]
pub async fn srv_players(core: Core<'_>, id: String) -> Res<PlayersView> {
    run!(core, |c| c.mcserver.players(&id))
}

#[tauri::command]
pub async fn srv_player(core: Core<'_>, id: String, name: String) -> Res<PlayerView> {
    run!(core, |c| c.mcserver.player(&id, &name))
}

#[tauri::command]
pub async fn srv_act(core: Core<'_>, id: String, name: String, action: Action) -> Res<String> {
    run!(core, |c| {
        let what = format!("{name}: {action:?}");
        audited(&c, "mcserver.player", &what.chars().take(160).collect::<String>(), c.mcserver.act(&id, &name, action))
    })
}

#[tauri::command]
pub async fn srv_whitelist(core: Core<'_>, id: String, on: bool) -> Res<View> {
    run!(core, |c| c.mcserver.set_whitelist(&id, on))
}

#[tauri::command]
pub async fn srv_watch(core: Core<'_>, id: String) -> Res<Vec<Online>> {
    run!(core, |c| Ok(c.mcserver.watch(&id)))
}

#[tauri::command]
pub async fn srv_map(core: Core<'_>, id: String, dim: String) -> Res<MapInfo> {
    run!(core, |c| c.mcserver.map_info(&id, &dim))
}

#[tauri::command]
pub async fn srv_map_tile(core: Core<'_>, id: String, dim: String, x: i32, z: i32) -> Res<String> {
    run!(core, |c| c.mcserver.map_tile(&id, &dim, x, z))
}

#[tauri::command]
pub async fn srv_content(core: Core<'_>, id: String) -> Res<Vec<Content>> {
    run!(core, |c| c.mcserver.content(&id))
}

#[tauri::command]
pub async fn srv_identify(core: Core<'_>, id: String) -> Res<Vec<Content>> {
    run!(core, |c| c.mcserver.identify_content(&id))
}

#[derive(Serialize)]
pub struct Found {
    hits: Vec<Hit>,
    total: u64,
}

#[tauri::command]
pub async fn srv_search(core: Core<'_>, id: String, kind: Kind, query: String, offset: usize) -> Res<Found> {
    run!(core, |c| c.mcserver.search(&id, kind, &query, offset).map(|(hits, total)| Found { hits, total }))
}

#[tauri::command]
pub async fn srv_install(core: Core<'_>, id: String, kind: Kind, project: String) -> Res<Vec<Content>> {
    run!(core, |c| audited(&c, "mcserver.install", &project, c.mcserver.install_content(&id, kind, &project)))
}

#[tauri::command]
pub async fn srv_add_file(core: Core<'_>, id: String, path: String) -> Res<Content> {
    run!(core, |c| audited(&c, "mcserver.install", &path, c.mcserver.add_content_file(&id, &path)))
}

#[tauri::command]
pub async fn srv_set_content(core: Core<'_>, id: String, file: String, on: bool) -> Res<()> {
    run!(core, |c| c.mcserver.set_content(&id, &file, on))
}

#[tauri::command]
pub async fn srv_remove_content(core: Core<'_>, id: String, file: String) -> Res<()> {
    run!(core, |c| audited(&c, "mcserver.uninstall", &file, c.mcserver.remove_content(&id, &file)))
}

#[tauri::command]
pub async fn srv_backups(core: Core<'_>, id: String) -> Res<Vec<Backup>> {
    run!(core, |c| c.mcserver.backups(&id))
}

#[tauri::command]
pub async fn srv_backup(core: Core<'_>, id: String) -> Res<Backup> {
    run!(core, |c| c.mcserver.backup_now(&id, false))
}

#[tauri::command]
pub async fn srv_restore(core: Core<'_>, id: String, name: String) -> Res<String> {
    run!(core, |c| audited(&c, "mcserver.restore", &name, c.mcserver.restore(&id, &name)))
}

#[tauri::command]
pub async fn srv_delete_backup(core: Core<'_>, id: String, name: String) -> Res<()> {
    run!(core, |c| c.mcserver.delete_backup(&id, &name))
}

#[tauri::command]
pub async fn srv_network(core: Core<'_>, id: String) -> Res<NetView> {
    run!(core, |c| c.mcserver.network(&id))
}

#[tauri::command]
pub async fn srv_set_upnp(core: Core<'_>, id: String, on: bool) -> Res<NetView> {
    run!(core, |c| audited(&c, "mcserver.upnp", if on { "on" } else { "off" }, c.mcserver.set_upnp(&id, on)))
}

/// Let friends' connections through Windows Firewall to the server's Java (asks for administrator approval once).
#[tauri::command]
pub async fn srv_fix_firewall(core: Core<'_>, id: String) -> Res<NetView> {
    run!(core, |c| {
        let java = c.mcserver.java_exe(&id).ok_or("Start the server once first, so its Java is there.")?;
        let mask = firewall::PROFILE_PRIVATE | firewall::PROFILE_DOMAIN | firewall::PROFILE_PUBLIC;
        let res = firewall::allow_program(&java, "OmniHub Minecraft server", mask).map_err(|e| e.to_string());
        audited(&c, "firewall.allow", "Minecraft server", res)?;
        c.mcserver.network(&id)
    })
}
