//! Freeze background apps while gaming: the programs you pick are paused
//! (every thread suspended, as Resource Monitor's "Suspend process" does)
//! for the game, and resumed when it closes. A paused program uses no
//! processor at all; it keeps its memory and carries on where it stopped.
//! Windows' own processes, OmniHub and anything a game needs are never
//! paused, and a crash during a game resumes them on the next start.

/// Programs that are never paused, whatever the list says: pausing them
/// would freeze the desktop, the sound or the game itself.
const NEVER: &[&str] = &["explorer.exe", "dwm.exe", "csrss.exe", "winlogon.exe", "lsass.exe", "services.exe", "svchost.exe", "audiodg.exe", "fontdrvhost.exe", "sihost.exe", "ctfmon.exe", "searchhost.exe", "startmenuexperiencehost.exe", "shellexperiencehost.exe", "textinputhost.exe", "runtimebroker.exe", "conhost.exe", "omnihub.exe", "msedgewebview2.exe", "steam.exe", "steamwebhelper.exe", "epicgameslauncher.exe", "easyanticheat.exe", "easyanticheat_eos.exe", "beservice.exe", "vgc.exe", "vgtray.exe", "riotclientservices.exe", "battle.net.exe", "eadesktop.exe", "upc.exe", "gamebar.exe", "nvcontainer.exe", "nvdisplay.container.exe"];

/// Whether `name` may be paused while `game` runs.
pub fn allowed(name: &str, game: &str) -> bool {
    let n = name.trim().to_ascii_lowercase();
    !n.is_empty() && n.ends_with(".exe") && !n.eq_ignore_ascii_case(game) && !NEVER.contains(&n.as_str()) && !crate::system::procs::is_protected(&n)
}

/// Pause (or resume) every thread of these processes; returns how many
/// processes were touched.
pub fn set(pids: &[u32], paused: bool) -> usize {
    imp::set(pids, paused)
}

#[cfg(windows)]
mod imp {
    use std::collections::HashSet;

    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32};
    use windows::Win32::System::Threading::{OpenThread, ResumeThread, SuspendThread, THREAD_SUSPEND_RESUME};

    pub fn set(pids: &[u32], paused: bool) -> usize {
        let want: HashSet<u32> = pids.iter().copied().filter(|p| *p > 4 && *p != std::process::id()).collect();
        if want.is_empty() {
            return 0;
        }
        let mut touched = HashSet::new();
        // SAFETY: a snapshot handle we own and close; THREADENTRY32 is sized before use.
        unsafe {
            let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) else { return 0 };
            let mut e = THREADENTRY32 { dwSize: std::mem::size_of::<THREADENTRY32>() as u32, ..Default::default() };
            let mut ok = Thread32First(snap, &mut e).is_ok();
            while ok {
                if want.contains(&e.th32OwnerProcessID) {
                    if let Ok(h) = OpenThread(THREAD_SUSPEND_RESUME, false, e.th32ThreadID) {
                        let r = if paused { SuspendThread(h) } else { ResumeThread(h) };
                        if r != u32::MAX {
                            touched.insert(e.th32OwnerProcessID);
                        }
                        let _ = CloseHandle(h);
                    }
                }
                ok = Thread32Next(snap, &mut e).is_ok();
            }
            let _ = CloseHandle(snap);
        }
        touched.len()
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn set(_pids: &[u32], _paused: bool) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_pauses_windows_or_the_game() {
        assert!(allowed("Spotify.exe", "cs2.exe"));
        assert!(allowed("OneDrive.exe", "cs2.exe"));
        assert!(!allowed("cs2.exe", "cs2.exe"));
        assert!(!allowed("explorer.exe", "cs2.exe"));
        assert!(!allowed("Steam.exe", "cs2.exe"));
        assert!(!allowed("svchost.exe", "cs2.exe"));
        assert!(!allowed("notanexe", "cs2.exe"));
        assert_eq!(set(&[], true), 0);
    }
}
