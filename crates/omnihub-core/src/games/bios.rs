//! BIOS checker: the BIOS files you dumped from your own consoles, checked
//! against the known good dumps emulators expect (by MD5, the hash their
//! documentation lists). OmniHub never downloads a BIOS; it only reads the
//! folder you pick and says what's there, what's missing and what doesn't
//! match.

use std::path::Path;

use serde::Serialize;

/// (platform, file name, MD5, what it is).
pub const KNOWN: &[(&str, &str, &str, &str)] = &[
    ("psx", "scph1001.bin", "924e392ed05558ffdb115408c263dccf", "PlayStation (USA, SCPH-1001)"),
    ("psx", "scph5501.bin", "490f666e1afb15b7362b406ed1cea246", "PlayStation (USA, SCPH-5501)"),
    ("psx", "scph7001.bin", "1e68c231d0896b7eadcad1d7d8e76129", "PlayStation (USA, SCPH-7001)"),
    ("psx", "scph5500.bin", "8dd7d5296a650fac7319bce665a6a53c", "PlayStation (Japan, SCPH-5500)"),
    ("psx", "scph5502.bin", "32736f17079d0b2b7024407c39bd3050", "PlayStation (Europe, SCPH-5502)"),
    ("gba", "gba_bios.bin", "a860e8c0b6d573d191e4ec7db1b1e4f6", "Game Boy Advance"),
    ("gb", "gb_bios.bin", "32fbbd84168d3482956eb3c5051637f5", "Game Boy boot ROM"),
    ("gbc", "gbc_bios.bin", "dbfce9db9deaa2567f6a84fde55f9680", "Game Boy Color boot ROM"),
    ("nds", "bios7.bin", "df692a80a5b1bc90728bc3dfc76cd948", "Nintendo DS ARM7 BIOS"),
    ("nds", "bios9.bin", "a392174eb3e572fed6447e956bde4b25", "Nintendo DS ARM9 BIOS"),
];

/// A PlayStation 2 BIOS is a 4 MB file; there are many versions, and PCSX2
/// checks them itself.
const PS2_SIZE: u64 = 4 << 20;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BiosFile {
    pub platform: String,
    pub name: String,
    /// "good" (matches), "different" (the name is known, the file isn't),
    /// "missing", or "ps2" (a 4 MB file that looks like a PS2 BIOS).
    pub status: String,
    pub detail: String,
}

/// MD5 (RFC 1321), only for comparing with the published BIOS hashes.
pub fn md5(data: &[u8]) -> String {
    const S: [u32; 64] = [7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21];
    let k: Vec<u32> = (0..64).map(|i| ((i as f64 + 1.0).sin().abs() * 4_294_967_296.0) as u32).collect();
    let (mut a0, mut b0, mut c0, mut d0) = (0x6745_2301u32, 0xefcd_ab89u32, 0x98ba_dcfeu32, 0x1032_5476u32);
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_le_bytes());
    for chunk in msg.chunks(64) {
        let m: Vec<u32> = chunk.chunks(4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]])).collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    [a0, b0, c0, d0].iter().flat_map(|v| v.to_le_bytes()).map(|b| format!("{b:02x}")).collect()
}

/// What each known file says, by MD5 (names don't matter: a renamed good
/// dump is still found).
pub fn check(dir: &Path) -> Vec<BiosFile> {
    let mut by_hash = std::collections::HashMap::new();
    let mut names = std::collections::HashMap::new();
    let mut ps2 = Vec::new();
    for e in walkdir::WalkDir::new(dir).max_depth(3).into_iter().filter_map(Result::ok).filter(|e| e.file_type().is_file()) {
        let Ok(m) = e.metadata() else { continue };
        let name = e.file_name().to_string_lossy().to_string();
        if m.len() == PS2_SIZE {
            ps2.push(name.clone());
            continue;
        }
        // BIOS files are small; skip anything that can't be one.
        if m.len() > 1 << 20 {
            continue;
        }
        let Ok(bytes) = std::fs::read(e.path()) else { continue };
        by_hash.insert(md5(&bytes), name.clone());
        names.insert(name.to_lowercase(), name);
    }
    let mut out = Vec::new();
    for (platform, file, hash, what) in KNOWN {
        let (status, detail) = match (by_hash.get(*hash), names.get(*file)) {
            (Some(found), _) if found.eq_ignore_ascii_case(file) => ("good", format!("{what}: matches the known dump.")),
            (Some(found), _) => ("good", format!("{what}: {found} matches; emulators look for it as {file}.")),
            (None, Some(found)) => ("different", format!("{what}: {found} isn't the known dump (a bad dump, or another revision). It may still work.")),
            (None, None) => ("missing", format!("{what}: not in this folder.")),
        };
        out.push(BiosFile { platform: platform.to_string(), name: file.to_string(), status: status.into(), detail });
    }
    for name in ps2 {
        out.push(BiosFile { platform: "ps2".into(), name: name.clone(), status: "ps2".into(), detail: format!("{name} is the size of a PlayStation 2 BIOS; PCSX2 checks it when you pick it.") });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_and_checks() {
        assert_eq!(md5(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(md5(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(md5(b"The quick brown fox jumps over the lazy dog"), "9e107d9d372bb6826bd81d3542a419d6");
        assert_eq!(md5(&[b'a'; 1000]), "cabe45dcc9ae5b66ba86600cca6b8ba8");
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("gba_bios.bin"), b"not a bios").unwrap();
        std::fs::write(t.path().join("ps2.bin"), vec![0u8; 4 << 20]).unwrap();
        let r = check(t.path());
        let gba = r.iter().find(|f| f.name == "gba_bios.bin").unwrap();
        assert_eq!(gba.status, "different");
        assert_eq!(r.iter().find(|f| f.name == "scph1001.bin").unwrap().status, "missing");
        assert!(r.iter().any(|f| f.status == "ps2" && f.name == "ps2.bin"));
    }
}
