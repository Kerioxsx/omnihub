//! Night light on a schedule: from evening to morning every screen turns
//! warmer (and optionally dimmer), fading in and out over a while, and the
//! monitors' own brightness can change for the night too (put back in the
//! morning). "Now" turns it on or off until the next change of schedule.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NightSettings {
    pub enabled: bool,
    /// "21:30"
    pub from: String,
    /// "07:00"
    pub to: String,
    /// Warmth at night, 1900–6500 K (6500 = as is).
    pub kelvin: u32,
    /// The picture's brightness at night (30–100 %, through the colours).
    pub dim: u8,
    /// Minutes to fade in and out.
    pub fade: u32,
    /// The monitors' own brightness at night (DDC/CI), if set.
    pub screen_brightness: Option<u8>,
}

impl Default for NightSettings {
    fn default() -> Self {
        NightSettings { enabled: false, from: "21:30".into(), to: "07:00".into(), kelvin: 3400, dim: 100, fade: 30, screen_brightness: None }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NightStatus {
    /// 0 = day, 1 = full night.
    pub level: f32,
    pub kelvin: u32,
    /// Turned on or off by hand until the schedule next changes.
    pub manual: Option<bool>,
}

/// "21:30" → minutes after midnight.
pub fn minutes(hhmm: &str) -> Option<u32> {
    let (h, m) = hhmm.trim().split_once(':')?;
    let (h, m) = (h.parse::<u32>().ok()?, m.parse::<u32>().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// How much night it is at `now` (minutes after midnight), fading over `fade` minutes.
pub fn level_at(now: u32, from: u32, to: u32, fade: u32) -> f32 {
    let day = 1440;
    let since = |a: u32| (now + day - a) % day;
    let length = (to + day - from) % day;
    if length == 0 {
        return 0.0;
    }
    let into = since(from);
    if into >= length {
        return 0.0;
    }
    let fade = fade.min(length / 2).max(1) as f32;
    let left = (length - into) as f32;
    (into as f32 / fade).min(left / fade).min(1.0)
}

pub struct NightLight {
    file: PathBuf,
    settings: Mutex<NightSettings>,
    manual: Mutex<Option<(bool, bool)>>,
    applied: Mutex<Option<(u32, u8)>>,
    /// Monitors' brightness before the night: (id, value).
    before: Mutex<Option<Vec<(String, u8)>>>,
}

impl NightLight {
    pub fn new(data: &Path) -> NightLight {
        let file = data.join("nightlight.json");
        let settings = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        NightLight { file, settings: Mutex::new(settings), manual: Mutex::new(None), applied: Mutex::new(None), before: Mutex::new(None) }
    }

    pub fn settings(&self) -> NightSettings {
        self.settings.lock().clone()
    }

    pub fn save(&self, mut s: NightSettings) -> Result<NightSettings, String> {
        if minutes(&s.from).is_none() || minutes(&s.to).is_none() {
            return Err("Use times like 21:30".into());
        }
        s.kelvin = s.kelvin.clamp(1900, 6500);
        s.dim = s.dim.clamp(30, 100);
        s.fade = s.fade.min(120);
        s.screen_brightness = s.screen_brightness.map(|b| b.min(100));
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        *self.settings.lock() = s.clone();
        Ok(s)
    }

    /// On or off by hand (None: follow the schedule again).
    pub fn set_manual(&self, on: Option<bool>, now: u32) {
        let s = self.settings();
        let scheduled = s.enabled && level_at(now, minutes(&s.from).unwrap_or(0), minutes(&s.to).unwrap_or(0), s.fade) > 0.0;
        *self.manual.lock() = on.map(|o| (o, scheduled));
    }

    pub fn status(&self, now: u32) -> NightStatus {
        let (level, kelvin) = self.target(now);
        NightStatus { level, kelvin, manual: self.manual.lock().map(|m| m.0) }
    }

    fn target(&self, now: u32) -> (f32, u32) {
        let s = self.settings();
        let scheduled = if s.enabled { level_at(now, minutes(&s.from).unwrap_or(0), minutes(&s.to).unwrap_or(0), s.fade) } else { 0.0 };
        let mut m = self.manual.lock();
        // A hand-made choice lasts until the schedule itself changes over.
        if let Some((_, was)) = *m {
            if was != (scheduled > 0.0) {
                *m = None;
            }
        }
        let level = match *m {
            Some((true, _)) => 1.0,
            Some((false, _)) => 0.0,
            None => scheduled,
        };
        (level, (6500.0 - (6500.0 - s.kelvin as f32) * level).round() as u32)
    }

    /// Called every minute: the colours to set now, if they changed (or `force`).
    pub fn tick(&self, now: u32, force: bool) -> Option<(u32, f32)> {
        let s = self.settings();
        let (level, kelvin) = self.target(now);
        let dim = (100.0 - (100.0 - s.dim as f32) * level).round() as u8;
        let mut a = self.applied.lock();
        if !force && *a == Some((kelvin, dim)) {
            return None;
        }
        // Nothing to do if it was never changed and it's day.
        if a.is_none() && level == 0.0 && !force {
            *a = Some((kelvin, dim));
            return None;
        }
        *a = Some((kelvin, dim));
        Some((kelvin, dim as f32 / 100.0))
    }

    /// The monitors' brightness for the night, and back in the morning: what to set.
    pub fn brightness_plan(&self, now: u32, current: &[(String, Option<u8>)]) -> Vec<(String, u8)> {
        let s = self.settings();
        let (level, _) = self.target(now);
        let mut before = self.before.lock();
        match (s.screen_brightness, level >= 1.0, before.is_some()) {
            (Some(night), true, false) => {
                let saved: Vec<(String, u8)> = current.iter().filter_map(|(id, b)| b.map(|b| (id.clone(), b))).collect();
                let plan = saved.iter().map(|(id, _)| (id.clone(), night)).collect();
                *before = Some(saved);
                plan
            }
            (_, false, true) if level == 0.0 => before.take().unwrap_or_default(),
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fades_over_the_night() {
        let (from, to) = (minutes("21:00").unwrap(), minutes("07:00").unwrap());
        assert_eq!(level_at(minutes("12:00").unwrap(), from, to, 30), 0.0);
        assert_eq!(level_at(minutes("21:15").unwrap(), from, to, 30), 0.5);
        assert_eq!(level_at(minutes("02:00").unwrap(), from, to, 30), 1.0);
        assert_eq!(level_at(minutes("06:45").unwrap(), from, to, 30), 0.5);
        assert_eq!(level_at(minutes("07:00").unwrap(), from, to, 30), 0.0);
        assert!(minutes("25:00").is_none());

        let t = tempfile::tempdir().unwrap();
        let n = NightLight::new(t.path());
        n.save(NightSettings { enabled: true, from: "21:00".into(), to: "07:00".into(), kelvin: 3000, dim: 80, fade: 30, screen_brightness: Some(20) }).unwrap();
        let noon = minutes("12:00").unwrap();
        assert_eq!(n.tick(noon, false), None, "day: nothing to change");
        let night = minutes("01:00").unwrap();
        assert_eq!(n.tick(night, false), Some((3000, 0.8)));
        assert_eq!(n.tick(night, false), None, "already set");
        // Monitors dimmed at night and put back in the morning.
        let cur = vec![("m1".to_string(), Some(70u8)), ("tv".to_string(), None)];
        assert_eq!(n.brightness_plan(night, &cur), vec![("m1".to_string(), 20)]);
        assert!(n.brightness_plan(night, &cur).is_empty());
        assert_eq!(n.brightness_plan(noon, &cur), vec![("m1".to_string(), 70)]);
        // Off by hand until the schedule changes over.
        n.set_manual(Some(false), night);
        assert_eq!(n.status(night).level, 0.0);
        assert_eq!(n.status(noon).manual, None, "the morning ends the hand-made choice");
    }
}
