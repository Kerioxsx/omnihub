//! Claude in OmniHub, with the user's own Anthropic API key: the key (kept
//! encrypted for this Windows account with DPAPI, never shown to the window
//! or written anywhere else), the model and effort to use, and what a
//! conversation costs. The Mods page's mod maker is what talks to Claude
//! (`crate::mods::maker`).

pub mod api;

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

const SECRET_ENTROPY: &[u8] = b"omnihub-claude";

/// The models offered, best first.
pub const MODELS: &[(&str, &str)] = &[
    ("claude-opus-5-5", "Claude Opus 5.5 (recommended)"),
    ("claude-sonnet-5-5", "Claude Sonnet 5.5 (faster, cheaper)"),
    ("claude-haiku-5-5", "Claude Haiku 5.5 (cheapest)"),
    ("claude-fable-5-1", "Claude Fable 5.1 (most capable, priciest)"),
];
pub const DEFAULT_MODEL: &str = "claude-opus-5-5";
pub const EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];

/// Price per million tokens: (input, output, cache read, cache write 5 min).
pub fn price(model: &str) -> (f64, f64, f64, f64) {
    match model {
        "claude-fable-5-1" => (10.0, 50.0, 0.25, 12.5),
        "claude-sonnet-5-5" => (2.0, 10.0, 0.20, 2.5),
        "claude-haiku-5-5" => (0.10, 0.50, 0.01, 0.125),
        _ => (4.0, 20.0, 0.20, 5.0),
    }
}

/// About what `u` cost in US dollars (web searches are $10 per thousand).
pub fn cost(model: &str, u: &api::Usage) -> f64 {
    let (i, o, r, w) = price(model);
    (u.input as f64 * i + u.output as f64 * o + u.cache_read as f64 * r + u.cache_write as f64 * w) / 1e6 + u.web_searches as f64 * 0.01
}

/// What a model's requests may use (the Haiku line has no server-side
/// fallback, progress-update display or the newest web tools).
pub struct ModelTraits {
    pub fallbacks: bool,
    pub updates: bool,
    pub web_search: &'static str,
    pub web_fetch: &'static str,
}

pub fn traits(model: &str) -> ModelTraits {
    if model.starts_with("claude-haiku") {
        ModelTraits { fallbacks: false, updates: false, web_search: "web_search_20250305", web_fetch: "web_fetch_20250910" }
    } else {
        ModelTraits { fallbacks: true, updates: true, web_search: "web_search_20260209", web_fetch: "web_fetch_20260209" }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ClaudeSettings {
    /// The key, DPAPI-encrypted and base64'd. Cleared before the window sees it.
    pub api_secret: String,
    pub model: String,
    pub effort: String,
    /// Let Claude search and read the web (modding docs) while it works.
    pub web: bool,
    /// Shown to the window instead of the key.
    #[serde(skip_deserializing)]
    pub has_key: bool,
}

impl Default for ClaudeSettings {
    fn default() -> Self {
        ClaudeSettings { api_secret: String::new(), model: DEFAULT_MODEL.into(), effort: "medium".into(), web: true, has_key: false }
    }
}

impl ClaudeSettings {
    fn sanitize(&mut self) {
        if !MODELS.iter().any(|(m, _)| *m == self.model) {
            self.model = DEFAULT_MODEL.into();
        }
        if !EFFORTS.contains(&self.effort.as_str()) {
            self.effort = "medium".into();
        }
        self.has_key = !self.api_secret.is_empty();
    }

    /// The settings without the key, for the window.
    pub fn public(&self) -> ClaudeSettings {
        ClaudeSettings { api_secret: String::new(), has_key: !self.api_secret.is_empty(), ..self.clone() }
    }
}

pub struct Claude {
    file: PathBuf,
    settings: Mutex<ClaudeSettings>,
    base: String,
    #[cfg(test)]
    test_key: Mutex<Option<String>>,
}

impl Claude {
    pub fn new(data_dir: &Path) -> Claude {
        Claude::with_base(data_dir, api::BASE)
    }

    pub fn with_base(data_dir: &Path, base: &str) -> Claude {
        let file = data_dir.join("claude.json");
        let mut s: ClaudeSettings = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        s.sanitize();
        Claude {
            file,
            settings: Mutex::new(s),
            base: base.to_string(),
            #[cfg(test)]
            test_key: Mutex::new(None),
        }
    }

    pub fn settings(&self) -> ClaudeSettings {
        self.settings.lock().public()
    }

    fn save(&self, s: &ClaudeSettings) -> Result<(), String> {
        if let Some(p) = self.file.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        let tmp = self.file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.file).map_err(|e| e.to_string())
    }

    /// Change the model, effort or web use (never the key).
    pub fn update(&self, patch: &serde_json::Value) -> Result<ClaudeSettings, String> {
        let mut s = self.settings.lock();
        let mut v = serde_json::to_value(&*s).map_err(|e| e.to_string())?;
        if let (Some(cur), Some(p)) = (v.as_object_mut(), patch.as_object()) {
            for (k, val) in p {
                if k != "apiSecret" && k != "hasKey" {
                    cur.insert(k.clone(), val.clone());
                }
            }
        }
        let mut next: ClaudeSettings = serde_json::from_value(v).map_err(|e| format!("Bad settings: {e}"))?;
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
        if !key.is_empty() && (key.len() > 300 || !key.starts_with("sk-ant-") || key.chars().any(|c| c.is_control() || c.is_whitespace())) {
            return Err("That doesn't look like an Anthropic API key (they start with sk-ant-).".into());
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

    /// A client with the saved key.
    pub fn client(&self) -> Result<api::Client, String> {
        let key = self.key().ok_or("Add your Anthropic API key first (Mods → Claude settings).")?;
        Ok(api::Client::with_base(&self.base, &key))
    }

    /// Check the saved key with a tiny request.
    pub fn test(&self) -> Result<(), String> {
        let model = self.settings.lock().model.clone();
        self.client()?.check(&model).map_err(|e| e.to_string())
    }

    #[cfg(test)]
    pub fn set_test_key(&self, key: &str) {
        *self.test_key.lock() = Some(key.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_keep_the_key_out_of_reach() {
        let t = tempfile::tempdir().unwrap();
        let c = Claude::new(t.path());
        assert_eq!((c.settings().model.as_str(), c.settings().effort.as_str(), c.settings().web, c.settings().has_key), ("claude-opus-5-5", "medium", true, false));
        let s = c.update(&serde_json::json!({"model": "claude-sonnet-5-5", "effort": "high", "apiSecret": "stolen", "hasKey": true})).unwrap();
        assert_eq!((s.model.as_str(), s.effort.as_str(), s.api_secret.as_str(), s.has_key), ("claude-sonnet-5-5", "high", "", false));
        assert_eq!(c.update(&serde_json::json!({"model": "gpt-9", "effort": "extreme"})).unwrap().model, DEFAULT_MODEL, "unknown models fall back");
        assert!(c.set_key("not a key").is_err());
        assert!(c.set_key("sk-ant-has space").is_err());
        assert!(c.client().is_err());
        c.set_test_key("sk-ant-test");
        assert!(c.has_key() && c.client().is_ok());
        let saved = std::fs::read_to_string(t.path().join("claude.json")).unwrap();
        assert!(!saved.contains("sk-ant") && !saved.contains("stolen"));

        let u = api::Usage { input: 1_000_000, output: 100_000, cache_read: 2_000_000, cache_write: 0, web_searches: 3 };
        assert!((cost("claude-opus-5-5", &u) - (4.0 + 2.0 + 0.4 + 0.03)).abs() < 1e-9);
        assert!(!traits("claude-haiku-5-5").fallbacks && traits("claude-opus-5-5").updates);
    }
}
