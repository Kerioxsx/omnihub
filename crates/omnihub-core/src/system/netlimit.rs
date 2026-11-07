//! Slow down an app's internet: a Windows QoS policy caps how fast a program
//! sends (its upload), so a cloud backup or a seeding torrent stops choking a
//! game or a call. Windows' QoS only shapes outgoing traffic, so downloads
//! aren't capped; making the policy needs administrator approval.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::games::tweaks::{valid_exe_name, AdminOp};

pub const MIN_KBPS: u32 = 64;
pub const MAX_KBPS: u32 = 1_000_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppLimit {
    /// "OneDrive.exe"
    pub exe: String,
    /// Kilobits per second (1000 bits).
    pub kbps: u32,
    /// Whether Windows has the policy right now.
    #[serde(default)]
    pub active: bool,
}

pub fn policy_name(exe: &str) -> String {
    format!("OmniHub limit - {exe}")
}

pub fn check(exe: &str, kbps: u32) -> Result<(), String> {
    if !valid_exe_name(exe) {
        return Err("Type the program's file name, like OneDrive.exe".into());
    }
    if !(MIN_KBPS..=MAX_KBPS).contains(&kbps) {
        return Err(format!("Pick a speed between {MIN_KBPS} Kbps and {} Mbps", MAX_KBPS / 1000));
    }
    Ok(())
}

pub struct NetLimits {
    file: PathBuf,
    list: Mutex<Vec<AppLimit>>,
}

impl NetLimits {
    pub fn new(data: &Path) -> NetLimits {
        let file = data.join("net-limits.json");
        let list = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        NetLimits { file, list: Mutex::new(list) }
    }

    pub fn list(&self) -> Vec<AppLimit> {
        let mut l = self.list.lock().clone();
        for x in l.iter_mut() {
            x.active = imp::active(&x.exe);
        }
        l
    }

    fn store(&self, l: Vec<AppLimit>) -> Result<(), String> {
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&l).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        *self.list.lock() = l;
        Ok(())
    }

    /// Remember a limit (after Windows made the policy).
    pub fn remember(&self, exe: &str, kbps: u32) -> Result<Vec<AppLimit>, String> {
        check(exe, kbps)?;
        let mut l = self.list.lock().clone();
        l.retain(|x| !x.exe.eq_ignore_ascii_case(exe));
        if l.len() >= 30 {
            return Err("Keep it to 30 limits".into());
        }
        l.push(AppLimit { exe: exe.to_string(), kbps, active: false });
        l.sort_by_key(|x| x.exe.to_lowercase());
        self.store(l)?;
        Ok(self.list())
    }

    pub fn forget(&self, exe: &str) -> Result<Vec<AppLimit>, String> {
        let mut l = self.list.lock().clone();
        l.retain(|x| !x.exe.eq_ignore_ascii_case(exe));
        self.store(l)?;
        Ok(self.list())
    }

    /// Cap a program's upload (asks for administrator approval).
    pub fn set(&self, exe: &str, kbps: u32) -> Result<Vec<AppLimit>, String> {
        check(exe, kbps)?;
        crate::games::tweaks::run_admin(&[AdminOp::LimitOn(exe.to_string(), kbps)])?;
        self.remember(exe, kbps)
    }

    pub fn remove(&self, exe: &str) -> Result<Vec<AppLimit>, String> {
        if !valid_exe_name(exe) {
            return Err("That isn't a program name".into());
        }
        if imp::active(exe) {
            crate::games::tweaks::run_admin(&[AdminOp::LimitOff(exe.to_string())])?;
        }
        self.forget(exe)
    }
}

#[cfg(windows)]
mod imp {
    pub fn active(exe: &str) -> bool {
        let key = format!(r"SOFTWARE\Policies\Microsoft\Windows\QoS\{}", super::policy_name(exe));
        winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE).open_subkey(key).and_then(|k| k.get_value::<String, _>("Application Name")).is_ok_and(|a| a.eq_ignore_ascii_case(exe))
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn active(_exe: &str) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembers_limits() {
        let t = tempfile::tempdir().unwrap();
        let n = NetLimits::new(t.path());
        assert!(check("OneDrive.exe", 10).is_err());
        assert!(check("evil'.exe", 1000).is_err());
        assert!(check("OneDrive.exe", 2000).is_ok());
        n.remember("qbittorrent.exe", 4000).unwrap();
        let l = n.remember("OneDrive.exe", 2000).unwrap();
        assert_eq!(l.iter().map(|x| x.exe.as_str()).collect::<Vec<_>>(), vec!["OneDrive.exe", "qbittorrent.exe"]);
        // Setting it again replaces it.
        assert_eq!(n.remember("onedrive.exe", 1000).unwrap().len(), 2);
        assert_eq!(NetLimits::new(t.path()).list().len(), 2);
        assert_eq!(n.forget("OneDrive.exe").unwrap().len(), 1);
        assert_eq!(policy_name("OneDrive.exe"), "OmniHub limit - OneDrive.exe");
    }
}
