//! Counters: count anything with a click or a key (shiny hunts, deaths,
//! push-ups…). Each has a name, an optional picture, a step, keys that add
//! or take away (watched across Windows without being swallowed, so a game
//! still gets them: see `system::keys`), and a timer. A hunt remembers the
//! Pokémon and how the odds are worked out; finds are kept in a history.
//! Everything stays in `counters.json` and `counters/` (pictures) on this PC.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;

/// A key that changes a counter: its Windows virtual-key code and what to call it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KeyBind {
    pub vk: u32,
    pub label: String,
}

/// What a shiny hunt is after and how the odds are worked out.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Hunt {
    /// National Pokédex number.
    pub species: u16,
    pub name: String,
    /// The game's generation (2–9): 2–5 roll 1/8192, 6+ 1/4096.
    pub generation: u8,
    pub masuda: bool,
    pub charm: bool,
    /// Odds of 1 in this many instead (another method).
    pub custom_odds: Option<u32>,
    /// What's being done ("Soft resets", "Random encounters"…).
    pub method: String,
}

impl Default for Hunt {
    fn default() -> Self {
        Hunt { species: 0, name: String::new(), generation: 9, masuda: false, charm: false, custom_odds: None, method: "Random encounters".into() }
    }
}

impl Hunt {
    /// The chance of a shiny per encounter.
    pub fn chance(&self) -> f64 {
        if let Some(n) = self.custom_odds.filter(|&n| n > 0) {
            return 1.0 / n as f64;
        }
        let base = if self.generation <= 5 { 8192.0 } else { 4096.0 };
        let mut rolls = 1.0;
        if self.masuda && self.generation >= 4 {
            rolls += if self.generation == 4 { 4.0 } else { 5.0 };
        }
        if self.charm && self.generation >= 5 {
            rolls += 2.0;
        }
        1.0 - (1.0f64 - 1.0 / base).powf(rolls)
    }
}

/// Something that happens during a count ("Phase", "Wrong shiny",
/// "Fainted"), with its own key. A phase starts a new phase count while
/// the total keeps going (like other shiny counters).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct EventKind {
    pub id: String,
    pub name: String,
    pub key: Option<KeyBind>,
    pub phase: bool,
}

/// One logged event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LogEntry {
    pub at: i64,
    /// The count when it happened.
    pub count: i64,
    pub name: String,
    pub phase: bool,
}

pub const MAX_LOG: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Counter {
    pub id: String,
    pub name: String,
    pub count: i64,
    pub step: i64,
    /// The picture's file name in `counters/`.
    pub image: Option<String>,
    /// Adds the step.
    pub key: Option<KeyBind>,
    /// Takes the step away.
    pub minus_key: Option<KeyBind>,
    /// Milliseconds on the timer, not counting the stretch running now.
    pub timer_ms: u64,
    /// Unix ms when the timer started, while it runs.
    pub timer_since: Option<i64>,
    pub hunt: Option<Hunt>,
    pub created: i64,
    pub updated: i64,
    /// Events with their own keys.
    pub events: Vec<EventKind>,
    /// What happened, newest first.
    pub log: Vec<LogEntry>,
    /// The count when the current phase began, and how many phases so far.
    pub phase_base: i64,
    pub phases: u32,
    /// Counts to be told about.
    pub goals: Vec<i64>,
    /// For hunts: be told at 1×, 2×, 3×… the odds.
    pub odds_alerts: bool,
    /// Goals already told about (until the next find).
    pub alerted: Vec<String>,
    /// Earlier counts, for Undo (not kept on disk).
    #[serde(skip)]
    pub undo: Vec<i64>,
}

impl Default for Counter {
    fn default() -> Self {
        Counter {
            id: String::new(),
            name: String::new(),
            count: 0,
            step: 1,
            image: None,
            key: None,
            minus_key: None,
            timer_ms: 0,
            timer_since: None,
            hunt: None,
            created: 0,
            updated: 0,
            events: Vec::new(),
            log: Vec::new(),
            phase_base: 0,
            phases: 0,
            goals: Vec::new(),
            odds_alerts: true,
            alerted: Vec::new(),
            undo: Vec::new(),
        }
    }
}

/// A goal reached (sent as `counters:goal`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GoalHit {
    pub counter: String,
    pub name: String,
    pub count: i64,
    pub message: String,
}

impl Counter {
    /// Change the count, remembering the old one for Undo.
    fn set(&mut self, count: i64) {
        let count = count.max(0);
        if count != self.count {
            self.undo.push(self.count);
            if self.undo.len() > 50 {
                self.undo.remove(0);
            }
            self.count = count;
        }
        self.updated = now_ms();
    }

    fn log(&mut self, name: &str, phase: bool) {
        self.log.insert(0, LogEntry { at: now_ms(), count: self.count, name: name.into(), phase });
        self.log.truncate(MAX_LOG);
        if phase {
            self.phases += 1;
            self.phase_base = self.count;
        }
        self.updated = now_ms();
    }

    /// Goals this count reaches that weren't told about yet (and marks them).
    fn new_goals(&mut self) -> Vec<GoalHit> {
        let mut hits = Vec::new();
        let mut reached: Vec<(String, String)> = self.goals.iter().filter(|&&g| g > 0 && self.count >= g).map(|g| (format!("g{g}"), format!("{} reached {}", self.name, g))).collect();
        if let (true, Some(h)) = (self.odds_alerts, &self.hunt) {
            let odds = 1.0 / h.chance();
            for k in 1..=10u32 {
                if self.count as f64 >= odds * k as f64 {
                    reached.push((format!("o{k}"), if k == 1 { format!("{}: past the odds ({} encounters)", self.name, self.count) } else { format!("{}: {k}× the odds ({} encounters)", self.name, self.count) }));
                }
            }
        }
        for (tag, message) in reached {
            if !self.alerted.contains(&tag) {
                self.alerted.push(tag);
                hits.push(GoalHit { counter: self.id.clone(), name: self.name.clone(), count: self.count, message });
            }
        }
        hits
    }
}

impl Counter {
    /// The timer's total now.
    pub fn elapsed_ms(&self, now: i64) -> u64 {
        self.timer_ms + self.timer_since.map_or(0, |s| (now - s).max(0) as u64)
    }
}

/// A shiny (or anything) found: kept after the counter moves on.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub id: String,
    pub counter: String,
    pub name: String,
    pub species: Option<u16>,
    pub count: i64,
    pub timer_ms: u64,
    pub at: i64,
    /// "1/4096" and the method, for a hunt.
    pub odds: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CountersState {
    pub counters: Vec<Counter>,
    pub found: Vec<Found>,
    /// Keys count while on (they can be paused all at once).
    pub keys_on: bool,
    /// The OBS page is served (on this PC only).
    pub overlay_on: bool,
    pub overlay_port: u16,
}

/// What the page sends to make or change a counter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CounterInput {
    pub id: Option<String>,
    pub name: String,
    pub step: i64,
    pub key: Option<KeyBind>,
    pub minus_key: Option<KeyBind>,
    pub hunt: Option<Hunt>,
    pub events: Vec<EventKind>,
    pub goals: Vec<i64>,
    pub odds_alerts: bool,
}

pub struct Counters {
    file: PathBuf,
    images: PathBuf,
    state: Mutex<CountersState>,
    events: EventBus,
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

const IMAGE_TYPES: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];
const MAX_IMAGE: u64 = 10 << 20;

impl Counters {
    pub fn new(data: &Path, events: EventBus) -> Counters {
        let file = data.join("counters.json");
        let mut state: CountersState = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(CountersState { keys_on: true, ..Default::default() });
        if state.overlay_port == 0 {
            state.overlay_port = crate::counters_overlay::DEFAULT_PORT;
        }
        Counters { file, images: data.join("counters"), state: Mutex::new(state), events }
    }

    pub fn state(&self) -> CountersState {
        self.state.lock().clone()
    }

    /// Change the state, write it and tell the windows.
    fn change<T>(&self, f: impl FnOnce(&mut CountersState) -> Result<T, String>) -> Result<T, String> {
        let (out, snapshot) = {
            let mut s = self.state.lock();
            let out = f(&mut s)?;
            (out, s.clone())
        };
        let json = serde_json::to_vec_pretty(&snapshot).map_err(|e| e.to_string())?;
        crate::settings::write_atomic(&self.file, &json).map_err(|e| e.to_string())?;
        self.events.emit("counters:changed", &snapshot);
        Ok(out)
    }

    /// Every key some counter listens to (for the key watcher), if keys are on.
    pub fn watched_keys(&self) -> Vec<u32> {
        let s = self.state.lock();
        if !s.keys_on {
            return Vec::new();
        }
        s.counters.iter().flat_map(|c| [c.key.as_ref(), c.minus_key.as_ref()].into_iter().chain(c.events.iter().map(|e| e.key.as_ref()))).flatten().map(|k| k.vk).collect()
    }

    pub fn save(&self, input: CounterInput) -> Result<Counter, String> {
        let name: String = input.name.trim().chars().take(60).collect();
        if name.is_empty() {
            return Err("Give the counter a name".into());
        }
        let step = input.step.clamp(1, 1_000_000);
        let mut keys: Vec<u32> = [&input.key, &input.minus_key].into_iter().flatten().map(|k| k.vk).collect();
        keys.extend(input.events.iter().filter_map(|e| e.key.as_ref().map(|k| k.vk)));
        let n = keys.len();
        keys.sort_unstable();
        keys.dedup();
        if keys.len() != n {
            return Err("Each key can only do one thing on a counter".into());
        }
        let mut events = Vec::new();
        for e in input.events.iter().take(12) {
            let name: String = e.name.trim().chars().take(40).collect();
            if name.is_empty() {
                return Err("Give each event a name".into());
            }
            events.push(EventKind { id: if e.id.is_empty() { crate::remote::auth::random_token(8) } else { e.id.clone() }, name, key: e.key.clone(), phase: e.phase });
        }
        let mut goals: Vec<i64> = input.goals.iter().copied().filter(|&g| g > 0).take(20).collect();
        goals.sort_unstable();
        goals.dedup();
        self.change(|s| {
            let now = now_ms();
            let c = match input.id.as_deref().and_then(|id| s.counters.iter_mut().find(|c| c.id == id)) {
                Some(c) => c,
                None => {
                    s.counters.push(Counter { id: crate::remote::auth::random_token(10), created: now, ..Default::default() });
                    s.counters.last_mut().expect("just pushed")
                }
            };
            c.name = name;
            c.step = step;
            c.key = input.key;
            c.minus_key = input.minus_key;
            c.hunt = input.hunt;
            c.events = events;
            c.goals = goals;
            c.odds_alerts = input.odds_alerts;
            // Goals moved: only ones already passed stay quiet.
            let count = c.count;
            let keep: Vec<String> = c.alerted.iter().filter(|t| t.strip_prefix('g').and_then(|g| g.parse::<i64>().ok()).is_none_or(|g| count >= g)).cloned().collect();
            c.alerted = keep;
            c.updated = now;
            Ok(c.clone())
        })
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let image = self.change(|s| {
            let i = s.counters.iter().position(|c| c.id == id).ok_or("No such counter")?;
            Ok(s.counters.remove(i).image)
        })?;
        if let Some(img) = image {
            let _ = std::fs::remove_file(self.images.join(img));
        }
        Ok(())
    }

    /// Add `times` steps (negative takes away). The count never goes below 0.
    pub fn bump(&self, id: &str, times: i64) -> Result<Counter, String> {
        let (c, hits) = self.change(|s| {
            let c = s.counters.iter_mut().find(|c| c.id == id).ok_or("No such counter")?;
            let next = c.count.saturating_add(times.saturating_mul(c.step));
            c.set(next);
            let hits = c.new_goals();
            Ok((c.clone(), hits))
        })?;
        self.tell(hits);
        Ok(c)
    }

    pub fn set_count(&self, id: &str, count: i64) -> Result<Counter, String> {
        let (c, hits) = self.change(|s| {
            let c = s.counters.iter_mut().find(|c| c.id == id).ok_or("No such counter")?;
            c.set(count);
            let hits = c.new_goals();
            Ok((c.clone(), hits))
        })?;
        self.tell(hits);
        Ok(c)
    }

    /// Start or stop a counter's timer; `reset` puts it back to zero.
    pub fn timer(&self, id: &str, run: bool, reset: bool) -> Result<Counter, String> {
        self.change(|s| {
            let now = now_ms();
            let c = s.counters.iter_mut().find(|c| c.id == id).ok_or("No such counter")?;
            c.timer_ms = if reset { 0 } else { c.elapsed_ms(now) };
            c.timer_since = run.then_some(now);
            Ok(c.clone())
        })
    }

    /// A key went down: every counter on it changes. True if any did.
    pub fn on_key(&self, vk: u32) -> bool {
        let on = |k: &Option<KeyBind>| k.as_ref().is_some_and(|k| k.vk == vk);
        let hit = {
            let s = self.state.lock();
            s.keys_on && s.counters.iter().any(|c| on(&c.key) || on(&c.minus_key) || c.events.iter().any(|e| on(&e.key)))
        };
        if !hit {
            return false;
        }
        let hits = self.change(|s| {
            let mut hits = Vec::new();
            for c in &mut s.counters {
                if on(&c.key) {
                    let next = c.count.saturating_add(c.step);
                    c.set(next);
                } else if on(&c.minus_key) {
                    let next = c.count - c.step;
                    c.set(next);
                }
                if let Some(e) = c.events.iter().find(|e| on(&e.key)).cloned() {
                    c.log(&e.name, e.phase);
                }
                hits.extend(c.new_goals());
            }
            Ok(hits)
        });
        match hits {
            Ok(h) => {
                self.tell(h);
                true
            }
            Err(_) => false,
        }
    }

    pub fn set_overlay(&self, on: bool, port: u16) -> Result<(), String> {
        if port < 1024 {
            return Err("Use a port from 1024 up".into());
        }
        self.change(|s| {
            s.overlay_on = on;
            s.overlay_port = port;
            Ok(())
        })
    }

    pub fn set_keys_on(&self, on: bool) -> Result<(), String> {
        self.change(|s| {
            s.keys_on = on;
            Ok(())
        })
    }

    /// Found it: into the history. `reset` starts the counter (and timer) again.
    pub fn found(&self, id: &str, reset: bool) -> Result<Found, String> {
        self.change(|s| {
            let now = now_ms();
            let c = s.counters.iter_mut().find(|c| c.id == id).ok_or("No such counter")?;
            let odds = c.hunt.as_ref().map(|h| format!("1/{} · {}", (1.0 / h.chance()).round(), h.method));
            let f = Found {
                id: crate::remote::auth::random_token(10),
                counter: c.id.clone(),
                name: c.hunt.as_ref().map_or_else(|| c.name.clone(), |h| h.name.clone()),
                species: c.hunt.as_ref().map(|h| h.species),
                count: c.count,
                timer_ms: c.elapsed_ms(now),
                at: now,
                odds,
            };
            c.log("Found", false);
            if reset {
                c.set(0);
                c.timer_ms = 0;
                c.timer_since = c.timer_since.map(|_| now);
                c.phase_base = 0;
                c.phases = 0;
                c.alerted.clear();
            }
            c.updated = now;
            s.found.insert(0, f.clone());
            s.found.truncate(500);
            Ok(f)
        })
    }

    fn tell(&self, hits: Vec<GoalHit>) {
        for h in hits {
            self.events.emit("counters:goal", &h);
        }
    }

    /// Log one of a counter's events (its button or key).
    pub fn event(&self, id: &str, event: &str) -> Result<Counter, String> {
        self.change(|s| {
            let c = s.counters.iter_mut().find(|c| c.id == id).ok_or("No such counter")?;
            let e = c.events.iter().find(|e| e.id == event).cloned().ok_or("No such event")?;
            c.log(&e.name, e.phase);
            Ok(c.clone())
        })
    }

    /// Put back the count before the last change.
    pub fn undo(&self, id: &str) -> Result<Counter, String> {
        self.change(|s| {
            let c = s.counters.iter_mut().find(|c| c.id == id).ok_or("No such counter")?;
            let prev = c.undo.pop().ok_or("Nothing to undo")?;
            c.count = prev;
            c.updated = now_ms();
            Ok(c.clone())
        })
    }

    /// Clear a counter's event log.
    pub fn clear_log(&self, id: &str) -> Result<Counter, String> {
        self.change(|s| {
            let c = s.counters.iter_mut().find(|c| c.id == id).ok_or("No such counter")?;
            c.log.clear();
            c.phases = 0;
            c.phase_base = 0;
            Ok(c.clone())
        })
    }

    pub fn counter(&self, id: &str) -> Option<Counter> {
        self.state.lock().counters.iter().find(|c| c.id == id).cloned()
    }

    pub fn forget_found(&self, id: &str) -> Result<(), String> {
        self.change(|s| {
            s.found.retain(|f| f.id != id);
            Ok(())
        })
    }

    /// Use a picture (copied in; PNG, JPG, GIF or WebP up to 10 MB).
    pub fn set_image(&self, id: &str, src: Option<&Path>) -> Result<Counter, String> {
        let new_name = match src {
            Some(src) => {
                let ext = src.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                if !IMAGE_TYPES.contains(&ext.as_str()) {
                    return Err("Use a PNG, JPG, GIF or WebP picture".into());
                }
                if std::fs::metadata(src).map_err(|e| e.to_string())?.len() > MAX_IMAGE {
                    return Err("That picture is over 10 MB".into());
                }
                std::fs::create_dir_all(&self.images).map_err(|e| e.to_string())?;
                let name = format!("{}-{}.{ext}", id.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>(), now_ms());
                std::fs::copy(src, self.images.join(&name)).map_err(|e| e.to_string())?;
                Some(name)
            }
            None => None,
        };
        let (old, c) = self.change(|s| {
            let c = s.counters.iter_mut().find(|c| c.id == id).ok_or("No such counter")?;
            let old = std::mem::replace(&mut c.image, new_name.clone());
            c.updated = now_ms();
            Ok((old, c.clone()))
        })?;
        if let Some(old) = old {
            let _ = std::fs::remove_file(self.images.join(old));
        }
        Ok(c)
    }

    /// A counter's picture as a data URL.
    pub fn image(&self, id: &str) -> Option<String> {
        use base64::Engine;
        let name = self.state.lock().counters.iter().find(|c| c.id == id)?.image.clone()?;
        let bytes = std::fs::read(self.images.join(&name)).ok()?;
        let ext = name.rsplit('.').next().unwrap_or("png");
        let mime = match ext {
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "webp" => "image/webp",
            _ => "image/png",
        };
        Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str) -> CounterInput {
        CounterInput { name: name.into(), step: 1, ..Default::default() }
    }

    #[test]
    fn counts_keys_timer_and_finds() {
        let dir = tempfile::tempdir().unwrap();
        let c = Counters::new(dir.path(), EventBus::new());
        assert!(c.save(input("  ")).is_err());
        let a = c.save(CounterInput { key: Some(KeyBind { vk: 0x20, label: "Space".into() }), minus_key: Some(KeyBind { vk: 0x08, label: "Backspace".into() }), step: 2, ..input("Deaths") }).unwrap();
        assert!(c.save(CounterInput { key: Some(KeyBind { vk: 1, label: "a".into() }), minus_key: Some(KeyBind { vk: 1, label: "a".into() }), ..input("x") }).is_err());
        assert_eq!(c.watched_keys(), vec![0x20, 0x08]);
        assert!(c.on_key(0x20) && c.on_key(0x20));
        assert!(!c.on_key(0x41), "keys nobody listens to change nothing");
        assert_eq!(c.state().counters[0].count, 4);
        c.on_key(0x08);
        assert_eq!(c.state().counters[0].count, 2);
        c.bump(&a.id, -5).unwrap();
        assert_eq!(c.state().counters[0].count, 0, "never below zero");
        c.set_keys_on(false).unwrap();
        assert!(!c.on_key(0x20) && c.watched_keys().is_empty());
        c.set_count(&a.id, 41).unwrap();
        c.timer(&a.id, true, false).unwrap();
        assert!(c.state().counters[0].timer_since.is_some());
        let f = c.found(&a.id, true).unwrap();
        assert_eq!((f.count, f.name.as_str()), (41, "Deaths"));
        assert_eq!(c.state().counters[0].count, 0);
        assert_eq!(c.state().found.len(), 1);
        // It's all still there after a restart.
        let again = Counters::new(dir.path(), EventBus::new());
        assert_eq!(again.state().found.len(), 1);
        assert!(!again.state().keys_on);
        again.delete(&a.id).unwrap();
        assert!(again.state().counters.is_empty());
    }

    #[test]
    fn events_phases_goals_and_undo() {
        let dir = tempfile::tempdir().unwrap();
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let c = Counters::new(dir.path(), bus);
        let phase = EventKind { name: "Wrong shiny".into(), key: Some(KeyBind { vk: 0x50, label: "P".into() }), phase: true, ..Default::default() };
        let faint = EventKind { name: "Fainted".into(), ..Default::default() };
        assert!(c.save(CounterInput { key: Some(KeyBind { vk: 0x50, label: "P".into() }), events: vec![phase.clone()], ..input("x") }).is_err(), "one key, one job");
        let a = c
            .save(CounterInput { key: Some(KeyBind { vk: 0x20, label: "Space".into() }), events: vec![phase, faint], goals: vec![5, 0, 5], hunt: Some(Hunt { custom_odds: Some(4), ..Default::default() }), odds_alerts: true, ..input("Hunt") })
            .unwrap();
        assert_eq!(a.goals, vec![5]);
        assert!(a.events.iter().all(|e| !e.id.is_empty()));
        assert!(c.watched_keys().contains(&0x50));
        for _ in 0..3 {
            c.on_key(0x20);
        }
        c.on_key(0x50); // phase at 3
        let s = c.state().counters[0].clone();
        assert_eq!((s.count, s.phases, s.phase_base, s.log[0].name.as_str(), s.log[0].count), (3, 1, 3, "Wrong shiny", 3));
        c.event(&a.id, &s.events[1].id).unwrap();
        assert_eq!(c.state().counters[0].log[0].name, "Fainted");
        // Goals and odds alerts: 1/4 odds, goal 5.
        c.bump(&a.id, 2).unwrap();
        let mut told = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            if ev.topic == "counters:goal" {
                told.push(ev.payload["message"].as_str().unwrap().to_string());
            }
        }
        assert!(told.iter().any(|m| m.contains("past the odds")), "{told:?}");
        assert!(told.iter().any(|m| m.contains("reached 5")), "{told:?}");
        c.bump(&a.id, 0).unwrap();
        assert!(!std::iter::from_fn(|| rx.try_recv().ok()).any(|e| e.topic == "counters:goal" && e.payload["message"].as_str().unwrap().contains("reached 5")), "told once");
        // Undo goes back one change at a time.
        assert_eq!(c.undo(&a.id).unwrap().count, 3);
        assert_eq!(c.undo(&a.id).unwrap().count, 2);
        // A find starts the phases again.
        c.found(&a.id, true).unwrap();
        let s = c.state().counters[0].clone();
        assert_eq!((s.count, s.phases, s.log[0].name.as_str()), (0, 0, "Found"));
    }

    #[test]
    fn shiny_odds() {
        let h = |generation, masuda, charm| Hunt { generation, masuda, charm, ..Default::default() };
        let one_in = |h: Hunt| (1.0 / h.chance()).round() as u32;
        assert_eq!(one_in(h(3, false, false)), 8192);
        assert_eq!(one_in(h(9, false, false)), 4096);
        assert_eq!(one_in(h(9, false, true)), 1366); // 3 rolls ≈ 1/1365.67
        assert_eq!(one_in(h(9, true, false)), 683);
        assert_eq!(one_in(h(9, true, true)), 512);
        assert_eq!(one_in(h(4, true, false)), 1639); // 5 rolls of 1/8192
        assert_eq!(one_in(h(5, true, true)), 1024);
        assert_eq!(one_in(Hunt { custom_odds: Some(300), ..Default::default() }), 300);
    }

    #[test]
    fn pictures() {
        let dir = tempfile::tempdir().unwrap();
        let c = Counters::new(dir.path(), EventBus::new());
        let a = c.save(input("Hunt")).unwrap();
        let png = dir.path().join("p.png");
        std::fs::write(&png, b"\x89PNG fake").unwrap();
        assert!(c.set_image(&a.id, Some(&dir.path().join("x.exe"))).is_err());
        c.set_image(&a.id, Some(&png)).unwrap();
        assert!(c.image(&a.id).unwrap().starts_with("data:image/png;base64,"));
        c.set_image(&a.id, None).unwrap();
        assert!(c.image(&a.id).is_none());
        assert_eq!(std::fs::read_dir(dir.path().join("counters")).unwrap().count(), 0, "the old picture is removed");
    }
}
