//! The PC health score: a quick look at the things that slow a PC down or
//! fill it up, each with what to do about it. Only reads; fixing is up to
//! the user (each check links to the page that does it).

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub id: String,
    pub title: String,
    /// "ok", "warn" or "bad".
    pub status: String,
    pub detail: String,
    /// What to do, when there's something to do.
    pub fix: Option<String>,
    /// The page that fixes it ("storage", "apps?filter=startup", "tools?tab=rules").
    pub route: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    /// 0–100.
    pub score: u8,
    pub checks: Vec<Check>,
    pub at: i64,
}

/// What the checks look at.
#[derive(Debug, Clone, Default)]
pub struct Inputs {
    /// (drive, free bytes, total bytes)
    pub disks: Vec<(String, u64, u64)>,
    pub memory_used: f64,
    pub uptime_days: f64,
    pub reboot_pending: bool,
    pub startup_on: usize,
    pub temp_bytes: u64,
    pub bin_bytes: u64,
    pub downloads_bytes: u64,
    pub downloads_files: usize,
}

const GB: u64 = 1 << 30;

fn gb(b: u64) -> String {
    if b >= 10 * GB {
        format!("{} GB", b / GB)
    } else {
        format!("{:.1} GB", b as f64 / GB as f64)
    }
}

fn check(id: &str, title: &str, status: &str, detail: String, fix: Option<&str>, route: Option<&str>) -> Check {
    let bad = status != "ok";
    Check { id: id.into(), title: title.into(), status: status.into(), detail, fix: fix.filter(|_| bad).map(String::from), route: route.filter(|_| bad).map(String::from) }
}

pub fn score(i: &Inputs, now: i64) -> Health {
    let mut checks = Vec::new();
    for (drive, free, total) in &i.disks {
        if *total < 8 * GB {
            continue;
        }
        let pct = *free as f64 / *total as f64 * 100.0;
        let status = if pct < 8.0 || *free < 10 * GB { "bad" } else if pct < 15.0 { "warn" } else { "ok" };
        checks.push(check(&format!("disk:{drive}"), &format!("Space on {drive}"), status, format!("{} free of {} ({pct:.0}%)", gb(*free), gb(*total)), Some("Free up space"), Some("storage")));
    }
    let status = if i.memory_used >= 92.0 { "bad" } else if i.memory_used >= 85.0 { "warn" } else { "ok" };
    checks.push(check("memory", "Memory", status, format!("{:.0}% in use", i.memory_used), Some("Close what you don't need"), Some("tasks")));
    let status = if i.reboot_pending || i.uptime_days >= 14.0 { "bad" } else if i.uptime_days >= 7.0 { "warn" } else { "ok" };
    let detail = if i.reboot_pending { "Windows is waiting for a restart to finish updating".to_string() } else { format!("On for {:.0} days", i.uptime_days.floor()) };
    checks.push(check("restart", "Restarted lately", status, detail, Some("Restart when you can"), None));
    let status = if i.startup_on > 15 { "bad" } else if i.startup_on > 8 { "warn" } else { "ok" };
    checks.push(check("startup", "Apps at startup", status, format!("{} start with Windows", i.startup_on), Some("Turn some off"), Some("apps?filter=startup")));
    let status = if i.temp_bytes >= 5 * GB { "bad" } else if i.temp_bytes >= GB { "warn" } else { "ok" };
    checks.push(check("temp", "Temporary files", status, gb(i.temp_bytes), Some("Clean them up"), Some("storage")));
    let status = if i.bin_bytes >= 10 * GB { "bad" } else if i.bin_bytes >= 2 * GB { "warn" } else { "ok" };
    checks.push(check("bin", "Recycle Bin", status, gb(i.bin_bytes), Some("Empty it"), Some("storage")));
    let status = if i.downloads_bytes >= 50 * GB || i.downloads_files > 2000 { "bad" } else if i.downloads_bytes >= 15 * GB || i.downloads_files > 500 { "warn" } else { "ok" };
    checks.push(check("downloads", "Downloads folder", status, format!("{} in {} files", gb(i.downloads_bytes), i.downloads_files), Some("Tidy it with folder rules"), Some("tools?tab=rules")));

    let lost: u32 = checks.iter().map(|c| match c.status.as_str() {
        "bad" => 15,
        "warn" => 6,
        _ => 0,
    }).sum();
    Health { score: 100u32.saturating_sub(lost) as u8, checks, at: now }
}

/// Size and file count of a folder (stops counting after 200,000 files).
fn folder_size(dir: &std::path::Path, depth: usize) -> (u64, usize) {
    let mut bytes = 0;
    let mut files = 0;
    for e in walkdir::WalkDir::new(dir).max_depth(depth).into_iter().flatten().take(200_000) {
        if e.file_type().is_file() {
            files += 1;
            bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    (bytes, files)
}

/// Look at this PC now (takes a second or two: it walks the temp folders).
pub fn gather() -> Inputs {
    let mut i = Inputs::default();
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut seen = std::collections::HashSet::new();
    for d in disks.list() {
        let name = d.mount_point().to_string_lossy().trim_end_matches(['\\', '/']).to_string();
        if d.is_removable() || !seen.insert(name.clone()) {
            continue;
        }
        i.disks.push((if name.is_empty() { "/".into() } else { name }, d.available_space(), d.total_space()));
    }
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    if sys.total_memory() > 0 {
        i.memory_used = sys.used_memory() as f64 / sys.total_memory() as f64 * 100.0;
    }
    i.uptime_days = sysinfo::System::uptime() as f64 / 86_400.0;
    i.startup_on = crate::startup::list().iter().filter(|s| s.enabled).count();
    i.temp_bytes = folder_size(&std::env::temp_dir(), 6).0;
    if let Some(d) = dirs::download_dir() {
        (i.downloads_bytes, i.downloads_files) = folder_size(&d, 3);
    }
    imp::extras(&mut i);
    i
}

#[cfg(windows)]
mod imp {
    pub fn extras(i: &mut super::Inputs) {
        use windows::Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO};
        let mut info = SHQUERYRBINFO { cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32, ..Default::default() };
        if unsafe { SHQueryRecycleBinW(windows::core::PCWSTR::null(), &mut info) }.is_ok() {
            i.bin_bytes = info.i64Size.max(0) as u64;
        }
        let hklm = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
        i.reboot_pending = hklm.open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired").is_ok()
            || hklm.open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending").is_ok();
        if let Some(local) = dirs::data_local_dir() {
            // Windows' own temp folder too, when it isn't %TEMP%.
            let win = local.join("Temp");
            if win != std::env::temp_dir() {
                i.temp_bytes += super::folder_size(&win, 6).0;
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn extras(_i: &mut super::Inputs) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_healthy_pc_scores_100_and_problems_cost_points() {
        let good = Inputs { disks: vec![("C:".into(), 300 * GB, 1000 * GB)], memory_used: 40.0, uptime_days: 1.0, startup_on: 5, temp_bytes: GB / 4, ..Default::default() };
        let h = score(&good, 1);
        assert_eq!(h.score, 100);
        assert!(h.checks.iter().all(|c| c.status == "ok" && c.fix.is_none() && c.route.is_none()));

        let bad = Inputs { disks: vec![("C:".into(), 20 * GB, 500 * GB), ("D:".into(), 100 * GB, 1000 * GB), ("E:".into(), 1, 4 * GB)], memory_used: 88.0, uptime_days: 9.0, startup_on: 20, temp_bytes: 6 * GB, reboot_pending: false, bin_bytes: 3 * GB, downloads_bytes: GB, downloads_files: 600 };
        let h = score(&bad, 1);
        let by: std::collections::HashMap<&str, &str> = h.checks.iter().map(|c| (c.id.as_str(), c.status.as_str())).collect();
        assert_eq!(by["disk:C:"], "bad", "4% free");
        assert_eq!(by["disk:D:"], "warn", "10% free");
        assert!(!by.contains_key("disk:E:"), "tiny drives are skipped");
        assert_eq!((by["memory"], by["restart"], by["startup"], by["temp"], by["bin"], by["downloads"]), ("warn", "warn", "bad", "bad", "warn", "warn"));
        assert_eq!(h.score, 100 - 15 * 3 - 6 * 5);
        let startup = h.checks.iter().find(|c| c.id == "startup").unwrap();
        assert_eq!(startup.route.as_deref(), Some("apps?filter=startup"));

        let pending = Inputs { reboot_pending: true, ..good };
        assert_eq!(score(&pending, 1).checks.iter().find(|c| c.id == "restart").unwrap().status, "bad");
    }
}
