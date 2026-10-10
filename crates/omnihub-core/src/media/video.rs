//! The song's music video, for Aurora's background. When allowed, OmniHub
//! searches YouTube for "<artist> <title> official music video" (the public
//! results page, no account and no key: the same page a browser gets) and
//! keeps the artist's own uploads only: the Official Artist Channel badge, a
//! VEVO channel, or a channel named like the artist. Music videos come first,
//! then the artist's "visualizer" (usually the cover, animated). Lyric videos,
//! audio-only uploads, live takes, covers, remixes and re-uploads are left
//! out. The video plays muted in YouTube's own player, so what the user hears
//! is still their music app.
//!
//! A video whose length matches the song's (within a few seconds) is played in
//! step with the song. A longer one (a story before or after the song) plays
//! from the same point and can be nudged in the settings.
//!
//! Only the title and artist are sent to YouTube; the answer is cached on disk.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::lyrics::clean_query;

/// How close a video's length must be to the song's to be played in step.
pub const SYNC_SLACK_MS: u64 = 6_000;
/// The results page is about 1.5 MB; anything far bigger is not it.
const MAX_BYTES: u64 = 8 << 20;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VideoKind {
    /// The music video.
    Video,
    /// The artist's visualizer: usually the cover, animated, as long as the song.
    Visualizer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MusicVideo {
    /// YouTube's id for it.
    pub id: String,
    pub title: String,
    pub channel: String,
    pub duration_ms: u64,
    pub kind: VideoKind,
    /// Uploaded in 4K or more.
    pub uhd: bool,
    /// As long as the song, so it can play in step with it.
    pub synced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum VideoStatus {
    /// Best first; the player tries the next one when YouTube will not play one here.
    Found { videos: Vec<MusicVideo> },
    Searching,
    /// The artist has no music video for it (that YouTube shows).
    None,
    /// Music videos are turned off.
    Off,
    NothingPlaying,
}

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// The words of a title, lower case, for whole-word checks.
fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase).collect()
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The search for this song (videos only).
pub fn search_url(title: &str, artist: &str) -> Option<String> {
    let (t, a) = clean_query(title, artist);
    if t.is_empty() {
        return None;
    }
    // sp=EgIQAQ== keeps the results to videos (no channels, playlists or shorts shelves).
    Some(format!("https://www.youtube.com/results?search_query={}&sp=EgIQAQ%253D%253D", enc(&format!("{a} {t} official music video"))))
}

/// One search result.
#[derive(Debug, Clone, PartialEq)]
pub struct Result_ {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub duration_ms: u64,
    /// "4K", "8K", "CC"…
    pub badges: Vec<String>,
    /// "Official Artist Channel", "Verified"…
    pub owner_badges: Vec<String>,
}

fn text(v: &Value) -> String {
    if let Some(s) = v.get("simpleText").and_then(Value::as_str) {
        return s.to_string();
    }
    v.get("runs").and_then(Value::as_array).map(|runs| runs.iter().filter_map(|r| r.get("text").and_then(Value::as_str)).collect()).unwrap_or_default()
}

/// "3:38" or "1:02:03" in ms.
fn length_ms(s: &str) -> u64 {
    s.split(':').try_fold(0u64, |acc, p| p.trim().parse::<u64>().ok().map(|n| acc * 60 + n)).unwrap_or(0) * 1000
}

fn walk(v: &Value, out: &mut Vec<Result_>) {
    match v {
        Value::Object(map) => {
            if let Some(r) = map.get("videoRenderer") {
                let id = r.get("videoId").and_then(Value::as_str).unwrap_or_default();
                if !id.is_empty() {
                    let labels = |key: &str, field: &str| -> Vec<String> {
                        r.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(|b| b.pointer(&format!("/metadataBadgeRenderer/{field}")).and_then(Value::as_str).map(str::to_string)).collect()).unwrap_or_default()
                    };
                    let channel = r.get("ownerText").or_else(|| r.get("longBylineText")).map(text).unwrap_or_default();
                    out.push(Result_ {
                        id: id.to_string(),
                        title: r.get("title").map(text).unwrap_or_default(),
                        channel,
                        duration_ms: r.get("lengthText").map(text).map(|s| length_ms(&s)).unwrap_or(0),
                        badges: labels("badges", "label"),
                        owner_badges: labels("ownerBadges", "tooltip"),
                    });
                }
                return;
            }
            map.values().for_each(|x| walk(x, out));
        }
        Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
        _ => {}
    }
}

/// The videos on a YouTube results page (its `ytInitialData`).
pub fn parse_results(html: &str) -> Vec<Result_> {
    let mut out = Vec::new();
    for marker in ["var ytInitialData = ", "ytInitialData = ", "window[\"ytInitialData\"] = "] {
        let Some(at) = html.find(marker) else { continue };
        let rest = &html[at + marker.len()..];
        if let Some(Ok(v)) = serde_json::Deserializer::from_str(rest).into_iter::<Value>().next() {
            walk(&v, &mut out);
            break;
        }
    }
    out
}

/// Words that mean an upload is not the music video itself.
const NOT_IT: &[&str] = &[
    "lyric", "lyrics", "letra", "letras", "karaoke", "instrumental", "reaction", "react", "cover", "live", "concert", "tour", "remix", "sped", "slowed", "nightcore", "8d", "reverb", "hour", "hours", "loop", "tiktok", "tutorial", "lesson", "teaser", "trailer", "interview", "snippet", "shorts", "practice", "choreography", "piano", "acoustic", "fanmade", "unofficial", "edit", "mashup", "parody", "behind", "making", "rehearsal", "vertical",
];

/// The best uploads for this song, music videos first: only the artist's own
/// (see the module comment), at most four.
pub fn pick(results: &[Result_], title: &str, artist: &str, duration_ms: u64) -> Vec<MusicVideo> {
    let (t, a) = clean_query(title, artist);
    let (nt, na) = (norm(&t), norm(&a));
    if nt.is_empty() {
        return Vec::new();
    }
    let song_words = words(&format!("{title} {artist}"));
    let mut scored: Vec<(u32, MusicVideo)> = Vec::new();
    for r in results {
        let vt = norm(&r.title);
        let ch = norm(&r.channel);
        let ch_bare = ch.trim_end_matches("vevo").trim_end_matches("official").to_string();
        if !vt.contains(&nt) {
            continue;
        }
        let tw = words(&r.title);
        if tw.iter().any(|w| NOT_IT.contains(&w.as_str()) && !song_words.contains(w)) {
            continue;
        }
        let audio_only = tw.iter().any(|w| w == "audio") && !song_words.iter().any(|w| w == "audio");
        let visualizer = tw.iter().any(|w| w == "visualizer" || w == "visualiser" || w == "visual");
        if audio_only && !visualizer {
            continue;
        }
        let artist_badge = r.owner_badges.iter().any(|b| b == "Official Artist Channel");
        let vevo = ch.ends_with("vevo");
        let named = !na.is_empty() && (ch == na || ch_bare == na);
        let verified_artist = r.owner_badges.iter().any(|b| b == "Verified") && !na.is_empty() && ch.contains(&na);
        // The artist's own channel, or nothing (re-uploads are someone else's copy).
        if !(artist_badge || vevo || named || verified_artist) {
            continue;
        }
        if !na.is_empty() && !vt.contains(&na) && !ch.contains(&na) && !na.contains(&ch_bare) {
            continue;
        }
        let d = r.duration_ms;
        if d == 0 {
            continue;
        }
        let synced = duration_ms > 0 && d.abs_diff(duration_ms) <= SYNC_SLACK_MS;
        if duration_ms > 0 {
            // A visualizer is the song itself; a video may add a story (up to 4 minutes).
            let fits = if visualizer { d.abs_diff(duration_ms) <= 10_000 } else { d + 15_000 >= duration_ms && d <= duration_ms + 240_000 };
            if !fits {
                continue;
            }
        }
        let uhd = r.badges.iter().any(|b| b == "4K" || b == "8K");
        let kind = if visualizer { VideoKind::Visualizer } else { VideoKind::Video };
        let says_video = r.title.to_lowercase().contains("video") || tw.iter().any(|w| w == "mv");
        // On the artist's own channel, an upload exactly as long as the song that
        // does not call itself a video is the song with its cover ("Art Track",
        // "(Explicit)"): a still picture. VEVO uploads are always videos.
        if kind == VideoKind::Video && synced && !says_video && !vevo {
            continue;
        }
        let score = u32::from(artist_badge) * 6 + u32::from(vevo) * 5 + u32::from(named) * 4 + u32::from(says_video) * 4 + u32::from(tw.iter().any(|w| w == "official")) + u32::from(synced) * 4 + u32::from(uhd) * 2;
        scored.push((score, MusicVideo { id: r.id.clone(), title: r.title.clone(), channel: r.channel.clone(), duration_ms: d, kind, uhd, synced }));
    }
    // Music videos before visualizers, then the strongest signs.
    scored.sort_by(|(sa, a), (sb, b)| (a.kind == VideoKind::Visualizer).cmp(&(b.kind == VideoKind::Visualizer)).then(sb.cmp(sa)));
    let mut seen = std::collections::HashSet::new();
    scored.into_iter().map(|(_, v)| v).filter(|v| seen.insert(v.id.clone())).take(4).collect()
}

/// Search YouTube for the song's video. `get(url)` returns the page.
pub fn find(title: &str, artist: &str, duration_ms: u64, get: &dyn Fn(&str) -> Result<String, String>) -> Result<Vec<MusicVideo>, String> {
    let Some(url) = search_url(title, artist) else { return Ok(Vec::new()) };
    let html = get(&url)?;
    Ok(pick(&parse_results(&html), title, artist, duration_ms))
}

/// Fetch a YouTube page the way a browser would (YouTube shows a cut-down page,
/// or its consent page in Europe, to anything else).
pub fn http_get(url: &str) -> Result<String, String> {
    if !url.starts_with("https://www.youtube.com/") {
        return Err("only YouTube is asked".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(15))).build().into();
    let mut res = agent
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0.0.0 Safari/537.36")
        .header("Accept-Language", "en-US,en;q=0.9")
        // "Reject all" on the consent page: no tracking cookies, just the results.
        .header("Cookie", "SOCS=CAI")
        .call()
        .map_err(|e| format!("could not reach YouTube: {e}"))?;
    res.body_mut().with_config().limit(MAX_BYTES).read_to_string().map_err(|e| e.to_string())
}

/// Answers kept on disk: a song is looked up once a month (a miss, every three days).
pub struct VideoCache {
    dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct Cached {
    at: u64,
    videos: Vec<MusicVideo>,
}

impl VideoCache {
    pub fn new(dir: PathBuf) -> Self {
        VideoCache { dir }
    }

    fn file(&self, key: &str) -> PathBuf {
        use sha2::Digest;
        self.dir.join(format!("{}.json", hex::encode(&sha2::Sha256::digest(key.as_bytes())[..12])))
    }

    pub fn get(&self, key: &str, now_ms: u64) -> Option<Vec<MusicVideo>> {
        let c: Cached = serde_json::from_slice(&std::fs::read(self.file(key)).ok()?).ok()?;
        let ttl = if c.videos.is_empty() { 3 } else { 30 } * 86_400_000;
        (now_ms.saturating_sub(c.at) < ttl).then_some(c.videos)
    }

    pub fn put(&self, key: &str, videos: &[MusicVideo], now_ms: u64) {
        let _ = std::fs::create_dir_all(&self.dir);
        if let Ok(bytes) = serde_json::to_vec(&Cached { at: now_ms, videos: videos.to_vec() }) {
            let _ = std::fs::write(self.file(key), bytes);
        }
        if let Ok(rd) = std::fs::read_dir(&self.dir) {
            let mut files: Vec<_> = rd.flatten().filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path()))).collect();
            if files.len() > 600 {
                files.sort();
                for (_, p) in files.iter().take(files.len() - 600) {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn renderer(id: &str, title: &str, channel: &str, length: &str, badges: &[&str], owner: &[&str]) -> Value {
        serde_json::json!({ "videoRenderer": {
            "videoId": id,
            "title": { "runs": [{ "text": title }] },
            "ownerText": { "runs": [{ "text": channel }] },
            "lengthText": { "simpleText": length },
            "badges": badges.iter().map(|b| serde_json::json!({ "metadataBadgeRenderer": { "label": b } })).collect::<Vec<_>>(),
            "ownerBadges": owner.iter().map(|b| serde_json::json!({ "metadataBadgeRenderer": { "tooltip": b } })).collect::<Vec<_>>(),
        }})
    }

    fn page(items: Vec<Value>) -> String {
        let data = serde_json::json!({ "contents": { "twoColumnSearchResultsRenderer": { "primaryContents": { "sectionListRenderer": { "contents": [{ "itemSectionRenderer": { "contents": items } }] } } } } });
        format!("<html><script>var ytInitialData = {data};</script><script>var other = 1;</script></html>")
    }

    #[test]
    fn reads_the_results_page() {
        let html = page(vec![renderer("abc", "Song (Official Video)", "Artist", "3:38", &["4K"], &["Official Artist Channel"]), serde_json::json!({ "shelfRenderer": {} })]);
        let r = parse_results(&html);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "abc");
        assert_eq!(r[0].duration_ms, 218_000);
        assert_eq!(r[0].badges, vec!["4K"]);
        assert_eq!(r[0].owner_badges, vec!["Official Artist Channel"]);
        assert_eq!(length_ms("1:02:03"), 3_723_000);
        assert!(parse_results("<html>nothing here</html>").is_empty());
    }

    #[test]
    fn keeps_the_artists_own_video_and_its_visualizer() {
        // The real results for this song (trimmed): the official video is two
        // minutes longer than the song (a story before it).
        let html = page(vec![
            renderer("audio1", "Tame Impala - The Less I Know The Better (Audio)", "Tame Impala", "3:38", &[], &["Official Artist Channel"]),
            renderer("lyric1", "Tame Impala - The less I know the better (lyrics)", "Freedomfight8480", "3:38", &[], &[]),
            renderer("reup1", "Tame Impala - The Less I Know The Better [Official Music Video]", "Bad Boy Edd", "5:44", &[], &[]),
            renderer("8k", "Tame Impala - The Less I Know The Better [8K, Remastered]", "Lel Keke", "5:48", &["8K"], &[]),
            renderer("live1", "Tame Impala - The Less I Know The Better | Later... with Jools Holland", "BBC Music", "3:43", &[], &["Verified"]),
            renderer("viz1", "Tame Impala - The Less I Know The Better (Official Visualizer)", "Tame Impala", "3:39", &[], &["Official Artist Channel"]),
            renderer("mv1", "Tame Impala - The Less I Know The Better (Official Video)", "Tame Impala", "5:43", &["CC"], &["Official Artist Channel"]),
        ]);
        let picked = pick(&parse_results(&html), "The Less I Know The Better", "Tame Impala", 218_000);
        let ids: Vec<_> = picked.iter().map(|v| v.id.as_str()).collect();
        assert_eq!(ids, vec!["mv1", "viz1"], "{picked:?}");
        assert_eq!(picked[0].kind, VideoKind::Video);
        assert!(!picked[0].synced, "a video with a story is not in step with the song");
        assert_eq!(picked[1].kind, VideoKind::Visualizer);
        assert!(picked[1].synced);
    }

    #[test]
    fn prefers_vevo_and_matching_lengths_and_says_4k() {
        let html = page(vec![
            renderer("long", "Drake - Hotline Bling (Official Video)", "DrakeVEVO", "4:55", &[], &[]),
            renderer("exact", "Drake - Hotline Bling", "DrakeVEVO", "4:27", &["4K"], &[]),
            renderer("other", "Hotline Bling", "Some Channel", "4:27", &[], &["Verified"]),
        ]);
        let picked = pick(&parse_results(&html), "Hotline Bling", "Drake", 267_000);
        assert_eq!(picked.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(), vec!["exact", "long"]);
        assert!(picked[0].uhd && picked[0].synced);
        // Nothing official for another song: nothing.
        assert!(pick(&parse_results(&html), "Some Other Song", "Drake", 200_000).is_empty());
        // A remix is only right when the song is the remix.
        let remix = page(vec![renderer("rmx", "Drake - Hotline Bling (Remix) (Official Video)", "DrakeVEVO", "4:27", &[], &[])]);
        assert!(pick(&parse_results(&remix), "Hotline Bling", "Drake", 267_000).is_empty());
        assert_eq!(pick(&parse_results(&remix), "Hotline Bling (Remix)", "Drake", 267_000).len(), 1);
        // Seen live: the song's own upload ("(Explicit)", song length, not called a
        // video) and a vertical cut are not the video.
        let live = page(vec![
            renderer("vert", "Billie Eilish - bad guy (Vertical Video)", "Billie Eilish", "3:13", &[], &["Official Artist Channel"]),
            renderer("expl", "Billie Eilish - bad guy (Explicit)", "Billie Eilish", "3:14", &[], &["Official Artist Channel"]),
            renderer("mv", "Billie Eilish - bad guy", "Billie Eilish", "3:26", &[], &["Official Artist Channel"]),
        ]);
        assert_eq!(pick(&parse_results(&live), "bad guy", "Billie Eilish", 194_000).iter().map(|v| v.id.as_str()).collect::<Vec<_>>(), vec!["mv"]);
    }

    /// Against the real YouTube (needs the internet): `cargo test -- --ignored live_youtube`.
    #[test]
    #[ignore]
    fn live_youtube() {
        for (title, artist, ms) in [("The Less I Know The Better", "Tame Impala", 218_000), ("Hotline Bling", "Drake", 267_000), ("Blinding Lights", "The Weeknd", 200_000), ("bad guy", "Billie Eilish", 194_000)] {
            let found = find(title, artist, ms, &http_get).unwrap();
            println!("{artist} - {title}:");
            for v in &found {
                println!("  {:?} {} | {} | {} | {}s | 4K {} | in step {}", v.kind, v.id, v.title, v.channel, v.duration_ms / 1000, v.uhd, v.synced);
            }
            assert!(!found.is_empty(), "{artist} - {title} has an official video");
        }
    }

    #[test]
    fn asks_youtube_for_videos_only_and_caches() {
        let url = search_url("Hotline Bling", "Drake").unwrap();
        assert!(url.starts_with("https://www.youtube.com/results?search_query=Drake+Hotline+Bling+official+music+video&sp="));
        let calls = std::cell::Cell::new(0);
        let get = |_: &str| -> Result<String, String> {
            calls.set(calls.get() + 1);
            Ok(page(vec![renderer("exact", "Drake - Hotline Bling (Official Video)", "DrakeVEVO", "4:27", &[], &[])]))
        };
        assert_eq!(find("Hotline Bling", "Drake", 267_000, &get).unwrap().len(), 1);
        assert_eq!(calls.get(), 1);
        let dir = std::env::temp_dir().join(format!("omnihub-videos-{}", std::process::id()));
        let cache = VideoCache::new(dir.clone());
        assert!(cache.get("k", 1_000).is_none());
        let found = find("Hotline Bling", "Drake", 267_000, &get).unwrap();
        cache.put("k", &found, 1_000);
        assert_eq!(cache.get("k", 2_000).unwrap(), found);
        assert!(cache.get("k", 1_000 + 31 * 86_400_000).is_none(), "a month later it is looked up again");
        cache.put("miss", &[], 1_000);
        assert_eq!(cache.get("miss", 2_000).unwrap(), Vec::<MusicVideo>::new());
        assert!(cache.get("miss", 1_000 + 4 * 86_400_000).is_none());
        let _ = std::fs::remove_dir_all(dir);
        assert!(http_get("https://example.com/").is_err());
    }
}
