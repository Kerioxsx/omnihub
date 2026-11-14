//! The network: download and upload speed now, data used per day, and
//! which programs have connections open (and to where). Windows tells
//! everyone the adapters' byte counts and the connection table; how many
//! bytes each program moves it only tells administrators, so per-program
//! data isn't counted here.

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppConnections {
    pub process: String,
    pub pid: u32,
    pub connections: usize,
    /// A few of the addresses it talks to.
    pub remotes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct DayUse {
    pub date: String,
    pub down: u64,
    pub up: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetStatus {
    /// Bytes per second now.
    pub down: u64,
    pub up: u64,
    /// The last two minutes: (unix ms, down, up) per sample.
    pub history: Vec<(i64, u64, u64)>,
    /// The last 30 days, oldest first (today last).
    pub days: Vec<DayUse>,
    pub apps: Vec<AppConnections>,
    pub supported: bool,
}

/// The change in a counter since the last reading (a counter that went back,
/// after an adapter restarted, starts again from its new value).
pub fn delta(before: u64, now: u64) -> u64 {
    if now >= before { now - before } else { now }
}

/// Programs with connections open, busiest first, from the connection table.
pub fn group_connections(ports: &[super::ports::PortUse]) -> Vec<AppConnections> {
    let mut by: HashMap<(String, u32), (usize, Vec<String>)> = HashMap::new();
    for p in ports.iter().filter(|p| p.proto == "TCP" && p.state == "Connected") {
        let Some(remote) = p.remote.as_deref() else { continue };
        if remote.starts_with("127.") || remote == "::1" || remote == "0.0.0.0" {
            continue;
        }
        let e = by.entry((p.process.clone(), p.pid)).or_default();
        e.0 += 1;
        let addr = match p.remote_port {
            Some(port) => format!("{remote}:{port}"),
            None => remote.to_string(),
        };
        if e.1.len() < 4 && !e.1.contains(&addr) {
            e.1.push(addr);
        }
    }
    let mut out: Vec<AppConnections> = by.into_iter().map(|((process, pid), (connections, remotes))| AppConnections { process, pid, connections, remotes }).collect();
    out.sort_by(|a, b| b.connections.cmp(&a.connections).then(a.process.cmp(&b.process)));
    out
}

const KEEP_DAYS: usize = 60;
const HISTORY: usize = 60;

pub struct NetMonitor {
    file: PathBuf,
    days: Mutex<BTreeMap<String, (u64, u64)>>,
    last: Mutex<Option<(i64, u64, u64)>>,
    history: Mutex<VecDeque<(i64, u64, u64)>>,
    dirty: Mutex<bool>,
}

impl NetMonitor {
    pub fn new(data: &Path) -> NetMonitor {
        let file = data.join("network-usage.json");
        let list: Vec<DayUse> = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let days = list.into_iter().map(|d| (d.date, (d.down, d.up))).collect();
        NetMonitor { file, days: Mutex::new(days), last: Mutex::new(None), history: Mutex::new(VecDeque::new()), dirty: Mutex::new(false) }
    }

    /// Add a reading of the adapters' totals (received, sent) at `at` (unix ms) on `date`.
    pub fn take(&self, at: i64, date: &str, rx: u64, tx: u64) {
        let mut last = self.last.lock();
        if let Some((t0, r0, s0)) = *last {
            let secs = ((at - t0) as f64 / 1000.0).max(0.001);
            let (dr, ds) = (delta(r0, rx), delta(s0, tx));
            let mut days = self.days.lock();
            let d = days.entry(date.to_string()).or_default();
            d.0 += dr;
            d.1 += ds;
            while days.len() > KEEP_DAYS {
                let first = days.keys().next().cloned();
                if let Some(k) = first {
                    days.remove(&k);
                }
            }
            let mut h = self.history.lock();
            h.push_back((at, (dr as f64 / secs) as u64, (ds as f64 / secs) as u64));
            while h.len() > HISTORY {
                h.pop_front();
            }
            *self.dirty.lock() = true;
        }
        *last = Some((at, rx, tx));
    }

    pub fn save(&self) {
        if !std::mem::take(&mut *self.dirty.lock()) {
            return;
        }
        let list: Vec<DayUse> = self.days.lock().iter().map(|(date, (down, up))| DayUse { date: date.clone(), down: *down, up: *up }).collect();
        if let Ok(json) = serde_json::to_vec(&list) {
            let _ = crate::settings::write_atomic(&self.file, &json);
        }
    }

    pub fn status(&self, apps: Vec<AppConnections>) -> NetStatus {
        let history: Vec<(i64, u64, u64)> = self.history.lock().iter().copied().collect();
        let (down, up) = history.last().map_or((0, 0), |h| (h.1, h.2));
        let days = self.days.lock();
        let days: Vec<DayUse> = days.iter().rev().take(30).rev().map(|(date, (down, up))| DayUse { date: date.clone(), down: *down, up: *up }).collect();
        NetStatus { down, up, history, days, apps, supported: imp::totals().is_some() }
    }
}

/// The adapters' received and sent byte totals (real network cards only,
/// so a VPN or filter driver on top isn't counted twice).
pub fn totals() -> Option<(u64, u64)> {
    imp::totals()
}

#[cfg(windows)]
mod imp {
    use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
    use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;

    pub fn totals() -> Option<(u64, u64)> {
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        if unsafe { GetIfTable2(&mut table) }.0 != 0 || table.is_null() {
            return None;
        }
        let (mut rx, mut tx) = (0u64, 0u64);
        unsafe {
            let n = (*table).NumEntries as usize;
            let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), n);
            for r in rows {
                let flags = r.InterfaceAndOperStatusFlags._bitfield;
                let hardware = flags & 1 != 0;
                let filter = flags & 2 != 0;
                if hardware && !filter && r.OperStatus == IfOperStatusUp {
                    rx += r.InOctets;
                    tx += r.OutOctets;
                }
            }
            FreeMibTable(table as *const _);
        }
        Some((rx, tx))
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn totals() -> Option<(u64, u64)> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::ports::PortUse;

    #[test]
    fn counts_days_and_speeds() {
        let t = tempfile::tempdir().unwrap();
        let m = NetMonitor::new(t.path());
        m.take(0, "2026-11-02", 1_000, 500);
        m.take(2_000, "2026-11-02", 5_000, 1_500);
        m.take(4_000, "2026-11-03", 7_000, 1_600);
        m.take(6_000, "2026-11-03", 300, 100); // the adapter restarted
        let st = m.status(vec![]);
        assert_eq!(st.days, vec![DayUse { date: "2026-11-02".into(), down: 4_000, up: 1_000 }, DayUse { date: "2026-11-03".into(), down: 2_300, up: 200 }]);
        assert_eq!((st.history[0].1, st.history[0].2), (2_000, 500), "bytes per second");
        m.save();
        assert_eq!(NetMonitor::new(t.path()).status(vec![]).days.len(), 2, "kept across starts");
    }

    #[test]
    fn groups_connections_by_program() {
        let p = |process: &str, pid: u32, remote: &str, port: u16, state: &str| PortUse { proto: "TCP".into(), local: "192.168.1.5".into(), port: 50000, remote: Some(remote.into()), remote_port: Some(port), state: state.into(), pid, process: process.into() };
        let apps = group_connections(&[
            p("chrome.exe", 10, "142.250.1.1", 443, "Connected"),
            p("chrome.exe", 10, "142.250.1.2", 443, "Connected"),
            p("Discord.exe", 20, "162.159.1.1", 443, "Connected"),
            p("Discord.exe", 20, "127.0.0.1", 6463, "Connected"),
            p("svchost.exe", 30, "0.0.0.0", 0, "Listening"),
        ]);
        assert_eq!(apps.iter().map(|a| (a.process.as_str(), a.connections)).collect::<Vec<_>>(), vec![("chrome.exe", 2), ("Discord.exe", 1)]);
        assert_eq!(apps[0].remotes, vec!["142.250.1.1:443", "142.250.1.2:443"]);
    }
}
