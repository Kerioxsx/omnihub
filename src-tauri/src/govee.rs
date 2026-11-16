//! Govee lights in the shell: the Lights page's commands, and Windows
//! Firewall for the lights' answers (UDP 4002).

use std::sync::Arc;

use omnihub_core::capture::stream::MonitorInfo;
use omnihub_core::core::AppCore;
use omnihub_core::govee::schedule::Timer;
use omnihub_core::govee::{GoveeSettings, GoveeStatus, Preset};
use omnihub_core::system::firewall::{self, FirewallReport, Proto};
use tauri::State;

type Core<'a> = State<'a, Arc<AppCore>>;
type Res<T> = Result<T, String>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Res<T> + Send + 'static) -> Res<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn govee_settings(core: Core<'_>) -> Res<GoveeSettings> {
    let mut s = core.govee.settings();
    // The window never sees the key, only whether there is one.
    s.api_secret.clear();
    Ok(s)
}

#[tauri::command]
pub async fn govee_update(core: Core<'_>, patch: serde_json::Value) -> Res<GoveeSettings> {
    let core = core.inner().clone();
    blocking(move || core.govee.update(&patch).map(|mut s| {
        s.api_secret.clear();
        s
    }))
    .await
}

#[tauri::command]
pub async fn govee_update_light(core: Core<'_>, id: String, patch: serde_json::Value) -> Res<GoveeSettings> {
    let core = core.inner().clone();
    blocking(move || core.govee.update_light(&id, &patch).map(|mut s| {
        s.api_secret.clear();
        s
    }))
    .await
}

/// The page is open (asked every few seconds): colours and states keep coming.
#[tauri::command]
pub async fn govee_hold(core: Core<'_>) -> Res<GoveeStatus> {
    Ok(core.govee.hold())
}

#[tauri::command]
pub async fn govee_scan(core: Core<'_>) -> Res<()> {
    core.govee.rescan();
    Ok(())
}

/// The Govee API key: kept encrypted for this Windows account; empty removes it.
#[tauri::command]
pub async fn govee_set_key(core: Core<'_>, key: String) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || {
        let had = !core.govee.settings().api_secret.is_empty();
        let res = core.govee.set_key(&key);
        core.audit.record("desktop", "govee.key", if key.trim().is_empty() { "removed" } else if had { "replaced" } else { "added" }, res.is_ok());
        res
    })
    .await
}

#[tauri::command]
pub async fn govee_control(core: Core<'_>, id: String, cap_type: String, instance: String, value: serde_json::Value) -> Res<()> {
    core.govee.control(&id, &cap_type, &instance, value)
}

#[tauri::command]
pub async fn govee_identify(core: Core<'_>, id: String) -> Res<()> {
    core.govee.identify(&id);
    Ok(())
}

#[tauri::command]
pub async fn govee_presets(core: Core<'_>) -> Res<Vec<Preset>> {
    Ok(core.govee.presets())
}

#[tauri::command]
pub async fn govee_save_preset(core: Core<'_>, preset: Preset) -> Res<Preset> {
    let core = core.inner().clone();
    blocking(move || core.govee.save_preset(preset)).await
}

#[tauri::command]
pub async fn govee_delete_preset(core: Core<'_>, id: String) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || core.govee.delete_preset(&id)).await
}

#[tauri::command]
pub async fn govee_apply_preset(core: Core<'_>, id: String, lights: Vec<String>) -> Res<GoveeSettings> {
    let core = core.inner().clone();
    blocking(move || core.govee.apply_preset(&id, &lights).map(|mut s| {
        s.api_secret.clear();
        s
    }))
    .await
}

#[tauri::command]
pub async fn govee_save_timer(core: Core<'_>, timer: Timer) -> Res<Timer> {
    let core = core.inner().clone();
    blocking(move || core.govee.save_timer(timer)).await
}

#[tauri::command]
pub async fn govee_delete_timer(core: Core<'_>, id: String) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || core.govee.delete_timer(&id)).await
}

/// The sleep timer: off in `minutes` (none cancels it).
#[tauri::command]
pub async fn govee_sleep(core: Core<'_>, minutes: Option<u32>) -> Res<GoveeStatus> {
    Ok(core.govee.sleep(minutes))
}

/// The monitors screen sync can follow.
#[tauri::command]
pub async fn govee_monitors() -> Res<Vec<MonitorInfo>> {
    blocking(|| Ok(omnihub_core::capture::stream::monitors())).await
}

/// Whether the lights' answers (UDP 4002) get through Windows Firewall.
#[tauri::command]
pub async fn govee_firewall() -> Res<FirewallReport> {
    blocking(|| {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        Ok(firewall::check(&exe, Some(omnihub_core::govee::lan::Ports::default().listen), Proto::Udp, "Govee lights"))
    })
    .await
}

/// Let the lights' answers through (asks for administrator approval once).
#[tauri::command]
pub async fn govee_fix_firewall(core: Core<'_>) -> Res<FirewallReport> {
    let core = core.inner().clone();
    blocking(move || {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let mask = firewall::PROFILE_PRIVATE | firewall::PROFILE_DOMAIN;
        let res = firewall::allow_program(&exe, "OmniHub Govee lights", mask);
        core.audit.record("desktop", "firewall.allow", "Govee lights", res.is_ok());
        res.map_err(|e| e.to_string())?;
        core.govee.rescan();
        Ok(firewall::check(&exe, Some(omnihub_core::govee::lan::Ports::default().listen), Proto::Udp, "Govee lights"))
    })
    .await
}
