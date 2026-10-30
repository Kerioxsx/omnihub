//! League of Legends' Live Client Data API: Riot's own local web service
//! that a running match answers on (`https://127.0.0.1:2999`). OmniHub
//! reads the match's events once a second while League is open — kills,
//! multikills, dragons, barons, the result — to let Capri react.

use std::time::Duration;

use serde_json::Value;

pub const PROCESS: &str = "League of Legends.exe";
const BASE: &str = "https://127.0.0.1:2999/liveclientdata";

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// You took someone down.
    Kill,
    /// Double (2) to penta (5).
    Multikill(u32),
    Died,
    Dragon { ours: bool },
    Baron { ours: bool },
    Ace { ours: bool },
    Result { won: bool },
}

/// The events after `last` (an `EventID`) concerning `me`, and the newest ID.
pub fn events(json: &Value, me: &str, my_team: &[String], last: Option<u64>) -> (Vec<Event>, Option<u64>) {
    let mut out = Vec::new();
    let mut newest = last;
    let ours = |name: &str| name == me || my_team.iter().any(|n| n == name);
    for e in json["Events"].as_array().into_iter().flatten() {
        let Some(id) = e["EventID"].as_u64() else { continue };
        if last.is_some_and(|l| id <= l) {
            continue;
        }
        newest = Some(newest.map_or(id, |n| n.max(id)));
        let s = |k: &str| e[k].as_str().unwrap_or_default();
        match s("EventName") {
            "ChampionKill" if s("KillerName") == me => out.push(Event::Kill),
            "ChampionKill" if s("VictimName") == me => out.push(Event::Died),
            "Multikill" if s("KillerName") == me => out.push(Event::Multikill(e["KillStreak"].as_u64().unwrap_or(2) as u32)),
            "DragonKill" => out.push(Event::Dragon { ours: ours(s("KillerName")) }),
            "BaronKill" => out.push(Event::Baron { ours: ours(s("KillerName")) }),
            "Ace" => out.push(Event::Ace { ours: ours(s("Acer")) }),
            "GameEnd" => out.push(Event::Result { won: s("Result").eq_ignore_ascii_case("win") }),
            _ => {}
        }
    }
    (out, newest)
}

fn agent() -> ureq::Agent {
    // Riot's local service uses its own certificate; this is a loopback address only.
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .tls_config(ureq::tls::TlsConfig::builder().disable_verification(true).build())
        .build()
        .into()
}

fn get(agent: &ureq::Agent, path: &str) -> Option<Value> {
    let mut res = agent.get(format!("{BASE}/{path}")).call().ok()?;
    let body = res.body_mut().with_config().limit(4 << 20).read_to_string().ok()?;
    serde_json::from_str(&body).ok()
}

/// Reads a running match: the player's name, team mates and new events.
pub struct Watcher {
    agent: ureq::Agent,
    me: Option<String>,
    team: Vec<String>,
    last: Option<u64>,
}

impl Default for Watcher {
    fn default() -> Self {
        Watcher { agent: agent(), me: None, team: Vec::new(), last: None }
    }
}

impl Watcher {
    /// New events since the last call (none while no match is on).
    pub fn poll(&mut self) -> Vec<Event> {
        if self.me.is_none() {
            let Some(name) = get(&self.agent, "activeplayername").and_then(|v| v.as_str().map(str::to_string)) else { return Vec::new() };
            let players = get(&self.agent, "playerlist").unwrap_or(Value::Null);
            let list = players.as_array().cloned().unwrap_or_default();
            let id = |p: &Value| p["riotId"].as_str().or(p["summonerName"].as_str()).unwrap_or_default().to_string();
            let my_team = list.iter().find(|p| id(p) == name).and_then(|p| p["team"].as_str().map(str::to_string));
            self.team = list.iter().filter(|p| my_team.as_deref().is_some_and(|t| p["team"].as_str() == Some(t))).map(id).collect();
            self.me = Some(name);
        }
        let Some(json) = get(&self.agent, "eventdata") else { return Vec::new() };
        // A new match counts its events from 0 again.
        let max = json["Events"].as_array().and_then(|a| a.iter().filter_map(|e| e["EventID"].as_u64()).max());
        if let (Some(m), Some(l)) = (max, self.last) {
            if m < l {
                *self = Watcher::default();
                return Vec::new();
            }
        }
        let first = self.last.is_none();
        let (evs, newest) = events(&json, self.me.as_deref().unwrap_or_default(), &self.team, self.last);
        self.last = newest.or(Some(0));
        // Joining mid-match: what already happened isn't news.
        if first { Vec::new() } else { evs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_the_match() {
        let j = json!({ "Events": [
            { "EventID": 0, "EventName": "GameStart" },
            { "EventID": 1, "EventName": "ChampionKill", "KillerName": "Me#EUW", "VictimName": "Them" },
            { "EventID": 2, "EventName": "Multikill", "KillerName": "Me#EUW", "KillStreak": 3 },
            { "EventID": 3, "EventName": "DragonKill", "KillerName": "Mate" },
            { "EventID": 4, "EventName": "ChampionKill", "KillerName": "Them", "VictimName": "Me#EUW" },
            { "EventID": 5, "EventName": "BaronKill", "KillerName": "Them" },
            { "EventID": 6, "EventName": "GameEnd", "Result": "Win" }
        ]});
        let team = vec!["Me#EUW".to_string(), "Mate".to_string()];
        let (e, last) = events(&j, "Me#EUW", &team, None);
        assert_eq!(e, vec![Event::Kill, Event::Multikill(3), Event::Dragon { ours: true }, Event::Died, Event::Baron { ours: false }, Event::Result { won: true }]);
        assert_eq!(last, Some(6));
        let (again, _) = events(&j, "Me#EUW", &team, Some(4));
        assert_eq!(again, vec![Event::Baron { ours: false }, Event::Result { won: true }]);
    }
}
