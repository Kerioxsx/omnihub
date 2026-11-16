//! LibreHardwareMonitor's remote web server: the temperatures, fan speeds
//! and fan controls it reads from the motherboard and graphics card (with
//! its own driver), as JSON on a local port (8085 by default), and fan
//! controls set over the same server (`/Sensor?action=Set`). OmniHub
//! loads no driver of its own.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_URL: &str = "http://127.0.0.1:8085";

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SensorKind {
    Temperature,
    Fan,
    /// A fan or pump output, in percent; can be set.
    Control,
    Load,
    Clock,
    Power,
    #[default]
    Other,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sensor {
    /// LibreHardwareMonitor's identifier ("/lpc/nct6799d/0/control/1").
    pub id: String,
    pub name: String,
    pub kind: SensorKind,
    pub value: Option<f32>,
    /// The part it belongs to ("AMD Ryzen 7 7800X3D", "NVIDIA GeForce RTX 4070 SUPER").
    pub hardware: String,
}

/// A number from a formatted value ("45.5 °C", "1 204 RPM", "37,5 %").
pub fn number(s: &str) -> Option<f32> {
    let t: String = s.chars().filter(|c| !c.is_whitespace() && *c != '\u{a0}').take_while(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-')).collect();
    t.replace(',', ".").parse().ok()
}

fn kind(t: &str) -> SensorKind {
    match t {
        "Temperature" => SensorKind::Temperature,
        "Fan" => SensorKind::Fan,
        "Control" => SensorKind::Control,
        "Load" => SensorKind::Load,
        "Clock" => SensorKind::Clock,
        "Power" => SensorKind::Power,
        _ => SensorKind::Other,
    }
}

/// Every sensor in `data.json`'s tree.
pub fn parse(tree: &Value) -> Vec<Sensor> {
    fn walk(n: &Value, hardware: &str, out: &mut Vec<Sensor>) {
        let hw = if n.get("HardwareId").is_some() { n["Text"].as_str().unwrap_or(hardware) } else { hardware };
        if let Some(id) = n.get("SensorId").and_then(Value::as_str) {
            let value = n.get("RawValue").and_then(Value::as_f64).map(|v| v as f32).or_else(|| n.get("Value").and_then(Value::as_str).and_then(number));
            out.push(Sensor { id: id.to_string(), name: n["Text"].as_str().unwrap_or("").to_string(), kind: kind(n["Type"].as_str().unwrap_or("")), value, hardware: hw.to_string() });
        }
        if let Some(children) = n.get("Children").and_then(Value::as_array) {
            for c in children {
                walk(c, hw, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(tree, "", &mut out);
    out
}

/// The processor's temperature: AMD's Tctl/Tdie, Intel's package, else
/// the hottest processor sensor.
pub fn cpu_temp(sensors: &[Sensor]) -> Option<f32> {
    let cpu = |s: &&Sensor| s.kind == SensorKind::Temperature && s.id.contains("cpu") && s.value.is_some();
    let named = |n: &str| sensors.iter().filter(cpu).find(|s| s.name.contains(n)).and_then(|s| s.value);
    named("Tctl/Tdie").or_else(|| named("Package")).or_else(|| sensors.iter().filter(cpu).filter_map(|s| s.value).reduce(f32::max))
}

/// The graphics card's temperature (its core, not the hot spot).
pub fn gpu_temp(sensors: &[Sensor]) -> Option<f32> {
    let gpu = |s: &&Sensor| s.kind == SensorKind::Temperature && s.id.starts_with("/gpu") && s.value.is_some();
    sensors.iter().filter(gpu).find(|s| s.name.contains("Core")).or_else(|| sensors.iter().find(|s| gpu(s))).and_then(|s| s.value)
}

/// A LibreHardwareMonitor server.
#[derive(Debug, Clone)]
pub struct Server {
    pub url: String,
    /// Basic auth, when the server asks for it.
    pub user: Option<(String, String)>,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(3))).http_status_as_error(false).proxy(None).build().into()
}

impl Server {
    fn auth(&self) -> Option<String> {
        use base64::Engine;
        self.user.as_ref().map(|(u, p)| format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("{u}:{p}"))))
    }

    /// Only this PC's own server: the fan controls must not be reachable
    /// from, or sent to, another machine.
    fn local(&self) -> Result<&str, String> {
        let u = self.url.trim_end_matches('/');
        let host = u.strip_prefix("http://").ok_or("LibreHardwareMonitor's address should start with http://")?.split('/').next().unwrap_or("");
        let name = host.rsplit_once(':').map_or(host, |(h, _)| h);
        if matches!(name, "127.0.0.1" | "localhost" | "[::1]") {
            Ok(u)
        } else {
            Err("Use LibreHardwareMonitor on this PC (http://127.0.0.1:8085).".into())
        }
    }

    pub fn sensors(&self) -> Result<Vec<Sensor>, String> {
        let base = self.local()?;
        let mut req = agent().get(format!("{base}/data.json"));
        if let Some(a) = self.auth() {
            req = req.header("Authorization", a);
        }
        let mut res = req.call().map_err(|e| format!("LibreHardwareMonitor isn't answering ({e}). Open it and turn on Options → Remote Web Server."))?;
        match res.status().as_u16() {
            200 => {}
            401 => return Err("LibreHardwareMonitor asks for a user name and password.".into()),
            s => return Err(format!("LibreHardwareMonitor answered {s}.")),
        }
        let body = res.body_mut().with_config().limit(32 << 20).read_to_string().map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
        Ok(parse(&v))
    }

    /// Set a fan control to `percent`, or hand it back to the hardware's
    /// own control (`None`).
    pub fn set(&self, id: &str, percent: Option<f32>) -> Result<(), String> {
        let base = self.local()?;
        let value = percent.map_or_else(|| "null".to_string(), |p| format!("{:.0}", p.clamp(0.0, 100.0)));
        let q = format!("{base}/Sensor?action=Set&id={}&value={value}", percent_encoding::utf8_percent_encode(id, percent_encoding::NON_ALPHANUMERIC));
        let mut req = agent().post(q);
        if let Some(a) = self.auth() {
            req = req.header("Authorization", a);
        }
        let mut res = req.send_empty().map_err(|e| e.to_string())?;
        let v: Value = res.body_mut().read_to_string().ok().and_then(|b| serde_json::from_str(&b).ok()).unwrap_or(Value::Null);
        if v["result"] == "ok" {
            Ok(())
        } else {
            Err(v["message"].as_str().map(|m| m.lines().next().unwrap_or(m).to_string()).unwrap_or_else(|| format!("LibreHardwareMonitor answered {}", res.status())))
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn sample() -> Value {
        let s = |id: &str, text: &str, t: &str, v: &str, raw: f64| serde_json::json!({"id": 1, "Text": text, "SensorId": id, "Type": t, "Value": v, "RawValue": raw, "Children": []});
        serde_json::json!({"id": 0, "Text": "Sensor", "Children": [{"id": 1, "Text": "DESKTOP", "Children": [
            {"id": 2, "Text": "ASUS ROG STRIX B650E-F", "HardwareId": "/motherboard", "Children": [
                {"id": 3, "Text": "Nuvoton NCT6799D", "HardwareId": "/lpc/nct6799d/0", "Children": [
                    {"id": 4, "Text": "Fans", "Children": [s("/lpc/nct6799d/0/fan/1", "CPU Fan", "Fan", "1 204 RPM", 1204.0)]},
                    {"id": 5, "Text": "Controls", "Children": [s("/lpc/nct6799d/0/control/1", "CPU Fan", "Control", "45 %", 45.0), s("/lpc/nct6799d/0/control/2", "Chassis Fan #1", "Control", "38 %", 38.0)]}
                ]}
            ]},
            {"id": 6, "Text": "AMD Ryzen 7 7800X3D", "HardwareId": "/amdcpu/0", "Children": [
                {"id": 7, "Text": "Temperatures", "Children": [s("/amdcpu/0/temperature/2", "Core (Tctl/Tdie)", "Temperature", "61,5 °C", 61.5), s("/amdcpu/0/temperature/3", "CCD1 (Tdie)", "Temperature", "58 °C", 58.0)]}
            ]},
            {"id": 8, "Text": "NVIDIA GeForce RTX 4070 SUPER", "HardwareId": "/gpu-nvidia/0", "Children": [
                {"id": 9, "Text": "Temperatures", "Children": [s("/gpu-nvidia/0/temperature/2", "GPU Hot Spot", "Temperature", "70 °C", 70.0), s("/gpu-nvidia/0/temperature/0", "GPU Core", "Temperature", "62 °C", 62.0)]},
                {"id": 10, "Text": "Controls", "Children": [s("/gpu-nvidia/0/control/1", "GPU Fan", "Control", "30 %", 30.0)]}
            ]}
        ]}]})
    }

    #[test]
    fn reads_temperatures_fans_and_controls() {
        let all = parse(&sample());
        assert_eq!(all.len(), 8);
        let c = all.iter().find(|s| s.id == "/gpu-nvidia/0/control/1").unwrap();
        assert_eq!((c.kind, c.value, c.hardware.as_str()), (SensorKind::Control, Some(30.0), "NVIDIA GeForce RTX 4070 SUPER"));
        assert_eq!(all.iter().find(|s| s.kind == SensorKind::Fan).unwrap().hardware, "Nuvoton NCT6799D");
        assert_eq!(cpu_temp(&all), Some(61.5));
        assert_eq!(gpu_temp(&all), Some(62.0));
        assert_eq!(number("1 204 RPM"), Some(1204.0));
        assert_eq!(number("37,5 %"), Some(37.5));
        assert_eq!(number("n/a"), None);
    }

    #[test]
    fn only_talks_to_this_pc() {
        let ok = Server { url: "http://127.0.0.1:8085/".into(), user: None };
        assert_eq!(ok.local().unwrap(), "http://127.0.0.1:8085");
        assert!(Server { url: "http://localhost:9000".into(), user: None }.local().is_ok());
        assert!(Server { url: "http://192.168.1.20:8085".into(), user: None }.local().is_err());
        assert!(Server { url: "https://127.0.0.1:8085".into(), user: None }.local().is_err());
        assert!(Server { url: "http://127.0.0.1.evil.com:8085".into(), user: None }.local().is_err());
    }
}
