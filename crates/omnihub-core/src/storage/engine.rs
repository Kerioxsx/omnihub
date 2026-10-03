//! Scan jobs, finished trees and the operations the UI runs on them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};

use super::cleanup::{self, CleanupOptions, DeleteResult, Suggestion, Tokens};
use super::dupes::{self, DupeGroup, DupeOptions, DupeProgress};
use super::ntfs::mft::MftProgress;
use super::snapshot::VolumeSnapshot;
use super::tree::{ExtensionStat, NodeView, ScanMethod, ScanTree, SearchQuery, SortKey, TreemapItem};
use super::volume_scan;
use super::volumes::{self, VolumeInfo};
use super::walk::{self, WalkOptions, WalkProgress};
use crate::events::EventBus;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ScanMode {
    /// MFT when already elevated and possible, otherwise a directory walk.
    #[default]
    Auto,
    /// MFT, asking for administrator approval if needed.
    Fast,
    /// Directory walk, no elevation.
    Standard,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ScanRequest {
    pub root: String,
    pub mode: ScanMode,
    /// Path of an NTFS image to read as a volume (diagnostics and tests).
    pub image: Option<String>,
    /// Paths to skip during a directory walk.
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JobProgress {
    pub job_id: String,
    pub root: String,
    pub method: String,
    pub phase: String,
    pub state: JobState,
    pub records_done: u64,
    pub records_total: u64,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub errors: u64,
    pub current: String,
    pub elapsed_ms: u64,
    pub scan_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum JobState {
    Running,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub scan_id: String,
    pub root: String,
    pub method: ScanMethod,
    pub scanned_at: i64,
    pub duration_ms: u64,
    pub files: u64,
    pub dirs: u64,
    pub size: u64,
    pub alloc: u64,
    pub errors: u64,
    pub from_cache: bool,
    pub volume_total: Option<u64>,
    pub volume_free: Option<u64>,
    pub nodes: u64,
    pub memory_bytes: u64,
}

struct Job {
    id: String,
    root: String,
    method: Mutex<String>,
    phase: Mutex<String>,
    cancel: AtomicBool,
    mft: MftProgress,
    walk: WalkProgress,
    started: Instant,
    state: Mutex<(JobState, Option<String>, Option<String>)>,
    /// For helper scans: where to drop the cancel marker.
    cancel_marker: Mutex<Option<PathBuf>>,
}

impl Job {
    fn progress(&self) -> JobProgress {
        let (state, scan_id, error) = self.state.lock().clone();
        JobProgress {
            job_id: self.id.clone(),
            root: self.root.clone(),
            method: self.method.lock().clone(),
            phase: self.phase.lock().clone(),
            state,
            records_done: self.mft.records_done.load(Ordering::Relaxed),
            records_total: self.mft.records_total.load(Ordering::Relaxed),
            files: self.walk.files.load(Ordering::Relaxed),
            dirs: self.walk.dirs.load(Ordering::Relaxed),
            bytes: self.walk.bytes.load(Ordering::Relaxed),
            errors: self.walk.errors.load(Ordering::Relaxed),
            current: self.walk.current.lock().clone(),
            elapsed_ms: self.started.elapsed().as_millis() as u64,
            scan_id,
            error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DupeJobProgress {
    pub job_id: String,
    pub phase: u64,
    pub files_done: u64,
    pub files_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub done: bool,
}

struct DupeJob {
    id: String,
    progress: DupeProgress,
    cancel: AtomicBool,
    result: Mutex<Option<Vec<DupeGroup>>>,
}

/// Something that can run this executable's helper mode with administrator
/// rights. Abstracted so tests (and other shells) can provide their own.
pub trait ElevatedRunner: Send + Sync {
    fn run(&self, args: &[String]) -> std::io::Result<i32>;
}

/// Runs `current_exe --omnihub-helper ...` through UAC.
pub struct SelfElevated;

impl ElevatedRunner for SelfElevated {
    fn run(&self, args: &[String]) -> std::io::Result<i32> {
        let exe = std::env::current_exe()?;
        crate::system::elevation::run_elevated_and_wait(&exe, args, false)
    }
}

pub struct StorageEngine {
    trees: RwLock<HashMap<String, Arc<RwLock<ScanTree>>>>,
    jobs: Mutex<HashMap<String, Arc<Job>>>,
    dupe_jobs: Mutex<HashMap<String, Arc<DupeJob>>>,
    events: EventBus,
    scan_dir: PathBuf,
    runner: Arc<dyn ElevatedRunner>,
    tokens: Tokens,
}

pub fn scan_id_for(root: &str) -> String {
    let t = root.trim_end_matches(['\\', '/']);
    let t = if t.is_empty() { root } else { t };
    t.to_lowercase()
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("no scan for {0}; scan it first")]
    UnknownScan(String),
    #[error("item {0} is not in this scan")]
    UnknownNode(u32),
    #[error("{0}")]
    Other(String),
}

impl StorageEngine {
    pub fn new(events: EventBus, scan_dir: PathBuf, runner: Arc<dyn ElevatedRunner>) -> Arc<Self> {
        Arc::new(StorageEngine {
            trees: RwLock::new(HashMap::new()),
            jobs: Mutex::new(HashMap::new()),
            dupe_jobs: Mutex::new(HashMap::new()),
            events,
            scan_dir,
            runner,
            tokens: cleanup::default_tokens(),
        })
    }

    pub fn volumes(&self) -> Vec<VolumeInfo> {
        volumes::list()
    }

    pub fn is_elevated(&self) -> bool {
        crate::system::elevation::is_elevated()
    }

    fn tree(&self, scan_id: &str) -> Result<Arc<RwLock<ScanTree>>, StorageError> {
        self.trees
            .read()
            .get(&scan_id_for(scan_id))
            .cloned()
            .ok_or_else(|| StorageError::UnknownScan(scan_id.to_string()))
    }

    pub fn summary_of(scan_id: &str, t: &ScanTree) -> ScanSummary {
        let root = t.node(0).unwrap();
        ScanSummary {
            scan_id: scan_id.to_string(),
            root: t.info.root_path.clone(),
            method: t.info.method,
            scanned_at: t.info.started_at,
            duration_ms: t.info.duration_ms,
            files: root.files as u64,
            dirs: root.desc.saturating_sub(root.files) as u64,
            size: root.size,
            alloc: root.alloc,
            errors: t.info.errors,
            from_cache: t.info.from_cache,
            volume_total: t.info.volume_total,
            volume_free: t.info.volume_free,
            nodes: t.len() as u64,
            memory_bytes: t.memory_bytes() as u64,
        }
    }

    pub fn scans(&self) -> Vec<ScanSummary> {
        let trees = self.trees.read();
        let mut v: Vec<ScanSummary> = trees.iter().map(|(id, t)| Self::summary_of(id, &t.read())).collect();
        v.sort_by(|a, b| a.root.cmp(&b.root));
        v
    }

    pub fn summary(&self, scan_id: &str) -> Result<ScanSummary, StorageError> {
        let t = self.tree(scan_id)?;
        let t = t.read();
        Ok(Self::summary_of(&scan_id_for(scan_id), &t))
    }

    fn install_tree(&self, tree: ScanTree) -> String {
        let id = scan_id_for(&tree.info.root_path);
        self.trees.write().insert(id.clone(), Arc::new(RwLock::new(tree)));
        id
    }

    /// Load the last MFT snapshot of a volume, if one is cached, so results
    /// show instantly (marked as possibly stale) while a refresh runs.
    pub fn open_cached(&self, root: &str) -> Option<ScanSummary> {
        let letter = volumes::drive_letter(root)?;
        let path = volume_scan::snapshot_path(&self.scan_dir, letter);
        let snap = VolumeSnapshot::load(&path).ok()?;
        let tree = snap.to_tree(true);
        let id = self.install_tree(tree);
        self.summary(&id).ok()
    }

    pub fn forget(&self, scan_id: &str) {
        self.trees.write().remove(&scan_id_for(scan_id));
    }

    pub fn start_scan(self: &Arc<Self>, req: ScanRequest) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let job = Arc::new(Job {
            id: id.clone(),
            root: req.root.clone(),
            method: Mutex::new(String::new()),
            phase: Mutex::new("starting".into()),
            cancel: AtomicBool::new(false),
            mft: MftProgress::default(),
            walk: WalkProgress::default(),
            started: Instant::now(),
            state: Mutex::new((JobState::Running, None, None)),
            cancel_marker: Mutex::new(None),
        });
        self.jobs.lock().insert(id.clone(), job.clone());

        let this = self.clone();
        let ticker_job = job.clone();
        let events = self.events.clone();
        std::thread::Builder::new()
            .name("scan-progress".into())
            .spawn(move || loop {
                let p = ticker_job.progress();
                let running = p.state == JobState::Running;
                events.emit("storage:progress", &p);
                if !running {
                    break;
                }
                std::thread::sleep(Duration::from_millis(200));
            })
            .ok();
        std::thread::Builder::new()
            .name("scan".into())
            .spawn(move || {
                let result = this.run_scan(&job, &req);
                let mut st = job.state.lock();
                match result {
                    Ok(tree) => {
                        let scan_id = this.install_tree(tree);
                        st.0 = JobState::Done;
                        st.1 = Some(scan_id.clone());
                        drop(st);
                        if let Ok(summary) = this.summary(&scan_id) {
                            this.events.emit("storage:done", serde_json::json!({ "jobId": job.id, "summary": summary }));
                        }
                    }
                    Err(e) => {
                        let cancelled = job.cancel.load(Ordering::Relaxed);
                        st.0 = if cancelled { JobState::Cancelled } else { JobState::Failed };
                        st.2 = Some(e.clone());
                        drop(st);
                        this.events.emit("storage:failed", serde_json::json!({ "jobId": job.id, "error": e, "cancelled": cancelled }));
                    }
                }
            })
            .expect("spawn scan thread");
        id
    }

    fn run_scan(&self, job: &Job, req: &ScanRequest) -> Result<ScanTree, String> {
        if let Some(image) = &req.image {
            *job.method.lock() = "mft".into();
            *job.phase.lock() = "mft".into();
            return scan_image(Path::new(image), &req.root, &job.mft, &job.cancel).map_err(|e| e.to_string());
        }

        let letter = volumes::drive_letter(&req.root);
        let vol = letter.and_then(|l| self.volumes().into_iter().find(|v| volumes::drive_letter(&v.root) == Some(l)));
        let mft_ok = vol.as_ref().is_some_and(|v| v.mft_capable);
        let elevated = self.is_elevated();
        let use_mft = mft_ok && match req.mode {
            ScanMode::Standard => false,
            ScanMode::Fast => true,
            ScanMode::Auto => elevated,
        };

        if use_mft {
            let letter = letter.unwrap();
            *job.method.lock() = "mft".into();
            let res = if elevated {
                self.mft_in_process(job, letter)
            } else {
                self.mft_via_helper(job, letter)
            };
            match res {
                Ok(mut tree) => {
                    // The MFT holds the whole drive; excluded folders are
                    // hidden (and left out of the totals) afterwards.
                    for path in &req.exclude {
                        if let Some(id) = tree.find_path(path) {
                            tree.mark_deleted(id);
                        }
                    }
                    return Ok(tree);
                }
                Err(e) if job.cancel.load(Ordering::Relaxed) => return Err(e),
                Err(e) if req.mode == ScanMode::Fast && e.contains("declined") => {
                    // The user said no to UAC: fall back to a normal scan.
                    tracing::info!("fast scan declined, walking instead: {e}");
                }
                Err(e) => {
                    tracing::warn!("MFT scan failed, walking instead: {e}");
                }
            }
        }

        *job.method.lock() = "walk".into();
        *job.phase.lock() = "walk".into();
        let opts = WalkOptions {
            exclude: req.exclude.iter().map(|e| e.to_lowercase()).collect(),
            cluster_size: vol.as_ref().map_or(4096, |v| v.cluster_size),
            ..Default::default()
        };
        let mut tree = walk::walk(Path::new(&req.root), &opts, &job.walk, &job.cancel).map_err(|e| e.to_string())?;
        if let Some((total, free)) = vol.map(|v| (v.total, v.free)) {
            tree.info.volume_total = Some(total);
            tree.info.volume_free = Some(free);
        }
        Ok(tree)
    }

    #[cfg(windows)]
    fn mft_in_process(&self, job: &Job, letter: char) -> Result<ScanTree, String> {
        let snap = volume_scan::scan_ntfs_volume(letter, &self.scan_dir, true, &job.mft, &job.cancel, &|p| {
            *job.phase.lock() = p.to_string();
        })
        .map_err(|e| e.to_string())?;
        Ok(snap.to_tree(false))
    }

    #[cfg(not(windows))]
    fn mft_in_process(&self, _job: &Job, _letter: char) -> Result<ScanTree, String> {
        Err("MFT scanning of live volumes is only available on Windows".into())
    }

    fn mft_via_helper(&self, job: &Job, letter: char) -> Result<ScanTree, String> {
        let dir = &self.scan_dir;
        for p in [volume_scan::progress_path(dir, letter), volume_scan::cancel_path(dir, letter), volume_scan::error_path(dir, letter)] {
            let _ = std::fs::remove_file(p);
        }
        *job.cancel_marker.lock() = Some(volume_scan::cancel_path(dir, letter));
        *job.phase.lock() = "elevating".into();
        let args = vec![
            crate::helper::HELPER_FLAG.to_string(),
            "scan-volume".to_string(),
            letter.to_string(),
            dir.to_string_lossy().to_string(),
        ];
        let done = AtomicBool::new(false);
        let progress_file = volume_scan::progress_path(dir, letter);
        let code = std::thread::scope(|scope| {
            scope.spawn(|| {
                while !done.load(Ordering::Relaxed) {
                    if let Ok(bytes) = std::fs::read(&progress_file) {
                        if let Ok(p) = serde_json::from_slice::<volume_scan::HelperProgress>(&bytes) {
                            job.mft.records_done.store(p.records_done, Ordering::Relaxed);
                            job.mft.records_total.store(p.records_total, Ordering::Relaxed);
                            job.mft.bytes_read.store(p.bytes_read, Ordering::Relaxed);
                            *job.phase.lock() = p.phase;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(150));
                }
            });
            let code = self.runner.run(&args);
            done.store(true, Ordering::Relaxed);
            code
        });
        let code = code.map_err(|e| e.to_string())?;
        match code {
            0 => {
                *job.phase.lock() = "loading".into();
                let snap = VolumeSnapshot::load(&volume_scan::snapshot_path(dir, letter)).map_err(|e| e.to_string())?;
                Ok(snap.to_tree(false))
            }
            2 => Err("scan cancelled".into()),
            _ => {
                let msg = std::fs::read_to_string(volume_scan::error_path(dir, letter)).unwrap_or_default();
                Err(if msg.is_empty() { format!("scan helper failed (exit code {code})") } else { msg })
            }
        }
    }

    pub fn progress(&self, job_id: &str) -> Option<JobProgress> {
        self.jobs.lock().get(job_id).map(|j| j.progress())
    }

    pub fn cancel(&self, job_id: &str) {
        if let Some(j) = self.jobs.lock().get(job_id) {
            j.cancel.store(true, Ordering::Relaxed);
            if let Some(marker) = j.cancel_marker.lock().as_ref() {
                let _ = std::fs::write(marker, b"cancel");
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn children(&self, scan_id: &str, node: u32, sort: SortKey, descending: bool, offset: usize, limit: usize, include_hidden: bool) -> Result<ChildrenPage, StorageError> {
        let t = self.tree(scan_id)?;
        let t = t.read();
        if t.node(node).is_none() {
            return Err(StorageError::UnknownNode(node));
        }
        let (items, total) = t.children(node, sort, descending, offset, limit, include_hidden);
        Ok(ChildrenPage {
            node: t.view(node),
            path: t.path(node),
            breadcrumbs: t.breadcrumbs(node).into_iter().map(|(id, name)| Crumb { id, name }).collect(),
            items,
            total,
        })
    }

    pub fn treemap(&self, scan_id: &str, node: u32, depth: u32, max_items: usize) -> Result<TreemapItem, StorageError> {
        let t = self.tree(scan_id)?;
        let t = t.read();
        if t.node(node).is_none() {
            return Err(StorageError::UnknownNode(node));
        }
        Ok(t.treemap(node, depth.min(8), max_items.min(5000)))
    }

    pub fn top_files(&self, scan_id: &str, node: u32, n: usize) -> Result<Vec<PathedNode>, StorageError> {
        let t = self.tree(scan_id)?;
        let t = t.read();
        Ok(t.top_files(node, n.min(1000)).into_iter().map(|v| PathedNode { path: t.path(v.id), node: v }).collect())
    }

    pub fn extensions(&self, scan_id: &str, node: u32) -> Result<Vec<ExtensionStat>, StorageError> {
        let t = self.tree(scan_id)?;
        let t = t.read();
        Ok(t.extensions(node))
    }

    pub fn search(&self, scan_id: &str, q: &SearchQuery) -> Result<SearchResult, StorageError> {
        let t = self.tree(scan_id)?;
        let t = t.read();
        let started = Instant::now();
        let (items, total) = t.search(q);
        Ok(SearchResult {
            items: items.into_iter().map(|v| PathedNode { path: t.path(v.id), node: v }).collect(),
            total,
            took_ms: started.elapsed().as_millis() as u64,
        })
    }

    pub fn path_of(&self, scan_id: &str, node: u32) -> Result<String, StorageError> {
        let t = self.tree(scan_id)?;
        let t = t.read();
        if t.node(node).is_none() {
            return Err(StorageError::UnknownNode(node));
        }
        Ok(t.path(node))
    }

    pub fn find(&self, scan_id: &str, path: &str) -> Result<Option<u32>, StorageError> {
        let t = self.tree(scan_id)?;
        let t = t.read();
        Ok(t.find_path(path))
    }

    /// Size of a folder from any finished scan that covers it.
    pub fn size_of_path(&self, path: &str) -> Option<(u64, u64)> {
        let lower = path.to_lowercase();
        let trees = self.trees.read();
        for (id, t) in trees.iter() {
            if lower.starts_with(id.as_str()) {
                let t = t.read();
                if let Some(n) = t.find_path(path) {
                    let node = t.node(n)?;
                    return Some((node.size, node.alloc));
                }
            }
        }
        None
    }

    pub fn cleanup_suggestions(&self, scan_id: &str, opts: &CleanupOptions) -> Result<Vec<Suggestion>, StorageError> {
        let t = self.tree(scan_id)?;
        let t = t.read();
        let now = chrono::Utc::now().timestamp() as u32;
        Ok(cleanup::suggest(&t, &self.tokens, opts, now))
    }

    /// Delete paths (Recycle Bin unless `permanent`) and update every tree
    /// that contains them.
    pub fn delete(&self, paths: &[String], permanent: bool) -> Vec<DeleteResult> {
        let results = cleanup::delete_paths(paths, permanent, &self.tokens);
        let trees = self.trees.read();
        for r in results.iter().filter(|r| r.ok) {
            for t in trees.values() {
                let mut t = t.write();
                if let Some(id) = t.find_path(&r.path) {
                    t.mark_deleted(id);
                }
            }
        }
        drop(trees);
        let freed = results.iter().filter(|r| r.ok).count();
        self.events.emit("storage:deleted", serde_json::json!({ "count": freed, "results": &results }));
        results
    }

    /// Empty the Recycle Bin of every drive.
    pub fn empty_recycle_bin(&self) -> Result<(), String> {
        #[cfg(windows)]
        unsafe {
            use windows::Win32::UI::Shell::{SHEmptyRecycleBinW, SHERB_NOCONFIRMATION, SHERB_NOPROGRESSUI, SHERB_NOSOUND};
            SHEmptyRecycleBinW(None, None, SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND).map_err(|e| e.message())?;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let items = trash::os_limited::list().map_err(|e| e.to_string())?;
            trash::os_limited::purge_all(items).map_err(|e| e.to_string())
        }
    }

    pub fn start_duplicates(self: &Arc<Self>, scan_id: &str, opts: DupeOptions) -> Result<String, StorageError> {
        let tree = self.tree(scan_id)?;
        let id = uuid::Uuid::new_v4().to_string();
        let job = Arc::new(DupeJob { id: id.clone(), progress: DupeProgress::default(), cancel: AtomicBool::new(false), result: Mutex::new(None) });
        self.dupe_jobs.lock().insert(id.clone(), job.clone());
        let events = self.events.clone();
        std::thread::spawn(move || {
            let groups = {
                let t = tree.read();
                dupes::find_duplicates(&t, &opts, &job.progress, &job.cancel)
            };
            let wasted: u64 = groups.iter().map(|g| g.wasted).sum();
            let count = groups.len();
            *job.result.lock() = Some(groups);
            events.emit("storage:dupes-done", serde_json::json!({ "jobId": job.id, "groups": count, "wasted": wasted }));
        });
        Ok(id)
    }

    pub fn duplicates_progress(&self, job_id: &str) -> Option<DupeJobProgress> {
        let j = self.dupe_jobs.lock().get(job_id)?.clone();
        let done = j.result.lock().is_some();
        let p = &j.progress;
        Some(DupeJobProgress {
            job_id: j.id.clone(),
            phase: p.phase.load(Ordering::Relaxed),
            files_done: p.files_done.load(Ordering::Relaxed),
            files_total: p.files_total.load(Ordering::Relaxed),
            bytes_done: p.bytes_done.load(Ordering::Relaxed),
            bytes_total: p.bytes_total.load(Ordering::Relaxed),
            done,
        })
    }

    pub fn duplicates_result(&self, job_id: &str) -> Option<Vec<DupeGroup>> {
        self.dupe_jobs.lock().get(job_id)?.result.lock().clone()
    }

    pub fn cancel_duplicates(&self, job_id: &str) {
        if let Some(j) = self.dupe_jobs.lock().get(job_id) {
            j.cancel.store(true, Ordering::Relaxed);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Crumb {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildrenPage {
    pub node: NodeView,
    pub path: String,
    pub breadcrumbs: Vec<Crumb>,
    pub items: Vec<NodeView>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathedNode {
    #[serde(flatten)]
    pub node: NodeView,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub items: Vec<PathedNode>,
    pub total: usize,
    pub took_ms: u64,
}

/// Scan an NTFS disk image file as if it were a volume.
pub fn scan_image(image: &Path, root: &str, progress: &MftProgress, cancel: &AtomicBool) -> Result<ScanTree, super::ntfs::mft::MftError> {
    use super::ntfs::mft::MftVolume;
    use super::snapshot::SnapshotMeta;
    let started = Instant::now();
    let vol = MftVolume::open(std::fs::File::open(image)?)?;
    let slots = vol.scan(progress, cancel)?;
    let snap = VolumeSnapshot {
        meta: SnapshotMeta {
            root_path: if root.is_empty() { "X:\\".into() } else { root.to_string() },
            volume_serial: vol.boot.serial,
            cluster_size: vol.boot.cluster_size,
            record_size: vol.boot.record_size,
            volume_total: Some(vol.boot.volume_size()),
            volume_free: None,
            journal: None,
            scanned_at: chrono::Utc::now().timestamp(),
            duration_ms: started.elapsed().as_millis() as u64,
            method: ScanMethod::Mft,
            records_changed: 0,
        },
        slots,
    };
    let mut tree = snap.to_tree(false);
    tree.info.duration_ms = started.elapsed().as_millis() as u64;
    Ok(tree)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoElevation;
    impl ElevatedRunner for NoElevation {
        fn run(&self, _args: &[String]) -> std::io::Result<i32> {
            Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "administrator approval was declined"))
        }
    }

    fn wait(engine: &StorageEngine, job: &str) -> JobProgress {
        for _ in 0..500 {
            let p = engine.progress(job).unwrap();
            if p.state != JobState::Running {
                return p;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("scan did not finish");
    }

    #[test]
    fn walk_scan_job_end_to_end() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        std::fs::create_dir_all(data.join("sub")).unwrap();
        std::fs::write(data.join("sub/a.bin"), vec![0u8; 10_000]).unwrap();
        std::fs::write(data.join("b.txt"), b"hello").unwrap();
        std::fs::write(data.join("c.bin"), vec![0u8; 10_000]).unwrap();

        let events = EventBus::new();
        let mut rx = events.subscribe();
        let engine = StorageEngine::new(events, dir.path().join("scans"), Arc::new(NoElevation));
        let root = data.to_string_lossy().to_string();
        let job = engine.start_scan(ScanRequest { root: root.clone(), mode: ScanMode::Fast, ..Default::default() });
        let p = wait(&engine, &job);
        assert_eq!(p.state, JobState::Done, "{p:?}");
        assert_eq!(p.method, "walk");
        let scan_id = p.scan_id.unwrap();

        let s = engine.summary(&scan_id).unwrap();
        assert_eq!(s.size, 20_005);
        assert_eq!(s.files, 3);
        let page = engine.children(&scan_id, 0, SortKey::Size, true, 0, 10, true).unwrap();
        assert_eq!(page.total, 3);
        assert_eq!(page.items[0].size, 10_000);
        let found = engine.search(&scan_id, &SearchQuery { text: "*.bin".into(), ..Default::default() }).unwrap();
        assert_eq!(found.total, 2);
        assert!(found.items[0].path.ends_with(".bin"));
        assert_eq!(engine.size_of_path(&data.join("sub").to_string_lossy()).map(|s| s.0), Some(10_000));

        let dj = engine.start_duplicates(&scan_id, DupeOptions { min_size: 1, ..Default::default() }).unwrap();
        for _ in 0..500 {
            if engine.duplicates_progress(&dj).unwrap().done {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let groups = engine.duplicates_result(&dj).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].files.len(), 2);

        let res = engine.delete(&[data.join("c.bin").to_string_lossy().to_string()], true);
        assert!(res[0].ok, "{res:?}");
        assert_eq!(engine.summary(&scan_id).unwrap().size, 10_005);

        let mut topics = Vec::new();
        while let Ok(e) = rx.try_recv() {
            topics.push(e.topic);
        }
        assert!(topics.iter().any(|t| t == "storage:done"));
        assert!(topics.iter().any(|t| t == "storage:deleted"));
    }

    #[test]
    fn image_scan_job() {
        let Some(dir) = std::env::var_os("OMNIHUB_NTFS_FIXTURES") else { return };
        let img = PathBuf::from(dir).join("basic.img");
        let tmp = tempfile::tempdir().unwrap();
        let engine = StorageEngine::new(EventBus::new(), tmp.path().to_path_buf(), Arc::new(NoElevation));
        let job = engine.start_scan(ScanRequest { root: "X:\\".into(), image: Some(img.to_string_lossy().into()), ..Default::default() });
        let p = wait(&engine, &job);
        assert_eq!(p.state, JobState::Done, "{p:?}");
        let s = engine.summary("X:\\").unwrap();
        assert_eq!(s.method, ScanMethod::Mft);
        assert!(s.files > 400);
    }
}
