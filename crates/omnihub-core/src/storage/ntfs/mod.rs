//! NTFS Master File Table scanning and USN change journal support.

pub mod format;
pub mod mft;
#[cfg(windows)]
pub mod usn;
#[cfg(windows)]
pub mod volume;
