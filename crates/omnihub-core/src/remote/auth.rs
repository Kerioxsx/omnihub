//! Pairing, device tokens, short-lived tickets and rate limiting.
//!
//! Pairing needs physical access to the PC: the desktop app opens a pairing
//! window that shows a QR code (with a 128-bit secret) and a 6-digit PIN.
//! A successful pairing returns a 256-bit device token; only its SHA-256 is
//! stored. Tokens can be revoked from the desktop at any time.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine;
use parking_lot::Mutex;
use rand::Rng;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::db::{now, Db};

pub const PAIRING_WINDOW: Duration = Duration::from_secs(5 * 60);
const MAX_PIN_ATTEMPTS: u32 = 5;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub user_agent: String,
    pub created: i64,
    pub last_seen: i64,
    pub last_ip: String,
    pub revoked: bool,
}

pub fn random_token(bytes: usize) -> String {
    let mut b = vec![0u8; bytes];
    rand::rngs::OsRng.fill(&mut b[..]);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Constant-time comparison for secrets of equal length.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub struct Devices {
    db: Arc<Db>,
    last_touch: Mutex<HashMap<String, Instant>>,
}

impl Devices {
    pub fn new(db: Arc<Db>) -> Self {
        Devices { db, last_touch: Mutex::new(HashMap::new()) }
    }

    fn row(r: &rusqlite::Row) -> rusqlite::Result<Device> {
        Ok(Device { id: r.get(0)?, name: r.get(1)?, user_agent: r.get(2)?, created: r.get(3)?, last_seen: r.get(4)?, last_ip: r.get(5)?, revoked: r.get(6)? })
    }

    pub fn create(&self, name: &str, user_agent: &str, ip: &str) -> rusqlite::Result<(Device, String)> {
        let token = random_token(32);
        let id = uuid::Uuid::new_v4().to_string();
        let t = now();
        let name: String = name.trim().chars().take(60).collect();
        let name = if name.is_empty() { "Phone".to_string() } else { name };
        let ua: String = user_agent.chars().take(300).collect();
        self.db.with(|c| {
            c.execute(
                "INSERT INTO devices (id, name, token_hash, user_agent, created, last_seen, last_ip) VALUES (?1,?2,?3,?4,?5,?5,?6)",
                params![id, name, hash_token(&token), ua, t, ip],
            )
        })?;
        Ok((self.get(&id)?.expect("just inserted"), token))
    }

    pub fn get(&self, id: &str) -> rusqlite::Result<Option<Device>> {
        self.db.with(|c| {
            c.query_row("SELECT id, name, user_agent, created, last_seen, last_ip, revoked FROM devices WHERE id=?1", params![id], Self::row).optional()
        })
    }

    /// The device a bearer token belongs to, if it is valid.
    pub fn authenticate(&self, token: &str, ip: &str) -> Option<Device> {
        if token.len() < 20 || token.len() > 100 {
            return None;
        }
        let hash = hash_token(token);
        let dev = self
            .db
            .with(|c| {
                c.query_row(
                    "SELECT id, name, user_agent, created, last_seen, last_ip, revoked FROM devices WHERE token_hash=?1",
                    params![hash],
                    Self::row,
                )
                .optional()
            })
            .ok()??;
        if dev.revoked {
            return None;
        }
        // Record activity at most every 30 s.
        let mut touched = self.last_touch.lock();
        let due = touched.get(&dev.id).is_none_or(|t| t.elapsed() > Duration::from_secs(30));
        if due {
            touched.insert(dev.id.clone(), Instant::now());
            let _ = self.db.with(|c| c.execute("UPDATE devices SET last_seen=?2, last_ip=?3 WHERE id=?1", params![dev.id, now(), ip]));
        }
        Some(dev)
    }

    pub fn list(&self) -> Vec<Device> {
        self.db
            .with(|c| {
                let mut st = c.prepare("SELECT id, name, user_agent, created, last_seen, last_ip, revoked FROM devices WHERE revoked=0 ORDER BY last_seen DESC")?;
                let rows = st.query_map([], Self::row)?;
                rows.collect()
            })
            .unwrap_or_default()
    }

    pub fn rename(&self, id: &str, name: &str) -> bool {
        let name: String = name.trim().chars().take(60).collect();
        !name.is_empty() && self.db.with(|c| c.execute("UPDATE devices SET name=?2 WHERE id=?1", params![id, name])).unwrap_or(0) > 0
    }

    pub fn revoke(&self, id: &str) -> bool {
        self.db.with(|c| c.execute("UPDATE devices SET revoked=1 WHERE id=?1", params![id])).unwrap_or(0) > 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PairingInfo {
    pub pin: String,
    pub secret: String,
    pub expires_at: i64,
    pub urls: Vec<String>,
    pub qr_svg: String,
    pub fingerprint: Option<String>,
}

struct Window {
    secret: String,
    pin: String,
    expires: Instant,
    attempts: u32,
}

#[derive(Default)]
pub struct Pairing {
    window: Mutex<Option<Window>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PairCheck {
    Ok,
    Closed,
    Wrong,
}

impl Pairing {
    /// Open (or reopen) the pairing window with a fresh secret and PIN.
    pub fn open(&self) -> (String, String, Instant) {
        let secret = random_token(16);
        let pin = format!("{:06}", rand::rngs::OsRng.gen_range(0..1_000_000));
        let expires = Instant::now() + PAIRING_WINDOW;
        *self.window.lock() = Some(Window { secret: secret.clone(), pin: pin.clone(), expires, attempts: 0 });
        (secret, pin, expires)
    }

    pub fn close(&self) {
        *self.window.lock() = None;
    }

    pub fn is_open(&self) -> bool {
        self.window.lock().as_ref().is_some_and(|w| w.expires > Instant::now())
    }

    /// Check a QR secret or a PIN. A correct one closes the window (one
    /// device per pairing); too many wrong PINs close it too.
    pub fn check(&self, secret: Option<&str>, pin: Option<&str>) -> PairCheck {
        let mut guard = self.window.lock();
        let Some(w) = guard.as_mut() else { return PairCheck::Closed };
        if w.expires <= Instant::now() {
            *guard = None;
            return PairCheck::Closed;
        }
        let ok = match (secret, pin) {
            (Some(s), _) if !s.is_empty() => ct_eq(s.as_bytes(), w.secret.as_bytes()),
            (_, Some(p)) => ct_eq(p.trim().as_bytes(), w.pin.as_bytes()),
            _ => false,
        };
        if ok {
            *guard = None;
            return PairCheck::Ok;
        }
        w.attempts += 1;
        if w.attempts >= MAX_PIN_ATTEMPTS {
            *guard = None;
        }
        PairCheck::Wrong
    }
}

/// Single-purpose short-lived tickets (WebSocket upgrades, download links).
#[derive(Default)]
pub struct Tickets {
    map: Mutex<HashMap<String, Ticket>>,
}

#[derive(Debug, Clone)]
pub struct Ticket {
    pub device_id: String,
    pub purpose: TicketPurpose,
    pub expires: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketPurpose {
    Socket,
    Screen,
    Download(std::path::PathBuf),
}

impl Tickets {
    pub fn issue(&self, device_id: &str, purpose: TicketPurpose, ttl: Duration) -> String {
        let t = random_token(24);
        let mut map = self.map.lock();
        let now = Instant::now();
        map.retain(|_, v| v.expires > now);
        map.insert(t.clone(), Ticket { device_id: device_id.to_string(), purpose, expires: now + ttl });
        t
    }

    /// Socket tickets are single use; download tickets stay valid until
    /// they expire so interrupted downloads can resume with Range requests.
    pub fn redeem(&self, ticket: &str, socket: bool) -> Option<Ticket> {
        let mut map = self.map.lock();
        let t = map.get(ticket)?.clone();
        if t.expires <= Instant::now() {
            map.remove(ticket);
            return None;
        }
        if socket {
            map.remove(ticket);
        }
        Some(t)
    }
}

/// Fixed-window rate limiter per key.
pub struct RateLimiter {
    max: u32,
    window: Duration,
    hits: Mutex<HashMap<IpAddr, (Instant, u32)>>,
}

impl RateLimiter {
    pub fn new(max: u32, window: Duration) -> Self {
        RateLimiter { max, window, hits: Mutex::new(HashMap::new()) }
    }

    pub fn allow(&self, key: IpAddr) -> bool {
        let mut hits = self.hits.lock();
        let now = Instant::now();
        if hits.len() > 10_000 {
            hits.retain(|_, (start, _)| now.duration_since(*start) < self.window);
        }
        let e = hits.entry(key).or_insert((now, 0));
        if now.duration_since(e.0) >= self.window {
            *e = (now, 0);
        }
        e.1 += 1;
        e.1 <= self.max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devices_and_tokens() {
        let d = Devices::new(Arc::new(Db::in_memory().unwrap()));
        let (dev, token) = d.create("  Pixel 9  ", "Mozilla/5.0", "192.168.1.5").unwrap();
        assert_eq!(dev.name, "Pixel 9");
        assert_eq!(d.authenticate(&token, "192.168.1.5").unwrap().id, dev.id);
        assert!(d.authenticate("wrong-token-wrong-token", "x").is_none());
        assert!(d.rename(&dev.id, "Work phone"));
        assert_eq!(d.list()[0].name, "Work phone");
        assert!(d.revoke(&dev.id));
        assert!(d.authenticate(&token, "x").is_none());
        assert!(d.list().is_empty());
    }

    #[test]
    fn pairing_window() {
        let p = Pairing::default();
        assert_eq!(p.check(None, Some("123456")), PairCheck::Closed);
        let (secret, pin, _) = p.open();
        assert_eq!(pin.len(), 6);
        assert_eq!(p.check(Some("nope"), None), PairCheck::Wrong);
        assert_eq!(p.check(Some(&secret), None), PairCheck::Ok);
        assert_eq!(p.check(Some(&secret), None), PairCheck::Closed, "one device per window");

        let (_, pin, _) = p.open();
        let wrong = if pin == "000000" { "111111" } else { "000000" };
        for _ in 0..MAX_PIN_ATTEMPTS {
            assert_eq!(p.check(None, Some(wrong)), PairCheck::Wrong);
        }
        assert_eq!(p.check(None, Some(&pin)), PairCheck::Closed, "locked after too many attempts");
        let (_, pin, _) = p.open();
        assert_eq!(p.check(None, Some(&format!(" {pin} "))), PairCheck::Ok);
    }

    #[test]
    fn tickets_and_limits() {
        let t = Tickets::default();
        let s = t.issue("dev", TicketPurpose::Socket, Duration::from_secs(5));
        assert!(t.redeem(&s, true).is_some());
        assert!(t.redeem(&s, true).is_none(), "socket tickets are single use");
        let d = t.issue("dev", TicketPurpose::Download("/x".into()), Duration::from_secs(5));
        assert!(t.redeem(&d, false).is_some());
        assert!(t.redeem(&d, false).is_some(), "download tickets allow resume");
        let e = t.issue("dev", TicketPurpose::Socket, Duration::from_millis(0));
        assert!(t.redeem(&e, true).is_none());

        let r = RateLimiter::new(3, Duration::from_secs(60));
        let ip: IpAddr = "192.168.1.9".parse().unwrap();
        assert!(r.allow(ip) && r.allow(ip) && r.allow(ip));
        assert!(!r.allow(ip));
        assert!(r.allow("192.168.1.10".parse().unwrap()));
        assert!(ct_eq(b"abc", b"abc") && !ct_eq(b"abc", b"abd") && !ct_eq(b"ab", b"abc"));
    }
}
