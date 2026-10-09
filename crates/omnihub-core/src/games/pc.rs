//! PC-wide settings that decide how well games run — only ones that make
//! a measurable difference: the monitor at its full refresh rate, a
//! performance power plan, no background game recording, Game Mode,
//! Windows 11's optimizations for windowed games, hardware-accelerated GPU
//! scheduling, and (for aim, not frames) no mouse acceleration. Memory
//! Integrity is reported with a link to Windows' own switch: it costs some
//! performance, but it is a security feature, so the choice stays there.
//!
//! What a setting was before OmniHub changed it is kept in a small journal,
//! so each one can be put back exactly.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum TweakId {
    RefreshRate,
    PowerPlan,
    GameDvr,
    GameMode,
    WindowedGames,
    GpuScheduling,
    MouseAcceleration,
    MemoryIntegrity,
}

impl TweakId {
    pub fn all() -> [TweakId; 8] {
        use TweakId::*;
        [RefreshRate, PowerPlan, GameDvr, GameMode, WindowedGames, GpuScheduling, MouseAcceleration, MemoryIntegrity]
    }

    fn key(self) -> &'static str {
        match self {
            TweakId::RefreshRate => "refreshRate",
            TweakId::PowerPlan => "powerPlan",
            TweakId::GameDvr => "gameDvr",
            TweakId::GameMode => "gameMode",
            TweakId::WindowedGames => "windowedGames",
            TweakId::GpuScheduling => "gpuScheduling",
            TweakId::MouseAcceleration => "mouseAcceleration",
            TweakId::MemoryIntegrity => "memoryIntegrity",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Impact {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tweak {
    pub id: TweakId,
    pub title: String,
    pub description: String,
    pub impact: Impact,
    pub available: bool,
    /// Already the fast setting.
    pub optimized: bool,
    /// The setting now, in words.
    pub current: String,
    pub admin: bool,
    pub restart: bool,
    /// Changed only in Windows' own settings; this opens them.
    pub settings_link: Option<String>,
    /// OmniHub changed it and can put the old value back.
    pub can_undo: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Display {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub hz: u32,
    pub max_hz: u32,
}

/// What sets the ceiling for frames per second.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Machine {
    pub cpu: String,
    pub cores: usize,
    pub threads: usize,
    pub ram_gb: f64,
    pub gpus: Vec<String>,
    pub displays: Vec<Display>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PcStatus {
    pub tweaks: Vec<Tweak>,
    pub machine: Machine,
}

/// Previous values, by tweak.
type Journal = HashMap<String, Value>;

fn read_journal(path: &Path) -> Journal {
    std::fs::read(path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

fn write_journal(path: &Path, j: &Journal) {
    if let Err(e) = std::fs::write(path, serde_json::to_vec_pretty(j).unwrap_or_default()) {
        tracing::warn!("pc tweaks journal: {e}");
    }
}

/// `k=v;` lists in Windows' DirectX settings.
pub fn dx_get(list: &str, key: &str) -> Option<String> {
    list.split(';').filter_map(|p| p.split_once('=')).find(|(k, _)| k.trim().eq_ignore_ascii_case(key)).map(|(_, v)| v.trim().to_string())
}

pub fn dx_set(list: &str, key: &str, value: Option<&str>) -> String {
    let mut parts: Vec<String> = list.split(';').map(str::trim).filter(|p| !p.is_empty()).filter(|p| p.split_once('=').is_none_or(|(k, _)| !k.trim().eq_ignore_ascii_case(key))).map(str::to_string).collect();
    if let Some(v) = value {
        parts.push(format!("{key}={v}"));
    }
    parts.iter().map(|p| format!("{p};")).collect()
}

fn machine() -> Machine {
    use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
    let sys = System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing()).with_memory(MemoryRefreshKind::nothing().with_ram()));
    Machine {
        cpu: sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default(),
        cores: System::physical_core_count().unwrap_or(0),
        threads: sys.cpus().len(),
        ram_gb: (sys.total_memory() as f64 / 1024f64.powi(3) * 10.0).round() / 10.0,
        gpus: crate::system::gpu::adapter_names(),
        displays: imp::displays(),
    }
}

fn describe(id: TweakId) -> (&'static str, &'static str, Impact, bool, bool) {
    // (title, description, impact, admin, restart)
    match id {
        TweakId::RefreshRate => ("Monitor at its full refresh rate", "Windows often leaves a 144 or 240 Hz monitor at 60 Hz. Every frame above that is drawn but never shown.", Impact::High, false, false),
        TweakId::PowerPlan => ("Ultimate Performance power plan", "Keeps the processor at full speed instead of waiting to ramp up — fewer stutters and higher lows. Uses more power (on a laptop, keep it plugged in).", Impact::Medium, false, false),
        TweakId::GameDvr => ("Background game recording off", "Xbox Game Bar can record games in the background (Game DVR), which costs frames. Screenshots with Win+Shift+S still work.", Impact::Medium, false, false),
        TweakId::GameMode => ("Game Mode", "Windows holds back updates and background work while a game runs.", Impact::Low, false, false),
        TweakId::WindowedGames => ("Optimizations for windowed games", "Windows 11 runs games in borderless windows with the same low input delay as fullscreen.", Impact::Medium, false, false),
        TweakId::GpuScheduling => ("Hardware-accelerated GPU scheduling", "The graphics card schedules its own work: lower delay, and needed for DLSS frame generation. Takes effect after a restart.", Impact::Medium, true, true),
        TweakId::MouseAcceleration => ("Mouse acceleration off", "“Enhance pointer precision” off, so the same hand movement always turns the same amount — for aim, not frames.", Impact::Low, false, false),
        TweakId::MemoryIntegrity => ("Memory Integrity", "A Windows security feature that Microsoft says can lower game performance. Your choice, in Windows Security → Core isolation.", Impact::High, false, true),
    }
}

/// Every tweak and its state.
pub fn status(journal_path: &Path, remembered_plan: Option<&str>) -> PcStatus {
    let j = read_journal(journal_path);
    let tweaks = TweakId::all()
        .into_iter()
        .map(|id| {
            let (title, description, impact, admin, restart) = describe(id);
            let s = imp::state(id, remembered_plan);
            Tweak { id, title: title.into(), description: description.into(), impact, available: s.available, optimized: s.optimized, current: s.current, admin, restart, settings_link: (id == TweakId::MemoryIntegrity).then(|| "windowsdefender://coreisolation".to_string()), can_undo: j.contains_key(id.key()) }
        })
        .collect();
    PcStatus { tweaks, machine: machine() }
}

/// Turn a tweak on (the fast setting) or put back what was there. Returns
/// the GUID of a power plan OmniHub created, to remember.
pub fn set(id: TweakId, on: bool, journal_path: &Path, remembered_plan: Option<&str>) -> Result<Option<String>, String> {
    let mut j = read_journal(journal_path);
    let key = id.key();
    if on {
        let (before, created) = imp::apply(id, remembered_plan)?;
        // Keep the first "before": that is your own setting.
        j.entry(key.to_string()).or_insert(before);
        write_journal(journal_path, &j);
        Ok(created)
    } else {
        imp::undo(id, j.get(key).cloned().unwrap_or(Value::Null))?;
        j.remove(key);
        write_journal(journal_path, &j);
        Ok(None)
    }
}

/// Admin-only changes, run by the elevated helper ("hags+", "hags-").
pub fn helper_admin(args: &[String]) -> i32 {
    match args {
        [a] if a == "hags+" => i32::from(imp::set_hags(true).is_err()),
        [a] if a == "hags-" => i32::from(imp::set_hags(false).is_err()),
        _ => 64,
    }
}

struct State {
    available: bool,
    optimized: bool,
    current: String,
}

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::games::tweaks;
    use serde_json::json;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;

    fn read_dword(root: winreg::HKEY, path: &str, name: &str) -> Option<u32> {
        RegKey::predef(root).open_subkey_with_flags(path, KEY_READ).ok()?.get_value::<u32, _>(name).ok()
    }

    fn read_string(root: winreg::HKEY, path: &str, name: &str) -> Option<String> {
        RegKey::predef(root).open_subkey_with_flags(path, KEY_READ).ok()?.get_value::<String, _>(name).ok()
    }

    fn hkcu_set_dword(path: &str, name: &str, v: Option<u32>) -> Result<(), String> {
        let (k, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(path).map_err(|e| e.to_string())?;
        match v {
            Some(v) => k.set_value(name, &v).map_err(|e| e.to_string()),
            None => match k.delete_value(name) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
                _ => Ok(()),
            },
        }
    }

    fn build() -> u32 {
        read_string(HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "CurrentBuildNumber").and_then(|b| b.parse().ok()).unwrap_or(0)
    }

    const DVR_STORE: &str = r"System\GameConfigStore";
    const DVR_APP: &str = r"Software\Microsoft\Windows\CurrentVersion\GameDVR";
    const GAMEBAR: &str = r"Software\Microsoft\GameBar";
    const DX: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";
    const DX_GLOBAL: &str = "DirectXUserGlobalSettings";
    const GRAPHICS: &str = r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers";
    const HVCI: &str = r"SYSTEM\CurrentControlSet\Control\DeviceGuard\Scenarios\HypervisorEnforcedCodeIntegrity";

    // ---------- displays ----------

    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Gdi::{ChangeDisplaySettingsExW, EnumDisplayDevicesW, EnumDisplaySettingsW, CDS_UPDATEREGISTRY, DEVMODEW, DISPLAY_DEVICEW, DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISP_CHANGE_SUCCESSFUL, DM_DISPLAYFREQUENCY, ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_MODE};

    struct Screen {
        device: [u16; 32],
        name: String,
        current: DEVMODEW,
        max_hz: u32,
    }

    fn wide_str(w: &[u16]) -> String {
        String::from_utf16_lossy(&w[..w.iter().position(|&c| c == 0).unwrap_or(w.len())])
    }

    fn screens() -> Vec<Screen> {
        let mut out = Vec::new();
        unsafe {
            let mut i = 0;
            loop {
                let mut dd = DISPLAY_DEVICEW { cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
                if !EnumDisplayDevicesW(PCWSTR::null(), i, &mut dd, 0).as_bool() {
                    break;
                }
                i += 1;
                if !dd.StateFlags.contains(DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) {
                    continue;
                }
                let dev = PCWSTR(dd.DeviceName.as_ptr());
                let mut cur = DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() };
                if !EnumDisplaySettingsW(dev, ENUM_CURRENT_SETTINGS, &mut cur).as_bool() {
                    continue;
                }
                let mut max_hz = cur.dmDisplayFrequency;
                let mut m = 0;
                loop {
                    let mut mode = DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() };
                    if !EnumDisplaySettingsW(dev, ENUM_DISPLAY_SETTINGS_MODE(m), &mut mode).as_bool() {
                        break;
                    }
                    m += 1;
                    if mode.dmPelsWidth == cur.dmPelsWidth && mode.dmPelsHeight == cur.dmPelsHeight && mode.dmBitsPerPel == cur.dmBitsPerPel && mode.dmDisplayFrequency > max_hz && mode.dmDisplayFrequency < 1000 {
                        max_hz = mode.dmDisplayFrequency;
                    }
                }
                // The monitor's name, from the device on that output.
                let mut mon = DISPLAY_DEVICEW { cb: std::mem::size_of::<DISPLAY_DEVICEW>() as u32, ..Default::default() };
                let name = if EnumDisplayDevicesW(dev, 0, &mut mon, 0).as_bool() { wide_str(&mon.DeviceString) } else { wide_str(&dd.DeviceString) };
                out.push(Screen { device: dd.DeviceName, name, current: cur, max_hz });
            }
        }
        out
    }

    pub fn displays() -> Vec<Display> {
        screens().into_iter().map(|s| Display { name: s.name, width: s.current.dmPelsWidth, height: s.current.dmPelsHeight, hz: s.current.dmDisplayFrequency, max_hz: s.max_hz }).collect()
    }

    fn set_hz(device: &[u16; 32], current: &DEVMODEW, hz: u32) -> Result<(), String> {
        let mut mode = *current;
        mode.dmDisplayFrequency = hz;
        mode.dmFields = DM_DISPLAYFREQUENCY;
        let r = unsafe { ChangeDisplaySettingsExW(PCWSTR(device.as_ptr()), Some(&mode), None, CDS_UPDATEREGISTRY, None) };
        if r == DISP_CHANGE_SUCCESSFUL {
            Ok(())
        } else {
            Err(format!("Windows did not switch {} to {hz} Hz (code {}).", wide_str(device), r.0))
        }
    }

    // ---------- mouse ----------

    use windows::Win32::UI::WindowsAndMessaging::{SystemParametersInfoW, SPIF_SENDCHANGE, SPIF_UPDATEINIFILE, SPI_GETMOUSE, SPI_SETMOUSE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS};

    fn mouse() -> Option<[i32; 3]> {
        let mut p = [0i32; 3];
        unsafe { SystemParametersInfoW(SPI_GETMOUSE, 0, Some(p.as_mut_ptr().cast()), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)) }.ok()?;
        Some(p)
    }

    fn set_mouse(mut p: [i32; 3]) -> Result<(), String> {
        unsafe { SystemParametersInfoW(SPI_SETMOUSE, 0, Some(p.as_mut_ptr().cast()), SPIF_UPDATEINIFILE | SPIF_SENDCHANGE) }.map_err(|e| e.message())
    }

    // ---------- state ----------

    pub(super) fn state(id: TweakId, remembered_plan: Option<&str>) -> State {
        let st = |available: bool, optimized: bool, current: String| State { available, optimized, current };
        match id {
            TweakId::RefreshRate => {
                let s = screens();
                if s.is_empty() {
                    return st(false, true, "No monitor found".into());
                }
                let slow: Vec<String> = s.iter().filter(|x| x.current.dmDisplayFrequency < x.max_hz).map(|x| format!("{} at {} Hz — can do {} Hz", x.name, x.current.dmDisplayFrequency, x.max_hz)).collect();
                if slow.is_empty() {
                    st(true, true, s.iter().map(|x| format!("{} Hz", x.current.dmDisplayFrequency)).collect::<Vec<_>>().join(", "))
                } else {
                    st(true, false, slow.join("; "))
                }
            }
            TweakId::PowerPlan => {
                let active = tweaks::active_plan();
                let fast = active.as_deref().is_some_and(|a| a == tweaks::ULTIMATE_PERFORMANCE || a == tweaks::HIGH_PERFORMANCE || Some(a) == remembered_plan);
                st(true, fast, if fast { "High or Ultimate Performance".into() } else { "Balanced or power saving".into() })
            }
            TweakId::GameDvr => {
                let store = read_dword(HKEY_CURRENT_USER, DVR_STORE, "GameDVR_Enabled");
                let app = read_dword(HKEY_CURRENT_USER, DVR_APP, "AppCaptureEnabled");
                let off = store == Some(0) && app == Some(0);
                st(true, off, if off { "Off".into() } else { "On".into() })
            }
            TweakId::GameMode => {
                let on = read_dword(HKEY_CURRENT_USER, GAMEBAR, "AutoGameModeEnabled") != Some(0);
                st(true, on, if on { "On".into() } else { "Off".into() })
            }
            TweakId::WindowedGames => {
                if build() < 22000 {
                    return st(false, false, "Needs Windows 11".into());
                }
                let on = read_string(HKEY_CURRENT_USER, DX, DX_GLOBAL).and_then(|l| dx_get(&l, "SwapEffectUpgradeEnable")).as_deref() == Some("1");
                st(true, on, if on { "On".into() } else { "Off".into() })
            }
            TweakId::GpuScheduling => {
                if build() < 19041 {
                    return st(false, false, "Needs Windows 10 2004 or later".into());
                }
                let on = read_dword(HKEY_LOCAL_MACHINE, GRAPHICS, "HwSchMode") == Some(2);
                st(true, on, if on { "On".into() } else { "Off".into() })
            }
            TweakId::MouseAcceleration => match mouse() {
                Some(p) => st(true, p[2] == 0, if p[2] == 0 { "Off".into() } else { "On".into() }),
                None => st(false, false, "Unknown".into()),
            },
            TweakId::MemoryIntegrity => {
                let on = read_dword(HKEY_LOCAL_MACHINE, HVCI, "Enabled") == Some(1);
                st(true, !on, if on { "On".into() } else { "Off".into() })
            }
        }
    }

    // ---------- apply / undo ----------

    pub(super) fn apply(id: TweakId, remembered_plan: Option<&str>) -> Result<(Value, Option<String>), String> {
        match id {
            TweakId::RefreshRate => {
                let mut before = Vec::new();
                let mut errors = Vec::new();
                for s in screens().into_iter().filter(|s| s.current.dmDisplayFrequency < s.max_hz) {
                    match set_hz(&s.device, &s.current, s.max_hz) {
                        Ok(()) => before.push(json!({ "device": wide_str(&s.device), "hz": s.current.dmDisplayFrequency })),
                        Err(e) => errors.push(e),
                    }
                }
                if before.is_empty() && !errors.is_empty() {
                    return Err(errors.join(" "));
                }
                Ok((Value::Array(before), None))
            }
            TweakId::PowerPlan => {
                let before = tweaks::active_plan();
                let (_, _, created) = tweaks::activate_plan(tweaks::PowerPlan::Ultimate, remembered_plan)?;
                Ok((json!(before), created))
            }
            TweakId::GameDvr => {
                let before = json!({ "store": read_dword(HKEY_CURRENT_USER, DVR_STORE, "GameDVR_Enabled"), "app": read_dword(HKEY_CURRENT_USER, DVR_APP, "AppCaptureEnabled") });
                hkcu_set_dword(DVR_STORE, "GameDVR_Enabled", Some(0))?;
                hkcu_set_dword(DVR_APP, "AppCaptureEnabled", Some(0))?;
                Ok((before, None))
            }
            TweakId::GameMode => {
                let before = json!(read_dword(HKEY_CURRENT_USER, GAMEBAR, "AutoGameModeEnabled"));
                hkcu_set_dword(GAMEBAR, "AutoGameModeEnabled", Some(1))?;
                Ok((before, None))
            }
            TweakId::WindowedGames => {
                let before = read_string(HKEY_CURRENT_USER, DX, DX_GLOBAL);
                let next = dx_set(before.as_deref().unwrap_or_default(), "SwapEffectUpgradeEnable", Some("1"));
                RegKey::predef(HKEY_CURRENT_USER).create_subkey(DX).and_then(|(k, _)| k.set_value(DX_GLOBAL, &next)).map_err(|e| e.to_string())?;
                Ok((json!(before), None))
            }
            TweakId::GpuScheduling => {
                let before = read_dword(HKEY_LOCAL_MACHINE, GRAPHICS, "HwSchMode");
                run_admin("hags+")?;
                Ok((json!(before), None))
            }
            TweakId::MouseAcceleration => {
                let before = mouse().ok_or("Could not read the mouse settings.")?;
                set_mouse([0, 0, 0])?;
                Ok((json!(before), None))
            }
            TweakId::MemoryIntegrity => Err("Memory Integrity is changed in Windows Security → Core isolation.".into()),
        }
    }

    pub(super) fn undo(id: TweakId, before: Value) -> Result<(), String> {
        match id {
            TweakId::RefreshRate => {
                let screens = screens();
                for b in before.as_array().cloned().unwrap_or_default() {
                    let (Some(dev), Some(hz)) = (b.get("device").and_then(Value::as_str), b.get("hz").and_then(Value::as_u64)) else { continue };
                    if let Some(s) = screens.iter().find(|s| wide_str(&s.device) == dev) {
                        set_hz(&s.device, &s.current, hz as u32)?;
                    }
                }
                Ok(())
            }
            TweakId::PowerPlan => match before.as_str() {
                Some(g) => tweaks::set_plan(g),
                None => tweaks::set_plan("381b4222-f694-41f0-9685-ff5bb260df2e"),
            },
            TweakId::GameDvr => {
                let n = |k: &str| before.get(k).and_then(Value::as_u64).map(|v| v as u32);
                hkcu_set_dword(DVR_STORE, "GameDVR_Enabled", n("store").or(Some(1)))?;
                hkcu_set_dword(DVR_APP, "AppCaptureEnabled", n("app"))
            }
            TweakId::GameMode => hkcu_set_dword(GAMEBAR, "AutoGameModeEnabled", before.as_u64().map(|v| v as u32)),
            TweakId::WindowedGames => {
                let k = RegKey::predef(HKEY_CURRENT_USER).create_subkey(DX).map_err(|e| e.to_string())?.0;
                match before.as_str() {
                    Some(v) => k.set_value(DX_GLOBAL, &v.to_string()).map_err(|e| e.to_string()),
                    None => {
                        let now = k.get_value::<String, _>(DX_GLOBAL).unwrap_or_default();
                        let rest = dx_set(&now, "SwapEffectUpgradeEnable", None);
                        if rest.is_empty() {
                            let _ = k.delete_value(DX_GLOBAL);
                            Ok(())
                        } else {
                            k.set_value(DX_GLOBAL, &rest).map_err(|e| e.to_string())
                        }
                    }
                }
            }
            TweakId::GpuScheduling => run_admin("hags-"),
            TweakId::MouseAcceleration => {
                let p = before.as_array().and_then(|a| (a.len() == 3).then(|| [a[0].as_i64().unwrap_or(6) as i32, a[1].as_i64().unwrap_or(10) as i32, a[2].as_i64().unwrap_or(1) as i32])).unwrap_or([6, 10, 1]);
                set_mouse(p)
            }
            TweakId::MemoryIntegrity => Ok(()),
        }
    }

    fn run_admin(op: &str) -> Result<(), String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let args = vec![crate::helper::HELPER_FLAG.to_string(), "pc-admin".into(), op.into()];
        match crate::system::elevation::run_elevated_and_wait(&exe, &args, false) {
            Ok(0) => Ok(()),
            Ok(code) => Err(format!("The change did not go through (code {code}).")),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Err("Windows' administrator prompt was declined.".into()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn set_hags(on: bool) -> Result<(), String> {
        RegKey::predef(HKEY_LOCAL_MACHINE).create_subkey(GRAPHICS).and_then(|(k, _)| k.set_value("HwSchMode", &(if on { 2u32 } else { 1u32 }))).map_err(|e| e.to_string())
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn displays() -> Vec<Display> {
        Vec::new()
    }

    pub(super) fn state(_id: TweakId, _remembered_plan: Option<&str>) -> State {
        State { available: false, optimized: false, current: "Windows only".into() }
    }

    pub(super) fn apply(_id: TweakId, _remembered_plan: Option<&str>) -> Result<(Value, Option<String>), String> {
        Err("Windows only".into())
    }

    pub(super) fn undo(_id: TweakId, _before: Value) -> Result<(), String> {
        Err("Windows only".into())
    }

    pub fn set_hags(_on: bool) -> Result<(), String> {
        Err("Windows only".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directx_setting_lists() {
        assert_eq!(dx_get("VRROptimizeEnable=0;SwapEffectUpgradeEnable=1;", "swapeffectupgradeenable").as_deref(), Some("1"));
        assert_eq!(dx_set("VRROptimizeEnable=0;", "SwapEffectUpgradeEnable", Some("1")), "VRROptimizeEnable=0;SwapEffectUpgradeEnable=1;");
        assert_eq!(dx_set("SwapEffectUpgradeEnable=0;VRROptimizeEnable=0;", "SwapEffectUpgradeEnable", Some("1")), "VRROptimizeEnable=0;SwapEffectUpgradeEnable=1;");
        assert_eq!(dx_set("SwapEffectUpgradeEnable=1;", "SwapEffectUpgradeEnable", None), "");
        assert_eq!(dx_set("", "SwapEffectUpgradeEnable", Some("1")), "SwapEffectUpgradeEnable=1;");
    }

    #[test]
    fn status_lists_every_tweak() {
        let dir = tempfile::tempdir().unwrap();
        let s = status(&dir.path().join("j.json"), None);
        assert_eq!(s.tweaks.len(), TweakId::all().len());
        assert!(s.tweaks.iter().any(|t| t.id == TweakId::MemoryIntegrity && t.settings_link.is_some()));
        assert!(s.machine.threads > 0);
        assert_eq!(helper_admin(&["rm".into()]), 64);
    }
}
