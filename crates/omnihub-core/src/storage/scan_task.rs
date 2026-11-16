//! Fast scans without a UAC prompt each time (opt-in). One approval creates
//! a Task Scheduler task that runs `OmniHub.exe --omnihub-helper scan-queue
//! <scan cache>` with administrator rights. To scan, the app drops a request
//! (just an id and a drive letter) into the queue folder, starts the task,
//! and waits for the result file.
//!
//! The task only ever runs the same validated scan job the UAC helper runs:
//! read a volume's file table into OmniHub's own scan cache.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::system::schedtask;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Request {
    id: String,
    letter: char,
}

pub fn queue_dir(scan_dir: &Path) -> PathBuf {
    scan_dir.join("queue")
}

fn valid_id(id: &str) -> bool {
    id.len() == 36 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

fn done_path(scan_dir: &Path, id: &str) -> PathBuf {
    queue_dir(scan_dir).join(format!("{id}.done"))
}

/// Whether the no-prompt task is set up.
pub fn enabled() -> bool {
    schedtask::installed(schedtask::SCAN_TASK)
}

/// Turn the no-prompt task on or off (one UAC prompt either way).
pub fn set_enabled(on: bool, scan_dir: &Path) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut args = vec![crate::helper::HELPER_FLAG.to_string(), "scan-task".into(), if on { "on" } else { "off" }.into()];
    if on {
        args.push(scan_dir.to_string_lossy().into_owned());
    }
    match crate::system::elevation::run_elevated_and_wait(&exe, &args, false) {
        Ok(0) => Ok(()),
        Ok(code) => Err(format!("Windows did not create the task (code {code}).")),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => Err("Windows' administrator prompt was declined.".into()),
        Err(e) => Err(e.to_string()),
    }
}

/// Helper side (elevated by UAC): create or delete the task.
pub fn helper_set(on: &str, scan_dir: Option<&str>) -> i32 {
    match (on, scan_dir) {
        ("on", Some(dir)) => {
            let dir = Path::new(dir);
            if !super::volume_scan::is_valid_scan_dir(dir) {
                return 64;
            }
            let Ok(exe) = std::env::current_exe() else { return 1 };
            let args = format!("{} scan-queue \"{}\"", crate::helper::HELPER_FLAG, dir.display());
            let xml = schedtask::task_xml(&exe, &args, "Lets OmniHub read drives' file tables for fast scans without asking for approval each time. Remove it in OmniHub → Settings → Storage.");
            i32::from(schedtask::install(schedtask::SCAN_TASK, &xml).is_err())
        }
        ("off", None) => i32::from(schedtask::remove(schedtask::SCAN_TASK).is_err()),
        _ => 64,
    }
}

/// App side: run a fast scan of `letter` through the task. `Err` means the
/// task could not be used (so the caller asks through UAC instead).
pub fn run_via_task(letter: char, scan_dir: &Path) -> std::io::Result<i32> {
    let q = queue_dir(scan_dir);
    std::fs::create_dir_all(&q)?;
    let id = uuid::Uuid::new_v4().to_string();
    let req = q.join(format!("{id}.json"));
    std::fs::write(&req, serde_json::to_vec(&Request { id: id.clone(), letter }).map_err(std::io::Error::other)?)?;
    if let Err(e) = schedtask::run(schedtask::SCAN_TASK) {
        let _ = std::fs::remove_file(&req);
        return Err(std::io::Error::other(e));
    }
    // The task should pick the request up within seconds.
    let started = Instant::now();
    while req.exists() {
        if started.elapsed() > Duration::from_secs(20) {
            let _ = std::fs::remove_file(&req);
            return Err(std::io::Error::other("the scan task did not start"));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let done = done_path(scan_dir, &id);
    loop {
        if let Ok(text) = std::fs::read_to_string(&done) {
            let _ = std::fs::remove_file(&done);
            return Ok(text.trim().parse().unwrap_or(1));
        }
        if started.elapsed() > Duration::from_secs(3 * 3600) {
            return Err(std::io::Error::other("the scan task did not finish"));
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

/// Task side (elevated by the scheduler): scan every queued drive.
pub fn helper_queue(scan_dir: &str, scan: impl Fn(char, &Path) -> i32) -> i32 {
    let dir = Path::new(scan_dir);
    if !super::volume_scan::is_valid_scan_dir(dir) {
        return 64;
    }
    let q = queue_dir(dir);
    for _ in 0..16 {
        let Ok(rd) = std::fs::read_dir(&q) else { return 0 };
        let reqs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect();
        if reqs.is_empty() {
            return 0;
        }
        for p in reqs {
            let parsed = std::fs::read(&p).ok().and_then(|b| serde_json::from_slice::<Request>(&b).ok());
            let _ = std::fs::remove_file(&p);
            let Some(r) = parsed.filter(|r| valid_id(&r.id) && r.letter.is_ascii_alphabetic()) else { continue };
            let code = scan(r.letter, dir);
            let _ = std::fs::write(done_path(dir, &r.id), code.to_string());
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_round_trip() {
        let base = tempfile::tempdir().unwrap();
        let dir = base.path().join(crate::paths::APP_DIR_NAME).join("cache").join("scans");
        std::fs::create_dir_all(queue_dir(&dir)).unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        std::fs::write(queue_dir(&dir).join(format!("{id}.json")), format!(r#"{{"id":"{id}","letter":"D"}}"#)).unwrap();
        // Junk is ignored, not run.
        std::fs::write(queue_dir(&dir).join("x.json"), r#"{"id":"../../evil","letter":"C"}"#).unwrap();
        let seen = std::sync::Mutex::new(Vec::new());
        assert_eq!(helper_queue(&dir.to_string_lossy(), |l, _| {
            seen.lock().unwrap().push(l);
            0
        }), 0);
        assert_eq!(*seen.lock().unwrap(), vec!['D']);
        assert_eq!(std::fs::read_to_string(done_path(&dir, &id)).unwrap(), "0");
        assert!(!queue_dir(&dir).join("x.json").exists());
        // A folder that is not OmniHub's scan cache is refused.
        assert_eq!(helper_queue(&base.path().to_string_lossy(), |_, _| 0), 64);
        assert_eq!(helper_set("on", Some("C:\\Windows")), 64);
        assert_eq!(helper_set("maybe", None), 64);
    }
}
