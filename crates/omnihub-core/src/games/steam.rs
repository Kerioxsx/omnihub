//! Steam, for the game library: who's signed in, playtime, artwork, the
//! soundtracks and screenshots on this PC, and from Steam's servers the live
//! player count, wishlist prices, and (with your own Steam Web API key)
//! achievements and friends. Nothing is sent but the requests themselves.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A Valve KeyValues text file as nested JSON objects (keys lower-cased).
pub fn parse_vdf(text: &str) -> Value {
    fn token(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<String> {
        loop {
            match chars.peek()? {
                c if c.is_whitespace() => {
                    chars.next();
                }
                '/' => {
                    // A // comment to the end of the line.
                    for c in chars.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                }
                '{' | '}' => return chars.next().map(String::from),
                '"' => {
                    chars.next();
                    let mut s = String::from("\u{1}");
                    while let Some(c) = chars.next() {
                        match c {
                            '\\' => match chars.next() {
                                Some('n') => s.push('\n'),
                                Some('t') => s.push('\t'),
                                Some(n) => s.push(n),
                                None => break,
                            },
                            '"' => break,
                            c => s.push(c),
                        }
                    }
                    return Some(s);
                }
                _ => {
                    let mut s = String::from("\u{1}");
                    while let Some(&c) = chars.peek() {
                        if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
                            break;
                        }
                        s.push(c);
                        chars.next();
                    }
                    return Some(s);
                }
            }
        }
    }
    fn block(chars: &mut std::iter::Peekable<std::str::Chars>, depth: usize) -> Map<String, Value> {
        let mut out = Map::new();
        while let Some(t) = token(chars) {
            if t == "}" {
                break;
            }
            let Some(key) = t.strip_prefix('\u{1}') else { continue };
            let key = key.to_lowercase();
            match token(chars) {
                Some(v) if v == "{" => {
                    let inner = if depth < 64 { block(chars, depth + 1) } else { Map::new() };
                    out.insert(key, Value::Object(inner));
                }
                Some(v) => {
                    out.insert(key, Value::String(v.trim_start_matches('\u{1}').to_string()));
                }
                None => break,
            }
        }
        out
    }
    let mut chars = text.chars().peekable();
    Value::Object(block(&mut chars, 0))
}

/// Find a value by a path of keys (case-insensitive).
pub fn vdf_get<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(v, |cur, k| cur.get(k.to_lowercase()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SteamUser {
    pub steamid: String,
    pub account_id: u32,
    pub name: String,
}

const STEAM64_BASE: u64 = 76_561_197_960_265_728;

/// The Steam account last signed in on this PC.
pub fn current_user(steam: &Path) -> Option<SteamUser> {
    let text = std::fs::read_to_string(steam.join("config").join("loginusers.vdf")).ok()?;
    let v = parse_vdf(&text);
    let users = vdf_get(&v, &["users"])?.as_object()?;
    let pick = users
        .iter()
        .max_by_key(|(_, u)| {
            let recent = u.get("mostrecent").and_then(Value::as_str) == Some("1");
            let ts = u.get("timestamp").and_then(Value::as_str).and_then(|t| t.parse::<i64>().ok()).unwrap_or(0);
            (recent, ts)
        })?;
    let id64: u64 = pick.0.parse().ok()?;
    let name = pick.1.get("personaname").and_then(Value::as_str).unwrap_or("").to_string();
    Some(SteamUser { steamid: id64.to_string(), account_id: id64.checked_sub(STEAM64_BASE)? as u32, name })
}

/// Minutes played and when last played (Unix seconds), by app id.
pub fn playtimes(steam: &Path, user: &SteamUser) -> HashMap<u32, (u32, i64)> {
    let file = steam.join("userdata").join(user.account_id.to_string()).join("config").join("localconfig.vdf");
    let Ok(text) = std::fs::read_to_string(file) else { return HashMap::new() };
    let v = parse_vdf(&text);
    let Some(apps) = vdf_get(&v, &["userlocalconfigstore", "software", "valve", "steam", "apps"]).and_then(Value::as_object) else { return HashMap::new() };
    apps.iter()
        .filter_map(|(id, a)| {
            let id: u32 = id.parse().ok()?;
            let num = |k: &str| a.get(k).and_then(Value::as_str).and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
            Some((id, (num("playtime") as u32, num("lastplayed"))))
        })
        .collect()
}

/// Steam's library folders (the main one first).
pub fn library_folders(steam: &Path) -> Vec<PathBuf> {
    let mut out = vec![steam.to_path_buf()];
    if let Ok(text) = std::fs::read_to_string(steam.join("steamapps").join("libraryfolders.vdf")) {
        let v = parse_vdf(&text);
        if let Some(folders) = vdf_get(&v, &["libraryfolders"]).and_then(Value::as_object) {
            for f in folders.values() {
                if let Some(p) = f.get("path").and_then(Value::as_str) {
                    let p = PathBuf::from(p);
                    if !out.iter().any(|o| o.to_string_lossy().eq_ignore_ascii_case(&p.to_string_lossy())) {
                        out.push(p);
                    }
                }
            }
        }
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Art {
    /// Tall cover (2:3).
    pub cover: Option<String>,
    /// Wide background.
    pub hero: Option<String>,
    pub logo: Option<String>,
    /// Wide banner, a fallback for the cover.
    pub header: Option<String>,
}

/// Steam's own artwork for an app (its servers).
pub fn art_urls(app: u32) -> Art {
    let base = format!("https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/{app}");
    Art { cover: Some(format!("{base}/library_600x900_2x.jpg")), hero: Some(format!("{base}/library_hero.jpg")), logo: Some(format!("{base}/logo.png")), header: Some(format!("{base}/header.jpg")) }
}

/// Artwork Steam keeps on this PC (also custom art set in Steam), by kind.
pub fn local_art(steam: &Path, app: u32) -> HashMap<&'static str, PathBuf> {
    let cache = steam.join("appcache").join("librarycache");
    let mut out = HashMap::new();
    for (kind, names) in [("cover", ["library_600x900_2x.jpg", "library_600x900.jpg"]), ("hero", ["library_hero.jpg", "library_hero_blur.jpg"]), ("logo", ["logo.png", "logo_2x.png"]), ("header", ["header.jpg", "library_header.jpg"])] {
        for n in names {
            let folder = cache.join(app.to_string()).join(n);
            let flat = cache.join(format!("{app}_{n}"));
            if let Some(p) = [folder, flat].into_iter().find(|p| p.is_file()) {
                out.insert(kind, p);
                break;
            }
        }
    }
    // Custom art set in Steam: userdata/<id>/config/grid/<app>p.png (cover), <app>_hero, <app>_logo.
    out
}

/// Custom artwork set in Steam's library (grid folder), by kind.
pub fn grid_art(steam: &Path, user: &SteamUser, app: u32) -> HashMap<&'static str, PathBuf> {
    let grid = steam.join("userdata").join(user.account_id.to_string()).join("config").join("grid");
    let mut out = HashMap::new();
    for (kind, stem) in [("cover", format!("{app}p")), ("hero", format!("{app}_hero")), ("logo", format!("{app}_logo")), ("header", app.to_string())] {
        for ext in ["png", "jpg", "jpeg", "webp"] {
            let p = grid.join(format!("{stem}.{ext}"));
            if p.is_file() {
                out.insert(kind, p);
                break;
            }
        }
    }
    out
}

/// Letters and digits only, lower case ("Hollow Knight: Silksong" → "hollowknightsilksong").
pub fn squash(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

const AUDIO: [&str; 7] = ["mp3", "flac", "ogg", "wav", "m4a", "opus", "aac"];

/// A game's soundtrack bought on Steam (steamapps/music/<name>), its tracks in order.
pub fn soundtrack(libraries: &[PathBuf], game: &str) -> Vec<PathBuf> {
    let want = squash(game);
    if want.len() < 3 {
        return Vec::new();
    }
    for lib in libraries {
        let Ok(rd) = std::fs::read_dir(lib.join("steamapps").join("music")) else { continue };
        for e in rd.flatten() {
            let folder = squash(&e.file_name().to_string_lossy());
            if folder.contains(&want) || (folder.len() > 4 && want.contains(folder.trim_end_matches("soundtrack").trim_end_matches("ost"))) {
                let mut tracks: Vec<PathBuf> = walkdir::WalkDir::new(e.path())
                    .max_depth(3)
                    .into_iter()
                    .flatten()
                    .map(|e| e.into_path())
                    .filter(|p| p.extension().is_some_and(|x| AUDIO.contains(&x.to_string_lossy().to_lowercase().as_str())))
                    .collect();
                tracks.sort();
                if !tracks.is_empty() {
                    return tracks;
                }
            }
        }
    }
    Vec::new()
}

/// Screenshots taken with Steam (F12) for an app, newest first.
pub fn screenshots(steam: &Path, user: &SteamUser, app: u32) -> Vec<PathBuf> {
    let dir = steam.join("userdata").join(user.account_id.to_string()).join("760").join("remote").join(app.to_string()).join("screenshots");
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut v: Vec<(std::time::SystemTime, PathBuf)> = rd
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("jpg") || x.eq_ignore_ascii_case("png")))
        .map(|e| (e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH), e.path()))
        .collect();
    v.sort_by_key(|x| std::cmp::Reverse(x.0));
    v.into_iter().map(|(_, p)| p).collect()
}

// ---------- Steam's servers ----------

pub type Get<'a> = &'a dyn Fn(&str) -> Result<String, String>;

fn enc(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}

/// GET a Steam (or SteamGridDB) URL; only those hosts.
pub fn http_get(url: &str) -> Result<String, String> {
    http_get_with(url, None)
}

pub fn http_get_with(url: &str, bearer: Option<&str>) -> Result<String, String> {
    let ok = ["https://api.steampowered.com/", "https://store.steampowered.com/", "https://www.steamgriddb.com/api/"].iter().any(|h| url.starts_with(h));
    if !ok {
        return Err("only Steam is asked".into());
    }
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(15))).http_status_as_error(false).build().into();
    let mut req = agent.get(url).header("User-Agent", "OmniHub");
    if let Some(b) = bearer {
        req = req.header("Authorization", &format!("Bearer {b}"));
    }
    let mut res = req.call().map_err(|e| format!("couldn't reach Steam: {e}"))?;
    let status = res.status().as_u16();
    let body = res.body_mut().with_config().limit(8 << 20).read_to_string().map_err(|e| e.to_string())?;
    match status {
        200..=299 => Ok(body),
        401 | 403 => Err("Steam said no: check the key, and that your profile and game details are public".into()),
        429 => Err("Steam asks to slow down: try again in a minute".into()),
        s => Err(format!("Steam answered {s}")),
    }
}

/// People playing right now.
pub fn player_count(app: u32, get: Get) -> Result<u64, String> {
    let v: Value = serde_json::from_str(&get(&format!("https://api.steampowered.com/ISteamUserStats/GetNumberOfCurrentPlayers/v1/?appid={app}"))?).map_err(|e| e.to_string())?;
    v["response"]["player_count"].as_u64().ok_or_else(|| "no player count for this game".into())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub icon_gray: String,
    pub unlocked: bool,
    /// Unix seconds.
    pub unlocked_at: i64,
    /// Share of players who have it.
    pub percent: f64,
    pub hidden: bool,
}

/// Your achievements for a game (needs your Steam Web API key).
pub fn achievements(key: &str, steamid: &str, app: u32, get: Get) -> Result<Vec<Achievement>, String> {
    let schema: Value = serde_json::from_str(&get(&format!("https://api.steampowered.com/ISteamUserStats/GetSchemaForGame/v2/?key={key}&appid={app}&l=english"))?).map_err(|e| e.to_string())?;
    let list = schema["game"]["availableGameStats"]["achievements"].as_array().cloned().unwrap_or_default();
    if list.is_empty() {
        return Ok(Vec::new());
    }
    let mine: Value = serde_json::from_str(&get(&format!("https://api.steampowered.com/ISteamUserStats/GetPlayerAchievements/v1/?key={key}&steamid={steamid}&appid={app}"))?).map_err(|e| e.to_string())?;
    let unlocked: HashMap<String, i64> = mine["playerstats"]["achievements"].as_array().into_iter().flatten().filter(|a| a["achieved"].as_i64() == Some(1)).filter_map(|a| Some((a["apiname"].as_str()?.to_string(), a["unlocktime"].as_i64().unwrap_or(0)))).collect();
    let global: HashMap<String, f64> = get(&format!("https://api.steampowered.com/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/?gameid={app}"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .map(|v| v["achievementpercentages"]["achievements"].as_array().into_iter().flatten().filter_map(|a| Some((a["name"].as_str()?.to_string(), a["percent"].as_f64().or_else(|| a["percent"].as_str()?.parse().ok())?))).collect())
        .unwrap_or_default();
    Ok(list
        .iter()
        .filter_map(|a| {
            let id = a["name"].as_str()?.to_string();
            Some(Achievement {
                name: a["displayName"].as_str().unwrap_or(&id).to_string(),
                description: a["description"].as_str().unwrap_or("").to_string(),
                icon: a["icon"].as_str().unwrap_or("").to_string(),
                icon_gray: a["icongray"].as_str().unwrap_or("").to_string(),
                unlocked: unlocked.contains_key(&id),
                unlocked_at: unlocked.get(&id).copied().unwrap_or(0),
                percent: global.get(&id).copied().unwrap_or(0.0),
                hidden: a["hidden"].as_i64() == Some(1),
                id,
            })
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Friend {
    pub steamid: String,
    pub name: String,
    pub avatar: String,
    /// 0 offline, 1 online, 2 busy, 3 away, 4 snooze, 5 looking to trade, 6 looking to play.
    pub state: u8,
    /// The game they're in.
    pub game: Option<String>,
    pub game_id: Option<u32>,
}

/// Your friends and what they play (needs your key; your friend list must be public).
pub fn friends(key: &str, steamid: &str, get: Get) -> Result<Vec<Friend>, String> {
    let list: Value = serde_json::from_str(&get(&format!("https://api.steampowered.com/ISteamUser/GetFriendList/v1/?key={key}&steamid={steamid}&relationship=friend"))?).map_err(|e| e.to_string())?;
    let ids: Vec<String> = list["friendslist"]["friends"].as_array().into_iter().flatten().filter_map(|f| f["steamid"].as_str().map(String::from)).collect();
    let mut out = Vec::new();
    for chunk in ids.chunks(100) {
        let v: Value = serde_json::from_str(&get(&format!("https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/?key={key}&steamids={}", chunk.join(",")))?).map_err(|e| e.to_string())?;
        for p in v["response"]["players"].as_array().into_iter().flatten() {
            out.push(Friend {
                steamid: p["steamid"].as_str().unwrap_or("").into(),
                name: p["personaname"].as_str().unwrap_or("").into(),
                avatar: p["avatarmedium"].as_str().unwrap_or("").into(),
                state: p["personastate"].as_u64().unwrap_or(0) as u8,
                game: p["gameextrainfo"].as_str().map(String::from),
                game_id: p["gameid"].as_str().and_then(|g| g.parse().ok()),
            });
        }
    }
    // In a game first, then online, then by name.
    out.sort_by_key(|f| (f.game.is_none(), f.state == 0, f.name.to_lowercase()));
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WishItem {
    pub app: u32,
    pub name: String,
    pub discount: u32,
    /// "€19.99" as Steam formats it.
    pub price: Option<String>,
    pub original: Option<String>,
}

/// Your wishlist with prices (public wishlists; no key needed).
pub fn wishlist(steamid: &str, get: Get) -> Result<Vec<WishItem>, String> {
    let v: Value = serde_json::from_str(&get(&format!("https://api.steampowered.com/IWishlistService/GetWishlist/v1/?steamid={steamid}"))?).map_err(|e| e.to_string())?;
    let apps: Vec<u32> = v["response"]["items"].as_array().into_iter().flatten().filter_map(|i| i["appid"].as_u64().map(|a| a as u32)).collect();
    let mut out = Vec::new();
    for chunk in apps.chunks(50) {
        let ids: Vec<Value> = chunk.iter().map(|a| serde_json::json!({ "appid": a })).collect();
        let input = serde_json::json!({ "ids": ids, "context": { "language": "english" }, "data_request": { "include_basic_info": false } });
        let url = format!("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/?input_json={}", enc(&input.to_string()));
        let Ok(text) = get(&url) else { continue };
        let Ok(v) = serde_json::from_str::<Value>(&text) else { continue };
        for item in v["response"]["store_items"].as_array().into_iter().flatten() {
            let Some(app) = item["appid"].as_u64() else { continue };
            let buy = &item["best_purchase_option"];
            out.push(WishItem {
                app: app as u32,
                name: item["name"].as_str().unwrap_or("").into(),
                discount: buy["discount_pct"].as_u64().unwrap_or(0) as u32,
                price: buy["formatted_final_price"].as_str().map(String::from),
                original: buy["formatted_original_price"].as_str().map(String::from),
            });
        }
    }
    out.sort_by_key(|w| (std::cmp::Reverse(w.discount), w.name.to_lowercase()));
    Ok(out)
}

/// SteamGridDB artwork for a game name (needs your SteamGridDB key): cover, hero, logo.
pub fn steamgriddb(key: &str, name: &str) -> Result<Art, String> {
    let get = |url: &str| -> Result<Value, String> { serde_json::from_str(&http_get_with(url, Some(key))?).map_err(|e| e.to_string()) };
    let found = get(&format!("https://www.steamgriddb.com/api/v2/search/autocomplete/{}", enc(name)))?;
    let id = found["data"][0]["id"].as_u64().ok_or("SteamGridDB doesn't know this game")?;
    let first = |kind: &str, q: &str| get(&format!("https://www.steamgriddb.com/api/v2/{kind}/game/{id}{q}")).ok().and_then(|v| v["data"][0]["url"].as_str().map(String::from));
    Ok(Art { cover: first("grids", "?dimensions=600x900"), hero: first("heroes", ""), logo: first("logos", ""), header: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vdf_nested_and_users() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("config");
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::write(
            cfg.join("loginusers.vdf"),
            "\"users\"\n{\n\t\"76561198000000001\"\n\t{\n\t\t\"AccountName\"\t\t\"old\"\n\t\t\"PersonaName\"\t\t\"Old\"\n\t\t\"MostRecent\"\t\t\"0\"\n\t\t\"Timestamp\"\t\t\"100\"\n\t}\n\t\"76561198000000002\" // the one\n\t{\n\t\t\"PersonaName\"\t\t\"Ash \\\"K\\\"\"\n\t\t\"MostRecent\"\t\t\"1\"\n\t\t\"Timestamp\"\t\t\"50\"\n\t}\n}\n",
        )
        .unwrap();
        let u = current_user(dir.path()).unwrap();
        assert_eq!((u.steamid.as_str(), u.account_id, u.name.as_str()), ("76561198000000002", 39_734_274, "Ash \"K\""));
        let conf = dir.path().join("userdata").join(u.account_id.to_string()).join("config");
        std::fs::create_dir_all(&conf).unwrap();
        std::fs::write(conf.join("localconfig.vdf"), r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" { "730" { "LastPlayed" "1700000000" "Playtime" "1234" } "570" { "Playtime" "5" } } } } } }"#).unwrap();
        let p = playtimes(dir.path(), &u);
        assert_eq!(p[&730], (1234, 1_700_000_000));
        assert_eq!(p[&570], (5, 0));
    }

    #[test]
    fn soundtracks_and_screenshots() {
        let dir = tempfile::tempdir().unwrap();
        let music = dir.path().join("steamapps").join("music").join("Hollow Knight Soundtrack");
        std::fs::create_dir_all(&music).unwrap();
        for n in ["02 Dirtmouth.mp3", "01 Enter Hallownest.mp3", "cover.jpg"] {
            std::fs::write(music.join(n), b"x").unwrap();
        }
        let t = soundtrack(&[dir.path().to_path_buf()], "Hollow Knight");
        assert_eq!(t.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect::<Vec<_>>(), vec!["01 Enter Hallownest.mp3", "02 Dirtmouth.mp3"]);
        assert!(soundtrack(&[dir.path().to_path_buf()], "Celeste").is_empty());
        let u = SteamUser { steamid: "1".into(), account_id: 7, name: String::new() };
        let shots = dir.path().join("userdata/7/760/remote/367520/screenshots");
        std::fs::create_dir_all(&shots).unwrap();
        std::fs::write(shots.join("a.jpg"), b"x").unwrap();
        std::fs::write(shots.join("notes.txt"), b"x").unwrap();
        assert_eq!(screenshots(dir.path(), &u, 367520).len(), 1);
    }

    #[test]
    fn servers_parse() {
        let get = |url: &str| -> Result<String, String> {
            Ok(if url.contains("GetNumberOfCurrentPlayers") {
                r#"{"response":{"player_count":812345,"result":1}}"#.into()
            } else if url.contains("GetSchemaForGame") {
                r#"{"game":{"availableGameStats":{"achievements":[{"name":"A1","displayName":"First","description":"d","icon":"i","icongray":"g","hidden":0},{"name":"A2","displayName":"Second","hidden":1}]}}}"#.into()
            } else if url.contains("GetPlayerAchievements") {
                r#"{"playerstats":{"achievements":[{"apiname":"A1","achieved":1,"unlocktime":99},{"apiname":"A2","achieved":0}]}}"#.into()
            } else if url.contains("GetGlobalAchievementPercentagesForApp") {
                r#"{"achievementpercentages":{"achievements":[{"name":"A1","percent":"52.5"},{"name":"A2","percent":1.25}]}}"#.into()
            } else if url.contains("GetWishlist") {
                r#"{"response":{"items":[{"appid":10},{"appid":20}]}}"#.into()
            } else if url.contains("IStoreBrowseService") {
                r#"{"response":{"store_items":[{"appid":10,"name":"Cheap","best_purchase_option":{"discount_pct":75,"formatted_final_price":"$4.99","formatted_original_price":"$19.99"}},{"appid":20,"name":"Full","best_purchase_option":{"formatted_final_price":"$9.99"}}]}}"#.into()
            } else if url.contains("GetFriendList") {
                r#"{"friendslist":{"friends":[{"steamid":"1"},{"steamid":"2"}]}}"#.into()
            } else if url.contains("GetPlayerSummaries") {
                r#"{"response":{"players":[{"steamid":"1","personaname":"Zed","personastate":0},{"steamid":"2","personaname":"Amy","personastate":1,"gameextrainfo":"Celeste","gameid":"504230"}]}}"#.into()
            } else {
                return Err("?".into());
            })
        };
        assert_eq!(player_count(730, &get).unwrap(), 812_345);
        let a = achievements("k", "s", 1, &get).unwrap();
        assert_eq!((a[0].unlocked, a[0].unlocked_at, a[0].percent), (true, 99, 52.5));
        assert!(!a[1].unlocked && a[1].hidden && a[1].percent == 1.25);
        let w = wishlist("s", &get).unwrap();
        assert_eq!((w[0].name.as_str(), w[0].discount, w[0].price.as_deref()), ("Cheap", 75, Some("$4.99")));
        let f = friends("k", "s", &get).unwrap();
        assert_eq!((f[0].name.as_str(), f[0].game.as_deref(), f[0].game_id), ("Amy", Some("Celeste"), Some(504230)));
        assert_eq!(art_urls(730).cover.unwrap(), "https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/730/library_600x900_2x.jpg");
        assert!(http_get("https://example.com/").is_err());
    }
}
