//! The game library as the pages show it (a Steam Deck–style shelf): every
//! installed game with its artwork, playtime and status; one game's details
//! (players now, screenshots, soundtrack, sessions, saves); session recaps
//! when a game closes; and the Steam extras (achievements, friends,
//! wishlist sales).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use super::shelf::{Backup, SessionRec, ShelfEntry};
use super::steam::{self, Art, SteamUser};
use crate::core::AppCore;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryGame {
    pub key: String,
    pub name: String,
    pub source: String,
    pub app: Option<u32>,
    pub process: String,
    pub art: Art,
    pub status: String,
    pub favorite: bool,
    pub minutes: u64,
    /// Unix ms (0: never).
    pub last_played: i64,
    pub profile_id: Option<String>,
    pub can_launch: bool,
}

/// A ROM's title without the No-Intro tags: "Pokemon - Emerald Version (USA, Europe)" → "Pokemon - Emerald Version".
pub fn rom_title(name: &str) -> String {
    let mut out = String::new();
    let mut depth = 0i32;
    for c in name.chars() {
        match c {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    let t = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() {
        name.to_string()
    } else {
        t
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    /// `gamemedia://localhost/<token>`.
    pub src: String,
    pub name: String,
    pub at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameDetail {
    pub key: String,
    pub players: Option<u64>,
    pub screenshots: Vec<Media>,
    pub soundtrack: Vec<Media>,
    pub sessions: Vec<SessionRec>,
    pub entry: ShelfEntry,
    pub suggested_dirs: Vec<String>,
    pub backups: Vec<Backup>,
    pub minutes_today: u64,
    pub install_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Recap {
    pub key: String,
    pub name: String,
    pub start: i64,
    pub end: i64,
    pub minutes: u64,
    pub avg_fps: Option<f32>,
    pub low_fps: Option<f32>,
    pub clips: Vec<Media>,
    pub screenshots: Vec<Media>,
    pub backup: Option<String>,
    pub art: Art,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SteamAccount {
    pub user: Option<SteamUser>,
    pub has_key: bool,
    pub has_sgdb: bool,
}

fn media_src(core: &AppCore, p: &Path) -> String {
    super::shelf::media_url(&core.shelf.media_token(p))
}

fn modified_ms(p: &Path) -> i64 {
    std::fs::metadata(p).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as i64)
}

impl AppCore {
    fn steam_root(&self) -> Option<PathBuf> {
        super::library::steam_root()
    }

    pub fn library_account(&self) -> SteamAccount {
        SteamAccount { user: self.steam_root().and_then(|s| steam::current_user(&s)), has_key: self.shelf.has_key("steam"), has_sgdb: self.shelf.has_key("sgdb") }
    }

    /// Every installed game, with artwork, playtime and its status here.
    pub fn library_games(&self) -> Vec<LibraryGame> {
        let steam_root = self.steam_root();
        let user = steam_root.as_deref().and_then(steam::current_user);
        let steam_times = match (&steam_root, &user) {
            (Some(s), Some(u)) => steam::playtimes(s, u),
            _ => HashMap::new(),
        };
        let entries = self.shelf.entries();
        let totals = self.shelf.totals();
        let mut list: Vec<LibraryGame> = self
            .games
            .library()
            .into_iter()
            .map(|g| {
                let app = g.key.strip_prefix("steam:").and_then(|a| a.parse::<u32>().ok());
                let e = entries.get(&g.key).cloned().unwrap_or_default();
                // Steam's current addresses when known, else its older fixed ones.
                let found = self.store_art.get(&g.key);
                let mut art = app.or(found.as_ref().and_then(|f| f.app)).map(steam::art_urls).unwrap_or_default();
                if let Some(f) = found.and_then(|f| f.art) {
                    art = steam::Art { cover: f.cover.or(art.cover), hero: f.hero.or(art.hero), logo: f.logo.or(art.logo), header: f.header.or(art.header) };
                }
                if let (Some(app), Some(root)) = (app, &steam_root) {
                    // Steam's own copies here (and custom art set in Steam) come first: they work offline.
                    let mut local = steam::local_art(root, app);
                    if let Some(u) = &user {
                        local.extend(steam::grid_art(root, u, app));
                    }
                    for (kind, p) in local {
                        let src = Some(media_src(self, &p));
                        match kind {
                            "cover" => art.cover = src,
                            "hero" => art.hero = src,
                            "logo" => art.logo = src,
                            _ => art.header = src,
                        }
                    }
                }
                if let Some(v) = e.cover.as_deref().and_then(|v| self.shelf.art_src(v)) {
                    art.cover = Some(v);
                }
                if let Some(v) = e.hero.as_deref().and_then(|v| self.shelf.art_src(v)) {
                    art.hero = Some(v);
                }
                if let Some(v) = e.logo.as_deref().and_then(|v| self.shelf.art_src(v)) {
                    art.logo = Some(v);
                }
                let tracked = totals.get(&g.key).copied().unwrap_or((0, 0));
                let (steam_min, steam_last) = app.and_then(|a| steam_times.get(&a)).map_or((0, 0), |&(m, l)| (m as u64, l * 1000));
                LibraryGame {
                    can_launch: !matches!(g.launch, super::Launch::None),
                    key: g.key,
                    name: g.name,
                    source: g.source,
                    app,
                    process: g.process,
                    art,
                    status: e.status,
                    favorite: e.favorite,
                    minutes: steam_min.max(tracked.0),
                    last_played: steam_last.max(tracked.1),
                    profile_id: g.profile_id,
                }
            })
            .collect();
        self.find_store_art(&list);
        // Your ROMs too, with libretro's box art, playing in their emulator.
        for r in self.emulators.state().roms {
            let key = format!("rom:{}", r.id);
            let e = entries.get(&key).cloned().unwrap_or_default();
            let tracked = totals.get(&key).copied().unwrap_or((0, 0));
            let mut art = Art { cover: r.cover.clone(), ..Default::default() };
            if let Some(v) = e.cover.as_deref().and_then(|v| self.shelf.art_src(v)) {
                art.cover = Some(v);
            }
            list.push(LibraryGame { name: rom_title(&r.name), key, source: "rom".into(), app: None, process: String::new(), art, status: e.status, favorite: e.favorite, minutes: tracked.0, last_played: tracked.1, profile_id: None, can_launch: true });
        }
        list
    }

    /// In the background, ask Steam's store for art it hasn't been asked
    /// about this week (games with their own art set here are skipped).
    fn find_store_art(&self, list: &[LibraryGame]) {
        let now = chrono::Utc::now().timestamp();
        let entries = self.shelf.entries();
        let todo = self.store_art.missing(list.iter().filter(|g| entries.get(&g.key).is_none_or(|e| e.cover.is_none())).map(|g| (g.key.as_str(), g.app, g.name.as_str())), now);
        if todo.is_empty() {
            return;
        }
        let store = self.store_art.clone();
        let events = self.events.clone();
        std::thread::Builder::new()
            .name("store-art".into())
            .spawn(move || {
                if store.fetch(todo, now, &steam::http_get) {
                    events.emit("library:changed", serde_json::json!({}));
                }
            })
            .ok();
    }

    fn find_game(&self, key: &str) -> Result<super::library::InstalledGame, String> {
        if let Some(id) = key.strip_prefix("rom:") {
            let r = self.emulators.state().roms.into_iter().find(|r| r.id == id).ok_or("That ROM isn't in your ROM folders any more")?;
            let dir = Path::new(&r.path).parent().map(|d| d.display().to_string());
            return Ok(super::library::InstalledGame { key: key.into(), name: rom_title(&r.name), source: "rom".into(), kind: Default::default(), launch: Default::default(), process: String::new(), exe_path: Some(r.path), install_dir: dir, profile_id: None });
        }
        self.games.library().into_iter().find(|g| g.key == key).ok_or_else(|| "That game isn't installed any more".into())
    }

    /// One game's details. The player count is asked of Steam (a second or so).
    pub fn library_detail(&self, key: &str) -> Result<GameDetail, String> {
        let g = self.find_game(key)?;
        let app = key.strip_prefix("steam:").and_then(|a| a.parse::<u32>().ok());
        let steam_root = self.steam_root();
        let user = steam_root.as_deref().and_then(steam::current_user);
        let players = app.and_then(|a| steam::player_count(a, &steam::http_get).ok());
        // Screenshots: OmniHub's own of this game's window, and Steam's (F12).
        let mut shots: Vec<Media> = Vec::new();
        let filter = crate::capture::screenshots::ShotFilter { query: String::new(), tag: None, app: None, favorites: false, limit: Some(2000) };
        let exe = g.process.to_lowercase();
        if let Ok(list) = self.screenshots.list(&filter) {
            for s in list.into_iter().filter(|s| s.exists && (s.app_exe.as_deref().is_some_and(|a| !exe.is_empty() && a.eq_ignore_ascii_case(&exe)) || s.app_title.as_deref().is_some_and(|t| t.to_lowercase().contains(&g.name.to_lowercase())))) {
                shots.push(Media { src: media_src(self, Path::new(&s.path)), name: s.app_title.unwrap_or_default(), at: s.created * 1000 });
            }
        }
        if let (Some(a), Some(root), Some(u)) = (app, &steam_root, &user) {
            for p in steam::screenshots(root, u, a).into_iter().take(60) {
                shots.push(Media { at: modified_ms(&p), name: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), src: media_src(self, &p) });
            }
        }
        shots.sort_by_key(|m| std::cmp::Reverse(m.at));
        shots.truncate(60);
        let soundtrack = steam_root
            .as_deref()
            .map(|r| steam::soundtrack(&steam::library_folders(r), &g.name))
            .unwrap_or_default()
            .into_iter()
            .map(|p| Media { name: p.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), at: 0, src: media_src(self, &p) })
            .collect();
        let entry = self.shelf.entry(key);
        let home = dirs::home_dir().unwrap_or_default();
        let steam_bits = match (&steam_root, &user, app) {
            (Some(r), Some(u), Some(a)) => Some((r.as_path(), u.account_id, a)),
            _ => None,
        };
        let suggested_dirs = super::shelf::guess_save_dirs(&home, &g.name, steam_bits).into_iter().filter(|d| !entry.save_dirs.contains(d)).collect();
        Ok(GameDetail {
            key: key.into(),
            players,
            screenshots: shots,
            soundtrack,
            sessions: self.shelf.sessions(Some(key), 30),
            backups: self.shelf.backups(key),
            minutes_today: self.shelf.minutes_today(key, None),
            entry,
            suggested_dirs,
            install_dir: g.install_dir,
        })
    }

    pub fn library_achievements(&self, key: &str) -> Result<Vec<steam::Achievement>, String> {
        let app = key.strip_prefix("steam:").and_then(|a| a.parse::<u32>().ok()).ok_or("Achievements are for Steam games")?;
        let api = self.shelf.steam_key().ok_or("Add your Steam Web API key first")?;
        let user = self.steam_root().and_then(|s| steam::current_user(&s)).ok_or("Sign in to Steam on this PC first")?;
        steam::achievements(&api, &user.steamid, app, &steam::http_get)
    }

    pub fn library_friends(&self) -> Result<Vec<steam::Friend>, String> {
        let api = self.shelf.steam_key().ok_or("Add your Steam Web API key first")?;
        let user = self.steam_root().and_then(|s| steam::current_user(&s)).ok_or("Sign in to Steam on this PC first")?;
        steam::friends(&api, &user.steamid, &steam::http_get)
    }

    pub fn library_wishlist(&self) -> Result<Vec<steam::WishItem>, String> {
        let user = self.steam_root().and_then(|s| steam::current_user(&s)).ok_or("Sign in to Steam on this PC first")?;
        steam::wishlist(&user.steamid, &steam::http_get)
    }

    pub fn library_play(&self, key: &str) -> Result<(), String> {
        if let Some(id) = key.strip_prefix("rom:") {
            let (exe, args) = self.emulators.command(id)?;
            let mut c = std::process::Command::new(&exe);
            c.args(&args);
            if let Some(dir) = Path::new(&exe).parent() {
                c.current_dir(dir);
            }
            return c.spawn().map(|_| ()).map_err(|e| format!("Couldn't start the emulator: {e}"));
        }
        let g = self.find_game(key)?;
        // Its scenes first (the screen mode is set before the game opens).
        self.scenes_game_started(key);
        match super::launch_target(&g.launch)? {
            Some((target, args)) => super::start(&target, &args).map_err(|e| e.to_string()),
            None => Err("OmniHub doesn't know how to start this one: open it from its launcher".into()),
        }
    }

    /// Cover, background and logo from SteamGridDB (with your key).
    pub fn library_find_art(&self, key: &str) -> Result<ShelfEntry, String> {
        let api = self.shelf.sgdb_key().ok_or("Add your SteamGridDB key first (steamgriddb.com → Preferences → API)")?;
        let g = self.find_game(key)?;
        let art = steam::steamgriddb(&api, &g.name)?;
        let mut e = self.shelf.entry(key);
        for (kind, url) in [("cover", art.cover), ("hero", art.hero), ("logo", art.logo)] {
            if let Some(u) = url.filter(|u| u.starts_with("https://")) {
                e = self.shelf.set_art(key, kind, Some(&u))?;
            }
        }
        Ok(e)
    }

    /// What a session added up to: FPS, clips and screenshots from that time.
    pub fn recap(&self, s: &SessionRec) -> Recap {
        let fps: Vec<_> = self.games.fps_history(None).into_iter().filter(|r| r.at * 1000 >= s.start && r.at * 1000 <= s.end + 60_000 && r.name.eq_ignore_ascii_case(&s.name)).collect();
        let n = fps.len() as f32;
        let (avg, low) = if fps.is_empty() { (None, None) } else { (Some(fps.iter().map(|r| r.summary.avg).sum::<f32>() / n), Some(fps.iter().map(|r| r.summary.low1).fold(f32::MAX, f32::min))) };
        let clips = self.play.clips(&self.settings.get().play).into_iter().filter(|c| c.created * 1000 >= s.start && c.created * 1000 <= s.end + 120_000).map(|c| Media { src: media_src(self, Path::new(&c.path)), name: c.name, at: c.created * 1000 }).collect();
        let filter = crate::capture::screenshots::ShotFilter { query: String::new(), tag: None, app: None, favorites: false, limit: Some(500) };
        let screenshots = self.screenshots.list(&filter).unwrap_or_default().into_iter().filter(|x| x.exists && x.created * 1000 >= s.start && x.created * 1000 <= s.end + 60_000).take(12).map(|x| Media { src: media_src(self, Path::new(&x.path)), name: x.app_title.unwrap_or_default(), at: x.created * 1000 }).collect();
        let art = self.library_games().into_iter().find(|g| g.key == s.key).map(|g| g.art).unwrap_or_default();
        Recap { key: s.key.clone(), name: s.name.clone(), start: s.start, end: s.end, minutes: s.minutes(), avg_fps: avg, low_fps: low, clips, screenshots, backup: None, art }
    }

    /// Follow games as their programs run: sessions, recaps, automatic save
    /// backups, daily limits, and now and then wishlist sales.
    pub fn start_game_tracking(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        std::thread::Builder::new()
            .name("game-sessions".into())
            .spawn(move || {
                let mut sys = sysinfo::System::new();
                let mut games: Vec<(String, String, String)> = Vec::new();
                let mut games_at = std::time::Instant::now() - Duration::from_secs(3600);
                let mut active: HashMap<String, (String, i64, u32)> = HashMap::new();
                let mut warned: HashMap<String, u8> = HashMap::new();
                let mut sales_at = std::time::Instant::now() - Duration::from_secs(5 * 3600);
                loop {
                    // Games with a scene (like a play resolution) are watched every 4 s, so the
                    // screen switches and comes back quickly; otherwise every 15 s.
                    let fast = {
                        let Some(core) = weak.upgrade() else { return };
                        core.quick.state().scenes.iter().any(|s| !s.game.is_empty())
                    };
                    std::thread::sleep(Duration::from_secs(if fast { 4 } else { 15 }));
                    let Some(core) = weak.upgrade() else { return };
                    if games_at.elapsed() > Duration::from_secs(600) {
                        games = core.games.library().into_iter().filter(|g| !g.process.is_empty()).map(|g| (g.key, g.name, g.process.to_lowercase())).collect();
                        games_at = std::time::Instant::now();
                    }
                    sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, sysinfo::ProcessRefreshKind::nothing());
                    let running: std::collections::HashSet<String> = sys.processes().values().map(|p| p.name().to_string_lossy().to_lowercase()).collect();
                    let now = chrono::Utc::now().timestamp_millis();
                    for (key, name, exe) in &games {
                        if running.contains(exe) {
                            if !active.contains_key(key) {
                                core.scenes_game_started(key);
                            }
                            let e = active.entry(key.clone()).or_insert_with(|| (name.clone(), now, 0));
                            e.2 = 0;
                        }
                    }
                    // What's playing now (the longest-running), for the Discord status.
                    *core.now_playing.lock() = active.values().filter(|a| a.2 == 0).min_by_key(|a| a.1).map(|a| (a.0.clone(), a.1));
                    let mut ended = Vec::new();
                    for (key, (name, start, misses)) in active.iter_mut() {
                        let exe = games.iter().find(|g| &g.0 == key).map(|g| g.2.as_str()).unwrap_or("");
                        if running.contains(exe) {
                            core.check_limit(key, name, *start, &mut warned);
                            continue;
                        }
                        *misses += 1;
                        if *misses >= 2 {
                            ended.push(SessionRec { key: key.clone(), name: name.clone(), start: *start, end: now - 15_000 });
                        }
                    }
                    for s in ended {
                        active.remove(&s.key);
                        warned.remove(&s.key);
                        core.scenes_game_ended(&s.key);
                        if core.shelf.record(s.clone()).is_err() || s.minutes() < 1 {
                            continue;
                        }
                        let mut recap = core.recap(&s);
                        if core.shelf.entry(&s.key).auto_backup {
                            recap.backup = core.shelf.backup(&s.key, "auto").ok().map(|b| b.id);
                        }
                        core.events.emit("games:recap", &recap);
                    }
                    // Wishlist sales, every six hours.
                    if sales_at.elapsed() > Duration::from_secs(6 * 3600) {
                        sales_at = std::time::Instant::now();
                        if let Ok(list) = core.library_wishlist() {
                            for w in core.shelf.new_sales(&list) {
                                core.events.emit("games:sale", &w);
                            }
                        }
                    }
                }
            })
            .ok();
    }

    /// Daily game-time limits: a warning 5 minutes before, then the game is closed.
    fn check_limit(&self, key: &str, name: &str, start: i64, warned: &mut HashMap<String, u8>) {
        let limit = self.shelf.entry(key).daily_limit as u64;
        if limit == 0 {
            return;
        }
        let used = self.shelf.minutes_today(key, Some(start));
        let level = warned.entry(key.to_string()).or_insert(0);
        if used + 5 >= limit && *level == 0 {
            *level = 1;
            self.events.emit("games:limit", serde_json::json!({ "key": key, "name": name, "minutes": limit, "left": limit.saturating_sub(used), "closing": false }));
        }
        if used >= limit && *level < 2 {
            *level = 2;
            if let Some(exe) = self.games.library().into_iter().find(|g| g.key == key).map(|g| g.process).filter(|p| !p.is_empty()) {
                let _ = self.procs.end(&exe);
            }
            self.events.emit("games:limit", serde_json::json!({ "key": key, "name": name, "minutes": limit, "left": 0, "closing": true }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rom_titles() {
        assert_eq!(rom_title("Pokemon - Emerald Version (USA, Europe)"), "Pokemon - Emerald Version");
        assert_eq!(rom_title("Metroid Fusion (USA) [!] (Rev 1)"), "Metroid Fusion");
        assert_eq!(rom_title("(Unknown)"), "(Unknown)");
    }
}
