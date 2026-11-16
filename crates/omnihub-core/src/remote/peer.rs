//! Control another PC: this OmniHub pairs with the companion server of
//! OmniHub on a second PC, the same way a phone does (its PIN from "Pair a
//! phone" there), then reads its status and locks, sleeps, restarts or
//! shuts it down, and controls its music. The other PC's certificate is
//! remembered by its SHA-256 fingerprint when pairing (shown on both PCs to
//! compare), and every later connection must present the same one. The
//! token it gives is kept encrypted for this Windows account.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const ENTROPY: &[u8] = b"omnihub-peer-pc";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PeerPc {
    pub id: String,
    pub name: String,
    /// "https://192.168.1.20:7878"
    pub url: String,
    /// The certificate's SHA-256, "AA:BB:…" (empty for plain http).
    pub fingerprint: String,
    #[serde(skip_serializing)]
    secret: String,
}

/// (https, host, port) from an address like "192.168.1.20:7878" or a full URL.
pub fn parse_url(input: &str) -> Result<(bool, String, u16), String> {
    let t = input.trim().trim_end_matches('/');
    let (tls, rest) = if let Some(r) = t.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = t.strip_prefix("http://") {
        (false, r)
    } else {
        (true, t)
    };
    if rest.contains('/') || rest.is_empty() {
        return Err("Type the other PC's address, like 192.168.1.20:7878".into());
    }
    let (host, port) = match rest.rsplit_once(':') {
        Some((h, p)) if !h.ends_with(']') || h.starts_with('[') => (h.trim_matches(['[', ']']).to_string(), p.parse::<u16>().map_err(|_| "That port isn't a number".to_string())?),
        _ => (rest.trim_matches(['[', ']']).to_string(), 7878),
    };
    if host.is_empty() || !host.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':')) {
        return Err("That isn't an address".into());
    }
    Ok((tls, host, port))
}

/// The SHA-256 of a certificate, as OmniHub shows it.
pub fn fingerprint(der: &[u8]) -> String {
    Sha256::digest(der).iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
}

/// Split an HTTP response into (status, body), undoing chunked encoding.
pub fn parse_response(raw: &[u8]) -> Result<(u16, String), String> {
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or("The other PC sent something unexpected")?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let mut body = raw[split + 4..].to_vec();
    let status = head.split_whitespace().nth(1).and_then(|s| s.parse().ok()).ok_or("The other PC sent something unexpected")?;
    if head.lines().any(|l| l.to_ascii_lowercase().starts_with("transfer-encoding:") && l.to_ascii_lowercase().contains("chunked")) {
        let mut out = Vec::new();
        let mut rest = &body[..];
        while let Some(nl) = rest.windows(2).position(|w| w == b"\r\n") {
            let size = usize::from_str_radix(String::from_utf8_lossy(&rest[..nl]).split(';').next().unwrap_or("").trim(), 16).unwrap_or(0);
            rest = &rest[nl + 2..];
            if size == 0 || rest.len() < size {
                break;
            }
            out.extend_from_slice(&rest[..size]);
            rest = rest.get(size + 2..).unwrap_or(&[]);
        }
        body = out;
    }
    Ok((status, String::from_utf8_lossy(&body).to_string()))
}

#[derive(Debug)]
struct Pinned {
    want: Option<String>,
    seen: Arc<Mutex<Option<String>>>,
    provider: Arc<rustls::crypto::CryptoProvider>,
}

impl rustls::client::danger::ServerCertVerifier for Pinned {
    fn verify_server_cert(&self, end_entity: &rustls::pki_types::CertificateDer<'_>, _intermediates: &[rustls::pki_types::CertificateDer<'_>], _name: &rustls::pki_types::ServerName<'_>, _ocsp: &[u8], _now: rustls::pki_types::UnixTime) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        let fp = fingerprint(end_entity.as_ref());
        *self.seen.lock() = Some(fp.clone());
        match &self.want {
            Some(w) if *w != fp => Err(rustls::Error::General("the other PC's certificate changed".into())),
            _ => Ok(rustls::client::danger::ServerCertVerified::assertion()),
        }
    }

    fn verify_tls12_signature(&self, message: &[u8], cert: &rustls::pki_types::CertificateDer<'_>, dss: &rustls::DigitallySignedStruct) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn verify_tls13_signature(&self, message: &[u8], cert: &rustls::pki_types::CertificateDer<'_>, dss: &rustls::DigitallySignedStruct) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}

/// One request to the other PC; returns (status, body, the certificate's fingerprint).
fn request(url: &str, pin: Option<&str>, method: &str, path: &str, token: Option<&str>, body: Option<&serde_json::Value>) -> Result<(u16, String, Option<String>), String> {
    let (tls, host, port) = parse_url(url)?;
    let addr = (host.as_str(), port).to_socket_addrs().map_err(|_| "That address can't be found".to_string())?.next().ok_or("That address can't be found")?;
    let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(5)).map_err(|_| "The other PC didn't answer (is OmniHub's companion on there?)".to_string())?;
    tcp.set_read_timeout(Some(Duration::from_secs(15))).ok();
    tcp.set_write_timeout(Some(Duration::from_secs(10))).ok();
    let payload = body.map(|b| b.to_string()).unwrap_or_default();
    let mut req = format!("{method} {path} HTTP/1.1\r\nHost: {host}:{port}\r\nUser-Agent: OmniHub-PC\r\nAccept: application/json\r\nConnection: close\r\n");
    if let Some(t) = token {
        req.push_str(&format!("Authorization: Bearer {t}\r\n"));
    }
    if body.is_some() {
        req.push_str(&format!("Content-Type: application/json\r\nContent-Length: {}\r\n", payload.len()));
    }
    req.push_str("\r\n");
    req.push_str(&payload);
    let mut raw = Vec::new();
    let mut seen_fp = None;
    if tls {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let seen = Arc::new(Mutex::new(None));
        let verifier = Pinned { want: pin.map(String::from), seen: seen.clone(), provider: provider.clone() };
        let config = rustls::ClientConfig::builder_with_provider(provider).with_safe_default_protocol_versions().map_err(|e| e.to_string())?.dangerous().with_custom_certificate_verifier(Arc::new(verifier)).with_no_client_auth();
        let name = rustls::pki_types::ServerName::try_from(host.clone()).map_err(|_| "That isn't an address".to_string())?;
        let conn = rustls::ClientConnection::new(Arc::new(config), name).map_err(|e| e.to_string())?;
        let mut s = rustls::StreamOwned::new(conn, tcp);
        let r = s.write_all(req.as_bytes()).and_then(|_| s.flush());
        seen_fp = seen.lock().clone();
        r.map_err(|e| if e.to_string().contains("certificate changed") { "The other PC's certificate changed since pairing: pair it again (and compare the fingerprint)".to_string() } else { format!("Couldn't talk to the other PC: {e}") })?;
        // Servers may close without TLS's goodbye; what arrived counts.
        let mut buf = [0u8; 16 * 1024];
        loop {
            match s.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => raw.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof && !raw.is_empty() => break,
                Err(e) => return Err(format!("Couldn't read the other PC's answer: {e}")),
            }
            if raw.len() > 4 << 20 {
                return Err("The other PC's answer was too big".into());
            }
        }
    } else {
        let mut s = tcp;
        s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
        s.take(4 << 20).read_to_end(&mut raw).map_err(|e| e.to_string())?;
    }
    let (status, text) = parse_response(&raw)?;
    Ok((status, text, seen_fp))
}

fn error_of(status: u16, body: &str) -> String {
    let msg = serde_json::from_str::<serde_json::Value>(body).ok().and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from)).unwrap_or_default();
    match status {
        401 => "The other PC doesn't know this one any more: pair it again".into(),
        _ if !msg.is_empty() => format!("The other PC said: {msg}"),
        s => format!("The other PC answered {s}"),
    }
}

pub struct Peers {
    file: PathBuf,
    list: Mutex<Vec<PeerPc>>,
}

impl Peers {
    pub fn new(data: &Path) -> Peers {
        let file = data.join("other-pcs.json");
        let list = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice::<Vec<Stored>>(&b).ok()).map(|v| v.into_iter().map(Into::into).collect()).unwrap_or_default();
        Peers { file, list: Mutex::new(list) }
    }

    pub fn list(&self) -> Vec<PeerPc> {
        self.list.lock().clone()
    }

    fn store(&self, l: &[PeerPc]) -> Result<(), String> {
        let stored: Vec<Stored> = l.iter().map(|p| Stored { id: p.id.clone(), name: p.name.clone(), url: p.url.clone(), fingerprint: p.fingerprint.clone(), secret: p.secret.clone() }).collect();
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&stored).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    /// Pair with the PIN the other PC shows under "Pair a phone".
    pub fn pair(&self, url: &str, pin: &str, this_name: &str) -> Result<PeerPc, String> {
        let (tls, host, port) = parse_url(url)?;
        let url = format!("{}://{host}:{port}", if tls { "https" } else { "http" });
        let pin = pin.trim();
        if pin.is_empty() || pin.len() > 32 {
            return Err("Type the PIN the other PC shows".into());
        }
        let (status, body, fp) = request(&url, None, "POST", "/api/pair", None, Some(&serde_json::json!({ "pin": pin, "deviceName": format!("{this_name} (PC)") })))?;
        if status != 200 {
            return Err(error_of(status, &body));
        }
        let v: serde_json::Value = serde_json::from_str(&body).map_err(|_| "The other PC sent something unexpected".to_string())?;
        let token = v.get("token").and_then(|t| t.as_str()).ok_or("The other PC didn't send a token")?;
        let secret = base64::engine::general_purpose::STANDARD.encode(crate::system::dpapi::protect(token.as_bytes(), ENTROPY).map_err(|e| e.to_string())?);
        let name = v.get("serverName").and_then(|n| n.as_str()).filter(|n| !n.is_empty()).unwrap_or(&host).to_string();
        let pc = PeerPc { id: uuid::Uuid::new_v4().simple().to_string()[..10].to_string(), name, url, fingerprint: fp.unwrap_or_default(), secret };
        let mut l = self.list.lock();
        l.retain(|p| p.url != pc.url);
        l.push(pc.clone());
        self.store(&l)?;
        Ok(pc)
    }

    pub fn forget(&self, id: &str) -> Result<Vec<PeerPc>, String> {
        let mut l = self.list.lock();
        l.retain(|p| p.id != id);
        self.store(&l)?;
        Ok(l.clone())
    }

    /// Call the other PC's API (as a paired device).
    pub fn call(&self, id: &str, method: &str, path: &str, body: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
        let pc = self.list.lock().iter().find(|p| p.id == id).cloned().ok_or("That PC isn't paired")?;
        let raw = base64::engine::general_purpose::STANDARD.decode(&pc.secret).map_err(|_| "Pair that PC again".to_string())?;
        let token = crate::system::dpapi::unprotect(&raw, ENTROPY).map_err(|_| "Pair that PC again".to_string())?;
        let token = String::from_utf8_lossy(&token).to_string();
        let pin = (!pc.fingerprint.is_empty()).then_some(pc.fingerprint.as_str());
        let (status, text, _) = request(&pc.url, pin, method, path, Some(&token), body.as_ref())?;
        if !(200..300).contains(&status) {
            return Err(error_of(status, &text));
        }
        Ok(serde_json::from_str(&text).unwrap_or(serde_json::Value::Null))
    }
}

/// The file's shape (the secret is written, never sent to the app).
#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Stored {
    id: String,
    name: String,
    url: String,
    fingerprint: String,
    secret: String,
}

impl From<Stored> for PeerPc {
    fn from(s: Stored) -> Self {
        PeerPc { id: s.id, name: s.name, url: s.url, fingerprint: s.fingerprint, secret: s.secret }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_and_answers() {
        assert_eq!(parse_url("192.168.1.20:7878").unwrap(), (true, "192.168.1.20".into(), 7878));
        assert_eq!(parse_url("http://gaming-pc").unwrap(), (false, "gaming-pc".into(), 7878));
        assert_eq!(parse_url("https://[fe80::1]:9000").unwrap(), (true, "fe80::1".into(), 9000));
        assert!(parse_url("https://x/y").is_err() && parse_url("a b").is_err());
        assert_eq!(parse_response(b"HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\n{}").unwrap(), (200, "{}".into()));
        let chunked = b"HTTP/1.1 403 Forbidden\r\nTransfer-Encoding: chunked\r\n\r\n5\r\n{\"err\r\n9\r\nor\":\"no\"}\r\n0\r\n\r\n";
        let (s, b) = parse_response(chunked).unwrap();
        assert_eq!((s, b.as_str()), (403, "{\"error\":\"no\"}"));
        assert_eq!(error_of(403, &b), "The other PC said: no");
        assert_eq!(fingerprint(b"abc").len(), 95);
        let t = tempfile::tempdir().unwrap();
        let p = Peers::new(t.path());
        assert!(p.list().is_empty() && p.call("x", "GET", "/api/status", None).is_err());
    }
}
