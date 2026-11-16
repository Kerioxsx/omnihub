//! Change a few values in a game's settings file without disturbing the
//! rest: every other line, comment, blank line, key order, spacing, quoting
//! and line ending stays as the game wrote it.
//!
//! Four shapes:
//! * INI files with `[Section]` headers and `Key=Value` lines (Unreal
//!   Engine games, Overwatch);
//! * `key:value` lines (Minecraft's options.txt);
//! * Valve KeyValues, `"key"  "value"` lines inside braces (Counter-Strike
//!   2, Apex Legends);
//! * Roblox's XML, `<int name="Key">value</int>` (values only, no adding).

/// A section name that matches a key in any section.
pub const ANY: &str = "*";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ini,
    Colon,
    KeyValues,
    Xml,
}

/// A settings file in memory.
#[derive(Debug, Clone, PartialEq)]
pub struct Doc {
    lines: Vec<String>,
    newline: &'static str,
    kind: Kind,
    utf16: bool,
}

/// Where a value sits in a line: the byte range of the value itself, and
/// whether it is wrapped in quotes.
struct Slot {
    start: usize,
    end: usize,
    quoted: bool,
}

fn unquote(v: &str) -> (&str, bool) {
    match v.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        Some(inner) => (inner, true),
        None => (v, false),
    }
}

impl Doc {
    /// Read an INI file's bytes (UTF-8, or UTF-16 with a byte-order mark).
    pub fn ini(bytes: &[u8]) -> Doc {
        Self::parse(bytes, Kind::Ini)
    }

    /// Read Minecraft's `key:value` options.
    pub fn colon(bytes: &[u8]) -> Doc {
        Self::parse(bytes, Kind::Colon)
    }

    /// Read a Valve KeyValues file (cs2_video.txt, videoconfig.txt).
    pub fn key_values(bytes: &[u8]) -> Doc {
        Self::parse(bytes, Kind::KeyValues)
    }

    /// Read Roblox's GlobalBasicSettings XML.
    pub fn xml(bytes: &[u8]) -> Doc {
        Self::parse(bytes, Kind::Xml)
    }

    fn parse(bytes: &[u8], kind: Kind) -> Doc {
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
        Doc { lines, newline, kind, utf16 }
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

    /// The key a line sets and where its value sits.
    fn entry(&self, line: &str) -> Option<(String, Slot)> {
        match self.kind {
            Kind::Ini | Kind::Colon => {
                let sep = if self.kind == Kind::Ini { '=' } else { ':' };
                let t = line.trim_start();
                if t.starts_with(';') || t.starts_with('#') || t.starts_with('[') {
                    return None;
                }
                let at = line.find(sep)?;
                let key = line[..at].trim().to_string();
                if key.is_empty() {
                    return None;
                }
                let rest = &line[at + 1..];
                let lead = rest.len() - rest.trim_start().len();
                let value = rest.trim();
                let start = at + 1 + lead;
                let (_, quoted) = unquote(value);
                Some((key, Slot { start, end: start + value.len(), quoted }))
            }
            Kind::KeyValues => {
                // "key"<space>"value": the first two quoted strings.
                let q: Vec<usize> = line.match_indices('"').map(|(i, _)| i).collect();
                if q.len() < 4 {
                    return None;
                }
                let key = line[q[0] + 1..q[1]].to_string();
                Some((key, Slot { start: q[2], end: q[3] + 1, quoted: true }))
            }
            Kind::Xml => {
                let a = line.find(" name=\"")? + 7;
                let b = a + line[a..].find('"')?;
                let open_end = b + line[b..].find('>')? + 1;
                let close = open_end + line[open_end..].find("</")?;
                Some((line[a..b].to_string(), Slot { start: open_end, end: close, quoted: false }))
            }
        }
    }

    /// The lines of `section` (`None`: the whole file, or for an INI file
    /// the lines before its first header; `"*"`: every section).
    fn range(&self, section: Option<&str>) -> Option<(usize, usize)> {
        match section {
            Some(ANY) => Some((0, self.lines.len())),
            None => {
                let end = if self.kind == Kind::Ini { self.lines.iter().position(|l| Self::section_of(l).is_some()).unwrap_or(self.lines.len()) } else { self.lines.len() };
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

    fn find(&self, section: Option<&str>, key: &str) -> Option<(usize, Slot)> {
        let (a, b) = self.range(section)?;
        (a..b).find_map(|i| self.entry(&self.lines[i]).filter(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, s)| (i, s)))
    }

    /// The value of `key` (case-insensitive) in `section`, without quotes.
    pub fn get(&self, section: Option<&str>, key: &str) -> Option<String> {
        let (i, s) = self.find(section, key)?;
        Some(unquote(&self.lines[i][s.start..s.end]).0.to_string())
    }

    /// Set `key` in `section`, keeping the line's spacing and quoting. With
    /// `add`, a missing key is appended to its section (and a missing
    /// section to the file) — INI and options files only; otherwise only
    /// existing keys change. Returns whether the file changed.
    pub fn set(&mut self, section: Option<&str>, key: &str, value: &str, add: bool) -> bool {
        if let Some((i, s)) = self.find(section, key) {
            let line = &self.lines[i];
            if unquote(&line[s.start..s.end]).0 == value {
                return false;
            }
            let v = if s.quoted { format!("\"{value}\"") } else { value.to_string() };
            self.lines[i] = format!("{}{v}{}", &line[..s.start], &line[s.end..]);
            return true;
        }
        if !add || matches!(self.kind, Kind::KeyValues | Kind::Xml) {
            return false;
        }
        let sep = if self.kind == Kind::Ini { '=' } else { ':' };
        let line = format!("{key}{sep}{value}");
        match (self.range(section), section) {
            (Some((a, b)), Some(_)) => {
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
            (_, None) => self.lines.push(line),
        }
        true
    }

    /// Every key in the file (for tests and diagnostics).
    pub fn keys(&self) -> Vec<String> {
        self.lines.iter().filter_map(|l| self.entry(l).map(|(k, _)| k)).collect()
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
    fn spacing_and_quotes_are_kept() {
        let mut d = Doc::ini(b"[Render.13]\nFrameRateCap = \"300.000000\"\nFullScreen = \"1\"\n");
        assert_eq!(d.get(Some("Render.13"), "FrameRateCap").as_deref(), Some("300.000000"));
        assert!(d.set(Some("Render.13"), "FrameRateCap", "600.000000", false));
        assert_eq!(String::from_utf8(d.to_bytes()).unwrap(), "[Render.13]\nFrameRateCap = \"600.000000\"\nFullScreen = \"1\"\n");
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
        assert_eq!(o.get(None, "renderClouds").as_deref(), Some("true"));
        assert!(o.set(None, "maxFps", "260", false));
        assert!(o.set(None, "enableVsync", "false", false));
        assert!(o.set(None, "renderClouds", "false", false));
        assert!(!o.set(None, "graphicsMode", "0", false));
        assert_eq!(String::from_utf8(o.to_bytes()).unwrap(), "version:3700\nmaxFps:260\nrenderClouds:\"false\"\nenableVsync:false\n");
    }

    #[test]
    fn valve_key_values() {
        let src = "\"video.cfg\"\r\n{\r\n\t\"Version\"\t\t\"14\"\r\n\t\"setting.mat_vsync\"\t\t\"1\"\r\n\t\"setting.r_low_latency\"\t\t\"0\"\r\n}\r\n";
        let mut d = Doc::key_values(src.as_bytes());
        assert_eq!(d.get(None, "setting.mat_vsync").as_deref(), Some("1"));
        assert!(d.set(None, "setting.mat_vsync", "0", true));
        assert!(d.set(None, "setting.r_low_latency", "1", false));
        assert!(!d.set(None, "setting.not_there", "1", true), "never added to a KeyValues file");
        assert_eq!(String::from_utf8(d.to_bytes()).unwrap(), "\"video.cfg\"\r\n{\r\n\t\"Version\"\t\t\"14\"\r\n\t\"setting.mat_vsync\"\t\t\"0\"\r\n\t\"setting.r_low_latency\"\t\t\"1\"\r\n}\r\n");
        assert_eq!(d.keys(), vec!["Version", "setting.mat_vsync", "setting.r_low_latency"]);
    }

    #[test]
    fn roblox_xml() {
        let src = "<roblox>\n<Item class=\"UserGameSettings\">\n<Properties>\n\t<int name=\"FramerateCap\">60</int>\n\t<token name=\"SavedQualityLevel\">7</token>\n</Properties>\n</Item>\n</roblox>\n";
        let mut d = Doc::xml(src.as_bytes());
        assert_eq!(d.get(None, "FramerateCap").as_deref(), Some("60"));
        assert!(d.set(None, "FramerateCap", "240", true));
        assert!(!d.set(None, "Missing", "1", true));
        assert!(String::from_utf8(d.to_bytes()).unwrap().contains("\t<int name=\"FramerateCap\">240</int>\n"));
    }
}
