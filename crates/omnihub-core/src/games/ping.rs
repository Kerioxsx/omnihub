//! Ping helper: latency, jitter and packet loss to a game's servers, and
//! lag under load: how much a busy connection (a download, a stream, cloud
//! sync) adds to the ping, which is behind most "my ping spikes" problems.
//!
//! Windows sends ICMP echo requests (no administrator rights needed);
//! a "host:port" target, and other systems, time a TCP connection instead.
//!
//! No setting makes the ping lower than the trip to the server: the
//! closest region is the floor. What can be removed is everything added on
//! top — Wi-Fi, queued downloads, background apps.

use std::io::Read;
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
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

/// One host, looked up once and probed as often as needed.
pub struct Prober {
    addr: SocketAddr,
    tcp_port: Option<u16>,
    #[cfg(windows)]
    icmp: Option<icmp::Pinger>,
}

impl Prober {
    pub fn new(host: &str) -> Result<Prober, String> {
        if !valid_host(host) {
            return Err("That is not a host name or address.".into());
        }
        // "host:port" (not a bare IPv6 address) → TCP connect timing.
        let tcp_port = match host.rsplit_once(':') {
            Some((h, p)) if !h.contains(':') || h.ends_with(']') => p.parse::<u16>().ok(),
            _ => None,
        };
        let found: Vec<SocketAddr> = match tcp_port {
            Some(_) => host.to_socket_addrs().map(Iterator::collect).unwrap_or_default(),
            None => (host.trim_matches(['[', ']']), 0).to_socket_addrs().map(Iterator::collect).unwrap_or_default(),
        };
        let addr = found.iter().find(|a| a.is_ipv4()).or(found.first()).copied().ok_or_else(|| format!("Could not find {host}"))?;
        Ok(Prober {
            addr,
            tcp_port,
            #[cfg(windows)]
            icmp: if tcp_port.is_none() { icmp::Pinger::new().ok() } else { None },
        })
    }

    pub fn address(&self) -> IpAddr {
        self.addr.ip()
    }

    /// One round trip in ms (at most a second), or None.
    pub fn probe(&self) -> Option<f32> {
        #[cfg(windows)]
        if let (Some(p), SocketAddr::V4(v4)) = (&self.icmp, self.addr) {
            return p.echo(*v4.ip());
        }
        let t = Instant::now();
        let a = SocketAddr::new(self.addr.ip(), self.tcp_port.unwrap_or(443));
        TcpStream::connect_timeout(&a, Duration::from_secs(1)).ok().map(|_| t.elapsed().as_secs_f32() * 1000.0)
    }
}

/// `count` probes, `interval` apart, each waiting at most a second.
pub fn measure(target: &PingTarget, count: u32, interval: Duration) -> PingResult {
    if !valid_host(&target.host) {
        return summarize(target, None, Vec::new(), Some("That is not a host name or address.".into()));
    }
    let count = count.clamp(1, 100);
    let prober = match Prober::new(&target.host) {
        Ok(p) => p,
        Err(e) => return summarize(target, None, vec![None; count as usize], Some(e)),
    };
    let mut samples = Vec::with_capacity(count as usize);
    for i in 0..count {
        if i > 0 {
            std::thread::sleep(interval);
        }
        samples.push(prober.probe());
    }
    summarize(target, Some(prober.address().to_string()), samples, None)
}

// ---------- lag under load ----------

/// How much a busy connection adds to the ping.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct LoadTest {
    /// What was pinged.
    pub target: String,
    pub idle_ms: Option<f32>,
    /// Ping while downloading / uploading as fast as the line allows.
    pub download_ms: Option<f32>,
    pub upload_ms: Option<f32>,
    pub download_mbps: Option<f32>,
    pub upload_mbps: Option<f32>,
    /// The larger of the two increases.
    pub extra_ms: Option<f32>,
    /// A+ to F, from `extra_ms`.
    pub grade: Option<String>,
    pub advice: Vec<String>,
    pub error: Option<String>,
}

/// Where the test downloads from and uploads to.
pub struct LoadUrls {
    pub download: String,
    pub upload: String,
}

impl Default for LoadUrls {
    fn default() -> Self {
        LoadUrls { download: "https://speed.cloudflare.com/__down?bytes=200000000".into(), upload: "https://speed.cloudflare.com/__up".into() }
    }
}

/// The grade a test like Waveform's bufferbloat test gives.
pub fn grade(extra_ms: f32) -> &'static str {
    match extra_ms {
        x if x < 5.0 => "A+",
        x if x < 30.0 => "A",
        x if x < 60.0 => "B",
        x if x < 200.0 => "C",
        x if x < 400.0 => "D",
        _ => "F",
    }
}

/// What to do about the result, plainly.
pub fn advice(t: &LoadTest, on_wifi: Option<bool>) -> Vec<String> {
    let mut out = Vec::new();
    match t.extra_ms {
        Some(x) if x >= 30.0 => out.push(format!("A busy connection adds {x:.0} ms. Your router lets downloads queue up in front of game traffic: turn on its Smart Queue / SQM / QoS (often called “Gaming mode” or “Adaptive QoS”), ideally limited to about 90% of your speed.")),
        Some(x) if x >= 5.0 => out.push(format!("A busy connection adds {x:.0} ms — fine for most games; Smart Queue / SQM on the router can take it lower.")),
        Some(_) => out.push("Downloads don't slow your ping down. Your connection handles load well.".into()),
        None => {}
    }
    if on_wifi == Some(true) {
        out.push("You're on Wi-Fi: an Ethernet cable removes the delay and the spikes Wi-Fi adds.".into());
    }
    if t.extra_ms.is_some_and(|x| x >= 5.0) {
        out.push("Pause downloads while you play (Steam, Epic, Windows Update, other people streaming). A boost closes OneDrive, Google Drive and Dropbox for you.".into());
    }
    out.push("Ping can't go below the distance to the game's server: pick the closest region in the game. No setting makes it 0.".into());
    out
}

fn agent(limit: Duration) -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
    ureq::Agent::config_builder().tls_config(tls).timeout_connect(Some(Duration::from_secs(10))).timeout_global(Some(limit)).http_status_as_error(false).build().new_agent()
}

/// Zeros until told to stop.
struct Filler {
    stop: Arc<AtomicBool>,
    sent: Arc<AtomicU64>,
    left: u64,
}

impl Read for Filler {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.stop.load(Ordering::Relaxed) || self.left == 0 {
            return Ok(0);
        }
        let n = buf.len().min(64 * 1024).min(self.left as usize);
        buf[..n].fill(0);
        self.left -= n as u64;
        self.sent.fetch_add(n as u64, Ordering::Relaxed);
        Ok(n)
    }
}

/// Keep the line busy in one direction until `stop`.
fn saturate(url: &str, upload: bool, streams: usize, limit: Duration, stop: &Arc<AtomicBool>, bytes: &Arc<AtomicU64>) -> Vec<std::thread::JoinHandle<Option<String>>> {
    (0..streams)
        .map(|_| {
            let (url, stop, bytes) = (url.to_string(), stop.clone(), bytes.clone());
            std::thread::spawn(move || {
                let agent = agent(limit);
                let mut error = None;
                while !stop.load(Ordering::Relaxed) {
                    let r = if upload {
                        let mut f = Filler { stop: stop.clone(), sent: bytes.clone(), left: 50_000_000 };
                        agent.post(&url).send(ureq::SendBody::from_reader(&mut f)).map(|_| ())
                    } else {
                        agent.get(&url).call().map(|mut res| {
                            let mut reader = res.body_mut().as_reader();
                            let mut buf = vec![0u8; 64 * 1024];
                            while !stop.load(Ordering::Relaxed) {
                                match reader.read(&mut buf) {
                                    Ok(0) | Err(_) => break,
                                    Ok(n) => {
                                        bytes.fetch_add(n as u64, Ordering::Relaxed);
                                    }
                                }
                            }
                        })
                    };
                    if let Err(e) = r {
                        error = Some(e.to_string());
                        break;
                    }
                }
                error
            })
        })
        .collect()
}

fn median(samples: &[Option<f32>]) -> Option<f32> {
    let mut got: Vec<f32> = samples.iter().flatten().copied().collect();
    if got.is_empty() {
        return None;
    }
    got.sort_by(f32::total_cmp);
    Some((got[got.len() / 2] * 10.0).round() / 10.0)
}

/// Ping `target` idle, then while downloading, then while uploading, each
/// for `phase`. `progress` gets the phase ("idle", "download", "upload"),
/// how far along it is (0–1) and the latest ping.
pub fn load_test(target: &PingTarget, urls: &LoadUrls, phase: Duration, on_wifi: Option<bool>, progress: &mut dyn FnMut(&str, f32, Option<f32>)) -> LoadTest {
    let mut t = LoadTest { target: target.label.clone(), ..Default::default() };
    let prober = match Prober::new(&target.host) {
        Ok(p) => p,
        Err(e) => {
            t.error = Some(e);
            return t;
        }
    };
    let probe_every = Duration::from_millis(100);
    let mut idle = Vec::new();
    for i in 0..10 {
        let s = prober.probe();
        idle.push(s);
        progress("idle", (i + 1) as f32 / 10.0, s);
        std::thread::sleep(probe_every);
    }
    t.idle_ms = median(&idle);
    if t.idle_ms.is_none() {
        t.error = Some(format!("{} didn't answer, so the test can't compare.", target.label));
        return t;
    }
    for (name, url, upload) in [("download", urls.download.as_str(), false), ("upload", urls.upload.as_str(), true)] {
        let stop = Arc::new(AtomicBool::new(false));
        let bytes = Arc::new(AtomicU64::new(0));
        let workers = saturate(url, upload, if upload { 3 } else { 4 }, phase + Duration::from_secs(15), &stop, &bytes);
        // Let the transfers ramp up before measuring.
        std::thread::sleep(Duration::from_millis(800).min(phase / 2));
        let (b0, t0) = (bytes.load(Ordering::Relaxed), Instant::now());
        let mut samples = Vec::new();
        while t0.elapsed() < phase {
            let s = prober.probe();
            samples.push(s);
            progress(name, (t0.elapsed().as_secs_f32() / phase.as_secs_f32()).min(1.0), s);
            std::thread::sleep(probe_every);
        }
        let moved = bytes.load(Ordering::Relaxed).saturating_sub(b0);
        let secs = t0.elapsed().as_secs_f32();
        stop.store(true, Ordering::Relaxed);
        let errors: Vec<String> = workers.into_iter().filter_map(|w| w.join().ok().flatten()).collect();
        let mbps = (moved > 0).then(|| (moved as f32 * 8.0 / secs / 1e6 * 10.0).round() / 10.0);
        if mbps.is_none() {
            t.error.get_or_insert_with(|| format!("The {name} didn't start{}", errors.first().map(|e| format!(": {e}")).unwrap_or_default()));
        }
        let ms = median(&samples);
        if upload {
            (t.upload_ms, t.upload_mbps) = (ms, mbps);
        } else {
            (t.download_ms, t.download_mbps) = (ms, mbps);
        }
    }
    let idle = t.idle_ms.unwrap_or(0.0);
    t.extra_ms = [t.download_ms, t.upload_ms].into_iter().flatten().map(|v| (v - idle).max(0.0)).reduce(f32::max).map(|v| (v * 10.0).round() / 10.0);
    t.grade = t.extra_ms.map(|x| grade(x).to_string());
    t.advice = advice(&t, on_wifi);
    t
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
    fn grades_and_advice() {
        assert_eq!([grade(1.0), grade(12.0), grade(45.0), grade(120.0), grade(300.0), grade(900.0)], ["A+", "A", "B", "C", "D", "F"]);
        let bad = LoadTest { extra_ms: Some(140.0), ..Default::default() };
        let a = advice(&bad, Some(true));
        assert!(a[0].contains("140 ms") && a[0].contains("SQM"));
        assert!(a.iter().any(|x| x.contains("Ethernet")));
        assert!(a.last().unwrap().contains("No setting makes it 0"));
        let good = LoadTest { extra_ms: Some(2.0), ..Default::default() };
        assert!(advice(&good, Some(false)).iter().all(|x| !x.contains("Ethernet") && !x.contains("Pause")));
    }

    /// A local server stands in for the speed test.
    #[test]
    fn lag_under_load_against_a_local_server() {
        use std::io::Write;
        let http = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = http.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for c in http.incoming().flatten() {
                std::thread::spawn(move || {
                    let mut c = c;
                    let mut buf = vec![0u8; 64 * 1024];
                    let n = c.read(&mut buf).unwrap_or(0);
                    if buf[..n].starts_with(b"GET") {
                        let _ = c.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1000000000\r\nConnection: close\r\n\r\n");
                        let zeros = vec![0u8; 64 * 1024];
                        while c.write_all(&zeros).is_ok() {}
                    } else {
                        // Read the (chunked) upload to its end.
                        let mut tail: Vec<u8> = buf[..n].to_vec();
                        while !tail.ends_with(b"0\r\n\r\n") {
                            match c.read(&mut buf) {
                                Ok(0) | Err(_) => return,
                                Ok(n) => {
                                    tail.extend_from_slice(&buf[..n]);
                                    let keep = tail.len().saturating_sub(8);
                                    tail.drain(..keep);
                                }
                            }
                        }
                        let _ = c.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    }
                });
            }
        });
        let echo = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let echo_port = echo.local_addr().unwrap().port();
        std::thread::spawn(move || for _ in echo.incoming() {});
        let target = PingTarget { id: "local".into(), label: "Local".into(), host: format!("127.0.0.1:{echo_port}") };
        let urls = LoadUrls { download: format!("http://127.0.0.1:{port}/down"), upload: format!("http://127.0.0.1:{port}/up") };
        let mut phases = Vec::new();
        let t = load_test(&target, &urls, Duration::from_millis(700), Some(false), &mut |p, _, _| {
            if phases.last().map(String::as_str) != Some(p) {
                phases.push(p.to_string());
            }
        });
        assert_eq!(phases, ["idle", "download", "upload"]);
        assert!(t.error.is_none(), "{t:?}");
        assert!(t.idle_ms.is_some() && t.download_ms.is_some() && t.upload_ms.is_some(), "{t:?}");
        assert!(t.download_mbps.unwrap() > 1.0 && t.upload_mbps.unwrap() > 1.0, "{t:?}");
        assert!(t.grade.is_some() && !t.advice.is_empty());

        let gone = load_test(&PingTarget { host: "no-such-host.invalid".into(), ..target }, &urls, Duration::from_millis(100), None, &mut |_, _, _| {});
        assert!(gone.error.is_some());
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
