//! Soundboard: your sound files on buttons and keys (keys work inside
//! games, through the same key hook as the counters). Sounds play through a
//! small hidden OmniHub window, to the speakers or to an output you pick
//! (a virtual audio cable puts them into your microphone for Discord).

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

pub const EXTS: &[&str] = &["mp3", "wav", "ogg", "opus", "m4a", "flac"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Sound {
    pub id: String,
    pub name: String,
    pub path: String,
    /// A Windows virtual-key code, or 0 for none.
    pub key: u32,
    /// 0–100.
    pub volume: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SoundboardSettings {
    pub sounds: Vec<Sound>,
    /// The output's name as the player window sees it ("" = default).
    pub output: String,
    pub keys_on: bool,
}

pub struct Soundboard {
    file: PathBuf,
    settings: Mutex<SoundboardSettings>,
}

impl Soundboard {
    pub fn new(data: &Path) -> Soundboard {
        let file = data.join("soundboard.json");
        let settings = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(SoundboardSettings { keys_on: true, ..Default::default() });
        Soundboard { file, settings: Mutex::new(settings) }
    }

    pub fn get(&self) -> SoundboardSettings {
        self.settings.lock().clone()
    }

    pub fn save(&self, mut s: SoundboardSettings) -> Result<SoundboardSettings, String> {
        if s.sounds.len() > 60 {
            return Err("Keep it to 60 sounds".into());
        }
        for x in s.sounds.iter_mut() {
            let ext = Path::new(&x.path).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
            if !EXTS.contains(&ext.as_str()) {
                return Err(format!("{} isn't a sound file OmniHub can play", x.path));
            }
            x.name = x.name.trim().chars().take(40).collect();
            if x.name.is_empty() {
                x.name = Path::new(&x.path).file_stem().map(|n| n.to_string_lossy().chars().take(40).collect()).unwrap_or_else(|| "Sound".into());
            }
            if x.id.is_empty() {
                x.id = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
            }
            x.volume = x.volume.min(100);
            if x.key > 0xFE {
                x.key = 0;
            }
        }
        s.output = s.output.chars().take(200).collect();
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        *self.settings.lock() = s.clone();
        Ok(s)
    }

    pub fn watched_keys(&self) -> Vec<u32> {
        let s = self.settings.lock();
        if !s.keys_on {
            return Vec::new();
        }
        s.sounds.iter().filter(|x| x.key != 0).map(|x| x.key).collect()
    }

    /// The sounds a key plays.
    pub fn for_key(&self, vk: u32) -> Vec<Sound> {
        let s = self.settings.lock();
        if !s.keys_on {
            return Vec::new();
        }
        s.sounds.iter().filter(|x| x.key == vk).cloned().collect()
    }

    pub fn find(&self, id: &str) -> Option<Sound> {
        self.settings.lock().sounds.iter().find(|x| x.id == id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_sounds_and_keys() {
        let t = tempfile::tempdir().unwrap();
        let b = Soundboard::new(t.path());
        assert!(b.save(SoundboardSettings { sounds: vec![Sound { path: "C:/x.exe".into(), ..Default::default() }], ..Default::default() }).is_err());
        let s = b.save(SoundboardSettings { sounds: vec![Sound { path: "C:/Sounds/air horn.mp3".into(), key: 0x61, volume: 150, ..Default::default() }], keys_on: true, ..Default::default() }).unwrap();
        assert_eq!((s.sounds[0].name.as_str(), s.sounds[0].volume), ("air horn", 100));
        assert_eq!(b.watched_keys(), vec![0x61]);
        assert_eq!(b.for_key(0x61).len(), 1);
        assert_eq!(Soundboard::new(t.path()).get().sounds.len(), 1);
    }
}
