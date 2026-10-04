//! What is using the PC right now: running programs grouped by name (one
//! row for all of Brave's processes), with CPU, memory, GPU and disk, plus
//! "End task" and priority.
//!
//! Processes Windows cannot run without, and OmniHub itself, are never
//! ended from here.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use super::gpu::{GpuAdapter, GpuSample, GpuSampler};

/// Never offered for "End task".
const PROTECTED: &[&str] = &[
    "system", "idle", "system idle process", "registry", "smss.exe", "csrss.exe", "wininit.exe", "winlogon.exe", "services.exe", "lsass.exe", "lsaiso.exe", "svchost.exe", "dwm.exe", "fontdrvhost.exe", "memory compression", "secure system", "sihost.exe", "ctfmon.exe", "audiodg.exe",
    "msmpeng.exe", "securityhealthservice.exe", "spoolsv.exe", "searchindexer.exe", "init", "systemd", "kthreadd", "launchd", "kernel_task", "windowserver", "loginwindow",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessGroup {
    /// Executable name ("brave.exe").
    pub name: String,
    /// Number of processes with that name.
    pub count: usize,
    /// Share of the whole CPU, 0–100.
    pub cpu: f32,
    /// Resident memory in bytes.
    pub memory: u64,
    /// Busiest GPU engine, 0–100 (0 when GPU figures were not asked for).
    pub gpu: f32,
    /// Dedicated video memory, bytes.
    pub gpu_memory: u64,
    /// Disk reads + writes, bytes per second.
    pub disk: u64,
    pub exe: Option<String>,
    pub pids: Vec<u32>,
    /// Whether "End task" is offered.
    pub can_end: bool,
    /// Windows priority of its first process, when readable.
    pub priority: Option<Priority>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ProcessSort {
    #[default]
    Cpu,
    Memory,
    Gpu,
    Disk,
    Name,
}

/// Windows process priority classes OmniHub offers (never "Realtime", which
/// can starve the mouse and keyboard).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Priority {
    Low,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
}

/// The whole PC at a glance plus the programs using it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    /// 0–100.
    pub cpu: f32,
    pub cpu_name: String,
    pub cores: usize,
    pub memory_used: u64,
    pub memory_total: u64,
    /// Whether this PC reports GPU figures (Windows 10 1709+).
    pub gpu_supported: bool,
    pub gpus: Vec<GpuAdapter>,
    /// Disk reads + writes of all programs, bytes per second.
    pub disk: u64,
    pub process_count: usize,
    pub processes: Vec<ProcessGroup>,
}

pub fn is_protected(name: &str) -> bool {
    PROTECTED.contains(&name.to_ascii_lowercase().as_str())
}

/// One process at the last refresh.
#[derive(Clone)]
struct Row {
    pid: u32,
    name: String,
    exe: Option<String>,
    cpu: f32,
    memory: u64,
    disk: u64,
}

struct Inner {
    sys: System,
    rows: Vec<Row>,
    at: Option<Instant>,
    gpu: Option<GpuSampler>,
    gpu_sample: Option<(Instant, GpuSample)>,
}

pub struct ProcessMonitor {
    inner: Mutex<Inner>,
}

impl Default for ProcessMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Two callers (Home and the Tasks page) may ask within moments of each
/// other; CPU figures over a few milliseconds are noise, so reuse a sample
/// this fresh.
const FRESH: Duration = Duration::from_millis(700);

impl ProcessMonitor {
    pub fn new() -> Self {
        ProcessMonitor { inner: Mutex::new(Inner { sys: System::new(), rows: Vec::new(), at: None, gpu: None, gpu_sample: None }) }
    }

    fn refresh(inner: &mut Inner) {
        if inner.at.is_some_and(|t| t.elapsed() < FRESH) {
            return;
        }
        let sys = &mut inner.sys;
        if sys.cpus().is_empty() {
            sys.refresh_cpu_list(sysinfo::CpuRefreshKind::nothing());
        }
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory().with_disk_usage().with_exe(sysinfo::UpdateKind::OnlyIfNotSet));
        let secs = inner.at.map(|t| t.elapsed().as_secs_f64()).unwrap_or(1.0).max(0.2);
        let cpus = sys.cpus().len().max(1) as f32;
        inner.rows = sys
            .processes()
            .iter()
            // Threads show up as processes on Linux; count each program once.
            .filter(|(pid, p)| pid.as_u32() > 4 && p.thread_kind().is_none())
            .filter_map(|(pid, p)| {
                let name = p.name().to_string_lossy().into_owned();
                if name.is_empty() {
                    return None;
                }
                let d = p.disk_usage();
                Some(Row { pid: pid.as_u32(), name, exe: p.exe().map(|e| e.to_string_lossy().into_owned()), cpu: p.cpu_usage() / cpus, memory: p.memory(), disk: ((d.read_bytes + d.written_bytes) as f64 / secs) as u64 })
            })
            .collect();
        inner.at = Some(Instant::now());
    }

    fn gpu(inner: &mut Inner) -> Option<GpuSample> {
        if let Some((t, s)) = &inner.gpu_sample {
            if t.elapsed() < FRESH {
                return Some(s.clone());
            }
        }
        let sampler = inner.gpu.get_or_insert_with(GpuSampler::new);
        let s = sampler.sample()?;
        inner.gpu_sample = Some((Instant::now(), s.clone()));
        Some(s)
    }

    /// The busiest programs. CPU figures need two samples, so the first call
    /// after start reports 0 % for everything.
    pub fn top(&self, sort: ProcessSort, limit: usize) -> Vec<ProcessGroup> {
        self.usage(sort, limit, false).processes
    }

    /// The whole PC and its busiest `limit` programs. GPU figures cost a
    /// counter query, so they are only gathered when asked for.
    pub fn usage(&self, sort: ProcessSort, limit: usize, with_gpu: bool) -> Usage {
        let mut inner = self.inner.lock();
        Self::refresh(&mut inner);
        let gpu = if with_gpu { Self::gpu(&mut inner) } else { None };
        let gpu_supported = with_gpu && inner.gpu.as_ref().is_some_and(|g| g.supported());
        let me = std::process::id();
        let mut groups: HashMap<String, ProcessGroup> = HashMap::new();
        for r in &inner.rows {
            let g = groups.entry(r.name.to_ascii_lowercase()).or_insert_with(|| ProcessGroup {
                name: r.name.clone(),
                count: 0,
                cpu: 0.0,
                memory: 0,
                gpu: 0.0,
                gpu_memory: 0,
                disk: 0,
                exe: r.exe.clone(),
                pids: Vec::new(),
                can_end: !is_protected(&r.name),
                priority: None,
            });
            g.count += 1;
            g.cpu += r.cpu;
            g.memory += r.memory;
            g.disk += r.disk;
            if let Some(u) = gpu.as_ref().and_then(|s| s.per_pid.get(&r.pid)) {
                // A program's processes usually share one engine (Brave's GPU
                // process); keep the busiest rather than a misleading sum.
                g.gpu = g.gpu.max(u.percent);
                g.gpu_memory += u.memory;
            }
            g.pids.push(r.pid);
            if r.pid == me {
                g.can_end = false;
            }
        }
        let process_count = inner.rows.len();
        let disk = inner.rows.iter().map(|r| r.disk).sum();
        let sys = &inner.sys;
        let cpu_name = sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default();
        let (cpu, cores, memory_used, memory_total) = (sys.global_cpu_usage(), sys.cpus().len(), sys.used_memory(), sys.total_memory());
        drop(inner);

        let mut list: Vec<ProcessGroup> = groups.into_values().collect();
        sort_groups(&mut list, sort);
        list.truncate(limit);
        for g in &mut list {
            g.cpu = (g.cpu * 10.0).round() / 10.0;
            g.gpu = (g.gpu * 10.0).round() / 10.0;
            g.pids.sort_unstable();
            g.priority = g.pids.first().and_then(|&p| priority_of(p));
        }
        Usage {
            cpu: (cpu * 10.0).round() / 10.0,
            cpu_name,
            cores,
            memory_used,
            memory_total,
            gpu_supported,
            gpus: gpu.map(|s| s.adapters).unwrap_or_default(),
            disk,
            process_count,
            processes: list,
        }
    }

    /// Process ids of every running program called `name`.
    pub fn pids_named(&self, name: &str) -> Vec<u32> {
        let mut inner = self.inner.lock();
        inner.at = None;
        Self::refresh(&mut inner);
        inner.rows.iter().filter(|r| r.name.eq_ignore_ascii_case(name)).map(|r| r.pid).collect()
    }

    /// End every process called `name`; returns how many were ended.
    pub fn end(&self, name: &str) -> Result<usize, String> {
        if is_protected(name) {
            return Err(format!("{name} is part of Windows and cannot be ended here"));
        }
        let mut inner = self.inner.lock();
        inner.at = None;
        Self::refresh(&mut inner);
        let me = Pid::from_u32(std::process::id());
        let mut ended = 0;
        let mut failed = 0;
        for (pid, p) in inner.sys.processes() {
            if *pid == me || pid.as_u32() <= 4 || !p.name().to_string_lossy().eq_ignore_ascii_case(name) {
                continue;
            }
            if p.kill() {
                ended += 1;
            } else {
                failed += 1;
            }
        }
        match (ended, failed) {
            (0, 0) => Err(format!("{name} is not running")),
            (0, _) => Err(format!("Windows did not let OmniHub end {name} (it may need administrator rights)")),
            _ => Ok(ended),
        }
    }

    /// Set the priority of every process called `name`; returns how many
    /// changed.
    pub fn set_priority(&self, name: &str, priority: Priority) -> Result<usize, String> {
        if is_protected(name) {
            return Err(format!("{name} is part of Windows; its priority stays as Windows set it"));
        }
        let pids = self.pids_named(name);
        if pids.is_empty() {
            return Err(format!("{name} is not running"));
        }
        let changed = pids.iter().filter(|&&p| set_priority_of(p, priority).is_ok()).count();
        if changed == 0 {
            return Err(format!("Windows did not let OmniHub change {name} (it may need administrator rights)"));
        }
        Ok(changed)
    }
}

fn sort_groups(list: &mut [ProcessGroup], sort: ProcessSort) {
    match sort {
        ProcessSort::Cpu => list.sort_by(|a, b| b.cpu.total_cmp(&a.cpu).then(b.memory.cmp(&a.memory))),
        ProcessSort::Memory => list.sort_by_key(|g| std::cmp::Reverse(g.memory)),
        ProcessSort::Gpu => list.sort_by(|a, b| b.gpu.total_cmp(&a.gpu).then(b.gpu_memory.cmp(&a.gpu_memory)).then(b.cpu.total_cmp(&a.cpu))),
        ProcessSort::Disk => list.sort_by(|a, b| b.disk.cmp(&a.disk).then(b.cpu.total_cmp(&a.cpu))),
        ProcessSort::Name => list.sort_by_cached_key(|g| g.name.to_ascii_lowercase()),
    }
}

#[cfg(windows)]
fn priority_of(pid: u32) -> Option<Priority> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::*;
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let c = GetPriorityClass(h);
        let _ = CloseHandle(h);
        Some(match PROCESS_CREATION_FLAGS(c) {
            IDLE_PRIORITY_CLASS => Priority::Low,
            BELOW_NORMAL_PRIORITY_CLASS => Priority::BelowNormal,
            ABOVE_NORMAL_PRIORITY_CLASS => Priority::AboveNormal,
            HIGH_PRIORITY_CLASS | REALTIME_PRIORITY_CLASS => Priority::High,
            NORMAL_PRIORITY_CLASS => Priority::Normal,
            _ => return None,
        })
    }
}

#[cfg(not(windows))]
fn priority_of(_pid: u32) -> Option<Priority> {
    None
}

#[cfg(windows)]
pub fn set_priority_of(pid: u32, priority: Priority) -> Result<(), String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::*;
    let class = match priority {
        Priority::Low => IDLE_PRIORITY_CLASS,
        Priority::BelowNormal => BELOW_NORMAL_PRIORITY_CLASS,
        Priority::Normal => NORMAL_PRIORITY_CLASS,
        Priority::AboveNormal => ABOVE_NORMAL_PRIORITY_CLASS,
        Priority::High => HIGH_PRIORITY_CLASS,
    };
    unsafe {
        let h = OpenProcess(PROCESS_SET_INFORMATION, false, pid).map_err(|e| e.message().to_string())?;
        let r = SetPriorityClass(h, class).map_err(|e| e.message().to_string());
        let _ = CloseHandle(h);
        r
    }
}

#[cfg(not(windows))]
pub fn set_priority_of(_pid: u32, _priority: Priority) -> Result<(), String> {
    Err("Process priority is a Windows feature.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_and_protection() {
        let m = ProcessMonitor::new();
        let top = m.top(ProcessSort::Memory, 50);
        assert!(!top.is_empty());
        // Sorted by memory, and our own process is never endable.
        assert!(top.windows(2).all(|w| w[0].memory >= w[1].memory));
        let me = std::process::id();
        if let Some(g) = m.top(ProcessSort::Memory, 10_000).iter().find(|g| g.pids.contains(&me)) {
            assert!(!g.can_end);
        }
        assert!(is_protected("CSRSS.EXE") && is_protected("svchost.exe") && !is_protected("brave.exe"));
        assert!(m.end("lsass.exe").is_err());
        assert!(m.end("definitely-not-running-omnihub-test.exe").is_err());
        assert!(m.set_priority("csrss.exe", Priority::High).is_err());
        assert!(m.set_priority("definitely-not-running-omnihub-test.exe", Priority::High).is_err());
    }

    #[test]
    fn whole_pc_usage() {
        let m = ProcessMonitor::new();
        let u = m.usage(ProcessSort::Name, 10_000, true);
        assert!(u.memory_total > 0 && u.memory_used <= u.memory_total);
        assert!(u.cores > 0 && u.process_count >= u.processes.len());
        assert!(u.processes.windows(2).all(|w| w[0].name.to_ascii_lowercase() <= w[1].name.to_ascii_lowercase()));
        // Our own test binary is in the list, holding memory.
        let me = std::process::id();
        let mine = u.processes.iter().find(|g| g.pids.contains(&me)).expect("own process listed");
        assert!(mine.memory > 0 && !mine.can_end);
        assert!(m.pids_named(&mine.name).contains(&me));
        if !cfg!(windows) {
            assert!(!u.gpu_supported && u.gpus.is_empty());
        }
    }
}
