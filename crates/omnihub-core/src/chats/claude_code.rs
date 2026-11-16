//! Claude Code's conversations on this PC, read from the transcripts it
//! keeps (`~/.claude/projects/<folder>/<session>.jsonl`, or under
//! `CLAUDE_CONFIG_DIR`), and continued with `claude -p --resume`, the way
//! its docs describe scripting it. Only what the user and Claude said and
//! which tools ran is shown; hidden context (reminders, tool output,
//! thinking) stays out.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{ellipsize, ts, Builder, Message, Role};

/// The permission modes offered from OmniHub (never `bypassPermissions`).
pub const MODES: &[&str] = &["plan", "default", "acceptEdits", "auto"];

pub fn home() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("CLAUDE_CONFIG_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    dirs::home_dir().map(|h| h.join(".claude"))
}

/// Every transcript file: `projects/*/<uuid>.jsonl`.
pub fn files(home: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    let Ok(rd) = std::fs::read_dir(home.join("projects")) else { return v };
    for p in rd.flatten().filter(|e| e.path().is_dir()) {
        let Ok(inner) = std::fs::read_dir(p.path()) else { continue };
        for f in inner.flatten() {
            let path = f.path();
            if path.extension().is_some_and(|e| e == "jsonl") && path.file_stem().and_then(|s| s.to_str()).is_some_and(|s| uuid::Uuid::parse_str(s).is_ok()) {
                v.push(path);
            }
        }
    }
    v
}

pub fn find(home: &Path, id: &str) -> Option<PathBuf> {
    uuid::Uuid::parse_str(id).ok()?;
    let rd = std::fs::read_dir(home.join("projects")).ok()?;
    rd.flatten().map(|d| d.path().join(format!("{id}.jsonl"))).find(|p| p.is_file())
}

/// The text of a slash command line (`<command-name>/model</command-name>
/// <command-args>opus</command-args>` → `/model opus`), the prompt without
/// reminders, or nothing for lines that aren't the user's own words.
fn clean_prompt(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.starts_with("Caveat:") || s.starts_with("<local-command-") || s.starts_with("<bash-stdout>") || s.starts_with("<bash-stderr>") || s.starts_with("[Request interrupted") {
        return None;
    }
    let tag = |s: &str, t: &str| -> Option<String> {
        let a = s.find(&format!("<{t}>"))? + t.len() + 2;
        let b = s[a..].find(&format!("</{t}>"))? + a;
        Some(s[a..b].trim().to_string())
    };
    if let Some(name) = tag(s, "command-name") {
        let args = tag(s, "command-args").unwrap_or_default();
        return Some(if args.is_empty() { name } else { format!("{name} {args}") });
    }
    if let Some(cmd) = tag(s, "bash-input") {
        return Some(format!("! {cmd}"));
    }
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("<system-reminder>") {
        out.push_str(&rest[..i]);
        match rest[i..].find("</system-reminder>") {
            Some(j) => rest = &rest[i + j + "</system-reminder>".len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    let out = out.trim().to_string();
    (!out.is_empty()).then_some(out)
}

/// What the user typed in a transcript line, if it is one.
pub fn human_text(o: &Value) -> Option<String> {
    if o["type"] != "user" || o["isMeta"] == true || o["isSidechain"] == true || o["isCompactSummary"] == true || o["isVisibleInTranscriptOnly"] == true {
        return None;
    }
    if o["origin"]["kind"].as_str().is_some_and(|k| k != "human") {
        return None;
    }
    let raw = match &o["message"]["content"] {
        Value::String(s) => s.clone(),
        Value::Array(a) => {
            if a.iter().any(|b| b["type"] == "tool_result") {
                return None;
            }
            a.iter().filter(|b| b["type"] == "text").filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join("\n")
        }
        _ => return None,
    };
    clean_prompt(&raw)
}

/// A short line for a tool call: "Ran npm test", "Edited main.rs".
pub fn tool_label(name: &str, input: &Value) -> String {
    let s = |k: &str| input[k].as_str().unwrap_or_default();
    let file = |k: &str| Path::new(s(k)).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
    let one = |t: &str| ellipsize(t.lines().next().unwrap_or_default(), 80);
    match name {
        "Bash" | "PowerShell" => {
            let d = s("description");
            if d.is_empty() { format!("Ran {}", one(s("command"))) } else { one(d) }
        }
        "Read" => format!("Read {}", file("file_path")),
        "Edit" | "MultiEdit" => format!("Edited {}", file("file_path")),
        "Write" => format!("Wrote {}", file("file_path")),
        "NotebookEdit" => format!("Edited {}", file("notebook_path")),
        "Grep" => format!("Searched for {}", one(s("pattern"))),
        "Glob" => format!("Looked for {}", one(s("pattern"))),
        "WebSearch" => format!("Searched the web for {}", one(s("query"))),
        "WebFetch" => format!("Read {}", s("url").split('/').nth(2).unwrap_or("a web page")),
        "Task" | "Agent" => format!("Asked a helper: {}", one(s("description"))),
        "TodoWrite" | "TaskCreate" | "TaskUpdate" => "Updated its to-do list".into(),
        n if n.starts_with("mcp__") => {
            let mut parts = n.splitn(3, "__").skip(1);
            let (server, tool) = (parts.next().unwrap_or_default(), parts.next().unwrap_or_default());
            format!("Used {} ({server})", tool.replace('_', " "))
        }
        n => format!("Used {n}"),
    }
}

/// What a transcript says about itself, from its start and end.
#[derive(Debug, Clone, Default)]
pub struct Info {
    pub cwd: Option<String>,
    pub branch: Option<String>,
    pub first: Option<String>,
    pub last: Option<String>,
    pub model: Option<String>,
    custom: Option<String>,
    ai: Option<String>,
    summary: Option<String>,
}

impl Info {
    /// The name the user gave it, else Claude Code's own, else its first words.
    pub fn title(&self) -> Option<String> {
        self.custom.clone().or(self.ai.clone()).or(self.summary.clone()).or(self.first.clone())
    }
}

fn read_range(f: &mut std::fs::File, from: u64, len: u64) -> String {
    let mut buf = Vec::new();
    if f.seek(SeekFrom::Start(from)).is_ok() {
        let _ = f.take(len).read_to_end(&mut buf);
    }
    String::from_utf8_lossy(&buf).into_owned()
}

fn note_line(o: &Value, i: &mut Info, tail: bool) {
    let text = |k: &str| o[k].as_str().map(str::trim).filter(|t| !t.is_empty()).map(String::from);
    match o["type"].as_str().unwrap_or_default() {
        "custom-title" => i.custom = text("customTitle").or(i.custom.take()),
        "ai-title" => i.ai = text("aiTitle").or(i.ai.take()),
        "summary" if i.summary.is_none() => i.summary = text("summary"),
        _ => {}
    }
    if i.cwd.is_none() || tail {
        if let Some(c) = o["cwd"].as_str() {
            i.cwd = Some(c.to_string());
        }
    }
    if let Some(b) = o["gitBranch"].as_str().filter(|b| !b.is_empty()) {
        if i.branch.is_none() || tail {
            i.branch = Some(b.to_string());
        }
    }
    if let Some(t) = human_text(o) {
        if i.first.is_none() {
            i.first = Some(t.clone());
        }
        if tail {
            i.last = Some(t);
        }
    }
    if o["type"] == "assistant" && o["isSidechain"] != true {
        if let Some(m) = o["message"]["model"].as_str().filter(|m| m.starts_with("claude")) {
            i.model = Some(m.to_string());
        }
        if tail {
            if let Some(t) = o["message"]["content"].as_array().and_then(|a| a.iter().rev().find(|b| b["type"] == "text")).and_then(|b| b["text"].as_str()) {
                i.last = Some(t.to_string());
            }
        }
    }
}

/// The start and end of a transcript, enough for the list.
pub fn info(path: &Path) -> Info {
    const CHUNK: u64 = 1 << 20;
    let mut i = Info::default();
    let Ok(mut f) = std::fs::File::open(path) else { return i };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let head = read_range(&mut f, 0, CHUNK.min(len));
    for l in head.lines() {
        if let Ok(o) = serde_json::from_str::<Value>(l) {
            note_line(&o, &mut i, len <= CHUNK);
        }
    }
    if len > CHUNK {
        let from = len.saturating_sub(CHUNK).max(CHUNK);
        let tail = read_range(&mut f, from, len - from);
        for l in tail.lines().skip(1) {
            if let Ok(o) = serde_json::from_str::<Value>(l) {
                note_line(&o, &mut i, true);
            }
        }
    }
    i
}

/// The conversation, newest `limit` messages. Very long transcripts are
/// read from their last 24 MB (`true` when older messages were left out).
pub fn transcript(path: &Path, limit: usize) -> (Vec<Message>, bool) {
    const MAX: u64 = 24 << 20;
    let Ok(mut f) = std::fs::File::open(path) else { return (Vec::new(), false) };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let cut = len > MAX;
    let text = read_range(&mut f, len.saturating_sub(MAX), MAX.min(len));
    let mut b = Builder::default();
    for (n, l) in text.lines().enumerate() {
        if cut && n == 0 {
            continue;
        }
        let Ok(o) = serde_json::from_str::<Value>(l) else { continue };
        let at = ts(&o["timestamp"]);
        if let Some(t) = human_text(&o) {
            b.push(Role::User, t, at);
            continue;
        }
        match o["type"].as_str().unwrap_or_default() {
            "assistant" if o["isSidechain"] != true => {
                if o["isApiErrorMessage"] == true {
                    let t: String = o["message"]["content"].as_array().into_iter().flatten().filter_map(|b| b["text"].as_str()).collect();
                    b.notice(if t.is_empty() { "Claude Code couldn't get an answer.".into() } else { t }, at, "warn");
                    continue;
                }
                for blk in o["message"]["content"].as_array().into_iter().flatten() {
                    match blk["type"].as_str().unwrap_or_default() {
                        "text" => b.push(Role::Assistant, blk["text"].as_str().unwrap_or_default().to_string(), at),
                        "tool_use" => b.tool(tool_label(blk["name"].as_str().unwrap_or_default(), &blk["input"]), at),
                        _ => {}
                    }
                }
            }
            "system" if o["subtype"] == "compact_boundary" => b.notice("The conversation was summarized here to make room.".into(), at, "info"),
            _ => {}
        }
    }
    b.finish(limit, cut)
}

/// The arguments for one message: continue `resume`, or start the session
/// `new_id`.
pub fn args(resume: Option<&str>, new_id: Option<&str>, mode: &str) -> Vec<String> {
    let mut a: Vec<String> = ["-p", "--output-format", "stream-json", "--verbose", "--include-partial-messages", "--permission-mode"].iter().map(|s| s.to_string()).collect();
    a.push(if MODES.contains(&mode) { mode.to_string() } else { "default".into() });
    if let Some(id) = resume {
        a.extend(["--resume".into(), id.to_string()]);
    } else if let Some(id) = new_id {
        a.extend(["--session-id".into(), id.to_string()]);
    }
    a
}

/// What a line of `--output-format stream-json` means for the window.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Text(String),
    Tool(String),
    Note(String),
    /// The run finished: whether it went well, its last words, what it cost
    /// (Claude Code's own estimate), and tools it wanted but wasn't allowed.
    Result { ok: bool, text: String, cost: Option<f64>, denied: Vec<String> },
}

#[derive(Default)]
pub struct Reader {
    streamed: bool,
    after_tool: bool,
}

impl Reader {
    pub fn read(&mut self, v: &Value) -> Vec<Event> {
        let mut out = Vec::new();
        let top = v["parent_tool_use_id"].is_null();
        match v["type"].as_str().unwrap_or_default() {
            "stream_event" if top => {
                let e = &v["event"];
                if e["type"] == "content_block_start" && e["content_block"]["type"] == "text" && self.after_tool {
                    self.after_tool = false;
                    out.push(Event::Text("\n\n".into()));
                }
                if e["type"] == "content_block_delta" && e["delta"]["type"] == "text_delta" {
                    self.streamed = true;
                    out.push(Event::Text(e["delta"]["text"].as_str().unwrap_or_default().to_string()));
                }
            }
            "assistant" if top => {
                for b in v["message"]["content"].as_array().into_iter().flatten() {
                    match b["type"].as_str().unwrap_or_default() {
                        "tool_use" => {
                            self.after_tool = true;
                            out.push(Event::Tool(tool_label(b["name"].as_str().unwrap_or_default(), &b["input"])));
                        }
                        "text" if !self.streamed => out.push(Event::Text(b["text"].as_str().unwrap_or_default().to_string())),
                        _ => {}
                    }
                }
            }
            "system" if v["subtype"] == "api_retry" => out.push(Event::Note(format!("Claude is busy; trying again (attempt {})…", v["attempt"].as_u64().unwrap_or(1)))),
            "result" => {
                let denied = v["permission_denials"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|d| tool_label(d["tool_name"].as_str().unwrap_or_default(), &d["tool_input"]))
                    .collect::<Vec<_>>();
                out.push(Event::Result { ok: v["is_error"] != true && v["subtype"] == "success", text: v["result"].as_str().unwrap_or_default().to_string(), cost: v["total_cost_usd"].as_f64(), denied });
            }
            _ => {}
        }
        out
    }
}
