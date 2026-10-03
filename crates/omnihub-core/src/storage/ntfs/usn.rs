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

/// Changes are only "settled" once NTFS has had time to write the MFT
/// records back to disk (the scanner reads the volume, not the cache).
/// Anything younger is read now *and* replayed on the next refresh.
pub const SETTLE_SECS: u64 = 15;

/// Current time as a Windows FILETIME minus `secs`.
pub fn filetime_ago(secs: u64) -> i64 {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    ((now.as_secs().saturating_sub(secs) + 11_644_473_600) * 10_000_000) as i64
}

#[derive(Debug, Default)]
pub struct Changes {
    /// Record numbers of changed files and their parent folders.
    pub records: HashSet<u64>,
    /// Where the next refresh should start: the first change younger than
    /// the settle cutoff, or the end of what was read.
    pub resume_usn: i64,
}

struct Rec {
    frn: u64,
    parent: u64,
    usn: i64,
    time: i64,
}

fn parse_records(data: &[u8], out: &mut Vec<Rec>) {
    let mut pos = 8usize;
    while pos + 8 <= data.len() {
        let len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        if len == 0 || pos + len > data.len() {
            break;
        }
        let r = &data[pos..pos + len];
        let major = u16::from_le_bytes(r[4..6].try_into().unwrap());
        let u64_at = |o: usize| u64::from_le_bytes(r[o..o + 8].try_into().unwrap());
        match major {
            2 if len >= 60 => out.push(Rec { frn: u64_at(8), parent: u64_at(16), usn: u64_at(24) as i64, time: u64_at(32) as i64 }),
            // 128-bit ids; on NTFS the low 64 bits are the file reference.
            3 if len >= 76 => out.push(Rec { frn: u64_at(8), parent: u64_at(24), usn: u64_at(40) as i64, time: u64_at(48) as i64 }),
            _ => {}
        }
        pos += len;
    }
}

/// Read journal records in `[from_usn, until_usn)`, calling `f` for each.
fn read_journal(handle: HANDLE, journal_id: u64, from_usn: i64, until_usn: i64, mut f: impl FnMut(&Rec) -> bool) -> Result<i64, UsnError> {
    let mut buf = vec![0u8; 1 << 20];
    let mut start = from_usn;
    let mut recs = Vec::new();
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
        recs.clear();
        parse_records(data, &mut recs);
        for r in &recs {
            if r.usn >= until_usn || !f(r) {
                return Ok(r.usn);
            }
        }
        if next <= start {
            break;
        }
        start = next;
    }
    Ok(start.min(until_usn).max(from_usn))
}

/// Files changed between `from_usn` and `until_usn`.
pub fn changed_records(handle: HANDLE, journal_id: u64, from_usn: i64, until_usn: i64) -> Result<Changes, UsnError> {
    let cutoff = filetime_ago(SETTLE_SECS);
    let mut changes = Changes { records: HashSet::new(), resume_usn: until_usn };
    let mut young: Option<i64> = None;
    read_journal(handle, journal_id, from_usn, until_usn, |r| {
        changes.records.insert(record_number(r.frn));
        changes.records.insert(record_number(r.parent));
        if r.time >= cutoff && young.is_none() {
            young = Some(r.usn);
        }
        true
    })?;
    if let Some(u) = young {
        changes.resume_usn = u;
    }
    Ok(changes)
}

/// The journal position of the first change in the last `SETTLE_SECS`
/// (used as the baseline of a full scan, so changes that may not have
/// reached the disk yet are replayed by the next refresh).
pub fn settled_position(handle: HANDLE, j: &JournalState) -> i64 {
    let cutoff = filetime_ago(SETTLE_SECS);
    // USNs are byte offsets into the journal; the last 8 MiB covers far
    // more than a few seconds of changes.
    let from = j.first_usn.max(j.next_usn - (8 << 20));
    let mut pos = j.next_usn;
    let res = read_journal(handle, j.id, from, j.next_usn, |r| {
        if r.time >= cutoff {
            pos = r.usn;
            return false;
        }
        true
    });
    match res {
        Ok(_) => pos,
        Err(_) => j.next_usn,
    }
}
