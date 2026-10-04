//! iPhone → PC screen mirroring through AirPlay.
//!
//! iOS mirrors its screen only to AirPlay receivers, so OmniHub runs one:
//! UxPlay (GPL-3.0, <https://github.com/FDH2/UxPlay>), as a separate program.
//! It is either the "AirPlay add-on" (UxPlay plus its GStreamer runtime,
//! built from the official source by this repository's release pipeline and
//! downloaded on demand, SHA-256 checked) or a UxPlay the user installed.
//!
//! OmniHub starts and stops the receiver with the user's options, reads its
//! log to show who is connected, and manages the mirror window (title, keep
//! on top, picture-in-picture corner). The iPhone picks the receiver in
//! Control Center → Screen Mirroring.

use std::collections::VecDeque;
use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::events::EventBus;

/// File name of the add-on in a release.
pub const ADDON_ASSET: &str = "OmniHub-AirPlay-addon-x64.zip";
/// Folder inside the zip (and under `addons/`).
const ADDON_DIR: &str = "airplay";
const LOG_LINES: usize = 60;

/// Where releases are downloaded from (overridable at build time).
pub fn releases_url() -> String {
    option_env!("OMNIHUB_RELEASES_URL").unwrap_or("https://github.com/Kerioxsx/omnihub/releases/download").trim_end_matches('/').to_string()
}

/// SHA-256 of the add-on built with this release, when the build knows it.
pub fn pinned_sha256() -> Option<&'static str> {
    option_env!("OMNIHUB_AIRPLAY_ADDON_SHA256").filter(|s| s.len() == 64)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AirPlayOptions {
    /// Name shown on the iPhone in Screen Mirroring.
    pub name: String,
    /// "1080p", "1440p" or "4k".
    pub quality: String,
    pub fps: u32,
    pub audio: bool,
    /// Ask for a PIN the first time a device connects (remembered after).
    pub require_pin: bool,
    /// Do not hold video back to keep audio in sync: lower delay.
    pub low_latency: bool,
    pub fullscreen: bool,
}

impl Default for AirPlayOptions {
    fn default() -> Self {
        AirPlayOptions { name: "OmniHub".into(), quality: "1080p".into(), fps: 60, audio: true, require_pin: true, low_latency: true, fullscreen: false }
    }
}

/// UxPlay command line for `o`. `pin` is a 4-digit code when a PIN is required.
pub fn uxplay_args(o: &AirPlayOptions, pin: Option<&str>, register: &Path) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    let name: String = o.name.trim().chars().filter(|c| !c.is_control()).take(40).collect();
    a.extend(["-n".into(), if name.is_empty() { "OmniHub".into() } else { name }, "-nh".into()]);
    // Fixed (legacy) ports: UDP 6000-6001, 7011; TCP 7000-7001, 7100.
    a.push("-p".into());
    let fps = o.fps.clamp(15, 60);
    let res = match o.quality.as_str() {
        "4k" => "3840x2160",
        "1440p" => "2560x1440",
        _ => "1920x1080",
    };
    if o.quality == "4k" {
        a.push("-h265".into());
    }
    a.extend(["-s".into(), format!("{res}@{fps}"), "-fps".into(), fps.to_string()]);
    if !o.audio {
        a.extend(["-as".into(), "0".into()]);
    }
    if o.low_latency {
        a.extend(["-vsync".into(), "no".into()]);
    }
    if o.fullscreen {
        a.push("-fs".into());
    }
    if let Some(pin) = pin {
        a.extend(["-pin".into(), pin.to_string(), "-reg".into(), register.to_string_lossy().into_owned()]);
    }
    // A new iPhone takes over from the one mirroring now.
    a.push("-nohold".into());
    a
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AirPlayClient {
    pub name: String,
    pub model: String,
    pub device_id: String,
}

/// What a UxPlay log line means for the status.
#[derive(Debug, Clone, PartialEq)]
pub enum LogEvent {
    Client(AirPlayClient),
    Lost,
    Error(String),
}

pub fn parse_log_line(line: &str) -> Option<LogEvent> {
    let l = line.trim();
    if let Some(rest) = l.strip_prefix("connection request from ") {
        // "connection request from NAME (MODEL) with deviceID = ID"
        let (who, id) = rest.rsplit_once(" with deviceID = ")?;
        let (name, model) = match who.rfind(" (") {
            Some(i) if who.ends_with(')') => (&who[..i], &who[i + 2..who.len() - 1]),
            _ => (who, ""),
        };
        return Some(LogEvent::Client(AirPlayClient { name: name.trim().to_string(), model: model.to_string(), device_id: id.trim().to_string() }));
    }
    if l.contains("lost connection with client") {
        return Some(LogEvent::Lost);
    }
    if l.contains("ERROR") || l.starts_with("Error") || l.contains("no element") || l.contains("failed with error") {
        return Some(LogEvent::Error(l.trim_start_matches("** ").to_string()));
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub phase: String,
    pub done: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AirPlayStatus {
    pub supported: bool,
    pub installed: bool,
    /// "addon", "custom" (a path in settings) or "found" (on PATH / MSYS2).
    pub source: Option<String>,
    pub path: Option<String>,
    pub version: Option<String>,
    pub running: bool,
    /// The video window is open: an iPhone is mirroring right now.
    pub mirroring: bool,
    pub name: String,
    pub pin: Option<String>,
    pub client: Option<AirPlayClient>,
    pub error: Option<String>,
    pub log: Vec<String>,
    pub install: Option<InstallProgress>,
    pub download_url: String,
}

#[derive(Default)]
struct Runtime {
    pin: Option<String>,
    name: String,
    client: Option<AirPlayClient>,
    error: Option<String>,
    log: VecDeque<String>,
    mirroring: bool,
}

pub struct AirPlay {
    addons: PathBuf,
    data: PathBuf,
    events: EventBus,
    child: Mutex<Option<Child>>,
    rt: Arc<Mutex<Runtime>>,
    install: Arc<Mutex<Option<InstallProgress>>>,
    keep_on_top: Arc<AtomicBool>,
    pip: Arc<AtomicBool>,
    stop_flag: Arc<AtomicBool>,
    #[cfg(windows)]
    job: Mutex<Option<win::Job>>,
}

fn hidden(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

impl AirPlay {
    /// `addons`: folder add-ons are installed into; `data`: for the device register.
    pub fn new(addons: &Path, data: &Path, events: EventBus) -> Self {
        AirPlay {
            addons: addons.to_path_buf(),
            data: data.to_path_buf(),
            events,
            child: Mutex::new(None),
            rt: Arc::new(Mutex::new(Runtime::default())),
            install: Arc::new(Mutex::new(None)),
            keep_on_top: Arc::new(AtomicBool::new(false)),
            pip: Arc::new(AtomicBool::new(false)),
            stop_flag: Arc::new(AtomicBool::new(false)),
            #[cfg(windows)]
            job: Mutex::new(None),
        }
    }

    pub fn addon_root(&self) -> PathBuf {
        self.addons.join(ADDON_DIR)
    }

    fn addon_exe(&self) -> PathBuf {
        self.addon_root().join("bin").join(if cfg!(windows) { "uxplay.exe" } else { "uxplay" })
    }

    /// The receiver to run: a configured path, the add-on, or one on the system.
    pub fn find(&self, configured: Option<&str>) -> Option<(PathBuf, &'static str)> {
        if let Some(p) = configured.map(PathBuf::from).filter(|p| p.is_file()) {
            return Some((p, "custom"));
        }
        let addon = self.addon_exe();
        if addon.is_file() {
            return Some((addon, "addon"));
        }
        let exe = if cfg!(windows) { "uxplay.exe" } else { "uxplay" };
        let mut candidates: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).map(|d| d.join(exe)).collect()).unwrap_or_default();
        if cfg!(windows) {
            candidates.push(PathBuf::from(r"C:\msys64\ucrt64\bin\uxplay.exe"));
        }
        candidates.into_iter().find(|p| p.is_file()).map(|p| (p, "found"))
    }

    fn addon_version(&self) -> Option<String> {
        let meta: serde_json::Value = serde_json::from_slice(&std::fs::read(self.addon_root().join("addon.json")).ok()?).ok()?;
        meta["uxplayVersion"].as_str().map(|v| format!("UxPlay {v}"))
    }

    pub fn is_running(&self) -> bool {
        let mut g = self.child.lock();
        match g.as_mut() {
            Some(c) => match c.try_wait() {
                Ok(None) => true,
                _ => {
                    *g = None;
                    false
                }
            },
            None => false,
        }
    }

    pub fn status(&self, configured: Option<&str>) -> AirPlayStatus {
        let found = self.find(configured);
        let running = self.is_running();
        let rt = self.rt.lock();
        AirPlayStatus {
            supported: cfg!(windows),
            installed: found.is_some(),
            source: found.as_ref().map(|(_, s)| s.to_string()),
            path: found.as_ref().map(|(p, _)| p.to_string_lossy().into_owned()),
            version: match found.as_ref().map(|(_, s)| *s) {
                Some("addon") => self.addon_version(),
                _ => None,
            },
            running,
            mirroring: running && rt.mirroring,
            name: rt.name.clone(),
            pin: if running { rt.pin.clone() } else { None },
            client: if running { rt.client.clone() } else { None },
            error: rt.error.clone(),
            log: rt.log.iter().cloned().collect(),
            install: self.install.lock().clone(),
            download_url: format!("{}/v{}/{ADDON_ASSET}", releases_url(), env!("CARGO_PKG_VERSION")),
        }
    }

    fn emit(&self) {
        self.events.emit("airplay:changed", serde_json::json!({}));
    }

    /// Start the receiver. Returns the PIN when one is required.
    pub fn start(&self, configured: Option<&str>, o: &AirPlayOptions, keep_on_top: bool, pip: bool) -> std::io::Result<Option<String>> {
        self.stop();
        let (exe, source) = self.find(configured).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "the AirPlay receiver is not installed"))?;
        let pin = o.require_pin.then(|| format!("{:04}", rand::random::<u16>() % 10_000));
        std::fs::create_dir_all(&self.data)?;
        let args = uxplay_args(o, pin.as_deref(), &self.data.join("airplay-devices.txt"));
        let mut cmd = Command::new(&exe);
        cmd.args(&args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        if source == "addon" {
            let root = self.addon_root();
            let bin = root.join("bin");
            let path = std::env::var_os("PATH").unwrap_or_default();
            let mut paths = vec![bin.clone()];
            paths.extend(std::env::split_paths(&path));
            cmd.env("PATH", std::env::join_paths(paths).map_err(std::io::Error::other)?)
                .env("GST_PLUGIN_SYSTEM_PATH_1_0", root.join("lib").join("gstreamer-1.0"))
                .env("GST_PLUGIN_PATH_1_0", "")
                .env("GST_PLUGIN_SCANNER_1_0", bin.join(if cfg!(windows) { "gst-plugin-scanner.exe" } else { "gst-plugin-scanner" }))
                .env("GST_REGISTRY_1_0", self.data.join("airplay-gst-registry.bin"));
        }
        hidden(&mut cmd);
        let mut child = cmd.spawn()?;
        #[cfg(windows)]
        {
            // Closing OmniHub (even a crash) ends the receiver with it.
            *self.job.lock() = win::Job::kill_on_close(&child).ok();
        }
        {
            let mut rt = self.rt.lock();
            *rt = Runtime { pin: pin.clone(), name: args[1].clone(), ..Default::default() };
        }
        self.keep_on_top.store(keep_on_top, Ordering::Relaxed);
        self.pip.store(pip, Ordering::Relaxed);
        self.stop_flag.store(false, Ordering::Relaxed);
        for stream in [child.stdout.take().map(|s| Box::new(s) as Box<dyn Read + Send>), child.stderr.take().map(|s| Box::new(s) as Box<dyn Read + Send>)].into_iter().flatten() {
            let rt = self.rt.clone();
            let events = self.events.clone();
            std::thread::spawn(move || {
                for line in std::io::BufReader::new(stream).lines().map_while(Result::ok) {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let ev = parse_log_line(&line);
                    let mut g = rt.lock();
                    g.log.push_back(line);
                    while g.log.len() > LOG_LINES {
                        g.log.pop_front();
                    }
                    match ev {
                        Some(LogEvent::Client(c)) => {
                            g.client = Some(c.clone());
                            g.error = None;
                            drop(g);
                            events.emit("airplay:client", &c);
                        }
                        Some(LogEvent::Lost) => {
                            g.client = None;
                            drop(g);
                            events.emit("airplay:changed", serde_json::json!({}));
                        }
                        Some(LogEvent::Error(e)) => g.error = Some(e),
                        None => {}
                    }
                }
            });
        }
        #[cfg(windows)]
        {
            let pid = child.id();
            let (rt, events, top, pip, stop) = (self.rt.clone(), self.events.clone(), self.keep_on_top.clone(), self.pip.clone(), self.stop_flag.clone());
            std::thread::spawn(move || win::watch_window(pid, rt, events, top, pip, stop));
        }
        *self.child.lock() = Some(child);
        self.emit();
        Ok(pin)
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(mut c) = self.child.lock().take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        #[cfg(windows)]
        {
            *self.job.lock() = None;
        }
        {
            let mut rt = self.rt.lock();
            rt.client = None;
            rt.mirroring = false;
        }
        self.emit();
    }

    /// Keep the mirror window above other windows (games included).
    pub fn set_keep_on_top(&self, on: bool) {
        self.keep_on_top.store(on, Ordering::Relaxed);
        #[cfg(windows)]
        if let Some(c) = self.child.lock().as_ref() {
            win::apply_top(c.id(), on);
        }
    }

    /// Move the mirror window: "pip" (small, bottom-right) or "center".
    pub fn place_window(&self, how: &str) {
        self.pip.store(how == "pip", Ordering::Relaxed);
        #[cfg(windows)]
        if let Some(c) = self.child.lock().as_ref() {
            win::place(c.id(), how);
        }
        #[cfg(not(windows))]
        let _ = how;
    }

    /// Download, verify and unpack the add-on. Progress is published as
    /// `airplay:install` events and in the status.
    pub fn install_addon(&self, url: &str, expected_sha256: Option<&str>) -> std::io::Result<()> {
        let set = |phase: &str, done: u64, total: u64| {
            *self.install.lock() = Some(InstallProgress { phase: phase.into(), done, total });
            self.events.emit("airplay:install", InstallProgress { phase: phase.into(), done, total });
        };
        let res = (|| {
            std::fs::create_dir_all(&self.addons)?;
            let zip = self.addons.join(format!("{ADDON_ASSET}.part"));
            set("download", 0, 0);
            let hash = download(url, &zip, &mut |done, total| set("download", done, total))?;
            let expected = match expected_sha256 {
                Some(h) => h.to_lowercase(),
                None => {
                    set("verify", 0, 0);
                    let sums_url = format!("{}/SHA256SUMS.txt", url.rsplit_once('/').map_or(url, |(base, _)| base));
                    let sums = fetch_text(&sums_url)?;
                    checksum_for(&sums, ADDON_ASSET).ok_or_else(|| std::io::Error::other("the release lists no checksum for the add-on"))?
                }
            };
            if hash != expected {
                let _ = std::fs::remove_file(&zip);
                return Err(std::io::Error::other("the download did not match its published SHA-256 checksum, so it was not installed"));
            }
            set("unpack", 0, 0);
            self.stop();
            let staging = self.addons.join(format!("{ADDON_DIR}.new"));
            let _ = std::fs::remove_dir_all(&staging);
            extract_addon(&zip, &staging)?;
            let _ = std::fs::remove_dir_all(self.addon_root());
            std::fs::rename(&staging, self.addon_root())?;
            let _ = std::fs::remove_file(&zip);
            Ok(())
        })();
        *self.install.lock() = None;
        self.events.emit("airplay:install", serde_json::json!({ "phase": if res.is_ok() { "done" } else { "failed" }, "error": res.as_ref().err().map(|e| e.to_string()) }));
        self.emit();
        res
    }

    pub fn uninstall_addon(&self) -> std::io::Result<()> {
        self.stop();
        match std::fs::remove_dir_all(self.addon_root()) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => {
                self.emit();
                Ok(())
            }
        }
    }
}

impl Drop for AirPlay {
    fn drop(&mut self) {
        self.stop();
    }
}

fn agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
    ureq::Agent::config_builder().tls_config(tls).timeout_connect(Some(Duration::from_secs(20))).build().new_agent()
}

/// Download `url` to `dest`, reporting progress; returns the SHA-256 (hex).
pub fn download(url: &str, dest: &Path, progress: &mut dyn FnMut(u64, u64)) -> std::io::Result<String> {
    let mut res = agent().get(url).call().map_err(|e| std::io::Error::other(format!("download failed: {e}")))?;
    let total = res.body().content_length().unwrap_or(0);
    let mut reader = res.body_mut().as_reader();
    let mut file = std::io::BufWriter::new(std::fs::File::create(dest)?);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut done = 0u64;
    let mut last = std::time::Instant::now();
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        hasher.update(&buf[..n]);
        done += n as u64;
        if last.elapsed() > Duration::from_millis(150) {
            progress(done, total);
            last = std::time::Instant::now();
        }
    }
    file.flush()?;
    progress(done, total);
    Ok(hex::encode(hasher.finalize()))
}

fn fetch_text(url: &str) -> std::io::Result<String> {
    agent().get(url).call().map_err(|e| std::io::Error::other(format!("download failed: {e}")))?.body_mut().read_to_string().map_err(std::io::Error::other)
}

/// The checksum of `name` in a `sha256sum` listing.
pub fn checksum_for(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let (hash, file) = l.split_once(char::is_whitespace)?;
        (file.trim().trim_start_matches('*') == name && hash.len() == 64).then(|| hash.to_lowercase())
    })
}

/// Unpack the add-on zip (its `OmniHub-AirPlay/` folder) into `dest`,
/// refusing entries that would land outside it.
pub fn extract_addon(zip: &Path, dest: &Path) -> std::io::Result<()> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(zip)?).map_err(std::io::Error::other)?;
    std::fs::create_dir_all(dest)?;
    let mut has_exe = false;
    for i in 0..archive.len() {
        let mut f = archive.by_index(i).map_err(std::io::Error::other)?;
        let Some(rel) = f.enclosed_name() else {
            return Err(std::io::Error::other(format!("unsafe path in the add-on: {}", f.name())));
        };
        let rel = rel.strip_prefix("OmniHub-AirPlay").unwrap_or(&rel).to_path_buf();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out = dest.join(&rel);
        if f.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut w = std::fs::File::create(&out)?;
        std::io::copy(&mut f, &mut w)?;
        if rel.file_name().is_some_and(|n| n == "uxplay.exe" || n == "uxplay") {
            has_exe = true;
        }
    }
    if !has_exe {
        return Err(std::io::Error::other("the add-on does not contain uxplay"));
    }
    Ok(())
}

#[cfg(windows)]
mod win {
    use super::*;
    use windows::core::BOOL;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, HWND, LPARAM, RECT};
    use windows::Win32::System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowRect, GetWindowThreadProcessId, IsWindowVisible, SetWindowPos, SetWindowTextW, SystemParametersInfoW, HWND_NOTOPMOST, HWND_TOPMOST, SPI_GETWORKAREA, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    };

    pub struct Job(HANDLE);
    unsafe impl Send for Job {}
    impl Job {
        pub fn kill_on_close(child: &Child) -> windows::core::Result<Job> {
            unsafe {
                let job = CreateJobObjectW(None, None)?;
                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(job, JobObjectExtendedLimitInformation, &info as *const _ as *const _, std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32)?;
                let proc = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, child.id())?;
                let r = AssignProcessToJobObject(job, proc);
                let _ = CloseHandle(proc);
                r?;
                Ok(Job(job))
            }
        }
    }
    impl Drop for Job {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    fn windows_of(pid: u32) -> Vec<HWND> {
        struct Ctx {
            pid: u32,
            out: Vec<HWND>,
        }
        unsafe extern "system" fn cb(hwnd: HWND, lp: LPARAM) -> BOOL {
            let ctx = unsafe { &mut *(lp.0 as *mut Ctx) };
            let mut p = 0u32;
            unsafe { GetWindowThreadProcessId(hwnd, Some(&mut p as *mut u32)) };
            if p == ctx.pid && unsafe { IsWindowVisible(hwnd) }.as_bool() {
                ctx.out.push(hwnd);
            }
            BOOL(1)
        }
        let mut ctx = Ctx { pid, out: Vec::new() };
        unsafe {
            let _ = EnumWindows(Some(cb), LPARAM(&mut ctx as *mut Ctx as isize));
        }
        ctx.out
    }

    pub fn apply_top(pid: u32, on: bool) {
        for h in windows_of(pid) {
            unsafe {
                let _ = SetWindowPos(h, Some(if on { HWND_TOPMOST } else { HWND_NOTOPMOST }), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            }
        }
    }

    pub fn place(pid: u32, how: &str) {
        let mut work = RECT::default();
        unsafe {
            if SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut work as *mut _ as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0)).is_err() {
                return;
            }
        }
        let (ww, wh) = (work.right - work.left, work.bottom - work.top);
        for h in windows_of(pid) {
            let mut r = RECT::default();
            unsafe {
                let _ = GetWindowRect(h, &mut r);
            }
            let (cw, ch) = ((r.right - r.left).max(1), (r.bottom - r.top).max(1));
            let aspect = cw as f64 / ch as f64;
            let (w, hgt) = if how == "pip" {
                let hgt = (wh as f64 * 0.42) as i32;
                ((hgt as f64 * aspect) as i32, hgt)
            } else {
                let hgt = (wh as f64 * 0.8) as i32;
                ((hgt as f64 * aspect).min(ww as f64 * 0.9) as i32, hgt)
            };
            let (x, y) = if how == "pip" { (work.right - w - 16, work.bottom - hgt - 16) } else { (work.left + (ww - w) / 2, work.top + (wh - hgt) / 2) };
            unsafe {
                let _ = SetWindowPos(h, None, x, y, w, hgt, SWP_NOZORDER);
            }
        }
    }

    /// Follow the receiver's video window: it opens when an iPhone starts
    /// mirroring and closes when it stops.
    pub fn watch_window(pid: u32, rt: Arc<Mutex<Runtime>>, events: EventBus, top: Arc<AtomicBool>, pip: Arc<AtomicBool>, stop: Arc<AtomicBool>) {
        let mut seen: Vec<isize> = Vec::new();
        let mut was = false;
        while !stop.load(Ordering::Relaxed) {
            let wins = windows_of(pid);
            let now = !wins.is_empty();
            for h in &wins {
                if !seen.contains(&(h.0 as isize)) {
                    seen.push(h.0 as isize);
                    let title: Vec<u16> = "iPhone · OmniHub".encode_utf16().chain(Some(0)).collect();
                    unsafe {
                        let _ = SetWindowTextW(*h, windows::core::PCWSTR(title.as_ptr()));
                    }
                    if top.load(Ordering::Relaxed) {
                        apply_top(pid, true);
                    }
                    if pip.load(Ordering::Relaxed) {
                        place(pid, "pip");
                    }
                }
            }
            if now != was {
                was = now;
                rt.lock().mirroring = now;
                events.emit("airplay:changed", serde_json::json!({ "mirroring": now }));
            }
            std::thread::sleep(Duration::from_millis(400));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments() {
        let reg = Path::new("/data/airplay-devices.txt");
        let o = AirPlayOptions { name: "Sam's PC".into(), ..Default::default() };
        let a = uxplay_args(&o, Some("0427"), reg);
        let s = a.join(" ");
        assert!(s.starts_with("-n Sam's PC -nh -p"), "{s}");
        assert!(s.contains("-s 1920x1080@60 -fps 60"), "{s}");
        assert!(s.contains("-vsync no"));
        assert!(s.contains("-pin 0427 -reg /data/airplay-devices.txt"));
        assert!(!s.contains("-as 0"));
        let o = AirPlayOptions { quality: "4k".into(), fps: 120, audio: false, require_pin: false, low_latency: false, fullscreen: true, name: "  ".into() };
        let s = uxplay_args(&o, None, reg).join(" ");
        assert!(s.contains("-n OmniHub"), "{s}");
        assert!(s.contains("-h265 -s 3840x2160@60 -fps 60"), "{s}");
        assert!(s.contains("-as 0") && s.contains("-fs") && !s.contains("-pin") && !s.contains("-vsync"), "{s}");
    }

    #[test]
    fn log_lines() {
        assert_eq!(
            parse_log_line("connection request from Sam's iPhone (iPhone16,2) with deviceID = 5E:12:AB:CD:00:01"),
            Some(LogEvent::Client(AirPlayClient { name: "Sam's iPhone".into(), model: "iPhone16,2".into(), device_id: "5E:12:AB:CD:00:01".into() }))
        );
        assert_eq!(parse_log_line("***ERROR lost connection with client (network problem?)"), Some(LogEvent::Lost));
        assert!(matches!(parse_log_line("** (uxplay.exe:6432): ERROR **: gst_parse_launch error"), Some(LogEvent::Error(_))));
        assert_eq!(parse_log_line("using network ports UDP 6000 6001 7011 TCP 7000 7001 7100"), None);
    }

    #[test]
    fn checksums() {
        let sums = "aa  OmniHub_0.2.0_x64-setup.exe\n0123456789abcdef0123456789abcdef0123456789abcdef0123456789ABCDEF *OmniHub-AirPlay-addon-x64.zip\n";
        assert_eq!(checksum_for(sums, ADDON_ASSET).as_deref(), Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"));
        assert_eq!(checksum_for(sums, "missing.zip"), None);
    }

    fn make_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut z = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, data) in entries {
            z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            z.write_all(data).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn extraction_is_contained() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("good.zip");
        make_zip(&good, &[("OmniHub-AirPlay/bin/uxplay.exe", b"MZ"), ("OmniHub-AirPlay/addon.json", br#"{"uxplayVersion":"1.74"}"#)]);
        let dest = dir.path().join("out");
        extract_addon(&good, &dest).unwrap();
        assert_eq!(std::fs::read(dest.join("bin/uxplay.exe")).unwrap(), b"MZ");
        let evil = dir.path().join("evil.zip");
        make_zip(&evil, &[("../escape.txt", b"x"), ("OmniHub-AirPlay/bin/uxplay.exe", b"MZ")]);
        assert!(extract_addon(&evil, &dir.path().join("out2")).is_err());
        assert!(!dir.path().join("escape.txt").exists());
        let empty = dir.path().join("empty.zip");
        make_zip(&empty, &[("OmniHub-AirPlay/readme.txt", b"x")]);
        assert!(extract_addon(&empty, &dir.path().join("out3")).is_err());
    }

    /// Serve `body` once over plain HTTP on a local port.
    fn serve_once(body: Vec<u8>) -> String {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        std::thread::spawn(move || {
            for _ in 0..2 {
                let Ok((mut s, _)) = l.accept() else { return };
                let mut req = [0u8; 2048];
                let _ = s.read(&mut req);
                let path = String::from_utf8_lossy(&req).split_whitespace().nth(1).unwrap_or("/").to_string();
                let data = if path.ends_with("SHA256SUMS.txt") { format!("{}  {ADDON_ASSET}\n", hex::encode(Sha256::digest(&body))).into_bytes() } else { body.clone() };
                let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", data.len());
                let _ = s.write_all(&data);
            }
        });
        format!("http://{addr}/v0.0.0/{ADDON_ASSET}")
    }

    #[test]
    fn install_verifies_and_unpacks() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("src.zip");
        make_zip(&zip, &[("OmniHub-AirPlay/bin/uxplay.exe", b"MZ"), ("OmniHub-AirPlay/addon.json", br#"{"uxplayVersion":"1.74"}"#)]);
        let bytes = std::fs::read(&zip).unwrap();
        let events = EventBus::new();
        let ap = AirPlay::new(&dir.path().join("addons"), &dir.path().join("data"), events);

        // Checksum taken from the release's SHA256SUMS.txt.
        ap.install_addon(&serve_once(bytes.clone()), None).unwrap();
        assert!(ap.addon_root().join("bin/uxplay.exe").is_file());
        assert_eq!(ap.addon_version().as_deref(), Some("UxPlay 1.74"));

        // A pinned checksum that does not match refuses the download.
        let err = ap.install_addon(&serve_once(bytes), Some(&"0".repeat(64))).unwrap_err();
        assert!(err.to_string().contains("checksum"), "{err}");
        assert!(ap.addon_root().join("bin/uxplay.exe").is_file(), "the installed add-on stays");
        ap.uninstall_addon().unwrap();
        assert!(!ap.addon_root().exists());
    }
}
