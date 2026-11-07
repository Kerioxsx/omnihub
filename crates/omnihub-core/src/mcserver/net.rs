//! Letting friends in: this PC's addresses on the home network, its public
//! address, and opening the server's port on the router with UPnP (when
//! the router allows it). Also spots when the internet provider shares one
//! public address between many homes (CGNAT), where a port can't be opened.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use serde::Serialize;

pub const IPIFY: &str = "https://api.ipify.org";
pub const SSDP: &str = "239.255.255.250:1900";

/// This PC's IPv4 addresses on its networks (not loopback or link-local).
pub fn lan_ips() -> Vec<Ipv4Addr> {
    let mut v: Vec<Ipv4Addr> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|i| match i.ip() {
            std::net::IpAddr::V4(a) if !a.is_loopback() && !a.is_link_local() && !a.is_unspecified() => Some(a),
            _ => None,
        })
        .collect();
    v.sort_by_key(|a| (!a.is_private(), a.octets()));
    v.dedup();
    v
}

/// The internet provider's shared range (100.64.0.0/10) or a private one: not reachable from outside.
pub fn not_public(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    ip.is_private() || ip.is_loopback() || ip.is_link_local() || (o[0] == 100 && (64..128).contains(&o[1]))
}

/// The address the internet sees this PC at.
pub fn public_ip(agent: &ureq::Agent, url: &str) -> Result<Ipv4Addr, String> {
    let mut res = agent.get(url).header("User-Agent", super::install::ua()).call().map_err(|e| format!("Couldn't look up the public address: {e}"))?;
    let t = res.body_mut().read_to_string().map_err(|e| e.to_string())?;
    t.trim().parse().map_err(|_| "The public address couldn't be read.".to_string())
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Gateway {
    pub control: String,
    pub service: String,
    /// This PC's address towards the router.
    pub local: String,
    pub name: String,
}

fn between<'a>(s: &'a str, a: &str, b: &str) -> Option<&'a str> {
    let start = s.find(a)? + a.len();
    let end = s[start..].find(b)? + start;
    Some(&s[start..end])
}

/// The scheme, host and port of an address ("http://192.168.1.1:5000").
fn origin(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("http://")?;
    let end = rest.find('/').unwrap_or(rest.len());
    Some(&url[..7 + end])
}

/// Find the router's port-forwarding service (UPnP IGD) on the network.
pub fn discover(agent: &ureq::Agent, target: &str, wait: Duration) -> Result<Gateway, String> {
    let sock = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    sock.set_read_timeout(Some(Duration::from_millis(250))).map_err(|e| e.to_string())?;
    let mut locations = Vec::new();
    let until = Instant::now() + wait;
    let mut sent = false;
    while Instant::now() < until {
        if !sent {
            for st in ["urn:schemas-upnp-org:device:InternetGatewayDevice:1", "urn:schemas-upnp-org:device:InternetGatewayDevice:2", "urn:schemas-upnp-org:service:WANIPConnection:1"] {
                let msg = format!("M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: {st}\r\n\r\n");
                let _ = sock.send_to(msg.as_bytes(), target);
            }
            sent = true;
        }
        let mut buf = [0u8; 2048];
        if let Ok((n, _)) = sock.recv_from(&mut buf) {
            let text = String::from_utf8_lossy(&buf[..n]);
            if let Some(loc) = text.lines().find_map(|l| l.split_once(':').filter(|(k, _)| k.trim().eq_ignore_ascii_case("location")).map(|(_, v)| v.trim().to_string())) {
                if loc.starts_with("http://") && !locations.contains(&loc) {
                    locations.push(loc);
                    // One answer is usually all there is; give others a moment.
                    if locations.len() == 1 {
                        let _ = sock.set_read_timeout(Some(Duration::from_millis(150)));
                    }
                }
            }
        } else if !locations.is_empty() {
            break;
        }
    }
    let mut last = "No router on this network answered (UPnP is off on it, or it doesn't have it).".to_string();
    for loc in locations {
        match describe(agent, &loc) {
            Ok(g) => return Ok(g),
            Err(e) => last = e,
        }
    }
    Err(last)
}

fn describe(agent: &ureq::Agent, loc: &str) -> Result<Gateway, String> {
    let xml = agent.get(loc).call().map_err(|e| format!("The router didn't describe itself: {e}"))?.body_mut().read_to_string().map_err(|e| e.to_string())?;
    let name = between(&xml, "<friendlyName>", "</friendlyName>").unwrap_or("Router").trim().to_string();
    let base = between(&xml, "<URLBase>", "</URLBase>").map(|s| s.trim().trim_end_matches('/').to_string()).or_else(|| origin(loc).map(String::from)).ok_or("The router's address is odd.")?;
    for block in xml.split("<service>").skip(1) {
        let block = block.split("</service>").next().unwrap_or(block);
        let Some(st) = between(block, "<serviceType>", "</serviceType>").map(str::trim) else { continue };
        if !(st.starts_with("urn:schemas-upnp-org:service:WANIPConnection:") || st.starts_with("urn:schemas-upnp-org:service:WANPPPConnection:")) {
            continue;
        }
        let Some(ctl) = between(block, "<controlURL>", "</controlURL>").map(str::trim) else { continue };
        let control = if ctl.starts_with("http://") { ctl.to_string() } else { format!("{base}/{}", ctl.trim_start_matches('/')) };
        let host: SocketAddr = control.trim_start_matches("http://").split('/').next().and_then(|h| std::net::ToSocketAddrs::to_socket_addrs(h).ok()?.next()).ok_or("The router's address is odd.")?;
        let s = UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
        s.connect(host).map_err(|e| e.to_string())?;
        let local = s.local_addr().map_err(|e| e.to_string())?.ip().to_string();
        return Ok(Gateway { control, service: st.to_string(), local, name });
    }
    Err("The router doesn't offer port forwarding over UPnP.".into())
}

impl Gateway {
    fn soap(&self, agent: &ureq::Agent, action: &str, args: &[(&str, String)]) -> Result<String, String> {
        let body: String = args.iter().map(|(k, v)| format!("<{k}>{v}</{k}>")).collect();
        let env = format!("<?xml version=\"1.0\"?><s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"><s:Body><u:{action} xmlns:u=\"{}\">{body}</u:{action}></s:Body></s:Envelope>", self.service);
        let mut res = agent
            .post(&self.control)
            .header("Content-Type", "text/xml; charset=\"utf-8\"")
            .header("SOAPAction", &format!("\"{}#{action}\"", self.service))
            .send(env)
            .map_err(|e| format!("The router didn't answer: {e}"))?;
        let status = res.status().as_u16();
        let text = res.body_mut().read_to_string().unwrap_or_default();
        if !(200..300).contains(&status) {
            let why = between(&text, "<errorDescription>", "</errorDescription>").map(str::trim).unwrap_or("").to_string();
            let code = between(&text, "<errorCode>", "</errorCode>").map(str::trim).unwrap_or("");
            return Err(match code {
                "718" => "The router already forwards that port to another device.".to_string(),
                "725" => "only-permanent".to_string(),
                _ if why.is_empty() => format!("The router refused (error {status})."),
                _ => format!("The router refused: {why}."),
            });
        }
        Ok(text)
    }

    pub fn external_ip(&self, agent: &ureq::Agent) -> Result<Ipv4Addr, String> {
        let t = self.soap(agent, "GetExternalIPAddress", &[])?;
        between(&t, "<NewExternalIPAddress>", "</NewExternalIPAddress>").and_then(|s| s.trim().parse().ok()).ok_or_else(|| "The router didn't say its address.".into())
    }

    /// Forward `port` (TCP) to this PC.
    pub fn open(&self, agent: &ureq::Agent, port: u16, what: &str) -> Result<(), String> {
        let args = |lease: u32| {
            vec![
                ("NewRemoteHost", String::new()),
                ("NewExternalPort", port.to_string()),
                ("NewProtocol", "TCP".to_string()),
                ("NewInternalPort", port.to_string()),
                ("NewInternalClient", self.local.clone()),
                ("NewEnabled", "1".to_string()),
                ("NewPortMappingDescription", what.chars().filter(|c| c.is_ascii_alphanumeric() || *c == ' ').take(40).collect()),
                ("NewLeaseDuration", lease.to_string()),
            ]
        };
        // A day at a time (renewed), or for good on routers that only take that.
        match self.soap(agent, "AddPortMapping", &args(86_400)) {
            Err(e) if e == "only-permanent" => self.soap(agent, "AddPortMapping", &args(0)).map(|_| ()),
            r => r.map(|_| ()),
        }
    }

    pub fn close(&self, agent: &ureq::Agent, port: u16) -> Result<(), String> {
        self.soap(agent, "DeletePortMapping", &[("NewRemoteHost", String::new()), ("NewExternalPort", port.to_string()), ("NewProtocol", "TCP".to_string())]).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mods::tests::{serve, Seen};

    #[test]
    fn opens_a_port_on_a_pretend_router() {
        let http = serve(Box::new(|r: &Seen| {
            let ok = |s: &str| Some((200, "text/xml", s.as_bytes().to_vec()));
            match r.path.as_str() {
                "/desc.xml" => ok("<root><device><friendlyName>Test Router</friendlyName><serviceList><service><serviceType>urn:schemas-upnp-org:service:Layer3Forwarding:1</serviceType><controlURL>/l3</controlURL></service><service><serviceType>urn:schemas-upnp-org:service:WANIPConnection:1</serviceType><controlURL>/ctl/IPConn</controlURL></service></serviceList></device></root>"),
                "/ctl/IPConn" => {
                    let body = String::from_utf8_lossy(&r.body).to_string();
                    let action = r.headers.iter().find(|(k, _)| k == "soapaction").map(|(_, v)| v.clone()).unwrap_or_default();
                    if action.ends_with("#GetExternalIPAddress\"") {
                        return ok("<s:Envelope><s:Body><u:GetExternalIPAddressResponse><NewExternalIPAddress>100.72.1.9</NewExternalIPAddress></u:GetExternalIPAddressResponse></s:Body></s:Envelope>");
                    }
                    if action.ends_with("#AddPortMapping\"") && body.contains("<NewLeaseDuration>86400</NewLeaseDuration>") {
                        return Some((500, "text/xml", b"<s:Envelope><s:Body><s:Fault><detail><UPnPError><errorCode>725</errorCode><errorDescription>OnlyPermanentLeasesSupported</errorDescription></UPnPError></detail></s:Fault></s:Body></s:Envelope>".to_vec()));
                    }
                    if action.ends_with("#AddPortMapping\"") {
                        assert!(body.contains("<NewExternalPort>25565</NewExternalPort>") && body.contains("<NewInternalClient>127.0.0.1</NewInternalClient>") && body.contains("<NewLeaseDuration>0</NewLeaseDuration>"), "{body}");
                        return ok("<ok/>");
                    }
                    if action.ends_with("#DeletePortMapping\"") {
                        return ok("<ok/>");
                    }
                    None
                }
                _ => None,
            }
        }));
        // A pretend SSDP answer from 127.0.0.1.
        let ssdp = UdpSocket::bind("127.0.0.1:0").unwrap();
        let target = ssdp.local_addr().unwrap().to_string();
        let loc = format!("{}/desc.xml", http.base);
        std::thread::spawn(move || {
            let mut buf = [0u8; 2048];
            if let Ok((n, from)) = ssdp.recv_from(&mut buf) {
                assert!(String::from_utf8_lossy(&buf[..n]).starts_with("M-SEARCH"));
                let _ = ssdp.send_to(format!("HTTP/1.1 200 OK\r\nCACHE-CONTROL: max-age=120\r\nLOCATION: {loc}\r\nST: urn:schemas-upnp-org:device:InternetGatewayDevice:1\r\n\r\n").as_bytes(), from);
            }
        });
        let agent: ureq::Agent = ureq::Agent::config_builder().proxy(None).http_status_as_error(false).build().into();
        let g = discover(&agent, &target, Duration::from_secs(3)).unwrap();
        assert_eq!((g.name.as_str(), g.local.as_str()), ("Test Router", "127.0.0.1"));
        assert!(g.control.ends_with("/ctl/IPConn"));
        let ip = g.external_ip(&agent).unwrap();
        assert!(not_public(ip), "100.72.x.x is the provider's shared range");
        g.open(&agent, 25565, "OmniHub Minecraft").unwrap();
        g.close(&agent, 25565).unwrap();
        assert!(!not_public("8.8.8.8".parse().unwrap()) && not_public("192.168.1.4".parse().unwrap()));
        assert_eq!(origin("http://192.168.1.1:5000/rootDesc.xml"), Some("http://192.168.1.1:5000"));
    }
}
