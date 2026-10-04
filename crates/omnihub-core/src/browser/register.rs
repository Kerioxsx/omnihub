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

/// What the browser should start: the stand-alone host when it sits next
/// to the running executable, otherwise the OmniHub executable itself
/// (which relays when started with the extension's origin).
pub fn host_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let standalone = exe.parent()?.join(ipc::host_file_name());
    Some(if standalone.is_file() { standalone } else { exe })
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
    let host = host_path().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "cannot find the OmniHub executable"))?;
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
    fn manifest_allows_only_our_extension() {
        let m = manifest(Path::new("/opt/omnihub/omnihub-browser-host"));
        assert_eq!(m["name"], HOST_NAME);
        assert_eq!(m["type"], "stdio");
        assert_eq!(m["allowed_origins"], serde_json::json!(["chrome-extension://hfkbdbcemgoondnmkeeoclpcmcjjbdeg/"]));
        // Host names may only use lowercase letters, digits, dots and underscores.
        assert!(HOST_NAME.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_'));
    }
}
