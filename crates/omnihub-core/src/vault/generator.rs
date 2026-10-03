//! Password generation and a strength estimate.

use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GeneratorOptions {
    pub length: usize,
    pub lowercase: bool,
    pub uppercase: bool,
    pub digits: bool,
    pub symbols: bool,
    pub avoid_ambiguous: bool,
}

impl Default for GeneratorOptions {
    fn default() -> Self {
        GeneratorOptions { length: 20, lowercase: true, uppercase: true, digits: true, symbols: true, avoid_ambiguous: true }
    }
}

const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS: &str = "0123456789";
const SYMBOLS: &str = "!@#$%^&*()-_=+[]{};:,.?/~";
const AMBIGUOUS: &str = "Il1O0o|`'\"";

pub fn generate(opts: &GeneratorOptions) -> String {
    let mut classes: Vec<Vec<char>> = Vec::new();
    for (on, set) in [(opts.lowercase, LOWER), (opts.uppercase, UPPER), (opts.digits, DIGITS), (opts.symbols, SYMBOLS)] {
        if on {
            let chars: Vec<char> = set.chars().filter(|c| !opts.avoid_ambiguous || !AMBIGUOUS.contains(*c)).collect();
            classes.push(chars);
        }
    }
    if classes.is_empty() {
        classes.push(LOWER.chars().collect());
    }
    let length = opts.length.clamp(classes.len().max(4), 128);
    let all: Vec<char> = classes.iter().flatten().copied().collect();
    let mut rng = rand::rngs::OsRng;
    // One from each chosen class, the rest from the union, then shuffle.
    let mut out: Vec<char> = classes.iter().map(|c| c[rng.gen_range(0..c.len())]).collect();
    while out.len() < length {
        out.push(all[rng.gen_range(0..all.len())]);
    }
    out.shuffle(&mut rng);
    out.into_iter().collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Strength {
    /// 0 (very weak) to 4 (very strong).
    pub score: u8,
    pub bits: f64,
    pub label: String,
    pub feedback: Vec<String>,
}

const COMMON: &[&str] = &[
    "password", "123456", "12345678", "qwerty", "abc123", "letmein", "welcome", "monkey", "dragon", "football", "iloveyou",
    "admin", "login", "princess", "sunshine", "master", "shadow", "baseball", "superman", "trustno1", "passw0rd", "starwars",
    "azerty", "qwertz", "111111", "000000", "google", "gmail", "hello", "secret", "freedom", "whatever", "michael", "jordan",
];

pub fn strength(pw: &str) -> Strength {
    let mut feedback = Vec::new();
    let len = pw.chars().count();
    if len == 0 {
        return Strength { score: 0, bits: 0.0, label: "Empty".into(), feedback: vec!["Enter a password.".into()] };
    }
    let mut pool = 0u32;
    if pw.chars().any(|c| c.is_ascii_lowercase()) {
        pool += 26;
    }
    if pw.chars().any(|c| c.is_ascii_uppercase()) {
        pool += 26;
    }
    if pw.chars().any(|c| c.is_ascii_digit()) {
        pool += 10;
    }
    if pw.chars().any(|c| c.is_ascii_punctuation() || c == ' ') {
        pool += 33;
    }
    if !pw.is_ascii() {
        pool += 100;
    }
    // Repeated characters and simple runs carry little information.
    let chars: Vec<char> = pw.chars().collect();
    let mut effective = 0.0f64;
    for (i, c) in chars.iter().enumerate() {
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let run = prev.is_some_and(|p| *c as i64 - p as i64 == 1 || p as i64 - *c as i64 == 1);
        effective += if prev == Some(*c) || run { 0.25 } else { 1.0 };
    }
    let mut bits = effective * (pool.max(1) as f64).log2();
    let lower = pw.to_lowercase();
    if let Some(word) = COMMON.iter().find(|w| lower.contains(*w)) {
        bits = (bits - word.len() as f64 * 3.5).max(8.0);
        feedback.push(format!("Avoid common words like \"{word}\"."));
    }
    if len < 12 {
        feedback.push("Use at least 12 characters (16+ for important accounts).".into());
    }
    if pool <= 36 && len < 16 {
        feedback.push("Mix upper and lower case, digits and symbols, or use a longer passphrase.".into());
    }
    let score = match bits {
        b if b < 28.0 => 0,
        b if b < 40.0 => 1,
        b if b < 60.0 => 2,
        b if b < 80.0 => 3,
        _ => 4,
    };
    let label = ["Very weak", "Weak", "Fair", "Strong", "Very strong"][score as usize].to_string();
    Strength { score, bits: (bits * 10.0).round() / 10.0, label, feedback }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_passwords_follow_options() {
        for _ in 0..50 {
            let p = generate(&GeneratorOptions::default());
            assert_eq!(p.chars().count(), 20);
            assert!(p.chars().any(|c| c.is_ascii_lowercase()));
            assert!(p.chars().any(|c| c.is_ascii_uppercase()));
            assert!(p.chars().any(|c| c.is_ascii_digit()));
            assert!(p.chars().any(|c| SYMBOLS.contains(c)));
            assert!(!p.chars().any(|c| AMBIGUOUS.contains(c)));
        }
        let digits = generate(&GeneratorOptions { length: 6, lowercase: false, uppercase: false, symbols: false, ..Default::default() });
        assert!(digits.chars().all(|c| c.is_ascii_digit()));
        assert_ne!(generate(&GeneratorOptions::default()), generate(&GeneratorOptions::default()));
    }

    #[test]
    fn strength_ordering() {
        assert_eq!(strength("password").score, 0);
        assert!(strength("aaaaaaaaaaaa").score <= 1);
        assert!(strength("Tr0ub4dor&3").score >= 2);
        assert_eq!(strength(&generate(&GeneratorOptions::default())).score, 4);
        assert!(strength("correct horse battery staple").score >= 3);
    }
}
