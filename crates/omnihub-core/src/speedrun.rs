//! Speedrun timer: splits with your personal best and best segments
//! ("golds"), started, split, paused and reset with global keys that work
//! inside games (the same key hook as the counters). The run is shown in a
//! small window above the game, and saved when it beats your best.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Split {
    pub name: String,
    /// Time at this split in the personal best (ms from the start).
    pub pb: Option<u64>,
    /// The fastest this segment has ever been (ms).
    pub gold: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct SpeedrunSettings {
    pub game: String,
    pub category: String,
    pub splits: Vec<Split>,
    /// Windows virtual-key codes.
    pub key_split: u32,
    pub key_reset: u32,
    pub key_pause: u32,
    pub key_undo: u32,
    pub attempts: u32,
}

impl Default for SpeedrunSettings {
    fn default() -> Self {
        SpeedrunSettings {
            game: String::new(),
            category: "Any%".into(),
            splits: ["Level 1", "Level 2", "Level 3"].iter().map(|n| Split { name: (*n).into(), ..Default::default() }).collect(),
            // Numpad 1, 3, 2 and 8: out of the way of most games.
            key_split: 0x61,
            key_reset: 0x63,
            key_pause: 0x62,
            key_undo: 0x68,
            attempts: 0,
        }
    }
}

/// The run going on now.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    /// Unix ms when it (re)started counting; None when stopped or paused.
    pub since: Option<i64>,
    /// Time counted before `since` (pauses), ms.
    pub banked: u64,
    /// Split times reached so far (ms from the start).
    pub times: Vec<u64>,
    pub finished: bool,
    /// The run just finished beat the personal best.
    pub new_pb: bool,
}

impl Run {
    pub fn elapsed(&self, now: i64) -> u64 {
        self.banked + self.since.map_or(0, |s| (now - s).max(0) as u64)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpeedrunState {
    pub settings: SpeedrunSettings,
    pub run: Run,
    /// The PC's clock now (to run the timer in the page).
    pub now: i64,
}

pub enum Press {
    Split,
    Reset,
    Pause,
    Undo,
}

pub struct Speedrun {
    file: PathBuf,
    settings: Mutex<SpeedrunSettings>,
    run: Mutex<Run>,
    events: EventBus,
}

fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// A finished run's splits folded into the settings: a new personal best
/// replaces the PB times; every faster segment becomes a gold.
pub fn fold(splits: &mut [Split], times: &[u64]) -> bool {
    let mut prev = 0;
    for (s, &t) in splits.iter_mut().zip(times) {
        let seg = t.saturating_sub(prev);
        if s.gold.is_none_or(|g| seg < g) {
            s.gold = Some(seg);
        }
        prev = t;
    }
    let total = times.last().copied();
    let pb_total = splits.last().and_then(|s| s.pb);
    let better = times.len() == splits.len() && total.is_some_and(|t| pb_total.is_none_or(|p| t < p));
    if better {
        for (s, &t) in splits.iter_mut().zip(times) {
            s.pb = Some(t);
        }
    }
    better
}

impl Speedrun {
    pub fn new(data: &Path, events: EventBus) -> Speedrun {
        let file = data.join("speedrun.json");
        let settings = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Speedrun { file, settings: Mutex::new(settings), run: Mutex::new(Run::default()), events }
    }

    pub fn state(&self) -> SpeedrunState {
        SpeedrunState { settings: self.settings.lock().clone(), run: self.run.lock().clone(), now: now() }
    }

    fn write(&self, s: &SpeedrunSettings) {
        if let Ok(j) = serde_json::to_vec_pretty(s) {
            let _ = crate::settings::write_atomic(&self.file, &j);
        }
    }

    pub fn save(&self, mut s: SpeedrunSettings) -> Result<SpeedrunState, String> {
        if s.splits.is_empty() || s.splits.len() > 200 {
            return Err("Add between 1 and 200 splits".into());
        }
        for sp in s.splits.iter_mut() {
            sp.name = sp.name.trim().chars().take(60).collect();
            if sp.name.is_empty() {
                return Err("Every split needs a name".into());
            }
        }
        s.game = s.game.trim().chars().take(80).collect();
        s.category = s.category.trim().chars().take(60).collect();
        let keys = [s.key_split, s.key_reset, s.key_pause, s.key_undo];
        if keys.iter().any(|k| *k == 0 || *k > 0xFE) {
            return Err("Pick a key for each action".into());
        }
        self.write(&s);
        *self.settings.lock() = s;
        *self.run.lock() = Run::default();
        let st = self.state();
        self.events.emit("speedrun:changed", &st);
        Ok(st)
    }

    pub fn watched_keys(&self) -> Vec<u32> {
        let s = self.settings.lock();
        vec![s.key_split, s.key_reset, s.key_pause, s.key_undo]
    }

    pub fn on_key(&self, vk: u32) {
        let s = self.settings.lock().clone();
        let p = if vk == s.key_split {
            Press::Split
        } else if vk == s.key_reset {
            Press::Reset
        } else if vk == s.key_pause {
            Press::Pause
        } else if vk == s.key_undo {
            Press::Undo
        } else {
            return;
        };
        self.press(p, now());
    }

    pub fn press(&self, p: Press, at: i64) {
        let mut r = self.run.lock();
        let n = self.settings.lock().splits.len();
        match p {
            Press::Split => {
                if r.finished {
                    return;
                }
                if r.since.is_none() && r.times.is_empty() && r.banked == 0 {
                    // The first press starts the run.
                    r.since = Some(at);
                    let mut s = self.settings.lock();
                    s.attempts += 1;
                    self.write(&s);
                } else if r.since.is_some() {
                    let t = r.elapsed(at);
                    r.times.push(t);
                    if r.times.len() >= n {
                        r.banked = t;
                        r.since = None;
                        r.finished = true;
                        let mut s = self.settings.lock();
                        let times = r.times.clone();
                        r.new_pb = fold(&mut s.splits, &times);
                        self.write(&s);
                    }
                }
            }
            Press::Pause => {
                if r.finished {
                    return;
                }
                match r.since {
                    Some(_) => {
                        r.banked = r.elapsed(at);
                        r.since = None;
                    }
                    None if r.banked > 0 => r.since = Some(at),
                    None => {}
                }
            }
            Press::Undo => {
                if r.finished {
                    // Back into the run before the last split.
                    r.finished = false;
                    r.new_pb = false;
                    r.since = Some(at);
                }
                r.times.pop();
            }
            Press::Reset => {
                // Golds from the splits reached still count.
                if !r.finished && !r.times.is_empty() {
                    let mut s = self.settings.lock();
                    let times = r.times.clone();
                    let mut prev = 0;
                    for (sp, &t) in s.splits.iter_mut().zip(&times) {
                        let seg = t.saturating_sub(prev);
                        if sp.gold.is_none_or(|g| seg < g) {
                            sp.gold = Some(seg);
                        }
                        prev = t;
                    }
                    self.write(&s);
                }
                *r = Run::default();
            }
        }
        drop(r);
        self.events.emit("speedrun:changed", self.state());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_splits_and_keeps_bests() {
        let t = tempfile::tempdir().unwrap();
        let s = Speedrun::new(t.path(), EventBus::new());
        let mut set = s.state().settings;
        set.splits = vec![Split { name: "A".into(), ..Default::default() }, Split { name: "B".into(), ..Default::default() }];
        s.save(set).unwrap();
        s.press(Press::Split, 1000); // start
        s.press(Press::Split, 11_000); // A at 10 s
        s.press(Press::Pause, 12_000);
        s.press(Press::Pause, 20_000); // 8 s paused
        s.press(Press::Split, 26_000); // B at 17 s
        let st = s.state();
        assert!(st.run.finished && st.run.new_pb);
        assert_eq!(st.run.times, vec![10_000, 17_000]);
        assert_eq!(st.settings.splits[1].pb, Some(17_000));
        assert_eq!(st.settings.splits[1].gold, Some(7_000));
        assert_eq!(st.settings.attempts, 1);
        // A slower run doesn't replace the PB, but a faster segment is a gold.
        s.press(Press::Reset, 30_000);
        s.press(Press::Split, 40_000);
        s.press(Press::Split, 52_000); // A 12 s
        s.press(Press::Split, 57_000); // B segment 5 s (gold), total 17 s (not faster)
        let st = s.state();
        assert!(!st.run.new_pb);
        assert_eq!((st.settings.splits[1].pb, st.settings.splits[1].gold), (Some(17_000), Some(5_000)));
        // Saved for next time.
        assert_eq!(Speedrun::new(t.path(), EventBus::new()).state().settings.splits[1].gold, Some(5_000));
        // Undo after finishing goes back into the run.
        s.press(Press::Undo, 58_000);
        assert!(!s.state().run.finished);
        assert_eq!(s.state().run.times.len(), 1);
    }
}
