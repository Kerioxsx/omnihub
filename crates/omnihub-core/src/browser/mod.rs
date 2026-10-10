//! Browser autofill.
//!
//! The OmniHub extension for Brave, Chrome and Edge fills usernames, emails
//! and passwords from the vault. The browser starts the OmniHub executable
//! as its native-messaging host (only our extension ID may launch it,
//! [`host`]), which relays
//! length-prefixed JSON to the running app over a local pipe only this
//! Windows user can open ([`ipc`]). Requests are handled here:
//!
//! * A browser must be **paired** once: the extension asks, the desktop app
//!   shows the request with a short code, the user allows it, and the
//!   extension receives a token (stored hashed here, revocable).
//! * The vault must be unlocked in the app (or with Windows Hello from the
//!   extension). The extension never sees the master password.
//! * Logins are offered and filled **only for the site they belong to**, as
//!   decided here from the page URL the browser reports ([`matching`]), so a
//!   compromised page cannot ask for another site's password.

pub mod host;
pub mod ipc;
pub mod matching;
pub mod register;

use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::core::AppCore;
use crate::db::Db;
use crate::events::EventBus;
use crate::vault::{EntryInput, EntryKind};

/// Native-messaging host name (registered with the browsers).
pub const HOST_NAME: &str = "app.omnihub.vault";
/// ID of the OmniHub extension (fixed by the public key in its manifest).
pub const EXTENSION_ID: &str = "hfkbdbcemgoondnmkeeoclpcmcjjbdeg";
/// Largest message in either direction (Chrome's limit towards extensions).
pub const MAX_MESSAGE: usize = 1 << 20;
const PAIR_TIMEOUT: Duration = Duration::from_secs(120);

pub fn extension_origin() -> String {
    format!("chrome-extension://{EXTENSION_ID}/")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BrowserClient {
    pub id: String,
    pub name: String,
    pub created: i64,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PairRequest {
    pub id: String,
    /// What the extension says it is ("Brave on Windows").
    pub name: String,
    /// Shown in the extension and in the app, so the user can tell they match.
    pub code: String,
    pub created: i64,
}

struct Pending {
    req: PairRequest,
    /// `Some(answer)` once the user decided.
    decision: Option<bool>,
}

pub struct BrowserBridge {
    db: Arc<Db>,
    events: EventBus,
    pending: Mutex<Option<Pending>>,
    decided: parking_lot::Condvar,
    server: Mutex<Option<ipc::Server>>,
}

fn hash_token(t: &str) -> String {
    hex::encode(Sha256::digest(t.as_bytes()))
}

impl BrowserBridge {
    pub fn new(db: Arc<Db>, events: EventBus) -> Self {
        BrowserBridge { db, events, pending: Mutex::new(None), decided: parking_lot::Condvar::new(), server: Mutex::new(None) }
    }

    pub fn clients(&self) -> Vec<BrowserClient> {
        self.db
            .with(|c| {
                let mut st = c.prepare("SELECT id, name, created, last_seen FROM browser_clients WHERE revoked = 0 ORDER BY last_seen DESC")?;
                let rows = st.query_map([], |r| Ok(BrowserClient { id: r.get(0)?, name: r.get(1)?, created: r.get(2)?, last_seen: r.get(3)? }))?;
                rows.collect()
            })
            .unwrap_or_default()
    }

    pub fn revoke(&self, id: &str) -> bool {
        let n = self.db.with(|c| c.execute("UPDATE browser_clients SET revoked = 1 WHERE id = ?1", [id])).unwrap_or(0);
        self.events.emit("browser:clients", self.clients());
        n > 0
    }

    fn authenticate(&self, token: &str) -> Option<BrowserClient> {
        let h = hash_token(token);
        let client = self
            .db
            .with(|c| c.query_row("SELECT id, name, created, last_seen FROM browser_clients WHERE token_hash = ?1 AND revoked = 0", [&h], |r| Ok(BrowserClient { id: r.get(0)?, name: r.get(1)?, created: r.get(2)?, last_seen: r.get(3)? })))
            .ok()?;
        let _ = self.db.with(|c| c.execute("UPDATE browser_clients SET last_seen = ?1 WHERE id = ?2", rusqlite::params![crate::db::now(), client.id]));
        Some(client)
    }

    fn is_active(&self, id: &str) -> bool {
        self.db.with(|c| c.query_row("SELECT revoked FROM browser_clients WHERE id = ?1", [id], |r| r.get::<_, i64>(0))).is_ok_and(|r| r == 0)
    }

    fn add_client(&self, name: &str) -> (BrowserClient, String) {
        let mut raw = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut raw);
        let token = hex::encode(raw);
        let now = crate::db::now();
        let client = BrowserClient { id: uuid::Uuid::new_v4().to_string(), name: name.to_string(), created: now, last_seen: now };
        let _ = self.db.with(|c| c.execute("INSERT INTO browser_clients (id, name, token_hash, created, last_seen) VALUES (?1, ?2, ?3, ?4, ?4)", rusqlite::params![client.id, client.name, hash_token(&token), now]));
        self.events.emit("browser:clients", self.clients());
        (client, token)
    }

    pub fn pending(&self) -> Option<PairRequest> {
        self.pending.lock().as_ref().filter(|p| p.decision.is_none()).map(|p| p.req.clone())
    }

    /// The user's answer to a pairing request.
    pub fn respond(&self, request_id: &str, allow: bool) -> bool {
        let mut g = self.pending.lock();
        match g.as_mut() {
            Some(p) if p.req.id == request_id && p.decision.is_none() => {
                p.decision = Some(allow);
                self.decided.notify_all();
                self.events.emit("browser:pair-done", json!({ "id": request_id, "allowed": allow }));
                true
            }
            _ => false,
        }
    }

    /// Show a pairing request in the app; returns it (with its code) at once.
    fn start_pairing(&self, name: &str) -> Result<PairRequest, &'static str> {
        let mut g = self.pending.lock();
        if let Some(p) = g.as_ref() {
            let stale = crate::db::now() - p.req.created > PAIR_TIMEOUT.as_secs() as i64;
            if p.decision.is_none() && !stale {
                return Err("busy");
            }
        }
        let req = PairRequest { id: uuid::Uuid::new_v4().to_string(), name: name.chars().take(60).collect(), code: format!("{:04}", rand::random::<u16>() % 10_000), created: crate::db::now() };
        *g = Some(Pending { req: req.clone(), decision: None });
        drop(g);
        self.events.emit("browser:pair-request", &req);
        Ok(req)
    }

    /// Wait for the user's answer to `request_id` (up to two minutes).
    fn wait_pairing(&self, request_id: &str) -> Result<(BrowserClient, String), &'static str> {
        let deadline = std::time::Instant::now() + PAIR_TIMEOUT;
        let mut g = self.pending.lock();
        loop {
            match g.as_ref() {
                Some(p) if p.req.id == request_id => match p.decision {
                    Some(true) => {
                        let name = p.req.name.clone();
                        *g = None;
                        drop(g);
                        return Ok(self.add_client(&name));
                    }
                    Some(false) => {
                        *g = None;
                        return Err("denied");
                    }
                    None => {
                        if self.decided.wait_until(&mut g, deadline).timed_out() {
                            *g = None;
                            drop(g);
                            self.events.emit("browser:pair-done", json!({ "id": request_id, "allowed": false }));
                            return Err("timeout");
                        }
                    }
                },
                _ => return Err("unknown"),
            }
        }
    }

    pub fn is_listening(&self) -> bool {
        self.server.lock().is_some()
    }

    /// Start or stop listening for the browser host.
    pub fn set_listening(&self, core: &Arc<AppCore>, on: bool) -> std::io::Result<()> {
        let mut g = self.server.lock();
        if on && g.is_none() {
            *g = Some(ipc::Server::start(core.clone())?);
        } else if !on {
            if let Some(s) = g.take() {
                s.stop();
            }
        }
        Ok(())
    }
}

/// State of one connected extension.
#[derive(Default)]
pub struct Session {
    pub origin_ok: bool,
    pub client: Option<BrowserClient>,
}

fn err(code: &str, msg: &str) -> Value {
    json!({ "ok": false, "code": code, "error": msg })
}

fn vault_json(core: &AppCore) -> Value {
    let st = core.vault.status();
    json!({ "exists": st.exists, "unlocked": st.unlocked, "helloEnabled": st.hello_enabled })
}

fn str_field<'a>(req: &'a Value, k: &str) -> &'a str {
    req.get(k).and_then(|v| v.as_str()).unwrap_or("")
}

/// Handle one request. Returns `None` for messages that need no reply.
/// Answer one request. `id` in a request is the extension's correlation
/// number (echoed in the reply); vault entries are named by `entry`.
pub fn handle(core: &Arc<AppCore>, session: &Mutex<Session>, req: &Value) -> Option<Value> {
    let kind = str_field(req, "type");
    if kind == "host-hello" {
        session.lock().origin_ok = str_field(req, "origin") == extension_origin();
        return None;
    }
    let mut out = handle_inner(core, session, kind, req);
    if let (Some(id), Some(obj)) = (req.get("id"), out.as_object_mut()) {
        obj.insert("id".into(), id.clone());
    }
    Some(out)
}

fn handle_inner(core: &Arc<AppCore>, session: &Mutex<Session>, kind: &str, req: &Value) -> Value {
    if !session.lock().origin_ok {
        return err("unknown-extension", "This extension is not the OmniHub extension.");
    }
    let settings = core.settings.get().vault;
    if kind == "ping" {
        return json!({ "ok": true, "app": "OmniHub", "version": env!("CARGO_PKG_VERSION") });
    }
    if !settings.browser_autofill {
        return err("disabled", "Browser autofill is turned off in OmniHub (Vault → Browser autofill).");
    }
    let bridge = &core.browser;
    match kind {
        "hello" => {
            let client = req.get("token").and_then(|t| t.as_str()).and_then(|t| bridge.authenticate(t));
            let paired = client.is_some();
            session.lock().client = client;
            return json!({ "ok": true, "paired": paired, "vault": vault_json(core), "offerSave": settings.browser_offer_save, "version": env!("CARGO_PKG_VERSION") });
        }
        "pair" => {
            let name = str_field(req, "browser");
            return match bridge.start_pairing(if name.is_empty() { "A browser" } else { name }) {
                Ok(r) => json!({ "ok": true, "requestId": r.id, "code": r.code }),
                Err(_) => err("busy", "Another browser is waiting for approval in OmniHub."),
            };
        }
        "pair-wait" => {
            return match bridge.wait_pairing(str_field(req, "requestId")) {
                Ok((client, token)) => {
                    core.audit.record("desktop", "browser.pair", &client.name, true);
                    session.lock().client = Some(client.clone());
                    json!({ "ok": true, "token": token, "client": client, "vault": vault_json(core) })
                }
                Err("denied") => {
                    core.audit.record("desktop", "browser.pair", "", false);
                    err("denied", "Pairing was declined in OmniHub.")
                }
                Err("timeout") => err("timeout", "Nobody answered the pairing request in OmniHub."),
                Err(_) => err("unknown", "That pairing request is no longer open."),
            };
        }
        _ => {}
    }
    let client = {
        let g = session.lock();
        match &g.client {
            Some(c) => c.clone(),
            None => return err("not-paired", "Pair this browser with OmniHub first."),
        }
    };
    if !bridge.is_active(&client.id) {
        session.lock().client = None;
        return err("not-paired", "This browser's access was removed in OmniHub.");
    }
    match kind {
        "status" => json!({ "ok": true, "vault": vault_json(core) }),
        "unlock" => {
            let st = core.vault.status();
            if st.unlocked {
                return json!({ "ok": true, "vault": vault_json(core) });
            }
            if str_field(req, "method") == "hello" && st.hello_enabled {
                return match core.vault.unlock_with_hello() {
                    Ok(()) => {
                        core.audit.record(&format!("browser:{}", client.name), "vault.unlock-hello", "", true);
                        json!({ "ok": true, "vault": vault_json(core) })
                    }
                    Err(e) => err("hello-failed", &e.to_string()),
                };
            }
            core.events.emit("browser:unlock-request", json!({ "browser": client.name }));
            json!({ "ok": true, "pending": true, "vault": vault_json(core) })
        }
        "lock" => {
            core.vault.lock();
            json!({ "ok": true })
        }
        "generate" => {
            let length = req.get("length").and_then(|l| l.as_u64()).unwrap_or(20).clamp(8, 64) as usize;
            json!({ "ok": true, "password": crate::vault::generator::generate(&crate::vault::generator::GeneratorOptions { length, ..Default::default() }) })
        }
        _ => vault_request(core, &client, kind, req),
    }
}

/// Requests that need the vault unlocked.
fn vault_request(core: &Arc<AppCore>, client: &BrowserClient, kind: &str, req: &Value) -> Value {
    if !core.vault.is_unlocked() {
        return err("locked", "The vault is locked. Unlock it in OmniHub.");
    }
    core.vault.touch();
    let actor = format!("browser:{}", client.name);
    match kind {
        "match" => {
            let Some(page) = matching::parse_url(str_field(req, "url")) else { return json!({ "ok": true, "entries": [] }) };
            let Ok(list) = core.vault.list() else { return err("locked", "The vault is locked.") };
            let mut found: Vec<(matching::Quality, Value)> = list
                .iter()
                .filter(|e| matches!(e.kind, EntryKind::Login | EntryKind::Email | EntryKind::Other) && (e.has_password || !e.username.is_empty() || !e.email.is_empty()))
                .filter_map(|e| {
                    matching::entry_matches(&e.url, &e.email, &e.username, &page)
                        .map(|q| (q, json!({ "id": e.id, "title": e.title, "username": e.username, "email": e.email, "url": e.url, "hasPassword": e.has_password, "favorite": e.favorite, "match": q, "hasTotp": e.has_totp })))
                })
                .collect();
            found.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1["favorite"].as_bool().cmp(&a.1["favorite"].as_bool())));
            json!({ "ok": true, "entries": found.into_iter().map(|(_, v)| v).collect::<Vec<_>>(), "site": matching::site_of(&page.host) })
        }
        "fill" => {
            let Some(page) = matching::parse_url(str_field(req, "url")) else { return err("bad-url", "That page cannot be filled.") };
            let Ok(entry) = core.vault.get(str_field(req, "entry")) else { return err("not-found", "That entry no longer exists.") };
            if matching::entry_matches(&entry.url, &entry.email, &entry.username, &page).is_none() {
                core.audit.record(&actor, "browser.fill-refused", &page.host, false);
                return err("wrong-site", "That login belongs to another website.");
            }
            core.audit.record(&actor, "browser.fill", &format!("{} on {}", entry.title, page.host), true);
            let totp = crate::vault::totp::code_now(&entry.totp).ok();
            json!({ "ok": true, "username": entry.username, "email": entry.email, "password": entry.password, "totp": totp })
        }
        "search" => {
            let q = str_field(req, "q").to_lowercase();
            let Ok(list) = core.vault.list() else { return err("locked", "The vault is locked.") };
            let entries: Vec<Value> = list
                .iter()
                .filter(|e| q.is_empty() || e.title.to_lowercase().contains(&q) || e.url.to_lowercase().contains(&q) || e.username.to_lowercase().contains(&q) || e.email.to_lowercase().contains(&q))
                .take(50)
                .map(|e| json!({ "id": e.id, "title": e.title, "username": e.username, "email": e.email, "url": e.url, "hasPassword": e.has_password, "hasTotp": e.has_totp }))
                .collect();
            json!({ "ok": true, "entries": entries })
        }
        "copy" => {
            let field = str_field(req, "field");
            if !matches!(field, "username" | "email" | "password" | "totp") {
                return err("bad-field", "That field cannot be copied.");
            }
            let id = str_field(req, "entry");
            let clear = Duration::from_secs(core.settings.get().vault.clipboard_clear_seconds as u64);
            match core.vault.copy_field(id, field, clear) {
                Ok(()) => {
                    core.audit.record(&actor, "vault.copy", field, true);
                    json!({ "ok": true })
                }
                Err(e) => err("copy-failed", &e.to_string()),
            }
        }
        "check" => {
            // Is a login typed on a page new, already saved, or a changed password?
            let Some(page) = matching::parse_url(str_field(req, "url")) else { return json!({ "ok": true, "status": "ignore" }) };
            let login = str_field(req, "username").trim();
            let password = str_field(req, "password");
            let Ok(list) = core.vault.list() else { return err("locked", "The vault is locked.") };
            let candidates: Vec<_> = list
                .iter()
                .filter(|e| matching::entry_matches(&e.url, &e.email, &e.username, &page).is_some_and(|q| q >= matching::Quality::Related))
                .filter(|e| login.is_empty() || e.username.eq_ignore_ascii_case(login) || e.email.eq_ignore_ascii_case(login))
                .collect();
            for e in &candidates {
                if let Ok(full) = core.vault.get(&e.id) {
                    if full.password == password {
                        return json!({ "ok": true, "status": "same", "id": e.id });
                    }
                }
            }
            match candidates.first() {
                Some(e) if !login.is_empty() => json!({ "ok": true, "status": "changed", "id": e.id, "title": e.title }),
                _ => json!({ "ok": true, "status": "new", "site": matching::site_of(&page.host) }),
            }
        }
        "save" => {
            if !core.settings.get().vault.browser_offer_save {
                return err("disabled", "Saving from the browser is turned off in OmniHub.");
            }
            let Some(page) = matching::parse_url(str_field(req, "url")) else { return err("bad-url", "That page cannot be saved.") };
            let login = str_field(req, "username").trim().to_string();
            let password = str_field(req, "password").to_string();
            if password.is_empty() || password.len() > 1024 || login.len() > 256 {
                return err("bad-login", "Nothing to save.");
            }
            let Ok(list) = core.vault.list() else { return err("locked", "The vault is locked.") };
            let same = list.iter().find(|e| {
                matching::entry_matches(&e.url, &e.email, &e.username, &page).is_some_and(|q| q >= matching::Quality::Site)
                    // Without a login, only an entry that has none either is the same account.
                    && if login.is_empty() { e.username.is_empty() && e.email.is_empty() } else { e.username.eq_ignore_ascii_case(&login) || e.email.eq_ignore_ascii_case(&login) }
            });
            let site = matching::site_of(&page.host);
            let input = match same {
                Some(e) => EntryInput { id: Some(e.id.clone()), kind: e.kind, title: e.title.clone(), username: e.username.clone(), email: e.email.clone(), password: Some(password), url: e.url.clone(), notes: None, tags: e.tags.clone(), favorite: e.favorite, totp: None },
                None => {
                    let title = str_field(req, "title").trim();
                    EntryInput {
                        id: None,
                        kind: EntryKind::Login,
                        title: if title.is_empty() { site.clone() } else { title.chars().take(80).collect() },
                        username: login.clone(),
                        email: if login.contains('@') { login.clone() } else { String::new() },
                        password: Some(password),
                        url: match page.port {
                            Some(port) => format!("{}://{}:{port}", page.scheme, page.host),
                            None => format!("{}://{}", page.scheme, page.host),
                        },
                        notes: None,
                        tags: vec!["from-browser".into()],
                        favorite: false,
                        totp: None,
                    }
                }
            };
            let updated = input.id.is_some();
            match core.vault.save(input) {
                Ok(s) => {
                    core.audit.record(&actor, if updated { "browser.update" } else { "browser.save" }, &format!("{} ({site})", s.title), true);
                    json!({ "ok": true, "id": s.id, "updated": updated, "title": s.title })
                }
                Err(e) => err("save-failed", &e.to_string()),
            }
        }
        _ => err("unknown", "Unknown request."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::CoreOptions;
    use crate::paths::AppPaths;
    use crate::vault::crypto::KdfParams;

    fn core() -> (tempfile::TempDir, Arc<AppCore>) {
        let dir = tempfile::tempdir().unwrap();
        let core = AppCore::new(AppPaths::at(dir.path()).unwrap(), CoreOptions { vault_kdf: Some(KdfParams::testing()), vault_dpapi: Some(false), browser_integration: false, ..Default::default() }).unwrap();
        core.update_settings(&json!({ "vault": { "browserAutofill": true } })).unwrap();
        (dir, core)
    }

    fn call(core: &Arc<AppCore>, s: &Mutex<Session>, req: Value) -> Value {
        handle(core, s, &req).unwrap()
    }

    #[test]
    fn origin_pairing_and_site_rules() {
        let (_d, core) = core();
        let s = Mutex::new(Session::default());
        // Only our extension, announced by the host.
        assert_eq!(call(&core, &s, json!({ "id": 1, "type": "hello" }))["code"], "unknown-extension");
        assert!(handle(&core, &s, &json!({ "type": "host-hello", "origin": "chrome-extension://evil/" })).is_none());
        assert_eq!(call(&core, &s, json!({ "type": "hello" }))["code"], "unknown-extension");
        handle(&core, &s, &json!({ "type": "host-hello", "origin": extension_origin() }));
        let hello = call(&core, &s, json!({ "id": 2, "type": "hello", "browser": "Brave" }));
        assert_eq!((hello["id"].clone(), hello["paired"].clone()), (json!(2), json!(false)));
        assert_eq!(call(&core, &s, json!({ "type": "match", "url": "https://a.com" }))["code"], "not-paired");

        // Pairing: the code is shown at once, the token arrives once the user allows it.
        let start = call(&core, &s, json!({ "type": "pair", "browser": "Brave on Windows" }));
        let req_id = start["requestId"].as_str().unwrap().to_string();
        assert_eq!(start["code"].as_str().unwrap().len(), 4);
        assert_eq!(core.browser.pending().unwrap().code, start["code"].as_str().unwrap());
        assert_eq!(call(&core, &Mutex::new(Session { origin_ok: true, client: None }), json!({ "type": "pair", "browser": "Other" }))["code"], "busy");
        let c2 = core.clone();
        let rid = req_id.clone();
        let approver = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            assert!(c2.browser.respond(&rid, true));
        });
        let done = call(&core, &s, json!({ "type": "pair-wait", "requestId": req_id }));
        approver.join().unwrap();
        let token = done["token"].as_str().unwrap().to_string();
        assert_eq!(core.browser.clients().len(), 1);

        // Locked vault.
        assert_eq!(call(&core, &s, json!({ "type": "match", "url": "https://accounts.google.com/" }))["code"], "locked");
        core.vault.create("correct horse battery").unwrap();
        let g = core.vault.save(EntryInput { title: "Google".into(), email: "alex@gmail.com".into(), password: Some("g-secret".into()), totp: Some("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".into()), ..Default::default() }).unwrap();
        let gh = core.vault.save(EntryInput { title: "GitHub".into(), username: "alex".into(), url: "https://github.com".into(), password: Some("gh-secret".into()), ..Default::default() }).unwrap();

        // Matching and filling only for the right site.
        let m = call(&core, &s, json!({ "type": "match", "url": "https://accounts.google.com/v3/signin" }));
        let ids: Vec<&str> = m["entries"].as_array().unwrap().iter().map(|e| e["id"].as_str().unwrap()).collect();
        assert_eq!(ids, vec![g.id.as_str()]);
        assert!(m["entries"][0].get("password").is_none(), "listings never carry secrets");
        let fill = call(&core, &s, json!({ "type": "fill", "entry": g.id, "url": "https://accounts.google.com/" }));
        assert_eq!(fill["password"], "g-secret");
        assert_eq!(fill["totp"].as_str().unwrap().len(), 6);
        let refused = call(&core, &s, json!({ "type": "fill", "entry": gh.id, "url": "https://accounts.google.com/" }));
        assert_eq!(refused["code"], "wrong-site");
        assert_eq!(call(&core, &s, json!({ "type": "fill", "entry": gh.id, "url": "http://github.com/login" }))["code"], "wrong-site");

        // Checking what the user typed: same, changed or new.
        assert_eq!(call(&core, &s, json!({ "type": "check", "url": "https://github.com/session", "username": "alex", "password": "gh-secret" }))["status"], "same");
        assert_eq!(call(&core, &s, json!({ "type": "check", "url": "https://github.com/session", "username": "alex", "password": "new-one" }))["status"], "changed");
        assert_eq!(call(&core, &s, json!({ "type": "check", "url": "https://gitlab.com/users/sign_in", "username": "alex", "password": "x" }))["status"], "new");

        // Saving: a new login, then a changed password for the same account.
        let saved = call(&core, &s, json!({ "type": "save", "url": "https://www.example.com/login", "username": "alex@example.com", "password": "p1" }));
        assert_eq!(saved["updated"], false);
        let again = call(&core, &s, json!({ "type": "save", "url": "https://example.com/account", "username": "alex@example.com", "password": "p2" }));
        assert_eq!((again["updated"].clone(), again["id"].clone()), (json!(true), saved["id"].clone()));
        assert_eq!(core.vault.get(saved["id"].as_str().unwrap()).unwrap().password, "p2");
        // A password typed without a username never overwrites a saved account.
        let anon = call(&core, &s, json!({ "type": "save", "url": "https://example.com:8443/login", "username": "", "password": "p3" }));
        assert_eq!(anon["updated"], false);
        assert_eq!(core.vault.get(saved["id"].as_str().unwrap()).unwrap().password, "p2");
        assert_eq!(core.vault.get(anon["id"].as_str().unwrap()).unwrap().url, "https://example.com:8443");

        // A new connection with the token is paired; revoking ends it.
        let s2 = Mutex::new(Session { origin_ok: true, client: None });
        assert_eq!(call(&core, &s2, json!({ "type": "hello", "token": token }))["paired"], true);
        assert!(core.browser.revoke(&core.browser.clients()[0].id));
        assert_eq!(call(&core, &s2, json!({ "type": "match", "url": "https://github.com" }))["code"], "not-paired");
        assert_eq!(call(&core, &Mutex::new(Session { origin_ok: true, client: None }), json!({ "type": "hello", "token": token }))["paired"], false);

        // Turning autofill off refuses everything but ping.
        core.update_settings(&json!({ "vault": { "browserAutofill": false } })).unwrap();
        assert_eq!(call(&core, &s, json!({ "type": "match", "url": "https://github.com" }))["code"], "disabled");
        assert_eq!(call(&core, &s, json!({ "type": "ping" }))["ok"], true);
    }

    #[test]
    fn declined_pairing() {
        let (_d, core) = core();
        let s = Mutex::new(Session { origin_ok: true, client: None });
        let start = call(&core, &s, json!({ "type": "pair", "browser": "Chrome" }));
        let rid = start["requestId"].as_str().unwrap().to_string();
        assert!(!core.browser.respond("someone-else", true));
        assert!(core.browser.respond(&rid, false));
        assert_eq!(call(&core, &s, json!({ "type": "pair-wait", "requestId": rid }))["code"], "denied");
        assert!(core.browser.clients().is_empty());
        assert!(core.browser.pending().is_none());
    }

    /// The whole path on Linux: host binary protocol over the Unix socket.
    /// (Synchronous, so the core — which owns runtimes — is dropped outside async code.)
    #[cfg(unix)]
    #[test]
    fn socket_round_trip() {
        let (dir, core) = core();
        let sock = dir.path().join("b.sock");
        let server = ipc::Server::start_at(core.clone(), sock.to_string_lossy().into_owned()).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let reply: Value = rt.block_on(async {
            let mut c = tokio::net::UnixStream::connect(&sock).await.unwrap();
            ipc::write_frame(&mut c, json!({ "type": "host-hello", "origin": extension_origin() }).to_string().as_bytes()).await.unwrap();
            ipc::write_frame(&mut c, json!({ "id": 7, "type": "hello", "browser": "Test" }).to_string().as_bytes()).await.unwrap();
            serde_json::from_slice(&ipc::read_frame(&mut c).await.unwrap().unwrap()).unwrap()
        });
        assert_eq!((reply["id"].clone(), reply["ok"].clone(), reply["paired"].clone()), (json!(7), json!(true), json!(false)));
        server.stop();
    }
}
