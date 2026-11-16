//! The privacy dashboard: which programs used the microphone, the camera
//! or your location, and when, from what Windows itself records (Settings →
//! Privacy & security keeps the same list). Nothing is watched or sent:
//! OmniHub reads Windows' record when you open the page.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Use {
    /// "microphone", "webcam" or "location".
    pub what: String,
    /// The program ("Discord", "chrome.exe").
    pub app: String,
    /// Its full path or Store app id.
    pub id: String,
    /// When it last started using it (unix seconds).
    pub started: Option<i64>,
    /// When it stopped; `None` while it is still using it.
    pub stopped: Option<i64>,
    pub in_use: bool,
}

pub const KINDS: &[&str] = &["microphone", "webcam", "location"];

/// A Windows FILETIME (100 ns since 1601) as unix seconds; 0 means never.
pub fn unix_of_filetime(ft: u64) -> Option<i64> {
    (ft != 0).then(|| (ft / 10_000_000) as i64 - 11_644_473_600)
}

/// A readable name from Windows' key: `C:#Program Files#App#app.exe` (a
/// desktop program, with `#` for `\`) or a Store app's package family name.
pub fn app_name(key: &str, packaged: bool) -> (String, String) {
    if packaged {
        // "Microsoft.WindowsCamera_8wekyb3d8bbwe" → "WindowsCamera".
        let family = key.split('_').next().unwrap_or(key);
        let name = family.rsplit('.').next().unwrap_or(family);
        return (spaced(name), key.to_string());
    }
    let path = key.replace('#', "\\");
    let file = path.rsplit('\\').next().unwrap_or(&path).to_string();
    (crate::media::mixer::name_of(&file.to_lowercase()), path)
}

/// "WindowsCamera" → "Windows Camera".
fn spaced(s: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && c.is_uppercase() && chars[i - 1].is_lowercase() {
            out.push(' ');
        }
        out.push(*c);
    }
    out
}

/// In use first, then the most recent.
pub fn sort(v: &mut [Use]) {
    v.sort_by(|a, b| b.in_use.cmp(&a.in_use).then(b.started.cmp(&a.started)));
}

pub fn list() -> Vec<Use> {
    #[cfg(windows)]
    {
        let mut v = imp::list();
        sort(&mut v);
        v
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

#[cfg(windows)]
mod imp {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    use super::{app_name, unix_of_filetime, Use, KINDS};

    const ROOT: &str = r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore";

    fn read(key: &RegKey, what: &str, name: &str, packaged: bool, out: &mut Vec<Use>) {
        let start: u64 = key.get_value("LastUsedTimeStart").unwrap_or(0);
        let stop: u64 = key.get_value("LastUsedTimeStop").unwrap_or(0);
        if start == 0 {
            return;
        }
        let (app, id) = app_name(name, packaged);
        out.push(Use { what: what.into(), app, id, started: unix_of_filetime(start), stopped: unix_of_filetime(stop), in_use: stop == 0 || stop < start });
    }

    pub fn list() -> Vec<Use> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let mut out = Vec::new();
        for what in KINDS {
            let Ok(store) = hkcu.open_subkey(format!(r"{ROOT}\{what}")) else { continue };
            for name in store.enum_keys().flatten() {
                let Ok(k) = store.open_subkey(&name) else { continue };
                if name == "NonPackaged" {
                    for app in k.enum_keys().flatten() {
                        if let Ok(a) = k.open_subkey(&app) {
                            read(&a, what, &app, false, &mut out);
                        }
                    }
                } else {
                    read(&k, what, &name, true, &mut out);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_windows_records() {
        // 2024-01-01 00:00:00 UTC.
        assert_eq!(unix_of_filetime(133_485_408_000_000_000), Some(1_704_067_200));
        assert_eq!(unix_of_filetime(0), None);
        assert_eq!(app_name("Microsoft.WindowsCamera_8wekyb3d8bbwe", true).0, "Windows Camera");
        let (name, path) = app_name("C:#Program Files#Mozilla Firefox#firefox.exe", false);
        assert_eq!(path, r"C:\Program Files\Mozilla Firefox\firefox.exe");
        assert!(!name.is_empty());
        let u = |app: &str, started, in_use| Use { what: "microphone".into(), app: app.into(), id: String::new(), started: Some(started), stopped: None, in_use };
        let mut v = vec![u("old", 1, false), u("live", 0, true), u("new", 5, false)];
        sort(&mut v);
        assert_eq!(v.iter().map(|x| x.app.as_str()).collect::<Vec<_>>(), vec!["live", "new", "old"]);
    }
}
