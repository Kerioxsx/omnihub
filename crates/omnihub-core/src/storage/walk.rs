//! Parallel directory traversal: the fallback when the MFT cannot be read
//! (no administrator rights, FAT/exFAT/ReFS, network shares, other OSes).
//!
//! Every directory is listed by a rayon task and its subdirectories are
//! fanned out to the pool. On Windows `DirEntry::metadata` comes from the
//! `FindNextFile` data, so listing a directory costs one call per batch of
//! entries rather than one per file.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Instant, UNIX_EPOCH};

use parking_lot::Mutex;
use rayon::prelude::*;

use super::tree::{flags_from_attributes, ScanInfo, ScanMethod, ScanTree, TreeBuilder, NO_PARENT};
#[cfg(not(unix))]
use super::tree::NODE_CLOUD;

#[derive(Debug, Default)]
pub struct WalkProgress {
    pub files: AtomicU64,
    pub dirs: AtomicU64,
    pub bytes: AtomicU64,
    pub errors: AtomicU64,
    pub current: Mutex<String>,
}

#[derive(Debug, Clone)]
pub struct WalkOptions {
    /// Do not descend into other file systems (Unix mount points).
    pub same_device: bool,
    /// Lower-cased absolute paths that are skipped entirely.
    pub exclude: Vec<String>,
    /// Cluster size used to estimate allocation where the OS does not say.
    pub cluster_size: u64,
}

impl Default for WalkOptions {
    fn default() -> Self {
        WalkOptions { same_device: true, exclude: Vec::new(), cluster_size: 4096 }
    }
}

struct FileOut {
    name: String,
    size: u64,
    alloc: u64,
    modified: u32,
    flags: u16,
}

struct DirOut {
    name: String,
    modified: u32,
    flags: u16,
    files: Vec<FileOut>,
    dirs: Vec<DirOut>,
}

#[derive(Debug, thiserror::Error)]
pub enum WalkError {
    #[error("cannot read {0}: {1}")]
    Root(PathBuf, std::io::Error),
    #[error("scan cancelled")]
    Cancelled,
}

pub fn walk(root: &Path, opts: &WalkOptions, progress: &WalkProgress, cancel: &AtomicBool) -> Result<ScanTree, WalkError> {
    let started = Instant::now();
    let started_at = chrono::Utc::now().timestamp();
    let meta = fs::metadata(root).map_err(|e| WalkError::Root(root.to_path_buf(), e))?;
    let ctx = Ctx {
        opts,
        progress,
        cancel,
        root_dev: device_of(&meta),
        #[cfg(unix)]
        seen_inodes: Mutex::new(std::collections::HashSet::new()),
    };
    let root_name = display_root(root);
    let mut out = scan_dir(&ctx, root, root_name.clone(), &meta);
    if cancel.load(Ordering::Relaxed) {
        return Err(WalkError::Cancelled);
    }
    out.name = root_name.clone();

    let total = progress.files.load(Ordering::Relaxed) + progress.dirs.load(Ordering::Relaxed) + 1;
    let mut b = TreeBuilder::with_capacity(total as usize);
    let root_idx = flatten(&mut b, NO_PARENT, out);
    let info = ScanInfo {
        root_path: root_name,
        method: ScanMethod::Walk,
        started_at,
        duration_ms: started.elapsed().as_millis() as u64,
        volume_serial: None,
        volume_total: None,
        volume_free: None,
        cluster_size: Some(opts.cluster_size),
        errors: progress.errors.load(Ordering::Relaxed),
        from_cache: false,
        separator: std::path::MAIN_SEPARATOR,
    };
    Ok(b.finish(root_idx, info))
}

fn flatten(b: &mut TreeBuilder, parent: u32, dir: DirOut) -> u32 {
    let idx = b.add(parent, &dir.name, true, 0, 0, dir.modified, dir.flags);
    for f in dir.files {
        b.add(idx, &f.name, false, f.size, f.alloc, f.modified, f.flags);
    }
    for d in dir.dirs {
        flatten(b, idx, d);
    }
    idx
}

fn display_root(root: &Path) -> String {
    let s = root.to_string_lossy().to_string();
    // `\\?\C:\` → `C:\`
    s.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(s)
}

struct Ctx<'a> {
    opts: &'a WalkOptions,
    progress: &'a WalkProgress,
    cancel: &'a AtomicBool,
    root_dev: Option<u64>,
    #[cfg(unix)]
    seen_inodes: Mutex<std::collections::HashSet<(u64, u64)>>,
}

fn scan_dir(ctx: &Ctx, path: &Path, name: String, meta: &fs::Metadata) -> DirOut {
    let mut out = DirOut {
        name,
        modified: modified_of(meta),
        flags: attr_flags(meta),
        files: Vec::new(),
        dirs: Vec::new(),
    };
    if ctx.cancel.load(Ordering::Relaxed) {
        return out;
    }
    ctx.progress.dirs.fetch_add(1, Ordering::Relaxed);
    let entries = match fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => {
            ctx.progress.errors.fetch_add(1, Ordering::Relaxed);
            return out;
        }
    };
    if let Some(mut cur) = ctx.progress.current.try_lock() {
        cur.clear();
        cur.push_str(&path.to_string_lossy());
    }
    let mut subdirs: Vec<(PathBuf, String, fs::Metadata)> = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else {
            ctx.progress.errors.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        let Ok(ft) = entry.file_type() else {
            ctx.progress.errors.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        let Ok(meta) = entry.metadata() else {
            ctx.progress.errors.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let child = entry.path();
        if !ctx.opts.exclude.is_empty() {
            let lower = child.to_string_lossy().to_lowercase();
            if ctx.opts.exclude.contains(&lower) {
                continue;
            }
        }
        // Symlinks and junctions are listed but never followed.
        if ft.is_dir() && !ft.is_symlink() {
            if ctx.opts.same_device && ctx.root_dev.is_some() && device_of(&meta) != ctx.root_dev {
                continue;
            }
            subdirs.push((child, name, meta));
            continue;
        }
        let (size, alloc) = sizes_of(ctx, &meta, &ft);
        ctx.progress.files.fetch_add(1, Ordering::Relaxed);
        ctx.progress.bytes.fetch_add(size, Ordering::Relaxed);
        out.files.push(FileOut { name, size, alloc, modified: modified_of(&meta), flags: attr_flags(&meta) });
    }
    out.dirs = subdirs
        .into_par_iter()
        .map(|(p, n, m)| scan_dir(ctx, &p, n, &m))
        .collect();
    out
}

fn modified_of(meta: &fs::Metadata) -> u32 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs().min(u32::MAX as u64) as u32)
        .unwrap_or(0)
}

#[cfg(windows)]
fn attr_flags(meta: &fs::Metadata) -> u16 {
    use std::os::windows::fs::MetadataExt;
    flags_from_attributes(meta.file_attributes())
}

#[cfg(not(windows))]
fn attr_flags(_meta: &fs::Metadata) -> u16 {
    flags_from_attributes(0)
}

#[cfg(unix)]
fn device_of(meta: &fs::Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    Some(meta.dev())
}

#[cfg(not(unix))]
fn device_of(_meta: &fs::Metadata) -> Option<u64> {
    None
}

#[cfg(unix)]
fn sizes_of(ctx: &Ctx, meta: &fs::Metadata, ft: &fs::FileType) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt;
    if ft.is_symlink() {
        return (0, 0);
    }
    // Count hard-linked files once.
    if meta.nlink() > 1 && !ctx.seen_inodes.lock().insert((meta.dev(), meta.ino())) {
        return (0, 0);
    }
    (meta.len(), meta.blocks() * 512)
}

#[cfg(not(unix))]
fn sizes_of(ctx: &Ctx, meta: &fs::Metadata, ft: &fs::FileType) -> (u64, u64) {
    if ft.is_symlink() {
        return (0, 0);
    }
    let size = meta.len();
    // Cloud placeholders, sparse and compressed files do not occupy their
    // full size; everything else is rounded up to whole clusters.
    let flags = attr_flags(meta);
    let alloc = if flags & NODE_CLOUD != 0 {
        0
    } else {
        let c = ctx.opts.cluster_size.max(512);
        size.div_ceil(c) * c
    };
    (size, alloc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn walk_matches_layout() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("a/b/c")).unwrap();
        fs::create_dir_all(root.join("empty")).unwrap();
        {
            // Closed before the walk: Windows only updates the size in the
            // directory entry when the handle is closed.
            let mut f = fs::File::create(root.join("a/b/c/deep.bin")).unwrap();
            f.write_all(&vec![7u8; 100_000]).unwrap();
        }
        fs::write(root.join("a/one.txt"), b"hello").unwrap();
        fs::write(root.join("top.dat"), vec![1u8; 4000]).unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("a"), root.join("link-to-a")).unwrap();
            fs::hard_link(root.join("top.dat"), root.join("a/top-link.dat")).unwrap();
        }

        let progress = WalkProgress::default();
        let tree = walk(root, &WalkOptions::default(), &progress, &AtomicBool::new(false)).unwrap();
        let rootn = tree.node(0).unwrap();
        assert_eq!(rootn.size, 100_000 + 5 + 4000);
        let deep = tree.find_path(&root.join("a/b/c/deep.bin").to_string_lossy()).unwrap();
        assert_eq!(tree.node(deep).unwrap().size, 100_000);
        assert!(tree.node(deep).unwrap().alloc >= 100_000);
        let a = tree.find_path(&root.join("a").to_string_lossy()).unwrap();
        assert_eq!(tree.node(a).unwrap().size, 100_005);
        let empty = tree.find_path(&root.join("empty").to_string_lossy()).unwrap();
        assert!(tree.node(empty).unwrap().is_dir());
        assert_eq!(tree.node(empty).unwrap().size, 0);
        assert_eq!(progress.errors.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn cancel_stops_walk() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("x")).unwrap();
        let res = walk(dir.path(), &WalkOptions::default(), &WalkProgress::default(), &AtomicBool::new(true));
        assert!(matches!(res, Err(WalkError::Cancelled)));
    }
}
