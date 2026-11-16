//! Watching on the couch: which browser opens the streaming sites (picked
//! the first time Big Picture opens one, changeable in settings), and
//! "Continue watching": the series and videos you were last watching on
//! those sites, read from that browser's own history on this PC.
//!
//! Only history rows whose address is a streaming site's watch page are
//! read (the query asks for those addresses and nothing else), from a copy
//! of the history file that is deleted right after. Nothing leaves the PC,
//! and it stays off until you turn it on.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Browser {
    /// "edge", "chrome", "brave", "firefox", "vivaldi", "opera", "operagx".
    pub id: String,
    pub name: String,
    pub exe: String,
    /// "chromium" or "firefox".
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WatchPrefs {
    /// The browser id; empty until picked.
    pub browser: String,
    /// Show "Continue watching" (reads the browser's history for these sites).
    pub continue_watching: bool,
    /// The controller works inside the browser while a site is open.
    pub remote: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Watching {
    /// The quick site it belongs to ("netflix").
    pub site: String,
    pub service: String,
    /// The series or video ("" when the site's history doesn't name it).
    pub show: String,
    pub episode: String,
    /// Opens where you were.
    pub url: String,
    /// Unix ms.
    pub at: i64,
    pub thumb: Option<String>,
}

/// (id, name, program, kind, user data folder under %LOCALAPPDATA% or %APPDATA%, in AppData\Roaming).
const KNOWN: &[(&str, &str, &str, &str, &str, bool)] = &[
    ("edge", "Microsoft Edge", "msedge.exe", "chromium", r"Microsoft\Edge\User Data", false),
    ("chrome", "Google Chrome", "chrome.exe", "chromium", r"Google\Chrome\User Data", false),
    ("brave", "Brave", "brave.exe", "chromium", r"BraveSoftware\Brave-Browser\User Data", false),
    ("vivaldi", "Vivaldi", "vivaldi.exe", "chromium", r"Vivaldi\User Data", false),
    ("opera", "Opera", "opera.exe", "chromium", r"Opera Software\Opera Stable", true),
    ("operagx", "Opera GX", "opera.exe", "chromium", r"Opera Software\Opera GX Stable", true),
    ("firefox", "Firefox", "firefox.exe", "firefox", r"Mozilla\Firefox\Profiles", true),
];

/// (site id, service, watch-page address patterns for SQL LIKE).
pub const RULES: &[(&str, &str, &[&str])] = &[
    ("netflix", "Netflix", &["https://www.netflix.com/watch/%"]),
    ("crunchyroll", "Crunchyroll", &["https://www.crunchyroll.com/watch/%", "https://www.crunchyroll.com/%/watch/%"]),
    ("disney", "Disney+", &["https://www.disneyplus.com/video/%", "https://www.disneyplus.com/play/%", "https://www.disneyplus.com/%/video/%", "https://www.disneyplus.com/%/play/%"]),
    ("hbomax", "HBO Max", &["https://play.max.com/video/watch/%", "https://play.hbomax.com/video/watch/%"]),
    ("prime", "Prime Video", &["https://www.primevideo.com/detail/%", "https://www.primevideo.com/%/detail/%"]),
    ("paramount", "Paramount+", &["https://www.paramountplus.com/shows/video/%", "https://www.paramountplus.com/movies/video/%"]),
    ("appletv", "Apple TV", &["https://tv.apple.com/%/episode/%"]),
    ("youtube", "YouTube", &["https://www.youtube.com/watch?v=%"]),
];

/// Words a page title ends with that only name the site.
const SITE_WORDS: &[&str] = &["netflix", "youtube", "crunchyroll", "disney+", "disney plus", "max", "hbo max", "prime video", "amazon", "paramount+", "paramount plus", "apple tv", "apple tv+", "official site"];

/// Which rule an address belongs to.
pub fn rule_for(url: &str) -> Option<&'static (&'static str, &'static str, &'static [&'static str])> {
    RULES.iter().find(|(_, _, pats)| pats.iter().any(|p| like(url, p)))
}

/// SQL LIKE with % only (what the patterns use), case-sensitive like the addresses.
fn like(s: &str, pat: &str) -> bool {
    let parts: Vec<&str> = pat.split('%').collect();
    let mut rest = s;
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            match rest.strip_prefix(part) {
                Some(r) => rest = r,
                None => return false,
            }
        } else if i == parts.len() - 1 && !part.is_empty() {
            return rest.ends_with(part);
        } else if let Some(at) = rest.find(part) {
            rest = &rest[at + part.len()..];
        } else {
            return false;
        }
    }
    true
}

/// A page title as (series, episode): the site's name and "Watch" taken off,
/// split where the episode starts ("Show - Episode 3", "Show S2:E4", "Show Season 1").
pub fn split_title(raw: &str) -> (String, String) {
    let mut t = raw.trim().to_string();
    // Trailing " | Netflix", " - YouTube", " • Max"… (possibly twice: "… | Watch on Crunchyroll").
    for _ in 0..3 {
        let cut = [" | ", " - ", " – ", " • ", " · "].iter().filter_map(|sep| t.rfind(sep).map(|i| (i, sep.len()))).max_by_key(|(i, _)| *i);
        let Some((i, n)) = cut else { break };
        let last = t[i + n..].trim().to_ascii_lowercase();
        let last = last.trim_start_matches("watch on ").trim_start_matches("watch ").to_string();
        if SITE_WORDS.iter().any(|w| last == *w) {
            t.truncate(i);
        } else {
            break;
        }
    }
    let lower = t.to_ascii_lowercase();
    if SITE_WORDS.contains(&lower.trim()) {
        return (String::new(), String::new());
    }
    if lower.starts_with("watch ") {
        t = t[6..].to_string();
    }
    let lower = t.to_ascii_lowercase();
    // Where the episode part starts.
    let mut at: Option<usize> = None;
    for m in [" episode ", " ep. ", " ep ", " season ", ": season", ": episode"] {
        if let Some(i) = lower.find(m) {
            at = Some(at.map_or(i, |a: usize| a.min(i)));
        }
    }
    // " S2:E4" / " S2 E4" / " S02E04"
    let b = lower.as_bytes();
    for i in 0..b.len().saturating_sub(3) {
        if b[i] == b' ' && b[i + 1] == b's' && b[i + 2].is_ascii_digit() && lower[i + 2..].chars().take(6).any(|c| c == 'e') {
            at = Some(at.map_or(i, |a| a.min(i)));
            break;
        }
    }
    if at.is_none() {
        at = lower.find(" - ");
    }
    match at {
        Some(i) if i > 0 => {
            let show = t[..i].trim().trim_end_matches([':', '-', '–', '|']).trim().to_string();
            let ep = t[i..].trim().trim_start_matches(['-', '–', ':', '|']).trim().to_string();
            (show, ep)
        }
        _ => (t.trim().to_string(), String::new()),
    }
}

/// The YouTube video id in a watch address.
pub fn youtube_id(url: &str) -> Option<&str> {
    let q = url.split_once("watch?")?.1;
    q.split('&').find_map(|kv| kv.strip_prefix("v=")).filter(|id| id.len() == 11 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
}

/// One history row: (address, page title, unix ms).
pub type Row = (String, String, i64);

/// The series and videos, newest first: one entry per series (the latest
/// episode), at most `limit`, YouTube kept to six.
pub fn summarize(rows: &[Row], now_ms: i64, limit: usize) -> Vec<Watching> {
    let mut out: Vec<Watching> = Vec::new();
    let mut rows: Vec<&Row> = rows.iter().filter(|r| now_ms - r.2 < 60 * 86_400_000).collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r.2));
    let mut youtube = 0;
    for (url, title, at) in rows {
        let Some((site, service, _)) = rule_for(url) else { continue };
        let (show, episode) = split_title(title);
        let key_show = if *site == "youtube" { url.clone() } else { show.to_lowercase() };
        if out.iter().any(|w| w.site == *site && (if *site == "youtube" { w.url == key_show } else { w.show.to_lowercase() == key_show })) {
            continue;
        }
        if *site == "youtube" {
            if youtube >= 6 || show.is_empty() {
                continue;
            }
            youtube += 1;
        }
        let thumb = if *site == "youtube" { youtube_id(url).map(|id| format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg")) } else { None };
        out.push(Watching { site: site.to_string(), service: service.to_string(), show, episode, url: url.clone(), at: *at, thumb });
        if out.len() >= limit {
            break;
        }
    }
    out
}

/// The SQL that reads only the watch pages from a Chromium or Firefox history.
pub fn query(kind: &str) -> (String, Vec<String>) {
    let pats: Vec<String> = RULES.iter().flat_map(|(_, _, p)| p.iter().map(|s| s.to_string())).collect();
    let (table, col, time) = if kind == "firefox" { ("moz_places", "url", "last_visit_date") } else { ("urls", "url", "last_visit_time") };
    let cond = (1..=pats.len()).map(|i| format!("{col} LIKE ?{i}")).collect::<Vec<_>>().join(" OR ");
    (format!("SELECT url, COALESCE(title, ''), {time} FROM {table} WHERE ({cond}) AND {time} IS NOT NULL ORDER BY {time} DESC LIMIT 600"), pats)
}

/// A history time as unix ms (Chromium counts µs from 1601, Firefox µs from 1970).
pub fn to_unix_ms(kind: &str, t: i64) -> i64 {
    if kind == "firefox" {
        t / 1000
    } else {
        t / 1000 - 11_644_473_600_000
    }
}

/// Read the watch pages from one history file (through a copy, deleted after).
pub fn read_history(file: &Path, kind: &str) -> Vec<Row> {
    let tmp = std::env::temp_dir().join(format!("omnihub-watch-{}.sqlite", uuid::Uuid::new_v4().simple()));
    if std::fs::copy(file, &tmp).is_err() {
        return Vec::new();
    }
    // Firefox keeps recent visits in its write-ahead log for a while.
    let wal = file.with_file_name(format!("{}-wal", file.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
    let tmp_wal = tmp.with_file_name(format!("{}-wal", tmp.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()));
    if wal.is_file() {
        let _ = std::fs::copy(&wal, &tmp_wal);
    }
    let rows = (|| -> rusqlite::Result<Vec<Row>> {
        let c = rusqlite::Connection::open_with_flags(&tmp, rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        let (sql, pats) = query(kind);
        let mut st = c.prepare(&sql)?;
        let it = st.query_map(rusqlite::params_from_iter(pats.iter()), |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?)))?;
        Ok(it.filter_map(Result::ok).map(|(u, t, at)| (u, t, to_unix_ms(kind, at))).collect())
    })()
    .unwrap_or_default();
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&tmp_wal);
    let _ = std::fs::remove_file(tmp.with_file_name(format!("{}-shm", tmp.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())));
    rows
}

/// The history files of a browser's profiles.
pub fn history_files(id: &str, local: Option<&Path>, roaming: Option<&Path>) -> Vec<PathBuf> {
    let Some((_, _, _, kind, rel, in_roaming)) = KNOWN.iter().find(|k| k.0 == id) else { return Vec::new() };
    let Some(base) = (if *in_roaming { roaming } else { local }).map(|b| b.join(rel)) else { return Vec::new() };
    let mut out = Vec::new();
    if *kind == "firefox" {
        for e in std::fs::read_dir(&base).into_iter().flatten().flatten() {
            let f = e.path().join("places.sqlite");
            if f.is_file() {
                out.push(f);
            }
        }
        return out;
    }
    // Opera keeps it at the top (older) or in Default (newer); others per profile.
    if base.join("History").is_file() {
        out.push(base.join("History"));
    }
    for e in std::fs::read_dir(&base).into_iter().flatten().flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if (n == "Default" || n.starts_with("Profile ")) && e.path().join("History").is_file() {
            out.push(e.path().join("History"));
        }
    }
    out
}

pub struct Watch {
    file: PathBuf,
    prefs: Mutex<WatchPrefs>,
}

impl Watch {
    pub fn new(data: &Path) -> Watch {
        let file = data.join("watch-prefs.json");
        let prefs = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(WatchPrefs { remote: true, ..Default::default() });
        Watch { file, prefs: Mutex::new(prefs) }
    }

    pub fn prefs(&self) -> WatchPrefs {
        self.prefs.lock().clone()
    }

    pub fn set_prefs(&self, p: WatchPrefs) -> Result<WatchPrefs, String> {
        if !p.browser.is_empty() && p.browser != "system" && !KNOWN.iter().any(|k| k.0 == p.browser) {
            return Err("Pick a browser from the list".into());
        }
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        *self.prefs.lock() = p.clone();
        Ok(p)
    }

    /// The chosen browser, if it's still installed.
    pub fn browser(&self) -> Option<Browser> {
        let id = self.prefs.lock().browser.clone();
        browsers().into_iter().find(|b| b.id == id)
    }

    /// Continue watching (empty when it's off or no browser is picked).
    pub fn continue_watching(&self, limit: usize) -> Vec<Watching> {
        let p = self.prefs();
        if !p.continue_watching {
            return Vec::new();
        }
        let Some(b) = KNOWN.iter().find(|k| k.0 == p.browser) else { return Vec::new() };
        let mut rows = Vec::new();
        for f in history_files(b.0, dirs::data_local_dir().as_deref(), dirs::data_dir().as_deref()) {
            rows.extend(read_history(&f, b.3));
        }
        summarize(&rows, chrono::Utc::now().timestamp_millis(), limit)
    }

    /// Open a site in the chosen browser (full screen with no bars where the
    /// browser can), else the old way (Edge full screen, or the usual browser).
    pub fn open(&self, url: &str, full_screen: bool) -> Result<Option<Browser>, String> {
        if !crate::sites::valid_url(url) {
            return Err("That isn't a web address".into());
        }
        match self.browser() {
            Some(b) => {
                let mut c = std::process::Command::new(&b.exe);
                match (b.kind.as_str(), full_screen) {
                    ("firefox", true) => c.arg("--kiosk").arg(url),
                    ("firefox", false) => c.arg("--new-window").arg(url),
                    (_, true) => c.arg(format!("--app={url}")).arg("--start-fullscreen"),
                    (_, false) => c.arg("--new-window").arg(url),
                };
                c.spawn().map_err(|e| format!("{} didn't start: {e}", b.name))?;
                Ok(Some(b))
            }
            None => crate::sites::open(url, full_screen).map(|_| None),
        }
    }
}

/// The browsers installed on this PC.
pub fn browsers() -> Vec<Browser> {
    KNOWN
        .iter()
        .filter_map(|(id, name, exe, kind, rel, roaming)| imp::find(id, exe, rel, *roaming).map(|p| Browser { id: id.to_string(), name: name.to_string(), exe: p.display().to_string(), kind: kind.to_string() }))
        .collect()
}

#[cfg(windows)]
mod imp {
    use std::path::PathBuf;

    pub fn find(id: &str, exe: &str, _rel: &str, _roaming: bool) -> Option<PathBuf> {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        use winreg::RegKey;
        let env = |v: &str| std::env::var_os(v).map(PathBuf::from);
        let mut tries: Vec<PathBuf> = Vec::new();
        // Opera and Opera GX share a program name: only their own folders tell them apart.
        if !id.starts_with("opera") {
            for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
                if let Ok(k) = RegKey::predef(root).open_subkey(format!(r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\{exe}")) {
                    if let Ok(p) = k.get_value::<String, _>("") {
                        tries.push(PathBuf::from(p.trim_matches('"')));
                    }
                }
            }
        }
        let (pf, pf86, local) = (env("ProgramFiles"), env("ProgramFiles(x86)"), env("LOCALAPPDATA"));
        let rels: &[&str] = match id {
            "edge" => &[r"Microsoft\Edge\Application\msedge.exe"],
            "chrome" => &[r"Google\Chrome\Application\chrome.exe"],
            "brave" => &[r"BraveSoftware\Brave-Browser\Application\brave.exe"],
            "vivaldi" => &[r"Vivaldi\Application\vivaldi.exe"],
            "firefox" => &[r"Mozilla Firefox\firefox.exe"],
            "opera" => &[r"Programs\Opera\opera.exe"],
            "operagx" => &[r"Programs\Opera GX\opera.exe"],
            _ => &[],
        };
        for base in [pf, pf86, local].into_iter().flatten() {
            for r in rels {
                tries.push(base.join(r));
            }
        }
        tries.into_iter().find(|p| p.is_file())
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn find(_id: &str, _exe: &str, _rel: &str, _roaming: bool) -> Option<std::path::PathBuf> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_and_addresses() {
        assert_eq!(split_title("Watch Frieren: Beyond Journey's End - Episode 5 | Crunchyroll"), ("Frieren: Beyond Journey's End".into(), "Episode 5".into()));
        assert_eq!(split_title("The Bear S3:E2 Next | Disney+"), ("The Bear".into(), "S3:E2 Next".into()));
        assert_eq!(split_title("Watch The Last of Us Season 1 | Max"), ("The Last of Us".into(), "Season 1".into()));
        assert_eq!(split_title("Netflix"), (String::new(), String::new()));
        assert_eq!(split_title("How I built a PC - YouTube"), ("How I built a PC".into(), String::new()));
        assert!(rule_for("https://www.netflix.com/watch/81234567?trackId=1").is_some_and(|r| r.0 == "netflix"));
        assert!(rule_for("https://www.crunchyroll.com/fr/watch/GX1/ep").is_some_and(|r| r.0 == "crunchyroll"));
        assert!(rule_for("https://www.netflix.com/browse").is_none());
        assert!(rule_for("https://evil.example/https://www.netflix.com/watch/1").is_none());
        assert_eq!(youtube_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=10"), Some("dQw4w9WgXcQ"));
        assert_eq!(to_unix_ms("chromium", 13_300_000_000_000_000), 13_300_000_000_000 - 11_644_473_600_000);
        let (sql, pats) = query("firefox");
        assert!(sql.contains("moz_places") && sql.contains(&format!("?{}", pats.len())) && !pats.is_empty());
    }

    #[test]
    fn one_entry_per_series() {
        let now = 1_800_000_000_000;
        let rows: Vec<Row> = vec![
            ("https://www.crunchyroll.com/watch/A/e6".into(), "Watch Frieren - Episode 6 | Crunchyroll".into(), now - 1000),
            ("https://www.crunchyroll.com/watch/A/e5".into(), "Watch Frieren - Episode 5 | Crunchyroll".into(), now - 90_000),
            ("https://www.netflix.com/watch/1".into(), "Netflix".into(), now - 5000),
            ("https://www.youtube.com/watch?v=dQw4w9WgXcQ".into(), "A video - YouTube".into(), now - 7000),
            ("https://www.disneyplus.com/video/x".into(), "Old show | Disney+".into(), now - 90 * 86_400_000),
        ];
        let w = summarize(&rows, now, 10);
        assert_eq!(w.iter().map(|x| (x.site.as_str(), x.show.as_str(), x.episode.as_str())).collect::<Vec<_>>(), vec![("crunchyroll", "Frieren", "Episode 6"), ("netflix", "", ""), ("youtube", "A video", "")]);
        assert_eq!(w[0].url, "https://www.crunchyroll.com/watch/A/e6");
        assert!(w[2].thumb.as_deref().is_some_and(|t| t.contains("dQw4w9WgXcQ")));
    }

    #[test]
    fn reads_a_chromium_history() {
        let t = tempfile::tempdir().unwrap();
        let f = t.path().join("History");
        let c = rusqlite::Connection::open(&f).unwrap();
        c.execute_batch("CREATE TABLE urls (id INTEGER PRIMARY KEY, url TEXT, title TEXT, last_visit_time INTEGER);").unwrap();
        c.execute("INSERT INTO urls (url, title, last_visit_time) VALUES ('https://www.netflix.com/watch/9', 'Netflix', 13300000000000000), ('https://bank.example/account', 'My bank', 13300000000000001)", []).unwrap();
        drop(c);
        let rows = read_history(&f, "chromium");
        assert_eq!(rows.len(), 1, "only the watch pages are read");
        assert_eq!(rows[0].0, "https://www.netflix.com/watch/9");
        let prof = t.path().join("Local").join(r"Google\Chrome\User Data").join("Default");
        std::fs::create_dir_all(&prof).unwrap();
        std::fs::copy(&f, prof.join("History")).unwrap();
        assert_eq!(history_files("chrome", Some(&t.path().join("Local")), None).len(), 1);
    }
}
