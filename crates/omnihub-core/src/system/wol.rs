//! What a phone's Wake-on-LAN app needs to wake this PC from sleep: the
//! network card's MAC address and the network's broadcast address. (A
//! sleeping PC can't run OmniHub's server, so the phone's browser can't wake
//! it; a Wake-on-LAN app sends the "magic packet" the network card listens
//! for, once waking is allowed in the card's settings.)

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WakeTarget {
    /// The adapter's name ("Ethernet", "Wi-Fi").
    pub name: String,
    /// "AA-BB-CC-DD-EE-FF".
    pub mac: String,
    pub ip: String,
    pub broadcast: String,
    /// Waking from sleep over Wi-Fi works on few PCs; a cable is reliable.
    pub wifi: bool,
}

fn is_mac(w: &str) -> bool {
    let p: Vec<&str> = w.split('-').collect();
    p.len() == 6 && p.iter().all(|x| x.len() == 2 && x.chars().all(|c| c.is_ascii_hexdigit())) && w != "00-00-00-00-00-00"
}

fn ipv4(w: &str) -> Option<[u8; 4]> {
    let w = w.trim_end_matches(|c: char| !c.is_ascii_digit());
    let p: Vec<u8> = w.split('.').filter_map(|x| x.parse().ok()).collect();
    (p.len() == 4 && w.split('.').count() == 4).then(|| [p[0], p[1], p[2], p[3]])
}

/// Read `ipconfig /all` (its labels are translated, the values aren't): each
/// adapter with a MAC and an IPv4 address, with its broadcast address.
pub fn parse_ipconfig(text: &str) -> Vec<WakeTarget> {
    let mut out = Vec::new();
    let mut block: Vec<&str> = Vec::new();
    let mut name = String::new();
    let flush = |name: &str, block: &[&str], out: &mut Vec<WakeTarget>| {
        let words: Vec<&str> = block.iter().flat_map(|l| l.split(|c: char| c.is_whitespace() || c == '(' || c == ')')).collect();
        let Some(mac) = words.iter().find(|w| is_mac(w)) else { return };
        let ips: Vec<[u8; 4]> = words.iter().filter_map(|w| ipv4(w)).collect();
        let Some(ip) = ips.iter().find(|ip| ip[0] != 255 && ip[0] != 0 && ip[0] != 169 && ip[0] != 127) else { return };
        let mask = ips.iter().find(|m| m[0] == 255).copied().unwrap_or([255, 255, 255, 0]);
        let b: Vec<String> = (0..4).map(|i| (ip[i] | !mask[i]).to_string()).collect();
        let lower = name.to_lowercase();
        out.push(WakeTarget {
            name: name.trim_end_matches(':').trim().to_string(),
            mac: mac.to_uppercase(),
            ip: ip.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("."),
            broadcast: b.join("."),
            wifi: lower.contains("wi-fi") || lower.contains("wireless") || lower.contains("wlan"),
        });
    };
    for line in text.lines() {
        if !line.starts_with(' ') && line.trim_end().ends_with(':') {
            flush(&name, &block, &mut out);
            block.clear();
            // "Ethernet adapter Ethernet:" → the last part is the connection's name.
            name = line.trim_end().trim_end_matches(':').rsplit(" adapter ").next().unwrap_or(line).to_string();
        } else if line.starts_with(' ') {
            block.push(line);
        }
    }
    flush(&name, &block, &mut out);
    // Cable first.
    out.sort_by_key(|t| t.wifi);
    out
}

pub fn targets() -> Vec<WakeTarget> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let out = std::process::Command::new("ipconfig").arg("/all").creation_flags(0x0800_0000).output();
        out.map(|o| parse_ipconfig(&String::from_utf8_lossy(&o.stdout))).unwrap_or_default()
    }
    #[cfg(not(windows))]
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
Windows IP Configuration

   Host Name . . . . . . . . . . . . : GAMING-PC

Ethernet adapter Ethernet:

   Description . . . . . . . . . . . : Intel(R) Ethernet Controller I225-V
   Physical Address. . . . . . . . . : 0C-9D-92-AA-BB-CC
   DHCP Enabled. . . . . . . . . . . : Yes
   IPv4 Address. . . . . . . . . . . : 192.168.1.24(Preferred)
   Subnet Mask . . . . . . . . . . . : 255.255.255.0
   Default Gateway . . . . . . . . . : 192.168.1.1

Wireless LAN adapter Wi-Fi:

   Media State . . . . . . . . . . . : Media disconnected
   Physical Address. . . . . . . . . : 3C-21-9C-11-22-33

Ethernet adapter vEthernet (WSL):

   Physical Address. . . . . . . . . : 00-15-5D-01-02-03
   Dirección IPv4. . . . . . . . . . : 172.20.0.1(Preferido)
   Máscara de subred . . . . . . . . : 255.255.240.0
";

    #[test]
    fn reads_adapters_in_any_language() {
        let t = parse_ipconfig(SAMPLE);
        assert_eq!(t.len(), 2, "the disconnected Wi-Fi has no address");
        assert_eq!((t[0].name.as_str(), t[0].mac.as_str(), t[0].ip.as_str(), t[0].broadcast.as_str()), ("Ethernet", "0C-9D-92-AA-BB-CC", "192.168.1.24", "192.168.1.255"));
        assert_eq!(t[1].broadcast, "172.20.15.255", "a /20 network");
        assert!(!t[0].wifi);
        assert!(is_mac("0C-9D-92-AA-BB-CC") && !is_mac("00-00-00-00-00-00") && !is_mac("12-34"));
    }
}
