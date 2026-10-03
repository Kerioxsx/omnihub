//! Notes and "Ideas for Claude".
//!
//! Notes live in SQLite. Ideas can be exported as Markdown files (with YAML
//! front matter, optionally a JSON sidecar) into a folder of the user's
//! choosing that Claude reads from. The folder is watched so replies Claude
//! writes there show up in the app.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use parking_lot::Mutex;
use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::db::{now, Db};
use crate::events::EventBus;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum NoteKind {
    #[default]
    Note,
    Idea,
}

impl NoteKind {
    fn as_str(self) -> &'static str {
        match self {
            NoteKind::Note => "note",
            NoteKind::Idea => "idea",
        }
    }

    fn parse(s: &str) -> Self {
        if s == "idea" { NoteKind::Idea } else { NoteKind::Note }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub kind: NoteKind,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub color: Option<String>,
    pub created: i64,
    pub updated: i64,
    pub exported_path: Option<String>,
    pub exported_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NoteInput {
    pub id: Option<String>,
    pub kind: NoteKind,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NoteFilter {
    pub kind: Option<NoteKind>,
    pub query: String,
    pub tag: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ExportOptions {
    pub sidecar_json: bool,
    pub index_file: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FolderFile {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: i64,
    pub is_dir: bool,
    /// Written by OmniHub (has our front matter) rather than by someone else.
    pub from_omnihub: bool,
    pub preview: String,
}

#[derive(Debug, thiserror::Error)]
pub enum NotesError {
    #[error("note not found")]
    NotFound,
    #[error("choose a Claude folder in Settings → Notes first")]
    NoFolder,
    #[error("that file is outside the Claude folder")]
    OutsideFolder,
    #[error(transparent)]
    Db(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct Notes {
    db: Arc<Db>,
    events: EventBus,
    watcher: Mutex<Option<(PathBuf, notify::RecommendedWatcher)>>,
}

fn row_to_note(r: &Row) -> rusqlite::Result<Note> {
    let tags: String = r.get(4)?;
    Ok(Note {
        id: r.get(0)?,
        kind: NoteKind::parse(&r.get::<_, String>(1)?),
        title: r.get(2)?,
        body: r.get(3)?,
        tags: serde_json::from_str(&tags).unwrap_or_default(),
        pinned: r.get(5)?,
        color: r.get(6)?,
        created: r.get(7)?,
        updated: r.get(8)?,
        exported_path: r.get(9)?,
        exported_at: r.get(10)?,
    })
}

const COLUMNS: &str = "id, kind, title, body, tags, pinned, color, created, updated, exported_path, exported_at";

/// File-name friendly version of a title.
pub fn slugify(title: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in title.chars() {
        if c.is_alphanumeric() {
            for l in c.to_lowercase() {
                out.push(l);
            }
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
        if out.chars().count() >= 60 {
            break;
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "untitled".into() } else { out }
}

fn yaml_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"))
}

/// The Markdown document written for a note.
pub fn render_markdown(n: &Note) -> String {
    let iso = |t: i64| chrono::DateTime::from_timestamp(t, 0).map(|d| d.to_rfc3339()).unwrap_or_default();
    let tags = n.tags.iter().map(|t| yaml_string(t)).collect::<Vec<_>>().join(", ");
    let mut s = String::new();
    s.push_str("---\n");
    s.push_str(&format!("title: {}\n", yaml_string(&n.title)));
    s.push_str(&format!("id: {}\n", n.id));
    s.push_str(&format!("kind: {}\n", n.kind.as_str()));
    s.push_str(&format!("tags: [{tags}]\n"));
    s.push_str(&format!("created: {}\n", iso(n.created)));
    s.push_str(&format!("updated: {}\n", iso(n.updated)));
    s.push_str("source: OmniHub\n");
    s.push_str("---\n\n");
    if !n.title.is_empty() && !n.body.trim_start().starts_with("# ") {
        s.push_str(&format!("# {}\n\n", n.title));
    }
    s.push_str(n.body.trim_end());
    s.push('\n');
    s
}

impl Notes {
    pub fn new(db: Arc<Db>, events: EventBus) -> Self {
        Notes { db, events, watcher: Mutex::new(None) }
    }

    pub fn list(&self, f: &NoteFilter) -> Result<Vec<Note>, NotesError> {
        let notes = self.db.with(|c| {
            let mut st = c.prepare(&format!("SELECT {COLUMNS} FROM notes ORDER BY pinned DESC, updated DESC"))?;
            let rows = st.query_map([], row_to_note)?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })?;
        let q = f.query.trim().to_lowercase();
        let tag = f.tag.as_ref().map(|t| t.to_lowercase());
        Ok(notes
            .into_iter()
            .filter(|n| f.kind.is_none_or(|k| k == n.kind))
            .filter(|n| tag.as_ref().is_none_or(|t| n.tags.iter().any(|x| x.to_lowercase() == *t)))
            .filter(|n| q.is_empty() || n.title.to_lowercase().contains(&q) || n.body.to_lowercase().contains(&q) || n.tags.iter().any(|t| t.to_lowercase().contains(&q)))
            .take(f.limit.unwrap_or(usize::MAX))
            .collect())
    }

    pub fn get(&self, id: &str) -> Result<Note, NotesError> {
        self.db
            .with(|c| c.query_row(&format!("SELECT {COLUMNS} FROM notes WHERE id = ?1"), params![id], row_to_note).optional())?
            .ok_or(NotesError::NotFound)
    }

    pub fn save(&self, input: NoteInput) -> Result<Note, NotesError> {
        let t = now();
        let tags: Vec<String> = {
            let mut v: Vec<String> = input.tags.iter().map(|t| t.trim().trim_start_matches('#').to_string()).filter(|t| !t.is_empty()).collect();
            v.dedup();
            v
        };
        let tags_json = serde_json::to_string(&tags).unwrap();
        let id = match input.id.filter(|i| !i.is_empty()) {
            Some(id) => {
                let n = self.db.with(|c| {
                    c.execute(
                        "UPDATE notes SET kind=?2, title=?3, body=?4, tags=?5, pinned=?6, color=?7, updated=?8 WHERE id=?1",
                        params![id, input.kind.as_str(), input.title, input.body, tags_json, input.pinned, input.color, t],
                    )
                })?;
                if n == 0 {
                    return Err(NotesError::NotFound);
                }
                id
            }
            None => {
                let id = uuid::Uuid::new_v4().to_string();
                self.db.with(|c| {
                    c.execute(
                        "INSERT INTO notes (id, kind, title, body, tags, pinned, color, created, updated) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?8)",
                        params![id, input.kind.as_str(), input.title, input.body, tags_json, input.pinned, input.color, t],
                    )
                })?;
                id
            }
        };
        let note = self.get(&id)?;
        self.events.emit("notes:changed", serde_json::json!({ "id": note.id }));
        Ok(note)
    }

    pub fn delete(&self, id: &str) -> Result<(), NotesError> {
        self.db.with(|c| c.execute("DELETE FROM notes WHERE id = ?1", params![id]))?;
        self.events.emit("notes:changed", serde_json::json!({ "id": id, "deleted": true }));
        Ok(())
    }

    pub fn tags(&self) -> Result<Vec<(String, usize)>, NotesError> {
        let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
        for n in self.list(&NoteFilter::default())? {
            for t in n.tags {
                *counts.entry(t).or_default() += 1;
            }
        }
        let mut v: Vec<_> = counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        Ok(v)
    }

    /// Write a note into the Claude folder. Re-exporting overwrites the same
    /// file as long as it is still inside the folder.
    pub fn export(&self, id: &str, folder: &Path, opts: &ExportOptions) -> Result<Note, NotesError> {
        let note = self.get(id)?;
        std::fs::create_dir_all(folder)?;
        let folder = std::fs::canonicalize(folder)?;
        let previous = note
            .exported_path
            .as_ref()
            .map(PathBuf::from)
            .filter(|p| p.parent().and_then(|d| std::fs::canonicalize(d).ok()).is_some_and(|d| d == folder));
        let path = previous.unwrap_or_else(|| {
            let date = chrono::DateTime::from_timestamp(note.created, 0).map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
            let base = format!("{date}-{}", slugify(&note.title));
            let mut candidate = folder.join(format!("{base}.md"));
            let mut i = 2;
            while candidate.exists() {
                candidate = folder.join(format!("{base}-{i}.md"));
                i += 1;
            }
            candidate
        });
        crate::settings::write_atomic(&path, render_markdown(&note).as_bytes())?;
        if opts.sidecar_json {
            crate::settings::write_atomic(&path.with_extension("json"), &serde_json::to_vec_pretty(&note).unwrap())?;
        }
        let t = now();
        let path_str = path.to_string_lossy().to_string();
        self.db.with(|c| c.execute("UPDATE notes SET exported_path=?2, exported_at=?3 WHERE id=?1", params![id, path_str, t]))?;
        if opts.index_file {
            self.write_index(&folder)?;
        }
        let note = self.get(id)?;
        self.events.emit("notes:exported", serde_json::json!({ "id": id, "path": path_str }));
        Ok(note)
    }

    fn write_index(&self, folder: &Path) -> Result<(), NotesError> {
        let ideas = self.list(&NoteFilter::default())?;
        let mut s = String::from("# Ideas for Claude\n\nExported from OmniHub. Newest first.\n\n");
        let mut rows: Vec<&Note> = ideas
            .iter()
            .filter(|n| n.exported_path.as_ref().is_some_and(|p| Path::new(p).parent().and_then(|d| std::fs::canonicalize(d).ok()).as_deref() == Some(folder)))
            .collect();
        rows.sort_by_key(|n| std::cmp::Reverse(n.updated));
        for n in rows {
            let file = Path::new(n.exported_path.as_ref().unwrap()).file_name().unwrap().to_string_lossy().to_string();
            let date = chrono::DateTime::from_timestamp(n.updated, 0).map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
            let tags = if n.tags.is_empty() { String::new() } else { format!(" — {}", n.tags.iter().map(|t| format!("#{t}")).collect::<Vec<_>>().join(" ")) };
            let title = if n.title.is_empty() { "Untitled" } else { &n.title };
            s.push_str(&format!("- [{title}](./{}) ({date}){tags}\n", file.replace(' ', "%20")));
        }
        crate::settings::write_atomic(&folder.join("INDEX.md"), s.as_bytes())?;
        Ok(())
    }

    /// Files in the Claude folder, newest first.
    pub fn folder_files(&self, folder: &Path) -> Result<Vec<FolderFile>, NotesError> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(folder)? {
            let e = e?;
            let meta = e.metadata()?;
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name.ends_with(".tmp") {
                continue;
            }
            let mut preview = String::new();
            let mut from_omnihub = false;
            if meta.is_file() && meta.len() < 4 << 20 && (name.ends_with(".md") || name.ends_with(".txt") || name.ends_with(".json")) {
                if let Ok(text) = std::fs::read_to_string(e.path()) {
                    from_omnihub = text.starts_with("---\n") && text.contains("\nsource: OmniHub\n");
                    let body = strip_front_matter(&text);
                    preview = body.chars().take(240).collect();
                }
            }
            out.push(FolderFile {
                name,
                path: e.path().to_string_lossy().to_string(),
                size: meta.len(),
                modified: meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64),
                is_dir: meta.is_dir(),
                from_omnihub,
                preview,
            });
        }
        out.sort_by_key(|f| std::cmp::Reverse(f.modified));
        Ok(out)
    }

    /// Read a text file from the Claude folder (refuses paths outside it).
    pub fn read_folder_file(&self, folder: &Path, path: &Path) -> Result<String, NotesError> {
        let folder = std::fs::canonicalize(folder)?;
        let path = std::fs::canonicalize(path)?;
        if !path.starts_with(&folder) {
            return Err(NotesError::OutsideFolder);
        }
        let meta = std::fs::metadata(&path)?;
        if meta.len() > 8 << 20 {
            return Err(NotesError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, "file too large to preview")));
        }
        Ok(String::from_utf8_lossy(&std::fs::read(&path)?).to_string())
    }

    /// Watch the Claude folder; emits `notes:folder-changed` (debounced).
    pub fn watch_folder(&self, folder: Option<&Path>) {
        use notify::{RecursiveMode, Watcher};
        let mut guard = self.watcher.lock();
        let Some(folder) = folder else {
            *guard = None;
            return;
        };
        if guard.as_ref().is_some_and(|(p, _)| p == folder) {
            return;
        }
        if std::fs::create_dir_all(folder).is_err() {
            *guard = None;
            return;
        }
        let events = self.events.clone();
        let last = Arc::new(Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(10)));
        let folder_str = folder.to_string_lossy().to_string();
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if res.is_ok() {
                let mut l = last.lock();
                if l.elapsed() > std::time::Duration::from_millis(300) {
                    *l = std::time::Instant::now();
                    events.emit("notes:folder-changed", serde_json::json!({ "folder": folder_str }));
                }
            }
        });
        match watcher {
            Ok(mut w) => {
                if w.watch(folder, RecursiveMode::NonRecursive).is_ok() {
                    *guard = Some((folder.to_path_buf(), w));
                }
            }
            Err(e) => tracing::warn!("cannot watch Claude folder: {e}"),
        }
    }
}

pub fn strip_front_matter(text: &str) -> &str {
    if let Some(rest) = text.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            return rest[end + 5..].trim_start();
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes() -> Notes {
        Notes::new(Arc::new(Db::in_memory().unwrap()), EventBus::new())
    }

    #[test]
    fn crud_and_filter() {
        let n = notes();
        let a = n.save(NoteInput { title: "Shopping".into(), body: "milk".into(), tags: vec!["#home".into(), " ".into()], ..Default::default() }).unwrap();
        assert_eq!(a.tags, vec!["home"]);
        let b = n.save(NoteInput { kind: NoteKind::Idea, title: "Treemap zoom".into(), body: "animate it".into(), ..Default::default() }).unwrap();
        assert_eq!(n.list(&NoteFilter::default()).unwrap().len(), 2);
        assert_eq!(n.list(&NoteFilter { kind: Some(NoteKind::Idea), ..Default::default() }).unwrap()[0].id, b.id);
        assert_eq!(n.list(&NoteFilter { query: "MILK".into(), ..Default::default() }).unwrap().len(), 1);
        assert_eq!(n.list(&NoteFilter { tag: Some("Home".into()), ..Default::default() }).unwrap().len(), 1);
        let a2 = n.save(NoteInput { id: Some(a.id.clone()), title: "Groceries".into(), pinned: true, ..Default::default() }).unwrap();
        assert_eq!(a2.title, "Groceries");
        assert_eq!(a2.created, a.created);
        assert_eq!(n.list(&NoteFilter::default()).unwrap()[0].id, a.id, "pinned first");
        n.delete(&a.id).unwrap();
        assert!(matches!(n.get(&a.id), Err(NotesError::NotFound)));
        assert!(matches!(n.save(NoteInput { id: Some("missing".into()), ..Default::default() }), Err(NotesError::NotFound)));
    }

    #[test]
    fn export_writes_markdown_and_index() {
        let n = notes();
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("claude");
        let idea = n.save(NoteInput { kind: NoteKind::Idea, title: "Faster \"scan\": MFT!".into(), body: "Use the *USN* journal.".into(), tags: vec!["perf".into()], ..Default::default() }).unwrap();
        let opts = ExportOptions { sidecar_json: true, index_file: true };
        let out = n.export(&idea.id, &folder, &opts).unwrap();
        let path = PathBuf::from(out.exported_path.clone().unwrap());
        assert!(path.file_name().unwrap().to_string_lossy().ends_with("-faster-scan-mft.md"));
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("---\ntitle: \"Faster \\\"scan\\\": MFT!\"\n"));
        assert!(text.contains("tags: [\"perf\"]"));
        assert!(text.contains("# Faster \"scan\": MFT!\n\nUse the *USN* journal.\n"));
        assert!(path.with_extension("json").exists());
        let index = std::fs::read_to_string(folder.join("INDEX.md")).unwrap();
        assert!(index.contains("Faster \"scan\": MFT!"));

        // Re-export overwrites the same file.
        n.save(NoteInput { id: Some(idea.id.clone()), kind: NoteKind::Idea, title: "Renamed".into(), body: "v2".into(), ..Default::default() }).unwrap();
        let again = n.export(&idea.id, &folder, &opts).unwrap();
        assert_eq!(again.exported_path, out.exported_path);
        assert!(std::fs::read_to_string(&path).unwrap().contains("v2"));

        std::fs::write(folder.join("reply-from-claude.md"), "Here is a plan").unwrap();
        let files = n.folder_files(&folder).unwrap();
        let reply = files.iter().find(|f| f.name == "reply-from-claude.md").unwrap();
        assert!(!reply.from_omnihub);
        assert_eq!(reply.preview, "Here is a plan");
        assert!(files.iter().find(|f| f.path == out.exported_path.clone().unwrap()).unwrap().from_omnihub);
        assert_eq!(n.read_folder_file(&folder, &folder.join("reply-from-claude.md")).unwrap(), "Here is a plan");
        let outside = dir.path().join("secret.txt");
        std::fs::write(&outside, "no").unwrap();
        assert!(matches!(n.read_folder_file(&folder, &outside), Err(NotesError::OutsideFolder)));
    }

    #[test]
    fn slugs() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
        assert_eq!(slugify("  ***  "), "untitled");
        assert_eq!(slugify("Ünïcødé idea 2"), "ünïcødé-idea-2");
    }
}
