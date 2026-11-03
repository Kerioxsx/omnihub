//! Screen-time limits for apps and websites: minutes a day in front of an
//! app (by its program) or a website (by a word in the browser's window
//! title, like "YouTube"). Only the window in front counts. Five minutes
//! before the limit you get a warning; at the limit an app is closed and a
//! website's browser window is minimized each time it comes back in front
//! (OmniHub can't close a single tab). The day starts again at midnight.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LimitKind {
    #[default]
    App,
    Site,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Limit {
    pub id: String,
    pub kind: LimitKind,
    /// A program ("tiktok.exe") or a word in the page title ("YouTube").
    pub target: String,
    pub minutes: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
struct Store {
    limits: Vec<Limit>,
    date: String,
    /// Seconds used today, by limit id.
    used: HashMap<String, u64>,
    warned: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LimitUse {
    pub limit: Limit,
    pub used_minutes: f32,
    pub over: bool,
}

/// What a check found (for the caller to act on).
#[derive(Debug, Clone, PartialEq)]
pub enum Act {
    Warn { name: String, left: u32 },
    CloseApp { exe: String, name: String },
    HideSite { name: String },
}

pub const BROWSERS: &[&str] = &["chrome.exe", "msedge.exe", "firefox.exe", "brave.exe", "opera.exe", "opera_gx.exe", "vivaldi.exe", "arc.exe"];

pub struct ScreenTime {
    file: PathBuf,
    store: Mutex<Store>,
}

fn name_of(l: &Limit) -> String {
    match l.kind {
        LimitKind::App => l.target.trim_end_matches(".exe").to_string(),
        LimitKind::Site => l.target.clone(),
    }
}

impl ScreenTime {
    pub fn new(data: &Path) -> ScreenTime {
        let file = data.join("screen-time.json");
        let store = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        ScreenTime { file, store: Mutex::new(store) }
    }

    fn write(&self, s: &Store) {
        if let Ok(j) = serde_json::to_vec(s) {
            let _ = crate::settings::write_atomic(&self.file, &j);
        }
    }

    pub fn list(&self, today: &str) -> Vec<LimitUse> {
        let mut s = self.store.lock();
        if s.date != today {
            s.date = today.to_string();
            s.used.clear();
            s.warned.clear();
        }
        s.limits.iter().map(|l| {
            let used = *s.used.get(&l.id).unwrap_or(&0) as f32 / 60.0;
            LimitUse { limit: l.clone(), used_minutes: used, over: used >= l.minutes as f32 }
        }).collect()
    }

    pub fn save(&self, mut l: Limit) -> Result<Limit, String> {
        l.target = l.target.trim().to_string();
        if l.target.is_empty() {
            return Err("Pick an app or type a website word".into());
        }
        if l.kind == LimitKind::App {
            l.target = l.target.to_lowercase();
            if !l.target.ends_with(".exe") {
                l.target.push_str(".exe");
            }
            if crate::system::procs::is_protected(&l.target) || ["explorer.exe", "omnihub.exe", "taskmgr.exe"].contains(&l.target.as_str()) {
                return Err("That's part of Windows, so it can't have a limit".into());
            }
        }
        l.minutes = l.minutes.clamp(5, 24 * 60);
        if l.id.is_empty() {
            l.id = uuid::Uuid::new_v4().simple().to_string()[..10].to_string();
        }
        let mut s = self.store.lock();
        match s.limits.iter_mut().find(|x| x.id == l.id) {
            Some(x) => *x = l.clone(),
            None => s.limits.push(l.clone()),
        }
        let snapshot = s.clone();
        drop(s);
        self.write(&snapshot);
        Ok(l)
    }

    pub fn delete(&self, id: &str) {
        let mut s = self.store.lock();
        s.limits.retain(|l| l.id != id);
        let snapshot = s.clone();
        drop(s);
        self.write(&snapshot);
    }

    /// Count `secs` for whatever is in front and say what to do.
    pub fn tick(&self, exe: Option<&str>, title: Option<&str>, secs: u64, today: &str) -> Vec<Act> {
        let exe = exe.map(|e| e.rsplit(['\\', '/']).next().unwrap_or(e).to_lowercase()).unwrap_or_default();
        let title = title.unwrap_or_default().to_lowercase();
        let browser = BROWSERS.contains(&exe.as_str());
        let mut s = self.store.lock();
        if !s.limits.iter().any(|l| l.enabled) {
            return Vec::new();
        }
        let mut changed = false;
        if s.date != today {
            s.date = today.to_string();
            s.used.clear();
            s.warned.clear();
            changed = true;
        }
        let mut out = Vec::new();
        let limits = s.limits.clone();
        for l in limits.iter().filter(|l| l.enabled) {
            let hit = match l.kind {
                LimitKind::App => !exe.is_empty() && exe == l.target,
                LimitKind::Site => browser && title.contains(&l.target.to_lowercase()),
            };
            if !hit {
                continue;
            }
            changed = true;
            let used = s.used.entry(l.id.clone()).or_insert(0);
            *used += secs;
            let used_min = *used / 60;
            let limit = l.minutes as u64;
            if used_min >= limit {
                out.push(match l.kind {
                    LimitKind::App => Act::CloseApp { exe: l.target.clone(), name: name_of(l) },
                    LimitKind::Site => Act::HideSite { name: name_of(l) },
                });
            } else if limit - used_min <= 5 && !s.warned.contains(&l.id) {
                s.warned.push(l.id.clone());
                out.push(Act::Warn { name: name_of(l), left: (limit - used_min) as u32 });
            }
        }
        if changed {
            let snapshot = s.clone();
            drop(s);
            self.write(&snapshot);
        }
        out
    }
}

/// Today on this PC ("2026-11-03"), the day limits count in.
pub fn local_today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Minimize the window in front (a website over its limit).
pub fn minimize_foreground() {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, ShowWindow, SW_MINIMIZE};
        let h = GetForegroundWindow();
        if !h.is_invalid() {
            let _ = ShowWindow(h, SW_MINIMIZE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_warns_and_acts() {
        let t = tempfile::tempdir().unwrap();
        let st = ScreenTime::new(t.path());
        let app = st.save(Limit { kind: LimitKind::App, target: "TikTok".into(), minutes: 10, enabled: true, ..Default::default() }).unwrap();
        assert_eq!(app.target, "tiktok.exe");
        st.save(Limit { kind: LimitKind::Site, target: "YouTube".into(), minutes: 6, enabled: true, ..Default::default() }).unwrap();
        assert!(st.save(Limit { kind: LimitKind::App, target: "explorer.exe".into(), minutes: 10, enabled: true, ..Default::default() }).is_err());

        let day = "2026-11-03";
        assert!(st.tick(Some(r"C:\Apps\TikTok.exe"), Some("TikTok"), 240, day).is_empty());
        let a = st.tick(Some(r"C:\Apps\TikTok.exe"), Some("TikTok"), 120, day);
        assert_eq!(a, vec![Act::Warn { name: "tiktok".into(), left: 4 }]);
        assert!(st.tick(Some(r"C:\Apps\TikTok.exe"), None, 60, day).is_empty(), "warned once");
        let a = st.tick(Some(r"C:\Apps\TikTok.exe"), None, 240, day);
        assert_eq!(a, vec![Act::CloseApp { exe: "tiktok.exe".into(), name: "tiktok".into() }]);

        assert!(st.tick(Some("notepad.exe"), Some("YouTube notes.txt"), 999, day).is_empty(), "only in a browser");
        let a = st.tick(Some(r"C:\Chrome\chrome.exe"), Some("Cats - YouTube - Google Chrome"), 360, day);
        assert_eq!(a, vec![Act::HideSite { name: "YouTube".into() }]);

        let list = st.list(day);
        assert!(list.iter().all(|l| l.over));
        assert!(st.list("2026-11-04").iter().all(|l| !l.over && l.used_minutes == 0.0), "a new day");
    }
}
