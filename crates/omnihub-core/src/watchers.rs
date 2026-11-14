//! Things OmniHub keeps an eye on for you: the Fortnite item shop (an alert
//! when a skin, emote or other item you want is in it) and prices of PC
//! parts or anything else (an alert when one drops). Prices come from the
//! product page's own price data (the schema.org details most shops put in
//! their pages for search engines); some shops, like Amazon, don't let
//! programs read their pages, and say so.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Today in UTC: the item shop changes at midnight UTC.
pub fn utc_today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

pub const SHOP_URL: &str = "https://fortnite-api.com/v2/shop";

// ---------- the Fortnite item shop ----------

/// Every item name in the shop answer (skins, emotes, bundles, tracks…).
pub fn shop_names(v: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    for e in v["data"]["entries"].as_array().into_iter().flatten() {
        if let Some(n) = e["bundle"]["name"].as_str() {
            out.push(n.to_string());
        }
        for list in ["brItems", "cars", "instruments", "legoKits", "tracks"] {
            for it in e[list].as_array().into_iter().flatten() {
                if let Some(n) = it["name"].as_str().or(it["title"].as_str()) {
                    out.push(n.to_string());
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The wanted items that are in the shop.
pub fn wanted_in(names: &[String], wanted: &[String]) -> Vec<String> {
    wanted.iter().filter(|w| names.iter().any(|n| n.eq_ignore_ascii_case(w.trim()))).cloned().collect()
}

// ---------- prices ----------

/// The price in a product page: schema.org JSON-LD offers, then itemprop and
/// Open Graph price tags. Returns (price, currency).
pub fn page_price(html: &str) -> Option<(f64, Option<String>)> {
    let num = |s: &str| -> Option<f64> {
        let s: String = s.chars().filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
        // "1.299,00" or "1,299.00" or "1299"
        let s = if s.contains(',') && s.contains('.') {
            if s.rfind(',') > s.rfind('.') { s.replace('.', "").replace(',', ".") } else { s.replace(',', "") }
        } else if s.contains(',') && s.rsplit(',').next().is_some_and(|d| d.len() == 2) {
            s.replace(',', ".")
        } else {
            s.replace(',', "")
        };
        s.parse::<f64>().ok().filter(|p| *p > 0.0)
    };
    // JSON-LD blocks.
    let mut rest = html;
    while let Some(i) = rest.find("application/ld+json") {
        rest = &rest[i..];
        let Some(start) = rest.find('>') else { break };
        let Some(end) = rest[start..].find("</script>") else { break };
        let block = &rest[start + 1..start + end];
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(block.trim()) {
            if let Some(found) = find_offer(&v) {
                return Some(found);
            }
        }
        rest = &rest[start + end..];
    }
    // <meta itemprop="price" content="…"> / property="product:price:amount" / og:price:amount
    for key in ["itemprop=\"price\"", "property=\"product:price:amount\"", "property=\"og:price:amount\""] {
        if let Some(i) = html.find(key) {
            let tag_start = html[..i].rfind('<').unwrap_or(i);
            let tag_end = html[i..].find('>').map(|e| i + e).unwrap_or(html.len());
            let tag = &html[tag_start..tag_end];
            if let Some(c) = tag.find("content=\"") {
                let v = &tag[c + 9..];
                if let Some(p) = v.split('"').next().and_then(num) {
                    let cur = ["itemprop=\"priceCurrency\"", "property=\"product:price:currency\"", "property=\"og:price:currency\""].iter().find_map(|k| {
                        let j = html.find(k)?;
                        let t0 = html[..j].rfind('<')?;
                        let t1 = html[j..].find('>')? + j;
                        let t = &html[t0..t1];
                        let c = t.find("content=\"")?;
                        t[c + 9..].split('"').next().map(String::from)
                    });
                    return Some((p, cur));
                }
            }
        }
    }
    None
}

fn find_offer(v: &serde_json::Value) -> Option<(f64, Option<String>)> {
    match v {
        serde_json::Value::Array(a) => a.iter().find_map(find_offer),
        serde_json::Value::Object(o) => {
            if let Some(offers) = o.get("offers") {
                let first = if offers.is_array() { offers.get(0).cloned().unwrap_or_default() } else { offers.clone() };
                let price = first.get("price").or(first.get("lowPrice")).and_then(|p| p.as_f64().or_else(|| p.as_str().and_then(|s| s.replace(',', ".").parse().ok())));
                if let Some(p) = price.filter(|p| *p > 0.0) {
                    return Some((p, first.get("priceCurrency").and_then(|c| c.as_str()).map(String::from)));
                }
            }
            o.get("@graph").and_then(find_offer)
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct PriceWatch {
    pub id: String,
    pub name: String,
    pub url: String,
    /// Alert below this price (else on any drop of 3% or more).
    pub target: Option<f64>,
    pub price: Option<f64>,
    pub currency: Option<String>,
    pub lowest: Option<f64>,
    /// (unix seconds, price)
    pub history: Vec<(i64, f64)>,
    pub checked: Option<i64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WatchStore {
    /// Creators to hear about when they go live.
    pub streamers: Vec<Streamer>,
    /// An evening summary on the phone at this time ("21:00"; empty: off).
    pub recap_time: String,
    /// The day it was last sent (local, "2026-11-07").
    pub recap_date: String,
    /// Tell me about games free to keep (Epic, and Steam via GamerPower).
    pub free_alerts: bool,
    pub free_seen: Vec<String>,
    /// Tell me when my Steam games get patch notes.
    pub patch_alerts: bool,
    /// App id → the newest patch note already seen.
    pub patch_seen: std::collections::HashMap<String, String>,
    pub shop_wanted: Vec<String>,
    pub shop_alerts: bool,
    pub shop_seen: Vec<String>,
    pub shop_date: String,
    pub shop_now: Vec<String>,
    pub prices: Vec<PriceWatch>,
    /// Tell me when Fortnite updates or its servers go down.
    pub fortnite_alerts: bool,
    /// The Fortnite version installed when last looked ("32.10").
    pub fortnite_version: String,
    /// Epic's status page said Fortnite was down when last looked.
    pub fortnite_down: bool,
    /// A summary of the week every Sunday evening.
    pub weekly_on: bool,
    /// The week it was last sent ("2026-W45").
    pub weekly_week: String,
}

/// Epic's public status page (Statuspage's summary).
pub const EPIC_STATUS: &str = "https://status.epicgames.com/api/v2/summary.json";

/// The version in an Epic launcher manifest (`.item`), if it is Fortnite's:
/// "++Fortnite+Release-32.10-CL-45678901-Windows" → "32.10".
pub fn fortnite_version(item: &serde_json::Value) -> Option<String> {
    if item.get("AppName")?.as_str()? != "Fortnite" {
        return None;
    }
    let v = item.get("AppVersionString")?.as_str()?;
    let rest = v.split("Release-").nth(1)?;
    let ver: String = rest.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    (!ver.is_empty()).then_some(ver)
}

/// Fortnite's state on Epic's status page, when it isn't running normally
/// ("Under maintenance", "Partial outage").
pub fn fortnite_trouble(summary: &serde_json::Value) -> Option<String> {
    let comps = summary.get("components")?.as_array()?;
    let fortnite = comps.iter().find(|c| c.get("name").and_then(|n| n.as_str()) == Some("Fortnite"))?;
    let status = fortnite.get("status")?.as_str()?;
    match status {
        "operational" => None,
        s => Some(match s {
            "under_maintenance" => "Under maintenance".to_string(),
            "degraded_performance" => "Running slowly".to_string(),
            "partial_outage" => "Partly down".to_string(),
            "major_outage" => "Down".to_string(),
            other => other.replace('_', " "),
        }),
    }
}

/// Fortnite's installed version, from the Epic launcher's manifests.
pub fn installed_fortnite_version() -> Option<String> {
    let dir = std::path::Path::new(&std::env::var_os("ProgramData")?).join("Epic").join("EpicGamesLauncher").join("Data").join("Manifests");
    std::fs::read_dir(dir).ok()?.flatten().filter(|e| e.path().extension().is_some_and(|x| x == "item")).find_map(|e| fortnite_version(&serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok()?))
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FortniteNews {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PriceDrop {
    pub name: String,
    pub url: String,
    pub price: f64,
    pub before: Option<f64>,
    pub currency: Option<String>,
}

pub struct Watchers {
    file: PathBuf,
    store: Mutex<WatchStore>,
}

impl Watchers {
    pub fn new(data: &Path) -> Watchers {
        let file = data.join("watchers.json");
        let store = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or(WatchStore { shop_alerts: true, ..Default::default() });
        Watchers { file, store: Mutex::new(store) }
    }

    pub fn get(&self) -> WatchStore {
        self.store.lock().clone()
    }

    fn put(&self, s: WatchStore) -> WatchStore {
        if let Ok(j) = serde_json::to_vec_pretty(&s) {
            let _ = crate::settings::write_atomic(&self.file, &j);
        }
        *self.store.lock() = s.clone();
        s
    }

    pub fn set_shop(&self, wanted: Vec<String>, alerts: bool) -> WatchStore {
        let mut s = self.get();
        s.shop_wanted = wanted.into_iter().map(|w| w.trim().to_string()).filter(|w| !w.is_empty()).take(100).collect();
        s.shop_alerts = alerts;
        self.put(s)
    }

    /// Read today's shop; returns the wanted items newly seen in it.
    pub fn check_shop(&self, today: &str) -> Result<Vec<String>, String> {
        let names = shop_names(&crate::web::get_json(SHOP_URL)?);
        let mut s = self.get();
        if s.shop_date != today {
            s.shop_date = today.to_string();
            s.shop_seen.clear();
        }
        let hits: Vec<String> = wanted_in(&names, &s.shop_wanted).into_iter().filter(|h| !s.shop_seen.contains(h)).collect();
        s.shop_seen.extend(hits.iter().cloned());
        s.shop_now = names;
        self.put(s);
        Ok(hits)
    }

    pub fn save_price(&self, mut w: PriceWatch) -> Result<PriceWatch, String> {
        if !(w.url.starts_with("https://") || w.url.starts_with("http://")) {
            return Err("Paste the product's web address".into());
        }
        if w.name.trim().is_empty() {
            w.name = crate::web::host(&w.url).trim_start_matches("www.").to_string();
        }
        let mut s = self.get();
        if w.id.is_empty() {
            w.id = uuid::Uuid::new_v4().simple().to_string()[..10].to_string();
            if s.prices.len() >= 50 {
                return Err("Up to 50 prices can be watched".into());
            }
            s.prices.push(w.clone());
        } else if let Some(x) = s.prices.iter_mut().find(|x| x.id == w.id) {
            x.name = w.name.clone();
            x.url = w.url.clone();
            x.target = w.target;
            w = x.clone();
        }
        self.put(s);
        Ok(w)
    }

    pub fn delete_price(&self, id: &str) {
        let mut s = self.get();
        s.prices.retain(|p| p.id != id);
        self.put(s);
    }

    /// Record a reading; returns a drop to tell about.
    pub fn record(&self, id: &str, read: Result<(f64, Option<String>), String>, now: i64) -> Option<PriceDrop> {
        let mut s = self.get();
        let w = s.prices.iter_mut().find(|p| p.id == id)?;
        w.checked = Some(now);
        let drop = match read {
            Ok((price, cur)) => {
                w.error = None;
                let before = w.price;
                w.price = Some(price);
                w.currency = cur.or(w.currency.take());
                w.lowest = Some(w.lowest.map_or(price, |l| l.min(price)));
                if w.history.last().is_none_or(|h| (h.1 - price).abs() > 0.001) {
                    w.history.push((now, price));
                    if w.history.len() > 200 {
                        w.history.remove(0);
                    }
                }
                let hit = match (w.target, before) {
                    (Some(t), b) => price <= t && b.is_none_or(|b| b > t),
                    (None, Some(b)) => price <= b * 0.97,
                    (None, None) => false,
                };
                hit.then(|| PriceDrop { name: w.name.clone(), url: w.url.clone(), price, before, currency: w.currency.clone() })
            }
            Err(e) => {
                w.error = Some(e);
                None
            }
        };
        self.put(s);
        drop
    }
}

/// Read a product page's price.
pub fn fetch_price(url: &str) -> Result<(f64, Option<String>), String> {
    let html = crate::web::get_text_as(url, crate::web::BROWSER_UA, 25)?;
    page_price(&html).ok_or_else(|| "No price found on that page (some shops, like Amazon, don't let programs read their prices)".to_string())
}

// ---------- creators going live (Twitch, YouTube) ----------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Streamer {
    pub id: String,
    /// "twitch" or "youtube"
    pub platform: String,
    /// Twitch login ("shroud") or YouTube handle ("@MrBeast").
    pub handle: String,
    /// Live right now (as last checked).
    pub live: bool,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WentLive {
    pub handle: String,
    pub platform: String,
    pub title: String,
    pub url: String,
}

/// A creator's page, where it's checked.
pub fn live_url(platform: &str, handle: &str) -> String {
    match platform {
        "youtube" => format!("https://www.youtube.com/@{}/live", handle.trim_start_matches('@')),
        _ => format!("https://www.twitch.tv/{}", handle.to_lowercase()),
    }
}

/// Whether the page says it's live now, and the stream's title.
pub fn page_live(platform: &str, html: &str) -> (bool, String) {
    let title = || {
        let t = html.split("<meta property=\"og:title\" content=\"").nth(1).or_else(|| html.split("<meta name=\"title\" content=\"").nth(1)).and_then(|r| r.split('"').next()).unwrap_or("");
        t.replace("&amp;", "&").replace("&#39;", "'").replace("&quot;", "\"").chars().take(140).collect::<String>()
    };
    let live = match platform {
        // Twitch's page carries its own structured data while live.
        "twitch" => html.contains("\"isLiveBroadcast\":true"),
        // YouTube's /live page is the stream itself only while it's on.
        _ => html.contains("\"isLiveNow\":true") || (html.contains("\"isLive\":true") && html.contains("watch?v=")),
    };
    (live, if live { title() } else { String::new() })
}

pub fn valid_handle(platform: &str, h: &str) -> bool {
    let h = h.trim_start_matches('@');
    let ok = |c: char| c.is_ascii_alphanumeric() || c == '_' || (platform == "youtube" && (c == '-' || c == '.'));
    (platform == "twitch" || platform == "youtube") && (2..=40).contains(&h.len()) && h.chars().all(ok)
}

impl Watchers {
    pub fn save_streamer(&self, mut st: Streamer) -> Result<WatchStore, String> {
        st.handle = st.handle.trim().trim_start_matches("https://").trim_start_matches("www.").trim_start_matches("twitch.tv/").trim_start_matches("youtube.com/").trim_end_matches('/').to_string();
        if !valid_handle(&st.platform, &st.handle) {
            return Err(if st.platform == "youtube" { "Use the channel's handle, like @MrBeast".into() } else { "Use the Twitch name, like shroud".into() });
        }
        let mut s = self.get();
        if s.streamers.len() >= 50 {
            return Err("That's a lot of creators; keep it under 50".into());
        }
        if st.id.is_empty() {
            st.id = uuid::Uuid::new_v4().simple().to_string()[..8].to_string();
        }
        st.live = false;
        st.title.clear();
        s.streamers.retain(|x| x.id != st.id);
        s.streamers.push(st);
        Ok(self.put(s))
    }

    pub fn delete_streamer(&self, id: &str) -> WatchStore {
        let mut s = self.get();
        s.streamers.retain(|x| x.id != id);
        self.put(s)
    }

    /// Note who's live now; returns those who just went live.
    pub fn record_live(&self, seen: &[(String, bool, String)]) -> Vec<WentLive> {
        let mut s = self.get();
        let mut out = Vec::new();
        for st in s.streamers.iter_mut() {
            let Some((_, live, title)) = seen.iter().find(|x| x.0 == st.id) else { continue };
            if *live && !st.live {
                out.push(WentLive { handle: st.handle.clone(), platform: st.platform.clone(), title: title.clone(), url: live_url(&st.platform, &st.handle) });
            }
            st.live = *live;
            st.title = title.clone();
        }
        self.put(s);
        out
    }

    /// Look at every creator's page (a few seconds each).
    pub fn check_streamers(&self) -> Vec<WentLive> {
        let seen: Vec<(String, bool, String)> = self
            .get()
            .streamers
            .iter()
            .filter_map(|st| {
                let html = crate::web::get_text_as(&live_url(&st.platform, &st.handle), crate::web::BROWSER_UA, 20).ok()?;
                let (live, title) = page_live(&st.platform, &html);
                Some((st.id.clone(), live, title))
            })
            .collect();
        self.record_live(&seen)
    }
}

// ---------- the daily recap ----------

/// The day's summary as (title, body) from today's play sessions (name,
/// minutes), the data used and the health score.
pub fn recap_text(games: &[(String, u64)], data_bytes: Option<u64>, health: Option<u8>) -> (String, String) {
    let total: u64 = games.iter().map(|g| g.1).sum();
    let mut lines = Vec::new();
    if total > 0 {
        let top: Vec<String> = games.iter().take(3).map(|(n, m)| format!("{n} {}", mins(*m))).collect();
        lines.push(format!("Played {}: {}.", mins(total), top.join(", ")));
    } else {
        lines.push("No games today.".into());
    }
    if let Some(b) = data_bytes.filter(|b| *b > 0) {
        lines.push(format!("{:.1} GB of internet used.", b as f64 / 1e9));
    }
    if let Some(h) = health {
        lines.push(format!("PC health {h}/100."));
    }
    ("Your day on the PC".into(), lines.join(" "))
}

/// The week's summary: (title, body).
pub fn weekly_text(games: &[(String, u64)], fps: &[(String, f32)], clips: usize, shots: usize, data_bytes: Option<u64>, health: Option<u8>) -> (String, String) {
    let total: u64 = games.iter().map(|g| g.1).sum();
    let mut lines = Vec::new();
    if total > 0 {
        let top: Vec<String> = games.iter().take(3).map(|(n, m)| format!("{n} {}", mins(*m))).collect();
        lines.push(format!("Played {} this week: {}.", mins(total), top.join(", ")));
    } else {
        lines.push("No games this week.".into());
    }
    if !fps.is_empty() {
        let f: Vec<String> = fps.iter().take(3).map(|(n, a)| format!("{n} {a:.0} FPS")).collect();
        lines.push(format!("Average frames: {}.", f.join(", ")));
    }
    if clips + shots > 0 {
        lines.push(format!("{clips} clip{} and {shots} screenshot{} saved.", if clips == 1 { "" } else { "s" }, if shots == 1 { "" } else { "s" }));
    }
    if let Some(b) = data_bytes.filter(|b| *b > 0) {
        lines.push(format!("{:.1} GB of internet used.", b as f64 / 1e9));
    }
    if let Some(h) = health {
        lines.push(format!("PC health {h}/100."));
    }
    ("Your week on the PC".into(), lines.join(" "))
}

fn mins(m: u64) -> String {
    if m >= 60 {
        format!("{} h {} min", m / 60, m % 60)
    } else {
        format!("{m} min")
    }
}

impl Watchers {
    pub fn set_recap(&self, time: &str) -> Result<WatchStore, String> {
        let t = time.trim();
        if !t.is_empty() && crate::nightlight::minutes(t).is_none() {
            return Err("Use a time like 21:00".into());
        }
        let mut s = self.get();
        s.recap_time = t.to_string();
        Ok(self.put(s))
    }

    pub fn set_weekly(&self, on: bool) -> WatchStore {
        let mut s = self.get();
        s.weekly_on = on;
        self.put(s)
    }

    /// Whether the weekly report is due: Sunday from 19:00, once a week.
    pub fn weekly_due(&self, week: &str, sunday: bool, minute: u32) -> bool {
        let s = self.get();
        if !s.weekly_on || !sunday || minute < 19 * 60 || s.weekly_week == week {
            return false;
        }
        let mut s2 = s;
        s2.weekly_week = week.to_string();
        self.put(s2);
        true
    }

    /// Whether the recap is due now (once a day, at or after its time).
    pub fn recap_due(&self, today: &str, minute: u32) -> bool {
        let s = self.get();
        let Some(at) = crate::nightlight::minutes(&s.recap_time) else { return false };
        if minute < at || s.recap_date == today {
            return false;
        }
        let mut s2 = s;
        s2.recap_date = today.to_string();
        self.put(s2);
        true
    }
}

impl crate::core::AppCore {
    /// The last seven days: games, frame rates, clips and screenshots,
    /// internet used, and the PC's health now.
    pub fn weekly_report(&self) -> (String, String) {
        let now = crate::db::now();
        let start_ms = (now - 7 * 86_400) * 1000;
        let mut by: Vec<(String, u64)> = Vec::new();
        for s in self.shelf.sessions(None, 2000).into_iter().filter(|s| s.end >= start_ms) {
            let m = ((s.end - s.start.max(start_ms)).max(0) / 60_000) as u64;
            match by.iter_mut().find(|g| g.0 == s.name) {
                Some(g) => g.1 += m,
                None => by.push((s.name.clone(), m)),
            }
        }
        by.retain(|g| g.1 > 0);
        by.sort_by_key(|g| std::cmp::Reverse(g.1));
        let mut fps: Vec<(String, f32, u32)> = Vec::new();
        for r in self.games.fps_history(None).into_iter().filter(|r| r.at >= now - 7 * 86_400) {
            match fps.iter_mut().find(|x| x.0 == r.name) {
                Some(x) => {
                    x.1 += r.summary.avg;
                    x.2 += 1;
                }
                None => fps.push((r.name.clone(), r.summary.avg, 1)),
            }
        }
        let fps: Vec<(String, f32)> = fps.into_iter().map(|(n, s, c)| (n, s / c as f32)).collect();
        let clips = self.play.clips(&self.settings.get().play).into_iter().filter(|c| c.created >= now - 7 * 86_400).count();
        let filter = crate::capture::screenshots::ShotFilter { query: String::new(), tag: None, app: None, favorites: false, limit: Some(2000) };
        let shots = self.screenshots.list(&filter).unwrap_or_default().into_iter().filter(|x| x.created >= now - 7 * 86_400).count();
        let days = self.netmon.status(Vec::new()).days;
        let data: u64 = days.iter().rev().take(7).map(|d| d.down + d.up).sum();
        let health = crate::health::score(&crate::health::gather(), now * 1000).score;
        weekly_text(&by, &fps, clips, shots, Some(data), Some(health))
    }

    /// Today's summary: games played, internet used, PC health.
    pub fn daily_recap(&self) -> (String, String) {
        use chrono::TimeZone;
        let start = chrono::Local::now().date_naive().and_hms_opt(0, 0, 0).and_then(|d| chrono::Local.from_local_datetime(&d).single()).map_or(0, |d| d.timestamp_millis());
        let mut by: Vec<(String, u64)> = Vec::new();
        for s in self.shelf.sessions(None, 200).into_iter().filter(|s| s.end >= start) {
            let m = ((s.end - s.start.max(start)).max(0) / 60_000) as u64;
            match by.iter_mut().find(|g| g.0 == s.name) {
                Some(g) => g.1 += m,
                None => by.push((s.name.clone(), m)),
            }
        }
        by.retain(|g| g.1 > 0);
        by.sort_by_key(|g| std::cmp::Reverse(g.1));
        let data = self.netmon.status(Vec::new()).days.last().map(|d| d.down + d.up);
        let health = crate::health::score(&crate::health::gather(), crate::db::now() * 1000).score;
        recap_text(&by, data, Some(health))
    }
}

// ---------- free games and patch notes ----------

pub const EPIC_FREE: &str = "https://store-site-backend-static-ipv4.ak.epicgames.com/freeGamesPromotions?locale=en-US&country=US&allowCountries=US";
pub const GAMERPOWER: &str = "https://www.gamerpower.com/api/giveaways?platform=steam&type=game";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FreeGame {
    pub title: String,
    pub store: String,
    pub url: String,
    /// When the offer ends ("2026-11-13"), when known.
    pub until: String,
}

/// Epic's free-to-keep games right now (100% off, live today).
pub fn epic_free(v: &serde_json::Value, today: &str) -> Vec<FreeGame> {
    let els = v.pointer("/data/Catalog/searchStore/elements").and_then(|e| e.as_array()).cloned().unwrap_or_default();
    els.iter()
        .filter_map(|e| {
            let offer = e.pointer("/promotions/promotionalOffers/0/promotionalOffers/0")?;
            let free = offer.pointer("/discountSetting/discountPercentage").and_then(|d| d.as_i64()) == Some(0) || e.pointer("/price/totalPrice/discountPrice").and_then(|d| d.as_i64()) == Some(0);
            let start = offer.get("startDate")?.as_str()?.get(..10)?;
            let end = offer.get("endDate")?.as_str()?.get(..10)?.to_string();
            if !free || start > today || end.as_str() < today {
                return None;
            }
            let slug = e.pointer("/catalogNs/mappings/0/pageSlug").or_else(|| e.get("productSlug")).and_then(|s| s.as_str()).filter(|s| !s.is_empty())?;
            Some(FreeGame { title: e.get("title")?.as_str()?.to_string(), store: "Epic Games".into(), url: format!("https://store.epicgames.com/p/{}", slug.trim_end_matches("/home")), until: end })
        })
        .collect()
}

/// Steam games free to keep, from GamerPower's list.
pub fn gamerpower_free(v: &serde_json::Value) -> Vec<FreeGame> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|g| {
                    let url = g.get("open_giveaway_url").or_else(|| g.get("gamerpower_url"))?.as_str()?.to_string();
                    url.starts_with("https://").then(|| FreeGame {
                        title: g.get("title").and_then(|t| t.as_str()).unwrap_or("A game").trim_end_matches(" (Steam) Giveaway").trim_end_matches(" Giveaway").to_string(),
                        store: "Steam".into(),
                        url,
                        until: g.get("end_date").and_then(|d| d.as_str()).filter(|d| *d != "N/A").and_then(|d| d.get(..10)).unwrap_or("").to_string(),
                    })
                })
                .take(10)
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PatchNote {
    pub game: String,
    pub title: String,
    pub url: String,
    pub gid: String,
}

/// The newest patch note in Steam's news for a game.
pub fn steam_patch(v: &serde_json::Value, game: &str) -> Option<PatchNote> {
    let items = v.pointer("/appnews/newsitems")?.as_array()?;
    items.iter().find_map(|n| {
        let title = n.get("title")?.as_str()?;
        let tags: Vec<&str> = n.get("tags").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|x| x.as_str()).collect()).unwrap_or_default();
        let t = title.to_lowercase();
        let patch = tags.contains(&"patchnotes") || ["patch", "update", "hotfix", "release notes"].iter().any(|w| t.contains(w));
        patch.then(|| PatchNote { game: game.to_string(), title: title.to_string(), url: n.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string(), gid: n.get("gid").and_then(|g| g.as_str()).unwrap_or("").to_string() })
    })
}

impl Watchers {
    pub fn set_game_alerts(&self, free: bool, patches: bool, fortnite: Option<bool>) -> WatchStore {
        let mut s = self.get();
        s.free_alerts = free;
        s.patch_alerts = patches;
        if let Some(f) = fortnite {
            s.fortnite_alerts = f;
        }
        self.put(s)
    }

    /// Fortnite updated (from the installed version) or its servers changed
    /// state (from Epic's status page). The first look only remembers.
    pub fn check_fortnite(&self, version: Option<String>, trouble: Option<Option<String>>) -> Vec<FortniteNews> {
        let mut s = self.get();
        let mut out = Vec::new();
        if let Some(v) = version {
            if !s.fortnite_version.is_empty() && s.fortnite_version != v {
                out.push(FortniteNews { title: format!("Fortnite updated to {v}"), body: "The first match after an update can stutter while shaders are rebuilt.".into() });
            }
            s.fortnite_version = v;
        }
        // None: the status page couldn't be read, so nothing changes.
        if let Some(t) = trouble {
            match (&t, s.fortnite_down) {
                (Some(what), false) => out.push(FortniteNews { title: format!("Fortnite: {what}"), body: "From Epic's status page.".into() }),
                (None, true) => out.push(FortniteNews { title: "Fortnite is back up".into(), body: "Epic's status page says it's running normally.".into() }),
                _ => {}
            }
            s.fortnite_down = t.is_some();
        }
        self.put(s);
        out
    }

    /// Free games not told about before (marks them as told).
    pub fn new_free(&self, found: Vec<FreeGame>) -> Vec<FreeGame> {
        let mut s = self.get();
        let fresh: Vec<FreeGame> = found.into_iter().filter(|g| !s.free_seen.contains(&g.url)).collect();
        s.free_seen.extend(fresh.iter().map(|g| g.url.clone()));
        let n = s.free_seen.len();
        if n > 300 {
            s.free_seen.drain(..n - 300);
        }
        self.put(s);
        fresh
    }

    pub fn check_free(&self) -> Vec<FreeGame> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let mut all = Vec::new();
        if let Ok(v) = crate::web::get_json(EPIC_FREE) {
            all.extend(epic_free(&v, &today));
        }
        if let Ok(v) = crate::web::get_json(GAMERPOWER) {
            all.extend(gamerpower_free(&v));
        }
        all
    }

    /// New patch notes for these Steam games: (app id, name).
    pub fn check_patches(&self, games: &[(u32, String)]) -> Vec<PatchNote> {
        let mut s = self.get();
        let mut out = Vec::new();
        for (app, name) in games.iter().take(40) {
            let url = format!("https://api.steampowered.com/ISteamNews/GetNewsForApp/v2/?appid={app}&count=5&maxlength=200");
            let Ok(v) = crate::web::get_json(&url) else { continue };
            let Some(p) = steam_patch(&v, name) else { continue };
            let key = app.to_string();
            match s.patch_seen.get(&key) {
                Some(seen) if *seen == p.gid => {}
                Some(_) => {
                    s.patch_seen.insert(key, p.gid.clone());
                    out.push(p);
                }
                // The first look only remembers what's there.
                None => {
                    s.patch_seen.insert(key, p.gid.clone());
                }
            }
        }
        self.put(s);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shop_names_and_wanted() {
        let v = serde_json::json!({ "data": { "entries": [
            { "bundle": { "name": "Ninja Bundle" }, "brItems": [{ "name": "Ninja" }, { "name": "Dab" }] },
            { "brItems": [{ "name": "Renegade Raider" }] },
            { "tracks": [{ "title": "Lose Yourself" }] }
        ] } });
        let names = shop_names(&v);
        assert!(names.contains(&"Renegade Raider".to_string()) && names.contains(&"Lose Yourself".to_string()) && names.contains(&"Ninja Bundle".to_string()));
        assert_eq!(wanted_in(&names, &["renegade raider".into(), "Peely".into()]), vec!["renegade raider"]);
    }

    #[test]
    fn prices_from_pages() {
        let ld = r#"<script type="application/ld+json">{"@context":"https://schema.org","@type":"Product","name":"RTX 5070","offers":{"@type":"Offer","price":"549.99","priceCurrency":"USD"}}</script>"#;
        assert_eq!(page_price(ld), Some((549.99, Some("USD".into()))));
        let graph = r#"<script type="application/ld+json">{"@graph":[{"@type":"WebPage"},{"@type":"Product","offers":[{"price":1299,"priceCurrency":"EUR"}]}]}</script>"#;
        assert_eq!(page_price(graph), Some((1299.0, Some("EUR".into()))));
        let meta = r#"<meta property="product:price:amount" content="1.299,00"><meta property="product:price:currency" content="EUR">"#;
        assert_eq!(page_price(meta), Some((1299.0, Some("EUR".into()))));
        assert_eq!(page_price("<html>no price</html>"), None);
    }

    #[test]
    fn drops_are_told_once() {
        let t = tempfile::tempdir().unwrap();
        let w = Watchers::new(t.path());
        assert!(w.save_price(PriceWatch { url: "ftp://x".into(), ..Default::default() }).is_err());
        let a = w.save_price(PriceWatch { url: "https://shop.example/gpu".into(), target: Some(500.0), ..Default::default() }).unwrap();
        assert_eq!(a.name, "shop.example");
        assert!(w.record(&a.id, Ok((549.0, Some("USD".into()))), 1).is_none());
        let d = w.record(&a.id, Ok((499.0, None)), 2).unwrap();
        assert_eq!((d.price, d.before, d.currency.as_deref()), (499.0, Some(549.0), Some("USD")));
        assert!(w.record(&a.id, Ok((489.0, None)), 3).is_none(), "already below the target");
        let b = w.save_price(PriceWatch { url: "https://shop.example/cpu".into(), ..Default::default() }).unwrap();
        w.record(&b.id, Ok((300.0, None)), 1);
        assert!(w.record(&b.id, Ok((295.0, None)), 2).is_none(), "under 3%");
        assert!(w.record(&b.id, Ok((280.0, None)), 3).is_some());
        assert!(w.record(&b.id, Err("blocked".into()), 4).is_none());
        assert_eq!(Watchers::new(t.path()).get().prices.len(), 2);
    }

    #[test]
    fn notices_creators_going_live() {
        assert!(page_live("twitch", r#"<script type="application/ld+json">[{"@type":"VideoObject","publication":{"@type":"BroadcastEvent","isLiveBroadcast":true}}]</script><meta property="og:title" content="shroud - Twitch">"#).0);
        assert!(!page_live("twitch", "<html>offline</html>").0);
        let (live, title) = page_live("youtube", r#"<link rel="canonical" href="https://www.youtube.com/watch?v=abc"><meta name="title" content="Big &amp; live">"isLiveNow":true"#);
        assert_eq!((live, title.as_str()), (true, "Big & live"));
        assert!(valid_handle("youtube", "@MrBeast") && valid_handle("twitch", "shroud") && !valid_handle("twitch", "a b") && !valid_handle("kick", "x1"));
        let t = tempfile::tempdir().unwrap();
        let w = Watchers::new(t.path());
        let s = w.save_streamer(Streamer { platform: "twitch".into(), handle: "https://www.twitch.tv/Shroud/".into(), ..Default::default() }).unwrap();
        let id = s.streamers[0].id.clone();
        assert_eq!(s.streamers[0].handle, "Shroud");
        assert_eq!(w.record_live(&[(id.clone(), true, "Ranked".into())]).len(), 1, "went live");
        assert!(w.record_live(&[(id.clone(), true, "Ranked".into())]).is_empty(), "still live: no second alert");
        w.record_live(&[(id.clone(), false, String::new())]);
        assert_eq!(w.record_live(&[(id, true, String::new())])[0].url, "https://www.twitch.tv/shroud");
    }

    #[test]
    fn writes_the_day() {
        let (t, b) = recap_text(&[("Fortnite".into(), 95), ("Minecraft".into(), 20)], Some(3_400_000_000), Some(88));
        assert_eq!(t, "Your day on the PC");
        assert_eq!(b, "Played 1 h 55 min: Fortnite 1 h 35 min, Minecraft 20 min. 3.4 GB of internet used. PC health 88/100.");
        assert!(recap_text(&[], None, None).1.starts_with("No games today"));
        let d = tempfile::tempdir().unwrap();
        let w = Watchers::new(d.path());
        assert!(w.set_recap("9pm").is_err());
        w.set_recap("21:00").unwrap();
        assert!(!w.recap_due("2026-11-07", 20 * 60));
        assert!(w.recap_due("2026-11-07", 21 * 60 + 5));
        assert!(!w.recap_due("2026-11-07", 22 * 60), "once a day");
        assert!(w.recap_due("2026-11-08", 23 * 60));
    }

    #[test]
    fn weekly_summary() {
        let (t, b) = weekly_text(&[("Fortnite".into(), 410), ("Minecraft".into(), 30)], &[("Fortnite".into(), 212.4)], 3, 1, Some(42_000_000_000), Some(91));
        assert_eq!(t, "Your week on the PC");
        assert_eq!(b, "Played 7 h 20 min this week: Fortnite 6 h 50 min, Minecraft 30 min. Average frames: Fortnite 212 FPS. 3 clips and 1 screenshot saved. 42.0 GB of internet used. PC health 91/100.");
        let tmp = tempfile::tempdir().unwrap();
        let w = Watchers::new(tmp.path());
        assert!(!w.weekly_due("2026-W45", true, 20 * 60), "off by default");
        w.set_weekly(true);
        assert!(!w.weekly_due("2026-W45", false, 20 * 60));
        assert!(!w.weekly_due("2026-W45", true, 18 * 60));
        assert!(w.weekly_due("2026-W45", true, 20 * 60));
        assert!(!w.weekly_due("2026-W45", true, 21 * 60), "once a week");
    }

    #[test]
    fn watches_fortnite() {
        let item = serde_json::json!({"AppName":"Fortnite","AppVersionString":"++Fortnite+Release-32.10-CL-45678901-Windows"});
        assert_eq!(fortnite_version(&item).as_deref(), Some("32.10"));
        assert_eq!(fortnite_version(&serde_json::json!({"AppName":"Other","AppVersionString":"++X+Release-1.0"})), None);
        let down = serde_json::json!({"components":[{"name":"Fortnite","status":"under_maintenance"},{"name":"Fall Guys","status":"operational"}]});
        assert_eq!(fortnite_trouble(&down).as_deref(), Some("Under maintenance"));
        assert_eq!(fortnite_trouble(&serde_json::json!({"components":[{"name":"Fortnite","status":"operational"}]})), None);
        let t = tempfile::tempdir().unwrap();
        let w = Watchers::new(t.path());
        // The first look only remembers.
        assert!(w.check_fortnite(Some("32.00".into()), Some(None)).is_empty());
        let news = w.check_fortnite(Some("32.10".into()), Some(Some("Under maintenance".into())));
        assert_eq!(news.iter().map(|n| n.title.as_str()).collect::<Vec<_>>(), vec!["Fortnite updated to 32.10", "Fortnite: Under maintenance"]);
        assert!(w.check_fortnite(Some("32.10".into()), None).is_empty());
        assert_eq!(w.check_fortnite(None, Some(None))[0].title, "Fortnite is back up");
    }

    #[test]
    fn finds_free_games_and_patches() {
        let epic = serde_json::json!({"data":{"Catalog":{"searchStore":{"elements":[
            {"title":"Free One","productSlug":"free-one","price":{"totalPrice":{"discountPrice":0}},"promotions":{"promotionalOffers":[{"promotionalOffers":[{"startDate":"2026-11-05T16:00:00.000Z","endDate":"2026-11-12T16:00:00.000Z","discountSetting":{"discountPercentage":0}}]}]}},
            {"title":"Next Week","productSlug":"later","promotions":{"promotionalOffers":[],"upcomingPromotionalOffers":[{}]}},
            {"title":"Half Off","productSlug":"half","price":{"totalPrice":{"discountPrice":999}},"promotions":{"promotionalOffers":[{"promotionalOffers":[{"startDate":"2026-11-05T16:00:00.000Z","endDate":"2026-11-12T16:00:00.000Z","discountSetting":{"discountPercentage":50}}]}]}}
        ]}}}});
        let f = epic_free(&epic, "2026-11-07");
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].url.as_str(), f[0].until.as_str()), ("https://store.epicgames.com/p/free-one", "2026-11-12"));
        let gp = serde_json::json!([{"title":"Indie Thing (Steam) Giveaway","open_giveaway_url":"https://www.gamerpower.com/open/indie","end_date":"2026-11-10 23:59:00"}]);
        assert_eq!(gamerpower_free(&gp)[0].title, "Indie Thing");
        let news = serde_json::json!({"appnews":{"newsitems":[{"gid":"1","title":"Community spotlight","tags":[]},{"gid":"2","title":"Patch 1.4 notes","url":"https://store.steampowered.com/news/2","tags":["patchnotes"]}]}});
        assert_eq!(steam_patch(&news, "Game").unwrap().gid, "2");
        let t = tempfile::tempdir().unwrap();
        let w = Watchers::new(t.path());
        assert_eq!(w.new_free(f.clone()).len(), 1);
        assert!(w.new_free(f).is_empty(), "told once");
    }
}
