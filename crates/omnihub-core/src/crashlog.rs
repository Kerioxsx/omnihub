//! "OmniHub closed unexpectedly last time": a marker file exists while the
//! app runs and is removed on a normal exit. When the next start finds it,
//! it gathers what is known — the end of OmniHub's log and Windows Error
//! Reporting's summaries of recent crashes of OmniHub, its WebView or the
//! AirPlay receiver — so it can be copied into a bug report. Nothing leaves
//! the PC unless the user pastes it somewhere.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MARKER: &str = "running.marker";
const REPORT: &str = "last-crash.txt";
const LOG: &str = "omnihub.log";
/// Programs whose crashes matter here (Windows Error Reporting folder prefixes).
const PROGRAMS: &[&str] = &["omnihub.exe", "uxplay.exe", "msedgewebview2.exe"];

/// Call once at start, after logging is set up. Returns the report when the
/// previous run did not end normally (also saved as `logs/last-crash.txt`).
pub fn begin_session(logs: &Path) -> Option<String> {
    let marker = logs.join(MARKER);
    let crashed = marker.exists();
    let _ = std::fs::write(&marker, std::process::id().to_string());
    if !crashed {
        return None;
    }
    let report = build_report(logs, &wer_dirs());
    let _ = std::fs::write(logs.join(REPORT), &report);
    Some(report)
}

/// Call on a normal exit.
pub fn end_session(logs: &Path) {
    let _ = std::fs::remove_file(logs.join(MARKER));
}

fn wer_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let base = PathBuf::from(local).join("Microsoft").join("Windows").join("WER");
        out.push(base.join("ReportArchive"));
        out.push(base.join("ReportQueue"));
    }
    if let Some(pd) = std::env::var_os("ProgramData") {
        let base = PathBuf::from(pd).join("Microsoft").join("Windows").join("WER");
        out.push(base.join("ReportArchive"));
        out.push(base.join("ReportQueue"));
    }
    out
}

pub fn build_report(logs: &Path, wer: &[PathBuf]) -> String {
    let mut out = format!("OmniHub {} did not close normally last time.\n", env!("CARGO_PKG_VERSION"));
    let crashes = recent_crashes(wer, Duration::from_secs(2 * 86400));
    if crashes.is_empty() {
        out.push_str("\nWindows recorded no crash of OmniHub, its window (WebView2) or the AirPlay receiver in the last two days.\n");
    } else {
        out.push_str("\nWindows crash reports (newest first):\n");
        for c in crashes.iter().take(6) {
            out.push_str(&format!("- {c}\n"));
        }
    }
    let tail = log_tail(&logs.join(LOG), 60);
    if !tail.is_empty() {
        out.push_str("\nEnd of OmniHub's log:\n");
        out.push_str(&tail);
        if !tail.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

/// The last `n` lines of a text file.
pub fn log_tail(path: &Path, n: usize) -> String {
    let Ok(bytes) = std::fs::read(path) else { return String::new() };
    // Only the end of a large log matters.
    let start = bytes.len().saturating_sub(64 * 1024);
    let text = String::from_utf8_lossy(&bytes[start..]);
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// One-line summaries of recent Windows Error Reporting crash reports for
/// the programs above, newest first.
pub fn recent_crashes(dirs: &[PathBuf], within: Duration) -> Vec<String> {
    let now = SystemTime::now();
    let mut found: Vec<(SystemTime, String)> = Vec::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_lowercase();
            // "AppCrash_OmniHub.exe_1a2b…" (also AppHang_).
            let Some(rest) = name.strip_prefix("appcrash_").or_else(|| name.strip_prefix("apphang_")) else { continue };
            if !PROGRAMS.iter().any(|p| rest.starts_with(p)) {
                continue;
            }
            let wer = e.path().join("Report.wer");
            let Ok(meta) = std::fs::metadata(&wer) else { continue };
            let when = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            if now.duration_since(when).unwrap_or_default() > within {
                continue;
            }
            if let Some(summary) = std::fs::read(&wer).ok().and_then(|b| summarize_wer(&b, name.starts_with("apphang_"))) {
                found.push((when, summary));
            }
        }
    }
    found.sort_by_key(|f| std::cmp::Reverse(f.0));
    found.into_iter().map(|(_, s)| s).collect()
}

/// "OmniHub.exe 0.2.3 crashed in ntdll.dll (exception c0000005)" from a
/// Report.wer file (UTF-16 with a byte-order mark, or plain text).
pub fn summarize_wer(bytes: &[u8], hang: bool) -> Option<String> {
    let text = if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = bytes[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    let mut sig: Vec<(String, String)> = Vec::new();
    let mut names: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        if let Some(idx) = k.strip_prefix("Sig[").and_then(|r| r.split_once(']')) {
            match idx.1 {
                ".Name" => {
                    names.insert(idx.0.to_string(), v.to_string());
                }
                ".Value" => sig.push((idx.0.to_string(), v.to_string())),
                _ => {}
            }
        }
    }
    let get = |want: &str| -> Option<String> {
        sig.iter().find(|(i, _)| names.get(i).is_some_and(|n| n.eq_ignore_ascii_case(want))).map(|(_, v)| v.clone())
    };
    let app = get("Application Name")?;
    let who = match get("Application Version") {
        Some(v) if !v.is_empty() => format!("{app} {v}"),
        _ => app,
    };
    if hang {
        return Some(format!("{who} stopped responding"));
    }
    let module = get("Fault Module Name").unwrap_or_else(|| "an unknown module".into());
    let code = get("Exception Code").map(|c| format!(" (exception {c})")).unwrap_or_default();
    Some(format!("{who} crashed in {module}{code}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(s: &str) -> Vec<u8> {
        let mut b = vec![0xFF, 0xFE];
        for u in s.encode_utf16() {
            b.extend_from_slice(&u.to_le_bytes());
        }
        b
    }

    #[test]
    fn reads_windows_crash_reports() {
        let wer = "Version=1\r\nEventType=APPCRASH\r\nSig[0].Name=Application Name\r\nSig[0].Value=OmniHub.exe\r\nSig[1].Name=Application Version\r\nSig[1].Value=0.2.3.0\r\nSig[3].Name=Fault Module Name\r\nSig[3].Value=ntdll.dll\r\nSig[6].Name=Exception Code\r\nSig[6].Value=c0000005\r\n";
        assert_eq!(summarize_wer(&utf16(wer), false).as_deref(), Some("OmniHub.exe 0.2.3.0 crashed in ntdll.dll (exception c0000005)"));
        assert_eq!(summarize_wer(wer.as_bytes(), true).as_deref(), Some("OmniHub.exe 0.2.3.0 stopped responding"));
        assert_eq!(summarize_wer(b"nothing useful", false), None);
    }

    #[test]
    fn marker_and_report() {
        let logs = tempfile::tempdir().unwrap();
        let wer = tempfile::tempdir().unwrap();
        // A crash report for the receiver, and one for an unrelated program.
        let r = wer.path().join("AppCrash_uxplay.exe_abc123");
        std::fs::create_dir_all(&r).unwrap();
        std::fs::write(r.join("Report.wer"), utf16("Sig[0].Name=Application Name\nSig[0].Value=uxplay.exe\nSig[3].Name=Fault Module Name\nSig[3].Value=libgstd3d12.dll\nSig[6].Name=Exception Code\nSig[6].Value=c0000005\n")).unwrap();
        let other = wer.path().join("AppCrash_notepad.exe_1");
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(other.join("Report.wer"), utf16("Sig[0].Name=Application Name\nSig[0].Value=notepad.exe\n")).unwrap();
        std::fs::write(logs.path().join(LOG), "line 1\nINFO starting AirPlay receiver\n").unwrap();

        assert_eq!(begin_session(logs.path()), None, "first start: nothing to report");
        end_session(logs.path());
        assert_eq!(begin_session(logs.path()), None, "after a normal exit");
        // No end_session: the next start reports.
        let report = build_report(logs.path(), &[wer.path().to_path_buf()]);
        assert!(report.contains("uxplay.exe crashed in libgstd3d12.dll (exception c0000005)"), "{report}");
        assert!(!report.contains("notepad"));
        assert!(report.contains("starting AirPlay receiver"));
        assert!(begin_session(logs.path()).is_some());
        assert!(logs.path().join(REPORT).exists());
    }
}
