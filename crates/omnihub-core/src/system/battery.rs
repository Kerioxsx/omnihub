//! Batteries: the PC's own (laptops), Xbox controllers (XInput) and
//! Bluetooth devices that report a level to Windows (headsets, mice,
//! keyboards, controllers). Levels are sampled every two minutes to show a
//! history, guess the time left, and warn once at 20% and 10%.

use std::collections::{HashMap, VecDeque};

use parking_lot::Mutex;
use serde::Serialize;

use crate::events::EventBus;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Battery {
    pub id: String,
    pub name: String,
    /// "pc", "controller" or "bluetooth".
    pub kind: String,
    /// 0–100, when the device says.
    pub percent: Option<u8>,
    pub charging: Option<bool>,
    /// Plugged in with no battery in use (a wired controller).
    pub wired: bool,
    /// Minutes left: Windows' own guess for the PC, else from the history.
    pub minutes_left: Option<u32>,
    /// (unix ms, percent), oldest first, the last 24 hours.
    pub history: Vec<(i64, u8)>,
}

const KEEP_MS: i64 = 24 * 3600 * 1000;

#[derive(Default)]
struct Track {
    history: VecDeque<(i64, u8)>,
    warned: u8,
}

pub struct Batteries {
    tracks: Mutex<HashMap<String, Track>>,
    last: Mutex<Vec<Battery>>,
    events: EventBus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LowBattery {
    pub name: String,
    pub percent: u8,
}

impl Batteries {
    pub fn new(events: EventBus) -> Batteries {
        Batteries { tracks: Mutex::new(HashMap::new()), last: Mutex::new(Vec::new()), events }
    }

    /// Read every battery now, add to the history and warn when low.
    pub fn sample(&self, now: i64) -> Vec<Battery> {
        let read = imp::read();
        let out = self.take(read, now);
        *self.last.lock() = out.clone();
        out
    }

    /// The last reading (read now if there's none yet).
    pub fn list(&self, now: i64) -> Vec<Battery> {
        let last = self.last.lock().clone();
        if last.is_empty() {
            self.sample(now)
        } else {
            last
        }
    }

    fn take(&self, read: Vec<Battery>, now: i64) -> Vec<Battery> {
        let mut tracks = self.tracks.lock();
        let mut out = Vec::new();
        for mut b in read {
            let t = tracks.entry(b.id.clone()).or_default();
            if let Some(p) = b.percent {
                if t.history.back().is_none_or(|l| now - l.0 >= 60_000 || l.1 != p) {
                    t.history.push_back((now, p));
                }
                while t.history.front().is_some_and(|f| now - f.0 > KEEP_MS) {
                    t.history.pop_front();
                }
                let draining = b.charging != Some(true) && !b.wired;
                if b.minutes_left.is_none() && draining {
                    b.minutes_left = minutes_left(&t.history, now);
                }
                // Warn once at 20% and once at 10% per discharge.
                if draining {
                    let step = if p <= 10 { 2 } else if p <= 20 { 1 } else { 0 };
                    if step > t.warned {
                        t.warned = step;
                        self.events.emit("battery:low", LowBattery { name: b.name.clone(), percent: p });
                    }
                } else if p > 25 || b.charging == Some(true) {
                    t.warned = 0;
                }
            }
            b.history = t.history.iter().copied().collect();
            out.push(b);
        }
        out
    }
}

/// Minutes until empty from the last steady fall (needs a drop of 3+ points
/// over 10+ minutes since the level last went up).
pub fn minutes_left(history: &VecDeque<(i64, u8)>, now: i64) -> Option<u32> {
    let (_, last) = *history.back()?;
    // Start of the current fall: just after the last rise.
    let mut start = history.len() - 1;
    while start > 0 && history[start - 1].1 >= history[start].1 {
        start -= 1;
    }
    let (t0, p0) = history[start];
    let drop = p0.checked_sub(last)? as f64;
    let mins = (now - t0) as f64 / 60_000.0;
    if drop < 3.0 || mins < 10.0 {
        return None;
    }
    Some((last as f64 / (drop / mins)).round().min(48.0 * 60.0) as u32)
}

#[cfg(windows)]
mod imp {
    use super::Battery;
    use windows::Win32::Devices::DeviceAndDriverInstallation::{SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW, SetupDiGetDevicePropertyW, DIGCF_ALLCLASSES, DIGCF_PRESENT, SP_DEVINFO_DATA};
    use windows::Win32::Devices::Properties::{DEVPKEY_Device_FriendlyName, DEVPKEY_Device_InstanceId, DEVPROPTYPE};
    use windows::Win32::Foundation::DEVPROPKEY;
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    use windows::Win32::UI::Input::XboxController::{XInputGetBatteryInformation, BATTERY_DEVTYPE_GAMEPAD, BATTERY_LEVEL_EMPTY, BATTERY_LEVEL_FULL, BATTERY_LEVEL_LOW, BATTERY_LEVEL_MEDIUM, BATTERY_TYPE_DISCONNECTED, BATTERY_TYPE_WIRED, XINPUT_BATTERY_INFORMATION};

    /// The battery level Windows keeps for Bluetooth devices
    /// ({104EA319-6EE2-4701-BD47-8DDBF425BBE5}, 2), a byte 0–100.
    const BATTERY_KEY: DEVPROPKEY = DEVPROPKEY { fmtid: windows::core::GUID::from_u128(0x104ea319_6ee2_4701_bd47_8ddbf425bbe5), pid: 2 };

    pub fn read() -> Vec<Battery> {
        let mut out = Vec::new();
        pc(&mut out);
        pads(&mut out);
        bluetooth(&mut out);
        out
    }

    fn blank(id: String, name: String, kind: &str) -> Battery {
        Battery { id, name, kind: kind.into(), percent: None, charging: None, wired: false, minutes_left: None, history: Vec::new() }
    }

    fn pc(out: &mut Vec<Battery>) {
        let mut s = SYSTEM_POWER_STATUS::default();
        if unsafe { GetSystemPowerStatus(&mut s) }.is_err() {
            return;
        }
        // 128 = no system battery; 255 = unknown.
        if s.BatteryFlag & 128 != 0 || s.BatteryFlag == 255 || s.BatteryLifePercent > 100 {
            return;
        }
        let mut b = blank("pc".into(), "This PC".into(), "pc");
        b.percent = Some(s.BatteryLifePercent);
        b.charging = Some(s.ACLineStatus == 1);
        if s.BatteryLifeTime != u32::MAX && s.ACLineStatus != 1 {
            b.minutes_left = Some(s.BatteryLifeTime / 60);
        }
        out.push(b);
    }

    fn pads(out: &mut Vec<Battery>) {
        for i in 0..4u32 {
            let mut info = XINPUT_BATTERY_INFORMATION::default();
            if unsafe { XInputGetBatteryInformation(i, BATTERY_DEVTYPE_GAMEPAD, &mut info) } != 0 || info.BatteryType == BATTERY_TYPE_DISCONNECTED {
                continue;
            }
            let mut b = blank(format!("pad{i}"), format!("Controller {}", i + 1), "controller");
            if info.BatteryType == BATTERY_TYPE_WIRED {
                b.wired = true;
            } else {
                // XInput only says empty / low / medium / full.
                b.percent = Some(match info.BatteryLevel {
                    BATTERY_LEVEL_EMPTY => 5,
                    BATTERY_LEVEL_LOW => 25,
                    BATTERY_LEVEL_MEDIUM => 60,
                    BATTERY_LEVEL_FULL => 100,
                    _ => continue,
                });
            }
            out.push(b);
        }
    }

    fn prop(set: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO, data: &SP_DEVINFO_DATA, key: &DEVPROPKEY) -> Option<Vec<u8>> {
        let mut ty = DEVPROPTYPE::default();
        let mut buf = vec![0u8; 512];
        let mut need = 0u32;
        unsafe { SetupDiGetDevicePropertyW(set, data, key, &mut ty, Some(&mut buf), Some(&mut need), 0) }.ok()?;
        buf.truncate(need as usize);
        Some(buf)
    }

    fn wide(b: &[u8]) -> String {
        let u: Vec<u16> = b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&c| c != 0).collect();
        String::from_utf16_lossy(&u)
    }

    fn bluetooth(out: &mut Vec<Battery>) {
        let Ok(set) = (unsafe { SetupDiGetClassDevsW(None, None, None, DIGCF_ALLCLASSES | DIGCF_PRESENT) }) else { return };
        let mut seen = std::collections::HashSet::new();
        for i in 0..4096u32 {
            let mut data = SP_DEVINFO_DATA { cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32, ..Default::default() };
            if unsafe { SetupDiEnumDeviceInfo(set, i, &mut data) }.is_err() {
                break;
            }
            let Some(level) = prop(set, &data, &BATTERY_KEY).and_then(|b| b.first().copied()) else { continue };
            if level > 100 {
                continue;
            }
            let name = prop(set, &data, &DEVPKEY_Device_FriendlyName).map(|b| wide(&b)).filter(|n| !n.is_empty()).unwrap_or_else(|| "Bluetooth device".into());
            // A device often shows up several times (one per profile).
            if !seen.insert(name.to_lowercase()) {
                continue;
            }
            let id = prop(set, &data, &DEVPKEY_Device_InstanceId).map(|b| wide(&b)).unwrap_or_else(|| name.clone());
            let mut b = blank(format!("bt:{}", blake3::hash(id.as_bytes()).to_hex()[..12].to_owned()), name, "bluetooth");
            b.percent = Some(level);
            out.push(b);
        }
        let _ = unsafe { SetupDiDestroyDeviceInfoList(set) };
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn read() -> Vec<super::Battery> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(id: &str, p: u8, charging: bool) -> Battery {
        Battery { id: id.into(), name: id.into(), kind: "bluetooth".into(), percent: Some(p), charging: Some(charging), wired: false, minutes_left: None, history: Vec::new() }
    }

    #[test]
    fn history_time_left_and_warnings() {
        let events = EventBus::new();
        let mut rx = events.subscribe();
        let bats = Batteries::new(events);
        let min = 60_000;
        // 1% every 3 minutes from 30%.
        let mut last = Vec::new();
        for n in 0..=10 {
            last = bats.take(vec![b("mouse", 30 - n, false)], n as i64 * 3 * min);
        }
        let m = &last[0];
        assert_eq!(m.history.len(), 11);
        assert_eq!(m.minutes_left, Some(60), "20% left at 1% per 3 minutes");
        let mut lows = Vec::new();
        while let Ok(e) = rx.try_recv() {
            if e.topic == "battery:low" {
                lows.push(e.payload["percent"].as_u64().unwrap());
            }
        }
        assert_eq!(lows, vec![20], "warned once at 20%");
        bats.take(vec![b("mouse", 9, false)], 40 * min);
        bats.take(vec![b("mouse", 8, false)], 43 * min);
        let n = std::iter::from_fn(|| rx.try_recv().ok()).filter(|e| e.topic == "battery:low").count();
        assert_eq!(n, 1, "and once at 10%");
        let c = bats.take(vec![b("mouse", 40, true)], 50 * min);
        assert_eq!(c[0].minutes_left, None, "no time left while charging");
    }

    #[test]
    fn minutes_left_needs_a_real_fall() {
        let h: VecDeque<(i64, u8)> = [(0, 50), (5 * 60_000, 49)].into();
        assert_eq!(minutes_left(&h, 5 * 60_000), None);
        let h: VecDeque<(i64, u8)> = [(0, 20), (10 * 60_000, 80), (40 * 60_000, 70)].into();
        assert_eq!(minutes_left(&h, 40 * 60_000), Some(210), "only since it was last charged");
    }
}
