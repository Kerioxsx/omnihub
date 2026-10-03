//! Whole-volume MFT scans with a persistent snapshot and incremental
//! refresh through the USN change journal.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
#[cfg(windows)]
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};

use super::ntfs::mft::MftProgress;

/// Progress as written by the elevated helper for the app to read.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HelperProgress {
    pub phase: String,
    pub records_done: u64,
    pub records_total: u64,
    pub bytes_read: u64,
}

impl HelperProgress {
    pub fn from_mft(phase: &str, p: &MftProgress) -> Self {
        HelperProgress {
            phase: phase.to_string(),
            records_done: p.records_done.load(Ordering::Relaxed),
            records_total: p.records_total.load(Ordering::Relaxed),
            bytes_read: p.bytes_read.load(Ordering::Relaxed),
        }
    }
}

pub fn snapshot_path(dir: &Path, letter: char) -> PathBuf {
    dir.join(format!("vol-{}.ohsnap", letter.to_ascii_uppercase()))
}

pub fn progress_path(dir: &Path, letter: char) -> PathBuf {
    dir.join(format!("vol-{}.progress.json", letter.to_ascii_uppercase()))
}

pub fn cancel_path(dir: &Path, letter: char) -> PathBuf {
    dir.join(format!("vol-{}.cancel", letter.to_ascii_uppercase()))
}

pub fn error_path(dir: &Path, letter: char) -> PathBuf {
    dir.join(format!("vol-{}.error.txt", letter.to_ascii_uppercase()))
}

#[derive(Debug, thiserror::Error)]
pub enum VolumeScanError {
    #[error("administrator rights are needed to read the MFT ({0})")]
    AccessDenied(String),
    #[error("{0}")]
    Mft(#[from] super::ntfs::mft::MftError),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("scan cancelled")]
    Cancelled,
}

/// Scan an NTFS volume through its MFT, reusing the cached snapshot and the
/// change journal when they are still valid. The snapshot is saved to
/// `cache_dir` before returning.
#[cfg(windows)]
pub fn scan_ntfs_volume(
    letter: char,
    cache_dir: &Path,
    allow_incremental: bool,
    progress: &MftProgress,
    cancel: &AtomicBool,
    on_phase: &dyn Fn(&str),
) -> Result<super::snapshot::VolumeSnapshot, VolumeScanError> {
    use std::time::Instant;

    use super::ntfs::mft::MftVolume;
    use super::ntfs::usn;
    use super::ntfs::volume::VolumeFile;
    use super::snapshot::{JournalPosition, Slot, SnapshotMeta, VolumeSnapshot};
    use super::tree::ScanMethod;

    let started = Instant::now();
    let file = VolumeFile::open(letter).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            VolumeScanError::AccessDenied(e.to_string())
        } else {
            VolumeScanError::Io(e)
        }
    })?;
    let handle = file.handle();
    // Journal position before reading: anything that changes during the
    // scan is picked up by the next refresh.
    let journal = usn::query(handle).ok();
    // Baseline for the next refresh: just before the changes of the last few
    // seconds, which may not be on disk yet.
    let baseline = journal.map(|j| usn::settled_position(handle, &j));
    on_phase("opening");
    let vol = MftVolume::open(file)?;
    let root = format!("{}:\\", letter.to_ascii_uppercase());
    let (volume_total, volume_free) = super::volumes::space_of(&root).map_or((None, None), |(t, f)| (Some(t), Some(f)));
    let snap_path = snapshot_path(cache_dir, letter);

    if allow_incremental {
        if let (Some(j), Ok(mut snap)) = (journal, VolumeSnapshot::load(&snap_path)) {
            let usable = snap.meta.volume_serial == vol.boot.serial
                && snap.meta.journal.is_some_and(|p| p.journal_id == j.id && p.next_usn >= j.first_usn && p.next_usn <= j.next_usn);
            if usable {
                on_phase("journal");
                let from = snap.meta.journal.unwrap().next_usn;
                match usn::changed_records(handle, j.id, from, j.next_usn) {
                    Ok(changes) if (changes.records.len() as u64) < vol.record_count() / 5 => {
                        on_phase("refresh");
                        let changed = changes.records;
                        let total = vol.record_count() as usize;
                        progress.records_total.store(changed.len() as u64, Ordering::Relaxed);
                        if snap.slots.len() < total {
                            snap.slots.resize_with(total, Slot::default);
                        }
                        for (i, rec) in changed.iter().enumerate() {
                            if cancel.load(Ordering::Relaxed) {
                                return Err(VolumeScanError::Cancelled);
                            }
                            let rec = *rec as usize;
                            if rec >= snap.slots.len() {
                                continue;
                            }
                            snap.slots[rec] = vol.read_file_slot(rec as u64)?.unwrap_or_default();
                            progress.records_done.store(i as u64 + 1, Ordering::Relaxed);
                        }
                        // Recent changes are replayed next time (see usn::SETTLE_SECS).
                        snap.meta.journal = Some(JournalPosition { journal_id: j.id, next_usn: changes.resume_usn });
                        snap.meta.scanned_at = chrono::Utc::now().timestamp();
                        snap.meta.duration_ms = started.elapsed().as_millis() as u64;
                        snap.meta.method = ScanMethod::MftIncremental;
                        snap.meta.records_changed = changed.len() as u64;
                        snap.meta.volume_total = volume_total;
                        snap.meta.volume_free = volume_free;
                        snap.save(&snap_path)?;
                        return Ok(snap);
                    }
                    // Too many changes (a full sequential read is faster) or
                    // the journal wrapped: fall through to a full scan.
                    _ => {}
                }
            }
        }
    }

    on_phase("mft");
    let slots = vol.scan(progress, cancel).map_err(|e| match e {
        super::ntfs::mft::MftError::Cancelled => VolumeScanError::Cancelled,
        other => VolumeScanError::Mft(other),
    })?;
    let snap = VolumeSnapshot {
        meta: SnapshotMeta {
            root_path: root,
            volume_serial: vol.boot.serial,
            cluster_size: vol.boot.cluster_size,
            record_size: vol.boot.record_size,
            volume_total,
            volume_free,
            journal: journal.zip(baseline).map(|(j, b)| JournalPosition { journal_id: j.id, next_usn: b }),
            scanned_at: chrono::Utc::now().timestamp(),
            duration_ms: started.elapsed().as_millis() as u64,
            method: ScanMethod::Mft,
            records_changed: 0,
        },
        slots,
    };
    on_phase("saving");
    snap.save(&snap_path)?;
    Ok(snap)
}

/// Is `dir` a plausible OmniHub scan cache directory (…\OmniHub\cache\scans)?
/// The elevated helper only writes where this holds, and refuses paths that
/// go through junctions or symlinks.
pub fn is_valid_scan_dir(dir: &Path) -> bool {
    if !dir.is_absolute() || dir.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        return false;
    }
    let names: Vec<String> = dir
        .components()
        .rev()
        .take(3)
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    if names.len() < 3 || names[0] != "scans" || names[1] != "cache" || names[2] != crate::paths::APP_DIR_NAME.to_lowercase() {
        return false;
    }
    let mut cur = Some(dir);
    while let Some(p) = cur {
        if let Ok(m) = std::fs::symlink_metadata(p) {
            if m.file_type().is_symlink() {
                return false;
            }
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                // A junction anywhere above the scan folder could redirect
                // the elevated writer; drive roots are fine.
                if m.file_attributes() & 0x400 != 0 && p.parent().is_some() {
                    return false;
                }
            }
        }
        cur = p.parent();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_dir_validation() {
        let tmp = tempfile::tempdir().unwrap();
        let good = tmp.path().join("OmniHub").join("cache").join("scans");
        std::fs::create_dir_all(&good).unwrap();
        assert!(is_valid_scan_dir(&good));
        assert!(!is_valid_scan_dir(&tmp.path().join("elsewhere")));
        assert!(!is_valid_scan_dir(Path::new("relative/OmniHub/cache/scans")));
        assert!(!is_valid_scan_dir(&good.join("..").join("scans")));
        #[cfg(unix)]
        {
            let link = tmp.path().join("link");
            std::os::unix::fs::symlink(tmp.path().join("OmniHub"), &link).unwrap();
            let via = tmp.path().join("x").join("OmniHub");
            std::fs::create_dir_all(via.parent().unwrap()).unwrap();
            std::os::unix::fs::symlink(tmp.path().join("OmniHub"), &via).unwrap();
            assert!(!is_valid_scan_dir(&via.join("cache").join("scans")));
        }
    }
}
