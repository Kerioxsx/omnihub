//! Music features from the sound the PC plays: the overall level, bass,
//! mids, treble, a 16-band spectrum, how much the spectrum changes (flux) and
//! beats. Every value is normalised to 0–1 against the music's own recent
//! loudness, and smoothed with separate attack and release times, so lights
//! follow the music without flickering.
//!
//! Beats are onsets in the bass-weighted spectral flux that stand out from
//! the last second or so (mean + k·σ), with hysteresis: after a beat, the
//! flux has to fall back before the next one counts. A sustained note makes
//! one onset, not a stream of them.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use super::fft::Fft;

/// Samples per analysis window (about 43 ms at 48 kHz).
pub const WINDOW: usize = 2048;
pub const BANDS: usize = 16;
/// Below this RMS (about −70 dBFS) the PC is silent.
const SILENCE: f32 = 3e-4;
/// How long the beat detector looks back, seconds.
const HISTORY_S: f32 = 1.2;

/// How strongly the features follow the music.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Tuning {
    /// 0.2–3: overall response, and how easily beats count.
    pub sensitivity: f32,
    /// 0–2: weight of the bass.
    pub bass: f32,
    /// 0–1: longer release (smoother) as it grows.
    pub smoothing: f32,
}

impl Default for Tuning {
    fn default() -> Self {
        Tuning { sensitivity: 1.0, bass: 1.0, smoothing: 0.5 }
    }
}

/// One analysis frame, each value 0–1 unless noted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Features {
    pub level: f32,
    pub low: f32,
    pub mid: f32,
    pub high: f32,
    /// Jumps on a beat and fades out.
    pub beat: f32,
    /// How much the spectrum is changing.
    pub flux: f32,
    pub bands: [f32; BANDS],
    /// Sound is playing.
    pub active: bool,
    /// Beats so far (a change means a new beat).
    pub beats: u32,
    /// Tempo, when the beats are steady.
    pub bpm: Option<f32>,
}

/// Peak follower: normalises a value against its recent maximum, never
/// below `floor` (so near-silence is not blown up to full scale).
#[derive(Debug, Clone, Copy)]
struct Agc {
    peak: f32,
    floor: f32,
}

impl Agc {
    const fn new(floor: f32) -> Agc {
        Agc { peak: floor, floor }
    }

    fn norm(&mut self, x: f32, dt: f32) -> f32 {
        // Half-life of about six seconds: loud passages set the scale for a while.
        self.peak = (self.peak * 0.5f32.powf(dt / 6.0)).max(x).max(self.floor);
        x / self.peak
    }
}

/// One-pole follower with separate attack and release time constants.
fn follow(cur: f32, target: f32, dt: f32, attack: f32, release: f32) -> f32 {
    let tau = if target > cur { attack } else { release };
    cur + (target - cur) * (1.0 - (-dt / tau.max(1e-4)).exp())
}

/// Compress a normalised value into 0–1 (1 → about 0.89).
fn soft(x: f32) -> f32 {
    1.0 - (-2.2 * x.max(0.0)).exp()
}

pub struct Analyzer {
    rate: f32,
    fft: Fft,
    hann: Vec<f32>,
    ring: Vec<f32>,
    pos: usize,
    re: Vec<f32>,
    im: Vec<f32>,
    prev: Vec<f32>,
    /// Bin ranges: bass, mids, treble, and the spectrum bands.
    low: (usize, usize),
    mid: (usize, usize),
    high: (usize, usize),
    flux_top: usize,
    bass_top: usize,
    band_bins: [(usize, usize); BANDS],
    agc_level: Agc,
    agc_low: Agc,
    agc_mid: Agc,
    agc_high: Agc,
    agc_bands: [Agc; BANDS],
    agc_flux: Agc,
    out: Features,
    /// (time, flux) for the beat threshold.
    history: VecDeque<(f32, f32)>,
    armed: bool,
    last_beat: f32,
    beat_times: VecDeque<f32>,
    silent_for: f32,
    clock: f32,
    tuning: Tuning,
}

impl Analyzer {
    pub fn new(rate: u32) -> Analyzer {
        let rate = rate.max(8_000) as f32;
        let bin = |hz: f32| ((hz * WINDOW as f32 / rate).round() as usize).clamp(1, WINDOW / 2 - 1);
        let mut band_bins = [(0, 0); BANDS];
        let (lo, hi) = (40.0f32, 16_000.0f32.min(rate * 0.45));
        for (i, b) in band_bins.iter_mut().enumerate() {
            let f0 = lo * (hi / lo).powf(i as f32 / BANDS as f32);
            let f1 = lo * (hi / lo).powf((i + 1) as f32 / BANDS as f32);
            let (a, z) = (bin(f0), bin(f1));
            *b = (a, z.max(a + 1));
        }
        Analyzer {
            rate,
            fft: Fft::new(WINDOW),
            hann: (0..WINDOW).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / WINDOW as f32).cos()).collect(),
            ring: vec![0.0; WINDOW],
            pos: 0,
            re: vec![0.0; WINDOW],
            im: vec![0.0; WINDOW],
            prev: vec![0.0; WINDOW / 2],
            low: (bin(25.0), bin(160.0)),
            mid: (bin(160.0), bin(2_500.0)),
            high: (bin(2_500.0), bin(14_000.0f32.min(rate * 0.45))),
            flux_top: bin(5_000.0),
            bass_top: bin(200.0),
            band_bins,
            agc_level: Agc::new(0.004),
            agc_low: Agc::new(0.006),
            agc_mid: Agc::new(0.004),
            agc_high: Agc::new(0.002),
            agc_bands: [Agc::new(0.002); BANDS],
            agc_flux: Agc::new(0.05),
            out: Features::default(),
            history: VecDeque::new(),
            armed: true,
            last_beat: -10.0,
            beat_times: VecDeque::new(),
            silent_for: 10.0,
            clock: 0.0,
            tuning: Tuning::default(),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.rate as u32
    }

    pub fn set_tuning(&mut self, t: Tuning) {
        self.tuning = Tuning { sensitivity: t.sensitivity.clamp(0.2, 3.0), bass: t.bass.clamp(0.0, 2.0), smoothing: t.smoothing.clamp(0.0, 1.0) };
    }

    /// A new track: forget the loudness scale, the beat history and the tempo.
    pub fn reset(&mut self) {
        for a in [&mut self.agc_level, &mut self.agc_low, &mut self.agc_mid, &mut self.agc_high, &mut self.agc_flux] {
            a.peak = a.floor;
        }
        for a in &mut self.agc_bands {
            a.peak = a.floor;
        }
        self.history.clear();
        self.beat_times.clear();
        self.out.bpm = None;
        self.armed = true;
    }

    /// Mono samples, oldest first.
    pub fn push(&mut self, samples: &[f32]) {
        for &s in samples {
            self.ring[self.pos] = if s.is_finite() { s } else { 0.0 };
            self.pos = (self.pos + 1) % WINDOW;
        }
    }

    pub fn features(&self) -> &Features {
        &self.out
    }

    /// Analyse the latest window; `dt` is the time since the last frame.
    pub fn update(&mut self, dt: f32) -> &Features {
        let dt = dt.clamp(1e-3, 0.5);
        self.clock += dt;
        let s = self.tuning.smoothing;
        let (atk, rel) = (0.6 + 0.8 * s, 0.5 + 1.5 * s);

        // Time-domain level of the window.
        let mut sum = 0.0f32;
        for i in 0..WINDOW {
            let v = self.ring[(self.pos + i) % WINDOW];
            sum += v * v;
            self.re[i] = v * self.hann[i];
            self.im[i] = 0.0;
        }
        let rms = (sum / WINDOW as f32).sqrt();
        self.silent_for = if rms < SILENCE { self.silent_for + dt } else { 0.0 };
        self.out.active = self.silent_for < 0.6;

        let o = &mut self.out;
        if !o.active {
            // Silent: no spectrum to compute, everything settles to zero.
            for v in [&mut o.level, &mut o.low, &mut o.mid, &mut o.high, &mut o.flux, &mut o.beat] {
                *v = follow(*v, 0.0, dt, 0.05, 0.35 * rel);
            }
            for b in &mut o.bands {
                *b = follow(*b, 0.0, dt, 0.05, 0.3 * rel);
            }
            self.prev.iter_mut().for_each(|p| *p = 0.0);
            return &self.out;
        }

        self.fft.run(&mut self.re, &mut self.im);
        // |X| / (N/4): a full-scale sine reads about 1 (Hann gain 0.5).
        let scale = 4.0 / WINDOW as f32;
        let mag = |re: &[f32], im: &[f32], k: usize| re[k].hypot(im[k]) * scale;
        let energy = |re: &[f32], im: &[f32], (a, z): (usize, usize)| (a..z).map(|k| mag(re, im, k).powi(2)).sum::<f32>().sqrt();

        let low = energy(&self.re, &self.im, self.low);
        let mid = energy(&self.re, &self.im, self.mid);
        let high = energy(&self.re, &self.im, self.high);

        // Bass-weighted spectral flux on log magnitudes.
        let mut flux = 0.0;
        for k in 1..self.flux_top {
            let m = (1.0 + 400.0 * mag(&self.re, &self.im, k)).ln();
            let rise = (m - self.prev[k]).max(0.0);
            flux += if k < self.bass_top { rise * (1.0 + 2.0 * self.tuning.bass) } else { rise };
            self.prev[k] = m;
        }
        flux /= self.flux_top as f32;

        let sens = self.tuning.sensitivity;
        let level_t = soft(self.agc_level.norm(rms, dt) * sens);
        let low_t = soft(self.agc_low.norm(low, dt) * sens * (0.4 + 0.6 * self.tuning.bass));
        let mid_t = soft(self.agc_mid.norm(mid, dt) * sens);
        let high_t = soft(self.agc_high.norm(high, dt) * sens);
        let flux_t = soft(self.agc_flux.norm(flux, dt) * sens * 0.8);

        let o = &mut self.out;
        o.level = follow(o.level, level_t, dt, 0.04 * atk, 0.45 * rel);
        o.low = follow(o.low, low_t, dt, 0.02 * atk, 0.26 * rel);
        o.mid = follow(o.mid, mid_t, dt, 0.035 * atk, 0.32 * rel);
        o.high = follow(o.high, high_t, dt, 0.012 * atk, 0.15 * rel);
        o.flux = follow(o.flux, flux_t, dt, 0.015 * atk, 0.2 * rel);
        for (i, &(a, z)) in self.band_bins.iter().enumerate() {
            let e = energy(&self.re, &self.im, (a, z));
            let t = soft(self.agc_bands[i].norm(e, dt) * sens);
            o.bands[i] = follow(o.bands[i], t, dt, 0.025 * atk, 0.25 * rel);
        }

        // Beats: flux standing out from the last second, with hysteresis.
        let now = self.clock;
        self.history.push_back((now, flux));
        while self.history.front().is_some_and(|(t, _)| now - t > HISTORY_S) {
            self.history.pop_front();
        }
        let n = self.history.len() as f32;
        let mean = self.history.iter().map(|h| h.1).sum::<f32>() / n;
        let sd = (self.history.iter().map(|h| (h.1 - mean).powi(2)).sum::<f32>() / n).sqrt();
        let k = 1.5 / sens.clamp(0.5, 2.0);
        let threshold = mean + k * sd;
        let mut beat = o.beat * (-dt / (0.18 * rel)).exp();
        if self.armed && n >= 20.0 && flux > threshold && flux > mean * 1.3 + 0.01 && now - self.last_beat > 0.25 {
            self.armed = false;
            self.last_beat = now;
            o.beats = o.beats.wrapping_add(1);
            let strength = 0.55 + 0.45 * ((flux - threshold) / threshold.max(1e-3)).clamp(0.0, 1.0);
            beat = beat.max(strength);
            self.beat_times.push_back(now);
            while self.beat_times.len() > 9 {
                self.beat_times.pop_front();
            }
            o.bpm = tempo(&self.beat_times);
        } else if !self.armed && flux < mean + 0.3 * sd {
            self.armed = true;
        }
        // Tempo goes stale without beats.
        if now - self.last_beat > 4.0 {
            o.bpm = None;
        }
        o.beat = beat;
        &self.out
    }
}

/// Beats per minute from steady beat times (folded into 70–180).
fn tempo(times: &VecDeque<f32>) -> Option<f32> {
    if times.len() < 5 {
        return None;
    }
    let mut gaps: Vec<f32> = times.iter().zip(times.iter().skip(1)).map(|(a, b)| b - a).collect();
    gaps.sort_by(f32::total_cmp);
    let median = gaps[gaps.len() / 2];
    let steady = gaps.iter().filter(|g| (*g - median).abs() < median * 0.15).count() * 3 >= gaps.len() * 2;
    if !steady || median <= 0.0 {
        return None;
    }
    let mut bpm = 60.0 / median;
    while bpm < 70.0 {
        bpm *= 2.0;
    }
    while bpm > 180.0 {
        bpm /= 2.0;
    }
    Some((bpm * 10.0).round() / 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;
    const FPS: f32 = 60.0;

    /// Run `seconds` of `signal(t)` through an analyzer at 60 frames a second.
    fn run(a: &mut Analyzer, seconds: f32, t0: f32, signal: &dyn Fn(f32) -> f32, mut each: impl FnMut(f32, &Features)) {
        let per = (RATE as f32 / FPS) as usize;
        let frames = (seconds * FPS) as usize;
        let mut buf = vec![0.0; per];
        for f in 0..frames {
            let start = t0 + f as f32 / FPS;
            for (i, s) in buf.iter_mut().enumerate() {
                *s = signal(start + i as f32 / RATE as f32);
            }
            a.push(&buf);
            let out = a.update(1.0 / FPS).clone();
            each(start, &out);
        }
    }

    fn sine(hz: f32, amp: f32) -> impl Fn(f32) -> f32 {
        move |t| amp * (std::f32::consts::TAU * hz * t).sin()
    }

    #[test]
    fn bass_and_treble_land_in_their_bands() {
        let mut a = Analyzer::new(RATE);
        let mut last = Features::default();
        run(&mut a, 2.0, 0.0, &sine(60.0, 0.5), |_, f| last = f.clone());
        assert!(last.active);
        assert!(last.low > 0.6 && last.high < 0.1 && last.mid < 0.3, "{last:?}");
        assert!(last.bands[0] > 0.5 || last.bands[1] > 0.5, "{:?}", last.bands);

        let mut a = Analyzer::new(RATE);
        run(&mut a, 2.0, 0.0, &sine(6_000.0, 0.3), |_, f| last = f.clone());
        assert!(last.high > 0.6 && last.low < 0.1, "{last:?}");
        assert!(last.bands[BANDS - 3..].iter().any(|b| *b > 0.5), "{:?}", last.bands);
    }

    /// A kick drum at 120 BPM over a quiet pad: about two beats a second and
    /// a tempo of 120.
    #[test]
    fn finds_beats_and_tempo() {
        let kick = |t: f32| {
            let phase = t % 0.5;
            let env = (-phase / 0.07).exp();
            0.8 * env * (std::f32::consts::TAU * 55.0 * phase).sin()
        };
        let pad = sine(220.0, 0.08);
        let hat = |t: f32| 0.03 * ((t * 12_345.678).sin() * 43_758.547).fract().mul_add(2.0, -1.0) * if (t % 0.25) < 0.03 { 1.0 } else { 0.2 };
        let signal = |t: f32| kick(t) + pad(t) + hat(t);
        let mut a = Analyzer::new(RATE);
        let mut beats_at = Vec::new();
        let mut last_count = 0;
        let mut peak_beat: f32 = 0.0;
        let mut out = Features::default();
        run(&mut a, 8.0, 0.0, &signal, |t, f| {
            if f.beats != last_count {
                last_count = f.beats;
                beats_at.push(t);
            }
            if t > 2.0 {
                peak_beat = peak_beat.max(f.beat);
            }
            out = f.clone();
        });
        let late = beats_at.iter().filter(|t| **t >= 2.0).count();
        assert!((11..=13).contains(&late), "{late} beats in 6 s: {beats_at:?}");
        assert!(peak_beat > 0.5);
        let bpm = out.bpm.expect("a steady tempo");
        assert!((bpm - 120.0).abs() < 4.0, "{bpm}");
    }

    /// A held note makes at most its onset, and the values move smoothly.
    #[test]
    fn sustained_notes_do_not_flicker() {
        let mut a = Analyzer::new(RATE);
        let mut beats = Vec::new();
        let mut max_step: f32 = 0.0;
        let mut prev: Option<Features> = None;
        let chord = |t: f32| sine(220.0, 0.2)(t) + sine(277.2, 0.15)(t) + sine(330.0, 0.12)(t);
        run(&mut a, 4.0, 0.0, &chord, |t, f| {
            if let Some(p) = &prev {
                if f.beats != p.beats {
                    beats.push(t);
                }
                if t > 0.5 {
                    max_step = max_step.max((f.mid - p.mid).abs()).max((f.level - p.level).abs());
                }
            }
            prev = Some(f.clone());
        });
        assert!(beats.iter().filter(|t| **t > 0.5).count() == 0, "{beats:?}");
        assert!(max_step < 0.02, "{max_step}");
    }

    #[test]
    fn silence_settles_and_reset_forgets() {
        let mut a = Analyzer::new(RATE);
        run(&mut a, 2.0, 0.0, &sine(80.0, 0.5), |_, _| {});
        assert!(a.features().active && a.features().low > 0.5);
        let mut at_1s = Features::default();
        run(&mut a, 2.0, 2.0, &|_| 0.0, |t, f| {
            if (2.99..3.01).contains(&t) {
                at_1s = f.clone();
            }
        });
        assert!(!at_1s.active);
        assert!(a.features().low < 0.05 && a.features().level < 0.05, "{:?}", a.features());
        // After a reset, a quiet song is measured against itself, not the loud one.
        a.reset();
        let mut last = Features::default();
        run(&mut a, 3.0, 4.0, &sine(80.0, 0.05), |_, f| last = f.clone());
        assert!(last.low > 0.5, "{last:?}");
        // Nonsense input does not poison the state.
        a.push(&[f32::NAN, f32::INFINITY]);
        assert!(a.update(1.0 / FPS).level.is_finite());
    }

    #[test]
    fn sensitivity_changes_the_response() {
        let mut quiet = Analyzer::new(RATE);
        quiet.set_tuning(Tuning { sensitivity: 0.4, ..Tuning::default() });
        let mut loud = Analyzer::new(RATE);
        loud.set_tuning(Tuning { sensitivity: 2.0, ..Tuning::default() });
        let (mut q, mut l) = (0.0, 0.0);
        // Half the level the analyzers first heard.
        let sig = |t: f32| if t < 1.0 { sine(100.0, 0.6)(t) } else { sine(100.0, 0.3)(t) };
        run(&mut quiet, 2.0, 0.0, &sig, |_, f| q = f.low);
        run(&mut loud, 2.0, 0.0, &sig, |_, f| l = f.low);
        assert!(l > q + 0.2, "{q} vs {l}");
        assert_eq!(tempo(&[0.0, 0.5, 1.0, 1.5, 2.0, 2.5].into_iter().collect()), Some(120.0));
        assert_eq!(tempo(&[0.0, 0.3, 1.0, 1.1, 2.0, 2.9].into_iter().collect()), None);
    }
}
