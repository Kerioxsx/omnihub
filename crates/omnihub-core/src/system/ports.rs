//! "What is using port 3000?": every TCP and UDP port in use, with the
//! program that has it, from Windows' own connection tables.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortUse {
    /// "TCP" or "UDP".
    pub proto: String,
    pub local: String,
    pub port: u16,
    pub remote: Option<String>,
    pub remote_port: Option<u16>,
    /// "Listening", "Connected", … (TCP only).
    pub state: String,
    pub pid: u32,
    pub process: String,
}

/// TCP states by their Windows number.
pub fn state_name(n: u32) -> &'static str {
    match n {
        1 => "Closed",
        2 => "Listening",
        3 => "Connecting",
        4 => "Connecting",
        5 => "Connected",
        6..=7 => "Closing",
        8 => "Closing (remote closed)",
        9..=10 => "Closing",
        11 => "Waiting to close",
        12 => "Closing",
        _ => "",
    }
}

/// Listening ports first, then by port.
pub fn sort(v: &mut [PortUse]) {
    v.sort_by(|a, b| (a.state != "Listening" && a.proto == "TCP").cmp(&(b.state != "Listening" && b.proto == "TCP")).then(a.port.cmp(&b.port)).then(a.proto.cmp(&b.proto)));
}

/// What a search box finds: a port number, a program name or an address.
pub fn matches(p: &PortUse, query: &str) -> bool {
    let q = query.trim().trim_start_matches(':').to_lowercase();
    if q.is_empty() {
        return true;
    }
    if let Ok(n) = q.parse::<u16>() {
        return p.port == n || p.remote_port == Some(n) || p.pid == n as u32;
    }
    p.process.to_lowercase().contains(&q) || p.local.contains(&q) || p.remote.as_deref().is_some_and(|r| r.contains(&q))
}

#[cfg(windows)]
pub fn list() -> Vec<PortUse> {
    let mut v = imp::list();
    sort(&mut v);
    v
}

#[cfg(not(windows))]
pub fn list() -> Vec<PortUse> {
    Vec::new()
}

/// End the program with this process id (not Windows' own).
pub fn end_process(pid: u32) -> Result<(), String> {
    if pid <= 4 {
        return Err("That's Windows itself.".into());
    }
    #[cfg(windows)]
    {
        imp::end(pid)
    }
    #[cfg(not(windows))]
    {
        Err("Windows only.".into())
    }
}

#[cfg(windows)]
mod imp {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use windows::core::PWSTR;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::NetworkManagement::IpHelper::{GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCPROW_OWNER_PID, MIB_UDP6ROW_OWNER_PID, MIB_UDPROW_OWNER_PID, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID};
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, TerminateProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE};

    use super::{state_name, PortUse};

    const AF_INET: u32 = 2;
    const AF_INET6: u32 = 23;

    fn name_of(pid: u32, cache: &mut std::collections::HashMap<u32, String>) -> String {
        if let Some(n) = cache.get(&pid) {
            return n.clone();
        }
        let name = match pid {
            0 => "System Idle".to_string(),
            4 => "System".to_string(),
            _ => unsafe {
                OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
                    .ok()
                    .and_then(|h| {
                        let mut buf = [0u16; 1024];
                        let mut len = buf.len() as u32;
                        let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
                        let _ = CloseHandle(h);
                        ok.then(|| String::from_utf16_lossy(&buf[..len as usize]))
                    })
                    .map(|p| p.rsplit('\\').next().unwrap_or(&p).to_string())
                    .unwrap_or_else(|| format!("Process {pid}"))
            },
        };
        cache.insert(pid, name.clone());
        name
    }

    /// A table from Windows: the number of rows, then the rows.
    fn table(get: impl Fn(Option<*mut core::ffi::c_void>, *mut u32) -> u32) -> Vec<u8> {
        let mut size = 0u32;
        get(None, &mut size);
        for _ in 0..3 {
            let mut buf = vec![0u8; size as usize + 4096];
            let mut len = buf.len() as u32;
            if get(Some(buf.as_mut_ptr() as *mut _), &mut len) == 0 {
                buf.truncate(len as usize);
                return buf;
            }
            size = len;
        }
        Vec::new()
    }

    fn rows<T: Copy>(buf: &[u8]) -> Vec<T> {
        if buf.len() < 4 {
            return Vec::new();
        }
        let n = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
        let size = std::mem::size_of::<T>();
        // Rows start after the count, aligned like the struct (4 bytes for these).
        (0..n).filter_map(|i| buf.get(4 + i * size..4 + (i + 1) * size).map(|b| unsafe { std::ptr::read_unaligned(b.as_ptr() as *const T) })).collect()
    }

    fn port(p: u32) -> u16 {
        u16::from_be((p & 0xffff) as u16)
    }

    pub fn list() -> Vec<PortUse> {
        let mut names = std::collections::HashMap::new();
        let mut out = Vec::new();
        let tcp4 = table(|b, s| unsafe { GetExtendedTcpTable(b, s, false, AF_INET, TCP_TABLE_OWNER_PID_ALL, 0) });
        for r in rows::<MIB_TCPROW_OWNER_PID>(&tcp4) {
            let listening = r.dwState == 2;
            out.push(PortUse {
                proto: "TCP".into(),
                local: Ipv4Addr::from(r.dwLocalAddr.to_le_bytes()).to_string(),
                port: port(r.dwLocalPort),
                remote: (!listening).then(|| Ipv4Addr::from(r.dwRemoteAddr.to_le_bytes()).to_string()),
                remote_port: (!listening).then(|| port(r.dwRemotePort)),
                state: state_name(r.dwState).into(),
                pid: r.dwOwningPid,
                process: name_of(r.dwOwningPid, &mut names),
            });
        }
        let tcp6 = table(|b, s| unsafe { GetExtendedTcpTable(b, s, false, AF_INET6, TCP_TABLE_OWNER_PID_ALL, 0) });
        for r in rows::<MIB_TCP6ROW_OWNER_PID>(&tcp6) {
            let listening = r.dwState == 2;
            out.push(PortUse {
                proto: "TCP".into(),
                local: Ipv6Addr::from(r.ucLocalAddr).to_string(),
                port: port(r.dwLocalPort),
                remote: (!listening).then(|| Ipv6Addr::from(r.ucRemoteAddr).to_string()),
                remote_port: (!listening).then(|| port(r.dwRemotePort)),
                state: state_name(r.dwState).into(),
                pid: r.dwOwningPid,
                process: name_of(r.dwOwningPid, &mut names),
            });
        }
        let udp4 = table(|b, s| unsafe { GetExtendedUdpTable(b, s, false, AF_INET, UDP_TABLE_OWNER_PID, 0) });
        for r in rows::<MIB_UDPROW_OWNER_PID>(&udp4) {
            out.push(PortUse { proto: "UDP".into(), local: Ipv4Addr::from(r.dwLocalAddr.to_le_bytes()).to_string(), port: port(r.dwLocalPort), remote: None, remote_port: None, state: String::new(), pid: r.dwOwningPid, process: name_of(r.dwOwningPid, &mut names) });
        }
        let udp6 = table(|b, s| unsafe { GetExtendedUdpTable(b, s, false, AF_INET6, UDP_TABLE_OWNER_PID, 0) });
        for r in rows::<MIB_UDP6ROW_OWNER_PID>(&udp6) {
            out.push(PortUse { proto: "UDP".into(), local: Ipv6Addr::from(r.ucLocalAddr).to_string(), port: port(r.dwLocalPort), remote: None, remote_port: None, state: String::new(), pid: r.dwOwningPid, process: name_of(r.dwOwningPid, &mut names) });
        }
        out
    }

    pub fn end(pid: u32) -> Result<(), String> {
        let name = name_of(pid, &mut Default::default());
        if crate::system::procs::is_protected(&name) {
            return Err(format!("{name} is part of Windows; ending it could crash the PC."));
        }
        unsafe {
            let h = OpenProcess(PROCESS_TERMINATE, false, pid).map_err(|e| format!("Couldn't end {name}: {}", e.message()))?;
            let r = TerminateProcess(h, 1).map_err(|e| format!("Couldn't end {name}: {}", e.message()));
            let _ = CloseHandle(h);
            r
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(proto: &str, port: u16, state: &str, process: &str) -> PortUse {
        PortUse { proto: proto.into(), local: "127.0.0.1".into(), port, remote: None, remote_port: None, state: state.into(), pid: 1234, process: process.into() }
    }

    #[test]
    fn search_and_order() {
        let mut v = vec![p("TCP", 50000, "Connected", "chrome.exe"), p("TCP", 3000, "Listening", "node.exe"), p("UDP", 53, "", "dns.exe")];
        sort(&mut v);
        assert_eq!(v.iter().map(|x| x.port).collect::<Vec<_>>(), vec![53, 3000, 50000]);
        assert!(matches(&v[1], "3000") && matches(&v[1], ":3000") && matches(&v[1], "node"));
        assert!(!matches(&v[1], "3001"));
        assert_eq!(state_name(2), "Listening");
        assert!(end_process(4).is_err());
    }
}
