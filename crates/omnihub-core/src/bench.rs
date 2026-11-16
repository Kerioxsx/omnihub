//! A repeatable benchmark: the same work every time (SHA-256 over a fixed
//! buffer on one core and on every core, and a large memory copy), timed,
//! so a score can be compared with last week's. A drop with the same parts
//! usually means heat, a background program or a power setting.

use std::path::{Path, PathBuf};
use std::time::Instant;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Bytes hashed per core.
const HASH_BYTES: usize = 192 << 20;
/// Bytes copied for memory (each pass).
const COPY_BYTES: usize = 512 << 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BenchResult {
    pub at: i64,
    /// MB/s of SHA-256 on one core.
    pub single: f64,
    /// MB/s on every core together.
    pub multi: f64,
    /// GB/s of memory copy.
    pub memory: f64,
    pub threads: usize,
    /// Hottest processor temperature during the run.
    pub cpu_temp: Option<f32>,
}

/// Percent change from `before` to `now` (positive is faster).
pub fn change(before: f64, now: f64) -> f64 {
    if before <= 0.0 {
        0.0
    } else {
        (now - before) / before * 100.0
    }
}

/// One line comparing a run with the one before.
pub fn compare(prev: &BenchResult, now: &BenchResult) -> String {
    let parts = [("one core", change(prev.single, now.single)), ("all cores", change(prev.multi, now.multi)), ("memory", change(prev.memory, now.memory))];
    let slower: Vec<String> = parts.iter().filter(|(_, c)| *c <= -5.0).map(|(n, c)| format!("{n} {:.0}% slower", -c)).collect();
    if slower.is_empty() {
        "Within a few percent of last time.".into()
    } else {
        format!("{} than last time: check temperatures, background programs and the power plan.", slower.join(", "))
    }
}

fn hash_mb_s(bytes: usize, threads: usize) -> f64 {
    let chunk = vec![0x5Au8; 1 << 20];
    let t = Instant::now();
    let work = |_| {
        let mut h = Sha256::new();
        for _ in 0..bytes / chunk.len() {
            h.update(&chunk);
        }
        std::hint::black_box(h.finalize());
    };
    if threads <= 1 {
        work(0);
    } else {
        use rayon::prelude::*;
        let pool = rayon::ThreadPoolBuilder::new().num_threads(threads).build();
        match pool {
            Ok(p) => p.install(|| (0..threads).into_par_iter().for_each(work)),
            Err(_) => (0..threads).for_each(work),
        }
    }
    let secs = t.elapsed().as_secs_f64().max(1e-6);
    (bytes * threads.max(1)) as f64 / (1 << 20) as f64 / secs
}

fn copy_gb_s(bytes: usize) -> f64 {
    let src = vec![1u8; bytes];
    let mut dst = vec![0u8; bytes];
    let passes = 4;
    let t = Instant::now();
    for _ in 0..passes {
        dst.copy_from_slice(&src);
        std::hint::black_box(&dst);
    }
    let secs = t.elapsed().as_secs_f64().max(1e-6);
    (bytes * passes) as f64 / 1e9 / secs
}

/// Run the benchmark (about 10–30 seconds). `temp` reads the processor's
/// temperature between parts; `progress` gets 0–100.
pub fn run(temp: &dyn Fn() -> Option<f32>, progress: &dyn Fn(u32)) -> BenchResult {
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let mut hottest: Option<f32> = None;
    let mut see = || {
        if let Some(t) = temp() {
            hottest = Some(hottest.map_or(t, |h| h.max(t)));
        }
    };
    progress(5);
    let single = hash_mb_s(HASH_BYTES, 1);
    see();
    progress(35);
    let multi = hash_mb_s(HASH_BYTES / 2, threads);
    see();
    progress(75);
    let memory = copy_gb_s(COPY_BYTES);
    see();
    progress(100);
    BenchResult { at: crate::db::now(), single: (single * 10.0).round() / 10.0, multi: multi.round(), memory: (memory * 100.0).round() / 100.0, threads, cpu_temp: hottest }
}

pub struct Bench {
    file: PathBuf,
    results: Mutex<Vec<BenchResult>>,
}

impl Bench {
    pub fn new(data: &Path) -> Bench {
        let file = data.join("benchmarks.json");
        let results = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Bench { file, results: Mutex::new(results) }
    }

    pub fn results(&self) -> Vec<BenchResult> {
        self.results.lock().clone()
    }

    pub fn add(&self, r: BenchResult) -> Vec<BenchResult> {
        let mut l = self.results.lock();
        l.push(r);
        let extra = l.len().saturating_sub(50);
        l.drain(..extra);
        let _ = crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&*l).unwrap_or_default());
        l.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_runs() {
        let a = BenchResult { single: 1000.0, multi: 8000.0, memory: 20.0, ..Default::default() };
        let b = BenchResult { single: 990.0, multi: 6800.0, memory: 20.5, ..Default::default() };
        assert!((change(8000.0, 6800.0) + 15.0).abs() < 0.01);
        assert_eq!(compare(&a, &b), "all cores 15% slower than last time: check temperatures, background programs and the power plan.");
        assert_eq!(compare(&a, &a), "Within a few percent of last time.");
        assert!(hash_mb_s(4 << 20, 2) > 0.0 && copy_gb_s(1 << 20) > 0.0);
        let t = tempfile::tempdir().unwrap();
        let bench = Bench::new(t.path());
        bench.add(a);
        assert_eq!(Bench::new(t.path()).results().len(), 1);
    }
}
