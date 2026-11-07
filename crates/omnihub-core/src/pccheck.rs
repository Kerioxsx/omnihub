//! Checks on the PC itself:
//! - Drive health: each drive's own health report (SMART, through Windows'
//!   storage reliability counters): health, temperature, wear, errors.
//! - Temperature stress test: every processor core busy for a few minutes
//!   while temperatures are read, to see how hot it gets and whether it
//!   slows itself down.
//! - Game-ready check: before playing, what could get in the way (a restart
//!   Windows is waiting for, low space, heavy background programs, heat,
//!   memory).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;

// ---------- drive health ----------

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DriveHealth {
    pub name: String,
    /// "SSD", "HDD" or "".
    pub media: String,
    pub bus: String,
    /// "Healthy", "Warning", "Unhealthy" or "Unknown".
    pub health: String,
    pub size: u64,
    pub temperature: Option<u32>,
    /// Wear, % used (SSDs).
    pub wear: Option<u32>,
    pub power_on_hours: Option<u64>,
    pub read_errors: Option<u64>,
    pub write_errors: Option<u64>,
    /// A sentence on what it means.
    pub verdict: String,
}

/// One drive from Windows' report (PowerShell's JSON).
pub fn drive_from(v: &serde_json::Value) -> DriveHealth {
    let s = |k: &str| v.get(k).map(|x| x.as_str().map(String::from).unwrap_or_else(|| x.to_string())).unwrap_or_default();
    let n = |k: &str| v.get(k).and_then(|x| x.as_u64());
    let media = match s("Media").as_str() {
        "4" | "SSD" => "SSD",
        "3" | "HDD" => "HDD",
        _ => "",
    };
    let health = match s("Health").as_str() {
        "0" | "Healthy" => "Healthy",
        "1" | "Warning" => "Warning",
        "2" | "Unhealthy" => "Unhealthy",
        _ => "Unknown",
    };
    let wear = n("Wear").map(|w| w as u32);
    let errors = n("ReadErrors").unwrap_or(0) + n("WriteErrors").unwrap_or(0);
    let verdict = if health == "Unhealthy" {
        "The drive reports it's failing. Back up what's on it now and plan to replace it.".to_string()
    } else if health == "Warning" || wear.is_some_and(|w| w >= 90) || errors > 0 {
        "Worth watching: back up what matters on it.".to_string()
    } else if health == "Healthy" {
        "Healthy.".to_string()
    } else {
        "Windows doesn't get a health report from this drive (common for USB drives).".to_string()
    };
    DriveHealth {
        name: s("Name").trim().to_string(),
        media: media.into(),
        bus: s("Bus"),
        health: health.into(),
        size: n("Size").unwrap_or(0),
        temperature: n("Temp").filter(|t| *t > 0 && *t < 120).map(|t| t as u32),
        wear,
        power_on_hours: n("PowerOnHours"),
        read_errors: n("ReadErrors"),
        write_errors: n("WriteErrors"),
        verdict,
    }
}

pub fn drives() -> Result<Vec<DriveHealth>, String> {
    imp::drives()
}

// ---------- the stress test ----------

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct StressResult {
    pub seconds: u32,
    pub start_temp: Option<f32>,
    pub max_temp: Option<f32>,
    pub end_temp: Option<f32>,
    /// The processor's speed fell while hot (thermal throttling), from the
    /// busiest core's work rate dropping over 15% between the first and last
    /// third.
    pub throttled: bool,
    pub verdict: String,
}

/// Judge a run from its temperature samples and work rates.
pub fn judge(seconds: u32, temps: &[f32], rates: &[f64]) -> StressResult {
    let max = temps.iter().copied().fold(None, |a: Option<f32>, t| Some(a.map_or(t, |a| a.max(t))));
    let third = rates.len() / 3;
    let throttled = third > 0 && {
        let first: f64 = rates[..third].iter().sum::<f64>() / third as f64;
        let last: f64 = rates[rates.len() - third..].iter().sum::<f64>() / third as f64;
        first > 0.0 && last < first * 0.85
    };
    let verdict = match (max, throttled) {
        (None, _) => "The test ran, but OmniHub can't read this processor's temperature (sensors need LibreHardwareMonitor support).".into(),
        (Some(m), true) => format!("It reached {m:.0} °C and slowed itself down to cool. Clean the dust, check the cooler's mounting and the case airflow."),
        (Some(m), false) if m >= 95.0 => format!("It reached {m:.0} °C: very hot. Check the cooler and the airflow."),
        (Some(m), false) if m >= 85.0 => format!("It reached {m:.0} °C: warm but fine for most processors under full load."),
        (Some(m), false) => format!("It stayed at {m:.0} °C or below under full load: cooling is good."),
    };
    StressResult { seconds, start_temp: temps.first().copied(), max_temp: max, end_temp: temps.last().copied(), throttled, verdict }
}

/// Every core busy for `seconds`, reading the temperature each second with
/// `temp`; `progress` gets (seconds done, temperature). Stops early on
/// `stop` or at 100 °C.
pub fn stress(seconds: u32, stop: &AtomicBool, temp: &dyn Fn() -> Option<f32>, progress: &dyn Fn(u32, Option<f32>)) -> StressResult {
    let seconds = seconds.clamp(10, 900);
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    let run = Arc::new(AtomicBool::new(true));
    let counters: Vec<Arc<std::sync::atomic::AtomicU64>> = (0..cores).map(|_| Arc::new(std::sync::atomic::AtomicU64::new(0))).collect();
    let workers: Vec<_> = counters
        .iter()
        .map(|c| {
            let (run, c) = (run.clone(), c.clone());
            std::thread::spawn(move || {
                let mut x = 1.000_001f64;
                while run.load(Ordering::Relaxed) {
                    for _ in 0..200_000 {
                        x = (x * 1.000_000_1).sqrt() + 0.5;
                    }
                    c.fetch_add(1, Ordering::Relaxed);
                }
                std::hint::black_box(x);
            })
        })
        .collect();
    let mut temps = Vec::new();
    let mut rates = Vec::new();
    let start = Instant::now();
    let mut last = 0u64;
    let mut done = 0;
    while done < seconds && !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_secs(1));
        done = start.elapsed().as_secs() as u32;
        let t = temp();
        if let Some(t) = t {
            temps.push(t);
        }
        let total: u64 = counters.iter().map(|c| c.load(Ordering::Relaxed)).sum();
        rates.push((total - last) as f64);
        last = total;
        progress(done, t);
        if t.is_some_and(|t| t >= 100.0) {
            break;
        }
    }
    run.store(false, Ordering::Relaxed);
    for w in workers {
        let _ = w.join();
    }
    judge(done, &temps, &rates)
}

// ---------- the game-ready check ----------

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReadyItem {
    pub title: String,
    /// "ok", "warn" or "bad".
    pub status: String,
    pub detail: String,
    pub route: Option<String>,
}

/// Programs that draw over games (process → the name people know). Each
/// can cost frames or add latency; some games' anti-cheat dislikes them.
pub const OVERLAYS: &[(&str, &str)] = &[
    ("NVIDIA Overlay.exe", "NVIDIA overlay"),
    ("GameBar.exe", "Xbox Game Bar"),
    ("RTSS.exe", "RivaTuner Statistics Server"),
    ("MSIAfterburner.exe", "MSI Afterburner"),
    ("Overwolf.exe", "Overwolf"),
    ("Medal.exe", "Medal"),
    ("Outplayed.exe", "Outplayed"),
    ("RazerCortex.exe", "Razer Cortex"),
    ("FPSMonitor.exe", "FPS Monitor"),
];

/// The overlays among the running programs (`running(name)` says whether one runs).
pub fn overlays(running: &dyn Fn(&str) -> bool) -> Vec<&'static str> {
    OVERLAYS.iter().filter(|(p, _)| running(p)).map(|(_, n)| *n).collect()
}

/// Graphics drivers' shader caches (vendor, folder under %LOCALAPPDATA%).
pub const SHADER_CACHES: &[(&str, &str)] = &[("NVIDIA", r"NVIDIA\DXCache"), ("NVIDIA", r"NVIDIA\GLCache"), ("AMD", r"AMD\DxCache"), ("AMD", r"AMD\DxcCache"), ("AMD", r"AMD\VkCache"), ("Intel", r"Intel\ShaderCache"), ("DirectX", "D3DSCache")];

/// Bytes in the shader caches that exist, per vendor (largest first).
pub fn shader_cache_sizes(local: &std::path::Path) -> Vec<(String, u64)> {
    let mut by: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    for (vendor, rel) in SHADER_CACHES {
        let dir = local.join(rel.replace('\\', std::path::MAIN_SEPARATOR_STR));
        if dir.is_dir() {
            let n: u64 = walkdir::WalkDir::new(&dir).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_file()).filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum();
            *by.entry(vendor.to_string()).or_default() += n;
        }
    }
    let mut v: Vec<(String, u64)> = by.into_iter().collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.1));
    v
}

fn item(title: &str, status: &str, detail: String, route: Option<&str>) -> ReadyItem {
    ReadyItem { title: title.into(), status: status.into(), detail, route: route.map(String::from) }
}

impl crate::core::AppCore {
    pub fn game_ready(&self) -> Vec<ReadyItem> {
        use crate::system::procs::ProcessSort;
        let mut out = Vec::new();
        let inputs = crate::health::gather();
        out.push(if inputs.reboot_pending { item("Windows updates", "warn", "Windows is waiting to restart for updates; it may do it mid-game. Restart first.".into(), None) } else { item("Windows updates", "ok", "No restart waiting.".into(), None) });
        let low: Vec<String> = inputs.disks.iter().filter(|(_, free, total)| *total > 0 && (*free as f64 / *total as f64) < 0.08).map(|(d, free, _)| format!("{d} ({:.0} GB free)", *free as f64 / 1e9)).collect();
        out.push(if low.is_empty() { item("Free space", "ok", "Enough room for updates and shaders.".into(), None) } else { item("Free space", "warn", format!("Low on {}: game updates can fail.", low.join(", ")), Some("storage")) });
        let u = self.procs.usage(ProcessSort::Cpu, 5, false);
        let heavy: Vec<String> = u.processes.iter().filter(|p| p.cpu >= 15.0).map(|p| format!("{} ({:.0}%)", p.name, p.cpu)).collect();
        out.push(if heavy.is_empty() { item("Background programs", "ok", format!("The processor is {:.0}% busy.", u.cpu), None) } else { item("Background programs", "warn", format!("Busy: {}. Close them or use the game's boost.", heavy.join(", ")), Some("tasks")) });
        let mem = if u.memory_total > 0 { u.memory_used as f64 / u.memory_total as f64 } else { 0.0 };
        out.push(if mem >= 0.8 { item("Memory", "warn", format!("{:.0}% in use: close browsers or other big programs.", mem * 100.0), Some("tasks?sort=memory")) } else { item("Memory", "ok", format!("{:.0}% in use.", mem * 100.0), None) });
        let rig = self.rig.status();
        let hottest = match (rig.cpu_temp, rig.gpu_temp) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        out.push(match hottest {
            Some(t) if t >= 80.0 => item("Temperatures", "warn", format!("Already {t:.0} °C before playing."), Some("rig?tab=fans")),
            Some(t) => item("Temperatures", "ok", format!("{t:.0} °C."), None),
            None => item("Temperatures", "ok", "Not readable on this PC.".into(), None),
        });
        let on = overlays(&|p| !self.procs.pids_named(p).is_empty());
        out.push(if on.is_empty() {
            item("Overlays", "ok", "None of the usual overlays are running (Discord's and Steam's are set in those apps).".into(), None)
        } else {
            item("Overlays", "warn", format!("Running: {}. Overlays can cost frames and add delay; close the ones you don't use. Discord's and Steam's are set in those apps.", on.join(", ")), Some("tasks"))
        });
        if let Some(local) = dirs::data_local_dir() {
            let caches = shader_cache_sizes(&local);
            let nvidia = caches.iter().find(|(v, _)| v == "NVIDIA").map(|x| x.1);
            if let Some(n) = nvidia {
                let gb = n as f64 / 1e9;
                out.push(if gb >= 3.6 {
                    item("Shader cache", "warn", format!("The NVIDIA shader cache holds {gb:.1} GB, close to the driver's usual 4 GB limit: older shaders get thrown out and rebuilt, which stutters. In the NVIDIA Control Panel → Manage 3D settings, set Shader Cache Size to 10 GB or Unlimited."), None)
                } else {
                    item("Shader cache", "ok", format!("The NVIDIA shader cache holds {gb:.1} GB."), None)
                });
            }
        }
        let quiet = self.quiet_status().on;
        if quiet {
            out.push(item("Quiet mode", "warn", "Quiet mode is on: the fans are at their lowest.".into(), None));
        }
        out
    }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::process::CommandExt;

    pub fn drives() -> Result<Vec<super::DriveHealth>, String> {
        let script = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-PhysicalDisk | ForEach-Object { $r = $_ | Get-StorageReliabilityCounter -ErrorAction SilentlyContinue; [pscustomobject]@{ Name=$_.FriendlyName; Media=[string]$_.MediaType; Bus=[string]$_.BusType; Health=[string]$_.HealthStatus; Size=$_.Size; Temp=$r.Temperature; Wear=$r.Wear; PowerOnHours=$r.PowerOnHours; ReadErrors=$r.ReadErrorsUncorrected; WriteErrors=$r.WriteErrorsUncorrected } } | ConvertTo-Json -Compress";
        let out = std::process::Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script]).creation_flags(0x0800_0000).output().map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&out.stdout);
        let v: serde_json::Value = serde_json::from_str(text.trim()).map_err(|_| "Windows didn't report the drives".to_string())?;
        let list = if v.is_array() { v.as_array().cloned().unwrap_or_default() } else { vec![v] };
        Ok(list.iter().map(super::drive_from).collect())
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn drives() -> Result<Vec<super::DriveHealth>, String> {
        Err("Drive health is read on Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlays_and_shader_caches() {
        assert_eq!(overlays(&|p| p == "RTSS.exe" || p == "chrome.exe"), vec!["RivaTuner Statistics Server"]);
        let t = tempfile::tempdir().unwrap();
        let nv = t.path().join("NVIDIA").join("DXCache");
        std::fs::create_dir_all(&nv).unwrap();
        std::fs::write(nv.join("a.bin"), vec![0u8; 3000]).unwrap();
        std::fs::create_dir_all(t.path().join("D3DSCache").join("x")).unwrap();
        std::fs::write(t.path().join("D3DSCache").join("x").join("b"), vec![0u8; 100]).unwrap();
        assert_eq!(shader_cache_sizes(t.path()), vec![("NVIDIA".to_string(), 3000), ("DirectX".to_string(), 100)]);
    }

    #[test]
    fn reads_drives_and_judges_heat() {
        let ssd = drive_from(&serde_json::json!({"Name":"Samsung SSD 990 PRO 2TB ","Media":"SSD","Bus":"NVMe","Health":"Healthy","Size":2000398934016u64,"Temp":41,"Wear":3,"PowerOnHours":1520,"ReadErrors":0,"WriteErrors":0}));
        assert_eq!((ssd.name.as_str(), ssd.health.as_str(), ssd.wear, ssd.verdict.as_str()), ("Samsung SSD 990 PRO 2TB", "Healthy", Some(3), "Healthy."));
        let old = drive_from(&serde_json::json!({"Name":"WDC","Media":"HDD","Health":"Healthy","ReadErrors":12}));
        assert!(old.verdict.starts_with("Worth watching"));
        assert_eq!(drive_from(&serde_json::json!({"Name":"USB","Health":"Unhealthy"})).health, "Unhealthy");
        let ok = judge(60, &[45.0, 70.0, 78.0], &[100.0; 9]);
        assert!(!ok.throttled && ok.verdict.contains("cooling is good"));
        let hot = judge(60, &[50.0, 96.0, 97.0], &[100.0, 100.0, 100.0, 90.0, 80.0, 75.0, 70.0, 70.0, 70.0]);
        assert!(hot.throttled && hot.verdict.contains("slowed itself down"));
        assert!(judge(30, &[], &[1.0; 3]).verdict.contains("can't read"));
        let stop = AtomicBool::new(false);
        let r = stress(10, &stop, &|| Some(50.0), &|s, _| {
            if s >= 2 {
                stop.store(true, Ordering::Relaxed);
            }
        });
        assert!(r.seconds >= 2 && r.max_temp == Some(50.0));
    }
}
