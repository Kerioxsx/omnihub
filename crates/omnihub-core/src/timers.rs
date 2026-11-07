//! Timers, alarms and the bedtime lock, on the PC and the phone.
//! - Timers count down ("pizza, 12 min") and ring on both.
//! - Alarms ring at a time on chosen days.
//! - Bedtime: a warning a few minutes before, then the PC locks, and locks
//!   again if it's used before the morning.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Timer {
    pub id: String,
    pub label: String,
    /// Unix ms when it rings.
    pub ends_at: i64,
    pub minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Alarm {
    pub id: String,
    pub label: String,
    /// "07:30"
    pub time: String,
    /// 0 = Monday … 6 = Sunday; empty = every day.
    pub days: Vec<u8>,
    pub enabled: bool,
    /// The day it last rang ("2026-11-07").
    pub fired: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Bedtime {
    pub enabled: bool,
    /// "23:30"
    pub from: String,
    /// "06:30"
    pub to: String,
    pub warn_minutes: u32,
    pub days: Vec<u8>,
}

impl Default for Bedtime {
    fn default() -> Self {
        Bedtime { enabled: false, from: "23:30".into(), to: "06:30".into(), warn_minutes: 10, days: Vec::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct TimersState {
    pub timers: Vec<Timer>,
    pub alarms: Vec<Alarm>,
    pub bedtime: Bedtime,
    /// Bedtime put off until this time (Unix ms), from the PC only.
    pub snoozed_until: i64,
}

/// What a tick asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Ring {
    Timer(String),
    Alarm(String),
    BedtimeSoon(u32),
    Lock,
}

pub struct Timers {
    file: PathBuf,
    state: Mutex<TimersState>,
    warned: Mutex<String>,
}

fn on_day(days: &[u8], weekday: u8) -> bool {
    days.is_empty() || days.contains(&weekday)
}

/// Whether `now` (minutes after midnight) falls in [from, to), across midnight too.
pub fn within(now: u32, from: u32, to: u32) -> bool {
    if from <= to {
        now >= from && now < to
    } else {
        now >= from || now < to
    }
}

impl Timers {
    pub fn new(data: &Path) -> Timers {
        let file = data.join("timers.json");
        let state = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Timers { file, state: Mutex::new(state), warned: Mutex::new(String::new()) }
    }

    pub fn get(&self) -> TimersState {
        self.state.lock().clone()
    }

    fn put(&self, s: &TimersState) {
        if let Ok(j) = serde_json::to_vec_pretty(s) {
            let _ = crate::settings::write_atomic(&self.file, &j);
        }
    }

    fn edit(&self, f: impl FnOnce(&mut TimersState)) -> TimersState {
        let mut s = self.state.lock();
        f(&mut s);
        let snap = s.clone();
        drop(s);
        self.put(&snap);
        snap
    }

    pub fn start(&self, minutes: u32, label: &str, now: i64) -> Result<TimersState, String> {
        if minutes == 0 || minutes > 24 * 60 {
            return Err("Pick from 1 minute to 24 hours".into());
        }
        if self.get().timers.len() >= 20 {
            return Err("That's a lot of timers already".into());
        }
        let label = label.trim().chars().take(40).collect::<String>();
        Ok(self.edit(|s| {
            s.timers.push(Timer { id: uuid::Uuid::new_v4().simple().to_string()[..8].to_string(), label: if label.is_empty() { format!("{minutes} min") } else { label }, ends_at: now + i64::from(minutes) * 60_000, minutes });
        }))
    }

    pub fn cancel(&self, id: &str) -> TimersState {
        self.edit(|s| s.timers.retain(|t| t.id != id))
    }

    pub fn save_alarm(&self, mut a: Alarm) -> Result<TimersState, String> {
        if crate::nightlight::minutes(&a.time).is_none() {
            return Err("Use a time like 07:30".into());
        }
        a.label = a.label.trim().chars().take(40).collect();
        a.days.retain(|d| *d < 7);
        if a.id.is_empty() {
            a.id = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
        }
        if self.get().alarms.len() >= 30 && !self.get().alarms.iter().any(|x| x.id == a.id) {
            return Err("That's a lot of alarms already".into());
        }
        Ok(self.edit(|s| match s.alarms.iter_mut().find(|x| x.id == a.id) {
            Some(x) => *x = a,
            None => s.alarms.push(a),
        }))
    }

    pub fn delete_alarm(&self, id: &str) -> TimersState {
        self.edit(|s| s.alarms.retain(|a| a.id != id))
    }

    pub fn save_bedtime(&self, mut b: Bedtime) -> Result<TimersState, String> {
        if crate::nightlight::minutes(&b.from).is_none() || crate::nightlight::minutes(&b.to).is_none() {
            return Err("Use times like 23:30".into());
        }
        b.warn_minutes = b.warn_minutes.min(60);
        b.days.retain(|d| *d < 7);
        Ok(self.edit(|s| s.bedtime = b))
    }

    /// Put bedtime off (from the PC).
    pub fn snooze(&self, minutes: u32, now: i64) -> TimersState {
        self.edit(|s| s.snoozed_until = now + i64::from(minutes.min(60)) * 60_000)
    }

    /// Every few seconds: what should ring now. `minute` is minutes after
    /// midnight here, `weekday` 0 = Monday, `idle_ms` how long since the PC
    /// was last used.
    pub fn tick(&self, now: i64, today: &str, minute: u32, weekday: u8, idle_ms: u64, locked_recently: bool) -> Vec<Ring> {
        let mut out = Vec::new();
        let mut changed = false;
        let mut s = self.state.lock();
        s.timers.retain(|t| {
            if t.ends_at <= now {
                out.push(Ring::Timer(t.label.clone()));
                changed = true;
                false
            } else {
                true
            }
        });
        for a in s.alarms.iter_mut().filter(|a| a.enabled && a.fired != today && on_day(&a.days, weekday)) {
            if crate::nightlight::minutes(&a.time).is_some_and(|m| minute >= m && minute < m + 5) {
                a.fired = today.to_string();
                out.push(Ring::Alarm(if a.label.is_empty() { format!("Alarm {}", a.time) } else { a.label.clone() }));
                changed = true;
            }
        }
        let b = s.bedtime.clone();
        let snoozed = s.snoozed_until > now;
        let snap = changed.then(|| s.clone());
        drop(s);
        if let Some(snap) = snap {
            self.put(&snap);
        }
        if b.enabled && !snoozed {
            if let (Some(from), Some(to)) = (crate::nightlight::minutes(&b.from), crate::nightlight::minutes(&b.to)) {
                // The night belongs to the day it starts on.
                let starts_today = on_day(&b.days, weekday);
                let started_yesterday = on_day(&b.days, (weekday + 6) % 7);
                let in_window = within(minute, from, to) && if minute >= from { starts_today } else { started_yesterday };
                let warn_at = (from + 1440 - b.warn_minutes) % 1440;
                if !in_window && b.warn_minutes > 0 && within(minute, warn_at, from) && starts_today {
                    let mut w = self.warned.lock();
                    if *w != today {
                        *w = today.to_string();
                        out.push(Ring::BedtimeSoon((from + 1440 - minute) % 1440));
                    }
                }
                // Lock when it starts, and again if the PC is used in the night.
                if in_window && idle_ms < 60_000 && !locked_recently {
                    out.push(Ring::Lock);
                }
            }
        }
        out
    }
}

/// How long since the keyboard or mouse was last used, ms.
pub fn idle_ms() -> u64 {
    #[cfg(windows)]
    {
        let now = unsafe { windows::Win32::System::SystemInformation::GetTickCount() };
        crate::quiet::last_input().map_or(u64::MAX, |t| u64::from(now.wrapping_sub(t)))
    }
    #[cfg(not(windows))]
    u64::MAX
}

impl crate::core::AppCore {
    /// Timers, alarms and bedtime, every few seconds.
    pub fn timers_tick(&self, last_lock: &mut i64) {
        use chrono::{Datelike, Timelike};
        let local = chrono::Local::now();
        let now = local.timestamp_millis();
        let today = local.format("%Y-%m-%d").to_string();
        let minute = local.hour() * 60 + local.minute();
        let weekday = local.weekday().num_days_from_monday() as u8;
        for r in self.timers.tick(now, &today, minute, weekday, idle_ms(), now - *last_lock < 120_000) {
            match r {
                Ring::Timer(label) => self.events.emit("timer:done", serde_json::json!({ "label": label })),
                Ring::Alarm(label) => self.events.emit("timer:alarm", serde_json::json!({ "label": label })),
                Ring::BedtimeSoon(m) => self.events.emit("timer:bedtime", serde_json::json!({ "minutes": m })),
                Ring::Lock => {
                    *last_lock = now;
                    let _ = crate::system::power::execute(crate::system::power::PowerAction::Lock);
                }
            }
        }
    }
}

// ---------- when downloads finish ----------

/// "Shut down (or sleep) when downloads finish": downloads count as done
/// when OmniHub has none running and the PC has been downloading less than
/// `QUIET_BPS` for `QUIET_MS` (so Steam, Epic and browsers count too).
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AfterDownloads {
    /// "shutdown", "sleep" or "" (off).
    pub action: String,
    /// Unix ms since it's been quiet (0: busy).
    pub quiet_since: i64,
}

pub const QUIET_BPS: u64 = 300_000;
pub const QUIET_MS: i64 = 5 * 60_000;

impl AfterDownloads {
    /// A reading every few seconds; true when it's time to act.
    pub fn sample(&mut self, now: i64, ours_running: bool, down_bps: u64) -> bool {
        if self.action.is_empty() {
            return false;
        }
        if ours_running || down_bps >= QUIET_BPS {
            self.quiet_since = 0;
            return false;
        }
        if self.quiet_since == 0 {
            self.quiet_since = now;
        }
        now - self.quiet_since >= QUIET_MS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_and_locks() {
        let t = tempfile::tempdir().unwrap();
        let x = Timers::new(t.path());
        x.start(10, "Pizza", 0).unwrap();
        assert!(x.start(0, "", 0).is_err());
        assert!(x.tick(9 * 60_000, "d", 600, 0, 999_999, false).is_empty());
        assert_eq!(x.tick(10 * 60_000, "d", 600, 0, 999_999, false), vec![Ring::Timer("Pizza".into())]);
        assert!(x.get().timers.is_empty());
        // An alarm on weekdays, once a day.
        x.save_alarm(Alarm { time: "07:30".into(), days: vec![0, 1, 2, 3, 4], enabled: true, label: "Up".into(), ..Default::default() }).unwrap();
        assert_eq!(x.tick(0, "mon", 450, 0, 999_999, false), vec![Ring::Alarm("Up".into())]);
        assert!(x.tick(0, "mon", 451, 0, 999_999, false).is_empty());
        assert!(x.tick(0, "sat", 450, 5, 999_999, false).is_empty(), "not on Saturday");
        // Bedtime 23:30–06:30 with a 10-minute warning.
        x.save_bedtime(Bedtime { enabled: true, from: "23:30".into(), to: "06:30".into(), warn_minutes: 10, days: vec![] }).unwrap();
        assert_eq!(x.tick(0, "tue", 23 * 60 + 22, 1, 0, false), vec![Ring::BedtimeSoon(8)]);
        assert!(x.tick(0, "tue", 23 * 60 + 25, 1, 0, false).is_empty(), "warned once");
        assert_eq!(x.tick(0, "tue", 23 * 60 + 31, 1, 0, false), vec![Ring::Lock]);
        assert!(x.tick(0, "tue", 23 * 60 + 31, 1, 0, true).is_empty(), "just locked");
        assert!(x.tick(0, "wed", 2 * 60, 2, 600_000, false).is_empty(), "nobody at the PC");
        assert_eq!(x.tick(0, "wed", 2 * 60, 2, 1_000, false), vec![Ring::Lock]);
        assert!(x.tick(0, "wed", 7 * 60, 2, 1_000, false).is_empty(), "morning");
        let mut a = AfterDownloads { action: "shutdown".into(), quiet_since: 0 };
        assert!(!a.sample(0, false, 5_000_000), "still downloading");
        assert!(!a.sample(10_000, false, 10_000), "quiet starts");
        assert!(!a.sample(200_000, true, 0), "our own download running");
        assert!(!a.sample(300_000, false, 0));
        assert!(a.sample(300_000 + QUIET_MS, false, 0), "quiet for 5 minutes");
        x.snooze(15, 0);
        assert!(x.tick(60_000, "wed", 2 * 60, 2, 1_000, false).is_empty(), "snoozed");
    }
}
