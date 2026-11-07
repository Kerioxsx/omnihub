//! Pin any window on top: keep a window (a video, a guide, a chat) above
//! everything else, and unpin it again. From the list of open windows, or
//! the window in front (a deck or wheel button).

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PinWindow {
    pub handle: i64,
    pub title: String,
    /// "chrome.exe"
    pub app: String,
    pub pinned: bool,
}

pub fn list() -> Vec<PinWindow> {
    imp::list()
}

/// Pin or unpin a window by its handle; says whether it's now pinned.
pub fn set(handle: i64, pinned: bool) -> Result<bool, String> {
    imp::set(handle, pinned)
}

/// Pin the window in front, or unpin it if it's pinned.
pub fn toggle_front() -> Result<(String, bool), String> {
    imp::toggle_front()
}

#[cfg(windows)]
mod imp {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowLongW, GetWindowTextW, IsWindow, SetWindowPos, GWL_EXSTYLE, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, WS_EX_TOPMOST};

    use super::PinWindow;

    fn pinned(h: HWND) -> bool {
        unsafe { GetWindowLongW(h, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0 }
    }

    pub fn list() -> Vec<PinWindow> {
        crate::system::open_apps::win::handles().into_iter().map(|(h, key, title, _)| PinWindow { handle: h as i64, title, app: key, pinned: pinned(HWND(h as *mut _)) }).collect()
    }

    pub fn set(handle: i64, on: bool) -> Result<bool, String> {
        let h = HWND(handle as isize as *mut _);
        if !unsafe { IsWindow(Some(h)) }.as_bool() {
            return Err("That window is closed".into());
        }
        unsafe { SetWindowPos(h, Some(if on { HWND_TOPMOST } else { HWND_NOTOPMOST }), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE) }.map_err(|e| format!("Windows didn't change it: {e}"))?;
        Ok(pinned(h))
    }

    pub fn toggle_front() -> Result<(String, bool), String> {
        let h = unsafe { GetForegroundWindow() };
        if h.is_invalid() {
            return Err("No window in front".into());
        }
        let mut buf = [0u16; 256];
        let n = unsafe { GetWindowTextW(h, &mut buf) } as usize;
        let title = String::from_utf16_lossy(&buf[..n]);
        let now = set(h.0 as isize as i64, !pinned(h))?;
        Ok((title, now))
    }
}

#[cfg(not(windows))]
mod imp {
    use super::PinWindow;
    pub fn list() -> Vec<PinWindow> {
        Vec::new()
    }
    pub fn set(_handle: i64, _on: bool) -> Result<bool, String> {
        Err("Pinning windows works on Windows".into())
    }
    pub fn toggle_front() -> Result<(String, bool), String> {
        Err("Pinning windows works on Windows".into())
    }
}
