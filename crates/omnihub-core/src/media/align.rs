//! Lining a music video up with the song.
//!
//! Many official videos open with a story before the song starts, so the
//! song plays later in the video than on the album, and a video started at
//! the song's position runs early. Here the two are compared by ear: the
//! video's soundtrack (downloaded once with yt-dlp, see `stream.rs`) and the
//! song as the PC plays it (the same loopback sound the light show listens
//! to) are each turned into an onset envelope (how much new sound starts, 50
//! times a second), and the video's envelope is slid along the song's until
//! they line up. The lag that fits best, when it clearly beats every other
//! one, is the video's offset: video time = song time + offset.

use std::collections::VecDeque;
use std::path::Path;

use super::visual::fft::Fft;

/// Envelope values per second.
pub const RATE: f32 = 50.0;
const FRAME: usize = 1024;
const BANDS: usize = 20;

/// Turns sound into an onset envelope: per hop of 20 ms, the rise in loudness
/// summed over 20 bands from 80 Hz to 6 kHz (log-compressed, so a quiet
/// intro and a loud chorus both count).
pub struct Onsets {
    hop: usize,
    fft: Fft,
    window: Vec<f32>,
    bands: Vec<(usize, usize)>,
    pending: Vec<f32>,
    prev: Vec<f32>,
    re: Vec<f32>,
    im: Vec<f32>,
}

impl Onsets {
    pub fn new(sample_rate: u32) -> Onsets {
        let sr = sample_rate.max(8000) as f32;
        let hop = (sr / RATE).round() as usize;
        let window = (0..FRAME).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / FRAME as f32).cos()).collect();
        let bin = |hz: f32| ((hz / sr * FRAME as f32).round() as usize).clamp(1, FRAME / 2 - 1);
        let (lo, hi) = (80f32, 6000f32.min(sr / 2.0 - 100.0));
        let edges: Vec<usize> = (0..=BANDS).map(|i| bin(lo * (hi / lo).powf(i as f32 / BANDS as f32))).collect();
        let bands = edges.windows(2).map(|w| (w[0], w[1].max(w[0] + 1))).collect();
        Onsets { hop, fft: Fft::new(FRAME), window, bands, pending: Vec::with_capacity(FRAME * 2), prev: vec![0.0; BANDS], re: vec![0.0; FRAME], im: vec![0.0; FRAME] }
    }

    /// Samples not yet turned into envelope values.
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// Feed mono samples; one value per hop is appended to `out`.
    pub fn push(&mut self, samples: &[f32], out: &mut Vec<f32>) {
        self.pending.extend_from_slice(samples);
        let mut start = 0;
        while self.pending.len() - start >= FRAME {
            let frame = &self.pending[start..start + FRAME];
            for ((re, im), (s, w)) in self.re.iter_mut().zip(self.im.iter_mut()).zip(frame.iter().zip(&self.window)) {
                *re = s * w;
                *im = 0.0;
            }
            self.fft.run(&mut self.re, &mut self.im);
            let mut flux = 0.0;
            for (b, &(a, z)) in self.bands.iter().enumerate() {
                let e: f32 = (a..z).map(|k| self.re[k] * self.re[k] + self.im[k] * self.im[k]).sum::<f32>() / (z - a) as f32;
                let v = (1.0 + 1000.0 * e).ln();
                flux += (v - self.prev[b]).max(0.0);
                self.prev[b] = v;
            }
            out.push(flux);
            start += self.hop;
        }
        self.pending.drain(..start);
    }
}

/// The envelope of a whole recording.
pub fn envelope(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    let mut o = Onsets::new(sample_rate);
    let mut out = Vec::with_capacity(samples.len() / o.hop + 1);
    o.push(samples, &mut out);
    out
}

/// Peaks over the local level: subtract a half-second moving average and
/// keep what sticks out, so steady loudness counts for nothing.
pub fn peaks(env: &[f32]) -> Vec<f32> {
    let half = (RATE * 0.25) as usize;
    let mut sum = vec![0.0f64; env.len() + 1];
    for (i, v) in env.iter().enumerate() {
        sum[i + 1] = sum[i] + *v as f64;
    }
    (0..env.len())
        .map(|i| {
            let (a, b) = (i.saturating_sub(half), (i + half + 1).min(env.len()));
            let mean = (sum[b] - sum[a]) / (b - a) as f64;
            (env[i] as f64 - mean).max(0.0) as f32
        })
        .collect()
}

/// Where the song's stretch fits in the video.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fit {
    /// Video time minus song time, in seconds.
    pub offset: f32,
    /// Normalised correlation at the best lag (0–1).
    pub score: f32,
    /// How far ahead of the best other lag (more than a second away) it is.
    pub margin: f32,
}

/// Slide `song` (peaks of the song from song time `song_start` s) along
/// `video` (peaks of the whole video) and find the lag that fits best, for
/// offsets between `min` and `max` seconds.
pub fn best_fit(song: &[f32], song_start: f32, video: &[f32], min: f32, max: f32) -> Option<Fit> {
    let n = song.len();
    if n < (RATE * 8.0) as usize || video.len() < n {
        return None;
    }
    let s0 = (song_start * RATE).round() as i64;
    let ss: f64 = song.iter().map(|v| (*v as f64).powi(2)).sum();
    if ss <= 1e-9 {
        return None;
    }
    let mut sq = vec![0.0f64; video.len() + 1];
    for (i, v) in video.iter().enumerate() {
        sq[i + 1] = sq[i] + (*v as f64).powi(2);
    }
    let lo = (min * RATE).floor() as i64;
    let hi = (max * RATE).ceil() as i64;
    let mut scores: Vec<(i64, f64)> = Vec::new();
    for lag in lo..=hi {
        let start = s0 + lag;
        if start < 0 || start as usize + n > video.len() {
            continue;
        }
        let st = start as usize;
        let vv = sq[st + n] - sq[st];
        if vv <= 1e-9 {
            continue;
        }
        let dot: f64 = song.iter().zip(&video[st..st + n]).map(|(a, b)| *a as f64 * *b as f64).sum();
        scores.push((lag, dot / (ss * vv).sqrt()));
    }
    let &(best_lag, best) = scores.iter().max_by(|a, b| a.1.total_cmp(&b.1))?;
    let away = RATE as i64;
    let second = scores.iter().filter(|(l, _)| (l - best_lag).abs() > away).map(|s| s.1).fold(0.0f64, f64::max);
    // A touch finer than one frame: the parabola through the peak and its neighbours.
    let at = |l: i64| scores.iter().find(|s| s.0 == l).map(|s| s.1);
    let fine = match (at(best_lag - 1), at(best_lag + 1)) {
        (Some(a), Some(c)) if (a - 2.0 * best + c).abs() > 1e-12 => (0.5 * (a - c) / (a - 2.0 * best + c)).clamp(-0.5, 0.5),
        _ => 0.0,
    };
    Some(Fit { offset: (best_lag as f64 + fine) as f32 / RATE, score: best as f32, margin: (best - second) as f32 })
}

/// Whether a fit is clear enough to move the video by.
pub fn trustworthy(f: &Fit) -> bool {
    f.score >= 0.3 && f.margin >= 0.06
}

/// The song as the PC plays it: envelope values with the time each was heard.
pub struct Live {
    onsets: Option<(u32, Onsets)>,
    pub frames: VecDeque<(i64, f32)>,
}

impl Default for Live {
    fn default() -> Self {
        Live { onsets: None, frames: VecDeque::with_capacity(4096) }
    }
}

impl Live {
    /// Mono loopback samples at `rate` that ended at `now_ms`.
    pub fn push(&mut self, samples: &[f32], rate: u32, now_ms: i64) {
        if self.onsets.as_ref().map(|o| o.0) != Some(rate) {
            self.onsets = Some((rate, Onsets::new(rate)));
            self.frames.clear();
        }
        let (_, o) = self.onsets.as_mut().unwrap();
        let mut out = Vec::new();
        o.push(samples, &mut out);
        if out.is_empty() {
            return;
        }
        // The newest value ends where the unprocessed samples begin.
        let end = now_ms - (o.pending() as f64 * 1000.0 / rate as f64) as i64;
        let step = 1000.0 / RATE as f64;
        for (i, v) in out.iter().enumerate() {
            let t = end - ((out.len() - 1 - i) as f64 * step) as i64;
            self.frames.push_back((t, *v));
        }
        // A minute is plenty.
        while self.frames.len() > (RATE * 60.0) as usize {
            self.frames.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }
}

/// Where the light show's listener hands its sound over while a video is
/// being matched (`on`), so the PC's sound is captured only once.
#[derive(Default)]
pub struct Tap {
    pub on: std::sync::atomic::AtomicBool,
    pub live: parking_lot::Mutex<Live>,
}

/// How far a music video is from being matched to the song.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum SyncStatus {
    /// Matching is turned off (or the video helper isn't allowed).
    Off,
    /// Working on it: what is happening.
    Matching { note: String },
    /// Video time = song time + `offset_ms`.
    #[serde(rename_all = "camelCase")]
    Matched { offset_ms: i64, score: f32 },
    /// It could not be told by ear (a different mix, a live version…): why.
    Unsure { note: String },
}

/// Matched offsets, kept on disk (per video and song length).
pub struct SyncCache {
    path: std::path::PathBuf,
    map: parking_lot::Mutex<std::collections::HashMap<String, i64>>,
}

impl SyncCache {
    pub fn new(path: std::path::PathBuf) -> Self {
        let map = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        SyncCache { path, map: parking_lot::Mutex::new(map) }
    }

    pub fn get(&self, key: &str) -> Option<i64> {
        self.map.lock().get(key).copied()
    }

    pub fn put(&self, key: &str, offset_ms: i64) {
        let mut m = self.map.lock();
        if m.len() > 2000 {
            m.clear();
        }
        m.insert(key.to_string(), offset_ms);
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(&self.path, serde_json::to_vec(&*m).unwrap_or_default());
    }
}

/// Wall time → song time, from checkpoints taken while the song played:
/// (wall ms, song ms, rate). A frame maps through the last checkpoint at or
/// before it, if the next one (or now) is less than `gap_ms` later.
pub fn song_time_at(checkpoints: &[(i64, f64, f64)], t: i64, gap_ms: i64) -> Option<f64> {
    let i = checkpoints.partition_point(|c| c.0 <= t);
    if i == 0 {
        return None;
    }
    let (w, s, r) = checkpoints[i - 1];
    let next = checkpoints.get(i).map_or(w + gap_ms, |c| c.0);
    (next - w <= gap_ms && t - w <= gap_ms).then(|| s + (t - w) as f64 * r)
}

/// The live envelope in song time: values heard while the song ran from
/// `from_ms`, mapped through `song_at` (wall ms → song ms, None while
/// paused or not this song), resampled to RATE. Returns (start s, values).
pub fn in_song_time(live: &Live, song_at: impl Fn(i64) -> Option<f64>) -> Option<(f32, Vec<f32>)> {
    let pts: Vec<(f64, f32)> = live.frames.iter().filter_map(|(t, v)| song_at(*t).map(|s| (s, *v))).collect();
    if pts.len() < 2 {
        return None;
    }
    let step = 1000.0 / RATE as f64;
    let (first, last) = (pts[0].0, pts[pts.len() - 1].0);
    let mut out = Vec::with_capacity(((last - first) / step) as usize + 1);
    let mut j = 0;
    let mut t = first;
    while t <= last {
        while j + 1 < pts.len() && pts[j + 1].0 <= t {
            j += 1;
        }
        out.push(pts[j].1);
        t += step;
    }
    Some(((first / 1000.0) as f32, out))
}

/// Decode an audio file (AAC in MP4/M4A) to mono samples.
pub fn decode(path: &Path) -> Result<(Vec<f32>, u32), String> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe().format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default()).map_err(|e| format!("not a sound file this can read: {e}"))?;
    let mut format = probed.format;
    let track = format.default_track().ok_or("no sound in the file")?;
    let id = track.id;
    let rate = track.codec_params.sample_rate.ok_or("no sample rate")?;
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).map_err(|e| format!("cannot decode it: {e}"))?;
    let mut mono = Vec::new();
    let mut buf: Option<SampleBuffer<f32>> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(symphonia::core::errors::Error::ResetRequired) => break,
            Err(e) => return Err(e.to_string()),
        };
        if packet.track_id() != id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e.to_string()),
        };
        let spec = *decoded.spec();
        let ch = spec.channels.count().max(1);
        let b = buf.get_or_insert_with(|| SampleBuffer::new(decoded.capacity() as u64, spec));
        if b.capacity() < decoded.capacity() * ch {
            *b = SampleBuffer::new(decoded.capacity() as u64, spec);
        }
        b.copy_interleaved_ref(decoded);
        for f in b.samples().chunks_exact(ch) {
            mono.push(f.iter().sum::<f32>() / ch as f32);
        }
        // Twenty minutes is no music video.
        if mono.len() > rate as usize * 1200 {
            break;
        }
    }
    if mono.is_empty() {
        return Err("the soundtrack is empty".into());
    }
    Ok((mono, rate))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up "song": a kick-and-snare pattern with a melody of notes that
    /// starts at different moments, so no two stretches look alike.
    fn song(sr: u32, secs: f32, seed: u32) -> Vec<f32> {
        let n = (sr as f32 * secs) as usize;
        let mut out = vec![0.0f32; n];
        let mut x = seed.wrapping_mul(2654435761);
        let mut rnd = || {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x
        };
        let mut t = 0.0f32;
        while t < secs {
            let start = (t * sr as f32) as usize;
            let freq = 110.0 * 2f32.powf((rnd() % 24) as f32 / 12.0);
            let len = (sr as f32 * (0.08 + (rnd() % 30) as f32 / 100.0)) as usize;
            for i in 0..len.min(n.saturating_sub(start)) {
                let env = (-(i as f32) / (sr as f32 * 0.06)).exp();
                out[start + i] += 0.4 * env * (std::f32::consts::TAU * freq * i as f32 / sr as f32).sin();
            }
            t += 0.12 + (rnd() % 40) as f32 / 100.0;
        }
        out
    }

    fn noise(sr: u32, secs: f32, amp: f32) -> Vec<f32> {
        let mut x = 12345u32;
        (0..(sr as f32 * secs) as usize)
            .map(|_| {
                x = x.wrapping_mul(1103515245).wrapping_add(12345);
                amp * ((x >> 9) as f32 / (1u32 << 23) as f32 - 0.5)
            })
            .collect()
    }

    #[test]
    fn finds_a_story_intro_before_the_song() {
        let sr = 44_100;
        let track = song(sr, 60.0, 7);
        // The video: 23.4 s of story (quiet talk and room noise), then the song, a bit quieter, with some hiss.
        let mut video = noise(sr, 23.4, 0.05);
        let hiss = noise(sr, 60.0, 0.02);
        video.extend(track.iter().zip(&hiss).map(|(a, b)| 0.8 * a + b));
        let v = peaks(&envelope(&video, sr));
        // What the PC played: 15 s from 20 s into the song, at another rate.
        let heard: Vec<f32> = song(48_000, 60.0, 7)[48_000 * 20..48_000 * 35].to_vec();
        let s = peaks(&envelope(&heard, 48_000));
        let fit = best_fit(&s, 20.0, &v, -10.0, 60.0).unwrap();
        assert!((fit.offset - 23.4).abs() < 0.06, "{fit:?}");
        assert!(trustworthy(&fit), "{fit:?}");
    }

    #[test]
    fn a_video_in_step_fits_at_zero_and_another_song_does_not_fit() {
        let sr = 44_100;
        let v = peaks(&envelope(&song(sr, 40.0, 3), sr));
        let s = peaks(&envelope(&song(sr, 40.0, 3)[sr as usize * 5..sr as usize * 20], sr));
        let fit = best_fit(&s, 5.0, &v, -10.0, 20.0).unwrap();
        assert!(fit.offset.abs() < 0.05 && trustworthy(&fit), "{fit:?}");
        let other = peaks(&envelope(&song(sr, 15.0, 99), sr));
        let fit = best_fit(&other, 5.0, &v, -10.0, 20.0).unwrap();
        assert!(!trustworthy(&fit), "a different song must not be trusted: {fit:?}");
        // Too little sound to judge.
        assert_eq!(best_fit(&s[..100], 5.0, &v, -10.0, 20.0), None);
    }

    #[test]
    fn live_sound_is_put_in_song_time() {
        let sr = 48_000;
        let track = song(sr, 30.0, 5);
        let mut live = Live::default();
        // Heard in 10 ms chunks from wall time 1_000_000 ms, while the song ran from 4 s.
        let chunk = sr as usize / 100;
        for (i, c) in track[sr as usize * 4..sr as usize * 24].chunks(chunk).enumerate() {
            live.push(c, sr, 1_000_000 + ((i + 1) * 10) as i64);
        }
        let (start, values) = in_song_time(&live, |t| Some(4000.0 + (t - 1_000_000) as f64)).unwrap();
        assert!((start - 4.0).abs() < 0.05, "{start}");
        assert!((values.len() as f32 / RATE - 20.0).abs() < 0.2, "{}", values.len());
        let v = peaks(&envelope(&track, sr));
        let fit = best_fit(&peaks(&values), start, &v, -10.0, 10.0).unwrap();
        assert!(fit.offset.abs() < 0.06 && trustworthy(&fit), "{fit:?}");
    }

    #[test]
    fn maps_wall_time_through_checkpoints_and_not_across_pauses() {
        // Playing from song 10 s at wall 1000, checkpoints every 200 ms, then paused for 5 s, then from song 11 s.
        let mut cps: Vec<(i64, f64, f64)> = (0..5).map(|i| (1000 + i * 200, 10_000.0 + (i * 200) as f64, 1.0)).collect();
        cps.extend((0..5).map(|i| (6800 + i * 200, 11_000.0 + (i * 200) as f64, 1.0)));
        assert_eq!(song_time_at(&cps, 1100, 600), Some(10_100.0));
        assert_eq!(song_time_at(&cps, 900, 600), None, "before the song was heard");
        assert_eq!(song_time_at(&cps, 3000, 600), None, "while paused");
        assert_eq!(song_time_at(&cps, 6900, 600), Some(11_100.0));
        assert_eq!(song_time_at(&cps, 8300, 600), None, "after the last checkpoint, too long ago");
    }

    #[test]
    fn remembers_matched_offsets() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("videos").join("sync.json");
        let c = SyncCache::new(path.clone());
        assert_eq!(c.get("abc|90"), None);
        c.put("abc|90", 23_400);
        assert_eq!(SyncCache::new(path).get("abc|90"), Some(23_400));
        let s = serde_json::to_value(SyncStatus::Matched { offset_ms: 23_400, score: 0.5 }).unwrap();
        assert_eq!(s, serde_json::json!({ "state": "matched", "offsetMs": 23400, "score": 0.5 }));
    }

    /// Real recordings: `OMNIHUB_ALIGN_SONG=song.m4a OMNIHUB_ALIGN_VIDEO=video.m4a
    /// OMNIHUB_ALIGN_EXPECT=17.3 cargo test -- --ignored aligns_real_recordings`.
    #[test]
    #[ignore]
    fn aligns_real_recordings() {
        let get = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("set {k}"));
        let (song, sr) = decode(Path::new(&get("OMNIHUB_ALIGN_SONG"))).unwrap();
        let (video, vr) = decode(Path::new(&get("OMNIHUB_ALIGN_VIDEO"))).unwrap();
        let expect: f32 = get("OMNIHUB_ALIGN_EXPECT").parse().unwrap();
        let from = sr as usize * 5;
        let to = (sr as usize * 25).min(song.len());
        let s = peaks(&envelope(&song[from..to], sr));
        let v = peaks(&envelope(&video, vr));
        let fit = best_fit(&s, 5.0, &v, -10.0, 60.0).unwrap();
        println!("song {:.1} s at {sr} Hz, video {:.1} s at {vr} Hz: {fit:?}", song.len() as f32 / sr as f32, video.len() as f32 / vr as f32);
        assert!((fit.offset - expect).abs() < 0.1 && trustworthy(&fit), "{fit:?}");
    }
}
