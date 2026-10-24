//! Leave when Windows asks: at sign-out and shutdown, and when an installer
//! closes running programs through the Restart Manager before replacing
//! their files. Both send WM_QUERYENDSESSION and WM_ENDSESSION to the
//! program's windows; a window that only hides (OmniHub's close-to-tray)
//! would keep omnihub.exe running and locked, so the setup fails with
//! "Error writing to file".

/// Call `on_end` when Windows ends the session or closes the app for an
/// installer. `hwnd` is a top-level window of this thread (the main window).
/// Returns false when the window could not be watched.
#[cfg(windows)]
pub fn watch(hwnd: isize, on_end: impl Fn() + Send + Sync + 'static) -> bool {
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{WM_ENDSESSION, WM_QUERYENDSESSION};

    type Handler = Box<dyn Fn() + Send + Sync>;

    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, data: usize) -> LRESULT {
        match msg {
            // Yes, OmniHub can close.
            WM_QUERYENDSESSION => LRESULT(1),
            WM_ENDSESSION if wparam.0 != 0 => {
                // SAFETY: `data` is the leaked handler below, alive for the process.
                let on_end = unsafe { &*(data as *const Handler) };
                on_end();
                LRESULT(0)
            }
            // SAFETY: forwarding the message this subclass received.
            _ => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },
        }
    }

    let handler: Handler = Box::new(on_end);
    // Lives as long as the window, which is as long as the app.
    let data = Box::into_raw(Box::new(handler)) as usize;
    // SAFETY: called on the window's own thread with a valid callback.
    unsafe { SetWindowSubclass(HWND(hwnd as *mut _), Some(proc), 0x4f48, data).as_bool() }
}

#[cfg(not(windows))]
pub fn watch(_hwnd: isize, _on_end: impl Fn() + Send + Sync + 'static) -> bool {
    false
}
