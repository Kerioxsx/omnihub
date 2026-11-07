//! Karaoke Studio's own data (kept apart from Aurora's): your first language
//! and accent, your practice history (summaries only, never recordings), and
//! songs from this PC with their lyrics. Lyrics come from the same places as
//! the music player's (an .lrc file next to the song or in your lyrics
//! folder, then LRCLIB, then NetEase), or an .lrc file you choose.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::media::lyrics::{self, Lyrics};

const MAX_SESSIONS: usize = 300;
pub const AUDIO: &[&str] = &["mp3", "m4a", "aac", "wav", "flac", "ogg", "opus"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    /// "es-MX", "en-GB", "pt-BR", "ru", "hi-IN"…
    pub lang: String,
}

/// One practice, as numbers only (and only what was really measured).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Session {
    pub at: i64,
    pub title: String,
    pub artist: String,
    /// "song" (follow the song) or "voice" (follow my voice).
    pub mode: String,
    pub seconds: u32,
    pub lines: u32,
    /// Average distance from each held note's own centre, in cents (lower is steadier).
    pub steadiness_cents: Option<f32>,
    /// Average entrance against the lyrics' timing, ms (negative = early).
    pub timing_ms: Option<f32>,
    /// Lowest and highest comfortable notes (MIDI numbers).
    pub low_note: Option<f32>,
    pub high_note: Option<f32>,
    /// Lines to try again (their text).
    pub retry: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct KaraokeData {
    pub profile: Option<Profile>,
    pub sessions: Vec<Session>,
}

/// A song from this PC, ready to play here.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub path: String,
    pub title: String,
    pub artist: String,
}

pub struct Karaoke {
    file: PathBuf,
    data: Mutex<KaraokeData>,
}

/// "Artist - Title.mp3" → ("Title", "Artist"); "Title.mp3" → ("Title", "").
pub fn guess_names(path: &Path) -> (String, String) {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let stem = stem.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ' || c == '-').trim().to_string();
    match stem.split_once(" - ") {
        Some((a, t)) => (t.trim().to_string(), a.trim().to_string()),
        None => (stem, String::new()),
    }
}

impl Karaoke {
    pub fn new(data: &Path) -> Karaoke {
        let file = data.join("karaoke.json");
        let data = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Karaoke { file, data: Mutex::new(data) }
    }

    pub fn get(&self) -> KaraokeData {
        self.data.lock().clone()
    }

    fn write(&self, d: &KaraokeData) -> Result<(), String> {
        crate::settings::write_atomic(&self.file, &serde_json::to_vec(d).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    pub fn set_profile(&self, lang: &str) -> Result<KaraokeData, String> {
        let lang = lang.trim();
        if lang.is_empty() || lang.len() > 20 || !lang.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err("Pick a language".into());
        }
        let mut d = self.data.lock();
        d.profile = Some(Profile { lang: lang.to_string() });
        let snap = d.clone();
        drop(d);
        self.write(&snap)?;
        Ok(snap)
    }

    pub fn add_session(&self, mut s: Session) -> Result<KaraokeData, String> {
        s.title = s.title.chars().take(200).collect();
        s.artist = s.artist.chars().take(200).collect();
        s.retry = s.retry.into_iter().take(5).map(|l| l.chars().take(200).collect()).collect();
        let mut d = self.data.lock();
        d.sessions.insert(0, s);
        d.sessions.truncate(MAX_SESSIONS);
        let snap = d.clone();
        drop(d);
        self.write(&snap)?;
        Ok(snap)
    }

    /// Delete the practice history (the language stays).
    pub fn clear(&self) -> Result<KaraokeData, String> {
        let mut d = self.data.lock();
        d.sessions.clear();
        let snap = d.clone();
        drop(d);
        self.write(&snap)?;
        Ok(snap)
    }
}

/// A song file from this PC (checked: an audio file that exists).
pub fn track(path: &Path) -> Result<Track, String> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !AUDIO.contains(&ext.as_str()) {
        return Err("Pick a song file (MP3, M4A, WAV, FLAC or OGG)".into());
    }
    if !path.is_file() {
        return Err("That file isn't there any more".into());
    }
    let (title, artist) = guess_names(path);
    Ok(Track { path: path.to_string_lossy().to_string(), title, artist })
}

/// Lyrics for a song: an .lrc you chose, else one beside the song, else your
/// lyrics folder, else LRCLIB, else NetEase.
pub fn lyrics_for(song: Option<&Path>, lrc: Option<&Path>, lyrics_dir: Option<&Path>, title: &str, artist: &str, duration_ms: u64) -> Result<Option<Lyrics>, String> {
    let from_file = |p: &Path| -> Option<Lyrics> {
        let text = std::fs::read_to_string(p).ok()?;
        let lines = lyrics::parse_lrc(&text);
        (!lines.is_empty()).then(|| Lyrics { lines, plain: None, instrumental: false, source: "Your file".into() })
    };
    if let Some(p) = lrc {
        return from_file(p).map(Some).ok_or_else(|| "That file has no timed lyrics (an .lrc file has lines like [01:23.45] …)".into());
    }
    if let Some(found) = song.map(|s| s.with_extension("lrc")).filter(|p| p.is_file()).and_then(|p| from_file(&p)) {
        return Ok(Some(found));
    }
    if let Some(found) = lyrics_dir.and_then(|d| lyrics::find_local(d, title, artist, duration_ms)) {
        return Ok(Some(found));
    }
    if title.trim().is_empty() {
        return Ok(None);
    }
    match lyrics::lookup(title, artist, "", duration_ms, &lyrics::http_get) {
        Ok(Some(l)) if !l.lines.is_empty() => Ok(Some(l)),
        other => {
            let net = lyrics::netease(title, artist, duration_ms, &lyrics::http_get).ok().flatten().filter(|l| !l.lines.is_empty());
            match (net, other) {
                (Some(n), _) => Ok(Some(n)),
                (None, Ok(l)) => Ok(l),
                (None, Err(e)) => Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_profile_and_history() {
        let t = tempfile::tempdir().unwrap();
        let k = Karaoke::new(t.path());
        assert!(k.get().profile.is_none());
        assert!(k.set_profile("es MX").is_err());
        k.set_profile("es-MX").unwrap();
        k.add_session(Session { title: "Song".into(), mode: "voice".into(), seconds: 90, timing_ms: Some(-40.0), ..Default::default() }).unwrap();
        let again = Karaoke::new(t.path()).get();
        assert_eq!(again.profile.unwrap().lang, "es-MX");
        assert_eq!(again.sessions.len(), 1);
        assert!(k.clear().unwrap().sessions.is_empty());
        assert_eq!(Karaoke::new(t.path()).get().profile.unwrap().lang, "es-MX", "the language stays");
    }

    #[test]
    fn reads_song_files_and_their_lyrics() {
        let t = tempfile::tempdir().unwrap();
        let song = t.path().join("01 - Nochnoy Gorod - Tikhiy vecher.mp3");
        std::fs::write(&song, b"ID3").unwrap();
        assert_eq!(guess_names(&song), ("Tikhiy vecher".to_string(), "Nochnoy Gorod".to_string()));
        assert!(track(&t.path().join("x.txt")).is_err());
        let tr = track(&song).unwrap();
        assert_eq!(tr.artist, "Nochnoy Gorod");
        std::fs::write(song.with_extension("lrc"), "[00:12.00]Тихий вечер над рекой\n[00:15.50]Звёзды падают в ладони").unwrap();
        let l = lyrics_for(Some(&song), None, None, "Tikhiy vecher", "Nochnoy Gorod", 0).unwrap().unwrap();
        assert_eq!(l.lines.len(), 2);
        assert_eq!(l.lines[1].ms, 15_500);
        let bad = t.path().join("bad.lrc");
        std::fs::write(&bad, "no timings").unwrap();
        assert!(lyrics_for(Some(&song), Some(&bad), None, "", "", 0).is_err());
    }
}
