//! Programs with a window open on the PC, for closing them from the phone:
//! "Close" asks politely, like clicking the window's × (the app can still
//! ask to save, and some — Discord, Steam, Spotify — only hide in the
//! tray); "Quit" ends the program (see `procs::ProcessMonitor::end`).

use std::sync::LazyLock;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::media::mixer::name_of;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpenApp {
    /// Lower-case executable name ("discord.exe").
    pub key: String,
    /// Executable file name as Windows spells it ("Discord.exe"), for "Quit".
    pub exe_name: String,
    pub name: String,
    /// Window titles, most recently used first.
    pub titles: Vec<String>,
    /// Full path of the executable, when readable (for its icon).
    #[serde(skip)]
    pub path: Option<String>,
    /// Whether "Quit" is offered (never for parts of Windows).
    pub can_quit: bool,
}

/// Executables whose windows are part of Windows itself.
const SHELL: &[&str] = &["explorer.exe", "applicationframehost.exe", "shellexperiencehost.exe", "startmenuexperiencehost.exe", "searchhost.exe", "textinputhost.exe", "systemsettings.exe", "lockapp.exe"];

/// Group windows (key, exe name, title, path) by program, keeping the
/// first-seen order (Windows lists the most recently used first).
pub fn group(windows: Vec<(String, String, String, Option<String>)>) -> Vec<OpenApp> {
    let mut out: Vec<OpenApp> = Vec::new();
    for (key, exe_name, title, path) in windows {
        if let Some(a) = out.iter_mut().find(|a| a.key == key) {
            if !a.titles.contains(&title) {
                a.titles.push(title);
            }
        } else {
            let can_quit = !super::procs::is_protected(&exe_name) && !SHELL.contains(&key.as_str());
            out.push(OpenApp { name: name_of(&key), key, exe_name, titles: vec![title], path, can_quit });
        }
    }
    out
}

fn is_fake(fake: bool) -> bool {
    fake || std::env::var("OMNIHUB_FAKE_MEDIA").is_ok_and(|v| v == "1")
}

pub fn list(fake: bool) -> Vec<OpenApp> {
    if is_fake(fake) {
        return FAKE.lock().clone();
    }
    #[cfg(windows)]
    {
        group(win::windows())
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Ask every window of the program to close. Returns how many were asked.
pub fn close(fake: bool, key: &str) -> Result<usize, String> {
    let key = key.to_ascii_lowercase();
    if is_fake(fake) {
        let mut l = FAKE.lock();
        let before = l.len();
        l.retain(|a| a.key != key);
        return if l.len() < before { Ok(1) } else { Err(format!("{} has no open window", name_of(&key))) };
    }
    #[cfg(windows)]
    {
        if key == "omnihub.exe" {
            return Err("Close OmniHub on the PC itself.".into());
        }
        match win::close(&key) {
            0 => Err(format!("{} has no open window", name_of(&key))),
            n => Ok(n),
        }
    }
    #[cfg(not(windows))]
    {
        Err("Closing apps is a Windows feature.".into())
    }
}

/// The executable path of an open program (for its icon).
pub fn path_of(fake: bool, key: &str) -> Option<String> {
    list(fake).into_iter().find(|a| a.key == key).and_then(|a| a.path)
}

static FAKE: LazyLock<Mutex<Vec<OpenApp>>> = LazyLock::new(|| Mutex::new(fake_list()));

fn fake_list() -> Vec<OpenApp> {
    group(vec![
        ("fortniteclient-win64-shipping.exe".into(), "FortniteClient-Win64-Shipping.exe".into(), "Fortnite".into(), None),
        ("discord.exe".into(), "Discord.exe".into(), "#general | Game Night - Discord".into(), None),
        ("spotify.exe".into(), "Spotify.exe".into(), "Midnight Atlas - Neon Afterglow".into(), None),
        ("brave.exe".into(), "brave.exe".into(), "YouTube - Brave".into(), None),
        ("brave.exe".into(), "brave.exe".into(), "Inbox - Brave".into(), None),
        ("nyxen.exe".into(), "Nyxen.exe".into(), "Nyxen".into(), None),
        ("code.exe".into(), "Code.exe".into(), "omnihub - Visual Studio Code".into(), None),
        ("explorer.exe".into(), "explorer.exe".into(), "Downloads".into(), None),
    ])
}

/// Put the pretend list back (tests).
pub fn reset_fake() {
    *FAKE.lock() = fake_list();
}

#[cfg(windows)]
mod win {
    use windows::core::{BOOL, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
    use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, EnumWindows, GetClassNameW, GetWindow, GetWindowLongW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, GWL_EXSTYLE, GW_OWNER, WM_CLOSE, WS_EX_TOOLWINDOW};

    fn exe_of(pid: u32) -> Option<String> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
            let _ = CloseHandle(h);
            ok.then(|| String::from_utf16_lossy(&buf[..len as usize]))
        }
    }

    fn pid_of(hwnd: HWND) -> u32 {
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        pid
    }

    fn class_of(hwnd: HWND) -> String {
        let mut buf = [0u16; 128];
        let n = unsafe { GetClassNameW(hwnd, &mut buf) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    /// A window the taskbar would show: visible, titled, not owned, not a
    /// tool window, not cloaked (hidden Store apps, other desktops).
    fn is_app_window(hwnd: HWND) -> Option<String> {
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() || GetWindow(hwnd, GW_OWNER).is_ok_and(|o| !o.is_invalid()) {
                return None;
            }
            if GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0 {
                return None;
            }
            let mut cloaked = 0u32;
            if DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut u32 as *mut _, 4).is_ok() && cloaked != 0 {
                return None;
            }
            if matches!(class_of(hwnd).as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
                return None;
            }
            let len = GetWindowTextLengthW(hwnd);
            if len <= 0 {
                return None;
            }
            let mut buf = vec![0u16; len as usize + 1];
            let n = GetWindowTextW(hwnd, &mut buf);
            let title = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
            (!title.trim().is_empty()).then_some(title)
        }
    }

    /// Store apps draw inside ApplicationFrameHost; the app is the child
    /// window from another process.
    fn store_app_pid(frame: HWND, host: u32) -> Option<u32> {
        struct Find {
            host: u32,
            found: Option<u32>,
        }
        unsafe extern "system" fn cb(hwnd: HWND, l: LPARAM) -> BOOL {
            let f = &mut *(l.0 as *mut Find);
            let pid = pid_of(hwnd);
            if pid != f.host && pid != 0 {
                f.found = Some(pid);
                return BOOL(0);
            }
            BOOL(1)
        }
        let mut f = Find { host, found: None };
        unsafe {
            let _ = EnumChildWindows(Some(frame), Some(cb), LPARAM(&mut f as *mut Find as isize));
        }
        f.found
    }

    fn top_windows() -> Vec<HWND> {
        unsafe extern "system" fn cb(hwnd: HWND, l: LPARAM) -> BOOL {
            (*(l.0 as *mut Vec<HWND>)).push(hwnd);
            BOOL(1)
        }
        let mut v: Vec<HWND> = Vec::new();
        unsafe {
            let _ = EnumWindows(Some(cb), LPARAM(&mut v as *mut Vec<HWND> as isize));
        }
        v
    }

    /// (key, exe name, path) of the program that owns an app window.
    fn owner(hwnd: HWND) -> Option<(String, String, String)> {
        let pid = pid_of(hwnd);
        if pid == std::process::id() {
            return None;
        }
        let mut path = exe_of(pid)?;
        if crate::media::mixer::key_of(&path) == "applicationframehost.exe" {
            if let Some(p) = store_app_pid(hwnd, pid).and_then(exe_of) {
                path = p;
            }
        }
        let exe_name = path.rsplit('\\').next().unwrap_or(&path).to_string();
        Some((crate::media::mixer::key_of(&path), exe_name, path))
    }

    pub fn windows() -> Vec<(String, String, String, Option<String>)> {
        top_windows()
            .into_iter()
            .filter_map(|h| {
                let title = is_app_window(h)?;
                let (key, exe, path) = owner(h)?;
                Some((key, exe, title, Some(path)))
            })
            .collect()
    }

    pub fn close(key: &str) -> usize {
        let mut n = 0;
        for h in top_windows() {
            if is_app_window(h).is_some() && owner(h).is_some_and(|(k, _, _)| k == key) && unsafe { PostMessageW(Some(h), WM_CLOSE, WPARAM(0), LPARAM(0)) }.is_ok() {
                n += 1;
            }
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouping_and_protection() {
        let g = group(vec![
            ("brave.exe".into(), "brave.exe".into(), "YouTube - Brave".into(), None),
            ("discord.exe".into(), "Discord.exe".into(), "Discord".into(), None),
            ("brave.exe".into(), "brave.exe".into(), "Inbox - Brave".into(), None),
            ("brave.exe".into(), "brave.exe".into(), "Inbox - Brave".into(), None),
            ("explorer.exe".into(), "explorer.exe".into(), "Downloads".into(), None),
        ]);
        assert_eq!(g.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), ["Brave", "Discord", "Explorer"]);
        assert_eq!(g[0].titles, ["YouTube - Brave", "Inbox - Brave"]);
        assert!(g[1].can_quit);
        assert!(!g[2].can_quit, "Explorer windows can be closed, but Explorer is never ended");
    }

    #[test]
    fn pretend_close() {
        reset_fake();
        assert!(list(true).iter().any(|a| a.key == "spotify.exe"));
        assert_eq!(close(true, "Spotify.exe"), Ok(1));
        assert!(!list(true).iter().any(|a| a.key == "spotify.exe"));
        assert!(close(true, "spotify.exe").is_err());
        reset_fake();
    }
}
