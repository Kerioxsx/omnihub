//! On-disk NTFS structures: boot sector, FILE records, attributes and data runs.
//!
//! Everything here is pure parsing over byte slices so it can be unit tested
//! and fuzzed without a real volume. Offsets follow the layout documented by
//! the Linux-NTFS project ("NTFS Documentation", R. Russon & Y. Fledel).

use std::fmt;

pub const ATTR_STANDARD_INFORMATION: u32 = 0x10;
pub const ATTR_ATTRIBUTE_LIST: u32 = 0x20;
pub const ATTR_FILE_NAME: u32 = 0x30;
pub const ATTR_DATA: u32 = 0x80;
pub const ATTR_INDEX_ALLOCATION: u32 = 0xA0;
pub const ATTR_END: u32 = 0xFFFF_FFFF;

pub const RECORD_IN_USE: u16 = 0x0001;
pub const RECORD_IS_DIRECTORY: u16 = 0x0002;

pub const ATTR_FLAG_COMPRESSED: u16 = 0x0001;
pub const ATTR_FLAG_SPARSE: u16 = 0x8000;

pub const NAMESPACE_POSIX: u8 = 0;
pub const NAMESPACE_WIN32: u8 = 1;
pub const NAMESPACE_DOS: u8 = 2;
pub const NAMESPACE_WIN32_AND_DOS: u8 = 3;

/// Record number of the root directory (`.`).
pub const ROOT_RECORD: u64 = 5;

/// Multi-sector transfer protection always works on 512-byte strides,
/// whatever the physical sector size.
const FIXUP_STRIDE: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    NotNtfs,
    BadBootSector(&'static str),
    BadRecord(&'static str),
    BadRuns,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::NotNtfs => write!(f, "volume is not NTFS"),
            FormatError::BadBootSector(why) => write!(f, "invalid NTFS boot sector: {why}"),
            FormatError::BadRecord(why) => write!(f, "invalid MFT record: {why}"),
            FormatError::BadRuns => write!(f, "invalid data run list"),
        }
    }
}

impl std::error::Error for FormatError {}

#[inline]
pub(crate) fn u16_at(b: &[u8], off: usize) -> Option<u16> {
    b.get(off..off + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}

#[inline]
pub(crate) fn u32_at(b: &[u8], off: usize) -> Option<u32> {
    b.get(off..off + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

#[inline]
pub(crate) fn u64_at(b: &[u8], off: usize) -> Option<u64> {
    b.get(off..off + 8).map(|s| {
        u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]])
    })
}

/// Lower 48 bits of a file reference are the record number.
#[inline]
pub fn record_number(file_ref: u64) -> u64 {
    file_ref & 0x0000_FFFF_FFFF_FFFF
}

/// Upper 16 bits of a file reference are the sequence number.
#[inline]
pub fn sequence_number(file_ref: u64) -> u16 {
    (file_ref >> 48) as u16
}

/// The parameters from the NTFS boot sector that the scanner needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootSector {
    pub bytes_per_sector: u32,
    pub cluster_size: u64,
    pub total_sectors: u64,
    pub mft_lcn: u64,
    pub record_size: u32,
    pub serial: u64,
}

impl BootSector {
    pub fn parse(b: &[u8]) -> Result<Self, FormatError> {
        if b.len() < 512 {
            return Err(FormatError::BadBootSector("short read"));
        }
        if &b[3..11] != b"NTFS    " {
            return Err(FormatError::NotNtfs);
        }
        let bytes_per_sector = u16_at(b, 0x0B).unwrap() as u32;
        if !(256..=4096).contains(&bytes_per_sector) || !bytes_per_sector.is_power_of_two() {
            return Err(FormatError::BadBootSector("bytes per sector"));
        }
        // Clusters above 64 KiB (allowed since Windows 10 1709) store the
        // sectors-per-cluster value as a negative power of two.
        let spc_raw = b[0x0D];
        let sectors_per_cluster: u64 = if spc_raw <= 0x80 {
            spc_raw as u64
        } else {
            1u64 << (256 - spc_raw as u32)
        };
        if sectors_per_cluster == 0 {
            return Err(FormatError::BadBootSector("sectors per cluster"));
        }
        let cluster_size = sectors_per_cluster * bytes_per_sector as u64;
        let total_sectors = u64_at(b, 0x28).unwrap();
        let mft_lcn = u64_at(b, 0x30).unwrap();
        let cpr = b[0x40] as i8;
        let record_size: u64 = if cpr > 0 {
            cpr as u64 * cluster_size
        } else {
            1u64 << (-(cpr as i32)) as u32
        };
        if !(256..=65536).contains(&record_size) || !record_size.is_power_of_two() {
            return Err(FormatError::BadBootSector("record size"));
        }
        if mft_lcn == 0 || mft_lcn.saturating_mul(cluster_size) / bytes_per_sector as u64 > total_sectors {
            return Err(FormatError::BadBootSector("MFT location"));
        }
        Ok(BootSector {
            bytes_per_sector,
            cluster_size,
            total_sectors,
            mft_lcn,
            record_size: record_size as u32,
            serial: u64_at(b, 0x48).unwrap(),
        })
    }

    pub fn volume_size(&self) -> u64 {
        self.total_sectors * self.bytes_per_sector as u64
    }
}

/// Apply the update sequence array ("fixups") in place.
///
/// The last two bytes of every 512-byte block of a record are replaced on
/// disk by the update sequence number; the real bytes live in the array.
/// A mismatch means a torn write and the record must be ignored.
pub fn apply_fixups(record: &mut [u8]) -> Result<(), FormatError> {
    let usa_off = u16_at(record, 0x04).ok_or(FormatError::BadRecord("header"))? as usize;
    let usa_count = u16_at(record, 0x06).ok_or(FormatError::BadRecord("header"))? as usize;
    if usa_count == 0 {
        return Err(FormatError::BadRecord("empty update sequence"));
    }
    let blocks = usa_count - 1;
    if blocks * FIXUP_STRIDE > record.len() || usa_off + usa_count * 2 > record.len() {
        return Err(FormatError::BadRecord("update sequence out of range"));
    }
    let usn = [record[usa_off], record[usa_off + 1]];
    for i in 0..blocks {
        let end = (i + 1) * FIXUP_STRIDE;
        if record[end - 2..end] != usn {
            return Err(FormatError::BadRecord("torn write"));
        }
        let fix = usa_off + 2 + i * 2;
        record[end - 2] = record[fix];
        record[end - 1] = record[fix + 1];
    }
    Ok(())
}

/// One contiguous run of clusters of a non-resident attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    /// First virtual cluster number covered by this run.
    pub vcn: u64,
    /// Logical cluster on the volume, or `None` for a sparse hole.
    pub lcn: Option<u64>,
    pub length: u64,
}

/// Decode a data run (mapping pairs) list starting at `start_vcn`.
pub fn decode_runs(b: &[u8], start_vcn: u64) -> Result<Vec<Run>, FormatError> {
    let mut runs = Vec::new();
    let mut pos = 0usize;
    let mut vcn = start_vcn;
    let mut lcn: i64 = 0;
    while pos < b.len() {
        let header = b[pos];
        if header == 0 {
            return Ok(runs);
        }
        let len_size = (header & 0x0F) as usize;
        let off_size = (header >> 4) as usize;
        pos += 1;
        if len_size == 0 || len_size > 8 || off_size > 8 || pos + len_size + off_size > b.len() {
            return Err(FormatError::BadRuns);
        }
        let mut length: u64 = 0;
        for i in 0..len_size {
            length |= (b[pos + i] as u64) << (8 * i);
        }
        pos += len_size;
        let run_lcn = if off_size == 0 {
            None
        } else {
            let mut delta: i64 = 0;
            for i in 0..off_size {
                delta |= (b[pos + i] as i64) << (8 * i);
            }
            // Sign-extend the delta.
            let shift = 64 - 8 * off_size as u32;
            delta = (delta << shift) >> shift;
            lcn = lcn.checked_add(delta).ok_or(FormatError::BadRuns)?;
            if lcn < 0 {
                return Err(FormatError::BadRuns);
            }
            Some(lcn as u64)
        };
        pos += off_size;
        if length == 0 {
            return Err(FormatError::BadRuns);
        }
        runs.push(Run { vcn, lcn: run_lcn, length });
        vcn = vcn.checked_add(length).ok_or(FormatError::BadRuns)?;
    }
    Err(FormatError::BadRuns)
}

/// A borrowed view of one attribute inside a FILE record.
#[derive(Debug, Clone, Copy)]
pub struct Attribute<'a> {
    pub type_code: u32,
    pub flags: u16,
    pub name_len: u8,
    raw: &'a [u8],
}

impl<'a> Attribute<'a> {
    pub fn is_resident(&self) -> bool {
        self.raw[8] == 0
    }

    pub fn is_named(&self) -> bool {
        self.name_len > 0
    }

    /// UTF-16 name of the attribute (e.g. `$I30` or an alternate stream name).
    pub fn name(&self) -> Option<Vec<u16>> {
        if self.name_len == 0 {
            return None;
        }
        let off = u16_at(self.raw, 0x0A)? as usize;
        let bytes = self.raw.get(off..off + self.name_len as usize * 2)?;
        Some(bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect())
    }

    /// Value bytes of a resident attribute.
    pub fn resident_value(&self) -> Option<&'a [u8]> {
        if !self.is_resident() {
            return None;
        }
        let len = u32_at(self.raw, 0x10)? as usize;
        let off = u16_at(self.raw, 0x14)? as usize;
        self.raw.get(off..off.checked_add(len)?)
    }

    pub fn lowest_vcn(&self) -> u64 {
        if self.is_resident() {
            0
        } else {
            u64_at(self.raw, 0x10).unwrap_or(0)
        }
    }

    /// Logical size of the attribute value. Only meaningful for resident
    /// attributes and for the first extent (lowest VCN 0) of non-resident ones.
    pub fn data_size(&self) -> u64 {
        if self.is_resident() {
            u32_at(self.raw, 0x10).unwrap_or(0) as u64
        } else {
            u64_at(self.raw, 0x30).unwrap_or(0)
        }
    }

    /// Bytes actually allocated on disk for this attribute (0 for resident).
    /// For compressed or sparse streams this is the compressed size.
    pub fn allocated_size(&self) -> u64 {
        if self.is_resident() {
            return 0;
        }
        let header_len = u16_at(self.raw, 0x20).unwrap_or(0) as usize;
        if self.flags & (ATTR_FLAG_COMPRESSED | ATTR_FLAG_SPARSE) != 0 && header_len >= 0x48 {
            if let Some(c) = u64_at(self.raw, 0x40) {
                return c;
            }
        }
        u64_at(self.raw, 0x28).unwrap_or(0)
    }

    pub fn runs(&self) -> Result<Vec<Run>, FormatError> {
        if self.is_resident() {
            return Ok(Vec::new());
        }
        let off = u16_at(self.raw, 0x20).ok_or(FormatError::BadRuns)? as usize;
        let bytes = self.raw.get(off..).ok_or(FormatError::BadRuns)?;
        decode_runs(bytes, self.lowest_vcn())
    }
}

/// A parsed FILE record header with an attribute iterator.
#[derive(Debug, Clone, Copy)]
pub struct FileRecord<'a> {
    pub flags: u16,
    pub sequence: u16,
    pub base_ref: u64,
    attrs_off: usize,
    used: usize,
    raw: &'a [u8],
}

impl<'a> FileRecord<'a> {
    /// Parse a record whose fixups have already been applied.
    pub fn parse(raw: &'a [u8]) -> Result<Self, FormatError> {
        if raw.len() < 0x30 || &raw[0..4] != b"FILE" {
            return Err(FormatError::BadRecord("signature"));
        }
        let attrs_off = u16_at(raw, 0x14).unwrap() as usize;
        let used = (u32_at(raw, 0x18).unwrap() as usize).min(raw.len());
        if attrs_off < 0x2A || attrs_off >= used {
            return Err(FormatError::BadRecord("attribute offset"));
        }
        Ok(FileRecord {
            flags: u16_at(raw, 0x16).unwrap(),
            sequence: u16_at(raw, 0x10).unwrap(),
            base_ref: u64_at(raw, 0x20).unwrap(),
            attrs_off,
            used,
            raw,
        })
    }

    pub fn in_use(&self) -> bool {
        self.flags & RECORD_IN_USE != 0
    }

    pub fn is_directory(&self) -> bool {
        self.flags & RECORD_IS_DIRECTORY != 0
    }

    /// True for extension records that hold overflow attributes of a base
    /// record. Compare the whole reference: the extension records of `$MFT`
    /// itself point at record 0 (with a non-zero sequence number).
    pub fn is_extension(&self) -> bool {
        self.base_ref != 0
    }

    pub fn attributes(&self) -> AttributeIter<'a> {
        AttributeIter { raw: &self.raw[..self.used], pos: self.attrs_off }
    }
}

pub struct AttributeIter<'a> {
    raw: &'a [u8],
    pos: usize,
}

impl<'a> Iterator for AttributeIter<'a> {
    type Item = Attribute<'a>;

    fn next(&mut self) -> Option<Attribute<'a>> {
        let type_code = u32_at(self.raw, self.pos)?;
        if type_code == ATTR_END {
            return None;
        }
        let len = u32_at(self.raw, self.pos + 4)? as usize;
        if len < 0x18 || !len.is_multiple_of(8) {
            return None;
        }
        let raw = self.raw.get(self.pos..self.pos + len)?;
        self.pos += len;
        Some(Attribute {
            type_code,
            name_len: raw[9],
            flags: u16_at(raw, 0x0C)?,
            raw,
        })
    }
}

/// The parts of `$STANDARD_INFORMATION` that the scanner keeps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StandardInfo {
    pub created: u64,
    pub modified: u64,
    pub file_attributes: u32,
}

impl StandardInfo {
    pub fn parse(v: &[u8]) -> Option<Self> {
        Some(StandardInfo {
            created: u64_at(v, 0x00)?,
            modified: u64_at(v, 0x08)?,
            file_attributes: u32_at(v, 0x20)?,
        })
    }
}

/// A decoded `$FILE_NAME` attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileName {
    pub parent_ref: u64,
    pub namespace: u8,
    pub name: Vec<u16>,
}

impl FileName {
    pub fn parse(v: &[u8]) -> Option<Self> {
        let parent_ref = u64_at(v, 0x00)?;
        let len = *v.get(0x40)? as usize;
        let namespace = *v.get(0x41)?;
        let bytes = v.get(0x42..0x42 + len * 2)?;
        let name = bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        Some(FileName { parent_ref, namespace, name })
    }

    /// Higher is better: Win32 names beat POSIX names, which beat 8.3 DOS aliases.
    pub fn preference(&self) -> u8 {
        match self.namespace {
            NAMESPACE_WIN32 | NAMESPACE_WIN32_AND_DOS => 3,
            NAMESPACE_POSIX => 2,
            NAMESPACE_DOS => 1,
            _ => 0,
        }
    }
}

/// One entry of an `$ATTRIBUTE_LIST`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttrListEntry {
    pub type_code: u32,
    pub start_vcn: u64,
    pub record_ref: u64,
}

pub fn parse_attribute_list(v: &[u8]) -> Vec<AttrListEntry> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos + 0x1A <= v.len() {
        let type_code = u32_at(v, pos).unwrap();
        let len = u16_at(v, pos + 4).unwrap() as usize;
        if len < 0x1A || type_code == ATTR_END {
            break;
        }
        out.push(AttrListEntry {
            type_code,
            start_vcn: u64_at(v, pos + 8).unwrap(),
            record_ref: u64_at(v, pos + 0x10).unwrap(),
        });
        pos += len;
    }
    out
}

/// Windows FILETIME (100 ns ticks since 1601) to Unix seconds, clamped to u32.
pub fn filetime_to_unix(ft: u64) -> u32 {
    const EPOCH_DIFF: u64 = 11_644_473_600;
    let secs = ft / 10_000_000;
    secs.saturating_sub(EPOCH_DIFF).min(u32::MAX as u64) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_basic_and_sparse() {
        // 0x21: 1-byte length, 2-byte offset. length=0x18, lcn=0x5634
        // 0x01: 1-byte length, no offset => sparse run of 0x10 clusters
        // 0x31: 1-byte length, 3-byte negative delta
        let b = [0x21, 0x18, 0x34, 0x56, 0x01, 0x10, 0x31, 0x08, 0x00, 0xFF, 0xFF, 0x00];
        let runs = decode_runs(&b, 0).unwrap();
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[0], Run { vcn: 0, lcn: Some(0x5634), length: 0x18 });
        assert_eq!(runs[1], Run { vcn: 0x18, lcn: None, length: 0x10 });
        assert_eq!(runs[2], Run { vcn: 0x28, lcn: Some(0x5634 - 0x100), length: 8 });
    }

    #[test]
    fn runs_reject_garbage() {
        assert!(decode_runs(&[0x21, 0x18], 0).is_err());
        assert!(decode_runs(&[0x9F, 0, 0], 0).is_err());
        // Negative absolute LCN.
        assert!(decode_runs(&[0x11, 0x01, 0x80, 0x00], 0).is_err());
    }

    #[test]
    fn fixups_round_trip() {
        let mut rec = vec![0u8; 1024];
        rec[0..4].copy_from_slice(b"FILE");
        rec[4..6].copy_from_slice(&0x30u16.to_le_bytes());
        rec[6..8].copy_from_slice(&3u16.to_le_bytes());
        rec[0x30..0x32].copy_from_slice(&[0xAB, 0xCD]);
        rec[0x32..0x34].copy_from_slice(&[1, 2]);
        rec[0x34..0x36].copy_from_slice(&[3, 4]);
        rec[510..512].copy_from_slice(&[0xAB, 0xCD]);
        rec[1022..1024].copy_from_slice(&[0xAB, 0xCD]);
        apply_fixups(&mut rec).unwrap();
        assert_eq!(&rec[510..512], &[1, 2]);
        assert_eq!(&rec[1022..1024], &[3, 4]);

        rec[0x30..0x32].copy_from_slice(&[0xEE, 0xEE]);
        assert!(apply_fixups(&mut rec).is_err());
    }

    #[test]
    fn filetime_conversion() {
        // 2020-01-01T00:00:00Z
        assert_eq!(filetime_to_unix(132_223_104_000_000_000), 1_577_836_800);
        assert_eq!(filetime_to_unix(0), 0);
    }

    #[test]
    fn large_cluster_boot_sector() {
        let mut b = vec![0u8; 512];
        b[3..11].copy_from_slice(b"NTFS    ");
        b[0x0B..0x0D].copy_from_slice(&512u16.to_le_bytes());
        b[0x0D] = 0xF4; // 2^12 sectors = 2 MiB clusters
        b[0x28..0x30].copy_from_slice(&(1u64 << 32).to_le_bytes());
        b[0x30..0x38].copy_from_slice(&4u64.to_le_bytes());
        b[0x40] = 0xF6; // -10 => 1024-byte records
        let bs = BootSector::parse(&b).unwrap();
        assert_eq!(bs.cluster_size, 2 * 1024 * 1024);
        assert_eq!(bs.record_size, 1024);
    }
}
