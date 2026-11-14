//! Monitor brightness and contrast. External monitors are set over DDC/CI,
//! the monitor's own setting, as with its buttons (what Twinkle Tray uses,
//! so both can run side by side); a laptop's built-in screen through WMI.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Display {
    /// Windows' name for the screen plus the monitor's place on it
    /// (`\\.\DISPLAY1#0`); stays the same while the monitors do.
    pub id: String,
    pub name: String,
    pub primary: bool,
    /// A laptop's own screen (set through Windows, not DDC/CI).
    pub builtin: bool,
    /// 0–100; None when the monitor doesn't answer (DDC/CI off in its menu).
    pub brightness: Option<u8>,
    pub contrast: Option<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Setting {
    Brightness,
    Contrast,
}

/// Monitor buses are slow (about 50 ms a message) and talk one at a time.
static BUS: parking_lot::Mutex<()> = parking_lot::Mutex::new(());

pub fn list() -> Vec<Display> {
    let _bus = BUS.lock();
    imp::list()
}

pub fn set(id: &str, what: Setting, value: u8) -> Result<(), String> {
    let _bus = BUS.lock();
    imp::set(id, what, value.min(100))
}

/// Two monitors that remember what they are set to, for demos and tests.
static FAKE: parking_lot::Mutex<Vec<Display>> = parking_lot::Mutex::new(Vec::new());

pub fn fake_list() -> Vec<Display> {
    let mut f = FAKE.lock();
    if f.is_empty() {
        *f = vec![
            Display { id: r"\\.\DISPLAY1#0".into(), name: "LG ULTRAGEAR".into(), primary: true, builtin: false, brightness: Some(70), contrast: Some(75) },
            Display { id: r"\\.\DISPLAY2#0".into(), name: "DELL S2421H".into(), primary: false, builtin: false, brightness: Some(55), contrast: Some(50) },
        ];
    }
    f.clone()
}

pub fn fake_set(id: &str, what: Setting, value: u8) -> Result<(), String> {
    fake_list();
    let mut f = FAKE.lock();
    let d = f.iter_mut().find(|d| d.id == id).ok_or("That monitor is no longer connected.")?;
    *(if what == Setting::Brightness { &mut d.brightness } else { &mut d.contrast }) = Some(value.min(100));
    Ok(())
}

/// A value read as `current` of `max` in percent, and back.
#[cfg_attr(not(windows), allow(dead_code))]
fn percent(current: u32, max: u32) -> u8 {
    ((current.min(max) * 100 + max / 2) / max.max(1)) as u8
}

#[cfg_attr(not(windows), allow(dead_code))]
fn raw(percent: u8, max: u32) -> u32 {
    (percent as u32 * max + 50) / 100
}

#[cfg(windows)]
mod imp {
    use std::collections::HashMap;
    use std::os::windows::process::CommandExt;

    use windows::core::BOOL;
    use windows::Win32::Devices::Display::*;
    use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE, LPARAM, RECT};
    use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW};
    use windows::Win32::UI::WindowsAndMessaging::MONITORINFOF_PRIMARY;

    use super::*;

    const BRIGHTNESS: u8 = 0x10;
    const CONTRAST: u8 = 0x12;

    fn wstr(w: &[u16]) -> String {
        String::from_utf16_lossy(&w[..w.iter().position(|&c| c == 0).unwrap_or(w.len())]).trim().to_string()
    }

    struct Screen {
        gdi: String,
        primary: bool,
        handle: HMONITOR,
    }

    fn screens() -> Vec<Screen> {
        unsafe extern "system" fn each(h: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
            // SAFETY: `data` is the Vec passed below, alive for the call.
            unsafe { (*(data.0 as *mut Vec<HMONITOR>)).push(h) };
            true.into()
        }
        let mut handles: Vec<HMONITOR> = Vec::new();
        // SAFETY: the callback only pushes into `handles`.
        unsafe {
            let _ = EnumDisplayMonitors(None, None, Some(each), LPARAM(&mut handles as *mut _ as isize));
        }
        handles
            .into_iter()
            .filter_map(|h| {
                let mut info = MONITORINFOEXW::default();
                info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
                // SAFETY: a sized MONITORINFOEXW, as GetMonitorInfoW allows.
                unsafe { GetMonitorInfoW(h, &mut info.monitorInfo) }.as_bool().then(|| Screen { gdi: wstr(&info.szDevice), primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0, handle: h })
            })
            .collect()
    }

    /// Windows' screen name (`\\.\DISPLAY1`) → the monitor's own name and
    /// whether it is built in.
    fn names() -> HashMap<String, (String, bool)> {
        let mut out = HashMap::new();
        // SAFETY: buffers sized by GetDisplayConfigBufferSizes; packets sized by their headers.
        unsafe {
            let (mut np, mut nm) = (0u32, 0u32);
            if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut np, &mut nm) != ERROR_SUCCESS {
                return out;
            }
            let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); np as usize];
            let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); nm as usize];
            if QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, &mut np, paths.as_mut_ptr(), &mut nm, modes.as_mut_ptr(), None) != ERROR_SUCCESS {
                return out;
            }
            for p in paths.iter().take(np as usize) {
                let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME { header: DISPLAYCONFIG_DEVICE_INFO_HEADER { r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, size: std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32, adapterId: p.sourceInfo.adapterId, id: p.sourceInfo.id }, ..Default::default() };
                let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME { header: DISPLAYCONFIG_DEVICE_INFO_HEADER { r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, size: std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32, adapterId: p.targetInfo.adapterId, id: p.targetInfo.id }, ..Default::default() };
                if DisplayConfigGetDeviceInfo(&mut source.header) != 0 || DisplayConfigGetDeviceInfo(&mut target.header) != 0 {
                    continue;
                }
                let builtin = matches!(target.outputTechnology, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL | DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED | DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED);
                out.entry(wstr(&source.viewGdiDeviceName)).or_insert((wstr(&target.monitorFriendlyDeviceName), builtin));
            }
        }
        out
    }

    /// The monitors behind one screen, closed again when dropped.
    struct Physical(Vec<PHYSICAL_MONITOR>);

    impl Physical {
        fn of(h: HMONITOR) -> Physical {
            let mut n = 0u32;
            // SAFETY: the array is sized to what Windows reported.
            unsafe {
                if GetNumberOfPhysicalMonitorsFromHMONITOR(h, &mut n).is_err() || n == 0 {
                    return Physical(Vec::new());
                }
                let mut v = vec![PHYSICAL_MONITOR::default(); n as usize];
                if GetPhysicalMonitorsFromHMONITOR(h, &mut v).is_err() {
                    return Physical(Vec::new());
                }
                Physical(v)
            }
        }
    }

    impl Drop for Physical {
        fn drop(&mut self) {
            if !self.0.is_empty() {
                // SAFETY: handles from GetPhysicalMonitorsFromHMONITOR, closed once.
                let _ = unsafe { DestroyPhysicalMonitors(&self.0) };
            }
        }
    }

    /// (current, max) of a monitor setting; monitors miss a message now and then.
    fn read(h: HANDLE, code: u8) -> Option<(u32, u32)> {
        for _ in 0..2 {
            let (mut cur, mut max) = (0u32, 0u32);
            // SAFETY: plain out-values.
            if unsafe { GetVCPFeatureAndVCPFeatureReply(h, code, None, &mut cur, Some(&mut max)) } != 0 && max > 0 {
                return Some((cur, max));
            }
        }
        None
    }

    fn powershell(script: &str) -> Option<String> {
        let out = std::process::Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", script]).creation_flags(0x0800_0000).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn builtin_brightness() -> Option<u8> {
        powershell("(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness -ErrorAction SilentlyContinue | Select-Object -First 1).CurrentBrightness")?.parse().ok()
    }

    pub fn list() -> Vec<Display> {
        let names = names();
        let mut out = Vec::new();
        for (n, s) in screens().into_iter().enumerate() {
            let (friendly, builtin) = names.get(&s.gdi).cloned().unwrap_or_default();
            if builtin {
                out.push(Display { id: format!("{}#0", s.gdi), name: if friendly.is_empty() { "Built-in screen".into() } else { friendly }, primary: s.primary, builtin: true, brightness: builtin_brightness(), contrast: None });
                continue;
            }
            let physical = Physical::of(s.handle);
            for (i, m) in physical.0.iter().enumerate() {
                // PHYSICAL_MONITOR is packed: copy the field out before borrowing it.
                let description = m.szPhysicalMonitorDescription;
                let described = wstr(&description);
                let name = [friendly.clone(), described].into_iter().find(|x| !x.is_empty() && !x.starts_with("Generic")).unwrap_or_else(|| format!("Display {}", n + 1));
                out.push(Display {
                    id: format!("{}#{i}", s.gdi),
                    name,
                    primary: s.primary,
                    builtin: false,
                    brightness: read(m.hPhysicalMonitor, BRIGHTNESS).map(|(c, x)| percent(c, x)),
                    contrast: read(m.hPhysicalMonitor, CONTRAST).map(|(c, x)| percent(c, x)),
                });
            }
        }
        out
    }

    pub fn set(id: &str, what: Setting, value: u8) -> Result<(), String> {
        let (gdi, index) = id.rsplit_once('#').and_then(|(g, i)| Some((g, i.parse::<usize>().ok()?))).ok_or("Unknown monitor.")?;
        let screen = screens().into_iter().find(|s| s.gdi == gdi).ok_or("That monitor is no longer connected.")?;
        if names().get(gdi).is_some_and(|n| n.1) {
            if what == Setting::Contrast {
                return Err("A built-in screen has no contrast setting.".into());
            }
            return powershell(&format!("Invoke-CimMethod -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods -MethodName WmiSetBrightness -Arguments @{{Timeout=0;Brightness={value}}} | Out-Null")).map(|_| ()).ok_or_else(|| "Windows didn't change the screen's brightness.".into());
        }
        let physical = Physical::of(screen.handle);
        let m = physical.0.get(index).ok_or("That monitor is no longer connected.")?;
        let code = if what == Setting::Brightness { BRIGHTNESS } else { CONTRAST };
        let max = read(m.hPhysicalMonitor, code).map(|(_, x)| x).ok_or("The monitor didn't answer: turn on DDC/CI in its own menu (the buttons on the monitor).")?;
        for _ in 0..2 {
            // SAFETY: a live physical-monitor handle.
            if unsafe { SetVCPFeature(m.hPhysicalMonitor, code, raw(value, max)) } != 0 {
                return Ok(());
            }
        }
        Err("The monitor didn't take the change; try again.".into())
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn list() -> Vec<Display> {
        Vec::new()
    }

    pub fn set(_id: &str, _what: Setting, _value: u8) -> Result<(), String> {
        Err("Monitor brightness works on Windows.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_monitors_remember() {
        let id = fake_list()[1].id.clone();
        fake_set(&id, Setting::Brightness, 120).unwrap();
        assert_eq!(fake_list()[1].brightness, Some(100));
        assert!(fake_set("nope#0", Setting::Contrast, 5).is_err());
    }

    #[test]
    fn percent_and_back() {
        assert_eq!(percent(50, 100), 50);
        assert_eq!(percent(127, 255), 50);
        assert_eq!(percent(300, 100), 100);
        assert_eq!(raw(50, 255), 128);
        assert_eq!(raw(100, 100), 100);
        for max in [100, 255, 80] {
            for p in [0u8, 1, 33, 50, 99, 100] {
                assert!((percent(raw(p, max), max) as i32 - p as i32).abs() <= 1, "{p} of {max}");
            }
        }
    }
}
