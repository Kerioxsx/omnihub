//! App updates through winget (Windows' own package manager): which
//! installed apps have a newer version, and updating them one by one.
//! winget prints a table, read by the columns its header line sets.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdate {
    pub name: String,
    /// winget's package id ("Mozilla.Firefox").
    pub id: String,
    pub version: String,
    pub available: String,
    pub source: String,
}

/// The rows of `winget upgrade`'s table.
pub fn parse_upgrades(out: &str) -> Vec<AppUpdate> {
    // Progress spinners and bars come before the table, joined by \r.
    let lines: Vec<&str> = out.lines().map(|l| l.rsplit('\r').next().unwrap_or(l).trim_end()).collect();
    let Some(dash) = lines.iter().position(|l| l.len() > 10 && l.chars().all(|c| c == '-' || c == '─')) else { return Vec::new() };
    let Some(header) = dash.checked_sub(1).and_then(|i| lines.get(i)) else { return Vec::new() };
    // Where each header word starts (counted in characters).
    let hc: Vec<char> = header.chars().collect();
    let starts: Vec<usize> = (0..hc.len()).filter(|&i| !hc[i].is_whitespace() && (i == 0 || hc[i - 1].is_whitespace())).collect();
    if starts.len() < 4 {
        return Vec::new();
    }
    let col = |cs: &[char], i: usize| -> String {
        let a = starts[i].min(cs.len());
        let b = starts.get(i + 1).copied().unwrap_or(cs.len()).min(cs.len());
        cs[a..b].iter().collect::<String>().trim().to_string()
    };
    let mut out = Vec::new();
    for line in &lines[dash + 1..] {
        let cs: Vec<char> = line.chars().collect();
        // The table ends at a blank line or winget's summary ("3 upgrades available.").
        if cs.len() < starts[3] || line.trim().is_empty() {
            break;
        }
        let (name, id, version, available) = (col(&cs, 0), col(&cs, 1), col(&cs, 2), col(&cs, 3));
        if id.is_empty() || available.is_empty() || id.contains(' ') {
            continue;
        }
        out.push(AppUpdate { name, id, version, available, source: if starts.len() > 4 { col(&cs, 4) } else { String::new() } });
    }
    out
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 128 && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'))
}

#[cfg(windows)]
fn winget(args: &[&str]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("winget")
        .args(args)
        .args(["--accept-source-agreements", "--disable-interactivity"])
        .creation_flags(0x0800_0000)
        .output()
        .map_err(|_| "winget isn't on this PC (it comes with “App Installer” from the Microsoft Store).".to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(not(windows))]
fn winget(_args: &[&str]) -> Result<String, String> {
    Err("winget is on Windows.".into())
}

/// Installed apps with a newer version.
pub fn updates() -> Result<Vec<AppUpdate>, String> {
    Ok(parse_upgrades(&winget(&["upgrade", "--include-unknown"])?))
}

/// Update one app quietly (its installer may still ask for administrator rights).
pub fn update(id: &str) -> Result<(), String> {
    if !valid_id(id) {
        return Err("That isn't a winget package id.".into());
    }
    let out = winget(&["upgrade", "--id", id, "--exact", "--silent", "--accept-package-agreements"])?;
    let low = out.to_lowercase();
    if low.contains("successfully installed") || low.contains("no newer package") || low.contains("no available upgrade") {
        Ok(())
    } else {
        Err(out.lines().rev().map(str::trim).find(|l| !l.is_empty() && !l.chars().all(|c| "-\\|/ █▒".contains(c))).unwrap_or("winget didn't update it.").to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_table() {
        let out = "   - \r   \\ \r\nName                   Id                      Version       Available     Source\r\n-----------------------------------------------------------------------------------\r\nMozilla Firefox (x64)  Mozilla.Firefox         127.0         128.0.3       winget\r\nDiscord                Discord.Discord         1.0.9150      1.0.9160      winget\r\nSome Tool              Vendor.Tool             < 2.0         2.1           winget\r\n3 upgrades available.\r\n";
        let u = parse_upgrades(out);
        assert_eq!(u.len(), 3, "{u:?}");
        assert_eq!((u[0].name.as_str(), u[0].id.as_str(), u[0].version.as_str(), u[0].available.as_str(), u[0].source.as_str()), ("Mozilla Firefox (x64)", "Mozilla.Firefox", "127.0", "128.0.3", "winget"));
        assert_eq!(u[2].version, "< 2.0");
        assert!(parse_upgrades("No installed package found matching input criteria.").is_empty());
        assert!(valid_id("Mozilla.Firefox") && !valid_id("a b") && !valid_id("x;rm"));
    }
}
