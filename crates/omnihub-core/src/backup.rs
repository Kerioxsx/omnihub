//! Backups of chosen folders to another drive (a USB disk, a second drive,
//! a network share): new and changed files are copied into
//! `<destination>\OmniHub Backups\<backup name>\<folder name>\…`, daily,
//! weekly, or whenever the drive is plugged in. Nothing is ever deleted in
//! the backup, so a file deleted or broken on the PC can still be found
//! there.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Schedule {
    /// Only when asked.
    Manual,
    #[default]
    Daily,
    Weekly,
    /// Whenever the destination shows up (a drive plugged in), at most every 6 hours.
    WhenConnected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct BackupJob {
    pub id: String,
    pub name: String,
    pub sources: Vec<String>,
    pub dest: String,
    pub schedule: Schedule,
    pub enabled: bool,
    /// Unix seconds.
    pub last_run: Option<i64>,
    pub last_result: Option<String>,
    pub last_ok: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BackupRun {
    pub job: String,
    pub name: String,
    pub copied: usize,
    pub bytes: u64,
    /// Already up to date.
    pub same: usize,
    /// Couldn't be read (in use, no permission).
    pub failed: usize,
    pub missing_dest: bool,
}

/// Where a job's copy of `source` goes.
pub fn target_dir(job: &BackupJob, source: &Path) -> PathBuf {
    let folder = source.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| source.to_string_lossy().replace([':', '\\', '/'], "_"));
    Path::new(&job.dest).join("OmniHub Backups").join(clean_name(&job.name)).join(folder)
}

fn clean_name(s: &str) -> String {
    let n: String = s.chars().map(|c| if r#"<>:"/\|?*"#.contains(c) || c.is_control() { '_' } else { c }).collect();
    let n = n.trim().trim_end_matches('.').to_string();
    if n.is_empty() { "Backup".into() } else { n }
}

/// Is a run due now (`dest_ok`: the destination is there)?
pub fn due(job: &BackupJob, now: i64, dest_ok: bool) -> bool {
    if !job.enabled || !dest_ok {
        return false;
    }
    let since = job.last_run.map_or(i64::MAX, |t| now - t);
    match job.schedule {
        Schedule::Manual => false,
        Schedule::Daily => since >= 20 * 3600,
        Schedule::Weekly => since >= 6 * 86_400 + 12 * 3600,
        Schedule::WhenConnected => since >= 6 * 3600,
    }
}

fn mtime(m: &std::fs::Metadata) -> std::time::SystemTime {
    m.modified().unwrap_or(std::time::UNIX_EPOCH)
}

/// Copy new and changed files from `src` into `dst` (never deleting).
pub fn mirror(src: &Path, dst: &Path, stop: &AtomicBool, run: &mut BackupRun) {
    for e in walkdir::WalkDir::new(src).follow_links(false).into_iter() {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let Ok(e) = e else {
            run.failed += 1;
            continue;
        };
        if !e.file_type().is_file() {
            continue;
        }
        let Ok(rel) = e.path().strip_prefix(src) else { continue };
        let target = dst.join(rel);
        let Ok(meta) = e.metadata() else {
            run.failed += 1;
            continue;
        };
        if let Ok(t) = std::fs::metadata(&target) {
            if t.len() == meta.len() && mtime(&t) >= mtime(&meta) {
                run.same += 1;
                continue;
            }
        }
        if let Some(parent) = target.parent() {
            if std::fs::create_dir_all(parent).is_err() {
                run.failed += 1;
                continue;
            }
        }
        // Into a temporary name first, so a cut-off copy never replaces a good one.
        let tmp = target.with_extension(format!("{}omnihub-part", target.extension().map(|x| format!("{}.", x.to_string_lossy())).unwrap_or_default()));
        match std::fs::copy(e.path(), &tmp).and_then(|n| std::fs::rename(&tmp, &target).map(|_| n)) {
            Ok(n) => {
                run.copied += 1;
                run.bytes += n;
            }
            Err(_) => {
                let _ = std::fs::remove_file(&tmp);
                run.failed += 1;
            }
        }
    }
}

/// Problems with a job before it is saved.
pub fn check(job: &BackupJob) -> Result<(), String> {
    if job.name.trim().is_empty() {
        return Err("Give the backup a name".into());
    }
    if job.sources.is_empty() {
        return Err("Pick at least one folder to back up".into());
    }
    if job.dest.trim().is_empty() {
        return Err("Pick where the backup goes (a drive or folder)".into());
    }
    let dest = Path::new(&job.dest);
    for s in &job.sources {
        let src = Path::new(s);
        if !src.is_dir() {
            return Err(format!("{s} isn't a folder"));
        }
        if dest.starts_with(src) {
            return Err("The backup can't go inside a folder it backs up".into());
        }
        if src.starts_with(dest.join("OmniHub Backups")) {
            return Err("That folder is itself a backup".into());
        }
    }
    Ok(())
}

pub struct Backups {
    file: PathBuf,
    jobs: Mutex<Vec<BackupJob>>,
    running: Mutex<Option<String>>,
    stop: AtomicBool,
    events: EventBus,
}

impl Backups {
    pub fn new(data: &Path, events: EventBus) -> Backups {
        let file = data.join("backups.json");
        let jobs = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Backups { file, jobs: Mutex::new(jobs), running: Mutex::new(None), stop: AtomicBool::new(false), events }
    }

    pub fn list(&self) -> Vec<BackupJob> {
        self.jobs.lock().clone()
    }

    pub fn running(&self) -> Option<String> {
        self.running.lock().clone()
    }

    fn write(&self, jobs: &[BackupJob]) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(jobs).map_err(|e| e.to_string())?;
        crate::settings::write_atomic(&self.file, &json).map_err(|e| e.to_string())?;
        self.events.emit("backup:changed", ());
        Ok(())
    }

    pub fn save(&self, mut job: BackupJob) -> Result<BackupJob, String> {
        job.name = job.name.trim().chars().take(60).collect();
        job.sources.retain(|s| !s.trim().is_empty());
        job.sources.dedup();
        check(&job)?;
        let mut jobs = self.jobs.lock();
        if job.id.is_empty() {
            job.id = uuid::Uuid::new_v4().simple().to_string()[..12].to_string();
            job.last_run = None;
            job.last_result = None;
            jobs.push(job.clone());
        } else {
            let slot = jobs.iter_mut().find(|j| j.id == job.id).ok_or("That backup is gone")?;
            job.last_run = slot.last_run;
            job.last_result = slot.last_result.clone();
            job.last_ok = slot.last_ok;
            *slot = job.clone();
        }
        self.write(&jobs)?;
        Ok(job)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut jobs = self.jobs.lock();
        jobs.retain(|j| j.id != id);
        self.write(&jobs)
    }

    pub fn cancel(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// Run one backup now (blocking).
    pub fn run(&self, id: &str, now: i64) -> Result<BackupRun, String> {
        let job = self.jobs.lock().iter().find(|j| j.id == id).cloned().ok_or("That backup is gone")?;
        {
            let mut r = self.running.lock();
            if r.is_some() {
                return Err("A backup is already running".into());
            }
            *r = Some(job.id.clone());
        }
        self.stop.store(false, Ordering::Relaxed);
        self.events.emit("backup:changed", ());
        let mut run = BackupRun { job: job.id.clone(), name: job.name.clone(), ..Default::default() };
        if !Path::new(&job.dest).is_dir() {
            run.missing_dest = true;
        } else {
            for s in &job.sources {
                let src = Path::new(s);
                if src.is_dir() {
                    mirror(src, &target_dir(&job, src), &self.stop, &mut run);
                } else {
                    run.failed += 1;
                }
            }
        }
        let stopped = self.stop.load(Ordering::Relaxed);
        let ok = !run.missing_dest && run.failed == 0 && !stopped;
        let result = if run.missing_dest {
            "The destination isn't there (is the drive plugged in?)".to_string()
        } else if stopped {
            format!("Stopped after {} files", run.copied)
        } else {
            format!("{} copied, {} up to date{}", run.copied, run.same, if run.failed > 0 { format!(", {} couldn't be read", run.failed) } else { String::new() })
        };
        {
            let mut jobs = self.jobs.lock();
            if let Some(j) = jobs.iter_mut().find(|j| j.id == job.id) {
                if !run.missing_dest {
                    j.last_run = Some(now);
                }
                j.last_result = Some(result);
                j.last_ok = ok;
            }
            let _ = self.write(&jobs);
        }
        *self.running.lock() = None;
        self.events.emit("backup:done", &run);
        Ok(run)
    }

    /// Run whatever is due (for the scheduler).
    pub fn run_due(&self, now: i64) {
        let due: Vec<String> = self.jobs.lock().iter().filter(|j| due(j, now, Path::new(&j.dest).is_dir())).map(|j| j.id.clone()).collect();
        for id in due {
            let _ = self.run(&id, now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_new_and_changed_files_and_never_deletes() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("Documents");
        std::fs::create_dir_all(src.join("work")).unwrap();
        std::fs::write(src.join("a.txt"), b"one").unwrap();
        std::fs::write(src.join("work/b.txt"), b"two").unwrap();
        let drive = t.path().join("USB");
        std::fs::create_dir_all(&drive).unwrap();
        let b = Backups::new(t.path(), EventBus::new());
        let job = b.save(BackupJob { name: "My stuff".into(), sources: vec![src.to_string_lossy().into()], dest: drive.to_string_lossy().into(), enabled: true, ..Default::default() }).unwrap();
        let r = b.run(&job.id, 1000).unwrap();
        assert_eq!((r.copied, r.same, r.failed), (2, 0, 0));
        let copy = drive.join("OmniHub Backups/My stuff/Documents");
        assert_eq!(std::fs::read(copy.join("work/b.txt")).unwrap(), b"two");

        let r = b.run(&job.id, 2000).unwrap();
        assert_eq!((r.copied, r.same), (0, 2), "unchanged files aren't copied again");

        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(src.join("a.txt"), b"one, changed").unwrap();
        std::fs::remove_file(src.join("work/b.txt")).unwrap();
        let r = b.run(&job.id, 3000).unwrap();
        assert_eq!(r.copied, 1);
        assert_eq!(std::fs::read(copy.join("a.txt")).unwrap(), b"one, changed");
        assert!(copy.join("work/b.txt").exists(), "a file deleted on the PC stays in the backup");
        assert_eq!(b.list()[0].last_run, Some(3000));
        assert!(b.list()[0].last_ok);
    }

    #[test]
    fn checks_and_schedules() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("Pics");
        std::fs::create_dir_all(&src).unwrap();
        let b = Backups::new(t.path(), EventBus::new());
        let inside = BackupJob { name: "x".into(), sources: vec![src.to_string_lossy().into()], dest: src.join("bak").to_string_lossy().into(), ..Default::default() };
        assert!(b.save(inside).is_err(), "not into the folder it backs up");
        assert!(b.save(BackupJob { name: "x".into(), dest: "D:\\".into(), ..Default::default() }).is_err());

        let mut j = BackupJob { enabled: true, schedule: Schedule::Daily, ..Default::default() };
        assert!(due(&j, 100, true));
        assert!(!due(&j, 100, false), "not while the drive is missing");
        j.last_run = Some(0);
        assert!(!due(&j, 3600, true));
        assert!(due(&j, 21 * 3600, true));
        j.schedule = Schedule::Manual;
        assert!(!due(&j, 99 * 86_400, true));

        let gone = b.save(BackupJob { name: "Away".into(), sources: vec![src.to_string_lossy().into()], dest: t.path().join("Drive").to_string_lossy().into(), ..Default::default() }).unwrap();
        let r = b.run(&gone.id, 50).unwrap();
        assert!(r.missing_dest);
        assert_eq!(b.list().iter().find(|x| x.id == gone.id).unwrap().last_run, None, "a missed run doesn't count");
    }
}
