//! Downloads that keep going when one place is down or blocked.
//!
//! A file is tried at its publisher first, then at mirrors that copy the
//! same files at the same paths (BMCLAPI's Minecraft and Forge mirror, and
//! Maven Central's second address). Each address is tried a few times with a
//! growing pause, through the proxy Windows is set to use (or the
//! HTTPS_PROXY variable) and then directly, in case that proxy is the
//! problem. Nothing is accepted unless it passes the caller's check: the
//! checksum the publisher lists, or one built into OmniHub. So a mirror can
//! make a download work, but it can't change what gets installed.

use std::io::Read;
use std::time::Duration;

/// BMCLAPI: a public mirror of Mojang's, Forge's, NeoForge's and Fabric's files.
pub const BMCL: &str = "https://bmclapi2.bangbang93.com";

/// Publisher prefix → mirror prefix with the same paths below it.
const MIRRORS: &[(&str, &str)] = &[
    ("https://maven.minecraftforge.net/", "https://bmclapi2.bangbang93.com/maven/"),
    ("https://files.minecraftforge.net/maven/", "https://maven.minecraftforge.net/"),
    ("https://files.minecraftforge.net/maven/", "https://bmclapi2.bangbang93.com/maven/"),
    ("https://libraries.minecraft.net/", "https://bmclapi2.bangbang93.com/maven/"),
    ("https://repo1.maven.org/maven2/", "https://repo.maven.apache.org/maven2/"),
    ("https://repo1.maven.org/maven2/", "https://bmclapi2.bangbang93.com/maven/"),
    ("https://maven.neoforged.net/releases/", "https://bmclapi2.bangbang93.com/maven/"),
    ("https://maven.fabricmc.net/", "https://bmclapi2.bangbang93.com/maven/"),
    ("https://piston-data.mojang.com/", "https://bmclapi2.bangbang93.com/"),
    ("https://launcher.mojang.com/", "https://bmclapi2.bangbang93.com/"),
];

/// `url` first, then the same file on each mirror that has it.
pub fn alternates(url: &str) -> Vec<String> {
    let mut out = vec![url.to_string()];
    for (from, to) in MIRRORS {
        if let Some(rest) = url.strip_prefix(from) {
            let u = format!("{to}{rest}");
            if !out.contains(&u) {
                out.push(u);
            }
        }
    }
    out
}

/// Every address for every one of `urls`, in order, without repeats.
pub fn expand(urls: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for u in urls {
        for a in alternates(u) {
            if !out.contains(&a) {
                out.push(a);
            }
        }
    }
    out
}

/// What one try came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Try {
    Got(Vec<u8>),
    /// Not there (404, 410): don't ask that address again.
    Missing(String),
    /// Might work on another try (no connection, 5xx, 429, cut off).
    Again(String),
}

/// The order things are tried in: every address through each route, each
/// up to `attempts` times, pausing `pause(n)` before the n-th retry. The
/// first answer that passes `ok` wins; otherwise the error lists what failed.
pub fn try_all(urls: &[String], routes: usize, attempts: u32, ok: &dyn Fn(&[u8]) -> bool, pause: &dyn Fn(u32), get: &mut dyn FnMut(usize, &str) -> Try) -> Result<Vec<u8>, String> {
    let mut why: Vec<String> = Vec::new();
    for route in 0..routes.max(1) {
        for u in urls {
            for n in 0..attempts.max(1) {
                if n > 0 {
                    pause(n);
                }
                match get(route, u) {
                    Try::Got(b) if ok(&b) => return Ok(b),
                    Try::Got(_) => {
                        why.push(format!("{} sent a file that didn't match its checksum", host(u)));
                        break;
                    }
                    Try::Missing(e) => {
                        why.push(e);
                        break;
                    }
                    Try::Again(e) => {
                        if n + 1 == attempts.max(1) {
                            why.push(e);
                        }
                    }
                }
            }
        }
    }
    why.dedup();
    Err(if why.is_empty() { "Nothing to download from.".into() } else { format!("Tried every source and none worked: {}.", why.join("; ")) })
}

fn host(u: &str) -> &str {
    u.split('/').nth(2).unwrap_or(u)
}

/// Fetches with the fallbacks above.
pub struct Fetcher {
    routes: Vec<ureq::Agent>,
    ua: String,
    attempts: u32,
}

impl Fetcher {
    pub fn new(ua: &str, global: Duration) -> Fetcher {
        let base = || ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(15))).timeout_global(Some(global)).http_status_as_error(false).https_only(true);
        // The proxy Windows (or HTTPS_PROXY) points at, then straight out.
        let routes = vec![base().build().into(), base().proxy(None).build().into()];
        Fetcher { routes, ua: ua.to_string(), attempts: 3 }
    }

    fn once(&self, route: usize, url: &str, limit: u64) -> Try {
        let h = host(url).to_string();
        let mut res = match self.routes[route].get(url).header("User-Agent", &self.ua).call() {
            Ok(r) => r,
            Err(e) => return Try::Again(format!("couldn't reach {h} ({e})")),
        };
        let status = res.status().as_u16();
        match status {
            200..=299 => {}
            404 | 410 => return Try::Missing(format!("{h} doesn't have it")),
            s => return Try::Again(format!("{h} answered {s}")),
        }
        let mut out = Vec::new();
        match res.body_mut().as_reader().take(limit).read_to_end(&mut out) {
            Ok(_) => Try::Got(out),
            Err(e) => Try::Again(format!("the download from {h} stopped ({e})")),
        }
    }

    /// The first copy of any of `urls` (and their mirrors) that passes `ok`.
    pub fn get_any(&self, urls: &[String], limit: u64, ok: &dyn Fn(&[u8]) -> bool) -> Result<Vec<u8>, String> {
        try_all(&expand(urls), self.routes.len(), self.attempts, ok, &|n| std::thread::sleep(Duration::from_millis(800 * u64::from(n * n))), &mut |r, u| self.once(r, u, limit))
    }

    /// `url` itself, with retries and both routes but no mirrors (for
    /// checksums, which must come from the publisher).
    pub fn get_official(&self, url: &str, limit: u64) -> Result<Vec<u8>, String> {
        try_all(&[url.to_string()], self.routes.len(), self.attempts, &|_| true, &|n| std::thread::sleep(Duration::from_millis(800 * u64::from(n * n))), &mut |r, u| self.once(r, u, limit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirrors_keep_the_path() {
        let a = alternates("https://maven.minecraftforge.net/net/minecraftforge/forge/1.8.9-11.15.1.2318-1.8.9/x.jar");
        assert_eq!(a[0], "https://maven.minecraftforge.net/net/minecraftforge/forge/1.8.9-11.15.1.2318-1.8.9/x.jar");
        assert_eq!(a[1], "https://bmclapi2.bangbang93.com/maven/net/minecraftforge/forge/1.8.9-11.15.1.2318-1.8.9/x.jar");
        assert_eq!(alternates("https://repo1.maven.org/maven2/a/b.jar").len(), 3);
        assert_eq!(alternates("https://example.com/a.jar"), vec!["https://example.com/a.jar".to_string()]);
        let e = expand(&["https://libraries.minecraft.net/a.jar".into(), "https://repo1.maven.org/maven2/a.jar".into()]);
        assert_eq!(e.iter().filter(|u| u.ends_with("/maven/a.jar")).count(), 1, "no repeats: {e:?}");
    }

    #[test]
    fn falls_back_retries_and_checks() {
        let urls = vec!["https://a/x".to_string(), "https://b/x".to_string()];
        let mut calls = Vec::new();
        // a: down twice then a bad file; b: missing on route 0, good on route 1.
        let r = try_all(&urls, 2, 3, &|b| b == b"good", &|_| {}, &mut |route, u| {
            calls.push(format!("{route}{u}"));
            match (route, u) {
                (0, "https://a/x") if calls.len() < 3 => Try::Again("down".into()),
                (_, "https://a/x") => Try::Got(b"evil".to_vec()),
                (0, _) => Try::Missing("gone".into()),
                _ => Try::Got(b"good".to_vec()),
            }
        });
        assert_eq!(r.unwrap(), b"good");
        assert_eq!(calls, ["0https://a/x", "0https://a/x", "0https://a/x", "0https://b/x", "1https://a/x", "1https://b/x"]);
        let e = try_all(&urls, 1, 2, &|_| true, &|_| {}, &mut |_, _| Try::Again("no network".into())).unwrap_err();
        assert!(e.contains("no network"), "{e}");
    }
}
