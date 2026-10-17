//! A client for OpenRGB's SDK server: OpenRGB knows the lighting of most
//! motherboards, graphics cards, memory, fans, coolers and strips (ASUS,
//! MSI, Gigabyte, Corsair, NZXT, Lian Li, Razer…), and lets other programs
//! set their colours over a local TCP port (6742). The protocol is
//! OpenRGB's own (see its `Documentation/OpenRGBSDK.md`); OmniHub speaks
//! protocol 3, which every OpenRGB from 0.7 on still answers.

use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const DEFAULT_PORT: u16 = 6742;
/// The newest protocol this client reads.
pub const PROTOCOL: u32 = 3;

const REQUEST_CONTROLLER_COUNT: u32 = 0;
const REQUEST_CONTROLLER_DATA: u32 = 1;
const REQUEST_PROTOCOL_VERSION: u32 = 40;
const SET_CLIENT_NAME: u32 = 50;
const DEVICE_LIST_UPDATED: u32 = 100;
const UPDATELEDS: u32 = 1050;
const SETCUSTOMMODE: u32 = 1100;

/// What a controller lights (OpenRGB's `device_type`).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeviceKind {
    Motherboard,
    Memory,
    Gpu,
    Cooler,
    LedStrip,
    Keyboard,
    Mouse,
    Mousemat,
    Headset,
    HeadsetStand,
    Gamepad,
    Light,
    Speaker,
    Virtual,
    Storage,
    Case,
    Microphone,
    Accessory,
    Keypad,
    #[default]
    Other,
}

impl DeviceKind {
    pub fn from_code(c: i32) -> DeviceKind {
        use DeviceKind::*;
        match c {
            0 => Motherboard,
            1 => Memory,
            2 => Gpu,
            3 => Cooler,
            4 => LedStrip,
            5 => Keyboard,
            6 => Mouse,
            7 => Mousemat,
            8 => Headset,
            9 => HeadsetStand,
            10 => Gamepad,
            11 => Light,
            12 => Speaker,
            13 => Virtual,
            14 => Storage,
            15 => Case,
            16 => Microphone,
            17 => Accessory,
            18 => Keypad,
            _ => Other,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Zone {
    pub name: String,
    pub leds: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Mode {
    pub name: String,
}

/// One lit device as OpenRGB describes it.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Controller {
    /// Its place in OpenRGB's list (what packets address).
    pub index: u32,
    pub kind: DeviceKind,
    pub name: String,
    pub vendor: String,
    pub description: String,
    pub location: String,
    pub modes: Vec<Mode>,
    pub active_mode: i32,
    pub zones: Vec<Zone>,
    pub leds: usize,
    /// The colours it shows now, one per LED, as "#rrggbb".
    pub colors: Vec<String>,
}

impl Controller {
    /// A stable key across restarts (the index can change).
    pub fn key(&self) -> String {
        format!("{}|{}", self.name, self.location)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
    pub fn parse(s: &str) -> Option<Rgb> {
        let h = s.trim().trim_start_matches('#');
        if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let v = u32::from_str_radix(h, 16).ok()?;
        Some(Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
    }
    /// OpenRGB's `RGBColor`: red in the lowest byte.
    fn wire(self) -> [u8; 4] {
        [self.0, self.1, self.2, 0]
    }
}

fn header(dev: u32, id: u32, size: u32) -> [u8; 16] {
    let mut h = [0u8; 16];
    h[..4].copy_from_slice(b"ORGB");
    h[4..8].copy_from_slice(&dev.to_le_bytes());
    h[8..12].copy_from_slice(&id.to_le_bytes());
    h[12..16].copy_from_slice(&size.to_le_bytes());
    h
}

/// A cursor over a little-endian packet body.
pub struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Reader { b, at: 0 }
    }
    fn take(&mut self, n: usize) -> io::Result<&'a [u8]> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.b.len()).ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "OpenRGB sent a short packet"))?;
        let s = &self.b[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap_or_default()))
    }
    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap_or_default()))
    }
    fn i32(&mut self) -> io::Result<i32> {
        Ok(self.u32()? as i32)
    }
    fn string(&mut self) -> io::Result<String> {
        let n = self.u16()? as usize;
        let s = self.take(n)?;
        let s = s.strip_suffix(&[0]).unwrap_or(s);
        Ok(String::from_utf8_lossy(s).into_owned())
    }
}

/// Parse a controller description (`REQUEST_CONTROLLER_DATA`'s answer,
/// after its leading size) written for protocol `version`.
pub fn parse_controller(index: u32, body: &[u8], version: u32) -> io::Result<Controller> {
    let mut r = Reader::new(body);
    let mut c = Controller { index, kind: DeviceKind::from_code(r.i32()?), name: r.string()?, ..Default::default() };
    if version >= 1 {
        c.vendor = r.string()?;
    }
    c.description = r.string()?;
    let _version = r.string()?;
    let _serial = r.string()?;
    c.location = r.string()?;
    let modes = r.u16()?;
    c.active_mode = r.i32()?;
    for _ in 0..modes {
        let name = r.string()?;
        // value, flags, speed min/max, [brightness min/max], colors min/max,
        // speed, [brightness], direction, color mode.
        let words = if version >= 3 { 12 } else { 9 };
        r.take(4 * words)?;
        let n = r.u16()? as usize;
        r.take(4 * n)?;
        c.modes.push(Mode { name });
    }
    let zones = r.u16()?;
    for _ in 0..zones {
        let name = r.string()?;
        let _kind = r.i32()?;
        let _min = r.u32()?;
        let _max = r.u32()?;
        let leds = r.u32()?;
        let matrix = r.u16()? as usize;
        r.take(matrix)?;
        if version >= 4 {
            let segments = r.u16()?;
            for _ in 0..segments {
                r.string()?;
                r.take(12)?;
            }
        }
        if version >= 5 {
            r.u32()?;
        }
        c.zones.push(Zone { name, leds });
    }
    let leds = r.u16()?;
    for _ in 0..leds {
        r.string()?;
        r.u32()?;
    }
    c.leds = leds as usize;
    let colors = r.u16()?;
    for _ in 0..colors {
        let b = r.take(4)?;
        c.colors.push(Rgb(b[0], b[1], b[2]).hex());
    }
    Ok(c)
}

/// The body of an `UPDATELEDS` packet (its size first, as OpenRGB wants).
pub fn update_leds_body(colors: &[Rgb]) -> Vec<u8> {
    let n = colors.len().min(u16::MAX as usize);
    let size = 4 + 2 + 4 * n;
    let mut b = Vec::with_capacity(size);
    b.extend_from_slice(&(size as u32).to_le_bytes());
    b.extend_from_slice(&(n as u16).to_le_bytes());
    for c in &colors[..n] {
        b.extend_from_slice(&c.wire());
    }
    b
}

/// A connection to OpenRGB.
pub struct Client {
    stream: TcpStream,
    pub version: u32,
    /// OpenRGB said its device list changed (devices must be read again).
    pub list_changed: bool,
}

impl Client {
    pub fn connect(addr: SocketAddr, timeout: Duration) -> io::Result<Client> {
        let stream = TcpStream::connect_timeout(&addr, timeout)?;
        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        let mut c = Client { stream, version: 0, list_changed: false };
        c.send(0, REQUEST_PROTOCOL_VERSION, &PROTOCOL.to_le_bytes())?;
        // A protocol-0 server doesn't answer.
        c.version = match c.recv(REQUEST_PROTOCOL_VERSION) {
            Ok(b) if b.len() >= 4 => u32::from_le_bytes(b[..4].try_into().unwrap_or_default()).min(PROTOCOL),
            Ok(_) => 0,
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => 0,
            Err(e) => return Err(e),
        };
        c.send(0, SET_CLIENT_NAME, b"OmniHub\0")?;
        Ok(c)
    }

    fn send(&mut self, dev: u32, id: u32, body: &[u8]) -> io::Result<()> {
        let mut p = Vec::with_capacity(16 + body.len());
        p.extend_from_slice(&header(dev, id, body.len() as u32));
        p.extend_from_slice(body);
        self.stream.write_all(&p)
    }

    /// The next packet with `id`; others (a device-list change) are noted.
    fn recv(&mut self, id: u32) -> io::Result<Vec<u8>> {
        loop {
            let mut h = [0u8; 16];
            self.stream.read_exact(&mut h)?;
            if &h[..4] != b"ORGB" {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "not an OpenRGB server"));
            }
            let got = u32::from_le_bytes(h[8..12].try_into().unwrap_or_default());
            let size = u32::from_le_bytes(h[12..16].try_into().unwrap_or_default()) as usize;
            if size > 16 << 20 {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "OpenRGB packet too large"));
            }
            let mut body = vec![0u8; size];
            self.stream.read_exact(&mut body)?;
            if got == id {
                return Ok(body);
            }
            if got == DEVICE_LIST_UPDATED {
                self.list_changed = true;
            }
        }
    }

    /// Every controller OpenRGB knows.
    pub fn controllers(&mut self) -> io::Result<Vec<Controller>> {
        self.list_changed = false;
        self.send(0, REQUEST_CONTROLLER_COUNT, &[])?;
        let b = self.recv(REQUEST_CONTROLLER_COUNT)?;
        let count = Reader::new(&b).u32()?.min(256);
        let mut out = Vec::new();
        for i in 0..count {
            if self.version >= 1 {
                self.send(i, REQUEST_CONTROLLER_DATA, &self.version.to_le_bytes())?;
            } else {
                self.send(i, REQUEST_CONTROLLER_DATA, &[])?;
            }
            let b = self.recv(REQUEST_CONTROLLER_DATA)?;
            // Leading size, then the description.
            out.push(parse_controller(i, b.get(4..).unwrap_or_default(), self.version)?);
        }
        Ok(out)
    }

    /// Hand the device to software control ("Direct"/"Custom" mode).
    pub fn take_control(&mut self, index: u32) -> io::Result<()> {
        self.send(index, SETCUSTOMMODE, &[])
    }

    pub fn set_leds(&mut self, index: u32, colors: &[Rgb]) -> io::Result<()> {
        self.send(index, UPDATELEDS, &update_leds_body(colors))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::net::TcpListener;

    /// A controller description as an OpenRGB server writes it (protocol 3).
    pub fn describe(kind: i32, name: &str, location: &str, zones: &[(&str, u32)], color: Rgb) -> Vec<u8> {
        fn s(b: &mut Vec<u8>, v: &str) {
            b.extend_from_slice(&((v.len() + 1) as u16).to_le_bytes());
            b.extend_from_slice(v.as_bytes());
            b.push(0);
        }
        let mut b = Vec::new();
        b.extend_from_slice(&kind.to_le_bytes());
        s(&mut b, name);
        s(&mut b, "Vendor");
        s(&mut b, "A test device");
        s(&mut b, "1.0");
        s(&mut b, "SN");
        s(&mut b, location);
        // Two modes, "Direct" active.
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&0i32.to_le_bytes());
        for (m, colors) in [("Direct", 0u16), ("Static", 1)] {
            s(&mut b, m);
            b.extend_from_slice(&[0u8; 48]);
            b.extend_from_slice(&colors.to_le_bytes());
            b.extend(std::iter::repeat_n(0u8, 4 * colors as usize));
        }
        let leds: u32 = zones.iter().map(|z| z.1).sum();
        b.extend_from_slice(&(zones.len() as u16).to_le_bytes());
        for (z, n) in zones {
            s(&mut b, z);
            b.extend_from_slice(&1i32.to_le_bytes());
            b.extend_from_slice(&n.to_le_bytes());
            b.extend_from_slice(&n.to_le_bytes());
            b.extend_from_slice(&n.to_le_bytes());
            // A 1×n matrix map.
            b.extend_from_slice(&((8 + 4 * n) as u16).to_le_bytes());
            b.extend_from_slice(&1u32.to_le_bytes());
            b.extend_from_slice(&n.to_le_bytes());
            for i in 0..*n {
                b.extend_from_slice(&i.to_le_bytes());
            }
        }
        b.extend_from_slice(&(leds as u16).to_le_bytes());
        for i in 0..leds {
            s(&mut b, &format!("LED {i}"));
            b.extend_from_slice(&i.to_le_bytes());
        }
        b.extend_from_slice(&(leds as u16).to_le_bytes());
        for _ in 0..leds {
            b.extend_from_slice(&color.wire());
        }
        let mut out = ((b.len() + 4) as u32).to_le_bytes().to_vec();
        out.extend(b);
        out
    }

    /// A pretend OpenRGB: answers the version, count and descriptions, and
    /// records the colour updates it gets.
    pub fn fake_server(devices: Vec<Vec<u8>>) -> (SocketAddr, std::sync::mpsc::Receiver<(u32, u32, Vec<u8>)>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for conn in l.incoming() {
                let Ok(mut s) = conn else { return };
                let devices = devices.clone();
                let tx = tx.clone();
                std::thread::spawn(move || loop {
                    let mut h = [0u8; 16];
                    if s.read_exact(&mut h).is_err() {
                        return;
                    }
                    assert_eq!(&h[..4], b"ORGB");
                    let dev = u32::from_le_bytes(h[4..8].try_into().unwrap());
                    let id = u32::from_le_bytes(h[8..12].try_into().unwrap());
                    let size = u32::from_le_bytes(h[12..16].try_into().unwrap()) as usize;
                    let mut body = vec![0u8; size];
                    s.read_exact(&mut body).unwrap();
                    let reply = |s: &mut TcpStream, id: u32, b: &[u8]| {
                        s.write_all(&header(dev, id, b.len() as u32)).unwrap();
                        s.write_all(b).unwrap();
                    };
                    match id {
                        REQUEST_PROTOCOL_VERSION => {
                            // A newer server; also says its list changed once.
                            reply(&mut s, DEVICE_LIST_UPDATED, &[]);
                            reply(&mut s, id, &5u32.to_le_bytes());
                        }
                        REQUEST_CONTROLLER_COUNT => reply(&mut s, id, &(devices.len() as u32).to_le_bytes()),
                        REQUEST_CONTROLLER_DATA => {
                            assert_eq!(body, PROTOCOL.to_le_bytes(), "asks for the protocol it reads");
                            reply(&mut s, id, &devices[dev as usize]);
                        }
                        _ => {
                            let _ = tx.send((dev, id, body));
                        }
                    }
                });
            }
        });
        (addr, rx)
    }

    #[test]
    fn reads_devices_and_sets_their_colours() {
        let red = Rgb(255, 0, 0);
        let (addr, rx) = fake_server(vec![
            describe(0, "ASUS ROG STRIX B650E-F", "HID: /dev/hidraw0", &[("Aura Mainboard", 3), ("Aura Addressable 1", 12)], red),
            describe(1, "Kingston Fury DDR5", "I2C: PIIX4, address 0x61", &[("DRAM", 10)], Rgb(0, 0, 255)),
        ]);
        let mut c = Client::connect(addr, Duration::from_secs(2)).unwrap();
        assert_eq!(c.version, PROTOCOL, "the lower of the two versions");
        let list = c.controllers().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].kind, DeviceKind::Motherboard);
        assert_eq!(list[0].name, "ASUS ROG STRIX B650E-F");
        assert_eq!(list[0].vendor, "Vendor");
        assert_eq!(list[0].zones, vec![Zone { name: "Aura Mainboard".into(), leds: 3 }, Zone { name: "Aura Addressable 1".into(), leds: 12 }]);
        assert_eq!(list[0].leds, 15);
        assert_eq!(list[0].colors[0], "#ff0000");
        assert_eq!(list[0].modes.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["Direct", "Static"]);
        assert_eq!(list[1].kind, DeviceKind::Memory);
        assert_eq!(list[1].colors[9], "#0000ff");

        c.take_control(1).unwrap();
        c.set_leds(1, &[Rgb(1, 2, 3); 10]).unwrap();
        let (dev, id, body) = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!((dev, id), (0, SET_CLIENT_NAME));
        assert_eq!(body, b"OmniHub\0");
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap().1, SETCUSTOMMODE);
        let (dev, id, body) = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!((dev, id), (1, UPDATELEDS));
        assert_eq!(body.len(), 4 + 2 + 40);
        assert_eq!(u32::from_le_bytes(body[..4].try_into().unwrap()) as usize, body.len(), "the size counts itself");
        assert_eq!(&body[6..10], &[1, 2, 3, 0], "red first");
        assert_eq!(Rgb::parse("#0a0B0c"), Some(Rgb(10, 11, 12)));
        assert_eq!(Rgb::parse("red"), None);
    }

    #[test]
    fn a_short_description_is_an_error_not_a_panic() {
        let full = describe(2, "GPU", "x", &[("GPU", 4)], Rgb(0, 255, 0));
        for cut in [5, 20, full.len() - 3] {
            assert!(parse_controller(0, &full[4..cut], 3).is_err());
        }
        assert!(parse_controller(0, &full[4..], 3).is_ok());
    }
}
