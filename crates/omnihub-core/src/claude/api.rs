//! The Claude Messages API over HTTPS (Rust has no official Anthropic SDK,
//! so this is the documented raw-HTTP shape): one streamed request at a
//! time, read as server-sent events and put back together into the
//! response's content blocks exactly as sent, so they can be passed back
//! unchanged on the next turn (thinking blocks keep their signatures).

use std::io::{BufRead, BufReader, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde_json::{json, Map, Value};

pub const BASE: &str = "https://api.anthropic.com";
const VERSION: &str = "2023-06-01";

/// What a turn used, for the running cost.
#[derive(Debug, Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub web_searches: u64,
}

impl Usage {
    pub fn add(&mut self, o: &Usage) {
        self.input += o.input;
        self.output += o.output;
        self.cache_read += o.cache_read;
        self.cache_write += o.cache_write;
        self.web_searches += o.web_searches;
    }

    fn read(u: &Value, into: &mut Usage) {
        let n = |k: &str| u.get(k).and_then(Value::as_u64);
        if let Some(v) = n("input_tokens") {
            into.input = v;
        }
        if let Some(v) = n("output_tokens") {
            into.output = v;
        }
        if let Some(v) = n("cache_read_input_tokens") {
            into.cache_read = v;
        }
        if let Some(v) = n("cache_creation_input_tokens") {
            into.cache_write = v;
        }
        if let Some(v) = u.get("server_tool_use").and_then(|s| s.get("web_search_requests")).and_then(Value::as_u64) {
            into.web_searches = v;
        }
    }
}

/// One finished response.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Turn {
    pub content: Vec<Value>,
    pub stop_reason: String,
    pub stop_details: Value,
    pub usage: Usage,
    pub model: String,
}

/// What streams in, for the window to show as it comes.
#[derive(Debug, Clone, PartialEq)]
pub enum Delta {
    Text(String),
    /// A progress note (a thinking block's text under `display: "updates"`).
    Note(String),
    /// A tool call started (`name`); its input is still coming.
    Tool { id: String, name: String },
    /// A server tool (web search / fetch) started.
    ServerTool { name: String },
    /// A refused turn moved to a fallback model.
    Fallback { to: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ApiError {
    /// The key was refused (401/403).
    Key(String),
    /// Out of credit or billing problem (402).
    Billing(String),
    /// Too many requests or overloaded, after retries.
    Busy(String),
    /// The request itself was wrong (400/404/413).
    Request(String),
    /// The connection failed or the stream broke.
    Network(String),
    Cancelled,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Key(m) => write!(f, "Anthropic refused your API key: {m}. Check it in the Claude settings."),
            ApiError::Billing(m) => write!(f, "Anthropic says there's a billing problem with your account: {m}"),
            ApiError::Busy(m) => write!(f, "Claude is busy right now ({m}). Try again in a minute."),
            ApiError::Request(m) => write!(f, "Claude couldn't take that request: {m}"),
            ApiError::Network(m) => write!(f, "Couldn't reach Claude: {m}"),
            ApiError::Cancelled => write!(f, "Stopped."),
        }
    }
}

/// Puts streamed content blocks back together.
#[derive(Default)]
struct Blocks {
    blocks: Vec<Value>,
    /// Tool input JSON as it streams, by block index.
    partial: Vec<String>,
}

impl Blocks {
    fn start(&mut self, index: usize, block: Value) {
        while self.blocks.len() <= index {
            self.blocks.push(Value::Null);
            self.partial.push(String::new());
        }
        self.blocks[index] = block;
    }

    fn delta(&mut self, index: usize, d: &Value) -> Option<Delta> {
        let b = self.blocks.get_mut(index)?.as_object_mut()?;
        let append = |b: &mut Map<String, Value>, k: &str, s: &str| {
            let cur = b.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
            b.insert(k.into(), Value::String(cur + s));
        };
        match d["type"].as_str()? {
            "text_delta" => {
                let t = d["text"].as_str()?;
                append(b, "text", t);
                Some(Delta::Text(t.to_string()))
            }
            "thinking_delta" => {
                let t = d["thinking"].as_str()?;
                append(b, "thinking", t);
                (!t.is_empty()).then(|| Delta::Note(t.to_string()))
            }
            "signature_delta" => {
                append(b, "signature", d["signature"].as_str()?);
                None
            }
            "input_json_delta" => {
                self.partial[index].push_str(d["partial_json"].as_str().unwrap_or_default());
                None
            }
            "citations_delta" => {
                let c = d.get("citation")?.clone();
                match b.get_mut("citations").and_then(Value::as_array_mut) {
                    Some(a) => a.push(c),
                    None => {
                        b.insert("citations".into(), Value::Array(vec![c]));
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// A block is complete: tool inputs become JSON. A tool input that isn't
    /// valid JSON (cut off, or malformed) is kept as `{"INVALID_JSON": text}`
    /// for the caller to report back instead of running.
    fn stop(&mut self, index: usize) {
        let Some(b) = self.blocks.get_mut(index).and_then(Value::as_object_mut) else { return };
        let kind = b.get("type").and_then(Value::as_str).unwrap_or_default();
        if matches!(kind, "tool_use" | "server_tool_use") {
            let raw = std::mem::take(&mut self.partial[index]);
            if !raw.trim().is_empty() {
                let input = serde_json::from_str::<Value>(&raw).ok().filter(Value::is_object).unwrap_or_else(|| json!({ "INVALID_JSON": raw }));
                b.insert("input".into(), input);
            } else if !b.contains_key("input") {
                b.insert("input".into(), json!({}));
            }
        }
    }

    fn finish(self) -> Vec<Value> {
        self.blocks.into_iter().filter(|b| !b.is_null()).collect()
    }
}

/// After a refused turn moved to a fallback model mid-answer, the blocks
/// before the last `fallback` marker that the next model can't take back
/// (thinking, tool calls, server tool calls without their results) are left
/// out of what's echoed back, as the API asks.
pub fn after_fallback(content: Vec<Value>) -> Vec<Value> {
    let Some(last) = content.iter().rposition(|b| b["type"] == "fallback") else { return content };
    let results: std::collections::HashSet<String> = content.iter().filter(|b| b["type"].as_str().is_some_and(|t| t.ends_with("_tool_result"))).filter_map(|b| b["tool_use_id"].as_str().map(String::from)).collect();
    content
        .into_iter()
        .enumerate()
        .filter(|(i, b)| {
            if *i >= last {
                return true;
            }
            match b["type"].as_str().unwrap_or_default() {
                "text" | "fallback" => true,
                "server_tool_use" => b["id"].as_str().is_some_and(|id| results.contains(id)),
                t if t.ends_with("_tool_result") => true,
                _ => false,
            }
        })
        .map(|(_, b)| b)
        .collect()
}

pub struct Client {
    base: String,
    key: zeroize::Zeroizing<String>,
    agent: ureq::Agent,
    /// Waits before retrying 429/5xx (shortened in tests).
    pub backoff: Duration,
}

impl Client {
    pub fn new(key: &str) -> Client {
        Client::with_base(BASE, key)
    }

    pub fn with_base(base: &str, key: &str) -> Client {
        let mut cfg = ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(20))).timeout_recv_response(Some(Duration::from_secs(300))).timeout_recv_body(Some(Duration::from_secs(1800))).http_status_as_error(false);
        if base.starts_with("http://127.0.0.1") {
            cfg = cfg.proxy(None);
        }
        Client { base: base.trim_end_matches('/').to_string(), key: zeroize::Zeroizing::new(key.trim().to_string()), agent: cfg.build().into(), backoff: Duration::from_secs(2) }
    }

    /// Check a key with a tiny request.
    pub fn check(&self, model: &str) -> Result<(), ApiError> {
        let body = json!({ "model": model, "max_tokens": 16, "messages": [{ "role": "user", "content": "Say OK." }] });
        let mut res = self.post("/v1/messages", &body, &[])?;
        let status = res.status().as_u16();
        let text = res.body_mut().read_to_string().unwrap_or_default();
        if (200..300).contains(&status) {
            Ok(())
        } else {
            Err(classify(status, &text))
        }
    }

    fn post(&self, path: &str, body: &Value, betas: &[&str]) -> Result<ureq::http::Response<ureq::Body>, ApiError> {
        let mut req = self.agent.post(&format!("{}{path}", self.base)).header("x-api-key", self.key.as_str()).header("anthropic-version", VERSION).header("content-type", "application/json");
        if !betas.is_empty() {
            req = req.header("anthropic-beta", betas.join(","));
        }
        req.send(body.to_string()).map_err(|e| ApiError::Network(e.to_string()))
    }

    /// Send one streamed request; `on` sees text, progress notes and tool
    /// starts as they arrive. Busy answers (429, 5xx, 529) before the stream
    /// starts are retried three times, honouring `retry-after`.
    pub fn stream(&self, body: &Value, betas: &[&str], cancel: &AtomicBool, on: &mut dyn FnMut(Delta)) -> Result<Turn, ApiError> {
        let mut attempt = 0;
        let res = loop {
            if cancel.load(Ordering::SeqCst) {
                return Err(ApiError::Cancelled);
            }
            let mut res = self.post("/v1/messages", body, betas)?;
            let status = res.status().as_u16();
            if (200..300).contains(&status) {
                break res;
            }
            let wait = res.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse::<u64>().ok());
            let text = res.body_mut().read_to_string().unwrap_or_default();
            let err = classify(status, &text);
            attempt += 1;
            if !matches!(err, ApiError::Busy(_)) || attempt > 3 {
                return Err(err);
            }
            let pause = wait.map(Duration::from_secs).unwrap_or(self.backoff * 2u32.pow(attempt - 1)).min(Duration::from_secs(60));
            let until = std::time::Instant::now() + pause;
            while std::time::Instant::now() < until {
                if cancel.load(Ordering::SeqCst) {
                    return Err(ApiError::Cancelled);
                }
                std::thread::sleep(Duration::from_millis(50).min(pause));
            }
        };
        read_events(res.into_body().into_reader(), cancel, on)
    }
}

/// An error answer in plain words, and whether it's worth retrying.
pub fn classify(status: u16, body: &str) -> ApiError {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let kind = v["error"]["type"].as_str().unwrap_or_default();
    let msg = v["error"]["message"].as_str().map(String::from).unwrap_or_else(|| format!("HTTP {status}"));
    match (status, kind) {
        (401, _) | (403, _) | (_, "authentication_error") | (_, "permission_error") => ApiError::Key(msg),
        (402, _) | (_, "billing_error") => ApiError::Billing(msg),
        (429, _) | (500..=599, _) | (_, "overloaded_error") | (_, "rate_limit_error") | (_, "api_error") => ApiError::Busy(msg),
        _ => ApiError::Request(msg),
    }
}

/// Read a server-sent event stream into a turn.
pub fn read_events(reader: impl Read, cancel: &AtomicBool, on: &mut dyn FnMut(Delta)) -> Result<Turn, ApiError> {
    let mut lines = BufReader::new(reader);
    let mut turn = Turn::default();
    let mut blocks = Blocks::default();
    let mut data = String::new();
    let mut line = String::new();
    let mut stopped = false;
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(ApiError::Cancelled);
        }
        line.clear();
        let n = lines.read_line(&mut line).map_err(|e| ApiError::Network(format!("the answer stopped coming: {e}")))?;
        let end = n == 0;
        let l = line.trim_end_matches(['\r', '\n']);
        if let Some(d) = l.strip_prefix("data:") {
            data.push_str(d.trim_start());
            if !end {
                continue;
            }
        }
        if !(l.is_empty() || end) {
            continue;
        }
        if !data.is_empty() {
            let ev: Value = serde_json::from_str(&data).map_err(|e| ApiError::Network(format!("an unreadable event: {e}")))?;
            data.clear();
            let idx = ev["index"].as_u64().unwrap_or(0) as usize;
            match ev["type"].as_str().unwrap_or_default() {
                "message_start" => {
                    turn.model = ev["message"]["model"].as_str().unwrap_or_default().to_string();
                    Usage::read(&ev["message"]["usage"], &mut turn.usage);
                }
                "content_block_start" => {
                    let b = ev["content_block"].clone();
                    match b["type"].as_str().unwrap_or_default() {
                        "tool_use" => on(Delta::Tool { id: b["id"].as_str().unwrap_or_default().into(), name: b["name"].as_str().unwrap_or_default().into() }),
                        "server_tool_use" => on(Delta::ServerTool { name: b["name"].as_str().unwrap_or_default().into() }),
                        "fallback" => on(Delta::Fallback { to: b["to"]["model"].as_str().unwrap_or_default().into() }),
                        _ => {}
                    }
                    blocks.start(idx, b);
                }
                "content_block_delta" => {
                    if let Some(d) = blocks.delta(idx, &ev["delta"]) {
                        on(d);
                    }
                }
                "content_block_stop" => blocks.stop(idx),
                "message_delta" => {
                    if let Some(r) = ev["delta"]["stop_reason"].as_str() {
                        turn.stop_reason = r.to_string();
                    }
                    turn.stop_details = ev["delta"]["stop_details"].clone();
                    Usage::read(&ev["usage"], &mut turn.usage);
                }
                "message_stop" => {
                    stopped = true;
                    break;
                }
                "error" => {
                    let kind = ev["error"]["type"].as_str().unwrap_or_default();
                    let msg = ev["error"]["message"].as_str().unwrap_or("error").to_string();
                    return Err(if matches!(kind, "overloaded_error" | "rate_limit_error" | "api_error") { ApiError::Busy(msg) } else { ApiError::Request(msg) });
                }
                _ => {}
            }
        }
        if end {
            break;
        }
    }
    if !stopped {
        return Err(ApiError::Network("the answer stopped before it finished".into()));
    }
    turn.content = blocks.finish();
    Ok(turn)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Server-sent events for a list of event objects.
    pub fn sse(events: &[Value]) -> String {
        events.iter().map(|e| format!("event: {}\ndata: {}\n\n", e["type"].as_str().unwrap_or("x"), e)).collect()
    }

    #[test]
    fn streams_are_put_back_together() {
        let events = vec![
            json!({"type":"message_start","message":{"model":"claude-opus-5-5","usage":{"input_tokens":120,"cache_read_input_tokens":4000,"cache_creation_input_tokens":0,"output_tokens":1}}}),
            json!({"type":"ping"}),
            json!({"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Looking at the game's card code."}}),
            json!({"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig-abc"}}),
            json!({"type":"content_block_stop","index":0}),
            json!({"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}),
            json!({"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"I'll write "}}),
            json!({"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"the joker."}}),
            json!({"type":"content_block_delta","index":1,"delta":{"type":"citations_delta","citation":{"type":"web_search_result_location","url":"https://smods.dev"}}}),
            json!({"type":"content_block_stop","index":1}),
            json!({"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"toolu_1","name":"write_file","input":{}}}),
            json!({"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"path\": \"mod/main.lua\", "}}),
            json!({"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"\"content\": \"print(1)\"}"}}),
            json!({"type":"content_block_stop","index":2}),
            json!({"type":"content_block_start","index":3,"content_block":{"type":"tool_use","id":"toolu_2","name":"read_file","input":{}}}),
            json!({"type":"content_block_delta","index":3,"delta":{"type":"input_json_delta","partial_json":"{\"path\": \"game/x"}}),
            json!({"type":"content_block_stop","index":3}),
            json!({"type":"message_delta","delta":{"stop_reason":"tool_use","stop_details":null},"usage":{"output_tokens":321}}),
            json!({"type":"message_stop"}),
        ];
        let mut seen = Vec::new();
        let turn = read_events(sse(&events).as_bytes(), &AtomicBool::new(false), &mut |d| seen.push(d)).unwrap();
        assert_eq!(turn.model, "claude-opus-5-5");
        assert_eq!(turn.stop_reason, "tool_use");
        assert_eq!(turn.usage, Usage { input: 120, output: 321, cache_read: 4000, cache_write: 0, web_searches: 0 });
        assert_eq!(turn.content[0], json!({"type":"thinking","thinking":"Looking at the game's card code.","signature":"sig-abc"}), "thinking comes back whole, signature included");
        assert_eq!(turn.content[1]["text"], "I'll write the joker.");
        assert_eq!(turn.content[1]["citations"][0]["url"], "https://smods.dev");
        assert_eq!(turn.content[2]["input"], json!({"path":"mod/main.lua","content":"print(1)"}));
        assert_eq!(turn.content[3]["input"], json!({"INVALID_JSON":"{\"path\": \"game/x"}), "a cut-off input is flagged, never guessed");
        assert_eq!(seen[0], Delta::Note("Looking at the game's card code.".into()));
        assert!(seen.contains(&Delta::Tool { id: "toolu_1".into(), name: "write_file".into() }));
        assert_eq!(seen.iter().filter(|d| matches!(d, Delta::Text(_))).count(), 2);

        // A stream that breaks off is an error, not a short answer.
        let cut = sse(&events[..8]);
        assert!(matches!(read_events(cut.as_bytes(), &AtomicBool::new(false), &mut |_| {}), Err(ApiError::Network(_))));
        // An error event mid-stream.
        let err = sse(&[events[0].clone(), json!({"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}})]);
        assert_eq!(read_events(err.as_bytes(), &AtomicBool::new(false), &mut |_| {}), Err(ApiError::Busy("Overloaded".into())));
        assert_eq!(read_events(sse(&events).as_bytes(), &AtomicBool::new(true), &mut |_| {}), Err(ApiError::Cancelled));
    }

    #[test]
    fn fallback_echo_and_errors() {
        let content = vec![
            json!({"type":"thinking","thinking":"","signature":"s"}),
            json!({"type":"text","text":"Partial"}),
            json!({"type":"server_tool_use","id":"srv_1","name":"web_search","input":{"query":"q"}}),
            json!({"type":"web_search_tool_result","tool_use_id":"srv_1","content":[]}),
            json!({"type":"server_tool_use","id":"srv_2","name":"web_search","input":{"query":"q2"}}),
            json!({"type":"tool_use","id":"t","name":"x","input":{}}),
            json!({"type":"fallback","from":{"model":"claude-opus-5-5"},"to":{"model":"claude-opus-5"}}),
            json!({"type":"thinking","thinking":"","signature":"s2"}),
            json!({"type":"text","text":"Rest"}),
        ];
        let kept: Vec<String> = after_fallback(content.clone()).iter().map(|b| b["type"].as_str().unwrap().to_string()).collect();
        assert_eq!(kept, ["text", "server_tool_use", "web_search_tool_result", "fallback", "thinking", "text"]);
        assert_eq!(after_fallback(content[..2].to_vec()).len(), 2, "no fallback, nothing changes");

        assert!(matches!(classify(401, r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#), ApiError::Key(m) if m == "invalid x-api-key"));
        assert!(matches!(classify(529, r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#), ApiError::Busy(_)));
        assert!(matches!(classify(429, ""), ApiError::Busy(_)));
        assert!(matches!(classify(402, ""), ApiError::Billing(_)));
        assert!(matches!(classify(400, r#"{"error":{"type":"invalid_request_error","message":"bad"}}"#), ApiError::Request(m) if m == "bad"));
    }
}
