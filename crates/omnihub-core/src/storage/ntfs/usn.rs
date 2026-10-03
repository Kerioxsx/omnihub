//! NTFS USN change journal: which files changed since a snapshot was taken.
//!
//! The journal only reports *that* a file record changed, so the refresh
//! re-reads exactly those MFT records. A refresh after a day of normal use
//! touches a few thousand records instead of millions.

use std::collections::HashSet;
use std::io;

use windows::Win32::Foundation::{ERROR_HANDLE_EOF, ERROR_JOURNAL_DELETE_IN_PROGRESS, ERROR_JOURNAL_ENTRY_DELETED, ERROR_JOURNAL_NOT_ACTIVE, HANDLE};
use windows::Win32::System::Ioctl::{FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL, READ_USN_JOURNAL_DATA_V0, USN_JOURNAL_DATA_V0};
use windows::Win32::System::IO::DeviceIoControl;

use super::format::record_number;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalState {
    pub id: u64,
    pub first_usn: i64,
    pub next_usn: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum UsnError {
    /// The journal was deleted, recreated, or wrapped past our position:
    /// only a full scan can bring the snapshot up to date.
    #[error("change journal no longer covers the snapshot")]
    Reset,
    #[error(transparent)]
    Io(#[from] io::Error),
}

fn map_err(e: windows::core::Error) -> UsnError {
    let code = e.code();
    if code == ERROR_JOURNAL_ENTRY_DELETED.to_hresult()
        || code == ERROR_JOURNAL_NOT_ACTIVE.to_hresult()
        || code == ERROR_JOURNAL_DELETE_IN_PROGRESS.to_hresult()
    {
        UsnError::Reset
    } else {
        UsnError::Io(io::Error::from_raw_os_error(code.0 & 0xFFFF))
    }
}

pub fn query(handle: HANDLE) -> Result<JournalState, UsnError> {
    let mut data = USN_JOURNAL_DATA_V0::default();
    let mut returned = 0u32;
    unsafe {
        DeviceIoControl(
            handle,
            FSCTL_QUERY_USN_JOURNAL,
            None,
            0,
            Some(&mut data as *mut _ as *mut _),
            std::mem::size_of::<USN_JOURNAL_DATA_V0>() as u32,
            Some(&mut returned),
            None,
        )
    }
    .map_err(map_err)?;
    Ok(JournalState { id: data.UsnJournalID, first_usn: data.FirstUsn, next_usn: data.NextUsn })
}

/// Record numbers changed between `from_usn` and `until_usn`.
pub fn changed_records(handle: HANDLE, journal_id: u64, from_usn: i64, until_usn: i64) -> Result<HashSet<u64>, UsnError> {
    let mut changed = HashSet::new();
    let mut buf = vec![0u8; 1 << 20];
    let mut start = from_usn;
    while start < until_usn {
        let req = READ_USN_JOURNAL_DATA_V0 {
            StartUsn: start,
            ReasonMask: 0xFFFF_FFFF,
            ReturnOnlyOnClose: 0,
            Timeout: 0,
            BytesToWaitFor: 0,
            UsnJournalID: journal_id,
        };
        let mut returned = 0u32;
        let res = unsafe {
            DeviceIoControl(
                handle,
                FSCTL_READ_USN_JOURNAL,
                Some(&req as *const _ as *const _),
                std::mem::size_of::<READ_USN_JOURNAL_DATA_V0>() as u32,
                Some(buf.as_mut_ptr() as *mut _),
                buf.len() as u32,
                Some(&mut returned),
                None,
            )
        };
        if let Err(e) = res {
            if e.code() == ERROR_HANDLE_EOF.to_hresult() {
                break;
            }
            return Err(map_err(e));
        }
        let data = &buf[..returned as usize];
        if data.len() < 8 {
            break;
        }
        let next = i64::from_le_bytes(data[0..8].try_into().unwrap());
        let mut pos = 8usize;
        while pos + 8 <= data.len() {
            let len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
            if len == 0 || pos + len > data.len() {
                break;
            }
            let major = u16::from_le_bytes(data[pos + 4..pos + 6].try_into().unwrap());
            match major {
                2 if len >= 24 => {
                    let frn = u64::from_le_bytes(data[pos + 8..pos + 16].try_into().unwrap());
                    let parent = u64::from_le_bytes(data[pos + 16..pos + 24].try_into().unwrap());
                    changed.insert(record_number(frn));
                    changed.insert(record_number(parent));
                }
                3 if len >= 40 => {
                    // 128-bit ids; on NTFS the low 64 bits are the reference.
                    let frn = u64::from_le_bytes(data[pos + 8..pos + 16].try_into().unwrap());
                    let parent = u64::from_le_bytes(data[pos + 24..pos + 32].try_into().unwrap());
                    changed.insert(record_number(frn));
                    changed.insert(record_number(parent));
                }
                _ => {}
            }
            pos += len;
        }
        if next <= start {
            break;
        }
        start = next;
    }
    Ok(changed)
}
