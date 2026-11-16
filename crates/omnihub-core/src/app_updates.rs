//! Keep installed programs up to date by themselves: at night (or the time
//! picked) OmniHub asks winget, Windows' own package manager, which apps
//! have updates and installs them one by one, skipping the ones you list.
//! Not while a game is boosted.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AutoUpdateSettings {
    pub enabled: bool,
    /// "HH:MM", local time.
    pub time: String,
    /// winget ids never updated automatically.
    pub skip: Vec<String>,
    pub last_run: Option<i64>,
    pub last_result: Option<String>,
}

impl Default for AutoUpdateSettings {
    fn default() -> Self {
        AutoUpdateSettings { enabled: false, time: "03:30".into(), skip: Vec::new(), last_run: None, last_result: None }
    }
}

pub struct AutoUpdates {
    file: PathBuf,
    settings: Mutex<AutoUpdateSettings>,
    running: Mutex<bool>,
}

/// Whether today's run is due: past the time, and not run since today's time.
pub fn is_due(s: &AutoUpdateSettings, now_local_min: u32, today_start: i64, now: i64) -> bool {
    let Some(at) = crate::quiet::minutes_of(&s.time) else { return false };
    if !s.enabled || now_local_min < at {
        return false;
    }
    let time_today = today_start + at as i64 * 60;
    s.last_run.is_none_or(|last| last < time_today) && now >= time_today
}

impl AutoUpdates {
    pub fn new(data: &Path) -> AutoUpdates {
        let file = data.join("app-updates.json");
        let settings = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        AutoUpdates { file, settings: Mutex::new(settings), running: Mutex::new(false) }
    }

    pub fn settings(&self) -> AutoUpdateSettings {
        self.settings.lock().clone()
    }

    fn write(&self, s: &AutoUpdateSettings) -> Result<(), String> {
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    pub fn save(&self, mut next: AutoUpdateSettings) -> Result<AutoUpdateSettings, String> {
        if crate::quiet::minutes_of(&next.time).is_none() {
            return Err("The time looks like 03:30".into());
        }
        let cur = self.settings();
        next.last_run = cur.last_run;
        next.last_result = cur.last_result;
        self.write(&next)?;
        *self.settings.lock() = next.clone();
        Ok(next)
    }

    pub fn running(&self) -> bool {
        *self.running.lock()
    }

    /// Update everything not skipped (blocking). Returns (updated, failed names).
    pub fn run(&self, now: i64) -> Result<(Vec<String>, Vec<String>), String> {
        {
            let mut r = self.running.lock();
            if *r {
                return Err("Apps are already being updated".into());
            }
            *r = true;
        }
        let skip = self.settings().skip;
        let result = crate::winget::updates().map(|list| {
            let mut done = Vec::new();
            let mut failed = Vec::new();
            for u in list.into_iter().filter(|u| !skip.iter().any(|s| s.eq_ignore_ascii_case(&u.id))) {
                match crate::winget::update(&u.id) {
                    Ok(()) => done.push(u.name),
                    Err(_) => failed.push(u.name),
                }
            }
            (done, failed)
        });
        let mut s = self.settings();
        s.last_run = Some(now);
        s.last_result = Some(match &result {
            Ok((d, f)) if d.is_empty() && f.is_empty() => "Everything was up to date".into(),
            Ok((d, f)) => format!("{} updated{}", d.len(), if f.is_empty() { String::new() } else { format!(", {} failed ({})", f.len(), f.join(", ")) }),
            Err(e) => e.clone(),
        });
        let _ = self.write(&s);
        *self.settings.lock() = s;
        *self.running.lock() = false;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_once_a_day_after_its_time() {
        let day = 1_000_000_000 - (1_000_000_000 % 86_400);
        let mut s = AutoUpdateSettings { enabled: true, time: "03:30".into(), ..Default::default() };
        assert!(!is_due(&s, 200, day, day + 200 * 60), "before 03:30");
        assert!(is_due(&s, 211, day, day + 211 * 60));
        s.last_run = Some(day + 212 * 60);
        assert!(!is_due(&s, 600, day, day + 600 * 60), "already ran today");
        assert!(is_due(&s, 300, day + 86_400, day + 86_400 + 300 * 60), "and tomorrow again");
        s.enabled = false;
        assert!(!is_due(&s, 300, day + 86_400, day + 86_400 + 300 * 60));
        let t = tempfile::tempdir().unwrap();
        let a = AutoUpdates::new(t.path());
        assert!(a.save(AutoUpdateSettings { time: "3pm".into(), ..Default::default() }).is_err());
        a.save(AutoUpdateSettings { enabled: true, time: "04:00".into(), skip: vec!["Valve.Steam".into()], ..Default::default() }).unwrap();
        assert_eq!(AutoUpdates::new(t.path()).settings().skip, vec!["Valve.Steam"]);
    }
}
