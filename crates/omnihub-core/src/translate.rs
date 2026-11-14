//! The screen translator: drag a box over text on a frozen copy of the
//! screen; Windows' own OCR reads it and the AI model on this PC (Ollama)
//! translates it. Nothing leaves the PC.

use serde::Serialize;

use crate::capture::screenshots::Rect;
use crate::core::AppCore;

pub const LANGUAGES: &[&str] = &["English", "Spanish", "French", "German", "Italian", "Portuguese", "Dutch", "Polish", "Russian", "Ukrainian", "Turkish", "Arabic", "Hindi", "Japanese", "Korean", "Chinese (Simplified)", "Chinese (Traditional)", "Vietnamese", "Thai", "Indonesian"];

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Translation {
    pub text: String,
    pub translated: String,
    pub model: String,
}

pub fn system_prompt(to: &str) -> String {
    format!("Translate the user's text into {to}. Reply with only the translation, keeping the line breaks. Text read from a screen can have small mistakes; fix them quietly. If it is already in {to}, reply with it unchanged.")
}

/// Join the short lines OCR returns into the lines a translator should see:
/// blank and stray one-character lines dropped.
pub fn tidy(text: &str) -> String {
    text.lines().map(str::trim).filter(|l| l.chars().count() > 1).collect::<Vec<_>>().join("\n")
}

impl AppCore {
    /// Read and translate part of the frozen screen (the overlay's).
    pub fn translate_region(&self, id: &str, rect: Rect, to: &str) -> Result<Translation, String> {
        if !LANGUAGES.contains(&to) {
            return Err("Pick a language from the list".into());
        }
        let png = self.screenshots.region_png(id, rect).map_err(|e| e.to_string())?;
        let text = tidy(&crate::capture::ocr::recognize_bytes(png)?);
        if text.is_empty() {
            return Err("No text there that Windows could read".into());
        }
        let model = self.chats.local.model()?;
        let out = self.chats.local.chat(&model, &system_prompt(to), &[("user".into(), text.clone())], &std::sync::atomic::AtomicBool::new(false), &mut |_| {})?;
        Ok(Translation { text, translated: out.trim().to_string(), model })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tidies_screen_text() {
        assert_eq!(tidy("  Bienvenue \n\n|\nAppuyez sur A  "), "Bienvenue\nAppuyez sur A");
        assert!(system_prompt("English").contains("into English"));
        assert!(LANGUAGES.contains(&"Japanese"));
    }
}
