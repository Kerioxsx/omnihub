//! What's playing and its synced lyrics, for games on this PC (Vesper
//! Client draws them in the Minecraft world). Read-only JSON at
//! `http://127.0.0.1:47830/vesper/lyrics`, listening on this PC only.
//! The lines are sent only when the caller doesn't have this song yet
//! (`?have=<key>`), so polling twice a second costs next to nothing.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;

use crate::core::AppCore;
use crate::media::{LyricsStatus, MediaState};

pub const PORT: u16 = 47830;

pub struct LyricsServer {
    stop: Arc<AtomicBool>,
}

impl Drop for LyricsServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

pub fn start(core: Weak<AppCore>) -> Result<LyricsServer, String> {
    let listener = TcpListener::bind(("127.0.0.1", PORT)).map_err(|e| format!("Port {PORT} is taken ({e})"))?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    std::thread::Builder::new()
        .name("game-lyrics".into())
        .spawn(move || {
            while !flag.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let core = core.clone();
                        std::thread::spawn(move || {
                            let _ = handle(stream, &core);
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(100)),
                    Err(_) => std::thread::sleep(Duration::from_millis(200)),
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(LyricsServer { stop })
}

/// `have` from `GET /vesper/lyrics?have=<key>` (None for another path).
pub fn parse_request(line: &str) -> Option<String> {
    let path = line.split_whitespace().nth(1)?;
    let (p, q) = path.split_once('?').unwrap_or((path, ""));
    if p != "/vesper/lyrics" {
        return None;
    }
    Some(q.split('&').find_map(|kv| kv.strip_prefix("have=")).unwrap_or_default().to_string())
}

/// The answer: the song, where it is now, and its lines when `have` differs.
pub fn body(state: Option<&MediaState>, lyrics: &LyricsStatus, have: &str, now_ms: i64) -> serde_json::Value {
    let Some(s) = state else { return serde_json::json!({ "key": null }) };
    // Where the song is right now (the player reports it every so often).
    let pos = if s.playing { s.position_ms as f64 + (now_ms - s.position_at).max(0) as f64 * s.rate.max(0.0) } else { s.position_ms as f64 };
    let mut out = serde_json::json!({
        "key": s.key,
        "title": s.title,
        "artist": s.artist,
        "playing": s.playing,
        "positionMs": pos.min(s.duration_ms.max(1) as f64 * 1.05) as u64,
        "durationMs": s.duration_ms,
        "rate": s.rate,
    });
    let status = match lyrics {
        LyricsStatus::Ready { lyrics } if !lyrics.lines.is_empty() => "synced",
        LyricsStatus::Ready { lyrics } if lyrics.instrumental => "instrumental",
        LyricsStatus::Ready { .. } => "unsynced",
        LyricsStatus::Searching => "searching",
        _ => "none",
    };
    out["lyrics"] = status.into();
    if have != s.key {
        if let LyricsStatus::Ready { lyrics } = lyrics {
            out["lines"] = lyrics.lines.iter().map(|l| serde_json::json!({ "ms": l.ms, "text": l.text })).collect();
        }
    }
    out
}

fn handle(stream: TcpStream, core: &Weak<AppCore>) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;
    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line)?;
    let mut s = stream;
    let Some(have) = parse_request(&line) else {
        return write!(s, "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    };
    let Some(core) = core.upgrade() else { return Ok(()) };
    let state = core.media.state();
    let lyrics = core.media.lyrics();
    let b = body(state.as_ref(), &lyrics, &have, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)).to_string();
    write!(s, "HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", b.len())?;
    s.write_all(b.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::lyrics::{Line, Lyrics};

    fn state() -> MediaState {
        serde_json::from_value(serde_json::json!({
            "key": "k1", "app": "spotify", "appName": "Spotify", "title": "Song", "artist": "Artist", "album": "",
            "durationMs": 200000, "positionMs": 10000, "positionAt": 1000, "playing": true, "rate": 1.0,
            "positionSource": "player", "canPlayPause": true, "canNext": true, "canPrevious": true, "canSeek": true, "art": null
        }))
        .unwrap()
    }

    #[test]
    fn requests_are_read() {
        assert_eq!(parse_request("GET /vesper/lyrics?have=abc HTTP/1.1"), Some("abc".into()));
        assert_eq!(parse_request("GET /vesper/lyrics HTTP/1.1"), Some(String::new()));
        assert_eq!(parse_request("GET /other HTTP/1.1"), None);
    }

    #[test]
    fn lines_only_when_the_caller_lacks_them() {
        let s = state();
        let ly = LyricsStatus::Ready { lyrics: Lyrics { lines: vec![Line { ms: 0, text: "hello".into(), words: vec![] }], plain: None, instrumental: false, source: "x".into() } };
        let first = body(Some(&s), &ly, "", 3000);
        assert_eq!(first["positionMs"], 12000);
        assert_eq!(first["lines"][0]["text"], "hello");
        assert_eq!(first["lyrics"], "synced");
        let again = body(Some(&s), &ly, "k1", 3000);
        assert!(again.get("lines").is_none());
        assert_eq!(body(None, &ly, "", 0)["key"], serde_json::Value::Null);
    }
}
