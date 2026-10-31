//! Background programs a boost may close or slow down while a game runs.
//!
//! Only programs nobody needs mid-game are closed: cloud sync, Windows
//! extras and updaters. Chat and voice apps (Discord, Teams), music players
//! and browsers stay open; browsers, launchers' web views and sync apps
//! only drop to "Below normal" priority, so the game gets the processor
//! first. Everything goes back when the game closes.

/// A program closed by "Close background junk".
pub struct Junk {
    pub exe: &'static str,
    pub label: &'static str,
    /// Started again afterwards with these arguments (sync apps). Windows
    /// starts the others itself when they are needed.
    pub reopen: Option<&'static str>,
}

pub const JUNK: &[Junk] = &[
    Junk { exe: "OneDrive.exe", label: "OneDrive", reopen: Some("/background") },
    Junk { exe: "GoogleDriveFS.exe", label: "Google Drive", reopen: Some("") },
    Junk { exe: "Dropbox.exe", label: "Dropbox", reopen: Some("/home") },
    Junk { exe: "Widgets.exe", label: "Widgets", reopen: None },
    Junk { exe: "WidgetService.exe", label: "Widgets", reopen: None },
    Junk { exe: "PhoneExperienceHost.exe", label: "Phone Link", reopen: None },
    Junk { exe: "YourPhone.exe", label: "Phone Link", reopen: None },
    Junk { exe: "Cortana.exe", label: "Cortana", reopen: None },
    Junk { exe: "SkypeBackgroundHost.exe", label: "Skype", reopen: None },
    Junk { exe: "AdobeARM.exe", label: "Adobe updater", reopen: None },
    Junk { exe: "AdobeCollabSync.exe", label: "Acrobat sync", reopen: None },
    Junk { exe: "CCXProcess.exe", label: "Creative Cloud", reopen: None },
    Junk { exe: "CoreSync.exe", label: "Creative Cloud sync", reopen: None },
    Junk { exe: "jusched.exe", label: "Java updater", reopen: None },
    Junk { exe: "MicrosoftEdgeUpdate.exe", label: "Edge updater", reopen: None },
    Junk { exe: "GoogleUpdate.exe", label: "Google updater", reopen: None },
];

/// Programs given "Below normal" priority while playing.
pub const LOWER: &[&str] = &[
    "chrome.exe",
    "msedge.exe",
    "brave.exe",
    "firefox.exe",
    "opera.exe",
    "vivaldi.exe",
    "steamwebhelper.exe",
    "EpicWebHelper.exe",
    "OneDrive.exe",
    "GoogleDriveFS.exe",
    "Dropbox.exe",
    "Spotify.exe",
    "SearchHost.exe",
    "PhoneExperienceHost.exe",
];

pub fn find(exe: &str) -> Option<&'static Junk> {
    JUNK.iter().find(|j| j.exe.eq_ignore_ascii_case(exe))
}

/// Whether a boost lowers this program's priority.
pub fn lowered(exe: &str) -> bool {
    LOWER.iter().any(|l| l.eq_ignore_ascii_case(exe))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_are_safe() {
        for j in JUNK {
            assert!(super::super::tweaks::valid_exe_name(j.exe), "{}", j.exe);
            assert!(!crate::system::procs::is_protected(j.exe), "{}", j.exe);
        }
        // Voice chat and the game launchers are never closed.
        for keep in ["Discord.exe", "steam.exe", "EpicGamesLauncher.exe", "Spotify.exe", "Teams.exe", "explorer.exe"] {
            assert!(find(keep).is_none(), "{keep}");
        }
        assert!(!lowered("Discord.exe") && lowered("CHROME.EXE"));
        assert_eq!(find("onedrive.exe").and_then(|j| j.reopen), Some("/background"));
    }
}
