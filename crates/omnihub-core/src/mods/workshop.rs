//! Steam Workshop items a Steam game has downloaded. Steam manages them
//! (subscribe and unsubscribe on the item's page); OmniHub lists them with
//! their names and pictures from Steam's public item details, which need
//! no key.

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkshopItem {
    pub id: String,
    pub title: String,
    pub image: String,
    pub size: u64,
    /// Unix seconds.
    pub updated: i64,
    pub url: String,
}

/// The items in `<library>/steamapps/workshop` for `app_id`, newest first.
pub fn installed(steamapps: &Path, app_id: u32) -> Vec<WorkshopItem> {
    let acf = steamapps.join("workshop").join(format!("appworkshop_{app_id}.acf"));
    let mut sizes: HashMap<String, (u64, i64)> = HashMap::new();
    if let Ok(t) = std::fs::read_to_string(&acf) {
        // "WorkshopItemsInstalled" { "<id>" { "size" "n" "timeupdated" "t" ... } }
        let mut cur: Option<String> = None;
        let mut in_installed = false;
        for raw in t.lines() {
            let l = raw.trim();
            let q: Vec<&str> = l.split('"').filter(|s| !s.trim().is_empty()).collect();
            if l.contains("WorkshopItemsInstalled") {
                in_installed = true;
                continue;
            }
            if l.contains("WorkshopItemDetails") {
                in_installed = false;
            }
            if !in_installed {
                continue;
            }
            match q.as_slice() {
                [id] if id.bytes().all(|b| b.is_ascii_digit()) => cur = Some(id.to_string()),
                [k, v] => {
                    if let Some(id) = &cur {
                        let e = sizes.entry(id.clone()).or_default();
                        match *k {
                            "size" => e.0 = v.parse().unwrap_or(0),
                            "timeupdated" => e.1 = v.parse().unwrap_or(0),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let content = steamapps.join("workshop").join("content").join(app_id.to_string());
    if let Ok(rd) = std::fs::read_dir(&content) {
        for e in rd.flatten() {
            let id = e.file_name().to_string_lossy().into_owned();
            if id.bytes().all(|b| b.is_ascii_digit()) && e.file_type().is_ok_and(|t| t.is_dir()) {
                sizes.entry(id).or_default();
            }
        }
    }
    let mut v: Vec<WorkshopItem> = sizes
        .into_iter()
        .map(|(id, (size, updated))| WorkshopItem { url: format!("https://steamcommunity.com/sharedfiles/filedetails/?id={id}"), title: String::new(), image: String::new(), id, size, updated })
        .collect();
    v.sort_by(|a, b| b.updated.cmp(&a.updated).then(a.id.cmp(&b.id)));
    v
}

/// Names and pictures from Steam (up to 100 at a time). Items Steam
/// doesn't answer for keep their id as the name.
pub fn fill_details(items: &mut [WorkshopItem], base: &str) -> Result<(), String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(15))).http_status_as_error(false).build().into();
    for chunk in items.chunks_mut(100) {
        let mut form: Vec<(String, String)> = vec![("itemcount".into(), chunk.len().to_string())];
        for (i, it) in chunk.iter().enumerate() {
            form.push((format!("publishedfileids[{i}]"), it.id.clone()));
        }
        let body = form.iter().map(|(k, v)| format!("{}={}", percent_encoding::utf8_percent_encode(k, percent_encoding::NON_ALPHANUMERIC), v)).collect::<Vec<_>>().join("&");
        let mut res = agent
            .post(&format!("{}/ISteamRemoteStorage/GetPublishedFileDetails/v1/", base.trim_end_matches('/')))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send(body)
            .map_err(|e| format!("Couldn't reach Steam: {e}"))?;
        let v: serde_json::Value = serde_json::from_str(&res.body_mut().read_to_string().unwrap_or_default()).unwrap_or_default();
        for d in v["response"]["publishedfiledetails"].as_array().into_iter().flatten() {
            let Some(id) = d["publishedfileid"].as_str() else { continue };
            if let Some(it) = chunk.iter_mut().find(|x| x.id == id) {
                it.title = d["title"].as_str().unwrap_or_default().to_string();
                it.image = d["preview_url"].as_str().filter(|u| u.starts_with("https://")).unwrap_or_default().to_string();
                if it.size == 0 {
                    it.size = d["file_size"].as_str().and_then(|s| s.parse().ok()).or(d["file_size"].as_u64()).unwrap_or(0);
                }
            }
        }
    }
    for it in items.iter_mut().filter(|i| i.title.is_empty()) {
        it.title = format!("Workshop item {}", it.id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_steams_workshop_records() {
        let t = tempfile::tempdir().unwrap();
        let apps = t.path();
        std::fs::create_dir_all(apps.join("workshop/content/294100/111")).unwrap();
        std::fs::create_dir_all(apps.join("workshop/content/294100/333")).unwrap();
        std::fs::write(
            apps.join("workshop/appworkshop_294100.acf"),
            "\"AppWorkshop\"\n{\n\t\"appid\"\t\t\"294100\"\n\t\"WorkshopItemsInstalled\"\n\t{\n\t\t\"111\"\n\t\t{\n\t\t\t\"size\"\t\t\"2048\"\n\t\t\t\"timeupdated\"\t\t\"1700000000\"\n\t\t}\n\t\t\"222\"\n\t\t{\n\t\t\t\"size\"\t\t\"10\"\n\t\t\t\"timeupdated\"\t\t\"1800000000\"\n\t\t}\n\t}\n\t\"WorkshopItemDetails\"\n\t{\n\t\t\"111\"\n\t\t{\n\t\t\t\"size\"\t\t\"999999\"\n\t\t}\n\t}\n}\n",
        )
        .unwrap();
        let items = installed(apps, 294100);
        assert_eq!(items.iter().map(|i| (i.id.as_str(), i.size)).collect::<Vec<_>>(), [("222", 10), ("111", 2048), ("333", 0)]);
        assert!(items[0].url.ends_with("?id=222"));
        assert!(installed(apps, 1).is_empty());
    }
}
