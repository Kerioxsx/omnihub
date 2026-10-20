//! ChatGPT's models through OpenAI's Responses API, with the user's own
//! OpenAI API key. The key is kept like the Claude key: encrypted for this
//! Windows account with DPAPI, never shown to the window, sent only to
//! OpenAI. Conversations are sent whole each turn with `store: false`, so
//! OpenAI keeps nothing of them.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const BASE: &str = "https://api.openai.com";
const SECRET_ENTROPY: &[u8] = b"omnihub-openai";
/// Used until the key's own model list says otherwise.
pub const DEFAULT_MODEL: &str = "gpt-6-astra";
/// `default` leaves the reasoning effort to the model.
pub const EFFORTS: &[&str] = &["default", "low", "medium", "high"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct OpenAiSettings {
    /// The key, DPAPI-encrypted and base64'd. Cleared before the window sees it.
    pub api_secret: String,
    pub model: String,
    pub effort: String,
    /// Let the model search the web.
    pub web: bool,
    #[serde(skip_deserializing)]
    pub has_key: bool,
}

impl Default for OpenAiSettings {
    fn default() -> Self {
        OpenAiSettings { api_secret: String::new(), model: DEFAULT_MODEL.into(), effort: "default".into(), web: false, has_key: false }
    }
}

fn model_ok(m: &str) -> bool {
    !m.is_empty() && m.len() <= 80 && m.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | ':'))
}

impl OpenAiSettings {
    fn sanitize(&mut self) {
        if !model_ok(&self.model) {
            self.model = DEFAULT_MODEL.into();
        }
        if !EFFORTS.contains(&self.effort.as_str()) {
            self.effort = "default".into();
        }
        self.has_key = !self.api_secret.is_empty();
    }

    pub fn public(&self) -> OpenAiSettings {
        OpenAiSettings { api_secret: String::new(), has_key: !self.api_secret.is_empty(), ..self.clone() }
    }
}

pub struct OpenAi {
    file: PathBuf,
    settings: Mutex<OpenAiSettings>,
    base: String,
    #[cfg(test)]
    test_key: Mutex<Option<String>>,
}

impl OpenAi {
    pub fn new(data_dir: &Path) -> OpenAi {
        OpenAi::with_base(data_dir, BASE)
    }

    pub fn with_base(data_dir: &Path, base: &str) -> OpenAi {
        let file = data_dir.join("openai.json");
        let mut s: OpenAiSettings = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        s.sanitize();
        OpenAi {
            file,
            settings: Mutex::new(s),
            base: base.to_string(),
            #[cfg(test)]
            test_key: Mutex::new(None),
        }
    }

    pub fn settings(&self) -> OpenAiSettings {
        self.settings.lock().public()
    }

    fn save(&self, s: &OpenAiSettings) -> Result<(), String> {
        if let Some(p) = self.file.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let tmp = self.file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.file).map_err(|e| e.to_string())
    }

    /// Change the model, effort or web use (never the key).
    pub fn update(&self, patch: &Value) -> Result<OpenAiSettings, String> {
        let mut s = self.settings.lock();
        let mut v = serde_json::to_value(&*s).map_err(|e| e.to_string())?;
        if let (Some(cur), Some(p)) = (v.as_object_mut(), patch.as_object()) {
            for (k, val) in p {
                if k != "apiSecret" && k != "hasKey" {
                    cur.insert(k.clone(), val.clone());
                }
            }
        }
        let mut next: OpenAiSettings = serde_json::from_value(v).map_err(|e| format!("Bad settings: {e}"))?;
        next.api_secret = s.api_secret.clone();
        next.sanitize();
        self.save(&next)?;
        *s = next;
        Ok(s.public())
    }

    /// Keep the key encrypted for this Windows account; empty removes it.
    pub fn set_key(&self, key: &str) -> Result<(), String> {
        use base64::Engine;
        let key = key.trim();
        if !key.is_empty() && (key.len() > 400 || !key.starts_with("sk-") || key.starts_with("sk-ant-") || key.chars().any(|c| c.is_control() || c.is_whitespace())) {
            return Err("That doesn't look like an OpenAI API key (they start with sk-).".into());
        }
        let secret = if key.is_empty() { String::new() } else { base64::engine::general_purpose::STANDARD.encode(crate::system::dpapi::protect(key.as_bytes(), SECRET_ENTROPY).map_err(|e| e.to_string())?) };
        let mut s = self.settings.lock();
        let mut next = s.clone();
        next.api_secret = secret;
        next.sanitize();
        self.save(&next)?;
        *s = next;
        Ok(())
    }

    pub fn has_key(&self) -> bool {
        #[cfg(test)]
        if self.test_key.lock().is_some() {
            return true;
        }
        !self.settings.lock().api_secret.is_empty()
    }

    fn key(&self) -> Option<zeroize::Zeroizing<String>> {
        use base64::Engine;
        #[cfg(test)]
        if let Some(k) = self.test_key.lock().clone() {
            return Some(zeroize::Zeroizing::new(k));
        }
        let secret = self.settings.lock().api_secret.clone();
        if secret.is_empty() {
            return None;
        }
        let bytes = base64::engine::general_purpose::STANDARD.decode(secret).ok()?;
        let plain = crate::system::dpapi::unprotect(&bytes, SECRET_ENTROPY).ok()?;
        Some(zeroize::Zeroizing::new(String::from_utf8_lossy(&plain).into_owned()))
    }

    pub fn client(&self) -> Result<Client, String> {
        let key = self.key().ok_or("Add your OpenAI API key first (Chats → Keys).")?;
        Ok(Client::with_base(&self.base, &key))
    }

    /// The models the saved key can use (which also checks the key).
    pub fn models(&self) -> Result<Vec<String>, String> {
        self.client()?.models().map_err(|e| e.to_string())
    }

    #[cfg(test)]
    pub fn set_test_key(&self, key: &str) {
        *self.test_key.lock() = Some(key.to_string());
    }
}

/// Tokens a reply used.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Usage {
    pub input: u64,
    /// Of `input`, read from OpenAI's cache.
    pub cached: u64,
    pub output: u64,
    /// Of `output`, spent reasoning.
    pub reasoning: u64,
}

impl Usage {
    pub fn add(&mut self, o: &Usage) {
        self.input += o.input;
        self.cached += o.cached;
        self.output += o.output;
        self.reasoning += o.reasoning;
    }

    fn read(u: &Value) -> Usage {
        let n = |v: &Value| v.as_u64().unwrap_or(0);
        Usage { input: n(&u["input_tokens"]), cached: n(&u["input_tokens_details"]["cached_tokens"]), output: n(&u["output_tokens"]), reasoning: n(&u["output_tokens_details"]["reasoning_tokens"]) }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reply {
    pub text: String,
    pub usage: Usage,
    pub model: String,
    /// Why the reply stopped early (`max_output_tokens`, `content_filter`).
    pub incomplete: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Delta {
    Text(String),
    /// A web search started.
    Search,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OaError {
    Key(String),
    /// No credit left (`insufficient_quota`).
    Billing(String),
    Busy(String),
    Request(String),
    Network(String),
    Cancelled,
}

impl std::fmt::Display for OaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OaError::Key(m) => write!(f, "OpenAI refused your API key: {m}. Check it in Chats → Keys."),
            OaError::Billing(m) => write!(f, "OpenAI says your account has no credit left: {m}"),
            OaError::Busy(m) => write!(f, "OpenAI is busy right now ({m}). Try again in a minute."),
            OaError::Request(m) => write!(f, "OpenAI couldn't take that request: {m}"),
            OaError::Network(m) => write!(f, "Couldn't reach OpenAI: {m}"),
            OaError::Cancelled => write!(f, "Stopped."),
        }
    }
}

pub fn classify(status: u16, body: &str) -> OaError {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let code = v["error"]["code"].as_str().or(v["error"]["type"].as_str()).unwrap_or_default();
    let msg = v["error"]["message"].as_str().map(String::from).unwrap_or_else(|| format!("HTTP {status}"));
    match (status, code) {
        (_, "insufficient_quota") | (402, _) => OaError::Billing(msg),
        (401, _) | (403, _) | (_, "invalid_api_key") => OaError::Key(msg),
        (429, _) | (500..=599, _) => OaError::Busy(msg),
        _ => OaError::Request(msg),
    }
}

pub struct Client {
    base: String,
    key: zeroize::Zeroizing<String>,
    agent: ureq::Agent,
    pub backoff: Duration,
}

impl Client {
    pub fn with_base(base: &str, key: &str) -> Client {
        let mut cfg = ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(20))).timeout_recv_response(Some(Duration::from_secs(300))).timeout_recv_body(Some(Duration::from_secs(1800))).http_status_as_error(false);
        if base.starts_with("http://127.0.0.1") {
            cfg = cfg.proxy(None);
        }
        Client { base: base.trim_end_matches('/').to_string(), key: zeroize::Zeroizing::new(key.trim().to_string()), agent: cfg.build().into(), backoff: Duration::from_secs(2) }
    }

    /// The chat models this key can use, newest first.
    pub fn models(&self) -> Result<Vec<String>, OaError> {
        let mut res = self.agent.get(&format!("{}/v1/models", self.base)).header("authorization", format!("Bearer {}", self.key.as_str())).call().map_err(|e| OaError::Network(e.to_string()))?;
        let status = res.status().as_u16();
        let text = res.body_mut().read_to_string().unwrap_or_default();
        if !(200..300).contains(&status) {
            return Err(classify(status, &text));
        }
        let v: Value = serde_json::from_str(&text).map_err(|e| OaError::Network(e.to_string()))?;
        let mut models: Vec<(i64, String)> = v["data"].as_array().into_iter().flatten().filter_map(|m| Some((m["created"].as_i64().unwrap_or(0), m["id"].as_str()?.to_string()))).filter(|(_, id)| chat_model(id)).collect();
        models.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        Ok(models.into_iter().map(|(_, id)| id).collect())
    }

    /// Stream one reply. Busy answers before the stream starts are retried
    /// three times.
    pub fn stream(&self, body: &Value, cancel: &AtomicBool, on: &mut dyn FnMut(Delta)) -> Result<Reply, OaError> {
        let mut attempt = 0;
        let res = loop {
            if cancel.load(Ordering::SeqCst) {
                return Err(OaError::Cancelled);
            }
            let mut res = self
                .agent
                .post(&format!("{}/v1/responses", self.base))
                .header("authorization", format!("Bearer {}", self.key.as_str()))
                .header("content-type", "application/json")
                .send(body.to_string())
                .map_err(|e| OaError::Network(e.to_string()))?;
            let status = res.status().as_u16();
            if (200..300).contains(&status) {
                break res;
            }
            let wait = res.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.trim().parse::<u64>().ok());
            let text = res.body_mut().read_to_string().unwrap_or_default();
            let err = classify(status, &text);
            attempt += 1;
            if !matches!(err, OaError::Busy(_)) || attempt > 3 {
                return Err(err);
            }
            let pause = wait.map(Duration::from_secs).unwrap_or(self.backoff * 2u32.pow(attempt - 1)).min(Duration::from_secs(60));
            let until = std::time::Instant::now() + pause;
            while std::time::Instant::now() < until {
                if cancel.load(Ordering::SeqCst) {
                    return Err(OaError::Cancelled);
                }
                std::thread::sleep(Duration::from_millis(50).min(pause));
            }
        };
        read_events(res.into_body().into_reader(), cancel, on)
    }
}

/// Models made for conversation (not audio, images, embeddings…).
fn chat_model(id: &str) -> bool {
    let chat = id.starts_with("gpt-") || id.starts_with("chatgpt-") || (id.starts_with('o') && id[1..].starts_with(|c: char| c.is_ascii_digit()));
    const NOT: &[&str] = &["audio", "realtime", "tts", "transcribe", "image", "embedding", "search", "instruct", "moderation", "dall-e", "whisper", "computer-use", "-0301", "-0314", "-0613"];
    chat && !NOT.iter().any(|n| id.contains(n))
}

/// The request for a conversation (`messages` are `(role, text)`, oldest first).
pub fn body(model: &str, system: &str, messages: &[(String, String)], effort: &str, web: bool) -> Value {
    let input: Vec<Value> = messages.iter().map(|(role, text)| json!({ "role": role, "content": text })).collect();
    let mut b = json!({ "model": model, "instructions": system, "input": input, "stream": true, "store": false });
    if effort != "default" {
        b["reasoning"] = json!({ "effort": effort });
    }
    if web {
        b["tools"] = json!([{ "type": "web_search" }]);
    }
    b
}

/// Read the Responses API's server-sent events into a reply.
pub fn read_events(reader: impl Read, cancel: &AtomicBool, on: &mut dyn FnMut(Delta)) -> Result<Reply, OaError> {
    let mut lines = BufReader::new(reader);
    let mut reply = Reply::default();
    let mut data = String::new();
    let mut line = String::new();
    let mut done = false;
    loop {
        if cancel.load(Ordering::SeqCst) {
            return Err(OaError::Cancelled);
        }
        line.clear();
        let n = lines.read_line(&mut line).map_err(|e| OaError::Network(format!("the answer stopped coming: {e}")))?;
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
        if !data.is_empty() && data != "[DONE]" {
            let ev: Value = serde_json::from_str(&data).map_err(|e| OaError::Network(format!("an unreadable event: {e}")))?;
            data.clear();
            match ev["type"].as_str().unwrap_or_default() {
                "response.created" => reply.model = ev["response"]["model"].as_str().unwrap_or_default().to_string(),
                "response.output_text.delta" | "response.refusal.delta" => {
                    let t = ev["delta"].as_str().unwrap_or_default();
                    reply.text.push_str(t);
                    on(Delta::Text(t.to_string()));
                }
                "response.web_search_call.in_progress" => on(Delta::Search),
                "response.completed" | "response.incomplete" => {
                    let r = &ev["response"];
                    reply.usage = Usage::read(&r["usage"]);
                    if let Some(m) = r["model"].as_str() {
                        reply.model = m.to_string();
                    }
                    reply.incomplete = r["incomplete_details"]["reason"].as_str().map(String::from);
                    done = true;
                    break;
                }
                "response.failed" => {
                    let e = &ev["response"]["error"];
                    let msg = e["message"].as_str().unwrap_or("the reply failed").to_string();
                    return Err(if e["code"] == "server_error" || e["code"] == "rate_limit_exceeded" { OaError::Busy(msg) } else { OaError::Request(msg) });
                }
                "error" => {
                    let msg = ev["message"].as_str().or(ev["error"]["message"].as_str()).unwrap_or("error").to_string();
                    let code = ev["code"].as_str().or(ev["error"]["code"].as_str()).unwrap_or_default();
                    return Err(match code {
                        "insufficient_quota" => OaError::Billing(msg),
                        "rate_limit_exceeded" | "server_error" => OaError::Busy(msg),
                        _ => OaError::Request(msg),
                    });
                }
                _ => {}
            }
        }
        data.clear();
        if end {
            break;
        }
    }
    if !done {
        return Err(OaError::Network("the answer stopped before it finished".into()));
    }
    Ok(reply)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sse(events: &[Value]) -> String {
        events.iter().map(|e| format!("event: {}\ndata: {e}\n\n", e["type"].as_str().unwrap_or_default())).collect()
    }

    #[test]
    fn settings_and_key_rules() {
        let t = tempfile::tempdir().unwrap();
        let o = OpenAi::new(t.path());
        assert_eq!((o.settings().model.as_str(), o.settings().effort.as_str(), o.settings().has_key), (DEFAULT_MODEL, "default", false));
        let s = o.update(&json!({"model": "gpt-6-sol", "effort": "high", "apiSecret": "x", "hasKey": true})).unwrap();
        assert_eq!((s.model.as_str(), s.effort.as_str(), s.api_secret.as_str()), ("gpt-6-sol", "high", ""));
        assert_eq!(o.update(&json!({"model": "a b; rm", "effort": "max"})).unwrap().model, DEFAULT_MODEL);
        assert!(o.set_key("sk-ant-api03-claude").is_err(), "a Claude key is not an OpenAI key");
        assert!(o.set_key("nope").is_err() && o.set_key("sk-has space").is_err());
        assert!(o.client().is_err());
        o.set_test_key("sk-proj-test");
        assert!(o.client().is_ok());
    }

    #[test]
    fn reads_a_streamed_reply() {
        let s = sse(&[
            json!({"type": "response.created", "response": {"model": "gpt-6-astra"}}),
            json!({"type": "response.web_search_call.in_progress"}),
            json!({"type": "response.output_text.delta", "delta": "Hel"}),
            json!({"type": "response.output_text.delta", "delta": "lo"}),
            json!({"type": "response.completed", "response": {"model": "gpt-6-astra-2026-09-03", "usage": {"input_tokens": 120, "input_tokens_details": {"cached_tokens": 100}, "output_tokens": 30, "output_tokens_details": {"reasoning_tokens": 12}}}}),
        ]);
        let mut seen = Vec::new();
        let r = read_events(s.as_bytes(), &AtomicBool::new(false), &mut |d| seen.push(d)).unwrap();
        assert_eq!(r.text, "Hello");
        assert_eq!(r.model, "gpt-6-astra-2026-09-03");
        assert_eq!(r.usage, Usage { input: 120, cached: 100, output: 30, reasoning: 12 });
        assert_eq!(seen, vec![Delta::Search, Delta::Text("Hel".into()), Delta::Text("lo".into())]);

        let cut = sse(&[json!({"type": "response.output_text.delta", "delta": "Hi"})]);
        assert!(matches!(read_events(cut.as_bytes(), &AtomicBool::new(false), &mut |_| {}), Err(OaError::Network(_))));
        let failed = sse(&[json!({"type": "response.failed", "response": {"error": {"code": "server_error", "message": "oops"}}})]);
        assert!(matches!(read_events(failed.as_bytes(), &AtomicBool::new(false), &mut |_| {}), Err(OaError::Busy(_))));
        let quota = sse(&[json!({"type": "error", "code": "insufficient_quota", "message": "You exceeded your quota"})]);
        assert!(matches!(read_events(quota.as_bytes(), &AtomicBool::new(false), &mut |_| {}), Err(OaError::Billing(_))));
        let short = sse(&[json!({"type": "response.incomplete", "response": {"incomplete_details": {"reason": "max_output_tokens"}, "usage": {"input_tokens": 1, "output_tokens": 2}}})]);
        assert_eq!(read_events(short.as_bytes(), &AtomicBool::new(false), &mut |_| {}).unwrap().incomplete.as_deref(), Some("max_output_tokens"));
    }

    #[test]
    fn errors_and_models_in_plain_words() {
        assert!(matches!(classify(401, r#"{"error":{"message":"Incorrect API key","code":"invalid_api_key"}}"#), OaError::Key(_)));
        assert!(matches!(classify(429, r#"{"error":{"message":"quota","code":"insufficient_quota"}}"#), OaError::Billing(_)));
        assert!(matches!(classify(429, r#"{"error":{"message":"slow down"}}"#), OaError::Busy(_)));
        assert!(matches!(classify(404, r#"{"error":{"message":"no such model"}}"#), OaError::Request(_)));
        for ok in ["gpt-6-astra", "gpt-5.5", "o4-mini", "chatgpt-6-latest", "gpt-5-codex"] {
            assert!(chat_model(ok), "{ok}");
        }
        for no in ["gpt-4o-audio-preview", "text-embedding-3-large", "gpt-image-2", "whisper-1", "omni-moderation-latest", "gpt-4o-realtime-preview", "dall-e-3", "gpt-4o-mini-tts"] {
            assert!(!chat_model(no), "{no}");
        }
        let b = body("gpt-6-astra", "Be kind.", &[("user".into(), "hi".into()), ("assistant".into(), "hello".into()), ("user".into(), "and?".into())], "low", true);
        assert_eq!(b["input"].as_array().unwrap().len(), 3);
        assert_eq!((b["store"].clone(), b["reasoning"]["effort"].clone(), b["tools"][0]["type"].clone()), (json!(false), json!("low"), json!("web_search")));
        assert!(body("m", "", &[], "default", false).get("reasoning").is_none());
    }
}
