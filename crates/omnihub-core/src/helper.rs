//! Helper mode: the same executable started with administrator rights to do
//! one privileged job and exit.
//!
//! `omnihub.exe --omnihub-helper scan-volume C <scan-dir>`
//!
//! Exit codes: 0 success, 1 failure (message in the error file), 2 cancelled,
//! 64 bad arguments.

use std::path::Path;

pub const HELPER_FLAG: &str = "--omnihub-helper";

/// If the process was started in helper mode, run the job and return the
/// exit code. Returns `None` for a normal start.
pub fn run_if_helper(args: &[String]) -> Option<i32> {
    let pos = args.iter().position(|a| a == HELPER_FLAG)?;
    let rest = &args[pos + 1..];
    Some(match rest.first().map(String::as_str) {
        Some("scan-volume") => scan_volume(&rest[1..]),
        _ => 64,
    })
}

fn scan_volume(args: &[String]) -> i32 {
    let (Some(letter), Some(dir)) = (args.first(), args.get(1)) else { return 64 };
    let mut chars = letter.chars();
    let (Some(letter), None) = (chars.next(), chars.next()) else { return 64 };
    if !letter.is_ascii_alphabetic() {
        return 64;
    }
    let dir = Path::new(dir);
    if !crate::storage::volume_scan::is_valid_scan_dir(dir) {
        return 64;
    }
    run_scan(letter, dir)
}

#[cfg(windows)]
fn run_scan(letter: char, dir: &Path) -> i32 {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use crate::storage::ntfs::mft::MftProgress;
    use crate::storage::volume_scan::{self, HelperProgress};

    let progress = Arc::new(MftProgress::default());
    let cancel = Arc::new(AtomicBool::new(false));
    let phase = Arc::new(parking_lot::Mutex::new(String::from("opening")));
    let done = Arc::new(AtomicBool::new(false));
    let progress_file = volume_scan::progress_path(dir, letter);
    let cancel_file = volume_scan::cancel_path(dir, letter);

    let writer = {
        let (progress, cancel, phase, done) = (progress.clone(), cancel.clone(), phase.clone(), done.clone());
        std::thread::spawn(move || {
            while !done.load(Ordering::Relaxed) {
                let p = HelperProgress::from_mft(&phase.lock(), &progress);
                if let Ok(bytes) = serde_json::to_vec(&p) {
                    let tmp = progress_file.with_extension("tmp");
                    if std::fs::write(&tmp, bytes).is_ok() {
                        let _ = std::fs::rename(&tmp, &progress_file);
                    }
                }
                if cancel_file.exists() {
                    cancel.store(true, Ordering::Relaxed);
                }
                std::thread::sleep(std::time::Duration::from_millis(120));
            }
        })
    };
    let res = volume_scan::scan_ntfs_volume(letter, dir, true, &progress, &cancel, &|p| *phase.lock() = p.to_string());
    done.store(true, Ordering::Relaxed);
    let _ = writer.join();
    let _ = std::fs::remove_file(volume_scan::progress_path(dir, letter));
    let _ = std::fs::remove_file(volume_scan::cancel_path(dir, letter));
    match res {
        Ok(_) => 0,
        Err(volume_scan::VolumeScanError::Cancelled) => 2,
        Err(e) => {
            let _ = std::fs::write(volume_scan::error_path(dir, letter), e.to_string());
            1
        }
    }
}

#[cfg(not(windows))]
fn run_scan(_letter: char, _dir: &Path) -> i32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_arguments() {
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(run_if_helper(&a(&["omnihub.exe"])), None);
        assert_eq!(run_if_helper(&a(&["x", HELPER_FLAG])), Some(64));
        assert_eq!(run_if_helper(&a(&["x", HELPER_FLAG, "format-c"])), Some(64));
        assert_eq!(run_if_helper(&a(&["x", HELPER_FLAG, "scan-volume", "CC", "/tmp"])), Some(64));
        assert_eq!(run_if_helper(&a(&["x", HELPER_FLAG, "scan-volume", "C", "/tmp/not-ours"])), Some(64));
    }
}
