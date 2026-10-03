//! Debug helper: dump parsed MFT records of an image or volume.
//! cargo run --example mftdump -- <image> [record...]
use std::fs::File;
use omnihub_core::storage::ntfs::mft::MftVolume;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let vol = MftVolume::open(File::open(&args[1]).expect("open")).expect("ntfs");
    println!("boot: {:?}\nmft size {} ({} records), {} runs", vol.boot, vol.mft_size, vol.record_count(), vol.runs.len());
    for r in &args[2..] {
        let rec: u64 = r.parse().unwrap();
        println!("#{rec}: {:#?}", vol.parse_one(rec).unwrap());
    }
}
