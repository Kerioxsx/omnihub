//! OBS Studio through its built-in WebSocket server (OBS 28 and later:
//! Tools → WebSocket Server Settings). Used by deck and wheel buttons to
//! switch scenes, start or stop the stream or recording, save the replay
//! buffer, and mute a source. Only to OBS on this PC by default; the
//! password stays encrypted for this Windows user.

use std::io::ErrorKind;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tungstenite::{Message, WebSocket};

pub const DEFAULT_URL: &str = "ws://127.0.0.1:4455";

/// What a button asks OBS to do.
pub const WHATS: &[(&str, &str)] = &[("scene", "Switch to a scene"), ("stream", "Start / stop streaming"), ("record", "Start / stop recording"), ("replay", "Save the replay buffer"), ("mute", "Mute / unmute a source")];

/// OBS's answer to its challenge: base64(sha256(base64(sha256(password + salt)) + challenge)).
pub fn auth_string(password: &str, salt: &str, challenge: &str) -> String {
    let b64 = base64::engine::general_purpose::STANDARD;
    let secret = b64.encode(Sha256::digest(format!("{password}{salt}").as_bytes()));
    b64.encode(Sha256::digest(format!("{secret}{challenge}").as_bytes()))
}

fn host_port(url: &str) -> Result<(String, u16), String> {
    let rest = url.strip_prefix("ws://").ok_or("The OBS address starts with ws:// (like ws://127.0.0.1:4455)")?;
    let hp = rest.split('/').next().unwrap_or(rest);
    let (h, p) = hp.rsplit_once(':').ok_or("Add OBS's port (4455 unless you changed it)")?;
    Ok((h.trim_matches(['[', ']']).to_string(), p.parse().map_err(|_| "That port isn't a number")?))
}

struct Conn(WebSocket<TcpStream>);

impl Conn {
    fn open(url: &str, password: &str) -> Result<Conn, String> {
        let (host, port) = host_port(url)?;
        let addr = (host.as_str(), port).to_socket_addrs().map_err(|e| e.to_string())?.next().ok_or("OBS's address didn't resolve")?;
        let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).map_err(|e| match e.kind() {
            ErrorKind::ConnectionRefused | ErrorKind::TimedOut => "OBS isn't answering: open OBS and turn on Tools → WebSocket Server Settings → Enable WebSocket server".to_string(),
            _ => format!("Couldn't reach OBS: {e}"),
        })?;
        tcp.set_read_timeout(Some(Duration::from_secs(4))).ok();
        let (ws, _) = tungstenite::client(url, tcp).map_err(|e| format!("OBS refused the connection: {e}"))?;
        let mut c = Conn(ws);
        let hello = c.read_op(0)?;
        let auth = hello["authentication"].as_object().map(|a| auth_string(password, a.get("salt").and_then(Value::as_str).unwrap_or(""), a.get("challenge").and_then(Value::as_str).unwrap_or("")));
        if auth.is_some() && password.is_empty() {
            return Err("OBS wants a password: copy it from Tools → WebSocket Server Settings → Show Connect Info".into());
        }
        let mut identify = json!({ "rpcVersion": 1, "eventSubscriptions": 0 });
        if let Some(a) = auth {
            identify["authentication"] = json!(a);
        }
        c.send(1, identify)?;
        c.read_op(2).map_err(|e| if e.contains("closed") { "OBS didn't accept the password".to_string() } else { e })?;
        Ok(c)
    }

    fn send(&mut self, op: u8, d: Value) -> Result<(), String> {
        self.0.send(Message::Text(json!({ "op": op, "d": d }).to_string().into())).map_err(|e| e.to_string())
    }

    fn read_op(&mut self, want: u8) -> Result<Value, String> {
        loop {
            match self.0.read() {
                Ok(Message::Text(t)) => {
                    let v: Value = serde_json::from_str(&t).map_err(|e| e.to_string())?;
                    if v["op"].as_u64() == Some(want as u64) {
                        return Ok(v["d"].clone());
                    }
                }
                Ok(Message::Close(_)) | Err(tungstenite::Error::ConnectionClosed) => return Err("OBS closed the connection".into()),
                Ok(_) => {}
                Err(e) => return Err(format!("OBS stopped answering: {e}")),
            }
        }
    }

    fn request(&mut self, kind: &str, data: Value) -> Result<Value, String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.send(6, json!({ "requestType": kind, "requestId": id, "requestData": data }))?;
        loop {
            let d = self.read_op(7)?;
            if d["requestId"].as_str() != Some(id.as_str()) {
                continue;
            }
            let st = &d["requestStatus"];
            if st["result"].as_bool() == Some(true) {
                return Ok(d["responseData"].clone());
            }
            let why = st["comment"].as_str().map(String::from).unwrap_or_else(|| format!("code {}", st["code"]));
            return Err(format!("OBS said no: {why}"));
        }
    }
}

/// Do a button's OBS action; says what was done.
pub fn run(url: &str, password: &str, what: &str, target: Option<&str>) -> Result<String, String> {
    let mut c = Conn::open(url, password)?;
    let target = target.map(str::trim).filter(|t| !t.is_empty());
    match what {
        "scene" => {
            let name = target.ok_or("Pick a scene")?;
            c.request("SetCurrentProgramScene", json!({ "sceneName": name }))?;
            Ok(format!("Scene: {name}"))
        }
        "stream" => {
            let r = c.request("ToggleStream", json!({}))?;
            Ok(if r["outputActive"].as_bool() == Some(true) { "Streaming".into() } else { "Stream stopped".into() })
        }
        "record" => {
            let r = c.request("ToggleRecord", json!({}))?;
            Ok(if r["outputActive"].as_bool() == Some(true) { "Recording".into() } else { "Recording stopped".into() })
        }
        "replay" => {
            c.request("SaveReplayBuffer", json!({})).map_err(|e| if e.contains("not active") || e.contains("604") { "Start OBS's replay buffer first".to_string() } else { e })?;
            Ok("Replay saved".into())
        }
        "mute" => {
            let name = target.ok_or("Pick a source")?;
            let r = c.request("ToggleInputMute", json!({ "inputName": name }))?;
            Ok(if r["inputMuted"].as_bool() == Some(true) { format!("{name} muted") } else { format!("{name} unmuted") })
        }
        _ => Err("Unknown OBS action".into()),
    }
}

/// Scenes and audio sources, for picking them in a button.
pub fn lists(url: &str, password: &str) -> Result<(Vec<String>, Vec<String>), String> {
    let mut c = Conn::open(url, password)?;
    let scenes = c.request("GetSceneList", json!({}))?["scenes"].as_array().map(|a| a.iter().filter_map(|s| s["sceneName"].as_str().map(String::from)).rev().collect()).unwrap_or_default();
    let inputs = c.request("GetInputList", json!({}))?["inputs"].as_array().map(|a| a.iter().filter_map(|s| s["inputName"].as_str().map(String::from)).collect()).unwrap_or_default();
    Ok((scenes, inputs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// A pretend OBS that checks the password and answers two requests.
    fn fake_obs(password: &'static str) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in l.incoming().take(2) {
                let mut ws = tungstenite::accept(stream.unwrap()).unwrap();
                ws.send(Message::Text(json!({ "op": 0, "d": { "rpcVersion": 1, "authentication": { "challenge": "ch", "salt": "sa" } } }).to_string().into())).unwrap();
                let id: Value = serde_json::from_str(&ws.read().unwrap().into_text().unwrap()).unwrap();
                if id["d"]["authentication"].as_str() != Some(auth_string(password, "sa", "ch").as_str()) {
                    let _ = ws.close(None);
                    continue;
                }
                ws.send(Message::Text(json!({ "op": 2, "d": { "negotiatedRpcVersion": 1 } }).to_string().into())).unwrap();
                while let Ok(Message::Text(t)) = ws.read() {
                    let r: Value = serde_json::from_str(&t).unwrap();
                    let kind = r["d"]["requestType"].as_str().unwrap().to_string();
                    let data = match kind.as_str() {
                        "ToggleStream" => json!({ "outputActive": true }),
                        "GetSceneList" => json!({ "scenes": [{ "sceneName": "Gameplay" }, { "sceneName": "Starting soon" }] }),
                        "GetInputList" => json!({ "inputs": [{ "inputName": "Mic/Aux" }] }),
                        _ => json!({}),
                    };
                    let ok = kind != "SetCurrentProgramScene" || r["d"]["requestData"]["sceneName"] == "Gameplay";
                    ws.send(Message::Text(json!({ "op": 7, "d": { "requestType": kind, "requestId": r["d"]["requestId"], "requestStatus": { "result": ok, "code": if ok { 100 } else { 600 }, "comment": if ok { Value::Null } else { json!("No source was found by the name of `Nope`.") } }, "responseData": data } }).to_string().into())).unwrap();
                }
            }
        });
        format!("ws://127.0.0.1:{port}")
    }

    #[test]
    fn talks_to_obs() {
        let url = fake_obs("hunter2");
        assert_eq!(run(&url, "hunter2", "stream", None).unwrap(), "Streaming");
        let (scenes, inputs) = lists(&url, "hunter2").unwrap();
        assert_eq!((scenes, inputs), (vec!["Starting soon".to_string(), "Gameplay".to_string()], vec!["Mic/Aux".to_string()]));
    }

    #[test]
    fn wrong_password_and_bad_addresses() {
        let url = fake_obs("right");
        assert!(run(&url, "wrong", "stream", None).unwrap_err().contains("password"));
        assert!(run(&url, "", "stream", None).unwrap_err().contains("password"));
        assert!(host_port("http://x").is_err());
        assert_eq!(host_port("ws://127.0.0.1:4455").unwrap(), ("127.0.0.1".to_string(), 4455));
        assert!(run("ws://127.0.0.1:1", "", "stream", None).unwrap_err().contains("OBS isn't answering"));
    }

    #[test]
    fn auth_matches_an_independent_calculation() {
        // The same formula computed separately (Python's hashlib and base64).
        assert_eq!(auth_string("supersecretpassword", "lM1GncleQOaCu9lT1yeUZhFYnqhsLLP1G5lAGo3ixaI=", "+IxH4CnCiqpX1rM9scsNynZzbOe4KhDeYcTNS3PDaeY="), "1Ct943GAT+6YQUUX47Ia/ncufilbe6+oD6lY+5kaCu4=");
    }
}
