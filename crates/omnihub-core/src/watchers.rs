//! Things OmniHub keeps an eye on for you: the Fortnite item shop (an alert
//! when a skin, emote or other item you want is in it) and prices of PC
//! parts or anything else (an alert when one drops). Prices come from the
//! product page's own price data (the schema.org details most shops put in
//! their pages for search engines); some shops, like Amazon, don't let
//! programs read their pages, and say so.

use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

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
    pub shop_wanted: Vec<String>,
    pub shop_alerts: bool,
    pub shop_seen: Vec<String>,
    pub shop_date: String,
    pub shop_now: Vec<String>,
    pub prices: Vec<PriceWatch>,
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
}
