//! Bluetooth devices paired with this PC: see them, connect or disconnect
//! headphones and speakers, and forget (unpair) any of them. Connecting uses
//! the same one-shot "reconnect / disconnect" request Windows' own sound
//! settings send to a Bluetooth audio device; other kinds of device connect
//! by themselves when they're turned on. New devices are paired in Windows.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BtDevice {
    /// "B8:F6:53:CA:1B:2E"
    pub address: String,
    pub name: String,
    pub connected: bool,
    /// "audio", "keyboard", "mouse", "gamepad", "phone", "computer", "other".
    pub kind: String,
    /// Headphones or speakers: can be connected and disconnected from here.
    pub audio: bool,
}

/// The kind of device from its Bluetooth class of device.
pub fn kind_of(cod: u32) -> &'static str {
    let major = (cod >> 8) & 0x1F;
    let minor = (cod >> 2) & 0x3F;
    match major {
        1 => "computer",
        2 => "phone",
        4 => "audio",
        5 if minor & 0x10 != 0 && minor & 0x20 != 0 => "keyboard",
        5 if minor & 0x20 != 0 => "mouse",
        5 if minor & 0x10 != 0 => "keyboard",
        5 if matches!(minor & 0x0F, 1 | 2) => "gamepad",
        _ => "other",
    }
}

/// The address as Windows writes it (bytes reversed), with colons.
pub fn format_address(bytes: [u8; 6]) -> String {
    bytes.iter().rev().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
}

/// "B8:F6:53:CA:1B:2E" back to the address as a number.
pub fn parse_address(s: &str) -> Option<u64> {
    let hex: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    (hex.len() == 12).then(|| u64::from_str_radix(&hex, 16).ok()).flatten()
}

pub fn list() -> Result<Vec<BtDevice>, String> {
    imp::list()
}

/// Unpair a device.
pub fn forget(address: &str) -> Result<(), String> {
    imp::forget(parse_address(address).ok_or("That isn't a Bluetooth address")?)
}

/// Connect (`on`) or disconnect headphones or speakers.
pub fn connect(address: &str, name: &str, on: bool) -> Result<(), String> {
    let addr = parse_address(address).ok_or("That isn't a Bluetooth address")?;
    let n = imp::connect(addr, name, on)?;
    if n == 0 {
        return Err("Windows has no sound device for it right now. Turn it on and try again, or pair it again in Windows.".into());
    }
    Ok(())
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    use windows::core::{Interface, PWSTR};
    use windows::Win32::Devices::Bluetooth::{BluetoothFindDeviceClose, BluetoothFindFirstDevice, BluetoothFindNextDevice, BluetoothRemoveDevice, BLUETOOTH_ADDRESS, BLUETOOTH_ADDRESS_0, BLUETOOTH_DEVICE_INFO, BLUETOOTH_DEVICE_SEARCH_PARAMS};
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Media::Audio::{eAll, IDeviceTopology, IMMDeviceEnumerator, IPart, MMDeviceEnumerator, DEVICE_STATE, DEVICE_STATE_ACTIVE, DEVICE_STATE_UNPLUGGED};
    use windows::Win32::Media::KernelStreaming::{IKsControl, KSIDENTIFIER, KSIDENTIFIER_0, KSIDENTIFIER_0_0, KSPROPERTY_TYPE_GET, KSPROPSETID_BtAudio};
    use windows::Win32::System::Com::StructuredStorage::PropVariantToStringAlloc;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, STGM_READ};

    use super::{format_address, kind_of, BtDevice};

    fn com<T: Send>(f: impl FnOnce() -> Result<T, String> + Send) -> Result<T, String> {
        std::thread::scope(|s| {
            s.spawn(|| unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                f()
            })
            .join()
        })
        .unwrap_or_else(|_| Err("Windows didn't answer".into()))
    }

    unsafe fn take(p: PWSTR) -> String {
        let s = p.to_string().unwrap_or_default();
        CoTaskMemFree(Some(p.0 as *const _));
        s
    }

    pub fn list() -> Result<Vec<BtDevice>, String> {
        let mut out = Vec::new();
        unsafe {
            let params = BLUETOOTH_DEVICE_SEARCH_PARAMS {
                dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
                fReturnAuthenticated: true.into(),
                fReturnRemembered: true.into(),
                fReturnUnknown: false.into(),
                fReturnConnected: true.into(),
                fIssueInquiry: false.into(),
                cTimeoutMultiplier: 0,
                hRadio: HANDLE::default(),
            };
            let mut info = BLUETOOTH_DEVICE_INFO { dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32, ..Default::default() };
            let Ok(find) = BluetoothFindFirstDevice(&params, &mut info) else { return Ok(out) };
            loop {
                let name_len = info.szName.iter().position(|&c| c == 0).unwrap_or(info.szName.len());
                let kind = kind_of(info.ulClassofDevice);
                out.push(BtDevice {
                    address: format_address(info.Address.Anonymous.rgBytes),
                    name: String::from_utf16_lossy(&info.szName[..name_len]),
                    connected: info.fConnected.as_bool(),
                    kind: kind.into(),
                    audio: kind == "audio",
                });
                info = BLUETOOTH_DEVICE_INFO { dwSize: std::mem::size_of::<BLUETOOTH_DEVICE_INFO>() as u32, ..Default::default() };
                if BluetoothFindNextDevice(find, &mut info).is_err() {
                    break;
                }
            }
            let _ = BluetoothFindDeviceClose(find);
        }
        out.sort_by(|a, b| b.connected.cmp(&a.connected).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        Ok(out)
    }

    pub fn forget(addr: u64) -> Result<(), String> {
        let a = BLUETOOTH_ADDRESS { Anonymous: BLUETOOTH_ADDRESS_0 { ullLong: addr } };
        match unsafe { BluetoothRemoveDevice(&a) } {
            0 => Ok(()),
            e => Err(format!("Windows couldn't forget it (error {e})")),
        }
    }

    /// Send the one-shot request to each Windows sound device that belongs
    /// to this Bluetooth device. Returns how many took it.
    pub fn connect(addr: u64, name: &str, on: bool) -> Result<usize, String> {
        let hex = format!("{addr:012x}");
        let name = name.to_lowercase();
        com(move || unsafe {
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            let col = en.EnumAudioEndpoints(eAll, DEVICE_STATE(DEVICE_STATE_ACTIVE.0 | DEVICE_STATE_UNPLUGGED.0)).map_err(|e| e.to_string())?;
            let mut done = 0;
            let mut seen = std::collections::HashSet::new();
            for i in 0..col.GetCount().map_err(|e| e.to_string())? {
                let Ok(dev) = col.Item(i) else { continue };
                let Ok(topo) = dev.Activate::<IDeviceTopology>(CLSCTX_ALL, None) else { continue };
                let Ok(conn) = topo.GetConnector(0) else { continue };
                let Ok(filter) = conn.GetDeviceIdConnectedTo().map(|p| take(p).to_lowercase()) else { continue };
                if !filter.contains("bth") {
                    continue;
                }
                let friendly = dev
                    .OpenPropertyStore(STGM_READ)
                    .ok()
                    .and_then(|st| st.GetValue(&PKEY_Device_FriendlyName).ok())
                    .and_then(|v| PropVariantToStringAlloc(&v).ok())
                    .map(|p| take(p).to_lowercase())
                    .unwrap_or_default();
                if !(filter.contains(&hex) || (!name.is_empty() && friendly.contains(&name))) || !seen.insert(filter.clone()) {
                    continue;
                }
                let Ok(other) = conn.GetConnectedTo() else { continue };
                let Ok(part) = other.cast::<IPart>() else { continue };
                let mut raw: *mut c_void = std::ptr::null_mut();
                if part.Activate(CLSCTX_INPROC_SERVER.0, &IKsControl::IID, Some(&mut raw)).is_err() || raw.is_null() {
                    continue;
                }
                let ks = IKsControl::from_raw(raw);
                let prop = KSIDENTIFIER { Anonymous: KSIDENTIFIER_0 { Anonymous: KSIDENTIFIER_0_0 { Set: KSPROPSETID_BtAudio, Id: if on { 0 } else { 1 }, Flags: KSPROPERTY_TYPE_GET } } };
                let mut got = 0u32;
                if ks.KsProperty(&prop, std::mem::size_of::<KSIDENTIFIER>() as u32, std::ptr::null_mut(), 0, &mut got).is_ok() {
                    done += 1;
                }
            }
            Ok(done)
        })
    }
}

#[cfg(not(windows))]
mod imp {
    use super::BtDevice;
    pub fn list() -> Result<Vec<BtDevice>, String> {
        Ok(Vec::new())
    }
    pub fn forget(_addr: u64) -> Result<(), String> {
        Err("Bluetooth control works on Windows".into())
    }
    pub fn connect(_addr: u64, _name: &str, _on: bool) -> Result<usize, String> {
        Err("Bluetooth control works on Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_addresses_and_kinds() {
        assert_eq!(format_address([0x2E, 0x1B, 0xCA, 0x53, 0xF6, 0xB8]), "B8:F6:53:CA:1B:2E");
        assert_eq!(parse_address("B8:F6:53:CA:1B:2E"), Some(0xB8F6_53CA_1B2E));
        assert_eq!(parse_address("nope"), None);
        assert_eq!(kind_of(0x240404), "audio"); // headphones
        assert_eq!(kind_of(0x002580), "mouse");
        assert_eq!(kind_of(0x002540), "keyboard");
        assert_eq!(kind_of(0x002508), "gamepad");
        assert_eq!(kind_of(0x5a020c), "phone");
        assert_eq!(kind_of(0x000000), "other");
    }
}
