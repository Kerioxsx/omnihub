//! Benchmark: scan an NTFS image (or, on Windows as administrator, a live
//! volume such as `C:`) and print timings.
//!
//!     cargo run --release --example mftbench -- target/bench.img
//!     cargo run --release --example mftbench -- C:
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use omnihub_core::storage::ntfs::mft::{MftProgress, MftVolume};
use omnihub_core::storage::snapshot::{SnapshotMeta, VolumeSnapshot};
use omnihub_core::storage::tree::ScanMethod;

fn main() {
    let target = std::env::args().nth(1).expect("usage: mftbench <image|C:>");
    let started = Instant::now();
    #[cfg(windows)]
    let slots_and_boot = if let Some(letter) = omnihub_core::storage::volumes::drive_letter(&target) {
        let vol = MftVolume::open(omnihub_core::storage::ntfs::volume::VolumeFile::open(letter).expect("open volume (run as administrator)")).expect("ntfs");
        let slots = vol.scan(&MftProgress::default(), &AtomicBool::new(false)).expect("scan");
        (slots, vol.boot, vol.runs.len())
    } else {
        image(&target)
    };
    #[cfg(not(windows))]
    let slots_and_boot = image(&target);
    let (slots, boot, runs) = slots_and_boot;
    let scanned = started.elapsed();
    let snap = VolumeSnapshot {
        meta: SnapshotMeta {
            root_path: "X:\\".into(),
            volume_serial: boot.serial,
            cluster_size: boot.cluster_size,
            record_size: boot.record_size,
            volume_total: None,
            volume_free: None,
            journal: None,
            scanned_at: 0,
            duration_ms: 0,
            method: ScanMethod::Mft,
            records_changed: 0,
        },
        slots,
    };
    let t = Instant::now();
    let tree = snap.to_tree(false);
    let built = t.elapsed();
    let root = tree.node(0).unwrap();
    println!("records:   {} ({} runs, {} live)", snap.slots.len(), runs, snap.live_records());
    println!("files:     {}  folders: {}", root.files, root.desc - root.files);
    println!("size:      {} bytes", root.size);
    println!("MFT read+parse: {:?}", scanned);
    println!("tree build:     {:?}", built);
    println!("tree memory:    {:.1} MiB", tree.memory_bytes() as f64 / 1048576.0);
    let t = Instant::now();
    let (hits, total) = tree.search(&omnihub_core::storage::tree::SearchQuery { text: "*0042*".into(), ..Default::default() });
    println!("search *0042*:  {} hits ({} shown) in {:?}", total, hits.len(), t.elapsed());
}

fn image(path: &str) -> (Vec<omnihub_core::storage::snapshot::Slot>, omnihub_core::storage::ntfs::format::BootSector, usize) {
    let vol = MftVolume::open(std::fs::File::open(path).expect("open image")).expect("ntfs");
    let slots = vol.scan(&MftProgress::default(), &AtomicBool::new(false)).expect("scan");
    (slots, vol.boot, vol.runs.len())
}
