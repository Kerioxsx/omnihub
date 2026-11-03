//! Quick actions: the clipboard history, the quick-launch wheel and the
//! macro deck (a grid of buttons on the PC and the phone).
//!
//! Clipboard history lives in memory; only pinned items are written, and
//! then encrypted for this Windows user. Anything a password manager marks
//! as private, anything OmniHub copied from the vault, and anything copied
//! from an ignored app is never kept.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use base64::Engine;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;

/// What a wheel slot or deck button does.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum QuickAction {
    /// An app, file, folder or web address.
    Open { target: String },
    /// A game from the library (its key, like `steam:730`).
    Game { key: String },
    /// A key or chord ("Ctrl+Shift+M", "F13").
    Keys { keys: String },
    /// Types this text.
    Text { text: String },
    /// playPause, next, previous, volumeUp, volumeDown, mute.
    Media { what: String },
    /// Mutes or unmutes every microphone.
    Mic,
    /// Adds one to a counter, or logs one of its events.
    Counter {
        id: String,
        #[serde(default)]
        event: Option<String>,
    },
    /// A screenshot of the whole screen.
    Screenshot,
    /// Locks the PC.
    Lock,
    /// Opens the clipboard history (on the PC only).
    Clipboard,
    /// Quiet mode on or off: screens off and fans at their lowest.
    Quiet,
    /// Capri Desktop (capriai.app) open or closed.
    CapriDesktop,
    /// OBS Studio: "scene", "stream", "record", "replay" or "mute" (with
    /// the scene or source name as `target`).
    Obs {
        what: String,
        #[serde(default)]
        target: Option<String>,
    },
}

impl QuickAction {
    pub fn check(&self) -> Result<(), String> {
        match self {
            QuickAction::Open { target } if target.trim().is_empty() => Err("Pick what to open".into()),
            QuickAction::Game { key } if key.is_empty() => Err("Pick a game".into()),
            QuickAction::Keys { keys } if !keys_ok(keys) => Err(format!("\"{keys}\" isn't a key OmniHub can press (try Ctrl+Shift+M or F13)")),
            QuickAction::Text { text } if text.is_empty() || text.chars().count() > 2000 => Err("Type up to 2000 characters".into()),
            QuickAction::Media { what } if !MEDIA.iter().any(|m| m.0 == what) => Err("Pick a media key".into()),
            QuickAction::Counter { id, .. } if id.is_empty() => Err("Pick a counter".into()),
            QuickAction::Obs { what, .. } if !crate::obs::WHATS.iter().any(|w| w.0 == what) => Err("Pick what OBS should do".into()),
            QuickAction::Obs { what, target } if (what == "scene" || what == "mute") && target.as_deref().is_none_or(|t| t.trim().is_empty()) => Err(if what == "scene" { "Pick the scene".into() } else { "Pick the source".into() }),
            _ => Ok(()),
        }
    }
}

/// Media actions and the key each one presses.
pub const MEDIA: &[(&str, &str)] = &[("playPause", "mediaplaypause"), ("next", "medianext"), ("previous", "mediaprev"), ("volumeUp", "volumeup"), ("volumeDown", "volumedown"), ("mute", "volumemute")];

fn keys_ok(spec: &str) -> bool {
    let parts: Vec<&str> = spec.split('+').map(str::trim).collect();
    !parts.is_empty() && parts.iter().all(|k| crate::capture::stream::input::vk_for(k).is_some())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct QuickButton {
    pub id: String,
    pub label: String,
    /// A Lucide icon name ("gamepad-2").
    pub icon: String,
    /// "#rrggbb" or empty.
    pub color: String,
    pub action: Option<QuickAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct QuickState {
    pub clip_on: bool,
    pub clip_hotkey: String,
    /// Program names whose copies are never kept ("keepass.exe").
    pub clip_ignore: Vec<String>,
    pub clip_max: usize,
    pub wheel_on: bool,
    pub wheel_hotkey: String,
    pub wheel: Vec<QuickButton>,
    pub deck: Vec<QuickButton>,
    pub deck_columns: u8,
    /// OBS's WebSocket server (its password is kept apart, encrypted).
    pub obs_url: String,
}

pub const MAX_WHEEL: usize = 8;
pub const MAX_DECK: usize = 30;

impl Default for QuickState {
    fn default() -> Self {
        QuickState {
            clip_on: true,
            clip_hotkey: "Alt+Shift+V".into(),
            clip_ignore: ["keepass.exe", "keepassxc.exe", "1password.exe", "bitwarden.exe", "lastpass.exe", "dashlane.exe", "enpass.exe", "proton pass.exe"].map(String::from).to_vec(),
            clip_max: 200,
            wheel_on: true,
            wheel_hotkey: "Alt+Shift+Q".into(),
            wheel: vec![
                button("w1", "Screenshot", "camera", "#38bdf8", QuickAction::Screenshot),
                button("w2", "Play / pause", "play", "#a78bfa", QuickAction::Media { what: "playPause".into() }),
                button("w3", "Clipboard", "clipboard-list", "#34d399", QuickAction::Clipboard),
                button("w4", "Microphone", "mic-off", "#f87171", QuickAction::Mic),
                button("w5", "Lock", "lock", "#fbbf24", QuickAction::Lock),
            ],
            deck: vec![
                button("d1", "Play / pause", "play", "#a78bfa", QuickAction::Media { what: "playPause".into() }),
                button("d2", "Previous", "skip-back", "#a78bfa", QuickAction::Media { what: "previous".into() }),
                button("d3", "Next", "skip-forward", "#a78bfa", QuickAction::Media { what: "next".into() }),
                button("d4", "Microphone", "mic-off", "#f87171", QuickAction::Mic),
                button("d5", "Volume down", "volume-1", "#60a5fa", QuickAction::Media { what: "volumeDown".into() }),
                button("d6", "Volume up", "volume-2", "#60a5fa", QuickAction::Media { what: "volumeUp".into() }),
                button("d7", "Screenshot", "camera", "#38bdf8", QuickAction::Screenshot),
                button("d8", "Task Manager", "activity", "#34d399", QuickAction::Keys { keys: "Ctrl+Shift+Esc".into() }),
                button("d9", "Lock", "lock", "#fbbf24", QuickAction::Lock),
            ],
            deck_columns: 3,
            obs_url: crate::obs::DEFAULT_URL.into(),
        }
    }
}

fn button(id: &str, label: &str, icon: &str, color: &str, action: QuickAction) -> QuickButton {
    QuickButton { id: id.into(), label: label.into(), icon: icon.into(), color: color.into(), action: Some(action) }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClipItem {
    pub id: String,
    pub at: i64,
    /// The text, or empty for a picture.
    pub text: String,
    /// A small PNG preview (data URL) for pictures.
    #[serde(default)]
    pub thumb: Option<String>,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub pinned: bool,
    /// The program it was copied from, when Windows says.
    #[serde(default)]
    pub app: Option<String>,
}

/// A picture's full pixels, kept beside its item.
struct Pixels {
    id: String,
    rgba: Vec<u8>,
}

const MAX_TEXT: usize = 100_000;
const MAX_PICTURES: usize = 25;
const ENTROPY: &[u8] = b"omnihub-clipboard";

pub struct Quick {
    file: PathBuf,
    pinned_file: PathBuf,
    state: Mutex<QuickState>,
    items: Mutex<VecDeque<ClipItem>>,
    pixels: Mutex<VecDeque<Pixels>>,
    paused: AtomicBool,
    events: EventBus,
}

impl Quick {
    pub fn new(data: &Path, events: EventBus) -> Quick {
        let file = data.join("quick.json");
        let pinned_file = data.join("clipboard-pinned.bin");
        let state = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let items: VecDeque<ClipItem> = read_pinned(&pinned_file).into();
        Quick { file, pinned_file, state: Mutex::new(state), items: Mutex::new(items), pixels: Mutex::new(VecDeque::new()), paused: AtomicBool::new(false), events }
    }

    pub fn state(&self) -> QuickState {
        self.state.lock().clone()
    }

    pub fn save(&self, mut next: QuickState) -> Result<QuickState, String> {
        if next.wheel.len() > MAX_WHEEL {
            return Err(format!("The wheel has room for {MAX_WHEEL}"));
        }
        if next.deck.len() > MAX_DECK {
            return Err(format!("The deck has room for {MAX_DECK} buttons"));
        }
        for b in next.wheel.iter_mut().chain(next.deck.iter_mut()) {
            if b.label.trim().is_empty() {
                return Err("Every button needs a name".into());
            }
            b.label = b.label.trim().chars().take(40).collect();
            if b.id.is_empty() {
                b.id = uuid::Uuid::new_v4().simple().to_string()[..10].to_string();
            }
            if let Some(a) = &b.action {
                a.check()?;
            }
        }
        if next.deck.iter().any(|b| b.action == Some(QuickAction::Clipboard)) {
            return Err("The clipboard history opens on the PC only, so it can't go on the deck".into());
        }
        if next.obs_url.trim().is_empty() {
            next.obs_url = crate::obs::DEFAULT_URL.into();
        }
        if !next.obs_url.trim().starts_with("ws://") {
            return Err("OBS's address starts with ws:// (like ws://127.0.0.1:4455)".into());
        }
        next.clip_max = next.clip_max.clamp(10, 1000);
        next.deck_columns = next.deck_columns.clamp(2, 6);
        next.clip_ignore = next.clip_ignore.iter().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).collect();
        let json = serde_json::to_vec_pretty(&next).map_err(|e| e.to_string())?;
        crate::settings::write_atomic(&self.file, &json).map_err(|e| e.to_string())?;
        *self.state.lock() = next.clone();
        self.events.emit("quick:changed", &next);
        Ok(next)
    }

    fn obs_password_file(&self) -> PathBuf {
        self.file.with_file_name("obs-password.bin")
    }

    /// OBS's WebSocket password, if one was saved.
    pub fn obs_password(&self) -> String {
        std::fs::read(self.obs_password_file()).ok().and_then(|b| unseal_with(&b, OBS_ENTROPY)).and_then(|p| String::from_utf8(p).ok()).unwrap_or_default()
    }

    pub fn has_obs_password(&self) -> bool {
        self.obs_password_file().is_file()
    }

    /// Save (or with "" forget) OBS's WebSocket password, encrypted.
    pub fn set_obs_password(&self, password: &str) -> Result<(), String> {
        let f = self.obs_password_file();
        if password.is_empty() {
            let _ = std::fs::remove_file(f);
            return Ok(());
        }
        crate::settings::write_atomic(&f, &seal_with(password.as_bytes(), OBS_ENTROPY)?).map_err(|e| e.to_string())
    }

    pub fn button(&self, id: &str) -> Option<QuickButton> {
        let s = self.state.lock();
        s.deck.iter().chain(s.wheel.iter()).find(|b| b.id == id).cloned()
    }

    // ---------- clipboard history ----------

    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, on: bool) {
        self.paused.store(on, Ordering::Relaxed);
        self.events.emit("clipboard:changed", ());
    }

    pub fn items(&self) -> Vec<ClipItem> {
        self.items.lock().iter().cloned().collect()
    }

    fn keeps(&self, app: Option<&str>) -> bool {
        let s = self.state.lock();
        s.clip_on && !self.paused() && !app.is_some_and(|a| s.clip_ignore.iter().any(|i| i.eq_ignore_ascii_case(a)))
    }

    /// Keep copied text (newest first; the same text again moves to the top).
    pub fn add_text(&self, text: &str, app: Option<&str>, now: i64) -> bool {
        if text.trim().is_empty() || !self.keeps(app) {
            return false;
        }
        let text: String = text.chars().take(MAX_TEXT).collect();
        let max = self.state.lock().clip_max;
        let mut items = self.items.lock();
        if items.front().is_some_and(|i| i.text == text) {
            return false;
        }
        let pinned = if let Some(at) = items.iter().position(|i| i.thumb.is_none() && i.text == text) { items.remove(at).is_some_and(|i| i.pinned) } else { false };
        items.push_front(ClipItem { id: new_id(), at: now, text, thumb: None, width: 0, height: 0, pinned, app: app.map(String::from) });
        trim(&mut items, max);
        drop(items);
        self.events.emit("clipboard:changed", ());
        true
    }

    /// Keep a copied picture (RGBA), with a small preview.
    pub fn add_image(&self, width: u32, height: u32, rgba: Vec<u8>, app: Option<&str>, now: i64) -> bool {
        if width == 0 || height == 0 || rgba.len() != (width * height * 4) as usize || !self.keeps(app) {
            return false;
        }
        let hash = blake3::hash(&rgba).to_hex()[..16].to_string();
        let mut items = self.items.lock();
        if items.front().is_some_and(|i| i.thumb.is_some() && i.id.ends_with(&hash)) {
            return false;
        }
        let Some(thumb) = thumb_of(width, height, &rgba) else { return false };
        let id = format!("{}-{hash}", new_id());
        items.push_front(ClipItem { id: id.clone(), at: now, text: String::new(), thumb: Some(thumb), width, height, pinned: false, app: app.map(String::from) });
        let max = self.state.lock().clip_max;
        trim(&mut items, max);
        let mut px = self.pixels.lock();
        px.push_front(Pixels { id, rgba });
        while px.len() > MAX_PICTURES {
            px.pop_back();
        }
        let keep: std::collections::HashSet<&str> = px.iter().map(|p| p.id.as_str()).collect();
        // A picture whose pixels were let go can't be pasted again.
        items.retain(|i| i.thumb.is_none() || i.pinned || keep.contains(i.id.as_str()));
        drop(items);
        self.events.emit("clipboard:changed", ());
        true
    }

    /// What to put back on the clipboard for an item.
    pub fn content(&self, id: &str) -> Result<ClipContent, String> {
        let items = self.items.lock();
        let item = items.iter().find(|i| i.id == id).ok_or("That item isn't in the history any more")?;
        if item.thumb.is_none() {
            return Ok(ClipContent::Text(item.text.clone()));
        }
        if let Some(p) = self.pixels.lock().iter().find(|p| p.id == id) {
            return Ok(ClipContent::Image { width: item.width as usize, height: item.height as usize, rgba: p.rgba.clone() });
        }
        Err("That picture is only kept as a preview now".into())
    }

    pub fn pin(&self, id: &str, on: bool) -> Result<(), String> {
        {
            let mut items = self.items.lock();
            let item = items.iter_mut().find(|i| i.id == id).ok_or("That item isn't in the history any more")?;
            if on && item.thumb.is_some() {
                return Err("Only text can be pinned".into());
            }
            item.pinned = on;
        }
        self.write_pinned()?;
        self.events.emit("clipboard:changed", ());
        Ok(())
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let was_pinned = {
            let mut items = self.items.lock();
            let at = items.iter().position(|i| i.id == id).ok_or("That item isn't in the history any more")?;
            items.remove(at).is_some_and(|i| i.pinned)
        };
        self.pixels.lock().retain(|p| p.id != id);
        if was_pinned {
            self.write_pinned()?;
        }
        self.events.emit("clipboard:changed", ());
        Ok(())
    }

    /// Forget everything but the pinned items.
    pub fn clear(&self) {
        self.items.lock().retain(|i| i.pinned);
        self.pixels.lock().clear();
        self.events.emit("clipboard:changed", ());
    }

    fn write_pinned(&self) -> Result<(), String> {
        let pinned: Vec<ClipItem> = self.items.lock().iter().filter(|i| i.pinned).cloned().collect();
        if pinned.is_empty() {
            let _ = std::fs::remove_file(&self.pinned_file);
            return Ok(());
        }
        let json = serde_json::to_vec(&pinned).map_err(|e| e.to_string())?;
        crate::settings::write_atomic(&self.pinned_file, &seal(&json)?).map_err(|e| e.to_string())
    }
}

pub enum ClipContent {
    Text(String),
    Image { width: usize, height: usize, rgba: Vec<u8> },
}

fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..12].to_string()
}

/// Keep at most `max`, dropping the oldest unpinned first.
fn trim(items: &mut VecDeque<ClipItem>, max: usize) {
    while items.len() > max {
        match items.iter().rposition(|i| !i.pinned) {
            Some(at) => {
                items.remove(at);
            }
            None => break,
        }
    }
}

fn thumb_of(width: u32, height: u32, rgba: &[u8]) -> Option<String> {
    let img = image::RgbaImage::from_raw(width, height, rgba.to_vec())?;
    let small = image::DynamicImage::ImageRgba8(img).thumbnail(240, 160);
    let mut png = Vec::new();
    small.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).ok()?;
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)))
}

const OBS_ENTROPY: &[u8] = b"omnihub-obs";

fn seal(b: &[u8]) -> Result<Vec<u8>, String> {
    seal_with(b, ENTROPY)
}
fn unseal(b: &[u8]) -> Option<Vec<u8>> {
    unseal_with(b, ENTROPY)
}
#[cfg(windows)]
fn seal_with(b: &[u8], entropy: &[u8]) -> Result<Vec<u8>, String> {
    crate::system::dpapi::protect(b, entropy).map_err(|e| e.to_string())
}
#[cfg(windows)]
fn unseal_with(b: &[u8], entropy: &[u8]) -> Option<Vec<u8>> {
    crate::system::dpapi::unprotect(b, entropy).ok().map(|z| z.to_vec())
}
// Only Windows builds ship; elsewhere (tests, the dev preview) these are plain.
#[cfg(not(windows))]
fn seal_with(b: &[u8], _entropy: &[u8]) -> Result<Vec<u8>, String> {
    Ok(b.to_vec())
}
#[cfg(not(windows))]
fn unseal_with(b: &[u8], _entropy: &[u8]) -> Option<Vec<u8>> {
    Some(b.to_vec())
}

fn read_pinned(path: &Path) -> Vec<ClipItem> {
    std::fs::read(path).ok().and_then(|b| unseal(&b)).and_then(|j| serde_json::from_slice(&j).ok()).unwrap_or_default()
}

/// Watch the clipboard and keep what's copied (runs for the app's life).
pub fn watch(quick: std::sync::Arc<Quick>) {
    std::thread::Builder::new()
        .name("clipboard-history".into())
        .spawn(move || {
            let mut last = imp::sequence();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(350));
                let now = imp::sequence();
                if now == last {
                    continue;
                }
                last = now;
                if !quick.state.lock().clip_on || quick.paused() || imp::private() {
                    continue;
                }
                let app = imp::owner();
                let at = crate::db::now() * 1000;
                if let Some(t) = crate::system::clipboard::get_text() {
                    quick.add_text(&t, app.as_deref(), at);
                } else if let Some((w, h, rgba)) = imp::image() {
                    quick.add_image(w, h, rgba, app.as_deref(), at);
                }
            }
        })
        .ok();
}

#[cfg(windows)]
mod imp {
    use windows::core::w;
    use windows::Win32::Foundation::{CloseHandle, HGLOBAL};
    use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, GetClipboardOwner, GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW};
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    use windows::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION};
    use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

    pub fn sequence() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    /// Whether the copier asked for it to stay out of clipboard history
    /// (the formats Microsoft documents for password managers).
    pub fn private() -> bool {
        unsafe {
            let exclude = RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing"));
            if exclude != 0 && IsClipboardFormatAvailable(exclude).is_ok() {
                return true;
            }
            let history = RegisterClipboardFormatW(w!("CanIncludeInClipboardHistory"));
            if history == 0 || IsClipboardFormatAvailable(history).is_err() {
                return false;
            }
            if OpenClipboard(None).is_err() {
                // Busy: err on the side of not keeping it.
                return true;
            }
            let mut no = true;
            if let Ok(h) = GetClipboardData(history) {
                let p = GlobalLock(HGLOBAL(h.0)) as *const u32;
                if !p.is_null() {
                    no = *p == 0;
                    let _ = GlobalUnlock(HGLOBAL(h.0));
                }
            }
            let _ = CloseClipboard();
            no
        }
    }

    /// The program that put this on the clipboard ("chrome.exe").
    pub fn owner() -> Option<String> {
        unsafe {
            let hwnd = GetClipboardOwner().ok()?;
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 {
                return None;
            }
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = [0u16; 520];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
            let _ = CloseHandle(h);
            if !ok {
                return None;
            }
            let path = String::from_utf16_lossy(&buf[..len as usize]);
            std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().to_lowercase())
        }
    }

    pub fn image() -> Option<(u32, u32, Vec<u8>)> {
        let img = arboard::Clipboard::new().ok()?.get_image().ok()?;
        // Skip huge pictures (over 64 MB of pixels).
        if img.bytes.len() > 64 << 20 {
            return None;
        }
        Some((img.width as u32, img.height as u32, img.bytes.into_owned()))
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn sequence() -> u32 {
        // No change counter here: look at the text itself.
        crate::system::clipboard::get_text().map_or(0, |t| blake3::hash(t.as_bytes()).as_bytes()[0] as u32 | ((t.len() as u32) << 8))
    }
    pub fn private() -> bool {
        false
    }
    pub fn owner() -> Option<String> {
        None
    }
    pub fn image() -> Option<(u32, u32, Vec<u8>)> {
        None
    }
}

impl crate::core::AppCore {
    /// Do a wheel slot's or deck button's action; says what was done.
    pub fn run_quick(&self, action: &QuickAction) -> Result<String, String> {
        use crate::capture::stream::input;
        match action {
            QuickAction::Open { target } => {
                let t = target.trim();
                if t.starts_with("http://") || t.starts_with("https://") {
                    crate::system::shell::open_url(t).map_err(|e| e.to_string())?;
                } else {
                    let p = Path::new(t);
                    if !p.exists() {
                        return Err(format!("{t} isn't there any more"));
                    }
                    crate::system::shell::open_path(p).map_err(|e| e.to_string())?;
                }
                Ok("Opened".into())
            }
            QuickAction::Game { key } => self.library_play(key).map(|_| "Starting the game".into()),
            QuickAction::Keys { keys } => {
                if input::chord(keys) {
                    Ok(format!("Pressed {keys}"))
                } else {
                    Err(format!("Couldn't press {keys}"))
                }
            }
            QuickAction::Text { text } => {
                input::text(text);
                Ok("Typed".into())
            }
            QuickAction::Media { what } => {
                let key = MEDIA.iter().find(|m| m.0 == what).map(|m| m.1).ok_or("Unknown media key")?;
                input::chord(key);
                Ok("Done".into())
            }
            QuickAction::Mic => {
                let fake = self.media.is_fake();
                let muted = crate::media::mixer::get(fake).mic.is_some_and(|m| m.muted);
                crate::media::mixer::set_mic(fake, !muted)?;
                Ok(if muted { "Microphone on".into() } else { "Microphone muted".into() })
            }
            QuickAction::Counter { id, event } => {
                let c = match event.as_deref().filter(|e| !e.is_empty()) {
                    Some(e) => self.counters.event(id, e)?,
                    None => self.counters.bump(id, 1)?,
                };
                Ok(format!("{}: {}", c.name, c.count))
            }
            QuickAction::Screenshot => {
                let format = self.settings.get().screenshots.format;
                let shot = self.screenshots.capture(crate::capture::screenshots::CaptureKind::Screen, &self.screenshot_dir(), format).map_err(|e| e.to_string())?;
                Ok(format!("Saved {}", Path::new(&shot.path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()))
            }
            QuickAction::Lock => {
                crate::system::power::execute(crate::system::power::PowerAction::Lock).map_err(|e| e.to_string())?;
                Ok("Locked".into())
            }
            QuickAction::Clipboard => Err("The clipboard history opens on the PC".into()),
            QuickAction::Obs { what, target } => crate::obs::run(&self.quick.state().obs_url, &self.quick.obs_password(), what, target.as_deref()),
            QuickAction::CapriDesktop => {
                if self.procs.pids_named(crate::capri_app::PROCESS).is_empty() {
                    self.capri_app.open()?;
                    Ok("Capri is open".into())
                } else {
                    self.procs.end(crate::capri_app::PROCESS)?;
                    Ok("Capri is closed".into())
                }
            }
            QuickAction::Quiet => {
                if self.quiet_status().on {
                    self.quiet_off()?;
                    Ok("Quiet mode off".into())
                } else {
                    self.quiet_on()?;
                    Ok("Screens off, fans at their lowest".into())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_keeps_dedups_pins_and_trims() {
        let t = tempfile::tempdir().unwrap();
        let q = Quick::new(t.path(), EventBus::new());
        assert!(q.add_text("hello", Some("chrome.exe"), 1));
        assert!(!q.add_text("hello", None, 2), "the same text twice in a row is kept once");
        assert!(q.add_text("world", None, 3));
        assert!(q.add_text("hello", None, 4), "copying older text again moves it up");
        let items = q.items();
        assert_eq!(items.iter().map(|i| i.text.as_str()).collect::<Vec<_>>(), vec!["hello", "world"]);
        assert!(!q.add_text("secret", Some("KeePassXC.exe"), 5), "ignored apps are never kept");
        assert!(!q.add_text("   ", None, 5));

        let first = q.items()[1].id.clone();
        q.pin(&first, true).unwrap();
        let mut s = q.state();
        s.clip_max = 10;
        q.save(s).unwrap();
        for n in 0..20 {
            q.add_text(&format!("item {n}"), None, 10 + n);
        }
        let items = q.items();
        assert_eq!(items.len(), 10);
        assert!(items.iter().any(|i| i.text == "world" && i.pinned), "pinned items survive trimming");

        // Pinned items come back after a restart; the rest doesn't.
        let q2 = Quick::new(t.path(), EventBus::new());
        assert_eq!(q2.items().iter().map(|i| i.text.as_str()).collect::<Vec<_>>(), vec!["world"]);
        q2.clear();
        assert_eq!(q2.items().len(), 1, "clearing keeps pinned items");

        q.set_paused(true);
        assert!(!q.add_text("while paused", None, 99));
    }

    #[test]
    fn pictures_get_a_preview_and_paste_back() {
        let t = tempfile::tempdir().unwrap();
        let q = Quick::new(t.path(), EventBus::new());
        let rgba = vec![200u8; 4 * 4 * 4];
        assert!(q.add_image(4, 4, rgba.clone(), None, 1));
        assert!(!q.add_image(4, 4, rgba.clone(), None, 2), "the same picture twice is kept once");
        let item = q.items()[0].clone();
        assert!(item.thumb.as_deref().unwrap().starts_with("data:image/png;base64,"));
        match q.content(&item.id).unwrap() {
            ClipContent::Image { width, height, rgba: back } => assert_eq!((width, height, back), (4, 4, rgba)),
            ClipContent::Text(_) => panic!("expected a picture"),
        }
        assert!(q.pin(&item.id, true).is_err());
        assert!(!q.add_image(4, 4, vec![0; 3], None, 3), "wrong sizes are refused");
    }

    #[test]
    fn buttons_are_checked() {
        let t = tempfile::tempdir().unwrap();
        let q = Quick::new(t.path(), EventBus::new());
        let mut s = q.state();
        assert_eq!(s.wheel.len(), 5);
        s.deck.push(button("", "Discord mute", "mic", "", QuickAction::Keys { keys: "Ctrl+Shift+M".into() }));
        let saved = q.save(s.clone()).unwrap();
        assert!(!saved.deck.last().unwrap().id.is_empty(), "new buttons get an id");
        let mut bad = s.clone();
        bad.deck.push(button("x", "Bad", "", "", QuickAction::Keys { keys: "Ctrl+Banana".into() }));
        assert!(q.save(bad).is_err());
        let mut bad = s.clone();
        bad.deck.push(button("y", "Clip", "", "", QuickAction::Clipboard));
        assert!(q.save(bad).is_err(), "the clipboard can't go on the phone's deck");
        let mut bad = s;
        bad.wheel = (0..9).map(|n| button(&n.to_string(), "x", "", "", QuickAction::Lock)).collect();
        assert!(q.save(bad).is_err());
        assert!(q.button("d1").is_some());
    }
}
