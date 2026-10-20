//! Govee's cloud API (developer.govee.com), for what the LAN API can't do:
//! a light's own scenes, DIY scenes and snapshots, its own music modes
//! (with its microphone), and switches such as gradient or night light.
//! It needs the user's API key (Govee Home app → Profile → Settings →
//! Apply for API key), kept encrypted for their Windows account. Govee
//! allows about two commands a second per light here, so nothing that
//! moves (screen or music sync) goes this way.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const BASE: &str = "https://openapi.api.govee.com";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Capability {
    /// "devices.capabilities.toggle"…
    #[serde(rename = "type")]
    pub kind: String,
    pub instance: String,
    #[serde(default)]
    pub parameters: Value,
}

impl Capability {
    /// The last part of the type: "toggle", "range", "dynamic_scene"…
    pub fn short(&self) -> &str {
        self.kind.rsplit('.').next().unwrap_or(&self.kind)
    }
}

/// A named value a capability takes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExtraKind {
    Toggle,
    Range,
    Choice,
}

/// A setting a light has beyond power, brightness and colour (gradient,
/// night light, a speaker's volume…), shown as the light describes it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Extra {
    pub kind: ExtraKind,
    #[serde(rename = "type")]
    pub cap_type: String,
    pub instance: String,
    pub label: String,
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub unit: Option<String>,
    pub choices: Vec<Choice>,
    /// About sound (volume, speaker, white noise).
    pub sound: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CloudDevice {
    pub sku: String,
    pub device: String,
    pub name: String,
    /// "devices.types.light"…
    pub kind: String,
    pub capabilities: Vec<Capability>,
}

fn options(p: &Value) -> Vec<Choice> {
    p["options"].as_array().into_iter().flatten().filter_map(|o| Some(Choice { name: o["name"].as_str().map(str::to_string).unwrap_or_else(|| o["value"].to_string()), value: o.get("value")?.clone() })).collect()
}

/// "dreamViewToggle" → "DreamView", "nightlightScene" → "Night light scene".
pub fn label(instance: &str) -> String {
    let known = [("dreamView", "DreamView"), ("nightlight", "Night light"), ("hdmi", "HDMI"), ("gradient", "Gradient")];
    let mut rest = instance.trim_end_matches("Toggle");
    let mut prefix = "";
    for (k, v) in known {
        if let Some(r) = rest.strip_prefix(k) {
            prefix = v;
            rest = r;
            break;
        }
    }
    let mut words = String::new();
    for c in rest.chars() {
        if c.is_ascii_uppercase() {
            words.push(' ');
            words.extend(c.to_lowercase());
        } else {
            words.push(c);
        }
    }
    let words = words.trim();
    let out = match (prefix.is_empty(), words.is_empty()) {
        (true, _) => words.to_string(),
        (false, true) => prefix.to_string(),
        (false, false) => format!("{prefix} {words}"),
    };
    let mut c = out.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

impl CloudDevice {
    pub fn cap(&self, instance: &str) -> Option<&Capability> {
        self.capabilities.iter().find(|c| c.instance == instance)
    }

    /// Segments it can colour one by one.
    pub fn segments(&self) -> Option<usize> {
        let c = self.cap("segmentedColorRgb")?;
        let n = c.parameters["fields"].as_array()?.iter().find(|f| f["fieldName"] == "segment")?["options"].as_array()?.len();
        (n > 0).then_some(n)
    }

    pub fn kelvin_range(&self) -> Option<(u32, u32)> {
        let r = &self.cap("colorTemperatureK")?.parameters["range"];
        Some((r["min"].as_u64()? as u32, r["max"].as_u64()? as u32))
    }

    /// The light's own music modes (it listens with its microphone).
    pub fn music_modes(&self) -> Vec<Choice> {
        let Some(c) = self.cap("musicMode") else { return Vec::new() };
        c.parameters["fields"].as_array().into_iter().flatten().find(|f| f["fieldName"] == "musicMode").map(options).unwrap_or_default()
    }

    /// Scenes listed with the device itself (snapshots, sometimes DIY).
    pub fn listed(&self, instance: &str) -> Vec<Choice> {
        self.cap(instance).map(|c| options(&c.parameters)).unwrap_or_default()
    }

    pub fn extras(&self) -> Vec<Extra> {
        let mut out = Vec::new();
        for c in &self.capabilities {
            let p = &c.parameters;
            let sound = ["volume", "sound", "speaker", "noise", "music"].iter().any(|w| c.instance.to_lowercase().contains(w));
            let base = Extra { kind: ExtraKind::Toggle, cap_type: c.kind.clone(), instance: c.instance.clone(), label: label(&c.instance), min: None, max: None, unit: None, choices: Vec::new(), sound };
            match c.short() {
                "toggle" => out.push(base),
                "range" if c.instance != "brightness" => {
                    out.push(Extra { kind: ExtraKind::Range, min: p["range"]["min"].as_i64(), max: p["range"]["max"].as_i64(), unit: p["unit"].as_str().map(|u| u.trim_start_matches("unit.").to_string()), ..base });
                }
                "mode" => {
                    let choices = options(p);
                    if !choices.is_empty() {
                        out.push(Extra { kind: ExtraKind::Choice, choices, ..base });
                    }
                }
                _ => {}
            }
        }
        out
    }
}

fn check(status: u16, body: &str) -> Result<Value, String> {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    // Govee answers 200 with its own code in the body too.
    let code = v["code"].as_u64().map(|c| c as u16).unwrap_or(status);
    match (status, code) {
        (200, 200) => Ok(v),
        (401 | 403, _) | (_, 401 | 403) => Err("Govee didn't accept the API key. Check it in the Govee Home app (Profile → Settings → Apply for API key).".into()),
        (429, _) | (_, 429) => Err("Govee's limit for now: try again in a minute.".into()),
        _ => Err(format!("Govee: {}", v["message"].as_str().or(v["msg"].as_str()).map(str::to_string).unwrap_or_else(|| format!("error {status}")))),
    }
}

pub fn parse_devices(v: &Value) -> Vec<CloudDevice> {
    v["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|d| {
            Some(CloudDevice {
                sku: d["sku"].as_str()?.to_string(),
                device: d["device"].as_str()?.to_string(),
                name: d["deviceName"].as_str().unwrap_or_default().to_string(),
                kind: d["type"].as_str().unwrap_or_default().to_string(),
                capabilities: serde_json::from_value(d["capabilities"].clone()).unwrap_or_default(),
            })
        })
        .collect()
}

/// Each capability's current value.
pub fn parse_state(v: &Value) -> Vec<(String, Value)> {
    v["payload"]["capabilities"].as_array().into_iter().flatten().filter_map(|c| Some((c["instance"].as_str()?.to_string(), c["state"]["value"].clone()))).collect()
}

pub fn parse_scenes(v: &Value) -> Vec<Choice> {
    v["payload"]["capabilities"].as_array().into_iter().flatten().flat_map(|c| options(&c["parameters"])).collect()
}

fn payload(sku: &str, device: &str, extra: Option<Value>) -> Value {
    let mut p = json!({ "sku": sku, "device": device });
    if let Some(Value::Object(m)) = extra {
        for (k, v) in m {
            p[k] = v;
        }
    }
    json!({ "requestId": uuid::Uuid::new_v4().to_string(), "payload": p })
}

pub fn control_body(sku: &str, device: &str, cap_type: &str, instance: &str, value: Value) -> Value {
    payload(sku, device, Some(json!({ "capability": { "type": cap_type, "instance": instance, "value": value } })))
}

pub struct Cloud {
    base: String,
    key: zeroize::Zeroizing<String>,
    agent: ureq::Agent,
}

impl Cloud {
    pub fn new(key: &str) -> Cloud {
        Cloud::with_base(BASE, key)
    }

    pub fn with_base(base: &str, key: &str) -> Cloud {
        let mut cfg = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).http_status_as_error(false);
        if base.starts_with("http://127.0.0.1") {
            cfg = cfg.proxy(None);
        }
        Cloud { base: base.trim_end_matches('/').to_string(), key: zeroize::Zeroizing::new(key.trim().to_string()), agent: cfg.build().into() }
    }

    fn call(&self, path: &str, body: Option<&Value>) -> Result<Value, String> {
        let url = format!("{}{path}", self.base);
        let res = match body {
            Some(b) => self.agent.post(&url).header("Govee-API-Key", self.key.as_str()).header("Content-Type", "application/json").send(b.to_string()),
            None => self.agent.get(&url).header("Govee-API-Key", self.key.as_str()).call(),
        };
        let mut res = res.map_err(|e| format!("Couldn't reach Govee: {e}"))?;
        let status = res.status().as_u16();
        let text = res.body_mut().read_to_string().unwrap_or_default();
        check(status, &text)
    }

    pub fn devices(&self) -> Result<Vec<CloudDevice>, String> {
        self.call("/router/api/v1/user/devices", None).map(|v| parse_devices(&v))
    }

    pub fn state(&self, d: &CloudDevice) -> Result<Vec<(String, Value)>, String> {
        self.call("/router/api/v1/device/state", Some(&payload(&d.sku, &d.device, None))).map(|v| parse_state(&v))
    }

    pub fn scenes(&self, d: &CloudDevice) -> Result<Vec<Choice>, String> {
        self.call("/router/api/v1/device/scenes", Some(&payload(&d.sku, &d.device, None))).map(|v| parse_scenes(&v))
    }

    pub fn diy_scenes(&self, d: &CloudDevice) -> Result<Vec<Choice>, String> {
        self.call("/router/api/v1/device/diy-scenes", Some(&payload(&d.sku, &d.device, None))).map(|v| parse_scenes(&v))
    }

    pub fn control(&self, d: &CloudDevice, cap_type: &str, instance: &str, value: Value) -> Result<(), String> {
        self.call("/router/api/v1/device/control", Some(&control_body(&d.sku, &d.device, cap_type, instance, value))).map(|_| ())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::sync::{Arc, Mutex};

    pub const DEVICES: &str = r#"{"code":200,"message":"success","data":[
      {"sku":"H6199","device":"AA:BB:CC:DD:EE:FF:00:11","deviceName":"TV backlight","type":"devices.types.light","capabilities":[
        {"type":"devices.capabilities.on_off","instance":"powerSwitch","parameters":{"dataType":"ENUM","options":[{"name":"on","value":1},{"name":"off","value":0}]}},
        {"type":"devices.capabilities.toggle","instance":"gradientToggle","parameters":{"dataType":"ENUM","options":[{"name":"on","value":1},{"name":"off","value":0}]}},
        {"type":"devices.capabilities.toggle","instance":"dreamViewToggle","parameters":{"dataType":"ENUM","options":[{"name":"on","value":1},{"name":"off","value":0}]}},
        {"type":"devices.capabilities.range","instance":"brightness","parameters":{"unit":"unit.percent","dataType":"INTEGER","range":{"min":1,"max":100,"precision":1}}},
        {"type":"devices.capabilities.range","instance":"volume","parameters":{"unit":"unit.percent","dataType":"INTEGER","range":{"min":0,"max":100,"precision":1}}},
        {"type":"devices.capabilities.segment_color_setting","instance":"segmentedColorRgb","parameters":{"dataType":"STRUCT","fields":[{"fieldName":"segment","dataType":"Array","options":[{"value":0},{"value":1},{"value":2},{"value":3},{"value":4},{"value":5},{"value":6},{"value":7},{"value":8},{"value":9},{"value":10},{"value":11},{"value":12},{"value":13},{"value":14}],"required":true},{"fieldName":"rgb","dataType":"INTEGER","range":{"min":0,"max":16777215,"precision":1},"required":true}]}},
        {"type":"devices.capabilities.color_setting","instance":"colorRgb","parameters":{"dataType":"INTEGER","range":{"min":0,"max":16777215,"precision":1}}},
        {"type":"devices.capabilities.color_setting","instance":"colorTemperatureK","parameters":{"dataType":"INTEGER","range":{"min":2000,"max":9000,"precision":1}}},
        {"type":"devices.capabilities.dynamic_scene","instance":"lightScene","parameters":{"dataType":"ENUM","options":[]}},
        {"type":"devices.capabilities.music_setting","instance":"musicMode","parameters":{"dataType":"STRUCT","fields":[{"fieldName":"musicMode","dataType":"ENUM","options":[{"name":"Energic","value":5},{"name":"Rhythm","value":3}],"required":true},{"fieldName":"sensitivity","dataType":"INTEGER","range":{"min":0,"max":100,"precision":1},"required":true}]}},
        {"type":"devices.capabilities.dynamic_scene","instance":"snapshot","parameters":{"dataType":"ENUM","options":[{"name":"Movie night","value":0}]}},
        {"type":"devices.capabilities.mode","instance":"nightlightScene","parameters":{"dataType":"ENUM","options":[{"name":"Forest","value":1},{"name":"Ocean","value":2}]}}
      ]},
      {"sku":"H6008","device":"11:22:33:44:55:66:77:88","deviceName":"Desk bulb","type":"devices.types.light","capabilities":[]},
      {"bad":"entry"}
    ]}"#;

    pub const SCENES: &str = r#"{"requestId":"x","msg":"success","code":200,"payload":{"sku":"H6199","device":"AA","capabilities":[{"type":"devices.capabilities.dynamic_scene","instance":"lightScene","parameters":{"dataType":"ENUM","options":[{"name":"Sunrise","value":{"paramId":1681,"id":1606}},{"name":"Aurora","value":{"paramId":1690,"id":1615}}]}}]}}"#;

    #[test]
    fn reads_devices_and_what_they_can_do() {
        let list = parse_devices(&serde_json::from_str(DEVICES).unwrap());
        assert_eq!(list.len(), 2, "the broken entry is skipped");
        let tv = &list[0];
        assert_eq!((tv.name.as_str(), tv.segments(), tv.kelvin_range()), ("TV backlight", Some(15), Some((2000, 9000))));
        assert_eq!(tv.music_modes().iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Energic", "Rhythm"]);
        assert_eq!(tv.listed("snapshot")[0].name, "Movie night");
        let extras = tv.extras();
        let labels: Vec<&str> = extras.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(labels, ["Gradient", "DreamView", "Volume", "Night light scene"]);
        let vol = extras.iter().find(|e| e.instance == "volume").unwrap();
        assert!(vol.sound && vol.kind == ExtraKind::Range && vol.max == Some(100) && vol.unit.as_deref() == Some("percent"));
        assert_eq!(extras[3].choices.len(), 2);
        assert_eq!(list[1].segments(), None);
        let scenes = parse_scenes(&serde_json::from_str(SCENES).unwrap());
        assert_eq!(scenes[1].name, "Aurora");
        assert_eq!(scenes[1].value["id"], 1615);
        assert_eq!(label("warmMistToggle"), "Warm mist");
        assert_eq!(label("brightness"), "Brightness");
        assert_eq!(label("hdmiSource"), "HDMI source");
    }

    #[test]
    fn errors_in_plain_words() {
        assert!(check(401, "{}").unwrap_err().contains("API key"));
        assert!(check(200, r#"{"code":429,"message":"too many"}"#).unwrap_err().contains("limit"));
        assert_eq!(check(400, r#"{"code":400,"message":"devices not exist"}"#).unwrap_err(), "Govee: devices not exist");
        assert!(check(200, r#"{"code":200}"#).is_ok());
    }

    /// A pretend Govee cloud: answers with `body` and keeps each request.
    fn serve(body: &'static str) -> (String, Arc<Mutex<Vec<String>>>) {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", l.local_addr().unwrap().port());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let log = seen.clone();
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                let mut r = BufReader::new(s.try_clone().unwrap());
                let mut head = String::new();
                let mut len = 0;
                loop {
                    let mut line = String::new();
                    if r.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                    head.push_str(&line);
                }
                let mut b = vec![0; len];
                let _ = r.read_exact(&mut b);
                log.lock().unwrap().push(format!("{head}\n{}", String::from_utf8_lossy(&b)));
                let mut s = s;
                let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        (base, seen)
    }

    #[test]
    fn talks_to_the_cloud_with_the_key() {
        let (base, seen) = serve(DEVICES);
        let c = Cloud::with_base(&base, " my-key ");
        let list = c.devices().unwrap();
        assert_eq!(list.len(), 2);
        c.control(&list[0], "devices.capabilities.toggle", "gradientToggle", json!(1)).unwrap();
        let log = seen.lock().unwrap();
        assert!(log[0].starts_with("GET /router/api/v1/user/devices") && log[0].to_lowercase().contains("govee-api-key: my-key"), "{}", log[0]);
        assert!(log[1].starts_with("POST /router/api/v1/device/control"));
        let body: Value = serde_json::from_str(log[1].rsplit('\n').next().unwrap()).unwrap();
        assert_eq!(body["payload"]["capability"], json!({ "type": "devices.capabilities.toggle", "instance": "gradientToggle", "value": 1 }));
        assert_eq!(body["payload"]["sku"], "H6199");
    }
}
