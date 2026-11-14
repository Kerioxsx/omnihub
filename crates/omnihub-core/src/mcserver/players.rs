//! Players: the whitelist, operators and bans (the server's own JSON
//! files, changed by command while it runs), the players it has seen, and
//! what a player carries and where they are — read live from the server or
//! from their saved file while they're away.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::nbt::{self, Tag};

/// A Minecraft name as the server accepts it.
pub fn valid_name(n: &str) -> bool {
    (1..=16).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// The UUID an offline-mode server gives a name (Java's nameUUIDFromBytes).
pub fn offline_uuid(name: &str) -> String {
    let mut h = md5(format!("OfflinePlayer:{name}").as_bytes());
    h[6] = (h[6] & 0x0f) | 0x30;
    h[8] = (h[8] & 0x3f) | 0x80;
    dashed(&hex::encode(h))
}

/// 32 hex digits as a UUID with dashes.
pub fn dashed(h: &str) -> String {
    let h = h.replace('-', "").to_lowercase();
    if h.len() != 32 {
        return h;
    }
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

pub fn valid_uuid(u: &str) -> bool {
    let h = u.replace('-', "");
    h.len() == 32 && h.bytes().all(|b| b.is_ascii_hexdigit()) && (u.len() == 32 || u.len() == 36)
}

/// MD5 (RFC 1321), only for offline-mode UUIDs.
fn md5(input: &[u8]) -> [u8; 16] {
    const S: [u32; 64] = [7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21];
    let k: Vec<u32> = (0..64).map(|i| ((i as f64 + 1.0).sin().abs() * 4_294_967_296.0) as u32).collect();
    let (mut a0, mut b0, mut c0, mut d0) = (0x6745_2301u32, 0xefcd_ab89u32, 0x98ba_dcfeu32, 0x1032_5476u32);
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend(((input.len() as u64).wrapping_mul(8)).to_le_bytes());
    for chunk in msg.chunks(64) {
        let m: Vec<u32> = chunk.chunks(4).map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]])).collect();
        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);
        for i in 0..64 {
            let (f, g) = match i / 16 {
                0 => ((b & c) | (!b & d), i),
                1 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                2 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let f = f.wrapping_add(a).wrapping_add(k[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }
    let mut out = [0u8; 16];
    for (i, v) in [a0, b0, c0, d0].iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_le_bytes());
    }
    out
}

/// One entry of whitelist.json, ops.json or banned-players.json.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Listed {
    pub uuid: String,
    pub name: String,
    /// ops.json: 1–4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bypasses_player_limit: Option<bool>,
    /// Bans: why, by whom, since and until.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<String>,
}

impl Listed {
    pub fn plain(uuid: &str, name: &str) -> Listed {
        Listed { uuid: uuid.into(), name: name.into(), level: None, bypasses_player_limit: None, reason: None, source: None, created: None, expires: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum List {
    Whitelist,
    Ops,
    Bans,
}

impl List {
    pub fn file(self) -> &'static str {
        match self {
            List::Whitelist => "whitelist.json",
            List::Ops => "ops.json",
            List::Bans => "banned-players.json",
        }
    }
}

pub fn read_list(dir: &Path, l: List) -> Vec<Listed> {
    std::fs::read(dir.join(l.file())).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

pub fn write_list(dir: &Path, l: List, v: &[Listed]) -> Result<(), String> {
    let p = dir.join(l.file());
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(v).unwrap_or_default()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

/// Add (or replace) someone on a list in its file (the server is stopped).
pub fn add_to(dir: &Path, l: List, mut who: Listed, reason: Option<&str>) -> Result<(), String> {
    let mut v = read_list(dir, l);
    v.retain(|x| !x.uuid.eq_ignore_ascii_case(&who.uuid) && !x.name.eq_ignore_ascii_case(&who.name));
    match l {
        List::Ops => {
            who.level = Some(4);
            who.bypasses_player_limit = Some(false);
        }
        List::Bans => {
            who.created = Some(chrono::Utc::now().format("%Y-%m-%d %H:%M:%S +0000").to_string());
            who.source = Some("Server".into());
            who.expires = Some("forever".into());
            who.reason = Some(reason.filter(|r| !r.trim().is_empty()).unwrap_or("Banned by an operator.").to_string());
        }
        List::Whitelist => {}
    }
    v.push(who);
    write_list(dir, l, &v)
}

pub fn remove_from(dir: &Path, l: List, name: &str) -> Result<(), String> {
    let mut v = read_list(dir, l);
    v.retain(|x| !x.name.eq_ignore_ascii_case(name));
    write_list(dir, l, &v)
}

/// A player the server has seen (usercache.json), newest first.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cached {
    pub name: String,
    pub uuid: String,
    #[serde(rename = "expiresOn", default)]
    pub expires_on: String,
}

pub fn seen(dir: &Path) -> Vec<Cached> {
    std::fs::read(dir.join("usercache.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// A player's saved file: `world/players/data/<uuid>.dat` (26.1+) or `world/playerdata/<uuid>.dat`.
pub fn data_file(world: &Path, uuid: &str) -> Option<PathBuf> {
    if !valid_uuid(uuid) {
        return None;
    }
    let u = dashed(uuid);
    [world.join("players").join("data").join(format!("{u}.dat")), world.join("playerdata").join(format!("{u}.dat"))].into_iter().find(|p| p.is_file())
}

/// One stack of items.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    /// Inventory slot: 0–8 hotbar, 9–35 the rest; 100–103 armour, -106 off hand.
    pub slot: i32,
    pub id: String,
    pub label: String,
    pub count: u32,
    /// A name given with an anvil.
    pub custom: Option<String>,
    pub enchanted: bool,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub pos: Option<[f64; 3]>,
    pub dimension: Option<String>,
    pub health: Option<f64>,
    pub food: Option<i64>,
    pub xp_level: Option<i64>,
    /// survival, creative, adventure, spectator
    pub gamemode: Option<String>,
    pub selected: Option<i64>,
    pub items: Vec<Item>,
    pub ender: Vec<Item>,
}

/// "minecraft:diamond_sword" → "Diamond Sword".
pub fn label(id: &str) -> String {
    let base = id.rsplit(':').next().unwrap_or(id);
    base.split('_').filter(|w| !w.is_empty()).map(|w| {
        let mut c = w.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    }).collect::<Vec<_>>().join(" ")
}

/// A text component's plain text (a JSON string, SNBT compound or plain text).
fn text_of(t: &Tag) -> Option<String> {
    match t {
        Tag::Str(s) if s.trim_start().starts_with('{') || s.trim_start().starts_with('"') => {
            let v: Value = serde_json::from_str(s).ok()?;
            Some(match v {
                Value::String(s) => s,
                v => {
                    let mut out = v["text"].as_str().unwrap_or_default().to_string();
                    for e in v["extra"].as_array().into_iter().flatten() {
                        out.push_str(e["text"].as_str().or(e.as_str()).unwrap_or_default());
                    }
                    out
                }
            })
        }
        Tag::Str(s) => Some(s.clone()),
        Tag::Compound(_) => t.get("text").and_then(Tag::str).map(String::from),
        _ => None,
    }
    .filter(|s| !s.is_empty())
}

fn item(t: &Tag, slot: Option<i32>) -> Option<Item> {
    let id = t.get("id")?.str()?.to_string();
    if id == "minecraft:air" {
        return None;
    }
    let slot = slot.or_else(|| t.get("Slot").and_then(Tag::int).map(|s| s as i32)).unwrap_or(0);
    let count = t.get("count").or(t.get("Count")).and_then(Tag::int).unwrap_or(1).max(0) as u32;
    let comps = t.get("components");
    let custom = comps.and_then(|c| c.get("minecraft:custom_name")).and_then(text_of).or_else(|| t.get("tag").and_then(|x| x.get("display")).and_then(|d| d.get("Name")).and_then(text_of));
    let enchanted = comps.is_some_and(|c| c.get("minecraft:enchantments").is_some_and(|e| e.get("levels").map(|l| !matches!(l, Tag::Compound(v) if v.is_empty())).unwrap_or(!matches!(e, Tag::Compound(v) if v.is_empty()))) || c.get("minecraft:stored_enchantments").is_some())
        || t.get("tag").is_some_and(|x| x.get("Enchantments").is_some_and(|e| !e.list().is_empty()));
    Some(Item { slot, label: label(&id), id, count, custom, enchanted })
}

/// What the player data (a saved file, or `data get entity` read live) says.
pub fn details(t: &Tag) -> Details {
    let mut d = Details::default();
    let pos: Vec<f64> = t.get("Pos").map(|p| p.list().iter().filter_map(Tag::num).collect()).unwrap_or_default();
    if let [x, y, z] = pos[..] {
        d.pos = Some([x, y, z]);
    }
    d.dimension = t.get("Dimension").and_then(|x| x.str().map(String::from).or_else(|| x.int().map(|n| match n { -1 => "minecraft:the_nether", 1 => "minecraft:the_end", _ => "minecraft:overworld" }.to_string())));
    d.health = t.get("Health").and_then(Tag::num);
    d.food = t.get("foodLevel").and_then(Tag::int);
    d.xp_level = t.get("XpLevel").and_then(Tag::int);
    d.gamemode = t.get("playerGameType").and_then(Tag::int).map(|g| match g { 1 => "creative", 2 => "adventure", 3 => "spectator", _ => "survival" }.to_string());
    d.selected = t.get("SelectedItemSlot").and_then(Tag::int);
    d.items = t.get("Inventory").map(|i| i.list().iter().filter_map(|x| item(x, None)).collect()).unwrap_or_default();
    // 1.21.5+: armour and the off hand are in `equipment`.
    if let Some(eq) = t.get("equipment") {
        for (k, slot) in [("feet", 100), ("legs", 101), ("chest", 102), ("head", 103), ("offhand", -106)] {
            if let Some(i) = eq.get(k).and_then(|x| item(x, Some(slot))) {
                d.items.push(i);
            }
        }
    }
    d.ender = t.get("EnderItems").map(|i| i.list().iter().filter_map(|x| item(x, None)).collect()).unwrap_or_default();
    d
}

/// A saved player's details from their file.
pub fn saved(world: &Path, uuid: &str) -> Option<Details> {
    let f = data_file(world, uuid)?;
    let b = std::fs::read(f).ok()?;
    nbt::read(&b).ok().map(|t| details(&t))
}

/// The command slot name for an inventory slot (`item replace entity …`).
pub fn slot_name(slot: i32) -> Option<String> {
    Some(match slot {
        0..=8 => format!("hotbar.{slot}"),
        9..=35 => format!("inventory.{}", slot - 9),
        100 => "armor.feet".into(),
        101 => "armor.legs".into(),
        102 => "armor.chest".into(),
        103 => "armor.head".into(),
        -106 => "weapon.offhand".into(),
        _ => return None,
    })
}

/// An item id a command may use: `minecraft:diamond` or a mod's `create:cogwheel`.
pub fn valid_item(id: &str) -> bool {
    let (ns, path) = id.split_once(':').unwrap_or(("minecraft", id));
    !ns.is_empty() && !path.is_empty() && id.len() <= 100 && ns.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-' | b'.')) && path.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-' | b'.' | b'/'))
}

/// Who a name belongs to, from Mojang (online-mode servers).
pub fn mojang_uuid(agent: &ureq::Agent, base: &str, name: &str) -> Result<Option<(String, String)>, String> {
    if !valid_name(name) {
        return Err("Minecraft names are 1–16 letters, digits or _.".into());
    }
    let mut res = agent.get(&format!("{base}/users/profiles/minecraft/{name}")).header("User-Agent", super::install::ua()).call().map_err(|e| format!("Couldn't reach Mojang: {e}"))?;
    let status = res.status().as_u16();
    if status == 204 || status == 404 {
        return Ok(None);
    }
    if !(200..300).contains(&status) {
        return Err(format!("Mojang answered {status}."));
    }
    let v: Value = serde_json::from_str(&res.body_mut().read_to_string().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let id = v["id"].as_str().filter(|i| valid_uuid(i)).ok_or("Mojang sent no id.")?;
    Ok(Some((dashed(id), v["name"].as_str().unwrap_or(name).to_string())))
}

/// Whitelisted, operators and bans together, for the window.
pub fn lists_json(dir: &Path) -> Value {
    json!({
        "whitelist": read_list(dir, List::Whitelist),
        "ops": read_list(dir, List::Ops),
        "bans": read_list(dir, List::Bans),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuids_match_the_server() {
        // As a 26.x server in offline mode wrote them.
        assert_eq!(offline_uuid("Steve"), "5627dd98-e6be-3c21-b8a8-e92344183641");
        assert_eq!(offline_uuid("notch"), "42653081-a90e-3475-b3d6-3550cdb43f8e");
        assert_eq!(offline_uuid("griefer"), "890b397a-1055-387b-b23b-f8d63f69bc33");
        assert_eq!(hex::encode(md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex::encode(md5(b"The quick brown fox jumps over the lazy dog")), "9e107d9d372bb6826bd81d3542a419d6");
        assert!(valid_name("Steve_01") && !valid_name("bad name") && !valid_name("") && !valid_name("x".repeat(17).as_str()));
        assert_eq!(dashed("069a79f444e94726a5befca90e38aaf5"), "069a79f4-44e9-4726-a5be-fca90e38aaf5");
    }

    #[test]
    fn lists_and_inventories() {
        let t = tempfile::tempdir().unwrap();
        std::fs::write(t.path().join("ops.json"), r#"[{"uuid":"5627dd98-e6be-3c21-b8a8-e92344183641","name":"Steve","level":4,"bypassesPlayerLimit":false}]"#).unwrap();
        assert_eq!(read_list(t.path(), List::Ops)[0].level, Some(4));
        add_to(t.path(), List::Whitelist, Listed::plain(&offline_uuid("Alex"), "Alex"), None).unwrap();
        add_to(t.path(), List::Whitelist, Listed::plain(&offline_uuid("Alex"), "Alex"), None).unwrap();
        assert_eq!(read_list(t.path(), List::Whitelist).len(), 1, "once");
        add_to(t.path(), List::Bans, Listed::plain(&offline_uuid("griefer"), "griefer"), Some("cheating")).unwrap();
        let bans = read_list(t.path(), List::Bans);
        assert_eq!((bans[0].reason.as_deref(), bans[0].expires.as_deref()), (Some("cheating"), Some("forever")));
        let raw: Value = serde_json::from_slice(&std::fs::read(t.path().join("whitelist.json")).unwrap()).unwrap();
        assert_eq!(raw, json!([{ "uuid": offline_uuid("Alex"), "name": "Alex" }]), "the server's own format");
        remove_from(t.path(), List::Whitelist, "alex").unwrap();
        assert!(read_list(t.path(), List::Whitelist).is_empty());

        // `data get entity Steve` on 26.1.2 (armour in equipment, components on items).
        let live = nbt::snbt(r#"{Pos: [10.5d, 78.85d, 10.5d], Dimension: "minecraft:the_nether", Health: 18.0f, foodLevel: 20, XpLevel: 3, playerGameType: 1, SelectedItemSlot: 0, Inventory: [{count: 1, Slot: 0b, id: "minecraft:diamond_sword", components: {"minecraft:custom_name": "Big Sword", "minecraft:enchantments": {"minecraft:sharpness": 5}}}, {count: 27, Slot: 1b, id: "minecraft:oak_log"}], equipment: {head: {count: 1, id: "minecraft:diamond_helmet"}, offhand: {count: 16, id: "minecraft:torch"}}}"#).unwrap();
        let d = details(&live);
        assert_eq!(d.pos, Some([10.5, 78.85, 10.5]));
        assert_eq!((d.dimension.as_deref(), d.gamemode.as_deref(), d.health, d.xp_level), (Some("minecraft:the_nether"), Some("creative"), Some(18.0), Some(3)));
        assert_eq!(d.items.len(), 4);
        assert_eq!((d.items[0].label.as_str(), d.items[0].custom.as_deref(), d.items[0].enchanted), ("Diamond Sword", Some("Big Sword"), true));
        assert_eq!(d.items.iter().find(|i| i.slot == 103).unwrap().id, "minecraft:diamond_helmet");
        assert_eq!(d.items.iter().find(|i| i.slot == -106).unwrap().count, 16);
        // An older (1.20.1) file: Count as a byte, armour in Inventory, a numeric dimension.
        let old = Tag::Compound(vec![
            ("Pos".into(), Tag::List(vec![Tag::Double(1.0), Tag::Double(2.0), Tag::Double(3.0)])),
            ("Dimension".into(), Tag::Int(-1)),
            ("Inventory".into(), Tag::List(vec![Tag::Compound(vec![("Slot".into(), Tag::Byte(103)), ("id".into(), Tag::Str("minecraft:iron_helmet".into())), ("Count".into(), Tag::Byte(1)), ("tag".into(), Tag::Compound(vec![("display".into(), Tag::Compound(vec![("Name".into(), Tag::Str(r#"{"text":"Lid"}"#.into()))]))]))])])),
        ]);
        let world = t.path().join("world");
        std::fs::create_dir_all(world.join("playerdata")).unwrap();
        let u = offline_uuid("Old");
        std::fs::write(world.join("playerdata").join(format!("{u}.dat")), nbt::tests::gzip(&nbt::tests::write(&old))).unwrap();
        let d = saved(&world, &u).unwrap();
        assert_eq!((d.dimension.as_deref(), d.items[0].slot, d.items[0].custom.as_deref()), (Some("minecraft:the_nether"), 103, Some("Lid")));
        assert!(saved(&world, "../../etc").is_none());
        assert_eq!((slot_name(0).unwrap(), slot_name(9).unwrap(), slot_name(103).unwrap(), slot_name(-106).unwrap()), ("hotbar.0".into(), "inventory.0".into(), "armor.head".into(), "weapon.offhand".into()));
        assert!(valid_item("minecraft:diamond") && valid_item("create:cogwheel") && valid_item("diamond") && !valid_item("diamond 64; op Steve") && !valid_item("Minecraft:X"));
    }
}
