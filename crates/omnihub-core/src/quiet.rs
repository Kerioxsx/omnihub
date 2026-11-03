//! Quiet mode, for the couch or the night: the screens off and the fans at
//! the lowest safe speed, with the PC still on (downloads, servers, the
//! phone keep working). Fans never stop completely — a running processor
//! would overheat — they sit at their floor and still speed up if it gets
//! warm; the graphics card goes back to its own control, which stops its
//! fans when it's cool on most cards. It ends when someone uses the PC's
//! mouse or keyboard, or from the phone, and the fans go back to what they
//! were.

use parking_lot::Mutex;
use serde::Serialize;
use serde_json::json;

use crate::rig::fans::FanProfile;

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct QuietStatus {
    pub on: bool,
    /// Unix ms.
    pub since: Option<i64>,
    /// OmniHub can set this PC's fans (LibreHardwareMonitor or a cooler is
    /// connected); otherwise quiet mode only turns the screens off.
    pub fans: bool,
}

struct State {
    before: FanProfile,
    since: i64,
    input: Option<u32>,
}

#[derive(Default)]
pub struct Quiet {
    state: Mutex<Option<State>>,
}

/// The tick of the last local keyboard or mouse input (Windows' own count).
pub fn last_input() -> Option<u32> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
        let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
        // SAFETY: fills the struct we pass.
        unsafe { GetLastInputInfo(&mut info) }.as_bool().then_some(info.dwTime)
    }
    #[cfg(not(windows))]
    None
}

impl crate::core::AppCore {
    pub fn quiet_status(&self) -> QuietStatus {
        let st = self.quiet.state.lock();
        let rig = self.rig.status();
        let fans = rig.fans == crate::rig::LinkState::On || !rig.cooler_devices.is_empty();
        QuietStatus { on: st.is_some(), since: st.as_ref().map(|s| s.since), fans }
    }

    /// Screens off and fans to their lowest; again turns the screens off.
    pub fn quiet_on(&self) -> Result<QuietStatus, String> {
        {
            let mut st = self.quiet.state.lock();
            if st.is_none() {
                let before = self.rig.settings().fans.profile;
                *st = Some(State { before, since: chrono::Utc::now().timestamp_millis(), input: None });
            }
        }
        self.rig.update(&json!({ "fans": { "profile": "lowest" } }))?;
        crate::system::power::execute(crate::system::power::PowerAction::DisplayOff).map_err(|e| e.to_string())?;
        // What counts as "someone's back": input after the screens went off.
        std::thread::sleep(std::time::Duration::from_millis(600));
        if let Some(s) = self.quiet.state.lock().as_mut() {
            s.input = last_input();
        }
        self.events.emit("quiet:changed", self.quiet_status());
        Ok(self.quiet_status())
    }

    /// Back to normal: the fans as they were, the screens on.
    pub fn quiet_off(&self) -> Result<QuietStatus, String> {
        let Some(st) = self.quiet.state.lock().take() else { return Ok(self.quiet_status()) };
        let before = serde_json::to_value(st.before).map_err(|e| e.to_string())?;
        self.rig.update(&json!({ "fans": { "profile": before } }))?;
        crate::system::awake::wake_screen();
        self.events.emit("quiet:changed", self.quiet_status());
        Ok(self.quiet_status())
    }

    /// Called every couple of seconds: someone used the mouse or keyboard.
    pub fn quiet_check(&self) {
        let back = {
            let st = self.quiet.state.lock();
            match (st.as_ref().and_then(|s| s.input), last_input()) {
                (Some(then), Some(now)) => now != then,
                _ => false,
            }
        };
        if back {
            let _ = self.quiet_off();
        }
    }
}
