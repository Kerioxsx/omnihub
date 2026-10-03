//! Bridges to best-in-class external tools.
//!
//! * **scrcpy** mirrors (and controls) an Android phone on the PC with
//!   hardware H.264/H.265/AV1, typically 35–70 ms glass to glass. OmniHub
//!   finds it, lists devices through adb, sets up wireless debugging and
//!   launches it with sensible presets.
//! * **Sunshine** + **Moonlight** stream the PC to a phone with NVENC/AMF/
//!   QuickSync at up to 4K60. OmniHub detects Sunshine and links to it.
//!
//! Both are separate, user-installed programs (GPL-3.0); OmniHub only starts
//! them, it does not bundle or link them.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn hidden(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdbDevice {
    pub serial: String,
    pub state: String,
    pub model: String,
    pub wireless: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScrcpyStatus {
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub adb: Option<String>,
    pub devices: Vec<AdbDevice>,
    pub running: bool,
    pub install_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ScrcpyOptions {
    pub serial: Option<String>,
    /// "lowLatency", "quality", "battery" or "custom".
    pub preset: String,
    pub max_size: Option<u32>,
    pub bitrate_mbps: Option<u32>,
    pub max_fps: Option<u32>,
    pub codec: Option<String>,
    pub audio: bool,
    pub turn_screen_off: bool,
    pub stay_awake: bool,
    pub fullscreen: bool,
    pub always_on_top: bool,
    pub control: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SunshineStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub running: bool,
    pub web_ui: String,
}

/// Parse `adb devices -l`.
pub fn parse_adb_devices(out: &str) -> Vec<AdbDevice> {
    out.lines()
        .skip_while(|l| !l.starts_with("List of devices"))
        .skip(1)
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            let serial = parts.next()?.to_string();
            let state = parts.next()?.to_string();
            let model = parts
                .find_map(|p| p.strip_prefix("model:"))
                .map(|m| m.replace('_', " "))
                .unwrap_or_else(|| serial.clone());
            let wireless = serial.contains(':') || serial.contains("._adb-tls-connect");
            Some(AdbDevice { serial, state, model, wireless })
        })
        .collect()
}

/// Build scrcpy's command line from the options.
pub fn scrcpy_args(o: &ScrcpyOptions) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    if let Some(s) = o.serial.as_ref().filter(|s| !s.is_empty()) {
        a.push(format!("--serial={s}"));
    }
    let (size, bitrate, fps, codec) = match o.preset.as_str() {
        "quality" => (Some(0), Some(24), Some(60), Some("h265")),
        "battery" => (Some(1280), Some(4), Some(30), Some("h264")),
        "custom" => (None, None, None, None),
        // Low latency: smaller frames, h264 (decoded fastest everywhere).
        _ => (Some(1920), Some(10), Some(60), Some("h264")),
    };
    if let Some(s) = o.max_size.or(size) {
        if s > 0 {
            a.push(format!("--max-size={s}"));
        }
    }
    if let Some(b) = o.bitrate_mbps.or(bitrate) {
        a.push(format!("--video-bit-rate={b}M"));
    }
    if let Some(f) = o.max_fps.or(fps) {
        a.push(format!("--max-fps={f}"));
    }
    if let Some(c) = o.codec.as_deref().or(codec) {
        if matches!(c, "h264" | "h265" | "av1") {
            a.push(format!("--video-codec={c}"));
        }
    }
    if !o.audio {
        a.push("--no-audio".into());
    }
    if o.turn_screen_off {
        a.push("--turn-screen-off".into());
    }
    if o.stay_awake {
        a.push("--stay-awake".into());
    }
    if o.fullscreen {
        a.push("--fullscreen".into());
    }
    if o.always_on_top {
        a.push("--always-on-top".into());
    }
    if !o.control {
        a.push("--no-control".into());
    }
    a.push("--window-title=Phone · OmniHub".into());
    a
}

fn which(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join(&exe)).find(|c| c.is_file()))
}

fn glob_first(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(prefix))).collect())
        .unwrap_or_default()
}

pub fn find_scrcpy(configured: Option<&str>) -> Option<PathBuf> {
    if let Some(c) = configured.filter(|c| !c.is_empty()) {
        let p = PathBuf::from(c);
        let p = if p.is_dir() { p.join(if cfg!(windows) { "scrcpy.exe" } else { "scrcpy" }) } else { p };
        if p.is_file() {
            return Some(p);
        }
    }
    if let Some(p) = which("scrcpy") {
        return Some(p);
    }
    #[cfg(windows)]
    {
        let local = dirs::data_local_dir().unwrap_or_default();
        let home = dirs::home_dir().unwrap_or_default();
        // winget installs into a versioned folder.
        for pkg in glob_first(&local.join("Microsoft").join("WinGet").join("Packages"), "Genymobile.scrcpy") {
            for inner in glob_first(&pkg, "scrcpy-win64") {
                let exe = inner.join("scrcpy.exe");
                if exe.is_file() {
                    return Some(exe);
                }
            }
        }
        for c in [
            home.join("scoop").join("apps").join("scrcpy").join("current").join("scrcpy.exe"),
            PathBuf::from(r"C:\ProgramData\chocolatey\bin\scrcpy.exe"),
            PathBuf::from(r"C:\Program Files\scrcpy\scrcpy.exe"),
            local.join("Programs").join("scrcpy").join("scrcpy.exe"),
        ] {
            if c.is_file() {
                return Some(c);
            }
        }
    }
    let _ = glob_first;
    None
}

fn find_adb(scrcpy: Option<&Path>) -> Option<PathBuf> {
    let name = if cfg!(windows) { "adb.exe" } else { "adb" };
    scrcpy.and_then(|s| s.parent()).map(|d| d.join(name)).filter(|p| p.is_file()).or_else(|| which("adb"))
}

fn run(cmd: &mut Command) -> std::io::Result<String> {
    let out = hidden(cmd).stdin(Stdio::null()).output()?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    if !out.status.success() && s.trim().is_empty() {
        return Err(std::io::Error::other(format!("exited with {}", out.status)));
    }
    Ok(s)
}

pub struct Bridges {
    scrcpy: Mutex<Option<Child>>,
}

impl Default for Bridges {
    fn default() -> Self {
        Self::new()
    }
}

impl Bridges {
    pub fn new() -> Self {
        Bridges { scrcpy: Mutex::new(None) }
    }

    fn scrcpy_running(&self) -> bool {
        let mut g = self.scrcpy.lock();
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

    pub fn scrcpy_status(&self, configured: Option<&str>) -> ScrcpyStatus {
        let path = find_scrcpy(configured);
        let adb = find_adb(path.as_deref());
        let version = path.as_ref().and_then(|p| run(Command::new(p).arg("--version")).ok()).and_then(|o| o.lines().next().map(|l| l.trim().to_string()));
        let devices = adb.as_ref().and_then(|a| run(Command::new(a).args(["devices", "-l"])).ok()).map(|o| parse_adb_devices(&o)).unwrap_or_default();
        ScrcpyStatus {
            found: path.is_some(),
            path: path.map(|p| p.to_string_lossy().to_string()),
            version,
            adb: adb.map(|p| p.to_string_lossy().to_string()),
            devices,
            running: self.scrcpy_running(),
            install_hint: "winget install --id Genymobile.scrcpy".into(),
        }
    }

    pub fn launch_scrcpy(&self, configured: Option<&str>, opts: &ScrcpyOptions) -> std::io::Result<()> {
        self.stop_scrcpy();
        let path = find_scrcpy(configured).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "scrcpy is not installed (winget install --id Genymobile.scrcpy)"))?;
        let mut cmd = Command::new(&path);
        cmd.args(scrcpy_args(opts)).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if let Some(dir) = path.parent() {
            cmd.current_dir(dir);
        }
        *self.scrcpy.lock() = Some(cmd.spawn()?);
        Ok(())
    }

    pub fn stop_scrcpy(&self) {
        if let Some(mut c) = self.scrcpy.lock().take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    fn adb(&self, configured: Option<&str>) -> std::io::Result<PathBuf> {
        find_adb(find_scrcpy(configured).as_deref()).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "adb not found (it ships with scrcpy)"))
    }

    /// Switch a USB-connected phone to wireless debugging and connect to it.
    /// Returns the address it is now reachable at.
    pub fn enable_wireless(&self, configured: Option<&str>, serial: &str) -> std::io::Result<String> {
        let adb = self.adb(configured)?;
        let route = run(Command::new(&adb).args(["-s", serial, "shell", "ip", "route"]))?;
        let ip = route
            .lines()
            .filter(|l| l.contains("wlan"))
            .find_map(|l| l.split_whitespace().skip_while(|w| *w != "src").nth(1))
            .ok_or_else(|| std::io::Error::other("the phone is not on Wi-Fi"))?
            .to_string();
        run(Command::new(&adb).args(["-s", serial, "tcpip", "5555"]))?;
        std::thread::sleep(std::time::Duration::from_millis(1500));
        let addr = format!("{ip}:5555");
        let out = run(Command::new(&adb).args(["connect", &addr]))?;
        if !out.contains("connected") {
            return Err(std::io::Error::other(out.trim().to_string()));
        }
        Ok(addr)
    }

    pub fn connect(&self, configured: Option<&str>, addr: &str) -> std::io::Result<String> {
        if !valid_addr(addr) {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "expected an address like 192.168.1.20:5555"));
        }
        let adb = self.adb(configured)?;
        let out = run(Command::new(&adb).args(["connect", addr]))?;
        Ok(out.trim().to_string())
    }

    /// Android 11+ wireless pairing ("Pair device with pairing code").
    pub fn pair(&self, configured: Option<&str>, addr: &str, code: &str) -> std::io::Result<String> {
        if !valid_addr(addr) || code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "enter the IP:port and 6-digit code shown on the phone"));
        }
        let adb = self.adb(configured)?;
        let out = run(Command::new(&adb).args(["pair", addr, code]))?;
        Ok(out.trim().to_string())
    }

    pub fn sunshine_status(&self, configured: Option<&str>) -> SunshineStatus {
        let path = configured
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .or_else(|| {
                [r"C:\Program Files\Sunshine\sunshine.exe", r"C:\Program Files (x86)\Sunshine\sunshine.exe"]
                    .iter()
                    .map(PathBuf::from)
                    .find(|p| p.is_file())
            })
            .or_else(|| which("sunshine"));
        let mut sys = sysinfo::System::new();
        sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        let running = sys.processes().values().any(|p| p.name().to_string_lossy().to_lowercase().starts_with("sunshine"));
        SunshineStatus { installed: path.is_some(), path: path.map(|p| p.to_string_lossy().to_string()), running, web_ui: "https://localhost:47990".into() }
    }
}

fn valid_addr(addr: &str) -> bool {
    let Some((host, port)) = addr.rsplit_once(':') else { return false };
    host.parse::<std::net::IpAddr>().is_ok() && port.parse::<u16>().is_ok_and(|p| p > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adb_device_list() {
        let out = "* daemon started successfully\nList of devices attached\nR58N12345\tdevice usb:1-1 product:beyond1 model:SM_G973F device:beyond1 transport_id:1\n192.168.1.40:5555\tdevice product:panther model:Pixel_7 device:panther transport_id:3\nemulator-5554\toffline\n\n";
        let d = parse_adb_devices(out);
        assert_eq!(d.len(), 3);
        assert_eq!(d[0].model, "SM G973F");
        assert!(!d[0].wireless);
        assert!(d[1].wireless);
        assert_eq!(d[2].state, "offline");
    }

    #[test]
    fn scrcpy_presets() {
        let a = scrcpy_args(&ScrcpyOptions { preset: "lowLatency".into(), control: true, ..Default::default() });
        assert!(a.contains(&"--max-size=1920".to_string()));
        assert!(a.contains(&"--video-codec=h264".to_string()));
        assert!(a.contains(&"--no-audio".to_string()));
        assert!(!a.contains(&"--no-control".to_string()));
        let q = scrcpy_args(&ScrcpyOptions { preset: "quality".into(), serial: Some("abc".into()), audio: true, ..Default::default() });
        assert!(q.contains(&"--serial=abc".to_string()));
        assert!(!q.iter().any(|x| x.starts_with("--max-size")), "quality keeps native resolution");
        assert!(q.contains(&"--video-codec=h265".to_string()));
        assert!(q.contains(&"--no-control".to_string()));
        let c = scrcpy_args(&ScrcpyOptions { preset: "custom".into(), codec: Some("rm -rf".into()), max_fps: Some(120), ..Default::default() });
        assert!(c.contains(&"--max-fps=120".to_string()));
        assert!(!c.iter().any(|x| x.contains("rm")));
        assert!(valid_addr("192.168.1.2:5555") && !valid_addr("evil;rm:5555") && !valid_addr("1.2.3.4"));
    }
}
