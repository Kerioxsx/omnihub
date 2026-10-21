//! Minecraft's NBT: the binary form (player files, level.dat, region
//! chunks; gzip, zlib or plain) and the text form (SNBT) a server prints
//! for `data get entity`. Both read into one `Tag` tree, with limits so a
//! damaged or hostile file can't use up memory.

use std::io::Read;

use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Tag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Bytes(Vec<i8>),
    Str(String),
    List(Vec<Tag>),
    Compound(Vec<(String, Tag)>),
    Ints(Vec<i32>),
    Longs(Vec<i64>),
}

impl Tag {
    pub fn get(&self, key: &str) -> Option<&Tag> {
        match self {
            Tag::Compound(v) => v.iter().find(|(k, _)| k == key).map(|(_, t)| t),
            _ => None,
        }
    }

    pub fn list(&self) -> &[Tag] {
        match self {
            Tag::List(v) => v,
            _ => &[],
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            Tag::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Any number, as f64.
    pub fn num(&self) -> Option<f64> {
        Some(match self {
            Tag::Byte(v) => *v as f64,
            Tag::Short(v) => *v as f64,
            Tag::Int(v) => *v as f64,
            Tag::Long(v) => *v as f64,
            Tag::Float(v) => *v as f64,
            Tag::Double(v) => *v,
            _ => return None,
        })
    }

    pub fn int(&self) -> Option<i64> {
        self.num().map(|n| n as i64)
    }

    pub fn longs(&self) -> &[i64] {
        match self {
            Tag::Longs(v) => v,
            _ => &[],
        }
    }

    /// As JSON for the window (numbers as numbers, arrays as lists).
    pub fn to_json(&self) -> Value {
        match self {
            Tag::Byte(v) => json!(v),
            Tag::Short(v) => json!(v),
            Tag::Int(v) => json!(v),
            Tag::Long(v) => json!(v),
            Tag::Float(v) => json!(v),
            Tag::Double(v) => json!(v),
            Tag::Bytes(v) => json!(v),
            Tag::Str(s) => json!(s),
            Tag::List(v) => Value::Array(v.iter().map(Tag::to_json).collect()),
            Tag::Compound(v) => Value::Object(v.iter().map(|(k, t)| (k.clone(), t.to_json())).collect()),
            Tag::Ints(v) => json!(v),
            Tag::Longs(v) => json!(v),
        }
    }
}

/// A player's UUID from the four ints NBT keeps it as.
pub fn uuid_of(t: &Tag) -> Option<String> {
    let Tag::Ints(v) = t else { return None };
    let [a, b, c, d] = v.as_slice() else { return None };
    let h = format!("{:08x}{:08x}{:08x}{:08x}", *a as u32, *b as u32, *c as u32, *d as u32);
    Some(format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..]))
}

const MAX_DEPTH: usize = 512;
/// Uncompressed size a file may grow to.
const MAX_INFLATED: u64 = 64 << 20;

/// Read a whole NBT file: gzip, zlib or uncompressed.
pub fn read(bytes: &[u8]) -> Result<Tag, String> {
    let raw = inflate(bytes)?;
    parse(&raw)
}

/// Undo gzip or zlib (whichever the bytes start with), within a size limit.
pub fn inflate(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let r = match bytes {
        [0x1f, 0x8b, ..] => flate2::read::GzDecoder::new(bytes).take(MAX_INFLATED).read_to_end(&mut out),
        [0x78, ..] => flate2::read::ZlibDecoder::new(bytes).take(MAX_INFLATED).read_to_end(&mut out),
        _ => return Ok(bytes.to_vec()),
    };
    r.map_err(|e| format!("The file is damaged: {e}"))?;
    Ok(out)
}

/// Parse uncompressed NBT: one named root tag.
pub fn parse(b: &[u8]) -> Result<Tag, String> {
    let mut r = Reader { b, at: 0 };
    let kind = r.u8()?;
    if kind == 0 {
        return Ok(Tag::Compound(vec![]));
    }
    let _name = r.string()?;
    r.payload(kind, 0)
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.b.len()).ok_or("The file ends too soon.")?;
        let s = &self.b[self.at..end];
        self.at = end;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    fn i16(&mut self) -> Result<i16, String> {
        Ok(i16::from_be_bytes(self.take(2)?.try_into().unwrap_or_default()))
    }

    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_be_bytes(self.take(4)?.try_into().unwrap_or_default()))
    }

    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_be_bytes(self.take(8)?.try_into().unwrap_or_default()))
    }

    fn len(&mut self, each: usize) -> Result<usize, String> {
        let n = self.i32()?;
        let n = usize::try_from(n).map_err(|_| "A negative length.".to_string())?;
        if n.saturating_mul(each.max(1)) > self.b.len() - self.at {
            return Err("A length longer than the file.".into());
        }
        Ok(n)
    }

    fn string(&mut self) -> Result<String, String> {
        let n = self.i16()? as u16 as usize;
        Ok(String::from_utf8_lossy(self.take(n)?).into_owned())
    }

    fn payload(&mut self, kind: u8, depth: usize) -> Result<Tag, String> {
        if depth > MAX_DEPTH {
            return Err("The data is nested too deeply.".into());
        }
        Ok(match kind {
            1 => Tag::Byte(self.u8()? as i8),
            2 => Tag::Short(self.i16()?),
            3 => Tag::Int(self.i32()?),
            4 => Tag::Long(self.i64()?),
            5 => Tag::Float(f32::from_bits(self.i32()? as u32)),
            6 => Tag::Double(f64::from_bits(self.i64()? as u64)),
            7 => {
                let n = self.len(1)?;
                Tag::Bytes(self.take(n)?.iter().map(|&x| x as i8).collect())
            }
            8 => Tag::Str(self.string()?),
            9 => {
                let inner = self.u8()?;
                let n = self.len(1)?;
                if inner == 0 && n > 0 {
                    return Err("A list of nothing.".into());
                }
                let mut v = Vec::with_capacity(n.min(4096));
                for _ in 0..n {
                    v.push(self.payload(inner, depth + 1)?);
                }
                Tag::List(v)
            }
            10 => {
                let mut v = Vec::new();
                loop {
                    let k = self.u8()?;
                    if k == 0 {
                        break;
                    }
                    let name = self.string()?;
                    v.push((name, self.payload(k, depth + 1)?));
                }
                Tag::Compound(v)
            }
            11 => {
                let n = self.len(4)?;
                Tag::Ints((0..n).map(|_| self.i32()).collect::<Result<_, _>>()?)
            }
            12 => {
                let n = self.len(8)?;
                Tag::Longs((0..n).map(|_| self.i64()).collect::<Result<_, _>>()?)
            }
            k => return Err(format!("Unknown NBT tag type {k}.")),
        })
    }
}

/// Parse SNBT, the text form (`{count: 1, Slot: 0b, id: "minecraft:stone"}`).
pub fn snbt(s: &str) -> Result<Tag, String> {
    let mut p = Snbt { c: s.chars().collect(), at: 0 };
    let t = p.value(0)?;
    p.ws();
    if p.at < p.c.len() {
        return Err(format!("Unexpected text after the data at {}.", p.at));
    }
    Ok(t)
}

struct Snbt {
    c: Vec<char>,
    at: usize,
}

impl Snbt {
    fn ws(&mut self) {
        while self.c.get(self.at).is_some_and(|c| c.is_whitespace()) {
            self.at += 1;
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.ws();
        self.c.get(self.at).copied()
    }

    fn eat(&mut self, ch: char) -> bool {
        if self.peek() == Some(ch) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn quoted(&mut self) -> Result<String, String> {
        let q = self.c[self.at];
        self.at += 1;
        let mut out = String::new();
        loop {
            let c = *self.c.get(self.at).ok_or("A quote is never closed.")?;
            self.at += 1;
            if c == q {
                return Ok(out);
            }
            if c == '\\' {
                let e = *self.c.get(self.at).ok_or("A quote is never closed.")?;
                self.at += 1;
                match e {
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    'u' => {
                        let hex: String = self.c.iter().skip(self.at).take(4).collect();
                        self.at += 4;
                        out.push(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32).unwrap_or('?'));
                    }
                    other => out.push(other),
                }
            } else {
                out.push(c);
            }
        }
    }

    fn word(&mut self) -> String {
        let start = self.at;
        while self.c.get(self.at).is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+')) {
            self.at += 1;
        }
        self.c[start..self.at].iter().collect()
    }

    fn key(&mut self) -> Result<String, String> {
        match self.peek() {
            Some('"') | Some('\'') => self.quoted(),
            Some(_) => {
                let w = self.word();
                if w.is_empty() {
                    Err(format!("A name was expected at {}.", self.at))
                } else {
                    Ok(w)
                }
            }
            None => Err("The data ends too soon.".into()),
        }
    }

    fn value(&mut self, depth: usize) -> Result<Tag, String> {
        if depth > MAX_DEPTH {
            return Err("The data is nested too deeply.".into());
        }
        match self.peek().ok_or("The data ends too soon.")? {
            '{' => {
                self.at += 1;
                let mut v = Vec::new();
                if self.eat('}') {
                    return Ok(Tag::Compound(v));
                }
                loop {
                    let k = self.key()?;
                    if !self.eat(':') {
                        return Err(format!("A ':' was expected after “{k}”."));
                    }
                    v.push((k, self.value(depth + 1)?));
                    if self.eat(',') {
                        continue;
                    }
                    if self.eat('}') {
                        return Ok(Tag::Compound(v));
                    }
                    return Err(format!("A ',' or '}}' was expected at {}.", self.at));
                }
            }
            '[' => {
                self.at += 1;
                // Typed arrays: [B; 1b, 2b], [I; 1, 2], [L; 1L].
                let typed = match (self.c.get(self.at), self.c.get(self.at + 1)) {
                    (Some(t @ ('B' | 'I' | 'L')), Some(';')) => Some(*t),
                    _ => None,
                };
                if typed.is_some() {
                    self.at += 2;
                }
                let mut v = Vec::new();
                if !self.eat(']') {
                    loop {
                        v.push(self.value(depth + 1)?);
                        if self.eat(',') {
                            continue;
                        }
                        if self.eat(']') {
                            break;
                        }
                        return Err(format!("A ',' or ']' was expected at {}.", self.at));
                    }
                }
                Ok(match typed {
                    Some('B') => Tag::Bytes(v.iter().filter_map(Tag::int).map(|n| n as i8).collect()),
                    Some('I') => Tag::Ints(v.iter().filter_map(Tag::int).map(|n| n as i32).collect()),
                    Some('L') => Tag::Longs(v.iter().filter_map(Tag::int).collect()),
                    _ => Tag::List(v),
                })
            }
            '"' | '\'' => Ok(Tag::Str(self.quoted()?)),
            _ => {
                let w = self.word();
                if w.is_empty() {
                    return Err(format!("Unexpected “{}” at {}.", self.c[self.at], self.at));
                }
                Ok(number(&w).unwrap_or(match w.as_str() {
                    "true" => Tag::Byte(1),
                    "false" => Tag::Byte(0),
                    _ => Tag::Str(w),
                }))
            }
        }
    }
}

/// A plain SNBT word as a number, by its suffix (1b, 2s, 3L, 4.5f, 6.0d, 7).
fn number(w: &str) -> Option<Tag> {
    let (body, suffix) = match w.char_indices().last() {
        Some((i, c)) if c.is_ascii_alphabetic() => (&w[..i], Some(c.to_ascii_lowercase())),
        _ => (w, None),
    };
    if body.is_empty() || !body.chars().all(|c| c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E' | '_')) || !body.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let body = body.replace('_', "");
    Some(match suffix {
        Some('b') => Tag::Byte(body.parse::<i64>().ok()? as i8),
        Some('s') => Tag::Short(body.parse::<i64>().ok()? as i16),
        Some('l') => Tag::Long(body.parse().ok()?),
        Some('f') => Tag::Float(body.parse().ok()?),
        Some('d') => Tag::Double(body.parse().ok()?),
        Some(_) => return None,
        None if body.contains(['.', 'e', 'E']) => Tag::Double(body.parse().ok()?),
        None => Tag::Int(body.parse::<i64>().ok()? as i32),
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Write NBT (for tests): one named root compound.
    pub fn write(root: &Tag) -> Vec<u8> {
        fn kind(t: &Tag) -> u8 {
            match t {
                Tag::Byte(_) => 1,
                Tag::Short(_) => 2,
                Tag::Int(_) => 3,
                Tag::Long(_) => 4,
                Tag::Float(_) => 5,
                Tag::Double(_) => 6,
                Tag::Bytes(_) => 7,
                Tag::Str(_) => 8,
                Tag::List(_) => 9,
                Tag::Compound(_) => 10,
                Tag::Ints(_) => 11,
                Tag::Longs(_) => 12,
            }
        }
        fn s(out: &mut Vec<u8>, v: &str) {
            out.extend((v.len() as u16).to_be_bytes());
            out.extend(v.as_bytes());
        }
        fn p(out: &mut Vec<u8>, t: &Tag) {
            match t {
                Tag::Byte(v) => out.push(*v as u8),
                Tag::Short(v) => out.extend(v.to_be_bytes()),
                Tag::Int(v) => out.extend(v.to_be_bytes()),
                Tag::Long(v) => out.extend(v.to_be_bytes()),
                Tag::Float(v) => out.extend(v.to_bits().to_be_bytes()),
                Tag::Double(v) => out.extend(v.to_bits().to_be_bytes()),
                Tag::Bytes(v) => {
                    out.extend((v.len() as i32).to_be_bytes());
                    out.extend(v.iter().map(|&x| x as u8));
                }
                Tag::Str(v) => s(out, v),
                Tag::List(v) => {
                    out.push(v.first().map(kind).unwrap_or(0));
                    out.extend((v.len() as i32).to_be_bytes());
                    v.iter().for_each(|x| p(out, x));
                }
                Tag::Compound(v) => {
                    for (k, x) in v {
                        out.push(kind(x));
                        s(out, k);
                        p(out, x);
                    }
                    out.push(0);
                }
                Tag::Ints(v) => {
                    out.extend((v.len() as i32).to_be_bytes());
                    v.iter().for_each(|x| out.extend(x.to_be_bytes()));
                }
                Tag::Longs(v) => {
                    out.extend((v.len() as i32).to_be_bytes());
                    v.iter().for_each(|x| out.extend(x.to_be_bytes()));
                }
            }
        }
        let mut out = vec![10];
        s(&mut out, "");
        p(&mut out, root);
        out
    }

    pub fn gzip(b: &[u8]) -> Vec<u8> {
        use std::io::Write;
        let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        e.write_all(b).unwrap();
        e.finish().unwrap()
    }

    #[test]
    fn snbt_from_a_real_server() {
        // Lines a 26.1.2 server printed for `data get entity Steve …`.
        let inv = snbt(r#"[{count: 1, Slot: 0b, id: "minecraft:diamond_sword"}, {count: 32, Slot: 1b, id: "minecraft:oak_log"}, {count: 1, Slot: 2b, id: "minecraft:diamond_helmet"}]"#).unwrap();
        assert_eq!(inv.list().len(), 3);
        assert_eq!(inv.list()[1].get("count"), Some(&Tag::Int(32)));
        assert_eq!(inv.list()[1].get("Slot"), Some(&Tag::Byte(1)));
        assert_eq!(inv.list()[0].get("id").and_then(Tag::str), Some("minecraft:diamond_sword"));
        assert_eq!(snbt("[-98.5d, 95.0d, -108.5d]").unwrap(), Tag::List(vec![Tag::Double(-98.5), Tag::Double(95.0), Tag::Double(-108.5)]));
        assert_eq!(snbt(r#""minecraft:overworld""#).unwrap(), Tag::Str("minecraft:overworld".into()));
        assert_eq!(snbt("20.0f").unwrap(), Tag::Float(20.0));
        assert_eq!(snbt("0").unwrap(), Tag::Int(0));
        assert_eq!(snbt("{}").unwrap(), Tag::Compound(vec![]));
        // Components, quoted keys, escapes, typed arrays, booleans.
        let t = snbt(r#"{components: {"minecraft:custom_name": '{"text":"Big \'Bob\'"}', "minecraft:unbreakable": {}}, UUID: [I; -1, 2, 3, 4], ok: true, big: 3L, half: 0.5}"#).unwrap();
        assert_eq!(t.get("components").and_then(|c| c.get("minecraft:custom_name")).and_then(Tag::str), Some(r#"{"text":"Big 'Bob'"}"#));
        assert_eq!(uuid_of(t.get("UUID").unwrap()).unwrap(), "ffffffff-0000-0002-0000-000300000004");
        assert_eq!((t.get("ok"), t.get("big"), t.get("half")), (Some(&Tag::Byte(1)), Some(&Tag::Long(3)), Some(&Tag::Double(0.5))));
        assert!(snbt("{a: 1").is_err());
        assert!(snbt("{a 1}").is_err());
        let deep = "[".repeat(2000) + &"]".repeat(2000);
        assert!(snbt(&deep).unwrap_err().contains("deeply"));
    }

    #[test]
    fn binary_nbt_round_trip_and_limits() {
        let root = Tag::Compound(vec![
            ("Pos".into(), Tag::List(vec![Tag::Double(1.5), Tag::Double(64.0), Tag::Double(-3.25)])),
            ("Dimension".into(), Tag::Str("minecraft:the_nether".into())),
            ("Health".into(), Tag::Float(17.5)),
            ("Inventory".into(), Tag::List(vec![Tag::Compound(vec![("Slot".into(), Tag::Byte(0)), ("id".into(), Tag::Str("minecraft:torch".into())), ("count".into(), Tag::Int(12))])])),
            ("data".into(), Tag::Longs(vec![1, -2, i64::MAX])),
            ("UUID".into(), Tag::Ints(vec![1, 2, 3, 4])),
        ]);
        let raw = write(&root);
        assert_eq!(parse(&raw).unwrap(), root);
        assert_eq!(read(&gzip(&raw)).unwrap(), root);
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
        std::io::Write::write_all(&mut z, &raw).unwrap();
        assert_eq!(read(&z.finish().unwrap()).unwrap(), root);
        assert_eq!(root.get("Inventory").unwrap().to_json()[0]["count"], 12);
        // Cut short, or a length bigger than the file: an error, not a crash.
        assert!(parse(&raw[..raw.len() - 5]).is_err());
        let mut bad = vec![10, 0, 0, 12, 0, 1, b'x'];
        bad.extend(i32::MAX.to_be_bytes());
        assert!(parse(&bad).unwrap_err().contains("longer"));
    }
}
