//! Focus: a focus timer (25/5 by default) that closes the apps you choose
//! to keep away while it runs, and a time tracker that adds up how long
//! each program was the one in front (only while you are at the PC). The
//! tracker is off until you turn it on, keeps its numbers on this PC only,
//! and never records window titles: just the program and the minutes.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::Db;

/// How often the program in front is looked at.
pub const SAMPLE: Duration = Duration::from_secs(5);
/// No keyboard or mouse for this long counts as away.
pub const AWAY_AFTER: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppTime {
    /// Lower-case executable name.
    pub app: String,
    pub name: String,
    pub seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DayTotal {
    /// "2026-10-31".
    pub day: String,
    pub seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TimeReport {
    pub tracking: bool,
    pub day: String,
    pub apps: Vec<AppTime>,
    /// The last 7 days, oldest first.
    pub week: Vec<DayTotal>,
    pub focus_minutes_today: u64,
    pub sessions_today: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Focus,
    Break,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FocusState {
    pub phase: Phase,
    /// Unix ms when this phase ends.
    pub ends_at: i64,
    pub focus_minutes: u32,
    pub break_minutes: u32,
    /// Rounds done in this session.
    pub rounds: u32,
    /// Executables closed when they open during focus ("discord.exe").
    pub blocked: Vec<String>,
    /// Apps closed during this session.
    pub closed: Vec<String>,
}

/// Which day a moment belongs to (local time).
pub fn day_of(t: chrono::DateTime<chrono::Local>) -> String {
    t.format("%Y-%m-%d").to_string()
}

/// Today (local time).
pub fn today() -> String {
    day_of(chrono::Local::now())
}

/// The last `n` days, oldest first, ending today.
pub fn last_days(n: i64) -> Vec<String> {
    let now = chrono::Local::now();
    (0..n).rev().map(|d| day_of(now - chrono::Duration::days(d))).collect()
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Add up samples: the same program in a row is one run.
pub fn add(totals: &mut HashMap<String, u64>, app: &str, seconds: u64) {
    if !app.is_empty() {
        *totals.entry(app.to_string()).or_default() += seconds;
    }
}

/// Programs that aren't "using" anything (the lock screen, the desktop itself).
pub fn counts(app: &str) -> bool {
    !matches!(app, "" | "lockapp.exe" | "logonui.exe" | "searchhost.exe" | "shellexperiencehost.exe" | "startmenuexperiencehost.exe" | "omnihub.exe")
}

pub struct Focus {
    db: Arc<Db>,
    /// Seconds not yet written, per app, for the current day.
    pending: Mutex<(String, HashMap<String, u64>)>,
    state: Mutex<Option<FocusState>>,
    last_flush: Mutex<Instant>,
}

impl Focus {
    pub fn new(db: Arc<Db>) -> Focus {
        Focus { db, pending: Mutex::new((String::new(), HashMap::new())), state: Mutex::new(None), last_flush: Mutex::new(Instant::now()) }
    }

    /// One sample: `app` was in front for `seconds`.
    pub fn sample(&self, app: &str, seconds: u64, day: &str) {
        if !counts(app) {
            return;
        }
        let mut p = self.pending.lock();
        if p.0 != day {
            let old = std::mem::take(&mut p.1);
            let old_day = std::mem::replace(&mut p.0, day.to_string());
            drop(p);
            self.write(&old_day, &old);
            p = self.pending.lock();
        }
        add(&mut p.1, app, seconds);
        drop(p);
        if self.last_flush.lock().elapsed() >= Duration::from_secs(60) {
            self.flush();
        }
    }

    fn write(&self, day: &str, totals: &HashMap<String, u64>) {
        if day.is_empty() || totals.is_empty() {
            return;
        }
        let _ = self.db.with(|c| {
            let tx = c.unchecked_transaction()?;
            for (app, secs) in totals {
                tx.execute("INSERT INTO app_time (day, app, seconds) VALUES (?1, ?2, ?3) ON CONFLICT(day, app) DO UPDATE SET seconds = seconds + ?3", params![day, app, *secs as i64])?;
            }
            tx.commit()
        });
    }

    /// Write what is pending.
    pub fn flush(&self) {
        let (day, totals) = {
            let mut p = self.pending.lock();
            (p.0.clone(), std::mem::take(&mut p.1))
        };
        self.write(&day, &totals);
        *self.last_flush.lock() = Instant::now();
    }

    pub fn report(&self, day: &str, tracking: bool, week_days: &[String]) -> TimeReport {
        self.flush();
        let apps: Vec<AppTime> = self
            .db
            .with(|c| {
                let mut st = c.prepare("SELECT app, seconds FROM app_time WHERE day = ?1 ORDER BY seconds DESC LIMIT 40")?;
                let rows = st.query_map(params![day], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
                rows.collect::<Result<Vec<_>, _>>()
            })
            .unwrap_or_default()
            .into_iter()
            .map(|(app, secs)| AppTime { name: crate::media::mixer::name_of(&app), app, seconds: secs.max(0) as u64 })
            .collect();
        let week = week_days
            .iter()
            .map(|d| DayTotal { day: d.clone(), seconds: self.db.with(|c| c.query_row("SELECT COALESCE(SUM(seconds), 0) FROM app_time WHERE day = ?1", params![d], |r| r.get::<_, i64>(0))).unwrap_or(0).max(0) as u64 })
            .collect();
        let (focus_minutes_today, sessions_today) = self
            .db
            .with(|c| c.query_row("SELECT COALESCE(SUM(minutes), 0), COUNT(*) FROM focus_sessions WHERE day = ?1", params![day], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))))
            .unwrap_or((0, 0));
        TimeReport { tracking, day: day.to_string(), apps, week, focus_minutes_today: focus_minutes_today.max(0) as u64, sessions_today: sessions_today.max(0) as u32 }
    }

    /// Forget every tracked minute.
    pub fn clear(&self) -> Result<(), String> {
        self.pending.lock().1.clear();
        self.db.with(|c| c.execute("DELETE FROM app_time", [])).map(|_| ()).map_err(|e| e.to_string())
    }

    // ---------- the focus timer ----------

    pub fn state(&self) -> Option<FocusState> {
        self.state.lock().clone()
    }

    pub fn start(&self, focus_minutes: u32, break_minutes: u32, blocked: Vec<String>, now_ms: i64) -> FocusState {
        let focus_minutes = focus_minutes.clamp(5, 180);
        let break_minutes = break_minutes.clamp(1, 60);
        let blocked = blocked.into_iter().map(|b| b.trim().to_lowercase()).filter(|b| !b.is_empty() && !crate::system::procs::is_protected(b)).collect();
        let s = FocusState { phase: Phase::Focus, ends_at: now_ms + focus_minutes as i64 * 60_000, focus_minutes, break_minutes, rounds: 0, blocked, closed: Vec::new() };
        *self.state.lock() = Some(s.clone());
        s
    }

    pub fn stop(&self, day: &str, now_ms: i64) -> Option<FocusState> {
        let s = self.state.lock().take()?;
        // A focus phase cut short still counts for the minutes it ran.
        if s.phase == Phase::Focus {
            let ran = (s.focus_minutes as i64 * 60_000 - (s.ends_at - now_ms)).max(0) / 60_000;
            self.log_session(day, ran as u32);
        }
        Some(s)
    }

    fn log_session(&self, day: &str, minutes: u32) {
        if minutes > 0 {
            let _ = self.db.with(|c| c.execute("INSERT INTO focus_sessions (day, minutes, at) VALUES (?1, ?2, ?3)", params![day, minutes, chrono::Utc::now().timestamp()]));
        }
    }

    /// Move the timer on; returns the new phase when one ended.
    pub fn tick(&self, day: &str, now_ms: i64) -> Option<Phase> {
        let mut st = self.state.lock();
        let s = st.as_mut()?;
        if now_ms < s.ends_at {
            return None;
        }
        match s.phase {
            Phase::Focus => {
                let minutes = s.focus_minutes;
                s.phase = Phase::Break;
                s.rounds += 1;
                s.ends_at = now_ms + s.break_minutes as i64 * 60_000;
                drop(st);
                self.log_session(day, minutes);
                Some(Phase::Break)
            }
            Phase::Break => {
                s.phase = Phase::Focus;
                s.ends_at = now_ms + s.focus_minutes as i64 * 60_000;
                Some(Phase::Focus)
            }
        }
    }

    /// Blocked apps to close now (during focus only).
    pub fn to_close(&self, running: &[String]) -> Vec<String> {
        let mut st = self.state.lock();
        let Some(s) = st.as_mut().filter(|s| s.phase == Phase::Focus) else { return Vec::new() };
        let hits: Vec<String> = running.iter().filter(|r| s.blocked.contains(&r.to_lowercase())).cloned().collect();
        for h in &hits {
            if !s.closed.contains(h) {
                s.closed.push(h.clone());
            }
        }
        hits
    }
}

/// The program in front and whether you are at the PC.
pub fn foreground() -> Option<String> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
        unsafe {
            let mut li = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
            if GetLastInputInfo(&mut li).as_bool() {
                let idle = windows::Win32::System::SystemInformation::GetTickCount().wrapping_sub(li.dwTime);
                if idle as u64 >= AWAY_AFTER.as_millis() as u64 {
                    return None;
                }
            }
            let hwnd = GetForegroundWindow();
            if hwnd.is_invalid() {
                return None;
            }
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            use windows::core::PWSTR;
            use windows::Win32::Foundation::CloseHandle;
            use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
            let _ = CloseHandle(h);
            ok.then(|| crate::media::mixer::key_of(&String::from_utf16_lossy(&buf[..len as usize])))
        }
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn focus() -> Focus {
        Focus::new(Arc::new(Db::in_memory().unwrap()))
    }

    #[test]
    fn tracks_time_per_app_and_day() {
        let f = focus();
        for _ in 0..12 {
            f.sample("code.exe", 5, "2026-10-30");
        }
        f.sample("lockapp.exe", 5, "2026-10-30");
        f.sample("chrome.exe", 5, "2026-10-30");
        // The day changes: yesterday's time is written.
        f.sample("chrome.exe", 5, "2026-10-31");
        let days = vec!["2026-10-30".to_string(), "2026-10-31".to_string()];
        let r = f.report("2026-10-30", true, &days);
        assert_eq!(r.apps.iter().map(|a| (a.app.as_str(), a.seconds)).collect::<Vec<_>>(), vec![("code.exe", 60), ("chrome.exe", 5)]);
        assert_eq!(r.week.iter().map(|d| d.seconds).collect::<Vec<_>>(), vec![65, 5]);
        f.clear().unwrap();
        assert!(f.report("2026-10-30", true, &days).apps.is_empty());
    }

    #[test]
    fn focus_timer_rounds_and_blocking() {
        let f = focus();
        let s = f.start(25, 5, vec!["Discord.exe".into(), "csrss.exe".into()], 0);
        assert_eq!(s.blocked, vec!["discord.exe"], "Windows' own processes are never closed");
        assert_eq!(f.to_close(&["discord.exe".into(), "code.exe".into()]), vec!["discord.exe"]);
        assert_eq!(f.tick("d", 10 * 60_000), None);
        assert_eq!(f.tick("d", 25 * 60_000), Some(Phase::Break));
        assert!(f.to_close(&["discord.exe".into()]).is_empty(), "breaks are free");
        assert_eq!(f.tick("d", 30 * 60_000), Some(Phase::Focus));
        assert_eq!(f.state().unwrap().rounds, 1);
        // Stopped 10 minutes into the second round: 10 more minutes count.
        f.stop("d", 40 * 60_000);
        let r = f.report("d", false, &[]);
        assert_eq!((r.focus_minutes_today, r.sessions_today), (35, 2));
        assert!(f.state().is_none());
    }
}
