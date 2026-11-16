//! Game artwork from Steam's store, at the addresses Steam uses now. Steam
//! keeps newer art under a hashed folder ("…/apps/730/<hash>/library_capsule.jpg"),
//! so the old fixed addresses miss it for many games. Its store asks Steam
//! for the right addresses (no key needed) and keeps them for a week. Games
//! from other stores (Epic, Xbox, EA…) get art too when Steam sells the same
//! game under the same name.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use super::steam::{squash, Art};

const ASSET_BASE: &str = "https://shared.steamstatic.com/store_item_assets/";
const KEEP_SECS: i64 = 7 * 86_400;
/// Name searches per run (one a third of a second, so Steam isn't flooded).
const SEARCHES_PER_RUN: usize = 60;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Found {
    /// The Steam app it matched (for games from other stores, by name).
    pub app: Option<u32>,
    pub art: Option<Art>,
    pub at: i64,
}

pub struct StoreArt {
    file: PathBuf,
    map: Mutex<HashMap<String, Found>>,
    busy: AtomicBool,
}

/// The art in an `IStoreBrowseService/GetItems` answer, by app.
pub fn parse_items(json: &str) -> HashMap<u32, Art> {
    let v: serde_json::Value = serde_json::from_str(json).unwrap_or_default();
    let mut out = HashMap::new();
    for it in v["response"]["store_items"].as_array().into_iter().flatten() {
        let Some(app) = it["appid"].as_u64().or_else(|| it["id"].as_u64()) else { continue };
        let a = &it["assets"];
        let Some(fmt) = a["asset_url_format"].as_str() else { continue };
        let url = |keys: &[&str]| keys.iter().find_map(|k| a[*k].as_str()).filter(|f| !f.is_empty() && !f.contains("..")).map(|f| format!("{ASSET_BASE}{}", fmt.replace("${FILENAME}", f)));
        let art = Art {
            cover: url(&["library_capsule_2x", "library_capsule"]),
            hero: url(&["library_hero", "library_hero_2x"]),
            logo: url(&["library_logo", "library_logo_2x"]),
            header: url(&["header", "header_2x", "main_capsule"]),
        };
        if art != Art::default() {
            out.insert(app as u32, art);
        }
    }
    out
}

/// The Steam app with exactly this name, from a store search answer.
pub fn parse_search(json: &str, name: &str) -> Option<u32> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let want = squash(name);
    if want.len() < 2 {
        return None;
    }
    v["items"].as_array()?.iter().find(|i| i["type"].as_str().is_none_or(|t| t == "app") && i["name"].as_str().is_some_and(|n| squash(n) == want)).and_then(|i| i["id"].as_u64()).map(|id| id as u32)
}

fn enc(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}

fn items_url(apps: &[u32]) -> String {
    let ids: Vec<String> = apps.iter().map(|a| format!("{{\"appid\":{a}}}")).collect();
    let input = format!("{{\"ids\":[{}],\"context\":{{\"language\":\"english\",\"country_code\":\"US\"}},\"data_request\":{{\"include_assets\":true}}}}", ids.join(","));
    format!("https://api.steampowered.com/IStoreBrowseService/GetItems/v1/?input_json={}", enc(&input))
}

impl StoreArt {
    pub fn new(cache: &Path) -> StoreArt {
        let file = cache.join("store-art.json");
        let map = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        StoreArt { file, map: Mutex::new(map), busy: AtomicBool::new(false) }
    }

    pub fn get(&self, key: &str) -> Option<Found> {
        self.map.lock().get(key).cloned()
    }

    /// The games (key, Steam app if known, name) not looked up this week.
    pub fn missing<'a>(&self, games: impl Iterator<Item = (&'a str, Option<u32>, &'a str)>, now: i64) -> Vec<(String, Option<u32>, String)> {
        let m = self.map.lock();
        games.filter(|(k, _, _)| m.get(*k).is_none_or(|f| now - f.at > KEEP_SECS)).map(|(k, a, n)| (k.to_string(), a, n.to_string())).collect()
    }

    /// Ask Steam about `todo` (blocking). Returns whether anything new was found.
    pub fn fetch(&self, todo: Vec<(String, Option<u32>, String)>, now: i64, get: &dyn Fn(&str) -> Result<String, String>) -> bool {
        if todo.is_empty() || self.busy.swap(true, Ordering::SeqCst) {
            return false;
        }
        let mut apps: HashMap<String, Option<u32>> = HashMap::new();
        let mut searches = 0;
        let mut offline = false;
        for (key, app, name) in &todo {
            let app = match app {
                Some(a) => Some(*a),
                None if searches < SEARCHES_PER_RUN && !offline => {
                    searches += 1;
                    let url = format!("https://store.steampowered.com/api/storesearch/?term={}&cc=US&l=english", enc(name));
                    let r = get(&url);
                    offline = r.is_err();
                    std::thread::sleep(std::time::Duration::from_millis(330));
                    match r {
                        Ok(j) => parse_search(&j, name),
                        Err(_) => continue,
                    }
                }
                None => continue,
            };
            apps.insert(key.clone(), app);
        }
        let ids: Vec<u32> = {
            let mut v: Vec<u32> = apps.values().flatten().copied().collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        let mut arts: HashMap<u32, Art> = HashMap::new();
        let mut failed = false;
        for chunk in ids.chunks(50) {
            match get(&items_url(chunk)) {
                Ok(j) => arts.extend(parse_items(&j)),
                Err(_) => failed = true,
            }
        }
        let mut found_any = false;
        {
            let mut m = self.map.lock();
            for (key, app) in apps {
                // Not reached: try again next time rather than remembering "nothing".
                if failed && app.is_some_and(|a| !arts.contains_key(&a)) {
                    continue;
                }
                let art = app.and_then(|a| arts.get(&a).cloned());
                found_any |= art.is_some();
                m.insert(key, Found { app, art, at: now });
            }
            if let Ok(j) = serde_json::to_vec(&*m) {
                let _ = crate::settings::write_atomic(&self.file, &j);
            }
        }
        self.busy.store(false, Ordering::SeqCst);
        found_any
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEMS: &str = r#"{"response":{"store_items":[{"id":1172470,"appid":1172470,"assets":{"asset_url_format":"steam/apps/1172470/${FILENAME}?t=1790164440","header":"6cf0/header.jpg","library_capsule":"f86c/library_capsule.jpg","library_capsule_2x":"f86c/library_capsule_2x.jpg","library_hero":"8172/library_hero.jpg"}},{"id":5,"success":2}]}}"#;

    #[test]
    fn reads_steams_new_addresses() {
        let a = parse_items(ITEMS);
        let apex = &a[&1172470];
        assert_eq!(apex.cover.as_deref(), Some("https://shared.steamstatic.com/store_item_assets/steam/apps/1172470/f86c/library_capsule_2x.jpg?t=1790164440"));
        assert_eq!(apex.hero.as_deref(), Some("https://shared.steamstatic.com/store_item_assets/steam/apps/1172470/8172/library_hero.jpg?t=1790164440"));
        assert!(apex.logo.is_none());
        assert!(!a.contains_key(&5));
        assert!(items_url(&[1, 2]).contains("include%5Fassets"));
    }

    #[test]
    fn matches_other_stores_games_by_exact_name() {
        let j = r#"{"total":2,"items":[{"type":"app","name":"9 Kings Demo","id":3122250},{"type":"app","name":"9 Kings","id":2784470}]}"#;
        assert_eq!(parse_search(j, "9 Kings"), Some(2784470));
        assert_eq!(parse_search(j, "9 Kings™"), Some(2784470));
        assert_eq!(parse_search(j, "Kings"), None);
    }

    #[test]
    fn looks_up_once_a_week_and_retries_when_offline() {
        let t = tempfile::tempdir().unwrap();
        let s = StoreArt::new(t.path());
        let games = [("steam:1172470", Some(1172470), "Apex Legends"), ("epic:abc", None, "9 Kings")];
        let todo = s.missing(games.iter().copied(), 1000);
        assert_eq!(todo.len(), 2);
        let offline = |_: &str| -> Result<String, String> { Err("offline".into()) };
        assert!(!s.fetch(todo.clone(), 1000, &offline));
        assert_eq!(s.missing(games.iter().copied(), 1000).len(), 2, "nothing remembered while offline");
        let online = |u: &str| -> Result<String, String> {
            Ok(if u.contains("storesearch") { r#"{"items":[{"type":"app","name":"9 Kings","id":1172470}]}"#.into() } else { ITEMS.into() })
        };
        assert!(s.fetch(todo, 1000, &online));
        assert!(s.get("epic:abc").unwrap().art.unwrap().cover.is_some());
        assert!(s.missing(games.iter().copied(), 2000).is_empty());
        assert_eq!(StoreArt::new(t.path()).missing(games.iter().copied(), 1000 + KEEP_SECS + 1).len(), 2, "a week later");
    }
}
