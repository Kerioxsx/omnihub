//! Keys pressed anywhere in Windows, for counters and the auto clicker's
//! start/stop key. A low-level keyboard hook sees every key, but only the
//! ones asked for (a 256-bit set) are passed on, and nothing is stored or
//! logged. Keys are never swallowed: the game in front still gets them.
//! Holding a key down counts once. The hook runs only while some key is
//! watched.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

use parking_lot::Mutex;

/// The virtual-key codes being watched.
static WATCHED: [AtomicU64; 4] = [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];
/// Keys down right now (so auto-repeat counts once).
#[cfg_attr(not(windows), allow(dead_code))]
static DOWN: [AtomicU64; 4] = [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];

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

type Handler = Box<dyn Fn(u32) + Send + Sync>;

struct Hub {
    sender: Option<mpsc::Sender<u32>>,
    /// The hook thread's id (to stop it).
    #[cfg_attr(not(windows), allow(dead_code))]
    thread: Option<u32>,
}

static HUB: Mutex<Hub> = Mutex::new(Hub { sender: None, thread: None });
static HANDLERS: Mutex<Vec<Handler>> = Mutex::new(Vec::new());

/// Call `f` with each watched key's code as it goes down.
pub fn on_key(f: impl Fn(u32) + Send + Sync + 'static) {
    HANDLERS.lock().push(Box::new(f));
}

/// Watch exactly these keys (empty stops the hook).
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
    if keys.is_empty() {
        imp::stop();
    } else {
        imp::start();
    }
}

/// What a key is called ("F6", "Space", "A").
pub fn key_name(vk: u32) -> String {
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
    use windows::Win32::UI::WindowsAndMessaging::{CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP};

    static SEND: Mutex<Option<mpsc::Sender<u32>>> = Mutex::new(None);

    unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if code == HC_ACTION as i32 {
            let kb = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            let vk = kb.vkCode;
            let msg = wparam.0 as u32;
            if bit(&WATCHED, vk) {
                if msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN {
                    if !bit(&DOWN, vk) {
                        set_bit(&DOWN, vk, true);
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

    pub fn start() {
        let mut hub = HUB.lock();
        if hub.thread.is_some() {
            return;
        }
        let tx = hub.sender.get_or_insert_with(|| {
            let (tx, rx) = mpsc::channel();
            std::thread::Builder::new().name("keys-dispatch".into()).spawn(move || dispatch(rx)).ok();
            tx
        });
        *SEND.lock() = Some(tx.clone());
        let (id_tx, id_rx) = mpsc::channel();
        let spawned = std::thread::Builder::new().name("keys-hook".into()).spawn(move || unsafe {
            let _ = id_tx.send(GetCurrentThreadId());
            let Ok(h) = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), None, 0) else { return };
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
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
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    /// No system-wide keys off Windows; `press` stands in for tests.
    pub fn start() {
        let mut hub = HUB.lock();
        if hub.sender.is_none() {
            let (tx, rx) = mpsc::channel();
            std::thread::Builder::new().name("keys-dispatch".into()).spawn(move || dispatch(rx)).ok();
            hub.sender = Some(tx);
        }
    }

    pub fn stop() {}
}

/// Act as if a key went down (tests; does nothing for keys not watched).
pub fn press(vk: u32) {
    if bit(&WATCHED, vk) {
        if let Some(tx) = HUB.lock().sender.as_ref() {
            let _ = tx.send(vk);
        }
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
    }

    #[test]
    fn bits() {
        let set = [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)];
        set_bit(&set, 200, true);
        assert!(bit(&set, 200) && !bit(&set, 201) && !bit(&set, 999));
        set_bit(&set, 200, false);
        assert!(!bit(&set, 200));
    }
}
