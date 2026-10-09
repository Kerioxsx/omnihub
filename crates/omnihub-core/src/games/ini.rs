//! Change a few values in a game's settings file without disturbing the
//! rest: every other line, comment, blank line, key order and line ending
//! stays as the game wrote it.
//!
//! Two shapes: INI files with `[Section]` headers and `Key=Value` lines
//! (Unreal Engine games), and `key:value` lines (Minecraft's options.txt).

/// A settings file in memory.
#[derive(Debug, Clone, PartialEq)]
pub struct Doc {
    lines: Vec<String>,
    newline: &'static str,
    /// `=` for INI files, `:` for options.txt.
    sep: char,
    utf16: bool,
}

impl Doc {
    /// Read an INI file's bytes (UTF-8, or UTF-16 with a byte-order mark).
    pub fn ini(bytes: &[u8]) -> Doc {
        Self::parse(bytes, '=')
    }

    /// Read Minecraft's `key:value` options.
    pub fn colon(bytes: &[u8]) -> Doc {
        Self::parse(bytes, ':')
    }

    fn parse(bytes: &[u8], sep: char) -> Doc {
        let (text, utf16) = if bytes.starts_with(&[0xFF, 0xFE]) {
            let units: Vec<u16> = bytes[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            (String::from_utf16_lossy(&units), true)
        } else {
            let b = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
            (String::from_utf8_lossy(b).into_owned(), false)
        };
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let mut lines: Vec<String> = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l).to_string()).collect();
        // A trailing newline leaves one empty piece; it comes back on save.
        if lines.last().is_some_and(String::is_empty) {
            lines.pop();
        }
        Doc { lines, newline, sep, utf16 }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut text = self.lines.join(self.newline);
        text.push_str(self.newline);
        if self.utf16 {
            let mut out = vec![0xFF, 0xFE];
            for u in text.encode_utf16() {
                out.extend_from_slice(&u.to_le_bytes());
            }
            out
        } else {
            text.into_bytes()
        }
    }

    fn section_of(line: &str) -> Option<&str> {
        let t = line.trim();
        t.strip_prefix('[')?.strip_suffix(']')
    }

    fn key_of(&self, line: &str) -> Option<String> {
        let t = line.trim_start();
        if t.starts_with(';') || t.starts_with('#') || t.starts_with('[') {
            return None;
        }
        let (k, _) = t.split_once(self.sep)?;
        Some(k.trim().to_string())
    }

    /// The lines of `section` (`None`: the whole file for `key:value` files,
    /// or the lines before the first header in an INI file).
    fn range(&self, section: Option<&str>) -> Option<(usize, usize)> {
        match section {
            None => {
                let end = if self.sep == '=' { self.lines.iter().position(|l| Self::section_of(l).is_some()).unwrap_or(self.lines.len()) } else { self.lines.len() };
                Some((0, end))
            }
            Some(name) => {
                let start = self.lines.iter().position(|l| Self::section_of(l) == Some(name))? + 1;
                let end = self.lines[start..].iter().position(|l| Self::section_of(l).is_some()).map_or(self.lines.len(), |p| start + p);
                Some((start, end))
            }
        }
    }

    pub fn has_section(&self, section: &str) -> bool {
        self.range(Some(section)).is_some()
    }

    /// The value of `key` (case-insensitive) in `section`.
    pub fn get(&self, section: Option<&str>, key: &str) -> Option<String> {
        let (a, b) = self.range(section)?;
        self.lines[a..b].iter().find(|l| self.key_of(l).is_some_and(|k| k.eq_ignore_ascii_case(key))).and_then(|l| l.split_once(self.sep)).map(|(_, v)| v.trim().to_string())
    }

    /// Set `key` in `section`. With `add`, a missing key is appended to its
    /// section (and a missing section to the file); otherwise only existing
    /// keys change. Returns whether the file changed.
    pub fn set(&mut self, section: Option<&str>, key: &str, value: &str, add: bool) -> bool {
        let sep = self.sep;
        let range = self.range(section);
        if let Some((a, b)) = range {
            for i in a..b {
                let Some(k) = self.key_of(&self.lines[i]) else { continue };
                if !k.eq_ignore_ascii_case(key) {
                    continue;
                }
                let old = self.lines[i].split_once(sep).map(|(_, v)| v.trim().to_string()).unwrap_or_default();
                if old == value {
                    return false;
                }
                self.lines[i] = format!("{k}{sep}{value}");
                return true;
            }
        }
        if !add {
            return false;
        }
        let line = format!("{key}{sep}{value}");
        match (range, section) {
            (Some((a, b)), _) => {
                // After the section's last setting, before trailing blank lines.
                let mut at = b;
                while at > a && self.lines[at - 1].trim().is_empty() {
                    at -= 1;
                }
                self.lines.insert(at, line);
            }
            (None, Some(name)) => {
                if self.lines.last().is_some_and(|l| !l.trim().is_empty()) {
                    self.lines.push(String::new());
                }
                self.lines.push(format!("[{name}]"));
                self.lines.push(line);
            }
            (None, None) => self.lines.push(line),
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_only_what_it_must() {
        let src = "[/Script/FortniteGame.FortGameUserSettings]\r\nbShowFPS=False\r\nFrameRateLimit=144.000000\r\n; a comment\r\nOther=1\r\n\r\n[ScalabilityGroups]\r\nsg.ShadowQuality=3\r\n\r\n";
        let mut d = Doc::ini(src.as_bytes());
        let s = Some("/Script/FortniteGame.FortGameUserSettings");
        assert_eq!(d.get(s, "framerateLIMIT").as_deref(), Some("144.000000"));
        assert!(d.set(s, "FrameRateLimit", "0.000000", true));
        assert!(!d.set(s, "FrameRateLimit", "0.000000", true), "same value: no change");
        assert!(d.set(s, "bShowFPS", "True", false));
        assert!(!d.set(s, "bNotThere", "True", false), "unknown keys are left out unless asked");
        assert!(d.set(s, "bUseVSync", "False", true));
        assert!(d.set(Some("ScalabilityGroups"), "sg.TextureQuality", "0", true));
        assert!(d.set(Some("D3DRHIPreference"), "PreferredFeatureLevel", "es31", true));
        let out = String::from_utf8(d.to_bytes()).unwrap();
        assert_eq!(
            out,
            "[/Script/FortniteGame.FortGameUserSettings]\r\nbShowFPS=True\r\nFrameRateLimit=0.000000\r\n; a comment\r\nOther=1\r\nbUseVSync=False\r\n\r\n[ScalabilityGroups]\r\nsg.ShadowQuality=3\r\nsg.TextureQuality=0\r\n\r\n[D3DRHIPreference]\r\nPreferredFeatureLevel=es31\r\n"
        );
        // Untouched documents come back byte for byte.
        assert_eq!(Doc::ini(src.as_bytes()).to_bytes(), src.as_bytes());
    }

    #[test]
    fn utf16_and_colon_files() {
        let mut b = vec![0xFF, 0xFE];
        for u in "[A]\nx=1\n".encode_utf16() {
            b.extend_from_slice(&u.to_le_bytes());
        }
        let mut d = Doc::ini(&b);
        assert!(d.set(Some("A"), "x", "2", false));
        assert_eq!(Doc::ini(&d.to_bytes()).get(Some("A"), "x").as_deref(), Some("2"));
        assert_eq!(&d.to_bytes()[..2], &[0xFF, 0xFE]);

        let mut o = Doc::colon(b"version:3700\nmaxFps:120\nrenderClouds:\"true\"\nenableVsync:true\n");
        assert_eq!(o.get(None, "maxFps").as_deref(), Some("120"));
        assert!(o.set(None, "maxFps", "260", false));
        assert!(o.set(None, "enableVsync", "false", false));
        assert!(!o.set(None, "graphicsMode", "0", false));
        assert_eq!(String::from_utf8(o.to_bytes()).unwrap(), "version:3700\nmaxFps:260\nrenderClouds:\"true\"\nenableVsync:false\n");
    }
}
