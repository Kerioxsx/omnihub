//! Music-reactive visuals: the sound the PC plays, analysed into light.
//!
//! [`VisualHub`] listens only while something shows the visuals: each view
//! (the Music page, the full-screen player, the desktop glow) holds a lease
//! and renews it every few seconds; when the last lease runs out, capture
//! stops. While listening it sends `audio:frame` sixty times a second with
//! the features from [`analyzer`] (fewer when the PC is silent, none once
//! everything has settled), and `audio:status` when that changes.
//!
//! Nothing is recorded: samples go through a 2048-sample window in memory
//! and only the features (levels, bands, beats) leave this module.

pub mod analyzer;
pub mod fft;
pub mod source;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;
use analyzer::{Analyzer, Features, Tuning};
use source::Source;

/// A view stops holding after this long without renewing.
const LEASE: Duration = Duration::from_secs(10);
const FRAME: Duration = Duration::from_micros(16_667);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureState {
    /// Nothing is showing the visuals.
    Off,
    /// Turned off in settings.
    Disabled,
    Starting,
    Listening,
    /// No sound can be captured (see `error`); the visuals run on their own.
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VisualStatus {
    pub state: CaptureState,
    /// What is listened to.
    pub source: Option<String>,
    pub sample_rate: u32,
    pub error: Option<String>,
}

impl Default for VisualStatus {
    fn default() -> Self {
        VisualStatus { state: CaptureState::Off, source: None, sample_rate: 0, error: None }
    }
}

/// One frame as sent to the views (rounded to keep it small).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    /// PC time, Unix ms.
    pub t: i64,
    #[serde(flatten)]
    pub features: Features,
}

fn rounded(mut f: Features) -> Features {
    let r = |v: &mut f32| *v = (*v * 1000.0).round() / 1000.0;
    for v in [&mut f.level, &mut f.low, &mut f.mid, &mut f.high, &mut f.beat, &mut f.flux] {
        r(v);
    }
    f.bands.iter_mut().for_each(r);
    f
}

pub struct VisualHub {
    events: EventBus,
    leases: Mutex<HashMap<String, Instant>>,
    running: AtomicBool,
    enabled: AtomicBool,
    status: Mutex<VisualStatus>,
    tuning: Mutex<Tuning>,
    fake: bool,
    /// The pretend player's state, for the test signal.
    fake_playing: Arc<AtomicBool>,
    fake_track: Arc<AtomicU32>,
}

impl VisualHub {
    pub fn new(events: EventBus, fake: bool) -> Arc<VisualHub> {
        Arc::new(VisualHub {
            events,
            leases: Mutex::new(HashMap::new()),
            running: AtomicBool::new(false),
            enabled: AtomicBool::new(true),
            status: Mutex::new(VisualStatus::default()),
            tuning: Mutex::new(Tuning::default()),
            fake,
            fake_playing: Arc::new(AtomicBool::new(true)),
            fake_track: Arc::new(AtomicU32::new(0)),
        })
    }

    pub fn status(&self) -> VisualStatus {
        self.status.lock().clone()
    }

    fn set_status(&self, s: VisualStatus) {
        let changed = {
            let mut cur = self.status.lock();
            let changed = *cur != s;
            *cur = s.clone();
            changed
        };
        if changed {
            self.events.emit("audio:status", &s);
        }
    }

    /// A view shows the visuals (call again every few seconds to keep it).
    pub fn hold(self: &Arc<Self>, who: &str) -> VisualStatus {
        if !self.enabled.load(Ordering::Relaxed) {
            return self.status();
        }
        self.leases.lock().insert(who.to_string(), Instant::now());
        if !self.running.swap(true, Ordering::SeqCst) {
            self.set_status(VisualStatus { state: CaptureState::Starting, ..Default::default() });
            let hub = self.clone();
            if std::thread::Builder::new().name("audio-visuals".into()).spawn(move || hub.run()).is_err() {
                self.running.store(false, Ordering::SeqCst);
            }
        }
        self.status()
    }

    /// A view no longer shows the visuals.
    pub fn release(&self, who: &str) {
        self.leases.lock().remove(who);
    }

    /// Listening to the PC's sound is allowed (settings).
    pub fn set_enabled(&self, on: bool) {
        self.enabled.store(on, Ordering::Relaxed);
        if !on {
            self.leases.lock().clear();
            self.set_status(VisualStatus { state: CaptureState::Disabled, ..Default::default() });
        } else if self.status().state == CaptureState::Disabled {
            self.set_status(VisualStatus::default());
        }
    }

    pub fn set_tuning(&self, t: Tuning) {
        *self.tuning.lock() = t;
    }

    fn open(&self) -> Result<Box<dyn Source>, String> {
        if self.fake {
            return Ok(Box::new(source::Synth::new(self.fake_playing.clone(), self.fake_track.clone())));
        }
        #[cfg(windows)]
        {
            source::Loopback::open().map(|l| Box::new(l) as Box<dyn Source>)
        }
        #[cfg(not(windows))]
        {
            Err("Listening to the PC's sound needs Windows.".into())
        }
    }

    fn run(self: Arc<Self>) {
        let mut rx = self.events.subscribe();
        let mut src: Option<Box<dyn Source>> = None;
        let mut analyzer: Option<Analyzer> = None;
        let mut retry_at = Instant::now();
        let mut track = String::new();
        let mut buf = Vec::with_capacity(8192);
        let mut last = Instant::now();
        let mut last_data = Instant::now();
        let mut last_emit = Instant::now() - Duration::from_secs(1);
        let mut settled = false;
        loop {
            // Leases.
            {
                let mut l = self.leases.lock();
                l.retain(|_, at| at.elapsed() < LEASE);
                if l.is_empty() {
                    break;
                }
            }
            // Track changes reset the analysis; the pretend player's state drives the test signal.
            while let Ok(ev) = rx.try_recv() {
                if ev.topic != "media:state" {
                    continue;
                }
                let st = &ev.payload["state"];
                let key = st["key"].as_str().unwrap_or("").to_string();
                self.fake_playing.store(st["playing"].as_bool().unwrap_or(false), Ordering::Relaxed);
                if key != track {
                    if !track.is_empty() {
                        self.fake_track.fetch_add(1, Ordering::Relaxed);
                        if let Some(a) = analyzer.as_mut() {
                            a.reset();
                        }
                    }
                    track = key;
                }
            }
            // (Re)open the source.
            if src.as_mut().is_some_and(|s| s.stale()) {
                src = None;
                retry_at = Instant::now();
            }
            if src.is_none() && Instant::now() >= retry_at {
                match self.open() {
                    Ok(s) => {
                        let rate = s.rate();
                        if analyzer.as_ref().map(|a| a.sample_rate()) != Some(rate) {
                            analyzer = Some(Analyzer::new(rate));
                        }
                        self.set_status(VisualStatus { state: CaptureState::Listening, source: Some(s.name()), sample_rate: rate, error: None });
                        src = Some(s);
                        last_data = Instant::now();
                    }
                    Err(e) => {
                        tracing::info!("visuals: no sound to listen to: {e}");
                        self.set_status(VisualStatus { state: CaptureState::Unavailable, source: None, sample_rate: 0, error: Some(e) });
                        retry_at = Instant::now() + Duration::from_secs(5);
                    }
                }
            }
            let dt = last.elapsed().as_secs_f32();
            last = Instant::now();
            if let (Some(s), Some(a)) = (src.as_mut(), analyzer.as_mut()) {
                buf.clear();
                match s.read(&mut buf) {
                    Ok(()) => {
                        if buf.is_empty() {
                            // Loopback delivers nothing while nothing plays: that is silence.
                            if last_data.elapsed() > Duration::from_millis(80) {
                                buf.resize((a.sample_rate() as f32 * dt) as usize, 0.0);
                            }
                        } else {
                            last_data = Instant::now();
                        }
                        a.push(&buf);
                    }
                    Err(e) => {
                        tracing::info!("visuals: capture stopped: {e}");
                        src = None;
                        retry_at = Instant::now() + Duration::from_secs(1);
                        self.set_status(VisualStatus { state: CaptureState::Unavailable, source: None, sample_rate: 0, error: Some(e) });
                    }
                }
                a.set_tuning(*self.tuning.lock());
                let f = a.update(dt);
                let quiet = !f.active && f.level < 0.002 && f.low < 0.002 && f.beat < 0.002;
                // 60 a second while there is something to show; a heartbeat when silent.
                let due = !quiet || (last_emit.elapsed() > Duration::from_millis(500) && !settled);
                if due {
                    self.events.emit("audio:frame", Frame { t: crate::media::now_ms(), features: rounded(f.clone()) });
                    last_emit = Instant::now();
                    settled = quiet;
                }
            }
            std::thread::sleep(if src.is_some() { FRAME } else { Duration::from_millis(100) });
        }
        drop(src);
        self.running.store(false, Ordering::SeqCst);
        if self.enabled.load(Ordering::Relaxed) {
            self.set_status(VisualStatus::default());
        }
        // A view may have asked in the moment before we stopped.
        if !self.leases.lock().is_empty() && !self.running.swap(true, Ordering::SeqCst) {
            let hub = self.clone();
            let _ = std::thread::Builder::new().name("audio-visuals".into()).spawn(move || hub.run());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait(what: &str, limit: Duration, mut f: impl FnMut() -> bool) {
        let t0 = Instant::now();
        while !f() {
            assert!(t0.elapsed() < limit, "timed out: {what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn leases_start_and_stop_capture() {
        let events = EventBus::new();
        let mut rx = events.subscribe();
        let hub = VisualHub::new(events.clone(), true);
        assert_eq!(hub.status().state, CaptureState::Off);
        hub.hold("page");
        events.emit("media:state", serde_json::json!({ "state": { "key": "a", "playing": true } }));
        // Frames arrive and follow the test signal.
        let mut loud = false;
        wait("a loud frame", Duration::from_secs(4), || {
            while let Ok(ev) = rx.try_recv() {
                if ev.topic == "audio:frame" && ev.payload["active"].as_bool() == Some(true) && ev.payload["low"].as_f64().unwrap_or(0.0) > 0.3 {
                    loud = true;
                }
            }
            loud
        });
        assert_eq!(hub.status().state, CaptureState::Listening);
        assert_eq!(hub.status().sample_rate, 48_000);

        // Paused: the frames go quiet.
        events.emit("media:state", serde_json::json!({ "state": { "key": "a", "playing": false } }));
        let mut quiet = false;
        wait("silence", Duration::from_secs(4), || {
            while let Ok(ev) = rx.try_recv() {
                if ev.topic == "audio:frame" {
                    quiet = ev.payload["active"].as_bool() == Some(false) && ev.payload["low"].as_f64().unwrap_or(1.0) < 0.05;
                }
            }
            quiet
        });

        // Turned off in settings: capture stops at once.
        hub.set_enabled(false);
        wait("capture to stop", Duration::from_secs(2), || !hub.running.load(Ordering::SeqCst));
        assert_eq!(hub.status().state, CaptureState::Disabled);
        assert_eq!(hub.hold("page").state, CaptureState::Disabled, "no capture while disabled");
        hub.set_enabled(true);
        assert_eq!(hub.status().state, CaptureState::Off);

        // Released: the thread ends.
        hub.hold("page");
        hub.release("page");
        wait("the thread to end", Duration::from_secs(2), || !hub.running.load(Ordering::SeqCst));
        assert_eq!(hub.status().state, CaptureState::Off);
    }

    #[cfg(not(windows))]
    #[test]
    fn no_sound_source_is_reported() {
        let hub = VisualHub::new(EventBus::new(), false);
        hub.hold("page");
        wait("unavailable", Duration::from_secs(2), || hub.status().state == CaptureState::Unavailable);
        assert!(hub.status().error.unwrap().contains("Windows"));
        hub.release("page");
    }
}
