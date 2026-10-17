//! "My PC": the PC's lighting, fans and screens, and what it is built from
//! (for the 3D view). OmniHub talks to the free tools that already know
//! the hardware instead of loading drivers of its own: OpenRGB for
//! lighting, LibreHardwareMonitor for motherboard and graphics-card fans
//! and temperatures, liquidctl for liquid coolers, fan hubs and cooler
//! screens. Nothing runs unless it is needed: one worker thread wakes for
//! lights that move, a fan profile other than Auto, or the page being
//! open, and stops when none of those is true.

pub mod fans;
pub mod lhm;
pub mod lighting;
pub mod liquidctl;
pub mod openrgb;
pub mod screens;
pub mod specs;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};
use serde::{Deserialize, Serialize};

use crate::events::EventBus;
use fans::{FanProfile, FanRole};
use lighting::{Effect, Inputs, LightProfile};
use openrgb::{Controller, DeviceKind, Rgb};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct LightingSettings {
    pub enabled: bool,
    /// The profile in use (a built-in id or one of `custom`).
    pub profile: String,
    pub custom: Vec<LightProfile>,
    /// OpenRGB's SDK port.
    pub port: u16,
    /// Also keyboards, mice, headsets and the like.
    pub peripherals: bool,
    /// Controllers left alone (`Controller::key`).
    pub skip: Vec<String>,
}

impl Default for LightingSettings {
    fn default() -> Self {
        LightingSettings { enabled: false, profile: "aurora".into(), custom: Vec::new(), port: openrgb::DEFAULT_PORT, peripherals: false, skip: Vec::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FanSettings {
    pub profile: FanProfile,
    pub lhm_url: String,
    pub lhm_user: String,
    /// The password, encrypted for this Windows account (DPAPI), base64.
    pub lhm_secret: String,
    /// Controls left alone (LibreHardwareMonitor ids).
    pub skip: Vec<String>,
    /// Use liquidctl for coolers and hubs when it is installed.
    pub coolers: bool,
    pub liquidctl_path: Option<String>,
}

impl Default for FanSettings {
    fn default() -> Self {
        FanSettings { profile: FanProfile::Auto, lhm_url: lhm::DEFAULT_URL.into(), lhm_user: String::new(), lhm_secret: String::new(), skip: Vec::new(), coolers: true, liquidctl_path: None }
    }
}

/// What a screen shows.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ScreenShow {
    #[default]
    Off,
    /// A picture, GIF or video from the library.
    Media { id: String },
    /// Temperatures, loads and fan speeds.
    Stats,
    /// The song playing, with its cover.
    NowPlaying,
    /// The cooler's own liquid-temperature screen (LCDs only).
    Liquid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ScreenSettings {
    /// The monitor that is the case's screen (its Windows name), if any.
    pub monitor: String,
    pub monitor_show: ScreenShow,
    /// Fill the screen (cropping) rather than fit inside it.
    pub fill: bool,
    /// The cooler LCD (liquidctl address), if any.
    pub lcd: String,
    pub lcd_show: ScreenShow,
    pub lcd_brightness: u8,
}

impl Default for ScreenSettings {
    fn default() -> Self {
        ScreenSettings { monitor: String::new(), monitor_show: ScreenShow::Off, fill: true, lcd: String::new(), lcd_show: ScreenShow::Off, lcd_brightness: 80 }
    }
}

/// What the 3D view can't detect, as the user describes it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelSettings {
    /// "black" or "white".
    pub case_color: String,
    /// A glass side panel.
    pub glass: bool,
    /// `None`: as detected.
    pub cooler: Option<specs::CoolerKind>,
    pub radiator_mm: Option<u16>,
    pub front_fans: u8,
    pub top_fans: u8,
    pub rear_fans: u8,
    pub bottom_fans: u8,
    pub ram_rgb: bool,
    pub vertical_gpu: bool,
    /// A screen inside the case (shown on its side in the 3D view).
    pub case_screen: bool,
}

impl Default for ModelSettings {
    fn default() -> Self {
        ModelSettings { case_color: "black".into(), glass: true, cooler: None, radiator_mm: None, front_fans: 3, top_fans: 0, rear_fans: 1, bottom_fans: 0, ram_rgb: true, vertical_gpu: false, case_screen: false }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RigSettings {
    pub lighting: LightingSettings,
    pub fans: FanSettings,
    pub screens: ScreenSettings,
    pub model: ModelSettings,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LinkState {
    /// Not in use.
    #[default]
    Off,
    /// In use and answering.
    On,
    /// The tool isn't running (or installed).
    Missing,
    Error,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LitDevice {
    pub key: String,
    pub name: String,
    pub kind: DeviceKind,
    pub leds: usize,
    pub zones: Vec<String>,
    /// Lit by OmniHub (not skipped, not a peripheral left alone).
    pub on: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FanControl {
    pub id: String,
    pub name: String,
    pub hardware: String,
    pub role: Option<FanRole>,
    /// The output now, percent.
    pub percent: Option<f32>,
    pub skipped: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RigStatus {
    pub lighting: LinkState,
    pub lighting_message: Option<String>,
    pub devices: Vec<LitDevice>,
    pub openrgb_installed: Option<String>,
    pub fans: LinkState,
    pub fans_message: Option<String>,
    pub controls: Vec<FanControl>,
    /// Fan speeds, RPM.
    pub rpm: Vec<(String, f32)>,
    pub cpu_temp: Option<f32>,
    pub gpu_temp: Option<f32>,
    pub coolers: LinkState,
    pub coolers_message: Option<String>,
    pub cooler_devices: Vec<liquidctl::LqDevice>,
    pub liquid_temp: Option<f32>,
    pub lcd_message: Option<String>,
}

/// OpenRGB's program, where its installer puts it.
pub fn openrgb_installed() -> Option<PathBuf> {
    let mut c = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
        if let Some(d) = std::env::var_os(var).map(PathBuf::from) {
            c.push(d.join("OpenRGB").join("OpenRGB.exe"));
            c.push(d.join("Programs").join("OpenRGB").join("OpenRGB.exe"));
        }
    }
    c.into_iter().find(|p| p.is_file())
}

fn peripheral(k: DeviceKind) -> bool {
    use DeviceKind::*;
    matches!(k, Keyboard | Mouse | Mousemat | Headset | HeadsetStand | Gamepad | Speaker | Microphone | Keypad)
}

const LEASE: Duration = Duration::from_secs(8);

struct Shared {
    settings: RigSettings,
    status: RigStatus,
    /// The page (or the case screen) is open until then.
    held_until: Option<Instant>,
    /// Settings changed: the worker should look again.
    dirty: bool,
}

pub struct RigHub {
    path: PathBuf,
    events: EventBus,
    visual: Arc<crate::media::visual::VisualHub>,
    pub library: screens::Library,
    shared: Mutex<Shared>,
    wake: Condvar,
    running: AtomicBool,
    specs: Mutex<Option<specs::Specs>>,
    /// Controls OmniHub set (handed back on Auto, on skip and on quit).
    touched: Mutex<HashMap<String, f32>>,
    /// The profile last sent to the coolers and hubs this run.
    cooler_profile: Mutex<Option<FanProfile>>,
}

impl RigHub {
    pub fn new(data_dir: &Path, events: EventBus, visual: Arc<crate::media::visual::VisualHub>) -> Arc<RigHub> {
        let path = data_dir.join("rig.json");
        let settings = std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        Arc::new(RigHub {
            path,
            events,
            visual,
            library: screens::Library::new(data_dir.join("rig-media")),
            shared: Mutex::new(Shared { settings, status: RigStatus::default(), held_until: None, dirty: true }),
            wake: Condvar::new(),
            running: AtomicBool::new(false),
            specs: Mutex::new(None),
            touched: Mutex::new(HashMap::new()),
            cooler_profile: Mutex::new(None),
        })
    }

    pub fn settings(&self) -> RigSettings {
        self.shared.lock().settings.clone()
    }

    pub fn status(&self) -> RigStatus {
        self.shared.lock().status.clone()
    }

    /// The PC's parts (detected once, then kept).
    pub fn specs(&self, refresh: bool) -> specs::Specs {
        let mut s = self.specs.lock();
        if refresh || s.is_none() {
            *s = Some(specs::detect());
        }
        s.clone().unwrap_or_default()
    }

    /// Every lighting profile: OmniHub's, then the user's.
    pub fn profiles(&self) -> Vec<LightProfile> {
        let mut all = lighting::built_in();
        all.extend(self.shared.lock().settings.lighting.custom.iter().cloned());
        all
    }

    fn profile(&self, s: &RigSettings) -> LightProfile {
        let id = &s.lighting.profile;
        s.lighting.custom.iter().find(|p| &p.id == id).cloned().or_else(|| lighting::built_in().into_iter().find(|p| &p.id == id)).unwrap_or_default()
    }

    /// Change settings with a JSON merge patch (as the app's settings).
    pub fn update(self: &Arc<Self>, patch: &serde_json::Value) -> Result<RigSettings, String> {
        let mut shared = self.shared.lock();
        let mut v = serde_json::to_value(&shared.settings).map_err(|e| e.to_string())?;
        merge(&mut v, patch);
        let mut next: RigSettings = serde_json::from_value(v).map_err(|e| format!("Those settings don't fit: {e}"))?;
        sanitize(&mut next);
        let lcd_changed = next.screens.lcd != shared.settings.screens.lcd || next.screens.lcd_show != shared.settings.screens.lcd_show || next.screens.lcd_brightness != shared.settings.screens.lcd_brightness;
        shared.settings = next.clone();
        shared.dirty = true;
        drop(shared);
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        std::fs::write(&self.path, serde_json::to_vec_pretty(&next).unwrap_or_default()).map_err(|e| e.to_string())?;
        if lcd_changed {
            let hub = self.clone();
            std::thread::spawn(move || hub.apply_lcd());
        }
        self.events.emit("rig:settings", &next);
        self.kick();
        Ok(next)
    }

    /// Save a lighting profile of the user's (new when its id is empty).
    pub fn save_profile(self: &Arc<Self>, mut p: LightProfile) -> Result<LightProfile, String> {
        p.name = p.name.trim().chars().take(40).collect();
        if p.name.is_empty() {
            return Err("Give the profile a name.".into());
        }
        p.colors.retain(|c| Rgb::parse(c).is_some());
        p.colors.truncate(8);
        p.speed = p.speed.clamp(0.2, 3.0);
        p.brightness = p.brightness.min(100);
        if lighting::built_in().iter().any(|b| b.id == p.id) || p.id.is_empty() {
            p.id = format!("my-{}", uuid::Uuid::new_v4());
        }
        let mut custom = self.settings().lighting.custom;
        match custom.iter().position(|q| q.id == p.id) {
            Some(i) => custom[i] = p.clone(),
            None if custom.len() >= 50 => return Err("That's a lot of profiles; remove one first.".into()),
            None => custom.push(p.clone()),
        }
        self.update(&serde_json::json!({ "lighting": { "custom": custom, "profile": p.id } }))?;
        Ok(p)
    }

    pub fn delete_profile(self: &Arc<Self>, id: &str) -> Result<(), String> {
        let s = self.settings();
        let custom: Vec<LightProfile> = s.lighting.custom.into_iter().filter(|p| p.id != id).collect();
        let profile = if s.lighting.profile == id { "aurora".to_string() } else { s.lighting.profile };
        self.update(&serde_json::json!({ "lighting": { "custom": custom, "profile": profile } })).map(|_| ())
    }

    /// LibreHardwareMonitor's password (kept encrypted for this account).
    pub fn set_lhm_login(self: &Arc<Self>, user: &str, password: &str) -> Result<(), String> {
        use base64::Engine;
        let secret = if password.is_empty() { String::new() } else { base64::engine::general_purpose::STANDARD.encode(crate::system::dpapi::protect(password.as_bytes(), b"omnihub-lhm").map_err(|e| e.to_string())?) };
        self.update(&serde_json::json!({ "fans": { "lhmUser": user.trim(), "lhmSecret": secret } })).map(|_| ())
    }

    fn lhm(&self, s: &FanSettings) -> lhm::Server {
        use base64::Engine;
        let password = base64::engine::general_purpose::STANDARD
            .decode(&s.lhm_secret)
            .ok()
            .and_then(|b| crate::system::dpapi::unprotect(&b, b"omnihub-lhm").ok())
            .map(|p| String::from_utf8_lossy(&p).into_owned())
            .unwrap_or_default();
        lhm::Server { url: s.lhm_url.clone(), user: (!s.lhm_user.is_empty()).then(|| (s.lhm_user.clone(), password)) }
    }

    /// The page or the case screen is showing (call every few seconds).
    pub fn hold(self: &Arc<Self>) -> RigStatus {
        self.shared.lock().held_until = Some(Instant::now() + LEASE);
        self.kick();
        self.status()
    }

    fn kick(self: &Arc<Self>) {
        self.wake.notify_all();
        if !self.running.swap(true, Ordering::SeqCst) {
            let hub = self.clone();
            if std::thread::Builder::new().name("rig".into()).spawn(move || hub.run()).is_err() {
                self.running.store(false, Ordering::SeqCst);
            }
        }
    }

    /// Start what the saved settings want (at app start).
    pub fn start(self: &Arc<Self>) {
        let s = self.settings();
        if s.lighting.enabled || s.fans.profile != FanProfile::Auto {
            self.kick();
        }
        if s.screens.lcd_show != ScreenShow::Off && !s.screens.lcd.is_empty() {
            let hub = self.clone();
            std::thread::spawn(move || hub.apply_lcd());
        }
    }

    /// Hand every fan OmniHub set back to the hardware (on quit).
    pub fn shutdown(&self) {
        let server = self.lhm(&self.settings().fans);
        for (id, _) in self.touched.lock().drain() {
            let _ = server.set(&id, None);
        }
    }

    fn liquidctl(&self, s: &FanSettings) -> Option<liquidctl::Liquidctl> {
        liquidctl::find(s.liquidctl_path.as_deref()).map(|exe| liquidctl::Liquidctl { exe })
    }

    /// Put the chosen picture, GIF or liquid temperature on the cooler's LCD.
    pub fn apply_lcd(&self) {
        let s = self.settings();
        let msg = (|| -> Result<Option<String>, String> {
            if s.screens.lcd.is_empty() {
                return Ok(None);
            }
            let lq = self.liquidctl(&s.fans).ok_or("Install liquidctl to set the cooler's screen.")?;
            match &s.screens.lcd_show {
                ScreenShow::Media { id } => {
                    let m = self.library.get(id).ok_or("That picture is no longer in the library.")?;
                    let path = self.library.path(id).ok_or("That picture is no longer in the library.")?;
                    let show = match m.kind {
                        screens::MediaKind::Picture => liquidctl::LcdShow::Picture(&path),
                        screens::MediaKind::Gif => liquidctl::LcdShow::Gif(&path),
                        screens::MediaKind::Video => return Err("Cooler screens take pictures and GIFs, not videos.".into()),
                    };
                    lq.set_lcd(&s.screens.lcd, show)?;
                }
                ScreenShow::Liquid | ScreenShow::Off => lq.set_lcd(&s.screens.lcd, liquidctl::LcdShow::Liquid)?,
                ScreenShow::Stats | ScreenShow::NowPlaying => return Err("Cooler screens show a picture, a GIF or the liquid temperature.".into()),
            }
            lq.set_lcd(&s.screens.lcd, liquidctl::LcdShow::Brightness(s.screens.lcd_brightness))?;
            Ok(None)
        })()
        .unwrap_or_else(Some);
        self.shared.lock().status.lcd_message = msg;
        self.events.emit("rig:status", self.status());
    }

    fn set_status(&self, f: impl FnOnce(&mut RigStatus)) {
        let (changed, held, status) = {
            let mut sh = self.shared.lock();
            let before = sh.status.clone();
            f(&mut sh.status);
            (before != sh.status, sh.held_until.is_some_and(|t| t > Instant::now()), sh.status.clone())
        };
        if changed && held {
            self.events.emit("rig:status", status);
        }
    }

    fn run(self: Arc<Self>) {
        let mut light = LightRun::default();
        let mut fan = FanRun::default();
        let mut audio = self.events.subscribe();
        loop {
            let (s, held) = {
                let mut sh = self.shared.lock();
                sh.dirty = false;
                (sh.settings.clone(), sh.held_until.is_some_and(|t| t > Instant::now()))
            };
            let profile = self.profile(&s);
            let lights = s.lighting.enabled;
            let fans_on = s.fans.profile != FanProfile::Auto || !self.touched.lock().is_empty();
            let temp_light = lights && profile.effect == Effect::Temperature;
            if !lights && !fans_on && !held {
                light.client = None;
                if profile.effect == Effect::Music {
                    self.visual.release("lighting");
                }
                break;
            }
            // Music for the lights.
            if lights && profile.effect == Effect::Music {
                if light.lease_at.is_none_or(|t| t.elapsed() > Duration::from_secs(2)) {
                    self.visual.hold("lighting");
                    light.lease_at = Some(Instant::now());
                }
            } else if light.lease_at.take().is_some() {
                self.visual.release("lighting");
            }
            loop {
                match audio.try_recv() {
                    Ok(e) if e.topic == "audio:frame" => {
                        light.level = e.payload["level"].as_f64().unwrap_or(0.0) as f32;
                        light.beat = e.payload["beat"].as_f64().unwrap_or(0.0) as f32;
                    }
                    Ok(_) => {}
                    Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {}
                    Err(_) => break,
                }
            }
            // Sensors and fans every two seconds.
            if (fans_on || held || temp_light) && fan.next.is_none_or(|t| Instant::now() >= t) {
                self.fan_tick(&s, &mut fan, held);
                fan.next = Some(Instant::now() + Duration::from_secs(2));
            }
            let wait = if lights { self.light_tick(&s, &profile, &mut light) } else { Duration::from_secs(2) };
            if !lights {
                light.client = None;
                self.set_status(|st| {
                    st.lighting = LinkState::Off;
                    st.lighting_message = None;
                });
            }
            let mut sh = self.shared.lock();
            if !sh.dirty {
                self.wake.wait_for(&mut sh, wait);
            }
        }
        self.running.store(false, Ordering::SeqCst);
        // A wake-up that came while stopping.
        let sh = self.shared.lock();
        let again = sh.settings.lighting.enabled || sh.settings.fans.profile != FanProfile::Auto || sh.held_until.is_some_and(|t| t > Instant::now());
        drop(sh);
        if again {
            self.kick();
        }
    }

    /// One frame of light; returns how long until the next.
    fn light_tick(&self, s: &RigSettings, profile: &LightProfile, run: &mut LightRun) -> Duration {
        if run.client.is_none() {
            if run.retry_at.is_some_and(|t| Instant::now() < t) {
                return Duration::from_secs(1);
            }
            let addr = SocketAddr::from(([127, 0, 0, 1], s.lighting.port));
            match openrgb::Client::connect(addr, Duration::from_millis(600)) {
                Ok(c) => {
                    run.client = Some(c);
                    run.controllers.clear();
                    run.sent.clear();
                }
                Err(e) => {
                    run.retry_at = Some(Instant::now() + Duration::from_secs(5));
                    let installed = openrgb_installed().map(|p| p.to_string_lossy().into_owned());
                    let refused = e.kind() == std::io::ErrorKind::ConnectionRefused || e.kind() == std::io::ErrorKind::TimedOut;
                    self.set_status(|st| {
                        st.lighting = if refused { LinkState::Missing } else { LinkState::Error };
                        st.lighting_message = Some(if refused { "OpenRGB isn't running with its SDK server on.".to_string() } else { format!("OpenRGB: {e}") });
                        st.openrgb_installed = installed;
                    });
                    return Duration::from_secs(1);
                }
            }
        }
        let Some(client) = run.client.as_mut() else { return Duration::from_secs(1) };
        if run.controllers.is_empty() || client.list_changed || run.chosen_for != (s.lighting.skip.clone(), s.lighting.peripherals) {
            match client.controllers() {
                Ok(list) => {
                    run.controllers = list;
                    run.sent.clear();
                    run.chosen_for = (s.lighting.skip.clone(), s.lighting.peripherals);
                    run.lit.clear();
                    let mut lit = Vec::new();
                    for c in &run.controllers {
                        let on = !s.lighting.skip.contains(&c.key()) && (s.lighting.peripherals || !peripheral(c.kind));
                        if on && client.take_control(c.index).is_ok() {
                            run.lit.push(c.index);
                        }
                        lit.push(LitDevice { key: c.key(), name: c.name.clone(), kind: c.kind, leds: c.leds, zones: c.zones.iter().map(|z| z.name.clone()).collect(), on });
                    }
                    self.set_status(|st| {
                        st.lighting = LinkState::On;
                        st.lighting_message = if lit.is_empty() { Some("OpenRGB found no lights. Let it detect your devices (run it once as administrator for motherboard and memory lighting).".into()) } else { None };
                        st.devices = lit;
                    });
                }
                Err(e) => {
                    run.client = None;
                    self.set_status(|st| {
                        st.lighting = LinkState::Error;
                        st.lighting_message = Some(format!("OpenRGB: {e}"));
                    });
                    return Duration::from_secs(1);
                }
            }
        }
        let inputs = Inputs { t: run.t0.get_or_insert_with(Instant::now).elapsed().as_secs_f32(), level: run.level, beat: run.beat, temp: self.status().cpu_temp };
        let colors = lighting::palette(profile);
        let refresh = run.last_full.is_none_or(|t| t.elapsed() > Duration::from_secs(5));
        let mut failed = false;
        for (dev, c) in run.controllers.iter().enumerate() {
            if !run.lit.contains(&c.index) || c.leds == 0 {
                continue;
            }
            let frame = lighting::frame(profile, &colors, c.leds, dev, &inputs);
            if !refresh && run.sent.get(&c.index) == Some(&frame) {
                continue;
            }
            if client.set_leds(c.index, &frame).is_err() {
                failed = true;
                break;
            }
            run.sent.insert(c.index, frame);
        }
        if refresh {
            run.last_full = Some(Instant::now());
        }
        if failed {
            run.client = None;
            run.retry_at = Some(Instant::now() + Duration::from_secs(2));
        }
        if profile.effect.animated() {
            Duration::from_millis(33)
        } else {
            Duration::from_secs(1)
        }
    }

    fn fan_tick(&self, s: &RigSettings, run: &mut FanRun, held: bool) {
        let server = self.lhm(&s.fans);
        match server.sensors() {
            Ok(sensors) => {
                let cpu = lhm::cpu_temp(&sensors);
                let gpu = lhm::gpu_temp(&sensors);
                let mut controls = Vec::new();
                let mut errors = Vec::new();
                for c in sensors.iter().filter(|c| c.kind == lhm::SensorKind::Control) {
                    let role = fans::role_of(&c.id, &c.name);
                    let skipped = s.fans.skip.contains(&c.id);
                    let curve = if skipped { None } else { fans::curve(s.fans.profile, role) };
                    let temp = if role == FanRole::Gpu { gpu } else { cpu };
                    match (curve, temp) {
                        (Some(curve), Some(t)) => {
                            let prev = self.touched.lock().get(&c.id).copied();
                            let v = fans::smooth(prev, fans::duty(curve, t, role));
                            if prev.is_none_or(|p| (p - v).abs() >= 1.0) || c.value.is_none_or(|now| (now - v).abs() >= 2.0) {
                                match server.set(&c.id, Some(v)) {
                                    Ok(()) => {
                                        self.touched.lock().insert(c.id.clone(), v);
                                    }
                                    Err(e) => errors.push(format!("{}: {e}", c.name)),
                                }
                            }
                        }
                        // Auto, skipped, or nothing to follow: back to the hardware.
                        _ => {
                            if self.touched.lock().remove(&c.id).is_some() {
                                if let Err(e) = server.set(&c.id, None) {
                                    errors.push(format!("{}: {e}", c.name));
                                }
                            }
                        }
                    }
                    controls.push(FanControl { id: c.id.clone(), name: c.name.clone(), hardware: c.hardware.clone(), role: Some(role), percent: c.value, skipped });
                }
                let rpm = sensors.iter().filter(|f| f.kind == lhm::SensorKind::Fan).filter_map(|f| Some((f.name.clone(), f.value?))).collect();
                let none = controls.is_empty();
                self.set_status(|st| {
                    st.fans = LinkState::On;
                    st.fans_message = if !errors.is_empty() {
                        Some(errors.join("; "))
                    } else if none {
                        Some("LibreHardwareMonitor shows no fan controls. Run it as administrator so it can reach the motherboard.".into())
                    } else {
                        None
                    };
                    st.controls = controls;
                    st.rpm = rpm;
                    st.cpu_temp = cpu;
                    st.gpu_temp = gpu;
                });
            }
            Err(e) => {
                let profile = s.fans.profile;
                self.set_status(|st| {
                    st.fans = if profile == FanProfile::Auto { LinkState::Off } else { LinkState::Missing };
                    st.fans_message = Some(e);
                    st.controls.clear();
                    st.rpm.clear();
                    st.cpu_temp = None;
                    st.gpu_temp = None;
                });
            }
        }
        // Coolers and hubs: the profile once, readings while the page is open.
        if !s.fans.coolers {
            self.set_status(|st| st.coolers = LinkState::Off);
            return;
        }
        let lq = run.exe.get_or_insert_with(|| self.liquidctl(&s.fans).map(|l| l.exe)).clone().map(|exe| liquidctl::Liquidctl { exe });
        let Some(lq) = lq else {
            self.set_status(|st| {
                st.coolers = LinkState::Missing;
                st.coolers_message = None;
            });
            return;
        };
        // Opening the page changes nothing: a profile goes to the coolers
        // only when one was picked (or Auto picked again after one).
        let sent = *self.cooler_profile.lock();
        let apply = if s.fans.profile == FanProfile::Auto { sent.is_some_and(|p| p != FanProfile::Auto) } else { sent != Some(s.fans.profile) };
        let read = held && run.status_at.is_none_or(|t| t.elapsed() > Duration::from_secs(6));
        if run.devices.is_none() && (apply || read) {
            let _ = lq.initialize();
            run.devices = Some(lq.list().unwrap_or_default());
        }
        let devices = run.devices.clone().unwrap_or_default();
        let mut errors = Vec::new();
        if apply && !devices.is_empty() {
            for d in &devices {
                let pump = lq.set_speed(&d.address, "pump", fans::liquid_curve(s.fans.profile, FanRole::Pump));
                let fan = lq.set_speed(&d.address, "fan", fans::liquid_curve(s.fans.profile, FanRole::Case));
                if pump.is_err() && fan.is_err() {
                    // A hub without a temperature probe: one duty per profile.
                    let duty = match s.fans.profile {
                        FanProfile::Silent => 35.0,
                        FanProfile::Auto => 55.0,
                        FanProfile::Fast => 85.0,
                    };
                    if let Err(e) = lq.set_duty(&d.address, "sync", duty) {
                        errors.push(format!("{}: {e}", d.description));
                    }
                }
            }
            *self.cooler_profile.lock() = Some(s.fans.profile);
        }
        let mut list = devices;
        let mut liquid = None;
        if read {
            run.status_at = Some(Instant::now());
            if let Ok(status) = lq.status() {
                run.readings = status;
            }
        }
        for d in &mut list {
            if let Some((_, r)) = run.readings.iter().find(|(a, _)| a == &d.address) {
                d.status = r.clone();
                liquid = liquid.or_else(|| r.iter().find(|x| x.key.to_ascii_lowercase().contains("liquid")).map(|x| x.value as f32));
            }
        }
        self.set_status(|st| {
            st.coolers = LinkState::On;
            st.coolers_message = if errors.is_empty() { None } else { Some(errors.join("; ")) };
            st.cooler_devices = list;
            st.liquid_temp = liquid;
        });
    }
}

#[derive(Default)]
struct LightRun {
    client: Option<openrgb::Client>,
    controllers: Vec<Controller>,
    /// Indexes OmniHub lights.
    lit: Vec<u32>,
    chosen_for: (Vec<String>, bool),
    sent: HashMap<u32, Vec<Rgb>>,
    last_full: Option<Instant>,
    retry_at: Option<Instant>,
    t0: Option<Instant>,
    lease_at: Option<Instant>,
    level: f32,
    beat: f32,
}

#[derive(Default)]
struct FanRun {
    next: Option<Instant>,
    /// liquidctl's program (looked for once per run).
    exe: Option<Option<PathBuf>>,
    devices: Option<Vec<liquidctl::LqDevice>>,
    status_at: Option<Instant>,
    readings: Vec<(String, Vec<liquidctl::Reading>)>,
}

/// JSON merge patch (RFC 7386).
fn merge(target: &mut serde_json::Value, patch: &serde_json::Value) {
    match (target, patch) {
        (serde_json::Value::Object(t), serde_json::Value::Object(p)) => {
            for (k, v) in p {
                if v.is_null() {
                    t.remove(k);
                } else {
                    merge(t.entry(k.clone()).or_insert(serde_json::Value::Null), v);
                }
            }
        }
        (t, p) => *t = p.clone(),
    }
}

fn sanitize(s: &mut RigSettings) {
    if s.lighting.port == 0 {
        s.lighting.port = openrgb::DEFAULT_PORT;
    }
    s.lighting.skip.truncate(200);
    s.fans.skip.truncate(200);
    if s.fans.lhm_url.trim().is_empty() {
        s.fans.lhm_url = lhm::DEFAULT_URL.into();
    }
    s.screens.lcd_brightness = s.screens.lcd_brightness.min(100);
    let m = &mut s.model;
    m.front_fans = m.front_fans.min(4);
    m.top_fans = m.top_fans.min(3);
    m.rear_fans = m.rear_fans.min(1);
    m.bottom_fans = m.bottom_fans.min(3);
    if !matches!(m.case_color.as_str(), "black" | "white") {
        m.case_color = "black".into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hub(dir: &Path) -> Arc<RigHub> {
        let events = EventBus::new();
        RigHub::new(dir, events.clone(), crate::media::visual::VisualHub::new(events, true))
    }

    #[test]
    fn settings_merge_save_and_sanitize() {
        let dir = tempfile::tempdir().unwrap();
        let h = hub(dir.path());
        assert_eq!(h.settings(), RigSettings::default());
        let s = h.update(&serde_json::json!({ "model": { "frontFans": 9, "caseColor": "pink" }, "screens": { "monitorShow": { "kind": "stats" } } })).unwrap();
        assert_eq!((s.model.front_fans, s.model.case_color.as_str()), (4, "black"));
        assert_eq!(s.screens.monitor_show, ScreenShow::Stats);
        assert!(h.update(&serde_json::json!({ "fans": { "profile": "loud" } })).is_err());
        let again = hub(dir.path());
        assert_eq!(again.settings().screens.monitor_show, ScreenShow::Stats, "kept on disk");
        let p = h.save_profile(LightProfile { name: "  Mine ".into(), colors: vec!["#ff0000".into(), "nope".into()], speed: 9.0, ..Default::default() }).unwrap();
        assert!(p.id.starts_with("my-") && p.name == "Mine" && p.colors == ["#ff0000"] && p.speed == 3.0);
        assert_eq!(h.settings().lighting.profile, p.id);
        assert!(h.profiles().iter().any(|q| q.id == p.id));
        h.delete_profile(&p.id).unwrap();
        assert_eq!(h.settings().lighting.profile, "aurora");
        // The worker started for the lease and stops again on its own.
        h.hold();
    }

    /// The lights reach a (pretend) OpenRGB and follow the profile.
    #[test]
    fn lights_follow_the_profile() {
        let dir = tempfile::tempdir().unwrap();
        let (addr, rx) = openrgb::tests::fake_server(vec![openrgb::tests::describe(1, "DDR5", "I2C", &[("DRAM", 4)], Rgb(0, 0, 0)), openrgb::tests::describe(5, "Keyboard", "HID", &[("Keys", 3)], Rgb(0, 0, 0))]);
        let h = hub(dir.path());
        h.update(&serde_json::json!({ "lighting": { "enabled": true, "profile": "white", "port": addr.port() } })).unwrap();
        let t = Instant::now();
        let mut got = None;
        while t.elapsed() < Duration::from_secs(5) {
            if let Ok((dev, id, body)) = rx.recv_timeout(Duration::from_millis(200)) {
                if id == 1050 {
                    got = Some((dev, body));
                    break;
                }
            }
        }
        let (dev, body) = got.expect("colours sent");
        assert_eq!(dev, 0, "the memory, not the keyboard");
        assert_eq!(&body[6..10], &[255, 255, 255, 0]);
        let st = h.status();
        assert_eq!(st.lighting, LinkState::On);
        assert_eq!(st.devices.iter().map(|d| d.on).collect::<Vec<_>>(), [true, false], "peripherals are left alone");
        h.update(&serde_json::json!({ "lighting": { "enabled": false } })).unwrap();
    }
}
