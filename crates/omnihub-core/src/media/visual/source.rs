//! Where the sound for the visuals comes from.
//!
//! On Windows: WASAPI loopback on the default output device. That is a copy
//! of what the PC is playing through its speakers or headphones, mixed by
//! Windows; it is not the microphone, needs no permission prompt, and stays
//! in memory (OmniHub never records or sends it anywhere). With the pretend
//! player (`OMNIHUB_FAKE_MEDIA=1`) a synthesised beat stands in for it.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub trait Source {
    /// Append the audio that arrived since the last call (mono).
    fn read(&mut self, out: &mut Vec<f32>) -> Result<(), String>;
    fn rate(&self) -> u32;
    /// What is being listened to, for the settings panel.
    fn name(&self) -> String;
    /// The default output changed (headphones plugged in): open it again.
    fn stale(&mut self) -> bool {
        false
    }
}

/// A test track: kick, bass, hats and a pad, silent while "paused". The
/// tempo and key follow `track`, so a track change is visible.
pub struct Synth {
    rate: u32,
    t: f64,
    last: Instant,
    playing: Arc<AtomicBool>,
    track: Arc<AtomicU32>,
}

impl Synth {
    pub fn new(playing: Arc<AtomicBool>, track: Arc<AtomicU32>) -> Synth {
        Synth { rate: 48_000, t: 0.0, last: Instant::now(), playing, track }
    }

    fn sample(&self, t: f64) -> f32 {
        let track = self.track.load(Ordering::Relaxed);
        let even = track.is_multiple_of(2);
        let bpm = if even { 118.0 } else { 96.0 };
        let beat = 60.0 / bpm;
        let root = if even { 55.0 } else { 49.0 };
        let tau = std::f64::consts::TAU;
        let p = t % beat;
        let kick = (-p / 0.08).exp() * (tau * (root + 60.0 * (-p / 0.03).exp()) * p).sin() * 0.7;
        let bar = (t / (beat * 4.0)).floor() as i64;
        let bass_note = [1.0, 1.0, 1.5, 1.335][(bar % 4) as usize];
        let bass = (tau * root * 2.0 * bass_note * t).sin() * 0.18 * (0.6 + 0.4 * (-(t % (beat / 2.0)) / 0.2).exp());
        let hp = t % (beat / 2.0);
        let noise = ((t * 91_731.37).sin() * 43_758.545).fract() * 2.0 - 1.0;
        let hat = noise * (-hp / 0.025).exp() * 0.06;
        let pad = [1.0, 1.26, 1.5].iter().map(|m| (tau * root * 4.0 * bass_note * m * t).sin()).sum::<f64>() * 0.025 * (1.0 + 0.3 * (t * 0.2).sin());
        (kick + bass + hat + pad) as f32
    }
}

impl Source for Synth {
    fn read(&mut self, out: &mut Vec<f32>) -> Result<(), String> {
        let n = (self.last.elapsed().as_secs_f64() * self.rate as f64) as usize;
        if n == 0 {
            return Ok(());
        }
        self.last = Instant::now();
        let n = n.min(self.rate as usize / 2);
        let on = self.playing.load(Ordering::Relaxed);
        for _ in 0..n {
            out.push(if on { self.sample(self.t) } else { 0.0 });
            self.t += 1.0 / self.rate as f64;
        }
        Ok(())
    }

    fn rate(&self) -> u32 {
        self.rate
    }

    fn name(&self) -> String {
        "Test signal (pretend player)".into()
    }
}

#[cfg(windows)]
pub use win::Loopback;

#[cfg(windows)]
mod win {
    use std::time::{Duration, Instant};

    use windows::core::GUID;
    use windows::Win32::Media::Audio::{eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDeviceEnumerator, MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX, WAVEFORMATEXTENSIBLE};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED};

    use super::Source;

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Kind {
        F32,
        I16,
        I24,
        I32,
    }

    /// WASAPI loopback capture of the default output device.
    pub struct Loopback {
        enumerator: IMMDeviceEnumerator,
        client: IAudioClient,
        capture: IAudioCaptureClient,
        device: String,
        channels: usize,
        block: usize,
        kind: Kind,
        rate: u32,
        checked: Instant,
    }

    fn default_id(en: &IMMDeviceEnumerator) -> Result<String, String> {
        unsafe {
            let dev = en.GetDefaultAudioEndpoint(eRender, eConsole).map_err(|_| "No speakers or headphones are set up in Windows.".to_string())?;
            let id = dev.GetId().map_err(|e| e.message().to_string())?;
            let s = id.to_string().unwrap_or_default();
            CoTaskMemFree(Some(id.0 as *const _));
            Ok(s)
        }
    }

    impl Loopback {
        /// Call on the thread that will read (it joins the COM apartment).
        pub fn open() -> Result<Loopback, String> {
            let e = |e: windows::core::Error| e.message().to_string();
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(e)?;
                let device = default_id(&enumerator)?;
                let dev = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).map_err(e)?;
                let client: IAudioClient = dev.Activate(CLSCTX_ALL, None).map_err(e)?;
                let fmt = client.GetMixFormat().map_err(e)?;
                let f: WAVEFORMATEX = std::ptr::read_unaligned(fmt);
                // Copies: the struct is packed.
                let (tag, cb, bits, channels, block, rate) = (f.wFormatTag, f.cbSize, f.wBitsPerSample, f.nChannels, f.nBlockAlign, f.nSamplesPerSec);
                let sub: Option<GUID> = (tag == 0xFFFE && cb >= 22).then(|| std::ptr::read_unaligned(std::ptr::addr_of!((*(fmt as *const WAVEFORMATEXTENSIBLE)).SubFormat)));
                // KSDATAFORMAT_SUBTYPE_*: data1 1 = PCM, 3 = IEEE float.
                let float = tag == 3 || sub.is_some_and(|g| g.data1 == 3);
                let pcm = tag == 1 || sub.is_some_and(|g| g.data1 == 1);
                let kind = match (float, pcm, bits) {
                    (true, _, 32) => Kind::F32,
                    (_, true, 16) => Kind::I16,
                    (_, true, 24) => Kind::I24,
                    (_, true, 32) => Kind::I32,
                    _ => {
                        CoTaskMemFree(Some(fmt as *const _));
                        return Err(format!("The output device uses an audio format OmniHub can't read ({bits} bit)."));
                    }
                };
                // 200 ms of buffer; read every frame.
                let r = client.Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, 2_000_000, 0, fmt, None);
                CoTaskMemFree(Some(fmt as *const _));
                r.map_err(e)?;
                let capture: IAudioCaptureClient = client.GetService().map_err(e)?;
                client.Start().map_err(e)?;
                Ok(Loopback { enumerator, client, capture, device, channels: channels.max(1) as usize, block: block.max(1) as usize, kind, rate, checked: Instant::now() })
            }
        }
    }

    impl Drop for Loopback {
        fn drop(&mut self) {
            unsafe {
                let _ = self.client.Stop();
            }
        }
    }

    impl Source for Loopback {
        fn read(&mut self, out: &mut Vec<f32>) -> Result<(), String> {
            let e = |e: windows::core::Error| e.message().to_string();
            unsafe {
                loop {
                    if self.capture.GetNextPacketSize().map_err(e)? == 0 {
                        return Ok(());
                    }
                    let (mut data, mut frames, mut flags) = (std::ptr::null_mut(), 0u32, 0u32);
                    self.capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None).map_err(e)?;
                    let n = frames as usize;
                    if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || data.is_null() {
                        out.extend(std::iter::repeat_n(0.0, n));
                    } else {
                        let bytes = std::slice::from_raw_parts(data, n * self.block);
                        let width = self.block / self.channels;
                        for frame in bytes.chunks_exact(self.block) {
                            let mut sum = 0.0f32;
                            for ch in frame.chunks_exact(width).take(self.channels) {
                                sum += match self.kind {
                                    Kind::F32 => f32::from_le_bytes([ch[0], ch[1], ch[2], ch[3]]),
                                    Kind::I16 => i16::from_le_bytes([ch[0], ch[1]]) as f32 / 32_768.0,
                                    Kind::I24 => (i32::from_le_bytes([0, ch[0], ch[1], ch[2]]) >> 8) as f32 / 8_388_608.0,
                                    Kind::I32 => i32::from_le_bytes([ch[0], ch[1], ch[2], ch[3]]) as f32 / 2_147_483_648.0,
                                };
                            }
                            out.push(sum / self.channels as f32);
                        }
                    }
                    self.capture.ReleaseBuffer(frames).map_err(e)?;
                }
            }
        }

        fn rate(&self) -> u32 {
            self.rate
        }

        fn name(&self) -> String {
            "Default speakers or headphones".into()
        }

        fn stale(&mut self) -> bool {
            if self.checked.elapsed() < Duration::from_secs(2) {
                return false;
            }
            self.checked = Instant::now();
            default_id(&self.enumerator).is_ok_and(|id| id != self.device)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synth_plays_and_pauses() {
        let playing = Arc::new(AtomicBool::new(true));
        let mut s = Synth::new(playing.clone(), Arc::new(AtomicU32::new(0)));
        std::thread::sleep(std::time::Duration::from_millis(30));
        let mut out = Vec::new();
        s.read(&mut out).unwrap();
        assert!(out.len() > 1000, "{}", out.len());
        assert!(out.iter().any(|v| v.abs() > 0.05));
        playing.store(false, Ordering::Relaxed);
        std::thread::sleep(std::time::Duration::from_millis(20));
        let mut quiet = Vec::new();
        s.read(&mut quiet).unwrap();
        assert!(!quiet.is_empty() && quiet.iter().all(|v| *v == 0.0));
    }
}
