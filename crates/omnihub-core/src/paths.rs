//! Where OmniHub keeps its files.

use std::io;
use std::path::{Path, PathBuf};

pub const APP_DIR_NAME: &str = "OmniHub";

/// `std::fs::canonicalize` on Windows returns verbatim paths (`\\?\C:\...`).
/// Turn them back into ordinary paths for display and comparison.
pub fn simplify(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        if rest.as_bytes().get(1) == Some(&b':') {
            return PathBuf::from(rest);
        }
    }
    p
}

/// Canonicalize and simplify.
pub fn canonical(p: &Path) -> io::Result<PathBuf> {
    std::fs::canonicalize(p).map(simplify)
}

#[derive(Debug, Clone)]
pub struct AppPaths {
    /// Settings and the paired-device database.
    pub config: PathBuf,
    /// Notes, vault, screenshots index.
    pub data: PathBuf,
    /// Scan snapshots, icon and thumbnail caches (safe to delete).
    pub cache: PathBuf,
    pub logs: PathBuf,
}

impl AppPaths {
    /// The per-user default locations (`%LOCALAPPDATA%\OmniHub\...` on Windows).
    pub fn default_for_user() -> io::Result<Self> {
        if let Some(dir) = std::env::var_os("OMNIHUB_HOME") {
            return Self::at(Path::new(&dir));
        }
        let base = dirs::data_local_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no local data directory"))?
            .join(APP_DIR_NAME);
        Self::at(&base)
    }

    /// Everything under one directory (used by tests and portable mode).
    pub fn at(base: &Path) -> io::Result<Self> {
        let p = AppPaths {
            config: base.join("config"),
            data: base.join("data"),
            cache: base.join("cache"),
            logs: base.join("logs"),
        };
        for d in [&p.config, &p.data, &p.cache, &p.logs, &p.scans()] {
            std::fs::create_dir_all(d)?;
        }
        Ok(p)
    }

    pub fn scans(&self) -> PathBuf {
        self.cache.join("scans")
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config.join("settings.json")
    }

    pub fn database(&self) -> PathBuf {
        self.data.join("omnihub.db")
    }

    pub fn vault_file(&self) -> PathBuf {
        self.data.join("vault.ohv")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbatim_prefixes_are_removed() {
        assert_eq!(simplify(PathBuf::from(r"\\?\C:\Users\me")), PathBuf::from(r"C:\Users\me"));
        assert_eq!(simplify(PathBuf::from(r"\\?\UNC\server\share\x")), PathBuf::from(r"\\server\share\x"));
        assert_eq!(simplify(PathBuf::from(r"\\?\Volume{abc}\x")), PathBuf::from(r"\\?\Volume{abc}\x"));
        assert_eq!(simplify(PathBuf::from("/home/me")), PathBuf::from("/home/me"));
    }
}
