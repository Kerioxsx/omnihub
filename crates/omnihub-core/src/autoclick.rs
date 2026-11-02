//! Auto clicker: clicks at a set interval, down to 0.01 ms. Below about a
//! millisecond Windows can't keep up with one click per interval, so it
//! sends several per call, waits by spinning (that keeps one CPU core busy
//! while it runs) and reports the speed it really reaches. It always stops
//! with its start/stop key (F6 unless changed), with Esc, by pushing the
//! mouse into the top-left corner of the screen, and at the limits set.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;

pub const ESC: u32 = 0x1B;
pub const MIN_INTERVAL_MS: f64 = 0.01;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ClickConfig {
    /// "left", "right" or "middle".
    pub button: String,
    pub double: bool,
    /// Milliseconds between clicks (0.01 at least).
    pub interval_ms: f64,
    /// Each wait is up to this many milliseconds longer or shorter.
    pub jitter_ms: f64,
    /// Stop after this many clicks (0: no limit).
    pub max_clicks: u64,
    /// Stop after this many seconds (0: no limit).
    pub max_seconds: u64,
    /// Click at this point instead of where the mouse is.
    pub at: Option<(i32, i32)>,
    /// Starts and stops it from anywhere (Windows virtual-key code).
    pub toggle_key: u32,
}

impl Default for ClickConfig {
    fn default() -> Self {
        ClickConfig { button: "left".into(), double: false, interval_ms: 100.0, jitter_ms: 0.0, max_clicks: 0, max_seconds: 0, at: None, toggle_key: 0x75 }
    }
}

impl ClickConfig {
    pub fn check(&self) -> Result<(), String> {
        if !["left", "right", "middle"].contains(&self.button.as_str()) {
            return Err("Pick left, right or middle".into());
        }
        if !self.interval_ms.is_finite() || self.interval_ms < MIN_INTERVAL_MS || self.interval_ms > 3_600_000.0 {
            return Err("The interval goes from 0.01 ms to an hour".into());
        }
        if !self.jitter_ms.is_finite() || self.jitter_ms < 0.0 || self.jitter_ms > self.interval_ms {
            return Err("The random part can't be more than the interval".into());
        }
        if self.toggle_key == 0 || self.toggle_key >= 256 || self.toggle_key == ESC {
            return Err("Pick another start/stop key (Esc always stops)".into());
        }
        Ok(())
    }

    /// Clicks sent per call: enough that each call covers about 50 µs.
    pub fn batch(&self) -> u32 {
        if self.interval_ms >= 0.05 || self.jitter_ms > 0.0 {
            1
        } else {
            ((0.05 / self.interval_ms).ceil() as u32).clamp(1, 50)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClickStatus {
    pub running: bool,
    pub clicks: u64,
    /// Clicks per second reached (over the last moment).
    pub rate: f64,
    pub seconds: f64,
    /// Why it stopped last time.
    pub stopped: Option<String>,
}

pub struct AutoClicker {
    config: Mutex<ClickConfig>,
    /// Its key is watched only once the auto clicker was opened.
    armed: AtomicBool,
    running: Arc<AtomicBool>,
    status: Arc<Mutex<ClickStatus>>,
    events: EventBus,
}

impl AutoClicker {
    pub fn new(events: EventBus) -> AutoClicker {
        AutoClicker { config: Mutex::new(ClickConfig::default()), armed: AtomicBool::new(false), running: Arc::new(AtomicBool::new(false)), status: Arc::new(Mutex::new(ClickStatus::default())), events }
    }

    pub fn config(&self) -> ClickConfig {
        self.config.lock().clone()
    }

    pub fn set_config(&self, c: ClickConfig) -> Result<(), String> {
        c.check()?;
        if self.running.load(Ordering::SeqCst) {
            return Err("Stop the clicker before changing it".into());
        }
        *self.config.lock() = c;
        Ok(())
    }

    pub fn status(&self) -> ClickStatus {
        self.status.lock().clone()
    }

    pub fn running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Let its start/stop key work from anywhere (after the page was opened).
    pub fn arm(&self) {
        self.armed.store(true, Ordering::SeqCst);
    }

    /// The keys it needs watched: its start/stop key once armed, and Esc while it runs.
    pub fn watched_keys(&self) -> Vec<u32> {
        let mut v = Vec::new();
        if self.armed.load(Ordering::SeqCst) {
            v.push(self.config.lock().toggle_key);
        }
        if self.running() {
            v.push(ESC);
        }
        v
    }

    /// A watched key went down. True when the clicker started or stopped.
    pub fn on_key(&self, vk: u32) -> bool {
        if vk == ESC && self.running() {
            self.stop("Esc");
            return true;
        }
        if vk == self.config.lock().toggle_key {
            if self.running() {
                self.stop("its key");
            } else {
                let _ = self.start();
            }
            return true;
        }
        false
    }

    pub fn start(&self) -> Result<(), String> {
        let cfg = self.config();
        cfg.check()?;
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        *self.status.lock() = ClickStatus { running: true, ..Default::default() };
        self.events.emit("autoclick:status", &*self.status.lock());
        let (running, status, events) = (self.running.clone(), self.status.clone(), self.events.clone());
        std::thread::Builder::new()
            .name("autoclick".into())
            .spawn(move || {
                let reason = run(&cfg, &running, &status, &events);
                running.store(false, Ordering::SeqCst);
                let mut s = status.lock();
                s.running = false;
                if s.stopped.is_none() {
                    s.stopped = Some(reason);
                }
                events.emit("autoclick:status", &*s);
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn stop(&self, why: &str) {
        if self.running.swap(false, Ordering::SeqCst) {
            self.status.lock().stopped = Some(format!("Stopped with {why}"));
        }
    }
}

/// Click until told to stop or a limit is reached; returns why it ended.
fn run(cfg: &ClickConfig, running: &AtomicBool, status: &Mutex<ClickStatus>, events: &EventBus) -> String {
    let batch = cfg.batch() as u64;
    let per_click = if cfg.double { 2 } else { 1 };
    let interval = Duration::from_secs_f64(cfg.interval_ms / 1000.0);
    let started = Instant::now();
    let mut next = started;
    let mut clicks = 0u64;
    let (mut last_report, mut last_clicks) = (started, 0u64);
    let mut last_check = started;
    imp::prepare();
    while running.load(Ordering::SeqCst) {
        let now = Instant::now();
        // Fail-safe: the mouse pushed into the top-left corner.
        if now.duration_since(last_check) >= Duration::from_millis(20) {
            last_check = now;
            if cfg.at.is_none_or(|(x, y)| x > 2 || y > 2) && imp::cursor_in_corner() {
                return "Stopped: the mouse went to the top-left corner".into();
            }
        }
        if let Err(e) = imp::click(cfg, batch as u32) {
            return e;
        }
        clicks += batch * per_click;
        if cfg.max_clicks > 0 && clicks >= cfg.max_clicks {
            report(status, events, clicks, started, &mut last_report, &mut last_clicks, true);
            return format!("Done: {} clicks", cfg.max_clicks);
        }
        if cfg.max_seconds > 0 && started.elapsed().as_secs() >= cfg.max_seconds {
            report(status, events, clicks, started, &mut last_report, &mut last_clicks, true);
            return format!("Done: {} s", cfg.max_seconds);
        }
        report(status, events, clicks, started, &mut last_report, &mut last_clicks, false);
        // The next click's time, with the random part.
        let mut wait = interval * batch as u32;
        if cfg.jitter_ms > 0.0 {
            let j = (rand::random::<f64>() * 2.0 - 1.0) * cfg.jitter_ms / 1000.0;
            wait = Duration::from_secs_f64((wait.as_secs_f64() + j).max(MIN_INTERVAL_MS / 1000.0));
        }
        next += wait;
        let now = Instant::now();
        if next > now {
            let left = next - now;
            if left > Duration::from_millis(2) {
                std::thread::sleep(left - Duration::from_millis(1));
            }
            while Instant::now() < next && running.load(Ordering::Relaxed) {
                std::hint::spin_loop();
            }
        } else if now - next > Duration::from_millis(100) {
            // Far behind (Windows can't go that fast): don't try to catch up.
            next = now;
        }
    }
    "Stopped".into()
}

fn report(status: &Mutex<ClickStatus>, events: &EventBus, clicks: u64, started: Instant, last: &mut Instant, last_clicks: &mut u64, force: bool) {
    let dt = last.elapsed();
    if !force && dt < Duration::from_millis(250) {
        return;
    }
    let rate = (clicks - *last_clicks) as f64 / dt.as_secs_f64().max(1e-6);
    *last = Instant::now();
    *last_clicks = clicks;
    let mut s = status.lock();
    s.clicks = clicks;
    s.rate = rate;
    s.seconds = started.elapsed().as_secs_f64();
    events.emit("autoclick:status", &*s);
}

/// Where the mouse is now (for picking a point).
pub fn cursor() -> Option<(i32, i32)> {
    imp::cursor()
}

#[cfg(windows)]
mod imp {
    use super::ClickConfig;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST};
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, MOUSE_EVENT_FLAGS};
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

    pub fn prepare() {
        unsafe {
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
        }
    }

    pub fn cursor() -> Option<(i32, i32)> {
        let mut p = POINT::default();
        unsafe { GetCursorPos(&mut p).ok().map(|_| (p.x, p.y)) }
    }

    pub fn cursor_in_corner() -> bool {
        cursor().is_some_and(|(x, y)| x <= 2 && y <= 2)
    }

    fn input(flags: MOUSE_EVENT_FLAGS) -> INPUT {
        INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 } } }
    }

    pub fn click(cfg: &ClickConfig, batch: u32) -> Result<(), String> {
        if let Some((x, y)) = cfg.at {
            unsafe {
                let _ = SetCursorPos(x, y);
            }
        }
        let (down, up) = match cfg.button.as_str() {
            "right" => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
            "middle" => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP),
            _ => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        };
        let per = if cfg.double { 2 } else { 1 };
        let mut inputs = Vec::with_capacity((batch * per * 2) as usize);
        for _ in 0..batch * per {
            inputs.push(input(down));
            inputs.push(input(up));
        }
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent == 0 {
            return Err("Windows blocked the clicks (the window in front may be running as administrator: run OmniHub as administrator too)".into());
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use super::ClickConfig;

    pub fn prepare() {}
    pub fn cursor() -> Option<(i32, i32)> {
        None
    }
    pub fn cursor_in_corner() -> bool {
        false
    }
    pub fn click(_cfg: &ClickConfig, _batch: u32) -> Result<(), String> {
        Err("The auto clicker works on Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_checks_and_batches() {
        let ok = ClickConfig::default();
        assert!(ok.check().is_ok());
        assert!(ClickConfig { interval_ms: 0.001, ..ok.clone() }.check().is_err());
        assert!(ClickConfig { interval_ms: 5.0, jitter_ms: 6.0, ..ok.clone() }.check().is_err());
        assert!(ClickConfig { button: "side".into(), ..ok.clone() }.check().is_err());
        assert!(ClickConfig { toggle_key: ESC, ..ok.clone() }.check().is_err());
        assert_eq!(ClickConfig { interval_ms: 0.01, ..ok.clone() }.batch(), 5);
        assert_eq!(ClickConfig { interval_ms: 1.0, ..ok.clone() }.batch(), 1);
    }

    #[test]
    fn keys_start_and_stop() {
        let a = AutoClicker::new(EventBus::new());
        assert!(a.watched_keys().is_empty(), "nothing watched until opened");
        a.arm();
        assert_eq!(a.watched_keys(), vec![0x75]);
        assert!(!a.on_key(0x41));
        // Off Windows it starts and stops at once (no clicks to send).
        a.on_key(0x75);
        for _ in 0..100 {
            if !a.running() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!a.running());
        assert!(a.status().stopped.is_some());
        assert!(a.set_config(ClickConfig { interval_ms: 0.0, ..Default::default() }).is_err());
    }
}
