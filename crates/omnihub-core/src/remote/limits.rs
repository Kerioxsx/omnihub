//! Family and guest phones: a paired phone can be limited to a few areas
//! (for example a sibling's phone that can only use the deck and the music).
//! Everything outside its areas is refused, including anything added later.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// The areas a limited phone can be given, with their names.
pub const AREAS: &[(&str, &str)] = &[
    ("deck", "Deck and scenes"),
    ("music", "Music, volume and the party queue"),
    ("files", "Files, photos and transfers"),
    ("games", "Games and counters"),
    ("notes", "Notes, ideas and tasks"),
    ("chats", "Chats"),
    ("screen", "Screen sharing and control"),
    ("power", "Power and quiet mode"),
];

/// Which area a phone request belongs to (None: needed by every phone).
pub fn area_of(path: &str) -> Option<&'static str> {
    let p = path.trim_start_matches("/api/");
    let first = p.split(['/', '?']).next().unwrap_or("");
    Some(match first {
        "info" | "me" | "ws" | "status" | "unpair" | "pair" | "approvals" | "ticket" => return None,
        "deck" | "scenes" | "capri-desktop" => "deck",
        "media" | "sound" | "party" | "displays" | "bluetooth" | "sites" => "music",
        "files" | "fs" | "upload" | "inbox" | "photos" | "downloads" | "scan" | "print" | "screenshot" | "video" => "files",
        "games" | "counters" | "servers" => "games",
        "notes" | "ideas" | "tasks" | "board" => "notes",
        "chats" | "voice" => "chats",
        "screen" | "type" | "clipboard" | "open-link" | "open-apps" | "apps" | "live" => "screen",
        "power" | "quiet" | "panic" => "power",
        // The vault and anything new: only for phones without limits.
        _ => "other",
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Limits {
    /// Device id → the areas it may use.
    pub phones: HashMap<String, Vec<String>>,
}

pub struct PhoneLimits {
    file: PathBuf,
    limits: Mutex<Limits>,
}

impl PhoneLimits {
    pub fn new(data: &Path) -> PhoneLimits {
        let file = data.join("phone-limits.json");
        let limits = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        PhoneLimits { file, limits: Mutex::new(limits) }
    }

    pub fn get(&self) -> Limits {
        self.limits.lock().clone()
    }

    /// Limit a phone to these areas (None: no limits, the whole app).
    pub fn set(&self, device: &str, areas: Option<Vec<String>>) -> Result<Limits, String> {
        let mut l = self.limits.lock();
        match areas {
            Some(a) => {
                let a: Vec<String> = a.into_iter().filter(|x| AREAS.iter().any(|k| k.0 == x)).collect();
                l.phones.insert(device.to_string(), a);
            }
            None => {
                l.phones.remove(device);
            }
        }
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&*l).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        Ok(l.clone())
    }

    /// Whether this phone may make this request.
    pub fn allows(&self, device: &str, path: &str) -> bool {
        let l = self.limits.lock();
        let Some(areas) = l.phones.get(device) else { return true };
        match area_of(path) {
            None => true,
            Some(a) => areas.iter().any(|x| x == a),
        }
    }

    /// Whether a limited phone hears about this event.
    pub fn allows_event(&self, device: &str, topic: &str) -> bool {
        let area = match topic.split(':').next().unwrap_or("") {
            "media" | "party" => "music",
            "notes" | "board" => "notes",
            "chats" => "chats",
            "games" | "counters" | "mcserver" | "capri" => "games",
            "power" | "quiet" => "power",
            "download" | "video" | "scan" | "inbox" => "files",
            _ => return true,
        };
        let l = self.limits.lock();
        l.phones.get(device).is_none_or(|a| a.iter().any(|x| x == area))
    }

    /// The areas a phone is limited to (None: all).
    pub fn areas(&self, device: &str) -> Option<Vec<String>> {
        self.limits.lock().phones.get(device).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limited_phones_only_reach_their_areas() {
        let t = tempfile::tempdir().unwrap();
        let l = PhoneLimits::new(t.path());
        assert!(l.allows("kid", "/api/power/shutdown"), "no limits: everything");
        l.set("kid", Some(vec!["deck".into(), "music".into(), "nonsense".into()])).unwrap();
        assert!(l.allows("kid", "/api/deck/d1"));
        assert!(l.allows("kid", "/api/media/state"));
        assert!(l.allows("kid", "/api/info"), "basics always work");
        assert!(!l.allows("kid", "/api/power/shutdown"));
        assert!(!l.allows("kid", "/api/vault/items"), "the vault never");
        assert!(!l.allows("kid", "/api/something-new"), "new areas aren't open by default");
        assert_eq!(PhoneLimits::new(t.path()).areas("kid"), Some(vec!["deck".to_string(), "music".to_string()]));
        assert!(l.allows_event("kid", "media:state") && !l.allows_event("kid", "notes:changed") && l.allows_event("kid", "alert:hot"));
        l.set("kid", None).unwrap();
        assert!(l.allows("kid", "/api/power/shutdown"));
    }
}
