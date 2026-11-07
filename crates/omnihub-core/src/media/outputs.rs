//! Where the sound comes out: the PC's playback devices (speakers,
//! headphones, a TV, Bluetooth headsets) and which one is the default.
//! Windows has no public call to change the default device; this uses the
//! same interface Windows' own sound settings use (IPolicyConfig), set for
//! every role (games and media, and calls).

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub id: String,
    pub name: String,
    pub default: bool,
}

pub fn list() -> Result<Vec<Output>, String> {
    imp::list()
}

pub fn set_default(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 512 {
        return Err("Pick a device".into());
    }
    imp::set_default(id)
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    use windows::core::{Interface, GUID, HRESULT, IUnknown, PCWSTR, PWSTR};
    use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
    use windows::Win32::Media::Audio::{eMultimedia, eRender, IMMDeviceEnumerator, MMDeviceEnumerator, DEVICE_STATE_ACTIVE};
    use windows::Win32::System::Com::StructuredStorage::PropVariantToStringAlloc;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ};

    use super::Output;

    const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);
    const IID_POLICY_CONFIG: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f290290c8);

    fn com<T: Send>(f: impl FnOnce() -> Result<T, String> + Send) -> Result<T, String> {
        std::thread::scope(|s| {
            s.spawn(|| unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                f()
            })
            .join()
        })
        .unwrap_or_else(|_| Err("The sound devices didn't answer".into()))
    }

    unsafe fn take(p: PWSTR) -> String {
        let s = p.to_string().unwrap_or_default();
        CoTaskMemFree(Some(p.0 as *const _));
        s
    }

    pub fn list() -> Result<Vec<Output>, String> {
        com(|| unsafe {
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            let def = en.GetDefaultAudioEndpoint(eRender, eMultimedia).ok().and_then(|d| d.GetId().ok()).map(|p| take(p)).unwrap_or_default();
            let col = en.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE).map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            for i in 0..col.GetCount().map_err(|e| e.to_string())? {
                let Ok(dev) = col.Item(i) else { continue };
                let Ok(id) = dev.GetId().map(|p| take(p)) else { continue };
                let name = dev
                    .OpenPropertyStore(STGM_READ)
                    .ok()
                    .and_then(|st| st.GetValue(&PKEY_Device_FriendlyName).ok())
                    .and_then(|v| PropVariantToStringAlloc(&v).ok())
                    .map(|p| take(p))
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| "Speakers".into());
                out.push(Output { default: id == def, id, name });
            }
            out.sort_by(|a, b| b.default.cmp(&a.default).then(a.name.cmp(&b.name)));
            Ok(out)
        })
    }

    type SetDefault = unsafe extern "system" fn(*mut c_void, PCWSTR, i32) -> HRESULT;
    type Release = unsafe extern "system" fn(*mut c_void) -> u32;

    pub fn set_default(id: &str) -> Result<(), String> {
        let wide: Vec<u16> = id.encode_utf16().chain([0]).collect();
        com(move || unsafe {
            let unk: IUnknown = CoCreateInstance(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL).map_err(|e| format!("Windows' sound settings didn't answer: {e}"))?;
            let mut raw: *mut c_void = std::ptr::null_mut();
            unk.query(&IID_POLICY_CONFIG, &mut raw).ok().map_err(|e| format!("Windows' sound settings didn't answer: {e}"))?;
            // IPolicyConfig's table: IUnknown (3), then ten methods, then SetDefaultEndpoint.
            let vtbl = *(raw as *const *const usize);
            let set: SetDefault = std::mem::transmute(*vtbl.add(13));
            let release: Release = std::mem::transmute(*vtbl.add(2));
            let mut result = Ok(());
            for role in 0..3 {
                let hr = set(raw, PCWSTR(wide.as_ptr()), role);
                if hr.is_err() {
                    result = Err(format!("Windows didn't switch the sound: {}", windows::core::Error::from(hr)));
                }
            }
            release(raw);
            result
        })
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Output;
    pub fn list() -> Result<Vec<Output>, String> {
        Ok(Vec::new())
    }
    pub fn set_default(_id: &str) -> Result<(), String> {
        Err("Choosing the sound output works on Windows".into())
    }
}
