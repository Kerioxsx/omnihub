//! Storage analysis: fast inventory of whole volumes, browsing, search,
//! cleanup suggestions and duplicate detection.

pub mod cleanup;
pub mod dupes;
pub mod engine;
pub mod ntfs;
pub mod snapshot;
pub mod tree;
pub mod volume_scan;
pub mod volumes;
pub mod walk;
