//! Running Claude Code and Codex the way their docs describe running them
//! from scripts: one process per message, the message on stdin (so nothing
//! the user typed is ever parsed as a command-line flag), one JSON event per
//! line on stdout, no console window. Stopping ends the whole process tree.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde_json::Value;

/// How a run ended.
#[derive(Debug, Clone, Default)]
pub struct Ended {
    pub code: Option<i32>,
    /// The last few lines it wrote to stderr, for a failure's explanation.
    pub stderr: String,
    pub cancelled: bool,
}

fn hidden(c: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    #[cfg(unix)]
    {
        // Its own process group, so stopping it reaches everything it started.
        use std::os::unix::process::CommandExt;
        c.process_group(0);
    }
}

fn kill_tree(child: &mut Child) {
    #[cfg(windows)]
    {
        let mut c = Command::new("taskkill");
        c.args(["/PID", &child.id().to_string(), "/T", "/F"]).stdout(Stdio::null()).stderr(Stdio::null());
        hidden(&mut c);
        let _ = c.status();
    }
    #[cfg(unix)]
    {
        // SAFETY: signals the process group the child leads (see `hidden`).
        unsafe {
            libc::killpg(child.id() as libc::pid_t, libc::SIGKILL);
        }
    }
    let _ = child.kill();
}

/// Run `exe args` in `cwd` with `input` on stdin, handing each JSON line of
/// its output to `on`. Lines that aren't JSON are ignored.
pub fn run(exe: &Path, args: &[String], cwd: Option<&Path>, input: &str, cancel: &Arc<AtomicBool>, on: &mut dyn FnMut(&Value)) -> Result<Ended, String> {
    let mut c = Command::new(exe);
    c.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).env("NO_COLOR", "1");
    if let Some(d) = cwd.filter(|d| d.is_dir()) {
        c.current_dir(d);
    }
    hidden(&mut c);
    let mut child = c.spawn().map_err(|e| format!("Couldn't start {}: {e}", exe.display()))?;
    let mut stdin = child.stdin.take();
    let stdout = child.stdout.take().ok_or("no output")?;
    let stderr = child.stderr.take();
    let child = Arc::new(Mutex::new(child));

    let text = input.to_string();
    let writer = std::thread::spawn(move || {
        if let Some(s) = stdin.as_mut() {
            let _ = s.write_all(text.as_bytes());
            let _ = s.flush();
        }
        drop(stdin);
    });
    let tail = Arc::new(Mutex::new(String::new()));
    let err_reader = stderr.map(|mut e| {
        let tail = tail.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = e.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let mut t = tail.lock();
                t.push_str(&String::from_utf8_lossy(&buf[..n]));
                if t.len() > 6000 {
                    let cut = t.len() - 4000;
                    let cut = (cut..t.len()).find(|i| t.is_char_boundary(*i)).unwrap_or(t.len());
                    t.drain(..cut);
                }
            }
        })
    });
    let done = Arc::new(AtomicBool::new(false));
    let watcher = {
        let (child, cancel, done) = (child.clone(), cancel.clone(), done.clone());
        std::thread::spawn(move || {
            while !done.load(Ordering::SeqCst) {
                if cancel.load(Ordering::SeqCst) {
                    kill_tree(&mut child.lock());
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        })
    };

    let mut lines = BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        match lines.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let l = line.trim();
        if l.starts_with('{') {
            if let Ok(v) = serde_json::from_str::<Value>(l) {
                on(&v);
            }
        }
    }
    let status = child.lock().wait().map_err(|e| e.to_string())?;
    done.store(true, Ordering::SeqCst);
    let _ = writer.join();
    if let Some(h) = err_reader {
        let _ = h.join();
    }
    let _ = watcher.join();
    let stderr = tail.lock().trim().to_string();
    Ok(Ended { code: status.code(), stderr, cancelled: cancel.load(Ordering::SeqCst) })
}

/// A program's first line of output for `--version`, waiting at most 15 s.
pub fn version(exe: &Path) -> Option<String> {
    let mut c = Command::new(exe);
    c.arg("--version").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    hidden(&mut c);
    let mut child = c.spawn().ok()?;
    let mut out = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < Duration::from_secs(15) => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                kill_tree(&mut child);
                return None;
            }
        }
    }
    let s = reader.join().ok()?;
    s.lines().map(str::trim).find(|l| !l.is_empty()).map(|l| l.chars().take(120).collect())
}

/// Folders to look in for a program: PATH (as this process got it, then as
/// it is now in the registry, in case it was installed after OmniHub
/// started) and the places its installers use.
fn search_dirs(extra: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        let read = |root, key: &str| winreg::RegKey::predef(root).open_subkey(key).ok().and_then(|k| k.get_value::<String, _>("Path").ok());
        for p in [read(HKEY_CURRENT_USER, "Environment"), read(HKEY_LOCAL_MACHINE, r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment")].into_iter().flatten() {
            for part in p.split(';').filter(|s| !s.trim().is_empty()) {
                dirs.push(PathBuf::from(expand_env(part.trim())));
            }
        }
    }
    dirs.extend(extra.iter().cloned());
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| seen.insert(d.to_string_lossy().to_lowercase()));
    dirs
}

/// `%NAME%` expanded from the environment.
pub fn expand_env(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('%') {
            Some(j) if j > 0 => {
                let name = &after[..j];
                match std::env::var(name) {
                    Ok(v) => out.push_str(&v),
                    Err(_) => out.push_str(&rest[i..i + j + 2]),
                }
                rest = &after[j + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Where `name` is installed, if anywhere.
pub fn find(name: &str, extra: &[PathBuf]) -> Option<PathBuf> {
    let names: Vec<String> = if cfg!(windows) { ["exe", "cmd", "bat"].iter().map(|e| format!("{name}.{e}")).collect() } else { vec![name.to_string()] };
    for d in search_dirs(extra) {
        for n in &names {
            let p = d.join(n);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

/// The usual install places of Claude Code and Codex besides PATH.
pub fn usual_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(h) = dirs::home_dir() {
        v.push(h.join(".local").join("bin"));
        v.push(h.join(".bun").join("bin"));
        v.push(h.join(".cargo").join("bin"));
    }
    if let Some(a) = std::env::var_os("APPDATA") {
        v.push(PathBuf::from(a).join("npm"));
    }
    if let Some(l) = std::env::var_os("LOCALAPPDATA") {
        v.push(PathBuf::from(&l).join("Microsoft").join("WinGet").join("Links"));
        v.push(PathBuf::from(&l).join("Programs").join("claude"));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_environment_names() {
        std::env::set_var("OMNIHUB_TEST_DIR", "C:\\Users\\me");
        assert_eq!(expand_env("%OMNIHUB_TEST_DIR%\\bin;%NOPE_NOT_SET%\\x;100%"), "C:\\Users\\me\\bin;%NOPE_NOT_SET%\\x;100%");
    }

    #[cfg(unix)]
    #[test]
    fn runs_reads_lines_and_stops() {
        use std::os::unix::fs::PermissionsExt;
        let t = tempfile::tempdir().unwrap();
        let script = t.path().join("fake");
        std::fs::write(&script, "#!/bin/sh\nread line\necho \"{\\\"got\\\": \\\"$line\\\"}\"\necho not json\necho oops >&2\necho '{\"n\": 2}'\nexit 3\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut seen = Vec::new();
        let end = run(&script, &[], Some(t.path()), "hello --flag\n", &Arc::new(AtomicBool::new(false)), &mut |v| seen.push(v.clone())).unwrap();
        assert_eq!(seen, vec![serde_json::json!({"got": "hello --flag"}), serde_json::json!({"n": 2})]);
        assert_eq!((end.code, end.stderr.as_str(), end.cancelled), (Some(3), "oops", false));

        let slow = t.path().join("slow");
        std::fs::write(&slow, "#!/bin/sh\necho '{\"a\": 1}'\nsleep 30\n").unwrap();
        std::fs::set_permissions(&slow, std::fs::Permissions::from_mode(0o755)).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let c2 = cancel.clone();
        let started = std::time::Instant::now();
        let end = run(&slow, &[], None, "", &cancel, &mut |_| c2.store(true, Ordering::SeqCst)).unwrap();
        assert!(end.cancelled && started.elapsed() < Duration::from_secs(10));
        assert_eq!(version(&script).as_deref(), Some("{\"got\": \"\"}"));
    }
}
