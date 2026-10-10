//! The Windows settings a game boost touches. Session tweaks remember what
//! was there before so it can be put back; per-game settings (GPU choice,
//! fullscreen optimizations, network priority, start priority) stay until
//! they are turned off in the profile.
//!
//! On other systems every tweak reports that it is Windows-only.

use serde::{Deserialize, Serialize};

pub const HIGH_PERFORMANCE: &str = "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c";
pub const ULTIMATE_PERFORMANCE: &str = "e9a42b02-d5df-448d-aa00-03f14749eb61";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum PowerPlan {
    /// Leave the plan alone.
    Keep,
    High,
    #[default]
    Ultimate,
}

/// Every GUID in powercfg's output (its text is translated, GUIDs are not).
pub fn guids(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 36 <= b.len() {
        let s = &text[i..i + 36];
        let ok = s.char_indices().all(|(k, c)| if matches!(k, 8 | 13 | 18 | 23) { c == '-' } else { c.is_ascii_hexdigit() });
        if ok && (i == 0 || !b[i - 1].is_ascii_hexdigit()) {
            out.push(s.to_ascii_lowercase());
            i += 36;
        } else {
            i += 1;
        }
    }
    out
}

/// Names a game's executable may have ("FortniteClient-Win64-Shipping.exe",
/// "League of Legends.exe"); anything else never reaches the registry or a
/// command line.
pub fn valid_exe_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".exe").or_else(|| name.strip_suffix(".EXE")) else { return false };
    !stem.is_empty() && name.len() <= 100 && stem.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) && stem.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '_' | '-' | '(' | ')'))
}

/// The admin-only per-game settings, as helper arguments ("qos+:Game.exe").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminOp {
    QosOn(String),
    QosOff(String),
    StartHighOn(String),
    StartHighOff(String),
}

impl AdminOp {
    pub fn arg(&self) -> String {
        match self {
            AdminOp::QosOn(e) => format!("qos+:{e}"),
            AdminOp::QosOff(e) => format!("qos-:{e}"),
            AdminOp::StartHighOn(e) => format!("ifeo+:{e}"),
            AdminOp::StartHighOff(e) => format!("ifeo-:{e}"),
        }
    }

    pub fn parse(arg: &str) -> Option<AdminOp> {
        let (op, exe) = arg.split_once(':')?;
        if !valid_exe_name(exe) {
            return None;
        }
        let exe = exe.to_string();
        Some(match op {
            "qos+" => AdminOp::QosOn(exe),
            "qos-" => AdminOp::QosOff(exe),
            "ifeo+" => AdminOp::StartHighOn(exe),
            "ifeo-" => AdminOp::StartHighOff(exe),
            _ => return None,
        })
    }
}

pub fn qos_policy_name(exe: &str) -> String {
    format!("OmniHub - {exe}")
}

pub use imp::*;

#[cfg(windows)]
mod imp {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE};
    use winreg::RegKey;

    use super::*;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn run(exe: &str, args: &[&str]) -> Result<String, String> {
        let out = Command::new(exe).args(args).creation_flags(CREATE_NO_WINDOW).output().map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        if out.status.success() {
            Ok(text)
        } else {
            Err(format!("{exe} failed: {}", String::from_utf8_lossy(&out.stderr).trim()))
        }
    }

    // ---------- power plan ----------

    pub fn active_plan() -> Option<String> {
        guids(&run("powercfg", &["/getactivescheme"]).ok()?).into_iter().next()
    }

    pub fn set_plan(guid: &str) -> Result<(), String> {
        run("powercfg", &["/setactive", guid]).map(|_| ())
    }

    /// Switch to the chosen plan, creating "Ultimate Performance" (hidden by
    /// default) from Windows' template the first time. Returns the plan used
    /// and, when one was created, its GUID to remember.
    pub fn activate_plan(choice: PowerPlan, remembered: Option<&str>) -> Result<(String, &'static str, Option<String>), String> {
        let list = guids(&run("powercfg", &["/list"])?);
        let mut created = None;
        let mut pick = |base: &str, remembered: Option<&str>| -> Option<String> {
            if list.iter().any(|g| g == base) {
                return Some(base.to_string());
            }
            if let Some(r) = remembered.filter(|r| list.iter().any(|g| g == r)) {
                return Some(r.to_string());
            }
            let made = guids(&run("powercfg", &["-duplicatescheme", base]).ok()?).into_iter().find(|g| g != base)?;
            created = Some(made.clone());
            Some(made)
        };
        let (guid, label) = match choice {
            PowerPlan::Keep => return Err("kept".into()),
            PowerPlan::Ultimate => match pick(ULTIMATE_PERFORMANCE, remembered) {
                Some(g) => (g, "Ultimate Performance"),
                None => (pick(HIGH_PERFORMANCE, None).ok_or("This PC only offers the Balanced plan (common on laptops with Modern Standby).")?, "High performance"),
            },
            PowerPlan::High => (pick(HIGH_PERFORMANCE, None).ok_or("This PC only offers the Balanced plan (common on laptops with Modern Standby).")?, "High performance"),
        };
        set_plan(&guid)?;
        Ok((guid, label, created))
    }

    // ---------- registry helpers ----------

    fn hkcu(path: &str) -> std::io::Result<RegKey> {
        RegKey::predef(HKEY_CURRENT_USER).create_subkey(path).map(|(k, _)| k)
    }

    fn read_dword(root: winreg::HKEY, path: &str, name: &str) -> Option<u32> {
        RegKey::predef(root).open_subkey_with_flags(path, KEY_READ).ok()?.get_value::<u32, _>(name).ok()
    }

    fn read_string(root: winreg::HKEY, path: &str, name: &str) -> Option<String> {
        RegKey::predef(root).open_subkey_with_flags(path, KEY_READ).ok()?.get_value::<String, _>(name).ok()
    }

    // ---------- notifications ----------

    const PUSH: &str = r"Software\Microsoft\Windows\CurrentVersion\PushNotifications";

    /// Turn notification pop-ups off; returns the previous value to restore.
    pub fn silence_notifications() -> Result<Option<u32>, String> {
        let before = read_dword(HKEY_CURRENT_USER, PUSH, "ToastEnabled");
        hkcu(PUSH).and_then(|k| k.set_value("ToastEnabled", &0u32)).map_err(|e| e.to_string())?;
        Ok(before)
    }

    pub fn restore_notifications(before: Option<u32>) -> Result<(), String> {
        let k = hkcu(PUSH).map_err(|e| e.to_string())?;
        match before {
            Some(v) => k.set_value("ToastEnabled", &v),
            None => k.delete_value("ToastEnabled").or_else(|e| if e.kind() == std::io::ErrorKind::NotFound { Ok(()) } else { Err(e) }),
        }
        .map_err(|e| e.to_string())
    }

    // ---------- Game Mode ----------

    /// Make sure Windows Game Mode is on. Returns true when it was off.
    pub fn ensure_game_mode() -> Result<bool, String> {
        const P: &str = r"Software\Microsoft\GameBar";
        if read_dword(HKEY_CURRENT_USER, P, "AutoGameModeEnabled") == Some(0) {
            hkcu(P).and_then(|k| k.set_value("AutoGameModeEnabled", &1u32)).map_err(|e| e.to_string())?;
            return Ok(true);
        }
        Ok(false)
    }

    // ---------- per-game: GPU and fullscreen optimizations ----------

    const GPU: &str = r"Software\Microsoft\DirectX\UserGpuPreferences";
    const LAYERS: &str = r"Software\Microsoft\Windows NT\CurrentVersion\AppCompatFlags\Layers";
    const NO_FSO: &str = "DISABLEDXMAXIMIZEDWINDOWEDMODE";

    /// The same choice as Settings → Display → Graphics → High performance.
    pub fn set_gpu_high_performance(exe_path: &str, on: bool) -> Result<bool, String> {
        let k = hkcu(GPU).map_err(|e| e.to_string())?;
        let current = k.get_value::<String, _>(exe_path).ok();
        if on {
            if current.as_deref().is_some_and(|v| v.contains("GpuPreference=2;")) {
                return Ok(false);
            }
            k.set_value(exe_path, &"GpuPreference=2;").map_err(|e| e.to_string())?;
            Ok(true)
        } else if current.is_some() {
            k.delete_value(exe_path).map_err(|e| e.to_string())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// The same as the program's Properties → Compatibility → "Disable
    /// fullscreen optimizations". Other compatibility flags are kept.
    pub fn set_fullscreen_optimizations_off(exe_path: &str, on: bool) -> Result<bool, String> {
        let k = hkcu(LAYERS).map_err(|e| e.to_string())?;
        let current = k.get_value::<String, _>(exe_path).ok().unwrap_or_default();
        let mut tokens: Vec<&str> = current.split_whitespace().filter(|t| *t != "~" && *t != NO_FSO).collect();
        let had = current.split_whitespace().any(|t| t == NO_FSO);
        if on == had {
            return Ok(false);
        }
        if on {
            tokens.push(NO_FSO);
        }
        if tokens.is_empty() {
            k.delete_value(exe_path).map_err(|e| e.to_string())?;
        } else {
            k.set_value(exe_path, &format!("~ {}", tokens.join(" "))).map_err(|e| e.to_string())?;
        }
        Ok(true)
    }

    // ---------- per-game, admin: network priority and start priority ----------

    const QOS: &str = r"SOFTWARE\Policies\Microsoft\Windows\QoS";
    const IFEO: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options";

    pub fn qos_enabled(exe: &str) -> bool {
        read_string(HKEY_LOCAL_MACHINE, &format!(r"{QOS}\{}", qos_policy_name(exe)), "Application Name").is_some_and(|a| a.eq_ignore_ascii_case(exe))
    }

    pub fn start_high_enabled(exe: &str) -> bool {
        read_dword(HKEY_LOCAL_MACHINE, &format!(r"{IFEO}\{exe}\PerfOptions"), "CpuPriorityClass") == Some(3)
    }

    /// Run admin-only changes through the elevated helper (one prompt).
    pub fn run_admin(ops: &[AdminOp]) -> Result<(), String> {
        if ops.is_empty() {
            return Ok(());
        }
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut args = vec![crate::helper::HELPER_FLAG.to_string(), "game-admin".into()];
        args.extend(ops.iter().map(AdminOp::arg));
        match crate::system::elevation::run_elevated_and_wait(&exe, &args, false) {
            Ok(0) => Ok(()),
            Ok(code) => Err(format!("The change did not go through (code {code}).")),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Err("Windows' administrator prompt was declined.".into()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Helper side (elevated): apply validated operations.
    pub fn helper_admin(args: &[String]) -> i32 {
        let ops: Option<Vec<AdminOp>> = args.iter().map(|a| AdminOp::parse(a)).collect();
        let Some(ops) = ops.filter(|o| !o.is_empty()) else { return 64 };
        let mut failed = false;
        for op in ops {
            let r = match &op {
                AdminOp::QosOn(exe) => qos_on(exe),
                AdminOp::QosOff(exe) => run("powershell", &["-NoProfile", "-NonInteractive", "-Command", &format!("Remove-NetQosPolicy -Name '{}' -Confirm:$false", qos_policy_name(exe))]).map(|_| ()),
                AdminOp::StartHighOn(exe) => RegKey::predef(HKEY_LOCAL_MACHINE).create_subkey(format!(r"{IFEO}\{exe}\PerfOptions")).and_then(|(k, _)| k.set_value("CpuPriorityClass", &3u32)).map_err(|e| e.to_string()),
                AdminOp::StartHighOff(exe) => start_high_off(exe),
            };
            if let Err(e) = r {
                tracing::warn!("{}: {e}", op.arg());
                failed = true;
            }
        }
        i32::from(failed)
    }

    fn qos_on(exe: &str) -> Result<(), String> {
        // Windows only applies QoS marking outside a domain with this set.
        RegKey::predef(HKEY_LOCAL_MACHINE).create_subkey(r"SYSTEM\CurrentControlSet\Services\Tcpip\QoS").and_then(|(k, _)| k.set_value("Do not use NLA", &"1")).map_err(|e| e.to_string())?;
        let name = qos_policy_name(exe);
        let _ = run("powershell", &["-NoProfile", "-NonInteractive", "-Command", &format!("Remove-NetQosPolicy -Name '{name}' -Confirm:$false -ErrorAction SilentlyContinue")]);
        run("powershell", &["-NoProfile", "-NonInteractive", "-Command", &format!("New-NetQosPolicy -Name '{name}' -AppPathNameMatchCondition '{exe}' -DSCPAction 46 -NetworkProfile All | Out-Null")]).map(|_| ())
    }

    fn start_high_off(exe: &str) -> Result<(), String> {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let Ok(k) = hklm.open_subkey_with_flags(format!(r"{IFEO}\{exe}\PerfOptions"), KEY_READ | KEY_SET_VALUE) else { return Ok(()) };
        let _ = k.delete_value("CpuPriorityClass");
        if k.enum_values().next().is_none() && k.enum_keys().next().is_none() {
            let _ = hklm.delete_subkey(format!(r"{IFEO}\{exe}\PerfOptions"));
        }
        Ok(())
    }

    // ---------- Wi-Fi low-latency mode ----------

    /// While held, Wi-Fi stops background scans (the cause of ping spikes
    /// every minute or so) and runs in its low-latency streaming mode.
    /// Windows undoes both when the handle closes — even if OmniHub quits.
    pub struct WifiLowLatency(windows::Win32::Foundation::HANDLE);

    unsafe impl Send for WifiLowLatency {}

    impl WifiLowLatency {
        pub fn start() -> Result<(WifiLowLatency, String), String> {
            use windows::Win32::NetworkManagement::WiFi::*;
            unsafe {
                let mut version = 0u32;
                let mut h = windows::Win32::Foundation::HANDLE::default();
                if WlanOpenHandle(2, None, &mut version, &mut h) != 0 {
                    return Err("This PC has no Wi-Fi.".into());
                }
                let me = WifiLowLatency(h);
                let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
                if WlanEnumInterfaces(h, None, &mut list) != 0 || list.is_null() {
                    return Err("This PC has no Wi-Fi.".into());
                }
                let infos = std::slice::from_raw_parts((*list).InterfaceInfo.as_ptr(), (*list).dwNumberOfItems as usize);
                let mut done = Vec::new();
                for i in infos.iter().filter(|i| i.isState == wlan_interface_state_connected) {
                    let off: u32 = 0;
                    let on: u32 = 1;
                    let a = WlanSetInterface(h, &i.InterfaceGuid, wlan_intf_opcode_background_scan_enabled, 4, (&off as *const u32).cast(), None);
                    let b = WlanSetInterface(h, &i.InterfaceGuid, wlan_intf_opcode_media_streaming_mode, 4, (&on as *const u32).cast(), None);
                    if a == 0 || b == 0 {
                        let len = i.strInterfaceDescription.iter().position(|&c| c == 0).unwrap_or(256);
                        done.push(String::from_utf16_lossy(&i.strInterfaceDescription[..len]));
                    }
                }
                WlanFreeMemory(list.cast());
                if done.is_empty() {
                    return Err("Not connected over Wi-Fi — nothing to do.".into());
                }
                Ok((me, done.join(", ")))
            }
        }
    }

    impl Drop for WifiLowLatency {
        fn drop(&mut self) {
            unsafe {
                windows::Win32::NetworkManagement::WiFi::WlanCloseHandle(self.0, None);
            }
        }
    }

    // ---------- timer resolution ----------

    type QueryTimer = unsafe extern "system" fn(*mut u32, *mut u32, *mut u32) -> i32;
    type SetTimer = unsafe extern "system" fn(u32, u8, *mut u32) -> i32;

    /// An ntdll function (not in the Windows SDK's import libraries).
    fn ntdll(name: &std::ffi::CStr) -> Option<unsafe extern "system" fn() -> isize> {
        use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
        unsafe {
            let h = GetModuleHandleW(windows::core::w!("ntdll.dll")).ok()?;
            GetProcAddress(h, windows::core::PCSTR(name.as_ptr().cast()))
        }
    }

    /// While held, Windows' timer ticks at its finest step (0.5 ms on most
    /// PCs) instead of 1–15.6 ms, so games that pace frames with short
    /// sleeps wake on time. Released when dropped, or by Windows if OmniHub
    /// exits.
    pub struct TimerResolution {
        set: SetTimer,
        value: u32,
    }

    impl TimerResolution {
        /// Returns the guard and the resolution in milliseconds.
        pub fn start() -> Result<(TimerResolution, f32), String> {
            let (Some(q), Some(s)) = (ntdll(c"NtQueryTimerResolution"), ntdll(c"NtSetTimerResolution")) else {
                return Err("This version of Windows has no timer setting.".into());
            };
            // Safety: the documented signatures of these two ntdll functions.
            let (query, set) = unsafe { (std::mem::transmute::<unsafe extern "system" fn() -> isize, QueryTimer>(q), std::mem::transmute::<unsafe extern "system" fn() -> isize, SetTimer>(s)) };
            let (mut coarsest, mut finest, mut now) = (0u32, 0u32, 0u32);
            if unsafe { query(&mut coarsest, &mut finest, &mut now) } < 0 || finest == 0 {
                return Err("Windows did not report its timer steps.".into());
            }
            let mut actual = 0u32;
            if unsafe { set(finest, 1, &mut actual) } < 0 {
                return Err("Windows did not change its timer.".into());
            }
            // Windows 11 stops honouring a hidden program's request otherwise.
            let _ = set_power_throttling(None, false);
            Ok((TimerResolution { set, value: finest }, actual.max(finest) as f32 / 10_000.0))
        }
    }

    impl Drop for TimerResolution {
        fn drop(&mut self) {
            let mut actual = 0u32;
            unsafe {
                (self.set)(self.value, 0, &mut actual);
            }
        }
    }

    const KERNEL: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\kernel";

    /// Whether a finer timer OmniHub asks for reaches games too. Since
    /// Windows 10 2004 each program gets only the timer it asked for, and
    /// Windows 11 brings back the shared one with GlobalTimerResolutionRequests.
    pub fn timer_reaches_games() -> Result<(), String> {
        let build = read_string(HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "CurrentBuildNumber").and_then(|b| b.parse::<u32>().ok()).unwrap_or(0);
        if build < 19041 || read_dword(HKEY_LOCAL_MACHINE, KERNEL, "GlobalTimerResolutionRequests") == Some(1) {
            return Ok(());
        }
        if build < 22000 {
            return Err("Windows 10 keeps each program's timer to itself, so only the game's own timer counts".into());
        }
        Err("turn on “Precise timer for games” in Optimize PC once (it takes effect after a restart)".into())
    }

    // ---------- power throttling ----------

    /// Opt a process (None: OmniHub itself) out of Windows' power
    /// throttling: no slower "efficiency" cores or clock for it (`speed`),
    /// and its timer requests count even with no visible window.
    pub fn set_power_throttling(pid: Option<u32>, speed: bool) -> Result<(), String> {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Threading::*;
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION | if speed { PROCESS_POWER_THROTTLING_EXECUTION_SPEED } else { 0 },
            // Controlled and not set: throttling off.
            StateMask: 0,
        };
        unsafe {
            let (h, owned) = match pid {
                Some(p) => (OpenProcess(PROCESS_SET_INFORMATION, false, p).map_err(|e| e.message().to_string())?, true),
                None => (GetCurrentProcess(), false),
            };
            let r = SetProcessInformation(h, ProcessPowerThrottling, (&state as *const PROCESS_POWER_THROTTLING_STATE).cast(), std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32).map_err(|e| e.message().to_string());
            if owned {
                let _ = CloseHandle(h);
            }
            r
        }
    }

    /// Whether the PC is online over Wi-Fi (for the "use a cable" tip).
    pub fn on_wifi() -> Option<bool> {
        let out = run("netsh", &["wlan", "show", "interfaces"]).ok()?;
        // "State : connected" — the word is translated, so look for an SSID line instead.
        Some(out.lines().any(|l| l.trim_start().starts_with("SSID") && !l.contains("BSSID") && l.split(':').nth(1).is_some_and(|v| !v.trim().is_empty())))
    }

    // ---------- launching ----------

    /// Riot Client's location, from its own install record.
    pub fn riot_client() -> Option<std::path::PathBuf> {
        let pd = std::env::var_os("ProgramData")?;
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(std::path::Path::new(&pd).join(r"Riot Games\RiotClientInstalls.json")).ok()?).ok()?;
        ["rc_default", "rc_live"].iter().filter_map(|k| v.get(k)?.as_str()).map(std::path::PathBuf::from).find(|p| p.is_file())
    }

    /// An Epic game's install folder, from the launcher's manifests.
    pub fn epic_install(app_name: &str) -> Option<std::path::PathBuf> {
        let pd = std::env::var_os("ProgramData")?;
        for e in std::fs::read_dir(std::path::Path::new(&pd).join(r"Epic\EpicGamesLauncher\Data\Manifests")).ok()?.flatten() {
            let Ok(bytes) = std::fs::read(e.path()) else { continue };
            let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else { continue };
            if v.get("AppName").and_then(|a| a.as_str()) == Some(app_name) {
                return v.get("InstallLocation").and_then(|l| l.as_str()).map(std::path::PathBuf::from);
            }
        }
        None
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    const ONLY: &str = "Windows only";

    pub fn active_plan() -> Option<String> {
        None
    }
    pub fn set_plan(_guid: &str) -> Result<(), String> {
        Err(ONLY.into())
    }
    pub fn activate_plan(_choice: PowerPlan, _remembered: Option<&str>) -> Result<(String, &'static str, Option<String>), String> {
        Err(ONLY.into())
    }
    pub fn silence_notifications() -> Result<Option<u32>, String> {
        Err(ONLY.into())
    }
    pub fn restore_notifications(_before: Option<u32>) -> Result<(), String> {
        Err(ONLY.into())
    }
    pub fn ensure_game_mode() -> Result<bool, String> {
        Err(ONLY.into())
    }
    pub fn set_gpu_high_performance(_exe_path: &str, _on: bool) -> Result<bool, String> {
        Err(ONLY.into())
    }
    pub fn set_fullscreen_optimizations_off(_exe_path: &str, _on: bool) -> Result<bool, String> {
        Err(ONLY.into())
    }
    pub fn qos_enabled(_exe: &str) -> bool {
        false
    }
    pub fn start_high_enabled(_exe: &str) -> bool {
        false
    }
    pub fn run_admin(ops: &[AdminOp]) -> Result<(), String> {
        if ops.is_empty() {
            Ok(())
        } else {
            Err(ONLY.into())
        }
    }
    pub fn helper_admin(_args: &[String]) -> i32 {
        64
    }
    pub struct WifiLowLatency;
    impl WifiLowLatency {
        pub fn start() -> Result<(WifiLowLatency, String), String> {
            Err(ONLY.into())
        }
    }
    pub fn on_wifi() -> Option<bool> {
        None
    }
    pub struct TimerResolution;
    impl TimerResolution {
        pub fn start() -> Result<(TimerResolution, f32), String> {
            Err(ONLY.into())
        }
    }
    pub fn timer_reaches_games() -> Result<(), String> {
        Err(ONLY.into())
    }
    pub fn set_power_throttling(_pid: Option<u32>, _speed: bool) -> Result<(), String> {
        Err(ONLY.into())
    }
    pub fn riot_client() -> Option<std::path::PathBuf> {
        None
    }
    pub fn epic_install(_app_name: &str) -> Option<std::path::PathBuf> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powercfg_guids() {
        let out = "Existing Power Schemes (* Active)\n-----------------------------------\nPower Scheme GUID: 381b4222-f694-41f0-9685-ff5bb260df2e  (Balanced) *\nPower Scheme GUID: 8C5E7FDA-E8BF-4A96-9A85-A6E23A8C635C  (Hochleistung)\n";
        assert_eq!(guids(out), vec!["381b4222-f694-41f0-9685-ff5bb260df2e".to_string(), HIGH_PERFORMANCE.to_string()]);
        assert!(guids("no guid here 1234").is_empty());
    }

    #[test]
    fn exe_names_and_admin_ops() {
        for ok in ["FortniteClient-Win64-Shipping.exe", "League of Legends.exe", "cs2.exe", "RobloxPlayerBeta.exe", "GTA5.exe", "game(1).exe"] {
            assert!(valid_exe_name(ok), "{ok}");
        }
        for bad in ["", ".exe", "game", "a'b.exe", "x\".exe", "..\\evil.exe", "a;b.exe", "c:\\x.exe", " lead.exe"] {
            assert!(!valid_exe_name(bad), "{bad}");
        }
        let op = AdminOp::QosOn("cs2.exe".into());
        assert_eq!(AdminOp::parse(&op.arg()), Some(op));
        assert_eq!(AdminOp::parse("ifeo-:GTA5.exe"), Some(AdminOp::StartHighOff("GTA5.exe".into())));
        assert_eq!(AdminOp::parse("qos+:evil'.exe"), None);
        assert_eq!(AdminOp::parse("rm:cs2.exe"), None);
        assert_eq!(qos_policy_name("cs2.exe"), "OmniHub - cs2.exe");
    }
}
