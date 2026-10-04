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

/// "12.3 GB" — binary units with the familiar names, like Explorer.
pub fn format_bytes(n: u64) -> String {
    const UNITS: [&str; 6] = ["bytes", "KB", "MB", "GB", "TB", "PB"];
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{n} bytes")
    } else {
        format!("{:.*} {}", if v < 10.0 { 1 } else { 0 }, v, UNITS[u])
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn bytes() {
        assert_eq!(super::format_bytes(512), "512 bytes");
        assert_eq!(super::format_bytes(1536), "1.5 KB");
        assert_eq!(super::format_bytes(25 * 1024 * 1024 * 1024), "25 GB");
    }
}
