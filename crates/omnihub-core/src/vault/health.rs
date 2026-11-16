//! Password health: weak, reused and old passwords, accounts that could use
//! two-factor codes, and (on request) passwords that appear in known data
//! breaches.
//!
//! The breach check uses Have I Been Pwned's range API with k-anonymity:
//! only the first five characters of each password's SHA-1 hash are sent,
//! and the answer (every hash suffix with that prefix, padded with decoys)
//! is matched here. Passwords never leave the PC.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use super::{generator, Entry, EntryKind};

/// Older than this counts as "old".
const OLD_AFTER_DAYS: i64 = 365;

/// Sites that offer authenticator-app codes (registrable domains).
const TWO_FACTOR_SITES: &[&str] = &[
    "google.com", "gmail.com", "youtube.com", "microsoft.com", "live.com", "outlook.com", "office.com", "xbox.com", "github.com", "gitlab.com", "discord.com", "amazon.com", "apple.com", "icloud.com", "steampowered.com", "steamcommunity.com", "epicgames.com", "paypal.com", "dropbox.com", "twitter.com", "x.com", "facebook.com", "instagram.com", "linkedin.com", "reddit.com", "twitch.tv", "coinbase.com", "binance.com", "kraken.com", "proton.me", "protonmail.com", "cloudflare.com", "digitalocean.com", "npmjs.com", "slack.com", "notion.so", "zoom.us", "ubisoft.com", "ea.com", "battle.net", "blizzard.com", "nintendo.com", "playstation.com",
];
/// Brands with a site per country (amazon.de, google.co.uk).
const TWO_FACTOR_BRANDS: &[&str] = &["amazon", "google"];

fn offers_two_factor(url: &str) -> bool {
    let Some(page) = crate::browser::matching::parse_url(url) else { return false };
    let site = crate::browser::matching::site_of(&page.host);
    TWO_FACTOR_SITES.contains(&site.as_str()) || site.split('.').next().is_some_and(|label| TWO_FACTOR_BRANDS.contains(&label))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HealthItem {
    pub id: String,
    pub title: String,
    pub username: String,
    pub url: String,
    /// Strength 0–4 (weak list), or how many times it was seen in breaches.
    pub detail: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    /// Entries with a password.
    pub checked: usize,
    /// 0–100: the share of passwords with no problem.
    pub score: u8,
    pub weak: Vec<HealthItem>,
    /// Groups of entries sharing one password.
    pub reused: Vec<Vec<HealthItem>>,
    pub old: Vec<HealthItem>,
    pub missing_two_factor: Vec<HealthItem>,
}

pub(crate) fn item(e: &Entry, detail: u64) -> HealthItem {
    HealthItem { id: e.id.clone(), title: e.title.clone(), username: if e.username.is_empty() { e.email.clone() } else { e.username.clone() }, url: e.url.clone(), detail }
}

pub fn report(entries: &[Entry], now: i64) -> HealthReport {
    let with_pw: Vec<&Entry> = entries.iter().filter(|e| !e.password.is_empty() && matches!(e.kind, EntryKind::Login | EntryKind::Email | EntryKind::Other | EntryKind::Wifi)).collect();
    let mut weak = Vec::new();
    let mut by_hash: HashMap<[u8; 32], Vec<&Entry>> = HashMap::new();
    let mut old = Vec::new();
    let mut missing = Vec::new();
    let mut flagged = std::collections::HashSet::new();
    for e in &with_pw {
        let s = generator::strength(&e.password).score;
        if s <= 1 {
            weak.push(item(e, s as u64));
            flagged.insert(e.id.clone());
        }
        let h: [u8; 32] = sha2::Sha256::digest(e.password.as_bytes()).into();
        by_hash.entry(h).or_default().push(e);
        if e.password_changed > 0 && now - e.password_changed > OLD_AFTER_DAYS * 86_400 {
            old.push(item(e, ((now - e.password_changed) / 86_400) as u64));
        }
        if e.totp.is_empty() && e.kind != EntryKind::Wifi && offers_two_factor(&e.url) {
            missing.push(item(e, 0));
        }
    }
    let mut reused: Vec<Vec<HealthItem>> = by_hash
        .into_values()
        .filter(|g| g.len() > 1)
        .map(|g| {
            for e in &g {
                flagged.insert(e.id.clone());
            }
            let mut v: Vec<HealthItem> = g.iter().map(|e| item(e, 0)).collect();
            v.sort_by_key(|i| i.title.to_lowercase());
            v
        })
        .collect();
    reused.sort_by_key(|g| std::cmp::Reverse(g.len()));
    weak.sort_by_key(|i| (i.detail, i.title.to_lowercase()));
    old.sort_by_key(|i| std::cmp::Reverse(i.detail));
    let checked = with_pw.len();
    let score = (100 * (checked - flagged.len())).checked_div(checked).map_or(100, |s| s as u8);
    HealthReport { checked, score, weak, reused, old, missing_two_factor: missing }
}

/// Upper-case hex SHA-1 of a password, as the range API expects.
pub fn sha1_hex(password: &str) -> String {
    hex::encode_upper(Sha1::digest(password.as_bytes()))
}

/// How often each hash appears in breaches, asking `fetch(prefix)` for each
/// distinct five-character prefix (the API returns `SUFFIX:COUNT` lines).
pub fn breach_counts(hashes: &[String], fetch: &dyn Fn(&str) -> Result<String, String>) -> Result<HashMap<String, u64>, String> {
    let mut by_prefix: HashMap<&str, Vec<&String>> = HashMap::new();
    for h in hashes {
        if h.len() == 40 {
            by_prefix.entry(&h[..5]).or_default().push(h);
        }
    }
    let mut out = HashMap::new();
    for (prefix, wanted) in by_prefix {
        let body = fetch(prefix)?;
        let found: HashMap<&str, u64> = body.lines().filter_map(|l| l.trim().split_once(':')).map(|(s, c)| (s, c.trim().parse().unwrap_or(0))).collect();
        for h in wanted {
            let n = found.get(&h[5..]).copied().unwrap_or(0);
            // Padding entries have a count of 0.
            if n > 0 {
                out.insert(h.clone(), n);
            }
        }
    }
    Ok(out)
}

/// The real range API (over HTTPS, with response padding).
pub fn fetch_range(prefix: &str) -> Result<String, String> {
    if prefix.len() != 5 || !prefix.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("bad prefix".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(20))).build().into();
    agent
        .get(&format!("https://api.pwnedpasswords.com/range/{prefix}"))
        .header("Add-Padding", "true")
        .header("User-Agent", concat!("OmniHub/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|e| format!("could not reach the breach database: {e}"))?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(id: &str, title: &str, pw: &str, url: &str, changed: i64, totp: &str) -> Entry {
        // (Entry implements Drop, so no struct-update syntax.)
        let mut x = Entry::default();
        (x.id, x.title, x.username, x.password, x.url, x.password_changed, x.totp) = (id.into(), title.into(), "alex".into(), pw.into(), url.into(), changed, totp.into());
        x
    }

    #[test]
    fn finds_weak_reused_old_and_missing_2fa() {
        let now = 2_000_000_000;
        let entries = vec![
            e("1", "GitHub", "vX7#qL2!pN9@wR4$zK", "https://github.com", now - 10 * 86_400, ""),
            e("2", "Netflix", "123456", "https://netflix.com", now - 800 * 86_400, ""),
            e("3", "Forum", "Shared-Pass-2024!x", "https://forum.example", now, ""),
            e("4", "Shop", "Shared-Pass-2024!x", "https://shop.example", now, ""),
            e("5", "Google", "Tr0ub4dor&3-sunrise-xx", "https://accounts.google.com", now, "JBSWY3DPEHPK3PXP"),
        ];
        let r = report(&entries, now);
        assert_eq!(r.checked, 5);
        assert_eq!(r.weak.iter().map(|i| i.title.as_str()).collect::<Vec<_>>(), ["Netflix"]);
        assert_eq!(r.reused.len(), 1);
        assert_eq!(r.reused[0].iter().map(|i| i.title.as_str()).collect::<Vec<_>>(), ["Forum", "Shop"]);
        assert_eq!(r.old.iter().map(|i| (i.title.as_str(), i.detail)).collect::<Vec<_>>(), [("Netflix", 800)]);
        // GitHub supports 2FA and has none; Google already has a code.
        assert_eq!(r.missing_two_factor.iter().map(|i| i.title.as_str()).collect::<Vec<_>>(), ["GitHub"]);
        // Domain matching, not substrings: netflix.com is not x.com.
        assert!(offers_two_factor("https://x.com/login") && offers_two_factor("amazon.co.uk") && !offers_two_factor("https://login.microsoftonline.com.evil.io"));
        assert!(!offers_two_factor("https://netflix.com") && !offers_two_factor("https://idea.com") && !offers_two_factor(""));
        // Netflix, Forum and Shop have problems: 2 of 5 are fine.
        assert_eq!(r.score, 40);
        assert_eq!(report(&[], now).score, 100);
    }

    #[test]
    fn breach_matching_uses_only_prefixes() {
        let pw = sha1_hex("password");
        assert_eq!(pw, "5BAA61E4C9B93F3F0682250B6CF8331B7EE68FD8");
        let other = sha1_hex("a much better passphrase");
        let asked = std::cell::RefCell::new(Vec::new());
        let fetch = |prefix: &str| {
            asked.borrow_mut().push(prefix.to_string());
            Ok(if prefix == "5BAA6" { "1E4C9B93F3F0682250B6CF8331B7EE68FD8:9545824\r\n0000000000000000000000000000000000A:0\r\n".to_string() } else { "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF:3\r\n".to_string() })
        };
        let counts = breach_counts(&[pw.clone(), other.clone(), pw.clone()], &fetch).unwrap();
        assert_eq!(counts.get(&pw), Some(&9_545_824));
        assert!(!counts.contains_key(&other));
        let mut a = asked.into_inner();
        a.sort();
        let mut want = vec!["5BAA6".to_string(), other[..5].to_string()];
        want.sort();
        // One request per distinct prefix, and nothing more than the prefix.
        assert_eq!(a, want);
        assert!(fetch_range("zzzzz").is_err());
    }
}
