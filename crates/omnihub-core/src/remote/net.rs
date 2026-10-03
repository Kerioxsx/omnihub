//! Network helpers: which peers may connect, which addresses to advertise,
//! the self-signed TLS certificate and the pairing QR code.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Only devices on the local network (and optionally Tailscale) may talk to
/// the companion server, whatever the firewall says.
pub fn is_allowed_peer(ip: IpAddr, allow_tailscale: bool) -> bool {
    match ip {
        IpAddr::V4(v4) => is_allowed_v4(v4, allow_tailscale),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_allowed_v4(v4, allow_tailscale);
            }
            let s = v6.segments();
            v6.is_loopback()
                || (s[0] & 0xfe00) == 0xfc00 // unique local fc00::/7
                || (s[0] & 0xffc0) == 0xfe80 // link local fe80::/10
                || (allow_tailscale && s[0] == 0xfd7a && s[1] == 0x115c && s[2] == 0xa1e0) // Tailscale ULA
        }
    }
}

fn is_allowed_v4(ip: Ipv4Addr, allow_tailscale: bool) -> bool {
    let o = ip.octets();
    ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || (allow_tailscale && o[0] == 100 && (o[1] & 0xc0) == 64) // 100.64.0.0/10
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LanAddress {
    pub interface: String,
    pub ip: String,
}

/// IPv4 addresses phones can reach, most likely first (Wi-Fi/Ethernet
/// private ranges before virtual adapters).
pub fn lan_addresses() -> Vec<LanAddress> {
    let mut out: Vec<(u8, LanAddress)> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback())
        .filter_map(|i| match i.ip() {
            IpAddr::V4(v4) if v4.is_private() || v4.octets()[0] == 100 => {
                let name = i.name.to_lowercase();
                let virtualish = ["vethernet", "virtualbox", "vmware", "docker", "wsl", "hyper-v", "vbox", "br-", "veth", "zt"]
                    .iter()
                    .any(|v| name.contains(v));
                let rank = if virtualish { 3 } else if v4.octets()[0] == 192 { 0 } else if v4.octets()[0] == 10 { 1 } else { 2 };
                Some((rank, LanAddress { interface: i.name.clone(), ip: v4.to_string() }))
            }
            _ => None,
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.ip.cmp(&b.1.ip)));
    out.dedup_by(|a, b| a.1.ip == b.1.ip);
    out.into_iter().map(|(_, a)| a).collect()
}

/// Host header check against DNS rebinding: only IP literals and this
/// machine's own names are accepted.
pub fn host_allowed(host: &str, hostname: &str, allow_tailscale: bool) -> bool {
    let h = host.trim().to_lowercase();
    // Strip the port (IPv6 literals are bracketed).
    let name = if let Some(rest) = h.strip_prefix('[') {
        rest.split(']').next().unwrap_or("").to_string()
    } else {
        h.rsplit_once(':').map_or(h.clone(), |(n, p)| if p.chars().all(|c| c.is_ascii_digit()) { n.to_string() } else { h.clone() })
    };
    if name.parse::<IpAddr>().is_ok() {
        return true;
    }
    let host_l = hostname.to_lowercase();
    name == "localhost" || name == host_l || name == format!("{host_l}.local") || (allow_tailscale && name.ends_with(".ts.net"))
}

/// Self-signed certificate for the companion server, created once and kept
/// in the config folder. Returns (cert PEM, key PEM).
pub fn load_or_create_cert(dir: &Path, hostname: &str) -> std::io::Result<(Vec<u8>, Vec<u8>)> {
    let cert_path = dir.join("remote-cert.pem");
    let key_path = dir.join("remote-key.pem");
    if let (Ok(c), Ok(k)) = (std::fs::read(&cert_path), std::fs::read(&key_path)) {
        if !c.is_empty() && !k.is_empty() {
            return Ok((c, k));
        }
    }
    let mut names = vec![hostname.to_lowercase(), format!("{}.local", hostname.to_lowercase()), "localhost".to_string()];
    names.extend(lan_addresses().into_iter().map(|a| a.ip));
    names.push("127.0.0.1".into());
    let mut params = rcgen::CertificateParams::new(names).map_err(std::io::Error::other)?;
    params.distinguished_name.push(rcgen::DnType::CommonName, format!("OmniHub on {hostname}"));
    params.distinguished_name.push(rcgen::DnType::OrganizationName, "OmniHub (self-signed)");
    use chrono::Datelike;
    let year = chrono::Utc::now().year();
    params.not_before = rcgen::date_time_ymd(year - 1, 1, 1);
    params.not_after = rcgen::date_time_ymd(year + 5, 1, 1);
    let key = rcgen::KeyPair::generate().map_err(std::io::Error::other)?;
    let cert = params.self_signed(&key).map_err(std::io::Error::other)?;
    let (c, k) = (cert.pem().into_bytes(), key.serialize_pem().into_bytes());
    crate::settings::write_atomic(&cert_path, &c)?;
    crate::settings::write_atomic(&key_path, &k)?;
    Ok((c, k))
}

/// rustls server configuration for the companion server (HTTP/1.1 only, so
/// browsers keep WebSockets on plain HTTP/1.1 connections).
pub fn server_config(cert_pem: &[u8], key_pem: &[u8]) -> std::io::Result<rustls::ServerConfig> {
    use rustls::pki_types::pem::PemObject;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    let certs: Vec<CertificateDer<'static>> = CertificateDer::pem_slice_iter(cert_pem).collect::<Result<_, _>>().map_err(|e| std::io::Error::other(e.to_string()))?;
    let key = PrivateKeyDer::from_pem_slice(key_pem).map_err(|e| std::io::Error::other(e.to_string()))?;
    let provider = std::sync::Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| std::io::Error::other(e.to_string()))?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(config)
}

/// SHA-256 fingerprint of the first certificate in a PEM, as AA:BB:...
pub fn fingerprint(cert_pem: &[u8]) -> String {
    let text = String::from_utf8_lossy(cert_pem);
    let b64: String = text
        .lines()
        .skip_while(|l| !l.starts_with("-----BEGIN CERTIFICATE"))
        .skip(1)
        .take_while(|l| !l.starts_with("-----END"))
        .collect();
    use base64::Engine;
    let der = base64::engine::general_purpose::STANDARD.decode(b64).unwrap_or_default();
    Sha256::digest(&der).iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
}

pub fn qr_svg(text: &str) -> String {
    match qrcode::QrCode::with_error_correction_level(text.as_bytes(), qrcode::EcLevel::M) {
        Ok(code) => code
            .render::<qrcode::render::svg::Color>()
            .min_dimensions(240, 240)
            .quiet_zone(true)
            .dark_color(qrcode::render::svg::Color("#0b0b12"))
            .light_color(qrcode::render::svg::Color("#ffffff"))
            .build(),
        Err(_) => String::new(),
    }
}

pub fn localhost() -> IpAddr {
    IpAddr::V4(Ipv4Addr::LOCALHOST)
}

pub fn unspecified() -> IpAddr {
    IpAddr::V6(Ipv6Addr::UNSPECIFIED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_filter() {
        let ok = |s: &str, ts: bool| is_allowed_peer(s.parse().unwrap(), ts);
        assert!(ok("192.168.1.20", false));
        assert!(ok("10.0.0.5", false));
        assert!(ok("172.20.1.1", false));
        assert!(ok("127.0.0.1", false));
        assert!(ok("169.254.3.3", false));
        assert!(ok("::ffff:192.168.0.9", false));
        assert!(ok("fe80::1", false));
        assert!(ok("fd12::1", false));
        assert!(!ok("8.8.8.8", false));
        assert!(!ok("2001:4860::8888", false));
        assert!(!ok("100.100.1.1", false));
        assert!(ok("100.100.1.1", true));
        assert!(!ok("100.128.1.1", true));
    }

    #[test]
    fn host_header() {
        assert!(host_allowed("192.168.1.2:47800", "DESKTOP-1", false));
        assert!(host_allowed("[fe80::1]:47800", "DESKTOP-1", false));
        assert!(host_allowed("desktop-1.local:47800", "DESKTOP-1", false));
        assert!(host_allowed("localhost", "x", false));
        assert!(!host_allowed("evil.example.com:47800", "DESKTOP-1", false));
        assert!(!host_allowed("pc.tail1234.ts.net", "DESKTOP-1", false));
        assert!(host_allowed("pc.tail1234.ts.net", "DESKTOP-1", true));
    }

    #[test]
    fn cert_is_created_once() {
        let dir = tempfile::tempdir().unwrap();
        let (c1, k1) = load_or_create_cert(dir.path(), "TESTPC").unwrap();
        let (c2, k2) = load_or_create_cert(dir.path(), "TESTPC").unwrap();
        assert_eq!((c1.clone(), k1), (c2, k2));
        server_config(&c1, &load_or_create_cert(dir.path(), "TESTPC").unwrap().1).unwrap();
        let fp = fingerprint(&c1);
        assert_eq!(fp.len(), 32 * 3 - 1);
        assert!(qr_svg("https://192.168.1.2:47800/#pair=abc").starts_with("<?xml"));
    }
}
