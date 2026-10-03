//! Raw access to a Windows volume (`\\.\C:`) for the MFT scanner.
//!
//! Opening a volume for reading requires administrator rights. Reads on a
//! volume handle must start and end on sector boundaries, which
//! [`VolumeFile`] guarantees by bouncing unaligned requests through an
//! aligned buffer.

use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::{FileExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;

use windows::Win32::Foundation::HANDLE;

use super::mft::ReadAt;

const FILE_SHARE_READ: u32 = 0x1;
const FILE_SHARE_WRITE: u32 = 0x2;
const FILE_SHARE_DELETE: u32 = 0x4;

pub struct VolumeFile {
    file: File,
    sector: u64,
}

impl VolumeFile {
    /// Open `\\.\X:` for raw reading. `letter` is a drive letter A–Z.
    pub fn open(letter: char) -> io::Result<Self> {
        if !letter.is_ascii_alphabetic() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "drive letter expected"));
        }
        let path = format!(r"\\.\{}:", letter.to_ascii_uppercase());
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .open(path)?;
        Ok(VolumeFile { file, sector: 4096 })
    }

    pub fn handle(&self) -> HANDLE {
        HANDLE(self.file.as_raw_handle())
    }

    pub fn file(&self) -> &File {
        &self.file
    }

    fn read_aligned(&self, mut offset: u64, mut buf: &mut [u8]) -> io::Result<()> {
        while !buf.is_empty() {
            match self.file.seek_read(buf, offset) {
                Ok(0) => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "short volume read")),
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

impl ReadAt for VolumeFile {
    fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        let s = self.sector;
        let len = buf.len() as u64;
        if offset.is_multiple_of(s) && len.is_multiple_of(s) {
            return self.read_aligned(offset, buf);
        }
        let start = offset / s * s;
        let end = (offset + len).div_ceil(s) * s;
        let mut bounce = vec![0u8; (end - start) as usize];
        self.read_aligned(start, &mut bounce)?;
        let skip = (offset - start) as usize;
        buf.copy_from_slice(&bounce[skip..skip + buf.len()]);
        Ok(())
    }

    fn set_sector_size(&mut self, bytes: u32) {
        if bytes.is_power_of_two() && (512..=4096).contains(&bytes) {
            self.sector = bytes as u64;
        }
    }
}
