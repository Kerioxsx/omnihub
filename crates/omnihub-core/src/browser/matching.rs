//! Which vault entries belong to the page a browser is showing.
//!
//! A login fills only on the site it was saved for: the same host, the same
//! registrable domain (public suffix list), or a known group of domains one
//! company uses for one account (google.com and gmail.com…). An entry saved
//! for https:// never fills into an http:// page (except on this PC or the
//! local network), so a downgraded page cannot collect the password.

use std::net::IpAddr;

/// Domain groups that share one account.
const GROUPS: &[&[&str]] = &[
    &["google.com", "gmail.com", "googlemail.com", "youtube.com"],
    &["microsoft.com", "live.com", "outlook.com", "hotmail.com", "office.com", "microsoftonline.com", "xbox.com", "skype.com", "msn.com"],
    &["apple.com", "icloud.com", "me.com", "mac.com"],
    &["yahoo.com", "ymail.com", "rocketmail.com"],
    &["proton.me", "protonmail.com", "pm.me"],
    &["discord.com", "discordapp.com"],
    &["steampowered.com", "steamcommunity.com"],
    &["epicgames.com", "unrealengine.com", "fortnite.com"],
    &["ea.com", "origin.com"],
    &["paypal.com", "paypal.me"],
    &["amazon.com", "amazon.co.uk", "amazon.de", "amazon.fr", "amazon.ca", "amazon.es", "amazon.it", "amazon.co.jp", "amazon.com.au"],
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageUrl {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
}

/// Scheme, host and port of a URL; a bare "gmail.com" counts as https.
pub fn parse_url(url: &str) -> Option<PageUrl> {
    let u = url.trim();
    if u.is_empty() {
        return None;
    }
    let (scheme, rest) = match u.split_once("://") {
        Some((s, r)) => (s.to_ascii_lowercase(), r),
        None => ("https".to_string(), u),
    };
    if !matches!(scheme.as_str(), "http" | "https") {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let (host, port) = if let Some(v6) = authority.strip_prefix('[') {
        let (h, after) = v6.split_once(']')?;
        (h.to_string(), after.strip_prefix(':').and_then(|p| p.parse().ok()))
    } else {
        match authority.rsplit_once(':') {
            Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => (h.to_string(), p.parse().ok()),
            _ => (authority.to_string(), None),
        }
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let hostname_like = !host.is_empty() && host.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_')) && !host.starts_with(['-', '.']);
    if !hostname_like && host.parse::<IpAddr>().is_err() {
        return None;
    }
    Some(PageUrl { scheme, host, port })
}

fn is_local(host: &str) -> bool {
    host == "localhost" || host.ends_with(".local") || host.parse::<IpAddr>().is_ok_and(|ip| crate::remote::net::is_allowed_peer(ip, false))
}

/// The registrable domain ("accounts.google.com" → "google.com"); IP
/// addresses and single-label names stay as they are.
pub fn site_of(host: &str) -> String {
    if host.parse::<IpAddr>().is_ok() || !host.contains('.') {
        return host.to_string();
    }
    match psl::domain_str(host) {
        Some(d) => d.to_string(),
        None => host.to_string(),
    }
}

fn group_of(site: &str) -> Option<&'static [&'static str]> {
    GROUPS.iter().copied().find(|g| g.contains(&site))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Quality {
    /// The entry has no URL, but its email is with this provider (a Gmail
    /// address on accounts.google.com).
    EmailProvider,
    /// Another domain of the same company (gmail.com for google.com).
    Related,
    /// Same registrable domain (login.example.com for example.com).
    Site,
    /// Same host.
    Exact,
}

/// How well an entry fits `page`, or `None` when it must not be offered.
pub fn entry_matches(entry_url: &str, entry_email: &str, entry_username: &str, page: &PageUrl) -> Option<Quality> {
    if let Some(saved) = parse_url(entry_url) {
        // Never hand an https password to a plain-http page on the internet.
        if saved.scheme == "https" && page.scheme == "http" && !is_local(&page.host) {
            return None;
        }
        if saved.host == page.host || saved.host.strip_prefix("www.") == Some(&page.host) || page.host.strip_prefix("www.") == Some(&saved.host) {
            return (saved.port.is_none() || saved.port == page.port).then_some(Quality::Exact);
        }
        let (a, b) = (site_of(&saved.host), site_of(&page.host));
        if a == b && a.contains('.') && a.parse::<IpAddr>().is_err() {
            return Some(Quality::Site);
        }
        if group_of(&a).is_some_and(|g| g.contains(&b.as_str())) {
            return Some(Quality::Related);
        }
        return None;
    }
    if page.scheme != "https" {
        return None;
    }
    let email = if entry_email.contains('@') { entry_email } else { entry_username };
    let domain = email.rsplit_once('@').map(|(_, d)| d.trim().to_ascii_lowercase())?;
    let page_site = site_of(&page.host);
    group_of(&domain).filter(|g| g.contains(&page_site.as_str())).map(|_| Quality::EmailProvider)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(u: &str) -> PageUrl {
        parse_url(u).unwrap()
    }

    #[test]
    fn urls() {
        assert_eq!(page("https://Accounts.Google.com/signin?x=1"), PageUrl { scheme: "https".into(), host: "accounts.google.com".into(), port: None });
        assert_eq!(page("gmail.com").scheme, "https");
        assert_eq!(page("http://192.168.1.1:8080/login").port, Some(8080));
        assert_eq!(page("https://user:pw@example.com/").host, "example.com");
        assert_eq!(page("http://[fd00::1]:80/").host, "fd00::1");
        assert!(parse_url("ftp://example.com").is_none());
        assert!(parse_url("javascript:alert(1)").is_none());
        assert_eq!(site_of("accounts.google.com"), "google.com");
        assert_eq!(site_of("www.bbc.co.uk"), "bbc.co.uk");
        assert_eq!(site_of("192.168.1.1"), "192.168.1.1");
    }

    #[test]
    fn matching_rules() {
        let g = page("https://accounts.google.com/v3/signin/identifier");
        assert_eq!(entry_matches("https://accounts.google.com", "", "", &g), Some(Quality::Exact));
        assert_eq!(entry_matches("https://mail.google.com", "", "", &g), Some(Quality::Site));
        assert_eq!(entry_matches("gmail.com", "", "", &g), Some(Quality::Related));
        assert_eq!(entry_matches("", "alex@gmail.com", "", &g), Some(Quality::EmailProvider));
        assert_eq!(entry_matches("", "", "alex@gmail.com", &g), Some(Quality::EmailProvider));
        assert_eq!(entry_matches("", "alex@outlook.com", "", &g), None);
        assert_eq!(entry_matches("https://github.com", "", "", &g), None);
        // Look-alike domains do not match.
        assert_eq!(entry_matches("https://google.com", "", "", &page("https://google.com.evil.io/login")), None);
        assert_eq!(entry_matches("https://example.com", "", "", &page("https://www.example.com/")), Some(Quality::Exact));
        // https entries never fill into http pages on the internet…
        assert_eq!(entry_matches("https://example.com", "", "", &page("http://example.com/login")), None);
        // …but routers and NAS boxes on the local network are fine.
        assert_eq!(entry_matches("https://192.168.1.1", "", "", &page("http://192.168.1.1/")), Some(Quality::Exact));
        // A port in the saved URL must match.
        assert_eq!(entry_matches("http://192.168.1.10:5000", "", "", &page("http://192.168.1.10:8080/")), None);
        // Shared hosting suffixes are separate sites.
        assert_eq!(entry_matches("https://alice.github.io", "", "", &page("https://mallory.github.io/")), None);
    }
}
