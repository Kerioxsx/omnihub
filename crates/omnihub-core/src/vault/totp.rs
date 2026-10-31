//! Two-factor codes (TOTP, RFC 6238) for vault entries.
//!
//! An entry can hold the secret from a site's "set up an authenticator app"
//! QR code, either as the `otpauth://totp/…` link or as the base32 secret.
//! The vault then shows the current 6-digit code and the browser extension
//! can fill it.

use hmac::{Hmac, KeyInit, Mac};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    Sha1,
    Sha256,
    Sha512,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotpSpec {
    pub secret: Vec<u8>,
    pub digits: u32,
    pub period: u64,
    pub algorithm: Algorithm,
    pub issuer: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TotpError {
    #[error("no two-factor secret is saved")]
    Missing,
    #[error("the two-factor secret is not valid base32")]
    BadSecret,
    #[error("the authenticator link is not a TOTP link")]
    BadLink,
}

/// Decode RFC 4648 base32 (case-insensitive, spaces, dashes and padding ignored).
pub fn base32_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let (mut buf, mut bits) = (0u64, 0u32);
    for c in s.chars().filter(|c| !matches!(c, ' ' | '-' | '=')) {
        let v = match c.to_ascii_uppercase() {
            c @ 'A'..='Z' => c as u64 - 'A' as u64,
            c @ '2'..='7' => c as u64 - '2' as u64 + 26,
            _ => return None,
        };
        buf = (buf << 5) | v;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    (!out.is_empty()).then_some(out)
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        k.eq_ignore_ascii_case(key).then(|| percent_encoding::percent_decode_str(&v.replace('+', " ")).decode_utf8_lossy().into_owned())
    })
}

/// Parse an `otpauth://totp/…` link or a bare base32 secret.
pub fn parse(s: &str) -> Result<TotpSpec, TotpError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(TotpError::Missing);
    }
    if let Some(rest) = s.strip_prefix("otpauth://") {
        let (kind, rest) = rest.split_once('/').ok_or(TotpError::BadLink)?;
        if !kind.eq_ignore_ascii_case("totp") {
            return Err(TotpError::BadLink);
        }
        let query = rest.split_once('?').map(|(_, q)| q).unwrap_or("");
        let secret = query_param(query, "secret").ok_or(TotpError::BadSecret)?;
        let digits = query_param(query, "digits").and_then(|d| d.parse().ok()).filter(|d| (6..=8).contains(d)).unwrap_or(6);
        let period = query_param(query, "period").and_then(|p| p.parse().ok()).filter(|p| (10..=300).contains(p)).unwrap_or(30);
        let algorithm = match query_param(query, "algorithm").map(|a| a.to_ascii_uppercase()).as_deref() {
            Some("SHA256") => Algorithm::Sha256,
            Some("SHA512") => Algorithm::Sha512,
            _ => Algorithm::Sha1,
        };
        return Ok(TotpSpec { secret: base32_decode(&secret).ok_or(TotpError::BadSecret)?, digits, period, algorithm, issuer: query_param(query, "issuer") });
    }
    Ok(TotpSpec { secret: base32_decode(s).ok_or(TotpError::BadSecret)?, digits: 6, period: 30, algorithm: Algorithm::Sha1, issuer: None })
}

fn hmac_of(algorithm: Algorithm, key: &[u8], msg: &[u8]) -> Vec<u8> {
    match algorithm {
        Algorithm::Sha1 => {
            let mut m = <Hmac<sha1::Sha1> as KeyInit>::new_from_slice(key).expect("HMAC takes any key length");
            m.update(msg);
            m.finalize().into_bytes().to_vec()
        }
        Algorithm::Sha256 => {
            let mut m = <Hmac<sha2::Sha256> as KeyInit>::new_from_slice(key).expect("HMAC takes any key length");
            m.update(msg);
            m.finalize().into_bytes().to_vec()
        }
        Algorithm::Sha512 => {
            let mut m = <Hmac<sha2::Sha512> as KeyInit>::new_from_slice(key).expect("HMAC takes any key length");
            m.update(msg);
            m.finalize().into_bytes().to_vec()
        }
    }
}

/// The code for Unix time `t`.
pub fn code_at(spec: &TotpSpec, t: u64) -> String {
    let counter = t / spec.period;
    let h = hmac_of(spec.algorithm, &spec.secret, &counter.to_be_bytes());
    let off = (h[h.len() - 1] & 0x0f) as usize;
    let bin = u32::from_be_bytes([h[off] & 0x7f, h[off + 1], h[off + 2], h[off + 3]]);
    format!("{:0width$}", bin % 10u32.pow(spec.digits), width = spec.digits as usize)
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// The current code for a stored secret.
pub fn code_now(secret: &str) -> Result<String, TotpError> {
    Ok(code_at(&parse(secret)?, now()))
}

/// Current code and seconds until it changes.
pub fn current(secret: &str) -> Result<(String, u64), TotpError> {
    let spec = parse(secret)?;
    let t = now();
    Ok((code_at(&spec, t), spec.period - t % spec.period))
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 6238 appendix B test vectors (8 digits).
    #[test]
    fn rfc6238_vectors() {
        let sha1 = TotpSpec { secret: b"12345678901234567890".to_vec(), digits: 8, period: 30, algorithm: Algorithm::Sha1, issuer: None };
        let sha256 = TotpSpec { secret: b"12345678901234567890123456789012".to_vec(), algorithm: Algorithm::Sha256, ..sha1.clone() };
        let sha512 = TotpSpec { secret: b"1234567890123456789012345678901234567890123456789012345678901234".to_vec(), algorithm: Algorithm::Sha512, ..sha1.clone() };
        assert_eq!(code_at(&sha1, 59), "94287082");
        assert_eq!(code_at(&sha256, 59), "46119246");
        assert_eq!(code_at(&sha512, 59), "90693936");
        assert_eq!(code_at(&sha1, 1111111109), "07081804");
        assert_eq!(code_at(&sha1, 20000000000), "65353130");
        assert_eq!(code_at(&sha256, 20000000000), "77737706");
    }

    #[test]
    fn links_and_secrets() {
        // "12345678901234567890" in base32.
        let b32 = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        assert_eq!(base32_decode(b32).unwrap(), b"12345678901234567890");
        assert_eq!(base32_decode("gezd gnbv gy3t qojq gezd gnbv gy3t qojq").unwrap(), b"12345678901234567890");
        assert!(base32_decode("not base32!").is_none());
        let s = parse(&format!("otpauth://totp/Google%3Aalex%40gmail.com?secret={b32}&issuer=Google&digits=8&period=30")).unwrap();
        assert_eq!((s.digits, s.period, s.issuer.as_deref()), (8, 30, Some("Google")));
        assert_eq!(code_at(&s, 59), "94287082");
        assert_eq!(parse(b32).unwrap().digits, 6);
        assert_eq!(parse(""), Err(TotpError::Missing));
        assert_eq!(parse("otpauth://hotp/x?secret=GEZD"), Err(TotpError::BadLink));
        let (code, left) = current(b32).unwrap();
        assert_eq!(code.len(), 6);
        assert!((1..=30).contains(&left));
    }
}
