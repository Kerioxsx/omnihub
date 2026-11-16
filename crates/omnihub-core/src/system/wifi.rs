//! Wi-Fi QR code for guests: the network this PC is on, with its password
//! from Windows' saved profile, as the standard "WIFI:" QR code that phone
//! cameras join with one tap. Shown on the PC only, never sent anywhere.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WifiQr {
    pub ssid: String,
    /// "WPA", "WEP" or "nopass".
    pub security: String,
    pub password: Option<String>,
    /// The QR code as an SVG.
    pub svg: String,
}

/// `netsh wlan show interfaces` → the connected network's name.
pub fn parse_ssid(out: &str) -> Option<String> {
    out.lines().find_map(|l| {
        let (k, v) = l.split_once(':')?;
        (k.trim() == "SSID").then(|| v.trim().to_string()).filter(|s| !s.is_empty())
    })
}

/// `netsh wlan show profile name=… key=clear` → (security, password).
pub fn parse_profile(out: &str) -> (String, Option<String>) {
    let get = |name: &str| out.lines().find_map(|l| l.split_once(':').filter(|(k, _)| k.trim() == name).map(|(_, v)| v.trim().to_string()));
    let auth = get("Authentication").unwrap_or_default().to_lowercase();
    let security = if auth.contains("open") {
        "nopass"
    } else if auth.contains("wep") {
        "WEP"
    } else {
        "WPA"
    };
    (security.into(), get("Key Content").filter(|k| !k.is_empty()))
}

fn escape(s: &str) -> String {
    s.chars().flat_map(|c| if matches!(c, '\\' | ';' | ',' | ':' | '"') { vec!['\\', c] } else { vec![c] }).collect()
}

/// The text a phone camera reads to join a network.
pub fn qr_text(ssid: &str, security: &str, password: Option<&str>) -> String {
    match password {
        Some(p) if security != "nopass" => format!("WIFI:T:{security};S:{};P:{};;", escape(ssid), escape(p)),
        _ => format!("WIFI:T:nopass;S:{};;", escape(ssid)),
    }
}

/// The current network's QR code.
pub fn current() -> Result<WifiQr, String> {
    let ssid = parse_ssid(&netsh(&["wlan", "show", "interfaces"])?).ok_or("This PC isn't on Wi-Fi right now")?;
    let name = format!("name={ssid}");
    let (security, password) = parse_profile(&netsh(&["wlan", "show", "profile", &name, "key=clear"])?);
    let svg = crate::remote::net::qr_svg(&qr_text(&ssid, &security, password.as_deref()));
    Ok(WifiQr { ssid, security, password, svg })
}

#[cfg(windows)]
fn netsh(args: &[&str]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("netsh").args(args).creation_flags(0x0800_0000).output().map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(not(windows))]
fn netsh(_args: &[&str]) -> Result<String, String> {
    Err("Wi-Fi details are read on Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_a_join_code() {
        let ifaces = "    Name                   : Wi-Fi\n    State                  : connected\n    SSID                   : Casa;Lopez\n    BSSID                  : aa:bb\n";
        assert_eq!(parse_ssid(ifaces).as_deref(), Some("Casa;Lopez"));
        let prof = "    Authentication         : WPA2-Personal\n    Cipher                 : CCMP\n    Security key           : Present\n    Key Content            : pa:ss\"word\n";
        let (sec, pw) = parse_profile(prof);
        assert_eq!((sec.as_str(), pw.as_deref()), ("WPA", Some("pa:ss\"word")));
        assert_eq!(qr_text("Casa;Lopez", &sec, pw.as_deref()), r#"WIFI:T:WPA;S:Casa\;Lopez;P:pa\:ss\"word;;"#);
        assert_eq!(qr_text("Cafe", "nopass", None), "WIFI:T:nopass;S:Cafe;;");
        assert_eq!(parse_profile("    Authentication         : Open\n").0, "nopass");
    }
}
