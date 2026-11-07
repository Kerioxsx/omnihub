//! `server.properties`: Java's properties format, read and changed in
//! place (comments, order and settings OmniHub doesn't know stay as they
//! are).

use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Props {
    lines: Vec<Line>,
}

#[derive(Debug, Clone, PartialEq)]
enum Line {
    Other(String),
    Pair(String, String),
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('f') => out.push('\x0c'),
            Some('u') => {
                let hex: String = it.by_ref().take(4).collect();
                out.push(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32).unwrap_or('?'));
            }
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn escape(s: &str, key: bool) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '=' | ':' | '#' | '!' => {
                out.push('\\');
                out.push(c);
            }
            ' ' if key || i == 0 => out.push_str("\\ "),
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{u:04X}"));
                }
            }
            c => out.push(c),
        }
    }
    out
}

impl Props {
    pub fn parse(text: &str) -> Props {
        let mut lines = Vec::new();
        let mut pending = String::new();
        for raw in text.lines() {
            let l = if pending.is_empty() { raw.to_string() } else { format!("{pending}{}", raw.trim_start()) };
            let trimmed = l.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                lines.push(Line::Other(raw.to_string()));
                pending.clear();
                continue;
            }
            // A line ending in an odd number of backslashes carries on.
            let tail = l.chars().rev().take_while(|&c| c == '\\').count();
            if tail % 2 == 1 {
                pending = l[..l.len() - 1].to_string();
                continue;
            }
            pending.clear();
            let mut key = String::new();
            let mut chars = trimmed.char_indices().peekable();
            let mut rest_at = trimmed.len();
            while let Some((i, c)) = chars.next() {
                if c == '\\' {
                    key.push(c);
                    if let Some((_, n)) = chars.next() {
                        key.push(n);
                    }
                    continue;
                }
                if c == '=' || c == ':' || c.is_whitespace() {
                    rest_at = i;
                    break;
                }
                key.push(c);
            }
            let mut rest = trimmed[rest_at..].trim_start_matches([' ', '\t']);
            if let Some(r) = rest.strip_prefix(['=', ':']) {
                rest = r.trim_start_matches([' ', '\t']);
            }
            lines.push(Line::Pair(unescape(&key), unescape(rest)));
        }
        Props { lines }
    }

    pub fn load(path: &Path) -> Props {
        std::fs::read(path).map(|b| Props::parse(&String::from_utf8_lossy(&b))).unwrap_or_default()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.lines.iter().rev().find_map(|l| match l {
            Line::Pair(k, v) if k == key => Some(v.as_str()),
            _ => None,
        })
    }

    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get(key).unwrap_or(default)
    }

    pub fn set(&mut self, key: &str, value: &str) {
        for l in self.lines.iter_mut() {
            if let Line::Pair(k, v) = l {
                if k == key {
                    *v = value.to_string();
                    return;
                }
            }
        }
        self.lines.push(Line::Pair(key.to_string(), value.to_string()));
    }

    /// Every setting, in file order.
    pub fn pairs(&self) -> Vec<(String, String)> {
        self.lines
            .iter()
            .filter_map(|l| match l {
                Line::Pair(k, v) => Some((k.clone(), v.clone())),
                _ => None,
            })
            .collect()
    }

    pub fn text(&self) -> String {
        let mut out = String::new();
        for l in &self.lines {
            match l {
                Line::Other(s) => out.push_str(s),
                Line::Pair(k, v) => {
                    out.push_str(&escape(k, true));
                    out.push('=');
                    out.push_str(&escape(v, false));
                }
            }
            out.push('\n');
        }
        out
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let tmp = path.with_extension("properties.tmp");
        std::fs::write(&tmp, self.text()).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_and_writes_like_minecraft() {
        let text = "#Minecraft server properties\n#Tue Oct 20 19:48:52 UTC 2026\naccepts-transfers=false\nlevel-type=minecraft\\:normal\nmotd=A Minecraft Server\nmax-players=20\ngenerator-settings={}\nmotd2 = spaced \\u00E9t\\u00E9\nlong=one \\\n    two\n";
        let mut p = Props::parse(text);
        assert_eq!(p.get("level-type"), Some("minecraft:normal"));
        assert_eq!(p.get("motd"), Some("A Minecraft Server"));
        assert_eq!(p.get("motd2"), Some("spaced été"));
        assert_eq!(p.get("long"), Some("one two"));
        assert_eq!(p.get("generator-settings"), Some("{}"));
        p.set("max-players", "8");
        p.set("motd", "§aMy café: open!");
        p.set("white-list", "true");
        let out = p.text();
        assert!(out.starts_with("#Minecraft server properties\n#Tue Oct 20"), "comments stay");
        assert!(out.contains("max-players=8\n"));
        assert!(out.contains("level-type=minecraft\\:normal\n"));
        assert!(out.contains("motd=\\u00A7aMy caf\\u00E9\\: open\\!\n"));
        assert!(out.ends_with("white-list=true\n"), "new settings go at the end");
        let back = Props::parse(&out);
        assert_eq!(back.get("motd"), Some("§aMy café: open!"));
        assert_eq!(back.get("max-players"), Some("8"));
        assert_eq!(back.pairs().len(), 8);
    }
}
