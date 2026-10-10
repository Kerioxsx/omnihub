//! The PC's master volume and mute (the default playback device).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    /// 0.0–1.0
    pub level: f32,
    pub muted: bool,
}

pub fn get() -> Option<Volume> {
    #[cfg(windows)]
    {
        win::with_endpoint(|ep| unsafe { Ok(Volume { level: ep.GetMasterVolumeLevelScalar()?, muted: ep.GetMute()?.as_bool() }) }).ok()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn set(level: Option<f32>, muted: Option<bool>) -> Result<Volume, String> {
    #[cfg(windows)]
    {
        win::with_endpoint(|ep| unsafe {
            if let Some(l) = level {
                ep.SetMasterVolumeLevelScalar(l.clamp(0.0, 1.0), std::ptr::null())?;
            }
            if let Some(m) = muted {
                ep.SetMute(m, std::ptr::null())?;
            }
            Ok(Volume { level: ep.GetMasterVolumeLevelScalar()?, muted: ep.GetMute()?.as_bool() })
        })
        .map_err(|e| e.message().to_string())
    }
    #[cfg(not(windows))]
    {
        let _ = (level, muted);
        Err("Volume control is a Windows feature.".into())
    }
}

#[cfg(windows)]
mod win {
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

    pub fn with_endpoint<T: Send>(f: impl FnOnce(&IAudioEndpointVolume) -> windows::core::Result<T> + Send) -> windows::core::Result<T> {
        // COM calls run on a short-lived thread of their own (any apartment
        // state on the caller's thread stays untouched).
        let r = std::thread::scope(|s| {
            s.spawn(|| unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
                let device = devices.GetDefaultAudioEndpoint(eRender, eConsole)?;
                let ep: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
                f(&ep)
            })
            .join()
        });
        r.unwrap_or_else(|_| Err(windows::core::Error::from(windows::core::HRESULT(0x8000_FFFFu32 as i32))))
    }
}
