//! Live tests on a real Windows machine (they run in CI on windows-latest,
//! where the runner is an administrator). Skipped elsewhere.
#![cfg(windows)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use omnihub_core::storage::ntfs::mft::{MftProgress, MftVolume};
use omnihub_core::storage::ntfs::volume::VolumeFile;
use omnihub_core::storage::snapshot::{SnapshotMeta, VolumeSnapshot};
use omnihub_core::storage::tree::{ScanMethod, ScanTree};
use omnihub_core::storage::volume_scan;
use omnihub_core::system::elevation::is_elevated;

fn long_path(p: &Path) -> String {
    let c = std::fs::canonicalize(p).unwrap();
    let s = c.to_string_lossy().to_string();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

fn write_file(path: &Path, len: usize) {
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(&vec![0x5Au8; len]).unwrap();
    // Push the metadata to disk: the scanner reads the volume directly.
    f.sync_all().unwrap();
}

fn full_scan(letter: char) -> (ScanTree, Duration, usize) {
    let started = Instant::now();
    let vol = MftVolume::open(VolumeFile::open(letter).unwrap()).unwrap();
    let slots = vol.scan(&MftProgress::default(), &AtomicBool::new(false)).unwrap();
    let elapsed = started.elapsed();
    let records = slots.len();
    let snap = VolumeSnapshot {
        meta: SnapshotMeta {
            root_path: format!("{letter}:\\"),
            volume_serial: vol.boot.serial,
            cluster_size: vol.boot.cluster_size,
            record_size: vol.boot.record_size,
            volume_total: None,
            volume_free: None,
            journal: None,
            scanned_at: 0,
            duration_ms: elapsed.as_millis() as u64,
            method: ScanMethod::Mft,
            records_changed: 0,
        },
        slots,
    };
    (snap.to_tree(false), elapsed, records)
}

/// Retry while freshly written metadata is still only in the cache.
fn find_with_retry(letter: char, path: &str) -> (ScanTree, u32) {
    for attempt in 0..6 {
        let (tree, elapsed, records) = full_scan(letter);
        if attempt == 0 {
            let root = tree.node(0).unwrap();
            println!(
                "MFT scan of {letter}: {records} records, {} files, {} folders, {:.1} GiB in {:?} (tree build excluded)",
                root.files,
                root.desc - root.files,
                root.size as f64 / (1u64 << 30) as f64,
                elapsed
            );
        }
        if let Some(id) = tree.find_path(path) {
            return (tree, id);
        }
        std::thread::sleep(Duration::from_secs(3));
    }
    panic!("{path} not found in the MFT scan");
}

#[test]
fn mft_scan_of_the_system_drive() {
    if !is_elevated() {
        eprintln!("not elevated; skipping live MFT test");
        return;
    }
    let base = tempfile::tempdir_in(std::env::temp_dir()).unwrap();
    let root = base.path().join("omnihub-live");
    std::fs::create_dir_all(root.join("sub")).unwrap();
    write_file(&root.join("big.bin"), 3_000_000);
    write_file(&root.join("small.txt"), 17);
    write_file(&root.join("sub").join("nested.dat"), 70_000);
    let root_s = long_path(&root);
    let letter = root_s.chars().next().unwrap();

    let (tree, id) = find_with_retry(letter, &root_s);
    let node = tree.node(id).unwrap();
    assert!(node.is_dir());
    assert_eq!(node.size, 3_000_000 + 17 + 70_000, "folder total");
    assert_eq!(node.files, 3);
    let big = tree.find_path(&format!("{root_s}\\big.bin")).unwrap();
    assert_eq!(tree.node(big).unwrap().size, 3_000_000);
    assert!(tree.node(big).unwrap().alloc >= 3_000_000);
    assert!(tree.find_path(&format!("{letter}:\\Windows\\System32")).is_some());
    // The MFT's own size is accounted for.
    let mft = tree.find_path(&format!("{letter}:\\$MFT")).unwrap();
    assert!(tree.node(mft).unwrap().size > 1 << 20);
}

#[test]
fn incremental_refresh_through_the_usn_journal() {
    if !is_elevated() {
        eprintln!("not elevated; skipping live USN test");
        return;
    }
    let cache = tempfile::tempdir().unwrap();
    let base = tempfile::tempdir_in(std::env::temp_dir()).unwrap();
    let dir = base.path().join("omnihub-usn");
    std::fs::create_dir_all(&dir).unwrap();
    let dir_s = long_path(&dir);
    let letter = dir_s.chars().next().unwrap();
    // The scanner reads the disk, not the cache: give the lazy writer time to
    // flush the new folders' MFT records so the baseline already has them
    // (later changes are picked up through the journal).
    std::thread::sleep(Duration::from_secs(8));
    let progress = MftProgress::default();
    let cancel = AtomicBool::new(false);

    let first = volume_scan::scan_ntfs_volume(letter, cache.path(), true, &progress, &cancel, &|_| {}).unwrap();
    assert_eq!(first.meta.method, ScanMethod::Mft);
    assert!(first.meta.journal.is_some(), "the system drive has a change journal");
    println!("full scan: {} ms", first.meta.duration_ms);

    let new_file: PathBuf = dir.join("created-after-scan.bin");
    write_file(&new_file, 123_456);
    let mut found = false;
    for _ in 0..6 {
        std::thread::sleep(Duration::from_secs(2));
        let snap = volume_scan::scan_ntfs_volume(letter, cache.path(), true, &progress, &cancel, &|_| {}).unwrap();
        assert_eq!(snap.meta.method, ScanMethod::MftIncremental, "second scan uses the journal");
        println!("incremental refresh: {} records changed in {} ms", snap.meta.records_changed, snap.meta.duration_ms);
        let tree = snap.to_tree(false);
        if let Some(id) = tree.find_path(&format!("{dir_s}\\created-after-scan.bin")) {
            assert_eq!(tree.node(id).unwrap().size, 123_456);
            found = true;
            break;
        }
    }
    assert!(found, "new file picked up by the incremental refresh");

    std::fs::remove_file(&new_file).unwrap();
    let mut gone = false;
    for _ in 0..6 {
        std::thread::sleep(Duration::from_secs(2));
        let snap = volume_scan::scan_ntfs_volume(letter, cache.path(), true, &progress, &cancel, &|_| {}).unwrap();
        if snap.to_tree(false).find_path(&format!("{dir_s}\\created-after-scan.bin")).is_none() {
            gone = true;
            break;
        }
    }
    assert!(gone, "deleted file removed by the incremental refresh");
}

#[test]
fn dpapi_round_trip() {
    let secret = b"vault key material";
    let sealed = omnihub_core::system::dpapi::protect(secret, b"entropy").unwrap();
    assert_ne!(&sealed[..], secret);
    assert_eq!(&omnihub_core::system::dpapi::unprotect(&sealed, b"entropy").unwrap()[..], secret);
    assert!(omnihub_core::system::dpapi::unprotect(&sealed, b"other").is_err());
}

#[test]
fn installed_apps_and_volumes() {
    let vols = omnihub_core::storage::volumes::list();
    let c = vols.iter().find(|v| v.root.eq_ignore_ascii_case("C:\\")).expect("C: listed");
    assert_eq!(c.file_system, "NTFS");
    assert!(c.mft_capable && c.total > 0 && c.cluster_size >= 512);

    let cache = tempfile::tempdir().unwrap();
    let lib = omnihub_core::apps::AppLibrary::new(cache.path());
    let apps = lib.list(&|_| None);
    println!("{} apps found ({} launchable)", apps.len(), apps.iter().filter(|a| a.launchable).count());
    assert!(!apps.is_empty());
    if let Some(a) = apps.iter().find(|a| a.launchable) {
        let icon = lib.icon_data_url(&a.id);
        println!("icon for {}: {}", a.name, icon.as_ref().map_or("none".into(), |i| format!("{} bytes", i.len())));
    }
}
