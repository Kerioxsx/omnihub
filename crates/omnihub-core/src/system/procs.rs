//! What is using the PC right now: running programs grouped by name (one
//! row for all of Brave's processes), with CPU and memory, and "End task".
//!
//! Processes Windows cannot run without, and OmniHub itself, are never
//! ended from here.

use std::collections::HashMap;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

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
    pub exe: Option<String>,
    pub pids: Vec<u32>,
    /// Whether "End task" is offered.
    pub can_end: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ProcessSort {
    #[default]
    Cpu,
    Memory,
}

pub fn is_protected(name: &str) -> bool {
    PROTECTED.contains(&name.to_ascii_lowercase().as_str())
}

pub struct ProcessMonitor {
    sys: Mutex<System>,
}

impl Default for ProcessMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessMonitor {
    pub fn new() -> Self {
        ProcessMonitor { sys: Mutex::new(System::new()) }
    }

    fn refresh(sys: &mut System) {
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing().with_cpu().with_memory().with_exe(sysinfo::UpdateKind::OnlyIfNotSet));
    }

    /// The busiest programs. CPU figures need two samples, so the first call
    /// after start reports 0 % for everything.
    pub fn top(&self, sort: ProcessSort, limit: usize) -> Vec<ProcessGroup> {
        let mut sys = self.sys.lock();
        if sys.cpus().is_empty() {
            sys.refresh_cpu_list(sysinfo::CpuRefreshKind::nothing());
        }
        Self::refresh(&mut sys);
        let cpus = sys.cpus().len().max(1) as f32;
        let me = std::process::id();
        let mut groups: HashMap<String, ProcessGroup> = HashMap::new();
        for (pid, p) in sys.processes() {
            let pid = pid.as_u32();
            if pid <= 4 {
                continue;
            }
            // Threads show up as processes on Linux; count each program once.
            if p.thread_kind().is_some() {
                continue;
            }
            let name = p.name().to_string_lossy().into_owned();
            if name.is_empty() {
                continue;
            }
            let g = groups.entry(name.to_ascii_lowercase()).or_insert_with(|| ProcessGroup {
                name: name.clone(),
                count: 0,
                cpu: 0.0,
                memory: 0,
                exe: p.exe().map(|e| e.to_string_lossy().into_owned()),
                pids: Vec::new(),
                can_end: !is_protected(&name),
            });
            g.count += 1;
            g.cpu += p.cpu_usage() / cpus;
            g.memory += p.memory();
            g.pids.push(pid);
            if pid == me {
                g.can_end = false;
            }
        }
        let mut list: Vec<ProcessGroup> = groups.into_values().collect();
        match sort {
            ProcessSort::Cpu => list.sort_by(|a, b| b.cpu.total_cmp(&a.cpu).then(b.memory.cmp(&a.memory))),
            ProcessSort::Memory => list.sort_by_key(|g| std::cmp::Reverse(g.memory)),
        }
        list.truncate(limit);
        for g in &mut list {
            g.cpu = (g.cpu * 10.0).round() / 10.0;
        }
        list
    }

    /// End every process called `name`; returns how many were ended.
    pub fn end(&self, name: &str) -> Result<usize, String> {
        if is_protected(name) {
            return Err(format!("{name} is part of Windows and cannot be ended here"));
        }
        let mut sys = self.sys.lock();
        Self::refresh(&mut sys);
        let me = Pid::from_u32(std::process::id());
        let mut ended = 0;
        let mut failed = 0;
        for (pid, p) in sys.processes() {
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
    }
}
