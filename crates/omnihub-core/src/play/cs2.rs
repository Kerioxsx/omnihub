//! Counter-Strike 2's Game State Integration: Valve's own way for a game to
//! tell an app what happens (rounds, kills, the bomb). OmniHub puts a small
//! config file in CS2's `cfg` folder naming a local address and a random
//! token; CS2 then posts its state there while you play. Nothing goes over
//! the network, and CS2 reads the file when it starts.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

/// Where CS2 posts (localhost only).
pub const PORT: u16 = 47_830;
pub const CFG_NAME: &str = "gamestate_integration_omnihub.cfg";

/// What happened between two states, for Capri.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Your kills this round so far (each new one).
    Kill { round_kills: u32 },
    RoundWon { ours: u32, theirs: u32 },
    RoundLost { ours: u32, theirs: u32 },
    Mvp,
    BombPlanted,
    MatchOver { won: bool, ours: u32, theirs: u32 },
}

/// The config file's text.
pub fn cfg_text(token: &str) -> String {
    format!(
        "\"OmniHub\"\n{{\n  \"uri\" \"http://127.0.0.1:{PORT}/cs2\"\n  \"timeout\" \"5.0\"\n  \"buffer\" \"0.1\"\n  \"throttle\" \"0.25\"\n  \"heartbeat\" \"30.0\"\n  \"auth\"\n  {{\n    \"token\" \"{token}\"\n  }}\n  \"data\"\n  {{\n    \"provider\" \"1\"\n    \"map\" \"1\"\n    \"round\" \"1\"\n    \"player_id\" \"1\"\n    \"player_state\" \"1\"\n    \"player_match_stats\" \"1\"\n  }}\n}}\n"
    )
}

/// CS2's `cfg` folder from where `cs2.exe` is (`…\game\bin\win64\cs2.exe`).
pub fn cfg_dir(exe: &Path) -> Option<PathBuf> {
    let game = exe.parent()?.parent()?.parent()?;
    let dir = game.join("csgo").join("cfg");
    dir.is_dir().then_some(dir)
}

/// Write the config (once; the token is kept so a running CS2 keeps working).
pub fn install(cfg_dir: &Path, token: &str) -> std::io::Result<bool> {
    let file = cfg_dir.join(CFG_NAME);
    let text = cfg_text(token);
    if std::fs::read_to_string(&file).is_ok_and(|t| t == text) {
        return Ok(false);
    }
    std::fs::write(file, text)?;
    Ok(true)
}

fn num(v: &Value, path: &[&str]) -> Option<u64> {
    path.iter().try_fold(v, |v, k| v.get(*k))?.as_u64()
}

fn text<'a>(v: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter().try_fold(v, |v, k| v.get(*k))?.as_str()
}

/// Is the player in the state you, not someone you're spectating?
fn mine(v: &Value) -> bool {
    match (text(v, &["provider", "steamid"]), text(v, &["player", "steamid"])) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

fn score(v: &Value) -> Option<(u32, u32)> {
    let team = text(v, &["player", "team"])?;
    let ct = num(v, &["map", "team_ct", "score"])? as u32;
    let t = num(v, &["map", "team_t", "score"])? as u32;
    Some(if team == "CT" { (ct, t) } else { (t, ct) })
}

/// The events between the previous state and this one.
pub fn diff(prev: &Value, next: &Value) -> Vec<Event> {
    let mut out = Vec::new();
    if !mine(next) {
        return out;
    }
    let same_player = mine(prev) && text(prev, &["player", "steamid"]) == text(next, &["player", "steamid"]);
    let kills = |v: &Value| num(v, &["player", "state", "round_kills"]).unwrap_or(0) as u32;
    if same_player && kills(next) > kills(prev) {
        out.push(Event::Kill { round_kills: kills(next) });
    }
    if same_player && num(next, &["player", "match_stats", "mvps"]) > num(prev, &["player", "match_stats", "mvps"]) {
        out.push(Event::Mvp);
    }
    if text(next, &["round", "bomb"]) == Some("planted") && text(prev, &["round", "bomb"]) != Some("planted") {
        out.push(Event::BombPlanted);
    }
    let over = |v: &Value| text(v, &["round", "phase"]) == Some("over");
    if over(next) && !over(prev) {
        if let (Some(winner), Some(team), Some((ours, theirs))) = (text(next, &["round", "win_team"]), text(next, &["player", "team"]), score(next)) {
            out.push(if winner == team { Event::RoundWon { ours, theirs } } else { Event::RoundLost { ours, theirs } });
        }
    }
    let gameover = |v: &Value| text(v, &["map", "phase"]) == Some("gameover");
    if gameover(next) && !gameover(prev) {
        if let Some((ours, theirs)) = score(next) {
            out.push(Event::MatchOver { won: ours > theirs, ours, theirs });
        }
    }
    out
}

/// Receives CS2's posts on [`PORT`] until dropped.
pub struct Receiver {
    stop: Arc<AtomicBool>,
}

impl Receiver {
    pub fn start(token: String, on_event: impl Fn(Event) + Send + 'static) -> std::io::Result<Receiver> {
        let listener = TcpListener::bind(("127.0.0.1", PORT))?;
        listener.set_nonblocking(true)?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        std::thread::Builder::new().name("cs2-gsi".into()).spawn(move || {
            let mut prev = Value::Null;
            while !flag.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((s, _)) => {
                        if let Some(state) = read_post(s, &token) {
                            for e in diff(&prev, &state) {
                                on_event(e);
                            }
                            prev = state;
                        }
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(50)),
                }
            }
        })?;
        Ok(Receiver { stop })
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// One HTTP post: its JSON when the token matches. Always answers 200 so CS2 keeps sending.
fn read_post(mut s: TcpStream, token: &str) -> Option<Value> {
    let _ = s.set_nonblocking(false);
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    let mut r = BufReader::new(s.try_clone().ok()?);
    let mut len = 0usize;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            if k.eq_ignore_ascii_case("content-length") {
                len = v.trim().parse().ok()?;
            }
        }
    }
    if len > 1 << 20 {
        return None;
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body).ok()?;
    let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    let v: Value = serde_json::from_slice(&body).ok()?;
    (text(&v, &["auth", "token"]) == Some(token)).then_some(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn state(kills: u64, phase: &str, win: Option<&str>, ct: u64, t: u64, mvps: u64) -> Value {
        json!({
            "provider": { "steamid": "7656" },
            "map": { "phase": if ct >= 13 || t >= 13 { "gameover" } else { "live" }, "team_ct": { "score": ct }, "team_t": { "score": t } },
            "round": { "phase": phase, "win_team": win },
            "player": { "steamid": "7656", "team": "CT", "state": { "round_kills": kills }, "match_stats": { "mvps": mvps } },
            "auth": { "token": "abc" }
        })
    }

    #[test]
    fn rounds_kills_and_the_match() {
        let a = state(0, "live", None, 3, 2, 0);
        let b = state(1, "live", None, 3, 2, 0);
        assert_eq!(diff(&a, &b), vec![Event::Kill { round_kills: 1 }]);
        let c = state(3, "over", Some("CT"), 4, 2, 1);
        assert_eq!(diff(&b, &c), vec![Event::Kill { round_kills: 3 }, Event::Mvp, Event::RoundWon { ours: 4, theirs: 2 }]);
        let d = state(0, "over", Some("T"), 12, 13, 1);
        assert!(diff(&c, &d).contains(&Event::MatchOver { won: false, ours: 12, theirs: 13 }));
        // Spectating someone else after dying: their kills aren't yours.
        let mut spec = state(2, "live", None, 3, 2, 0);
        spec["player"]["steamid"] = json!("other");
        assert!(diff(&a, &spec).is_empty());
        assert!(diff(&spec, &b).is_empty(), "back on yourself: no made-up kill");
    }

    #[test]
    fn config_and_posts() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("game").join("bin").join("win64").join("cs2.exe");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::create_dir_all(dir.path().join("game").join("csgo").join("cfg")).unwrap();
        let cfg = cfg_dir(&exe).unwrap();
        assert!(install(&cfg, "tok").unwrap());
        assert!(!install(&cfg, "tok").unwrap(), "unchanged: not written again");
        let text = std::fs::read_to_string(cfg.join(CFG_NAME)).unwrap();
        assert!(text.contains("\"uri\" \"http://127.0.0.1:47830/cs2\"") && text.contains("\"token\" \"tok\""));

        let (tx, rx) = std::sync::mpsc::channel();
        let Ok(_r) = Receiver::start("abc".into(), move |e| tx.send(e).unwrap()) else { return }; // port taken on this machine
        let post = |v: &Value| {
            let body = v.to_string();
            let mut s = TcpStream::connect(("127.0.0.1", PORT)).unwrap();
            write!(s, "POST /cs2 HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
            let mut reply = String::new();
            let _ = s.read_to_string(&mut reply);
            assert!(reply.starts_with("HTTP/1.1 200"));
        };
        post(&state(0, "live", None, 0, 0, 0));
        post(&state(1, "live", None, 0, 0, 0));
        let mut wrong = state(2, "live", None, 0, 0, 0);
        wrong["auth"]["token"] = json!("nope");
        post(&wrong);
        assert_eq!(rx.recv_timeout(Duration::from_secs(3)).unwrap(), Event::Kill { round_kills: 1 });
        assert!(rx.recv_timeout(Duration::from_millis(300)).is_err(), "a wrong token is ignored");
    }
}
