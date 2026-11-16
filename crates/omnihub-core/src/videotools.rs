//! Video tools: shrink a video (or make it fit a size, like Discord's 10 MB),
//! change its format, cut a part out, or take the sound out. The FFmpeg that
//! instant replay downloads does the work, on this PC; the original is never
//! changed.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Size {
    Small,
    #[default]
    Balanced,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Format {
    Mp4,
    Webm,
    Gif,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AudioFormat {
    Mp3,
    M4a,
    Wav,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Job {
    /// Smaller MP4; `target_mb` makes it fit that size (it shrinks the picture if it must).
    #[serde(rename_all = "camelCase")]
    Compress { size: Size, max_height: Option<u32>, target_mb: Option<f64> },
    Convert { to: Format },
    /// Seconds. `exact` encodes again for a frame-exact cut; otherwise it cuts at keyframes, with no loss.
    Trim { start: f64, end: f64, exact: bool },
    Audio { to: AudioFormat },
}

impl Job {
    fn label(&self) -> String {
        match self {
            Job::Compress { target_mb: Some(mb), .. } => format!("Fit in {mb} MB"),
            Job::Compress { .. } => "Compress".into(),
            Job::Convert { to } => format!("To {}", format!("{to:?}").to_uppercase()),
            Job::Trim { start, end, .. } => format!("Cut {} – {}", clock(*start), clock(*end)),
            Job::Audio { to } => format!("Sound as {}", format!("{to:?}").to_uppercase()),
        }
    }
}

fn clock(s: f64) -> String {
    let s = s.max(0.0) as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// What FFmpeg says is in the file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    /// kb/s for the whole file.
    pub bitrate: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub source: String,
    pub output: String,
    pub label: String,
    /// 0–100.
    pub percent: f32,
    pub state: State,
    pub error: Option<String>,
    pub before: u64,
    pub after: u64,
    pub started_at: i64,
}

/// H.264 encoders in the order tried: the graphics card's, then Windows', then the processor's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Encoder {
    Nvenc,
    Amf,
    Qsv,
    Mf,
    OpenH264,
    Mpeg4,
}

impl Encoder {
    const ALL: [Encoder; 6] = [Encoder::Nvenc, Encoder::Amf, Encoder::Qsv, Encoder::Mf, Encoder::OpenH264, Encoder::Mpeg4];

    fn args(self, kbps: u32) -> Vec<String> {
        let (name, pix): (&str, &str) = match self {
            Encoder::Nvenc => ("h264_nvenc", "yuv420p"),
            Encoder::Amf => ("h264_amf", "nv12"),
            Encoder::Qsv => ("h264_qsv", "nv12"),
            Encoder::Mf => ("h264_mf", "nv12"),
            Encoder::OpenH264 => ("libopenh264", "yuv420p"),
            Encoder::Mpeg4 => ("mpeg4", "yuv420p"),
        };
        let mut a: Vec<String> = vec!["-c:v".into(), name.into(), "-pix_fmt".into(), pix.into()];
        match self {
            Encoder::Nvenc => a.extend(["-preset", "p6", "-rc", "vbr"].map(String::from)),
            Encoder::Amf => a.extend(["-quality", "quality", "-rc", "vbr_peak"].map(String::from)),
            Encoder::Qsv => a.extend(["-preset", "slower"].map(String::from)),
            _ => {}
        }
        a.extend(["-b:v".into(), format!("{kbps}k"), "-maxrate".into(), format!("{}k", kbps * 3 / 2), "-bufsize".into(), format!("{}k", kbps * 2)]);
        a
    }
}

pub struct VideoTools {
    ffmpeg: PathBuf,
    events: EventBus,
    tasks: Arc<Mutex<Vec<Task>>>,
    cancel: Arc<Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>>,
    encoder: Mutex<Option<Encoder>>,
}

impl VideoTools {
    pub fn new(ffmpeg: PathBuf, events: EventBus) -> Self {
        VideoTools { ffmpeg, events, tasks: Default::default(), cancel: Default::default(), encoder: Mutex::new(None) }
    }

    pub fn ready(&self) -> bool {
        self.ffmpeg.is_file()
    }

    fn need(&self) -> Result<(), String> {
        if self.ready() {
            Ok(())
        } else {
            Err("Video tools use FFmpeg. Download it once from Tools → Video (it's the same one instant replay uses).".into())
        }
    }

    pub fn tasks(&self) -> Vec<Task> {
        self.tasks.lock().iter().rev().cloned().collect()
    }

    /// Forget finished tasks.
    pub fn clear(&self) {
        self.tasks.lock().retain(|t| t.state == State::Running);
        self.events.emit("video:changed", serde_json::json!({}));
    }

    pub fn cancel(&self, id: &str) {
        if let Some(c) = self.cancel.lock().get(id) {
            c.store(true, Ordering::SeqCst);
        }
    }

    pub fn probe(&self, path: &Path) -> Result<Media, String> {
        self.need()?;
        let out = hidden(Command::new(&self.ffmpeg)).args(["-hide_banner", "-i"]).arg(path).stdin(Stdio::null()).output().map_err(|e| format!("Couldn't run FFmpeg: {e}"))?;
        let text = String::from_utf8_lossy(&out.stderr);
        parse_probe(&text).ok_or_else(|| {
            let last = text.lines().last().unwrap_or("").to_string();
            if last.contains("Invalid data") {
                "This isn't a video FFmpeg can read.".to_string()
            } else {
                format!("Couldn't read the video: {last}")
            }
        })
    }

    fn encoder(&self) -> Encoder {
        let mut e = self.encoder.lock();
        *e.get_or_insert_with(|| {
            Encoder::ALL
                .into_iter()
                .find(|enc| {
                    let mut a: Vec<String> = ["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc=size=640x360:rate=30", "-frames:v", "15"].map(String::from).to_vec();
                    a.extend(enc.args(1500));
                    a.extend(["-f", "null", "-"].map(String::from));
                    hidden(Command::new(&self.ffmpeg)).args(&a).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
                })
                .unwrap_or(Encoder::Mpeg4)
        })
    }

    /// Start `job` on `path`; returns the task's id. Progress comes as `video:changed` events.
    pub fn start(&self, path: &Path, job: Job) -> Result<String, String> {
        self.need()?;
        if !path.is_file() {
            return Err(format!("{} isn't there any more.", path.display()));
        }
        let media = self.probe(path)?;
        let encoder = if needs_h264(&job, &media) { self.encoder() } else { Encoder::Mpeg4 };
        let plan = plan(path, &job, &media, encoder)?;
        let id = crate::remote::auth::random_token(10);
        let task = Task {
            id: id.clone(),
            source: path.display().to_string(),
            output: plan.output.display().to_string(),
            label: job.label(),
            percent: 0.0,
            state: State::Running,
            error: None,
            before: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
            after: 0,
            started_at: chrono::Utc::now().timestamp(),
        };
        self.tasks.lock().push(task.clone());
        let stop = Arc::new(AtomicBool::new(false));
        self.cancel.lock().insert(id.clone(), stop.clone());
        let (ffmpeg, events, tasks, cancel) = (self.ffmpeg.clone(), self.events.clone(), self.tasks.clone(), self.cancel.clone());
        events.emit("video:changed", &task);
        std::thread::spawn(move || {
            let update = |f: &dyn Fn(&mut Task)| {
                let mut l = tasks.lock();
                if let Some(t) = l.iter_mut().find(|t| t.id == task.id) {
                    f(t);
                    events.emit("video:changed", &*t);
                }
            };
            let r = run(&ffmpeg, &plan, &stop, &|p| update(&|t| t.percent = p));
            cancel.lock().remove(&task.id);
            match r {
                Ok(()) => {
                    let after = std::fs::metadata(&plan.output).map(|m| m.len()).unwrap_or(0);
                    update(&|t| {
                        t.state = State::Done;
                        t.percent = 100.0;
                        t.after = after;
                    });
                    events.emit("video:done", serde_json::json!({ "label": task.label, "output": plan.output.display().to_string() }));
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&plan.output);
                    let cancelled = stop.load(Ordering::SeqCst);
                    update(&|t| {
                        t.state = if cancelled { State::Cancelled } else { State::Failed };
                        t.error = (!cancelled).then(|| e.clone());
                    });
                }
            }
        });
        Ok(id)
    }
}

fn needs_h264(job: &Job, m: &Media) -> bool {
    match job {
        Job::Compress { .. } => true,
        Job::Trim { exact, .. } => *exact,
        Job::Convert { to: Format::Mp4 } => !can_copy_to_mp4(m),
        _ => false,
    }
}

fn can_copy_to_mp4(m: &Media) -> bool {
    matches!(m.video_codec.as_deref(), Some("h264" | "hevc" | "av1")) && matches!(m.audio_codec.as_deref(), None | Some("aac" | "mp3"))
}

/// The arguments and where the result goes.
#[derive(Debug)]
struct Plan {
    args: Vec<String>,
    output: PathBuf,
    /// Seconds of output (for the progress).
    length: f64,
}

fn plan(path: &Path, job: &Job, m: &Media, encoder: Encoder) -> Result<Plan, String> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("video");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("mp4").to_ascii_lowercase();
    let mut a: Vec<String> = ["-hide_banner", "-loglevel", "error", "-nostats", "-progress", "pipe:1", "-y"].map(String::from).to_vec();
    let input = path.to_string_lossy().into_owned();
    let mut length = m.duration;
    let has_video = m.video_codec.is_some() && m.width > 0;
    let (name, args): (String, Vec<String>) = match *job {
        Job::Compress { size, max_height, target_mb } => {
            if !has_video {
                return Err("This file has no picture to compress.".into());
            }
            let audio = if target_mb.is_some_and(|t| t < 25.0) { 96 } else { 128 };
            let fps = if m.fps > 1.0 { m.fps.min(120.0) } else { 30.0 };
            let mut height = max_height.map_or(m.height, |h| h.min(m.height));
            let bpp = match size {
                Size::Small => 0.045,
                Size::Balanced => 0.07,
                Size::High => 0.11,
            };
            let width_at = |h: u32| (m.width as f64 * h as f64 / m.height.max(1) as f64) as u32;
            let mut kbps = (width_at(height) as f64 * height as f64 * fps * bpp / 1000.0) as u32;
            if let Some(mb) = target_mb {
                if m.duration <= 0.0 {
                    return Err("FFmpeg couldn't tell how long the video is.".into());
                }
                let total = mb * 8_000.0 * 0.94 / m.duration; // kb/s for everything
                let video = total - audio as f64;
                if video < 80.0 {
                    return Err(format!("{} is too long to fit in {mb} MB. Cut it shorter first, or pick a bigger size.", clock(m.duration)));
                }
                kbps = video as u32;
                // Smaller picture when the budget is thin, so it doesn't turn to mush.
                for h in [height, 1080, 720, 540, 480, 360, 240] {
                    if h <= height && (width_at(h) as f64 * h as f64 * fps * 0.035 / 1000.0) <= video {
                        height = h;
                        break;
                    }
                    height = h.min(height);
                }
            } else if let Some(src) = m.bitrate.filter(|b| *b > 200) {
                // Never bigger than it was.
                kbps = kbps.min(src * 85 / 100);
            }
            let kbps = kbps.clamp(80, 60_000);
            let mut v = vec!["-i".into(), input, "-map".into(), "0:v:0".into(), "-map".into(), "0:a:0?".into()];
            if height < m.height {
                v.extend(["-vf".into(), format!("scale=-2:{}", height & !1)]);
            }
            v.extend(encoder.args(kbps));
            v.extend(["-c:a".into(), "aac".into(), "-b:a".into(), format!("{audio}k"), "-movflags".into(), "+faststart".into()]);
            let suffix = target_mb.map_or(" (small)".to_string(), |mb| format!(" ({mb} MB)"));
            (format!("{stem}{suffix}.mp4"), v)
        }
        Job::Convert { to } => {
            let mut v = vec!["-i".into(), input];
            match to {
                Format::Mp4 if can_copy_to_mp4(m) => v.extend(["-map", "0:v:0", "-map", "0:a:0?", "-c", "copy", "-movflags", "+faststart"].map(String::from)),
                Format::Mp4 => {
                    let kbps = ((m.width.max(640) as f64 * m.height.max(360) as f64 * m.fps.clamp(24.0, 60.0) * 0.08 / 1000.0) as u32).clamp(500, 40_000);
                    v.extend(["-map", "0:v:0", "-map", "0:a:0?"].map(String::from));
                    v.extend(encoder.args(kbps));
                    v.extend(["-c:a", "aac", "-b:a", "160k", "-movflags", "+faststart"].map(String::from));
                }
                Format::Webm => {
                    let kbps = ((m.width.max(640) as f64 * m.height.max(360) as f64 * m.fps.clamp(24.0, 60.0) * 0.06 / 1000.0) as u32).clamp(400, 30_000);
                    v.extend(["-map", "0:v:0", "-map", "0:a:0?", "-c:v", "libvpx-vp9", "-b:v"].map(String::from));
                    v.push(format!("{kbps}k"));
                    v.extend(["-deadline", "good", "-cpu-used", "4", "-row-mt", "1", "-c:a", "libopus", "-b:a", "128k"].map(String::from));
                }
                Format::Gif => {
                    if !has_video {
                        return Err("This file has no picture.".into());
                    }
                    v.extend(["-vf", "fps=12,scale='min(480,iw)':-2:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=4", "-an", "-loop", "0"].map(String::from));
                }
            }
            let new_ext = format!("{to:?}").to_lowercase();
            let name = if new_ext == ext { format!("{stem} (converted).{new_ext}") } else { format!("{stem}.{new_ext}") };
            (name, v)
        }
        Job::Trim { start, end, exact } => {
            let start = start.max(0.0);
            let end = if m.duration > 0.0 { end.min(m.duration) } else { end };
            if end - start < 0.1 {
                return Err("Pick an end after the start.".into());
            }
            length = end - start;
            let mut v = vec!["-ss".into(), format!("{start:.3}"), "-i".into(), input, "-t".into(), format!("{length:.3}")];
            if exact && has_video {
                let kbps = ((m.width as f64 * m.height as f64 * m.fps.clamp(24.0, 60.0) * 0.11 / 1000.0) as u32).clamp(500, 60_000);
                v.extend(["-map", "0:v:0", "-map", "0:a:0?"].map(String::from));
                v.extend(encoder.args(kbps));
                v.extend(["-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart"].map(String::from));
                (format!("{stem} (cut).mp4"), v)
            } else {
                v.extend(["-map", "0:v?", "-map", "0:a?", "-c", "copy", "-avoid_negative_ts", "make_zero"].map(String::from));
                (format!("{stem} (cut).{ext}"), v)
            }
        }
        Job::Audio { to } => {
            if m.audio_codec.is_none() {
                return Err("This video has no sound.".into());
            }
            let mut v = vec!["-i".into(), input, "-vn".into(), "-map".into(), "0:a:0".into()];
            match to {
                AudioFormat::Mp3 => v.extend(["-c:a", "libmp3lame", "-q:a", "2"].map(String::from)),
                AudioFormat::M4a => v.extend(["-c:a", "aac", "-b:a", "192k"].map(String::from)),
                AudioFormat::Wav => v.extend(["-c:a", "pcm_s16le"].map(String::from)),
            }
            (format!("{stem}.{}", format!("{to:?}").to_lowercase()), v)
        }
    };
    a.extend(args);
    let output = crate::remote::transfer::unique_path(dir, &name);
    a.push(output.to_string_lossy().into_owned());
    Ok(Plan { args: a, output, length })
}

fn run(ffmpeg: &Path, plan: &Plan, stop: &AtomicBool, progress: &dyn Fn(f32)) -> Result<(), String> {
    let mut child = hidden(Command::new(ffmpeg)).args(&plan.args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("Couldn't run FFmpeg: {e}"))?;
    let stderr = child.stderr.take();
    let errors = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(mut e) = stderr {
            let _ = std::io::Read::read_to_string(&mut e, &mut s);
        }
        s
    });
    let mut last = Instant::now();
    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if stop.load(Ordering::SeqCst) {
                let _ = child.kill();
                break;
            }
            let us = line.strip_prefix("out_time_us=").or_else(|| line.strip_prefix("out_time_ms=")).and_then(|v| v.trim().parse::<f64>().ok());
            if let Some(us) = us {
                if plan.length > 0.0 && last.elapsed().as_millis() >= 400 {
                    last = Instant::now();
                    progress(((us / 1e6 / plan.length * 100.0) as f32).clamp(0.0, 99.0));
                }
            }
        }
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    let err = errors.join().unwrap_or_default();
    if stop.load(Ordering::SeqCst) {
        return Err("Cancelled.".into());
    }
    if !status.success() || !plan.output.is_file() {
        let last = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("unknown error").trim().to_string();
        return Err(format!("FFmpeg stopped: {last}"));
    }
    Ok(())
}

fn parse_probe(text: &str) -> Option<Media> {
    let mut m = Media::default();
    let mut any = false;
    for line in text.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("Duration: ") {
            any = true;
            let d = rest.split(',').next().unwrap_or("");
            let p: Vec<f64> = d.split(':').filter_map(|x| x.trim().parse().ok()).collect();
            if p.len() == 3 {
                m.duration = p[0] * 3600.0 + p[1] * 60.0 + p[2];
            }
            m.bitrate = rest.split("bitrate: ").nth(1).and_then(|b| b.split_whitespace().next()).and_then(|b| b.parse().ok());
        } else if l.starts_with("Stream #") {
            if let Some(v) = l.split("Video: ").nth(1).filter(|_| m.video_codec.is_none()) {
                // Cover art inside an MP3/M4A isn't a video.
                if l.contains("(attached pic)") {
                    continue;
                }
                any = true;
                m.video_codec = v.split([' ', ',']).next().map(String::from);
                for tok in v.split([',', ' ', '[']) {
                    if let Some((w, h)) = tok.split_once('x') {
                        if let (Ok(w), Ok(h)) = (w.parse::<u32>(), h.parse::<u32>()) {
                            if (16..20_000).contains(&w) && (16..20_000).contains(&h) {
                                (m.width, m.height) = (w, h);
                                break;
                            }
                        }
                    }
                }
                let parts: Vec<&str> = v.split(',').map(str::trim).collect();
                m.fps = parts.iter().find_map(|p| p.strip_suffix(" fps")).or_else(|| parts.iter().find_map(|p| p.strip_suffix(" tbr"))).and_then(|f| f.trim().parse().ok()).unwrap_or(0.0);
            } else if let Some(a) = l.split("Audio: ").nth(1).filter(|_| m.audio_codec.is_none()) {
                any = true;
                m.audio_codec = a.split([' ', ',']).next().map(String::from);
            }
        }
    }
    any.then_some(m)
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

#[cfg(test)]
mod tests {
    use super::*;

    const PROBE: &str = "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'a.mp4':
  Duration: 00:01:02.50, start: 0.000000, bitrate: 8123 kb/s
  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(tv, bt709, progressive), 1920x1080 [SAR 1:1 DAR 16:9], 7990 kb/s, 29.97 fps, 29.97 tbr, 30k tbn (default)
  Stream #0:1[0x2](und): Audio: aac (LC) (mp4a / 0x6134706D), 48000 Hz, stereo, fltp, 128 kb/s (default)
At least one output file must be specified";

    #[test]
    fn reads_ffmpeg_output() {
        let m = parse_probe(PROBE).unwrap();
        assert_eq!(m, Media { duration: 62.5, width: 1920, height: 1080, fps: 29.97, video_codec: Some("h264".into()), audio_codec: Some("aac".into()), bitrate: Some(8123) });
        assert!(can_copy_to_mp4(&m));
        let song = "  Duration: 00:03:00.00, start: 0.0, bitrate: 320 kb/s\n  Stream #0:0: Audio: mp3, 44100 Hz, stereo, fltp, 320 kb/s\n  Stream #0:1: Video: mjpeg (Baseline), yuvj420p, 600x600, 90k tbr (attached pic)";
        let m = parse_probe(song).unwrap();
        assert!(m.video_codec.is_none() && m.audio_codec.as_deref() == Some("mp3"));
        assert!(parse_probe("a.txt: Invalid data found when processing input").is_none());
    }

    #[test]
    fn plans() {
        let m = parse_probe(PROBE).unwrap();
        let p = Path::new("/v/clip.mp4");
        // Fit in 10 MB: about 1.07 Mb/s for the picture, so it drops to 720 lines.
        let pl = plan(p, &Job::Compress { size: Size::Balanced, max_height: None, target_mb: Some(10.0) }, &m, Encoder::Nvenc).unwrap();
        let s = pl.args.join(" ");
        assert!(s.contains("-c:v h264_nvenc") && s.contains("-b:v 1107k") && s.contains("scale=-2:720") && s.contains("-b:a 96k"), "{s}");
        assert!(pl.output.ends_with("clip (10 MB).mp4"));
        assert!(plan(p, &Job::Compress { size: Size::Small, max_height: None, target_mb: Some(0.5) }, &m, Encoder::Mf).unwrap_err().contains("too long"));
        // Without a target, bits per pixel (and never more than 85% of what it was).
        let s = plan(p, &Job::Compress { size: Size::High, max_height: None, target_mb: None }, &m, Encoder::Mf).unwrap().args.join(" ");
        assert!(s.contains("-b:v 6836k") && !s.contains("scale"), "{s}");
        let s = plan(p, &Job::Convert { to: Format::Mp4 }, &m, Encoder::Mpeg4).unwrap();
        assert!(s.args.join(" ").contains("-c copy") && s.output.ends_with("clip (converted).mp4"));
        let s = plan(p, &Job::Trim { start: 10.0, end: 100.0, exact: false }, &m, Encoder::Mpeg4).unwrap();
        assert_eq!(s.length, 52.5);
        assert!(s.args.join(" ").starts_with("-hide_banner -loglevel error -nostats -progress pipe:1 -y -ss 10.000 -i /v/clip.mp4 -t 52.500"));
        assert!(plan(p, &Job::Trim { start: 5.0, end: 5.0, exact: false }, &m, Encoder::Mpeg4).is_err());
        assert!(plan(p, &Job::Audio { to: AudioFormat::Mp3 }, &Media::default(), Encoder::Mpeg4).unwrap_err().contains("no sound"));
    }

    /// The real thing, with the FFmpeg on this machine (skipped without one).
    #[test]
    fn runs_with_ffmpeg() {
        let Ok(out) = Command::new("which").arg("ffmpeg").output() else { return };
        if !out.status.success() {
            return;
        }
        let ffmpeg = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("test.mp4");
        let ok = Command::new(&ffmpeg)
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc=size=640x360:rate=30:duration=4", "-f", "lavfi", "-i", "sine=duration=4", "-c:v", "mpeg4", "-q:v", "2", "-c:a", "aac", "-shortest"])
            .arg(&src)
            .status()
            .unwrap();
        assert!(ok.success());
        let tools = VideoTools::new(ffmpeg, EventBus::new());
        let m = tools.probe(&src).unwrap();
        assert_eq!((m.width, m.height, m.video_codec.as_deref()), (640, 360, Some("mpeg4")));
        assert!((m.duration - 4.0).abs() < 0.2);
        let wait = |id: String| {
            for _ in 0..600 {
                let t = tools.tasks().into_iter().find(|t| t.id == id).unwrap();
                if t.state != State::Running {
                    return t;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            panic!("took too long");
        };
        let t = wait(tools.start(&src, Job::Compress { size: Size::Small, max_height: Some(240), target_mb: None }).unwrap());
        assert_eq!(t.state, State::Done, "{:?}", t.error);
        let small = tools.probe(Path::new(&t.output)).unwrap();
        assert_eq!(small.height, 240);
        let t = wait(tools.start(&src, Job::Trim { start: 1.0, end: 3.0, exact: false }).unwrap());
        assert_eq!(t.state, State::Done, "{:?}", t.error);
        assert!(t.output.ends_with("test (cut).mp4"));
        let t = wait(tools.start(&src, Job::Audio { to: AudioFormat::M4a }).unwrap());
        assert_eq!(t.state, State::Done, "{:?}", t.error);
        assert!(tools.probe(Path::new(&t.output)).unwrap().video_codec.is_none());
        let t = wait(tools.start(&src, Job::Convert { to: Format::Gif }).unwrap());
        assert_eq!(t.state, State::Done, "{:?}", t.error);
        assert!(std::fs::read(&t.output).unwrap().starts_with(b"GIF8"));
    }
}
