//! Updates from the GitHub releases OmniHub comes from: see whether a newer
//! version is out, download its installer, check it against the release's
//! SHA256SUMS.txt, and run it. The installer closes OmniHub, installs over
//! it (settings and data stay) and opens the new version.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;

pub const LATEST_RELEASE_API: &str = "https://api.github.com/repos/Kerioxsx/omnihub/releases/latest";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub page_url: String,
    pub published_at: Option<String>,
    /// The per-user installer (OmniHub_x.y.z_x64-setup.exe).
    pub setup: Option<Asset>,
    /// The MSI, for installs in Program Files.
    pub msi: Option<Asset>,
    pub sums_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate { checked_at: i64 },
    Available { release: Release },
    Downloading { version: String, done: u64, total: u64 },
    /// The installer is running; OmniHub is about to close.
    Installing { version: String },
    Failed { message: String },
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

/// The parts of GitHub's "latest release" answer OmniHub needs.
pub fn parse_release(json: &[u8]) -> Result<Release, String> {
    let r: GhRelease = serde_json::from_slice(json).map_err(|e| format!("unexpected answer from GitHub: {e}"))?;
    if r.draft || r.prerelease {
        return Err("the latest release is not final".into());
    }
    let version = r.tag_name.trim().trim_start_matches(['v', 'V']).to_string();
    if parse_version(&version).is_none() {
        return Err(format!("“{}” is not a version", r.tag_name));
    }
    let find = |pred: &dyn Fn(&str) -> bool| r.assets.iter().find(|a| pred(&a.name)).map(|a| Asset { name: a.name.clone(), url: a.browser_download_url.clone(), size: a.size });
    let setup = find(&|n| n.starts_with("OmniHub_") && n.ends_with("_x64-setup.exe"));
    let msi = find(&|n| n.starts_with("OmniHub_") && n.ends_with(".msi"));
    let sums_url = r.assets.iter().find(|a| a.name == "SHA256SUMS.txt").map(|a| a.browser_download_url.clone());
    Ok(Release { version, notes: r.body.unwrap_or_default(), page_url: r.html_url, published_at: r.published_at, setup, msi, sums_url })
}

/// "0.2.10" → [0, 2, 10]; anything after a '-' or '+' is ignored.
pub fn parse_version(v: &str) -> Option<Vec<u64>> {
    let core = v.trim().trim_start_matches(['v', 'V']).split(['-', '+']).next()?;
    let parts: Option<Vec<u64>> = core.split('.').map(|p| p.parse().ok()).collect();
    parts.filter(|p| !p.is_empty())
}

/// Whether `latest` is newer than `current`.
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(mut a), Some(mut b)) => {
            let n = a.len().max(b.len());
            a.resize(n, 0);
            b.resize(n, 0);
            a > b
        }
        _ => false,
    }
}

/// Where an install lives decides the installer: per-user installs (under
/// %LOCALAPPDATA%) use the setup .exe, Program Files installs the MSI.
pub fn per_user_install(exe: &Path) -> bool {
    match dirs::data_local_dir() {
        Some(local) => exe.starts_with(local),
        None => true,
    }
}

type Exit = Box<dyn Fn() + Send + Sync>;

pub struct Updater {
    events: EventBus,
    dir: PathBuf,
    current: String,
    api_url: Mutex<String>,
    state: Mutex<UpdateState>,
    busy: AtomicBool,
    /// Closes the app once the installer is running (set by the desktop shell).
    on_exit: Mutex<Option<Exit>>,
}

impl Updater {
    pub fn new(cache_dir: &Path, events: EventBus) -> Self {
        Updater {
            events,
            dir: cache_dir.join("updates"),
            current: env!("CARGO_PKG_VERSION").to_string(),
            api_url: Mutex::new(std::env::var("OMNIHUB_UPDATE_URL").unwrap_or_else(|_| LATEST_RELEASE_API.to_string())),
            state: Mutex::new(UpdateState::Idle),
            busy: AtomicBool::new(false),
            on_exit: Mutex::new(None),
        }
    }

    /// For tests: a local server instead of GitHub, and another "current" version.
    pub fn with_source(cache_dir: &Path, events: EventBus, api_url: &str, current: &str) -> Self {
        let mut u = Self::new(cache_dir, events);
        *u.api_url.get_mut() = api_url.to_string();
        u.current = current.to_string();
        u
    }

    pub fn current_version(&self) -> &str {
        &self.current
    }

    pub fn set_exit(&self, f: impl Fn() + Send + Sync + 'static) {
        *self.on_exit.lock() = Some(Box::new(f));
    }

    pub fn state(&self) -> UpdateState {
        self.state.lock().clone()
    }

    fn set(&self, s: UpdateState) {
        *self.state.lock() = s.clone();
        self.events.emit("update:state", &s);
    }

    /// Ask GitHub for the latest release.
    pub fn check(&self) -> UpdateState {
        if matches!(self.state(), UpdateState::Downloading { .. } | UpdateState::Installing { .. }) {
            return self.state();
        }
        self.set(UpdateState::Checking);
        let url = self.api_url.lock().clone();
        let r = fetch(&url).and_then(|b| parse_release(&b));
        let s = match r {
            Ok(rel) if is_newer(&rel.version, &self.current) => UpdateState::Available { release: rel },
            Ok(_) => UpdateState::UpToDate { checked_at: crate::db::now() },
            Err(e) => UpdateState::Failed { message: format!("Could not check for updates: {e}") },
        };
        self.set(s.clone());
        s
    }

    /// Download and verify the installer for `release`. Returns its path and
    /// whether it is an MSI.
    pub fn download(&self, release: &Release) -> Result<(PathBuf, bool), String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let (asset, msi) = match (per_user_install(&exe), &release.setup, &release.msi) {
            (true, Some(a), _) => (a, false),
            (false, _, Some(a)) => (a, true),
            (_, Some(a), _) => (a, false),
            _ => return Err("This release has no Windows installer.".into()),
        };
        // Only files from this project's own releases.
        if !asset.url.starts_with("https://github.com/Kerioxsx/omnihub/releases/download/") && !self.api_url.lock().starts_with("http://127.0.0.1") {
            return Err("The installer is not from OmniHub's releases.".into());
        }
        let sums_url = release.sums_url.as_deref().ok_or("This release has no SHA256SUMS.txt, so its installer can't be checked.")?;
        let sums = String::from_utf8(fetch(sums_url)?).map_err(|e| e.to_string())?;
        let expected = crate::capture::airplay::checksum_for(&sums, &asset.name).ok_or_else(|| format!("{} is not listed in SHA256SUMS.txt", asset.name))?;
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let dest = self.dir.join(&asset.name);
        let version = release.version.clone();
        let total = asset.size;
        let mut progress = |done: u64, t: u64| self.set(UpdateState::Downloading { version: version.clone(), done, total: if t > 0 { t } else { total } });
        let got = crate::capture::airplay::download(&asset.url, &dest, &mut progress).map_err(|e| e.to_string())?;
        if !got.eq_ignore_ascii_case(&expected) {
            let _ = std::fs::remove_file(&dest);
            return Err("The downloaded installer did not match its published checksum, so it was deleted.".into());
        }
        Ok((dest, msi))
    }

    /// Download, verify and run the installer, then close OmniHub.
    pub fn install(&self, release: &Release) -> Result<(), String> {
        if self.busy.swap(true, Ordering::SeqCst) {
            return Err("An update is already being installed.".into());
        }
        let r = self.download(release).and_then(|(path, msi)| {
            self.set(UpdateState::Installing { version: release.version.clone() });
            run_installer(&path, msi)
        });
        match r {
            Ok(()) => {
                if let Some(exit) = self.on_exit.lock().as_ref() {
                    // Give the installer a moment to start before closing.
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    exit();
                }
                Ok(())
            }
            Err(e) => {
                self.busy.store(false, Ordering::SeqCst);
                self.set(UpdateState::Failed { message: e.clone() });
                Err(e)
            }
        }
    }

    /// Remove installers left from earlier updates.
    pub fn clean(&self) {
        if let Ok(rd) = std::fs::read_dir(&self.dir) {
            for e in rd.flatten() {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
}

fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
    let agent: ureq::Agent = ureq::Agent::config_builder().tls_config(tls).timeout_global(Some(std::time::Duration::from_secs(30))).build().into();
    let mut res = agent
        .get(url)
        .header("User-Agent", concat!("OmniHub/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(404) => "no release found".to_string(),
            ureq::Error::StatusCode(403) | ureq::Error::StatusCode(429) => "GitHub is rate-limiting requests; try again later".to_string(),
            e => e.to_string(),
        })?;
    res.body_mut().with_config().limit(32 * 1024 * 1024).read_to_vec().map_err(|e| e.to_string())
}

#[cfg(windows)]
fn run_installer(path: &Path, msi: bool) -> Result<(), String> {
    let mut c = if msi {
        // Program Files installs need administrator approval (msiexec asks).
        let mut c = std::process::Command::new("msiexec");
        c.arg("/i").arg(path).arg("/passive");
        c
    } else {
        // Passive (a progress bar, no questions), close the running app,
        // and start the new version when done.
        let mut c = std::process::Command::new(path);
        c.args(["/P", "/R", "/UPDATE"]);
        c
    };
    c.spawn().map(|_| ()).map_err(|e| format!("Could not start the installer: {e}"))
}

#[cfg(not(windows))]
fn run_installer(_path: &Path, _msi: bool) -> Result<(), String> {
    Err("Updates install on Windows only.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "tag_name": "v0.3.0", "html_url": "https://github.com/Kerioxsx/omnihub/releases/tag/v0.3.0",
        "body": "New things", "published_at": "2026-10-05T10:00:00Z", "draft": false, "prerelease": false,
        "assets": [
            {"name": "OmniHub_0.3.0_x64-setup.exe", "browser_download_url": "https://github.com/Kerioxsx/omnihub/releases/download/v0.3.0/OmniHub_0.3.0_x64-setup.exe", "size": 9000000},
            {"name": "OmniHub_0.3.0_x64_en-US.msi", "browser_download_url": "https://github.com/Kerioxsx/omnihub/releases/download/v0.3.0/OmniHub_0.3.0_x64_en-US.msi", "size": 12000000},
            {"name": "SHA256SUMS.txt", "browser_download_url": "https://github.com/Kerioxsx/omnihub/releases/download/v0.3.0/SHA256SUMS.txt", "size": 300},
            {"name": "OmniHub-AirPlay-addon-x64.zip", "browser_download_url": "https://example/zip", "size": 1}
        ]
    }"#;

    #[test]
    fn reads_the_latest_release() {
        let r = parse_release(SAMPLE.as_bytes()).unwrap();
        assert_eq!(r.version, "0.3.0");
        assert_eq!(r.setup.as_ref().unwrap().name, "OmniHub_0.3.0_x64-setup.exe");
        assert_eq!(r.msi.as_ref().unwrap().size, 12_000_000);
        assert!(r.sums_url.unwrap().ends_with("/SHA256SUMS.txt"));
        assert!(parse_release(br#"{"tag_name":"v9.0.0","prerelease":true}"#).is_err());
        assert!(parse_release(br#"{"tag_name":"nightly"}"#).is_err());
        assert!(parse_release(b"<html>").is_err());
    }

    #[test]
    fn compares_versions() {
        assert!(is_newer("0.2.1", "0.2.0"));
        assert!(is_newer("v0.10.0", "0.9.9"));
        assert!(is_newer("1.0", "0.99.99"));
        assert!(!is_newer("0.2.0", "0.2.0"));
        assert!(!is_newer("0.1.9", "0.2.0"));
        assert!(!is_newer("0.2.0-beta", "0.2.0"));
        assert!(!is_newer("garbage", "0.2.0"));
    }

    /// A one-shot HTTP server for the "GitHub" answers.
    fn serve(responses: Vec<(&'static str, Vec<u8>)>) -> String {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://127.0.0.1:{}", l.local_addr().unwrap().port());
        std::thread::spawn(move || {
            use std::io::{Read, Write};
            for mut s in l.incoming().flatten() {
                let mut buf = [0u8; 4096];
                let n = s.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                let body = responses.iter().find(|(p, _)| *p == path).map(|(_, b)| b.clone());
                let (status, body) = match body {
                    Some(b) => ("200 OK", b),
                    None => ("404 Not Found", b"missing".to_vec()),
                };
                let _ = write!(s, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = s.write_all(&body);
            }
        });
        base
    }

    #[test]
    fn checks_downloads_and_verifies() {
        use sha2::{Digest, Sha256};
        let installer = b"MZ pretend installer".to_vec();
        let hash = hex::encode(Sha256::digest(&installer));
        // The release JSON points at the file server.
        let files = serve(vec![
            ("/OmniHub_0.3.0_x64-setup.exe", installer.clone()),
            ("/SHA256SUMS.txt", format!("{hash}  OmniHub_0.3.0_x64-setup.exe\n").into_bytes()),
            ("/bad/SHA256SUMS.txt", format!("{}  OmniHub_0.3.0_x64-setup.exe\n", "0".repeat(64)).into_bytes()),
        ]);
        let release = |sums: &str| {
            format!(r#"{{"tag_name":"v0.3.0","html_url":"x","assets":[{{"name":"OmniHub_0.3.0_x64-setup.exe","browser_download_url":"{files}/OmniHub_0.3.0_x64-setup.exe","size":{}}},{{"name":"SHA256SUMS.txt","browser_download_url":"{files}{sums}"}}]}}"#, installer.len())
        };
        let api = serve(vec![("/latest", release("/SHA256SUMS.txt").into_bytes()), ("/bad", release("/bad/SHA256SUMS.txt").into_bytes())]);
        let dir = tempfile::tempdir().unwrap();

        let u = Updater::with_source(dir.path(), EventBus::new(), &format!("{api}/latest"), "0.2.0");
        let UpdateState::Available { release: rel } = u.check() else { panic!("{:?}", u.state()) };
        assert_eq!(rel.version, "0.3.0");
        let (path, msi) = u.download(&rel).unwrap();
        assert!(!msi);
        assert_eq!(std::fs::read(&path).unwrap(), installer);

        // A checksum that doesn't match: refused and deleted.
        let bad = Updater::with_source(dir.path(), EventBus::new(), &format!("{api}/bad"), "0.2.0");
        let UpdateState::Available { release: rel } = bad.check() else { panic!() };
        std::fs::remove_file(&path).unwrap();
        assert!(bad.download(&rel).unwrap_err().contains("checksum"));
        assert!(!path.exists());

        // Already on the latest version.
        let same = Updater::with_source(dir.path(), EventBus::new(), &format!("{api}/latest"), "0.3.0");
        assert!(matches!(same.check(), UpdateState::UpToDate { .. }));
        // No server: a readable failure.
        let none = Updater::with_source(dir.path(), EventBus::new(), "http://127.0.0.1:9/latest", "0.2.0");
        assert!(matches!(none.check(), UpdateState::Failed { .. }));
    }
}
