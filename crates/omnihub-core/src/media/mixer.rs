//! The PC's volume mixer for the phone: each app's volume and mute (on
//! every playback device), muting the microphone, and the apps recording
//! from a microphone right now. A calling app (Discord, WhatsApp, Nyxen,
//! Teams, a browser…) doing that counts as a call; anything else (a game's
//! voice chat) is only listed as using the microphone.
//!
//! Muting the microphone mutes the PC's recording devices themselves, so it
//! works in every app; "deafen" mutes the call app's own sound. Windows
//! offers no way for another program to hang up a call.

use std::sync::LazyLock;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use super::audio::Volume;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppVolume {
    /// Lower-case executable name ("discord.exe"), or "system" for Windows' own sounds.
    pub key: String,
    /// "Discord".
    pub name: String,
    /// 0.0–1.0
    pub level: f32,
    pub muted: bool,
    /// Playing sound right now.
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Call {
    /// Same key as the app's [`AppVolume`] when it plays sound.
    pub key: String,
    pub name: String,
    /// When it started using the microphone (ms since the epoch).
    pub since_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Mic {
    pub muted: bool,
    /// Recording devices (all of them are muted and unmuted together).
    pub devices: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Mixer {
    pub master: Option<Volume>,
    pub mic: Option<Mic>,
    pub apps: Vec<AppVolume>,
    /// Calling apps recording from a microphone right now.
    pub calls: Vec<Call>,
    /// Other apps recording from a microphone right now ("Skate").
    pub mic_apps: Vec<String>,
}

/// Friendly names, and the executables one app runs under.
const KNOWN: &[(&str, &str)] = &[
    ("discord.exe", "Discord"),
    ("discordptb.exe", "Discord PTB"),
    ("discordcanary.exe", "Discord Canary"),
    ("whatsapp.exe", "WhatsApp"),
    ("nyxen.exe", "Nyxen"),
    ("ms-teams.exe", "Teams"),
    ("teams.exe", "Teams"),
    ("zoom.exe", "Zoom"),
    ("skype.exe", "Skype"),
    ("telegram.exe", "Telegram"),
    ("signal.exe", "Signal"),
    ("slack.exe", "Slack"),
    ("spotify.exe", "Spotify"),
    ("chrome.exe", "Chrome"),
    ("msedge.exe", "Edge"),
    ("brave.exe", "Brave"),
    ("firefox.exe", "Firefox"),
    ("opera.exe", "Opera"),
    ("opera_gx.exe", "Opera GX"),
    ("steam.exe", "Steam"),
    ("steamwebhelper.exe", "Steam"),
    ("vlc.exe", "VLC"),
    ("obs64.exe", "OBS"),
    ("applemusic.exe", "Apple Music"),
    ("fortniteclient-win64-shipping.exe", "Fortnite"),
    ("robloxplayerbeta.exe", "Roblox"),
    ("valorant-win64-shipping.exe", "VALORANT"),
    ("cs2.exe", "Counter-Strike 2"),
];

/// Apps whose use of the microphone means a call.
const CALL_APPS: &[&str] = &[
    "discord.exe", "discordptb.exe", "discordcanary.exe", "whatsapp.exe", "nyxen.exe", "ms-teams.exe", "teams.exe", "zoom.exe", "skype.exe", "telegram.exe", "signal.exe", "slack.exe", "chrome.exe", "msedge.exe", "brave.exe", "firefox.exe", "opera.exe", "opera_gx.exe",
];

pub fn is_call_app(key: &str) -> bool {
    CALL_APPS.contains(&key)
}

/// Helper processes that play and record for the app that started them.
const HELPERS: &[&str] = &["msedgewebview2.exe"];

/// The app a process belongs to, from a table of pid → (parent pid, exe
/// name): WebView2 helpers count as the app that started them.
pub fn owner_key(pid: u32, table: &std::collections::HashMap<u32, (u32, String)>) -> Option<String> {
    let mut cur = pid;
    let mut key = key_of(&table.get(&cur)?.1);
    for _ in 0..4 {
        if !HELPERS.contains(&key.as_str()) {
            break;
        }
        let parent = table.get(&cur)?.0;
        match table.get(&parent) {
            Some((_, exe)) if parent != 0 && parent != cur => {
                cur = parent;
                key = key_of(exe);
            }
            _ => break,
        }
    }
    Some(key)
}

/// Split the apps recording right now into calls and others.
pub fn split_calls(recording: Vec<String>, since: impl Fn(&str) -> Option<i64>) -> (Vec<Call>, Vec<String>) {
    let mut calls: Vec<Call> = Vec::new();
    let mut others: Vec<String> = Vec::new();
    for key in recording {
        if key == "omnihub.exe" {
            continue;
        }
        if is_call_app(&key) {
            if !calls.iter().any(|c| c.key == key) {
                calls.push(Call { since_ms: since(&key), name: name_of(&key), key });
            }
        } else {
            let name = name_of(&key);
            if !others.contains(&name) {
                others.push(name);
            }
        }
    }
    (calls, others)
}

/// Executables that are the same app as another one.
const ALIASES: &[(&str, &str)] = &[("whatsapp.root.exe", "whatsapp.exe"), ("steamwebhelper.exe", "steam.exe"), ("msteams.exe", "ms-teams.exe")];

/// The grouping key of an executable name or path.
pub fn key_of(exe: &str) -> String {
    let file = exe.rsplit(['\\', '/']).next().unwrap_or(exe).to_ascii_lowercase();
    ALIASES.iter().find(|(a, _)| *a == file).map_or(file, |(_, to)| to.to_string())
}

/// "Discord" for "discord.exe"; otherwise the file name without ".exe".
pub fn name_of(key: &str) -> String {
    if let Some((_, n)) = KNOWN.iter().find(|(k, _)| *k == key) {
        return n.to_string();
    }
    let stem = key.strip_suffix(".exe").unwrap_or(key);
    let mut c = stem.chars();
    c.next().map_or_else(String::new, |f| f.to_uppercase().chain(c).collect())
}

/// A Store app's package family name ("5319275A.WhatsAppDesktop_cv1g1gvanyjgm")
/// as a key and a name.
pub fn packaged_app(family: &str) -> (String, String) {
    let lower = family.to_ascii_lowercase();
    for (needle, key) in [("whatsapp", "whatsapp.exe"), ("teams", "ms-teams.exe"), ("skype", "skype.exe"), ("zoom", "zoom.exe"), ("telegram", "telegram.exe"), ("discord", "discord.exe"), ("spotify", "spotify.exe")] {
        if lower.contains(needle) {
            return (key.to_string(), name_of(key));
        }
    }
    // "Publisher.AppName_hash" → "AppName".
    let mid = family.split('_').next().unwrap_or(family);
    let name = mid.rsplit('.').next().unwrap_or(mid).to_string();
    (format!("{}.exe", name.to_ascii_lowercase()), name)
}

/// One app per key, the loudest session's level; system sounds last.
pub fn group(sessions: Vec<(String, Option<String>, f32, bool, bool)>) -> Vec<AppVolume> {
    let mut out: Vec<AppVolume> = Vec::new();
    for (key, display, level, muted, active) in sessions {
        if let Some(a) = out.iter_mut().find(|a| a.key == key) {
            a.level = a.level.max(level);
            a.muted &= muted;
            a.active |= active;
        } else {
            let name = display.filter(|d| !d.trim().is_empty() && !d.starts_with('@')).unwrap_or_else(|| if key == "system" { "System sounds".into() } else { name_of(&key) });
            out.push(AppVolume { key, name, level, muted, active });
        }
    }
    out.sort_by(|a, b| (a.key == "system").cmp(&(b.key == "system")).then(b.active.cmp(&a.active)).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    out
}

fn is_fake(fake: bool) -> bool {
    fake || std::env::var("OMNIHUB_FAKE_MEDIA").is_ok_and(|v| v == "1")
}

/// Everything the phone's mixer shows.
pub fn get(fake: bool) -> Mixer {
    if is_fake(fake) {
        return FAKE.lock().clone();
    }
    #[cfg(windows)]
    {
        let (calls, mic_apps) = split_calls(win::recording().unwrap_or_default(), win::mic_since);
        Mixer { master: super::audio::get(), mic: win::mic().ok().flatten(), apps: win::apps().unwrap_or_default(), calls, mic_apps }
    }
    #[cfg(not(windows))]
    {
        Mixer::default()
    }
}

/// Change one app's volume or mute (every session it has, on every device).
pub fn set_app(fake: bool, key: &str, level: Option<f32>, muted: Option<bool>) -> Result<Mixer, String> {
    let level = level.filter(|l| l.is_finite()).map(|l| l.clamp(0.0, 1.0));
    if is_fake(fake) {
        let mut m = FAKE.lock();
        let a = m.apps.iter_mut().find(|a| a.key == key).ok_or_else(|| format!("{} is not playing sound", name_of(key)))?;
        if let Some(l) = level {
            a.level = l;
        }
        if let Some(x) = muted {
            a.muted = x;
        }
        return Ok(m.clone());
    }
    #[cfg(windows)]
    {
        let changed = win::set_app(key, level, muted).map_err(|e| e.message().to_string())?;
        if changed == 0 {
            return Err(format!("{} is not playing sound", name_of(key)));
        }
        Ok(get(false))
    }
    #[cfg(not(windows))]
    {
        let _ = (level, muted);
        Err("The volume mixer is a Windows feature.".into())
    }
}

/// Mute or unmute every recording device.
pub fn set_mic(fake: bool, muted: bool) -> Result<Mixer, String> {
    if is_fake(fake) {
        let mut m = FAKE.lock();
        if let Some(mic) = m.mic.as_mut() {
            mic.muted = muted;
        }
        return Ok(m.clone());
    }
    #[cfg(windows)]
    {
        match win::set_mic(muted).map_err(|e| e.message().to_string())? {
            0 => Err("This PC has no microphone.".into()),
            _ => Ok(get(false)),
        }
    }
    #[cfg(not(windows))]
    {
        let _ = muted;
        Err("Muting the microphone is a Windows feature.".into())
    }
}

/// Master volume, for the pretend mixer too.
pub fn set_master(fake: bool, level: Option<f32>, muted: Option<bool>) -> Result<Mixer, String> {
    if is_fake(fake) {
        let mut m = FAKE.lock();
        if let Some(v) = m.master.as_mut() {
            if let Some(l) = level.filter(|l| l.is_finite()) {
                v.level = l.clamp(0.0, 1.0);
            }
            if let Some(x) = muted {
                v.muted = x;
            }
        }
        return Ok(m.clone());
    }
    super::audio::set(level, muted)?;
    Ok(get(false))
}

/// The pretend mixer (`OMNIHUB_FAKE_MEDIA=1`): music, a Discord call, a game.
static FAKE: LazyLock<Mutex<Mixer>> = LazyLock::new(|| Mutex::new(fake_mixer()));

fn fake_mixer() -> Mixer {
    let app = |key: &str, level: f32, active: bool| AppVolume { key: key.into(), name: name_of(key), level, muted: false, active };
    Mixer {
        master: Some(Volume { level: 0.62, muted: false }),
        mic: Some(Mic { muted: false, devices: 1 }),
        apps: vec![app("discord.exe", 0.8, true), app("spotify.exe", 0.55, true), app("fortniteclient-win64-shipping.exe", 0.7, true), app("brave.exe", 1.0, false), app("nyxen.exe", 0.9, false), AppVolume { key: "system".into(), name: "System sounds".into(), level: 0.5, muted: false, active: false }],
        calls: vec![Call { key: "discord.exe".into(), name: "Discord".into(), since_ms: Some(crate::media::now_ms() - 14 * 60_000) }],
        mic_apps: vec![],
    }
}

/// Put the pretend mixer back as it started (tests).
pub fn reset_fake() {
    *FAKE.lock() = fake_mixer();
}

#[cfg(windows)]
mod win {
    use windows::core::{Interface, Result, PWSTR};
    use windows::Win32::Foundation::{CloseHandle, S_OK};
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{eCapture, eRender, AudioSessionStateActive, AudioSessionStateExpired, EDataFlow, IAudioSessionControl2, IAudioSessionManager2, IMMDevice, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator, DEVICE_STATE_ACTIVE};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_MULTITHREADED};
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};

    use std::collections::HashMap;

    use super::{AppVolume, Mic};

    /// COM work on a short-lived thread of its own (see audio.rs).
    fn com<T: Send>(f: impl FnOnce() -> Result<T> + Send) -> Result<T> {
        std::thread::scope(|s| {
            s.spawn(|| unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                f()
            })
            .join()
        })
        .unwrap_or_else(|_| Err(windows::core::Error::from(windows::core::HRESULT(0x8000_FFFFu32 as i32))))
    }

    unsafe fn devices(flow: EDataFlow) -> Result<Vec<IMMDevice>> {
        let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let col = en.EnumAudioEndpoints(flow, DEVICE_STATE_ACTIVE)?;
        (0..col.GetCount()?).map(|i| col.Item(i)).collect()
    }

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

    /// pid → (parent pid, exe name), read only when a WebView2 helper needs it.
    fn process_table() -> HashMap<u32, (u32, String)> {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};
        let mut sys = System::new();
        sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
        sys.processes().iter().map(|(pid, p)| (pid.as_u32(), (p.parent().map_or(0, |x| x.as_u32()), p.name().to_string_lossy().into_owned()))).collect()
    }

    /// The app key of a session's process (WebView2 helpers resolved).
    fn key_for(pid: u32, table: &mut Option<HashMap<u32, (u32, String)>>) -> Option<String> {
        let key = super::key_of(&exe_of(pid)?);
        if !super::HELPERS.contains(&key.as_str()) {
            return Some(key);
        }
        super::owner_key(pid, table.get_or_insert_with(process_table)).or(Some(key))
    }

    struct Session {
        key: String,
        display: Option<String>,
        active: bool,
        volume: ISimpleAudioVolume,
    }

    unsafe fn sessions() -> Result<Vec<Session>> {
        sessions_on(eRender, false)
    }

    /// Sessions on every active device of one direction (`only_active`: those
    /// playing or recording right now).
    unsafe fn sessions_on(flow: EDataFlow, only_active: bool) -> Result<Vec<Session>> {
        let me = std::process::id();
        let mut table = None;
        let mut out = Vec::new();
        for dev in devices(flow)? {
            let Ok(mgr) = dev.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else { continue };
            let en = mgr.GetSessionEnumerator()?;
            for i in 0..en.GetCount()? {
                let Ok(ctl) = en.GetSession(i) else { continue };
                let state = ctl.GetState().unwrap_or(AudioSessionStateExpired);
                if state == AudioSessionStateExpired || (only_active && state != AudioSessionStateActive) {
                    continue;
                }
                let Ok(ctl2) = ctl.cast::<IAudioSessionControl2>() else { continue };
                let system = ctl2.IsSystemSoundsSession() == S_OK;
                let pid = ctl2.GetProcessId().unwrap_or(0);
                if pid == me {
                    continue;
                }
                let key = if system {
                    "system".to_string()
                } else {
                    match key_for(pid, &mut table) {
                        Some(key) => key,
                        None => continue,
                    }
                };
                let display = ctl.GetDisplayName().ok().map(|p| {
                    let s = p.to_string().unwrap_or_default();
                    CoTaskMemFree(Some(p.0 as *const _));
                    s
                });
                let Ok(volume) = ctl.cast::<ISimpleAudioVolume>() else { continue };
                out.push(Session { key, display, active: state == AudioSessionStateActive, volume });
            }
        }
        Ok(out)
    }

    pub fn apps() -> Result<Vec<AppVolume>> {
        com(|| unsafe {
            let rows = sessions()?
                .into_iter()
                .filter_map(|s| Some((s.key, s.display, s.volume.GetMasterVolume().ok()?, s.volume.GetMute().ok()?.as_bool(), s.active)))
                .collect();
            Ok(super::group(rows))
        })
    }

    pub fn set_app(key: &str, level: Option<f32>, muted: Option<bool>) -> Result<usize> {
        let key = key.to_string();
        com(move || unsafe {
            let mut n = 0;
            for s in sessions()?.into_iter().filter(|s| s.key == key) {
                if let Some(l) = level {
                    s.volume.SetMasterVolume(l, std::ptr::null())?;
                }
                if let Some(m) = muted {
                    s.volume.SetMute(m, std::ptr::null())?;
                }
                n += 1;
            }
            Ok(n)
        })
    }

    pub fn mic() -> Result<Option<Mic>> {
        com(|| unsafe {
            let mut muted = true;
            let mut n = 0;
            for dev in devices(eCapture)? {
                let Ok(ep) = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) else { continue };
                muted &= ep.GetMute()?.as_bool();
                n += 1;
            }
            Ok((n > 0).then_some(Mic { muted, devices: n }))
        })
    }

    pub fn set_mic(muted: bool) -> Result<usize> {
        com(move || unsafe {
            let mut n = 0;
            for dev in devices(eCapture)? {
                let Ok(ep) = dev.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) else { continue };
                ep.SetMute(muted, std::ptr::null())?;
                n += 1;
            }
            Ok(n)
        })
    }

    /// Apps recording from a microphone right now: an active session on a
    /// recording device. (Windows' consent store, behind the taskbar's
    /// microphone icon, can stay "in use" after an app stopped or crashed.)
    pub fn recording() -> Result<Vec<String>> {
        com(|| unsafe { Ok(sessions_on(eCapture, true)?.into_iter().filter(|s| s.key != "system").map(|s| s.key).collect()) })
    }

    /// When an app started using the microphone, from Windows' consent store.
    pub fn mic_since(key: &str) -> Option<i64> {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        const STORE: &str = r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone";
        let root = RegKey::predef(HKEY_CURRENT_USER).open_subkey(STORE).ok()?;
        let started = |k: &RegKey| -> Option<i64> {
            let start: u64 = k.get_value("LastUsedTimeStart").ok()?;
            let stop: u64 = k.get_value("LastUsedTimeStop").unwrap_or(0);
            // FILETIME (100 ns since 1601) → ms since 1970.
            (start > stop).then(|| (start / 10_000).checked_sub(11_644_473_600_000).map(|ms| ms as i64)).flatten()
        };
        if let Ok(np) = root.open_subkey("NonPackaged") {
            for path in np.enum_keys().flatten() {
                if super::key_of(&path.replace('#', "\\")) == key {
                    if let Some(t) = np.open_subkey(&path).ok().as_ref().and_then(started) {
                        return Some(t);
                    }
                }
            }
        }
        root.enum_keys().flatten().filter(|f| f != "NonPackaged" && super::packaged_app(f).0 == key).find_map(|f| root.open_subkey(&f).ok().as_ref().and_then(started))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_keys() {
        assert_eq!(key_of(r"C:\Users\A\AppData\Local\Discord\app-1.0.9\Discord.exe"), "discord.exe");
        assert_eq!(key_of(r"C:\Program Files\WindowsApps\x\WhatsApp.Root.exe"), "whatsapp.exe");
        assert_eq!(name_of("discord.exe"), "Discord");
        assert_eq!(name_of("nyxen.exe"), "Nyxen");
        assert_eq!(name_of("someapp.exe"), "Someapp");
        assert_eq!(packaged_app("5319275A.WhatsAppDesktop_cv1g1gvanyjgm"), ("whatsapp.exe".into(), "WhatsApp".into()));
        assert_eq!(packaged_app("MSTeams_8wekyb3d8bbwe"), ("ms-teams.exe".into(), "Teams".into()));
        assert_eq!(packaged_app("Contoso.Recorder_abc123").1, "Recorder");
    }

    #[test]
    fn calls_are_calling_apps_recording_now() {
        let (calls, others) = split_calls(vec!["discord.exe".into(), "skate.exe".into(), "discord.exe".into(), "omnihub.exe".into(), "whatsapp.exe".into()], |k| (k == "discord.exe").then_some(5));
        assert_eq!(calls.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Discord", "WhatsApp"]);
        assert_eq!(calls[0].since_ms, Some(5));
        assert_eq!(others, ["Skate"], "a game's voice chat is not a call");
        // WebView2 helpers count as the app that started them.
        let table: std::collections::HashMap<u32, (u32, String)> = [(10, (1, "WhatsApp.exe".to_string())), (11, (10, "msedgewebview2.exe".into())), (12, (11, "msedgewebview2.exe".into())), (20, (0, "msedgewebview2.exe".into()))].into_iter().collect();
        assert_eq!(owner_key(12, &table).as_deref(), Some("whatsapp.exe"));
        assert_eq!(owner_key(20, &table).as_deref(), Some("msedgewebview2.exe"));
        assert_eq!(owner_key(99, &table), None);
    }

    #[test]
    fn grouping() {
        let g = group(vec![
            ("system".into(), None, 0.5, false, false),
            ("discord.exe".into(), None, 0.4, true, false),
            ("discord.exe".into(), Some("@%SystemRoot%\\x.dll,-1".into()), 0.8, false, true),
            ("spotify.exe".into(), Some("Spotify".into()), 0.6, false, true),
        ]);
        assert_eq!(g.iter().map(|a| a.key.as_str()).collect::<Vec<_>>(), ["discord.exe", "spotify.exe", "system"]);
        let d = &g[0];
        assert_eq!((d.name.as_str(), d.level, d.muted, d.active), ("Discord", 0.8, false, true));
        assert_eq!(g[2].name, "System sounds");
    }

    #[test]
    fn pretend_mixer() {
        reset_fake();
        let m = get(true);
        assert_eq!(m.calls[0].name, "Discord");
        let m = set_app(true, "discord.exe", Some(2.0), Some(true)).unwrap();
        let d = m.apps.iter().find(|a| a.key == "discord.exe").unwrap();
        assert_eq!((d.level, d.muted), (1.0, true));
        assert!(set_mic(true, true).unwrap().mic.unwrap().muted);
        assert!(set_app(true, "nothing.exe", None, Some(true)).is_err());
        reset_fake();
    }
}
