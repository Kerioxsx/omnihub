//! Keep the screen on while the music player shows Aurora and music plays
//! (Windows would otherwise dim and turn it off as if nobody were there).
//! The request belongs to one long-lived thread, as Windows ties it to the
//! thread that made it; it ends when turned off or when OmniHub exits.

use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

static TX: OnceLock<Mutex<Sender<bool>>> = OnceLock::new();

/// Ask Windows to keep the display on (`true`) or let it sleep again.
pub fn keep_display_on(on: bool) {
    let tx = TX.get_or_init(|| {
        let (tx, rx) = channel::<bool>();
        let _ = std::thread::Builder::new().name("keep-awake".into()).spawn(move || {
            while let Ok(on) = rx.recv() {
                apply(on);
            }
        });
        Mutex::new(tx)
    });
    if let Ok(tx) = tx.lock() {
        let _ = tx.send(on);
    }
}

#[cfg(windows)]
fn apply(on: bool) {
    use windows::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED};
    // SAFETY: only changes this thread's execution state.
    unsafe {
        SetThreadExecutionState(if on { ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED } else { ES_CONTINUOUS });
    }
}

#[cfg(not(windows))]
fn apply(_on: bool) {}

/// A game, video or presentation fills the screen (Windows' own "busy"
/// signal, the one it uses to hold back notifications).
#[cfg(windows)]
pub fn fullscreen_app_running() -> bool {
    use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN};
    // SAFETY: plain query.
    unsafe { SHQueryUserNotificationState().is_ok_and(|s| s == QUNS_BUSY || s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE) }
}

#[cfg(not(windows))]
pub fn fullscreen_app_running() -> bool {
    false
}
