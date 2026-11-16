//! A Discord status ("Playing …") from what OmniHub knows: the game you're
//! playing and your shiny hunt. Through Discord's own local connection (the
//! "discord-ipc" pipe the desktop app opens), so nothing goes through a
//! server of ours. Discord shows statuses only for an application, so this
//! needs the Application ID of one you make yourself (free, a minute in
//! Discord's Developer Portal); its name is what Discord shows after
//! "Playing".

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct PresenceSettings {
    pub enabled: bool,
    /// The Discord Application ID (a long number).
    pub client_id: String,
    pub show_game: bool,
    pub show_hunt: bool,
}

impl Default for PresenceSettings {
    fn default() -> Self {
        PresenceSettings { enabled: false, client_id: String::new(), show_game: true, show_hunt: true }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub details: String,
    pub state: Option<String>,
    /// Unix ms the "elapsed" timer counts from.
    pub start: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PresenceStatus {
    pub settings: PresenceSettings,
    pub connected: bool,
    pub showing: Option<Activity>,
    pub error: Option<String>,
}

/// One message on Discord's pipe: op and length (little-endian), then JSON.
pub fn frame(op: u32, body: &Value) -> Vec<u8> {
    let text = body.to_string();
    let mut out = Vec::with_capacity(8 + text.len());
    out.extend_from_slice(&op.to_le_bytes());
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(text.as_bytes());
    out
}

/// What to show, from the game running and the hunt going on.
pub fn activity(s: &PresenceSettings, game: Option<(String, i64)>, hunt: Option<(String, i64, String)>) -> Option<Activity> {
    let game = game.filter(|_| s.show_game);
    let hunt = hunt.filter(|_| s.show_hunt);
    match (game, hunt) {
        (Some((name, start)), Some((pokemon, count, odds))) => Some(Activity { details: name, state: Some(format!("Shiny hunting {pokemon}: {count} encounters ({odds})")), start: Some(start) }),
        (Some((name, start)), None) => Some(Activity { details: name, state: None, start: Some(start) }),
        (None, Some((pokemon, count, odds))) => Some(Activity { details: format!("Shiny hunting {pokemon}"), state: Some(format!("{count} encounters · {odds}")), start: None }),
        (None, None) => None,
    }
}

pub fn valid_client_id(id: &str) -> bool {
    (15..=22).contains(&id.len()) && id.chars().all(|c| c.is_ascii_digit())
}

pub struct Presence {
    file: PathBuf,
    settings: Mutex<PresenceSettings>,
    pipe: Mutex<Option<std::fs::File>>,
    showing: Mutex<Option<Activity>>,
    error: Mutex<Option<String>>,
}

impl Presence {
    pub fn new(data: &Path) -> Presence {
        let file = data.join("discord-status.json");
        let settings = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Presence { file, settings: Mutex::new(settings), pipe: Mutex::new(None), showing: Mutex::new(None), error: Mutex::new(None) }
    }

    pub fn settings(&self) -> PresenceSettings {
        self.settings.lock().clone()
    }

    pub fn status(&self) -> PresenceStatus {
        PresenceStatus { settings: self.settings(), connected: self.pipe.lock().is_some(), showing: self.showing.lock().clone(), error: self.error.lock().clone() }
    }

    pub fn save(&self, mut s: PresenceSettings) -> Result<PresenceStatus, String> {
        s.client_id = s.client_id.trim().to_string();
        if s.enabled && !valid_client_id(&s.client_id) {
            return Err("Paste your Discord Application ID (a long number from the Developer Portal)".into());
        }
        let json = serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?;
        crate::settings::write_atomic(&self.file, &json).map_err(|e| e.to_string())?;
        let changed_app = self.settings.lock().client_id != s.client_id;
        *self.settings.lock() = s;
        if changed_app {
            *self.pipe.lock() = None;
        }
        *self.showing.lock() = None;
        Ok(self.status())
    }

    /// Show `want` (or clear the status), connecting to Discord when needed.
    pub fn update(&self, want: Option<Activity>) {
        let s = self.settings();
        let want = want.filter(|_| s.enabled && valid_client_id(&s.client_id));
        if *self.showing.lock() == want && (want.is_none() || self.pipe.lock().is_some()) {
            return;
        }
        if want.is_none() {
            // Closing the connection clears the status.
            *self.pipe.lock() = None;
            *self.showing.lock() = None;
            return;
        }
        let r = self.send_activity(&s.client_id, want.as_ref());
        match r {
            Ok(()) => {
                *self.showing.lock() = want;
                *self.error.lock() = None;
            }
            Err(e) => {
                *self.pipe.lock() = None;
                *self.showing.lock() = None;
                *self.error.lock() = Some(e);
            }
        }
    }

    fn send_activity(&self, client_id: &str, a: Option<&Activity>) -> Result<(), String> {
        let mut pipe = self.pipe.lock();
        if pipe.is_none() {
            let mut p = imp::open().ok_or("Discord isn't running (or isn't the desktop app)")?;
            imp::exchange(&mut p, &frame(0, &json!({ "v": 1, "client_id": client_id })))?;
            *pipe = Some(p);
        }
        let activity = a.map(|a| {
            let mut v = json!({ "details": a.details, "instance": false });
            if let Some(st) = &a.state {
                v["state"] = json!(st);
            }
            if let Some(start) = a.start {
                v["timestamps"] = json!({ "start": start / 1000 });
            }
            v
        });
        let body = json!({ "cmd": "SET_ACTIVITY", "args": { "pid": std::process::id(), "activity": activity }, "nonce": uuid::Uuid::new_v4().to_string() });
        let answer = imp::exchange(pipe.as_mut().ok_or("not connected")?, &frame(1, &body))?;
        if answer.get("evt").and_then(Value::as_str) == Some("ERROR") {
            let msg = answer["data"]["message"].as_str().unwrap_or("Discord refused the status").to_string();
            return Err(if msg.contains("client_id") || msg.contains("Invalid") { "Discord didn't recognise that Application ID".into() } else { msg });
        }
        Ok(())
    }
}

mod imp {
    use serde_json::Value;
    use std::io::{Read, Write};

    #[cfg(windows)]
    pub fn open() -> Option<std::fs::File> {
        (0..10).find_map(|n| std::fs::OpenOptions::new().read(true).write(true).open(format!(r"\\.\pipe\discord-ipc-{n}")).ok())
    }

    #[cfg(not(windows))]
    pub fn open() -> Option<std::fs::File> {
        None
    }

    /// Send a frame and read Discord's answer (it answers every one).
    pub fn exchange(p: &mut std::fs::File, frame: &[u8]) -> Result<Value, String> {
        p.write_all(frame).map_err(|e| format!("Discord closed the connection: {e}"))?;
        let mut head = [0u8; 8];
        p.read_exact(&mut head).map_err(|e| format!("Discord didn't answer: {e}"))?;
        let op = u32::from_le_bytes(head[..4].try_into().unwrap_or_default());
        let len = u32::from_le_bytes(head[4..].try_into().unwrap_or_default()) as usize;
        if len > 1 << 20 {
            return Err("Discord sent something unexpected".into());
        }
        let mut body = vec![0u8; len];
        p.read_exact(&mut body).map_err(|e| format!("Discord didn't answer: {e}"))?;
        let v: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        if op == 2 {
            // CLOSE: usually a wrong Application ID.
            return Err(v["message"].as_str().map(|m| format!("Discord closed the connection: {m}")).unwrap_or_else(|| "Discord closed the connection".into()));
        }
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_and_what_to_show() {
        let f = frame(1, &json!({ "a": 1 }));
        assert_eq!(&f[..4], &1u32.to_le_bytes());
        assert_eq!(u32::from_le_bytes(f[4..8].try_into().unwrap()) as usize, f.len() - 8);
        let s = PresenceSettings::default();
        let hunt = Some(("Charizard".to_string(), 3127, "1/1,366".to_string()));
        let both = activity(&s, Some(("Fortnite".into(), 5_000)), hunt.clone()).unwrap();
        assert_eq!((both.details.as_str(), both.state.as_deref(), both.start), ("Fortnite", Some("Shiny hunting Charizard: 3127 encounters (1/1,366)"), Some(5_000)));
        let only_hunt = activity(&PresenceSettings { show_game: false, ..s.clone() }, Some(("Fortnite".into(), 1)), hunt).unwrap();
        assert_eq!(only_hunt.details, "Shiny hunting Charizard");
        assert!(activity(&s, None, None).is_none());
        assert!(valid_client_id("1234567890123456789") && !valid_client_id("abc") && !valid_client_id("12"));
    }

    #[test]
    fn settings_are_checked_and_kept() {
        let t = tempfile::tempdir().unwrap();
        let p = Presence::new(t.path());
        assert!(p.save(PresenceSettings { enabled: true, client_id: "nope".into(), ..Default::default() }).is_err());
        p.save(PresenceSettings { enabled: true, client_id: " 1234567890123456789 ".into(), ..Default::default() }).unwrap();
        assert_eq!(Presence::new(t.path()).settings().client_id, "1234567890123456789");
        // Without Discord running there's nothing to connect to: it says so and shows nothing.
        p.update(Some(Activity { details: "Fortnite".into(), state: None, start: None }));
        let st = p.status();
        assert!(!st.connected && st.showing.is_none());
        assert!(st.error.is_some());
    }
}
