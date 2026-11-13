//! Play a song in Spotify straight away, without bringing Spotify to the
//! front: Spotify's Web API (Spotify Connect) starts it on this PC's Spotify.
//!
//! Spotify only allows that to apps the user registers themselves: the user
//! makes a free app at developer.spotify.com (Spotify Premium needed), pastes
//! its Client ID and signs in once in the browser (Authorization Code with
//! PKCE, no client secret). The sign-in comes back to
//! `http://127.0.0.1:43827/callback` on this PC only. The refresh token is
//! kept encrypted with Windows DPAPI in `spotify.json`; nothing goes anywhere
//! but accounts.spotify.com and api.spotify.com.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use base64::Engine as _;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::catalog::Item;

pub const PORT: u16 = 43827;
pub const REDIRECT: &str = "http://127.0.0.1:43827/callback";
const SCOPES: &str = "user-read-playback-state user-modify-playback-state";
const ENTROPY: &[u8] = b"omnihub-spotify-v1";

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Saved {
    #[serde(default)]
    client_id: String,
    /// Refresh token, DPAPI-encrypted, base64.
    #[serde(default)]
    refresh: String,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub client_id: String,
    pub connected: bool,
    /// The Spotify account's display name.
    pub name: String,
    pub redirect: String,
}

struct Access {
    token: String,
    until: Instant,
}

pub struct Spotify {
    file: PathBuf,
    saved: Mutex<Saved>,
    access: Mutex<Option<Access>>,
}

/// A Client ID is 32 hex characters.
pub fn client_id_ok(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn b64url(b: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
}

/// PKCE: a random verifier and its S256 challenge.
pub fn pkce() -> (String, String) {
    use rand::RngCore;
    use sha2::Digest;
    let mut raw = [0u8; 48];
    rand::thread_rng().fill_bytes(&mut raw);
    let verifier = b64url(&raw);
    let challenge = b64url(&sha2::Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

pub fn auth_url(client_id: &str, challenge: &str, state: &str) -> String {
    format!(
        "https://accounts.spotify.com/authorize?response_type=code&client_id={}&scope={}&redirect_uri={}&code_challenge_method=S256&code_challenge={}&state={}",
        enc(client_id),
        enc(SCOPES),
        enc(REDIRECT),
        enc(challenge),
        enc(state)
    )
}

/// `code` and `state` from the browser's `GET /callback?...` line.
pub fn parse_callback(request_line: &str) -> Option<(Option<String>, Option<String>, Option<String>)> {
    let path = request_line.split_whitespace().nth(1)?;
    let query = path.strip_prefix("/callback?")?;
    let mut code = None;
    let mut state = None;
    let mut error = None;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = v.to_string();
        match k {
            "code" => code = Some(v),
            "state" => state = Some(v),
            "error" => error = Some(v),
            _ => {}
        }
    }
    Some((code, state, error))
}

fn norm(s: &str) -> String {
    let s = s.to_lowercase();
    let s = s.split(" (feat").next().unwrap_or(&s).split(" - ").next().unwrap_or(&s).to_string();
    s.chars().filter(|c| c.is_alphanumeric()).collect()
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(12))).http_status_as_error(false).build().into()
}

/// The best search result for `item`: its URI and, for a song, its album's.
pub fn pick(item: &Item, results: &Value) -> Option<(String, Option<String>)> {
    let (kind, want_name, want_artist) = match item {
        Item::Track { title, artist, .. } => ("tracks", norm(title), norm(artist)),
        Item::Album { title, artist } => ("albums", norm(title), norm(artist)),
        Item::Artist { name } => ("artists", norm(name), String::new()),
    };
    let items = results.get(kind)?.get("items")?.as_array()?;
    let artist_ok = |v: &Value| {
        if want_artist.is_empty() {
            return true;
        }
        v.get("artists").and_then(Value::as_array).is_some_and(|a| {
            a.iter().any(|x| {
                let n = norm(x.get("name").and_then(Value::as_str).unwrap_or_default());
                !n.is_empty() && (want_artist.contains(&n) || n.contains(&want_artist))
            })
        })
    };
    let name_of = |v: &Value| norm(v.get("name").and_then(Value::as_str).unwrap_or_default());
    let best = items
        .iter()
        .filter(|v| !v.is_null())
        .find(|v| name_of(v) == want_name && artist_ok(v))
        .or_else(|| items.iter().filter(|v| !v.is_null()).find(|v| (name_of(v).contains(&want_name) || want_name.contains(&name_of(v))) && artist_ok(v)))
        .or_else(|| items.iter().find(|v| !v.is_null()))?;
    let uri = best.get("uri")?.as_str()?.to_string();
    let album = best.get("album").and_then(|a| a.get("uri")).and_then(Value::as_str).map(str::to_string);
    Some((uri, album))
}

/// This PC's Spotify among the devices: the active computer, else one named
/// like this PC, else any computer.
pub fn pick_device(devices: &Value, pc_name: &str) -> Option<String> {
    let list = devices.get("devices")?.as_array()?;
    let computer = |d: &&Value| d.get("type").and_then(Value::as_str).is_some_and(|t| t.eq_ignore_ascii_case("computer")) && !d.get("is_restricted").and_then(Value::as_bool).unwrap_or(false);
    let pc = pc_name.to_lowercase();
    let named = |d: &&Value| !pc.is_empty() && d.get("name").and_then(Value::as_str).is_some_and(|n| n.to_lowercase() == pc);
    let active = |d: &&Value| d.get("is_active").and_then(Value::as_bool).unwrap_or(false);
    list.iter()
        .filter(computer)
        .find(|d| active(d) && named(d))
        .or_else(|| list.iter().filter(computer).find(named))
        .or_else(|| list.iter().filter(computer).find(active))
        .or_else(|| list.iter().find(computer))
        .and_then(|d| d.get("id").and_then(Value::as_str))
        .map(str::to_string)
}

/// The body that starts `uri`: a song inside its album (so the album goes
/// on afterwards), or the album/artist itself.
pub fn play_body(uri: &str, album: Option<&str>) -> Value {
    if uri.starts_with("spotify:track:") {
        match album {
            Some(a) => json!({ "context_uri": a, "offset": { "uri": uri } }),
            None => json!({ "uris": [uri] }),
        }
    } else {
        json!({ "context_uri": uri })
    }
}

impl Spotify {
    pub fn new(dir: PathBuf) -> Self {
        let file = dir.join("spotify.json");
        let saved = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Spotify { file, saved: Mutex::new(saved), access: Mutex::new(None) }
    }

    fn store(&self, s: &Saved) -> Result<(), String> {
        if let Some(p) = self.file.parent() {
            let _ = std::fs::create_dir_all(p);
        }
        std::fs::write(&self.file, serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    pub fn status(&self) -> Status {
        let s = self.saved.lock();
        Status { client_id: s.client_id.clone(), connected: !s.refresh.is_empty(), name: s.name.clone(), redirect: REDIRECT.into() }
    }

    pub fn connected(&self) -> bool {
        !self.saved.lock().refresh.is_empty()
    }

    pub fn disconnect(&self) -> Result<(), String> {
        let mut s = self.saved.lock();
        s.refresh.clear();
        s.name.clear();
        *self.access.lock() = None;
        self.store(&s)
    }

    fn refresh_token(&self) -> Option<zeroize::Zeroizing<String>> {
        let s = self.saved.lock();
        if s.refresh.is_empty() {
            return None;
        }
        let bytes = base64::engine::general_purpose::STANDARD.decode(&s.refresh).ok()?;
        let plain = crate::system::dpapi::unprotect(&bytes, ENTROPY).ok()?;
        Some(zeroize::Zeroizing::new(String::from_utf8_lossy(&plain).into_owned()))
    }

    /// Ask accounts.spotify.com for tokens; keeps the new refresh token.
    fn token_request(&self, form: &str) -> Result<String, String> {
        let mut res = agent()
            .post("https://accounts.spotify.com/api/token")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send(form)
            .map_err(|e| format!("couldn't reach Spotify: {e}"))?;
        let code = res.status().as_u16();
        let body: Value = serde_json::from_str(&res.body_mut().read_to_string().unwrap_or_default()).unwrap_or(Value::Null);
        if code != 200 {
            let why = body.get("error_description").and_then(Value::as_str).unwrap_or("refused");
            if code == 400 && body.get("error").and_then(Value::as_str) == Some("invalid_grant") {
                let _ = self.disconnect();
                return Err("Spotify signed OmniHub out. Connect Spotify again in Aurora's settings.".into());
            }
            return Err(format!("Spotify said: {why}"));
        }
        let token = body.get("access_token").and_then(Value::as_str).ok_or("Spotify sent no token")?.to_string();
        let secs = body.get("expires_in").and_then(Value::as_u64).unwrap_or(3600);
        if let Some(r) = body.get("refresh_token").and_then(Value::as_str) {
            let enc = crate::system::dpapi::protect(r.as_bytes(), ENTROPY).map_err(|e| e.to_string())?;
            let mut s = self.saved.lock();
            s.refresh = base64::engine::general_purpose::STANDARD.encode(enc);
            self.store(&s)?;
        }
        *self.access.lock() = Some(Access { token: token.clone(), until: Instant::now() + Duration::from_secs(secs.saturating_sub(60)) });
        Ok(token)
    }

    fn token(&self) -> Result<String, String> {
        if let Some(a) = self.access.lock().as_ref().filter(|a| a.until > Instant::now()) {
            return Ok(a.token.clone());
        }
        let refresh = self.refresh_token().ok_or("Connect Spotify first (Aurora → Settings → Spotify).")?;
        let id = self.saved.lock().client_id.clone();
        self.token_request(&format!("grant_type=refresh_token&refresh_token={}&client_id={}", enc(&refresh), enc(&id)))
    }

    /// Call the Web API: (status, JSON body or Null).
    fn api(&self, method: &str, path: &str, body: Option<&Value>) -> Result<(u16, Value), String> {
        let token = self.token()?;
        let url = format!("https://api.spotify.com/v1{path}");
        let a = agent();
        let auth = format!("Bearer {token}");
        let res = match (method, body) {
            ("GET", _) => a.get(&url).header("Authorization", &auth).call(),
            ("PUT", Some(b)) => a.put(&url).header("Authorization", &auth).header("Content-Type", "application/json").send(b.to_string()),
            ("PUT", None) => a.put(&url).header("Authorization", &auth).header("Content-Length", "0").send(""),
            ("POST", None) => a.post(&url).header("Authorization", &auth).header("Content-Length", "0").send(""),
            _ => return Err("unsupported".into()),
        };
        let mut res = res.map_err(|e| format!("couldn't reach Spotify: {e}"))?;
        let code = res.status().as_u16();
        let text = res.body_mut().with_config().limit(2 << 20).read_to_string().unwrap_or_default();
        Ok((code, serde_json::from_str(&text).unwrap_or(Value::Null)))
    }

    /// Sign in: opens the browser on Spotify's consent page and waits (up to
    /// 3 minutes) for it to come back to 127.0.0.1. Returns the account name.
    pub fn connect(&self, client_id: &str) -> Result<String, String> {
        let client_id = client_id.trim();
        if !client_id_ok(client_id) {
            return Err("That isn't a Spotify Client ID (32 letters and numbers, from your app's page on developer.spotify.com).".into());
        }
        {
            let mut s = self.saved.lock();
            s.client_id = client_id.to_string();
            self.store(&s)?;
        }
        let listener = TcpListener::bind(("127.0.0.1", PORT)).map_err(|e| format!("Port {PORT} is busy on this PC ({e}). Close whatever uses it and try again."))?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let (verifier, challenge) = pkce();
        let state = b64url(&rand::random::<[u8; 16]>());
        crate::system::shell::shell_execute(&auth_url(client_id, &challenge, &state), "").map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(180);
        let code = loop {
            if Instant::now() > deadline {
                return Err("Spotify's sign-in wasn't finished in 3 minutes.".into());
            }
            let Ok((mut sock, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(150));
                continue;
            };
            let _ = sock.set_nonblocking(false);
            let _ = sock.set_read_timeout(Some(Duration::from_secs(5)));
            let mut line = String::new();
            let _ = BufReader::new(&sock).read_line(&mut line);
            let reply = |sock: &mut std::net::TcpStream, msg: &str| {
                let html = format!("<!doctype html><meta charset=utf-8><title>OmniHub</title><body style=\"font:16px system-ui;background:#0b0b10;color:#eee;display:grid;place-items:center;height:90vh\"><p>{msg}</p>");
                let _ = write!(sock, "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}", html.len());
            };
            match parse_callback(&line) {
                Some((_, Some(st), _)) if st != state => reply(&mut sock, "This sign-in wasn't started by OmniHub."),
                Some((_, _, Some(err))) => {
                    reply(&mut sock, "Spotify didn't connect. You can close this tab.");
                    return Err(if err == "access_denied" { "You cancelled the Spotify sign-in.".into() } else { format!("Spotify said: {err}") });
                }
                Some((Some(code), Some(_), None)) => {
                    reply(&mut sock, "Spotify is connected to OmniHub. You can close this tab.");
                    break code;
                }
                _ => {
                    let _ = write!(sock, "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                }
            }
        };
        self.token_request(&format!("grant_type=authorization_code&code={}&redirect_uri={}&client_id={}&code_verifier={}", enc(&code), enc(REDIRECT), enc(client_id), enc(&verifier)))?;
        let name = self.api("GET", "/me", None).ok().and_then(|(_, v)| v.get("display_name").and_then(Value::as_str).map(str::to_string)).unwrap_or_default();
        let mut s = self.saved.lock();
        s.name = name.clone();
        self.store(&s)?;
        Ok(name)
    }

    fn devices(&self) -> Result<Option<String>, String> {
        let (code, v) = self.api("GET", "/me/player/devices", None)?;
        if code != 200 {
            return Err(format!("Spotify answered {code} for the device list"));
        }
        Ok(pick_device(&v, &std::env::var("COMPUTERNAME").unwrap_or_default()))
    }

    /// Start Spotify in the background (minimized) so it shows up as a device.
    fn wake_app(&self) {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            let exe = std::env::var("APPDATA").map(|a| PathBuf::from(a).join("Spotify").join("Spotify.exe")).ok().filter(|p| p.exists());
            match exe {
                Some(exe) => {
                    let _ = std::process::Command::new(exe).arg("--minimized").creation_flags(0x0800_0000).spawn();
                }
                // The Microsoft Store version: its link scheme starts it.
                None => {
                    let _ = crate::system::shell::shell_execute("spotify:", "");
                }
            }
        }
    }

    /// Search Spotify for `item`: its URI and, for a song, its album's.
    fn find(&self, item: &Item) -> Result<(String, Option<String>), String> {
        let (kind, q) = match item {
            Item::Track { title, artist, .. } => {
                let (t, a) = super::lyrics::clean_query(title, artist);
                ("track", format!("track:{t} artist:{a}"))
            }
            Item::Album { title, artist } => ("album", format!("album:{title} artist:{artist}")),
            Item::Artist { name } => ("artist", name.clone()),
        };
        let (code, found) = self.api("GET", &format!("/search?type={kind}&limit=8&q={}", enc(&q)), None)?;
        if code != 200 {
            return Err(format!("Spotify's search answered {code}"));
        }
        pick(item, &found).ok_or_else(|| "Spotify doesn't have it.".to_string())
    }

    /// Put a song after the one playing (in Spotify's queue).
    pub fn queue(&self, item: &Item) -> Result<(), String> {
        if !matches!(item, Item::Track { .. }) {
            return Err("Only songs can be queued.".into());
        }
        let (uri, _) = self.find(item)?;
        let (code, _) = self.api("POST", &format!("/me/player/queue?uri={}", enc(&uri)), None)?;
        match code {
            200..=204 => Ok(()),
            404 => Err("Nothing is playing in Spotify right now: play something first.".into()),
            403 => Err("Spotify only lets other apps queue songs with Spotify Premium.".into()),
            c => Err(format!("Spotify didn't queue it ({c}).")),
        }
    }

    /// Find `item` on Spotify and start it on this PC. Returns what plays.
    pub fn play(&self, item: &Item) -> Result<String, String> {
        let (uri, album) = self.find(item)?;
        let mut device = self.devices()?;
        if device.is_none() {
            self.wake_app();
            for _ in 0..16 {
                std::thread::sleep(Duration::from_millis(700));
                device = self.devices()?;
                if device.is_some() {
                    break;
                }
            }
        }
        let device = device.ok_or("Spotify isn't open on this PC (and didn't start). Open Spotify once, then try again.")?;
        let body = play_body(&uri, album.as_deref());
        let path = format!("/me/player/play?device_id={}", enc(&device));
        let (mut code, mut v) = self.api("PUT", &path, Some(&body))?;
        if code == 404 {
            // Not the active device yet: make it active, then play.
            let _ = self.api("PUT", "/me/player", Some(&json!({ "device_ids": [device], "play": false })));
            std::thread::sleep(Duration::from_millis(400));
            (code, v) = self.api("PUT", &path, Some(&body))?;
        }
        match code {
            200..=204 => Ok(uri),
            403 => Err("Spotify only lets other apps start songs with Spotify Premium.".into()),
            401 => Err("Spotify signed OmniHub out. Connect Spotify again.".into()),
            _ => Err(format!("Spotify didn't play it ({}).", v.get("error").and_then(|e| e.get("message")).and_then(Value::as_str).unwrap_or("error"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_s256_of_the_verifier() {
        use sha2::Digest;
        let (v, c) = pkce();
        assert!(v.len() >= 43 && v.len() <= 128);
        assert_eq!(c, b64url(&sha2::Sha256::digest(v.as_bytes())));
        assert!(!c.contains('=') && !c.contains('+') && !c.contains('/'));
    }

    #[test]
    fn client_ids_and_auth_url() {
        assert!(client_id_ok("0123456789abcdef0123456789ABCDEF"));
        assert!(!client_id_ok("short"));
        assert!(!client_id_ok("0123456789abcdef0123456789abcdeg"));
        let u = auth_url("0123456789abcdef0123456789abcdef", "ch", "st");
        assert!(u.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A43827%2Fcallback"));
        assert!(u.contains("code_challenge_method=S256") && u.contains("user-modify-playback-state"));
    }

    #[test]
    fn callback_is_read() {
        assert_eq!(parse_callback("GET /callback?code=abc&state=xyz HTTP/1.1"), Some((Some("abc".into()), Some("xyz".into()), None)));
        assert_eq!(parse_callback("GET /callback?error=access_denied&state=xyz HTTP/1.1"), Some((None, Some("xyz".into()), Some("access_denied".into()))));
        assert_eq!(parse_callback("GET /favicon.ico HTTP/1.1"), None);
    }

    #[test]
    fn picks_the_right_song_and_its_album() {
        let r = json!({"tracks":{"items":[
            {"name":"Noticed (Live)","uri":"spotify:track:live","artists":[{"name":"Lil Mosey"}],"album":{"uri":"spotify:album:l"}},
            {"name":"Noticed","uri":"spotify:track:other","artists":[{"name":"Someone"}],"album":{"uri":"spotify:album:o"}},
            {"name":"Noticed","uri":"spotify:track:good","artists":[{"name":"Lil Mosey"}],"album":{"uri":"spotify:album:g"}}
        ]}});
        let item = Item::Track { title: "Noticed".into(), artist: "Lil Mosey".into(), album: String::new() };
        assert_eq!(pick(&item, &r), Some(("spotify:track:good".into(), Some("spotify:album:g".into()))));
        assert_eq!(play_body("spotify:track:good", Some("spotify:album:g")), json!({"context_uri":"spotify:album:g","offset":{"uri":"spotify:track:good"}}));
        assert_eq!(play_body("spotify:artist:x", None), json!({"context_uri":"spotify:artist:x"}));
    }

    #[test]
    fn picks_this_pcs_spotify() {
        let d = json!({"devices":[
            {"id":"phone","type":"Smartphone","is_active":true,"name":"iPhone"},
            {"id":"other","type":"Computer","is_active":false,"name":"LAPTOP"},
            {"id":"me","type":"Computer","is_active":false,"name":"GAMING-PC"}
        ]});
        assert_eq!(pick_device(&d, "GAMING-PC").as_deref(), Some("me"));
        assert_eq!(pick_device(&d, "").as_deref(), Some("other"));
        assert_eq!(pick_device(&json!({"devices":[{"id":"phone","type":"Smartphone"}]}), "PC"), None);
    }
}
