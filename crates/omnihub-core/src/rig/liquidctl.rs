//! liquidctl, the open-source command-line tool for liquid coolers and fan
//! hubs (NZXT Kraken and Smart Device, Corsair Hydro and Commander, ASUS
//! Ryujin, Lian Li…). OmniHub runs it when it is installed (it needs
//! Python: `pip install liquidctl`) for pump and fan profiles, which the
//! coolers then run themselves, and for the LCD screens of the Kraken Z,
//! 2023 and 2024 models.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LqDevice {
    pub description: String,
    pub vendor_id: u16,
    pub product_id: u16,
    /// What `--address` takes.
    pub address: String,
    pub driver: String,
    /// It has an LCD screen liquidctl can set.
    pub lcd: bool,
    /// Liquid temperature, fan and pump speeds, as liquidctl reports them.
    pub status: Vec<Reading>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    pub key: String,
    pub value: f64,
    pub unit: String,
}

/// liquidctl's own program, when it is installed.
pub fn find(chosen: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = chosen.map(PathBuf::from).filter(|p| p.is_file()) {
        return Some(p);
    }
    let exe = if cfg!(windows) { "liquidctl.exe" } else { "liquidctl" };
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    // pip's user installs: %APPDATA%\Python\Python3xx\Scripts.
    if let Some(app) = std::env::var_os("APPDATA").map(PathBuf::from) {
        if let Ok(rd) = std::fs::read_dir(app.join("Python")) {
            dirs.extend(rd.flatten().map(|e| e.path().join("Scripts")));
        }
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        if let Ok(rd) = std::fs::read_dir(local.join("Programs").join("Python")) {
            dirs.extend(rd.flatten().map(|e| e.path().join("Scripts")));
        }
    }
    dirs.into_iter().map(|d| d.join(exe)).find(|p| p.is_file())
}

/// LCD models (Kraken Z, 2023, 2024 Elite RGB).
pub fn has_lcd(vendor: u16, product: u16) -> bool {
    vendor == 0x1E71 && matches!(product, 0x3008 | 0x300C | 0x300E | 0x3012)
}

pub fn parse_list(v: &Value) -> Vec<LqDevice> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|d| {
                    let vendor_id = d["vendor_id"].as_u64().unwrap_or(0) as u16;
                    let product_id = d["product_id"].as_u64().unwrap_or(0) as u16;
                    LqDevice {
                        description: d["description"].as_str().unwrap_or("").to_string(),
                        vendor_id,
                        product_id,
                        address: d["address"].as_str().unwrap_or("").to_string(),
                        driver: d["driver"].as_str().unwrap_or("").to_string(),
                        lcd: has_lcd(vendor_id, product_id),
                        status: Vec::new(),
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Readings by device address, from `status --json`.
pub fn parse_status(v: &Value) -> Vec<(String, Vec<Reading>)> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|d| {
                    let readings = d["status"]
                        .as_array()
                        .map(|s| s.iter().filter_map(|r| Some(Reading { key: r["key"].as_str()?.to_string(), value: r["value"].as_f64()?, unit: r["unit"].as_str().unwrap_or("").to_string() })).collect())
                        .unwrap_or_default();
                    (d["address"].as_str().unwrap_or("").to_string(), readings)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// "20 50 30 60 …": temperature–duty pairs for `set … speed`.
pub fn pairs(curve: &[(f32, f32)]) -> Vec<String> {
    curve.iter().flat_map(|(t, d)| [format!("{t:.0}"), format!("{:.0}", d.clamp(0.0, 100.0))]).collect()
}

pub struct Liquidctl {
    pub exe: PathBuf,
}

impl Liquidctl {
    fn run(&self, args: &[&str]) -> Result<String, String> {
        let mut c = Command::new(&self.exe);
        c.args(args).env("LANG", "C");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let out = c.output().map_err(|e| format!("liquidctl did not start: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            let err = String::from_utf8_lossy(&out.stderr);
            Err(err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("liquidctl failed").trim().to_string())
        }
    }

    pub fn list(&self) -> Result<Vec<LqDevice>, String> {
        let v: Value = serde_json::from_str(&self.run(&["list", "--json"])?).map_err(|e| e.to_string())?;
        Ok(parse_list(&v))
    }

    /// Devices need this once after the PC starts.
    pub fn initialize(&self) -> Result<(), String> {
        self.run(&["initialize", "all"]).map(|_| ())
    }

    pub fn status(&self) -> Result<Vec<(String, Vec<Reading>)>, String> {
        let v: Value = serde_json::from_str(&self.run(&["status", "--json"])?).map_err(|e| e.to_string())?;
        Ok(parse_status(&v))
    }

    /// A temperature profile (or one duty for every temperature) on a
    /// channel ("fan", "pump"); the device runs it itself.
    pub fn set_speed(&self, address: &str, channel: &str, curve: &[(f32, f32)]) -> Result<(), String> {
        let mut args = vec!["--address".to_string(), address.to_string(), "set".into(), channel.into(), "speed".into()];
        args.extend(pairs(curve));
        self.run(&args.iter().map(String::as_str).collect::<Vec<_>>()).map(|_| ())
    }

    pub fn set_duty(&self, address: &str, channel: &str, duty: f32) -> Result<(), String> {
        self.run(&["--address", address, "set", channel, "speed", &format!("{:.0}", duty.clamp(0.0, 100.0))]).map(|_| ())
    }

    /// The LCD: a picture, an animated GIF, or the liquid temperature.
    pub fn set_lcd(&self, address: &str, what: LcdShow<'_>) -> Result<(), String> {
        match what {
            LcdShow::Picture(p) => self.run(&["--address", address, "set", "lcd", "screen", "static", &p.to_string_lossy()]),
            LcdShow::Gif(p) => self.run(&["--address", address, "set", "lcd", "screen", "gif", &p.to_string_lossy()]),
            LcdShow::Liquid => self.run(&["--address", address, "set", "lcd", "screen", "liquid"]),
            LcdShow::Brightness(b) => self.run(&["--address", address, "set", "lcd", "screen", "brightness", &b.min(100).to_string()]),
        }
        .map(|_| ())
    }
}

pub enum LcdShow<'a> {
    Picture(&'a Path),
    Gif(&'a Path),
    Liquid,
    Brightness(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_its_json() {
        let list = serde_json::json!([{"description": "NZXT Kraken 2023", "vendor_id": 7793, "product_id": 12302, "release_number": 512, "serial_number": "x", "bus": "hid", "address": "\\\\?\\HID#VID_1E71&PID_300E", "port": null, "driver": "KrakenZ3"}]);
        let d = parse_list(&list);
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].vendor_id, d[0].product_id, d[0].lcd), (0x1E71, 0x300E, true));
        let status = serde_json::json!([{"bus": "hid", "address": "a", "description": "x", "status": [{"key": "Liquid temperature", "value": 30.1, "unit": "°C"}, {"key": "Pump speed", "value": 2100, "unit": "rpm"}]}]);
        let s = parse_status(&status);
        assert_eq!(s[0].0, "a");
        assert_eq!(s[0].1[0], Reading { key: "Liquid temperature".into(), value: 30.1, unit: "°C".into() });
        assert_eq!(s[0].1[1].value, 2100.0);
        assert_eq!(pairs(&[(20.0, 50.0), (40.0, 120.0)]), ["20", "50", "40", "100"]);
        assert!(!has_lcd(0x1E71, 0x2007));
    }
}
