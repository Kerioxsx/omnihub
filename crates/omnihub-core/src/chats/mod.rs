//! Chats: talk to Claude and to ChatGPT's models with the user's own API
//! keys, and pick up the Claude Code and Codex conversations on this PC,
//! from the window or from a paired phone. Messages can be saved as ideas
//! (notes), and the Usage view adds up what each of them used.
//!
//! OmniHub's own chats are kept in `data/chats/omni/<id>.json`. Claude Code
//! and Codex keep their own transcripts; OmniHub reads them and, to send a
//! message, runs `claude -p --resume` or `codex exec resume` in the
//! conversation's folder with the permission mode the user picked.

pub mod agent;
pub mod claude_code;
pub mod codex;
pub mod openai;
pub mod usage;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::claude::api::{self, Delta};
use crate::claude::Claude;
use crate::db::now;
use crate::events::EventBus;
use openai::OpenAi;

/// The longest message OmniHub sends.
pub const MAX_MESSAGE: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Omni,
    ClaudeCode,
    Codex,
}

impl Source {
    pub fn parse(s: &str) -> Option<Source> {
        match s {
            "omni" => Some(Source::Omni),
            "claudeCode" => Some(Source::ClaudeCode),
            "codex" => Some(Source::Codex),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Source::Omni => "OmniHub",
            Source::ClaudeCode => "Claude Code",
            Source::Codex => "Codex",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    User,
    Assistant,
    /// A run of tool calls ("Ran npm test", "Edited main.rs").
    Tools,
    /// Something that happened (stopped, an error, a summary point).
    Notice,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub role: Role,
    pub text: String,
    /// Unix seconds.
    pub at: i64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
}

pub fn ellipsize(s: &str, n: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= n {
        return s.to_string();
    }
    let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
    out = out.trim_end().to_string();
    out.push('…');
    out
}

/// A transcript timestamp (RFC 3339, or milliseconds) in Unix seconds.
pub fn ts(v: &Value) -> i64 {
    match v {
        Value::String(s) => chrono::DateTime::parse_from_rfc3339(s).map(|d| d.timestamp()).unwrap_or(0),
        Value::Number(n) => n.as_i64().map(|ms| if ms > 100_000_000_000 { ms / 1000 } else { ms }).unwrap_or(0),
        _ => 0,
    }
}

/// Puts a transcript's lines together into readable messages: an answer's
/// text pieces join up, and runs of tool calls become one line.
#[derive(Default)]
pub struct Builder {
    msgs: Vec<Message>,
}

impl Builder {
    pub fn push(&mut self, role: Role, text: String, at: i64) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.msgs.last_mut() {
            if role == Role::Assistant && last.role == Role::Assistant {
                last.text.push_str("\n\n");
                last.text.push_str(text);
                return;
            }
        }
        self.msgs.push(Message { role, text: text.to_string(), at, tools: Vec::new(), tone: None });
    }

    pub fn tool(&mut self, label: String, at: i64) {
        if let Some(last) = self.msgs.last_mut() {
            if last.role == Role::Tools {
                if last.tools.len() < 40 {
                    last.tools.push(label);
                }
                return;
            }
        }
        self.msgs.push(Message { role: Role::Tools, text: String::new(), at, tools: vec![label], tone: None });
    }

    pub fn notice(&mut self, text: String, at: i64, tone: &str) {
        self.msgs.push(Message { role: Role::Notice, text, at, tools: Vec::new(), tone: Some(tone.into()) });
    }

    /// The newest `limit` messages, and whether older ones were left out.
    pub fn finish(mut self, limit: usize, cut: bool) -> (Vec<Message>, bool) {
        let n = self.msgs.len();
        if n > limit {
            self.msgs.drain(..n - limit);
        }
        (self.msgs, cut || n > limit)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSummary {
    pub source: Option<Source>,
    pub id: String,
    pub title: String,
    pub preview: String,
    pub updated: i64,
    /// The folder a Claude Code or Codex conversation works in.
    pub folder: Option<String>,
    pub model: Option<String>,
    /// OmniHub's own chats: `claude` or `openai`.
    pub provider: Option<String>,
    pub branch: Option<String>,
    pub running: bool,
}

/// What's streaming in right now.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Live {
    /// What the user just sent (shown until the transcript has it).
    pub prompt: String,
    pub text: String,
    pub tools: Vec<String>,
    pub note: String,
    pub started: i64,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatView {
    pub summary: ChatSummary,
    pub messages: Vec<Message>,
    /// Older messages weren't included.
    pub earlier: bool,
    pub live: Option<Live>,
    /// The conversation changed in the last two minutes, not through
    /// OmniHub: it's probably open on the PC.
    pub busy_elsewhere: bool,
}

/// One of OmniHub's own conversations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct OwnChat {
    id: String,
    title: String,
    provider: String,
    model: String,
    created: i64,
    updated: i64,
    messages: Vec<Message>,
    claude_usage: api::Usage,
    openai_usage: openai::Usage,
    cost: f64,
}

/// What a new conversation is with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NewKind {
    Claude,
    Openai,
    ClaudeCode,
    Codex,
}

/// A folder Claude Code or Codex has worked in.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub path: String,
    pub name: String,
    pub last: i64,
    pub claude_code: bool,
    pub codex: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    /// Its conversations were found on this PC.
    pub has_chats: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tools {
    pub claude_code: Tool,
    pub codex: Tool,
    pub claude_key: bool,
    pub openai_key: bool,
}

/// Where Claude Code and Codex keep their files and where their programs
/// are (found on first use unless given).
#[derive(Debug, Clone, Default)]
pub struct Places {
    pub claude_home: Option<PathBuf>,
    pub codex_home: Option<PathBuf>,
    pub claude_exe: Option<PathBuf>,
    pub codex_exe: Option<PathBuf>,
    /// Look for the programs on PATH and in their usual folders.
    pub search: bool,
}

/// What a Claude Code or Codex run continues or starts.
enum Target {
    Resume(String),
    /// A new Claude Code session with this id.
    NewClaude(String),
    NewCodex,
}

struct Run {
    cancel: Arc<AtomicBool>,
    live: Live,
}

type Key = (Source, String);

const SYSTEM: &str = "You're chatting with the user through OmniHub, an app on their Windows PC; they may be on their PC or their phone. Answer naturally and to the point, and use Markdown when it helps (lists, code blocks, tables). The user can save any of your messages as an idea in their notes.";

pub struct ChatHub {
    dir: PathBuf,
    events: EventBus,
    pub claude: Arc<Claude>,
    pub openai: Arc<OpenAi>,
    places: Mutex<Places>,
    runs: Mutex<HashMap<Key, Run>>,
    /// When OmniHub last finished running a conversation (to tell its own
    /// changes from the PC's).
    ours: Mutex<HashMap<Key, i64>>,
    infos: Mutex<HashMap<PathBuf, (u64, i64, ChatSummary)>>,
    /// `--version` answers by program file and its modified time.
    versions: Mutex<HashMap<PathBuf, (i64, Option<String>)>>,
    pub usage: usage::Usage,
}

fn modified(p: &Path) -> (u64, i64) {
    std::fs::metadata(p).map(|m| (m.len(), m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64))).unwrap_or((0, 0))
}

fn check_text(text: &str) -> Result<String, String> {
    let t = text.trim();
    if t.is_empty() {
        return Err("Type a message first.".into());
    }
    if t.chars().count() > MAX_MESSAGE {
        return Err("That message is too long (100,000 characters at most).".into());
    }
    Ok(t.to_string())
}

impl ChatHub {
    pub fn new(data_dir: &Path, events: EventBus, claude: Arc<Claude>) -> Arc<ChatHub> {
        let openai = Arc::new(OpenAi::new(data_dir));
        ChatHub::with(data_dir, events, claude, openai, Places { claude_home: claude_code::home(), codex_home: codex::home(), search: true, ..Default::default() })
    }

    pub fn with(data_dir: &Path, events: EventBus, claude: Arc<Claude>, openai: Arc<OpenAi>, places: Places) -> Arc<ChatHub> {
        let dir = data_dir.join("chats");
        let _ = std::fs::create_dir_all(dir.join("omni"));
        Arc::new(ChatHub { usage: usage::Usage::new(&dir), dir, events, claude, openai, places: Mutex::new(places), runs: Mutex::new(HashMap::new()), ours: Mutex::new(HashMap::new()), infos: Mutex::new(HashMap::new()), versions: Mutex::new(HashMap::new()) })
    }

    fn exe(&self, source: Source) -> Option<PathBuf> {
        let name = match source {
            Source::ClaudeCode => "claude",
            Source::Codex => "codex",
            Source::Omni => return None,
        };
        let (given, search) = {
            let p = self.places.lock();
            (if source == Source::ClaudeCode { p.claude_exe.clone() } else { p.codex_exe.clone() }, p.search)
        };
        if let Some(e) = given.filter(|e| e.is_file()) {
            return Some(e);
        }
        if !search {
            return None;
        }
        let found = agent::find(name, &agent::usual_dirs())?;
        let mut p = self.places.lock();
        *(if source == Source::ClaudeCode { &mut p.claude_exe } else { &mut p.codex_exe }) = Some(found.clone());
        Some(found)
    }

    fn version(&self, exe: &Path) -> Option<String> {
        let m = modified(exe).1;
        if let Some((at, v)) = self.versions.lock().get(exe) {
            if *at == m {
                return v.clone();
            }
        }
        let v = agent::version(exe);
        self.versions.lock().insert(exe.to_path_buf(), (m, v.clone()));
        v
    }

    fn homes(&self) -> (Option<PathBuf>, Option<PathBuf>) {
        let p = self.places.lock();
        (p.claude_home.clone(), p.codex_home.clone())
    }

    pub fn tools(&self) -> Tools {
        let (ch, xh) = self.homes();
        let tool = |s: Source, home: Option<PathBuf>, has: &dyn Fn(&Path) -> bool| {
            let exe = self.exe(s);
            Tool { installed: exe.is_some(), version: exe.as_deref().and_then(|e| self.version(e)), path: exe.map(|e| e.to_string_lossy().into_owned()), has_chats: home.as_deref().is_some_and(has) }
        };
        Tools {
            claude_code: tool(Source::ClaudeCode, ch, &|h| !claude_code::files(h).is_empty()),
            codex: tool(Source::Codex, xh, &|h| !codex::files(h).is_empty()),
            claude_key: self.claude.has_key(),
            openai_key: self.openai.has_key(),
        }
    }

    fn emit_changed(&self, source: Source, id: &str) {
        self.events.emit("chats:changed", json!({ "source": source, "id": id }));
    }

    fn delta(&self, key: &Key, kind: &str, text: &str) {
        if let Some(r) = self.runs.lock().get_mut(key) {
            match kind {
                "text" => r.live.text.push_str(text),
                "tool" => r.live.tools.push(text.to_string()),
                "note" => r.live.note = text.to_string(),
                _ => {}
            }
        }
        self.events.emit("chats:delta", json!({ "source": key.0, "id": key.1, "kind": kind, "text": text }));
    }

    fn running(&self, key: &Key) -> bool {
        self.runs.lock().contains_key(key)
    }

    fn start_run(&self, key: &Key, prompt: &str) -> Result<Arc<AtomicBool>, String> {
        let mut runs = self.runs.lock();
        if runs.contains_key(key) {
            return Err("It's still answering. Wait for it, or press Stop.".into());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        runs.insert(key.clone(), Run { cancel: cancel.clone(), live: Live { prompt: prompt.to_string(), started: now(), ..Default::default() } });
        Ok(cancel)
    }

    fn end_run(&self, key: &Key, error: Option<String>) {
        self.runs.lock().remove(key);
        self.ours.lock().insert(key.clone(), now());
        self.infos.lock().retain(|_, (_, _, s)| !(s.source == Some(key.0) && s.id == key.1));
        match &error {
            Some(e) => self.events.emit("chats:delta", json!({ "source": key.0, "id": key.1, "kind": "error", "text": e })),
            None => self.events.emit("chats:delta", json!({ "source": key.0, "id": key.1, "kind": "done", "text": "" })),
        }
        self.emit_changed(key.0, &key.1);
    }

    pub fn stop(&self, source: Source, id: &str) {
        if let Some(r) = self.runs.lock().get(&(source, id.to_string())) {
            r.cancel.store(true, Ordering::SeqCst);
        }
    }

    // ---------- OmniHub's own chats ----------

    fn own_path(&self, id: &str) -> Result<PathBuf, String> {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err("No such chat.".into());
        }
        Ok(self.dir.join("omni").join(format!("{id}.json")))
    }

    fn load_own(&self, id: &str) -> Result<OwnChat, String> {
        let b = std::fs::read(self.own_path(id)?).map_err(|_| "No such chat.".to_string())?;
        serde_json::from_slice(&b).map_err(|e| e.to_string())
    }

    fn save_own(&self, c: &OwnChat) -> Result<(), String> {
        let p = self.own_path(&c.id)?;
        let tmp = p.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(c).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
    }

    fn edit_own(&self, id: &str, f: impl FnOnce(&mut OwnChat)) -> Result<OwnChat, String> {
        let mut c = self.load_own(id)?;
        f(&mut c);
        c.updated = now();
        self.save_own(&c)?;
        Ok(c)
    }

    fn own_summary(&self, c: &OwnChat) -> ChatSummary {
        let last = c.messages.iter().rev().find(|m| matches!(m.role, Role::User | Role::Assistant)).map(|m| ellipsize(&m.text, 140)).unwrap_or_default();
        ChatSummary { source: Some(Source::Omni), id: c.id.clone(), title: c.title.clone(), preview: last, updated: c.updated, model: Some(c.model.clone()), provider: Some(c.provider.clone()), running: self.running(&(Source::Omni, c.id.clone())), ..Default::default() }
    }

    pub fn rename(&self, id: &str, title: &str) -> Result<(), String> {
        let t = ellipsize(title, 120);
        if t.is_empty() {
            return Err("Give it a name.".into());
        }
        self.edit_own(id, |c| c.title = t)?;
        self.emit_changed(Source::Omni, id);
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        self.stop(Source::Omni, id);
        std::fs::remove_file(self.own_path(id)?).map_err(|e| e.to_string())?;
        self.emit_changed(Source::Omni, id);
        Ok(())
    }

    fn run_own(self: &Arc<Self>, id: String, cancel: Arc<AtomicBool>) {
        let key = (Source::Omni, id.clone());
        let result = self.answer_own(&id, &key, &cancel);
        let stopped = cancel.load(Ordering::SeqCst);
        let partial = self.runs.lock().get(&key).map(|r| r.live.text.clone()).unwrap_or_default();
        let error = match result {
            Ok(()) => None,
            Err(e) => {
                let _ = self.edit_own(&id, |c| {
                    if !partial.trim().is_empty() {
                        c.messages.push(Message { role: Role::Assistant, text: partial.trim().to_string(), at: now(), tools: Vec::new(), tone: None });
                    }
                    let (text, tone) = if stopped { ("Stopped.".to_string(), "info") } else { (e.clone(), "bad") };
                    c.messages.push(Message { role: Role::Notice, text, at: now(), tools: Vec::new(), tone: Some(tone.into()) });
                });
                (!stopped).then_some(e)
            }
        };
        self.end_run(&key, error);
    }

    /// The conversation so far as (role, text), consecutive turns of the
    /// same side joined (a reply that failed leaves two user turns).
    fn turns(c: &OwnChat) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = Vec::new();
        for m in &c.messages {
            let role = match m.role {
                Role::User => "user",
                Role::Assistant => "assistant",
                _ => continue,
            };
            match v.last_mut() {
                Some((r, t)) if r == role => {
                    t.push_str("\n\n");
                    t.push_str(&m.text);
                }
                _ => v.push((role.to_string(), m.text.clone())),
            }
        }
        while v.first().is_some_and(|(r, _)| r != "user") {
            v.remove(0);
        }
        v
    }

    fn answer_own(&self, id: &str, key: &Key, cancel: &AtomicBool) -> Result<(), String> {
        let c = self.load_own(id)?;
        let turns = Self::turns(&c);
        if c.provider == "openai" {
            let client = self.openai.client()?;
            let s = self.openai.settings();
            let body = openai::body(&c.model, SYSTEM, &turns, &s.effort, s.web);
            let reply = client
                .stream(&body, cancel, &mut |d| match d {
                    openai::Delta::Text(t) => self.delta(key, "text", &t),
                    openai::Delta::Search => self.delta(key, "tool", "Searched the web"),
                })
                .map_err(|e| e.to_string())?;
            let tokens = usage::Tokens { input: reply.usage.input.saturating_sub(reply.usage.cached), cache_read: reply.usage.cached, output: reply.usage.output, cache_write: 0, cost: 0.0 };
            self.usage.record(None, Some(tokens));
            self.edit_own(id, |c| {
                c.openai_usage.add(&reply.usage);
                if !reply.model.is_empty() {
                    c.model = reply.model.clone();
                }
                c.messages.push(Message { role: Role::Assistant, text: reply.text.trim().to_string(), at: now(), tools: Vec::new(), tone: None });
                if let Some(why) = &reply.incomplete {
                    let text = if why == "max_output_tokens" { "The answer reached its length limit. Say “continue” to get the rest.".to_string() } else { format!("The answer stopped early ({why}).") };
                    c.messages.push(Message { role: Role::Notice, text, at: now(), tools: Vec::new(), tone: Some("info".into()) });
                }
            })?;
            return Ok(());
        }

        let client = self.claude.client()?;
        let settings = self.claude.settings();
        let model = c.model.clone();
        let t = crate::claude::traits(&model);
        let mut betas: Vec<&str> = Vec::new();
        if t.fallbacks {
            betas.push("server-side-fallback-2026-07-01");
        }
        if t.updates {
            betas.push("thinking-display-updates-2026-08-18");
        }
        let mut msgs: Vec<Value> = turns.iter().map(|(r, t)| json!({ "role": r, "content": t })).collect();
        let mut text = String::new();
        let mut total = api::Usage::default();
        let mut cost = 0.0;
        let mut notice: Option<(String, &str)> = None;
        for _ in 0..8 {
            let mut body = json!({
                "model": model,
                "max_tokens": 32000,
                "stream": true,
                "thinking": { "type": "adaptive", "display": if t.updates { "updates" } else { "summarized" } },
                "output_config": { "effort": settings.effort },
                "cache_control": { "type": "ephemeral" },
                "system": [{ "type": "text", "text": SYSTEM }],
                "messages": msgs,
            });
            if settings.web {
                body["tools"] = json!([{ "type": t.web_search, "name": "web_search", "max_uses": 5 }, { "type": t.web_fetch, "name": "web_fetch", "max_uses": 5 }]);
            }
            if t.fallbacks {
                body["fallbacks"] = json!("default");
            }
            let turn = client
                .stream(&body, &betas, cancel, &mut |d| match d {
                    Delta::Text(s) => self.delta(key, "text", &s),
                    Delta::Note(_) => self.delta(key, "note", "Thinking…"),
                    Delta::ServerTool { name } => self.delta(key, "tool", if name == "web_fetch" { "Read a web page" } else { "Searched the web" }),
                    Delta::Tool { .. } => {}
                    Delta::Fallback { to } => self.delta(key, "note", &format!("Answering with {to}…")),
                })
                .map_err(|e| e.to_string())?;
            let mut u = turn.usage;
            if u.web_searches == 0 {
                u.web_searches = turn.content.iter().filter(|b| b["type"] == "server_tool_use" && b["name"] == "web_search").count() as u64;
            }
            let served = if turn.model.is_empty() { model.clone() } else { turn.model.clone() };
            total.add(&u);
            cost += crate::claude::cost(&served, &u);
            if turn.stop_reason == "refusal" {
                notice = Some(("Claude declined to answer that.".into(), "warn"));
                break;
            }
            let content = api::after_fallback(turn.content);
            let piece: Vec<&str> = content.iter().filter(|b| b["type"] == "text").filter_map(|b| b["text"].as_str()).collect();
            text.push_str(&piece.concat());
            if turn.stop_reason == "pause_turn" {
                msgs.push(json!({ "role": "assistant", "content": content }));
                continue;
            }
            if turn.stop_reason == "max_tokens" {
                notice = Some(("The answer reached its length limit. Say “continue” to get the rest.".into(), "info"));
            }
            break;
        }
        self.usage.record(Some(usage::Tokens { input: total.input, output: total.output, cache_read: total.cache_read, cache_write: total.cache_write, cost }), None);
        self.edit_own(id, |c| {
            c.claude_usage.add(&total);
            c.cost += cost;
            if !text.trim().is_empty() {
                c.messages.push(Message { role: Role::Assistant, text: text.trim().to_string(), at: now(), tools: Vec::new(), tone: None });
            }
            if let Some((n, tone)) = notice {
                c.messages.push(Message { role: Role::Notice, text: n, at: now(), tools: Vec::new(), tone: Some(tone.into()) });
            }
        })?;
        Ok(())
    }

    // ---------- Claude Code and Codex ----------

    fn agent_path(&self, source: Source, id: &str) -> Option<PathBuf> {
        let (ch, xh) = self.homes();
        match source {
            Source::ClaudeCode => claude_code::find(&ch?, id),
            Source::Codex => codex::find(&xh?, id),
            Source::Omni => None,
        }
    }

    fn agent_summary(&self, source: Source, path: &Path, names: &HashMap<String, String>) -> Option<ChatSummary> {
        let (len, m) = modified(path);
        if let Some((l, mm, s)) = self.infos.lock().get(path) {
            if *l == len && *mm == m {
                return Some(s.clone());
            }
        }
        let s = match source {
            Source::ClaudeCode => {
                let id = path.file_stem()?.to_str()?.to_string();
                let i = claude_code::info(path);
                ChatSummary { source: Some(source), title: ellipsize(&i.title().unwrap_or_else(|| "Claude Code conversation".into()), 90), preview: ellipsize(i.last.as_deref().unwrap_or_default(), 140), updated: m, folder: i.cwd, model: i.model, branch: i.branch, id, ..Default::default() }
            }
            Source::Codex => {
                let id = codex::id_of(path)?;
                let i = codex::info(path);
                let title = names.get(&id).cloned().or(i.first.clone()).unwrap_or_else(|| "Codex conversation".into());
                ChatSummary { source: Some(source), title: ellipsize(&title, 90), preview: ellipsize(i.last.as_deref().unwrap_or_default(), 140), updated: m, folder: i.cwd, model: i.model, branch: i.branch, id, ..Default::default() }
            }
            Source::Omni => return None,
        };
        self.infos.lock().insert(path.to_path_buf(), (len, m, s.clone()));
        Some(s)
    }

    /// The newest `limit` agent conversations of one kind.
    fn agent_list(&self, source: Source, limit: usize) -> Vec<ChatSummary> {
        let (ch, xh) = self.homes();
        let (files, names) = match source {
            Source::ClaudeCode => (ch.as_deref().map(claude_code::files).unwrap_or_default(), HashMap::new()),
            Source::Codex => match xh.as_deref() {
                Some(h) => (codex::files(h), codex::names(h)),
                None => (Vec::new(), HashMap::new()),
            },
            Source::Omni => return Vec::new(),
        };
        let mut files: Vec<(i64, PathBuf)> = files.into_iter().map(|p| (modified(&p).1, p)).collect();
        files.sort_by_key(|f| std::cmp::Reverse(f.0));
        // Codex keeps a resumed session's later turns in a new file with the same id.
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for (_, p) in files {
            if out.len() >= limit {
                break;
            }
            if let Some(mut s) = self.agent_summary(source, &p, &names) {
                if seen.insert(s.id.clone()) {
                    s.running = self.running(&(source, s.id.clone()));
                    out.push(s);
                }
            }
        }
        out
    }

    /// Every conversation, newest first.
    pub fn list(&self) -> Vec<ChatSummary> {
        let mut v: Vec<ChatSummary> = std::fs::read_dir(self.dir.join("omni"))
            .map(|rd| rd.flatten().filter_map(|e| e.path().file_stem().and_then(|s| s.to_str()).and_then(|id| self.load_own(id).ok())).map(|c| self.own_summary(&c)).collect())
            .unwrap_or_default();
        v.extend(self.agent_list(Source::ClaudeCode, 300));
        v.extend(self.agent_list(Source::Codex, 300));
        // Conversations just started whose files don't exist yet.
        for ((source, id), r) in self.runs.lock().iter() {
            if !v.iter().any(|s| s.source == Some(*source) && &s.id == id) && !id.starts_with("pending-") {
                v.push(ChatSummary { source: Some(*source), id: id.clone(), title: ellipsize(&r.live.prompt, 90), updated: r.live.started, running: true, ..Default::default() });
            }
        }
        v.sort_by_key(|s| std::cmp::Reverse(s.updated));
        v
    }

    pub fn view(&self, source: Source, id: &str) -> Result<ChatView, String> {
        let key = (source, id.to_string());
        let live = self.runs.lock().get(&key).map(|r| r.live.clone());
        if source == Source::Omni {
            let c = self.load_own(id)?;
            let (messages, earlier) = Builder { msgs: c.messages.clone() }.finish(600, false);
            return Ok(ChatView { summary: self.own_summary(&c), messages, earlier, live, busy_elsewhere: false });
        }
        let Some(path) = self.agent_path(source, id) else {
            return match live {
                Some(l) => Ok(ChatView { summary: ChatSummary { source: Some(source), id: id.to_string(), title: ellipsize(&l.prompt, 90), updated: l.started, running: true, ..Default::default() }, live: Some(l), ..Default::default() }),
                None => Err("That conversation isn't on this PC any more.".into()),
            };
        };
        let mut summary = self.agent_summary(source, &path, &if source == Source::Codex { self.homes().1.map(|h| codex::names(&h)).unwrap_or_default() } else { HashMap::new() }).ok_or("That conversation can't be read.")?;
        summary.running = live.is_some();
        let (messages, earlier) = match source {
            Source::ClaudeCode => claude_code::transcript(&path, 400),
            _ => codex::transcript(&path, 400),
        };
        let m = modified(&path).1;
        let ours = self.ours.lock().get(&key).copied();
        let busy_elsewhere = live.is_none() && now() - m < 120 && ours.is_none_or(|t| m > t + 5);
        Ok(ChatView { summary, messages, earlier, live, busy_elsewhere })
    }

    /// Folders Claude Code and Codex have worked in, most recent first.
    pub fn folders(&self) -> Vec<Folder> {
        let mut m: HashMap<String, Folder> = HashMap::new();
        for s in self.agent_list(Source::ClaudeCode, 300).into_iter().chain(self.agent_list(Source::Codex, 300)) {
            let Some(path) = s.folder.clone() else { continue };
            let name = Path::new(&path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.clone());
            let f = m.entry(path.to_lowercase()).or_insert(Folder { path, name, last: 0, claude_code: false, codex: false });
            f.last = f.last.max(s.updated);
            match s.source {
                Some(Source::ClaudeCode) => f.claude_code = true,
                Some(Source::Codex) => f.codex = true,
                _ => {}
            }
        }
        let mut v: Vec<Folder> = m.into_values().filter(|f| Path::new(&f.path).is_dir()).collect();
        v.sort_by_key(|f| std::cmp::Reverse(f.last));
        v
    }

    /// Whether `folder` is one Claude Code or Codex already works in.
    pub fn known_folder(&self, folder: &str) -> bool {
        let f = folder.trim_end_matches(['\\', '/']).to_lowercase();
        self.folders().iter().any(|k| k.path.trim_end_matches(['\\', '/']).to_lowercase() == f)
    }

    /// Send a message to an existing conversation; the answer streams in as
    /// `chats:delta` events.
    pub fn send(self: &Arc<Self>, source: Source, id: &str, text: &str, mode: &str) -> Result<(), String> {
        let text = check_text(text)?;
        let key = (source, id.to_string());
        if source == Source::Omni {
            self.load_own(id)?;
            let cancel = self.start_run(&key, &text)?;
            if let Err(e) = self.edit_own(id, |c| c.messages.push(Message { role: Role::User, text: text.clone(), at: now(), tools: Vec::new(), tone: None })) {
                self.runs.lock().remove(&key);
                return Err(e);
            }
            self.emit_changed(source, id);
            let me = self.clone();
            let id = id.to_string();
            std::thread::Builder::new().name("chat".into()).spawn(move || me.run_own(id, cancel)).map_err(|e| e.to_string())?;
            return Ok(());
        }
        let path = self.agent_path(source, id).ok_or("That conversation isn't on this PC any more.")?;
        let folder = match source {
            Source::ClaudeCode => claude_code::info(&path).cwd,
            _ => codex::info(&path).cwd,
        };
        self.run_agent(source, Target::Resume(id.to_string()), folder.map(PathBuf::from), text, mode, id.to_string(), None)
    }

    /// Start a conversation. Claude Code's id is chosen up front; Codex's
    /// arrives once it starts, and this waits for it.
    pub fn create(self: &Arc<Self>, kind: NewKind, folder: Option<&str>, text: &str, mode: &str) -> Result<ChatSummary, String> {
        let text = check_text(text)?;
        match kind {
            NewKind::Claude | NewKind::Openai => {
                let (provider, model) = if kind == NewKind::Claude {
                    if !self.claude.has_key() {
                        return Err("Add your Anthropic API key first (Chats → Keys).".into());
                    }
                    ("claude", self.claude.settings().model)
                } else {
                    if !self.openai.has_key() {
                        return Err("Add your OpenAI API key first (Chats → Keys).".into());
                    }
                    ("openai", self.openai.settings().model)
                };
                let id = uuid::Uuid::new_v4().simple().to_string();
                let c = OwnChat { id: id.clone(), title: ellipsize(text.lines().next().unwrap_or("New chat"), 60), provider: provider.into(), model, created: now(), updated: now(), ..Default::default() };
                self.save_own(&c)?;
                self.send(Source::Omni, &id, &text, mode)?;
                Ok(self.own_summary(&self.load_own(&id)?))
            }
            NewKind::ClaudeCode | NewKind::Codex => {
                let source = if kind == NewKind::ClaudeCode { Source::ClaudeCode } else { Source::Codex };
                let dir = PathBuf::from(folder.unwrap_or_default());
                if !dir.is_absolute() || !dir.is_dir() {
                    return Err("Pick the project folder it should work in.".into());
                }
                if source == Source::ClaudeCode {
                    let id = uuid::Uuid::new_v4().to_string();
                    self.run_agent(source, Target::NewClaude(id.clone()), Some(dir.clone()), text.clone(), mode, id.clone(), None)?;
                    return Ok(ChatSummary { source: Some(source), id, title: ellipsize(&text, 90), updated: now(), folder: Some(dir.to_string_lossy().into_owned()), running: true, ..Default::default() });
                }
                let (tx, rx) = crossbeam_channel::bounded::<Result<String, String>>(1);
                let pending = format!("pending-{}", uuid::Uuid::new_v4().simple());
                self.run_agent(source, Target::NewCodex, Some(dir.clone()), text.clone(), mode, pending, Some(tx))?;
                let id = rx.recv_timeout(std::time::Duration::from_secs(120)).map_err(|_| "Codex didn't start in time.".to_string())??;
                Ok(ChatSummary { source: Some(source), id, title: ellipsize(&text, 90), updated: now(), folder: Some(dir.to_string_lossy().into_owned()), running: true, ..Default::default() })
            }
        }
    }

    /// Run Claude Code or Codex for one message. `key_id` names the run
    /// until Codex says its id (`started` gets it).
    #[allow(clippy::too_many_arguments)]
    fn run_agent(self: &Arc<Self>, source: Source, target: Target, folder: Option<PathBuf>, text: String, mode: &str, key_id: String, started: Option<crossbeam_channel::Sender<Result<String, String>>>) -> Result<(), String> {
        let exe = self.exe(source).ok_or(match source {
            Source::ClaudeCode => "Claude Code isn't installed on this PC (OmniHub looks for `claude` on PATH and where its installer puts it).",
            _ => "Codex isn't installed on this PC (OmniHub looks for `codex` on PATH and where npm puts it).",
        })?;
        let args = match (&target, source) {
            (Target::Resume(id), Source::ClaudeCode) => claude_code::args(Some(id), None, mode),
            (Target::NewClaude(id), _) => claude_code::args(None, Some(id), mode),
            (Target::Resume(id), _) => codex::args(Some(id), mode),
            (Target::NewCodex, _) => codex::args(None, mode),
        };
        let key = (source, key_id);
        let cancel = self.start_run(&key, &text)?;
        self.emit_changed(source, &key.1);
        let me = self.clone();
        std::thread::Builder::new()
            .name("chat-agent".into())
            .spawn(move || {
                let mut key = key;
                let mut started = started;
                let mut error: Option<String> = None;
                let mut finished = false;
                let mut cc = claude_code::Reader::default();
                let mut cx = codex::Reader::default();
                let res = agent::run(&exe, &args, folder.as_deref(), &text, &cancel, &mut |v| {
                    if source == Source::ClaudeCode {
                        for e in cc.read(v) {
                            match e {
                                claude_code::Event::Text(t) => me.delta(&key, "text", &t),
                                claude_code::Event::Tool(t) => me.delta(&key, "tool", &t),
                                claude_code::Event::Note(t) => me.delta(&key, "note", &t),
                                claude_code::Event::Result { ok, text, denied, .. } => {
                                    finished = true;
                                    if !denied.is_empty() {
                                        me.delta(&key, "notice", &format!("It needed your OK for: {}. Pick a mode that allows it, or allow it in Claude Code's settings.", denied.join("; ")));
                                    }
                                    if !ok {
                                        error = Some(if text.trim().is_empty() { "Claude Code couldn't finish.".into() } else { text });
                                    }
                                }
                            }
                        }
                    } else {
                        for e in cx.read(v) {
                            match e {
                                codex::Event::Started(id) => {
                                    if key.1.starts_with("pending-") {
                                        let mut runs = me.runs.lock();
                                        if let Some(r) = runs.remove(&key) {
                                            key = (source, id.clone());
                                            runs.insert(key.clone(), r);
                                        }
                                    }
                                    if let Some(tx) = started.take() {
                                        let _ = tx.send(Ok(id));
                                    }
                                }
                                codex::Event::Text(t) => me.delta(&key, "text", &t),
                                codex::Event::Tool(t) => me.delta(&key, "tool", &t),
                                codex::Event::Note(t) => me.delta(&key, "note", &t),
                                codex::Event::Notice(t) => me.delta(&key, "notice", &t),
                                codex::Event::Done { error: e, .. } => {
                                    finished = true;
                                    error = e;
                                }
                            }
                        }
                    }
                });
                let error = match res {
                    Err(e) => Some(e),
                    Ok(end) if end.cancelled => None,
                    Ok(end) if !finished => Some(match end.stderr.lines().rev().find(|l| !l.trim().is_empty()) {
                        Some(l) => format!("{} stopped: {}", source.label(), ellipsize(l, 300)),
                        None => format!("{} stopped without answering (exit code {}).", source.label(), end.code.map_or("unknown".into(), |c| c.to_string())),
                    }),
                    Ok(_) => error,
                };
                if let Some(tx) = started.take() {
                    let _ = tx.send(Err(error.clone().unwrap_or_else(|| "Codex didn't start.".into())));
                }
                me.end_run(&key, error);
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// What the mod maker's projects have used, all told.
    pub fn maker_tokens(projects: &[crate::mods::maker::Project]) -> usage::Tokens {
        let mut t = usage::Tokens::default();
        for p in projects {
            t.add(&usage::Tokens { input: p.usage.input, output: p.usage.output, cache_read: p.usage.cache_read, cache_write: p.usage.cache_write, cost: p.cost });
        }
        t
    }

    pub fn usage_report(&self, maker: usage::Tokens) -> usage::Report {
        let (ch, xh) = self.homes();
        self.usage.report(ch.as_deref(), xh.as_deref(), maker, now())
    }
}

/// Save text as an idea (a note of kind Idea), noting where it came from,
/// and copy it to the Claude folder when ideas go there automatically.
pub fn save_idea(core: &crate::core::AppCore, text: &str, from: Option<&str>) -> Result<crate::notes::Note, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("The idea is empty.".into());
    }
    let title = ellipsize(text.lines().find(|l| !l.trim().is_empty()).unwrap_or("Idea").trim_start_matches(['#', ' ', '*', '-']), 80);
    let body = match from {
        Some(f) if !f.trim().is_empty() => format!("{text}\n\n— from {}", ellipsize(f, 120)),
        _ => text.to_string(),
    };
    let note = core.notes.save(crate::notes::NoteInput { id: None, kind: crate::notes::NoteKind::Idea, title, body, tags: vec!["chat".into()], pinned: false, color: None }).map_err(|e| e.to_string())?;
    let s = core.settings.get().notes;
    match (s.auto_export_ideas, s.claude_folder) {
        (true, Some(folder)) => core.notes.export(&note.id, Path::new(&folder), &core.export_options()).map_err(|e| e.to_string()),
        _ => Ok(note),
    }
}
