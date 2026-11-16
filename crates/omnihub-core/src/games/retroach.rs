//! RetroAchievements: your recently played games and the achievements you
//! unlocked, from retroachievements.org with your own username and web API
//! key (from your account's settings page there). The key is kept on this
//! PC only, encrypted for your Windows account, and only ever sent to
//! retroachievements.org over HTTPS.

use std::path::{Path, PathBuf};

use base64::Engine;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const ENTROPY: &[u8] = b"omnihub-retroachievements";
const MEDIA: &str = "https://media.retroachievements.org";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
struct Stored {
    user: String,
    /// The key, DPAPI-encrypted, base64.
    secret: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RaGame {
    pub id: u64,
    pub title: String,
    pub console: String,
    pub icon: String,
    pub last_played: String,
    pub achieved: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RaAchievement {
    pub title: String,
    pub description: String,
    pub game: String,
    pub badge: String,
    pub points: u32,
    pub date: String,
    pub hardcore: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RaSummary {
    pub user: String,
    pub points: u64,
    pub rank: Option<u64>,
    pub games: Vec<RaGame>,
    pub recent: Vec<RaAchievement>,
}

fn num(v: &Value) -> u64 {
    v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())).unwrap_or(0)
}

fn text(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

/// Read API_GetUserSummary's answer.
pub fn parse(v: &Value) -> RaSummary {
    let awarded = &v["Awarded"];
    let games = v["RecentlyPlayed"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|g| {
                    let id = num(&g["GameID"]);
                    let aw = &awarded[id.to_string()];
                    RaGame {
                        id,
                        title: text(&g["Title"]),
                        console: text(&g["ConsoleName"]),
                        icon: if text(&g["ImageIcon"]).is_empty() { String::new() } else { format!("{MEDIA}{}", text(&g["ImageIcon"])) },
                        last_played: text(&g["LastPlayed"]),
                        achieved: num(&aw["NumAchieved"]).max(num(&aw["NumAchievedHardcore"])) as u32,
                        total: (num(&aw["NumPossibleAchievements"]).max(num(&g["AchievementsTotal"]))) as u32,
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let mut recent: Vec<RaAchievement> = v["RecentAchievements"]
        .as_object()
        .map(|by_game| {
            by_game
                .values()
                .filter_map(|g| g.as_object())
                .flat_map(|g| g.values())
                .map(|a| RaAchievement {
                    title: text(&a["Title"]),
                    description: text(&a["Description"]),
                    game: text(&a["GameTitle"]),
                    badge: if text(&a["BadgeName"]).is_empty() { String::new() } else { format!("{MEDIA}/Badge/{}.png", text(&a["BadgeName"])) },
                    points: num(&a["Points"]) as u32,
                    date: text(&a["DateAwarded"]),
                    hardcore: num(&a["HardcoreAchieved"]) == 1,
                })
                .collect()
        })
        .unwrap_or_default();
    recent.sort_by(|a, b| b.date.cmp(&a.date));
    RaSummary { user: text(&v["User"]), points: num(&v["TotalPoints"]), rank: Some(num(&v["Rank"])).filter(|r| *r > 0), games, recent }
}

pub struct RetroAchievements {
    file: PathBuf,
    stored: Mutex<Stored>,
}

impl RetroAchievements {
    pub fn new(data: &Path) -> RetroAchievements {
        let file = data.join("retroachievements.json");
        let stored = std::fs::read(&file).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        RetroAchievements { file, stored: Mutex::new(stored) }
    }

    /// The username, when connected.
    pub fn user(&self) -> Option<String> {
        let s = self.stored.lock();
        (!s.user.is_empty() && !s.secret.is_empty()).then(|| s.user.clone())
    }

    /// Save the account (an empty key disconnects).
    pub fn connect(&self, user: &str, key: &str) -> Result<(), String> {
        let user = user.trim();
        let key = key.trim();
        let s = if key.is_empty() {
            Stored::default()
        } else {
            if user.is_empty() || user.len() > 40 || !user.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err("Type your RetroAchievements username".into());
            }
            if key.len() > 64 || !key.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Err("That doesn't look like a web API key (it's on your RetroAchievements settings page)".into());
            }
            let secret = base64::engine::general_purpose::STANDARD.encode(crate::system::dpapi::protect(key.as_bytes(), ENTROPY).map_err(|e| e.to_string())?);
            Stored { user: user.into(), secret }
        };
        crate::settings::write_atomic(&self.file, &serde_json::to_vec_pretty(&s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        *self.stored.lock() = s;
        Ok(())
    }

    pub fn summary(&self) -> Result<RaSummary, String> {
        let s = self.stored.lock().clone();
        if s.user.is_empty() || s.secret.is_empty() {
            return Err("Connect your RetroAchievements account first".into());
        }
        let raw = base64::engine::general_purpose::STANDARD.decode(&s.secret).map_err(|_| "The saved key can't be read; connect again".to_string())?;
        let key = crate::system::dpapi::unprotect(&raw, ENTROPY).map_err(|_| "The saved key can't be read on this Windows account; connect again".to_string())?;
        let key = String::from_utf8_lossy(&key).to_string();
        let url = format!("https://retroachievements.org/API/API_GetUserSummary.php?u={}&y={key}&g=10&a=10", s.user);
        // The address holds the key: errors never repeat it.
        let v = crate::web::get_json(&url).map_err(|_| "RetroAchievements didn't answer (check the username and key)".to_string())?;
        if v.get("User").is_none() {
            return Err("RetroAchievements didn't accept that username and key".into());
        }
        Ok(parse(&v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_summary() {
        let v = serde_json::json!({
            "User": "Player", "TotalPoints": 1234, "Rank": "5678",
            "RecentlyPlayed": [{"GameID": "11240", "ConsoleName": "Game Boy Advance", "Title": "Pokemon Emerald", "ImageIcon": "/Images/001.png", "LastPlayed": "2026-11-06 20:00:00", "AchievementsTotal": 60}],
            "Awarded": {"11240": {"NumPossibleAchievements": 60, "NumAchieved": 12, "NumAchievedHardcore": 10}},
            "RecentAchievements": {"11240": {"1": {"Title": "Badge", "Description": "Win", "GameTitle": "Pokemon Emerald", "BadgeName": "123", "Points": 5, "DateAwarded": "2026-11-06 20:01:00", "HardcoreAchieved": 1}, "2": {"Title": "Older", "GameTitle": "Pokemon Emerald", "DateAwarded": "2026-11-01 10:00:00", "HardcoreAchieved": 0}}}
        });
        let s = parse(&v);
        assert_eq!((s.user.as_str(), s.points, s.rank), ("Player", 1234, Some(5678)));
        assert_eq!((s.games[0].achieved, s.games[0].total, s.games[0].icon.as_str()), (12, 60, "https://media.retroachievements.org/Images/001.png"));
        assert_eq!(s.recent.iter().map(|a| a.title.as_str()).collect::<Vec<_>>(), vec!["Badge", "Older"]);
        assert!(s.recent[0].hardcore && s.recent[0].badge.ends_with("/Badge/123.png"));
        let t = tempfile::tempdir().unwrap();
        let ra = RetroAchievements::new(t.path());
        assert!(ra.connect("bad name!", "abc").is_err());
        assert!(ra.connect("Player", "key with spaces").is_err());
        ra.connect("", "").unwrap();
        assert!(ra.user().is_none());
    }
}
