//! Tell Brave, Chrome, Edge and Chromium where the native-messaging host
//! is: a manifest naming the OmniHub executable (see [`host_path`]) and
//! allowing only the OmniHub extension, registered per user (no
//! administrator rights).
//!
//! * Windows: `HKCU\Software\<browser>\NativeMessagingHosts\app.omnihub.vault`
//!   → path of the manifest.
//! * Linux: the manifest copied into each browser's `NativeMessagingHosts`
//!   folder under `~/.config`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{extension_origin, ipc, HOST_NAME};

/// Browsers we register with: display name, Windows registry path, Linux config dir.
const BROWSERS: &[(&str, &str, &str)] = &[
    ("Brave", r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts", "BraveSoftware/Brave-Browser"),
    ("Chrome", r"Software\Google\Chrome\NativeMessagingHosts", "google-chrome"),
    ("Edge", r"Software\Microsoft\Edge\NativeMessagingHosts", "microsoft-edge"),
    ("Chromium", r"Software\Chromium\NativeMessagingHosts", "chromium"),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Registration {
    pub browser: String,
    pub registered: bool,
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

fn manifest_file(dir: &Path) -> PathBuf {
    dir.join(format!("{HOST_NAME}.json"))
}

/// Write the manifest into `data_dir` and register it with every browser.
pub fn register(data_dir: &Path) -> std::io::Result<Vec<Registration>> {
    let host = host_path(data_dir).ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "cannot find the OmniHub executable"))?;
    let dir = data_dir.join("browser");
    std::fs::create_dir_all(&dir)?;
    let file = manifest_file(&dir);
    crate::settings::write_atomic(&file, &serde_json::to_vec_pretty(&manifest(&host))?)?;
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
        for (_, key, _) in BROWSERS {
            let (k, _) = hkcu.create_subkey(format!(r"{key}\{HOST_NAME}"))?;
            k.set_value("", &file.to_string_lossy().to_string())?;
        }
    }
    #[cfg(unix)]
    {
        if let Some(config) = dirs::config_dir() {
            for (_, _, sub) in BROWSERS {
                let browser_dir = config.join(sub);
                // Only for browsers that are installed (have a profile folder).
                if browser_dir.is_dir() {
                    let target = browser_dir.join("NativeMessagingHosts");
                    std::fs::create_dir_all(&target)?;
                    std::fs::copy(&file, manifest_file(&target))?;
                }
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
        for (_, key, _) in BROWSERS {
            let _ = hkcu.delete_subkey_all(format!(r"{key}\{HOST_NAME}"));
        }
    }
    #[cfg(unix)]
    if let Some(config) = dirs::config_dir() {
        for (_, _, sub) in BROWSERS {
            let _ = std::fs::remove_file(manifest_file(&config.join(sub).join("NativeMessagingHosts")));
        }
    }
}

/// Which browsers currently point at a manifest.
pub fn status() -> Vec<Registration> {
    BROWSERS
        .iter()
        .map(|(name, _key, _sub)| {
            #[cfg(windows)]
            let registered = {
                use winreg::enums::HKEY_CURRENT_USER;
                winreg::RegKey::predef(HKEY_CURRENT_USER)
                    .open_subkey(format!(r"{_key}\{HOST_NAME}"))
                    .and_then(|k| k.get_value::<String, _>(""))
                    .is_ok_and(|p| Path::new(&p).is_file())
            };
            #[cfg(unix)]
            let registered = dirs::config_dir().is_some_and(|c| manifest_file(&c.join(_sub).join("NativeMessagingHosts")).is_file());
            Registration { browser: name.to_string(), registered }
        })
        .collect()
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
    fn manifest_allows_only_our_extension() {
        let m = manifest(Path::new("/opt/omnihub/omnihub-browser-host"));
        assert_eq!(m["name"], HOST_NAME);
        assert_eq!(m["type"], "stdio");
        assert_eq!(m["allowed_origins"], serde_json::json!(["chrome-extension://hfkbdbcemgoondnmkeeoclpcmcjjbdeg/"]));
        // Host names may only use lowercase letters, digits, dots and underscores.
        assert!(HOST_NAME.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_'));
    }
}
