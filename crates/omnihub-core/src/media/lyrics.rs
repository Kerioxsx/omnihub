//! Time-synced lyrics for the track that is playing, from LRCLIB
//! (lrclib.net — a free, open lyrics database; no account or key).
//!
//! LRC files time each line (`[01:02.50] words`); some also time each word
//! (`<01:02.80> word`), which the phone uses to fill words as they are sung.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Word {
    pub ms: u64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    /// When the line starts (track time, milliseconds).
    pub ms: u64,
    pub text: String,
    /// Word timings, when the file has them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<Word>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    /// Time-synced lines (empty when only plain text is known).
    pub lines: Vec<Line>,
    pub plain: Option<String>,
    pub instrumental: bool,
    pub source: String,
}

fn parse_time(t: &str) -> Option<u64> {
    // mm:ss(.xx|.xxx) — minutes may exceed 59.
    let (m, rest) = t.split_once(':')?;
    let (s, frac) = rest.split_once(['.', ':']).unwrap_or((rest, "0"));
    let (m, s): (u64, u64) = (m.trim().parse().ok()?, s.trim().parse().ok()?);
    if s >= 60 {
        return None;
    }
    let frac = frac.trim();
    let ms = match frac.len() {
        0 => 0,
        1 => frac.parse::<u64>().ok()? * 100,
        2 => frac.parse::<u64>().ok()? * 10,
        _ => frac[..3].parse::<u64>().ok()?,
    };
    Some(m * 60_000 + s * 1000 + ms)
}

/// Parse an LRC document (line and optional word timings, `[offset:]`).
pub fn parse_lrc(src: &str) -> Vec<Line> {
    let mut offset: i64 = 0;
    let mut out = Vec::new();
    for raw in src.lines() {
        let mut rest = raw.trim();
        let mut stamps = Vec::new();
        while let Some(inner) = rest.strip_prefix('[') {
            let Some(end) = inner.find(']') else { break };
            let tag = &inner[..end];
            if let Some(v) = tag.strip_prefix("offset:") {
                offset = v.trim().parse().unwrap_or(0);
            } else if let Some(t) = parse_time(tag) {
                stamps.push(t);
            }
            rest = inner[end + 1..].trim_start();
        }
        if stamps.is_empty() {
            continue;
        }
        // Word timings: "<00:12.34> word <00:12.80> word".
        let mut words = Vec::new();
        let mut text = String::new();
        if rest.contains('<') {
            let mut s = rest;
            while let Some(i) = s.find('<') {
                text.push_str(&s[..i]);
                let Some(j) = s[i..].find('>') else { break };
                let t = parse_time(&s[i + 1..i + j]);
                let after = &s[i + j + 1..];
                let next = after.find('<').unwrap_or(after.len());
                let w = &after[..next];
                if let Some(t) = t {
                    if !w.trim().is_empty() {
                        words.push(Word { ms: t, text: w.to_string() });
                    }
                }
                text.push_str(w);
                s = &after[next..];
            }
            text.push_str(s);
        } else {
            text = rest.to_string();
        }
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        for t in stamps {
            out.push(Line { ms: t, text: text.clone(), words: words.clone() });
        }
    }
    // A positive offset shows lyrics earlier.
    for l in &mut out {
        l.ms = (l.ms as i64 - offset).max(0) as u64;
        for w in &mut l.words {
            w.ms = (w.ms as i64 - offset).max(0) as u64;
        }
    }
    out.sort_by_key(|l| l.ms);
    out
}

/// The line playing at `ms` (index into `lines`).
pub fn line_at(lines: &[Line], ms: u64) -> Option<usize> {
    let i = lines.partition_point(|l| l.ms <= ms);
    i.checked_sub(1)
}

/// Title and artist as a lyrics database knows them: drop "(Remastered
/// 2011)", "[Official Video]", "feat." parts, and split "Artist - Title"
/// titles from video sites whose "artist" is a channel name.
pub fn clean_query(title: &str, artist: &str) -> (String, String) {
    let mut artist = artist.trim().to_string();
    let mut title = title.trim().to_string();
    for suffix in [" - Topic", "VEVO", "Official"] {
        if let Some(a) = artist.strip_suffix(suffix) {
            artist = a.trim().to_string();
        }
    }
    if let Some((a, t)) = title.split_once(" - ") {
        // "Artist - Title" (a video title), unless the artist field already matches.
        // Compare letters and digits only ("DaftPunk" is "Daft Punk").
        let norm = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect::<String>();
        let (na, nartist) = (norm(a), norm(&artist));
        let channelish = nartist.is_empty() || na.contains(&nartist) || nartist.contains(&na);
        if channelish {
            artist = a.trim().to_string();
            title = t.trim().to_string();
        }
    }
    let strip = |s: &str| -> String {
        let mut out = String::new();
        let mut depth = 0;
        for c in s.chars() {
            match c {
                '(' | '[' => depth += 1,
                ')' | ']' if depth > 0 => depth -= 1,
                _ if depth == 0 => out.push(c),
                _ => {}
            }
        }
        out
    };
    let mut title = strip(&title);
    for cut in [" - Remaster", " - Live", " - Radio Edit", " feat.", " ft.", " Feat."] {
        if let Some(i) = title.find(cut) {
            title.truncate(i);
        }
    }
    let artist = artist.split([',', '&']).next().unwrap_or("").split(" feat").next().unwrap_or("").trim().to_string();
    (title.split_whitespace().collect::<Vec<_>>().join(" "), artist)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LrclibTrack {
    #[serde(default)]
    duration: f64,
    #[serde(default)]
    instrumental: bool,
    plain_lyrics: Option<String>,
    synced_lyrics: Option<String>,
}

fn to_lyrics(t: LrclibTrack) -> Lyrics {
    Lyrics { lines: t.synced_lyrics.as_deref().map(parse_lrc).unwrap_or_default(), plain: t.plain_lyrics.filter(|p| !p.trim().is_empty()), instrumental: t.instrumental, source: "LRCLIB".into() }
}

/// Look a track up: the exact match first, then a search (closest length).
/// `get(url)` returns the body, or `Ok(None)` for "not found".
pub fn lookup(title: &str, artist: &str, album: &str, duration_ms: u64, get: &dyn Fn(&str) -> Result<Option<String>, String>) -> Result<Option<Lyrics>, String> {
    let (t, a) = clean_query(title, artist);
    if t.is_empty() {
        return Ok(None);
    }
    let enc = |s: &str| url_encode(s);
    let secs = duration_ms / 1000;
    let mut url = format!("https://lrclib.net/api/get?track_name={}&artist_name={}", enc(&t), enc(&a));
    if !album.is_empty() {
        url.push_str(&format!("&album_name={}", enc(album)));
    }
    if secs > 0 {
        url.push_str(&format!("&duration={secs}"));
    }
    if let Some(body) = get(&url)? {
        if let Ok(track) = serde_json::from_str::<LrclibTrack>(&body) {
            return Ok(Some(to_lyrics(track)));
        }
    }
    let Some(body) = get(&format!("https://lrclib.net/api/search?track_name={}&artist_name={}", enc(&t), enc(&a)))? else { return Ok(None) };
    let mut found: Vec<LrclibTrack> = serde_json::from_str(&body).unwrap_or_default();
    // Prefer synced lyrics, then the closest length.
    found.sort_by_key(|f| (f.synced_lyrics.is_none(), ((f.duration - secs as f64).abs() * 10.0) as i64));
    Ok(found.into_iter().find(|f| secs == 0 || (f.duration - secs as f64).abs() < 8.0).map(to_lyrics))
}

fn url_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// HTTP GET for [`lookup`] (404 → `Ok(None)`).
pub fn http_get(url: &str) -> Result<Option<String>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(12))).http_status_as_error(false).build().into();
    let mut res = agent
        .get(url)
        .header("User-Agent", concat!("OmniHub/", env!("CARGO_PKG_VERSION"), " (https://github.com/Kerioxsx/omnihub)"))
        .call()
        .map_err(|e| format!("could not reach the lyrics service: {e}"))?;
    match res.status().as_u16() {
        200 => res.body_mut().read_to_string().map(Some).map_err(|e| e.to_string()),
        404 => Ok(None),
        code => Err(format!("the lyrics service answered {code}")),
    }
}

/// Lyrics kept on disk, so a song is looked up once.
pub struct LyricsCache {
    dir: PathBuf,
}

impl LyricsCache {
    pub fn new(dir: PathBuf) -> Self {
        LyricsCache { dir }
    }

    fn file(&self, key: &str) -> PathBuf {
        use sha2::Digest;
        self.dir.join(format!("{}.json", hex::encode(&sha2::Sha256::digest(key.as_bytes())[..12])))
    }

    /// `Some(None)` = known to have no lyrics.
    pub fn get(&self, key: &str) -> Option<Option<Lyrics>> {
        serde_json::from_slice(&std::fs::read(self.file(key)).ok()?).ok()
    }

    pub fn put(&self, key: &str, lyrics: &Option<Lyrics>) {
        let _ = std::fs::create_dir_all(&self.dir);
        if let Ok(bytes) = serde_json::to_vec(lyrics) {
            let _ = crate::settings::write_atomic(&self.file(key), &bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lrc_lines_words_and_offset() {
        let src = "[ar:Someone]\n[offset:+500]\n[00:12.34]First line\n[00:15.00][01:15.5]Chorus\n\n[02:00.123] <02:00.123>Word <02:00.900>by <02:01.400>word\nnot a lyric";
        let l = parse_lrc(src);
        assert_eq!(l.iter().map(|x| (x.ms, x.text.as_str())).collect::<Vec<_>>(), [(11_840, "First line"), (14_500, "Chorus"), (75_000, "Chorus"), (119_623, "Word by word")]);
        assert_eq!(l[3].words.iter().map(|w| (w.ms, w.text.trim())).collect::<Vec<_>>(), [(119_623, "Word"), (120_400, "by"), (120_900, "word")]);
        assert_eq!(line_at(&l, 0), None);
        assert_eq!(line_at(&l, 14_499), Some(0));
        assert_eq!(line_at(&l, 14_500), Some(1));
        assert_eq!(line_at(&l, 10_000_000), Some(3));
        assert!(parse_time("1:75").is_none());
        assert_eq!(parse_time("3:04.5"), Some(184_500));
    }

    #[test]
    fn queries_are_cleaned() {
        assert_eq!(clean_query("Bohemian Rhapsody - Remastered 2011", "Queen"), ("Bohemian Rhapsody".into(), "Queen".into()));
        assert_eq!(clean_query("Daft Punk - Get Lucky (Official Video) [HD]", "DaftPunkVEVO"), ("Get Lucky".into(), "Daft Punk".into()));
        assert_eq!(clean_query("Blinding Lights", "The Weeknd, Rosalía"), ("Blinding Lights".into(), "The Weeknd".into()));
        assert_eq!(clean_query("Song (feat. X)", "A & B"), ("Song".into(), "A".into()));
        assert_eq!(url_encode("a b&c/é"), "a%20b%26c%2F%C3%A9");
    }

    #[test]
    fn lookup_prefers_exact_then_closest_synced() {
        let calls = std::cell::RefCell::new(Vec::new());
        let get = |url: &str| -> Result<Option<String>, String> {
            calls.borrow_mut().push(url.to_string());
            if url.contains("/api/get") {
                return Ok(None);
            }
            Ok(Some(r#"[{"duration":100,"plainLyrics":"x","syncedLyrics":null},{"duration":212,"syncedLyrics":"[00:01.00]far"},{"duration":200,"syncedLyrics":"[00:01.00]close","plainLyrics":"close"}]"#.into()))
        };
        let l = lookup("Get Lucky", "Daft Punk", "Random Access Memories", 201_000, &get).unwrap().unwrap();
        assert_eq!(l.lines[0].text, "close");
        assert!(calls.borrow()[0].contains("track_name=Get%20Lucky&artist_name=Daft%20Punk&album_name=Random%20Access%20Memories&duration=201"));
        // Nothing within a few seconds of the length: no lyrics.
        assert!(lookup("Get Lucky", "Daft Punk", "", 500_000, &get).unwrap().is_none());
    }

    #[test]
    fn cache_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let c = LyricsCache::new(dir.path().join("lyrics"));
        assert!(c.get("a").is_none());
        c.put("a", &None);
        assert_eq!(c.get("a"), Some(None));
        let l = Lyrics { lines: parse_lrc("[00:01.00]hi"), plain: None, instrumental: false, source: "LRCLIB".into() };
        c.put("b", &Some(l.clone()));
        assert_eq!(c.get("b"), Some(Some(l)));
    }
}
