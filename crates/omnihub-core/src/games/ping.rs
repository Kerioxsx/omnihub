//! Ping helper: latency, jitter and packet loss to a game's servers.
//!
//! Windows sends ICMP echo requests (no administrator rights needed);
//! a "host:port" target, and other systems, time a TCP connection instead.

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PingTarget {
    pub id: String,
    pub label: String,
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PingResult {
    pub id: String,
    pub label: String,
    pub host: String,
    /// The address actually measured.
    pub address: Option<String>,
    pub sent: u32,
    pub received: u32,
    pub avg_ms: Option<f32>,
    pub min_ms: Option<f32>,
    pub max_ms: Option<f32>,
    /// Average change between consecutive replies.
    pub jitter_ms: Option<f32>,
    /// 0–1.
    pub loss: f32,
    pub samples: Vec<Option<f32>>,
    pub error: Option<String>,
}

/// Fortnite's matchmaking regions (the hosts the game itself pings).
pub fn fortnite_regions() -> Vec<PingTarget> {
    [
        ("nae", "NA-East"),
        ("nac", "NA-Central"),
        ("naw", "NA-West"),
        ("eu", "Europe"),
        ("br", "Brazil"),
        ("me", "Middle East"),
        ("asia", "Asia"),
        ("oce", "Oceania"),
    ]
    .into_iter()
    .map(|(id, label)| PingTarget { id: id.into(), label: label.into(), host: format!("ping-{id}.ds.on.epicgames.com") })
    .collect()
}

/// For games without public ping hosts: the nearest Cloudflare and Google
/// servers show how good the connection itself is.
pub fn general_targets() -> Vec<PingTarget> {
    vec![
        PingTarget { id: "cloudflare".into(), label: "Nearest Cloudflare".into(), host: "1.1.1.1".into() },
        PingTarget { id: "google".into(), label: "Nearest Google".into(), host: "8.8.8.8".into() },
    ]
}

/// Hosts are names, IPv4/IPv6 literals or "host:port"; nothing that could
/// smuggle in anything else.
pub fn valid_host(host: &str) -> bool {
    !host.is_empty() && host.len() <= 253 && host.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'))
}

pub fn summarize(target: &PingTarget, address: Option<String>, samples: Vec<Option<f32>>, error: Option<String>) -> PingResult {
    let got: Vec<f32> = samples.iter().flatten().copied().collect();
    let sent = samples.len() as u32;
    let received = got.len() as u32;
    let round = |v: f32| (v * 10.0).round() / 10.0;
    let jitter = {
        let diffs: Vec<f32> = got.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        (!diffs.is_empty()).then(|| round(diffs.iter().sum::<f32>() / diffs.len() as f32))
    };
    PingResult {
        id: target.id.clone(),
        label: target.label.clone(),
        host: target.host.clone(),
        address,
        sent,
        received,
        avg_ms: (!got.is_empty()).then(|| round(got.iter().sum::<f32>() / got.len() as f32)),
        min_ms: got.iter().copied().reduce(f32::min).map(round),
        max_ms: got.iter().copied().reduce(f32::max).map(round),
        jitter_ms: jitter,
        loss: if sent == 0 { 1.0 } else { 1.0 - received as f32 / sent as f32 },
        samples: samples.into_iter().map(|s| s.map(round)).collect(),
        error,
    }
}

/// `count` probes, `interval` apart, each waiting at most a second.
pub fn measure(target: &PingTarget, count: u32, interval: Duration) -> PingResult {
    if !valid_host(&target.host) {
        return summarize(target, None, Vec::new(), Some("That is not a host name or address.".into()));
    }
    let count = count.clamp(1, 100);
    // "host:port" (not a bare IPv6 address) → TCP connect timing.
    let tcp_port = match target.host.rsplit_once(':') {
        Some((h, p)) if !h.contains(':') || h.ends_with(']') => p.parse::<u16>().ok(),
        _ => None,
    };
    let found: Vec<SocketAddr> = match tcp_port {
        Some(_) => target.host.to_socket_addrs().map(Iterator::collect).unwrap_or_default(),
        None => (target.host.trim_matches(['[', ']']), 0).to_socket_addrs().map(Iterator::collect).unwrap_or_default(),
    };
    let Some(addr) = found.iter().find(|a| a.is_ipv4()).or(found.first()).copied() else {
        return summarize(target, None, vec![None; count as usize], Some(format!("Could not find {}", target.host)));
    };
    let mut samples = Vec::with_capacity(count as usize);
    #[cfg(windows)]
    let icmp = if tcp_port.is_none() { icmp::Pinger::new().ok() } else { None };
    for i in 0..count {
        if i > 0 {
            std::thread::sleep(interval);
        }
        #[cfg(windows)]
        if let (Some(p), SocketAddr::V4(v4)) = (&icmp, addr) {
            samples.push(p.echo(*v4.ip()));
            continue;
        }
        let port = tcp_port.unwrap_or(443);
        let t = Instant::now();
        let a = SocketAddr::new(addr.ip(), port);
        samples.push(TcpStream::connect_timeout(&a, Duration::from_secs(1)).ok().map(|_| t.elapsed().as_secs_f32() * 1000.0));
    }
    summarize(target, Some(addr.ip().to_string()), samples, None)
}

/// Several targets at once (one thread each).
pub fn measure_all(targets: &[PingTarget], count: u32, interval: Duration) -> Vec<PingResult> {
    std::thread::scope(|s| {
        let handles: Vec<_> = targets.iter().map(|t| s.spawn(move || measure(t, count, interval))).collect();
        handles.into_iter().zip(targets).map(|(h, t)| h.join().unwrap_or_else(|_| summarize(t, None, Vec::new(), Some("failed".into())))).collect()
    })
}

#[cfg(windows)]
mod icmp {
    use std::net::Ipv4Addr;
    use std::time::Instant;

    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::NetworkManagement::IpHelper::{IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho, ICMP_ECHO_REPLY};

    pub struct Pinger(HANDLE);

    impl Pinger {
        pub fn new() -> windows::core::Result<Self> {
            unsafe { IcmpCreateFile().map(Pinger) }
        }

        /// Round trip in ms, or None on timeout/unreachable.
        pub fn echo(&self, ip: Ipv4Addr) -> Option<f32> {
            let payload = [0x4fu8; 32];
            let mut reply = [0u8; std::mem::size_of::<ICMP_ECHO_REPLY>() + 32 + 8];
            let t = Instant::now();
            let n = unsafe { IcmpSendEcho(self.0, u32::from_ne_bytes(ip.octets()), payload.as_ptr().cast(), payload.len() as u16, None, reply.as_mut_ptr().cast(), reply.len() as u32, 1000) };
            let elapsed = t.elapsed().as_secs_f32() * 1000.0;
            if n == 0 {
                return None;
            }
            let r = unsafe { std::ptr::read_unaligned(reply.as_ptr().cast::<ICMP_ECHO_REPLY>()) };
            // IP_SUCCESS; the API's own figure is whole milliseconds, so keep
            // the finer local timing when it agrees.
            (r.Status == 0).then(|| if (elapsed - r.RoundTripTime as f32).abs() < 5.0 { elapsed } else { r.RoundTripTime as f32 })
        }
    }

    impl Drop for Pinger {
        fn drop(&mut self) {
            unsafe {
                let _ = IcmpCloseHandle(self.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics() {
        let t = PingTarget { id: "x".into(), label: "X".into(), host: "example.com".into() };
        let r = summarize(&t, None, vec![Some(20.0), Some(30.0), None, Some(25.0)], None);
        assert_eq!((r.sent, r.received), (4, 3));
        assert_eq!(r.avg_ms, Some(25.0));
        assert_eq!((r.min_ms, r.max_ms), (Some(20.0), Some(30.0)));
        // |30-20| and |25-30| → 7.5
        assert_eq!(r.jitter_ms, Some(7.5));
        assert!((r.loss - 0.25).abs() < 1e-6);
        let none = summarize(&t, None, vec![None, None], None);
        assert_eq!((none.avg_ms, none.jitter_ms, none.loss), (None, None, 1.0));
    }

    #[test]
    fn hosts_are_checked() {
        assert!(valid_host("ping-eu.ds.on.epicgames.com") && valid_host("1.1.1.1") && valid_host("[::1]:80") && valid_host("127.0.0.1:443"));
        assert!(!valid_host("") && !valid_host("a b") && !valid_host("x;rm -rf") && !valid_host("evil\"host"));
        assert_eq!(fortnite_regions()[3].host, "ping-eu.ds.on.epicgames.com");
    }

    #[test]
    fn measures_a_local_server() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || for _ in l.incoming() {});
        let t = PingTarget { id: "local".into(), label: "Local".into(), host: format!("127.0.0.1:{port}") };
        let r = measure(&t, 3, Duration::from_millis(10));
        assert_eq!((r.sent, r.received), (3, 3), "{r:?}");
        assert!(r.avg_ms.unwrap() < 200.0);
        assert_eq!(r.address.as_deref(), Some("127.0.0.1"));
        let bad = measure(&PingTarget { host: "no-such-host.invalid".into(), ..t.clone() }, 2, Duration::from_millis(1));
        assert!(bad.error.is_some() && bad.received == 0);
        let all = measure_all(&[t.clone(), t], 2, Duration::from_millis(1));
        assert!(all.iter().all(|r| r.received == 2));
    }
}
