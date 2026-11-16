//! Game deals across stores: CheapShark's public price list (Steam, GOG,
//! Humble, Fanatical, Green Man Gaming, Epic and more), searched by name.
//! No account or key; prices are CheapShark's, in US dollars. Links go
//! through CheapShark's redirect, which is how it lists deals.

use serde::Serialize;
use serde_json::Value;

const API: &str = "https://www.cheapshark.com/api/1.0";

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DealGame {
    pub id: String,
    pub title: String,
    pub thumb: String,
    /// The lowest price now, USD.
    pub cheapest: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Deal {
    pub store: String,
    pub price: f64,
    pub retail: f64,
    /// Percent off.
    pub savings: f64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameDeals {
    pub title: String,
    pub deals: Vec<Deal>,
    /// The lowest price it has ever had, USD, and when (unix seconds).
    pub lowest_ever: Option<(f64, i64)>,
}

fn f(v: &Value) -> f64 {
    v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())).unwrap_or(0.0)
}

fn s(v: &Value) -> String {
    v.as_str().map(String::from).or_else(|| v.as_u64().map(|n| n.to_string())).unwrap_or_default()
}

pub fn parse_search(v: &Value) -> Vec<DealGame> {
    v.as_array().map(|a| a.iter().map(|g| DealGame { id: s(&g["gameID"]), title: s(&g["external"]), thumb: s(&g["thumb"]), cheapest: f(&g["cheapest"]) }).filter(|g| !g.id.is_empty()).collect()).unwrap_or_default()
}

/// Store names by id, from /stores.
pub fn parse_stores(v: &Value) -> std::collections::HashMap<String, String> {
    v.as_array().map(|a| a.iter().map(|x| (s(&x["storeID"]), s(&x["storeName"]))).collect()).unwrap_or_default()
}

pub fn parse_game(v: &Value, stores: &std::collections::HashMap<String, String>) -> GameDeals {
    let mut deals: Vec<Deal> = v["deals"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|d| {
                    let id = s(&d["storeID"]);
                    Deal { store: stores.get(&id).cloned().unwrap_or_else(|| format!("Store {id}")), price: f(&d["price"]), retail: f(&d["retailPrice"]), savings: (f(&d["savings"]) * 10.0).round() / 10.0, url: format!("https://www.cheapshark.com/redirect?dealID={}", s(&d["dealID"])) }
                })
                .collect()
        })
        .unwrap_or_default();
    deals.sort_by(|a, b| a.price.total_cmp(&b.price));
    let ever = &v["cheapestPriceEver"];
    GameDeals { title: s(&v["info"]["title"]), deals, lowest_ever: (ever.is_object()).then(|| (f(&ever["price"]), ever["date"].as_i64().unwrap_or(0))) }
}

fn enc(q: &str) -> String {
    q.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

pub fn search(title: &str) -> Result<Vec<DealGame>, String> {
    let t = title.trim();
    if t.is_empty() || t.len() > 100 {
        return Err("Type a game's name".into());
    }
    Ok(parse_search(&crate::web::get_json(&format!("{API}/games?title={}&limit=12", enc(t)))?))
}

pub fn game(id: &str) -> Result<GameDeals, String> {
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err("That isn't a CheapShark game".into());
    }
    let stores = crate::web::get_json(&format!("{API}/stores")).map(|v| parse_stores(&v)).unwrap_or_default();
    Ok(parse_game(&crate::web::get_json(&format!("{API}/games?id={id}"))?, &stores))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_cheapshark() {
        let hits = parse_search(&serde_json::json!([{"gameID":"612","steamAppID":"8870","cheapest":"3.99","external":"BioShock Infinite","thumb":"https://x/y.jpg"}]));
        assert_eq!(hits[0], DealGame { id: "612".into(), title: "BioShock Infinite".into(), thumb: "https://x/y.jpg".into(), cheapest: 3.99 });
        let stores = parse_stores(&serde_json::json!([{"storeID":"1","storeName":"Steam"},{"storeID":"7","storeName":"GOG"}]));
        let g = parse_game(&serde_json::json!({"info":{"title":"BioShock Infinite"},"cheapestPriceEver":{"price":"2.99","date":1543028665},"deals":[{"storeID":"1","dealID":"abc","price":"29.99","retailPrice":"29.99","savings":"0.000000"},{"storeID":"7","dealID":"def","price":"3.99","retailPrice":"29.99","savings":"86.695565"}]}), &stores);
        assert_eq!(g.deals[0].store, "GOG");
        assert_eq!((g.deals[0].price, g.deals[0].savings), (3.99, 86.7));
        assert_eq!(g.deals[0].url, "https://www.cheapshark.com/redirect?dealID=def");
        assert_eq!(g.lowest_ever, Some((2.99, 1543028665)));
        assert_eq!(enc("Halo: Infinite & co"), "Halo%3A%20Infinite%20%26%20co");
    }
}
