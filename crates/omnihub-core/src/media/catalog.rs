//! Aurora's music hub: search songs, albums and artists, see an artist's
//! top songs, albums and a short bio, and play what you pick in the music
//! service you choose.
//!
//! The catalogue is Deezer's public API (no account, no key: the same one
//! the cover lookup uses); bios come from Wikipedia's page summaries. Only
//! what you type, or the playing song's title and artist (for "up next"),
//! is sent.
//!
//! Playing hands the song to a music service, the way a link would:
//! YouTube Music opens the song itself (found on YouTube's public results
//! page); Apple Music opens the song's page (from Apple's public catalogue
//! search), in the Apple Music app when it is installed; Spotify, SoundCloud
//! and Amazon Music open on a search for it, because they only let apps
//! their users signed in to start a song. Nothing is downloaded or played
//! by OmniHub itself.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Where a song picked in Aurora plays.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    /// The app playing now, else the music app installed, else YouTube Music.
    #[default]
    Auto,
    Spotify,
    SpotifyWeb,
    AppleMusic,
    YoutubeMusic,
    Soundcloud,
    AmazonMusic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artist_id: String,
    pub album: String,
    pub album_id: String,
    pub cover: String,
    pub duration_ms: u64,
    pub explicit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub artist_id: String,
    pub cover: String,
    /// "2024".
    pub year: String,
    /// "album", "single", "ep" or "compile".
    pub kind: String,
    pub tracks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub picture: String,
    pub fans: u64,
    pub albums: u32,
}

/// The best match for the search, shown big.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "kind", content = "item")]
pub enum Top {
    Artist(Artist),
    Track(Track),
    Album(Album),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Results {
    pub top: Option<Top>,
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ArtistPage {
    pub artist: Artist,
    /// The first lines of the artist's Wikipedia article.
    pub bio: Option<String>,
    pub top: Vec<Track>,
    pub albums: Vec<Album>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AlbumPage {
    pub album: Album,
    pub tracks: Vec<Track>,
}

/// What comes with the song that plays: the rest of its album and more by the artist.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpNext {
    /// The song as the catalogue knows it (`None` when it isn't there).
    pub track: Option<Track>,
    pub album: Option<AlbumPage>,
    pub more: Vec<Track>,
    pub artist: Option<Artist>,
}

/// Something to play.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Item {
    Track { title: String, artist: String, album: String },
    Album { title: String, artist: String },
    Artist { name: String },
}

/// Which music apps are on this PC.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    pub spotify: bool,
    pub apple_music: bool,
}

pub type Get<'a> = &'a dyn Fn(&str) -> Result<Option<String>, String>;

// ---------- fetching ----------

const HOSTS: &[&str] = &["https://api.deezer.com/", "https://en.wikipedia.org/", "https://itunes.apple.com/"];

/// URL → (when it was fetched, the answer).
type Cache = HashMap<String, (Instant, Option<String>)>;
static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(Default::default);
const CACHE_FOR: Duration = Duration::from_secs(15 * 60);

/// GET from the catalogue services (404 → `Ok(None)`), answers kept a while.
pub fn http_get(url: &str) -> Result<Option<String>, String> {
    if !HOSTS.iter().any(|h| url.starts_with(h)) {
        return Err("only the music catalogue is asked".into());
    }
    if let Some((at, body)) = CACHE.lock().get(url) {
        if at.elapsed() < CACHE_FOR {
            return Ok(body.clone());
        }
    }
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(12))).http_status_as_error(false).build().into();
    let mut res = agent
        .get(url)
        .header("User-Agent", concat!("OmniHub/", env!("CARGO_PKG_VERSION"), " (https://github.com/Kerioxsx/omnihub)"))
        .header("Accept-Language", "en")
        .call()
        .map_err(|e| format!("couldn't reach the music catalogue: {e}"))?;
    let body = match res.status().as_u16() {
        200 => Some(res.body_mut().with_config().limit(4 << 20).read_to_string().map_err(|e| e.to_string())?),
        404 => None,
        code => return Err(format!("the music catalogue answered {code}")),
    };
    let mut c = CACHE.lock();
    if c.len() > 300 {
        c.retain(|_, (at, _)| at.elapsed() < CACHE_FOR);
    }
    c.insert(url.to_string(), (Instant::now(), body.clone()));
    Ok(body)
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn json(get: Get, url: &str) -> Result<Value, String> {
    let Some(body) = get(url)? else { return Ok(Value::Null) };
    let v: Value = serde_json::from_str(&body).map_err(|_| "the music catalogue sent something unexpected".to_string())?;
    // Deezer reports errors (quota, not found) inside a 200 answer.
    if let Some(e) = v.get("error") {
        if e.get("code").and_then(Value::as_u64) == Some(800) {
            return Ok(Value::Null);
        }
        return Err(format!("the music catalogue said: {}", e.get("message").and_then(Value::as_str).unwrap_or("error")));
    }
    Ok(v)
}

fn s(v: &Value, k: &str) -> String {
    match v.get(k) {
        Some(Value::String(x)) => x.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn track_of(v: &Value, album: Option<&Album>) -> Option<Track> {
    let id = s(v, "id");
    let title = s(v, "title");
    if id.is_empty() || title.is_empty() || v.get("readable") == Some(&Value::Bool(false)) {
        return None;
    }
    let a = v.get("artist").unwrap_or(&Value::Null);
    let al = v.get("album").unwrap_or(&Value::Null);
    Some(Track {
        id,
        title,
        artist: s(a, "name"),
        artist_id: s(a, "id"),
        album: album.map(|x| x.title.clone()).unwrap_or_else(|| s(al, "title")),
        album_id: album.map(|x| x.id.clone()).unwrap_or_else(|| s(al, "id")),
        cover: album.map(|x| x.cover.clone()).unwrap_or_else(|| s(al, "cover_xl")),
        duration_ms: v.get("duration").and_then(Value::as_u64).unwrap_or(0) * 1000,
        explicit: v.get("explicit_lyrics").and_then(Value::as_bool).unwrap_or(false),
    })
}

fn album_of(v: &Value, artist: Option<&Artist>) -> Option<Album> {
    let id = s(v, "id");
    if id.is_empty() {
        return None;
    }
    let a = v.get("artist").unwrap_or(&Value::Null);
    Some(Album {
        id,
        title: s(v, "title"),
        artist: artist.map(|x| x.name.clone()).unwrap_or_else(|| s(a, "name")),
        artist_id: artist.map(|x| x.id.clone()).unwrap_or_else(|| s(a, "id")),
        cover: s(v, "cover_xl"),
        year: s(v, "release_date").chars().take(4).collect(),
        kind: s(v, "record_type"),
        tracks: v.get("nb_tracks").and_then(Value::as_u64).unwrap_or(0) as u32,
    })
}

fn artist_of(v: &Value) -> Option<Artist> {
    let id = s(v, "id");
    if id.is_empty() {
        return None;
    }
    Some(Artist { id, name: s(v, "name"), picture: s(v, "picture_xl"), fans: v.get("nb_fan").and_then(Value::as_u64).unwrap_or(0), albums: v.get("nb_album").and_then(Value::as_u64).unwrap_or(0) as u32 })
}

fn data(v: &Value) -> &[Value] {
    v.get("data").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

pub fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// A Deezer id: digits only (it goes into a URL path).
fn id_ok(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 20 || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err("unknown item".into());
    }
    Ok(())
}

// ---------- search, artists, albums ----------

pub fn search(query: &str, get: Get) -> Result<Results, String> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Results::default());
    }
    let q: String = q.chars().take(120).collect();
    let tracks: Vec<Track> = data(&json(get, &format!("https://api.deezer.com/search/track?q={}&limit=15", enc(&q)))?).iter().filter_map(|v| track_of(v, None)).collect();
    let artists: Vec<Artist> = data(&json(get, &format!("https://api.deezer.com/search/artist?q={}&limit=8", enc(&q)))?).iter().filter_map(artist_of).collect();
    let albums: Vec<Album> = data(&json(get, &format!("https://api.deezer.com/search/album?q={}&limit=12", enc(&q)))?).iter().filter_map(|v| album_of(v, None)).collect();
    let nq = norm(&q);
    // An artist whose name is what was typed beats a song; then a song or
    // album named exactly that; else the first song.
    let top = artists
        .iter()
        .find(|a| norm(&a.name) == nq)
        .cloned()
        .map(Top::Artist)
        .or_else(|| tracks.iter().find(|t| norm(&t.title) == nq || norm(&format!("{} {}", t.artist, t.title)) == nq).cloned().map(Top::Track))
        .or_else(|| albums.iter().find(|a| norm(&a.title) == nq).cloned().map(Top::Album))
        .or_else(|| tracks.first().cloned().map(Top::Track));
    Ok(Results { top, tracks, albums, artists })
}

const MUSICAL: &[&str] = &["rapper", "singer", "songwriter", "musician", "band", "group", "duo", "dj", "producer", "composer", "artist", "vocalist", "trio", "rock", "hip hop", "pop", "music"];

/// The first lines of the artist's Wikipedia article, when there is one about a musician.
pub fn bio(name: &str, get: Get) -> Option<String> {
    let base = name.trim();
    if base.is_empty() {
        return None;
    }
    for title in [base.to_string(), format!("{base} (rapper)"), format!("{base} (singer)"), format!("{base} (musician)"), format!("{base} (band)")] {
        let url = format!("https://en.wikipedia.org/api/rest_v1/page/summary/{}", enc(&title.replace(' ', "_")));
        let Ok(v) = json(get, &url) else { return None };
        if v.is_null() || s(&v, "type") == "disambiguation" {
            continue;
        }
        let desc = s(&v, "description").to_lowercase();
        let extract = s(&v, "extract");
        if MUSICAL.iter().any(|m| desc.contains(m)) && !extract.is_empty() {
            return Some(extract);
        }
    }
    None
}

pub fn artist(id: &str, get: Get) -> Result<ArtistPage, String> {
    id_ok(id)?;
    let v = json(get, &format!("https://api.deezer.com/artist/{id}"))?;
    let artist = artist_of(&v).ok_or("That artist isn't in the catalogue.")?;
    let top = data(&json(get, &format!("https://api.deezer.com/artist/{id}/top?limit=12"))?).iter().filter_map(|t| track_of(t, None)).collect();
    let mut albums: Vec<Album> = data(&json(get, &format!("https://api.deezer.com/artist/{id}/albums?limit=60"))?).iter().filter_map(|a| album_of(a, Some(&artist))).collect();
    // Newest first; the same title twice (clean and explicit) once.
    albums.sort_by(|a, b| b.year.cmp(&a.year));
    let mut seen = std::collections::HashSet::new();
    albums.retain(|a| seen.insert(norm(&a.title)));
    let bio = bio(&artist.name, get);
    Ok(ArtistPage { artist, bio, top, albums })
}

pub fn album(id: &str, get: Get) -> Result<AlbumPage, String> {
    id_ok(id)?;
    let v = json(get, &format!("https://api.deezer.com/album/{id}"))?;
    let mut album = album_of(&v, None).ok_or("That album isn't in the catalogue.")?;
    let list = v.pointer("/tracks/data").and_then(Value::as_array).cloned().unwrap_or_default();
    if album.tracks == 0 {
        album.tracks = list.len() as u32;
    }
    let tracks = list.iter().filter_map(|t| track_of(t, Some(&album))).collect();
    Ok(AlbumPage { album, tracks })
}

/// The song in the catalogue: same title (versions aside) and artist.
fn find_track(title: &str, artist: &str, get: Get) -> Result<Option<Track>, String> {
    let (t, a) = super::lyrics::clean_query(title, artist);
    if t.is_empty() {
        return Ok(None);
    }
    let found: Vec<Track> = data(&json(get, &format!("https://api.deezer.com/search/track?q={}&limit=10", enc(&format!("{a} {t}"))))?).iter().filter_map(|v| track_of(v, None)).collect();
    let (nt, na) = (norm(&t), norm(&a));
    Ok(found.into_iter().find(|x| {
        let xt = norm(&super::lyrics::clean_query(&x.title, &x.artist).0);
        let xa = norm(&x.artist);
        (xt == nt || xt.starts_with(&nt) || nt.starts_with(&xt)) && (na.is_empty() || xa.contains(&na) || na.contains(&xa))
    }))
}

pub fn up_next(title: &str, artist: &str, get: Get) -> Result<UpNext, String> {
    let Some(track) = find_track(title, artist, get)? else { return Ok(UpNext::default()) };
    let album = if track.album_id.is_empty() { None } else { album(&track.album_id, get).ok().filter(|a| a.tracks.len() > 1) };
    let page = if track.artist_id.is_empty() { None } else { artist_light(&track.artist_id, get).ok() };
    let (artist, mut more) = page.unwrap_or_default();
    let on_album: std::collections::HashSet<String> = album.as_ref().map(|a| a.tracks.iter().map(|t| norm(&t.title)).collect()).unwrap_or_default();
    more.retain(|t| norm(&t.title) != norm(&track.title) && !on_album.contains(&norm(&t.title)));
    Ok(UpNext { track: Some(track), album, more, artist })
}

/// The artist and top songs, without the bio and albums.
fn artist_light(id: &str, get: Get) -> Result<(Option<Artist>, Vec<Track>), String> {
    id_ok(id)?;
    let artist = artist_of(&json(get, &format!("https://api.deezer.com/artist/{id}"))?);
    let top = data(&json(get, &format!("https://api.deezer.com/artist/{id}/top?limit=12"))?).iter().filter_map(|t| track_of(t, None)).collect();
    Ok((artist, top))
}

// ---------- playing it somewhere ----------

/// Which service `Auto` means: the app playing now, else an installed one, else YouTube Music.
pub fn resolve(source: Source, playing_app: Option<&str>, installed: Installed) -> Source {
    if source != Source::Auto {
        return source;
    }
    let app = playing_app.unwrap_or_default().to_lowercase();
    if app.contains("spotify") {
        Source::Spotify
    } else if app.contains("apple music") || app.contains("itunes") || app.contains("applemusic") {
        Source::AppleMusic
    } else if app.contains("amazon music") {
        Source::AmazonMusic
    } else if installed.spotify {
        Source::Spotify
    } else if installed.apple_music {
        Source::AppleMusic
    } else {
        Source::YoutubeMusic
    }
}

fn words_of(item: &Item) -> String {
    match item {
        Item::Track { title, artist, .. } => {
            let (t, a) = super::lyrics::clean_query(title, artist);
            format!("{a} {t}")
        }
        Item::Album { title, artist } => format!("{artist} {title}"),
        Item::Artist { name } => name.clone(),
    }
    .trim()
    .to_string()
}

/// The song on YouTube: the artist's "- Topic" upload (what YouTube Music
/// plays), else the artist's own channel.
pub fn youtube_song(title: &str, artist: &str, page: &str) -> Option<String> {
    let (t, a) = super::lyrics::clean_query(title, artist);
    let (nt, na) = (norm(&t), norm(&a));
    if nt.is_empty() {
        return None;
    }
    let results = super::video::parse_results(page);
    let fits = |r: &&super::video::Result_| norm(&r.title).contains(&nt) && !["live", "remix", "sped up", "slowed", "karaoke", "cover"].iter().any(|w| r.title.to_lowercase().contains(w) && !title.to_lowercase().contains(w));
    let topic = |r: &&super::video::Result_| r.channel.ends_with(" - Topic") && (na.is_empty() || norm(r.channel.trim_end_matches(" - Topic")).contains(&na) || na.contains(&norm(r.channel.trim_end_matches(" - Topic"))));
    let own = |r: &&super::video::Result_| {
        let ch = norm(&r.channel);
        r.owner_badges.iter().any(|b| b == "Official Artist Channel") || ch.ends_with("vevo") || (!na.is_empty() && (ch.contains(&na) || na.contains(&ch)))
    };
    results.iter().filter(fits).find(topic).or_else(|| results.iter().filter(fits).find(own)).map(|r| r.id.clone())
}

/// The item's page on Apple Music, from Apple's public catalogue search.
pub fn apple_link(item: &Item, get: Get) -> Option<String> {
    let (entity, key) = match item {
        Item::Track { .. } => ("song", "trackViewUrl"),
        Item::Album { .. } => ("album", "collectionViewUrl"),
        Item::Artist { .. } => ("musicArtist", "artistLinkUrl"),
    };
    let v = json(get, &format!("https://itunes.apple.com/search?term={}&media=music&entity={entity}&limit=8", enc(&words_of(item)))).ok()?;
    let results = v.get("results").and_then(Value::as_array)?;
    let want = match item {
        Item::Track { title, artist, .. } => (norm(&super::lyrics::clean_query(title, artist).0), norm(artist)),
        Item::Album { title, artist } => (norm(title), norm(artist)),
        Item::Artist { name } => (String::new(), norm(name)),
    };
    let name_key = match item {
        Item::Track { .. } => "trackName",
        Item::Album { .. } => "collectionName",
        Item::Artist { .. } => "artistName",
    };
    let pick = results.iter().find(|r| {
        let n = norm(&s(r, name_key));
        let a = norm(&s(r, "artistName"));
        let name_ok = want.0.is_empty() || n.contains(&want.0) || want.0.contains(&n);
        name_ok && (want.1.is_empty() || a.contains(&want.1) || want.1.contains(&a))
    })?;
    let url = s(pick, key);
    if !url.starts_with("https://music.apple.com/") {
        return None;
    }
    let base = url.split('?').next().unwrap_or(&url);
    Some(format!("{base}{}", pick_track_param(&url)))
}

/// Keep `?i=<track id>` on a song link (it selects the song on its album's page).
fn pick_track_param(url: &str) -> String {
    url.split_once('?').and_then(|(_, q)| q.split('&').find(|p| p.starts_with("i="))).map(|p| format!("?{p}")).unwrap_or_default()
}

/// The link that plays (or shows) `item` in `source` (never `Auto`: resolve it first).
/// `youtube(url)` fetches a YouTube results page; `get` the catalogues.
pub fn link(source: Source, item: &Item, installed: Installed, youtube: &dyn Fn(&str) -> Result<String, String>, get: Get) -> String {
    let q = words_of(item);
    let path = enc(&q);
    match source {
        Source::YoutubeMusic => {
            if let Item::Track { title, artist, .. } = item {
                let url = format!("https://www.youtube.com/results?search_query={}&sp=EgIQAQ%253D%253D", enc(&q));
                if let Some(id) = youtube(&url).ok().and_then(|page| youtube_song(title, artist, &page)) {
                    return format!("https://music.youtube.com/watch?v={id}");
                }
            }
            format!("https://music.youtube.com/search?q={path}")
        }
        Source::Spotify => format!("spotify:search:{path}"),
        Source::SpotifyWeb => format!("https://open.spotify.com/search/{path}"),
        Source::AppleMusic => match apple_link(item, get) {
            // The Apple Music app takes the same link with its own scheme.
            Some(url) if installed.apple_music => url.replacen("https://", "musics://", 1),
            Some(url) => url,
            None => format!("https://music.apple.com/search?term={path}"),
        },
        Source::Soundcloud => format!(
            "https://soundcloud.com/search/{}?q={path}",
            match item {
                Item::Track { .. } => "sounds",
                Item::Album { .. } => "albums",
                Item::Artist { .. } => "people",
            }
        ),
        Source::AmazonMusic => format!("https://music.amazon.com/search/{path}"),
        Source::Auto => format!("https://music.youtube.com/search?q={path}"),
    }
}

/// Play `item` in `source` (`Auto` resolved with the app playing now).
/// Returns the service it went to.
pub fn play(item: &Item, source: Source, playing_app: Option<&str>) -> Result<Source, String> {
    let have = installed();
    let to = resolve(source, playing_app, have);
    let url = link(to, item, have, &super::video::http_get, &http_get);
    crate::system::shell::shell_execute(&url, "").map_err(|e| match to {
        Source::Spotify => "Spotify isn't installed on this PC. Pick Spotify Web Player, or install Spotify.".to_string(),
        Source::AppleMusic if url.starts_with("musics:") => "The Apple Music app didn't open the song.".to_string(),
        _ => e.to_string(),
    })?;
    Ok(to)
}

/// Which music apps can take a link (their link schemes are registered).
pub fn installed() -> Installed {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CLASSES_ROOT;
        let has = |scheme: &str| winreg::RegKey::predef(HKEY_CLASSES_ROOT).open_subkey(scheme).is_ok_and(|k| k.get_value::<String, _>("URL Protocol").is_ok());
        Installed { spotify: has("spotify"), apple_music: has("musics") || has("music") }
    }
    #[cfg(not(windows))]
    {
        Installed::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake(url: &str) -> Result<Option<String>, String> {
        let body = if url.contains("/search/track?q=lil%20mosey") || url.contains("/search/track?q=Lil%20Mosey%20Noticed") {
            r#"{"data":[{"id":1,"title":"Noticed","duration":163,"explicit_lyrics":true,"artist":{"id":7,"name":"Lil Mosey"},"album":{"id":70,"title":"Northsbest","cover_xl":"https://cdn-images.dzcdn.net/a.jpg"}},{"id":2,"title":"Blueberry Faygo","duration":163,"artist":{"id":7,"name":"Lil Mosey"},"album":{"id":71,"title":"Certified Hitmaker","cover_xl":"x"}}]}"#
        } else if url.contains("/search/artist?q=lil%20mosey") {
            r#"{"data":[{"id":7,"name":"Lil Mosey","picture_xl":"https://cdn-images.dzcdn.net/p.jpg","nb_fan":900000,"nb_album":40}]}"#
        } else if url.contains("/search/album?") {
            r#"{"data":[{"id":70,"title":"Northsbest","cover_xl":"c","nb_tracks":11,"record_type":"album","artist":{"id":7,"name":"Lil Mosey"}}]}"#
        } else if url.ends_with("/artist/7") {
            r#"{"id":7,"name":"Lil Mosey","picture_xl":"p","nb_fan":900000,"nb_album":40}"#
        } else if url.contains("/artist/7/top") {
            r#"{"data":[{"id":1,"title":"Noticed","duration":163,"artist":{"id":7,"name":"Lil Mosey"},"album":{"id":70,"title":"Northsbest","cover_xl":"c"}},{"id":3,"title":"Kamikaze","duration":120,"artist":{"id":7,"name":"Lil Mosey"},"album":{"id":72,"title":"Kamikaze","cover_xl":"k"}}]}"#
        } else if url.contains("/artist/7/albums") {
            r#"{"data":[{"id":70,"title":"Northsbest","release_date":"2018-10-12","record_type":"album","cover_xl":"c"},{"id":73,"title":"Northsbest","release_date":"2018-10-12","record_type":"album","cover_xl":"c"},{"id":74,"title":"Mercer Hotel","release_date":"2026-01-01","record_type":"single","cover_xl":"m"}]}"#
        } else if url.ends_with("/album/70") {
            r#"{"id":70,"title":"Northsbest","cover_xl":"c","release_date":"2018-10-12","record_type":"album","artist":{"id":7,"name":"Lil Mosey"},"tracks":{"data":[{"id":1,"title":"Noticed","duration":163,"artist":{"id":7,"name":"Lil Mosey"}},{"id":5,"title":"Pull Up","duration":150,"artist":{"id":7,"name":"Lil Mosey"}}]}}"#
        } else if url.contains("wikipedia.org/api/rest_v1/page/summary/Lil_Mosey") && !url.contains("%28") {
            r#"{"type":"standard","title":"Lil Mosey","description":"American rapper (born 2002)","extract":"Lathan Moses Stanley Echols, known professionally as Lil Mosey, is an American rapper."}"#
        } else if url.contains("itunes.apple.com/search") {
            r#"{"results":[{"trackName":"Noticed","artistName":"Lil Mosey","trackViewUrl":"https://music.apple.com/us/album/noticed/1440?i=1441&uo=4"}]}"#
        } else {
            return Ok(None);
        };
        Ok(Some(body.to_string()))
    }

    #[test]
    fn search_artist_album_and_up_next() {
        let r = search("lil mosey", &fake).unwrap();
        assert_eq!(r.tracks.len(), 2);
        assert!(matches!(r.top, Some(Top::Artist(ref a)) if a.name == "Lil Mosey"));
        assert_eq!(r.albums[0].kind, "album");

        let page = artist("7", &fake).unwrap();
        assert_eq!(page.top.len(), 2);
        assert_eq!(page.albums.iter().map(|a| a.title.as_str()).collect::<Vec<_>>(), vec!["Mercer Hotel", "Northsbest"], "newest first, no duplicates");
        assert!(page.bio.unwrap().contains("American rapper"));
        assert!(artist("7/../x", &fake).is_err());

        let al = album("70", &fake).unwrap();
        assert_eq!(al.tracks.len(), 2);
        assert_eq!(al.tracks[1].cover, "c", "the album's cover for its songs");

        let next = up_next("Noticed", "Lil Mosey", &fake).unwrap();
        assert_eq!(next.track.unwrap().id, "1");
        assert_eq!(next.album.unwrap().tracks.len(), 2);
        assert_eq!(next.more.iter().map(|t| t.title.as_str()).collect::<Vec<_>>(), vec!["Kamikaze"], "not the song or its album again");
        assert_eq!(up_next("Unknown", "Nobody", &fake).unwrap(), UpNext::default());
    }

    #[test]
    fn links_for_every_service() {
        let none = Installed::default();
        let track = Item::Track { title: "Noticed".into(), artist: "Lil Mosey".into(), album: "Northsbest".into() };
        let page = r#"<script>var ytInitialData = {"contents":[{"videoRenderer":{"videoId":"live1","title":{"runs":[{"text":"Noticed (Live)"}]},"ownerText":{"runs":[{"text":"Lil Mosey - Topic"}]},"lengthText":{"simpleText":"2:43"}}},{"videoRenderer":{"videoId":"abc123","title":{"runs":[{"text":"Noticed"}]},"ownerText":{"runs":[{"text":"Lil Mosey - Topic"}]},"lengthText":{"simpleText":"2:43"}}}]};</script>"#;
        let yt = |_: &str| Ok(page.to_string());
        assert_eq!(link(Source::YoutubeMusic, &track, none, &yt, &fake), "https://music.youtube.com/watch?v=abc123");
        let nothing = |_: &str| Err("offline".to_string());
        assert_eq!(link(Source::YoutubeMusic, &track, none, &nothing, &fake), "https://music.youtube.com/search?q=Lil%20Mosey%20Noticed");
        assert_eq!(link(Source::Spotify, &track, none, &yt, &fake), "spotify:search:Lil%20Mosey%20Noticed");
        assert_eq!(link(Source::SpotifyWeb, &Item::Artist { name: "Lil Mosey".into() }, none, &yt, &fake), "https://open.spotify.com/search/Lil%20Mosey");
        assert_eq!(link(Source::AppleMusic, &track, none, &yt, &fake), "https://music.apple.com/us/album/noticed/1440?i=1441");
        assert_eq!(link(Source::AppleMusic, &track, Installed { apple_music: true, spotify: false }, &yt, &fake), "musics://music.apple.com/us/album/noticed/1440?i=1441");
        assert!(link(Source::Soundcloud, &Item::Album { title: "Northsbest".into(), artist: "Lil Mosey".into() }, none, &yt, &fake).starts_with("https://soundcloud.com/search/albums?q="));
        assert_eq!(link(Source::AmazonMusic, &track, none, &yt, &fake), "https://music.amazon.com/search/Lil%20Mosey%20Noticed");

        assert_eq!(resolve(Source::Auto, Some("Spotify"), none), Source::Spotify);
        assert_eq!(resolve(Source::Auto, Some("Apple Music"), none), Source::AppleMusic);
        assert_eq!(resolve(Source::Auto, Some("Google Chrome"), Installed { spotify: true, apple_music: false }), Source::Spotify);
        assert_eq!(resolve(Source::Auto, None, none), Source::YoutubeMusic);
        assert_eq!(resolve(Source::Soundcloud, None, none), Source::Soundcloud);
        assert!(http_get("https://example.com/").is_err(), "only the catalogue hosts");
    }
}

