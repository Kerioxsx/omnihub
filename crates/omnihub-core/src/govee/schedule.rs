//! Timers for the lights: at a time of day, on chosen days, switch them on
//! (fading in, like a sunrise), off (fading out), to a preset, or to a
//! brightness. They run while OmniHub runs (it sits in the tray), on the
//! PC's own clock.

use chrono::{Datelike, Duration, NaiveDateTime, NaiveTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TimerAction {
    #[default]
    On,
    Off,
    /// Switch to a preset (`Timer::preset`).
    Preset,
    /// Set the brightness (`Timer::brightness`).
    Brightness,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Timer {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    /// "07:30", 24-hour.
    pub at: String,
    /// 1 = Monday … 7 = Sunday; empty = every day.
    pub days: Vec<u8>,
    pub action: TimerAction,
    pub preset: String,
    pub brightness: u8,
    /// The lights (device ids); empty = all of them.
    pub lights: Vec<String>,
    /// Minutes to fade in (On) or out (Off); 0 = at once.
    pub fade: u8,
}

impl Default for Timer {
    fn default() -> Self {
        Timer { id: String::new(), name: String::new(), enabled: true, at: "07:00".into(), days: Vec::new(), action: TimerAction::On, preset: String::new(), brightness: 100, lights: Vec::new(), fade: 0 }
    }
}

pub fn parse_time(s: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(s.trim(), "%H:%M").ok()
}

impl Timer {
    fn on_day(&self, d: chrono::NaiveDate) -> bool {
        self.days.is_empty() || self.days.contains(&(d.weekday().number_from_monday() as u8))
    }
}

/// How late a timer may still fire (after sleep, the PC may wake later).
const GRACE: i64 = 2;

/// The timers whose time came after `from` and by `to` (local times).
pub fn due(timers: &[Timer], from: NaiveDateTime, to: NaiveDateTime) -> Vec<&Timer> {
    let from = from.max(to - Duration::minutes(GRACE));
    timers
        .iter()
        .filter(|t| t.enabled)
        .filter(|t| {
            let Some(at) = parse_time(&t.at) else { return false };
            let mut day = from.date();
            while day <= to.date() {
                let when = day.and_time(at);
                if when > from && when <= to && t.on_day(day) {
                    return true;
                }
                day = day.succ_opt().unwrap_or(day + Duration::days(1));
            }
            false
        })
        .collect()
}

/// Keep a timer within bounds (from the window).
pub fn sanitize(t: &mut Timer) {
    t.name = t.name.trim().chars().take(40).collect();
    if parse_time(&t.at).is_none() {
        t.at = "07:00".into();
    }
    t.days.retain(|d| (1..=7).contains(d));
    t.days.sort_unstable();
    t.days.dedup();
    t.brightness = t.brightness.clamp(1, 100);
    t.fade = t.fade.min(60);
    t.lights.truncate(64);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn timers_fire_once_on_their_days() {
        let wake = Timer { id: "w".into(), at: "07:00".into(), days: vec![1, 2, 3, 4, 5], ..Default::default() };
        let night = Timer { id: "n".into(), at: "23:30".into(), action: TimerAction::Off, ..Default::default() };
        let off = Timer { id: "x".into(), at: "07:00".into(), enabled: false, ..Default::default() };
        let all = [wake, night, off];
        // 2026-10-19 is a Monday.
        let ids = |v: Vec<&Timer>| v.into_iter().map(|t| t.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(due(&all, at("2026-10-19 06:59:59"), at("2026-10-19 07:00:00"))), ["w"]);
        assert!(due(&all, at("2026-10-19 07:00:00"), at("2026-10-19 07:00:01")).is_empty(), "not twice");
        // Saturday: the weekday alarm stays quiet.
        assert!(due(&all, at("2026-10-24 06:59:59"), at("2026-10-24 07:00:01")).is_empty());
        // Across midnight.
        assert_eq!(ids(due(&all, at("2026-10-19 23:29:30"), at("2026-10-19 23:31:00"))), ["n"]);
        // After a long sleep, an old time doesn't fire late.
        assert!(due(&all, at("2026-10-19 06:00:00"), at("2026-10-19 08:00:00")).is_empty());
        assert_eq!(ids(due(&all, at("2026-10-19 06:00:00"), at("2026-10-19 07:01:00"))), ["w"]);

        let mut t = Timer { at: "25:99".into(), days: vec![9, 3, 3, 1], brightness: 0, fade: 200, name: "  A very very very long name for a timer that goes on  ".into(), ..Default::default() };
        sanitize(&mut t);
        assert_eq!((t.at.as_str(), t.days.as_slice(), t.brightness, t.fade, t.name.len()), ("07:00", &[1u8, 3][..], 1, 60, 40));
    }
}
