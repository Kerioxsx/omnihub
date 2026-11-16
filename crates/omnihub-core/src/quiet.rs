//! Quiet mode, for the couch or the night: the screens off and the fans at
//! the lowest safe speed, with the PC still on (downloads, servers, the
//! phone keep working). Fans never stop completely — a running processor
//! would overheat — they sit at their floor and still speed up if it gets
//! warm; the graphics card goes back to its own control, which stops its
//! fans when it's cool on most cards. It ends when someone uses the PC's
//! mouse or keyboard, or from the phone, and the fans go back to what they
//! were.

use parking_lot::Mutex;
use serde::Serialize;
use serde_json::json;

use crate::rig::fans::FanProfile;

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuietStatus {
    pub schedule: QuietSchedule,
    /// The sleep timer, when set (unix ms).
    pub timer: Option<i64>,
    pub on: bool,
    /// Unix ms.
    pub since: Option<i64>,
    /// OmniHub can set this PC's fans (LibreHardwareMonitor or a cooler is
    /// connected); otherwise quiet mode only turns the screens off.
    pub fans: bool,
}

struct State {
    before: FanProfile,
    since: i64,
    input: Option<u32>,
    /// Started by the schedule (so the schedule's end time ends it).
    scheduled: bool,
}

/// Every night: quiet at `start`, back to normal at `end` ("23:00", "07:30").
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct QuietSchedule {
    pub enabled: bool,
    pub start: String,
    pub end: String,
}

impl Default for QuietSchedule {
    fn default() -> Self {
        QuietSchedule { enabled: false, start: "23:00".into(), end: "07:30".into() }
    }
}

/// Minutes since midnight for "HH:MM".
pub fn minutes_of(hhmm: &str) -> Option<u32> {
    let (h, m) = hhmm.trim().split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// Whether `now` (minutes since midnight) is within two minutes after `at`.
pub fn due(at: u32, now: u32) -> bool {
    (now + 1440 - at) % 1440 < 2
}

pub struct Quiet {
    state: Mutex<Option<State>>,
    file: std::path::PathBuf,
    schedule: Mutex<QuietSchedule>,
    /// Sleep timer: quiet at this time (unix ms).
    timer: Mutex<Option<i64>>,
    /// The last minute the schedule acted (so it acts once).
    acted: Mutex<Option<(String, u32)>>,
}

impl Quiet {
    pub fn new(data: &std::path::Path) -> Quiet {
        let file = data.join("quiet-schedule.json");
        let schedule = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Quiet { state: Mutex::new(None), file, schedule: Mutex::new(schedule), timer: Mutex::new(None), acted: Mutex::new(None) }
    }

    pub fn schedule(&self) -> QuietSchedule {
        self.schedule.lock().clone()
    }

    pub fn set_schedule(&self, s: QuietSchedule) -> Result<QuietSchedule, String> {
        if minutes_of(&s.start).is_none() || minutes_of(&s.end).is_none() {
            return Err("Times look like 23:00".into());
        }
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        *self.schedule.lock() = s.clone();
        Ok(s)
    }

    /// Quiet in `minutes` (None cancels).
    pub fn set_timer(&self, minutes: Option<u32>) {
        *self.timer.lock() = minutes.map(|m| chrono::Utc::now().timestamp_millis() + m.clamp(1, 600) as i64 * 60_000);
    }

    pub fn timer(&self) -> Option<i64> {
        *self.timer.lock()
    }
}

/// The tick of the last local keyboard or mouse input (Windows' own count).
pub fn last_input() -> Option<u32> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
        let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
        // SAFETY: fills the struct we pass.
        unsafe { GetLastInputInfo(&mut info) }.as_bool().then_some(info.dwTime)
    }
    #[cfg(not(windows))]
    None
}

impl crate::core::AppCore {
    pub fn quiet_status(&self) -> QuietStatus {
        let st = self.quiet.state.lock();
        let rig = self.rig.status();
        let fans = rig.fans == crate::rig::LinkState::On || !rig.cooler_devices.is_empty();
        QuietStatus { schedule: self.quiet.schedule(), timer: self.quiet.timer(), on: st.is_some(), since: st.as_ref().map(|s| s.since), fans }
    }

    /// Screens off and fans to their lowest; again turns the screens off.
    pub fn quiet_on(&self) -> Result<QuietStatus, String> {
        self.quiet_start(false)
    }

    fn quiet_start(&self, scheduled: bool) -> Result<QuietStatus, String> {
        *self.quiet.timer.lock() = None;
        {
            let mut st = self.quiet.state.lock();
            if st.is_none() {
                let before = self.rig.settings().fans.profile;
                *st = Some(State { before, since: chrono::Utc::now().timestamp_millis(), input: None, scheduled });
            }
        }
        self.rig.update(&json!({ "fans": { "profile": "lowest" } }))?;
        crate::system::power::execute(crate::system::power::PowerAction::DisplayOff).map_err(|e| e.to_string())?;
        // What counts as "someone's back": input after the screens went off.
        std::thread::sleep(std::time::Duration::from_millis(600));
        if let Some(s) = self.quiet.state.lock().as_mut() {
            s.input = last_input();
        }
        self.events.emit("quiet:changed", self.quiet_status());
        Ok(self.quiet_status())
    }

    /// Back to normal: the fans as they were, the screens on.
    pub fn quiet_off(&self) -> Result<QuietStatus, String> {
        let Some(st) = self.quiet.state.lock().take() else { return Ok(self.quiet_status()) };
        let before = serde_json::to_value(st.before).map_err(|e| e.to_string())?;
        self.rig.update(&json!({ "fans": { "profile": before } }))?;
        crate::system::awake::wake_screen();
        self.events.emit("quiet:changed", self.quiet_status());
        Ok(self.quiet_status())
    }

    /// Called every couple of seconds: the sleep timer, the schedule, and
    /// someone using the mouse or keyboard.
    pub fn quiet_check(&self) {
        let now = chrono::Utc::now().timestamp_millis();
        if self.quiet.timer().is_some_and(|t| now >= t) {
            let _ = self.quiet_start(false);
            return;
        }
        let sched = self.quiet.schedule();
        if sched.enabled {
            let local = chrono::Local::now();
            let today = local.format("%Y-%m-%d").to_string();
            let minute = local.format("%H").to_string().parse::<u32>().unwrap_or(0) * 60 + local.format("%M").to_string().parse::<u32>().unwrap_or(0);
            let once = |at: u32| {
                let mut acted = self.quiet.acted.lock();
                if acted.as_ref() == Some(&(today.clone(), at)) {
                    false
                } else {
                    *acted = Some((today.clone(), at));
                    true
                }
            };
            if let Some(start) = minutes_of(&sched.start).filter(|s| due(*s, minute)) {
                if self.quiet.state.lock().is_none() && once(start) {
                    let _ = self.quiet_start(true);
                    return;
                }
            }
            if let Some(end) = minutes_of(&sched.end).filter(|e| due(*e, minute)) {
                let scheduled = self.quiet.state.lock().as_ref().is_some_and(|s| s.scheduled);
                if scheduled && once(end) {
                    let _ = self.quiet_off();
                    return;
                }
            }
        }
        let back = {
            let st = self.quiet.state.lock();
            match (st.as_ref().and_then(|s| s.input), last_input()) {
                (Some(then), Some(now)) => now != then,
                _ => false,
            }
        };
        if back {
            let _ = self.quiet_off();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_and_schedule() {
        assert_eq!(minutes_of("23:00"), Some(1380));
        assert_eq!(minutes_of("7:05"), Some(425));
        assert_eq!(minutes_of("25:00"), None);
        assert!(due(1380, 1380) && due(1380, 1381) && !due(1380, 1382) && !due(1380, 1379));
        assert!(due(1439, 0), "just after midnight still counts");
        let t = tempfile::tempdir().unwrap();
        let q = Quiet::new(t.path());
        assert!(q.set_schedule(QuietSchedule { enabled: true, start: "nope".into(), end: "07:00".into() }).is_err());
        q.set_schedule(QuietSchedule { enabled: true, start: "22:30".into(), end: "06:45".into() }).unwrap();
        assert_eq!(Quiet::new(t.path()).schedule().start, "22:30");
        q.set_timer(Some(30));
        assert!(q.timer().is_some());
        q.set_timer(None);
        assert!(q.timer().is_none());
    }
}
