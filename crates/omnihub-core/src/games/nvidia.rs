//! NVIDIA's driver settings for one game: its profile in the driver, the
//! same place the NVIDIA Control Panel's "Program settings" tab writes.
//! Through NVAPI's documented driver-settings (DRS) interface, with the
//! ids from NVIDIA's open SDK headers (`nvapi_interface.h`,
//! `NvApiDriverSettings.h`).
//!
//! Five settings, none of which touches the picture beyond texture
//! filtering's own trade-off: the GPU stays at full clocks, the CPU queues
//! at most one frame ahead, textures are filtered the fast way, and the
//! driver neither forces VSync on nor caps the frame rate (a global "Max
//! Frame Rate" is a common, forgotten FPS ceiling). A value the user picked in the control
//! panel themselves is left alone. Saving needs administrator rights, so
//! changes run in the elevated helper; reading does not.

use serde::{Deserialize, Serialize};

/// "Power management mode".
pub const PREFERRED_PSTATE_ID: u32 = 0x1057_EB71;
/// "Maximum pre-rendered frames" (Low Latency Mode "On" is 1).
pub const PRERENDERLIMIT_ID: u32 = 0x007B_A09E;
/// "Texture filtering - Quality".
pub const QUALITY_ENHANCEMENTS_ID: u32 = 0x00CE_2691;
/// "Vertical Sync".
pub const VSYNCMODE_ID: u32 = 0x00A8_79CF;
/// "Max Frame Rate" (the driver's own frame cap; 0 is off).
pub const FRL_FPS_ID: u32 = 0x1083_5002;

/// One driver setting OmniHub sets for a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice {
    pub id: u32,
    pub value: u32,
    pub label: &'static str,
}

pub const CHOICES: [Choice; 5] = [
    // PREFERRED_PSTATE_PREFER_MAX
    Choice { id: PREFERRED_PSTATE_ID, value: 0x1, label: "Power management: prefer maximum performance" },
    Choice { id: PRERENDERLIMIT_ID, value: 0x1, label: "Low latency mode: on" },
    // QUALITY_ENHANCEMENTS_HIGHPERFORMANCE
    Choice { id: QUALITY_ENHANCEMENTS_ID, value: 0x14, label: "Texture filtering: high performance" },
    // VSYNCMODE_FORCEOFF
    Choice { id: VSYNCMODE_ID, value: 0x0841_6747, label: "Vertical sync: off" },
    // A global "Max Frame Rate" in the control panel caps every game,
    // often well under what the PC can do.
    Choice { id: FRL_FPS_ID, value: 0, label: "Driver frame cap: off" },
];

/// How a game's NVIDIA profile stands.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NvidiaState {
    /// Every choice is set.
    pub on: bool,
    /// Labels of the choices that are set.
    pub set: Vec<String>,
}

/// NVAPI's status codes worth a sentence.
pub fn status_text(code: i32) -> String {
    match code {
        -2 => "the NVIDIA driver's NVAPI library isn't there".into(),
        -3 | -104 => "this NVIDIA driver doesn't support it".into(),
        -6 => "no NVIDIA graphics card drives a display".into(),
        -9 => "this NVIDIA driver is too old or too new for OmniHub's request".into(),
        -137 => "the NVIDIA driver wants administrator rights for this".into(),
        -164 => "a driver profile with OmniHub's name exists already".into(),
        -167 => "the game is already in another driver profile".into(),
        c => format!("the NVIDIA driver answered {c}"),
    }
}

/// What to do with one setting, given what the profile holds now:
/// `Some(true)` set it, `Some(false)` it is already set, `None` leave it
/// (the user chose another value themselves).
pub fn plan(choice: &Choice, current: Option<(u32, bool)>) -> Option<bool> {
    match current {
        Some((v, _)) if v == choice.value => Some(false),
        // A value the user set on this game's profile by hand.
        Some((_, true)) => None,
        _ => Some(true),
    }
}

pub use imp::*;

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    use super::*;

    const STRING_MAX: usize = 2048;
    const BINARY_MAX: usize = 4096;
    const OK: i32 = 0;
    const SETTING_NOT_FOUND: i32 = -160;
    const PROFILE_NOT_FOUND: i32 = -163;
    const EXECUTABLE_NOT_FOUND: i32 = -166;
    const DWORD_TYPE: u32 = 0;
    const CURRENT_PROFILE_LOCATION: u32 = 0;

    /// `NVDRS_SETTING_V1` (packed to 4 bytes in nvapi.h).
    #[repr(C, packed(4))]
    #[derive(Clone, Copy)]
    struct Setting {
        version: u32,
        name: [u16; STRING_MAX],
        id: u32,
        kind: u32,
        location: u32,
        is_current_predefined: u32,
        is_predefined_valid: u32,
        predefined: [u8; 4 + BINARY_MAX],
        current: [u8; 4 + BINARY_MAX],
    }

    /// `NVDRS_APPLICATION_V4`.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Application {
        version: u32,
        is_predefined: u32,
        app_name: [u16; STRING_MAX],
        friendly_name: [u16; STRING_MAX],
        launcher: [u16; STRING_MAX],
        file_in_folder: [u16; STRING_MAX],
        flags: u32,
        command_line: [u16; STRING_MAX],
    }

    /// `NVDRS_PROFILE_V1`.
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct Profile {
        version: u32,
        name: [u16; STRING_MAX],
        gpu_support: u32,
        is_predefined: u32,
        num_apps: u32,
        num_settings: u32,
    }

    const fn version<T>(v: u32) -> u32 {
        std::mem::size_of::<T>() as u32 | (v << 16)
    }

    const _: () = assert!(std::mem::size_of::<Setting>() == 12320);
    const _: () = assert!(std::mem::size_of::<Application>() == 20492);
    const _: () = assert!(std::mem::size_of::<Profile>() == 4116);

    type Handle = *mut c_void;
    type QueryInterface = unsafe extern "C" fn(u32) -> *mut c_void;
    type Initialize = unsafe extern "C" fn() -> i32;
    type SessionFn = unsafe extern "C" fn(Handle) -> i32;
    type CreateSession = unsafe extern "C" fn(*mut Handle) -> i32;
    type FindApplicationByName = unsafe extern "C" fn(Handle, *const u16, *mut Handle, *mut Application) -> i32;
    type FindProfileByName = unsafe extern "C" fn(Handle, *const u16, *mut Handle) -> i32;
    type CreateProfile = unsafe extern "C" fn(Handle, *mut Profile, *mut Handle) -> i32;
    type CreateApplication = unsafe extern "C" fn(Handle, Handle, *mut Application) -> i32;
    type GetSetting = unsafe extern "C" fn(Handle, Handle, u32, *mut Setting) -> i32;
    type SetSetting = unsafe extern "C" fn(Handle, Handle, *mut Setting) -> i32;
    type RestoreDefault = unsafe extern "C" fn(Handle, Handle, u32) -> i32;

    // From nvapi_interface.h.
    const ID_INITIALIZE: u32 = 0x0150_e828;
    const ID_CREATE_SESSION: u32 = 0x0694_d52e;
    const ID_DESTROY_SESSION: u32 = 0xdad9_cff8;
    const ID_LOAD_SETTINGS: u32 = 0x375d_bd6b;
    const ID_SAVE_SETTINGS: u32 = 0xfcbc_7e14;
    const ID_CREATE_PROFILE: u32 = 0xcc17_6068;
    const ID_FIND_PROFILE: u32 = 0x7e4a_9a0b;
    const ID_CREATE_APPLICATION: u32 = 0x4347_a9de;
    const ID_FIND_APPLICATION: u32 = 0xeee5_66b2;
    const ID_SET_SETTING: u32 = 0x577d_d202;
    const ID_GET_SETTING: u32 = 0x73bf_8338;
    const ID_RESTORE_DEFAULT: u32 = 0x53f0_381e;

    fn wide(s: &str) -> [u16; STRING_MAX] {
        let mut out = [0u16; STRING_MAX];
        for (slot, c) in out.iter_mut().zip(s.encode_utf16().take(STRING_MAX - 1)) {
            *slot = c;
        }
        out
    }

    /// An NVAPI session with the driver's settings loaded.
    struct Drs {
        query: QueryInterface,
        session: Handle,
    }

    impl Drs {
        fn open() -> Result<Drs, String> {
            use windows::core::{s, w};
            use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
            // SAFETY: nvapi64.dll is NVIDIA's driver library; QueryInterface
            // takes an id and returns the function or null.
            unsafe {
                let lib = LoadLibraryW(w!("nvapi64.dll")).map_err(|_| status_text(-2))?;
                let query = GetProcAddress(lib, s!("nvapi_QueryInterface")).ok_or_else(|| status_text(-2))?;
                let query: QueryInterface = std::mem::transmute(query);
                let mut drs = Drs { query, session: std::ptr::null_mut() };
                let init: Initialize = drs.func(ID_INITIALIZE)?;
                check(init())?;
                let create: CreateSession = drs.func(ID_CREATE_SESSION)?;
                check(create(&mut drs.session))?;
                let load: SessionFn = drs.func(ID_LOAD_SETTINGS)?;
                check(load(drs.session))?;
                Ok(drs)
            }
        }

        /// # Safety
        /// `F` must be the signature NVAPI documents for `id`.
        unsafe fn func<F: Copy>(&self, id: u32) -> Result<F, String> {
            let p = unsafe { (self.query)(id) };
            if p.is_null() {
                return Err(status_text(-3));
            }
            Ok(unsafe { std::mem::transmute_copy::<*mut c_void, F>(&p) })
        }

        /// The profile the driver uses for `exe`, if any.
        fn find(&self, exe: &str) -> Result<Option<Handle>, String> {
            let name = wide(&exe.to_ascii_lowercase());
            let mut app: Box<Application> = Box::new(unsafe { std::mem::zeroed() });
            app.version = version::<Application>(4);
            let mut profile: Handle = std::ptr::null_mut();
            unsafe {
                let find: FindApplicationByName = self.func(ID_FIND_APPLICATION)?;
                match find(self.session, name.as_ptr(), &mut profile, &mut *app) {
                    OK => Ok(Some(profile)),
                    EXECUTABLE_NOT_FOUND => Ok(None),
                    c => Err(status_text(c)),
                }
            }
        }

        /// The game's profile, created (with the game in it) when the
        /// driver has none.
        fn find_or_create(&self, exe: &str) -> Result<Handle, String> {
            if let Some(p) = self.find(exe)? {
                return Ok(p);
            }
            let title = wide(&format!("OmniHub - {exe}"));
            let mut profile: Handle = std::ptr::null_mut();
            unsafe {
                let by_name: FindProfileByName = self.func(ID_FIND_PROFILE)?;
                match by_name(self.session, title.as_ptr(), &mut profile) {
                    OK => {}
                    PROFILE_NOT_FOUND => {
                        let mut info: Box<Profile> = Box::new(std::mem::zeroed());
                        info.version = version::<Profile>(1);
                        info.name = title;
                        let create: CreateProfile = self.func(ID_CREATE_PROFILE)?;
                        check(create(self.session, &mut *info, &mut profile))?;
                    }
                    c => return Err(status_text(c)),
                }
                let mut app: Box<Application> = Box::new(std::mem::zeroed());
                app.version = version::<Application>(4);
                app.app_name = wide(&exe.to_ascii_lowercase());
                app.friendly_name = wide(exe);
                let add: CreateApplication = self.func(ID_CREATE_APPLICATION)?;
                check(add(self.session, profile, &mut *app))?;
            }
            Ok(profile)
        }

        /// The value of a DWORD setting on a profile and whether the user
        /// set it there (rather than NVIDIA's predefined value or the global
        /// one); `None` when the profile doesn't hold it.
        fn get(&self, profile: Handle, id: u32) -> Result<Option<(u32, bool)>, String> {
            let mut s: Box<Setting> = Box::new(unsafe { std::mem::zeroed() });
            s.version = version::<Setting>(1);
            unsafe {
                let get: GetSetting = self.func(ID_GET_SETTING)?;
                match get(self.session, profile, id, &mut *s) {
                    OK => {}
                    SETTING_NOT_FOUND => return Ok(None),
                    c => return Err(status_text(c)),
                }
            }
            if s.kind != DWORD_TYPE {
                return Ok(None);
            }
            let value = u32::from_le_bytes([s.current[0], s.current[1], s.current[2], s.current[3]]);
            let by_user = s.location == CURRENT_PROFILE_LOCATION && s.is_current_predefined == 0;
            Ok(Some((value, by_user)))
        }

        fn set(&self, profile: Handle, id: u32, value: u32) -> Result<(), String> {
            let mut s: Box<Setting> = Box::new(unsafe { std::mem::zeroed() });
            s.version = version::<Setting>(1);
            s.id = id;
            s.kind = DWORD_TYPE;
            s.current[..4].copy_from_slice(&value.to_le_bytes());
            unsafe {
                let set: SetSetting = self.func(ID_SET_SETTING)?;
                check(set(self.session, profile, &mut *s))
            }
        }

        fn restore(&self, profile: Handle, id: u32) -> Result<(), String> {
            unsafe {
                let restore: RestoreDefault = self.func(ID_RESTORE_DEFAULT)?;
                match restore(self.session, profile, id) {
                    OK | SETTING_NOT_FOUND => Ok(()),
                    c => Err(status_text(c)),
                }
            }
        }

        fn save(&self) -> Result<(), String> {
            unsafe {
                let save: SessionFn = self.func(ID_SAVE_SETTINGS)?;
                check(save(self.session))
            }
        }
    }

    impl Drop for Drs {
        fn drop(&mut self) {
            if !self.session.is_null() {
                if let Ok(destroy) = unsafe { self.func::<SessionFn>(ID_DESTROY_SESSION) } {
                    unsafe { destroy(self.session) };
                }
            }
        }
    }

    fn check(code: i32) -> Result<(), String> {
        if code == OK {
            Ok(())
        } else {
            Err(status_text(code))
        }
    }

    /// Whether this PC has an NVIDIA driver to talk to.
    pub fn available() -> bool {
        Drs::open().is_ok()
    }

    /// The game's profile as it stands (no administrator rights needed).
    pub fn state(exe: &str) -> Result<NvidiaState, String> {
        let drs = Drs::open()?;
        let Some(profile) = drs.find(exe)? else { return Ok(NvidiaState { on: false, set: Vec::new() }) };
        let mut set = Vec::new();
        for c in &CHOICES {
            if drs.get(profile, c.id)?.is_some_and(|(v, _)| v == c.value) {
                set.push(c.label.to_string());
            }
        }
        Ok(NvidiaState { on: set.len() == CHOICES.len(), set })
    }

    /// Elevated: set the choices on the game's profile. Returns what changed.
    pub fn apply(exe: &str) -> Result<Vec<String>, String> {
        let drs = Drs::open()?;
        let profile = drs.find_or_create(exe)?;
        let mut changed = Vec::new();
        for c in &CHOICES {
            if plan(c, drs.get(profile, c.id)?) == Some(true) {
                drs.set(profile, c.id, c.value)?;
                changed.push(c.label.to_string());
            }
        }
        if !changed.is_empty() {
            drs.save()?;
        }
        Ok(changed)
    }

    /// Elevated: put OmniHub's choices back to the driver's defaults.
    pub fn undo(exe: &str) -> Result<(), String> {
        let drs = Drs::open()?;
        let Some(profile) = drs.find(exe)? else { return Ok(()) };
        let mut any = false;
        for c in &CHOICES {
            if drs.get(profile, c.id)?.is_some_and(|(v, _)| v == c.value) {
                drs.restore(profile, c.id)?;
                any = true;
            }
        }
        if any {
            drs.save()?;
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn available() -> bool {
        false
    }
    pub fn state(_exe: &str) -> Result<NvidiaState, String> {
        Err("Windows only".into())
    }
    pub fn apply(_exe: &str) -> Result<Vec<String>, String> {
        Err("Windows only".into())
    }
    pub fn undo(_exe: &str) -> Result<(), String> {
        Err("Windows only".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_what_the_user_chose() {
        let c = CHOICES[0];
        assert_eq!(plan(&c, None), Some(true), "not on the profile: set it");
        assert_eq!(plan(&c, Some((5, false))), Some(true), "NVIDIA's default for the game: set it");
        assert_eq!(plan(&c, Some((c.value, true))), Some(false), "already set");
        assert_eq!(plan(&c, Some((5, true))), None, "the user's own choice stays");
        assert!(status_text(-137).contains("administrator"));
        let ids: std::collections::HashSet<u32> = CHOICES.iter().map(|c| c.id).collect();
        assert_eq!(ids.len(), CHOICES.len());
    }
}
