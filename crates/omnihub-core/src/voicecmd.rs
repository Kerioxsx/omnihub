//! Voice commands said into the phone: the phone's browser turns speech
//! into text (its own speech recognition) and sends the text here. The
//! same sentences as "Capri, …" on the PC work (with or without "Capri"),
//! plus a few for the phone: quiet mode, wake up, and starting games.

use crate::core::AppCore;

fn clean(t: &str) -> String {
    let t: String = t.to_lowercase().chars().map(|c| if c.is_alphanumeric() || c == '\'' { c } else { ' ' }).collect();
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    for w in ["hey capri ", "okay capri ", "ok capri ", "capri "] {
        if let Some(rest) = t.strip_prefix(w) {
            return rest.to_string();
        }
    }
    t
}

impl AppCore {
    /// Understand and do a sentence; says what was done. `power_ok`: quiet
    /// mode may be turned on and off (the phone's power permission).
    pub fn voice_text(&self, text: &str, power_ok: bool) -> Result<String, String> {
        let t = clean(text);
        if t.is_empty() {
            return Err("I didn't hear anything".into());
        }
        match t.as_str() {
            "quiet mode" | "quiet" | "go quiet" | "night mode" | "good night" | "goodnight" | "wake up" | "normal mode" | "quiet mode off" | "good morning" if !power_ok => {
                return Err("Power actions are off on the PC (OmniHub → Phone → Permissions)".into());
            }
            "quiet mode" | "quiet" | "go quiet" | "night mode" | "good night" | "goodnight" => {
                self.quiet_on()?;
                return Ok("Quiet mode on".into());
            }
            "wake up" | "normal mode" | "quiet mode off" | "good morning" => {
                self.quiet_off()?;
                return Ok("Quiet mode off".into());
            }
            _ => {}
        }
        for verb in ["start ", "launch ", "play "] {
            if let Some(name) = t.strip_prefix(verb) {
                let want: String = name.chars().filter(|c| c.is_alphanumeric()).collect();
                let games = self.library_games();
                if let Some(g) = games.iter().find(|g| g.name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>() == want).or_else(|| games.iter().find(|g| g.name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect::<String>().contains(&want) && want.len() >= 3)) {
                    crate::system::awake::wake_screen();
                    self.library_play(&g.key)?;
                    return Ok(format!("Starting {}", g.name));
                }
            }
        }
        let s = self.settings.get().play;
        let apps = self.apps.cached_launchable().unwrap_or_default();
        if let Some(cmd) = crate::play::voice::parse(&format!("capri {t}"), &s.nicknames, &apps) {
            let said = format!("{cmd:?}");
            return if self.play.run_voice(cmd) { Ok(format!("Done: {}", said.split(['(', '{']).next().unwrap_or(&said))) } else { Err("Capri's voice commands are off on the PC (Games → Clips & Capri)".into()) };
        }
        Err(format!("I didn't understand “{text}”. Try “clip that”, “screenshot”, “quiet mode”, “start Fortnite” or “open Spotify”."))
    }
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn cleans_the_wake_word() {
        assert_eq!(clean("Hey Capri, clip that!"), "clip that");
        assert_eq!(clean("Quiet mode."), "quiet mode");
        assert_eq!(clean("capri start Fortnite"), "start fortnite");
    }
}
