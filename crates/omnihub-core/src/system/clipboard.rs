//! Clipboard access for secrets: copied passwords are kept out of Windows
//! clipboard history and cloud sync, and cleared after a timeout if the
//! clipboard still holds them.

use std::time::Duration;

#[cfg(windows)]
mod imp {
    use windows::core::w;
    use windows::Win32::Foundation::{HANDLE, HGLOBAL};
    use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, RegisterClipboardFormatW, SetClipboardData};
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    const CF_UNICODETEXT: u32 = 13;

    struct Open;
    impl Open {
        fn new() -> std::io::Result<Self> {
            for _ in 0..10 {
                if unsafe { OpenClipboard(None) }.is_ok() {
                    return Ok(Open);
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(std::io::Error::other("clipboard is busy"))
        }
    }
    impl Drop for Open {
        fn drop(&mut self) {
            let _ = unsafe { CloseClipboard() };
        }
    }

    unsafe fn global_from(bytes: &[u8]) -> std::io::Result<HGLOBAL> {
        unsafe {
            let h = GlobalAlloc(GMEM_MOVEABLE, bytes.len()).map_err(|e| std::io::Error::other(e.message()))?;
            let p = GlobalLock(h) as *mut u8;
            if p.is_null() {
                return Err(std::io::Error::other("GlobalLock failed"));
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, bytes.len());
            let _ = GlobalUnlock(h);
            Ok(h)
        }
    }

    pub fn set_secret(text: &str) -> std::io::Result<()> {
        let _open = Open::new()?;
        unsafe {
            EmptyClipboard().map_err(|e| std::io::Error::other(e.message()))?;
            let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes = std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2);
            let h = global_from(bytes)?;
            SetClipboardData(CF_UNICODETEXT, Some(HANDLE(h.0))).map_err(|e| std::io::Error::other(e.message()))?;
            // Formats documented by Microsoft for password managers.
            let zero = 0u32.to_le_bytes();
            for fmt in [w!("CanIncludeInClipboardHistory"), w!("CanUploadToCloudClipboard")] {
                let id = RegisterClipboardFormatW(fmt);
                if id != 0 {
                    if let Ok(h) = global_from(&zero) {
                        let _ = SetClipboardData(id, Some(HANDLE(h.0)));
                    }
                }
            }
            let id = RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing"));
            if id != 0 {
                if let Ok(h) = global_from(&zero) {
                    let _ = SetClipboardData(id, Some(HANDLE(h.0)));
                }
            }
        }
        Ok(())
    }

    pub fn get_text() -> Option<String> {
        let _open = Open::new().ok()?;
        unsafe {
            let h = GetClipboardData(CF_UNICODETEXT).ok()?;
            let p = GlobalLock(HGLOBAL(h.0)) as *const u16;
            if p.is_null() {
                return None;
            }
            let mut len = 0usize;
            while *p.add(len) != 0 {
                len += 1;
            }
            let s = String::from_utf16_lossy(std::slice::from_raw_parts(p, len));
            let _ = GlobalUnlock(HGLOBAL(h.0));
            Some(s)
        }
    }

    pub fn clear() {
        if let Ok(_open) = Open::new() {
            let _ = unsafe { EmptyClipboard() };
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn set_secret(text: &str) -> std::io::Result<()> {
        let mut c = arboard::Clipboard::new().map_err(|e| std::io::Error::other(e.to_string()))?;
        c.set_text(text.to_string()).map_err(|e| std::io::Error::other(e.to_string()))
    }

    pub fn get_text() -> Option<String> {
        arboard::Clipboard::new().ok()?.get_text().ok()
    }

    pub fn clear() {
        if let Ok(mut c) = arboard::Clipboard::new() {
            let _ = c.clear();
        }
    }
}

/// Copy a secret and clear it after `clear_after` unless something else
/// was copied in the meantime.
pub fn copy_secret(text: &str, clear_after: Duration) -> std::io::Result<()> {
    imp::set_secret(text)?;
    if clear_after.is_zero() {
        return Ok(());
    }
    let fingerprint = blake3::hash(text.as_bytes());
    std::thread::spawn(move || {
        std::thread::sleep(clear_after);
        if imp::get_text().is_some_and(|t| blake3::hash(t.as_bytes()) == fingerprint) {
            imp::clear();
        }
    });
    Ok(())
}

/// Copy ordinary text (no history exclusion, no timeout).
pub fn copy_text(text: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let mut c = arboard::Clipboard::new().map_err(|e| std::io::Error::other(e.to_string()))?;
        c.set_text(text.to_string()).map_err(|e| std::io::Error::other(e.to_string()))
    }
    #[cfg(not(windows))]
    imp::set_secret(text)
}

/// Copy an RGBA image.
pub fn copy_image(width: usize, height: usize, rgba: &[u8]) -> std::io::Result<()> {
    let mut c = arboard::Clipboard::new().map_err(|e| std::io::Error::other(e.to_string()))?;
    c.set_image(arboard::ImageData { width, height, bytes: std::borrow::Cow::Borrowed(rgba) })
        .map_err(|e| std::io::Error::other(e.to_string()))
}
