//! Govee lights. Every light with *LAN Control* on is found on the local
//! network and driven directly: on/off, brightness, a colour or a white,
//! and live colour streams (the screen, the music with the album cover's
//! colours and its beats, and the PC lighting effects) up to 30 times a
//! second, eased so changes glide. With the user's Govee API key, the
//! cloud API adds each light's own scenes, music modes and switches
//! (gradient, night light, and a speaker's volume where a light offers
//! one). Timers and a sleep timer switch them on, off, to a preset or a
//! brightness.
//!
//! One worker thread runs while OmniHub controls the lights, a timer is
//! set or the page is open, and sleeps between frames; screen capture and
//! listening to the PC's sound run only while a light shows them.

pub mod cloud;
pub mod lan;
pub mod look;
pub mod schedule;

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::events::EventBus;
use crate::rig::lighting::Effect;
use crate::rig::openrgb::Rgb;
use crate::rig::LinkState;
use cloud::{Choice, CloudDevice, Extra};
use lan::{Lan, LanDevice, LanStatus, Ports, Reply};
use look::{MusicRun, MusicStyle, Smooth, Sound, Zone};
use schedule::{Timer, TimerAction};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    /// One colour.
    #[default]
    Color,
    /// A white at a colour temperature.
    White,
    /// One of the lighting effects (moving colours).
    Effect,
    /// The screen's colours, live.
    Screen,
    /// The music: the album cover's colours with its beats.
    Music,
    /// The light's own scene (cloud).
    Scene,
    /// The light's own music mode, with its microphone (cloud).
    DeviceMusic,
}

impl Mode {
    /// Colours streamed from the PC.
    pub fn live(self) -> bool {
        matches!(self, Mode::Effect | Mode::Screen | Mode::Music)
    }
}

/// What lights show.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Look {
    pub mode: Mode,
    /// Color: "#rrggbb".
    pub color: String,
    /// White: 2000–9000 K.
    pub kelvin: u32,
    /// Effect: which, its colours and speed (0.2–3).
    pub effect: Effect,
    pub colors: Vec<String>,
    pub speed: f32,
    /// Screen: the monitor (its Windows name; empty: the main one).
    pub monitor: String,
    /// Screen: 0 (instant) – 100 (very smooth).
    pub smoothness: u8,
    /// Screen: 100 = as on screen, up to 200 = twice as vivid.
    pub saturation: u8,
    /// Screen: black bars and dark menus don't dim the lights.
    pub skip_dark: bool,
    /// Screen: 0 = as bright as on screen, 100 = dim colours lifted to full.
    pub boost: u8,
    /// Music: how it moves.
    pub style: MusicStyle,
    /// Music: the album cover's colours (else `colors`).
    pub album_colors: bool,
    /// Music: 0–100.
    pub sensitivity: u8,
    /// Scene: its name (each light's own scene of that name).
    pub scene: String,
    /// DeviceMusic: the light's music mode by name, and its sensitivity.
    pub device_music: String,
}

impl Default for Look {
    fn default() -> Self {
        Look {
            mode: Mode::Color,
            color: "#00a2ff".into(),
            kelvin: 4000,
            effect: Effect::Wave,
            colors: vec!["#7c3aed".into(), "#00d4ff".into(), "#ff2bd6".into()],
            speed: 1.0,
            monitor: String::new(),
            smoothness: 45,
            saturation: 130,
            skip_dark: true,
            boost: 40,
            style: MusicStyle::Pulse,
            album_colors: true,
            sensitivity: 60,
            scene: String::new(),
            device_music: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StreamKind {
    /// Segments when the light can stream them (Govee's account says so,
    /// or doesn't know the light), else one colour.
    #[default]
    Auto,
    /// A colour per segment (Govee's DreamView streaming).
    Segments,
    /// A colour per segment through Govee's own segment commands: slower,
    /// for lights that ignore the streaming (up to 15 segments).
    SegmentsLite,
    /// One colour for the whole light (for lights that don't stream).
    Single,
}

/// One light, as the user set it up.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct LightSettings {
    /// Govee's id for it.
    pub id: String,
    /// The user's name for it (empty: Govee's).
    pub name: String,
    /// OmniHub controls it.
    pub control: bool,
    /// It shows what all the lights show (else `look`).
    pub follow: bool,
    pub look: Look,
    /// Its own power and brightness (with `follow`, the shared ones count).
    pub on: bool,
    pub brightness: u8,
    /// Segments to stream to; 0: what Govee says (or 10).
    pub segments: u8,
    pub stream: StreamKind,
    /// Where it sits around the screen.
    pub zone: Zone,
    /// Its segments run the other way.
    pub reverse: bool,
    /// The light blends between segments.
    pub gradient: bool,
}

impl Default for LightSettings {
    fn default() -> Self {
        LightSettings { id: String::new(), name: String::new(), control: true, follow: true, look: Look::default(), on: true, brightness: 100, segments: 0, stream: StreamKind::Auto, zone: Zone::Whole, reverse: false, gradient: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub look: Look,
    /// Also set the brightness.
    pub brightness: Option<u8>,
}

impl Default for Preset {
    fn default() -> Self {
        Preset { id: String::new(), name: "My preset".into(), look: Look::default(), brightness: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct GoveeSettings {
    /// OmniHub controls the lights.
    pub enabled: bool,
    /// All lights on (each also has its own switch).
    pub on: bool,
    pub brightness: u8,
    /// What the lights that follow show.
    pub look: Look,
    pub lights: Vec<LightSettings>,
    /// The user's presets (OmniHub's are in `built_in_presets`).
    pub presets: Vec<Preset>,
    /// The preset last applied.
    pub preset: String,
    pub timers: Vec<Timer>,
    /// The Govee API key, encrypted for this Windows account (DPAPI), base64.
    pub api_secret: String,
    /// Which one-off fixes to saved settings have run.
    #[serde(default)]
    pub revision: u8,
    /// Addresses to ask directly (lights the scan doesn't reach).
    pub manual: Vec<String>,
}

impl Default for GoveeSettings {
    fn default() -> Self {
        GoveeSettings { enabled: false, on: true, brightness: 80, look: Look::default(), lights: Vec::new(), presets: Vec::new(), preset: String::new(), timers: Vec::new(), api_secret: String::new(), manual: Vec::new(), revision: 1 }
    }
}

impl GoveeSettings {
    pub fn light(&self, id: &str) -> LightSettings {
        self.lights.iter().find(|l| l.id == id).cloned().unwrap_or_else(|| LightSettings { id: id.to_string(), ..Default::default() })
    }
}

/// OmniHub's presets.
pub fn built_in_presets() -> Vec<Preset> {
    let p = |id: &str, name: &str, look: Look, brightness: Option<u8>| Preset { id: id.into(), name: name.into(), look, brightness };
    let d = Look::default();
    let colors = |c: &[&str]| c.iter().map(|s| s.to_string()).collect();
    vec![
        p("movie", "Movie", Look { mode: Mode::Screen, smoothness: 60, saturation: 130, skip_dark: true, ..d.clone() }, None),
        p("gaming", "Gaming", Look { mode: Mode::Screen, smoothness: 15, saturation: 155, skip_dark: true, ..d.clone() }, Some(100)),
        p("party", "Party", Look { mode: Mode::Music, style: MusicStyle::Pulse, album_colors: true, sensitivity: 70, ..d.clone() }, Some(100)),
        p("spectrum", "Spectrum", Look { mode: Mode::Music, style: MusicStyle::Spectrum, album_colors: true, sensitivity: 60, ..d.clone() }, None),
        p("flow", "Flow", Look { mode: Mode::Music, style: MusicStyle::Flow, album_colors: true, sensitivity: 55, ..d.clone() }, None),
        p("rainbow", "Rainbow", Look { mode: Mode::Effect, effect: Effect::Wave, speed: 1.0, ..d.clone() }, None),
        p("chill", "Chill", Look { mode: Mode::Effect, effect: Effect::Gradient, colors: colors(&["#ff4d6d", "#ffb703", "#7b2cbf"]), speed: 0.5, ..d.clone() }, Some(60)),
        p("ocean", "Ocean", Look { mode: Mode::Effect, effect: Effect::Breathing, colors: colors(&["#0077b6", "#00b4d8", "#90e0ef"]), speed: 0.6, ..d.clone() }, None),
        p("focus", "Focus", Look { mode: Mode::White, kelvin: 5600, ..d.clone() }, Some(100)),
        p("reading", "Reading", Look { mode: Mode::White, kelvin: 3200, ..d.clone() }, Some(80)),
        p("night", "Night light", Look { mode: Mode::Color, color: "#ff5a00".into(), ..d }, Some(12)),
    ]
}

/// A light as found (LAN, cloud or both) and as it is now.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GoveeLight {
    pub id: String,
    pub sku: String,
    pub name: String,
    pub ip: Option<String>,
    /// Answers on the local network (live colours work).
    pub lan: bool,
    /// Known to the cloud API (scenes, music modes, switches).
    pub cloud: bool,
    pub on: Option<bool>,
    pub brightness: Option<u8>,
    pub color: Option<String>,
    pub kelvin: Option<u32>,
    /// Segments Govee says it has.
    pub segments: Option<usize>,
    pub kelvin_range: Option<(u32, u32)>,
    pub scenes: Vec<Choice>,
    pub diy_scenes: Vec<Choice>,
    pub snapshots: Vec<Choice>,
    pub music_modes: Vec<Choice>,
    pub extras: Vec<Extra>,
    /// Current values of the cloud capabilities (instance → value).
    pub values: HashMap<String, Value>,
    /// OmniHub is streaming colours to it.
    pub streaming: bool,
    /// A colour per segment (else one colour).
    pub per_segment: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GoveeStatus {
    pub lan: LinkState,
    pub lan_message: Option<String>,
    pub cloud: LinkState,
    pub cloud_message: Option<String>,
    pub has_key: bool,
    pub lights: Vec<GoveeLight>,
    pub screen_message: Option<String>,
    pub music_message: Option<String>,
    /// The album cover's colours now (music).
    pub palette: Vec<String>,
    /// The sleep timer: lights off at this time (Unix ms).
    pub sleep_until: Option<i64>,
}

/// How OmniHub reaches the lights (tests use other ports on 127.0.0.1).
#[derive(Debug, Clone, Copy)]
pub struct LanConfig {
    pub addr: Ipv4Addr,
    pub ports: Ports,
    /// Scan from the PC's networks (off in tests).
    pub multicast: bool,
}

impl Default for LanConfig {
    fn default() -> Self {
        LanConfig { addr: Ipv4Addr::UNSPECIFIED, ports: Ports::default(), multicast: true }
    }
}

const LEASE: Duration = Duration::from_secs(8);
const SECRET_ENTROPY: &[u8] = b"omnihub-govee";

/// A brightness change spread over time (timers fading in or out).
#[derive(Debug, Clone)]
struct Fade {
    lights: Vec<String>,
    start: Instant,
    dur: Duration,
    from: f32,
    to: f32,
    /// Switch off at the end.
    off: bool,
}

impl Fade {
    fn factor(&self) -> f32 {
        let k = (self.start.elapsed().as_secs_f32() / self.dur.as_secs_f32().max(0.001)).clamp(0.0, 1.0);
        self.from + (self.to - self.from) * k
    }
    fn covers(&self, id: &str) -> bool {
        self.lights.is_empty() || self.lights.iter().any(|l| l == id)
    }
}

/// A light's scenes, DIY scenes and current capability values (cloud).
type CloudExtra = (Vec<Choice>, Vec<Choice>, HashMap<String, Value>);

enum CloudJob {
    Refresh,
    Control { id: String, cap_type: String, instance: String, value: Value },
}

struct Shared {
    settings: GoveeSettings,
    status: GoveeStatus,
    held_until: Option<Instant>,
    dirty: bool,
    rescan: bool,
    sleep_until: Option<(i64, Instant)>,
    fades: Vec<Fade>,
    identify: HashMap<String, Instant>,
    lan_devices: HashMap<String, (LanDevice, Instant)>,
    lan_state: HashMap<String, LanStatus>,
    cloud_devices: HashMap<String, CloudDevice>,
    cloud_extra: HashMap<String, CloudExtra>,
    frames: HashMap<String, Vec<Rgb>>,
    /// Per light: streaming now, and what's wrong (if anything).
    /// Per light: streaming, per segment, a message.
    flags: HashMap<String, (bool, bool, Option<String>)>,
    /// Something the page shows changed.
    status_dirty: bool,
}

pub struct GoveeHub {
    path: PathBuf,
    events: EventBus,
    visual: Arc<crate::media::visual::VisualHub>,
    media: Arc<crate::media::MediaHub>,
    lan_config: LanConfig,
    cloud_base: String,
    shared: Mutex<Shared>,
    wake: Condvar,
    running: AtomicBool,
    cloud_tx: Mutex<Option<Sender<CloudJob>>>,
}

fn norm_id(id: &str) -> String {
    id.trim().to_uppercase()
}

/// A white's colour, for previews.
pub fn kelvin_rgb(k: u32) -> Rgb {
    let t = k.clamp(1000, 40000) as f32 / 100.0;
    let r = if t <= 66.0 { 255.0 } else { 329.7 * (t - 60.0).powf(-0.1332) };
    let g = if t <= 66.0 { 99.47 * t.ln() - 161.12 } else { 288.12 * (t - 60.0).powf(-0.0755) };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.52 * (t - 10.0).ln() - 305.04
    };
    let q = |v: f32| v.clamp(0.0, 255.0).round() as u8;
    Rgb(q(r), q(g), q(b))
}

fn merge(a: &mut Value, b: &Value) {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                if v.is_null() {
                    a.remove(k);
                } else {
                    merge(a.entry(k.clone()).or_insert(Value::Null), v);
                }
            }
        }
        (a, b) => *a = b.clone(),
    }
}

/// What the lights show now, for the PC's own lighting to follow.
static SHOW: std::sync::Mutex<Option<(Vec<Rgb>, Instant)>> = std::sync::Mutex::new(None);

/// The colours the Govee lights show now (every light's, one after
/// another); empty when OmniHub hasn't driven them in the last two seconds.
pub fn show_colors() -> Vec<Rgb> {
    SHOW.lock().ok().and_then(|s| s.as_ref().filter(|(_, at)| at.elapsed() < Duration::from_secs(2)).map(|(c, _)| c.clone())).unwrap_or_default()
}

/// Models known to take DreamView (`razer`) streaming on the network.
/// Only the DreamView kits: other models (an H6076 floor lamp, for one)
/// ignore the stream and stay on their last colour.
const STREAMERS: &[&str] = &["H6199", "H6604"];

/// How a light's live colours travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    /// `razer` frames, a colour per segment, up to ~30 a second.
    Stream,
    /// Segment-colour commands for the segments that changed, ~8 a second.
    Commands,
    /// One colour for the whole light.
    Single,
}

/// Segments of models whose count Govee's account isn't needed for.
const KNOWN_SEGMENTS: &[(&str, usize)] = &[("H6076", 14)];

/// The segments a model is known to have.
fn known_segments(sku: Option<&str>) -> Option<usize> {
    let sku = sku?.trim();
    KNOWN_SEGMENTS.iter().find(|(k, _)| k.eq_ignore_ascii_case(sku)).map(|(_, n)| *n)
}

/// `Auto`: streaming when Govee's account says so (DreamView) or the model
/// is known to stream; segment commands when the light has segments (the
/// account or the model says how many, or they were set by hand); else one
/// colour, which every light takes.
fn route(kind: StreamKind, cloud: Option<&CloudDevice>, sku: Option<&str>, set_segments: u8) -> Route {
    match kind {
        StreamKind::Segments => Route::Stream,
        StreamKind::SegmentsLite => Route::Commands,
        StreamKind::Single => Route::Single,
        StreamKind::Auto => {
            let streams = match cloud {
                Some(d) => d.has("dreamViewToggle"),
                None => sku.is_some_and(|s| STREAMERS.iter().any(|k| k.eq_ignore_ascii_case(s.trim()))),
            };
            let segments = if set_segments > 0 { set_segments as usize } else { cloud.and_then(|d| d.segments()).or_else(|| known_segments(sku)).unwrap_or(0) };
            if streams {
                Route::Stream
            } else if segments > 1 {
                Route::Commands
            } else {
                Route::Single
            }
        }
    }
}

/// A colour rounded to 32 steps a channel: segment commands go out only
/// for changes that show.
fn coarse(c: Rgb) -> Rgb {
    let q = |v: u8| ((v as u16 * 31 + 127) / 255 * 255 / 31) as u8;
    Rgb(q(c.0), q(c.1), q(c.2))
}

fn sanitize_look(l: &mut Look) {
    if Rgb::parse(&l.color).is_none() {
        l.color = Look::default().color;
    }
    l.kelvin = l.kelvin.clamp(2000, 9000);
    l.colors.retain(|c| Rgb::parse(c).is_some());
    l.colors.truncate(8);
    l.speed = if l.speed.is_finite() { l.speed.clamp(0.2, 3.0) } else { 1.0 };
    l.smoothness = l.smoothness.min(100);
    l.saturation = l.saturation.clamp(50, 200);
    l.boost = l.boost.min(100);
    l.sensitivity = l.sensitivity.min(100);
    l.scene = l.scene.chars().take(80).collect();
    l.device_music = l.device_music.chars().take(40).collect();
    l.monitor = l.monitor.chars().take(200).collect();
}

fn sanitize(s: &mut GoveeSettings) {
    if s.revision < 1 {
        // 0.13.4: the brightness boost's first default (65) was too bright.
        for l in std::iter::once(&mut s.look).chain(s.lights.iter_mut().map(|l| &mut l.look)).chain(s.presets.iter_mut().map(|p| &mut p.look)) {
            if l.boost == 65 {
                l.boost = 40;
            }
        }
        s.revision = 1;
    }
    s.brightness = s.brightness.clamp(1, 100);
    sanitize_look(&mut s.look);
    s.lights.truncate(64);
    for l in &mut s.lights {
        l.id = norm_id(&l.id);
        l.name = l.name.trim().chars().take(40).collect();
        l.brightness = l.brightness.clamp(1, 100);
        l.segments = l.segments.min(lan::RAZER_MAX as u8);
        sanitize_look(&mut l.look);
    }
    s.lights.retain(|l| !l.id.is_empty());
    s.presets.truncate(50);
    for p in &mut s.presets {
        sanitize_look(&mut p.look);
        p.brightness = p.brightness.map(|b| b.clamp(1, 100));
    }
    s.timers.truncate(30);
    for t in &mut s.timers {
        schedule::sanitize(t);
    }
    s.manual.retain(|a| a.parse::<IpAddr>().is_ok());
    s.manual.truncate(32);
}

impl GoveeHub {
    pub fn new(data_dir: &Path, events: EventBus, visual: Arc<crate::media::visual::VisualHub>, media: Arc<crate::media::MediaHub>) -> Arc<GoveeHub> {
        GoveeHub::with(data_dir, events, visual, media, LanConfig::default(), cloud::BASE)
    }

    pub fn with(data_dir: &Path, events: EventBus, visual: Arc<crate::media::visual::VisualHub>, media: Arc<crate::media::MediaHub>, lan_config: LanConfig, cloud_base: &str) -> Arc<GoveeHub> {
        let path = data_dir.join("govee.json");
        let mut settings: GoveeSettings = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        sanitize(&mut settings);
        let has_key = !settings.api_secret.is_empty();
        Arc::new(GoveeHub {
            path,
            events,
            visual,
            media,
            lan_config,
            cloud_base: cloud_base.to_string(),
            shared: Mutex::new(Shared {
                settings,
                status: GoveeStatus { has_key, ..Default::default() },
                held_until: None,
                dirty: true,
                rescan: true,
                sleep_until: None,
                fades: Vec::new(),
                identify: HashMap::new(),
                lan_devices: HashMap::new(),
                lan_state: HashMap::new(),
                cloud_devices: HashMap::new(),
                cloud_extra: HashMap::new(),
                frames: HashMap::new(),
                flags: HashMap::new(),
                status_dirty: false,
            }),
            wake: Condvar::new(),
            running: AtomicBool::new(false),
            cloud_tx: Mutex::new(None),
        })
    }

    pub fn settings(&self) -> GoveeSettings {
        self.shared.lock().settings.clone()
    }

    pub fn status(&self) -> GoveeStatus {
        let mut sh = self.shared.lock();
        Self::build_status(&mut sh);
        sh.status.clone()
    }

    pub fn presets(&self) -> Vec<Preset> {
        let mut all = built_in_presets();
        all.extend(self.shared.lock().settings.presets.iter().cloned());
        all
    }

    /// Change settings with a JSON merge patch.
    pub fn update(self: &Arc<Self>, patch: &Value) -> Result<GoveeSettings, String> {
        let next = {
            let mut sh = self.shared.lock();
            let mut v = serde_json::to_value(&sh.settings).map_err(|e| e.to_string())?;
            // The key only changes through `set_key`.
            if let Some(o) = v.as_object_mut() {
                o.remove("apiSecret");
            }
            let mut patch = patch.clone();
            if let Some(o) = patch.as_object_mut() {
                o.remove("apiSecret");
            }
            merge(&mut v, &patch);
            let mut next: GoveeSettings = serde_json::from_value(v).map_err(|e| format!("Those settings don't fit: {e}"))?;
            next.api_secret = sh.settings.api_secret.clone();
            sanitize(&mut next);
            sh.settings = next.clone();
            sh.dirty = true;
            next
        };
        self.save(&next)?;
        self.events.emit("govee:settings", &next);
        self.kick();
        Ok(next)
    }

    /// Change one light's settings.
    pub fn update_light(self: &Arc<Self>, id: &str, patch: &Value) -> Result<GoveeSettings, String> {
        let id = norm_id(id);
        let s = self.settings();
        let mut lights = s.lights.clone();
        let i = match lights.iter().position(|l| l.id == id) {
            Some(i) => i,
            None => {
                lights.push(s.light(&id));
                lights.len() - 1
            }
        };
        let mut v = serde_json::to_value(&lights[i]).map_err(|e| e.to_string())?;
        merge(&mut v, patch);
        lights[i] = serde_json::from_value(v).map_err(|e| format!("Those settings don't fit: {e}"))?;
        lights[i].id = id;
        self.update(&json!({ "lights": lights }))
    }

    fn save(&self, s: &GoveeSettings) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        std::fs::write(&self.path, serde_json::to_vec_pretty(s).unwrap_or_default()).map_err(|e| e.to_string())
    }

    /// The Govee API key (kept encrypted for this account); empty removes it.
    pub fn set_key(self: &Arc<Self>, key: &str) -> Result<(), String> {
        use base64::Engine;
        let key = key.trim();
        if key.len() > 200 || key.chars().any(|c| c.is_control()) {
            return Err("That doesn't look like a Govee API key.".into());
        }
        let secret = if key.is_empty() { String::new() } else { base64::engine::general_purpose::STANDARD.encode(crate::system::dpapi::protect(key.as_bytes(), SECRET_ENTROPY).map_err(|e| e.to_string())?) };
        let next = {
            let mut sh = self.shared.lock();
            sh.settings.api_secret = secret;
            sh.status.has_key = !key.is_empty();
            if key.is_empty() {
                sh.cloud_devices.clear();
                sh.cloud_extra.clear();
                sh.status.cloud = LinkState::Off;
                sh.status.cloud_message = None;
            }
            sh.settings.clone()
        };
        self.save(&next)?;
        if !key.is_empty() {
            self.cloud_job(CloudJob::Refresh);
        }
        self.kick();
        Ok(())
    }

    fn key(&self) -> Option<zeroize::Zeroizing<String>> {
        use base64::Engine;
        let secret = self.shared.lock().settings.api_secret.clone();
        if secret.is_empty() {
            return None;
        }
        let bytes = base64::engine::general_purpose::STANDARD.decode(secret).ok()?;
        let plain = crate::system::dpapi::unprotect(&bytes, SECRET_ENTROPY).ok()?;
        Some(zeroize::Zeroizing::new(String::from_utf8_lossy(&plain).into_owned()))
    }

    /// The page is open (call every few seconds).
    pub fn hold(self: &Arc<Self>) -> GoveeStatus {
        self.shared.lock().held_until = Some(Instant::now() + LEASE);
        self.kick();
        self.status()
    }

    /// Look for lights again (and ask the cloud again).
    pub fn rescan(self: &Arc<Self>) {
        self.shared.lock().rescan = true;
        if self.shared.lock().status.has_key {
            self.cloud_job(CloudJob::Refresh);
        }
        self.kick();
    }

    /// Blink a light so you can tell which it is.
    pub fn identify(self: &Arc<Self>, id: &str) {
        self.shared.lock().identify.insert(norm_id(id), Instant::now() + Duration::from_millis(1800));
        self.kick();
    }

    /// Set one of a light's own settings through the cloud (scenes, music
    /// modes, gradient, night light, volume…).
    pub fn control(self: &Arc<Self>, id: &str, cap_type: &str, instance: &str, value: Value) -> Result<(), String> {
        let id = norm_id(id);
        if !self.shared.lock().cloud_devices.contains_key(&id) {
            return Err("That needs your Govee API key (and the light in your Govee account).".into());
        }
        if !cap_type.starts_with("devices.capabilities.") || instance.is_empty() || instance.len() > 60 {
            return Err("Unknown setting.".into());
        }
        self.cloud_job(CloudJob::Control { id, cap_type: cap_type.into(), instance: instance.into(), value });
        Ok(())
    }

    pub fn save_preset(self: &Arc<Self>, mut p: Preset) -> Result<Preset, String> {
        p.name = p.name.trim().chars().take(40).collect();
        if p.name.is_empty() {
            return Err("Give the preset a name.".into());
        }
        if p.id.is_empty() || built_in_presets().iter().any(|b| b.id == p.id) {
            p.id = format!("my-{}", uuid::Uuid::new_v4());
        }
        let mut presets = self.settings().presets;
        match presets.iter().position(|q| q.id == p.id) {
            Some(i) => presets[i] = p.clone(),
            None if presets.len() >= 50 => return Err("That's a lot of presets; remove one first.".into()),
            None => presets.push(p.clone()),
        }
        self.update(&json!({ "presets": presets }))?;
        Ok(p)
    }

    pub fn delete_preset(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let presets: Vec<Preset> = self.settings().presets.into_iter().filter(|p| p.id != id).collect();
        self.update(&json!({ "presets": presets })).map(|_| ())
    }

    /// Show a preset on all the lights that follow, or on `lights` only.
    pub fn apply_preset(self: &Arc<Self>, id: &str, lights: &[String]) -> Result<GoveeSettings, String> {
        let p = self.presets().into_iter().find(|p| p.id == id).ok_or("That preset no longer exists.")?;
        let mut s = self.settings();
        if lights.is_empty() {
            s.look = p.look.clone();
            if let Some(b) = p.brightness {
                s.brightness = b;
            }
            s.on = true;
        } else {
            for id in lights {
                let mut l = s.light(&norm_id(id));
                l.follow = false;
                l.look = p.look.clone();
                l.on = true;
                if let Some(b) = p.brightness {
                    l.brightness = b;
                }
                match s.lights.iter().position(|x| x.id == l.id) {
                    Some(i) => s.lights[i] = l,
                    None => s.lights.push(l),
                }
            }
        }
        self.update(&json!({ "look": s.look, "brightness": s.brightness, "on": s.on, "lights": s.lights, "preset": p.id, "enabled": true }))
    }

    pub fn save_timer(self: &Arc<Self>, mut t: Timer) -> Result<Timer, String> {
        schedule::sanitize(&mut t);
        if t.id.is_empty() {
            t.id = uuid::Uuid::new_v4().to_string();
        }
        let mut timers = self.settings().timers;
        match timers.iter().position(|x| x.id == t.id) {
            Some(i) => timers[i] = t.clone(),
            None if timers.len() >= 30 => return Err("That's a lot of timers; remove one first.".into()),
            None => timers.push(t.clone()),
        }
        self.update(&json!({ "timers": timers }))?;
        Ok(t)
    }

    pub fn delete_timer(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let timers: Vec<Timer> = self.settings().timers.into_iter().filter(|t| t.id != id).collect();
        self.update(&json!({ "timers": timers })).map(|_| ())
    }

    /// The sleep timer: the lights fade out over the last minute and go
    /// off in `minutes` (None cancels it).
    pub fn sleep(self: &Arc<Self>, minutes: Option<u32>) -> GoveeStatus {
        {
            let mut sh = self.shared.lock();
            sh.sleep_until = minutes.map(|m| {
                let d = Duration::from_secs(m.clamp(1, 600) as u64 * 60);
                (crate::media::now_ms() + d.as_millis() as i64, Instant::now() + d)
            });
            sh.dirty = true;
        }
        self.kick();
        self.status()
    }

    /// Start what the saved settings want (at app start).
    pub fn start(self: &Arc<Self>) {
        let s = self.settings();
        if !s.api_secret.is_empty() {
            self.cloud_job(CloudJob::Refresh);
        }
        if s.enabled || s.timers.iter().any(|t| t.enabled) {
            self.kick();
        }
    }

    /// Hand the lights back to their own modes (on quit).
    pub fn shutdown(&self) {
        let ips: Vec<IpAddr> = {
            let sh = self.shared.lock();
            sh.flags.iter().filter(|(_, f)| f.0).filter_map(|(id, _)| sh.lan_devices.get(id).map(|(d, _)| d.ip)).collect()
        };
        if ips.is_empty() {
            return;
        }
        if let Ok(lan) = Lan::open_on(self.lan_config.addr, Ports { listen: 0, ..self.lan_config.ports }) {
            for ip in ips {
                let _ = lan.send(ip, &lan::razer(&lan::razer_switch(false)));
            }
        }
    }

    fn kick(self: &Arc<Self>) {
        self.wake.notify_all();
        if !self.running.swap(true, Ordering::SeqCst) {
            let hub = self.clone();
            if std::thread::Builder::new().name("govee".into()).spawn(move || hub.run()).is_err() {
                self.running.store(false, Ordering::SeqCst);
            }
        }
    }

    fn cloud_job(self: &Arc<Self>, job: CloudJob) {
        let mut tx = self.cloud_tx.lock();
        let job = match tx.as_ref() {
            Some(t) => match t.send(job) {
                Ok(()) => return,
                // The worker stopped: start another.
                Err(std::sync::mpsc::SendError(j)) => j,
            },
            None => job,
        };
        self.spawn_cloud(&mut tx, Some(job));
    }

    fn spawn_cloud(self: &Arc<Self>, tx: &mut Option<Sender<CloudJob>>, first: Option<CloudJob>) {
        let (t, rx) = std::sync::mpsc::channel();
        if let Some(j) = first {
            let _ = t.send(j);
        }
        *tx = Some(t);
        let hub = self.clone();
        let _ = std::thread::Builder::new().name("govee-cloud".into()).spawn(move || hub.cloud_worker(rx));
    }

    /// Cloud calls, one at a time and spaced out (Govee's limits).
    fn cloud_worker(self: Arc<Self>, rx: Receiver<CloudJob>) {
        let mut queue: Vec<CloudJob> = Vec::new();
        loop {
            if queue.is_empty() {
                match rx.recv_timeout(Duration::from_secs(120)) {
                    Ok(j) => queue.push(j),
                    Err(_) => break,
                }
            }
            // A slider dragged queues many: only the last of each counts.
            while let Ok(j) = rx.try_recv() {
                let same = |a: &CloudJob, b: &CloudJob| match (a, b) {
                    (CloudJob::Refresh, CloudJob::Refresh) => true,
                    (CloudJob::Control { id: i1, instance: n1, .. }, CloudJob::Control { id: i2, instance: n2, .. }) => i1 == i2 && n1 == n2,
                    _ => false,
                };
                queue.retain(|q| !same(q, &j));
                queue.push(j);
            }
            let job = queue.remove(0);
            let Some(key) = self.key() else {
                self.set_cloud(LinkState::Off, None);
                continue;
            };
            let c = cloud::Cloud::with_base(&self.cloud_base, &key);
            match job {
                CloudJob::Refresh => match c.devices() {
                    Ok(list) => {
                        let mut extra = HashMap::new();
                        for d in list.iter().filter(|d| d.cap("lightScene").is_some() || d.cap("diyScene").is_some()) {
                            let scenes = if d.cap("lightScene").is_some() { c.scenes(d).unwrap_or_default() } else { Vec::new() };
                            std::thread::sleep(Duration::from_millis(150));
                            let diy = if d.cap("diyScene").is_some() { c.diy_scenes(d).unwrap_or_default() } else { Vec::new() };
                            std::thread::sleep(Duration::from_millis(150));
                            let values = c.state(d).map(|v| v.into_iter().collect()).unwrap_or_default();
                            extra.insert(norm_id(&d.device), (scenes, diy, values));
                        }
                        let mut sh = self.shared.lock();
                        sh.cloud_devices = list.into_iter().map(|d| (norm_id(&d.device), d)).collect();
                        sh.cloud_extra = extra;
                        sh.status.cloud = LinkState::On;
                        sh.status.cloud_message = None;
                        sh.dirty = true;
                        drop(sh);
                        self.emit_status();
                    }
                    Err(e) => self.set_cloud(LinkState::Error, Some(e)),
                },
                CloudJob::Control { id, cap_type, instance, value } => {
                    let d = self.shared.lock().cloud_devices.get(&id).cloned();
                    if let Some(d) = d {
                        match c.control(&d, &cap_type, &instance, value.clone()) {
                            Ok(()) => {
                                let mut sh = self.shared.lock();
                                sh.cloud_extra.entry(id).or_default().2.insert(instance, value);
                                drop(sh);
                                self.emit_status();
                            }
                            Err(e) => self.set_cloud(LinkState::Error, Some(e)),
                        }
                    }
                    // Govee allows about two a second per light.
                    std::thread::sleep(Duration::from_millis(550));
                }
            }
        }
        let mut tx = self.cloud_tx.lock();
        drop(rx);
        *tx = None;
    }

    fn set_cloud(&self, state: LinkState, message: Option<String>) {
        {
            let mut sh = self.shared.lock();
            sh.status.cloud = state;
            sh.status.cloud_message = message;
        }
        self.emit_status();
    }

    fn emit_status(&self) {
        let status = self.status();
        self.events.emit("govee:status", status);
    }

    /// Put together what's known about each light.
    fn build_status(sh: &mut Shared) {
        let mut ids: Vec<String> = sh.lan_devices.keys().chain(sh.cloud_devices.keys()).cloned().collect();
        ids.sort();
        ids.dedup();
        sh.status.sleep_until = sh.sleep_until.map(|(ms, _)| ms);
        sh.status.lights = ids
            .into_iter()
            .map(|id| {
                let lan = sh.lan_devices.get(&id).map(|(d, _)| d);
                let cd = sh.cloud_devices.get(&id);
                let st = sh.lan_state.get(&id);
                let extra = sh.cloud_extra.get(&id);
                let values = extra.map(|e| e.2.clone()).unwrap_or_default();
                let own = sh.settings.lights.iter().find(|l| l.id == id).map(|l| l.name.clone()).filter(|n| !n.is_empty());
                let sku = lan.map(|d| d.sku.clone()).or_else(|| cd.map(|d| d.sku.clone())).unwrap_or_default();
                GoveeLight {
                    name: own.or_else(|| cd.map(|d| d.name.clone()).filter(|n| !n.is_empty())).unwrap_or_else(|| if sku.is_empty() { "Govee light".into() } else { format!("Govee {sku}") }),
                    sku,
                    ip: lan.map(|d| d.ip.to_string()),
                    lan: lan.is_some(),
                    cloud: cd.is_some(),
                    on: st.map(|s| s.on).or_else(|| values.get("powerSwitch").and_then(|v| v.as_u64()).map(|v| v != 0)),
                    brightness: st.map(|s| s.brightness).or_else(|| values.get("brightness").and_then(|v| v.as_u64()).map(|v| v.min(100) as u8)),
                    color: st.filter(|s| s.kelvin == 0).map(|s| s.color.hex()),
                    kelvin: st.map(|s| s.kelvin).filter(|k| *k > 0),
                    segments: cd.and_then(|d| d.segments()),
                    kelvin_range: cd.and_then(|d| d.kelvin_range()),
                    scenes: extra.map(|e| e.0.clone()).unwrap_or_default(),
                    diy_scenes: extra.map(|e| e.1.clone()).filter(|v| !v.is_empty()).or_else(|| cd.map(|d| d.listed("diyScene"))).unwrap_or_default(),
                    snapshots: cd.map(|d| d.listed("snapshot")).unwrap_or_default(),
                    music_modes: cd.map(|d| d.music_modes()).unwrap_or_default(),
                    extras: cd.map(|d| d.extras()).unwrap_or_default(),
                    values,
                    streaming: sh.flags.get(&id).is_some_and(|f| f.0),
                    per_segment: sh.flags.get(&id).is_some_and(|f| f.1),
                    message: sh.flags.get(&id).and_then(|f| f.2.clone()),
                    id,
                }
            })
            .collect();
    }

    fn set_light_state(&self, id: &str, streaming: bool, per_segment: bool, message: Option<String>) {
        let mut sh = self.shared.lock();
        let next = (streaming, per_segment, message);
        if sh.flags.get(id) != Some(&next) {
            sh.flags.insert(id.to_string(), next);
            sh.status_dirty = true;
        }
    }

    fn run(self: Arc<Self>) {
        let mut w = Worker::default();
        let mut audio = self.events.subscribe();
        let mut last_clock = chrono::Local::now().naive_local();
        loop {
            let (s, held, rescan, sleep_until, fades, identify) = {
                let mut sh = self.shared.lock();
                sh.dirty = false;
                let rescan = std::mem::take(&mut sh.rescan);
                sh.fades.retain(|f| f.start.elapsed() < f.dur + Duration::from_secs(1));
                sh.identify.retain(|_, until| *until > Instant::now());
                (sh.settings.clone(), sh.held_until.is_some_and(|t| t > Instant::now()), rescan, sh.sleep_until, sh.fades.clone(), sh.identify.clone())
            };
            let timers = s.timers.iter().any(|t| t.enabled);
            if !s.enabled && !held && !timers && sleep_until.is_none() && identify.is_empty() {
                self.stop_streams(&mut w);
                break;
            }
            // The local network.
            if w.lan.is_none() && w.lan_retry.is_none_or(|t| Instant::now() >= t) {
                match Lan::open_on(self.lan_config.addr, self.lan_config.ports) {
                    Ok(l) => {
                        w.lan = Some(l);
                        w.next_scan = None;
                        let mut sh = self.shared.lock();
                        sh.status.lan = LinkState::On;
                        sh.status.lan_message = None;
                    }
                    Err(e) => {
                        w.lan_retry = Some(Instant::now() + Duration::from_secs(10));
                        let mut sh = self.shared.lock();
                        sh.status.lan = LinkState::Error;
                        sh.status.lan_message = Some(e.to_string());
                    }
                }
            }
            if let Some(l) = w.lan.as_ref() {
                if rescan || w.next_scan.is_none_or(|t| Instant::now() >= t) {
                    let ifaces = if self.lan_config.multicast { lan::interfaces() } else { Vec::new() };
                    let manual: Vec<IpAddr> = s.manual.iter().filter_map(|a| a.parse().ok()).collect();
                    // Twice: a lost packet costs a light a whole round.
                    l.scan(&ifaces, &manual);
                    l.scan(&ifaces, &manual);
                    if self.lan_config.multicast && (rescan || w.next_sweep.is_none_or(|t| Instant::now() >= t)) {
                        l.sweep(&lan::sweep_networks());
                        w.next_sweep = Some(Instant::now() + Duration::from_secs(300));
                    }
                    let found = !self.shared.lock().lan_devices.is_empty();
                    w.next_scan = Some(Instant::now() + if found { Duration::from_secs(60) } else { Duration::from_secs(10) });
                    w.status_due = Some(Instant::now() + Duration::from_millis(1500));
                }
                // Ask each light how it is now and then (and when the page is open).
                if w.next_status.is_none_or(|t| Instant::now() >= t) {
                    let ips: Vec<IpAddr> = self.shared.lock().lan_devices.values().map(|(d, _)| d.ip).collect();
                    for ip in ips {
                        let _ = l.send(ip, &lan::status());
                    }
                    w.next_status = Some(Instant::now() + if held { Duration::from_secs(5) } else { Duration::from_secs(30) });
                }
                let mut changed = false;
                while let Some(r) = l.recv(Duration::from_millis(1)) {
                    let mut sh = self.shared.lock();
                    match r {
                        Reply::Scan(d) => {
                            let id = norm_id(&d.device);
                            if sh.lan_devices.get(&id).is_none_or(|(old, _)| old != &d) {
                                changed = true;
                                let _ = l.send(d.ip, &lan::status());
                            }
                            sh.lan_devices.insert(id, (d, Instant::now()));
                        }
                        Reply::Status(ip, st) => {
                            if let Some(id) = sh.lan_devices.iter().find(|(_, (d, _))| d.ip == ip).map(|(k, _)| k.clone()) {
                                changed |= sh.lan_state.get(&id) != Some(&st);
                                sh.lan_state.insert(id, st);
                            }
                        }
                    }
                }
                if changed || w.status_due.is_some_and(|t| Instant::now() >= t) {
                    w.status_due = None;
                    self.shared.lock().status_dirty = true;
                }
            }

            // Timers, once a second.
            let now_local = chrono::Local::now().naive_local();
            if now_local.signed_duration_since(last_clock).num_milliseconds() >= 1000 || now_local < last_clock {
                for t in schedule::due(&s.timers, last_clock, now_local) {
                    self.fire(t);
                }
                last_clock = now_local;
            }
            if let Some((_, until)) = sleep_until {
                if Instant::now() >= until {
                    self.shared.lock().sleep_until = None;
                    let _ = self.update(&json!({ "on": false }));
                }
            }

            let frame = self.lights_tick(&s, held, sleep_until.map(|(_, u)| u), &fades, &identify, &mut w, &mut audio);
            let dirty = std::mem::take(&mut self.shared.lock().status_dirty);
            if dirty && w.status_at.is_none_or(|t| t.elapsed() > Duration::from_millis(250)) {
                w.status_at = Some(Instant::now());
                self.emit_status();
            } else if dirty {
                self.shared.lock().status_dirty = true;
            }
            if held && w.last_preview.is_none_or(|t| t.elapsed() > Duration::from_millis(80)) {
                w.last_preview = Some(Instant::now());
                let frames: HashMap<String, Vec<String>> = self.shared.lock().frames.iter().map(|(k, v)| (k.clone(), v.iter().map(|c| c.hex()).collect())).collect();
                if !frames.is_empty() {
                    self.events.emit("govee:frame", json!({ "lights": frames }));
                }
            }
            let mut sh = self.shared.lock();
            if !sh.dirty {
                self.wake.wait_for(&mut sh, frame);
            }
        }
        self.visual.release("govee");
        self.running.store(false, Ordering::SeqCst);
        let sh = self.shared.lock();
        let again = sh.settings.enabled || sh.held_until.is_some_and(|t| t > Instant::now()) || sh.settings.timers.iter().any(|t| t.enabled) || sh.sleep_until.is_some();
        drop(sh);
        if again {
            self.kick();
        }
    }

    fn fire(self: &Arc<Self>, t: &Timer) {
        let s = self.settings();
        let ids: Vec<String> = t.lights.iter().map(|i| norm_id(i)).collect();
        let fade = |from: f32, to: f32, off: bool| Fade { lights: ids.clone(), start: Instant::now(), dur: Duration::from_secs(t.fade as u64 * 60), from, to, off };
        let set_power = |on: bool| -> Value {
            if ids.is_empty() {
                json!({ "on": on, "enabled": true })
            } else {
                let mut lights = s.lights.clone();
                for id in &ids {
                    let mut l = s.light(id);
                    l.on = on;
                    match lights.iter().position(|x| x.id == l.id) {
                        Some(i) => lights[i] = l,
                        None => lights.push(l),
                    }
                }
                json!({ "lights": lights, "enabled": true })
            }
        };
        match t.action {
            TimerAction::On => {
                if t.fade > 0 {
                    self.shared.lock().fades.push(fade(0.02, 1.0, false));
                }
                let _ = self.update(&set_power(true));
            }
            TimerAction::Off => {
                if t.fade > 0 {
                    self.shared.lock().fades.push(fade(1.0, 0.02, true));
                    self.kick();
                } else {
                    let _ = self.update(&set_power(false));
                }
            }
            TimerAction::Preset => {
                let _ = self.apply_preset(&t.preset, &t.lights);
            }
            TimerAction::Brightness => {
                if ids.is_empty() {
                    let _ = self.update(&json!({ "brightness": t.brightness, "enabled": true }));
                } else {
                    for id in &ids {
                        let _ = self.update_light(id, &json!({ "brightness": t.brightness }));
                    }
                }
            }
        }
    }

    /// Hand every streaming light back to its own modes.
    fn stop_streams(&self, w: &mut Worker) {
        if let Some(lan) = w.lan.as_ref() {
            for r in w.runs.values_mut() {
                if r.razer_on {
                    if let Some(ip) = r.ip {
                        let _ = lan.send(ip, &lan::razer(&lan::razer_switch(false)));
                    }
                    r.razer_on = false;
                }
            }
        }
        for r in w.runs.values_mut() {
            r.applied = None;
            r.power = None;
            r.bright = None;
        }
        let mut sh = self.shared.lock();
        for f in sh.flags.values_mut() {
            f.0 = false;
        }
        sh.status_dirty = true;
        sh.frames.clear();
    }

    /// One frame for every light; returns how long until the next.
    #[allow(clippy::too_many_arguments)]
    fn lights_tick(self: &Arc<Self>, s: &GoveeSettings, held: bool, sleep_at: Option<Instant>, fades: &[Fade], identify: &HashMap<String, Instant>, w: &mut Worker, audio: &mut tokio::sync::broadcast::Receiver<crate::events::Event>) -> Duration {
        let now = Instant::now();
        let dt = w.last_tick.map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.033).clamp(0.001, 0.25);
        w.last_tick = Some(now);
        w.t += dt;
        let (lan_devices, cloud_devices, light_ids) = {
            let sh = self.shared.lock();
            let mut ids: Vec<String> = sh.lan_devices.keys().chain(sh.cloud_devices.keys()).cloned().collect();
            ids.sort();
            ids.dedup();
            (sh.lan_devices.clone(), sh.cloud_devices.clone(), ids)
        };
        if !s.enabled {
            if w.runs.values().any(|r| r.razer_on || r.applied.is_some()) {
                self.stop_streams(w);
            }
            // Only blinking (identify) while OmniHub doesn't control them.
            for id in identify.keys() {
                if let (Some(lan), Some((d, _))) = (w.lan.as_ref(), lan_devices.get(id)) {
                    let on = ((w.t * 4.0) as u32).is_multiple_of(2);
                    let _ = lan.send(d.ip, &lan::color(if on { Rgb(255, 255, 255) } else { Rgb(0, 0, 40) }));
                }
            }
            return if identify.is_empty() { Duration::from_millis(500) } else { Duration::from_millis(250) };
        }

        // What each light shows.
        let looks: Vec<(String, LightSettings, Look, bool, f32)> = light_ids
            .iter()
            .map(|id| {
                let l = s.light(id);
                let look = if l.follow { s.look.clone() } else { l.look.clone() };
                let on = l.control && l.on && (!l.follow || s.on);
                let mut b = if l.follow { s.brightness } else { l.brightness } as f32 / 100.0;
                for f in fades.iter().filter(|f| f.covers(id)) {
                    b *= f.factor();
                }
                if let Some(at) = sleep_at {
                    let left = at.saturating_duration_since(now).as_secs_f32();
                    if left < 60.0 {
                        b *= (left / 60.0).max(0.02);
                    }
                }
                (id.clone(), l, look, on, b)
            })
            .collect();
        // A fade-out that ended switches its lights off.
        let finished: Vec<Fade> = fades.iter().filter(|f| f.off && f.start.elapsed() >= f.dur).cloned().collect();
        if !finished.is_empty() {
            self.shared.lock().fades.retain(|f| !(f.off && f.start.elapsed() >= f.dur));
            for f in finished {
                if f.lights.is_empty() {
                    let _ = self.update_quiet(&json!({ "on": false }));
                } else {
                    for id in f.lights {
                        let mut l = s.light(&id);
                        l.on = false;
                        let mut lights = s.lights.clone();
                        match lights.iter().position(|x| x.id == l.id) {
                            Some(i) => lights[i] = l,
                            None => lights.push(l),
                        }
                        let _ = self.update_quiet(&json!({ "lights": lights }));
                    }
                }
            }
        }

        let need_music = looks.iter().any(|(_, l, look, on, _)| l.control && *on && look.mode == Mode::Music);
        let need_screen: Option<String> = looks.iter().find(|(_, l, look, on, _)| l.control && *on && look.mode == Mode::Screen).map(|(_, _, look, _, _)| look.monitor.clone());
        let any_live = looks.iter().any(|(_, l, look, on, _)| l.control && *on && look.mode.live());

        // The PC's sound, while a light dances to it.
        if need_music {
            if w.lease_at.is_none_or(|t| t.elapsed() > Duration::from_secs(2)) {
                self.visual.hold("govee");
                w.lease_at = Some(now);
            }
        } else if w.lease_at.take().is_some() {
            self.visual.release("govee");
            w.sound = Sound::default();
        }
        loop {
            match audio.try_recv() {
                Ok(e) if e.topic == "audio:frame" => w.sound = Sound::from_json(&e.payload),
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {}
                Err(_) => break,
            }
        }
        let mut music_message = None;
        let mut palette_now = Vec::new();
        if need_music {
            let st = self.visual.status();
            if st.state == crate::media::visual::CaptureState::Disabled {
                music_message = Some("Listening to the PC's sound is off: turn on Settings → Visuals → React to sound.".to_string());
            } else if st.state == crate::media::visual::CaptureState::Unavailable {
                music_message = st.error.clone();
            }
            // The album cover's colours, once per cover.
            let art = self.media.state().and_then(|m| m.art);
            if art != w.palette_for {
                w.palette = art.as_deref().and_then(|id| self.media.art(id)).map(|(b, _)| look::palette(&b)).unwrap_or_default();
                w.palette_for = art;
            }
            palette_now = w.palette.iter().map(|c| c.hex()).collect();
        }

        // The screen, while a light follows it.
        let mut screen_message = None;
        if let Some(monitor) = need_screen {
            if w.sampler.is_none() || w.sampler_for != monitor {
                w.sampler = None;
                if w.sampler_retry.is_none_or(|t| now >= t) {
                    let mons = crate::capture::stream::monitors();
                    let index = mons.iter().find(|m| !monitor.is_empty() && m.name == monitor).or_else(|| mons.iter().find(|m| m.primary)).map(|m| m.index).unwrap_or(0);
                    match crate::capture::sampler::open(index) {
                        Ok(sm) => {
                            w.sampler = Some(sm);
                            w.sampler_for = monitor;
                        }
                        Err(e) => {
                            w.sampler_retry = Some(now + Duration::from_secs(5));
                            screen_message = Some(format!("Can't see the screen: {e}"));
                        }
                    }
                }
            }
            if let Some(sm) = w.sampler.as_mut() {
                if let Err(e) = sm.next(Duration::ZERO) {
                    screen_message = Some(format!("Can't see the screen: {e}"));
                    w.sampler = None;
                    w.sampler_retry = Some(now + Duration::from_secs(2));
                }
            }
        } else {
            w.sampler = None;
        }

        let mut frames: HashMap<String, Vec<Rgb>> = HashMap::new();
        for (dev_index, (id, l, look, on, bright)) in looks.iter().enumerate() {
            let run = w.runs.entry(id.clone()).or_default();
            let lan_dev = lan_devices.get(id).map(|(d, _)| d);
            run.ip = lan_dev.map(|d| d.ip);
            let cloud_dev = cloud_devices.get(id);
            if !l.control {
                if run.razer_on {
                    if let (Some(lan), Some(ip)) = (w.lan.as_ref(), run.ip) {
                        let _ = lan.send(ip, &lan::razer(&lan::razer_switch(false)));
                    }
                    run.razer_on = false;
                }
                run.applied = None;
                run.power = None;
                run.bright = None;
                self.set_light_state(id, false, false, None);
                continue;
            }
            let send = |v: &Value| -> bool { matches!((w.lan.as_ref(), lan_dev), (Some(lan), Some(d)) if lan.send(d.ip, v).is_ok()) };
            // Power.
            if run.power != Some(*on) {
                let ok = if lan_dev.is_some() {
                    send(&lan::turn(*on))
                } else if cloud_dev.is_some() {
                    self.queue_control(id, "devices.capabilities.on_off", "powerSwitch", json!(*on as u8));
                    true
                } else {
                    false
                };
                if ok {
                    run.power = Some(*on);
                    if *on {
                        run.applied = None;
                        run.bright = None;
                    }
                }
            }
            if !on {
                if run.razer_on {
                    send(&lan::razer(&lan::razer_switch(false)));
                    run.razer_on = false;
                }
                self.set_light_state(id, false, false, None);
                continue;
            }
            // Identify: blink white for a moment.
            if identify.contains_key(id) && lan_dev.is_some() {
                let blink = ((w.t * 4.0) as u32).is_multiple_of(2);
                if run.razer_on {
                    send(&lan::razer(&lan::razer_switch(false)));
                    run.razer_on = false;
                }
                send(&lan::color(if blink { Rgb(255, 255, 255) } else { Rgb(0, 0, 40) }));
                run.applied = None;
                continue;
            }
            // Brightness (a slider dragged sends a few, not hundreds).
            let b = ((bright * 100.0).round() as u8).clamp(1, 100);
            if run.bright != Some(b) && run.bright_at.is_none_or(|t| t.elapsed() > Duration::from_millis(120)) {
                let ok = if lan_dev.is_some() {
                    send(&lan::brightness(b))
                } else if cloud_dev.is_some() {
                    self.queue_control(id, "devices.capabilities.range", "brightness", json!(b));
                    true
                } else {
                    false
                };
                if ok {
                    run.bright = Some(b);
                    run.bright_at = Some(Instant::now());
                }
            }
            let n = if l.segments > 0 { l.segments as usize } else { cloud_dev.and_then(|d| d.segments()).or_else(|| known_segments(lan_dev.map(|d| d.sku.as_str()))).unwrap_or(10) }.clamp(1, lan::RAZER_MAX);
            if look.mode.live() {
                if lan_dev.is_none() {
                    self.set_light_state(id, false, false, Some("Live colours need LAN Control: switch it on for this light in the Govee Home app.".into()));
                    continue;
                }
                // One colour: worked out from the whole zone, not averaged from segments.
                let way = route(l.stream, cloud_dev, lan_dev.map(|d| d.sku.as_str()), l.segments);
                let n = match way {
                    Route::Stream => n,
                    Route::Commands => n.min(lan::SEGMENT_MAX),
                    Route::Single => 1,
                };
                let target: Vec<look::C> = match look.mode {
                    Mode::Effect => {
                        let colors: Vec<Rgb> = look.colors.iter().filter_map(|c| Rgb::parse(c)).collect();
                        look::effect_colors(look.effect, &colors, look.speed, n, dev_index, &crate::rig::lighting::Inputs { t: w.t, level: w.sound.level, beat: w.sound.beat, temp: None })
                    }
                    Mode::Screen => match w.sampler.as_ref().and_then(|sm| sm.grid()) {
                        Some(g) => look::screen_colors(g, l.zone, n, look.skip_dark).into_iter().map(|c| look::screen_tone(c, look.saturation as f32 / 100.0, look.boost as f32 / 100.0)).collect(),
                        None => vec![[0.0; 3]; n],
                    },
                    _ => {
                        let pal: Vec<Rgb> = if look.album_colors && !w.palette.is_empty() { w.palette.clone() } else { look.colors.iter().filter_map(|c| Rgb::parse(c)).collect() };
                        let gain = 0.5 + look.sensitivity as f32 / 100.0 * 1.5;
                        let blend = 0.05 + look.smoothness as f32 / 100.0 * 0.6;
                        look::music_colors(look.style, &pal, &w.sound, n, dt, gain, blend, &mut run.music)
                    }
                };
                let (rise, fall) = match look.mode {
                    Mode::Screen => {
                        let tau = 0.02 + look.smoothness as f32 / 100.0 * 0.6;
                        (tau, tau)
                    }
                    // A beat still lands at once; smoother fades slower.
                    Mode::Music => {
                        let sm = look.smoothness as f32 / 100.0;
                        (0.015 + sm * 0.08, 0.1 + sm * 0.45)
                    }
                    _ => (0.0, 0.0),
                };
                let mut colors = run.smooth.step(&target, dt, rise, fall);
                if l.reverse {
                    colors.reverse();
                }
                let changed = run.last.as_ref().is_none_or(|p| p.len() != colors.len() || p.iter().zip(&colors).any(|(a, b)| a.0.abs_diff(b.0) > 1 || a.1.abs_diff(b.1) > 1 || a.2.abs_diff(b.2) > 1));
                let keepalive = run.sent_at.is_none_or(|t| t.elapsed() > Duration::from_millis(900));
                if way != Route::Stream && run.razer_on {
                    send(&lan::razer(&lan::razer_switch(false)));
                    run.razer_on = false;
                }
                if way == Route::Commands {
                    // Only the segments that changed, a few times a second; all of
                    // them now and then in case a packet was lost.
                    if (changed || keepalive) && run.sent_at.is_none_or(|t| t.elapsed() >= Duration::from_millis(120)) {
                        let full = keepalive || run.cmd_sent.len() != colors.len();
                        let todo: Vec<(usize, Rgb)> = colors.iter().map(|&c| coarse(c)).enumerate().filter(|(i, c)| full || run.cmd_sent.get(*i) != Some(c)).collect();
                        let packets = lan::segment_packets(&todo);
                        if !packets.is_empty() {
                            let refs: Vec<&[u8]> = packets.iter().map(|p| &p[..]).collect();
                            send(&lan::pt_real(&refs));
                        }
                        run.cmd_sent = colors.iter().map(|&c| coarse(c)).collect();
                        run.sent_at = Some(Instant::now());
                        run.last = Some(colors.clone());
                    }
                } else if way == Route::Stream {
                    if (changed || keepalive) && run.sent_at.is_none_or(|t| t.elapsed() >= Duration::from_millis(30)) {
                        // Switched on once (and after a pause): switching it on again
                        // while streaming makes some lamps blink off.
                        if !run.razer_on || run.sent_at.is_none_or(|t| t.elapsed() > Duration::from_secs(2)) {
                            send(&lan::razer(&lan::razer_switch(true)));
                            run.razer_on = true;
                        }
                        send(&lan::razer(&lan::razer_frame(&colors, l.gradient)));
                        run.sent_at = Some(Instant::now());
                        run.last = Some(colors.clone());
                    }
                } else {
                    if (changed || keepalive) && run.sent_at.is_none_or(|t| t.elapsed() >= Duration::from_millis(100)) {
                        let k = colors.len().max(1) as u32;
                        let sum = colors.iter().fold((0u32, 0u32, 0u32), |a, c| (a.0 + c.0 as u32, a.1 + c.1 as u32, a.2 + c.2 as u32));
                        send(&lan::color(Rgb((sum.0 / k) as u8, (sum.1 / k) as u8, (sum.2 / k) as u8)));
                        run.sent_at = Some(Instant::now());
                        run.last = Some(colors.clone());
                    }
                }
                run.applied = None;
                frames.insert(id.clone(), colors);
                self.set_light_state(id, true, way != Route::Single, None);
                continue;
            }
            // A still look: sent once, again only when it changes.
            run.smooth.reset();
            run.last = None;
            let sig = match look.mode {
                Mode::Color => format!("c{}", look.color),
                Mode::White => format!("w{}", look.kelvin),
                Mode::Scene => format!("s{}", look.scene),
                Mode::DeviceMusic => format!("m{}:{}", look.device_music, look.sensitivity),
                _ => String::new(),
            };
            if run.applied.as_deref() != Some(sig.as_str()) {
                if run.razer_on {
                    send(&lan::razer(&lan::razer_switch(false)));
                    run.razer_on = false;
                }
                let message = match look.mode {
                    Mode::Color => {
                        let c = Rgb::parse(&look.color).unwrap_or(Rgb(255, 255, 255));
                        if lan_dev.is_some() {
                            send(&lan::color(c));
                        } else if cloud_dev.is_some() {
                            self.queue_control(id, "devices.capabilities.color_setting", "colorRgb", json!(((c.0 as u32) << 16) | ((c.1 as u32) << 8) | c.2 as u32));
                        }
                        None
                    }
                    Mode::White => {
                        if lan_dev.is_some() {
                            send(&lan::kelvin(look.kelvin));
                        } else if cloud_dev.is_some() {
                            self.queue_control(id, "devices.capabilities.color_setting", "colorTemperatureK", json!(look.kelvin));
                        }
                        None
                    }
                    Mode::Scene => match cloud_dev {
                        None => Some("Scenes need your Govee API key.".to_string()),
                        Some(cd) => {
                            let extra = self.shared.lock().cloud_extra.get(id).cloned().unwrap_or_default();
                            let pick = |list: &[Choice]| list.iter().find(|c| c.name.eq_ignore_ascii_case(&look.scene)).map(|c| c.value.clone());
                            if let Some(v) = pick(&extra.0) {
                                self.queue_control(id, "devices.capabilities.dynamic_scene", "lightScene", v);
                                None
                            } else if let Some(v) = pick(&extra.1).or_else(|| pick(&cd.listed("diyScene"))) {
                                self.queue_control(id, "devices.capabilities.dynamic_scene", "diyScene", v);
                                None
                            } else if let Some(v) = pick(&cd.listed("snapshot")) {
                                self.queue_control(id, "devices.capabilities.dynamic_scene", "snapshot", v);
                                None
                            } else {
                                Some(format!("This light has no scene called {}.", look.scene))
                            }
                        }
                    },
                    Mode::DeviceMusic => match cloud_dev {
                        None => Some("The light's own music modes need your Govee API key.".to_string()),
                        Some(cd) => {
                            let modes = cd.music_modes();
                            match modes.iter().find(|c| c.name.eq_ignore_ascii_case(&look.device_music)).or(modes.first()) {
                                Some(m) => {
                                    self.queue_control(id, "devices.capabilities.music_setting", "musicMode", json!({ "musicMode": m.value, "sensitivity": look.sensitivity, "autoColor": 1 }));
                                    None
                                }
                                None => Some("This light has no music mode of its own.".to_string()),
                            }
                        }
                    },
                    _ => None,
                };
                if lan_dev.is_some() || cloud_dev.is_some() {
                    run.applied = Some(sig);
                }
                self.set_light_state(id, false, false, message);
            }
            let preview = match look.mode {
                Mode::Color => Rgb::parse(&look.color).map(|c| vec![c]),
                Mode::White => Some(vec![kelvin_rgb(look.kelvin)]),
                _ => None,
            };
            if let Some(p) = preview {
                frames.insert(id.clone(), p);
            }
        }
        {
            let mut ids: Vec<&String> = frames.keys().collect();
            ids.sort();
            let all: Vec<Rgb> = ids.iter().flat_map(|k| frames[*k].iter().copied()).collect();
            if !all.is_empty() {
                if let Ok(mut show) = SHOW.lock() {
                    *show = Some((all, Instant::now()));
                }
            }
            let mut sh = self.shared.lock();
            sh.frames = frames;
            let st = &mut sh.status;
            st.screen_message = screen_message;
            st.music_message = music_message;
            if need_music {
                st.palette = palette_now;
            }
        }
        if any_live {
            Duration::from_millis(if held { 16 } else { 20 })
        } else if held || !fades.is_empty() || sleep_at.is_some() {
            Duration::from_millis(250)
        } else {
            Duration::from_millis(500)
        }
    }

    /// A settings change made by the worker itself (no new wake-up).
    fn update_quiet(self: &Arc<Self>, patch: &Value) -> Result<(), String> {
        self.update(patch).map(|_| ())
    }

    fn queue_control(self: &Arc<Self>, id: &str, cap_type: &str, instance: &str, value: Value) {
        self.cloud_job(CloudJob::Control { id: id.into(), cap_type: cap_type.into(), instance: instance.into(), value });
    }
}

/// Per-light state of the worker.
#[derive(Default)]
struct Run {
    ip: Option<IpAddr>,
    razer_on: bool,
    smooth: Smooth,
    music: MusicRun,
    last: Option<Vec<Rgb>>,
    /// Segment colours last sent as commands.
    cmd_sent: Vec<Rgb>,
    sent_at: Option<Instant>,
    /// The still look last sent.
    applied: Option<String>,
    power: Option<bool>,
    bright: Option<u8>,
    bright_at: Option<Instant>,
}

#[derive(Default)]
struct Worker {
    lan: Option<Lan>,
    lan_retry: Option<Instant>,
    next_scan: Option<Instant>,
    next_sweep: Option<Instant>,
    next_status: Option<Instant>,
    status_due: Option<Instant>,
    status_at: Option<Instant>,
    runs: HashMap<String, Run>,
    t: f32,
    last_tick: Option<Instant>,
    last_preview: Option<Instant>,
    lease_at: Option<Instant>,
    sound: Sound,
    palette: Vec<Rgb>,
    palette_for: Option<String>,
    sampler: Option<Box<dyn crate::capture::sampler::Sampler>>,
    sampler_for: String,
    sampler_retry: Option<Instant>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lan::tests::FakeLight;

    /// A free port below the range the OS hands out for "any port" (32768+
    /// on Linux, 49152+ on Windows), so no other test's socket can take it
    /// between this check and the hub binding it, which would leave the hub
    /// retrying for 10 seconds.
    fn answer_port() -> u16 {
        static NEXT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(0);
        let base = 20_000 + (std::process::id() % 500) as u16 * 20;
        loop {
            let p = base + NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 2_000;
            if std::net::UdpSocket::bind(("127.0.0.1", p)).is_ok() {
                return p;
            }
        }
    }

    fn hub(dir: &Path) -> (Arc<GoveeHub>, FakeLight, EventBus) {
        let events = EventBus::new();
        // A free answer port, then the light answering on it.
        let listen = answer_port();
        let fake = FakeLight::start(listen, "aa:bb:cc:dd:ee:ff:00:11", "H6199");
        let cfg = LanConfig { addr: Ipv4Addr::LOCALHOST, ports: Ports { listen, ..fake.ports }, multicast: false };
        let visual = crate::media::visual::VisualHub::new(events.clone(), true);
        let media = crate::media::MediaHub::new(dir, events.clone(), true);
        let h = GoveeHub::with(dir, events.clone(), visual, media, cfg, "http://127.0.0.1:9");
        h.update(&json!({ "manual": ["127.0.0.1"] })).unwrap();
        (h, fake, events)
    }

    fn wait(what: &str, mut f: impl FnMut() -> bool) {
        let t0 = Instant::now();
        while !f() {
            assert!(t0.elapsed() < Duration::from_secs(6), "timed out: {what}");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn finds_lights_and_shows_colours_whites_and_effects() {
        let dir = tempfile::tempdir().unwrap();
        let (h, fake, _) = hub(dir.path());
        h.hold();
        wait("the light found", || h.status().lights.iter().any(|l| l.lan && l.sku == "H6199"));
        let st = h.status();
        assert_eq!(st.lan, LinkState::On);
        let light = &st.lights[0];
        assert_eq!((light.id.as_str(), light.name.as_str(), light.ip.as_deref()), ("AA:BB:CC:DD:EE:FF:00:11", "Govee H6199", Some("127.0.0.1")));
        assert!(fake.commands("colorwc").is_empty(), "nothing changes until OmniHub controls the lights");
        wait("its state read", || h.status().lights[0].brightness == Some(80));

        // A colour and a brightness.
        h.update(&json!({ "enabled": true, "brightness": 35, "look": { "mode": "color", "color": "#ff8800" } })).unwrap();
        wait("the colour", || fake.commands("colorwc").iter().any(|c| c["msg"]["data"]["color"] == json!({ "r": 255, "g": 136, "b": 0 })));
        wait("the brightness", || fake.commands("brightness").iter().any(|c| c["msg"]["data"]["value"] == 35));
        // A white.
        h.update(&json!({ "look": { "mode": "white", "kelvin": 3000 } })).unwrap();
        wait("the white", || fake.commands("colorwc").iter().any(|c| c["msg"]["data"]["colorTemInKelvin"] == 3000));
        // An effect streams segments.
        h.update_light("AA:BB:CC:DD:EE:FF:00:11", &json!({ "segments": 6 })).unwrap();
        h.update(&json!({ "look": { "mode": "effect", "effect": "wave" } })).unwrap();
        wait("razer frames", || fake.commands("razer").len() > 5);
        use base64::Engine;
        let frames = fake.commands("razer");
        let first = base64::engine::general_purpose::STANDARD.decode(frames[0]["msg"]["data"]["pt"].as_str().unwrap()).unwrap();
        assert_eq!(first, lan::razer_switch(true), "streaming is switched on first");
        let frame = base64::engine::general_purpose::STANDARD.decode(frames.last().unwrap()["msg"]["data"]["pt"].as_str().unwrap()).unwrap();
        assert_eq!(&frame[..6], &[0xBB, 0x00, 2 + 18, 0xB0, 1, 6], "six segments, blended");
        wait("streaming shown", || h.status().lights[0].streaming);
        // Back to a colour: streaming off first.
        h.update(&json!({ "look": { "mode": "color", "color": "#00ff00" } })).unwrap();
        wait("streaming off", || {
            fake.commands("razer").iter().any(|c| base64::engine::general_purpose::STANDARD.decode(c["msg"]["data"]["pt"].as_str().unwrap()).unwrap() == lan::razer_switch(false))
        });
        wait("the green", || fake.commands("colorwc").iter().any(|c| c["msg"]["data"]["color"] == json!({ "r": 0, "g": 255, "b": 0 })));
        // Off.
        h.update(&json!({ "on": false })).unwrap();
        wait("off", || fake.commands("turn").iter().any(|c| c["msg"]["data"]["value"] == 0));
        // Not controlled any more: left alone.
        h.update(&json!({ "enabled": false })).unwrap();
        let n = fake.commands("colorwc").len();
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(fake.commands("colorwc").len(), n);
        // The key is never in the settings patch.
        assert!(h.update(&json!({ "apiSecret": "stolen" })).unwrap().api_secret.is_empty());
    }

    #[test]
    fn screen_and_music_stream_live() {
        let dir = tempfile::tempdir().unwrap();
        let (h, fake, events) = hub(dir.path());
        h.update(&json!({ "enabled": true, "look": { "mode": "screen", "smoothness": 10 }, "lights": [{ "id": "AA:BB:CC:DD:EE:FF:00:11", "segments": 4, "zone": "around" }] })).unwrap();
        h.hold();
        wait("screen frames", || fake.commands("razer").len() > 10);
        // The test pattern isn't black: the frames carry colour.
        use base64::Engine;
        let lit = fake.commands("razer").iter().rev().take(5).any(|c| {
            let p = base64::engine::general_purpose::STANDARD.decode(c["msg"]["data"]["pt"].as_str().unwrap()).unwrap();
            p.len() > 10 && p[6..p.len() - 1].iter().any(|&v| v > 20)
        });
        assert!(lit);
        // Music: the pretend player's sound drives it.
        let mut rx = events.subscribe();
        h.update(&json!({ "look": { "mode": "music", "style": "pulse" } })).unwrap();
        events.emit("media:state", json!({ "state": { "key": "song", "playing": true } }));
        let before = fake.commands("razer").len();
        wait("music frames", || fake.commands("razer").len() > before + 10);
        wait("the page sees the frames", || {
            let mut seen = false;
            while let Ok(e) = rx.try_recv() {
                seen |= e.topic == "govee:frame" && e.payload["lights"]["AA:BB:CC:DD:EE:FF:00:11"].as_array().is_some_and(|a| a.len() == 4);
            }
            seen
        });
    }

    #[test]
    fn segment_commands_for_lights_that_dont_stream() {
        let dir = tempfile::tempdir().unwrap();
        let (h, fake, _events) = hub(dir.path());
        h.update(&json!({ "enabled": true, "look": { "mode": "screen", "smoothness": 10 }, "lights": [{ "id": "AA:BB:CC:DD:EE:FF:00:11", "segments": 4, "zone": "around", "stream": "segmentsLite" }] })).unwrap();
        h.hold();
        wait("segment commands", || !fake.commands("ptReal").is_empty());
        use base64::Engine;
        let c = &fake.commands("ptReal")[0];
        let p = base64::engine::general_purpose::STANDARD.decode(c["msg"]["data"]["command"][0].as_str().unwrap()).unwrap();
        assert_eq!((p.len(), &p[..4]), (20, &[0x33u8, 0x05, 0x15, 0x01][..]));
        assert!(fake.commands("razer").iter().all(|c| base64::engine::general_purpose::STANDARD.decode(c["msg"]["data"]["pt"].as_str().unwrap()).unwrap() != lan::razer_switch(true)), "no streaming switched on");
        wait("shown as per segment", || h.status().lights[0].per_segment && h.status().lights[0].streaming);
    }

    #[test]
    fn presets_timers_and_the_sleep_timer() {
        let dir = tempfile::tempdir().unwrap();
        let (h, fake, _) = hub(dir.path());
        let s = h.apply_preset("focus", &[]).unwrap();
        assert!(s.enabled && s.look.mode == Mode::White && s.look.kelvin == 5600 && s.brightness == 100 && s.preset == "focus");
        wait("the focus white", || fake.commands("colorwc").iter().any(|c| c["msg"]["data"]["colorTemInKelvin"] == 5600));
        let mine = h.save_preset(Preset { name: "  Cozy ".into(), look: Look { mode: Mode::Color, color: "#ff2200".into(), ..Default::default() }, ..Default::default() }).unwrap();
        assert!(mine.id.starts_with("my-") && mine.name == "Cozy");
        assert_eq!(h.presets().len(), built_in_presets().len() + 1);
        // To one light only: it stops following.
        let s = h.apply_preset(&mine.id, &["aa:bb:cc:dd:ee:ff:00:11".into()]).unwrap();
        let l = s.light("AA:BB:CC:DD:EE:FF:00:11");
        assert!(!l.follow && l.look.color == "#ff2200");
        h.delete_preset(&mine.id).unwrap();
        assert!(h.apply_preset(&mine.id, &[]).is_err());

        let t = h.save_timer(Timer { name: "Wake".into(), at: "07:15".into(), days: vec![1, 2, 3], ..Default::default() }).unwrap();
        assert!(!t.id.is_empty());
        assert_eq!(h.settings().timers.len(), 1);
        h.fire(&Timer { action: TimerAction::Off, ..Default::default() });
        assert!(!h.settings().on);
        h.fire(&Timer { action: TimerAction::Brightness, brightness: 25, ..Default::default() });
        assert_eq!(h.settings().brightness, 25);
        h.delete_timer(&t.id).unwrap();
        assert!(h.settings().timers.is_empty());

        let st = h.sleep(Some(30));
        let left = st.sleep_until.unwrap() - crate::media::now_ms();
        assert!((29 * 60_000..=30 * 60_000).contains(&left));
        assert!(h.sleep(None).sleep_until.is_none());
    }

    #[test]
    fn auto_streams_segments_only_to_lights_that_can() {
        let dev = |caps: &[&str]| CloudDevice { sku: "H6022".into(), device: "x".into(), name: "Lamp".into(), kind: "devices.types.light".into(), capabilities: caps.iter().map(|i| cloud::Capability { kind: "devices.capabilities.toggle".into(), instance: i.to_string(), parameters: Value::Null }).collect() };
        let (dream, plain) = (dev(&["dreamViewToggle"]), dev(&["gradientToggle"]));
        assert_eq!(route(StreamKind::Auto, Some(&dream), Some("H6022"), 0), Route::Stream);
        assert_eq!(route(StreamKind::Auto, Some(&plain), Some("H6199"), 0), Route::Single);
        // Not in the account: models known to stream stream; lights with
        // segments (known model, or set by hand) get segment commands.
        assert_eq!(route(StreamKind::Auto, None, Some("h6199"), 0), Route::Stream);
        assert!(route(StreamKind::Auto, None, Some("H6008"), 0) == Route::Single && route(StreamKind::Auto, None, None, 0) == Route::Single);
        assert_eq!(route(StreamKind::Auto, None, Some("H6076"), 0), Route::Commands, "the floor lamp takes segment commands");
        assert_eq!(known_segments(Some("h6076")), Some(14));
        assert_eq!(route(StreamKind::Auto, None, Some("H6008"), 6), Route::Commands, "segments set by hand");
        assert!(route(StreamKind::Segments, Some(&plain), None, 0) == Route::Stream && route(StreamKind::Single, Some(&dream), None, 0) == Route::Single);
        assert_eq!(route(StreamKind::SegmentsLite, None, Some("H6076"), 0), Route::Commands);
        assert_eq!(coarse(Rgb(255, 0, 130)), coarse(Rgb(253, 2, 132)));
        assert_eq!(LightSettings::default().stream, StreamKind::Auto);
        let mut s = GoveeSettings::default();
        s.look.boost = 250;
        sanitize(&mut s);
        assert_eq!(s.look.boost, 100);
        // Saved with the first default: brought down once, then left alone.
        let mut old: GoveeSettings = serde_json::from_value(json!({ "look": { "boost": 65 }, "lights": [{ "id": "a", "look": { "boost": 65 } }] })).unwrap();
        sanitize(&mut old);
        assert_eq!((old.look.boost, old.lights[0].look.boost, old.revision), (40, 40, 1));
        old.look.boost = 65;
        sanitize(&mut old);
        assert_eq!(old.look.boost, 65, "a boost picked later stays");
    }

    #[test]
    fn settings_are_kept_in_bounds() {
        let mut s: GoveeSettings = serde_json::from_value(json!({
            "brightness": 0, "manual": ["10.0.0.5", "not an address"],
            "look": { "color": "blue", "kelvin": 100, "speed": 50.0, "saturation": 250, "colors": ["#ff0000", "nope"] },
            "lights": [{ "id": " aa:bb ", "segments": 250, "brightness": 0 }, { "id": "" }]
        }))
        .unwrap();
        sanitize(&mut s);
        assert_eq!(s.brightness, 1);
        assert_eq!(s.manual, ["10.0.0.5"]);
        assert_eq!((s.look.color.as_str(), s.look.kelvin, s.look.speed, s.look.saturation, s.look.colors.len()), ("#00a2ff", 2000, 3.0, 200, 1));
        assert_eq!(s.lights.len(), 1);
        assert_eq!((s.lights[0].id.as_str(), s.lights[0].segments, s.lights[0].brightness), ("AA:BB", lan::RAZER_MAX as u8, 1));
        assert_eq!(kelvin_rgb(6600), Rgb(255, 255, 255));
        assert!(kelvin_rgb(2700).2 < kelvin_rgb(5000).2);
        assert!(built_in_presets().iter().all(|p| !p.id.is_empty() && !p.name.is_empty()));
    }
}
