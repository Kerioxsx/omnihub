//! Smart home: switch lights, plugs, fans, scenes and scripts in Home
//! Assistant (your own server's address and a long-lived access token from
//! your Home Assistant profile), and Shelly smart plugs straight on the
//! local network. Both work from scenes and deck buttons. The token stays
//! on this PC, encrypted for your Windows account, and is only sent to your
//! Home Assistant.

use std::path::{Path, PathBuf};

use base64::Engine;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const ENTROPY: &[u8] = b"omnihub-home-assistant";
/// The kinds of Home Assistant entities OmniHub switches.
/// (Door locks, alarms and garage doors are left out on purpose.)
pub const DOMAINS: &[&str] = &["light", "switch", "fan", "scene", "script", "input_boolean", "media_player"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
struct Stored {
    url: String,
    secret: String,
    /// Shelly plugs: (name, IP address).
    shelly: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HomeDevice {
    /// "ha:light.desk" or "shelly:192.168.1.40".
    pub id: String,
    pub name: String,
    /// light, switch, scene… or "plug" for a Shelly.
    pub kind: String,
    /// "on", "off", or "" when it has no on/off (scenes, scripts).
    pub state: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct HomeStatus {
    /// Home Assistant's address, when connected.
    pub url: String,
    pub connected: bool,
    pub shelly: Vec<(String, String)>,
}

/// Home Assistant's address, checked: http(s), host and optional port only.
pub fn clean_url(url: &str) -> Result<String, String> {
    let u = url.trim().trim_end_matches('/');
    let rest = u.strip_prefix("http://").or_else(|| u.strip_prefix("https://")).ok_or("Type Home Assistant's address, like http://homeassistant.local:8123")?;
    if rest.is_empty() || rest.contains(['/', '?', '#', '@', ' ']) {
        return Err("Type only the address, like http://192.168.1.10:8123".into());
    }
    Ok(u.to_string())
}

pub fn valid_ip(ip: &str) -> bool {
    ip.parse::<std::net::Ipv4Addr>().is_ok_and(|a| a.is_private() || a.is_link_local())
}

/// The switchable entities in Home Assistant's /api/states answer.
pub fn parse_states(v: &Value) -> Vec<HomeDevice> {
    let mut out: Vec<HomeDevice> = v
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|e| {
                    let id = e.get("entity_id")?.as_str()?;
                    let domain = id.split('.').next()?;
                    if !DOMAINS.contains(&domain) {
                        return None;
                    }
                    let state = e.get("state").and_then(|s| s.as_str()).unwrap_or("");
                    let name = e.pointer("/attributes/friendly_name").and_then(|n| n.as_str()).unwrap_or(id).to_string();
                    Some(HomeDevice { id: format!("ha:{id}"), name, kind: domain.to_string(), state: if matches!(state, "on" | "off") { state.into() } else { String::new() } })
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    out
}

/// The Home Assistant service for an action on an entity: (domain, service).
pub fn service(entity: &str, action: &str) -> Result<(String, String), String> {
    let domain = entity.split('.').next().unwrap_or("");
    if !DOMAINS.contains(&domain) || !entity.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
        return Err("That isn't a Home Assistant device OmniHub switches".into());
    }
    let svc = match (domain, action) {
        ("scene" | "script", _) => "turn_on",
        (_, "on") => "turn_on",
        (_, "off") => "turn_off",
        (_, "toggle") => "toggle",
        _ => return Err("Pick on, off or toggle".into()),
    };
    Ok((domain.to_string(), svc.to_string()))
}

/// A Shelly's address for an action: Gen 2+ (RPC), then Gen 1.
pub fn shelly_urls(ip: &str, action: &str) -> Result<[String; 2], String> {
    let (rpc, gen1) = match action {
        "on" => ("Switch.Set?id=0&on=true", "relay/0?turn=on"),
        "off" => ("Switch.Set?id=0&on=false", "relay/0?turn=off"),
        "toggle" => ("Switch.Toggle?id=0", "relay/0?turn=toggle"),
        _ => return Err("Pick on, off or toggle".into()),
    };
    Ok([format!("http://{ip}/rpc/{rpc}"), format!("http://{ip}/{gen1}")])
}

fn agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
    ureq::Agent::config_builder().tls_config(tls).timeout_global(Some(std::time::Duration::from_secs(8))).http_status_as_error(false).build().into()
}

pub struct Home {
    file: PathBuf,
    stored: Mutex<Stored>,
}

impl Home {
    pub fn new(data: &Path) -> Home {
        let file = data.join("smart-home.json");
        let stored = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Home { file, stored: Mutex::new(stored) }
    }

    fn put(&self, s: Stored) -> Result<(), String> {
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        *self.stored.lock() = s;
        Ok(())
    }

    pub fn status(&self) -> HomeStatus {
        let s = self.stored.lock();
        HomeStatus { url: s.url.clone(), connected: !s.url.is_empty() && !s.secret.is_empty(), shelly: s.shelly.clone() }
    }

    /// Connect Home Assistant (an empty token disconnects). Checks the token works.
    pub fn connect(&self, url: &str, token: &str) -> Result<HomeStatus, String> {
        let mut s = self.stored.lock().clone();
        if token.trim().is_empty() {
            s.url.clear();
            s.secret.clear();
        } else {
            let url = clean_url(url)?;
            let t = token.trim();
            if t.len() > 1000 || t.contains(char::is_whitespace) {
                return Err("That doesn't look like a long-lived access token".into());
            }
            let res = agent().get(&format!("{url}/api/")).header("Authorization", &format!("Bearer {t}")).call().map_err(|_| "Home Assistant didn't answer at that address".to_string())?;
            match res.status().as_u16() {
                200 | 201 => {}
                401 | 403 => return Err("Home Assistant didn't accept that token".into()),
                n => return Err(format!("Home Assistant answered {n}")),
            }
            s.secret = base64::engine::general_purpose::STANDARD.encode(crate::system::dpapi::protect(t.as_bytes(), ENTROPY).map_err(|e| e.to_string())?);
            s.url = url;
        }
        self.put(s)?;
        Ok(self.status())
    }

    pub fn set_shelly(&self, plugs: Vec<(String, String)>) -> Result<HomeStatus, String> {
        let mut s = self.stored.lock().clone();
        if plugs.len() > 30 {
            return Err("Keep it to 30 plugs".into());
        }
        for (name, ip) in &plugs {
            if !valid_ip(ip) {
                return Err(format!("{ip} isn't a local network address"));
            }
            if name.trim().is_empty() || name.len() > 40 {
                return Err("Give each plug a short name".into());
            }
        }
        s.shelly = plugs.into_iter().map(|(n, ip)| (n.trim().to_string(), ip)).collect();
        self.put(s)?;
        Ok(self.status())
    }

    fn ha(&self) -> Result<(String, String), String> {
        let s = self.stored.lock().clone();
        if s.url.is_empty() || s.secret.is_empty() {
            return Err("Connect Home Assistant first".into());
        }
        let raw = base64::engine::general_purpose::STANDARD.decode(&s.secret).map_err(|_| "Connect Home Assistant again".to_string())?;
        let t = crate::system::dpapi::unprotect(&raw, ENTROPY).map_err(|_| "Connect Home Assistant again".to_string())?;
        Ok((s.url, String::from_utf8_lossy(&t).to_string()))
    }

    /// Everything that can be switched: Home Assistant's entities and the Shelly plugs.
    pub fn devices(&self) -> Result<Vec<HomeDevice>, String> {
        let mut out = Vec::new();
        if self.status().connected {
            let (url, token) = self.ha()?;
            let mut res = agent().get(&format!("{url}/api/states")).header("Authorization", &format!("Bearer {token}")).call().map_err(|_| "Home Assistant didn't answer".to_string())?;
            let text = res.body_mut().with_config().limit(16 << 20).read_to_string().map_err(|e| e.to_string())?;
            out.extend(parse_states(&serde_json::from_str(&text).unwrap_or(Value::Null)));
        }
        for (name, ip) in self.stored.lock().shelly.iter() {
            out.push(HomeDevice { id: format!("shelly:{ip}"), name: name.clone(), kind: "plug".into(), state: String::new() });
        }
        Ok(out)
    }

    /// Switch one device: "on", "off" or "toggle" (scenes and scripts just run).
    pub fn run(&self, device: &str, action: &str) -> Result<String, String> {
        if let Some(entity) = device.strip_prefix("ha:") {
            let (domain, svc) = service(entity, action)?;
            let (url, token) = self.ha()?;
            let res = agent().post(&format!("{url}/api/services/{domain}/{svc}")).header("Authorization", &format!("Bearer {token}")).header("Content-Type", "application/json").send(serde_json::json!({ "entity_id": entity }).to_string()).map_err(|_| "Home Assistant didn't answer".to_string())?;
            return match res.status().as_u16() {
                200 | 201 => Ok(format!("{entity}: {svc}")),
                401 => Err("Home Assistant didn't accept the token: connect it again".into()),
                n => Err(format!("Home Assistant answered {n}")),
            };
        }
        if let Some(ip) = device.strip_prefix("shelly:") {
            if !self.stored.lock().shelly.iter().any(|(_, p)| p == ip) {
                return Err("That plug isn't in your list".into());
            }
            for u in shelly_urls(ip, action)? {
                if agent().get(&u).call().is_ok_and(|r| r.status().is_success()) {
                    return Ok(format!("Plug {ip}: {action}"));
                }
            }
            return Err(format!("The plug at {ip} didn't answer"));
        }
        Err("Pick a device".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_switches() {
        assert_eq!(clean_url("http://homeassistant.local:8123/").unwrap(), "http://homeassistant.local:8123");
        assert!(clean_url("homeassistant.local").is_err() && clean_url("http://x/api?a").is_err() && clean_url("http://user@x").is_err());
        let v = serde_json::json!([
            {"entity_id": "light.desk", "state": "on", "attributes": {"friendly_name": "Desk lamp"}},
            {"entity_id": "sensor.temp", "state": "21"},
            {"entity_id": "scene.movie", "state": "2026-01-01", "attributes": {"friendly_name": "Movie"}},
        ]);
        let d = parse_states(&v);
        assert_eq!(d.iter().map(|x| (x.id.as_str(), x.state.as_str())).collect::<Vec<_>>(), vec![("ha:light.desk", "on"), ("ha:scene.movie", "")]);
        assert_eq!(service("light.desk", "toggle").unwrap(), ("light".into(), "toggle".into()));
        assert_eq!(service("scene.movie", "off").unwrap().1, "turn_on");
        assert!(service("lock.front_door", "off").is_err() && service("light.a/../b", "on").is_err());
        assert_eq!(shelly_urls("192.168.1.40", "on").unwrap()[0], "http://192.168.1.40/rpc/Switch.Set?id=0&on=true");
        assert!(valid_ip("192.168.1.40") && !valid_ip("8.8.8.8") && !valid_ip("x"));
        let t = tempfile::tempdir().unwrap();
        let h = Home::new(t.path());
        assert!(h.set_shelly(vec![("Desk".into(), "8.8.8.8".into())]).is_err());
        h.set_shelly(vec![("Desk".into(), "192.168.1.40".into())]).unwrap();
        assert_eq!(Home::new(t.path()).status().shelly.len(), 1);
        assert!(h.run("shelly:10.0.0.9", "on").unwrap_err().contains("isn't in your list"));
    }
}
