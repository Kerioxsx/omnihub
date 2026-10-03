//! The phone companion server.
//!
//! An HTTPS (self-signed, or plain HTTP if the user chooses) server on the
//! local network that serves the phone web app and its API: file browsing
//! and transfer, power control, notes, the app list, an opt-in vault view
//! and screen sharing. Off by default; only private-network peers are
//! accepted; every request is authenticated with a paired-device token.

pub mod api;
pub mod assets;
pub mod auth;
pub mod net;
pub mod transfer;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::capture::stream::{MonitorInfo, StreamControl};
use crate::core::AppCore;
use crate::db::Db;
use crate::events::EventBus;
use crate::settings::Bind;
use crate::vault::Entry;
use auth::{Devices, Pairing, PairingInfo, RateLimiter, Tickets};
use transfer::{Inbox, InboxItem, Uploads};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub running: bool,
    pub port: u16,
    pub tls: bool,
    pub bind: Bind,
    pub urls: Vec<String>,
    pub fingerprint: Option<String>,
    pub error: Option<String>,
    pub viewers: Vec<ViewerInfo>,
    pub pairing_open: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ViewerInfo {
    pub id: String,
    pub device: String,
    pub monitor: Option<MonitorInfo>,
    pub since: i64,
    pub controlling: bool,
}

pub(crate) struct Viewer {
    pub info: ViewerInfo,
    pub ctl: Arc<StreamControl>,
}

pub(crate) struct VaultSession {
    pub device_id: String,
    pub entries: Vec<Entry>,
    pub last_used: Instant,
}

struct Running {
    handle: axum_server::Handle<SocketAddr>,
    port: u16,
    tls: bool,
    bind: Bind,
    fingerprint: Option<String>,
}

/// Idle time after which a phone's vault session ends.
pub const VAULT_SESSION_IDLE: Duration = Duration::from_secs(5 * 60);

pub struct RemoteServer {
    rt: tokio::runtime::Runtime,
    running: Mutex<Option<Running>>,
    last_error: Mutex<Option<String>>,
    pub devices: Devices,
    pub pairing: Pairing,
    pub tickets: Tickets,
    pub inbox: Inbox,
    pub(crate) uploads: Mutex<Option<Arc<Uploads>>>,
    pub(crate) pair_limiter: RateLimiter,
    pub(crate) vault_limiter: RateLimiter,
    pub(crate) viewers: Mutex<HashMap<String, Viewer>>,
    pub(crate) vault_sessions: Mutex<HashMap<String, VaultSession>>,
    events: EventBus,
}

impl RemoteServer {
    pub fn new(db: Arc<Db>, events: EventBus) -> Self {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .thread_name("omnihub-remote")
            .enable_all()
            .build()
            .expect("tokio runtime");
        RemoteServer {
            rt,
            running: Mutex::new(None),
            last_error: Mutex::new(None),
            devices: Devices::new(db),
            pairing: Pairing::default(),
            tickets: Tickets::default(),
            inbox: Inbox::default(),
            uploads: Mutex::new(None),
            pair_limiter: RateLimiter::new(10, Duration::from_secs(60)),
            vault_limiter: RateLimiter::new(5, Duration::from_secs(60)),
            viewers: Mutex::new(HashMap::new()),
            vault_sessions: Mutex::new(HashMap::new()),
            events,
        }
    }

    pub fn runtime(&self) -> &tokio::runtime::Runtime {
        &self.rt
    }

    pub(crate) fn uploads(&self, core: &AppCore) -> Arc<Uploads> {
        let mut g = self.uploads.lock();
        g.get_or_insert_with(|| Arc::new(Uploads::new(&core.incoming_dir(), self.events.clone()))).clone()
    }

    pub fn is_running(&self) -> bool {
        self.running.lock().is_some()
    }

    /// Start listening with the current settings.
    pub fn start(&self, core: Arc<AppCore>) -> anyhow::Result<ServerStatus> {
        self.stop();
        let s = core.settings.get().remote;
        let ip = match s.bind {
            Bind::Lan => net::unspecified(),
            Bind::Localhost => net::localhost(),
        };
        let addr = SocketAddr::new(ip, s.port);
        // Bind synchronously so "port in use" is reported to the caller.
        let listener = match std::net::TcpListener::bind(addr).or_else(|e| {
            // Dual-stack [::] may be unavailable; fall back to IPv4.
            if s.bind == Bind::Lan {
                std::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], s.port)))
            } else {
                Err(e)
            }
        }) {
            Ok(l) => l,
            Err(e) => {
                let msg = format!("cannot listen on port {}: {e}", s.port);
                *self.last_error.lock() = Some(msg.clone());
                anyhow::bail!(msg);
            }
        };
        listener.set_nonblocking(true)?;
        // Converting the listener needs the runtime's reactor.
        let _rt = self.rt.enter();
        *self.uploads.lock() = Some(Arc::new(Uploads::new(&core.incoming_dir(), self.events.clone())));
        if let Some(u) = self.uploads.lock().as_ref() {
            u.cleanup_stale(7);
        }
        let app = api::router(core.clone());
        let handle = axum_server::Handle::new();
        let hostname = gethostname::gethostname().to_string_lossy().to_string();
        let mut fingerprint = None;
        let server_handle = handle.clone();
        if s.tls {
            let _ = rustls::crypto::ring::default_provider().install_default();
            let (cert, key) = net::load_or_create_cert(&core.paths.config, &hostname)?;
            fingerprint = Some(net::fingerprint(&cert));
            // Built synchronously: `start` may be called from inside another
            // async runtime (the desktop shell's), where block_on panics.
            let config = axum_server::tls_rustls::RustlsConfig::from_config(Arc::new(net::server_config(&cert, &key)?));
            let server = axum_server::from_tcp_rustls(listener, config)?.handle(server_handle);
            self.rt.spawn(async move {
                if let Err(e) = server.serve(app.into_make_service_with_connect_info::<SocketAddr>()).await {
                    tracing::error!("companion server stopped: {e}");
                }
            });
        } else {
            let server = axum_server::from_tcp(listener)?.handle(server_handle);
            self.rt.spawn(async move {
                if let Err(e) = server.serve(app.into_make_service_with_connect_info::<SocketAddr>()).await {
                    tracing::error!("companion server stopped: {e}");
                }
            });
        }
        *self.running.lock() = Some(Running { handle, port: s.port, tls: s.tls, bind: s.bind, fingerprint });
        *self.last_error.lock() = None;
        core.audit.record("desktop", "remote.start", &format!("port {} ({})", s.port, if s.tls { "https" } else { "http" }), true);
        let status = self.status();
        self.events.emit("remote:status", &status);
        Ok(status)
    }

    pub fn stop(&self) {
        // Take the handle first: `status()` below locks `running` again.
        let running = self.running.lock().take();
        if let Some(r) = running {
            r.handle.graceful_shutdown(Some(Duration::from_secs(2)));
            for v in self.viewers.lock().values() {
                v.ctl.stop.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            self.vault_sessions.lock().clear();
            self.pairing.close();
            self.events.emit("remote:status", self.status());
        }
    }

    pub fn status(&self) -> ServerStatus {
        let r = self.running.lock();
        let viewers = self.viewers.lock().values().map(|v| v.info.clone()).collect();
        match r.as_ref() {
            Some(r) => {
                let scheme = if r.tls { "https" } else { "http" };
                let urls = match r.bind {
                    Bind::Localhost => vec![format!("{scheme}://localhost:{}", r.port)],
                    Bind::Lan => net::lan_addresses().into_iter().map(|a| format!("{scheme}://{}:{}", a.ip, r.port)).collect(),
                };
                ServerStatus { running: true, port: r.port, tls: r.tls, bind: r.bind, urls, fingerprint: r.fingerprint.clone(), error: None, viewers, pairing_open: self.pairing.is_open() }
            }
            None => ServerStatus { running: false, port: 0, tls: false, bind: Bind::Lan, urls: vec![], fingerprint: None, error: self.last_error.lock().clone(), viewers, pairing_open: false },
        }
    }

    /// Open the pairing window and describe how a phone can join.
    pub fn begin_pairing(&self) -> anyhow::Result<PairingInfo> {
        let status = self.status();
        if !status.running {
            anyhow::bail!("turn on the phone companion first");
        }
        let (secret, pin, expires) = self.pairing.open();
        let urls: Vec<String> = status.urls.iter().map(|u| format!("{u}/#pair={secret}")).collect();
        let qr = urls.first().map(|u| net::qr_svg(u)).unwrap_or_default();
        let expires_at = crate::db::now() + expires.saturating_duration_since(Instant::now()).as_secs() as i64;
        Ok(PairingInfo { pin, secret, expires_at, urls, qr_svg: qr, fingerprint: status.fingerprint })
    }

    pub fn cancel_pairing(&self) {
        self.pairing.close();
    }

    pub fn send_to_phone(&self, paths: &[String], device_id: Option<String>) -> anyhow::Result<Vec<InboxItem>> {
        let mut out = Vec::new();
        for p in paths {
            let item = self.inbox.offer(std::path::Path::new(p), device_id.clone())?;
            self.events.emit("inbox:new", serde_json::json!({ "item": &item, "deviceId": &item.device_id }));
            out.push(item);
        }
        Ok(out)
    }

    pub fn stop_viewer(&self, id: &str) {
        if let Some(v) = self.viewers.lock().get(id) {
            v.ctl.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub fn stop_all_viewers(&self) {
        for v in self.viewers.lock().values() {
            v.ctl.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    pub fn expire_vault_sessions(&self) {
        self.vault_sessions.lock().retain(|_, s| s.last_used.elapsed() < VAULT_SESSION_IDLE);
    }

    pub fn revoke_device(&self, id: &str) -> bool {
        self.vault_sessions.lock().retain(|_, s| s.device_id != id);
        let ok = self.devices.revoke(id);
        self.events.emit("remote:devices", self.devices.list());
        ok
    }
}

impl Drop for RemoteServer {
    fn drop(&mut self) {
        self.stop();
    }
}
