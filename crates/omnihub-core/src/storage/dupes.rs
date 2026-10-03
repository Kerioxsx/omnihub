//! Duplicate file detection.
//!
//! Candidates come from the scan for free: only files of exactly the same
//! size can be identical. Those are narrowed by hashing the first and last
//! 64 KiB, and only the survivors are hashed in full (BLAKE3). Cloud
//! placeholders are skipped so the search never triggers downloads.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::tree::{ScanTree, NODE_CLOUD, NODE_REPARSE};

const EDGE: u64 = 64 * 1024;

/// (size, content hash) identifies a group of identical files.
type GroupKey = (u64, [u8; 32]);
/// A candidate file: its node id and path.
type Candidate = (u32, String);

#[derive(Debug, Default)]
pub struct DupeProgress {
    pub phase: AtomicU64,
    pub files_total: AtomicU64,
    pub files_done: AtomicU64,
    pub bytes_total: AtomicU64,
    pub bytes_done: AtomicU64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DupeOptions {
    pub under: u32,
    pub min_size: u64,
    pub max_groups: usize,
}

impl Default for DupeOptions {
    fn default() -> Self {
        DupeOptions { under: 0, min_size: 1 << 20, max_groups: 500 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DupeFile {
    pub id: u32,
    pub path: String,
    pub modified: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DupeGroup {
    pub hash: String,
    pub size: u64,
    /// Bytes freed by keeping one copy.
    pub wasted: u64,
    pub files: Vec<DupeFile>,
}

fn edge_hash(path: &str, size: u64, progress: &DupeProgress) -> Option<[u8; 32]> {
    let mut f = File::open(path).ok()?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; EDGE as usize];
    let head = size.min(EDGE) as usize;
    f.read_exact(&mut buf[..head]).ok()?;
    h.update(&buf[..head]);
    if size > EDGE {
        let tail_start = size.saturating_sub(EDGE).max(EDGE);
        let tail = (size - tail_start) as usize;
        f.seek(SeekFrom::Start(tail_start)).ok()?;
        f.read_exact(&mut buf[..tail]).ok()?;
        h.update(&buf[..tail]);
    }
    progress.files_done.fetch_add(1, Ordering::Relaxed);
    Some(*h.finalize().as_bytes())
}

fn full_hash(path: &str, progress: &DupeProgress, cancel: &AtomicBool) -> Option<[u8; 32]> {
    let mut f = File::open(path).ok()?;
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        progress.bytes_done.fetch_add(n as u64, Ordering::Relaxed);
    }
    progress.files_done.fetch_add(1, Ordering::Relaxed);
    Some(*h.finalize().as_bytes())
}

pub fn find_duplicates(tree: &ScanTree, opts: &DupeOptions, progress: &DupeProgress, cancel: &AtomicBool) -> Vec<DupeGroup> {
    let under = if (opts.under as usize) < tree.len() { opts.under } else { 0 };
    let mut by_size: HashMap<u64, Vec<u32>> = HashMap::new();
    for id in tree.files_under(under) {
        let n = tree.node(id).unwrap();
        if n.size < opts.min_size.max(1) || n.flags & (NODE_CLOUD | NODE_REPARSE) != 0 || !tree.path_is_exact(id) {
            continue;
        }
        by_size.entry(n.size).or_default().push(id);
    }
    by_size.retain(|_, v| v.len() > 1);

    // Phase 1: first + last 64 KiB.
    progress.phase.store(1, Ordering::Relaxed);
    let candidates: Vec<(u64, u32, String)> = by_size
        .iter()
        .flat_map(|(&size, ids)| ids.iter().map(move |&id| (size, id)))
        .map(|(size, id)| (size, id, tree.path(id)))
        .collect();
    progress.files_total.store(candidates.len() as u64, Ordering::Relaxed);
    progress.files_done.store(0, Ordering::Relaxed);
    let edged: Vec<(GroupKey, Candidate)> = candidates
        .into_par_iter()
        .filter_map(|(size, id, path)| {
            if cancel.load(Ordering::Relaxed) {
                return None;
            }
            let h = edge_hash(&path, size, progress)?;
            Some(((size, h), (id, path)))
        })
        .collect();
    if cancel.load(Ordering::Relaxed) {
        return Vec::new();
    }
    let mut by_edge: HashMap<GroupKey, Vec<Candidate>> = HashMap::new();
    for (k, v) in edged {
        by_edge.entry(k).or_default().push(v);
    }
    by_edge.retain(|_, v| v.len() > 1);

    // Phase 2: full content, only where the edges matched and the file is
    // larger than what the edge hash already covered.
    progress.phase.store(2, Ordering::Relaxed);
    let work: Vec<(u64, u32, String)> = by_edge
        .iter()
        .flat_map(|((size, _), v)| v.iter().map(move |(id, p)| (*size, *id, p.clone())))
        .collect();
    let full_needed: u64 = work.iter().filter(|w| w.0 > 2 * EDGE).map(|w| w.0).sum();
    progress.bytes_total.store(full_needed, Ordering::Relaxed);
    progress.files_total.store(work.len() as u64, Ordering::Relaxed);
    progress.files_done.store(0, Ordering::Relaxed);
    let edge_of: HashMap<u32, [u8; 32]> = by_edge
        .iter()
        .flat_map(|((_, h), v)| v.iter().map(move |(id, _)| (*id, *h)))
        .collect();
    let hashed: Vec<(GroupKey, Candidate)> = work
        .into_par_iter()
        .filter_map(|(size, id, path)| {
            let h = if size <= 2 * EDGE {
                progress.files_done.fetch_add(1, Ordering::Relaxed);
                edge_of[&id]
            } else {
                full_hash(&path, progress, cancel)?
            };
            Some(((size, h), (id, path)))
        })
        .collect();
    if cancel.load(Ordering::Relaxed) {
        return Vec::new();
    }
    let mut groups: HashMap<GroupKey, Vec<Candidate>> = HashMap::new();
    for (k, v) in hashed {
        groups.entry(k).or_default().push(v);
    }
    let mut out: Vec<DupeGroup> = groups
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|((size, hash), mut files)| {
            files.sort_by(|a, b| a.1.cmp(&b.1));
            DupeGroup {
                hash: hex::encode(&hash[..16]),
                size,
                wasted: size * (files.len() as u64 - 1),
                files: files
                    .into_iter()
                    .map(|(id, path)| DupeFile { id, path, modified: tree.node(id).unwrap().modified })
                    .collect(),
            }
        })
        .collect();
    out.sort_by(|a, b| b.wasted.cmp(&a.wasted).then_with(|| a.hash.cmp(&b.hash)));
    out.truncate(opts.max_groups);
    progress.phase.store(3, Ordering::Relaxed);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::walk::{walk, WalkOptions, WalkProgress};

    #[test]
    fn finds_real_duplicates_only() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("a/b")).unwrap();
        let big: Vec<u8> = (0..400_000u32).map(|i| (i * 7 % 251) as u8).collect();
        std::fs::write(p.join("one.bin"), &big).unwrap();
        std::fs::write(p.join("a/b/copy.bin"), &big).unwrap();
        std::fs::write(p.join("a/third.bin"), &big).unwrap();
        // Same size and same edges, different middle.
        let mut tricky = big.clone();
        tricky[200_000] ^= 0xFF;
        std::fs::write(p.join("a/tricky.bin"), &tricky).unwrap();
        // Small identical files below the threshold.
        std::fs::write(p.join("s1"), b"same").unwrap();
        std::fs::write(p.join("s2"), b"same").unwrap();
        // Same size, different content.
        std::fs::write(p.join("other.bin"), vec![1u8; 400_000]).unwrap();

        let tree = walk(p, &WalkOptions::default(), &WalkProgress::default(), &AtomicBool::new(false)).unwrap();
        let progress = DupeProgress::default();
        let groups = find_duplicates(&tree, &DupeOptions { min_size: 1000, ..Default::default() }, &progress, &AtomicBool::new(false));
        assert_eq!(groups.len(), 1, "{groups:#?}");
        assert_eq!(groups[0].files.len(), 3);
        assert_eq!(groups[0].wasted, 800_000);

        let small = find_duplicates(&tree, &DupeOptions { min_size: 1, ..Default::default() }, &progress, &AtomicBool::new(false));
        assert_eq!(small.len(), 2);
    }
}
