//! Music videos YouTube will not show in an embedded player.
//!
//! Aurora plays videos in YouTube's own player first (see `video.rs` and
//! site/player.js). Some owners turn embedding off everywhere; for those, when
//! the user allows it, OmniHub asks yt-dlp (the open-source tool media players
//! such as mpv use) for the video's own stream address and Aurora plays that,
//! muted, like any video file. yt-dlp needs a JavaScript runtime for YouTube,
//! so Deno comes with it.
//!
//! Both are downloaded the first time they are needed, from their projects'
//! latest GitHub releases, and kept only when they match the SHA-256 published
//! with that release. yt-dlp keeps itself current (`-U`, at most once a day),
//! since YouTube changes often. Nothing is downloaded or run unless a video
//! needs it, and the stream itself is never saved.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;

const YTDLP_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
const DENO_URL: &str = "https://github.com/denoland/deno/releases/latest/download";
#[cfg(windows)]
const YTDLP_ASSET: &str = "yt-dlp.exe";
#[cfg(not(windows))]
const YTDLP_ASSET: &str = "yt-dlp";
#[cfg(windows)]
const DENO_ASSET: &str = "deno-x86_64-pc-windows-msvc.zip";
#[cfg(not(windows))]
const DENO_ASSET: &str = "deno-x86_64-unknown-linux-gnu.zip";
#[cfg(windows)]
const DENO_EXE: &str = "deno.exe";
#[cfg(not(windows))]
const DENO_EXE: &str = "deno";

/// A stream address is good for hours; ask again well before it lapses.
const KEEP_MS: u64 = 3 * 3600 * 1000;

/// The SHA-256 for `name` in a checksum file: `sha256sum` lines ("<hash>
/// <name>") or PowerShell's Get-FileHash listing ("Hash : <HASH>"), which
/// names one file only.
pub fn checksum_in(text: &str, name: &str) -> Option<String> {
    let hex64 = |s: &str| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit());
    let listed = text.lines().find_map(|l| {
        let (hash, file) = l.trim().split_once(char::is_whitespace)?;
        (hex64(hash) && file.trim().trim_start_matches('*') == name).then(|| hash.to_lowercase())
    });
    listed.or_else(|| {
        text.lines().find_map(|l| {
            let (k, v) = l.split_once(':')?;
            (k.trim().eq_ignore_ascii_case("hash") && hex64(v.trim())).then(|| v.trim().to_lowercase())
        })
    })
}

/// yt-dlp's format choice: the best video-only stream up to `max_height`,
/// preferring codecs every PC decodes well (H.264, VP9) over AV1.
pub fn format_for(max_height: u32) -> String {
    let h = max_height.clamp(360, 4320);
    format!("bv*[height<=?{h}][vcodec!^=av01]/bv*[height<=?{h}]/b[height<=?{h}]")
}

/// The stream address in yt-dlp's `-g` output (the first https line).
pub fn url_in(stdout: &str) -> Option<String> {
    stdout.lines().map(str::trim).find(|l| l.starts_with("https://")).map(str::to_string)
}

/// What to tell the user when yt-dlp fails.
pub fn explain(stderr: &str) -> String {
    let e = stderr.to_lowercase();
    if e.contains("confirm you") && e.contains("bot") {
        "YouTube asked this PC to sign in to confirm it isn't a bot, so the video can't be fetched right now.".into()
    } else if e.contains("private video") || e.contains("video unavailable") || e.contains("removed") {
        "YouTube says this video is unavailable.".into()
    } else if e.contains("not available in your country") || e.contains("geo") {
        "This video isn't available in your country.".into()
    } else if e.contains("age") && e.contains("sign in") {
        "YouTube only shows this video to signed-in adults.".into()
    } else {
        let last = stderr.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("yt-dlp failed").trim();
        format!("Could not get the video: {}", last.trim_start_matches("ERROR:").trim())
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StreamToolStatus {
    pub installed: bool,
    pub installing: bool,
}

pub struct StreamTool {
    dir: PathBuf,
    installing: AtomicBool,
    /// (video id, max height) → (address, when it was got).
    cache: Mutex<HashMap<(String, u32), (String, u64)>>,
    updated: Mutex<u64>,
}

impl StreamTool {
    pub fn new(dir: PathBuf) -> Self {
        StreamTool { dir, installing: AtomicBool::new(false), cache: Mutex::new(HashMap::new()), updated: Mutex::new(0) }
    }

    fn ytdlp(&self) -> PathBuf {
        self.dir.join(YTDLP_ASSET)
    }

    fn deno(&self) -> PathBuf {
        self.dir.join(DENO_EXE)
    }

    pub fn status(&self) -> StreamToolStatus {
        StreamToolStatus { installed: self.ytdlp().is_file() && self.deno().is_file(), installing: self.installing.load(Ordering::SeqCst) }
    }

    /// Download yt-dlp and Deno (once). `progress(done, total)` in bytes, over both.
    pub fn install(&self, progress: &mut dyn FnMut(u64, u64)) -> Result<(), String> {
        if self.status().installed {
            return Ok(());
        }
        if self.installing.swap(true, Ordering::SeqCst) {
            return Err("The video helper is already downloading.".into());
        }
        let r = self.install_now(progress);
        self.installing.store(false, Ordering::SeqCst);
        r
    }

    fn install_now(&self, progress: &mut dyn FnMut(u64, u64)) -> Result<(), String> {
        use crate::capture::airplay::{download, fetch_text};
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        // About 18 MB and 43 MB: report them as one bar.
        const GUESS: u64 = 61 << 20;
        if !self.ytdlp().is_file() {
            let sums = fetch_text(&format!("{YTDLP_URL}/SHA2-256SUMS")).map_err(|e| format!("Could not reach yt-dlp's releases: {e}"))?;
            let want = checksum_in(&sums, YTDLP_ASSET).ok_or("yt-dlp's release lists no checksum for its program.")?;
            let part = self.dir.join(format!("{YTDLP_ASSET}.part"));
            let got = download(&format!("{YTDLP_URL}/{YTDLP_ASSET}"), &part, &mut |d, _| progress(d, GUESS)).map_err(|e| format!("Could not download yt-dlp: {e}"))?;
            keep(&part, &self.ytdlp(), &got, &want)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(self.ytdlp(), std::fs::Permissions::from_mode(0o755));
            }
        }
        if !self.deno().is_file() {
            let sums = fetch_text(&format!("{DENO_URL}/{DENO_ASSET}.sha256sum")).map_err(|e| format!("Could not reach Deno's releases: {e}"))?;
            let want = checksum_in(&sums, DENO_ASSET).ok_or("Deno's release lists no checksum for its program.")?;
            let base = std::fs::metadata(self.ytdlp()).map(|m| m.len()).unwrap_or(0);
            let part = self.dir.join(format!("{DENO_ASSET}.part"));
            let got = download(&format!("{DENO_URL}/{DENO_ASSET}"), &part, &mut |d, _| progress(base + d, GUESS)).map_err(|e| format!("Could not download Deno: {e}"))?;
            if got != want {
                let _ = std::fs::remove_file(&part);
                return Err("The download did not match its published checksum, so it was not kept.".into());
            }
            let r = unzip_one(&part, DENO_EXE, &self.deno());
            let _ = std::fs::remove_file(&part);
            r?;
        }
        progress(GUESS, GUESS);
        Ok(())
    }

    /// The video's stream address, up to `max_height` lines (cached for hours).
    pub fn url(&self, id: &str, max_height: u32) -> Result<String, String> {
        if !(id.len() == 11 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')) {
            return Err("Not a YouTube video id.".into());
        }
        let now = crate::db::now().max(0) as u64;
        if let Some((u, at)) = self.cache.lock().get(&(id.to_string(), max_height)) {
            if now.saturating_sub(*at) < KEEP_MS {
                return Ok(u.clone());
            }
        }
        if !self.status().installed {
            return Err("The video helper isn't downloaded.".into());
        }
        self.update_now_and_then(now);
        let out = command(&self.ytdlp())
            .arg("--js-runtimes")
            .arg(format!("deno:{}", self.deno().display()))
            .args(["--no-playlist", "--no-warnings", "--no-progress", "--ignore-config", "--cache-dir"])
            .arg(self.dir.join("cache"))
            .args(["-f", &format_for(max_height), "-g", "--"])
            .arg(format!("https://www.youtube.com/watch?v={id}"))
            .output()
            .map_err(|e| format!("Could not start yt-dlp: {e}"))?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        match url_in(&stdout) {
            Some(u) if out.status.success() => {
                let mut c = self.cache.lock();
                if c.len() > 100 {
                    c.clear();
                }
                c.insert((id.to_string(), max_height), (u.clone(), now));
                Ok(u)
            }
            _ => Err(explain(&String::from_utf8_lossy(&out.stderr))),
        }
    }

    /// yt-dlp updates itself (checked against its release's checksums), at
    /// most once a day, in the background.
    fn update_now_and_then(&self, now: u64) {
        let mut last = self.updated.lock();
        if now.saturating_sub(*last) < 24 * 3600 * 1000 {
            return;
        }
        *last = now;
        let exe = self.ytdlp();
        std::thread::spawn(move || {
            let _ = command(&exe).args(["-U", "--no-warnings"]).output();
        });
    }
}

fn command(exe: &Path) -> std::process::Command {
    #[allow(unused_mut)]
    let mut c = std::process::Command::new(exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    c
}

fn keep(part: &Path, dest: &Path, got: &str, want: &str) -> Result<(), String> {
    if got != want {
        let _ = std::fs::remove_file(part);
        return Err("The download did not match its published checksum, so it was not kept.".into());
    }
    std::fs::rename(part, dest).map_err(|e| e.to_string())
}

/// Take `name` out of a zip file.
fn unzip_one(zip_path: &Path, name: &str, dest: &Path) -> Result<(), String> {
    let f = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| e.to_string())?;
    let mut entry = z.by_name(name).map_err(|_| format!("{name} is not in the download."))?;
    let tmp = dest.with_extension("part");
    let mut out = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    drop(out);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
    }
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_both_checksum_formats() {
        let ytdlp = "1fa6733c37ea6fb51c99ad8fe785e7b7e5f3246c9b980230329d4fb72ed8d4d6  yt-dlp\n66674953FE251B89F4D08C5F0E35E0728679BD67AB3D7D05C0562AF101DD3E7A  yt-dlp.exe\n";
        assert_eq!(checksum_in(ytdlp, "yt-dlp.exe").as_deref(), Some("66674953fe251b89f4d08c5f0e35e0728679bd67ab3d7d05c0562af101dd3e7a"));
        assert_eq!(checksum_in(ytdlp, "yt-dlp").as_deref(), Some("1fa6733c37ea6fb51c99ad8fe785e7b7e5f3246c9b980230329d4fb72ed8d4d6"));
        // Deno publishes PowerShell's Get-FileHash output.
        let deno = "\nAlgorithm : SHA256\nHash      : A0C3101B4158D1DFB7D6A78A7BF0F3DE80C96BB423C152BEEC8BEB22786F2238\nPath      : C:\\a\\deno\\deno\\target\\release\\deno-x86_64-pc-windows-msvc.zip\n";
        assert_eq!(checksum_in(deno, "deno-x86_64-pc-windows-msvc.zip").as_deref(), Some("a0c3101b4158d1dfb7d6a78a7bf0f3de80c96bb423c152beec8beb22786f2238"));
        assert_eq!(checksum_in("nothing here", "x"), None);
    }

    #[test]
    fn asks_for_a_video_stream_within_the_quality_cap() {
        assert_eq!(format_for(1080), "bv*[height<=?1080][vcodec!^=av01]/bv*[height<=?1080]/b[height<=?1080]");
        assert!(format_for(100_000).contains("4320"));
        assert_eq!(url_in("\nhttps://rr1---sn.googlevideo.com/videoplayback?expire=1\n").as_deref(), Some("https://rr1---sn.googlevideo.com/videoplayback?expire=1"));
        assert_eq!(url_in("WARNING: x\n"), None);
    }

    #[test]
    fn says_why_a_stream_could_not_be_had() {
        assert!(explain("ERROR: [youtube] abc: Sign in to confirm you’re not a bot. Use --cookies").contains("isn't a bot"));
        assert!(explain("ERROR: [youtube] abc: Video unavailable").contains("unavailable"));
        assert_eq!(explain("ERROR: something new\n"), "Could not get the video: something new");
    }

    #[test]
    fn refuses_odd_ids_and_needs_the_helper() {
        let t = StreamTool::new(std::env::temp_dir().join(format!("omnihub-stream-{}", std::process::id())));
        assert!(t.url("../../etc", 1080).is_err());
        assert_eq!(t.url("_PJvpq8uOZM", 1080).unwrap_err(), "The video helper isn't downloaded.");
        assert!(!t.status().installed);
    }
}
