//! Keep the screen on while the music player shows Aurora and music plays
//! (Windows would otherwise dim and turn it off as if nobody were there),
//! and keep the PC from sleeping while a Minecraft server runs (the screen
//! may still turn off). The request belongs to one long-lived thread, as
//! Windows ties it to the thread that made it; it ends when turned off or
//! when OmniHub exits.

use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

/// What wants the PC awake: the display (music) or just the system (servers).
enum Ask {
    Display(bool),
    System(bool),
}

static TX: OnceLock<Mutex<Sender<Ask>>> = OnceLock::new();

fn send(a: Ask) {
    let tx = TX.get_or_init(|| {
        let (tx, rx) = channel::<Ask>();
        let _ = std::thread::Builder::new().name("keep-awake".into()).spawn(move || {
            let (mut display, mut system) = (false, false);
            while let Ok(a) = rx.recv() {
                match a {
                    Ask::Display(on) => display = on,
                    Ask::System(on) => system = on,
                }
                apply(display, system);
            }
        });
        Mutex::new(tx)
    });
    if let Ok(tx) = tx.lock() {
        let _ = tx.send(a);
    }
}

/// Ask Windows to keep the display on (`true`) or let it sleep again.
pub fn keep_display_on(on: bool) {
    send(Ask::Display(on));
}

/// Keep the PC from going to sleep (the display may still turn off).
pub fn keep_system_on(on: bool) {
    send(Ask::System(on));
}

#[cfg(windows)]
fn apply(display: bool, system: bool) {
    use windows::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED};
    let mut flags = ES_CONTINUOUS;
    if display {
        flags |= ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED;
    }
    if system {
        flags |= ES_SYSTEM_REQUIRED;
    }
    // SAFETY: only changes this thread's execution state.
    unsafe {
        SetThreadExecutionState(flags);
    }
}

#[cfg(not(windows))]
fn apply(_display: bool, _system: bool) {}

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
