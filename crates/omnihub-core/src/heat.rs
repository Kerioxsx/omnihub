//! Overheat alerts: a notification on the PC and the phone when the
//! processor or graphics card stays above a limit for 20 seconds (again at
//! most every 10 minutes). Temperatures come from LibreHardwareMonitor,
//! like the fan controls.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct HeatSettings {
    pub enabled: bool,
    pub cpu: f32,
    pub gpu: f32,
}

impl Default for HeatSettings {
    fn default() -> Self {
        HeatSettings { enabled: true, cpu: 90.0, gpu: 85.0 }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HeatAlert {
    /// "Processor" or "Graphics card".
    pub part: String,
    pub temp: f32,
    pub limit: f32,
}

const HOLD_MS: i64 = 20_000;
const AGAIN_MS: i64 = 10 * 60_000;

pub struct Heat {
    file: PathBuf,
    settings: Mutex<HeatSettings>,
    over_since: Mutex<HashMap<&'static str, i64>>,
    told_at: Mutex<HashMap<&'static str, i64>>,
}

impl Heat {
    pub fn new(data: &Path) -> Heat {
        let file = data.join("heat-alerts.json");
        let settings = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Heat { file, settings: Mutex::new(settings), over_since: Mutex::new(HashMap::new()), told_at: Mutex::new(HashMap::new()) }
    }

    pub fn settings(&self) -> HeatSettings {
        self.settings.lock().clone()
    }

    pub fn save(&self, mut s: HeatSettings) -> Result<HeatSettings, String> {
        s.cpu = s.cpu.clamp(60.0, 105.0);
        s.gpu = s.gpu.clamp(60.0, 100.0);
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        *self.settings.lock() = s.clone();
        Ok(s)
    }

    /// The alerts to send for these readings at `now` (unix ms).
    pub fn check(&self, cpu: Option<f32>, gpu: Option<f32>, now: i64) -> Vec<HeatAlert> {
        let s = self.settings();
        if !s.enabled {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (key, part, temp, limit) in [("cpu", "Processor", cpu, s.cpu), ("gpu", "Graphics card", gpu, s.gpu)] {
            let mut over = self.over_since.lock();
            match temp {
                Some(t) if t >= limit => {
                    let since = *over.entry(key).or_insert(now);
                    let mut told = self.told_at.lock();
                    if now - since >= HOLD_MS && told.get(key).is_none_or(|at| now - at >= AGAIN_MS) {
                        told.insert(key, now);
                        out.push(HeatAlert { part: part.into(), temp: t, limit });
                    }
                }
                _ => {
                    over.remove(key);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warns_after_twenty_seconds_and_not_too_often() {
        let t = tempfile::tempdir().unwrap();
        let h = Heat::new(t.path());
        assert!(h.check(Some(95.0), None, 0).is_empty(), "not yet");
        assert!(h.check(Some(95.0), None, 10_000).is_empty());
        let a = h.check(Some(96.0), Some(70.0), 21_000);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].part, "Processor");
        assert!(h.check(Some(96.0), None, 60_000).is_empty(), "once per ten minutes");
        assert!(h.check(Some(80.0), None, 70_000).is_empty(), "cooled down");
        assert!(h.check(Some(95.0), None, 80_000).is_empty(), "the 20 s start again");
        h.save(HeatSettings { enabled: false, ..Default::default() }).unwrap();
        assert!(h.check(Some(110.0), Some(110.0), 999_999_999).is_empty());
        assert!(!Heat::new(t.path()).settings().enabled);
    }
}
