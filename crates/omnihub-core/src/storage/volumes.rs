//! Listing the drives that can be scanned.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VolumeInfo {
    /// Root path, e.g. `C:\` or `/home`.
    pub root: String,
    pub label: String,
    pub file_system: String,
    pub kind: DriveKind,
    pub total: u64,
    pub free: u64,
    pub cluster_size: u64,
    pub serial: Option<u64>,
    /// NTFS volumes on Windows can be read through the MFT.
    pub mft_capable: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DriveKind {
    Fixed,
    Removable,
    Network,
    Optical,
    Ram,
    Unknown,
}

/// Drive letter of a Windows volume root such as `C:\` or `c:`.
pub fn drive_letter(root: &str) -> Option<char> {
    let root = root.strip_prefix(r"\\?\").unwrap_or(root);
    let mut chars = root.chars();
    let letter = chars.next()?;
    if !letter.is_ascii_alphabetic() || chars.next()? != ':' {
        return None;
    }
    match chars.as_str() {
        "" | "\\" | "/" => Some(letter.to_ascii_uppercase()),
        _ => None,
    }
}

#[cfg(windows)]
pub fn list() -> Vec<VolumeInfo> {
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDiskFreeSpaceW, GetDriveTypeW, GetLogicalDriveStringsW, GetVolumeInformationW};

    let mut buf = vec![0u16; 512];
    let n = unsafe { GetLogicalDriveStringsW(Some(&mut buf)) } as usize;
    let mut out = Vec::new();
    for root in buf[..n.min(buf.len())].split(|&c| c == 0).filter(|s| !s.is_empty()) {
        let mut wide: Vec<u16> = root.to_vec();
        wide.push(0);
        let p = PCWSTR(wide.as_ptr());
        let root_str = String::from_utf16_lossy(root);
        let kind = match unsafe { GetDriveTypeW(p) } {
            2 => DriveKind::Removable,
            3 => DriveKind::Fixed,
            4 => DriveKind::Network,
            5 => DriveKind::Optical,
            6 => DriveKind::Ram,
            _ => DriveKind::Unknown,
        };
        let mut label = vec![0u16; 261];
        let mut fs = vec![0u16; 64];
        let mut serial = 0u32;
        // Fails for empty card readers and optical drives: skip those.
        if unsafe { GetVolumeInformationW(p, Some(&mut label), Some(&mut serial), None, None, Some(&mut fs)) }.is_err() {
            continue;
        }
        let (mut free, mut total) = (0u64, 0u64);
        let _ = unsafe { GetDiskFreeSpaceExW(p, None, Some(&mut total), Some(&mut free)) };
        let (mut spc, mut bps, mut fc, mut tc) = (0u32, 0u32, 0u32, 0u32);
        let cluster = if unsafe { GetDiskFreeSpaceW(p, Some(&mut spc), Some(&mut bps), Some(&mut fc), Some(&mut tc)) }.is_ok() {
            spc as u64 * bps as u64
        } else {
            4096
        };
        let fs_name = trim_wide(&fs);
        out.push(VolumeInfo {
            mft_capable: fs_name.eq_ignore_ascii_case("NTFS") && kind != DriveKind::Network,
            root: root_str,
            label: trim_wide(&label),
            file_system: fs_name,
            kind,
            total,
            free,
            cluster_size: cluster,
            serial: Some(serial as u64),
        });
    }
    out
}

#[cfg(windows)]
fn trim_wide(w: &[u16]) -> String {
    let end = w.iter().position(|&c| c == 0).unwrap_or(w.len());
    String::from_utf16_lossy(&w[..end])
}

#[cfg(not(windows))]
pub fn list() -> Vec<VolumeInfo> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut out: Vec<VolumeInfo> = disks
        .list()
        .iter()
        .filter(|d| {
            let fs = d.file_system().to_string_lossy();
            !matches!(fs.as_ref(), "overlay" | "squashfs" | "tmpfs" | "devtmpfs" | "proc" | "sysfs")
        })
        .map(|d| VolumeInfo {
            root: d.mount_point().to_string_lossy().to_string(),
            label: d.name().to_string_lossy().to_string(),
            file_system: d.file_system().to_string_lossy().to_string(),
            kind: if d.is_removable() { DriveKind::Removable } else { DriveKind::Fixed },
            total: d.total_space(),
            free: d.available_space(),
            cluster_size: 4096,
            serial: None,
            mft_capable: false,
        })
        .collect();
    if out.is_empty() {
        out.push(VolumeInfo {
            root: "/".into(),
            label: "Root".into(),
            file_system: "unknown".into(),
            kind: DriveKind::Fixed,
            total: 0,
            free: 0,
            cluster_size: 4096,
            serial: None,
            mft_capable: false,
        });
    }
    out
}

/// Total and free bytes of the volume holding `path`.
pub fn space_of(path: &str) -> Option<(u64, u64)> {
    let lower = path.to_lowercase();
    list()
        .into_iter()
        .filter(|v| lower.starts_with(&v.root.to_lowercase()))
        .max_by_key(|v| v.root.len())
        .map(|v| (v.total, v.free))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_letters() {
        assert_eq!(drive_letter("C:\\"), Some('C'));
        assert_eq!(drive_letter("d:"), Some('D'));
        assert_eq!(drive_letter(r"\\?\E:\"), Some('E'));
        assert_eq!(drive_letter("C:\\Users"), None);
        assert_eq!(drive_letter("/home"), None);
    }
}
