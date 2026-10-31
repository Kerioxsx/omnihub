//! Animated album covers ("motion artwork"), for Aurora's background.
//!
//! Many albums on Apple Music have a looping, animated cover, up to 4K. When
//! allowed, OmniHub finds the playing song in Apple's catalogue with the same
//! public key Apple's own web player at music.apple.com uses (read from that
//! page, renewed when it expires; no account), and asks for its album's
//! "editorial video". What Aurora gets is an HLS address on Apple's video
//! servers, which allow any app to play them. Only the title and artist are
//! sent; answers are cached on disk.

use std::path::PathBuf;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::lyrics::clean_query;

const API: &str = "https://amp-api.music.apple.com/v1/catalog/us";
const PLAYER: &str = "https://music.apple.com/us/new";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum MotionStatus {
    /// The album's animated cover (an HLS playlist), square.
    Found { url: String },
    Searching,
    None,
    Off,
    NothingPlaying,
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The web player's key in its script: a JWT issued by "AMPWebPlay".
pub fn token_in(script: &str) -> Option<String> {
    let mut rest = script;
    while let Some(i) = rest.find("eyJ") {
        let cand: String = rest[i..].chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '_' || *c == '-').collect();
        if cand.matches('.').count() == 2 && cand.len() > 100 {
            if let Some((exp, iss)) = claims(&cand) {
                if iss == "AMPWebPlay" && exp > 0 {
                    return Some(cand);
                }
            }
        }
        rest = &rest[i + 3..];
    }
    None
}

/// (expiry in seconds, issuer) of a JWT.
pub fn claims(jwt: &str) -> Option<(u64, String)> {
    use base64::Engine;
    let payload = jwt.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')).ok()?;
    let v: Value = serde_json::from_slice(&bytes).ok()?;
    Some((v.get("exp")?.as_u64()?, v.get("iss")?.as_str()?.to_string()))
}

/// The web player's main script, named in its page.
pub fn script_in(page: &str) -> Option<String> {
    let i = page.find("/assets/index")?;
    let end = page[i..].find(".js")? + i + 3;
    Some(format!("https://music.apple.com{}", &page[i..end]))
}

/// The album id of the best-matching song in a catalogue search answer.
pub fn album_in(search: &Value, title: &str, artist: &str, duration_ms: u64) -> Option<String> {
    let (t, a) = clean_query(title, artist);
    let (nt, na) = (norm(&t), norm(&a));
    let version = super::artwork::version_of(title);
    let songs = search.pointer("/results/songs/data")?.as_array()?;
    songs.iter().find_map(|s| {
        let at = s.get("attributes")?;
        let name = at.get("name")?.as_str()?;
        let st = norm(&clean_query(name, "\u{1}omnihub\u{1}").0);
        let ar = norm(at.get("artistName")?.as_str()?);
        let ms = at.get("durationInMillis").and_then(Value::as_u64).unwrap_or(0);
        let ok = (st == nt || (nt.len() >= 4 && (st.starts_with(&nt) || nt.starts_with(&st)) && st.len().abs_diff(nt.len()) <= 10))
            && (na.is_empty() || ar.contains(&na) || na.contains(&ar))
            && (duration_ms == 0 || ms == 0 || ms.abs_diff(duration_ms) <= 8_000)
            && super::artwork::version_of(name) == version;
        if !ok {
            return None;
        }
        // ".../album/name/1690685331?i=1690685353"
        let url = at.get("url")?.as_str()?;
        let path = url.split('?').next()?;
        path.rsplit('/').next().filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())).map(str::to_string)
    })
}

/// The square animated cover in an album answer (`extend=editorialVideo`).
pub fn motion_in(album: &Value) -> Option<String> {
    let ev = album.pointer("/data/0/attributes/editorialVideo")?;
    ["motionDetailSquare", "motionSquareVideo1x1"].iter().find_map(|k| ev.pointer(&format!("/{k}/video")).and_then(Value::as_str).filter(|u| u.starts_with("https://")).map(str::to_string))
}

fn get(url: &str, token: Option<&str>) -> Result<(u16, String), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(15))).http_status_as_error(false).build().into();
    let mut req = agent.get(url).header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36");
    if let Some(t) = token {
        req = req.header("Authorization", &format!("Bearer {t}")).header("Origin", "https://music.apple.com");
    }
    let mut res = req.call().map_err(|e| format!("could not reach Apple Music: {e}"))?;
    let code = res.status().as_u16();
    let body = res.body_mut().with_config().limit(8 << 20).read_to_string().map_err(|e| e.to_string())?;
    Ok((code, body))
}

#[derive(Serialize, Deserialize)]
struct Cached {
    at: u64,
    url: Option<String>,
}

pub struct MotionFinder {
    dir: PathBuf,
    token: Mutex<Option<String>>,
}

impl MotionFinder {
    pub fn new(dir: PathBuf) -> Self {
        MotionFinder { dir, token: Mutex::new(None) }
    }

    fn file(&self, key: &str) -> PathBuf {
        use sha2::Digest;
        self.dir.join(format!("{}.json", hex::encode(&sha2::Sha256::digest(key.as_bytes())[..12])))
    }

    /// A cached answer: a month for a cover, three days for none.
    pub fn cached(&self, key: &str, now_ms: u64) -> Option<Option<String>> {
        let c: Cached = serde_json::from_slice(&std::fs::read(self.file(key)).ok()?).ok()?;
        let keep = if c.url.is_some() { 30 } else { 3 } * 86_400_000;
        (now_ms.saturating_sub(c.at) < keep).then_some(c.url)
    }

    pub fn remember(&self, key: &str, url: &Option<String>, now_ms: u64) {
        let _ = std::fs::create_dir_all(&self.dir);
        if let Ok(b) = serde_json::to_vec(&Cached { at: now_ms, url: url.clone() }) {
            let _ = crate::settings::write_atomic(&self.file(key), &b);
        }
    }

    /// The web player's key (kept, and on disk, until it expires).
    fn token(&self, fresh: bool) -> Result<String, String> {
        let now = (crate::db::now().max(0) as u64) / 1000;
        let path = self.dir.join("key.txt");
        if !fresh {
            if let Some(t) = self.token.lock().clone() {
                return Ok(t);
            }
            if let Ok(t) = std::fs::read_to_string(&path) {
                if claims(t.trim()).is_some_and(|(exp, _)| exp > now + 3600) {
                    *self.token.lock() = Some(t.trim().to_string());
                    return Ok(t.trim().to_string());
                }
            }
        }
        let (_, page) = get(PLAYER, None)?;
        let script = script_in(&page).ok_or("Apple Music's page has changed")?;
        let (_, js) = get(&script, None)?;
        let t = token_in(&js).ok_or("Apple Music's web player key was not found")?;
        let _ = std::fs::create_dir_all(&self.dir);
        let _ = std::fs::write(&path, &t);
        *self.token.lock() = Some(t.clone());
        Ok(t)
    }

    fn api(&self, url: &str) -> Result<Value, String> {
        let mut token = self.token(false)?;
        for attempt in 0..2 {
            let (code, body) = get(url, Some(&token))?;
            match code {
                200 => return serde_json::from_str(&body).map_err(|e| e.to_string()),
                401 | 403 if attempt == 0 => token = self.token(true)?,
                404 => return Ok(Value::Null),
                c => return Err(format!("Apple Music answered {c}")),
            }
        }
        Err("Apple Music refused the web player key".into())
    }

    /// The song's album's animated cover, if it has one.
    pub fn find(&self, title: &str, artist: &str, duration_ms: u64) -> Result<Option<String>, String> {
        let (t, a) = clean_query(title, artist);
        if t.is_empty() {
            return Ok(None);
        }
        let search = self.api(&format!("{API}/search?term={}&types=songs&limit=10", enc(&format!("{a} {t}"))))?;
        let Some(album) = album_in(&search, title, artist, duration_ms) else { return Ok(None) };
        let album = self.api(&format!("{API}/albums/{album}?extend=editorialVideo"))?;
        Ok(motion_in(&album))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jwt(iss: &str, exp: u64) -> String {
        use base64::Engine;
        let e = |v: &str| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v);
        format!("{}.{}.{}", e(r#"{"typ":"JWT","alg":"ES256","kid":"WebPlayKid"}"#), e(&format!(r#"{{"iss":"{iss}","iat":1,"exp":{exp},"root_https_origin":["apple.com"]}}"#)), "c2lnbmF0dXJlLXNpZ25hdHVyZS1zaWduYXR1cmUtc2lnbmF0dXJl")
    }

    #[test]
    fn finds_the_web_players_key() {
        let other = jwt("SomeoneElse", 5);
        let ours = jwt("AMPWebPlay", 1_796_942_536);
        let script = format!(r#"var a="{other}";const b={{token:"{ours}"}};"#);
        assert_eq!(token_in(&script).as_deref(), Some(ours.as_str()));
        assert_eq!(claims(&ours), Some((1_796_942_536, "AMPWebPlay".to_string())));
        assert_eq!(script_in(r#"<script type="module" crossorigin src="/assets/index~c10ba4a68d.js"></script>"#).as_deref(), Some("https://music.apple.com/assets/index~c10ba4a68d.js"));
    }

    #[test]
    fn picks_the_album_and_its_animated_cover() {
        let search: Value = serde_json::from_str(r#"{"results":{"songs":{"data":[
            {"id":"1700523775","attributes":{"name":"Am I Dreaming (Instrumental)","artistName":"Metro Boomin","durationInMillis":256000,"url":"https://music.apple.com/us/album/x/1700520936?i=1700523775"}},
            {"id":"1690685353","attributes":{"name":"Am I Dreaming","artistName":"Metro Boomin, A$AP Rocky & Roisee","durationInMillis":256033,"url":"https://music.apple.com/us/album/am-i-dreaming/1690685331?i=1690685353"}}
        ]}}}"#).unwrap();
        assert_eq!(album_in(&search, "Am I Dreaming", "Metro Boomin", 256_000).as_deref(), Some("1690685331"));
        assert_eq!(album_in(&search, "Am I Dreaming", "Metro Boomin", 180_000), None);
        let album: Value = serde_json::from_str(r#"{"data":[{"attributes":{"editorialVideo":{"motionDetailTall":{"video":"https://mvod.itunes.apple.com/tall.m3u8"},"motionDetailSquare":{"video":"https://mvod.itunes.apple.com/square.m3u8"}}}}]}"#).unwrap();
        assert_eq!(motion_in(&album).as_deref(), Some("https://mvod.itunes.apple.com/square.m3u8"));
        assert_eq!(motion_in(&serde_json::json!({"data":[{"attributes":{}}]})), None);
    }

    /// Against the real Apple Music (needs the internet): `cargo test -- --ignored live_motion`.
    #[test]
    #[ignore]
    fn live_motion() {
        let dir = std::env::temp_dir().join(format!("omnihub-motion-{}", std::process::id()));
        let f = MotionFinder::new(dir);
        let url = f.find("Am I Dreaming", "Metro Boomin", 256_000).unwrap();
        println!("Am I Dreaming: {url:?}");
        assert!(url.is_some_and(|u| u.contains(".m3u8")));
    }
}
