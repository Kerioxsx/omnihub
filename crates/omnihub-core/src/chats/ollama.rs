//! Local AI in Chats: models running on this PC through Ollama
//! (ollama.com), free and offline. Nothing leaves the PC: OmniHub talks to
//! Ollama on 127.0.0.1 only.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const BASE: &str = "http://127.0.0.1:11434";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LocalSettings {
    /// The model new local chats use ("llama3.2", "gemma3:4b"…).
    pub model: String,
}

pub struct Local {
    file: PathBuf,
    base: String,
    settings: Mutex<LocalSettings>,
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_recv_response(Some(Duration::from_secs(600)))
        .timeout_recv_body(Some(Duration::from_secs(1800)))
        .http_status_as_error(false)
        .proxy(None)
        .build()
        .into()
}

/// Model names from Ollama's `/api/tags` answer.
pub fn parse_tags(v: &Value) -> Vec<String> {
    let mut out: Vec<String> = v.get("models").and_then(|m| m.as_array()).map(|a| a.iter().filter_map(|m| m.get("name").and_then(|n| n.as_str()).map(String::from)).collect()).unwrap_or_default();
    // Embedding-only models can't chat.
    out.retain(|n| !n.contains("embed"));
    out.sort();
    out
}

/// Read Ollama's streamed answer (one JSON object per line).
pub fn read_stream(r: impl Read, cancel: &AtomicBool, on: &mut dyn FnMut(&str)) -> Result<String, String> {
    let mut text = String::new();
    for line in BufReader::new(r).lines() {
        if cancel.load(Ordering::SeqCst) {
            return Err("Stopped".into());
        }
        let line = line.map_err(|e| format!("Ollama stopped answering: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(&line).map_err(|_| "Ollama sent something unexpected".to_string())?;
        if let Some(e) = v.get("error").and_then(|e| e.as_str()) {
            return Err(format!("Ollama: {e}"));
        }
        if let Some(t) = v.pointer("/message/content").and_then(|c| c.as_str()) {
            text.push_str(t);
            on(t);
        }
        if v.get("done").and_then(|d| d.as_bool()) == Some(true) {
            break;
        }
    }
    Ok(text)
}

impl Local {
    pub fn new(dir: &Path) -> Local {
        Local::with_base(dir, BASE)
    }

    pub fn with_base(dir: &Path, base: &str) -> Local {
        let file = dir.join("local.json");
        let settings = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Local { file, base: base.trim_end_matches('/').to_string(), settings: Mutex::new(settings) }
    }

    pub fn settings(&self) -> LocalSettings {
        self.settings.lock().clone()
    }

    pub fn set_model(&self, model: &str) -> Result<LocalSettings, String> {
        let m = model.trim();
        if m.is_empty() || m.len() > 120 || !m.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '.' | '-' | '_' | '/')) {
            return Err("Pick one of your Ollama models".into());
        }
        let s = LocalSettings { model: m.to_string() };
        crate::settings::write_atomic(&self.file, &serde_json::to_vec(&s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        *self.settings.lock() = s.clone();
        Ok(s)
    }

    /// The models Ollama has on this PC (an error if Ollama isn't running).
    pub fn models(&self) -> Result<Vec<String>, String> {
        let mut res = agent().get(&format!("{}/api/tags", self.base)).call().map_err(|_| "Ollama isn't running. Install it from ollama.com (or start it), then get a model, like: ollama pull llama3.2".to_string())?;
        let v: Value = serde_json::from_str(&res.body_mut().read_to_string().unwrap_or_default()).map_err(|_| "Ollama sent something unexpected".to_string())?;
        Ok(parse_tags(&v))
    }

    /// The model to use: the chosen one, else the first Ollama has.
    pub fn model(&self) -> Result<String, String> {
        let chosen = self.settings().model;
        let have = self.models()?;
        if have.is_empty() {
            return Err("Ollama has no models yet. Get one in a terminal, like: ollama pull llama3.2".into());
        }
        Ok(if have.contains(&chosen) { chosen } else { have[0].clone() })
    }

    /// Answer a conversation, streaming the text as it comes.
    pub fn chat(&self, model: &str, system: &str, turns: &[(String, String)], cancel: &AtomicBool, on: &mut dyn FnMut(&str)) -> Result<String, String> {
        let mut messages = vec![json!({ "role": "system", "content": system })];
        messages.extend(turns.iter().map(|(r, t)| json!({ "role": r, "content": t })));
        let body = json!({ "model": model, "messages": messages, "stream": true });
        let mut res = agent().post(&format!("{}/api/chat", self.base)).header("content-type", "application/json").send(body.to_string()).map_err(|_| "Ollama isn't running. Start it and try again.".to_string())?;
        if !res.status().is_success() {
            let t = res.body_mut().read_to_string().unwrap_or_default();
            let msg = serde_json::from_str::<Value>(&t).ok().and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from)).unwrap_or(t);
            return Err(format!("Ollama: {msg}"));
        }
        read_stream(res.body_mut().as_reader(), cancel, on)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_models_and_streams() {
        let tags = json!({ "models": [{ "name": "llama3.2:latest" }, { "name": "nomic-embed-text" }, { "name": "gemma3:4b" }] });
        assert_eq!(parse_tags(&tags), vec!["gemma3:4b", "llama3.2:latest"]);
        let stream = "{\"message\":{\"role\":\"assistant\",\"content\":\"Hel\"},\"done\":false}\n{\"message\":{\"content\":\"lo\"},\"done\":false}\n\n{\"done\":true}\n{\"message\":{\"content\":\"ignored\"}}\n";
        let mut got = String::new();
        let text = read_stream(stream.as_bytes(), &AtomicBool::new(false), &mut |t| got.push_str(t)).unwrap();
        assert_eq!((text.as_str(), got.as_str()), ("Hello", "Hello"));
        assert!(read_stream("{\"error\":\"model not found\"}\n".as_bytes(), &AtomicBool::new(false), &mut |_| {}).unwrap_err().contains("model not found"));
        let t = tempfile::tempdir().unwrap();
        let l = Local::new(t.path());
        assert!(l.set_model("bad model; rm").is_err());
        l.set_model("llama3.2").unwrap();
        assert_eq!(Local::new(t.path()).settings().model, "llama3.2");
        // Ollama not running here: a clear message.
        let off = Local::with_base(t.path(), "http://127.0.0.1:9");
        assert!(off.models().unwrap_err().contains("ollama"));
    }
}
