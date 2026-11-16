//! Quick sites: Netflix, YouTube, Crunchyroll, Disney+, HBO Max and the
//! rest, a tap away in Big Picture and on the phone. "Full screen" opens the
//! site in its own Microsoft Edge window with no browser bars (Edge comes
//! with Windows and plays these services' videos in full quality); otherwise
//! it opens in your usual browser. The list is yours to change.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Site {
    pub id: String,
    pub name: String,
    pub url: String,
    /// Tile colour ("#e50914").
    pub color: String,
}

fn site(id: &str, name: &str, url: &str, color: &str) -> Site {
    Site { id: id.into(), name: name.into(), url: url.into(), color: color.into() }
}

pub fn defaults() -> Vec<Site> {
    vec![
        site("netflix", "Netflix", "https://www.netflix.com/browse", "#e50914"),
        site("youtube", "YouTube", "https://www.youtube.com", "#ff0033"),
        site("crunchyroll", "Crunchyroll", "https://www.crunchyroll.com", "#f47521"),
        site("disney", "Disney+", "https://www.disneyplus.com", "#113ccf"),
        site("hbomax", "HBO Max", "https://play.hbomax.com", "#5b2ee5"),
        site("prime", "Prime Video", "https://www.primevideo.com", "#1399ff"),
        site("appletv", "Apple TV", "https://tv.apple.com", "#555555"),
        site("paramount", "Paramount+", "https://www.paramountplus.com", "#0064ff"),
        site("twitch", "Twitch", "https://www.twitch.tv", "#9146ff"),
        site("ytmusic", "YouTube Music", "https://music.youtube.com", "#ff0000"),
        site("spotify", "Spotify", "https://open.spotify.com", "#1db954"),
    ]
}

/// Only plain web addresses, with no spaces.
pub fn valid_url(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://")) && url.len() <= 2048 && !url.chars().any(|c| c.is_whitespace() || c == '"')
}

pub struct Sites {
    file: PathBuf,
    list: Mutex<Vec<Site>>,
}

impl Sites {
    pub fn new(data: &Path) -> Sites {
        let file = data.join("sites.json");
        let list = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_else(defaults);
        Sites { file, list: Mutex::new(list) }
    }

    pub fn list(&self) -> Vec<Site> {
        self.list.lock().clone()
    }

    pub fn save(&self, list: Vec<Site>) -> Result<Vec<Site>, String> {
        if list.len() > 60 {
            return Err("That's a lot of sites; keep it under 60".into());
        }
        let mut clean = Vec::new();
        for mut s in list {
            s.name = s.name.trim().chars().take(40).collect();
            s.url = s.url.trim().to_string();
            if s.name.is_empty() || !valid_url(&s.url) {
                return Err(format!("“{}” needs a name and a web address (https://…)", s.name));
            }
            if !(s.color.len() == 7 && s.color.starts_with('#') && s.color[1..].chars().all(|c| c.is_ascii_hexdigit())) {
                s.color = "#6366f1".into();
            }
            if s.id.is_empty() {
                s.id = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
            }
            clean.push(s);
        }
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&clean).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        *self.list.lock() = clean.clone();
        Ok(clean)
    }

    pub fn reset(&self) -> Result<Vec<Site>, String> {
        self.save(defaults())
    }
}

/// Open a site: full screen in its own Edge window, or in the usual browser.
pub fn open(url: &str, full_screen: bool) -> Result<(), String> {
    if !valid_url(url) {
        return Err("That isn't a web address".into());
    }
    if full_screen {
        if let Some(edge) = imp::edge() {
            return std::process::Command::new(edge).arg(format!("--app={url}")).arg("--start-fullscreen").spawn().map(|_| ()).map_err(|e| format!("Edge didn't start: {e}"));
        }
    }
    crate::system::shell::open_url(url).map_err(|e| e.to_string())
}

#[cfg(windows)]
mod imp {
    use std::path::PathBuf;

    /// Where Microsoft Edge is (Windows' list of app paths, else its usual folder).
    pub fn edge() -> Option<PathBuf> {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        use winreg::RegKey;
        const KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\msedge.exe";
        for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
            if let Ok(k) = RegKey::predef(root).open_subkey(KEY) {
                if let Ok(p) = k.get_value::<String, _>("") {
                    let p = PathBuf::from(p.trim_matches('"'));
                    if p.is_file() {
                        return Some(p);
                    }
                }
            }
        }
        ["ProgramFiles(x86)", "ProgramFiles"].iter().filter_map(std::env::var_os).map(|d| PathBuf::from(d).join(r"Microsoft\Edge\Application\msedge.exe")).find(|p| p.is_file())
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn edge() -> Option<std::path::PathBuf> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_a_clean_list() {
        let t = tempfile::tempdir().unwrap();
        let s = Sites::new(t.path());
        assert!(s.list().iter().any(|x| x.name == "Crunchyroll"));
        let mut l = s.list();
        l.push(Site { id: String::new(), name: " Plex ".into(), url: "https://app.plex.tv".into(), color: "nope".into() });
        let saved = s.save(l).unwrap();
        let plex = saved.last().unwrap();
        assert_eq!((plex.name.as_str(), plex.color.as_str()), ("Plex", "#6366f1"));
        assert!(!plex.id.is_empty());
        assert_eq!(Sites::new(t.path()).list().len(), saved.len());
        assert!(s.save(vec![Site { id: "x".into(), name: "Bad".into(), url: "file:///c:/x".into(), color: "#000000".into() }]).is_err());
        assert!(!valid_url("https://a b"));
        assert!(open("javascript:alert(1)", false).is_err());
    }
}
