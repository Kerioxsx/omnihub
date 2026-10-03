//! Opening files, folders and URLs with the user's default handlers.

use std::path::Path;

#[cfg(windows)]
pub fn shell_execute(file: &str, params: &str) -> std::io::Result<()> {
    use windows::core::HSTRING;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let r = unsafe { ShellExecuteW(None, &HSTRING::from("open"), &HSTRING::from(file), &HSTRING::from(params), None, SW_SHOWNORMAL) };
    // ShellExecute returns a value > 32 on success.
    if r.0 as isize > 32 {
        Ok(())
    } else {
        Err(std::io::Error::other(format!("could not open {file} (error {})", r.0 as isize)))
    }
}

#[cfg(not(windows))]
pub fn shell_execute(file: &str, params: &str) -> std::io::Result<()> {
    let mut c = std::process::Command::new(file);
    if !params.is_empty() {
        c.args(params.split_whitespace());
    }
    c.spawn().map(|_| ())
}

/// Open a file or folder with its default application.
pub fn open_path(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        return Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("{} does not exist", path.display())));
    }
    #[cfg(windows)]
    return shell_execute(&path.to_string_lossy(), "");
    #[cfg(not(windows))]
    std::process::Command::new("xdg-open").arg(path).spawn().map(|_| ())
}

/// Show a file selected in Explorer.
pub fn reveal(path: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // explorer's /select needs the raw, quoted argument.
        std::process::Command::new("explorer.exe").raw_arg(format!("/select,\"{}\"", path.display())).spawn().map(|_| ())
    }
    #[cfg(not(windows))]
    {
        let dir = if path.is_dir() { path } else { path.parent().unwrap_or(path) };
        std::process::Command::new("xdg-open").arg(dir).spawn().map(|_| ())
    }
}

/// Open an http(s) or ms-settings: URL.
pub fn open_url(url: &str) -> std::io::Result<()> {
    let ok = url.starts_with("https://") || url.starts_with("http://") || url.starts_with("ms-settings:");
    if !ok {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "only web and settings links can be opened"));
    }
    #[cfg(windows)]
    return shell_execute(url, "");
    #[cfg(not(windows))]
    std::process::Command::new("xdg-open").arg(url).spawn().map(|_| ())
}

/// Well-known user folders (Desktop, Documents, Downloads, ...).
pub fn user_folders() -> Vec<(String, std::path::PathBuf)> {
    [
        ("Desktop", dirs::desktop_dir()),
        ("Documents", dirs::document_dir()),
        ("Downloads", dirs::download_dir()),
        ("Pictures", dirs::picture_dir()),
        ("Videos", dirs::video_dir()),
        ("Music", dirs::audio_dir()),
    ]
    .into_iter()
    .filter_map(|(n, p)| p.filter(|p| p.is_dir()).map(|p| (n.to_string(), p)))
    .collect()
}
