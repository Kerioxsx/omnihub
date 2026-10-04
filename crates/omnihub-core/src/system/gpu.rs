//! GPU use per program and per graphics card, from the performance counters
//! Task Manager reads ("GPU Engine", "GPU Process Memory", "GPU Adapter
//! Memory").
//!
//! A program's GPU figure is its busiest engine (3D, video decode, copy…),
//! like Task Manager's GPU column; a card's figure is its busiest engine
//! across all programs.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpuAdapter {
    pub name: String,
    /// Busiest engine, 0–100.
    pub percent: f32,
    /// Video memory in use / on the card, bytes.
    pub memory_used: u64,
    pub memory_total: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpuUse {
    /// Engine use, 0–100.
    pub percent: f32,
    /// Dedicated video memory, bytes.
    pub memory: u64,
}

#[derive(Debug, Clone, Default)]
pub struct GpuSample {
    pub per_pid: HashMap<u32, GpuUse>,
    pub adapters: Vec<GpuAdapter>,
}

/// "pid_1234_luid_0x00000000_0x0000D1B5_phys_0_eng_0_engtype_3D" →
/// (1234, "0x00000000_0x0000d1b5", "3d").
#[cfg_attr(not(windows), allow(dead_code))]
fn parse_engine(name: &str) -> Option<(u32, String, String)> {
    let pid = name.strip_prefix("pid_")?.split('_').next()?.parse().ok()?;
    let luid = parse_luid(name)?;
    let engtype = name.rsplit_once("engtype_")?.1.to_ascii_lowercase();
    Some((pid, luid, engtype))
}

/// "pid_1234_luid_0x00000000_0x0000D1B5_phys_0" → (1234, luid).
#[cfg_attr(not(windows), allow(dead_code))]
fn parse_process(name: &str) -> Option<(u32, String)> {
    let pid = name.strip_prefix("pid_")?.split('_').next()?.parse().ok()?;
    Some((pid, parse_luid(name)?))
}

/// The "0x…_0x…" after "luid_", lower-cased.
#[cfg_attr(not(windows), allow(dead_code))]
fn parse_luid(name: &str) -> Option<String> {
    let rest = name.split_once("luid_")?.1;
    let mut parts = rest.splitn(3, '_');
    let (hi, lo) = (parts.next()?, parts.next()?);
    if !hi.starts_with("0x") || !lo.starts_with("0x") {
        return None;
    }
    Some(format!("{hi}_{lo}").to_ascii_lowercase())
}

pub fn luid_key(high: i32, low: u32) -> String {
    format!("0x{:08x}_0x{:08x}", high as u32, low)
}

/// Engine load per card (by LUID) and per program from raw counter rows.
#[cfg_attr(not(windows), allow(dead_code))]
fn aggregate(engines: &[(String, f64)], process_memory: &[(String, f64)]) -> (HashMap<u32, GpuUse>, HashMap<String, f32>) {
    // Sum each engine type per program per card, then keep the busiest.
    let mut per_engine: HashMap<(u32, &str, &str), f64> = HashMap::new();
    let parsed: Vec<_> = engines.iter().filter_map(|(n, v)| parse_engine(n).map(|p| (p, *v))).collect();
    for ((pid, luid, eng), v) in &parsed {
        *per_engine.entry((*pid, luid.as_str(), eng.as_str())).or_default() += v.max(0.0);
    }
    let mut per_pid: HashMap<u32, GpuUse> = HashMap::new();
    let mut per_card_engine: HashMap<(&str, &str), f64> = HashMap::new();
    for ((pid, luid, eng), v) in per_engine {
        let u = per_pid.entry(pid).or_default();
        u.percent = u.percent.max(v.min(100.0) as f32);
        *per_card_engine.entry((luid, eng)).or_default() += v;
    }
    let mut per_card: HashMap<String, f32> = HashMap::new();
    for ((luid, _), v) in per_card_engine {
        let c = per_card.entry(luid.to_string()).or_default();
        *c = c.max(v.min(100.0) as f32);
    }
    for (n, v) in process_memory {
        if let Some((pid, _)) = parse_process(n) {
            per_pid.entry(pid).or_default().memory += v.max(0.0) as u64;
        }
    }
    per_pid.retain(|_, u| u.percent > 0.0 || u.memory > 0);
    (per_pid, per_card)
}

/// Keeps the counter query open between samples (engine use is a rate, so
/// it needs two readings; the first sample after start reads as 0 %).
pub struct GpuSampler {
    #[cfg(windows)]
    inner: Option<win::Query>,
}

impl Default for GpuSampler {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuSampler {
    pub fn new() -> Self {
        GpuSampler {
            #[cfg(windows)]
            inner: win::Query::open().map_err(|e| tracing::debug!("GPU counters unavailable: {e}")).ok(),
        }
    }

    /// Whether this PC exposes GPU counters (Windows 10 1709 and later).
    pub fn supported(&self) -> bool {
        #[cfg(windows)]
        {
            self.inner.is_some()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    pub fn sample(&mut self) -> Option<GpuSample> {
        #[cfg(windows)]
        {
            let q = self.inner.as_mut()?;
            let raw = q.collect()?;
            let (per_pid, per_card) = aggregate(&raw.engines, &raw.process_memory);
            let mut used: HashMap<String, u64> = HashMap::new();
            for (n, v) in &raw.adapter_memory {
                if let Some(l) = parse_luid(n) {
                    *used.entry(l).or_default() += v.max(0.0) as u64;
                }
            }
            let adapters = q
                .adapters
                .iter()
                .map(|(luid, name, total)| GpuAdapter { name: name.clone(), percent: per_card.get(luid).copied().unwrap_or(0.0), memory_used: used.get(luid).copied().unwrap_or(0), memory_total: *total })
                .collect();
            Some(GpuSample { per_pid, adapters })
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
}

#[cfg(windows)]
mod win {
    use windows::core::PCWSTR;
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE};
    use windows::Win32::System::Performance::*;

    pub struct Raw {
        pub engines: Vec<(String, f64)>,
        pub process_memory: Vec<(String, f64)>,
        pub adapter_memory: Vec<(String, f64)>,
    }

    pub struct Query {
        query: PDH_HQUERY,
        engine: PDH_HCOUNTER,
        process_memory: PDH_HCOUNTER,
        adapter_memory: PDH_HCOUNTER,
        /// (luid, name, dedicated memory) of each real graphics card.
        pub adapters: Vec<(String, String, u64)>,
    }

    // PDH handles are plain handles; the query is only used behind a lock.
    unsafe impl Send for Query {}

    impl Drop for Query {
        fn drop(&mut self) {
            unsafe {
                PdhCloseQuery(self.query);
            }
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    impl Query {
        pub fn open() -> Result<Query, String> {
            unsafe {
                let mut query = PDH_HQUERY::default();
                let r = PdhOpenQueryW(PCWSTR::null(), 0, &mut query);
                if r != 0 {
                    return Err(format!("PdhOpenQuery {r:#x}"));
                }
                let add = |path: &str| -> Result<PDH_HCOUNTER, String> {
                    let mut c = PDH_HCOUNTER::default();
                    let p = wide(path);
                    match PdhAddEnglishCounterW(query, PCWSTR(p.as_ptr()), 0, &mut c) {
                        0 => Ok(c),
                        r => Err(format!("{path}: {r:#x}")),
                    }
                };
                let made = (|| Ok::<_, String>((add(r"\GPU Engine(*)\Utilization Percentage")?, add(r"\GPU Process Memory(*)\Dedicated Usage")?, add(r"\GPU Adapter Memory(*)\Dedicated Usage")?)))();
                let (engine, process_memory, adapter_memory) = match made {
                    Ok(c) => c,
                    Err(e) => {
                        PdhCloseQuery(query);
                        return Err(e);
                    }
                };
                PdhCollectQueryData(query);
                Ok(Query { query, engine, process_memory, adapter_memory, adapters: adapters() })
            }
        }

        pub fn collect(&mut self) -> Option<Raw> {
            unsafe {
                if PdhCollectQueryData(self.query) != 0 {
                    return None;
                }
            }
            Some(Raw { engines: read(self.engine), process_memory: read(self.process_memory), adapter_memory: read(self.adapter_memory) })
        }
    }

    /// Every instance of a wildcard counter as (instance name, value).
    fn read(counter: PDH_HCOUNTER) -> Vec<(String, f64)> {
        // PDH_FMT_NOCAP100: engine figures of several instances are summed.
        let fmt = PDH_FMT(PDH_FMT_DOUBLE.0 | 0x8000);
        unsafe {
            let (mut size, mut count) = (0u32, 0u32);
            let r = PdhGetFormattedCounterArrayW(counter, fmt, &mut size, &mut count, None);
            if r != PDH_MORE_DATA || size == 0 {
                return Vec::new();
            }
            // The buffer holds the items followed by their names; keep it
            // aligned for the item structs.
            let item = std::mem::size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            let mut buf: Vec<PDH_FMT_COUNTERVALUE_ITEM_W> = vec![PDH_FMT_COUNTERVALUE_ITEM_W::default(); (size as usize).div_ceil(item) + 1];
            if PdhGetFormattedCounterArrayW(counter, fmt, &mut size, &mut count, Some(buf.as_mut_ptr())) != 0 {
                return Vec::new();
            }
            buf[..count as usize]
                .iter()
                .filter(|i| i.FmtValue.CStatus == PDH_CSTATUS_VALID_DATA || i.FmtValue.CStatus == PDH_CSTATUS_NEW_DATA)
                .filter_map(|i| Some((i.szName.to_string().ok()?, i.FmtValue.Anonymous.doubleValue)))
                .collect()
        }
    }

    fn adapters() -> Vec<(String, String, u64)> {
        let mut out = Vec::new();
        unsafe {
            let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else { return out };
            let mut i = 0;
            while let Ok(a) = factory.EnumAdapters1(i) {
                i += 1;
                let Ok(d) = a.GetDesc1() else { continue };
                if d.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 || d.VendorId == 0x1414 {
                    continue;
                }
                let len = d.Description.iter().position(|&c| c == 0).unwrap_or(d.Description.len());
                let name = String::from_utf16_lossy(&d.Description[..len]).trim().to_string();
                let luid = super::luid_key(d.AdapterLuid.HighPart, d.AdapterLuid.LowPart);
                if !out.iter().any(|(l, _, _): &(String, String, u64)| *l == luid) {
                    out.push((luid, name, d.DedicatedVideoMemory as u64));
                }
            }
        }
        // The card with the most video memory first (the gaming GPU on a
        // laptop with two).
        out.sort_by_key(|(_, _, m)| std::cmp::Reverse(*m));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_counter_instances() {
        assert_eq!(parse_engine("pid_1234_luid_0x00000000_0x0000D1B5_phys_0_eng_0_engtype_3D"), Some((1234, "0x00000000_0x0000d1b5".into(), "3d".into())));
        assert_eq!(parse_engine("pid_8_luid_0x00000000_0x0000D1B5_phys_0_eng_12_engtype_VideoDecode"), Some((8, "0x00000000_0x0000d1b5".into(), "videodecode".into())));
        assert_eq!(parse_process("pid_77_luid_0x00000000_0x00011A2B_phys_0"), Some((77, "0x00000000_0x00011a2b".into())));
        assert_eq!(parse_luid("luid_0x00000000_0x00011A2B_phys_0"), Some("0x00000000_0x00011a2b".into()));
        assert_eq!(parse_engine("_Total"), None);
        assert_eq!(luid_key(0, 0xD1B5), "0x00000000_0x0000d1b5");
        assert_eq!(luid_key(-1, 1), "0xffffffff_0x00000001");
    }

    #[test]
    fn busiest_engine_wins() {
        let card = "luid_0x00000000_0x0000d1b5_phys_0";
        let e = |pid: u32, eng: u32, ty: &str, v: f64| (format!("pid_{pid}_{card}_eng_{eng}_engtype_{ty}"), v);
        let engines = vec![
            // The game: two 3D engine instances (summed) and a bit of copy.
            e(10, 0, "3D", 40.0),
            e(10, 1, "3D", 25.0),
            e(10, 2, "Copy", 5.0),
            // A browser decoding video.
            e(20, 3, "VideoDecode", 12.0),
            e(20, 0, "3D", 3.0),
            // Idle program: dropped unless it holds video memory.
            e(30, 0, "3D", 0.0),
        ];
        let mem = vec![(format!("pid_10_{card}"), 3.0e9), (format!("pid_30_{card}"), 1.0e8)];
        let (pids, cards) = aggregate(&engines, &mem);
        assert_eq!(pids[&10], GpuUse { percent: 65.0, memory: 3_000_000_000 });
        assert_eq!(pids[&20].percent, 12.0);
        assert_eq!(pids[&30], GpuUse { percent: 0.0, memory: 100_000_000 });
        // The card's 3D engine: 40 + 25 + 3.
        assert_eq!(cards["0x00000000_0x0000d1b5"], 68.0);
    }
}
