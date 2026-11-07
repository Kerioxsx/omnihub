//! The phone as a controller: its on-screen pad sends buttons, and the PC
//! holds the matching keys (RetroArch's default keyboard layout, which most
//! emulators can be set to). Keys are let go when the phone says so, when
//! it stops talking for a few seconds, or when the page closes, so nothing
//! stays stuck. A real virtual Xbox controller would need a driver
//! (ViGEmBus, no longer maintained), so OmniHub uses the keyboard.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// (button, key, key name): RetroArch's defaults.
pub const BUTTONS: &[(&str, u16, &str)] = &[
    ("up", 0x26, "Up"),
    ("down", 0x28, "Down"),
    ("left", 0x25, "Left"),
    ("right", 0x27, "Right"),
    ("a", 0x58, "X"),
    ("b", 0x5A, "Z"),
    ("x", 0x53, "S"),
    ("y", 0x41, "A"),
    ("l", 0x51, "Q"),
    ("r", 0x57, "W"),
    ("start", 0x0D, "Enter"),
    ("select", 0xA1, "Right Shift"),
];

/// Held keys are let go when the phone has been quiet this long.
pub const QUIET: Duration = Duration::from_secs(4);

pub fn key_for(button: &str) -> Option<u16> {
    BUTTONS.iter().find(|(b, _, _)| *b == button).map(|(_, k, _)| *k)
}

#[derive(Default)]
pub struct PadRemote {
    /// Held keys and when the phone last spoke about them.
    held: Mutex<HashMap<u16, Instant>>,
}

impl PadRemote {
    /// A button pressed or let go; returns whether it's a known button.
    pub fn button(&self, button: &str, down: bool, send: &dyn Fn(u16, bool)) -> bool {
        let Some(vk) = key_for(button) else { return false };
        let mut held = self.held.lock();
        if down {
            if held.insert(vk, Instant::now()).is_none() {
                send(vk, true);
            }
        } else if held.remove(&vk).is_some() {
            send(vk, false);
        }
        true
    }

    /// The phone is still there (sent while buttons are held).
    pub fn alive(&self) {
        let now = Instant::now();
        for t in self.held.lock().values_mut() {
            *t = now;
        }
    }

    pub fn release_all(&self, send: &dyn Fn(u16, bool)) {
        for (vk, _) in self.held.lock().drain() {
            send(vk, false);
        }
    }

    /// Let go of keys the phone stopped talking about.
    pub fn sweep(&self, send: &dyn Fn(u16, bool)) {
        let mut held = self.held.lock();
        let stale: Vec<u16> = held.iter().filter(|(_, t)| t.elapsed() >= QUIET).map(|(k, _)| *k).collect();
        for vk in stale {
            held.remove(&vk);
            send(vk, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_and_lets_go() {
        let log = Mutex::new(Vec::new());
        let send = |vk: u16, down: bool| log.lock().push((vk, down));
        let p = PadRemote::default();
        assert!(p.button("a", true, &send));
        assert!(p.button("a", true, &send), "a repeat doesn't press again");
        assert!(p.button("up", true, &send));
        assert!(!p.button("turbo", true, &send));
        assert!(p.button("a", false, &send));
        p.release_all(&send);
        assert_eq!(*log.lock(), vec![(0x58, true), (0x26, true), (0x58, false), (0x26, false)]);
        p.sweep(&send);
        assert_eq!(log.lock().len(), 4);
    }
}
