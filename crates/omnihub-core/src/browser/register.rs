//! Tell the browsers where the native-messaging host is: a manifest naming
//! the OmniHub executable (see [`host_path`]) and allowing only the OmniHub
//! extension, registered per user (no administrator rights), for the
//! browsers the user picked.
//!
//! * Windows: `HKCU\Software\<browser>\NativeMessagingHosts\app.omnihub.vault`
//!   → path of the manifest. Opera, Opera GX and Vivaldi read Chrome's key;
//!   Firefox reads Mozilla's, with its own manifest (`allowed_extensions`).
//! * Linux: the manifest copied into each browser's `NativeMessagingHosts`
//!   folder under `~/.config` (Firefox: `~/.mozilla/native-messaging-hosts`).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{extension_origin, ipc, FIREFOX_ID, HOST_NAME};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Chromium,
    Firefox,
}

#[cfg_attr(not(windows), allow(dead_code))]
struct Browser {
    id: &'static str,
    name: &'static str,
    kind: Kind,
    /// The HKCU key it reads hosts from.
    key: &'static str,
    /// Its folder under `~/.config` on Linux ("" = none).
    linux: &'static str,
    /// Where its program usually is: (folder variable, path below it).
    exe: &'static [(&'static str, &'static str)],
    /// Its name under Windows' "App Paths", when it registers one.
    app_path: &'static str,
    /// Its extensions page.
    page: &'static str,
}

const CHROME_KEY: &str = r"Software\Google\Chrome\NativeMessagingHosts";
const PF: &str = "ProgramFiles";
const PF86: &str = "ProgramFiles(x86)";
const LAD: &str = "LOCALAPPDATA";

const BROWSERS: &[Browser] = &[
    Browser { id: "chrome", name: "Chrome", kind: Kind::Chromium, key: CHROME_KEY, linux: "google-chrome", exe: &[(PF, r"Google\Chrome\Application\chrome.exe"), (PF86, r"Google\Chrome\Application\chrome.exe"), (LAD, r"Google\Chrome\Application\chrome.exe")], app_path: "chrome.exe", page: "chrome://extensions" },
    Browser { id: "edge", name: "Edge", kind: Kind::Chromium, key: r"Software\Microsoft\Edge\NativeMessagingHosts", linux: "microsoft-edge", exe: &[(PF86, r"Microsoft\Edge\Application\msedge.exe"), (PF, r"Microsoft\Edge\Application\msedge.exe")], app_path: "msedge.exe", page: "edge://extensions" },
    Browser { id: "brave", name: "Brave", kind: Kind::Chromium, key: r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts", linux: "BraveSoftware/Brave-Browser", exe: &[(PF, r"BraveSoftware\Brave-Browser\Application\brave.exe"), (LAD, r"BraveSoftware\Brave-Browser\Application\brave.exe")], app_path: "brave.exe", page: "brave://extensions" },
    Browser { id: "opera", name: "Opera", kind: Kind::Chromium, key: CHROME_KEY, linux: "opera", exe: &[(LAD, r"Programs\Opera\opera.exe"), (PF, r"Opera\opera.exe")], app_path: "", page: "opera://extensions" },
    Browser { id: "opera-gx", name: "Opera GX", kind: Kind::Chromium, key: CHROME_KEY, linux: "", exe: &[(LAD, r"Programs\Opera GX\opera.exe"), (PF, r"Opera GX\opera.exe")], app_path: "", page: "opera://extensions" },
    Browser { id: "vivaldi", name: "Vivaldi", kind: Kind::Chromium, key: CHROME_KEY, linux: "vivaldi", exe: &[(LAD, r"Vivaldi\Application\vivaldi.exe"), (PF, r"Vivaldi\Application\vivaldi.exe")], app_path: "vivaldi.exe", page: "vivaldi://extensions" },
    Browser { id: "chromium", name: "Chromium", kind: Kind::Chromium, key: r"Software\Chromium\NativeMessagingHosts", linux: "chromium", exe: &[(LAD, r"Chromium\Application\chrome.exe")], app_path: "", page: "chrome://extensions" },
    Browser { id: "firefox", name: "Firefox", kind: Kind::Firefox, key: r"Software\Mozilla\NativeMessagingHosts", linux: "", exe: &[(PF, r"Mozilla Firefox\firefox.exe"), (PF86, r"Mozilla Firefox\firefox.exe"), (LAD, r"Mozilla Firefox\firefox.exe"), (PF, r"Firefox Developer Edition\firefox.exe"), (PF, r"Firefox Nightly\firefox.exe")], app_path: "firefox.exe", page: "about:debugging#/runtime/this-firefox" },
];

/// The browser's program, if it is installed.
fn exe_of(b: &Browser) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
        if !b.app_path.is_empty() {
            for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
                let found = winreg::RegKey::predef(root)
                    .open_subkey(format!(r"Software\Microsoft\Windows\CurrentVersion\App Paths\{}", b.app_path))
                    .and_then(|k| k.get_value::<String, _>(""))
                    .map(|p| PathBuf::from(p.trim_matches('"')));
                if let Ok(p) = found {
                    // Opera's and Firefox's "App Paths" can name another edition: only the expected one counts.
                    if p.is_file() && b.exe.iter().any(|(_, rel)| p.ends_with(rel)) {
                        return Some(p);
                    }
                }
            }
        }
        b.exe.iter().filter_map(|(var, rel)| std::env::var_os(var).map(|base| PathBuf::from(base).join(rel))).find(|p| p.is_file())
    }
    #[cfg(not(windows))]
    {
        let _ = b;
        None
    }
}

fn installed(b: &Browser) -> bool {
    if cfg!(windows) {
        exe_of(b).is_some()
    } else {
        linux_dir(b).is_some_and(|d| d.parent().is_some_and(Path::is_dir))
    }
}

/// Where the browser looks for host manifests on Linux.
fn linux_dir(b: &Browser) -> Option<PathBuf> {
    match b.kind {
        Kind::Firefox => dirs::home_dir().map(|h| h.join(".mozilla").join("native-messaging-hosts")),
        Kind::Chromium if !b.linux.is_empty() => dirs::config_dir().map(|c| c.join(b.linux).join("NativeMessagingHosts")),
        Kind::Chromium => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Registration {
    pub id: String,
    pub browser: String,
    pub kind: Kind,
    pub installed: bool,
    pub registered: bool,
    /// Its extensions page (to type in the address bar).
    pub page: String,
}

/// File name start of the host copies in `data/browser`.
pub const HOST_COPY_PREFIX: &str = "omnihub-host-";
/// Next to the copies: the OmniHub executable a copy starts on `launch-app`.
pub const APP_PATH_FILE: &str = "app-path.txt";

/// What the browser should start: the stand-alone host when it sits next
/// to the running executable; on Windows otherwise a copy of the OmniHub
/// executable in the data folder (see [`copy_host`]); elsewhere the
/// executable itself. OmniHub relays when started with the extension's
/// origin.
pub fn host_path(data_dir: &Path) -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let standalone = exe.parent()?.join(ipc::host_file_name());
    if standalone.is_file() {
        return Some(standalone);
    }
    if cfg!(windows) {
        match copy_host(&exe, &data_dir.join("browser")) {
            Ok(copy) => return Some(copy),
            Err(e) => tracing::warn!("could not copy the browser host: {e}"),
        }
    }
    Some(exe)
}

/// A browser keeps its host running while the extension is in use. Were
/// that the installed omnihub.exe, Windows would keep it locked and setups
/// fail with "Error writing to file … omnihub.exe" — so browsers start a
/// copy, `omnihub-host-<version>.exe` in `dir`, made once per version.
/// Copies of other versions are removed once no browser runs them.
pub fn copy_host(exe: &Path, dir: &Path) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let name = format!("{HOST_COPY_PREFIX}{}{}", env!("CARGO_PKG_VERSION"), std::env::consts::EXE_SUFFIX);
    let copy = dir.join(&name);
    let len = |p: &Path| std::fs::metadata(p).map(|m| m.len()).ok();
    if len(&copy).is_none() || len(&copy) != len(exe) {
        let tmp = dir.join(format!("{name}.tmp"));
        let r = std::fs::copy(exe, &tmp).and_then(|_| std::fs::rename(&tmp, &copy));
        if let Err(e) = r {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
    }
    crate::settings::write_atomic(&dir.join(APP_PATH_FILE), exe.to_string_lossy().as_bytes())?;
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.starts_with(HOST_COPY_PREFIX) && n != name {
                // Still running in a browser: removed another time.
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    Ok(copy)
}

/// The OmniHub executable for a host copy made by [`copy_host`].
pub fn app_for_copy(host: &Path) -> Option<PathBuf> {
    let name = host.file_name()?.to_string_lossy();
    if !name.starts_with(HOST_COPY_PREFIX) {
        return None;
    }
    let app = std::fs::read_to_string(host.parent()?.join(APP_PATH_FILE)).ok()?;
    Some(PathBuf::from(app.trim()))
}

pub fn manifest(host: &Path) -> serde_json::Value {
    serde_json::json!({
        "name": HOST_NAME,
        "description": "OmniHub vault for the OmniHub browser extension",
        "path": host.to_string_lossy(),
        "type": "stdio",
        "allowed_origins": [extension_origin()],
    })
}

/// Firefox's manifest names the extension by its ID instead.
pub fn firefox_manifest(host: &Path) -> serde_json::Value {
    serde_json::json!({
        "name": HOST_NAME,
        "description": "OmniHub vault for the OmniHub browser extension",
        "path": host.to_string_lossy(),
        "type": "stdio",
        "allowed_extensions": [FIREFOX_ID],
    })
}

fn manifest_file(dir: &Path, kind: Kind) -> PathBuf {
    match kind {
        Kind::Chromium => dir.join(format!("{HOST_NAME}.json")),
        Kind::Firefox => dir.join(format!("{HOST_NAME}.firefox.json")),
    }
}

/// The browsers to set up: `ids`, or every installed one when empty.
fn chosen(ids: &[String]) -> Vec<&'static Browser> {
    BROWSERS.iter().filter(|b| if ids.is_empty() { installed(b) } else { ids.iter().any(|i| i == b.id) }).collect()
}

/// Write the manifests into `data_dir` and register them with the browsers
/// in `ids` (every installed one when empty); browsers left out are
/// unregistered unless one that is chosen shares their key.
pub fn register(data_dir: &Path, ids: &[String]) -> std::io::Result<Vec<Registration>> {
    let host = host_path(data_dir).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "cannot find the OmniHub executable"))?;
    let dir = data_dir.join("browser");
    std::fs::create_dir_all(&dir)?;
    crate::settings::write_atomic(&manifest_file(&dir, Kind::Chromium), &serde_json::to_vec_pretty(&manifest(&host))?)?;
    crate::settings::write_atomic(&manifest_file(&dir, Kind::Firefox), &serde_json::to_vec_pretty(&firefox_manifest(&host))?)?;
    let picked = chosen(ids);
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
        for b in BROWSERS {
            let path = format!(r"{}\{HOST_NAME}", b.key);
            if picked.iter().any(|p| p.key == b.key) {
                let (k, _) = hkcu.create_subkey(&path)?;
                k.set_value("", &manifest_file(&dir, b.kind).to_string_lossy().to_string())?;
            } else {
                let _ = hkcu.delete_subkey_all(&path);
            }
        }
    }
    #[cfg(unix)]
    {
        for b in BROWSERS {
            let Some(target) = linux_dir(b) else { continue };
            if picked.iter().any(|p| p.id == b.id) && target.parent().is_some_and(Path::is_dir) {
                std::fs::create_dir_all(&target)?;
                std::fs::copy(manifest_file(&dir, b.kind), manifest_file(&target, Kind::Chromium))?;
            } else if !picked.iter().any(|p| linux_dir(p) == Some(target.clone())) {
                let _ = std::fs::remove_file(manifest_file(&target, Kind::Chromium));
            }
        }
    }
    Ok(status())
}

pub fn unregister() {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
        for b in BROWSERS {
            let _ = hkcu.delete_subkey_all(format!(r"{}\{HOST_NAME}", b.key));
        }
    }
    #[cfg(unix)]
    for b in BROWSERS {
        if let Some(d) = linux_dir(b) {
            let _ = std::fs::remove_file(manifest_file(&d, Kind::Chromium));
        }
    }
}

/// Every browser OmniHub knows, whether it is installed and set up.
pub fn status() -> Vec<Registration> {
    BROWSERS
        .iter()
        .map(|b| {
            #[cfg(windows)]
            let registered = {
                use winreg::enums::HKEY_CURRENT_USER;
                winreg::RegKey::predef(HKEY_CURRENT_USER)
                    .open_subkey(format!(r"{}\{HOST_NAME}", b.key))
                    .and_then(|k| k.get_value::<String, _>(""))
                    .is_ok_and(|p| Path::new(&p).is_file() && Path::new(&p).file_name() == manifest_file(Path::new(""), b.kind).file_name())
            };
            #[cfg(unix)]
            let registered = linux_dir(b).is_some_and(|d| manifest_file(&d, Kind::Chromium).is_file());
            Registration { id: b.id.into(), browser: b.name.into(), kind: b.kind, installed: installed(b), registered, page: b.page.into() }
        })
        .collect()
}

/// Open a browser's extensions page (where its extension is added).
pub fn open_page(id: &str) -> std::io::Result<()> {
    let b = BROWSERS.iter().find(|b| b.id == id).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "unknown browser"))?;
    let exe = exe_of(b).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, format!("{} isn't installed", b.name)))?;
    std::process::Command::new(exe).arg(b.page).spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browsers_run_a_copy_of_the_app() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("app").join("omnihub.exe");
        std::fs::create_dir_all(app.parent().unwrap()).unwrap();
        std::fs::write(&app, b"version one").unwrap();
        let hosts = dir.path().join("data").join("browser");
        std::fs::create_dir_all(&hosts).unwrap();
        // A copy left by an older version.
        let old = hosts.join(format!("{HOST_COPY_PREFIX}0.0.1{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(&old, b"old").unwrap();

        let copy = copy_host(&app, &hosts).unwrap();
        assert_ne!(copy, app);
        assert_eq!(std::fs::read(&copy).unwrap(), b"version one");
        assert_eq!(app_for_copy(&copy), Some(app.clone()));
        assert!(!old.exists(), "older copies are removed");
        assert_eq!(app_for_copy(&app), None, "the app itself is no copy");

        // A rebuilt app of the same version replaces the copy; an unchanged one is left alone.
        std::fs::write(&app, b"version one, rebuilt").unwrap();
        assert_eq!(std::fs::read(copy_host(&app, &hosts).unwrap()).unwrap(), b"version one, rebuilt");
        let leftovers: Vec<_> = std::fs::read_dir(&hosts).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(leftovers.len(), 2, "the copy and app-path.txt: {leftovers:?}");
    }

    #[test]
    fn firefox_gets_its_own_manifest() {
        let m = firefox_manifest(Path::new("C:/OmniHub/omnihub.exe"));
        assert_eq!(m["allowed_extensions"], serde_json::json!([FIREFOX_ID]));
        assert!(m.get("allowed_origins").is_none());
        assert_ne!(manifest_file(Path::new("d"), Kind::Firefox), manifest_file(Path::new("d"), Kind::Chromium));
        // Every browser has a unique id and an extensions page.
        let mut ids: Vec<_> = BROWSERS.iter().map(|b| b.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), BROWSERS.len());
        assert!(BROWSERS.iter().all(|b| !b.page.is_empty() && !b.exe.is_empty()));
    }

    #[test]
    fn manifest_allows_only_our_extension() {
        let m = manifest(Path::new("/opt/omnihub/omnihub-browser-host"));
        assert_eq!(m["name"], HOST_NAME);
        assert_eq!(m["type"], "stdio");
        assert_eq!(m["allowed_origins"], serde_json::json!(["chrome-extension://hfkbdbcemgoondnmkeeoclpcmcjjbdeg/"]));
        // Host names may only use lowercase letters, digits, dots and underscores.
        assert!(HOST_NAME.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_'));
    }
}
