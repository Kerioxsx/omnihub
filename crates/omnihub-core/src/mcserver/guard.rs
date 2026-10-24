//! Servers that let players without a paid Minecraft account join (offline
//! mode). Minecraft can't check names there, so anyone could join with an
//! operator's name. While names aren't checked OmniHub holds the operator
//! list itself and makes someone an operator only when they join from a
//! place (network address) the owner allowed; an operator's name from a new
//! place is kicked and waits for the owner's OK, on the PC or the phone.
//!
//! A name has a different UUID on each kind of server (Mojang's, or one made
//! from the name), so switching kinds moves the whitelist, operators, bans
//! and saved players over too.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::players::{self, List, Listed};

/// What a kicked operator name is told.
pub const KICK: &str = "This name is an operator here and joined from a new place. Ask the server owner to allow it in OmniHub, then join again.";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Guard {
    /// Operators OmniHub holds while names aren't checked.
    pub ops: Vec<Listed>,
    /// Places each operator may join from (lower-case name → addresses).
    pub allowed: BTreeMap<String, Vec<String>>,
    /// Where each player last joined from (lower-case name → address).
    pub last: BTreeMap<String, String>,
    /// Operator names that tried to join from a new place.
    pub waiting: Vec<Waiting>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Waiting {
    pub name: String,
    pub place: String,
    pub at: i64,
}

fn file(dir: &Path) -> PathBuf {
    dir.join(".omnihub").join("guard.json")
}

pub fn load(dir: &Path) -> Guard {
    std::fs::read(file(dir)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub fn save(dir: &Path, g: &Guard) -> Result<(), String> {
    let p = file(dir);
    std::fs::create_dir_all(p.parent().unwrap_or(dir)).map_err(|e| e.to_string())?;
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(g).unwrap_or_default()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

/// The address part of what the server logs ("127.0.0.1:5123", "[::1]:5123").
pub fn place(addr: &str) -> String {
    let a = addr.trim().trim_start_matches('/');
    let host = match a.strip_prefix('[') {
        Some(rest) => rest.split_once(']').map_or(rest, |(h, _)| h),
        None if a.matches(':').count() == 1 => a.split_once(':').map_or(a, |(h, _)| h),
        // Older Java: IPv6 without brackets, port last.
        None => a.rsplit_once(':').map_or(a, |(h, _)| h),
    };
    let host = host.split('%').next().unwrap_or(host);
    host.parse::<IpAddr>().map(|ip| ip.to_string()).unwrap_or_else(|_| host.to_string())
}

/// Playing on this PC itself.
pub fn is_local(place: &str) -> bool {
    place.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

/// A UUID made from the name (version 3), as servers that don't check accounts use.
pub fn is_offline_uuid(u: &str) -> bool {
    let h: String = u.chars().filter(|c| *c != '-').collect();
    h.len() == 32 && h.as_bytes()[12] == b'3'
}

impl Guard {
    pub fn is_op(&self, name: &str) -> bool {
        self.ops.iter().any(|o| o.name.eq_ignore_ascii_case(name))
    }

    pub fn may_join(&self, name: &str, place: &str) -> bool {
        is_local(place) || self.allowed.get(&name.to_lowercase()).is_some_and(|v| v.iter().any(|p| p == place))
    }

    pub fn allow(&mut self, name: &str, place: &str) {
        let v = self.allowed.entry(name.to_lowercase()).or_default();
        if !v.iter().any(|p| p == place) {
            v.push(place.to_string());
        }
        self.waiting.retain(|w| !(w.name.eq_ignore_ascii_case(name) && w.place == place));
    }

    /// Note a kept-out attempt; true when it's a new one (worth telling the owner).
    pub fn wait(&mut self, name: &str, place: &str, at: i64) -> bool {
        if let Some(w) = self.waiting.iter_mut().find(|w| w.name.eq_ignore_ascii_case(name) && w.place == place) {
            w.at = at;
            return false;
        }
        self.waiting.push(Waiting { name: name.into(), place: place.into(), at });
        let extra = self.waiting.len().saturating_sub(20);
        self.waiting.drain(..extra);
        true
    }

    /// Hold an operator (no places allowed yet).
    pub fn add_op(&mut self, mut who: Listed) {
        who.level = Some(who.level.unwrap_or(4));
        who.bypasses_player_limit = Some(who.bypasses_player_limit.unwrap_or(false));
        self.ops.retain(|o| !o.name.eq_ignore_ascii_case(&who.name));
        self.ops.push(who);
    }

    /// Hold an operator the owner (or another operator) picked, allowing
    /// where they last played from (logins that were kept out don't count).
    pub fn add_op_from_last(&mut self, who: Listed) {
        if let Some(p) = self.last.get(&who.name.to_lowercase()).cloned() {
            self.allow(&who.name, &p);
        }
        self.add_op(who);
    }

    pub fn remove_op(&mut self, name: &str) {
        self.ops.retain(|o| !o.name.eq_ignore_ascii_case(name));
        self.allowed.remove(&name.to_lowercase());
        self.waiting.retain(|w| !w.name.eq_ignore_ascii_case(name));
    }
}

/// Before the server starts: hold the operators (names unchecked) or give
/// them back to ops.json (names checked).
pub fn hold_or_return_ops(dir: &Path, offline: bool) -> Result<Guard, String> {
    let mut g = load(dir);
    if offline {
        let ops = players::read_list(dir, List::Ops);
        if !ops.is_empty() {
            for o in ops {
                g.add_op_from_last(o);
            }
            players::write_list(dir, List::Ops, &[])?;
        }
    } else if !g.ops.is_empty() {
        let mut ops = players::read_list(dir, List::Ops);
        for o in g.ops.drain(..) {
            if !ops.iter().any(|x| x.name.eq_ignore_ascii_case(&o.name)) {
                ops.push(o);
            }
        }
        players::write_list(dir, List::Ops, &ops)?;
        g.waiting.clear();
    }
    save(dir, &g)?;
    Ok(g)
}

/// A saved player's files, under both world layouts.
fn player_files(world: &Path, uuid: &str) -> Vec<(PathBuf, &'static str)> {
    let mut v = Vec::new();
    for (sub, ext) in [("players/data", "dat"), ("players/stats", "json"), ("players/advancements", "json"), ("playerdata", "dat"), ("stats", "json"), ("advancements", "json")] {
        v.push((world.join(sub).join(format!("{uuid}.{ext}")), ext));
    }
    v
}

/// Make the lists and saved players use the kind of UUID the server will use
/// (`offline`: made from the name; else Mojang's, looked up by name when not
/// known). Saved players are copied, not moved, so switching back works.
/// Returns the names that were moved over.
pub fn match_accounts(dir: &Path, world: &Path, offline: bool, lookup: &mut dyn FnMut(&str) -> Option<(String, String)>) -> Vec<String> {
    let wrong = |u: &str| players::valid_uuid(u) && is_offline_uuid(u) != offline;
    let lists = [List::Whitelist, List::Ops, List::Bans].map(|l| (l, players::read_list(dir, l)));
    let seen = players::seen(dir);
    // Every UUID each name has had.
    let mut had: HashMap<String, (String, BTreeSet<String>)> = HashMap::new();
    for (name, uuid) in lists.iter().flat_map(|(_, v)| v.iter().map(|x| (&x.name, &x.uuid))).chain(seen.iter().map(|c| (&c.name, &c.uuid))) {
        if players::valid_name(name) && players::valid_uuid(uuid) {
            had.entry(name.to_lowercase()).or_insert_with(|| (name.clone(), BTreeSet::new())).1.insert(players::dashed(uuid));
        }
    }
    // Only names on a list with the wrong kind of UUID, or with saved things under one, need work.
    let mut lookups = 0;
    let mut target: HashMap<String, String> = HashMap::new();
    for (key, (name, uuids)) in &had {
        let listed_wrong = lists.iter().any(|(_, v)| v.iter().any(|x| x.name.eq_ignore_ascii_case(name) && wrong(&x.uuid)));
        let saved_wrong = uuids.iter().any(|u| wrong(u) && player_files(world, u).iter().any(|(p, _)| p.is_file()));
        if !listed_wrong && !saved_wrong {
            continue;
        }
        let t = if offline {
            Some(players::offline_uuid(name))
        } else if let Some(u) = uuids.iter().find(|u| !is_offline_uuid(u)) {
            Some(u.clone())
        } else if lookups < 60 {
            lookups += 1;
            lookup(name).map(|(u, _)| u)
        } else {
            None
        };
        if let Some(t) = t {
            target.insert(key.clone(), t);
        }
    }
    let mut moved: BTreeSet<String> = BTreeSet::new();
    for (l, mut v) in lists {
        let mut changed = false;
        for x in v.iter_mut() {
            if let Some(t) = target.get(&x.name.to_lowercase()).filter(|t| wrong(&x.uuid) && **t != x.uuid) {
                x.uuid = t.clone();
                changed = true;
                moved.insert(x.name.clone());
            }
        }
        if changed {
            let _ = players::write_list(dir, l, &v);
        }
    }
    for (key, t) in &target {
        let Some((name, uuids)) = had.get(key) else { continue };
        for u in uuids.iter().filter(|u| *u != t && wrong(u)) {
            for ((from, _), (to, _)) in player_files(world, u).into_iter().zip(player_files(world, t)) {
                if from.is_file() && !to.exists() && std::fs::copy(&from, &to).is_ok() {
                    moved.insert(name.clone());
                }
            }
        }
    }
    moved.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_from_log_addresses() {
        assert_eq!(place("/127.0.0.1:55206"), "127.0.0.1");
        assert_eq!(place("203.0.113.5:1234"), "203.0.113.5");
        assert_eq!(place("[0:0:0:0:0:0:0:1]:55206"), "::1");
        assert_eq!(place("0:0:0:0:0:0:0:1:55206"), "::1");
        assert_eq!(place("[fe80:0:0:0:1:2:3:4%eth0]:5"), "fe80::1:2:3:4");
        assert!(is_local("127.0.0.1") && is_local("::1") && !is_local("192.168.1.20"));
    }

    #[test]
    fn uuid_kinds() {
        assert!(is_offline_uuid(&players::offline_uuid("Steve")));
        assert!(!is_offline_uuid("069a79f4-44e9-4726-a5be-fca90e38aaf5"));
        assert!(!is_offline_uuid("nonsense"));
    }

    #[test]
    fn operators_are_held_and_given_back() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path();
        players::add_to(dir, List::Ops, Listed::plain("069a79f4-44e9-4726-a5be-fca90e38aaf5", "Notch"), None).unwrap();
        let mut g = load(dir);
        g.last.insert("notch".into(), "203.0.113.5".into());
        save(dir, &g).unwrap();
        let g = hold_or_return_ops(dir, true).unwrap();
        assert!(players::read_list(dir, List::Ops).is_empty(), "nobody is an operator until they join from an allowed place");
        assert!(g.is_op("notch"));
        assert!(g.may_join("Notch", "203.0.113.5"), "where they last played is allowed");
        assert!(!g.may_join("Notch", "198.51.100.7"));
        assert!(g.may_join("Notch", "127.0.0.1"), "this PC itself");
        let mut g = g;
        assert!(g.wait("Notch", "198.51.100.7", 1));
        assert!(!g.wait("Notch", "198.51.100.7", 2), "the same attempt again isn't news");
        g.allow("notch", "198.51.100.7");
        assert!(g.waiting.is_empty() && g.may_join("Notch", "198.51.100.7"));
        save(dir, &g).unwrap();
        // Names checked again: the operators go back.
        let g = hold_or_return_ops(dir, false).unwrap();
        assert!(g.ops.is_empty());
        assert_eq!(players::read_list(dir, List::Ops)[0].name, "Notch");
        assert_eq!(players::read_list(dir, List::Ops)[0].level, Some(4));
    }

    #[test]
    fn switching_account_kinds_moves_lists_and_saved_players() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path();
        let world = dir.join("world");
        std::fs::create_dir_all(world.join("players/data")).unwrap();
        std::fs::create_dir_all(world.join("players/stats")).unwrap();
        let premium = "069a79f4-44e9-4726-a5be-fca90e38aaf5";
        let offline = players::offline_uuid("Notch");
        players::add_to(dir, List::Whitelist, Listed::plain(premium, "Notch"), None).unwrap();
        players::add_to(dir, List::Bans, Listed::plain(premium, "Notch"), Some("test")).unwrap();
        std::fs::write(world.join(format!("players/data/{premium}.dat")), b"inventory").unwrap();
        std::fs::write(world.join(format!("players/stats/{premium}.json")), b"{}").unwrap();
        std::fs::write(dir.join("usercache.json"), format!(r#"[{{"name":"Notch","uuid":"{premium}","expiresOn":"x"}}]"#)).unwrap();

        let mut asked = 0;
        let moved = match_accounts(dir, &world, true, &mut |_| {
            asked += 1;
            None
        });
        assert_eq!(moved, vec!["Notch".to_string()]);
        assert_eq!(asked, 0, "name-made UUIDs need no lookups");
        assert_eq!(players::read_list(dir, List::Whitelist)[0].uuid, offline);
        assert_eq!(players::read_list(dir, List::Bans)[0].uuid, offline);
        assert_eq!(players::read_list(dir, List::Bans)[0].reason.as_deref(), Some("test"));
        assert_eq!(std::fs::read(world.join(format!("players/data/{offline}.dat"))).unwrap(), b"inventory");
        assert!(world.join(format!("players/data/{premium}.dat")).is_file(), "copied, so switching back keeps it");
        assert!(world.join(format!("players/stats/{offline}.json")).is_file());
        // Nothing left to do the second time.
        assert!(match_accounts(dir, &world, true, &mut |_| None).is_empty());

        // Back to checked names: Mojang's UUID is already known from before.
        std::fs::write(world.join(format!("players/data/{offline}.dat")), b"newer").unwrap();
        let moved = match_accounts(dir, &world, false, &mut |_| panic!("no lookup needed"));
        assert_eq!(moved, vec!["Notch".to_string()]);
        assert_eq!(players::read_list(dir, List::Whitelist)[0].uuid, premium);
        assert_eq!(std::fs::read(world.join(format!("players/data/{premium}.dat"))).unwrap(), b"inventory", "an existing save is never overwritten");

        // A name only ever seen without an account is looked up.
        players::add_to(dir, List::Whitelist, Listed::plain(&players::offline_uuid("Jeb_"), "Jeb_"), None).unwrap();
        let moved = match_accounts(dir, &world, false, &mut |n| (n == "Jeb_").then(|| ("853c80ef-3c37-49fd-aa49-938b674adae6".to_string(), "jeb_".to_string())));
        assert_eq!(moved, vec!["Jeb_".to_string()]);
        assert!(players::read_list(dir, List::Whitelist).iter().any(|x| x.uuid == "853c80ef-3c37-49fd-aa49-938b674adae6"));
    }
}
