//! Frames per second while a game runs, measured by Intel's PresentMon
//! (MIT license), the tool hardware reviewers use. It reads the frame
//! timings Windows itself records for every frame a game shows, so it works
//! with any game, anti-cheat included, without touching the game.
//!
//! Each release publishes PresentMon next to the installers
//! (`OmniHub-PresentMon-x64.exe`) and this build pins its checksum; it is
//! downloaded the first time the meter is turned on. Windows lets
//! administrators and members of "Performance Log Users" read frame
//! timings: one approval adds you to that group, which counts from your
//! next sign-in.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// The release asset.
pub const ASSET: &str = "OmniHub-PresentMon-x64.exe";
/// Our ETW session, so a capture never collides with another PresentMon.
const SESSION: &str = "OmniHubFps";

/// SHA-256 of the PresentMon published with this release, when the build
/// knows it.
pub fn pinned_sha256() -> Option<&'static str> {
    option_env!("OMNIHUB_PRESENTMON_SHA256").filter(|s| s.len() == 64)
}

/// The arguments for capturing one process (checked by the release
/// workflow against the PresentMon it publishes).
pub fn capture_args(pid: u32) -> Vec<String> {
    ["--process_id", &pid.to_string(), "--output_stdout", "--no_console_stats", "--v1_metrics", "--no_track_gpu", "--no_track_input", "--session_name", SESSION, "--stop_existing_session", "--terminate_on_proc_exit"].iter().map(|s| s.to_string()).collect()
}

/// The last second, for the live readout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Live {
    pub fps: f32,
    /// 1% low over the last ten seconds.
    pub low1: f32,
    /// Average frame time, ms.
    pub frame_ms: f32,
    /// The slowest frame of the second, ms.
    pub worst_ms: f32,
}

/// A whole session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub frames: u64,
    pub seconds: f32,
    pub avg: f32,
    /// FPS at the slowest 1% of frames (99th percentile frame time).
    pub low1: f32,
    /// The slowest 0.1%.
    pub low01: f32,
    /// Frames that took more than 2.5× the average (and over 8 ms): the
    /// stutters you notice.
    pub hitches: u32,
}

/// 0.01 ms steps up to 100 ms; slower frames are kept as they are.
const STEP_MS: f64 = 0.01;
const STEPS: usize = 10_000;
/// The live 1% low looks back this far.
const RECENT_MS: f64 = 10_000.0;

/// Frame times as they arrive.
#[derive(Debug, Default)]
pub struct Stats {
    hist: Vec<u32>,
    slow: Vec<f32>,
    frames: u64,
    total_ms: f64,
    hitches: u32,
    recent: VecDeque<f32>,
    recent_ms: f64,
    second: (u32, f64, f32),
}

impl Stats {
    pub fn add(&mut self, ms: f32) {
        // PresentMon reports 0 for a swap chain's first frame.
        if !ms.is_finite() || ms <= 0.0 || ms > 60_000.0 {
            return;
        }
        if self.frames >= 30 {
            let avg = self.total_ms / self.frames as f64;
            if f64::from(ms) > (avg * 2.5).max(8.0) {
                self.hitches += 1;
            }
        }
        if self.hist.is_empty() {
            self.hist = vec![0; STEPS];
        }
        let i = (f64::from(ms) / STEP_MS) as usize;
        if i < STEPS {
            self.hist[i] += 1;
        } else {
            self.slow.push(ms);
        }
        self.frames += 1;
        self.total_ms += f64::from(ms);
        self.recent.push_back(ms);
        self.recent_ms += f64::from(ms);
        while self.recent_ms > RECENT_MS && self.recent.len() > 1 {
            self.recent_ms -= f64::from(self.recent.pop_front().unwrap_or(0.0));
        }
        self.second.0 += 1;
        self.second.1 += f64::from(ms);
        self.second.2 = self.second.2.max(ms);
    }

    /// The second since the last tick (None when no frame arrived).
    pub fn tick(&mut self) -> Option<Live> {
        let (n, ms, worst) = std::mem::take(&mut self.second);
        if n == 0 || ms <= 0.0 {
            return None;
        }
        let mut recent: Vec<f32> = self.recent.iter().copied().collect();
        recent.sort_by(f32::total_cmp);
        let p99 = recent[((recent.len() as f64 * 0.99) as usize).min(recent.len() - 1)];
        Some(Live { fps: (f64::from(n) * 1000.0 / ms) as f32, low1: 1000.0 / p99, frame_ms: (ms / f64::from(n)) as f32, worst_ms: worst })
    }

    /// The frame time below which a share `q` of all frames fall.
    fn percentile(&self, q: f64) -> Option<f32> {
        if self.frames == 0 {
            return None;
        }
        let want = ((self.frames as f64 * q).ceil() as u64).max(1);
        let mut seen = 0u64;
        for (i, n) in self.hist.iter().enumerate() {
            seen += u64::from(*n);
            if seen >= want {
                return Some(((i as f64 + 0.5) * STEP_MS) as f32);
            }
        }
        let mut slow = self.slow.clone();
        slow.sort_by(f32::total_cmp);
        slow.get((want - seen - 1) as usize).or(slow.last()).copied()
    }

    pub fn summary(&self) -> Option<Summary> {
        if self.frames < 2 || self.total_ms <= 0.0 {
            return None;
        }
        Some(Summary {
            frames: self.frames,
            seconds: (self.total_ms / 1000.0) as f32,
            avg: (self.frames as f64 * 1000.0 / self.total_ms) as f32,
            low1: 1000.0 / self.percentile(0.99)?,
            low01: 1000.0 / self.percentile(0.999)?,
            hitches: self.hitches,
        })
    }
}

/// PresentMon's CSV, one line at a time.
#[derive(Debug, Default)]
pub struct CsvReader {
    ms: Option<usize>,
    swap: Option<usize>,
    /// Frames per swap chain: a game can have more than one (a launcher
    /// overlay, a loading screen); the busiest is the game.
    counts: std::collections::HashMap<String, u64>,
    main: Option<String>,
}

impl CsvReader {
    /// The frame time on this line, if it belongs to the game's main swap
    /// chain.
    pub fn line(&mut self, line: &str) -> Option<f32> {
        let cols: Vec<&str> = line.trim_end().split(',').collect();
        if cols.first().is_some_and(|c| c.eq_ignore_ascii_case("Application")) {
            let find = |names: &[&str]| cols.iter().position(|c| names.iter().any(|n| c.trim().eq_ignore_ascii_case(n)));
            self.ms = find(&["MsBetweenPresents", "MsBetweenAppStart", "FrameTime"]);
            self.swap = find(&["SwapChainAddress"]);
            return None;
        }
        let ms: f32 = cols.get(self.ms?)?.trim().parse().ok()?;
        let swap = self.swap.and_then(|i| cols.get(i)).map(|s| s.trim().to_string()).unwrap_or_default();
        let n = {
            let c = self.counts.entry(swap.clone()).or_default();
            *c += 1;
            *c
        };
        let main_count = self.main.as_ref().and_then(|m| self.counts.get(m)).copied().unwrap_or(0);
        if self.main.is_none() || n > main_count {
            self.main = Some(swap.clone());
        }
        (self.main.as_deref() == Some(swap.as_str())).then_some(ms)
    }

    pub fn has_header(&self) -> bool {
        self.ms.is_some()
    }
}

fn command(exe: &Path) -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut c = Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c
}

/// A running capture.
pub struct Meter {
    exe: PathBuf,
    child: Child,
    reader: Option<JoinHandle<()>>,
}

impl Meter {
    /// Start measuring process `pid`. Frames go into `stats`; `on_tick`
    /// gets the live figures once a second while frames arrive.
    pub fn start(exe: &Path, pid: u32, stats: Arc<Mutex<Stats>>, mut on_tick: impl FnMut(Live) + Send + 'static) -> Result<Meter, String> {
        let mut child = command(exe).args(capture_args(pid)).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().map_err(|e| format!("Could not start PresentMon: {e}"))?;
        let stdout = child.stdout.take().ok_or("PresentMon has no output")?;
        let mut stderr = child.stderr.take().ok_or("PresentMon has no output")?;
        let errors = Arc::new(Mutex::new(String::new()));
        let errs = errors.clone();
        std::thread::spawn(move || {
            let mut buf = String::new();
            let _ = stderr.read_to_string(&mut buf);
            *errs.lock() = buf;
        });
        let header = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let seen = header.clone();
        let reader = std::thread::Builder::new()
            .name("fps-meter".into())
            .spawn(move || {
                let mut csv = CsvReader::default();
                let mut last = Instant::now();
                for line in BufReader::new(stdout).lines() {
                    let Ok(line) = line else { break };
                    if let Some(ms) = csv.line(&line) {
                        stats.lock().add(ms);
                    } else if csv.has_header() {
                        seen.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    if last.elapsed() >= Duration::from_secs(1) {
                        last = Instant::now();
                        let live = stats.lock().tick();
                        if let Some(l) = live {
                            on_tick(l);
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        let mut m = Meter { exe: exe.to_path_buf(), child, reader: Some(reader) };
        // Access problems end it within moments; say why.
        let t = Instant::now();
        while t.elapsed() < Duration::from_millis(2500) && !header.load(std::sync::atomic::Ordering::Relaxed) {
            if let Ok(Some(status)) = m.child.try_wait() {
                if let Some(r) = m.reader.take() {
                    let _ = r.join();
                }
                std::thread::sleep(Duration::from_millis(50));
                let text = errors.lock().clone();
                return Err(explain(&text, status.code()));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(m)
    }

    /// Still capturing (it ends by itself when the game closes).
    pub fn running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// End the capture, letting PresentMon close its trace session.
    pub fn stop(mut self) {
        if self.running() {
            let _ = command(&self.exe).args(["--terminate_existing_session", "--session_name", SESSION]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
            let t = Instant::now();
            while self.running() && t.elapsed() < Duration::from_secs(3) {
                std::thread::sleep(Duration::from_millis(50));
            }
            if self.running() {
                let _ = self.child.kill();
            }
        }
        let _ = self.child.wait();
        if let Some(r) = self.reader.take() {
            let _ = r.join();
        }
    }
}

fn explain(stderr: &str, code: Option<i32>) -> String {
    let low = stderr.to_ascii_lowercase();
    if low.contains("access denied") || low.contains("access is denied") {
        return "Windows didn't let OmniHub read frame timings — allow the FPS meter once in Games".into();
    }
    match stderr.lines().map(str::trim).rfind(|l| !l.is_empty()) {
        Some(l) => format!("PresentMon stopped: {l}"),
        None => format!("PresentMon stopped (code {})", code.unwrap_or(-1)),
    }
}

/// What the FPS meter needs before it can run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct FpsStatus {
    pub installed: bool,
    /// Windows lets OmniHub read frame timings.
    pub allowed: bool,
    /// Approved, but it counts from the next sign-in.
    pub sign_out_needed: bool,
    pub supported: bool,
}

/// One session's result, kept to compare boosts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FpsRecord {
    pub profile_id: String,
    pub name: String,
    pub mode: super::BoostMode,
    pub at: i64,
    pub summary: Summary,
}

pub use imp::{allowed, helper_admin, request_access};

#[cfg(windows)]
mod imp {
    use windows::core::PWSTR;
    use windows::Win32::Security::{CheckTokenMembership, CreateWellKnownSid, LookupAccountSidW, WinBuiltinPerfLoggingUsersSid, PSID, SID_NAME_USE};

    fn group_sid(buf: &mut [u8; 68]) -> Option<PSID> {
        let mut size = buf.len() as u32;
        let sid = PSID(buf.as_mut_ptr().cast());
        unsafe { CreateWellKnownSid(WinBuiltinPerfLoggingUsersSid, None, Some(sid), &mut size) }.ok()?;
        Some(sid)
    }

    /// Administrator, or a member of "Performance Log Users" in this sign-in.
    pub fn allowed() -> bool {
        if crate::system::elevation::is_elevated() {
            return true;
        }
        let mut buf = [0u8; 68];
        let Some(sid) = group_sid(&mut buf) else { return false };
        let mut member = windows::core::BOOL(0);
        unsafe { CheckTokenMembership(None, sid, &mut member) }.is_ok() && member.as_bool()
    }

    fn me() -> Option<String> {
        use windows::Win32::Security::Authentication::Identity::{GetUserNameExW, NameSamCompatible};
        let mut buf = vec![0u16; 512];
        let mut n = buf.len() as u32;
        unsafe { GetUserNameExW(NameSamCompatible, Some(PWSTR(buf.as_mut_ptr())), &mut n) }.then(|| String::from_utf16_lossy(&buf[..n as usize]))
    }

    /// Ask Windows (one administrator prompt) to add this user to
    /// "Performance Log Users".
    pub fn request_access() -> Result<(), String> {
        let user = me().ok_or("Could not tell which Windows account this is.")?;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let args = vec![crate::helper::HELPER_FLAG.to_string(), "fps-admin".into(), "allow".into(), user];
        match crate::system::elevation::run_elevated_and_wait(&exe, &args, false) {
            Ok(0) => Ok(()),
            Ok(code) => Err(format!("Windows did not add you to Performance Log Users (code {code}).")),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Err("Windows' administrator prompt was declined.".into()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Helper side (elevated): "allow DOMAIN\user".
    pub fn helper_admin(args: &[String]) -> i32 {
        let [op, user] = args else { return 64 };
        if op != "allow" || user.is_empty() || user.len() > 256 || user.chars().any(|c| c == '"' || c.is_control()) {
            return 64;
        }
        // The group's name is translated; look it up from its SID.
        let mut buf = [0u8; 68];
        let Some(sid) = group_sid(&mut buf) else { return 1 };
        let mut name = vec![0u16; 256];
        let mut domain = vec![0u16; 256];
        let (mut n, mut d) = (name.len() as u32, domain.len() as u32);
        let mut kind = SID_NAME_USE(0);
        if unsafe { LookupAccountSidW(windows::core::PCWSTR::null(), sid, Some(PWSTR(name.as_mut_ptr())), &mut n, Some(PWSTR(domain.as_mut_ptr())), &mut d, &mut kind) }.is_err() {
            return 1;
        }
        let group = String::from_utf16_lossy(&name[..n as usize]);
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("net").args(["localgroup", &group, user, "/add"]).creation_flags(0x0800_0000).output();
        match out {
            Ok(o) if o.status.success() => 0,
            // "already a member" (system error 1378).
            Ok(o) if String::from_utf8_lossy(&o.stderr).contains("1378") || String::from_utf8_lossy(&o.stdout).contains("1378") => 0,
            _ => 1,
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn allowed() -> bool {
        false
    }
    pub fn request_access() -> Result<(), String> {
        Err("Windows only".into())
    }
    pub fn helper_admin(_args: &[String]) -> i32 {
        64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_presentmon_csv() {
        let mut r = CsvReader::default();
        assert_eq!(r.line("Application,ProcessID,SwapChainAddress,PresentRuntime,SyncInterval,PresentFlags,AllowsTearing,PresentMode,TimeInSeconds,MsBetweenSimulationStart,MsBetweenPresents,MsBetweenDisplayChange"), None);
        assert!(r.has_header());
        assert_eq!(r.line("game.exe,42,0x1,DXGI,0,512,1,Hardware: Independent Flip,1.5,NA,2.083,2.083"), Some(2.083));
        // A second, quieter swap chain is not the game.
        assert_eq!(r.line("game.exe,42,0x2,DXGI,0,512,1,Composed: Flip,1.5,NA,16.6,16.6"), None);
        assert_eq!(r.line("game.exe,42,0x1,DXGI,0,512,1,Hardware: Independent Flip,1.5,NA,2.1,2.1"), Some(2.1));
        assert_eq!(r.line("garbage"), None);
        // PresentMon 1.x spelling.
        let mut v1 = CsvReader::default();
        v1.line("Application,ProcessID,SwapChainAddress,Runtime,SyncInterval,PresentFlags,Dropped,TimeInSeconds,msInPresentAPI,msBetweenPresents");
        assert_eq!(v1.line("a.exe,1,0xA,DXGI,0,0,0,0.1,0.2,3.5"), Some(3.5));
    }

    #[test]
    fn release_checks_the_same_arguments() {
        // The release workflow runs the PresentMon it publishes with these.
        let workflow = include_str!("../../../../.github/workflows/presentmon.yml");
        for arg in capture_args(1).iter().filter(|a| a.starts_with("--") || a.chars().any(char::is_alphabetic)) {
            assert!(workflow.contains(&format!("'{arg}'")), "{arg} is not checked by .github/workflows/presentmon.yml");
        }
    }

    #[test]
    fn frame_statistics() {
        let mut s = Stats::default();
        // 990 frames at 2 ms (500 FPS) and 10 at 10 ms.
        for i in 0..1000 {
            s.add(if i % 100 == 99 { 10.0 } else { 2.0 });
        }
        s.add(0.0);
        let sum = s.summary().unwrap();
        assert_eq!(sum.frames, 1000);
        assert!((sum.avg - 1000.0 / 2.08).abs() < 1.0, "{sum:?}");
        // The slowest 1% are the 10 ms frames → 100 FPS.
        assert!((sum.low1 - 1000.0 / 2.005).abs() < 2.0 || (sum.low1 - 100.0).abs() < 1.0, "{sum:?}");
        assert!((sum.low01 - 100.0).abs() < 1.0, "{sum:?}");
        assert_eq!(sum.hitches, 10);
        let live = s.tick().unwrap();
        assert!((live.fps - sum.avg).abs() < 1.0 && live.worst_ms == 10.0);
        assert!(s.tick().is_none(), "nothing new");
        // Frames slower than 100 ms are kept exactly.
        let mut slow = Stats::default();
        for _ in 0..10 {
            slow.add(250.0);
        }
        assert!((slow.summary().unwrap().low1 - 4.0).abs() < 0.01);
        assert!(Stats::default().summary().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn runs_a_capture_and_stops_it() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("presentmon");
        // Stands in for PresentMon: a header, frames, then waits to be stopped.
        let stop = dir.path().join("stop");
        std::fs::write(&exe, format!("#!/bin/sh\ncase \"$*\" in *terminate_existing*) touch '{0}'; exit 0;; esac\necho Application,ProcessID,SwapChainAddress,MsBetweenPresents\ni=0\nwhile [ $i -lt 300 ]; do echo game.exe,7,0x1,2.5; i=$((i+1)); done\nsleep 1.2\necho game.exe,7,0x1,2.5\nwhile [ ! -f '{0}' ]; do sleep 0.05; done\n", stop.display())).unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        let stats = Arc::new(Mutex::new(Stats::default()));
        let ticks = Arc::new(Mutex::new(Vec::new()));
        let t = ticks.clone();
        let mut m = Meter::start(&exe, 7, stats.clone(), move |l| t.lock().push(l)).unwrap();
        std::thread::sleep(Duration::from_millis(1600));
        assert!(m.running());
        m.stop();
        let sum = stats.lock().summary().unwrap();
        assert_eq!(sum.frames, 301);
        assert!((sum.avg - 400.0).abs() < 0.5);
        assert!(ticks.lock().first().is_some_and(|l| (l.fps - 400.0).abs() < 0.5), "{:?}", ticks.lock());

        // One that can't read frame timings says why.
        let denied = dir.path().join("denied");
        std::fs::write(&denied, "#!/bin/sh\necho 'error: failed to start trace session (access denied).' >&2\nexit 5\n").unwrap();
        std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o755)).unwrap();
        let e = Meter::start(&denied, 7, stats, |_| {}).err().unwrap();
        assert!(e.contains("allow the FPS meter"), "{e}");
    }
}
