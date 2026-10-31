//! Folder rules: "when a PDF lands in Downloads, move it to Documents\PDFs",
//! "copy photos from the phone folder to Pictures", "send installers older
//! than 30 days to the Recycle Bin". Folders are looked at every half
//! minute; a file is only touched once it has stopped changing (a download
//! still being written is left alone), never in subfolders, and anything
//! deleted goes to the Recycle Bin.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Action {
    /// Move into this folder.
    Move { to: String },
    /// Copy into this folder (the file stays).
    Copy { to: String },
    /// To the Recycle Bin once older than this many days.
    Recycle { days: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub folder: String,
    /// Extensions without the dot ("pdf", "jpg"); empty = any.
    pub extensions: Vec<String>,
    /// Words the name must contain (any of them); empty = any.
    pub contains: Vec<String>,
    pub action: Action,
}

impl Default for Rule {
    fn default() -> Self {
        Rule { id: String::new(), name: String::new(), enabled: true, folder: String::new(), extensions: Vec::new(), contains: Vec::new(), action: Action::Move { to: String::new() } }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Done {
    pub at: i64,
    pub rule: String,
    pub file: String,
    /// "Moved to …", "Copied to …", "Sent to the Recycle Bin", or what went wrong.
    pub what: String,
    pub ok: bool,
}

/// Names of files still being written by a browser or a download tool.
const PARTIAL: &[&str] = &["crdownload", "part", "partial", "download", "tmp", "opdownload", "!ut", "aria2"];

/// Whether the rule applies to this file name.
pub fn matches(rule: &Rule, name: &str) -> bool {
    let lower = name.to_lowercase();
    let ext = Path::new(&lower).extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    if PARTIAL.contains(&ext.as_str()) || lower.starts_with("~$") || lower == "desktop.ini" || lower == "thumbs.db" {
        return false;
    }
    let ext_ok = rule.extensions.is_empty() || rule.extensions.iter().any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(&ext));
    let words_ok = rule.contains.is_empty() || rule.contains.iter().filter(|w| !w.trim().is_empty()).any(|w| lower.contains(&w.trim().to_lowercase()));
    ext_ok && words_ok
}

/// Problems with a rule before it is saved.
pub fn check(rule: &Rule) -> Result<(), String> {
    if rule.name.trim().is_empty() {
        return Err("Give the rule a name.".into());
    }
    let folder = Path::new(&rule.folder);
    if !folder.is_dir() {
        return Err("Pick the folder to watch.".into());
    }
    // As picked (a canonical path's \\?\ prefix would not match the protected folders).
    if let Some(why) = crate::storage::cleanup::refusal_reason(&folder.join("file"), &crate::storage::cleanup::default_tokens()) {
        return Err(format!("OmniHub won't touch files there: {why}."));
    }
    match &rule.action {
        Action::Move { to } | Action::Copy { to } => {
            if to.trim().is_empty() {
                return Err("Pick where the files go.".into());
            }
            if same(Path::new(to), folder) {
                return Err("Pick a different folder to send the files to.".into());
            }
            if let Some(why) = crate::storage::cleanup::refusal_reason(&Path::new(to).join("file"), &crate::storage::cleanup::default_tokens()) {
                return Err(format!("OmniHub won't put files there: {why}."));
            }
        }
        Action::Recycle { days } => {
            if *days == 0 {
                return Err("Pick how many days old.".into());
            }
            if rule.extensions.is_empty() && rule.contains.is_empty() {
                return Err("A rule that sends files to the Recycle Bin needs file types or words to look for, so it doesn't take everything.".into());
            }
        }
    }
    Ok(())
}

fn same(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => a == b,
    }
}

/// A free name in `dir` for `name` ("name (2).ext" when taken).
fn free_path(dir: &Path, name: &str) -> PathBuf {
    crate::remote::guest::unique_path(dir, name)
}

/// Do the rule's action to one file; returns what happened.
pub fn apply(rule: &Rule, file: &Path) -> Result<String, String> {
    let name = file.file_name().map(|n| n.to_string_lossy().to_string()).ok_or("no name")?;
    match &rule.action {
        Action::Move { to } | Action::Copy { to } => {
            let dir = Path::new(to);
            std::fs::create_dir_all(dir).map_err(|e| format!("couldn't make {}: {e}", dir.display()))?;
            let dest = free_path(dir, &name);
            if matches!(rule.action, Action::Move { .. }) {
                if std::fs::rename(file, &dest).is_err() {
                    // Another drive: copy, then remove the original.
                    std::fs::copy(file, &dest).map_err(|e| e.to_string())?;
                    std::fs::remove_file(file).map_err(|e| e.to_string())?;
                }
                Ok(format!("Moved to {}", dir.display()))
            } else {
                std::fs::copy(file, &dest).map_err(|e| e.to_string())?;
                Ok(format!("Copied to {}", dir.display()))
            }
        }
        Action::Recycle { .. } => {
            trash::delete(file).map_err(|e| e.to_string())?;
            Ok("Sent to the Recycle Bin".into())
        }
    }
}

/// What a file looked like at the last look (size, modified).
type Seen = HashMap<PathBuf, (u64, SystemTime)>;

pub struct Rules {
    file: PathBuf,
    list: Mutex<Vec<Rule>>,
    seen: Mutex<Seen>,
    /// Files a copy rule already copied (so they aren't copied again each look).
    copied: Mutex<HashMap<String, std::collections::HashSet<PathBuf>>>,
    log: Mutex<Vec<Done>>,
}

impl Rules {
    pub fn new(data_dir: &Path) -> Rules {
        let file = data_dir.join("rules.json");
        let list = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Rules { file, list: Mutex::new(list), seen: Mutex::new(HashMap::new()), copied: Mutex::new(HashMap::new()), log: Mutex::new(Vec::new()) }
    }

    pub fn list(&self) -> Vec<Rule> {
        self.list.lock().clone()
    }

    pub fn log(&self) -> Vec<Done> {
        self.log.lock().iter().rev().cloned().collect()
    }

    fn write(&self, list: &[Rule]) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(list).map_err(|e| e.to_string())?;
        crate::settings::write_atomic(&self.file, &json).map_err(|e| e.to_string())
    }

    pub fn save(&self, mut rule: Rule) -> Result<Rule, String> {
        check(&rule)?;
        rule.name = rule.name.trim().chars().take(60).collect();
        rule.extensions = rule.extensions.iter().map(|e| e.trim().trim_start_matches('.').to_lowercase()).filter(|e| !e.is_empty()).collect();
        rule.contains = rule.contains.iter().map(|w| w.trim().to_string()).filter(|w| !w.is_empty()).collect();
        if rule.id.is_empty() {
            rule.id = uuid::Uuid::new_v4().to_string();
        }
        let mut list = self.list.lock();
        match list.iter_mut().find(|r| r.id == rule.id) {
            Some(r) => *r = rule.clone(),
            None => list.push(rule.clone()),
        }
        self.write(&list)?;
        Ok(rule)
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut list = self.list.lock();
        list.retain(|r| r.id != id);
        self.copied.lock().remove(id);
        self.write(&list)
    }

    /// Look at every watched folder once; returns what was done.
    pub fn run_once(&self, now: SystemTime) -> Vec<Done> {
        let rules: Vec<Rule> = self.list.lock().iter().filter(|r| r.enabled).cloned().collect();
        let mut done = Vec::new();
        let mut seen = self.seen.lock();
        let mut next: Seen = HashMap::new();
        for rule in &rules {
            let Ok(entries) = std::fs::read_dir(&rule.folder) else { continue };
            for e in entries.flatten() {
                let path = e.path();
                let Ok(meta) = e.metadata() else { continue };
                if !meta.is_file() {
                    continue;
                }
                let name = e.file_name().to_string_lossy().to_string();
                if !matches(rule, &name) {
                    continue;
                }
                let modified = meta.modified().unwrap_or(now);
                let state = (meta.len(), modified);
                next.insert(path.clone(), state);
                // Unchanged since the last look, and not touched in the last 10 s.
                let settled = seen.get(&path) == Some(&state) && now.duration_since(modified).unwrap_or_default() >= Duration::from_secs(10);
                if !settled {
                    continue;
                }
                let due = match &rule.action {
                    Action::Recycle { days } => now.duration_since(modified).unwrap_or_default() >= Duration::from_secs(*days as u64 * 86_400),
                    Action::Copy { .. } => !self.copied.lock().get(&rule.id).is_some_and(|s| s.contains(&path)),
                    Action::Move { .. } => true,
                };
                if !due {
                    continue;
                }
                let r = apply(rule, &path);
                if r.is_ok() && matches!(rule.action, Action::Copy { .. }) {
                    self.copied.lock().entry(rule.id.clone()).or_default().insert(path.clone());
                }
                done.push(Done { at: chrono::Utc::now().timestamp(), rule: rule.name.clone(), file: name, ok: r.is_ok(), what: r.unwrap_or_else(|e| format!("Couldn't: {e}")) });
            }
        }
        *seen = next;
        if !done.is_empty() {
            let mut log = self.log.lock();
            log.extend(done.iter().cloned());
            let extra = log.len().saturating_sub(200);
            log.drain(..extra);
        }
        done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(folder: &Path, action: Action) -> Rule {
        Rule { id: String::new(), name: "PDFs".into(), enabled: true, folder: folder.to_string_lossy().into(), extensions: vec!["PDF".into()], contains: vec![], action }
    }

    #[test]
    fn matching() {
        let r = Rule { extensions: vec!["pdf".into(), ".jpg".into()], contains: vec!["invoice".into()], ..Default::default() };
        assert!(matches(&r, "Invoice March.PDF"));
        assert!(!matches(&r, "report.pdf"));
        assert!(!matches(&r, "invoice.pdf.crdownload"), "still downloading");
        let any = Rule::default();
        assert!(matches(&any, "anything.zip"));
        assert!(!matches(&any, "~$draft.docx"));
    }

    #[test]
    fn moves_settled_files_only() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("Downloads");
        let to = dir.path().join("PDFs");
        std::fs::create_dir_all(&from).unwrap();
        let rules = Rules::new(dir.path());
        assert!(rules.save(Rule { name: " ".into(), ..rule(&from, Action::Move { to: to.to_string_lossy().into() }) }).is_err());
        assert!(rules.save(rule(&from, Action::Move { to: from.to_string_lossy().into() })).is_err(), "not into itself");
        let saved = rules.save(rule(&from, Action::Move { to: to.to_string_lossy().into() })).unwrap();
        assert_eq!(saved.extensions, vec!["pdf"]);
        std::fs::write(from.join("a.pdf"), "pdf").unwrap();
        std::fs::write(from.join("b.txt"), "txt").unwrap();
        let later = SystemTime::now() + Duration::from_secs(60);
        // First look: just noted. Second look, unchanged: moved.
        assert!(rules.run_once(later).is_empty());
        let done = rules.run_once(later);
        assert_eq!(done.len(), 1, "{done:?}");
        assert!(done[0].ok);
        assert!(to.join("a.pdf").is_file() && !from.join("a.pdf").exists());
        assert!(from.join("b.txt").exists());
        // A second file with the same name doesn't overwrite.
        std::fs::write(from.join("a.pdf"), "pdf2").unwrap();
        rules.run_once(later);
        rules.run_once(later);
        assert!(to.join("a (2).pdf").is_file());
        assert_eq!(rules.log().len(), 2);
        assert_eq!(Rules::new(dir.path()).list().len(), 1, "saved");
    }

    #[test]
    fn copies_once_and_recycle_needs_a_filter() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("in");
        let to = dir.path().join("out");
        std::fs::create_dir_all(&from).unwrap();
        let rules = Rules::new(dir.path());
        rules.save(rule(&from, Action::Copy { to: to.to_string_lossy().into() })).unwrap();
        std::fs::write(from.join("x.pdf"), "1").unwrap();
        let later = SystemTime::now() + Duration::from_secs(60);
        for _ in 0..4 {
            rules.run_once(later);
        }
        assert!(from.join("x.pdf").exists());
        assert_eq!(std::fs::read_dir(&to).unwrap().count(), 1, "copied once");
        let mut r = rule(&from, Action::Recycle { days: 30 });
        r.extensions.clear();
        assert!(check(&r).is_err());
    }
}
