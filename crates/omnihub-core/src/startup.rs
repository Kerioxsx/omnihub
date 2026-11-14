//! Apps that start with Windows: the `Run` registry keys and the Startup
//! folders, switched on and off the way Task Manager does it — through the
//! `StartupApproved` keys, so nothing is deleted and Task Manager shows
//! the same state.
//!
//! Entries for all users live under HKEY_LOCAL_MACHINE; changing those asks
//! for administrator approval (the elevated helper job `startup-set`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StartupLocation {
    /// HKCU\…\Run — this user.
    RunUser,
    /// HKLM\…\Run — all users.
    RunMachine,
    /// HKLM\…\WOW6432Node\…\Run — all users, 32-bit apps.
    RunMachine32,
    /// The user's Startup folder.
    FolderUser,
    /// The all-users Startup folder.
    FolderCommon,
}

impl StartupLocation {
    pub const ALL: [StartupLocation; 5] = [Self::RunUser, Self::RunMachine, Self::RunMachine32, Self::FolderUser, Self::FolderCommon];

    pub fn key(self) -> &'static str {
        match self {
            Self::RunUser => "run-user",
            Self::RunMachine => "run-machine",
            Self::RunMachine32 => "run-machine32",
            Self::FolderUser => "folder-user",
            Self::FolderCommon => "folder-common",
        }
    }

    pub fn from_key(k: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|l| l.key() == k)
    }

    /// Changing it needs administrator rights.
    pub fn machine_wide(self) -> bool {
        !matches!(self, Self::RunUser | Self::FolderUser)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StartupItem {
    /// `<location>:<name>`.
    pub id: String,
    /// Registry value name, or the file name in a Startup folder.
    pub name: String,
    /// Shown name (file names without `.lnk`).
    pub display_name: String,
    pub command: String,
    /// The program the command starts, when it can be told.
    pub target: Option<String>,
    pub location: StartupLocation,
    pub enabled: bool,
    pub needs_admin: bool,
}

/// Whether a `StartupApproved` value means "enabled" (missing = enabled;
/// Task Manager writes 02/06 for on and 03/07 for off in the first byte).
pub fn approved_enabled(data: Option<&[u8]>) -> bool {
    data.and_then(|d| d.first()).is_none_or(|b| b & 1 == 0)
}

/// The value Task Manager writes: 02 + zeros (on), or 03 + the time it was
/// switched off (FILETIME) + zeros (off).
pub fn approved_value(enabled: bool, filetime: u64) -> Vec<u8> {
    let mut v = vec![0u8; 12];
    if enabled {
        v[0] = 2;
    } else {
        v[0] = 3;
        v[4..12].copy_from_slice(&filetime.to_le_bytes());
    }
    v
}

#[cfg_attr(not(windows), allow(dead_code))]
fn display_name(name: &str, loc: StartupLocation) -> String {
    if matches!(loc, StartupLocation::FolderUser | StartupLocation::FolderCommon) {
        std::path::Path::new(name).file_stem().map_or_else(|| name.to_string(), |s| s.to_string_lossy().into_owned())
    } else {
        name.to_string()
    }
}

pub fn list() -> Vec<StartupItem> {
    #[cfg(windows)]
    {
        win::list()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Switch an entry on or off; entries for all users ask for administrator approval.
pub fn set_enabled(id: &str, enabled: bool) -> std::io::Result<()> {
    let (loc, name) = id.split_once(':').and_then(|(l, n)| Some((StartupLocation::from_key(l)?, n))).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "unknown startup entry"))?;
    if !list().iter().any(|i| i.location == loc && i.name == name) {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "that startup entry no longer exists"));
    }
    if loc.machine_wide() && !crate::system::elevation::is_elevated() {
        let exe = std::env::current_exe()?;
        let args = vec![crate::helper::HELPER_FLAG.to_string(), "startup-set".into(), loc.key().into(), name.to_string(), if enabled { "1" } else { "0" }.into()];
        return match crate::system::elevation::run_elevated_and_wait(&exe, &args, false)? {
            0 => Ok(()),
            code => Err(std::io::Error::other(format!("the change was not made (code {code})"))),
        };
    }
    write_approved(loc, name, enabled)
}

/// Helper job (elevated): validate and apply a machine-wide change.
pub fn helper_set(loc: &str, name: &str, on: &str) -> i32 {
    let (Some(loc), Some(enabled)) = (StartupLocation::from_key(loc), match on {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }) else {
        return 64;
    };
    if name.is_empty() || name.len() > 260 || !list().iter().any(|i| i.location == loc && i.name == name) {
        return 64;
    }
    match write_approved(loc, name, enabled) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

fn write_approved(loc: StartupLocation, name: &str, enabled: bool) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        win::write_approved(loc, name, enabled)
    }
    #[cfg(not(windows))]
    {
        let _ = (loc, name, enabled);
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "startup apps are a Windows feature"))
    }
}

#[cfg(windows)]
mod win {
    use std::path::PathBuf;

    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, KEY_WOW64_64KEY};
    use winreg::{RegKey, RegValue};

    use super::*;

    const RUN: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const RUN32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Run";
    const APPROVED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved";

    fn hive(loc: StartupLocation) -> RegKey {
        RegKey::predef(if loc.machine_wide() { HKEY_LOCAL_MACHINE } else { HKEY_CURRENT_USER })
    }

    fn approved_key(loc: StartupLocation) -> String {
        let sub = match loc {
            StartupLocation::RunUser | StartupLocation::RunMachine => "Run",
            StartupLocation::RunMachine32 => "Run32",
            StartupLocation::FolderUser | StartupLocation::FolderCommon => "StartupFolder",
        };
        format!(r"{APPROVED}\{sub}")
    }

    fn folder(loc: StartupLocation) -> Option<PathBuf> {
        let rel = r"Microsoft\Windows\Start Menu\Programs\Startup";
        match loc {
            StartupLocation::FolderUser => std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join(rel)),
            StartupLocation::FolderCommon => std::env::var_os("PROGRAMDATA").map(|a| PathBuf::from(a).join(rel)),
            _ => None,
        }
    }

    fn approved_data(loc: StartupLocation, name: &str) -> Option<Vec<u8>> {
        hive(loc).open_subkey_with_flags(approved_key(loc), KEY_READ | KEY_WOW64_64KEY).ok()?.get_raw_value(name).ok().map(|v| v.bytes.into_owned())
    }

    pub fn list() -> Vec<StartupItem> {
        let mut out = Vec::new();
        for loc in StartupLocation::ALL {
            let entries: Vec<(String, String, Option<String>)> = match loc {
                StartupLocation::RunUser | StartupLocation::RunMachine | StartupLocation::RunMachine32 => {
                    let path = if loc == StartupLocation::RunMachine32 { RUN32 } else { RUN };
                    let Ok(key) = hive(loc).open_subkey_with_flags(path, KEY_READ | KEY_WOW64_64KEY) else { continue };
                    key.enum_values()
                        .flatten()
                        .filter_map(|(name, _)| {
                            let cmd: String = key.get_value(&name).ok()?;
                            let target = crate::apps::split_command(&cmd).map(|(exe, _)| exe);
                            Some((name, cmd, target))
                        })
                        .collect()
                }
                StartupLocation::FolderUser | StartupLocation::FolderCommon => {
                    let Some(dir) = folder(loc) else { continue };
                    let Ok(rd) = std::fs::read_dir(&dir) else { continue };
                    rd.flatten()
                        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
                        .filter_map(|e| {
                            let name = e.file_name().to_string_lossy().into_owned();
                            (!name.eq_ignore_ascii_case("desktop.ini")).then(|| (name, e.path().to_string_lossy().into_owned(), Some(e.path().to_string_lossy().into_owned())))
                        })
                        .collect()
                }
            };
            for (name, command, target) in entries {
                let enabled = approved_enabled(approved_data(loc, &name).as_deref());
                out.push(StartupItem { id: format!("{}:{name}", loc.key()), display_name: display_name(&name, loc), name, command, target, location: loc, enabled, needs_admin: loc.machine_wide() });
            }
        }
        out.sort_by_key(|i| i.display_name.to_lowercase());
        out
    }

    pub fn write_approved(loc: StartupLocation, name: &str, enabled: bool) -> std::io::Result<()> {
        let (key, _) = hive(loc).create_subkey_with_flags(approved_key(loc), KEY_SET_VALUE | KEY_WOW64_64KEY)?;
        let filetime = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| (d.as_secs() + 11_644_473_600) * 10_000_000);
        key.set_raw_value(name, &RegValue { bytes: approved_value(enabled, filetime).into(), vtype: winreg::enums::RegType::REG_BINARY })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_values_round_trip() {
        assert!(approved_enabled(None));
        assert!(approved_enabled(Some(&[2, 0, 0, 0])));
        assert!(approved_enabled(Some(&[6, 0])));
        assert!(!approved_enabled(Some(&[3, 0])));
        assert!(!approved_enabled(Some(&[7])));
        assert!(approved_enabled(Some(&approved_value(true, 0))));
        let off = approved_value(false, 0x0102_0304_0506_0708);
        assert!(!approved_enabled(Some(&off)));
        assert_eq!(off.len(), 12);
        assert_eq!(&off[4..12], &0x0102_0304_0506_0708u64.to_le_bytes());
    }

    #[test]
    fn ids_and_validation() {
        assert_eq!(StartupLocation::from_key("folder-common"), Some(StartupLocation::FolderCommon));
        assert!(StartupLocation::RunMachine32.machine_wide() && !StartupLocation::FolderUser.machine_wide());
        assert_eq!(display_name("Discord.lnk", StartupLocation::FolderUser), "Discord");
        assert_eq!(display_name("OneDrive", StartupLocation::RunUser), "OneDrive");
        assert!(set_enabled("nonsense", true).is_err());
        assert_eq!(helper_set("bogus", "x", "1"), 64);
        assert_eq!(helper_set("run-machine", "x", "maybe"), 64);
    }
}
