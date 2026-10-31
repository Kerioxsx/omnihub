//! Developer tools: your Git projects at a glance (branch, changes, ahead
//! and behind, last commit) found in the folders you pick, and an editor
//! for your user environment variables and PATH (Windows' own, the same as
//! System Properties → Environment Variables; the system PATH is shown but
//! only an administrator can change it).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

// ---------- git projects ----------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub path: String,
    pub name: String,
    pub branch: String,
    /// Files changed, added or not tracked yet.
    pub changes: u32,
    pub ahead: u32,
    pub behind: u32,
    /// The branch follows a remote one.
    pub upstream: bool,
    /// Unix seconds of the last commit.
    pub last_commit: Option<i64>,
    pub last_message: String,
    /// Why the details are missing (Git not installed…).
    pub note: Option<String>,
}

/// Folders skipped while looking for projects (big and never projects themselves).
const SKIP: &[&str] = &["node_modules", "target", ".venv", "venv", "dist", "build", ".gradle", "AppData", "$RECYCLE.BIN", "System Volume Information", "Library"];

/// Git repositories under `roots`, up to `depth` folders down.
pub fn find_projects(roots: &[String], depth: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in roots {
        let mut it = walkdir::WalkDir::new(root).max_depth(depth).follow_links(false).into_iter();
        while let Some(Ok(e)) = it.next() {
            if !e.file_type().is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy();
            if e.depth() > 0 && (SKIP.iter().any(|s| name.eq_ignore_ascii_case(s)) || name.starts_with('.')) {
                it.skip_current_dir();
                continue;
            }
            if e.path().join(".git").exists() {
                out.push(e.path().to_path_buf());
                // A project's own folders are not more projects.
                it.skip_current_dir();
            }
            if out.len() >= 300 {
                return out;
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Read `git status --porcelain=v2 --branch` output.
pub fn parse_status(out: &str) -> (String, u32, u32, u32, bool) {
    let (mut branch, mut changes, mut ahead, mut behind, mut upstream) = (String::new(), 0, 0, 0, false);
    for line in out.lines() {
        if let Some(b) = line.strip_prefix("# branch.head ") {
            branch = if b == "(detached)" { "detached".into() } else { b.to_string() };
        } else if line.starts_with("# branch.upstream ") {
            upstream = true;
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            for part in ab.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    behind = n.parse().unwrap_or(0);
                }
            }
        } else if !line.starts_with('#') && !line.is_empty() {
            changes += 1;
        }
    }
    (branch, changes, ahead, behind, upstream)
}

/// The branch from `.git/HEAD` when Git itself isn't installed.
fn head_branch(repo: &Path) -> String {
    let git = repo.join(".git");
    // A worktree or submodule: ".git" is a file pointing at the real folder.
    let dir = if git.is_file() {
        std::fs::read_to_string(&git).ok().and_then(|s| s.strip_prefix("gitdir:").map(|p| repo.join(p.trim()))).unwrap_or(git)
    } else {
        git
    };
    std::fs::read_to_string(dir.join("HEAD")).ok().map(|h| h.trim().strip_prefix("ref: refs/heads/").map(str::to_string).unwrap_or_else(|| "detached".into())).unwrap_or_default()
}

fn git(repo: &Path, args: &[&str]) -> Option<String> {
    let mut c = std::process::Command::new("git");
    c.arg("-C").arg(repo).args(args).env("GIT_OPTIONAL_LOCKS", "0").env("GIT_TERMINAL_PROMPT", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // no console window
    }
    let out = c.output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Details of one project (Git is asked only to read; nothing is fetched).
pub fn project(repo: &Path) -> Project {
    let name = repo.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let mut p = Project { path: repo.to_string_lossy().into(), name, ..Default::default() };
    match git(repo, &["status", "--porcelain=v2", "--branch", "--untracked-files=normal"]) {
        Some(out) => {
            let (branch, changes, ahead, behind, upstream) = parse_status(&out);
            p.branch = branch;
            p.changes = changes;
            p.ahead = ahead;
            p.behind = behind;
            p.upstream = upstream;
            if let Some(log) = git(repo, &["log", "-1", "--format=%ct%x00%s"]) {
                let mut it = log.trim_end().splitn(2, '\0');
                p.last_commit = it.next().and_then(|t| t.parse().ok());
                p.last_message = it.next().unwrap_or("").chars().take(120).collect();
            }
        }
        None => {
            p.branch = head_branch(repo);
            p.note = Some("Install Git to see changes and commits.".into());
        }
    }
    p
}

// ---------- environment variables ----------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PathEntry {
    pub path: String,
    /// The folder exists (after expanding %VARIABLES%).
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub user_path: Vec<PathEntry>,
    pub system_path: Vec<PathEntry>,
    /// Your other variables (name, value), PATH aside.
    pub user_vars: Vec<(String, String)>,
}

/// PATH as entries: split on ';', trimmed, empty ones dropped.
pub fn split_path(value: &str) -> Vec<String> {
    value.split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// Entries back into one value, without duplicates (first one wins, case-insensitive).
pub fn join_path(entries: &[String]) -> String {
    let mut seen = std::collections::HashSet::new();
    entries.iter().map(|e| e.trim()).filter(|e| !e.is_empty() && seen.insert(e.trim_end_matches('\\').to_lowercase())).collect::<Vec<_>>().join(";")
}

/// `%USERPROFILE%\bin` → `C:\Users\Sam\bin`.
pub fn expand(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find('%') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('%') {
            Some(j) if j > 0 => {
                let name = &after[..j];
                out.push_str(&std::env::var(name).unwrap_or_else(|_| format!("%{name}%")));
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

/// A variable name Windows accepts.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 255 && !name.contains('=') && !name.chars().any(|c| c.is_control())
}

fn entries(value: &str) -> Vec<PathEntry> {
    split_path(value).into_iter().map(|p| PathEntry { exists: Path::new(&expand(&p)).is_dir(), path: p }).collect()
}

pub fn environment() -> Environment {
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        let user = winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey("Environment").ok();
        let system = winreg::RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment").ok();
        let get = |k: &Option<winreg::RegKey>, n: &str| k.as_ref().and_then(|k| k.get_value::<String, _>(n).ok()).unwrap_or_default();
        let mut vars: Vec<(String, String)> = user.as_ref().map(|k| k.enum_values().flatten().filter(|(n, _)| !n.eq_ignore_ascii_case("Path")).map(|(n, v)| (n, v.to_string())).collect()).unwrap_or_default();
        vars.sort_by_key(|(n, _)| n.to_lowercase());
        Environment { user_path: entries(&get(&user, "Path")), system_path: entries(&get(&system, "Path")), user_vars: vars }
    }
    #[cfg(not(windows))]
    {
        Environment { user_path: entries(&std::env::var("PATH").unwrap_or_default().replace(':', ";")), ..Default::default() }
    }
}

/// Save your PATH (in this order) and tell running programs. New terminals see it.
pub fn set_user_path(entries: &[String]) -> Result<(), String> {
    set_var("Path", &join_path(entries))
}

/// Set (or with `None`, remove) one of your variables.
pub fn set_var(name: &str, value: &str) -> Result<(), String> {
    if !valid_name(name) {
        return Err("That isn't a name Windows accepts.".into());
    }
    #[cfg(windows)]
    {
        use winreg::enums::{RegType, HKEY_CURRENT_USER, KEY_SET_VALUE};
        let key = winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Environment", KEY_SET_VALUE).map_err(|e| e.to_string())?;
        // REG_EXPAND_SZ, so %VARIABLES% inside keep working.
        let wide: Vec<u8> = value.encode_utf16().chain([0]).flat_map(|u| u.to_le_bytes()).collect();
        key.set_raw_value(name, &winreg::RegValue { bytes: wide.into(), vtype: RegType::REG_EXPAND_SZ }).map_err(|e| e.to_string())?;
        broadcast();
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = value;
        Err("Windows only.".into())
    }
}

pub fn remove_var(name: &str) -> Result<(), String> {
    if name.eq_ignore_ascii_case("Path") {
        return Err("Edit PATH's folders instead of removing it.".into());
    }
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
        let key = winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Environment", KEY_SET_VALUE).map_err(|e| e.to_string())?;
        key.delete_value(name).map_err(|e| e.to_string())?;
        broadcast();
        Ok(())
    }
    #[cfg(not(windows))]
    {
        Err("Windows only.".into())
    }
}

/// "Environment changed" to every window, like System Properties does.
#[cfg(windows)]
fn broadcast() {
    use windows::core::w;
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE};
    unsafe {
        let _ = SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, WPARAM(0), LPARAM(w!("Environment").as_ptr() as isize), SMTO_ABORTIFHUNG, 2000, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_status_and_projects() {
        let out = "# branch.oid abc\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +2 -1\n1 .M N... 100644 100644 100644 a b src/x.rs\n? new.txt\n";
        assert_eq!(parse_status(out), ("main".into(), 2, 2, 1, true));
        assert_eq!(parse_status("# branch.head (detached)\n").0, "detached");

        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("work").join("app");
        std::fs::create_dir_all(a.join(".git")).unwrap();
        std::fs::write(a.join(".git").join("HEAD"), "ref: refs/heads/feature/x\n").unwrap();
        // Inside a project, and in node_modules: not separate projects.
        std::fs::create_dir_all(a.join("vendor").join("lib").join(".git")).unwrap();
        std::fs::create_dir_all(dir.path().join("node_modules").join("pkg").join(".git")).unwrap();
        let found = find_projects(&[dir.path().to_string_lossy().into()], 4);
        assert_eq!(found, vec![a.clone()]);
        assert_eq!(head_branch(&a), "feature/x");
        let p = project(&a);
        assert_eq!(p.name, "app");
        assert!(!p.branch.is_empty());
    }

    #[test]
    fn path_editing() {
        assert_eq!(split_path(" C:\\a ;;C:\\b;"), vec!["C:\\a", "C:\\b"]);
        assert_eq!(join_path(&["C:\\a".into(), "c:\\A\\".into(), " ".into(), "C:\\b".into()]), "C:\\a;C:\\b");
        std::env::set_var("OMNIHUB_TEST_DIR", "/tmp");
        assert_eq!(expand("%OMNIHUB_TEST_DIR%/x"), "/tmp/x");
        assert_eq!(expand("100% sure %NOPE_X%"), "100% sure %NOPE_X%");
        assert!(valid_name("JAVA_HOME") && !valid_name("A=B") && !valid_name(""));
        assert!(remove_var("PATH").is_err());
    }
}
