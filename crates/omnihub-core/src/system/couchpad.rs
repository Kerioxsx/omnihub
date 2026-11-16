//! The controller as a remote while a streaming site is open: when Big
//! Picture opens Netflix, Crunchyroll or another channel in the browser,
//! an Xbox-style controller (anything Windows sees through XInput) drives
//! the browser: the left stick moves the pointer, A clicks, the right stick
//! scrolls, X plays and pauses, Y goes full screen, the D-pad presses the
//! arrow keys (seek and volume on most players), B is Escape, the bumpers
//! move between buttons on the page, LT goes back a page, and View closes
//! the channel and returns to Big Picture (and ends the remote). It only acts
//! while that browser is the window in front, so games and OmniHub itself are
//! never affected.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;

/// XInput's stick dead zone (left stick, as Microsoft recommends).
pub const DEAD: i32 = 7849;

/// Pointer speed (pixels per tick) for a stick position: nothing inside the
/// dead zone, then a curve so small moves are precise and full tilt is fast.
pub fn stick_speed(v: i16) -> i32 {
    let v = i32::from(v);
    if v.abs() <= DEAD {
        return 0;
    }
    let n = (v.abs() - DEAD) as f32 / (32767 - DEAD) as f32;
    let s = (n * n * 22.0 + n * 2.0).round() as i32;
    s.max(1) * v.signum()
}

/// What a button does in the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Act {
    Click,
    Key(u16),
    /// A key with a modifier held (modifier, key).
    Chord(u16, u16),
    Home,
}

/// XInput button bits → what they do. D-pad keys are held while held.
pub const MAP: &[(u16, Act)] = &[
    (0x1000, Act::Click),             // A
    (0x2000, Act::Key(0x1B)),         // B: Escape
    (0x4000, Act::Key(0x20)),         // X: Space (play / pause)
    (0x8000, Act::Key(0x46)),         // Y: F (full screen)
    (0x0001, Act::Key(0x26)),         // D-pad up
    (0x0002, Act::Key(0x28)),         // D-pad down
    (0x0004, Act::Key(0x25)),         // D-pad left
    (0x0008, Act::Key(0x27)),         // D-pad right
    (0x0010, Act::Key(0x0D)),         // Start: Enter
    (0x0020, Act::Home),              // View / Back: close the channel, back to Big Picture
    (0x0100, Act::Chord(0x10, 0x09)), // LB: Shift+Tab
    (0x0200, Act::Key(0x09)),         // RB: Tab
];

/// The controls, for the screen shown when a channel opens.
pub const LEGEND: &[(&str, &str)] = &[("Left stick", "Pointer"), ("A", "Click"), ("Right stick", "Scroll"), ("X", "Play / pause"), ("Y", "Full screen"), ("D-pad", "Seek and volume"), ("B", "Esc"), ("LB / RB", "Previous / next button"), ("LT", "Back a page"), ("View", "Close and return")];

#[derive(Default)]
pub struct CouchPad {
    stop: Mutex<Option<Arc<AtomicBool>>>,
}

impl CouchPad {
    /// Drive `exe` (the browser's program name) until stopped. `home` runs
    /// when View is pressed (the channel is closed first).
    pub fn start(&self, exe: String, home: Arc<dyn Fn() + Send + Sync>) {
        self.stop();
        let stop = Arc::new(AtomicBool::new(false));
        *self.stop.lock() = Some(stop.clone());
        imp::run(exe, stop, home);
    }

    pub fn stop(&self) {
        if let Some(s) = self.stop.lock().take() {
            s.store(true, Ordering::Relaxed);
        }
    }

    pub fn running(&self) -> bool {
        self.stop.lock().as_ref().is_some_and(|s| !s.load(Ordering::Relaxed))
    }
}

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};

    use super::{stick_speed, Act, MAP};
    use crate::capture::stream::input;

    fn tap(vk: u16) {
        input::key_state(vk, true);
        input::key_state(vk, false);
    }

    fn front_is(exe: &str) -> bool {
        crate::capture::screenshots::foreground_app().exe.as_deref().and_then(|p| std::path::Path::new(p).file_name().map(|n| n.to_string_lossy().to_string())).is_some_and(|n| n.eq_ignore_ascii_case(exe))
    }

    pub fn run(exe: String, stop: Arc<AtomicBool>, home: Arc<dyn Fn() + Send + Sync>) {
        let _ = std::thread::Builder::new().name("couch-pad".into()).spawn(move || {
            let started = Instant::now();
            let mut prev: u16 = 0;
            let mut lt_prev = false;
            let mut wheel_acc = 0f32;
            let mut held_keys: Vec<u16> = Vec::new();
            let mut clicking = false;
            let mut front = false;
            let mut checked = Instant::now() - Duration::from_secs(1);
            let release = |held: &mut Vec<u16>, clicking: &mut bool| {
                for k in held.drain(..) {
                    input::key_state(k, false);
                }
                if *clicking {
                    input::mouse_button(false, false);
                    *clicking = false;
                }
            };
            'run: while !stop.load(Ordering::Relaxed) && started.elapsed() < Duration::from_secs(6 * 3600) {
                if checked.elapsed() > Duration::from_millis(250) {
                    checked = Instant::now();
                    let now_front = front_is(&exe);
                    if front && !now_front {
                        release(&mut held_keys, &mut clicking);
                        prev = 0;
                    }
                    front = now_front;
                }
                if !front {
                    std::thread::sleep(Duration::from_millis(60));
                    continue;
                }
                // The first controller that answers.
                let mut st = None;
                for i in 0..4u32 {
                    let mut s = XINPUT_STATE::default();
                    if unsafe { XInputGetState(i, &mut s) } == 0 {
                        st = Some(s);
                        break;
                    }
                }
                let Some(s) = st else {
                    std::thread::sleep(Duration::from_millis(250));
                    continue;
                };
                let g = s.Gamepad;
                let now = g.wButtons.0;
                let (dx, dy) = (stick_speed(g.sThumbLX), -stick_speed(g.sThumbLY));
                if dx != 0 || dy != 0 {
                    input::mouse_move(dx, dy);
                }
                let ry = i32::from(g.sThumbRY);
                if ry.abs() > 8689 {
                    wheel_acc += (ry.abs() - 8689) as f32 / (32767 - 8689) as f32 * 30.0 * ry.signum() as f32;
                    if wheel_acc.abs() >= 40.0 {
                        // Stick up scrolls up (`wheel` takes a page-style delta: positive is down).
                        input::wheel(-wheel_acc, 0.0);
                        wheel_acc = 0.0;
                    }
                } else {
                    wheel_acc = 0.0;
                }
                for (bit, act) in MAP {
                    let down = now & bit != 0;
                    let was = prev & bit != 0;
                    if down == was {
                        continue;
                    }
                    match (act, down) {
                        (Act::Click, d) => {
                            input::mouse_button(false, d);
                            clicking = d;
                        }
                        // The D-pad repeats like a held key.
                        (Act::Key(k), true) if (0x25..=0x28).contains(k) => {
                            input::key_state(*k, true);
                            held_keys.push(*k);
                        }
                        (Act::Key(k), false) if (0x25..=0x28).contains(k) => {
                            input::key_state(*k, false);
                            held_keys.retain(|x| x != k);
                        }
                        (Act::Key(k), true) => tap(*k),
                        (Act::Chord(m, k), true) => {
                            input::key_state(*m, true);
                            tap(*k);
                            input::key_state(*m, false);
                        }
                        (Act::Home, true) => {
                            release(&mut held_keys, &mut clicking);
                            // Ctrl+W closes an app window (and a kiosk tab).
                            input::key_state(0x11, true);
                            tap(0x57);
                            input::key_state(0x11, false);
                            home();
                            // The channel is closed: the remote's job is done.
                            stop.store(true, Ordering::Relaxed);
                            break 'run;
                        }
                        _ => {}
                    }
                }
                let lt = g.bLeftTrigger > 140;
                if lt && !lt_prev {
                    input::key_state(0x12, true);
                    tap(0x25);
                    input::key_state(0x12, false);
                }
                lt_prev = lt;
                prev = now;
                std::thread::sleep(Duration::from_millis(8));
            }
            release(&mut held_keys, &mut clicking);
        });
    }
}

#[cfg(not(windows))]
mod imp {
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    pub fn run(_exe: String, _stop: Arc<AtomicBool>, _home: Arc<dyn Fn() + Send + Sync>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_curve_and_map() {
        assert_eq!(stick_speed(0), 0);
        assert_eq!(stick_speed(7000), 0);
        assert_eq!(stick_speed(-7000), 0);
        assert!(stick_speed(9000) >= 1 && stick_speed(9000) <= 2);
        assert_eq!(stick_speed(32767), 24);
        assert_eq!(stick_speed(-32768), -24);
        assert!(MAP.iter().any(|(b, a)| *b == 0x0020 && *a == Act::Home));
        let c = CouchPad::default();
        assert!(!c.running());
        c.start("msedge.exe".into(), Arc::new(|| {}));
        assert!(c.running());
        c.stop();
        assert!(!c.running());
    }
}
