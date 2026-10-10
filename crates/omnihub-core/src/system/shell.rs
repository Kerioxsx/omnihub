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
    // Windows Security's Core isolation page (Memory Integrity) too.
    let ok = url.starts_with("https://") || url.starts_with("http://") || url.starts_with("ms-settings:") || url == "windowsdefender://coreisolation";
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

/// File name of the Explorer "Send to" shortcut.
pub const SEND_TO_LINK: &str = "OmniHub (phone).lnk";
/// Argument the shortcut passes; Explorer appends the selected paths.
pub const SEND_TO_ARG: &str = "--send-to-phone";

/// Add or remove "Send to → OmniHub (phone)" in Explorer's context menu.
/// Recreated at every start so it follows the installed executable.
#[cfg(windows)]
pub fn set_send_to_shortcut(enabled: bool) -> std::io::Result<()> {
    let Some(dir) = dirs::config_dir().map(|d| d.join("Microsoft").join("Windows").join("SendTo")) else { return Ok(()) };
    let link = dir.join(SEND_TO_LINK);
    if !enabled {
        if link.exists() {
            std::fs::remove_file(&link)?;
        }
        return Ok(());
    }
    if !dir.is_dir() {
        return Ok(());
    }
    let exe = std::env::current_exe()?;
    std::thread::spawn(move || create_shortcut(&link, &exe, SEND_TO_ARG, "Send to your phone with OmniHub")).join().map_err(|_| std::io::Error::other("shortcut thread panicked"))?
}

#[cfg(not(windows))]
pub fn set_send_to_shortcut(_enabled: bool) -> std::io::Result<()> {
    Ok(())
}

#[cfg(windows)]
fn create_shortcut(link: &Path, target: &Path, args: &str, description: &str) -> std::io::Result<()> {
    use windows::core::{Interface, HSTRING};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};
    let io = |e: windows::core::Error| std::io::Error::other(e.message());
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let res = (|| {
            let sl: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).map_err(io)?;
            sl.SetPath(&HSTRING::from(target.as_os_str())).map_err(io)?;
            sl.SetArguments(&HSTRING::from(args)).map_err(io)?;
            sl.SetDescription(&HSTRING::from(description)).map_err(io)?;
            sl.SetIconLocation(&HSTRING::from(target.as_os_str()), 0).map_err(io)?;
            let pf: IPersistFile = sl.cast().map_err(io)?;
            pf.Save(&HSTRING::from(link.as_os_str()), true).map_err(io)
        })();
        CoUninitialize();
        res
    }
}

/// Paths passed after `--send-to-phone` on a command line.
pub fn send_to_paths(argv: &[String]) -> Option<Vec<String>> {
    let pos = argv.iter().position(|a| a == SEND_TO_ARG)?;
    Some(argv[pos + 1..].iter().filter(|a| !a.starts_with("--")).cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_to_arguments() {
        let a = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(send_to_paths(&a(&["omnihub.exe"])), None);
        assert_eq!(send_to_paths(&a(&["omnihub.exe", SEND_TO_ARG, r"C:\a b.jpg", "--minimized", r"D:\x"])), Some(a(&[r"C:\a b.jpg", r"D:\x"])));
    }
}
