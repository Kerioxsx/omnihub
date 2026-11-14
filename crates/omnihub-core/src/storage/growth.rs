//! "What grew": folder sizes compared with the previous scan of a drive.
//!
//! After every fresh scan a compact map of folder sizes (folders of 16 MB
//! or more, four levels deep) is saved next to the scan cache; the map of
//! the scan before is kept as the baseline. The report lists the folders
//! that changed most, preferring the specific folder over its parents
//! (Downloads, not "C:\Users", when Downloads accounts for the growth).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::tree::ScanTree;

const DEPTH: u32 = 4;
const MIN_SIZE: u64 = 16 << 20;
/// Changes smaller than this are not reported.
const MIN_DELTA: u64 = 64 << 20;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SizeMap {
    /// When the scan was made (Unix seconds).
    pub at: i64,
    /// Folder path → logical size.
    pub sizes: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GrowthItem {
    pub path: String,
    /// Node in the current scan (None when the folder is gone).
    pub node: Option<u32>,
    pub before: u64,
    pub after: u64,
    pub delta: i64,
    /// Not in the previous scan (or too small to be recorded then).
    pub is_new: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct GrowthReport {
    /// When the previous scan was made; None when there is nothing to compare with.
    pub since: Option<i64>,
    pub total_before: u64,
    pub total_after: u64,
    pub grew: Vec<GrowthItem>,
    pub shrank: Vec<GrowthItem>,
}

/// Sizes of the bigger folders of `tree`, a few levels deep.
pub fn size_map(tree: &ScanTree, at: i64) -> SizeMap {
    let mut sizes = Vec::new();
    let root = tree.root();
    if let Some(n) = tree.node(root) {
        sizes.push((tree.path(root), n.size));
    }
    let mut level = vec![root];
    for _ in 0..DEPTH {
        let mut next = Vec::new();
        for id in level {
            for c in tree.child_ids(id) {
                let Some(n) = tree.node(c) else { continue };
                if n.is_dir() && !n.is_deleted() && n.size >= MIN_SIZE {
                    sizes.push((tree.path(c), n.size));
                    next.push(c);
                }
            }
        }
        level = next;
    }
    SizeMap { at, sizes }
}

fn file_name(scan_id: &str, suffix: &str) -> String {
    let safe: String = scan_id.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    format!("growth-{safe}{suffix}.json")
}

fn latest_path(dir: &Path, scan_id: &str) -> PathBuf {
    dir.join(file_name(scan_id, ""))
}

fn previous_path(dir: &Path, scan_id: &str) -> PathBuf {
    dir.join(file_name(scan_id, "-prev"))
}

/// Remember a fresh scan: its map becomes the latest, the old latest the baseline.
pub fn record_scan(dir: &Path, scan_id: &str, tree: &ScanTree, at: i64) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let latest = latest_path(dir, scan_id);
    if latest.is_file() {
        std::fs::rename(&latest, previous_path(dir, scan_id))?;
    }
    crate::settings::write_atomic(&latest, &serde_json::to_vec(&size_map(tree, at))?)
}

pub fn baseline(dir: &Path, scan_id: &str) -> Option<SizeMap> {
    serde_json::from_slice(&std::fs::read(previous_path(dir, scan_id)).ok()?).ok()
}

/// Compare `tree` with an earlier map.
pub fn compare(tree: &ScanTree, base: &SizeMap, limit: usize) -> GrowthReport {
    let now = size_map(tree, 0);
    let before: HashMap<String, u64> = base.sizes.iter().map(|(p, s)| (p.to_lowercase(), *s)).collect();
    let after: HashMap<String, (&str, u64)> = now.sizes.iter().map(|(p, s)| (p.to_lowercase(), (p.as_str(), *s))).collect();
    let mut items: Vec<GrowthItem> = Vec::new();
    for (key, (path, size)) in &after {
        let b = before.get(key).copied();
        items.push(GrowthItem { path: path.to_string(), node: tree.find_path(path), before: b.unwrap_or(0), after: *size, delta: *size as i64 - b.unwrap_or(0) as i64, is_new: b.is_none() });
    }
    for (path, size) in &base.sizes {
        if after.contains_key(&path.to_lowercase()) {
            continue;
        }
        // Shrunk below the recording size, or deleted.
        let node = tree.find_path(path);
        let now_size = node.and_then(|n| tree.node(n)).filter(|n| !n.is_deleted()).map_or(0, |n| n.size);
        items.push(GrowthItem { path: path.clone(), node, before: *size, after: now_size, delta: now_size as i64 - *size as i64, is_new: false });
    }
    let root_key = tree.path(tree.root()).to_lowercase();
    let total_before = before.get(&root_key).copied().unwrap_or(0);
    let total_after = tree.node(tree.root()).map_or(0, |n| n.size);
    items.retain(|i| i.delta.unsigned_abs() >= MIN_DELTA && i.path.to_lowercase() != root_key);
    let pick = |grow: bool| {
        let cands: Vec<&GrowthItem> = items.iter().filter(|i| (i.delta > 0) == grow).collect();
        let mut keep: Vec<GrowthItem> = cands
            .iter()
            .filter(|p| {
                // Drop a parent when one folder inside it explains most of the change.
                let parent = p.path.to_lowercase();
                let parent = parent.trim_end_matches(['\\', '/']);
                !cands.iter().any(|c| {
                    let child = c.path.to_lowercase();
                    child.len() > parent.len() + 1 && child.starts_with(parent) && matches!(child.as_bytes()[parent.len()], b'\\' | b'/') && c.delta.unsigned_abs() as f64 >= 0.75 * p.delta.unsigned_abs() as f64
                })
            })
            .map(|i| (*i).clone())
            .collect();
        keep.sort_by_key(|i| std::cmp::Reverse(i.delta.unsigned_abs()));
        keep.truncate(limit);
        keep
    };
    GrowthReport { since: Some(base.at), total_before, total_after, grew: pick(true), shrank: pick(false) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::tree::{ScanInfo, ScanMethod, TreeBuilder};

    const MB: u64 = 1 << 20;

    fn tree(downloads: u64, games: u64, extra: bool) -> ScanTree {
        let mut b = TreeBuilder::with_capacity(16);
        let root = b.add(u32::MAX, "C:\\", true, 0, 0, 0, 0);
        let users = b.add(root, "Users", true, 0, 0, 0, 0);
        let sam = b.add(users, "Sam", true, 0, 0, 0, 0);
        let dl = b.add(sam, "Downloads", true, 0, 0, 0, 0);
        b.add(dl, "big.iso", false, downloads, downloads, 0, 0);
        let g = b.add(root, "Games", true, 0, 0, 0, 0);
        b.add(g, "game.pak", false, games, games, 0, 0);
        if extra {
            let n = b.add(root, "NewStuff", true, 0, 0, 0, 0);
            b.add(n, "x.bin", false, 500 * MB, 500 * MB, 0, 0);
        }
        b.finish(
            root,
            ScanInfo { root_path: "C:\\".into(), method: ScanMethod::Walk, started_at: 0, duration_ms: 0, volume_serial: None, volume_total: None, volume_free: None, cluster_size: None, errors: 0, from_cache: false, separator: '\\' },
        )
    }

    #[test]
    fn reports_the_specific_folder_that_grew() {
        let old = size_map(&tree(1000 * MB, 4000 * MB, false), 100);
        let report = compare(&tree(3000 * MB, 2000 * MB, true), &old, 10);
        assert_eq!(report.since, Some(100));
        let grew: Vec<(&str, i64)> = report.grew.iter().map(|i| (i.path.rsplit(['\\', '/']).next().unwrap(), i.delta / MB as i64)).collect();
        // Downloads (not Users or Sam), and the new folder.
        assert_eq!(grew, [("Downloads", 2000), ("NewStuff", 500)]);
        assert!(report.grew[1].is_new);
        assert_eq!(report.shrank.len(), 1);
        assert_eq!(report.shrank[0].delta, -2000 * MB as i64);
        assert_eq!(report.total_after, 5500 * MB);
    }

    #[test]
    fn files_rotate() {
        let dir = tempfile::tempdir().unwrap();
        assert!(baseline(dir.path(), "c:").is_none());
        record_scan(dir.path(), "c:", &tree(MB * 100, MB * 100, false), 1).unwrap();
        assert!(baseline(dir.path(), "c:").is_none());
        record_scan(dir.path(), "c:", &tree(MB * 200, MB * 100, false), 2).unwrap();
        assert_eq!(baseline(dir.path(), "c:").unwrap().at, 1);
    }
}
