//! Your iPhone's notifications on the PC, over Bluetooth, the way watches and
//! Windows' Phone Link get them: Apple's Notification Center Service (ANCS).
//! Pair the iPhone with Windows once (Settings → Bluetooth), pick it here,
//! and allow "Share System Notifications" on the iPhone. Nothing is
//! installed on the phone, and notifications stay on this PC (in memory
//! only; the last 60).
//!
//! The protocol: the iPhone announces each notification on the
//! Notification Source (8 bytes), OmniHub asks for its app, title and
//! message on the Control Point, and the answer arrives on the Data Source,
//! possibly split over several packets.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

pub const ANCS: &str = "7905F431-B5CE-4E99-A40F-4B1E122D00D0";
pub const NOTIFICATION_SOURCE: &str = "9FBF120D-6301-42D9-8C58-25E699A21DBD";
pub const CONTROL_POINT: &str = "69D1D8F3-45E1-49A8-9821-9BBDFDAAD9D9";
pub const DATA_SOURCE: &str = "22EAC6E9-24D6-4BB5-BE44-B36ACE7C7BFB";
const KEEP: usize = 60;

// ---------- protocol ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Added,
    Modified,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceEvent {
    pub kind: EventKind,
    /// Already on the phone when OmniHub connected.
    pub pre_existing: bool,
    pub silent: bool,
    pub category: u8,
    pub uid: u32,
}

/// One Notification Source packet.
pub fn parse_source(b: &[u8]) -> Option<SourceEvent> {
    if b.len() < 8 {
        return None;
    }
    let kind = match b[0] {
        0 => EventKind::Added,
        1 => EventKind::Modified,
        2 => EventKind::Removed,
        _ => return None,
    };
    Some(SourceEvent { kind, silent: b[1] & 1 != 0, pre_existing: b[1] & 4 != 0, category: b[2], uid: u32::from_le_bytes([b[4], b[5], b[6], b[7]]) })
}

/// "Get Notification Attributes": app, title, message, date.
pub fn attrs_command(uid: u32) -> Vec<u8> {
    let mut v = vec![0u8];
    v.extend_from_slice(&uid.to_le_bytes());
    v.push(0); // app identifier
    v.push(1); // title (max length follows)
    v.extend_from_slice(&96u16.to_le_bytes());
    v.push(3); // message
    v.extend_from_slice(&400u16.to_le_bytes());
    v.push(5); // date
    v
}
const ATTRS_ASKED: usize = 4;

/// "Get App Attributes": the app's display name.
pub fn app_command(app: &str) -> Vec<u8> {
    let mut v = vec![1u8];
    v.extend_from_slice(app.as_bytes());
    v.push(0);
    v.push(0);
    v
}

/// "Perform Notification Action": 0 positive (open / answer), 1 negative (dismiss).
pub fn action_command(uid: u32, negative: bool) -> Vec<u8> {
    let mut v = vec![2u8];
    v.extend_from_slice(&uid.to_le_bytes());
    v.push(u8::from(negative));
    v
}

#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    Notification { uid: u32, app: String, title: String, message: String, date: String },
    App { app: String, name: String },
}

/// Joins Data Source packets into whole answers.
#[derive(Default)]
pub struct DataParser {
    buf: Vec<u8>,
}

impl DataParser {
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Answer> {
        self.buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        while let Some((answer, used)) = Self::one(&self.buf) {
            self.buf.drain(..used);
            if let Some(a) = answer {
                out.push(a);
            }
        }
        if self.buf.len() > 8192 {
            // Lost track (a dropped packet): start over with the next answer.
            self.buf.clear();
        }
        out
    }

    /// One whole answer at the start of `b` (None while it's still arriving).
    fn one(b: &[u8]) -> Option<(Option<Answer>, usize)> {
        let attrs = |mut i: usize, n: usize| -> Option<(HashMap<u8, String>, usize)> {
            let mut m = HashMap::new();
            for _ in 0..n {
                if b.len() < i + 3 {
                    return None;
                }
                let id = b[i];
                let len = u16::from_le_bytes([b[i + 1], b[i + 2]]) as usize;
                if b.len() < i + 3 + len {
                    return None;
                }
                m.insert(id, String::from_utf8_lossy(&b[i + 3..i + 3 + len]).into_owned());
                i += 3 + len;
            }
            Some((m, i))
        };
        match *b.first()? {
            0 => {
                if b.len() < 5 {
                    return None;
                }
                let uid = u32::from_le_bytes([b[1], b[2], b[3], b[4]]);
                let (mut m, used) = attrs(5, ATTRS_ASKED)?;
                let mut take = |k: u8| m.remove(&k).unwrap_or_default();
                Some((Some(Answer::Notification { uid, app: take(0), title: take(1), message: take(3), date: take(5) }), used))
            }
            1 => {
                let end = b[1..].iter().position(|&c| c == 0)? + 1;
                let app = String::from_utf8_lossy(&b[1..end]).into_owned();
                let (mut m, used) = attrs(end + 1, 1)?;
                Some((Some(Answer::App { app, name: m.remove(&0).unwrap_or_default() }), used))
            }
            // Not ours: skip a byte and look again.
            _ => Some((None, 1)),
        }
    }
}

/// Names for common apps (others are asked from the phone).
pub fn known_app(id: &str) -> Option<&'static str> {
    Some(match id {
        "com.apple.MobileSMS" => "Messages",
        "com.apple.mobilephone" => "Phone",
        "com.apple.mobilemail" => "Mail",
        "com.apple.mobilecal" => "Calendar",
        "com.apple.reminders" => "Reminders",
        "com.apple.facetime" => "FaceTime",
        "com.apple.Passbook" => "Wallet",
        "com.apple.Health" => "Health",
        "com.apple.findmy" => "Find My",
        "net.whatsapp.WhatsApp" => "WhatsApp",
        "com.burbn.instagram" => "Instagram",
        "com.atebits.Tweetie2" => "X",
        "com.zhiliaoapp.musically" => "TikTok",
        "com.toyopagroup.picaboo" => "Snapchat",
        "com.hammerandchisel.discord" => "Discord",
        "ph.telegra.Telegraph" => "Telegram",
        "com.facebook.Messenger" => "Messenger",
        "com.facebook.Facebook" => "Facebook",
        "com.google.Gmail" => "Gmail",
        "com.google.ios.youtube" => "YouTube",
        "com.spotify.client" => "Spotify",
        "com.tinyspeck.chatlyio" => "Slack",
        "com.microsoft.skype.teams" => "Teams",
        "com.microsoft.Office.Outlook" => "Outlook",
        "com.netflix.Netflix" => "Netflix",
        "com.reddit.Reddit" => "Reddit",
        _ => return None,
    })
}

/// ANCS dates are "yyyyMMdd'T'HHmmSS" in the phone's local time.
pub fn parse_date(s: &str) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%S").ok()
}

pub fn category_name(c: u8) -> &'static str {
    match c {
        1 => "call",
        2 => "missed call",
        3 => "voicemail",
        4 => "social",
        5 => "schedule",
        6 => "email",
        7 => "news",
        8 => "health",
        9 => "business",
        10 => "location",
        11 => "entertainment",
        _ => "other",
    }
}

// ---------- the store ----------

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IphoneNotif {
    pub uid: u32,
    pub app: String,
    pub app_name: String,
    pub title: String,
    pub message: String,
    /// Unix ms (when the phone got it, or when OmniHub heard of it).
    pub at: i64,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct IphonePrefs {
    pub enabled: bool,
    /// The paired iPhone's Windows device id.
    pub device_id: String,
    pub device_name: String,
    /// Show a Windows notification for each one.
    pub toasts: bool,
}

impl Default for IphonePrefs {
    fn default() -> Self {
        IphonePrefs { enabled: false, device_id: String::new(), device_name: String::new(), toasts: true }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct IphoneStatus {
    /// "off", "connecting", "connected", "waiting", "unsupported".
    pub state: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PairedPhone {
    pub id: String,
    pub name: String,
}

pub type Emit = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;

pub struct IphoneHub {
    file: PathBuf,
    prefs: Mutex<IphonePrefs>,
    status: Mutex<IphoneStatus>,
    notifs: Mutex<VecDeque<IphoneNotif>>,
    apps: Mutex<HashMap<String, String>>,
    stop: Mutex<Option<Arc<AtomicBool>>>,
    /// Actions waiting for the Bluetooth thread: (uid, negative).
    actions: Mutex<Vec<(u32, bool)>>,
}

impl IphoneHub {
    pub fn new(data: &Path) -> IphoneHub {
        let file = data.join("iphone.json");
        let prefs = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        IphoneHub { file, prefs: Mutex::new(prefs), status: Mutex::new(IphoneStatus { state: "off".into(), message: String::new() }), notifs: Mutex::new(VecDeque::new()), apps: Mutex::new(HashMap::new()), stop: Mutex::new(None), actions: Mutex::new(Vec::new()) }
    }

    pub fn prefs(&self) -> IphonePrefs {
        self.prefs.lock().clone()
    }

    pub fn status(&self) -> IphoneStatus {
        self.status.lock().clone()
    }

    pub fn list(&self) -> Vec<IphoneNotif> {
        self.notifs.lock().iter().cloned().collect()
    }

    fn save(&self, p: &IphonePrefs) -> Result<(), String> {
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    pub fn set_status(&self, state: &str, message: impl Into<String>, emit: &Emit) {
        let s = IphoneStatus { state: state.into(), message: message.into() };
        let changed = *self.status.lock() != s;
        *self.status.lock() = s.clone();
        if changed {
            emit("iphone:status", serde_json::to_value(s).unwrap_or_default());
        }
    }

    /// Remember and announce a notification.
    pub fn add(&self, n: IphoneNotif, announce: bool, emit: &Emit) {
        let mut q = self.notifs.lock();
        q.retain(|x| x.uid != n.uid);
        q.push_front(n.clone());
        q.truncate(KEEP);
        drop(q);
        emit("iphone:notification", serde_json::json!({ "notif": n, "announce": announce && self.prefs.lock().toasts }));
    }

    pub fn remove(&self, uid: u32, emit: &Emit) {
        let mut q = self.notifs.lock();
        let before = q.len();
        q.retain(|x| x.uid != uid);
        if q.len() != before {
            drop(q);
            emit("iphone:removed", serde_json::json!({ "uid": uid }));
        }
    }

    pub fn clear(&self) {
        self.notifs.lock().clear();
    }

    pub fn app_name(&self, id: &str) -> Option<String> {
        known_app(id).map(str::to_string).or_else(|| self.apps.lock().get(id).cloned())
    }

    pub fn learn_app(&self, id: String, name: String) {
        if !name.is_empty() {
            self.apps.lock().insert(id, name);
        }
    }

    /// Dismiss on the phone too (the iPhone's "negative action").
    pub fn dismiss(&self, uid: u32) {
        self.actions.lock().push((uid, true));
    }

    pub fn take_actions(&self) -> Vec<(u32, bool)> {
        std::mem::take(&mut *self.actions.lock())
    }

    /// Connect to `device` (a paired iPhone) and keep listening, or stop with None.
    pub fn connect(self: &Arc<Self>, device: Option<(String, String)>, emit: Emit) -> Result<IphonePrefs, String> {
        if let Some(s) = self.stop.lock().take() {
            s.store(true, Ordering::Relaxed);
        }
        let mut p = self.prefs();
        match device {
            Some((id, name)) => {
                p.enabled = true;
                p.device_id = id;
                p.device_name = name;
            }
            None => p.enabled = false,
        }
        self.save(&p)?;
        *self.prefs.lock() = p.clone();
        if p.enabled {
            self.start(emit);
        } else {
            self.set_status("off", "", &emit);
        }
        Ok(p)
    }

    pub fn set_toasts(&self, on: bool) -> Result<IphonePrefs, String> {
        let mut p = self.prefs();
        p.toasts = on;
        self.save(&p)?;
        *self.prefs.lock() = p.clone();
        Ok(p)
    }

    /// Start listening if it was on (at launch).
    pub fn start(self: &Arc<Self>, emit: Emit) {
        let p = self.prefs();
        if !p.enabled || p.device_id.is_empty() {
            return;
        }
        let stop = Arc::new(AtomicBool::new(false));
        if let Some(old) = self.stop.lock().replace(stop.clone()) {
            old.store(true, Ordering::Relaxed);
        }
        imp::run(self.clone(), p.device_id, stop, emit);
    }
}

pub fn paired_phones() -> Vec<PairedPhone> {
    imp::paired()
}

/// Turn one answered notification into what's shown.
pub fn to_notif(hub: &IphoneHub, uid: u32, app: String, title: String, message: String, date: &str, category: u8) -> IphoneNotif {
    let at = parse_date(date).and_then(|d| d.and_local_timezone(chrono::Local).single()).map(|d| d.timestamp_millis()).unwrap_or_else(|| chrono::Utc::now().timestamp_millis());
    let app_name = hub.app_name(&app).unwrap_or_else(|| app.rsplit('.').next().unwrap_or(&app).to_string());
    IphoneNotif { uid, app, app_name, title, message, at, category: category_name(category).into() }
}

#[cfg(windows)]
mod imp {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc};
    use std::time::{Duration, Instant};

    use windows::core::{GUID, HSTRING};
    use windows::Devices::Bluetooth::GenericAttributeProfile::{GattCharacteristic, GattClientCharacteristicConfigurationDescriptorValue, GattCommunicationStatus, GattValueChangedEventArgs};
    use windows::Devices::Bluetooth::{BluetoothCacheMode, BluetoothConnectionStatus, BluetoothLEDevice};
    use windows::Devices::Enumeration::DeviceInformation;
    use windows::Foundation::TypedEventHandler;
    use windows::Storage::Streams::{DataReader, DataWriter, IBuffer};
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    use super::{action_command, app_command, attrs_command, parse_source, to_notif, Answer, DataParser, Emit, EventKind, IphoneHub, PairedPhone, ANCS, CONTROL_POINT, DATA_SOURCE, NOTIFICATION_SOURCE};

    fn guid(s: &str) -> GUID {
        GUID::try_from(s).unwrap_or_default()
    }

    pub fn paired() -> Vec<PairedPhone> {
        let run = || -> windows::core::Result<Vec<PairedPhone>> {
            let sel = BluetoothLEDevice::GetDeviceSelectorFromPairingState(true)?;
            let all = DeviceInformation::FindAllAsyncAqsFilter(&sel)?.join()?;
            let mut out = Vec::new();
            for i in 0..all.Size()? {
                let d = all.GetAt(i)?;
                out.push(PairedPhone { id: d.Id()?.to_string(), name: d.Name()?.to_string() });
            }
            Ok(out)
        };
        let mut v = run().unwrap_or_default();
        // iPhones first.
        v.sort_by_key(|p| !p.name.to_lowercase().contains("iphone"));
        v
    }

    fn bytes(buf: &IBuffer) -> windows::core::Result<Vec<u8>> {
        let r = DataReader::FromBuffer(buf)?;
        let mut v = vec![0u8; r.UnconsumedBufferLength()? as usize];
        r.ReadBytes(&mut v)?;
        Ok(v)
    }

    fn write(cp: &GattCharacteristic, data: &[u8]) -> windows::core::Result<bool> {
        let w = DataWriter::new()?;
        w.WriteBytes(data)?;
        let r = cp.WriteValueWithResultAsync(&w.DetachBuffer()?)?.join()?;
        Ok(r.Status()? == GattCommunicationStatus::Success)
    }

    fn characteristic(svc: &windows::Devices::Bluetooth::GenericAttributeProfile::GattDeviceService, id: &str) -> windows::core::Result<Option<GattCharacteristic>> {
        let r = svc.GetCharacteristicsForUuidAsync(guid(id))?.join()?;
        let c = r.Characteristics()?;
        Ok(if c.Size()? > 0 { Some(c.GetAt(0)?) } else { None })
    }

    enum Msg {
        Source(Vec<u8>),
        Data(Vec<u8>),
    }

    pub fn run(hub: Arc<IphoneHub>, device: String, stop: Arc<AtomicBool>, emit: Emit) {
        let _ = std::thread::Builder::new().name("iphone-ancs".into()).spawn(move || {
            unsafe {
                let _ = RoInitialize(RO_INIT_MULTITHREADED);
            }
            while !stop.load(Ordering::Relaxed) {
                hub.set_status("connecting", "Looking for your iPhone…", &emit);
                match session(&hub, &device, &stop, &emit) {
                    Ok(()) => {}
                    Err(e) => hub.set_status("waiting", e, &emit),
                }
                // Try again in a little while (the phone may be away or locked).
                for _ in 0..30 {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
            hub.set_status("off", "", &emit);
        });
    }

    /// One connection, until it drops or OmniHub stops.
    fn session(hub: &IphoneHub, device: &str, stop: &AtomicBool, emit: &Emit) -> Result<(), String> {
        let w = |e: windows::core::Error| format!("Bluetooth: {}", e.message());
        let dev = BluetoothLEDevice::FromIdAsync(&HSTRING::from(device)).map_err(w)?.join().map_err(|_| "The iPhone isn't paired with this PC any more. Pair it again in Windows Settings → Bluetooth.".to_string())?;
        let svcs = dev.GetGattServicesForUuidWithCacheModeAsync(guid(ANCS), BluetoothCacheMode::Uncached).map_err(w)?.join().map_err(w)?;
        let list = svcs.Services().map_err(w)?;
        if svcs.Status().map_err(w)? != GattCommunicationStatus::Success || list.Size().map_err(w)? == 0 {
            return Err("Can't reach the iPhone's notifications. Keep it nearby and unlocked; on the iPhone open Settings → Bluetooth → ⓘ next to this PC and turn on Share System Notifications.".into());
        }
        let svc = list.GetAt(0).map_err(w)?;
        if let Ok(s) = svc.Session() {
            let _ = s.SetMaintainConnection(true);
        }
        let (Some(ns), Some(cp), Some(ds)) = (characteristic(&svc, NOTIFICATION_SOURCE).map_err(w)?, characteristic(&svc, CONTROL_POINT).map_err(w)?, characteristic(&svc, DATA_SOURCE).map_err(w)?) else {
            return Err("This iPhone doesn't offer notifications over Bluetooth.".into());
        };
        let (tx, rx) = mpsc::channel::<Msg>();
        let txd = tx.clone();
        let tok_d = ds
            .ValueChanged(&TypedEventHandler::<GattCharacteristic, GattValueChangedEventArgs>::new(move |_, a| {
                if let Ok(a) = a.ok() {
                    let _ = txd.send(Msg::Data(bytes(&a.CharacteristicValue()?)?));
                }
                Ok(())
            }))
            .map_err(w)?;
        let tok_n = ns
            .ValueChanged(&TypedEventHandler::<GattCharacteristic, GattValueChangedEventArgs>::new(move |_, a| {
                if let Ok(a) = a.ok() {
                    let _ = tx.send(Msg::Source(bytes(&a.CharacteristicValue()?)?));
                }
                Ok(())
            }))
            .map_err(w)?;
        // Data Source first, so no answer is missed.
        for c in [&ds, &ns] {
            let st = c.WriteClientCharacteristicConfigurationDescriptorAsync(GattClientCharacteristicConfigurationDescriptorValue::Notify).map_err(w)?.join().map_err(w)?;
            if st != GattCommunicationStatus::Success {
                let _ = ds.RemoveValueChanged(tok_d);
                let _ = ns.RemoveValueChanged(tok_n);
                return Err("Allow notifications on the iPhone: Settings → Bluetooth → ⓘ next to this PC → Share System Notifications.".into());
            }
        }
        let name = dev.Name().map(|n| n.to_string()).unwrap_or_else(|_| "your iPhone".into());
        hub.set_status("connected", format!("Connected to {name}"), emit);
        let mut parser = DataParser::default();
        // Notifications waiting to be asked about (uid, category, announce), one at a time.
        let mut queue: VecDeque<(u32, u8, bool)> = VecDeque::new();
        let mut asking: Option<((u32, u8, bool), Instant)> = None;
        let mut app_asking: Option<(String, Instant)> = None;
        let mut waiting_app: Vec<(super::IphoneNotif, bool)> = Vec::new();
        let result = loop {
            if stop.load(Ordering::Relaxed) {
                break Ok(());
            }
            if dev.ConnectionStatus().map(|s| s == BluetoothConnectionStatus::Disconnected).unwrap_or(true) {
                break Err("The iPhone went out of range. OmniHub reconnects when it's back.".to_string());
            }
            for (uid, neg) in hub.take_actions() {
                let _ = write(&cp, &action_command(uid, neg));
            }
            match rx.recv_timeout(Duration::from_millis(400)) {
                Ok(Msg::Source(b)) => {
                    if let Some(ev) = parse_source(&b) {
                        match ev.kind {
                            EventKind::Removed => hub.remove(ev.uid, emit),
                            EventKind::Added | EventKind::Modified => queue.push_back((ev.uid, ev.category, !ev.pre_existing && !ev.silent && ev.kind == EventKind::Added)),
                        }
                    }
                }
                Ok(Msg::Data(b)) => {
                    for a in parser.feed(&b) {
                        match a {
                            Answer::Notification { uid, app, title, message, date } => {
                                let (cat, announce) = asking.take().map(|((_, c, an), _)| (c, an)).unwrap_or((0, false));
                                if hub.app_name(&app).is_none() && app_asking.is_none() && !app.is_empty() {
                                    let _ = write(&cp, &app_command(&app));
                                    app_asking = Some((app.clone(), Instant::now()));
                                    waiting_app.push((to_notif(hub, uid, app, title, message, &date, cat), announce));
                                } else {
                                    hub.add(to_notif(hub, uid, app, title, message, &date, cat), announce, emit);
                                }
                            }
                            Answer::App { app, name } => {
                                hub.learn_app(app, name);
                                app_asking = None;
                                for (mut n, announce) in waiting_app.drain(..) {
                                    if let Some(a) = hub.app_name(&n.app) {
                                        n.app_name = a;
                                    }
                                    hub.add(n, announce, emit);
                                }
                            }
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break Err("Bluetooth stopped.".to_string()),
            }
            // An unanswered question: move on after a few seconds.
            if asking.as_ref().is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(4)) {
                asking = None;
            }
            if app_asking.as_ref().is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(4)) {
                app_asking = None;
                for (n, announce) in waiting_app.drain(..) {
                    hub.add(n, announce, emit);
                }
            }
            if asking.is_none() && app_asking.is_none() {
                if let Some(next) = queue.pop_front() {
                    if write(&cp, &attrs_command(next.0)).unwrap_or(false) {
                        asking = Some((next, Instant::now()));
                    }
                }
            }
        };
        let _ = ds.RemoveValueChanged(tok_d);
        let _ = ns.RemoveValueChanged(tok_n);
        result
    }
}

#[cfg(not(windows))]
mod imp {
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    use super::{Emit, IphoneHub, PairedPhone};

    pub fn paired() -> Vec<PairedPhone> {
        Vec::new()
    }

    pub fn run(hub: Arc<IphoneHub>, _device: String, _stop: Arc<AtomicBool>, emit: Emit) {
        hub.set_status("unsupported", "iPhone notifications need Windows and Bluetooth.", &emit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_three_packets() {
        // Added, not pre-existing, social, uid 0x01020304.
        let ev = parse_source(&[0, 0, 4, 1, 4, 3, 2, 1]).unwrap();
        assert_eq!((ev.kind, ev.pre_existing, ev.category, ev.uid), (EventKind::Added, false, 4, 0x0102_0304));
        assert!(parse_source(&[2, 4, 0, 0, 1, 0, 0, 0]).unwrap().pre_existing);
        assert!(parse_source(&[0, 0, 0]).is_none());
        assert_eq!(attrs_command(7)[..5], [0, 7, 0, 0, 0]);
        assert_eq!(action_command(9, true), vec![2, 9, 0, 0, 0, 1]);
        assert_eq!(app_command("a.b"), vec![1, b'a', b'.', b'b', 0, 0]);

        // A notification answer split over three packets, then an app answer.
        let mut a = vec![0u8, 7, 0, 0, 0];
        for (id, v) in [(0u8, "net.whatsapp.WhatsApp"), (1, "Mum"), (3, "Dinner's ready 🍝"), (5, "20261109T183000")] {
            a.push(id);
            a.extend_from_slice(&(v.len() as u16).to_le_bytes());
            a.extend_from_slice(v.as_bytes());
        }
        let mut p = DataParser::default();
        assert!(p.feed(&a[..6]).is_empty());
        assert!(p.feed(&a[6..20]).is_empty());
        let got = p.feed(&a[20..]);
        assert_eq!(got, vec![Answer::Notification { uid: 7, app: "net.whatsapp.WhatsApp".into(), title: "Mum".into(), message: "Dinner's ready 🍝".into(), date: "20261109T183000".into() }]);
        let mut b = vec![1u8];
        b.extend_from_slice(b"com.example.app\0");
        b.extend_from_slice(&[0, 7, 0]);
        b.extend_from_slice(b"Example");
        assert_eq!(p.feed(&b), vec![Answer::App { app: "com.example.app".into(), name: "Example".into() }]);
        assert!(parse_date("20261109T183000").is_some());
        assert_eq!(known_app("net.whatsapp.WhatsApp"), Some("WhatsApp"));
    }

    #[test]
    fn keeps_the_latest_and_removes() {
        let t = tempfile::tempdir().unwrap();
        let hub = IphoneHub::new(t.path());
        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let s2 = seen.clone();
        let emit: Emit = Arc::new(move |topic, _| s2.lock().push(topic.to_string()));
        for uid in 0..70 {
            hub.add(to_notif(&hub, uid, "com.apple.MobileSMS".into(), "Sam".into(), format!("hi {uid}"), "", 4), true, &emit);
        }
        let l = hub.list();
        assert_eq!(l.len(), KEEP);
        assert_eq!(l[0].uid, 69);
        assert_eq!(l[0].app_name, "Messages");
        hub.remove(69, &emit);
        assert_eq!(hub.list()[0].uid, 68);
        assert!(seen.lock().contains(&"iphone:removed".to_string()));
        hub.dismiss(5);
        assert_eq!(hub.take_actions(), vec![(5, true)]);
        assert!(hub.take_actions().is_empty());
    }
}
