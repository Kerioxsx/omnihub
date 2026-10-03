//! Compares the MFT scanner against NTFS images whose contents are known.
//!
//! The images are built by `scripts/make-ntfs-fixtures.sh` (needs root and
//! ntfs-3g); point `OMNIHUB_NTFS_FIXTURES` at the output directory. Without
//! it the test is skipped so `cargo test` works everywhere.

use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use omnihub_core::storage::ntfs::mft::{MftProgress, MftVolume};
use omnihub_core::storage::snapshot::{SnapshotMeta, VolumeSnapshot};
use omnihub_core::storage::tree::ScanMethod;

#[derive(serde::Deserialize)]
struct Manifest {
    entries: Vec<Entry>,
}

#[derive(serde::Deserialize)]
struct Entry {
    path: String,
    dir: bool,
    size: u64,
    blocks: u64,
    nlink: u64,
    ino: u64,
}

fn fixtures() -> Option<PathBuf> {
    let dir = std::env::var_os("OMNIHUB_NTFS_FIXTURES")?;
    let dir = PathBuf::from(dir);
    if !dir.is_dir() {
        panic!("OMNIHUB_NTFS_FIXTURES={} is not a directory", dir.display());
    }
    Some(dir)
}

fn scan_image(path: &std::path::Path) -> (VolumeSnapshot, usize) {
    let file = File::open(path).unwrap();
    let vol = MftVolume::open(file).unwrap();
    let runs = vol.runs.len();
    let progress = MftProgress::default();
    let slots = vol.scan(&progress, &AtomicBool::new(false)).unwrap();
    let snap = VolumeSnapshot {
        meta: SnapshotMeta {
            root_path: "X:\\".into(),
            volume_serial: vol.boot.serial,
            cluster_size: vol.boot.cluster_size,
            record_size: vol.boot.record_size,
            volume_total: Some(vol.boot.volume_size()),
            volume_free: None,
            journal: None,
            scanned_at: 0,
            duration_ms: 0,
            method: ScanMethod::Mft,
            records_changed: 0,
        },
        slots,
    };
    (snap, runs)
}

#[test]
fn mft_scan_matches_mounted_view() {
    let Some(dir) = fixtures() else {
        eprintln!("OMNIHUB_NTFS_FIXTURES not set; skipping NTFS image tests");
        return;
    };
    let mut images: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "img"))
        .collect();
    images.sort();
    assert!(!images.is_empty(), "no .img files in {}", dir.display());

    for img in images {
        let manifest: Manifest =
            serde_json::from_reader(File::open(img.with_extension("json")).unwrap()).unwrap();
        let (snap, runs) = scan_image(&img);
        let tree = snap.to_tree(false);
        let name = img.file_name().unwrap().to_string_lossy().to_string();
        eprintln!("{name}: {} nodes, $MFT in {runs} run(s), {} records", tree.len(), snap.slots.len());

        // Every entry the file system shows must be in the tree with the
        // same size; hard links are counted once, under one of their names.
        let mut by_inode: HashMap<u64, Vec<&Entry>> = HashMap::new();
        let mut expected_total = 0u64;
        for e in &manifest.entries {
            if e.nlink > 1 && !e.dir {
                by_inode.entry(e.ino).or_default().push(e);
                continue;
            }
            let path = format!("X:\\{}", e.path);
            let id = tree.find_path(&path).unwrap_or_else(|| panic!("{name}: missing {path}"));
            let node = tree.node(id).unwrap();
            assert_eq!(node.is_dir(), e.dir, "{name}: {path} kind");
            if !e.dir {
                expected_total += e.size;
                assert_eq!(node.size, e.size, "{name}: {path} size");
                // ntfs-3g reports the allocation of the main stream; the
                // scanner adds alternate streams, so it can only be larger.
                // Resident files have no clusters of their own.
                if e.size > 1024 {
                    assert!(node.alloc >= e.blocks, "{name}: {path} alloc {} < {}", node.alloc, e.blocks);
                }
            }
            // MFT record numbers are the ntfs-3g inode numbers.
            assert!(snap.slots[e.ino as usize].in_use(), "{name}: record {} not in use", e.ino);
        }
        for (ino, names) in &by_inode {
            let found: Vec<_> = names
                .iter()
                .filter(|e| tree.find_path(&format!("X:\\{}", e.path)).is_some())
                .collect();
            assert_eq!(found.len(), 1, "{name}: hard link {ino} should appear exactly once");
            expected_total += found[0].size;
        }

        // The root also holds the NTFS metadata files ($MFT, $LogFile, ...).
        let user_total: u64 = tree
            .child_ids(0)
            .into_iter()
            .filter(|&c| !tree.name(c).starts_with('$'))
            .map(|c| tree.node(c).unwrap().size)
            .sum();
        assert_eq!(user_total, expected_total, "{name}: total of user files");
        let mft = tree.find_path("X:\\$MFT").expect("$MFT listed");
        assert_eq!(tree.node(mft).unwrap().size, snap.slots.len() as u64 * snap.meta.record_size as u64);

        if name == "basic.img" {
            let sparse = tree.node(tree.find_path("X:\\Users\\alice\\Downloads\\sparse.vhd").unwrap()).unwrap();
            assert_eq!(sparse.size, 50 * 1024 * 1024);
            assert!(sparse.alloc < 1024 * 1024, "sparse file allocation {}", sparse.alloc);
            let stream = tree.node(tree.find_path("X:\\Users\\alice\\Documents\\with-stream.txt").unwrap()).unwrap();
            assert_eq!(stream.size, 5);
            assert!(stream.alloc >= 200_000, "alternate stream counted in allocation");
            let uni = tree.find_path("X:\\Users\\alice\\Documents\\Ünïcødé – 名前 😀.txt");
            assert!(uni.is_some_and(|id| tree.path_is_exact(id)));
        }

        // The snapshot cache round-trips to the same tree.
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path().join("snap.ohsnap");
        snap.save(&cache).unwrap();
        let back = VolumeSnapshot::load(&cache).unwrap().to_tree(true);
        assert_eq!(back.len(), tree.len());
        assert_eq!(back.node(0).unwrap().size, tree.node(0).unwrap().size);
    }
}

#[test]
fn single_record_reads_match_full_scan() {
    let Some(dir) = fixtures() else { return };
    let img = dir.join("basic.img");
    if !img.exists() {
        return;
    }
    let (snap, _) = scan_image(&img);
    let vol = MftVolume::open(File::open(&img).unwrap()).unwrap();
    for (rec, slot) in snap.slots.iter().enumerate() {
        let single = vol.read_file_slot(rec as u64).unwrap();
        if slot.in_use() {
            assert_eq!(single.as_ref(), Some(slot), "record {rec}");
        }
    }
}
