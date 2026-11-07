//! Instant replay: the last minutes of the screen (and the game's sound),
//! kept on disk while a game runs, so "clip that" can save what just
//! happened.
//!
//! FFmpeg (published with each release as `OmniHub-ffmpeg-x64.exe`, its
//! checksum built in, LGPL) grabs the screen with Windows' desktop
//! duplication (`ddagrab`) and encodes on the graphics card (NVENC, AMF or
//! Quick Sync; Media Foundation otherwise), so a game loses next to no
//! frames. It writes two-second segments; segments older than the replay
//! length are deleted as new ones arrive. Saving joins the newest segments
//! into one MP4 without encoding again, which takes a moment.
//!
//! The game's sound is what Windows plays (WASAPI loopback, the same as
//! Aurora's), handed to FFmpeg on its standard input, with silence filled in
//! when nothing plays so picture and sound stay together.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// The release asset.
pub const ASSET: &str = "OmniHub-ffmpeg-x64.exe";
/// Seconds per segment (and between keyframes).
pub const SEGMENT: u32 = 2;
/// The sound handed to FFmpeg: 48 kHz stereo, 32-bit float.
pub const AUDIO_RATE: u32 = 48_000;

/// SHA-256 of the FFmpeg published with this release, when the build knows it.
pub fn pinned_sha256() -> Option<&'static str> {
    option_env!("OMNIHUB_FFMPEG_SHA256").filter(|s| s.len() == 64)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Encoder {
    /// NVIDIA.
    Nvenc,
    /// AMD.
    Amf,
    /// Intel Quick Sync.
    Qsv,
    /// Windows' Media Foundation (any graphics card, or the processor).
    Mf,
    /// A processor encoder (tests, and machines with nothing else).
    Software,
}

impl Encoder {
    /// Tried in this order.
    pub const ALL: [Encoder; 4] = [Encoder::Nvenc, Encoder::Amf, Encoder::Qsv, Encoder::Mf];

    pub fn label(self) -> &'static str {
        match self {
            Encoder::Nvenc => "NVIDIA NVENC",
            Encoder::Amf => "AMD AMF",
            Encoder::Qsv => "Intel Quick Sync",
            Encoder::Mf => "Windows Media Foundation",
            Encoder::Software => "Processor",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Quality {
    #[default]
    Standard,
    High,
}

/// Where the picture comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum Video {
    /// A monitor, by its number in Windows (0 = the first).
    Desktop { monitor: u32 },
    /// A test pattern (tests).
    Test,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    pub video: Video,
    pub fps: u32,
    pub quality: Quality,
    pub encoder: Encoder,
    /// The game's sound comes in on standard input.
    pub audio: bool,
}

/// FFmpeg's arguments for the replay buffer, writing segments into `dir`.
pub fn record_args(o: &Options, dir: &Path) -> Vec<String> {
    let mut a: Vec<String> = ["-hide_banner", "-loglevel", "error", "-nostats"].map(String::from).to_vec();
    match &o.video {
        Video::Desktop { monitor } => a.extend(["-f".into(), "lavfi".into(), "-i".into(), format!("ddagrab=output_idx={monitor}:framerate={}:draw_mouse=1", o.fps)]),
        Video::Test => a.extend(["-re".into(), "-f".into(), "lavfi".into(), "-i".into(), format!("testsrc=size=320x240:rate={}", o.fps)]),
    }
    if o.audio {
        // Timestamps from the clock, so a pause in what Windows plays can't pull the sound ahead.
        a.extend(["-f", "f32le", "-ar", &AUDIO_RATE.to_string(), "-ac", "2", "-i", "pipe:0"].map(String::from));
    }
    a.extend(["-map", "0:v"].map(String::from));
    if o.audio {
        a.extend(["-map", "1:a", "-c:a", "aac", "-b:a", "160k"].map(String::from));
    }
    let high = o.quality == Quality::High;
    let q = |standard: u32, better: u32| (if high { better } else { standard }).to_string();
    match o.encoder {
        Encoder::Nvenc => a.extend(["-c:v".into(), "h264_nvenc".into(), "-preset".into(), "p4".into(), "-tune".into(), "ll".into(), "-rc".into(), "vbr".into(), "-cq".into(), q(26, 21), "-b:v".into(), "0".into()]),
        Encoder::Amf => a.extend(["-c:v".into(), "h264_amf".into(), "-usage".into(), "lowlatency".into(), "-rc".into(), "cqp".into(), "-qp_i".into(), q(24, 20), "-qp_p".into(), q(26, 22)]),
        Encoder::Qsv => a.extend(["-vf".into(), "hwmap=derive_device=qsv,format=qsv".into(), "-c:v".into(), "h264_qsv".into(), "-global_quality".into(), q(26, 21)]),
        Encoder::Mf => a.extend(["-vf".into(), "hwdownload,format=bgra,format=nv12".into(), "-c:v".into(), "h264_mf".into(), "-b:v".into(), if high { "24M".into() } else { "12M".into() }]),
        Encoder::Software => a.extend(["-c:v".into(), "mpeg2video".into(), "-q:v".into(), "5".into()]),
    }
    a.extend(["-g".into(), (o.fps * SEGMENT).to_string()]);
    a.extend(["-f", "segment", "-segment_time", &SEGMENT.to_string(), "-segment_format", "mpegts", "-reset_timestamps", "0"].map(String::from));
    a.push(dir.join("seg%06d.ts").to_string_lossy().into_owned());
    a
}

/// A quick check that `encoder` works with the screen grab on this PC.
pub fn probe_args(encoder: Encoder, monitor: u32) -> Vec<String> {
    let o = Options { video: Video::Desktop { monitor }, fps: 30, quality: Quality::Standard, encoder, audio: false };
    let mut a = record_args(&o, Path::new("x"));
    // Everything up to the segment output, then ten frames to nowhere.
    let cut = a.iter().position(|x| x == "-f").and_then(|first| a.iter().skip(first + 1).position(|x| x == "-f").map(|p| p + first + 1)).unwrap_or(a.len());
    a.truncate(cut);
    a.extend(["-frames:v", "10", "-f", "null", "-"].map(String::from));
    a
}

/// The segment numbers in `dir`, oldest first.
fn segments(dir: &Path) -> Vec<(u64, PathBuf)> {
    let mut v: Vec<(u64, PathBuf)> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let n = name.strip_prefix("seg")?.strip_suffix(".ts")?.parse().ok()?;
                    Some((n, e.path()))
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|(n, _)| *n);
    v
}

/// The finished segments that cover the last `secs` seconds (the newest
/// file is still being written and is left out).
pub fn pick(segs: &[(u64, PathBuf)], secs: u32, from: Option<u64>) -> Vec<PathBuf> {
    let done = &segs[..segs.len().saturating_sub(1)];
    let want = (secs.div_ceil(SEGMENT)) as usize;
    let start = done.len().saturating_sub(want);
    done[start..].iter().filter(|(n, _)| from.is_none_or(|f| *n >= f)).map(|(_, p)| p.clone()).collect()
}

/// Join segments into one MP4 without encoding again.
pub fn join(ffmpeg: &Path, parts: &[PathBuf], dest: &Path) -> Result<(), String> {
    if parts.is_empty() {
        return Err("Nothing recorded yet — give it a few seconds.".into());
    }
    if let Some(d) = dest.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let list = dest.with_extension("txt");
    let text: String = parts.iter().map(|p| format!("file '{}'\n", p.to_string_lossy().replace('\'', "'\\''"))).collect();
    std::fs::write(&list, text).map_err(|e| e.to_string())?;
    let out = hidden(Command::new(ffmpeg))
        .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "concat", "-safe", "0", "-i"])
        .arg(&list)
        .args(["-c", "copy", "-movflags", "+faststart"])
        .arg(dest)
        .stdin(Stdio::null())
        .output();
    let _ = std::fs::remove_file(&list);
    let out = out.map_err(|e| format!("Couldn't run the recorder: {e}"))?;
    if !out.status.success() || !dest.is_file() {
        let _ = std::fs::remove_file(dest);
        return Err(format!("The clip couldn't be saved: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("unknown error")));
    }
    Ok(())
}

#[cfg_attr(not(windows), allow(unused_mut))]
fn hidden(mut c: Command) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c
}

/// Find the first encoder that works here (a few seconds, once).
pub fn detect(ffmpeg: &Path, monitor: u32) -> Option<Encoder> {
    Encoder::ALL.into_iter().find(|e| {
        let Ok(mut child) = hidden(Command::new(ffmpeg)).args(probe_args(*e, monitor)).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() else { return false };
        let until = Instant::now() + Duration::from_secs(15);
        loop {
            match child.try_wait() {
                Ok(Some(s)) => return s.success(),
                Ok(None) if Instant::now() < until => std::thread::sleep(Duration::from_millis(100)),
                _ => {
                    let _ = child.kill();
                    return false;
                }
            }
        }
    })
}

/// Where the sound comes from: interleaved stereo f32 at [`AUDIO_RATE`].
pub trait Sound {
    /// Append what arrived since the last call.
    fn read(&mut self, out: &mut Vec<f32>);
}

/// Opens the sound on the thread that feeds FFmpeg (audio devices stay on
/// the thread that opened them).
pub type OpenSound = Box<dyn FnOnce() -> Option<Box<dyn Sound>> + Send>;

/// A running replay buffer.
pub struct Recorder {
    ffmpeg: PathBuf,
    dir: PathBuf,
    child: Mutex<Option<Child>>,
    stop: Arc<AtomicBool>,
    /// Keep segments from here on (a recording is going).
    mark: Arc<Mutex<Option<u64>>>,
    keep_secs: Arc<std::sync::atomic::AtomicU32>,
    started: Instant,
    pub encoder: Encoder,
}

impl Recorder {
    /// Start FFmpeg writing segments into `dir` (emptied first), keeping
    /// `keep_secs`; `sound` feeds the game's sound when `opts.audio`.
    pub fn start(ffmpeg: &Path, dir: &Path, opts: &Options, keep_secs: u32, sound: Option<OpenSound>) -> Result<Recorder, String> {
        let _ = std::fs::remove_dir_all(dir);
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let log = std::fs::File::create(dir.join("ffmpeg.log")).map_err(|e| e.to_string())?;
        let mut child = hidden(Command::new(ffmpeg))
            .args(record_args(opts, dir))
            .stdin(if opts.audio { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()
            .map_err(|e| format!("Couldn't start the recorder: {e}"))?;
        let stop = Arc::new(AtomicBool::new(false));
        if let (true, Some(stdin)) = (opts.audio, child.stdin.take()) {
            let stop = stop.clone();
            std::thread::Builder::new().name("replay-sound".into()).spawn(move || feed(stdin, sound, &stop)).map_err(|e| e.to_string())?;
        }
        let mark = Arc::new(Mutex::new(None));
        let keep = Arc::new(std::sync::atomic::AtomicU32::new(keep_secs));
        {
            let (dir, stop, mark, keep) = (dir.to_path_buf(), stop.clone(), mark.clone(), keep.clone());
            std::thread::Builder::new()
                .name("replay-trim".into())
                .spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(1000));
                        let segs = segments(&dir);
                        let keep_n = (keep.load(Ordering::Relaxed).div_ceil(SEGMENT) + 2) as usize;
                        let from = *mark.lock();
                        for (n, p) in segs.iter().take(segs.len().saturating_sub(keep_n)) {
                            if from.is_none_or(|f| *n < f) {
                                let _ = std::fs::remove_file(p);
                            }
                        }
                    }
                })
                .map_err(|e| e.to_string())?;
        }
        Ok(Recorder { ffmpeg: ffmpeg.to_path_buf(), dir: dir.to_path_buf(), child: Mutex::new(Some(child)), stop, mark, keep_secs: keep, started: Instant::now(), encoder: opts.encoder })
    }

    pub fn set_keep(&self, secs: u32) {
        self.keep_secs.store(secs, Ordering::Relaxed);
    }

    /// Still recording? (FFmpeg stops by itself when the screen grab fails.)
    pub fn alive(&self) -> Result<(), String> {
        let mut c = self.child.lock();
        match c.as_mut().map(|c| c.try_wait()) {
            Some(Ok(None)) => Ok(()),
            _ => {
                let log = std::fs::read_to_string(self.dir.join("ffmpeg.log")).unwrap_or_default();
                Err(log.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("The recorder stopped.").to_string())
            }
        }
    }

    /// How long it has been recording.
    pub fn age(&self) -> Duration {
        self.started.elapsed()
    }

    /// How many seconds are kept so far.
    pub fn buffered(&self) -> u32 {
        (self.started.elapsed().as_secs() as u32).min(self.keep_secs.load(Ordering::Relaxed))
    }

    /// Wait (up to a segment and a bit) for the segment being written now to be finished.
    fn settle(&self) {
        let newest = |d: &Path| segments(d).last().map(|(n, _)| *n);
        let before = newest(&self.dir);
        let until = Instant::now() + Duration::from_secs(SEGMENT as u64 + 2);
        while Instant::now() < until && newest(&self.dir) == before && self.alive().is_ok() {
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Save the last `secs` seconds to `dest`.
    pub fn save(&self, secs: u32, dest: &Path) -> Result<(), String> {
        self.alive()?;
        self.settle();
        join(&self.ffmpeg, &pick(&segments(&self.dir), secs, None), dest)
    }

    pub fn recording(&self) -> bool {
        self.mark.lock().is_some()
    }

    /// Keep everything from now until [`Recorder::stop_recording`].
    pub fn start_recording(&self) {
        let next = segments(&self.dir).last().map_or(0, |(n, _)| *n);
        *self.mark.lock() = Some(next);
    }

    /// Save everything since [`Recorder::start_recording`] to `dest`.
    pub fn stop_recording(&self, dest: &Path) -> Result<(), String> {
        let from = self.mark.lock().ok_or("I'm not recording right now.")?;
        self.settle();
        let r = join(&self.ffmpeg, &pick(&segments(&self.dir), u32::MAX / 2, Some(from)), dest);
        *self.mark.lock() = None;
        r
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(mut c) = self.child.lock().take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Hand the sound to FFmpeg in real time, silence included.
fn feed(mut stdin: ChildStdin, open: Option<OpenSound>, stop: &AtomicBool) {
    let mut sound = open.and_then(|o| o());
    let start = Instant::now();
    let mut sent: u64 = 0; // stereo frames
    let mut buf: Vec<f32> = Vec::with_capacity(AUDIO_RATE as usize);
    let mut bytes: Vec<u8> = Vec::with_capacity(AUDIO_RATE as usize * 8);
    while !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(10));
        buf.clear();
        if let Some(s) = sound.as_mut() {
            s.read(&mut buf);
        }
        let due = (start.elapsed().as_secs_f64() * AUDIO_RATE as f64) as u64;
        let have = (buf.len() / 2) as u64;
        // Behind the clock (nothing playing): fill with silence. Far ahead: drop the surplus.
        if sent + have < due {
            buf.resize(((due - sent) * 2) as usize, 0.0);
        } else if sent + have > due + AUDIO_RATE as u64 / 5 {
            buf.truncate(((due + AUDIO_RATE as u64 / 10).saturating_sub(sent) * 2) as usize);
        }
        bytes.clear();
        for v in &buf {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        if stdin.write_all(&bytes).is_err() {
            return;
        }
        sent += (buf.len() / 2) as u64;
    }
}

/// The game's sound from Windows (WASAPI loopback), at 48 kHz stereo.
#[cfg(windows)]
pub fn windows_sound() -> Option<OpenSound> {
    struct Loop {
        src: crate::media::visual::source::Loopback,
        tmp: Vec<f32>,
        pos: f64,
    }
    impl Sound for Loop {
        fn read(&mut self, out: &mut Vec<f32>) {
            use crate::media::visual::source::Source;
            self.tmp.clear();
            if self.src.read_stereo(&mut self.tmp).is_err() {
                return;
            }
            // Resample to 48 kHz by stepping through the device's frames.
            let step = self.src.rate() as f64 / AUDIO_RATE as f64;
            let frames = self.tmp.len() / 2;
            while (self.pos as usize) < frames {
                let i = self.pos as usize * 2;
                out.extend_from_slice(&self.tmp[i..i + 2]);
                self.pos += step;
            }
            self.pos -= frames as f64;
        }
    }
    Some(Box::new(|| {
        let src = crate::media::visual::source::Loopback::open().ok()?;
        Some(Box::new(Loop { src, tmp: Vec::new(), pos: 0.0 }) as Box<dyn Sound>)
    }))
}

#[cfg(not(windows))]
pub fn windows_sound() -> Option<OpenSound> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_cover_the_last_seconds() {
        let segs: Vec<(u64, PathBuf)> = (0..10).map(|n| (n, PathBuf::from(format!("seg{n:06}.ts")))).collect();
        // seg 9 is still being written: the last 6 s are 6, 7 and 8.
        assert_eq!(pick(&segs, 6, None), vec![PathBuf::from("seg000006.ts"), PathBuf::from("seg000007.ts"), PathBuf::from("seg000008.ts")]);
        assert_eq!(pick(&segs, 5, None).len(), 3);
        assert_eq!(pick(&segs, 600, None).len(), 9, "everything there is");
        assert_eq!(pick(&segs, 600, Some(7)).len(), 2, "a recording keeps from its start");
        assert!(pick(&segs[..1], 30, None).is_empty());
    }

    #[test]
    fn arguments_for_each_encoder() {
        let o = Options { video: Video::Desktop { monitor: 1 }, fps: 60, quality: Quality::High, encoder: Encoder::Nvenc, audio: true };
        let a = record_args(&o, Path::new("buf"));
        let s = a.join(" ");
        assert!(s.contains("ddagrab=output_idx=1:framerate=60:draw_mouse=1"));
        assert!(s.contains("-c:v h264_nvenc") && s.contains("-cq 21") && s.contains("-g 120"));
        assert!(s.contains("-i pipe:0") && s.contains("-map 1:a"));
        assert!(a.last().unwrap().ends_with("seg%06d.ts"));
        let p = probe_args(Encoder::Qsv, 0).join(" ");
        assert!(p.contains("h264_qsv") && p.ends_with("-frames:v 10 -f null -") && !p.contains("segment"));
        assert!(record_args(&Options { encoder: Encoder::Mf, audio: false, ..o }, Path::new("b")).join(" ").contains("hwdownload,format=bgra,format=nv12 -c:v h264_mf"));
    }

    struct Tone(u64);
    impl Sound for Tone {
        fn read(&mut self, out: &mut Vec<f32>) {
            // 10 ms of a 440 Hz tone, then pauses: silence must be filled in.
            if self.0.is_multiple_of(3) {
                for i in 0..480 {
                    let v = ((self.0 * 480 + i) as f32 * 440.0 * std::f32::consts::TAU / AUDIO_RATE as f32).sin() * 0.3;
                    out.extend([v, v]);
                }
            }
            self.0 += 1;
        }
    }

    /// The whole pipeline with a real FFmpeg, when one is installed (CI has none on Windows).
    #[test]
    fn records_and_saves_with_ffmpeg() {
        let Ok(out) = Command::new("ffmpeg").arg("-version").output() else { return };
        if !out.status.success() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let o = Options { video: Video::Test, fps: 15, quality: Quality::Standard, encoder: Encoder::Software, audio: true };
        let r = Recorder::start(Path::new("ffmpeg"), &dir.path().join("buf"), &o, 4, Some(Box::new(|| Some(Box::new(Tone(0)) as Box<dyn Sound>)))).unwrap();
        std::thread::sleep(Duration::from_millis(7500));
        r.alive().unwrap();
        let clip = dir.path().join("clips").join("clip.mp4");
        r.save(4, &clip).unwrap();
        let probe = Command::new("ffprobe").args(["-v", "error", "-show_entries", "stream=codec_type:format=duration", "-of", "csv=p=0"]).arg(&clip).output().unwrap();
        let info = String::from_utf8_lossy(&probe.stdout).to_string();
        assert!(info.contains("video") && info.contains("audio"), "{info}");
        let dur: f64 = info.lines().filter_map(|l| l.trim().parse().ok()).next_back().unwrap();
        assert!((3.0..=6.5).contains(&dur), "a 4 s clip lasted {dur} s");
        // Old segments go; the replay keeps about 4 s (+2 segments).
        std::thread::sleep(Duration::from_millis(1500));
        assert!(segments(&dir.path().join("buf")).len() <= 6);
        r.start_recording();
        std::thread::sleep(Duration::from_millis(4500));
        let rec = dir.path().join("clips").join("rec.mp4");
        r.stop_recording(&rec).unwrap();
        assert!(rec.is_file());
        drop(r);
        assert!(!dir.path().join("buf").exists(), "the buffer is removed when it stops");
    }
}
