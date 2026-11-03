//! Small HTTP helpers for features that read public pages and APIs (shops,
//! release feeds, patch notes, prices). A plain user agent naming the app;
//! nothing about the user is sent.

use std::time::Duration;

pub const UA: &str = concat!("OmniHub/", env!("CARGO_PKG_VERSION"), " (+https://github.com/Kerioxsx/omnihub)");
/// Some shops only answer browsers.
pub const BROWSER_UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";

fn agent(secs: u64) -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
    ureq::Agent::config_builder().tls_config(tls).timeout_global(Some(Duration::from_secs(secs))).http_status_as_error(false).build().into()
}

/// GET a page as text (up to 8 MB), with `ua` as the user agent.
pub fn get_text_as(url: &str, ua: &str, secs: u64) -> Result<String, String> {
    let mut res = agent(secs).get(url).header("User-Agent", ua).header("Accept-Language", "en").call().map_err(|e| format!("couldn't reach {}: {e}", host(url)))?;
    let status = res.status().as_u16();
    let body = res.body_mut().with_config().limit(8 << 20).read_to_string().map_err(|e| e.to_string())?;
    if status >= 400 {
        return Err(format!("{} answered {status}", host(url)));
    }
    Ok(body)
}

pub fn get_text(url: &str) -> Result<String, String> {
    get_text_as(url, UA, 20)
}

pub fn get_json(url: &str) -> Result<serde_json::Value, String> {
    serde_json::from_str(&get_text(url)?).map_err(|e| format!("{} sent something unexpected: {e}", host(url)))
}

/// POST JSON and read the answer line by line (for streamed progress).
pub fn post_lines(url: &str, body: &serde_json::Value, secs: u64, mut on_line: impl FnMut(&str)) -> Result<(), String> {
    use std::io::BufRead;
    let mut res = agent(secs).post(url).header("User-Agent", UA).header("Content-Type", "application/json").send(body.to_string()).map_err(|e| e.to_string())?;
    if res.status().as_u16() >= 400 {
        let text = res.body_mut().read_to_string().unwrap_or_default();
        return Err(if text.is_empty() { format!("{} answered {}", host(url), res.status()) } else { text });
    }
    let reader = std::io::BufReader::new(res.body_mut().as_reader());
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if !line.trim().is_empty() {
            on_line(&line);
        }
    }
    Ok(())
}

pub fn host(url: &str) -> &str {
    url.split("://").nth(1).unwrap_or(url).split('/').next().unwrap_or(url)
}
