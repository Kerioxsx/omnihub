//! Codex's conversations on this PC, read from the session files it keeps
//! (`~/.codex/sessions/YYYY/MM/DD/rollout-<time>-<id>.jsonl`, older ones
//! compressed as `.jsonl.zst`, or under `CODEX_HOME`), and continued with
//! `codex exec resume <id>`. Each line is `{"timestamp", "type", "payload"}`:
//! `session_meta` (id, cwd, git), `turn_context` (model), `response_item`
//! (what the model saw and did) and `event_msg` (what the user saw:
//! `user_message`, `agent_message`, `token_count` with usage and the plan's
//! rate limits).

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{ellipsize, ts, Builder, Message, Role};

/// The sandboxes offered from OmniHub (never `danger-full-access`).
pub const SANDBOXES: &[&str] = &["read-only", "workspace-write"];

pub fn home() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("CODEX_HOME").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    dirs::home_dir().map(|h| h.join(".codex"))
}

/// The session id in a rollout file name.
pub fn id_of(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let core = name.strip_suffix(".zst").unwrap_or(name).strip_prefix("rollout-")?.strip_suffix(".jsonl")?;
    if core.get(19..20)? != "-" {
        return None;
    }
    let ids = core.get(20..)?;
    let id = ids.split_once('_').map_or(ids, |(a, _)| a);
    uuid::Uuid::parse_str(id).ok()?;
    Some(id.to_string())
}

pub fn files(home: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    for sub in ["sessions", "archived_sessions"] {
        for e in walkdir::WalkDir::new(home.join(sub)).max_depth(4).into_iter().flatten() {
            if e.file_type().is_file() && id_of(e.path()).is_some() {
                v.push(e.into_path());
            }
        }
    }
    v
}

pub fn find(home: &Path, id: &str) -> Option<PathBuf> {
    files(home).into_iter().filter(|p| id_of(p).as_deref() == Some(id)).max_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
}

/// The names given to threads (`session_index.jsonl`, newest entry wins).
pub fn names(home: &Path) -> HashMap<String, String> {
    let mut m = HashMap::new();
    let Ok(text) = std::fs::read_to_string(home.join("session_index.jsonl")) else { return m };
    for l in text.lines() {
        if let Ok(o) = serde_json::from_str::<Value>(l) {
            if let (Some(id), Some(n)) = (o["id"].as_str(), o["thread_name"].as_str().map(str::trim).filter(|n| !n.is_empty())) {
                m.insert(id.to_string(), n.to_string());
            }
        }
    }
    m
}

/// A session file's text (decompressed), at most its last `max` bytes.
pub fn read_text(path: &Path, max: usize) -> (String, bool) {
    let compressed = path.extension().is_some_and(|e| e == "zst");
    let mut buf = Vec::new();
    if compressed {
        let Ok(f) = std::fs::File::open(path) else { return (String::new(), false) };
        let Ok(mut d) = ruzstd::decoding::StreamingDecoder::new(std::io::BufReader::new(f)) else { return (String::new(), false) };
        let _ = (&mut d).take(512 << 20).read_to_end(&mut buf);
    } else if let Ok(b) = std::fs::read(path) {
        buf = b;
    }
    let cut = buf.len() > max;
    if cut {
        buf.drain(..buf.len() - max);
    }
    let mut s = String::from_utf8_lossy(&buf).into_owned();
    if cut {
        s = s.split_once('\n').map(|(_, r)| r.to_string()).unwrap_or_default();
    }
    (s, cut)
}

#[derive(Debug, Clone, Default)]
pub struct Info {
    pub cwd: Option<String>,
    pub branch: Option<String>,
    pub first: Option<String>,
    pub last: Option<String>,
    pub model: Option<String>,
}

/// User messages as the model saw them start with context blocks like
/// `<environment_context>`; those aren't the user's words.
fn plain_user(t: &str) -> Option<String> {
    let t = t.trim();
    (!t.is_empty() && !t.starts_with('<')).then(|| t.to_string())
}

fn texts(content: &Value) -> String {
    content.as_array().into_iter().flatten().filter_map(|c| c["text"].as_str()).collect::<Vec<_>>().join("\n")
}

pub fn info(path: &Path) -> Info {
    let (text, _) = read_text(path, 48 << 20);
    let mut i = Info::default();
    let mut fallback_first = None;
    let mut fallback_last = None;
    for l in text.lines() {
        let Ok(o) = serde_json::from_str::<Value>(l) else { continue };
        let p = &o["payload"];
        match (o["type"].as_str().unwrap_or_default(), p["type"].as_str().unwrap_or_default()) {
            ("session_meta", _) => {
                i.cwd = p["cwd"].as_str().map(String::from).or(i.cwd.take());
                i.branch = p["git"]["branch"].as_str().map(String::from).or(i.branch.take());
            }
            ("turn_context", _) => i.model = p["model"].as_str().map(String::from).or(i.model.take()),
            ("event_msg", "user_message") => {
                let t = p["message"].as_str().unwrap_or_default().trim().to_string();
                if !t.is_empty() {
                    i.first.get_or_insert_with(|| t.clone());
                    i.last = Some(t);
                }
            }
            ("event_msg", "agent_message") => i.last = p["message"].as_str().map(String::from).or(i.last.take()),
            ("response_item", "message") => {
                if let Some(t) = if p["role"] == "user" { plain_user(&texts(&p["content"])) } else if p["role"] == "assistant" { Some(texts(&p["content"])) } else { None } {
                    if p["role"] == "user" {
                        fallback_first.get_or_insert_with(|| t.clone());
                    }
                    fallback_last = Some(t);
                }
            }
            _ => {}
        }
    }
    if i.first.is_none() {
        i.first = fallback_first;
        i.last = i.last.or(fallback_last);
    }
    i
}

/// What a patch touched: "Edited main.rs, lib.rs".
fn patch_label(patch: &str) -> String {
    let files: Vec<String> = patch
        .lines()
        .filter_map(|l| l.strip_prefix("*** Update File: ").or(l.strip_prefix("*** Add File: ")).or(l.strip_prefix("*** Delete File: ")))
        .map(|f| Path::new(f.trim()).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())
        .collect();
    if files.is_empty() {
        "Edited files".into()
    } else {
        format!("Edited {}", ellipsize(&files.join(", "), 80))
    }
}

fn command_text(v: &Value) -> String {
    match v {
        Value::Array(a) => {
            let parts: Vec<&str> = a.iter().filter_map(Value::as_str).collect();
            // ["bash", "-lc", "the command"] → the command
            if parts.len() == 3 && matches!(parts[1], "-lc" | "-c" | "/c" | "-Command") {
                parts[2].to_string()
            } else {
                parts.join(" ")
            }
        }
        Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

pub fn tool_label(p: &Value) -> Option<String> {
    let one = |t: &str| ellipsize(t.lines().next().unwrap_or_default(), 80);
    Some(match p["type"].as_str()? {
        "function_call" => {
            let name = p["name"].as_str().unwrap_or_default();
            let args: Value = p["arguments"].as_str().and_then(|a| serde_json::from_str(a).ok()).unwrap_or(Value::Null);
            match name {
                "shell" | "exec_command" | "shell_command" | "container.exec" => format!("Ran {}", one(&command_text(if args["command"].is_null() { &args["cmd"] } else { &args["command"] }))),
                "apply_patch" => patch_label(args["input"].as_str().unwrap_or_default()),
                "update_plan" => "Updated its plan".into(),
                "view_image" => "Looked at a picture".into(),
                n => format!("Used {}", n.replace('_', " ")),
            }
        }
        "custom_tool_call" => match p["name"].as_str().unwrap_or_default() {
            "apply_patch" => patch_label(p["input"].as_str().unwrap_or_default()),
            n => format!("Used {}", n.replace('_', " ")),
        },
        "local_shell_call" => format!("Ran {}", one(&command_text(&p["action"]["command"]))),
        "web_search_call" => match p["action"]["query"].as_str() {
            Some(q) => format!("Searched the web for {}", one(q)),
            None => "Searched the web".into(),
        },
        _ => return None,
    })
}

pub fn transcript(path: &Path, limit: usize) -> (Vec<Message>, bool) {
    let (text, cut) = read_text(path, 32 << 20);
    let has_events = text.contains("\"user_message\"");
    let mut b = Builder::default();
    for l in text.lines() {
        let Ok(o) = serde_json::from_str::<Value>(l) else { continue };
        let at = ts(&o["timestamp"]);
        let p = &o["payload"];
        match (o["type"].as_str().unwrap_or_default(), p["type"].as_str().unwrap_or_default()) {
            ("event_msg", "user_message") => {
                if let Some(t) = p["message"].as_str().map(str::trim).filter(|t| !t.is_empty()) {
                    b.push(Role::User, t.to_string(), at);
                }
            }
            ("event_msg", "agent_message") => b.push(Role::Assistant, p["message"].as_str().unwrap_or_default().to_string(), at),
            ("event_msg", "turn_aborted") => b.notice("Stopped.".into(), at, "info"),
            ("event_msg", "error") => b.notice(p["message"].as_str().unwrap_or("Codex hit an error.").to_string(), at, "warn"),
            ("compacted", _) => b.notice("The conversation was summarized here to make room.".into(), at, "info"),
            ("response_item", "message") if !has_events => {
                if p["role"] == "user" {
                    if let Some(t) = plain_user(&texts(&p["content"])) {
                        b.push(Role::User, t, at);
                    }
                } else if p["role"] == "assistant" {
                    b.push(Role::Assistant, texts(&p["content"]), at);
                }
            }
            ("response_item", _) => {
                if let Some(t) = tool_label(p) {
                    b.tool(t, at);
                }
            }
            _ => {}
        }
    }
    b.finish(limit, cut)
}

pub fn args(resume: Option<&str>, sandbox: &str) -> Vec<String> {
    let sb = if SANDBOXES.contains(&sandbox) { sandbox } else { "read-only" };
    let mut a: Vec<String> = ["exec", "--json", "--skip-git-repo-check", "--sandbox", sb].iter().map(|s| s.to_string()).collect();
    if let Some(id) = resume {
        a.extend(["resume".into(), id.to_string()]);
    }
    a.push("-".into());
    a
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Started(String),
    Text(String),
    Tool(String),
    Note(String),
    Notice(String),
    /// The turn ended (`error` when it failed), with its token use.
    Done { error: Option<String>, input: u64, output: u64 },
}

#[derive(Default)]
pub struct Reader {
    texts: usize,
}

impl Reader {
    pub fn read(&mut self, v: &Value) -> Vec<Event> {
        let mut out = Vec::new();
        let item = &v["item"];
        let one = |t: &str| ellipsize(t.lines().next().unwrap_or_default(), 80);
        match v["type"].as_str().unwrap_or_default() {
            "thread.started" => {
                if let Some(id) = v["thread_id"].as_str() {
                    out.push(Event::Started(id.to_string()));
                }
            }
            "item.started" => match item["type"].as_str().unwrap_or_default() {
                "command_execution" => out.push(Event::Tool(format!("Ran {}", one(item["command"].as_str().unwrap_or_default())))),
                "web_search" => out.push(Event::Tool(format!("Searched the web for {}", one(item["query"].as_str().unwrap_or_default())))),
                "mcp_tool_call" => out.push(Event::Tool(format!("Used {} ({})", item["tool"].as_str().unwrap_or_default().replace('_', " "), item["server"].as_str().unwrap_or_default()))),
                _ => {}
            },
            "item.completed" => match item["type"].as_str().unwrap_or_default() {
                "agent_message" => {
                    let t = item["text"].as_str().unwrap_or_default();
                    if self.texts > 0 {
                        out.push(Event::Text("\n\n".into()));
                    }
                    self.texts += 1;
                    out.push(Event::Text(t.to_string()));
                }
                "file_change" => {
                    let files: Vec<String> = item["changes"].as_array().into_iter().flatten().filter_map(|c| c["path"].as_str()).map(|p| Path::new(p).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()).collect();
                    out.push(Event::Tool(if files.is_empty() { "Edited files".into() } else { format!("Edited {}", ellipsize(&files.join(", "), 80)) }));
                }
                "reasoning" => {
                    let t = item["text"].as_str().unwrap_or_default().trim().trim_matches('*').trim();
                    if !t.is_empty() {
                        out.push(Event::Note(one(t)));
                    }
                }
                "error" => out.push(Event::Notice(item["message"].as_str().unwrap_or_default().to_string())),
                _ => {}
            },
            "turn.completed" => {
                let u = &v["usage"];
                out.push(Event::Done { error: None, input: u["input_tokens"].as_u64().unwrap_or(0), output: u["output_tokens"].as_u64().unwrap_or(0) });
            }
            "turn.failed" => out.push(Event::Done { error: Some(v["error"]["message"].as_str().unwrap_or("Codex couldn't finish.").to_string()), input: 0, output: 0 }),
            "error" => out.push(Event::Notice(v["message"].as_str().unwrap_or_default().to_string())),
            _ => {}
        }
        out
    }
}
