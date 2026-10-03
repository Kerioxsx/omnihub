//! A per-record snapshot of an NTFS volume.
//!
//! The snapshot is what the MFT scanner produces and what the change journal
//! updates. It is also the cache format on disk and the hand-off format
//! between the elevated scan helper and the unelevated app: the helper
//! writes a snapshot file, the app reads it and builds a [`ScanTree`].

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::ntfs::format::ROOT_RECORD;
use super::tree::{flags_from_attributes, ScanInfo, ScanMethod, ScanTree, TreeBuilder, NODE_LOSSY_NAME, NO_PARENT};

pub const SLOT_IN_USE: u8 = 0x01;
pub const SLOT_DIR: u8 = 0x02;
pub const SLOT_LOSSY_NAME: u8 = 0x04;

const MAGIC: &[u8; 8] = b"OHSNAP01";

/// Everything the tree needs to know about one file record.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Slot {
    /// Record number of the parent directory.
    pub parent: u32,
    pub flags: u8,
    /// Namespace preference of `name` (0 = no name yet).
    pub name_pref: u8,
    pub modified: u32,
    pub attributes: u32,
    pub size: u64,
    pub allocated: u64,
    pub name: Box<str>,
}

impl Slot {
    pub fn in_use(&self) -> bool {
        self.flags & SLOT_IN_USE != 0
    }

    pub fn is_dir(&self) -> bool {
        self.flags & SLOT_DIR != 0
    }
}

/// Change-journal position the snapshot is current as of.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct JournalPosition {
    pub journal_id: u64,
    pub next_usn: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotMeta {
    /// Root path shown for the volume, e.g. `C:\`.
    pub root_path: String,
    pub volume_serial: u64,
    pub cluster_size: u64,
    pub record_size: u32,
    pub volume_total: Option<u64>,
    pub volume_free: Option<u64>,
    pub journal: Option<JournalPosition>,
    pub scanned_at: i64,
    pub duration_ms: u64,
    pub method: ScanMethod,
    pub records_changed: u64,
}

pub struct VolumeSnapshot {
    pub meta: SnapshotMeta,
    /// Indexed by MFT record number.
    pub slots: Vec<Slot>,
}

impl VolumeSnapshot {
    pub fn live_records(&self) -> usize {
        self.slots.iter().filter(|s| s.in_use()).count()
    }

    /// Build the browsable tree.
    pub fn to_tree(&self, from_cache: bool) -> ScanTree {
        let n = self.slots.len();
        let mut b = TreeBuilder::with_capacity(self.live_records() + 1);
        let mut index = vec![NO_PARENT; n];

        let root_rec = ROOT_RECORD as usize;
        let root_slot = self.slots.get(root_rec);
        let root = b.add(
            NO_PARENT,
            &self.meta.root_path,
            true,
            0,
            root_slot.map_or(0, |s| s.allocated),
            root_slot.map_or(0, |s| s.modified),
            0,
        );
        if root_rec < n {
            index[root_rec] = root;
        }
        for (rec, slot) in self.slots.iter().enumerate() {
            if rec == root_rec || !slot.in_use() || slot.name_pref == 0 {
                continue;
            }
            let mut flags = flags_from_attributes(slot.attributes);
            if slot.flags & SLOT_LOSSY_NAME != 0 {
                flags |= NODE_LOSSY_NAME;
            }
            // Parents are resolved in a second pass; park the record number.
            let idx = b.add(slot.parent, &slot.name, slot.is_dir(), slot.size, slot.allocated, slot.modified, flags);
            index[rec] = idx;
        }
        for (rec, slot) in self.slots.iter().enumerate() {
            let idx = index[rec];
            if idx == NO_PARENT || idx == root {
                continue;
            }
            let p = slot.parent as usize;
            let parent_idx = if p < n && p != rec && self.slots[p].is_dir() { index[p] } else { NO_PARENT };
            b.set_parent(idx, parent_idx);
        }
        let info = ScanInfo {
            root_path: self.meta.root_path.clone(),
            method: self.meta.method,
            started_at: self.meta.scanned_at,
            duration_ms: self.meta.duration_ms,
            volume_serial: Some(self.meta.volume_serial),
            volume_total: self.meta.volume_total,
            volume_free: self.meta.volume_free,
            cluster_size: Some(self.meta.cluster_size),
            errors: 0,
            from_cache,
            separator: '\\',
        };
        b.finish(root, info)
    }

    /// Write atomically: to a temporary file, then rename over `path`.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        let tmp = path.with_extension("tmp");
        {
            let file = File::create(&tmp)?;
            let mut w = BufWriter::with_capacity(1 << 20, file);
            w.write_all(MAGIC)?;
            let meta = serde_json::to_vec(&self.meta)?;
            w.write_all(&(meta.len() as u32).to_le_bytes())?;
            w.write_all(&meta)?;
            w.write_all(&(self.slots.len() as u32).to_le_bytes())?;
            let mut enc = lz4_flex::frame::FrameEncoder::new(w);
            for (rec, s) in self.slots.iter().enumerate() {
                if !s.in_use() {
                    continue;
                }
                enc.write_all(&(rec as u32).to_le_bytes())?;
                enc.write_all(&s.parent.to_le_bytes())?;
                enc.write_all(&[s.flags, s.name_pref])?;
                enc.write_all(&s.modified.to_le_bytes())?;
                enc.write_all(&s.attributes.to_le_bytes())?;
                enc.write_all(&s.size.to_le_bytes())?;
                enc.write_all(&s.allocated.to_le_bytes())?;
                let name = s.name.as_bytes();
                let len = name.len().min(u16::MAX as usize);
                enc.write_all(&(len as u16).to_le_bytes())?;
                enc.write_all(&name[..len])?;
            }
            // End marker.
            enc.write_all(&u32::MAX.to_le_bytes())?;
            let mut w = enc.finish().map_err(io::Error::other)?;
            w.flush()?;
            w.get_ref().sync_all()?;
        }
        std::fs::rename(&tmp, path)
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let mut r = BufReader::with_capacity(1 << 20, file);
        let mut magic = [0u8; 8];
        r.read_exact(&mut magic)?;
        if &magic != MAGIC {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "not an OmniHub snapshot"));
        }
        let meta_len = read_u32(&mut r)? as usize;
        if meta_len > 1 << 20 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "snapshot header too large"));
        }
        let mut meta = vec![0u8; meta_len];
        r.read_exact(&mut meta)?;
        let meta: SnapshotMeta = serde_json::from_slice(&meta)?;
        let count = read_u32(&mut r)? as usize;
        let mut slots = Vec::with_capacity(count);
        slots.resize_with(count, Slot::default);
        let mut dec = BufReader::with_capacity(1 << 20, lz4_flex::frame::FrameDecoder::new(r));
        let mut fixed = [0u8; 32];
        loop {
            let rec = read_u32(&mut dec)?;
            if rec == u32::MAX {
                break;
            }
            dec.read_exact(&mut fixed)?;
            let name_len = u16::from_le_bytes([fixed[30], fixed[31]]) as usize;
            let mut name = vec![0u8; name_len];
            dec.read_exact(&mut name)?;
            let slot = slots
                .get_mut(rec as usize)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "record out of range"))?;
            *slot = Slot {
                parent: u32::from_le_bytes(fixed[0..4].try_into().unwrap()),
                flags: fixed[4],
                name_pref: fixed[5],
                modified: u32::from_le_bytes(fixed[6..10].try_into().unwrap()),
                attributes: u32::from_le_bytes(fixed[10..14].try_into().unwrap()),
                size: u64::from_le_bytes(fixed[14..22].try_into().unwrap()),
                allocated: u64::from_le_bytes(fixed[22..30].try_into().unwrap()),
                name: String::from_utf8(name)
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "bad name"))?
                    .into_boxed_str(),
            };
        }
        Ok(VolumeSnapshot { meta, slots })
    }
}

fn read_u32(r: &mut impl Read) -> io::Result<u32> {
    let mut b = [0u8; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(parent: u32, name: &str, dir: bool, size: u64) -> Slot {
        Slot {
            parent,
            flags: SLOT_IN_USE | if dir { SLOT_DIR } else { 0 },
            name_pref: 3,
            modified: 1_700_000_000,
            attributes: 0,
            size,
            allocated: size.div_ceil(4096) * 4096,
            name: name.into(),
        }
    }

    fn snapshot() -> VolumeSnapshot {
        let mut slots = vec![Slot::default(); 40];
        slots[0] = slot(5, "$MFT", false, 262_144);
        slots[5] = slot(5, ".", true, 0);
        slots[30] = slot(5, "Users", true, 0);
        slots[31] = slot(30, "me", true, 0);
        slots[32] = slot(31, "big.bin", false, 10_000_000);
        slots[33] = slot(31, "small.txt", false, 12);
        slots[34] = slot(36, "lost.dat", false, 99); // parent is free
        slots[35] = slot(33, "under-file.dat", false, 1); // parent is a file
        VolumeSnapshot {
            meta: SnapshotMeta {
                root_path: "C:\\".into(),
                volume_serial: 0xABCD,
                cluster_size: 4096,
                record_size: 1024,
                volume_total: Some(1 << 30),
                volume_free: Some(1 << 29),
                journal: Some(JournalPosition { journal_id: 7, next_usn: 4242 }),
                scanned_at: 1_700_000_000,
                duration_ms: 12,
                method: ScanMethod::Mft,
                records_changed: 0,
            },
            slots,
        }
    }

    #[test]
    fn tree_from_snapshot() {
        let t = snapshot().to_tree(false);
        assert_eq!(t.name(0), "C:\\");
        let big = t.find_path("C:\\Users\\me\\big.bin").unwrap();
        assert_eq!(t.node(big).unwrap().size, 10_000_000);
        assert_eq!(t.path(big), "C:\\Users\\me\\big.bin");
        assert!(t.find_path("C:\\$MFT").is_some());
        let orphan = t.find_path("C:\\$Orphaned").unwrap();
        assert_eq!(t.child_ids(orphan).len(), 2);
        assert_eq!(t.node(0).unwrap().size, 262_144 + 10_000_000 + 12 + 99 + 1);
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vol.ohsnap");
        let snap = snapshot();
        snap.save(&path).unwrap();
        let back = VolumeSnapshot::load(&path).unwrap();
        assert_eq!(back.meta, snap.meta);
        assert_eq!(back.slots, snap.slots);
    }
}
