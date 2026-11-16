//! Text shortcuts: saved text (an address, a reply, a code) that OmniHub
//! types into the app in front when you press its key (or copies, from the
//! list). Keys go through the same hook as the counters, which only
//! sees the keys asked for; the key itself still reaches the app, so spare
//! keys (F13–F24, Pause, Scroll Lock, the numpad) work best.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

pub const MAX_TEXT: usize = 2000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub text: String,
    /// A Windows virtual-key code, or 0 for none.
    pub key: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SnippetSettings {
    pub snippets: Vec<Snippet>,
    pub keys_on: bool,
}

/// Keys a shortcut can't use: typing goes through them (Enter, packets) or
/// they'd fire while you type (letters, digits, space, modifiers).
pub fn key_allowed(vk: u32) -> bool {
    vk != 0 && vk <= 0xFE && !matches!(vk, 0x08 | 0x09 | 0x0D | 0x10..=0x12 | 0x20 | 0x30..=0x39 | 0x41..=0x5A | 0xA0..=0xA5 | 0xE7)
}

pub struct Snippets {
    file: PathBuf,
    settings: Mutex<SnippetSettings>,
}

impl Snippets {
    pub fn new(data: &Path) -> Snippets {
        let file = data.join("snippets.json");
        let settings = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(SnippetSettings { keys_on: true, ..Default::default() });
        Snippets { file, settings: Mutex::new(settings) }
    }

    pub fn get(&self) -> SnippetSettings {
        self.settings.lock().clone()
    }

    pub fn save(&self, mut s: SnippetSettings) -> Result<SnippetSettings, String> {
        if s.snippets.len() > 100 {
            return Err("Keep it to 100 shortcuts".into());
        }
        for x in s.snippets.iter_mut() {
            if x.text.is_empty() {
                return Err("A shortcut needs some text".into());
            }
            if x.text.chars().count() > MAX_TEXT {
                return Err(format!("Keep each text under {MAX_TEXT} characters"));
            }
            x.text = x.text.replace("\r\n", "\n");
            x.name = x.name.trim().chars().take(40).collect();
            if x.name.is_empty() {
                x.name = x.text.lines().next().unwrap_or("").trim().chars().take(24).collect();
            }
            if x.id.is_empty() {
                x.id = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
            }
            if !key_allowed(x.key) {
                x.key = 0;
            }
        }
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        *self.settings.lock() = s.clone();
        Ok(s)
    }

    pub fn watched_keys(&self) -> Vec<u32> {
        let s = self.settings.lock();
        if !s.keys_on {
            return Vec::new();
        }
        s.snippets.iter().filter(|x| x.key != 0).map(|x| x.key).collect()
    }

    /// The text a key types (the first shortcut on it).
    pub fn for_key(&self, vk: u32) -> Option<String> {
        let s = self.settings.lock();
        if !s.keys_on {
            return None;
        }
        s.snippets.iter().find(|x| x.key == vk).map(|x| x.text.clone())
    }

    pub fn find(&self, id: &str) -> Option<Snippet> {
        self.settings.lock().snippets.iter().find(|x| x.id == id).cloned()
    }
}

/// Type text into the app in front, a moment later (so the key that asked
/// for it is let go first and focus is back on that app).
pub fn type_soon(text: String, delay_ms: u64) {
    let _ = std::thread::Builder::new().name("type-text".into()).spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        crate::capture::stream::input::text(&text);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_shortcuts() {
        let t = tempfile::tempdir().unwrap();
        let b = Snippets::new(t.path());
        assert!(b.get().keys_on);
        assert!(b.save(SnippetSettings { snippets: vec![Snippet::default()], ..Default::default() }).is_err());
        let s = b
            .save(SnippetSettings {
                snippets: vec![Snippet { text: "Good game!\r\nSee you".into(), key: 0x7C, ..Default::default() }, Snippet { text: "x".into(), key: 0x41, ..Default::default() }],
                keys_on: true,
            })
            .unwrap();
        assert_eq!(s.snippets[0].name, "Good game!");
        assert_eq!(s.snippets[0].text, "Good game!\nSee you");
        // Letters would fire while typing, so the key is dropped.
        assert_eq!(s.snippets[1].key, 0);
        assert_eq!(b.watched_keys(), vec![0x7C]);
        assert_eq!(b.for_key(0x7C).as_deref(), Some("Good game!\nSee you"));
        assert_eq!(Snippets::new(t.path()).get().snippets.len(), 2);
        assert!(key_allowed(0x13) && key_allowed(0x91) && key_allowed(0x87) && !key_allowed(0x0D));
    }
}
