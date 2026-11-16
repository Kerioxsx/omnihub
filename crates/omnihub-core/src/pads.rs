//! PlayStation controllers' light bar (DualShock 4, DualSense, DualSense
//! Edge), set with the controller's own output report, the format the Linux
//! kernel's hid-playstation driver documents. Everything else about
//! controllers (buttons, sticks, vibration) the pages read and drive
//! through the browser's gamepad support.
//!
//! Over Bluetooth this report also switches the controller to the full
//! mode Steam uses; programs that read it the plain way (some older games)
//! then don't see it until it's turned off and on, so Bluetooth is opt-in.

use serde::Serialize;

pub const SONY: u16 = 0x054C;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PsKind {
    DualShock4,
    DualSense,
    DualSenseEdge,
}

pub fn ps_kind(vendor: u16, product: u16) -> Option<PsKind> {
    if vendor != SONY {
        return None;
    }
    match product {
        0x05C4 | 0x09CC | 0x0BA0 => Some(PsKind::DualShock4),
        0x0CE6 => Some(PsKind::DualSense),
        0x0DF2 => Some(PsKind::DualSenseEdge),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LightResult {
    /// Controllers whose light was set.
    pub set: usize,
    /// PlayStation controllers on Bluetooth left alone (not allowed).
    pub skipped_bluetooth: usize,
    /// Controllers that didn't take the report.
    pub failed: usize,
}

/// CRC-32 (IEEE) of `seed` followed by `data`, as the controllers check it.
pub fn crc32(seed: u8, data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in std::iter::once(&seed).chain(data) {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

fn seal(buf: &mut [u8]) {
    let n = buf.len() - 4;
    let c = crc32(0xA2, &buf[..n]);
    buf[n..].copy_from_slice(&c.to_le_bytes());
}

/// DualShock 4: light bar colour and the two motors (strong, weak).
pub fn ds4_report(bluetooth: bool, rgb: [u8; 3], rumble: (u8, u8)) -> Vec<u8> {
    if bluetooth {
        let mut b = vec![0u8; 78];
        b[0] = 0x11;
        b[1] = 0xC0; // HID data + CRC follow
        b[3] = 0x03; // motors + light
        b[6] = rumble.1;
        b[7] = rumble.0;
        b[8..11].copy_from_slice(&rgb);
        seal(&mut b);
        b
    } else {
        let mut b = vec![0u8; 32];
        b[0] = 0x05;
        b[1] = 0x03;
        b[4] = rumble.1;
        b[5] = rumble.0;
        b[6..9].copy_from_slice(&rgb);
        b
    }
}

/// DualSense: light bar colour, the first player light, and (once) the
/// light bar's own start-up animation faded out so the colour shows.
pub fn dualsense_report(bluetooth: bool, seq: u8, rgb: [u8; 3]) -> Vec<u8> {
    // The shared part starts after the report id (and, on Bluetooth, the
    // sequence and tag bytes).
    let (mut b, o) = if bluetooth { (vec![0u8; 78], 3) } else { (vec![0u8; 63], 1) };
    b[0] = if bluetooth { 0x31 } else { 0x02 };
    if bluetooth {
        b[1] = (seq & 0x0F) << 4;
        b[2] = 0x10;
    }
    b[o + 1] = 0x04 | 0x10; // light bar + player lights
    b[o + 38] = 0x02; // light bar setup
    b[o + 41] = 0x02; // fade the start-up light out
    b[o + 43] = 0x04; // player 1 (middle light)
    b[o + 44..o + 47].copy_from_slice(&rgb);
    if bluetooth {
        seal(&mut b);
    }
    b
}

/// "#ff0080" → [255, 0, 128].
pub fn parse_color(hex: &str) -> Option<[u8; 3]> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

/// Set the light bar of every PlayStation controller connected.
pub fn set_light(rgb: [u8; 3], bluetooth: bool) -> LightResult {
    imp::set_light(rgb, bluetooth)
}

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicU8, Ordering};

    use windows::core::PCWSTR;
    use windows::Win32::Devices::DeviceAndDriverInstallation::{SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW, DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA, SP_DEVICE_INTERFACE_DETAIL_DATA_W};
    use windows::Win32::Devices::HumanInterfaceDevice::{HidD_FreePreparsedData, HidD_GetAttributes, HidD_GetHidGuid, HidD_GetPreparsedData, HidD_SetOutputReport, HidP_GetCaps, HIDD_ATTRIBUTES, HIDP_CAPS, PHIDP_PREPARSED_DATA};
    use windows::Win32::Foundation::{CloseHandle, GENERIC_READ, GENERIC_WRITE, HANDLE};
    use windows::Win32::Storage::FileSystem::{CreateFileW, WriteFile, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};

    use super::{dualsense_report, ds4_report, ps_kind, LightResult, PsKind};

    static SEQ: AtomicU8 = AtomicU8::new(0);
    /// Bluetooth HID devices' paths carry the HID service's id.
    const BT_HID: &str = "00001124-0000-1000-8000-00805f9b34fb";

    /// Every HID device interface path present.
    fn hid_paths() -> Vec<String> {
        let mut out = Vec::new();
        unsafe {
            let guid = HidD_GetHidGuid();
            let Ok(set) = SetupDiGetClassDevsW(Some(&guid), PCWSTR::null(), None, DIGCF_PRESENT | DIGCF_DEVICEINTERFACE) else { return out };
            let mut i = 0;
            loop {
                let mut data = SP_DEVICE_INTERFACE_DATA { cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32, ..Default::default() };
                if SetupDiEnumDeviceInterfaces(set, None, &guid, i, &mut data).is_err() {
                    break;
                }
                i += 1;
                let mut need = 0u32;
                let _ = SetupDiGetDeviceInterfaceDetailW(set, &data, None, 0, Some(&mut need), None);
                if need < 8 {
                    continue;
                }
                // A u64 buffer keeps the detail structure aligned.
                let mut buf = vec![0u64; (need as usize).div_ceil(8)];
                let detail = buf.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
                (*detail).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
                if SetupDiGetDeviceInterfaceDetailW(set, &data, Some(detail), need, None, None).is_err() {
                    continue;
                }
                let start = std::ptr::addr_of!((*detail).DevicePath) as *const u16;
                let max = (need as usize - 4) / 2;
                let wide = std::slice::from_raw_parts(start, max);
                let len = wide.iter().position(|&c| c == 0).unwrap_or(max);
                out.push(String::from_utf16_lossy(&wide[..len]));
            }
            let _ = SetupDiDestroyDeviceInfoList(set);
        }
        out
    }

    fn open(path: &str, access: u32) -> Option<HANDLE> {
        let wide: Vec<u16> = path.encode_utf16().chain([0]).collect();
        unsafe { CreateFileW(PCWSTR(wide.as_ptr()), access, FILE_SHARE_READ | FILE_SHARE_WRITE, None, OPEN_EXISTING, FILE_FLAGS_AND_ATTRIBUTES(0), None).ok() }
    }

    fn output_len(h: HANDLE) -> usize {
        unsafe {
            let mut pre = PHIDP_PREPARSED_DATA::default();
            if !HidD_GetPreparsedData(h, &mut pre) {
                return 0;
            }
            let mut caps = HIDP_CAPS::default();
            let _ = HidP_GetCaps(pre, &mut caps);
            let _ = HidD_FreePreparsedData(pre);
            caps.OutputReportByteLength as usize
        }
    }

    /// Send a report: the exact size through the control channel, else
    /// padded to what the device declares.
    fn send(h: HANDLE, report: &[u8]) -> bool {
        unsafe {
            if HidD_SetOutputReport(h, report.as_ptr() as *const _, report.len() as u32) {
                return true;
            }
            let len = output_len(h);
            if len == 0 {
                return false;
            }
            let mut buf = report.to_vec();
            buf.resize(len, 0);
            let mut wrote = 0u32;
            WriteFile(h, Some(&buf), Some(&mut wrote), None).is_ok()
        }
    }

    pub fn set_light(rgb: [u8; 3], bluetooth: bool) -> LightResult {
        let mut r = LightResult::default();
        let mut seen = std::collections::HashSet::new();
        let mut failed = std::collections::HashSet::new();
        for path in hid_paths() {
            let lower = path.to_lowercase();
            // One controller shows several HID interfaces on some systems; the
            // first one that takes the report is enough.
            let Some(h) = open(&path, 0) else { continue };
            let mut attr = HIDD_ATTRIBUTES { Size: std::mem::size_of::<HIDD_ATTRIBUTES>() as u32, ..Default::default() };
            let ok = unsafe { HidD_GetAttributes(h, &mut attr) };
            unsafe {
                let _ = CloseHandle(h);
            }
            if !ok {
                continue;
            }
            let Some(kind) = ps_kind(attr.VendorID, attr.ProductID) else { continue };
            let bt = lower.contains(BT_HID);
            let key = lower.split('#').take(3).collect::<Vec<_>>().join("#");
            if seen.contains(&key) {
                continue;
            }
            if bt && !bluetooth {
                r.skipped_bluetooth += 1;
                seen.insert(key);
                continue;
            }
            let Some(h) = open(&path, GENERIC_READ.0 | GENERIC_WRITE.0) else { continue };
            let report = match kind {
                PsKind::DualShock4 => ds4_report(bt, rgb, (0, 0)),
                PsKind::DualSense | PsKind::DualSenseEdge => dualsense_report(bt, SEQ.fetch_add(1, Ordering::Relaxed), rgb),
            };
            if send(h, &report) {
                r.set += 1;
                seen.insert(key);
            } else {
                failed.insert(key);
            }
            unsafe {
                let _ = CloseHandle(h);
            }
        }
        r.failed = failed.difference(&seen).count();
        r
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn set_light(_rgb: [u8; 3], _bluetooth: bool) -> super::LightResult {
        super::LightResult::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_the_controllers() {
        assert_eq!(crc32(0xA2, b"hello"), 0x1524_40dd);
    }

    #[test]
    fn builds_light_reports() {
        let usb = ds4_report(false, [255, 0, 128], (10, 20));
        assert_eq!((usb.len(), usb[0], usb[1]), (32, 0x05, 0x03));
        assert_eq!(&usb[4..9], &[20, 10, 255, 0, 128]);
        let bt = ds4_report(true, [255, 0, 128], (0, 0));
        assert_eq!((bt.len(), bt[0], bt[1], bt[3]), (78, 0x11, 0xC0, 0x03));
        assert_eq!(&bt[74..], &[0x25, 0xae, 0x77, 0xe0]);

        let usb = dualsense_report(false, 0, [1, 2, 3]);
        assert_eq!((usb.len(), usb[0], usb[2]), (63, 0x02, 0x14));
        assert_eq!(&usb[45..48], &[1, 2, 3]);
        assert_eq!((usb[39], usb[42], usb[44]), (0x02, 0x02, 0x04));
        let bt = dualsense_report(true, 3, [10, 20, 30]);
        assert_eq!((bt[0], bt[1], bt[2], bt[4]), (0x31, 0x30, 0x10, 0x14));
        assert_eq!(&bt[47..50], &[10, 20, 30]);
        assert_eq!(&bt[74..], &[0x87, 0x88, 0xe5, 0x1c]);
    }

    #[test]
    fn knows_playstation_controllers() {
        assert_eq!(ps_kind(0x054C, 0x0CE6), Some(PsKind::DualSense));
        assert_eq!(ps_kind(0x054C, 0x09CC), Some(PsKind::DualShock4));
        assert_eq!(ps_kind(0x054C, 0x0DF2), Some(PsKind::DualSenseEdge));
        assert_eq!(ps_kind(0x045E, 0x0B13), None);
        assert_eq!(parse_color("#FF0080"), Some([255, 0, 128]));
        assert_eq!(parse_color("red"), None);
    }
}
