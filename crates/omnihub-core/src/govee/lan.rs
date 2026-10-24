//! Govee's LAN API. Lights with *LAN Control* switched on (Govee Home app →
//! the light → settings) answer a scan on the local network and take JSON
//! commands over UDP: on/off, brightness, a colour or a white, and their
//! status. Two binary sub-protocols ride along in base64: `ptReal` (the
//! 20-byte packets of Govee's Bluetooth protocol) and `razer` (DreamView
//! streaming, a colour per segment: what screen and music sync use).
//!
//! Scan: JSON to 239.255.255.250:4001; answers come to UDP 4002. Commands
//! go to the light's port 4003; `devStatus` answers come to 4002 too.
//! Nothing here leaves the local network.

use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Duration;

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::rig::openrgb::Rgb;

pub const MULTICAST: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);

/// The ports of the protocol (changed only by tests).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ports {
    /// Lights listen for scans here.
    pub scan: u16,
    /// Answers come back here.
    pub listen: u16,
    /// Lights take commands here.
    pub cmd: u16,
}

impl Default for Ports {
    fn default() -> Self {
        Ports { scan: 4001, listen: 4002, cmd: 4003 }
    }
}

pub fn xor(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0, |a, b| a ^ b)
}

/// A Bluetooth-protocol packet: the payload padded to 19 bytes, then its
/// XOR checksum.
pub fn ble_packet(payload: &[u8]) -> [u8; 20] {
    let mut p = [0u8; 20];
    let n = payload.len().min(19);
    p[..n].copy_from_slice(&payload[..n]);
    p[19] = xor(&p[..19]);
    p
}

/// A razer packet: `BB 00 <length> <command> <data…> <xor>`.
pub fn razer_packet(cmd: u8, data: &[u8]) -> Vec<u8> {
    let len = data.len().min(255);
    let mut p = Vec::with_capacity(len + 5);
    p.extend_from_slice(&[0xBB, 0x00, len as u8, cmd]);
    p.extend_from_slice(&data[..len]);
    p.push(xor(&p));
    p
}

/// Streaming on (the light takes razer frames) or off (back to its own modes).
pub fn razer_switch(on: bool) -> Vec<u8> {
    razer_packet(0xB1, &[on as u8])
}

/// The most colours one frame carries (its length is a single byte).
pub const RAZER_MAX: usize = 84;

/// One frame: a colour per segment, in order. With `gradient` the light
/// blends between neighbours.
pub fn razer_frame(colors: &[Rgb], gradient: bool) -> Vec<u8> {
    let n = colors.len().clamp(1, RAZER_MAX);
    let mut data = Vec::with_capacity(2 + n * 3);
    data.push(gradient as u8);
    data.push(n as u8);
    for i in 0..n {
        let c = colors.get(i).copied().unwrap_or(Rgb(0, 0, 0));
        data.extend_from_slice(&[c.0, c.1, c.2]);
    }
    razer_packet(0xB0, &data)
}

fn msg(cmd: &str, data: Value) -> Value {
    json!({ "msg": { "cmd": cmd, "data": data } })
}

pub fn scan() -> Value {
    msg("scan", json!({ "account_topic": "reserve" }))
}

pub fn turn(on: bool) -> Value {
    msg("turn", json!({ "value": on as u8 }))
}

pub fn brightness(percent: u8) -> Value {
    msg("brightness", json!({ "value": percent.clamp(1, 100) }))
}

pub fn color(c: Rgb) -> Value {
    msg("colorwc", json!({ "color": { "r": c.0, "g": c.1, "b": c.2 }, "colorTemInKelvin": 0 }))
}

/// A white at a colour temperature (the colour is ignored).
pub fn kelvin(k: u32) -> Value {
    msg("colorwc", json!({ "color": { "r": 0, "g": 0, "b": 0 }, "colorTemInKelvin": k.clamp(2000, 9000) }))
}

pub fn status() -> Value {
    msg("devStatus", json!({}))
}

pub fn razer(packet: &[u8]) -> Value {
    msg("razer", json!({ "pt": base64::engine::general_purpose::STANDARD.encode(packet) }))
}

pub fn pt_real(packets: &[&[u8]]) -> Value {
    let list: Vec<String> = packets.iter().map(|p| base64::engine::general_purpose::STANDARD.encode(p)).collect();
    msg("ptReal", json!({ "command": list }))
}

/// A light that answered the scan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LanDevice {
    pub ip: IpAddr,
    /// Govee's id for it (as the cloud API uses).
    pub device: String,
    /// The model, "H6199".
    pub sku: String,
    pub firmware: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LanStatus {
    pub on: bool,
    pub brightness: u8,
    pub color: Rgb,
    /// 0 when showing a colour.
    pub kelvin: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Scan(LanDevice),
    Status(IpAddr, LanStatus),
}

fn int(v: &Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_bool().map(|b| b as u64)).or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

/// Read an answer that came from `from`.
pub fn parse(data: &[u8], from: IpAddr) -> Option<Reply> {
    let v: Value = serde_json::from_slice(data).ok()?;
    let d = &v["msg"]["data"];
    match v["msg"]["cmd"].as_str()? {
        "scan" => {
            // Some newer lights leave out their address: use the sender's.
            let ip = d["ip"].as_str().and_then(|s| s.parse().ok()).filter(|ip: &IpAddr| !ip.is_unspecified()).unwrap_or(from);
            let device = d["device"].as_str()?.to_string();
            let sku = d["sku"].as_str().unwrap_or_default().to_string();
            let firmware = d["wifiVersionSoft"].as_str().unwrap_or_default().to_string();
            Some(Reply::Scan(LanDevice { ip, device, sku, firmware }))
        }
        "devStatus" => {
            let c = &d["color"];
            let ch = |k: &str| int(&c[k]).unwrap_or(0).min(255) as u8;
            Some(Reply::Status(
                from,
                LanStatus { on: int(&d["onOff"]).unwrap_or(0) != 0, brightness: int(&d["brightness"]).unwrap_or(0).min(100) as u8, color: Rgb(ch("r"), ch("g"), ch("b")), kelvin: int(&d["colorTemInKelvin"]).unwrap_or(0) as u32 },
            ))
        }
        _ => None,
    }
}

/// The sockets: one bound to the answer port, one to send from.
pub struct Lan {
    listen: UdpSocket,
    send: UdpSocket,
    ports: Ports,
}

impl Lan {
    /// Listen on all addresses for answers (port 4002).
    pub fn open() -> std::io::Result<Lan> {
        Lan::open_on(Ipv4Addr::UNSPECIFIED, Ports::default())
    }

    pub fn open_on(addr: Ipv4Addr, ports: Ports) -> std::io::Result<Lan> {
        let listen = UdpSocket::bind((addr, ports.listen)).map_err(|e| {
            if e.kind() == std::io::ErrorKind::AddrInUse {
                std::io::Error::new(e.kind(), "Another program is using UDP port 4002 for Govee lights (Govee Desktop, Home Assistant or a similar tool). Close it, then try again.")
            } else {
                e
            }
        })?;
        let send = UdpSocket::bind((addr, 0))?;
        Ok(Lan { listen, send, ports })
    }

    /// The answer port actually bound (tests bind port 0).
    pub fn listen_port(&self) -> u16 {
        self.listen.local_addr().map(|a| a.port()).unwrap_or(self.ports.listen)
    }

    /// Ask every light to answer: multicast from each local network, and
    /// directly to `extra` addresses (lights added by hand).
    pub fn scan(&self, interfaces: &[Ipv4Addr], extra: &[IpAddr]) {
        let body = scan().to_string();
        for ip in interfaces {
            match multicast_socket(*ip) {
                Ok(s) => {
                    if let Err(e) = s.send_to(body.as_bytes(), (MULTICAST, self.ports.scan)) {
                        tracing::debug!("govee scan from {ip}: {e}");
                    }
                }
                Err(e) => tracing::debug!("govee scan socket on {ip}: {e}"),
            }
        }
        for ip in extra {
            let _ = self.send.send_to(body.as_bytes(), (*ip, self.ports.scan));
        }
    }

    /// Ask every address of each local /24 network directly: routers that
    /// drop multicast (or Wi-Fi with "IGMP snooping") still pass these.
    pub fn sweep(&self, networks: &[Ipv4Addr]) {
        let body = scan().to_string();
        for ip in sweep_targets(networks) {
            let _ = self.send.send_to(body.as_bytes(), (ip, self.ports.scan));
        }
    }

    pub fn send(&self, ip: IpAddr, msg: &Value) -> std::io::Result<()> {
        self.send.send_to(msg.to_string().as_bytes(), (ip, self.ports.cmd)).map(|_| ())
    }

    /// The next answer, waiting up to `timeout`.
    pub fn recv(&self, timeout: Duration) -> Option<Reply> {
        let _ = self.listen.set_read_timeout(Some(timeout.max(Duration::from_millis(1))));
        let mut buf = [0u8; 2048];
        loop {
            match self.listen.recv_from(&mut buf) {
                Ok((n, from)) => {
                    if let Some(r) = parse(&buf[..n], from.ip()) {
                        return Some(r);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => continue,
                Err(_) => return None,
            }
        }
    }
}

/// A socket that sends multicast out of the network `ip` belongs to.
fn multicast_socket(ip: Ipv4Addr) -> std::io::Result<UdpSocket> {
    use socket2::{Domain, Protocol, Socket, Type};
    let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    s.set_multicast_if_v4(&ip)?;
    s.set_multicast_ttl_v4(2)?;
    s.bind(&SocketAddr::from((ip, 0)).into())?;
    Ok(s.into())
}

/// Every other host of each address's /24 (at most 4 networks).
pub fn sweep_targets(networks: &[Ipv4Addr]) -> Vec<Ipv4Addr> {
    let mut nets: Vec<[u8; 3]> = Vec::new();
    for ip in networks {
        let o = ip.octets();
        if !nets.contains(&[o[0], o[1], o[2]]) && nets.len() < 4 {
            nets.push([o[0], o[1], o[2]]);
        }
    }
    nets.iter().flat_map(|n| (1..=254u8).map(move |h| Ipv4Addr::new(n[0], n[1], n[2], h))).filter(|a| !networks.contains(a)).collect()
}

/// The Wi-Fi and Ethernet addresses (no virtual adapters) to sweep.
pub fn sweep_networks() -> Vec<Ipv4Addr> {
    crate::remote::net::lan_addresses().into_iter().filter(|a| !a.virtual_adapter).filter_map(|a| a.ip.parse().ok()).collect()
}

/// The local IPv4 addresses to scan from (Wi-Fi and Ethernet first;
/// virtual adapters too, since a light can only answer the right one).
pub fn interfaces() -> Vec<Ipv4Addr> {
    crate::remote::net::lan_addresses().into_iter().filter_map(|a| a.ip.parse().ok()).collect()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// A pretend Govee light on 127.0.0.1: answers scans sent to its scan
    /// port and status requests, and keeps every command it was sent.
    pub struct FakeLight {
        pub ports: Ports,
        pub got: Arc<Mutex<Vec<Value>>>,
    }

    impl FakeLight {
        pub fn start(listen: u16, device: &str, sku: &str) -> FakeLight {
            let scan = UdpSocket::bind("127.0.0.1:0").unwrap();
            let cmd = UdpSocket::bind("127.0.0.1:0").unwrap();
            let ports = Ports { scan: scan.local_addr().unwrap().port(), listen, cmd: cmd.local_addr().unwrap().port() };
            let got = Arc::new(Mutex::new(Vec::new()));
            let (d, s) = (device.to_string(), sku.to_string());
            std::thread::spawn(move || {
                let mut buf = [0u8; 2048];
                while let Ok((n, from)) = scan.recv_from(&mut buf) {
                    let v: Value = serde_json::from_slice(&buf[..n]).unwrap_or_default();
                    if v["msg"]["cmd"] == "scan" {
                        // Like newer lights: no "ip" field.
                        let reply = json!({ "msg": { "cmd": "scan", "data": { "device": d, "sku": s, "wifiVersionSoft": "1.02.03" } } });
                        let _ = scan.send_to(reply.to_string().as_bytes(), (from.ip(), listen));
                    }
                }
            });
            let log = got.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 2048];
                let mut state = json!({ "onOff": 1, "brightness": 80, "color": { "r": 255, "g": 0, "b": 0 }, "colorTemInKelvin": 0 });
                while let Ok((n, from)) = cmd.recv_from(&mut buf) {
                    let v: Value = serde_json::from_slice(&buf[..n]).unwrap_or_default();
                    match v["msg"]["cmd"].as_str() {
                        Some("devStatus") => {
                            let reply = json!({ "msg": { "cmd": "devStatus", "data": state } });
                            let _ = cmd.send_to(reply.to_string().as_bytes(), (from.ip(), listen));
                        }
                        Some("turn") => state["onOff"] = v["msg"]["data"]["value"].clone(),
                        Some("brightness") => state["brightness"] = v["msg"]["data"]["value"].clone(),
                        Some("colorwc") => state["color"] = v["msg"]["data"]["color"].clone(),
                        _ => {}
                    }
                    log.lock().unwrap().push(v);
                }
            });
            FakeLight { ports, got }
        }

        pub fn commands(&self, cmd: &str) -> Vec<Value> {
            self.got.lock().unwrap().iter().filter(|v| v["msg"]["cmd"] == cmd).cloned().collect()
        }
    }

    #[test]
    fn sweeps_each_network_once() {
        let t = sweep_targets(&[Ipv4Addr::new(192, 168, 1, 20), Ipv4Addr::new(192, 168, 1, 21), Ipv4Addr::new(10, 0, 0, 5)]);
        assert_eq!(t.len(), 254 * 2 - 3);
        assert!(t.contains(&Ipv4Addr::new(192, 168, 1, 1)) && t.contains(&Ipv4Addr::new(10, 0, 0, 254)));
        assert!(!t.contains(&Ipv4Addr::new(192, 168, 1, 20)) && !t.contains(&Ipv4Addr::new(10, 0, 0, 0)));
    }

    #[test]
    fn packets() {
        // The razer switch, as LedFx and DreamView send it.
        assert_eq!(razer_switch(true), vec![0xBB, 0x00, 0x01, 0xB1, 0x01, 0xBB ^ 0x01 ^ 0xB1 ^ 0x01]);
        let f = razer_frame(&[Rgb(1, 2, 3), Rgb(4, 5, 6)], true);
        assert_eq!(&f[..12], &[0xBB, 0x00, 8, 0xB0, 1, 2, 1, 2, 3, 4, 5, 6]);
        assert_eq!(f[12], xor(&f[..12]));
        // Five segments, as an H1A42 strip takes it.
        let five = razer_frame(&[Rgb(255, 0, 0); 5], true);
        assert_eq!(&five[..6], &[0xBB, 0x00, 0x11, 0xB0, 0x01, 0x05]);
        assert_eq!(razer_frame(&vec![Rgb(9, 9, 9); 200], false)[2] as usize, 2 + 3 * RAZER_MAX);
        let p = ble_packet(&[0x33, 0x01, 0x01]);
        assert_eq!((p.len(), p[0], p[19]), (20, 0x33, 0x33 ^ 0x01 ^ 0x01));
        assert_eq!(brightness(0)["msg"]["data"]["value"], 1);
        assert_eq!(kelvin(12000)["msg"]["data"]["colorTemInKelvin"], 9000);
        let r = razer(&[0xBB, 0x00]);
        assert_eq!(r["msg"]["data"]["pt"], "uwA=");
        assert_eq!(pt_real(&[&p])["msg"]["data"]["command"][0].as_str().unwrap().len(), 28);
    }

    #[test]
    fn answers() {
        let from: IpAddr = "192.168.1.40".parse().unwrap();
        let scan = br#"{"msg":{"cmd":"scan","data":{"ip":"192.168.1.23","device":"1F:80:C5:32:32:36:72:4E","sku":"H618A","bleVersionHard":"3.01.01","bleVersionSoft":"1.03.01","wifiVersionHard":"1.00.10","wifiVersionSoft":"1.02.03"}}}"#;
        assert_eq!(parse(scan, from), Some(Reply::Scan(LanDevice { ip: "192.168.1.23".parse().unwrap(), device: "1F:80:C5:32:32:36:72:4E".into(), sku: "H618A".into(), firmware: "1.02.03".into() })));
        let no_ip = br#"{"msg":{"cmd":"scan","data":{"device":"AA","sku":"H6199"}}}"#;
        assert!(matches!(parse(no_ip, from), Some(Reply::Scan(d)) if d.ip == from));
        let st = br#"{"msg":{"cmd":"devStatus","data":{"onOff":1,"brightness":100,"color":{"r":255,"g":0,"b":0},"colorTemInKelvin":7200}}}"#;
        assert_eq!(parse(st, from), Some(Reply::Status(from, LanStatus { on: true, brightness: 100, color: Rgb(255, 0, 0), kelvin: 7200 })));
        assert_eq!(parse(b"nonsense", from), None);
        assert_eq!(parse(br#"{"msg":{"cmd":"other","data":{}}}"#, from), None);
    }

    #[test]
    fn finds_and_drives_a_light() {
        let lan = Lan::open_on(Ipv4Addr::LOCALHOST, Ports { scan: 0, listen: 0, cmd: 0 }).unwrap();
        let fake = FakeLight::start(lan.listen_port(), "AA:BB:CC:DD:EE:FF:00:11", "H6199");
        let lan = Lan { ports: Ports { listen: lan.listen_port(), ..fake.ports }, ..lan };
        lan.scan(&[], &[IpAddr::V4(Ipv4Addr::LOCALHOST)]);
        let found = lan.recv(Duration::from_secs(2));
        assert!(matches!(&found, Some(Reply::Scan(d)) if d.sku == "H6199" && d.ip == IpAddr::V4(Ipv4Addr::LOCALHOST)), "{found:?}");
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        lan.send(ip, &color(Rgb(0, 128, 255))).unwrap();
        lan.send(ip, &brightness(40)).unwrap();
        lan.send(ip, &status()).unwrap();
        let st = lan.recv(Duration::from_secs(2));
        assert_eq!(st, Some(Reply::Status(ip, LanStatus { on: true, brightness: 40, color: Rgb(0, 128, 255), kelvin: 0 })));
    }
}
