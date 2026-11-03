//! Keys pressed anywhere in Windows, and controller buttons, for counters
//! and the auto clicker's start/stop key. A low-level keyboard hook sees
//! every key, but only the ones asked for (a 256-bit set) are passed on, and
//! nothing is stored or logged. Keys are never swallowed: the game in front
//! still gets them. Holding a key down counts once.
//!
//! Windows quietly drops a low-level hook that is slow to answer (a busy
//! PC), so the hook is put back every 15 seconds while it runs; and a key
//! whose release was missed counts again on its next press instead of
//! staying "held". Controller buttons (Xbox and anything shown to Windows as
//! one) are read through XInput while one is watched: codes `PAD + n`, n in
//! the standard order (A, B, X, Y, LB, RB, LT, RT, Back, Start, LS, RS, Up,
//! Down, Left, Right), the same order browsers use for gamepads.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::Instant;

use parking_lot::Mutex;

/// Controller buttons are `PAD + n`.
pub const PAD: u32 = 0x1000;
pub const PAD_NAMES: [&str; 16] = ["A", "B", "X", "Y", "LB", "RB", "LT", "RT", "Back", "Start", "Left stick", "Right stick", "D-pad up", "D-pad down", "D-pad left", "D-pad right"];

/// The virtual-key codes being watched.
static WATCHED: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
/// Keys down right now (so auto-repeat counts once).
#[cfg_attr(not(windows), allow(dead_code))]
static DOWN: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
/// When each key last went down (ms since start), to spot a missed release.
#[cfg_attr(not(windows), allow(dead_code))]
static LAST: [AtomicU64; 256] = [const { AtomicU64::new(0) }; 256];
/// Controller buttons being watched (bit n = PAD + n).
static PAD_WATCHED: AtomicU32 = AtomicU32::new(0);

/// Auto-repeat sends a key-down about every 30–50 ms while a key is held;
/// a gap longer than this means it was let go (and the release was missed).
#[cfg_attr(not(windows), allow(dead_code))]
const REPEAT_GAP_MS: u64 = 600;

fn bit(set: &[AtomicU64; 4], vk: u32) -> bool {
    vk < 256 && set[(vk / 64) as usize].load(Ordering::Relaxed) & (1 << (vk % 64)) != 0
}

#[cfg_attr(not(windows), allow(dead_code))]
fn set_bit(set: &[AtomicU64; 4], vk: u32, on: bool) {
    if vk < 256 {
        let m = 1u64 << (vk % 64);
        if on {
            set[(vk / 64) as usize].fetch_or(m, Ordering::Relaxed);
        } else {
            set[(vk / 64) as usize].fetch_and(!m, Ordering::Relaxed);
        }
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn now_ms() -> u64 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64 + 1
}

/// A key went down (`repeat_or_held`: the hook's own "already down" state).
/// True when it counts as a new press.
#[cfg_attr(not(windows), allow(dead_code))]
fn press_counts(vk: u32, now: u64) -> bool {
    let last = LAST[(vk & 0xFF) as usize].swap(now, Ordering::Relaxed);
    if !bit(&DOWN, vk) {
        set_bit(&DOWN, vk, true);
        return true;
    }
    // Marked down but quiet for a while: its release never arrived.
    now.saturating_sub(last) > REPEAT_GAP_MS
}

type Handler = Box<dyn Fn(u32) + Send + Sync>;

struct Hub {
    sender: Option<mpsc::Sender<u32>>,
    /// The hook thread's id (to stop it).
    #[cfg_attr(not(windows), allow(dead_code))]
    thread: Option<u32>,
    /// The controller thread runs.
    #[cfg_attr(not(windows), allow(dead_code))]
    pad: bool,
}

static HUB: Mutex<Hub> = Mutex::new(Hub { sender: None, thread: None, pad: false });
static HANDLERS: Mutex<Vec<Handler>> = Mutex::new(Vec::new());

/// Call `f` with each watched key's (or button's) code as it goes down.
pub fn on_key(f: impl Fn(u32) + Send + Sync + 'static) {
    HANDLERS.lock().push(Box::new(f));
}

fn sender() -> mpsc::Sender<u32> {
    let mut hub = HUB.lock();
    hub.sender
        .get_or_insert_with(|| {
            let (tx, rx) = mpsc::channel();
            std::thread::Builder::new().name("keys-dispatch".into()).spawn(move || dispatch(rx)).ok();
            tx
        })
        .clone()
}

/// Watch exactly these keys and buttons (none stops the hook and the controller).
pub fn watch(keys: &[u32]) {
    for (i, w) in WATCHED.iter().enumerate() {
        let mut v = 0u64;
        for &k in keys {
            if k < 256 && (k / 64) as usize == i {
                v |= 1 << (k % 64);
            }
        }
        w.store(v, Ordering::Relaxed);
    }
    let pads = keys.iter().filter(|&&k| (PAD..PAD + 16).contains(&k)).fold(0u32, |m, &k| m | 1 << (k - PAD));
    PAD_WATCHED.store(pads, Ordering::Relaxed);
    if keys.iter().any(|&k| k < 256) {
        imp::start();
    } else {
        imp::stop();
    }
    if pads != 0 {
        imp::start_pad();
    }
}

/// What a key or button is called ("F6", "Space", "A", "Pad A").
pub fn key_name(vk: u32) -> String {
    if (PAD..PAD + 16).contains(&vk) {
        return format!("Pad {}", PAD_NAMES[(vk - PAD) as usize]);
    }
    match vk {
        0x08 => "Backspace".into(),
        0x09 => "Tab".into(),
        0x0D => "Enter".into(),
        0x1B => "Esc".into(),
        0x20 => "Space".into(),
        0x21 => "Page Up".into(),
        0x22 => "Page Down".into(),
        0x23 => "End".into(),
        0x24 => "Home".into(),
        0x25 => "Left".into(),
        0x26 => "Up".into(),
        0x27 => "Right".into(),
        0x28 => "Down".into(),
        0x2D => "Insert".into(),
        0x2E => "Delete".into(),
        0x30..=0x39 | 0x41..=0x5A => char::from_u32(vk).map(String::from).unwrap_or_default(),
        0x60..=0x69 => format!("Num {}", vk - 0x60),
        0x6A => "Num *".into(),
        0x6B => "Num +".into(),
        0x6D => "Num -".into(),
        0x6F => "Num /".into(),
        0x70..=0x87 => format!("F{}", vk - 0x6F),
        _ => format!("Key {vk}"),
    }
}

fn dispatch(rx: mpsc::Receiver<u32>) {
    while let Ok(vk) = rx.recv() {
        for h in HANDLERS.lock().iter() {
            h(vk);
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, DispatchMessageW, GetMessageW, KillTimer, PostThreadMessageW, SetTimer, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER,
    };

    static SEND: Mutex<Option<mpsc::Sender<u32>>> = Mutex::new(None);

    unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            let vk = kb.vkCode;
            let msg = wparam.0 as u32;
            if bit(&WATCHED, vk) {
                if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                    if press_counts(vk, now_ms()) {
                        if let Some(tx) = SEND.lock().as_ref() {
                            let _ = tx.send(vk);
                        }
                    }
                } else if msg == WM_KEYUP || msg == WM_SYSKEYUP {
                    set_bit(&DOWN, vk, false);
                }
            }
        }
        // Always passed on: the key still reaches whatever is in front.
        unsafe { CallNextHookEx(None, code, wparam, lparam) }
    }

    fn install() -> Option<HHOOK> {
        unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), None, 0).ok() }
    }

    pub fn start() {
        let tx = sender();
        let mut hub = HUB.lock();
        if hub.thread.is_some() {
            return;
        }
        *SEND.lock() = Some(tx);
        let (id_tx, id_rx) = mpsc::channel();
        let spawned = std::thread::Builder::new().name("keys-hook".into()).spawn(move || unsafe {
            let _ = id_tx.send(GetCurrentThreadId());
            let Some(mut h) = install() else { return };
            // Put the hook back now and then: Windows removes one that was slow to answer.
            let timer = SetTimer(None, 0, 15_000, None);
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                if msg.message == WM_TIMER {
                    if let Some(fresh) = install() {
                        let _ = UnhookWindowsHookEx(h);
                        h = fresh;
                    }
                    continue;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            let _ = KillTimer(None, timer);
            let _ = UnhookWindowsHookEx(h);
        });
        if spawned.is_ok() {
            hub.thread = id_rx.recv().ok();
        }
    }

    pub fn stop() {
        if let Some(id) = HUB.lock().thread.take() {
            unsafe {
                let _ = PostThreadMessageW(id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        for d in &DOWN {
            d.store(0, Ordering::Relaxed);
        }
    }

    /// XInput button bits, in the standard order (LT and RT are the triggers).
    const XINPUT_BITS: [u16; 16] = [0x1000, 0x2000, 0x4000, 0x8000, 0x0100, 0x0200, 0, 0, 0x0020, 0x0010, 0x0040, 0x0080, 0x0001, 0x0002, 0x0004, 0x0008];

    fn pressed(s: &XINPUT_STATE) -> u32 {
        let b = s.Gamepad.wButtons.0;
        let mut m = 0u32;
        for (i, &x) in XINPUT_BITS.iter().enumerate() {
            if x != 0 && b & x != 0 {
                m |= 1 << i;
            }
        }
        if s.Gamepad.bLeftTrigger > 100 {
            m |= 1 << 6;
        }
        if s.Gamepad.bRightTrigger > 100 {
            m |= 1 << 7;
        }
        m
    }

    /// Read the controllers ~120 times a second while a button is watched.
    pub fn start_pad() {
        let tx = sender();
        {
            let mut hub = HUB.lock();
            if hub.pad {
                return;
            }
            hub.pad = true;
        }
        std::thread::Builder::new()
            .name("keys-pad".into())
            .spawn(move || {
                let mut before = [0u32; 4];
                let mut idle = 0u32;
                loop {
                    let watched = PAD_WATCHED.load(Ordering::Relaxed);
                    if watched == 0 {
                        break;
                    }
                    let mut any = false;
                    for (i, prev) in before.iter_mut().enumerate() {
                        let mut s = XINPUT_STATE::default();
                        if unsafe { XInputGetState(i as u32, &mut s) } != 0 {
                            *prev = 0;
                            continue;
                        }
                        any = true;
                        let now = pressed(&s);
                        let went_down = now & !*prev & watched;
                        for n in 0..16 {
                            if went_down & (1 << n) != 0 {
                                let _ = tx.send(PAD + n);
                            }
                        }
                        *prev = now;
                    }
                    // No controller: look for one less often.
                    idle = if any { 0 } else { idle.saturating_add(1) };
                    std::thread::sleep(std::time::Duration::from_millis(if idle > 120 { 500 } else { 8 }));
                }
                HUB.lock().pad = false;
            })
            .ok();
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    /// No system-wide keys off Windows; `press` stands in for tests.
    pub fn start() {
        let _ = sender();
    }
    pub fn stop() {}
    pub fn start_pad() {
        let _ = sender();
    }
}

/// Act as if a key or button went down (tests; nothing for ones not watched).
pub fn press(vk: u32) {
    let watched = bit(&WATCHED, vk) || ((PAD..PAD + 16).contains(&vk) && PAD_WATCHED.load(Ordering::Relaxed) & (1 << (vk - PAD)) != 0);
    if watched {
        let _ = sender().send(vk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(key_name(0x75), "F6");
        assert_eq!(key_name(0x41), "A");
        assert_eq!(key_name(0x20), "Space");
        assert_eq!(key_name(0x61), "Num 1");
        assert_eq!(key_name(PAD), "Pad A");
        assert_eq!(key_name(PAD + 15), "Pad D-pad right");
    }

    #[test]
    fn bits() {
        let set = [const { AtomicU64::new(0) }; 4];
        set_bit(&set, 200, true);
        assert!(bit(&set, 200) && !bit(&set, 201) && !bit(&set, 999));
        set_bit(&set, 200, false);
        assert!(!bit(&set, 200));
    }

    #[test]
    fn held_keys_count_once_and_a_lost_release_recovers() {
        let vk = 0x87; // F24: nothing else uses it
        set_bit(&DOWN, vk, false);
        assert!(press_counts(vk, 10_000), "first press");
        assert!(!press_counts(vk, 10_040), "auto-repeat while held");
        assert!(!press_counts(vk, 10_080));
        // The release never came; a press a second later still counts.
        assert!(press_counts(vk, 11_200));
        set_bit(&DOWN, vk, false);
    }
}
