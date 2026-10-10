//! What is playing on the PC — from Windows' media controls (the same
//! sessions the volume flyout shows: Spotify, the Apple Music app, browsers
//! with YouTube Music or SoundCloud, …) — with play/pause, next, previous
//! and seek, the cover art, and time-synced lyrics ([`lyrics`]).
//!
//! Phones stay in sync without polling: every `media:state` event carries
//! the track position, the PC time it was valid at, and the PC's clock, so a
//! phone can run the clock itself between events.

pub mod artwork;
pub mod audio;
pub mod eq;
pub mod lyrics;
pub mod mixer;
pub mod video;
pub mod visual;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::events::EventBus;
use lyrics::{Lyrics, LyricsCache};

/// What a media session reports at one moment.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot {
    /// The app playing (Windows app id or executable).
    pub app: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    /// Position and the time (Unix ms) it was valid at, when the player reports it.
    pub position: Option<(u64, i64)>,
    pub playing: bool,
    pub rate: f64,
    pub can_play_pause: bool,
    pub can_next: bool,
    pub can_previous: bool,
    pub can_seek: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    /// Identifies the track (changes when the song changes).
    pub key: String,
    pub app: String,
    /// Friendly name of the app ("Spotify", "Apple Music", "Brave").
    pub app_name: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
    /// Track position at `position_at` (PC time, Unix ms).
    pub position_ms: u64,
    pub position_at: i64,
    pub playing: bool,
    pub rate: f64,
    /// "player" when the app reports its position, "estimated" when OmniHub
    /// counts from the start of the track (some apps never report it).
    pub position_source: String,
    pub can_play_pause: bool,
    pub can_next: bool,
    pub can_previous: bool,
    pub can_seek: bool,
    /// Cover art, if any (fetch with this key).
    pub art: Option<String>,
}

impl MediaState {
    /// Position now, extrapolated while playing.
    pub fn position_at_time(&self, now: i64) -> u64 {
        if !self.playing {
            return self.position_ms;
        }
        let p = self.position_ms as f64 + (now - self.position_at).max(0) as f64 * self.rate;
        let p = p.max(0.0) as u64;
        if self.duration_ms > 0 { p.min(self.duration_ms) } else { p }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    /// Jump to `position_ms`.
    Seek,
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

fn friendly_app(app: &str) -> String {
    let a = app.to_lowercase();
    let known = [
        ("spotify", "Spotify"),
        ("applemusic", "Apple Music"),
        ("itunes", "iTunes"),
        ("zunemusic", "Media Player"),
        ("microsoft.media", "Media Player"),
        ("brave", "Brave"),
        ("chrome", "Chrome"),
        ("msedge", "Edge"),
        ("firefox", "Firefox"),
        ("opera", "Opera"),
        ("vlc", "VLC"),
        ("tidal", "TIDAL"),
        ("deezer", "Deezer"),
        ("amazonmusic", "Amazon Music"),
        ("youtube", "YouTube Music"),
        ("foobar", "foobar2000"),
        ("musicbee", "MusicBee"),
        ("omnihub", "OmniHub test player"),
    ];
    known.iter().find(|(k, _)| a.contains(k)).map(|(_, v)| v.to_string()).unwrap_or_else(|| {
        let base = app.rsplit(['\\', '/', '!']).next().unwrap_or(app);
        base.trim_end_matches(".exe").to_string()
    })
}

/// Turns snapshots into [`MediaState`], estimating the position for players
/// that do not report one, and decides when phones need an update.
#[derive(Default)]
pub struct Tracker {
    key: String,
    /// Estimated position: base, at (Unix ms), playing.
    est: (u64, i64, bool),
    last: Option<MediaState>,
    last_sent: i64,
}

pub fn track_key(s: &Snapshot) -> String {
    format!("{}\u{1f}{}\u{1f}{}\u{1f}{}", s.app, s.title, s.artist, s.album)
}

impl Tracker {
    /// The new state, and whether it should be announced now.
    pub fn update(&mut self, snap: Option<&Snapshot>, art: Option<String>, now: i64) -> (Option<MediaState>, bool) {
        let Some(s) = snap.filter(|s| !s.title.is_empty()) else {
            let changed = self.last.is_some();
            self.last = None;
            self.key.clear();
            return (None, changed);
        };
        let key = track_key(s);
        let new_track = key != self.key;
        if new_track {
            self.key = key.clone();
            self.est = (s.position.map_or(0, |(p, _)| p), now, s.playing);
        }
        // Trust the player when its timestamp is plausible (recent enough
        // to extrapolate from); otherwise count ourselves.
        let reported = s.position.filter(|(_, at)| {
            let age = now - at;
            (-5_000..=(s.duration_ms.max(60_000) as i64 + 5_000)).contains(&age) && (s.playing || age < 600_000)
        });
        let (position_ms, position_at, source) = match reported {
            Some((p, at)) => {
                self.est = (p, at, s.playing);
                (p, at, "player")
            }
            None => {
                let (base, at, was_playing) = self.est;
                let cur = if was_playing { base + (now - at).max(0) as u64 } else { base };
                let cur = if s.duration_ms > 0 { cur.min(s.duration_ms) } else { cur };
                self.est = (cur, now, s.playing);
                (cur, now, "estimated")
            }
        };
        let state = MediaState {
            key,
            app: s.app.clone(),
            app_name: friendly_app(&s.app),
            title: s.title.clone(),
            artist: s.artist.clone(),
            album: s.album.clone(),
            duration_ms: s.duration_ms,
            position_ms,
            position_at,
            playing: s.playing,
            rate: if s.rate > 0.0 { s.rate } else { 1.0 },
            position_source: source.into(),
            can_play_pause: s.can_play_pause,
            can_next: s.can_next,
            can_previous: s.can_previous,
            can_seek: s.can_seek,
            art,
        };
        let announce = match &self.last {
            None => true,
            Some(prev) => {
                let drift = (prev.position_at_time(now) as i64 - state.position_at_time(now) as i64).abs();
                new_track
                    || prev.playing != state.playing
                    || prev.art != state.art
                    || (prev.can_next, prev.can_previous, prev.can_seek, prev.duration_ms) != (state.can_next, state.can_previous, state.can_seek, state.duration_ms)
                    // A seek, or a player that drifted from our estimate.
                    || drift > 1_200
                    // Keep phones in step now and then.
                    || (state.playing && now - self.last_sent > 15_000)
            }
        };
        if announce {
            self.last_sent = now;
        }
        self.last = Some(state.clone());
        (Some(state), announce)
    }
}

/// Where snapshots come from.
pub trait MediaBackend: Send {
    fn snapshot(&mut self) -> Option<Snapshot>;
    /// Cover art of the current track: bytes and content type.
    fn art(&mut self) -> Option<(Vec<u8>, String)>;
    fn control(&mut self, action: Action, position_ms: u64) -> Result<(), String>;
}

/// No media sessions (platforms without them).
pub struct NoBackend;

impl MediaBackend for NoBackend {
    fn snapshot(&mut self) -> Option<Snapshot> {
        None
    }
    fn art(&mut self) -> Option<(Vec<u8>, String)> {
        None
    }
    fn control(&mut self, _: Action, _: u64) -> Result<(), String> {
        Err("No app is playing media on this PC.".into())
    }
}

/// A pretend player (`OMNIHUB_FAKE_MEDIA=1`) for development and tests.
pub struct FakeBackend {
    tracks: Vec<(&'static str, &'static str, &'static str, u64)>,
    index: usize,
    /// Position base and when it was set; playing.
    pos: (u64, i64),
    playing: bool,
}

impl Default for FakeBackend {
    fn default() -> Self {
        FakeBackend {
            tracks: vec![
                ("Daylight Drive", "The Test Signals", "Synthetic Summer", 200_000),
                ("Night Loop", "The Test Signals", "Synthetic Summer", 180_000),
                ("Static Bloom", "The Test Signals", "Synthetic Summer", 150_000),
                ("Unwritten Demo", "The Test Signals", "Synthetic Summer", 120_000),
            ],
            index: 0,
            pos: (0, now_ms()),
            playing: true,
        }
    }
}

/// Lyrics of the fake player's tracks (no network needed): words timed
/// with instrumental breaks, lines only, an instrumental, and none.
pub fn fake_lyrics(title: &str) -> Option<Lyrics> {
    let lrc = |src: &str| Some(Lyrics { lines: lyrics::parse_lrc(src), plain: None, instrumental: false, source: "OmniHub test lyrics".into() });
    match title {
        "Daylight Drive" => lrc(include_str!("testdata/daylight-drive.lrc")),
        "Night Loop" => lrc(include_str!("testdata/night-loop.lrc")),
        "Static Bloom" => Some(Lyrics { lines: Vec::new(), plain: None, instrumental: true, source: "OmniHub test lyrics".into() }),
        _ => None,
    }
}

impl FakeBackend {
    fn position(&self) -> u64 {
        let (base, at) = self.pos;
        let p = if self.playing { base + (now_ms() - at).max(0) as u64 } else { base };
        p.min(self.tracks[self.index].3)
    }
}

impl MediaBackend for FakeBackend {
    fn snapshot(&mut self) -> Option<Snapshot> {
        if self.position() >= self.tracks[self.index].3 {
            self.index = (self.index + 1) % self.tracks.len();
            self.pos = (0, now_ms());
        }
        let (t, a, al, d) = self.tracks[self.index];
        Some(Snapshot { app: "OmniHub.TestPlayer".into(), title: t.into(), artist: a.into(), album: al.into(), duration_ms: d, position: Some((self.position(), now_ms())), playing: self.playing, rate: 1.0, can_play_pause: true, can_next: true, can_previous: true, can_seek: true })
    }

    fn art(&mut self) -> Option<(Vec<u8>, String)> {
        let hue = [[255u8, 120, 60], [70, 90, 255], [40, 200, 150], [230, 60, 140]][self.index % 4];
        let img = image::RgbImage::from_fn(300, 300, |x, y| {
            let k = (x + y) as f32 / 600.0;
            image::Rgb([(hue[0] as f32 * (1.0 - k) + 30.0 * k) as u8, (hue[1] as f32 * (1.0 - k) + 20.0 * k) as u8, (hue[2] as f32 * (1.0 - k) + 60.0 * k) as u8])
        });
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).ok()?;
        Some((png, "image/png".into()))
    }

    fn control(&mut self, action: Action, position_ms: u64) -> Result<(), String> {
        let p = self.position();
        match action {
            Action::Play => self.playing = true,
            Action::Pause => self.playing = false,
            Action::Toggle => self.playing = !self.playing,
            Action::Next => {
                self.index = (self.index + 1) % self.tracks.len();
                self.pos = (0, now_ms());
                return Ok(());
            }
            Action::Previous => {
                if p < 3_000 {
                    self.index = (self.index + self.tracks.len() - 1) % self.tracks.len();
                }
                self.pos = (0, now_ms());
                return Ok(());
            }
            Action::Seek => {
                self.pos = (position_ms.min(self.tracks[self.index].3), now_ms());
                return Ok(());
            }
        }
        self.pos = (p, now_ms());
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum LyricsStatus {
    /// Found (lines may be empty with only plain text).
    Ready { lyrics: Lyrics },
    Searching,
    /// No lyrics known for this track.
    None,
    /// Looking up lyrics online is turned off.
    Off,
    NothingPlaying,
}

/// The playing track's cover: the player's thumbnail, or a sharper copy (`hd`).
struct Art {
    key: String,
    bytes: Arc<Vec<u8>>,
    mime: String,
    hd: bool,
}

impl Art {
    fn id(&self) -> String {
        art_id(&if self.hd { format!("{}\u{1f}hd", self.key) } else { self.key.clone() })
    }
}

struct Inner {
    backend: Box<dyn MediaBackend>,
    tracker: Tracker,
    state: Option<MediaState>,
    art: Option<Art>,
}

pub struct MediaHub {
    inner: Mutex<Inner>,
    events: EventBus,
    cache: LyricsCache,
    lyrics: Mutex<HashMap<String, LyricsStatus>>,
    fake: bool,
    /// The user's .lrc files.
    lrc_folder: Mutex<Option<PathBuf>>,
    /// Look the current track's lyrics up again (the folder changed).
    refetch: AtomicBool,
    /// Look sharper covers up online (settings).
    hires_art: AtomicBool,
    covers: artwork::ArtCache,
    /// Music videos per track (looked up when Aurora asks).
    videos: Mutex<HashMap<String, video::VideoStatus>>,
    video_cache: video::VideoCache,
}

impl MediaHub {
    pub fn new(data_dir: &std::path::Path, events: EventBus, fake: bool) -> Arc<Self> {
        let fake = fake || std::env::var("OMNIHUB_FAKE_MEDIA").is_ok_and(|v| v == "1");
        let backend: Box<dyn MediaBackend> = if fake {
            Box::new(FakeBackend::default())
        } else {
            #[cfg(windows)]
            {
                Box::new(win::SessionBackend::new())
            }
            #[cfg(not(windows))]
            {
                Box::new(NoBackend)
            }
        };
        Arc::new(MediaHub { inner: Mutex::new(Inner { backend, tracker: Tracker::default(), state: None, art: None }), events, cache: LyricsCache::new(data_dir.join("lyrics")), lyrics: Mutex::new(HashMap::new()), fake, lrc_folder: Mutex::new(None), refetch: AtomicBool::new(false), hires_art: AtomicBool::new(false), covers: artwork::ArtCache::new(data_dir.join("covers")), videos: Mutex::new(HashMap::new()), video_cache: video::VideoCache::new(data_dir.join("videos")) })
    }

    /// Whether this is the pretend player (the mixer and open apps pretend too).
    pub fn is_fake(&self) -> bool {
        self.fake
    }

    /// Poll the player a few times a second (cheap) and announce changes.
    pub fn start(self: &Arc<Self>, lyrics_online: impl Fn() -> bool + Send + 'static) {
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("media".into())
            .spawn(move || {
                #[cfg(windows)]
                // SAFETY: initialises WinRT for this thread.
                let _ = unsafe { windows::Win32::System::WinRT::RoInitialize(windows::Win32::System::WinRT::RO_INIT_MULTITHREADED) };
                loop {
                    let Some(hub) = weak.upgrade() else { return };
                    hub.poll(&lyrics_online);
                    drop(hub);
                    std::thread::sleep(Duration::from_millis(250));
                }
            })
            .ok();
    }

    fn poll(self: &Arc<Self>, lyrics_online: &dyn Fn() -> bool) {
        let now = now_ms();
        let mut g = self.inner.lock();
        let snap = g.backend.snapshot();
        let key = snap.as_ref().map(track_key);
        // Fetch the artwork once per track (and a sharper copy, when allowed).
        if let (Some(k), Some(sn)) = (&key, &snap) {
            if g.art.as_ref().map(|a| &a.key) != Some(k) {
                let (bytes, mime) = g.backend.art().unwrap_or_default();
                let bytes = Arc::new(bytes);
                g.art = Some(Art { key: k.clone(), bytes: bytes.clone(), mime, hd: false });
                if !self.fake && self.hires_art.load(Ordering::Relaxed) {
                    self.upgrade_art(k.clone(), sn.title.clone(), sn.artist.clone(), sn.album.clone(), sn.duration_ms, bytes);
                }
            }
        }
        let art = g.art.as_ref().filter(|a| !a.bytes.is_empty()).map(Art::id);
        let (state, announce) = g.tracker.update(snap.as_ref(), art, now);
        let new_track = state.as_ref().map(|s| &s.key) != g.state.as_ref().map(|s| &s.key);
        g.state = state.clone();
        drop(g);
        if new_track || self.refetch.swap(false, Ordering::Relaxed) {
            if let Some(s) = &state {
                self.fetch_lyrics(s.clone(), lyrics_online());
            }
        }
        if announce {
            self.events.emit("media:state", serde_json::json!({ "state": state, "nowMs": now_ms() }));
        }
    }

    pub fn state(&self) -> Option<MediaState> {
        self.inner.lock().state.clone()
    }

    /// Cover art for `id` (as given in [`MediaState::art`]).
    pub fn art(&self, id: &str) -> Option<(Arc<Vec<u8>>, String)> {
        let g = self.inner.lock();
        g.art.as_ref().filter(|a| !a.bytes.is_empty() && a.id() == id).map(|a| (a.bytes.clone(), a.mime.clone()))
    }

    /// Cover art as a `data:` URL (for the desktop window).
    pub fn art_data_url(&self, id: &str) -> Option<String> {
        use base64::Engine as _;
        self.art(id).map(|(b, mime)| format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(&*b)))
    }

    pub fn control(&self, action: Action, position_ms: u64) -> Result<(), String> {
        let mut g = self.inner.lock();
        if g.state.is_none() && !matches!(action, Action::Play | Action::Toggle) {
            return Err("Nothing is playing on the PC.".into());
        }
        g.backend.control(action, position_ms)
    }

    pub fn lyrics(&self) -> LyricsStatus {
        let Some(s) = self.state() else { return LyricsStatus::NothingPlaying };
        self.lyrics.lock().get(&s.key).cloned().unwrap_or(LyricsStatus::Searching)
    }

    /// The playing song's music video (see [`video`]): looked up the first time
    /// Aurora asks, then announced with `media:video`. `allowed` is the setting.
    pub fn video(self: &Arc<Self>, allowed: bool) -> video::VideoStatus {
        let Some(s) = self.state() else { return video::VideoStatus::NothingPlaying };
        if !allowed {
            return video::VideoStatus::Off;
        }
        let mut g = self.videos.lock();
        if let Some(st) = g.get(&s.key) {
            return st.clone();
        }
        if self.fake {
            // The pretend player's songs have no videos.
            return video::VideoStatus::None;
        }
        if g.len() > 200 {
            g.clear();
        }
        g.insert(s.key.clone(), video::VideoStatus::Searching);
        drop(g);
        let hub = self.clone();
        std::thread::spawn(move || {
            let cache_key = format!("{}\u{1f}{}\u{1f}{}", s.title.to_lowercase(), s.artist.to_lowercase(), s.duration_ms / 4000);
            let now = now_ms().max(0) as u64;
            let found = match hub.video_cache.get(&cache_key, now) {
                Some(v) => Some(v),
                None => match video::find(&s.title, &s.artist, s.duration_ms, &video::http_get) {
                    Ok(v) => {
                        hub.video_cache.put(&cache_key, &v, now);
                        Some(v)
                    }
                    Err(e) => {
                        // Offline: no video this time; the next song asks again.
                        tracing::info!("music video lookup failed: {e}");
                        None
                    }
                },
            };
            let st = match found {
                Some(videos) if !videos.is_empty() => video::VideoStatus::Found { videos },
                _ => video::VideoStatus::None,
            };
            hub.videos.lock().insert(s.key.clone(), st);
            hub.events.emit("media:video", serde_json::json!({ "key": s.key }));
        });
        video::VideoStatus::Searching
    }

    /// Look sharper covers up online (settings).
    pub fn set_hires_art(&self, on: bool) {
        self.hires_art.store(on, Ordering::Relaxed);
    }

    /// Swap the player's small thumbnail for the full-size cover (3000×3000 for
    /// most songs), from the cache or the iTunes catalogue (see [`artwork`]).
    fn upgrade_art(self: &Arc<Self>, key: String, title: String, artist: String, album: String, duration_ms: u64, thumb: Arc<Vec<u8>>) {
        let hub = self.clone();
        std::thread::spawn(move || {
            // "full": covers cached before 0.6 were 1200×1200; these are the originals.
            let cache_key = format!("full\u{1f}{}\u{1f}{}\u{1f}{}\u{1f}{}", title.to_lowercase(), artist.to_lowercase(), album.to_lowercase(), duration_ms / 4000);
            let bytes = match hub.covers.get(&cache_key) {
                Some(found) => found,
                None => {
                    let found = match artwork::find(&title, &artist, &album, duration_ms, &lyrics::http_get).and_then(|u| u.map(|u| artwork::download(&u)).transpose()) {
                        Ok(found) => found,
                        Err(e) => {
                            // Offline or refused: try again next time.
                            tracing::info!("cover lookup failed: {e}");
                            return;
                        }
                    };
                    // Only a picture of the same cover (when the player gave one to compare).
                    let found = found.filter(|b| artwork::mime_of(b).is_some()).filter(|b| thumb.is_empty() || artwork::difference(&thumb, b).is_some_and(|d| d < 0.16));
                    hub.covers.put(&cache_key, found.as_deref());
                    found
                }
            };
            let Some(bytes) = bytes else { return };
            let Some(mime) = artwork::mime_of(&bytes) else { return };
            let mut g = hub.inner.lock();
            if g.art.as_ref().is_some_and(|a| a.key == key) {
                g.art = Some(Art { key, bytes: Arc::new(bytes), mime: mime.into(), hd: true });
            }
        });
    }

    /// Where the user keeps .lrc files (checked before anything else).
    pub fn set_lrc_folder(&self, dir: Option<PathBuf>) {
        let mut g = self.lrc_folder.lock();
        if *g != dir {
            *g = dir;
            self.lyrics.lock().clear();
            self.refetch.store(true, Ordering::Relaxed);
        }
    }

    /// Store a track's lyrics status and tell the views.
    fn put_lyrics(&self, key: &str, st: LyricsStatus) {
        let mut g = self.lyrics.lock();
        if g.len() > 200 {
            g.clear();
        }
        g.insert(key.to_string(), st);
        drop(g);
        self.events.emit("media:lyrics", serde_json::json!({ "key": key }));
    }

    fn fetch_lyrics(self: &Arc<Self>, s: MediaState, online: bool) {
        if self.lyrics.lock().contains_key(&s.key) {
            return;
        }
        let Some(dir) = self.lrc_folder.lock().clone() else { return self.fetch_elsewhere(s, online) };
        // The user's files first, off the polling thread.
        self.put_lyrics(&s.key, LyricsStatus::Searching);
        let hub = self.clone();
        std::thread::spawn(move || {
            let found = lyrics::find_local(&dir, &s.title, &s.artist, s.duration_ms);
            if hub.lrc_folder.lock().as_ref() != Some(&dir) {
                return; // the folder changed meanwhile; that lookup wins
            }
            match found {
                Some(lyrics) => hub.put_lyrics(&s.key, LyricsStatus::Ready { lyrics }),
                None => hub.fetch_elsewhere(s, online),
            }
        });
    }

    /// The test lyrics, the cache, then LRCLIB.
    fn fetch_elsewhere(self: &Arc<Self>, s: MediaState, online: bool) {
        let ready = |found: Option<Lyrics>| found.map_or(LyricsStatus::None, |lyrics| LyricsStatus::Ready { lyrics });
        if self.fake {
            return self.put_lyrics(&s.key, ready(fake_lyrics(&s.title)));
        }
        let cache_key = format!("{}\u{1f}{}\u{1f}{}", s.title.to_lowercase(), s.artist.to_lowercase(), s.duration_ms / 2000);
        if let Some(cached) = self.cache.get(&cache_key) {
            return self.put_lyrics(&s.key, ready(cached));
        }
        if !online {
            return self.put_lyrics(&s.key, LyricsStatus::Off);
        }
        self.put_lyrics(&s.key, LyricsStatus::Searching);
        let hub = self.clone();
        std::thread::spawn(move || {
            let st = match lyrics::lookup(&s.title, &s.artist, &s.album, s.duration_ms, &lyrics::http_get) {
                Ok(found) => {
                    hub.cache.put(&cache_key, &found);
                    ready(found)
                }
                Err(e) => {
                    tracing::info!("lyrics lookup failed: {e}");
                    LyricsStatus::None
                }
            };
            hub.put_lyrics(&s.key, st);
        });
    }
}

fn art_id(key: &str) -> String {
    use sha2::Digest;
    hex::encode(&sha2::Sha256::digest(key.as_bytes())[..8])
}

#[cfg(windows)]
mod win {
    use windows::Media::Control::{GlobalSystemMediaTransportControlsSession as Session, GlobalSystemMediaTransportControlsSessionManager as Manager, GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status};
    use windows::Storage::Streams::DataReader;

    use super::*;

    /// Windows' Global System Media Transport Controls.
    pub struct SessionBackend {
        manager: Option<Manager>,
        last_try: std::time::Instant,
    }

    impl SessionBackend {
        pub fn new() -> Self {
            SessionBackend { manager: None, last_try: std::time::Instant::now() - Duration::from_secs(60) }
        }

        fn session(&mut self) -> Option<Session> {
            if self.manager.is_none() && self.last_try.elapsed() > Duration::from_secs(5) {
                self.last_try = std::time::Instant::now();
                self.manager = Manager::RequestAsync().and_then(|op| op.join()).ok();
            }
            self.manager.as_ref()?.GetCurrentSession().ok()
        }
    }

    /// Windows FILETIME-style DateTime (100 ns since 1601) → Unix ms.
    fn unix_ms(universal: i64) -> i64 {
        universal / 10_000 - 11_644_473_600_000
    }

    impl MediaBackend for SessionBackend {
        fn snapshot(&mut self) -> Option<Snapshot> {
            let s = self.session()?;
            let props = s.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
            let info = s.GetPlaybackInfo().ok()?;
            let tl = s.GetTimelineProperties().ok()?;
            let controls = info.Controls().ok();
            let c = |f: &dyn Fn(&windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackControls) -> windows::core::Result<bool>| controls.as_ref().and_then(|c| f(c).ok()).unwrap_or(false);
            let start = tl.StartTime().map(|t| t.Duration).unwrap_or(0);
            let end = tl.EndTime().map(|t| t.Duration).unwrap_or(0);
            let duration_ms = ((end - start).max(0) / 10_000) as u64;
            let pos = tl.Position().map(|t| ((t.Duration - start).max(0) / 10_000) as u64).ok();
            let at = tl.LastUpdatedTime().map(|t| unix_ms(t.UniversalTime)).ok();
            let rate = info.PlaybackRate().and_then(|r| r.Value()).unwrap_or(1.0);
            Some(Snapshot {
                app: s.SourceAppUserModelId().map(|h| h.to_string()).unwrap_or_default(),
                title: props.Title().map(|h| h.to_string()).unwrap_or_default(),
                artist: props.Artist().map(|h| h.to_string()).unwrap_or_default(),
                album: props.AlbumTitle().map(|h| h.to_string()).unwrap_or_default(),
                duration_ms,
                // Some players never fill the timeline in (all zeros).
                position: match (pos, at) {
                    (Some(p), Some(at)) if duration_ms > 0 && at > 0 => Some((p, at)),
                    _ => None,
                },
                playing: info.PlaybackStatus().is_ok_and(|st| st == Status::Playing),
                rate,
                can_play_pause: c(&|c| c.IsPlayPauseToggleEnabled()) || c(&|c| c.IsPlayEnabled()),
                can_next: c(&|c| c.IsNextEnabled()),
                can_previous: c(&|c| c.IsPreviousEnabled()),
                can_seek: c(&|c| c.IsPlaybackPositionEnabled()),
            })
        }

        fn art(&mut self) -> Option<(Vec<u8>, String)> {
            let s = self.session()?;
            let props = s.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
            let thumb = props.Thumbnail().ok()?;
            let stream = thumb.OpenReadAsync().ok()?.join().ok()?;
            let size = stream.Size().ok()? as u32;
            if size == 0 || size > 8 << 20 {
                return None;
            }
            let mime = stream.ContentType().map(|h| h.to_string()).unwrap_or_else(|_| "image/png".into());
            let reader = DataReader::CreateDataReader(&stream).ok()?;
            reader.LoadAsync(size).ok()?.join().ok()?;
            let mut buf = vec![0u8; size as usize];
            reader.ReadBytes(&mut buf).ok()?;
            Some((buf, if mime.starts_with("image/") { mime } else { "image/png".into() }))
        }

        fn control(&mut self, action: Action, position_ms: u64) -> Result<(), String> {
            let s = self.session().ok_or("No app is playing media on this PC.")?;
            let e = |e: windows::core::Error| e.message().to_string();
            let ok = match action {
                Action::Play => s.TryPlayAsync().map_err(e)?.join().map_err(e)?,
                Action::Pause => s.TryPauseAsync().map_err(e)?.join().map_err(e)?,
                Action::Toggle => s.TryTogglePlayPauseAsync().map_err(e)?.join().map_err(e)?,
                Action::Next => s.TrySkipNextAsync().map_err(e)?.join().map_err(e)?,
                Action::Previous => s.TrySkipPreviousAsync().map_err(e)?.join().map_err(e)?,
                Action::Seek => {
                    let start = s.GetTimelineProperties().and_then(|t| t.StartTime()).map(|t| t.Duration).unwrap_or(0);
                    s.TryChangePlaybackPositionAsync(start + position_ms as i64 * 10_000).map_err(e)?.join().map_err(e)?
                }
            };
            if ok { Ok(()) } else { Err("The player did not accept that.".into()) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(title: &str, pos: Option<(u64, i64)>, playing: bool) -> Snapshot {
        Snapshot { app: "Spotify.exe".into(), title: title.into(), artist: "A".into(), album: "B".into(), duration_ms: 200_000, position: pos, playing, rate: 1.0, can_play_pause: true, can_next: true, can_previous: true, can_seek: true }
    }

    #[test]
    fn trusts_reported_positions_and_spots_seeks() {
        let mut t = Tracker::default();
        let (s, a) = t.update(Some(&snap("One", Some((10_000, 1_000_000)), true)), None, 1_000_500);
        let s = s.unwrap();
        assert!(a);
        assert_eq!((s.position_ms, s.position_at, s.position_source.as_str(), s.app_name.as_str()), (10_000, 1_000_000, "player", "Spotify"));
        assert_eq!(s.position_at_time(1_002_000), 12_000);
        // Same track, consistent position: nothing to announce.
        let (_, a) = t.update(Some(&snap("One", Some((11_000, 1_001_000)), true)), None, 1_001_250);
        assert!(!a);
        // A seek is announced.
        let (_, a) = t.update(Some(&snap("One", Some((90_000, 1_001_500)), true)), None, 1_001_500);
        assert!(a);
        // Pausing is announced; a paused position does not move.
        let (s, a) = t.update(Some(&snap("One", Some((90_200, 1_001_700)), false)), None, 1_001_800);
        assert!(a);
        assert_eq!(s.unwrap().position_at_time(1_100_000), 90_200);
        // A new track is announced.
        let (s, a) = t.update(Some(&snap("Two", Some((0, 1_002_000)), true)), None, 1_002_000);
        assert!(a && s.unwrap().title == "Two");
        // Nothing playing.
        let (s, a) = t.update(None, None, 1_003_000);
        assert!(s.is_none() && a);
    }

    #[test]
    fn estimates_when_the_player_reports_nothing() {
        let mut t = Tracker::default();
        let (s, _) = t.update(Some(&snap("One", None, true)), None, 5_000);
        assert_eq!(s.unwrap().position_source, "estimated");
        let (s, _) = t.update(Some(&snap("One", None, true)), None, 15_000);
        assert_eq!(s.unwrap().position_at_time(15_000), 10_000);
        // Paused: the estimate stops.
        t.update(Some(&snap("One", None, false)), None, 20_000);
        let (s, _) = t.update(Some(&snap("One", None, false)), None, 60_000);
        assert_eq!(s.unwrap().position_at_time(60_000), 15_000);
        // A stale timestamp from the player is not trusted.
        let (s, _) = t.update(Some(&snap("One", Some((1, 1)), true)), None, 10_000_000);
        assert_eq!(s.unwrap().position_source, "estimated");
    }

    #[test]
    fn own_lrc_files_come_first_and_never_go_stale() {
        let data = tempfile::tempdir().unwrap();
        let hub = MediaHub::new(data.path(), EventBus::new(), true);
        let wait_ready = |hub: &Arc<MediaHub>| {
            let t0 = std::time::Instant::now();
            loop {
                hub.poll(&|| false);
                if let LyricsStatus::Ready { lyrics } = hub.lyrics() {
                    return lyrics;
                }
                assert!(t0.elapsed() < Duration::from_secs(5), "{:?}", hub.lyrics());
                std::thread::sleep(Duration::from_millis(20));
            }
        };
        assert_eq!(wait_ready(&hub).source, "OmniHub test lyrics");
        let lrc = tempfile::tempdir().unwrap();
        std::fs::write(lrc.path().join("The Test Signals - Daylight Drive.lrc"), "[length:03:20]\n[00:01.00]mine").unwrap();
        hub.set_lrc_folder(Some(lrc.path().to_path_buf()));
        let l = wait_ready(&hub);
        assert_eq!((l.lines[0].text.as_str(), l.source.contains("Daylight Drive.lrc")), ("mine", true));
        // The next track has no file: its own lyrics, never the last track's.
        hub.control(Action::Next, 0).unwrap();
        let l = wait_ready(&hub);
        assert_eq!(hub.state().unwrap().title, "Night Loop");
        assert!(l.lines.iter().all(|x| x.text != "mine"));
        // An instrumental, then a track without lyrics.
        hub.control(Action::Next, 0).unwrap();
        assert!(wait_ready(&hub).instrumental);
        hub.control(Action::Next, 0).unwrap();
        let t0 = std::time::Instant::now();
        while !matches!(hub.lyrics(), LyricsStatus::None) {
            hub.poll(&|| false);
            assert!(t0.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn fake_player_and_hub() {
        let mut f = FakeBackend::default();
        let s = f.snapshot().unwrap();
        assert_eq!(s.title, "Daylight Drive");
        f.control(Action::Pause, 0).unwrap();
        assert!(!f.snapshot().unwrap().playing);
        f.control(Action::Seek, 50_000).unwrap();
        assert_eq!(f.snapshot().unwrap().position.unwrap().0, 50_000);
        f.control(Action::Next, 0).unwrap();
        assert_eq!(f.snapshot().unwrap().title, "Night Loop");
        assert!(f.art().unwrap().0.starts_with(b"\x89PNG"));
        assert!(fake_lyrics("Daylight Drive").unwrap().lines.iter().any(|l| !l.words.is_empty()));
        assert!(fake_lyrics("Night Loop").unwrap().lines.iter().all(|l| l.words.is_empty()));
        assert!(fake_lyrics("Static Bloom").unwrap().instrumental);
        assert!(fake_lyrics("Unwritten Demo").is_none());
        assert_eq!(friendly_app("AppleInc.AppleMusicWin_nzyj5cx40ttqa!App"), "Apple Music");
        assert_eq!(friendly_app("C:\\Tools\\player.exe"), "player");
    }
}
