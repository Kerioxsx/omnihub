//! What is playing on the PC — from Windows' media controls (the same
//! sessions the volume flyout shows: Spotify, the Apple Music app, browsers
//! with YouTube Music or SoundCloud, …) — with play/pause, next, previous
//! and seek, the cover art, and time-synced lyrics ([`lyrics`]).
//!
//! Phones stay in sync without polling: every `media:state` event carries
//! the track position, the PC time it was valid at, and the PC's clock, so a
//! phone can run the clock itself between events.

pub mod audio;
pub mod eq;
pub mod lyrics;
pub mod mixer;

use std::collections::HashMap;
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
        FakeBackend { tracks: vec![("Daylight Drive", "The Test Signals", "Synthetic Summer", 200_000), ("Night Loop", "The Test Signals", "Synthetic Summer", 180_000)], index: 0, pos: (0, now_ms()), playing: true }
    }
}

/// Lyrics of the fake player's tracks (no network needed).
pub fn fake_lyrics(title: &str) -> Option<Lyrics> {
    let lines = (0..40).map(|i| format!("[{:02}:{:02}.00]{} line {}", (i * 4 + 2) / 60, (i * 4 + 2) % 60, title, i + 1)).collect::<Vec<_>>().join("\n");
    Some(Lyrics { lines: lyrics::parse_lrc(&lines), plain: None, instrumental: false, source: "OmniHub test".into() })
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
        let hue = if self.index == 0 { [255u8, 120, 60] } else { [70, 90, 255] };
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

struct Inner {
    backend: Box<dyn MediaBackend>,
    tracker: Tracker,
    state: Option<MediaState>,
    art: Option<(String, Arc<Vec<u8>>, String)>,
}

pub struct MediaHub {
    inner: Mutex<Inner>,
    events: EventBus,
    cache: LyricsCache,
    lyrics: Mutex<HashMap<String, LyricsStatus>>,
    fake: bool,
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
        Arc::new(MediaHub { inner: Mutex::new(Inner { backend, tracker: Tracker::default(), state: None, art: None }), events, cache: LyricsCache::new(data_dir.join("lyrics")), lyrics: Mutex::new(HashMap::new()), fake })
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
        // Fetch the artwork once per track.
        if key.is_some() && g.art.as_ref().map(|(k, _, _)| Some(k)) != Some(key.as_ref()) {
            let art = g.backend.art();
            g.art = art.map(|(b, mime)| (key.clone().unwrap_or_default(), Arc::new(b), mime));
            if g.art.is_none() {
                // Remember "no art" for this track too.
                g.art = Some((key.clone().unwrap_or_default(), Arc::new(Vec::new()), String::new()));
            }
        }
        let art = g.art.as_ref().filter(|(_, b, _)| !b.is_empty()).map(|(k, _, _)| art_id(k));
        let (state, announce) = g.tracker.update(snap.as_ref(), art, now);
        let new_track = state.as_ref().map(|s| &s.key) != g.state.as_ref().map(|s| &s.key);
        g.state = state.clone();
        drop(g);
        if new_track {
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
        g.art.as_ref().filter(|(k, b, _)| !b.is_empty() && art_id(k) == id).map(|(_, b, m)| (b.clone(), m.clone()))
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

    fn fetch_lyrics(self: &Arc<Self>, s: MediaState, online: bool) {
        if self.lyrics.lock().contains_key(&s.key) {
            return;
        }
        let set = |hub: &MediaHub, st: LyricsStatus| {
            let mut g = hub.lyrics.lock();
            if g.len() > 200 {
                g.clear();
            }
            g.insert(s.key.clone(), st);
        };
        if self.fake {
            set(self, fake_lyrics(&s.title).map_or(LyricsStatus::None, |lyrics| LyricsStatus::Ready { lyrics }));
            return;
        }
        let cache_key = format!("{}\u{1f}{}\u{1f}{}", s.title.to_lowercase(), s.artist.to_lowercase(), s.duration_ms / 2000);
        if let Some(cached) = self.cache.get(&cache_key) {
            set(self, cached.map_or(LyricsStatus::None, |lyrics| LyricsStatus::Ready { lyrics }));
            return;
        }
        if !online {
            set(self, LyricsStatus::Off);
            return;
        }
        set(self, LyricsStatus::Searching);
        let hub = self.clone();
        std::thread::spawn(move || {
            let res = lyrics::lookup(&s.title, &s.artist, &s.album, s.duration_ms, &lyrics::http_get);
            let st = match res {
                Ok(found) => {
                    hub.cache.put(&cache_key, &found);
                    found.map_or(LyricsStatus::None, |lyrics| LyricsStatus::Ready { lyrics })
                }
                Err(e) => {
                    tracing::info!("lyrics lookup failed: {e}");
                    LyricsStatus::None
                }
            };
            hub.lyrics.lock().insert(s.key.clone(), st);
            hub.events.emit("media:lyrics", serde_json::json!({ "key": s.key }));
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
        assert_eq!(fake_lyrics("X").unwrap().lines.len(), 40);
        assert_eq!(friendly_app("AppleInc.AppleMusicWin_nzyj5cx40ttqa!App"), "Apple Music");
        assert_eq!(friendly_app("C:\\Tools\\player.exe"), "player");
    }
}
