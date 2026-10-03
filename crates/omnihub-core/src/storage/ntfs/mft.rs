//! Reading the Master File Table of an NTFS volume.
//!
//! The reader works on anything that implements [`ReadAt`]: a raw volume
//! handle (`\\.\C:`, needs administrator rights on Windows) or a disk image
//! file, which is how the parser is tested on any OS.
//!
//! A full scan streams the `$MFT` in large sequential chunks on one thread
//! while a rayon pool parses the records of the previous chunk, so the scan
//! is bound by disk throughput rather than CPU.

use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rayon::prelude::*;

use super::format::*;
use crate::storage::snapshot::{Slot, SLOT_DIR, SLOT_IN_USE, SLOT_LOSSY_NAME};

/// Size of one sequential read. Large enough that NVMe drives reach full
/// throughput, small enough that the parse of one chunk overlaps the next read.
const CHUNK_BYTES: usize = 8 * 1024 * 1024;

/// Positional reads without a shared cursor, so several threads can read.
pub trait ReadAt: Send + Sync {
    fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<()>;

    /// Told once the boot sector is known, for readers that must align I/O.
    fn set_sector_size(&mut self, _bytes: u32) {}
}

impl ReadAt for std::fs::File {
    #[cfg(unix)]
    fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        std::os::unix::fs::FileExt::read_exact_at(self, buf, offset)
    }

    #[cfg(windows)]
    fn read_exact_at(&self, mut offset: u64, mut buf: &mut [u8]) -> io::Result<()> {
        use std::os::windows::fs::FileExt;
        while !buf.is_empty() {
            match self.seek_read(buf, offset) {
                Ok(0) => {
                    return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "short volume read"))
                }
                Ok(n) => {
                    buf = &mut buf[n..];
                    offset += n as u64;
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

impl<T: AsRef<[u8]> + Send + Sync> ReadAt for io::Cursor<T> {
    fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        let data = self.get_ref().as_ref();
        let start = usize::try_from(offset).map_err(|_| io::ErrorKind::UnexpectedEof)?;
        let src = data
            .get(start..start + buf.len())
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "read past end"))?;
        buf.copy_from_slice(src);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MftError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Format(#[from] FormatError),
    #[error("scan cancelled")]
    Cancelled,
    #[error("the MFT has more records than this scanner supports")]
    TooLarge,
}

/// Progress counters shared with the UI thread.
#[derive(Debug, Default)]
pub struct MftProgress {
    pub records_total: AtomicU64,
    pub records_done: AtomicU64,
    pub bytes_read: AtomicU64,
}

/// What a single FILE record contributes to its (base) file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedRecord {
    pub record: u32,
    /// Record number of the base record; equal to `record` for base records.
    pub base: u32,
    pub sequence: u16,
    pub in_use: bool,
    pub is_dir: bool,
    /// Best `$FILE_NAME` in this record: (preference, parent record, name, lossy).
    pub name: Option<(u8, u32, Box<str>, bool)>,
    pub standard: Option<StandardInfo>,
    /// Size of the unnamed `$DATA` stream when its first extent is here.
    pub data_size: Option<u64>,
    /// Allocated bytes of every stream / index whose first extent is here.
    pub allocated: u64,
    /// Attribute list, when present and resident (used for incremental updates).
    pub attribute_list: Option<Vec<AttrListEntry>>,
    /// True when the record has a non-resident attribute list.
    pub nonresident_attribute_list: bool,
}

/// Parse one FILE record (fixups must already be applied).
pub fn parse_record(raw: &[u8], record: u32) -> Option<ParsedRecord> {
    let rec = FileRecord::parse(raw).ok()?;
    let base = if rec.is_extension() {
        u32::try_from(record_number(rec.base_ref)).ok()?
    } else {
        record
    };
    let mut out = ParsedRecord {
        record,
        base,
        sequence: rec.sequence,
        in_use: rec.in_use(),
        is_dir: rec.is_directory(),
        ..Default::default()
    };
    if !out.in_use {
        return Some(out);
    }
    for attr in rec.attributes() {
        match attr.type_code {
            ATTR_STANDARD_INFORMATION => {
                if let Some(v) = attr.resident_value() {
                    out.standard = StandardInfo::parse(v);
                }
            }
            ATTR_FILE_NAME => {
                let Some(fname) = attr.resident_value().and_then(FileName::parse) else {
                    continue;
                };
                let pref = fname.preference();
                if out.name.as_ref().is_some_and(|(p, ..)| *p >= pref) {
                    continue;
                }
                let Ok(parent) = u32::try_from(record_number(fname.parent_ref)) else {
                    continue;
                };
                let name = String::from_utf16(&fname.name);
                let lossy = name.is_err();
                let name = name.unwrap_or_else(|_| String::from_utf16_lossy(&fname.name));
                out.name = Some((pref, parent, name.into_boxed_str(), lossy));
            }
            ATTR_DATA => {
                if attr.lowest_vcn() != 0 {
                    continue;
                }
                if !attr.is_named() {
                    out.data_size = Some(attr.data_size());
                }
                out.allocated += attr.allocated_size();
            }
            ATTR_INDEX_ALLOCATION => {
                if attr.lowest_vcn() == 0 {
                    out.allocated += attr.allocated_size();
                }
            }
            ATTR_ATTRIBUTE_LIST => {
                if let Some(v) = attr.resident_value() {
                    out.attribute_list = Some(parse_attribute_list(v));
                } else {
                    out.nonresident_attribute_list = true;
                }
            }
            _ => {}
        }
    }
    Some(out)
}

/// Fold a parsed record into the slot of its base file.
pub fn merge_into_slot(slot: &mut Slot, p: &ParsedRecord) {
    if p.record == p.base {
        if p.in_use {
            slot.flags |= SLOT_IN_USE;
        }
        if p.is_dir {
            slot.flags |= SLOT_DIR;
        }
    }
    if !p.in_use {
        return;
    }
    if let Some((pref, parent, name, lossy)) = &p.name {
        if *pref > slot.name_pref {
            slot.name_pref = *pref;
            slot.parent = *parent;
            slot.name = name.clone();
            if *lossy {
                slot.flags |= SLOT_LOSSY_NAME;
            } else {
                slot.flags &= !SLOT_LOSSY_NAME;
            }
        }
    }
    if let Some(si) = p.standard {
        slot.modified = filetime_to_unix(si.modified);
        slot.attributes = si.file_attributes;
    }
    if let Some(size) = p.data_size {
        slot.size = size;
    }
    slot.allocated += p.allocated;
}

/// An opened NTFS volume with the location of its `$MFT` resolved.
pub struct MftVolume<R: ReadAt> {
    reader: R,
    pub boot: BootSector,
    /// Extents of the `$MFT` data stream, sorted by VCN.
    pub runs: Vec<Run>,
    /// Logical size of the `$MFT` data stream in bytes.
    pub mft_size: u64,
}

impl<R: ReadAt> MftVolume<R> {
    pub fn open(reader: R) -> Result<Self, MftError> {
        // 4 KiB covers the boot sector on 512e and 4Kn drives alike.
        let mut boot_buf = vec![0u8; 4096];
        reader.read_exact_at(0, &mut boot_buf)?;
        let boot = BootSector::parse(&boot_buf)?;
        let mut reader = reader;
        reader.set_sector_size(boot.bytes_per_sector);

        let mut vol = MftVolume { reader, boot, runs: Vec::new(), mft_size: 0 };
        let mut rec0 = vec![0u8; boot.record_size as usize];
        vol.reader.read_exact_at(boot.mft_lcn * boot.cluster_size, &mut rec0)?;
        apply_fixups(&mut rec0)?;
        let record = FileRecord::parse(&rec0)?;

        let mut attr_list: Option<Vec<AttrListEntry>> = None;
        let mut nonresident_list: Option<Vec<Run>> = None;
        let mut list_size = 0u64;
        for attr in record.attributes() {
            match attr.type_code {
                ATTR_DATA if !attr.is_named() => {
                    if attr.is_resident() {
                        return Err(FormatError::BadRecord("resident $MFT data").into());
                    }
                    if attr.lowest_vcn() == 0 {
                        vol.mft_size = attr.data_size();
                    }
                    vol.runs.extend(attr.runs()?);
                }
                ATTR_ATTRIBUTE_LIST => {
                    if let Some(v) = attr.resident_value() {
                        attr_list = Some(parse_attribute_list(v));
                    } else {
                        list_size = attr.data_size();
                        nonresident_list = Some(attr.runs()?);
                    }
                }
                _ => {}
            }
        }
        if vol.runs.is_empty() || vol.mft_size == 0 {
            return Err(FormatError::BadRecord("$MFT has no data").into());
        }
        if let Some(runs) = nonresident_list {
            let mut buf = vec![0u8; list_size as usize];
            read_runs(&vol.reader, &runs, boot.cluster_size, 0, &mut buf)?;
            attr_list = Some(parse_attribute_list(&buf));
        }

        // A heavily fragmented $MFT keeps the rest of its run list in
        // extension records, listed in its attribute list.
        if let Some(entries) = attr_list {
            let mut buf = vec![0u8; boot.record_size as usize];
            for entry in entries.iter().filter(|e| e.type_code == ATTR_DATA) {
                let ext = record_number(entry.record_ref);
                if ext == 0 {
                    continue;
                }
                vol.read_record_into(ext, &mut buf)?;
                apply_fixups(&mut buf)?;
                let ext_rec = FileRecord::parse(&buf)?;
                for attr in ext_rec.attributes() {
                    if attr.type_code == ATTR_DATA && !attr.is_named() {
                        let new_runs = attr.runs()?;
                        for r in new_runs {
                            if !vol.runs.iter().any(|x| x.vcn == r.vcn) {
                                vol.runs.push(r);
                            }
                        }
                    }
                }
            }
            vol.runs.sort_by_key(|r| r.vcn);
        }
        Ok(vol)
    }

    pub fn record_count(&self) -> u64 {
        self.mft_size / self.boot.record_size as u64
    }

    /// Read the raw bytes of one MFT record (fixups not applied).
    pub fn read_record_into(&self, record: u64, buf: &mut [u8]) -> Result<(), MftError> {
        let rs = self.boot.record_size as u64;
        if (record + 1) * rs > self.mft_size {
            return Err(FormatError::BadRecord("record beyond end of MFT").into());
        }
        read_runs(&self.reader, &self.runs, self.boot.cluster_size, record * rs, buf)?;
        Ok(())
    }

    /// Read and parse one record including its fixups.
    pub fn parse_one(&self, record: u64) -> Result<Option<ParsedRecord>, MftError> {
        let mut buf = vec![0u8; self.boot.record_size as usize];
        self.read_record_into(record, &mut buf)?;
        if apply_fixups(&mut buf).is_err() {
            return Ok(None);
        }
        Ok(parse_record(&buf, record as u32))
    }

    /// Rebuild the slot of one file from its base record and every extension
    /// record listed in its attribute list. Used by incremental refresh.
    pub fn read_file_slot(&self, record: u64) -> Result<Option<Slot>, MftError> {
        let Some(base) = self.parse_one(record)? else { return Ok(None) };
        if !base.in_use || base.base != base.record {
            return Ok(None);
        }
        let mut slot = Slot::default();
        merge_into_slot(&mut slot, &base);
        let mut extensions: Vec<u64> = base
            .attribute_list
            .as_ref()
            .map(|l| {
                l.iter()
                    .map(|e| record_number(e.record_ref))
                    .filter(|&r| r != record)
                    .collect()
            })
            .unwrap_or_default();
        if base.nonresident_attribute_list {
            extensions.extend(self.nonresident_attribute_list(record)?);
        }
        extensions.sort_unstable();
        extensions.dedup();
        for ext in extensions {
            if let Some(p) = self.parse_one(ext)? {
                if p.base == base.record {
                    merge_into_slot(&mut slot, &p);
                }
            }
        }
        Ok(Some(slot))
    }

    fn nonresident_attribute_list(&self, record: u64) -> Result<Vec<u64>, MftError> {
        let mut buf = vec![0u8; self.boot.record_size as usize];
        self.read_record_into(record, &mut buf)?;
        apply_fixups(&mut buf)?;
        let rec = FileRecord::parse(&buf)?;
        for attr in rec.attributes() {
            if attr.type_code == ATTR_ATTRIBUTE_LIST && !attr.is_resident() {
                let runs = attr.runs()?;
                let mut list = vec![0u8; attr.data_size() as usize];
                read_runs(&self.reader, &runs, self.boot.cluster_size, 0, &mut list)?;
                return Ok(parse_attribute_list(&list)
                    .into_iter()
                    .map(|e| record_number(e.record_ref))
                    .filter(|&r| r != record)
                    .collect());
            }
        }
        Ok(Vec::new())
    }

    /// Read the whole MFT and fold every record into per-file slots indexed
    /// by record number.
    pub fn scan(&self, progress: &MftProgress, cancel: &AtomicBool) -> Result<Vec<Slot>, MftError> {
        let rs = self.boot.record_size as usize;
        let total = self.record_count();
        if total > u32::MAX as u64 {
            return Err(MftError::TooLarge);
        }
        let total = total as usize;
        progress.records_total.store(total as u64, Ordering::Relaxed);

        let per_chunk = (CHUNK_BYTES / rs).max(1);
        let mut slots: Vec<Slot> = Vec::with_capacity(total);
        slots.resize_with(total, Slot::default);

        let (tx, rx) = crossbeam_channel::bounded::<Result<(usize, Vec<u8>), MftError>>(3);
        let (pool_tx, pool_rx) = crossbeam_channel::unbounded::<Vec<u8>>();

        std::thread::scope(|scope| -> Result<(), MftError> {
            scope.spawn(|| {
                let tx = tx;
                let mut first = 0usize;
                while first < total {
                    if cancel.load(Ordering::Relaxed) {
                        let _ = tx.send(Err(MftError::Cancelled));
                        return;
                    }
                    let count = per_chunk.min(total - first);
                    let mut buf = pool_rx.try_recv().unwrap_or_default();
                    buf.resize(count * rs, 0);
                    let res = read_runs(
                        &self.reader,
                        &self.runs,
                        self.boot.cluster_size,
                        (first * rs) as u64,
                        &mut buf,
                    );
                    progress.bytes_read.fetch_add(buf.len() as u64, Ordering::Relaxed);
                    let msg = res.map(|_| (first, buf)).map_err(MftError::from);
                    let failed = msg.is_err();
                    if tx.send(msg).is_err() || failed {
                        return;
                    }
                    first += count;
                }
            });

            for msg in rx.iter() {
                let (first, mut buf) = msg?;
                let parsed: Vec<ParsedRecord> = buf
                    .par_chunks_exact_mut(rs)
                    .enumerate()
                    .filter_map(|(i, raw)| {
                        if &raw[0..4] != b"FILE" || apply_fixups(raw).is_err() {
                            return None;
                        }
                        parse_record(raw, (first + i) as u32)
                    })
                    .collect();
                for p in &parsed {
                    if let Some(slot) = slots.get_mut(p.base as usize) {
                        merge_into_slot(slot, p);
                    }
                }
                progress.records_done.fetch_add((buf.len() / rs) as u64, Ordering::Relaxed);
                buf.clear();
                let _ = pool_tx.send(buf);
            }
            Ok(())
        })?;

        if cancel.load(Ordering::Relaxed) {
            return Err(MftError::Cancelled);
        }
        // Extension records may have contributed to slots whose base record
        // turned out to be free; drop those.
        for slot in &mut slots {
            if slot.flags & SLOT_IN_USE == 0 {
                *slot = Slot::default();
            }
        }
        Ok(slots)
    }
}

/// Read `buf.len()` bytes of a non-resident stream starting at byte `offset`
/// of the stream, following its run list. Sparse holes read as zeros.
pub fn read_runs<R: ReadAt + ?Sized>(
    reader: &R,
    runs: &[Run],
    cluster_size: u64,
    offset: u64,
    buf: &mut [u8],
) -> io::Result<()> {
    let mut done = 0usize;
    while done < buf.len() {
        let pos = offset + done as u64;
        let vcn = pos / cluster_size;
        let run = runs
            .iter()
            .find(|r| vcn >= r.vcn && vcn < r.vcn + r.length)
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "offset not mapped by run list"))?;
        let run_end = (run.vcn + run.length) * cluster_size;
        let n = ((run_end - pos) as usize).min(buf.len() - done);
        let dst = &mut buf[done..done + n];
        match run.lcn {
            Some(lcn) => {
                let vol_off = lcn * cluster_size + (pos - run.vcn * cluster_size);
                reader.read_exact_at(vol_off, dst)?;
            }
            None => dst.fill(0),
        }
        done += n;
    }
    Ok(())
}
