//! The mod manager and the mod maker end to end, against pretend
//! Thunderstore, GitHub and Claude servers and pretend game folders.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde_json::{json, Value};

use super::*;
use crate::claude::api::tests::sse;

/// One request the pretend servers saw.
#[derive(Debug, Clone)]
pub struct Seen {
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

type Route = Box<dyn Fn(&Seen) -> Option<(u16, &'static str, Vec<u8>)> + Send + Sync>;

pub struct Server {
    pub base: String,
    pub seen: Arc<Mutex<Vec<Seen>>>,
    pub claude: Arc<Mutex<VecDeque<String>>>,
}

pub fn serve(route: Route) -> Server {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", l.local_addr().unwrap());
    let seen: Arc<Mutex<Vec<Seen>>> = Arc::default();
    let claude: Arc<Mutex<VecDeque<String>>> = Arc::default();
    let (s2, c2) = (seen.clone(), claude.clone());
    let route = Arc::new(route);
    std::thread::spawn(move || {
        for stream in l.incoming().flatten() {
            let (seen, claude, route) = (s2.clone(), c2.clone(), route.clone());
            std::thread::spawn(move || {
                let mut r = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                if r.read_line(&mut line).is_err() {
                    return;
                }
                let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
                let mut headers = Vec::new();
                loop {
                    let mut h = String::new();
                    if r.read_line(&mut h).is_err() || h.trim().is_empty() {
                        break;
                    }
                    if let Some((k, v)) = h.split_once(':') {
                        headers.push((k.trim().to_lowercase(), v.trim().to_string()));
                    }
                }
                let len = headers.iter().find(|(k, _)| k == "content-length").and_then(|(_, v)| v.parse::<usize>().ok()).unwrap_or(0);
                let mut body = vec![0; len];
                let _ = r.read_exact(&mut body);
                let req = Seen { path: path.clone(), headers, body };
                seen.lock().push(req.clone());
                let (status, ctype, out) = if path.starts_with("/v1/messages") {
                    match claude.lock().pop_front() {
                        Some(s) => (200, "text/event-stream", s.into_bytes()),
                        None => (500, "application/json", br#"{"type":"error","error":{"type":"api_error","message":"no script"}}"#.to_vec()),
                    }
                } else {
                    route(&req).unwrap_or((404, "text/plain", b"not found".to_vec()))
                };
                let mut w = stream;
                let _ = write!(w, "HTTP/1.1 {status} X\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", out.len());
                let _ = w.write_all(&out);
            });
        }
    });
    Server { base, seen, claude }
}

pub fn zip_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, data) in files {
        z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(data).unwrap();
    }
    z.finish().unwrap().into_inner()
}

fn gz(s: &str) -> Vec<u8> {
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    e.write_all(s.as_bytes()).unwrap();
    e.finish().unwrap()
}

fn write(p: &Path, b: &[u8]) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, b).unwrap();
}

fn tree(dir: &Path) -> Vec<(String, Vec<u8>)> {
    install::files_in(dir).into_iter().map(|f| (f.clone(), std::fs::read(dir.join(&f)).unwrap())).collect()
}

pub struct Fixture {
    pub _t: tempfile::TempDir,
    pub hub: Arc<ModHub>,
    pub server: Server,
    pub steam: PathBuf,
    pub roaming: PathBuf,
}

impl Fixture {
    pub fn game_dir(&self, name: &str) -> PathBuf {
        self.steam.join("steamapps").join("common").join(name)
    }
}

fn pkg(full: &str, version: &str, deps: &[&str], base: &str, downloads: u64) -> Value {
    let (owner, name) = full.split_once('-').unwrap();
    json!({
        "name": name, "full_name": full, "owner": owner, "package_url": format!("https://thunderstore.io/p/{owner}/{name}/"), "date_updated": "2026-10-01T00:00:00Z",
        "rating_score": 1, "categories": ["Mods"],
        "versions": [{ "version_number": version, "description": format!("{name} mod"), "dependencies": deps, "download_url": format!("{base}/dl/{full}/{version}/"), "downloads": downloads }]
    })
}

pub fn fixture() -> Fixture {
    let t = tempfile::tempdir().unwrap();
    let steam = t.path().join("Steam");
    let roaming = t.path().join("Roaming");
    std::fs::create_dir_all(&roaming).unwrap();
    let apps = steam.join("steamapps");
    let manifest = |id: u32, name: &str, dir: &str| write(&apps.join(format!("appmanifest_{id}.acf")), format!("\"AppState\" {{ \"appid\" \"{id}\" \"name\" \"{name}\" \"installdir\" \"{dir}\" }}").as_bytes());

    // Balatro: a fused LÖVE program, and an old Lovely (version.dll).
    manifest(BALATRO, "Balatro", "Balatro");
    let b = apps.join("common/Balatro");
    let mut exe = b"MZ program ".to_vec();
    exe.extend(zip_of(&[("card.lua", b"function Card:set_ability(center)\n  self.ability = {}\nend\n"), ("main.lua", b"love.load = function() end\n")]));
    write(&b.join("Balatro.exe"), &exe);
    write(&b.join("love.dll"), b"love");
    write(&b.join("lua51.dll"), b"lua");
    write(&b.join("version.dll"), b"old lovely");
    // Lethal Company: Unity (Mono).
    manifest(1_966_720, "Lethal Company", "Lethal Company");
    let lc = apps.join("common/Lethal Company");
    write(&lc.join("Lethal Company.exe"), b"MZ");
    write(&lc.join("UnityPlayer.dll"), b"u");
    write(&lc.join("Lethal Company_Data/globalgamemanagers"), b"\x00\x00\x00\x002022.3.9f1\x00\x00 data");
    write(&lc.join("Lethal Company_Data/Managed/Assembly-CSharp.dll"), b"a");
    // GTA V Enhanced with BattlEye.
    manifest(GTA_ENHANCED, "Grand Theft Auto V Enhanced", "Grand Theft Auto V Enhanced");
    let gta = apps.join("common/Grand Theft Auto V Enhanced");
    write(&gta.join("GTA5_Enhanced.exe"), b"MZ");
    write(&gta.join("PlayGTAV.exe"), b"MZ");
    write(&gta.join("BattlEye/BEClient_x64.dll"), b"be");
    // Counter-Strike 2: online competitive.
    manifest(730, "Counter-Strike 2", "Counter-Strike Global Offensive");
    write(&apps.join("common/Counter-Strike Global Offensive/cs2.exe"), b"MZ");

    let lovely_zip = zip_of(&[("winmm.dll", b"lovely 0.10"), ("lovely/config.toml", b"[workspace]"), ("manifest.json", b"{}"), ("icon.png", b"png"), ("README.md", b"readme")]);
    let smods_zip = zip_of(&[("manifest.json", b"{}"), ("icon.png", b"png"), ("lovely/core.toml", b"[manifest]"), ("src/core.lua", b"SMODS = {}")]);
    let amulet_zip = zip_of(&[("manifest.json", b"{}"), ("Amulet.lua", b"-- big numbers")]);
    let cryptid_zip = zip_of(&[("manifest.json", b"{}"), ("Cryptid-main/Cryptid.lua", b"-- jokers"), ("Cryptid-main/lovely/patch.toml", b"[manifest]")]);
    let bep_zip = zip_of(&[("manifest.json", b"{}"), ("icon.png", b"png"), ("BepInExPack/winhttp.dll", b"doorstop"), ("BepInExPack/doorstop_config.ini", b"[General]"), ("BepInExPack/BepInEx/core/BepInEx.dll", b"bep")]);
    let more_zip = zip_of(&[("manifest.json", b"{}"), ("icon.png", b"png"), ("README.md", b"r"), ("MoreCompany.dll", b"more")]);
    let shvdn_zip = zip_of(&[("ScriptHookVDotNet.asi", b"asi"), ("ScriptHookVDotNet2.dll", b"v2"), ("ScriptHookVDotNet3.dll", b"v3"), ("ScriptHookVDotNet.ini", b"[x]"), ("README.md", b"r")]);
    let shvdn_sha = {
        use sha2::Digest;
        hex::encode(sha2::Sha256::digest(&shvdn_zip))
    };

    let base_cell: Arc<Mutex<String>> = Arc::default();
    let bc = base_cell.clone();
    let server = serve(Box::new(move |r| {
        let base = bc.lock().clone();
        let p = r.path.as_str();
        let zip = |b: &Vec<u8>| Some((200, "application/zip", b.clone()));
        match p {
            "/api/experimental/schema/dev/latest/" => Some((200, "application/json", crate::mods::thunderstore::tests::SCHEMA.as_bytes().to_vec())),
            "/c/balatro/api/v1/package-listing-index/" => Some((200, "application/octet-stream", gz(&json!([format!("{base}/chunk/balatro")]).to_string()))),
            "/c/lethal-company/api/v1/package-listing-index/" => Some((200, "application/octet-stream", gz(&json!([format!("{base}/chunk/lc")]).to_string()))),
            "/chunk/balatro" => Some((200, "application/octet-stream", gz(&json!([
                pkg("Steamodded-Steamodded", "26.1002.0", &["Thunderstore-lovely-0.9.0"], &base, 9000),
                pkg("just_frostice482-Amulet", "3.5.4", &["Steamodded-Steamodded-1.0.0"], &base, 5000),
                pkg("spectralpack-Cryptid", "0.5.17", &["just_frostice482-Amulet-3.5.4", "Steamodded-Steamodded-1.1814.0"], &base, 6000),
                pkg("Thunderstore-lovely", "0.10.0", &[], &base, 10000),
            ]).to_string()))),
            "/chunk/lc" => Some((200, "application/octet-stream", gz(&json!([
                pkg("BepInEx-BepInExPack", "5.4.2305", &[], &base, 900000),
                pkg("notnotnotswipez-MoreCompany", "1.11.0", &["BepInEx-BepInExPack-5.4.2100"], &base, 100000),
            ]).to_string()))),
            "/api/experimental/package/Thunderstore/lovely/" => Some((200, "application/json", json!({ "total_downloads": 1, "latest": { "version_number": "0.10.0", "download_url": format!("{base}/dl/Thunderstore-lovely/0.10.0/"), "dependencies": [] } }).to_string().into_bytes())),
            "/dl/Thunderstore-lovely/0.10.0/" => zip(&lovely_zip),
            "/dl/Steamodded-Steamodded/26.1002.0/" => zip(&smods_zip),
            "/dl/just_frostice482-Amulet/3.5.4/" => zip(&amulet_zip),
            "/dl/spectralpack-Cryptid/0.5.17/" => zip(&cryptid_zip),
            "/dl/BepInEx-BepInExPack/5.4.2305/" => zip(&bep_zip),
            "/dl/notnotnotswipez-MoreCompany/1.11.0/" => zip(&more_zip),
            "/gh/repos/scripthookvdotnet/scripthookvdotnet-nightly/releases/latest" => Some((200, "application/json", json!({ "tag_name": "v3.7.0-nightly.193", "assets": [{ "name": "ScriptHookVDotNet.zip", "browser_download_url": format!("{base}/gh/dl/shvdn.zip"), "digest": format!("sha256:{shvdn_sha}") }] }).to_string().into_bytes())),
            "/gh/dl/shvdn.zip" => zip(&shvdn_zip),
            _ => None,
        }
    }));
    *base_cell.lock() = server.base.clone();

    let events = crate::events::EventBus::new();
    let procs = Arc::new(crate::system::procs::ProcessMonitor::new());
    let data = t.path().join("data");
    let games = crate::games::GameHub::new(&data, events.clone(), procs.clone());
    games.set_roblox_roots(vec![]);
    games.set_test_paths(vec![], crate::games::library::Roots { steam: Some(steam.clone()), ..Default::default() });
    let ep = Endpoints { thunderstore: server.base.clone(), github: format!("{}/gh", server.base), steam_api: server.base.clone(), claude: server.base.clone(), modrinth: server.base.clone(), fabric_meta: server.base.clone(), quilt_meta: server.base.clone(), pollinations_gen: server.base.clone(), pollinations_enter: server.base.clone(), roaming: Some(roaming.clone()), home: Some(roaming.join("home")) };
    let hub = ModHub::with(&data, events, games, procs, ep);
    hub.refresh_schema().unwrap();
    Fixture { _t: t, hub, server, steam, roaming }
}

#[test]
fn every_game_is_described_with_how_it_mods() {
    let f = fixture();
    let games = f.hub.games();
    let g = |name: &str| games.iter().find(|g| g.name == name).unwrap().clone();
    let b = g("Balatro");
    assert_eq!((b.method, b.thunderstore.as_deref(), b.ts_installs, b.maker.kind, b.engine), (Method::Lovely, Some("balatro"), true, Some(MakerKind::Steamodded), Engine::Love));
    assert_eq!(b.data_dir.as_deref(), Some(f.roaming.join("Balatro").to_string_lossy().as_ref()));
    assert_eq!(b.loaders.iter().map(|l| (l.kind, l.installed)).collect::<Vec<_>>(), [(LoaderKind::Lovely, true)], "the old version.dll counts as Lovely");
    let lc = g("Lethal Company");
    assert_eq!((lc.method, lc.ts_installs, lc.maker.kind), (Method::BepInEx, true, Some(MakerKind::BepInExPlugin)));
    let gta = g("Grand Theft Auto V Enhanced");
    assert_eq!((gta.method, gta.anti_cheat.clone(), gta.maker.kind), (Method::Gta, vec!["BattlEye".to_string()], Some(MakerKind::ShvdnScript)));
    assert!(gta.notes.iter().any(|n| n.text.contains("Story Mode")) && gta.notes.iter().any(|n| n.link.as_deref() == Some(SCRIPT_HOOK_V)));
    assert_eq!(gta.loaders.iter().map(|l| (l.kind, l.auto)).collect::<Vec<_>>(), [(LoaderKind::ScriptHookV, false), (LoaderKind::ScriptHookVDotNet, true)]);
    let cs = g("Counter-Strike 2");
    assert!(cs.online_only && cs.method == Method::None && !cs.maker.available);
    assert!(f.hub.install_file(&cs.key, "/nonexistent").is_err());
    assert!(cs.image.unwrap().contains("/730/"));
}

#[test]
fn balatro_mods_install_with_steamodded_and_lovely_and_come_out_cleanly() {
    let f = fixture();
    let bdir = f.game_dir("Balatro");
    let before = tree(&bdir);
    let key = format!("steam:{BALATRO}");

    let br = f.hub.browse(&key, "", None, thunderstore::Sort::Downloads, 0, 20).unwrap();
    // The list comes in the background the first time.
    let br = if br.loading {
        let start = Instant::now();
        loop {
            let b = f.hub.browse(&key, "", None, thunderstore::Sort::Downloads, 0, 20).unwrap();
            if !b.loading || start.elapsed() > Duration::from_secs(10) {
                break b;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    } else {
        br
    };
    assert_eq!(br.packages.iter().map(|p| (p.package.name.as_str(), p.is_loader)).collect::<Vec<_>>(), [("lovely", true), ("Steamodded", false), ("Cryptid", false), ("Amulet", false)]);

    let done = f.hub.install_package(&key, "spectralpack-Cryptid").unwrap();
    assert_eq!(done.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["Steamodded", "Amulet", "Cryptid"]);
    assert!(done[0].dependency && !done[2].dependency);
    let mods = f.roaming.join("Balatro/Mods");
    assert!(mods.join("spectralpack-Cryptid/Cryptid.lua").is_file(), "the zip's own folder is dropped");
    assert!(mods.join("spectralpack-Cryptid/lovely/patch.toml").is_file());
    assert!(mods.join("Steamodded-Steamodded/src/core.lua").is_file());
    // Lovely was already there (the old version.dll), so it wasn't reinstalled.
    assert!(!bdir.join("winmm.dll").exists());
    assert!(f.hub.install_package(&key, "spectralpack-Cryptid").unwrap_err().contains("already"));

    let list = f.hub.mods(&key);
    let cryptid = list.iter().find(|m| m.name == "Cryptid").unwrap().clone();
    f.hub.set_enabled(&cryptid.id, false).unwrap();
    assert!(!mods.join("spectralpack-Cryptid/Cryptid.lua").exists());
    f.hub.set_enabled(&cryptid.id, true).unwrap();
    assert!(mods.join("spectralpack-Cryptid/Cryptid.lua").is_file());

    // Removing everything leaves the game and its mod folder as they were.
    for m in f.hub.mods(&key) {
        f.hub.uninstall(&m.id).unwrap();
    }
    assert_eq!(tree(&bdir), before);
    assert!(install::files_in(&mods).is_empty());
    assert!(f.hub.mods(&key).is_empty());
}

#[test]
fn lethal_company_gets_bepinex_and_its_mods_in_the_right_folders() {
    let f = fixture();
    let dir = f.game_dir("Lethal Company");
    let before = tree(&dir);
    let key = "steam:1966720";
    f.hub.load_listing("lethal-company").unwrap();
    let done = f.hub.install_package(key, "notnotnotswipez-MoreCompany").unwrap();
    assert_eq!(done.len(), 1);
    assert_eq!(std::fs::read(dir.join("winhttp.dll")).unwrap(), b"doorstop", "BepInEx came first, from the game's own pack");
    assert!(dir.join("BepInEx/core/BepInEx.dll").is_file());
    assert!(dir.join("BepInEx/plugins/notnotnotswipez-MoreCompany/MoreCompany.dll").is_file());
    let all = f.hub.mods(key);
    assert_eq!(all[0].loader, Some(LoaderKind::BepInEx), "loaders list first");
    let g = f.hub.games().into_iter().find(|g| g.key == key).unwrap();
    assert!(g.loaders[0].installed && g.loaders[0].managed);
    assert_eq!((g.mods, g.enabled_mods), (1, 1));

    // Switch everything off (to play vanilla) and on again.
    assert_eq!(f.hub.set_all(key, false).unwrap(), 2);
    assert_eq!(tree(&dir), before);
    assert_eq!(f.hub.set_all(key, true).unwrap(), 2);
    assert!(dir.join("BepInEx/plugins/notnotnotswipez-MoreCompany/MoreCompany.dll").is_file());
    for m in f.hub.mods(key) {
        f.hub.uninstall(&m.id).unwrap();
    }
    assert_eq!(tree(&dir), before);
}

#[test]
fn gta_takes_script_hook_v_from_a_file_and_shvdn_from_github() {
    let f = fixture();
    let dir = f.game_dir("Grand Theft Auto V Enhanced");
    let before = tree(&dir);
    let key = format!("steam:{GTA_ENHANCED}");
    let t = tempfile::tempdir().unwrap();
    let shv = t.path().join("ScriptHookV_1.0.3586.0.zip");
    std::fs::write(&shv, zip_of(&[("bin/ScriptHookV.dll", b"shv"), ("bin/dinput8.dll", b"asi loader"), ("bin/NativeTrainer.asi", b"trainer"), ("readme.txt", b"r")])).unwrap();
    let m = f.hub.install_file(&key, shv.to_str().unwrap()).unwrap();
    assert_eq!((m.loader, m.name.as_str()), (Some(LoaderKind::ScriptHookV), "Script Hook V"));
    assert!(dir.join("ScriptHookV.dll").is_file() && dir.join("dinput8.dll").is_file() && !dir.join("readme.txt").exists());

    // A script mod: SHVDN comes from GitHub first (checksum checked).
    let script = t.path().join("SlowMo.zip");
    std::fs::write(&script, zip_of(&[("SlowMo/SlowMo.cs", b"class SlowMo : GTA.Script {}"), ("SlowMo/SlowMo.ini", b"[keys]")])).unwrap();
    f.hub.install_file(&key, script.to_str().unwrap()).unwrap();
    assert!(dir.join("ScriptHookVDotNet.asi").is_file() && dir.join("ScriptHookVDotNet3.dll").is_file());
    assert!(dir.join("scripts/SlowMo.cs").is_file() && dir.join("scripts/SlowMo.ini").is_file());
    let g = f.hub.games().into_iter().find(|g| g.key == key).unwrap();
    assert!(g.loaders.iter().all(|l| l.installed));

    // Online play: everything off, the folder as it was.
    f.hub.set_all(&key, false).unwrap();
    assert_eq!(tree(&dir), before);
    f.hub.set_all(&key, true).unwrap();
    for m in f.hub.mods(&key) {
        f.hub.uninstall(&m.id).unwrap();
    }
    assert_eq!(tree(&dir), before);
}

/// A scripted Claude turn.
fn turn(blocks: Vec<Value>, stop: &str) -> String {
    let mut ev = vec![json!({"type":"message_start","message":{"model":"claude-opus-5-5","usage":{"input_tokens":1000,"cache_read_input_tokens":0,"output_tokens":1}}})];
    for (i, b) in blocks.into_iter().enumerate() {
        match b["type"].as_str().unwrap() {
            "tool_use" => {
                let input = b["input"].clone();
                ev.push(json!({"type":"content_block_start","index":i,"content_block":{"type":"tool_use","id":b["id"],"name":b["name"],"input":{}}}));
                ev.push(json!({"type":"content_block_delta","index":i,"delta":{"type":"input_json_delta","partial_json":input.to_string()}}));
            }
            "thinking" => {
                ev.push(json!({"type":"content_block_start","index":i,"content_block":{"type":"thinking","thinking":"","signature":""}}));
                ev.push(json!({"type":"content_block_delta","index":i,"delta":{"type":"thinking_delta","thinking":b["thinking"]}}));
                ev.push(json!({"type":"content_block_delta","index":i,"delta":{"type":"signature_delta","signature":"sig"}}));
            }
            _ => {
                ev.push(json!({"type":"content_block_start","index":i,"content_block":{"type":"text","text":""}}));
                ev.push(json!({"type":"content_block_delta","index":i,"delta":{"type":"text_delta","text":b["text"]}}));
            }
        }
        ev.push(json!({"type":"content_block_stop","index":i}));
    }
    ev.push(json!({"type":"message_delta","delta":{"stop_reason":stop,"stop_details":null},"usage":{"output_tokens":500}}));
    ev.push(json!({"type":"message_stop"}));
    sse(&ev)
}

fn wait_idle(hub: &Arc<ModHub>, id: &str) -> maker::Project {
    let start = Instant::now();
    loop {
        let v = hub.maker.view(id).unwrap();
        if v.project.status != maker::Status::Working && v.project.status != maker::Status::Waiting {
            return v.project;
        }
        assert!(start.elapsed() < Duration::from_secs(20), "Claude never finished: {:?}", v.items);
        std::thread::sleep(Duration::from_millis(30));
    }
}

fn claude_bodies(s: &Server) -> Vec<Value> {
    s.seen.lock().iter().filter(|r| r.path.starts_with("/v1/messages")).map(|r| serde_json::from_slice(&r.body).unwrap()).collect()
}

#[test]
fn claude_makes_a_balatro_mod_that_installs_with_steamodded() {
    let f = fixture();
    let key = format!("steam:{BALATRO}");
    assert!(f.hub.maker.create(&f.hub, &key, "a joker").unwrap_err().contains("API key"));
    f.hub.claude.set_test_key("sk-ant-test-key");
    // Lovely is the old version.dll here; the mod should bring the new one and Steamodded.
    std::fs::remove_file(f.game_dir("Balatro").join("version.dll")).unwrap();
    {
        let mut q = f.server.claude.lock();
        q.push_back(turn(vec![
            json!({"type":"thinking","thinking":"Checking how cards get their abilities."}),
            json!({"type":"text","text":"Let me look at the game first."}),
            json!({"type":"tool_use","id":"t1","name":"list_files","input":{"path":"game/Balatro.exe!/"}}),
            json!({"type":"tool_use","id":"t2","name":"read_file","input":{"path":"game/Balatro.exe!/card.lua"}}),
            json!({"type":"tool_use","id":"t3","name":"write_file","input":{"path":"mod/LuckyJoker.json","content":"{\"id\":\"lucky\",\"main_file\":\"main.lua\"}"}}),
            json!({"type":"tool_use","id":"t4","name":"write_file","input":{"path":"mod/main.lua","content":"SMODS.Joker{ key = 'lucky' }\n"}}),
            json!({"type":"tool_use","id":"t5","name":"write_file","input":{"path":"game/card.lua","content":"x"}}),
            json!({"type":"tool_use","id":"t6","name":"write_file","input":{"path":"mod/evil.dll","content":"MZ"}}),
        ], "tool_use"));
        q.push_back(turn(vec![json!({"type":"tool_use","id":"t7","name":"finish","input":{"name":"Lucky Joker","summary":"Adds a lucky joker.","how_to_use":"Find it in the shop."}})], "tool_use"));
        q.push_back(turn(vec![json!({"type":"text","text":"Done. Press Install, then play."})], "end_turn"));
    }
    let p = f.hub.maker.create(&f.hub, &key, "Make a joker that gives +4 mult").unwrap();
    let p = wait_idle(&f.hub, &p.id);
    assert_eq!((p.ready, p.version, p.name.as_str(), p.summary.as_str(), p.kind), (true, 1, "Lucky Joker", "Adds a lucky joker.", MakerKind::Steamodded));
    assert!(p.cost > 0.0 && p.usage.input == 3000);

    let bodies = claude_bodies(&f.server);
    assert_eq!(bodies.len(), 3);
    let b0 = &bodies[0];
    assert_eq!(b0["model"], "claude-opus-5-5");
    assert_eq!(b0["thinking"], json!({"type":"adaptive","display":"updates"}));
    assert_eq!(b0["output_config"]["effort"], "medium");
    assert_eq!(b0["fallbacks"], "default");
    assert_eq!(b0["cache_control"], json!({"type":"ephemeral"}));
    assert!(b0["stream"].as_bool().unwrap());
    let tools = b0["tools"].as_array().unwrap();
    assert!(tools.iter().filter(|t| t.get("input_schema").is_some()).all(|t| t["eager_input_streaming"] == true));
    assert!(tools.iter().any(|t| t["type"] == "web_search_20260209") && !tools.iter().any(|t| t["name"] == "build"));
    assert!(b0["system"][0]["text"].as_str().unwrap().contains("Steamodded"));
    assert!(b0["system"][0]["text"].as_str().unwrap().contains("Never make cheats"));
    let req = f.server.seen.lock().iter().find(|r| r.path.starts_with("/v1/messages")).unwrap().clone();
    let h = |k: &str| req.headers.iter().find(|(x, _)| x == k).map(|(_, v)| v.clone()).unwrap_or_default();
    assert_eq!(h("x-api-key"), "sk-ant-test-key");
    assert_eq!(h("anthropic-version"), "2023-06-01");
    assert_eq!(h("anthropic-beta"), "server-side-fallback-2026-07-01,thinking-display-updates-2026-08-18");
    // Append-only: each request carries the previous one's messages unchanged.
    for w in bodies.windows(2) {
        let (a, b) = (w[0]["messages"].as_array().unwrap(), w[1]["messages"].as_array().unwrap());
        assert_eq!(&b[..a.len()], &a[..]);
        assert_eq!(b.len(), a.len() + 2);
    }
    let m1 = bodies[1]["messages"].as_array().unwrap();
    assert_eq!(m1[1]["content"][0], json!({"type":"thinking","thinking":"Checking how cards get their abilities.","signature":"sig"}), "thinking goes back exactly");
    let results = m1[2]["content"].as_array().unwrap();
    assert!(results[0]["content"].as_str().unwrap().contains("card.lua"));
    assert!(results[1]["content"].as_str().unwrap().contains("set_ability"));
    assert_eq!(results[1]["is_error"], false);
    assert!(results[4]["content"].as_str().unwrap().contains("Only files under mod/") && results[4]["is_error"] == true);
    assert!(results[5]["is_error"] == true, "programs can't be written");
    assert!(!f.game_dir("Balatro").join("card.lua").exists());

    let v = f.hub.maker.view(&p.id).unwrap();
    assert_eq!(v.files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["LuckyJoker.json", "main.lua"]);
    let kinds: Vec<&str> = v.items.iter().map(|i| i.kind.as_str()).collect();
    assert_eq!(kinds[..3], ["user", "note", "text"]);
    assert!(v.items.iter().any(|i| i.text == "Read game/Balatro.exe!/card.lua"));

    // Install: Lovely and Steamodded come with it.
    let p = f.hub.maker.install(&f.hub, &p.id).unwrap();
    assert_eq!(p.installed_version, 1);
    assert!(f.roaming.join("Balatro/Mods/Lucky_Joker/main.lua").is_file());
    assert!(f.roaming.join("Balatro/Mods/Steamodded-Steamodded/src/core.lua").is_file());
    assert_eq!(std::fs::read(f.game_dir("Balatro").join("winmm.dll")).unwrap(), b"lovely 0.10");
    let made = f.hub.mods(&key).into_iter().find(|m| m.name == "Lucky Joker").unwrap();
    assert_eq!(made.source, ModSource::Claude { project: p.id.clone() });

    // A refusal leaves the conversation as it was; the next message joins the unanswered one.
    let before = f.hub.maker.view(&p.id).unwrap();
    f.server.claude.lock().push_back(sse(&[
        json!({"type":"message_start","message":{"model":"claude-opus-5-5","usage":{"input_tokens":10,"output_tokens":1}}}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Partial"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"message_delta","delta":{"stop_reason":"refusal","stop_details":{"type":"refusal","category":null}},"usage":{"output_tokens":3}}),
        json!({"type":"message_stop"}),
    ]));
    f.hub.maker.send(&f.hub, &p.id, "now something else").unwrap();
    wait_idle(&f.hub, &p.id);
    let after = f.hub.maker.view(&p.id).unwrap();
    assert!(after.items.last().unwrap().text.contains("declined"));
    assert_eq!(after.items.len(), before.items.len() + 2);
    f.server.claude.lock().push_back(turn(vec![json!({"type":"text","text":"Sure."})], "end_turn"));
    f.hub.maker.send(&f.hub, &p.id, "ok, a blue joker").unwrap();
    wait_idle(&f.hub, &p.id);
    let last = claude_bodies(&f.server).pop().unwrap();
    let msgs = last["messages"].as_array().unwrap();
    let tail = msgs.last().unwrap();
    assert_eq!(tail["role"], "user");
    assert_eq!(tail["content"].as_array().unwrap().iter().map(|c| c["text"].as_str().unwrap()).collect::<Vec<_>>(), ["now something else", "ok, a blue joker"]);
}

#[test]
fn claude_builds_a_unity_plugin_only_when_the_user_approves() {
    let f = fixture();
    let key = "steam:1966720";
    f.hub.claude.set_test_key("sk-ant-test-key");
    f.hub.load_listing("lethal-company").unwrap();
    let built: Arc<Mutex<u32>> = Arc::default();
    let b2 = built.clone();
    *f.hub.maker.builder.lock() = Arc::new(move |proj: &Path, out: &Path| {
        *b2.lock() += 1;
        assert!(proj.ends_with("LuckyLoot.csproj"));
        std::fs::create_dir_all(out).unwrap();
        std::fs::write(out.join("LuckyLoot.dll"), b"plugin").unwrap();
        std::fs::write(out.join("BepInEx.dll"), b"reference").unwrap();
        Ok("Build succeeded. 0 Warning(s)".into())
    });
    let csproj = r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><TargetFramework>netstandard2.1</TargetFramework></PropertyGroup><ItemGroup><PackageReference Include="BepInEx.Core" Version="5.*" /></ItemGroup></Project>"#;
    {
        let mut q = f.server.claude.lock();
        q.push_back(turn(vec![
            json!({"type":"tool_use","id":"a","name":"write_file","input":{"path":"mod/LuckyLoot.csproj","content":csproj}}),
            json!({"type":"tool_use","id":"b","name":"write_file","input":{"path":"mod/Plugin.cs","content":"[BepInPlugin(\"lucky\",\"Lucky\",\"1.0\")] class P : BaseUnityPlugin {}"}}),
            json!({"type":"tool_use","id":"c","name":"build","input":{}}),
        ], "tool_use"));
        q.push_back(turn(vec![json!({"type":"tool_use","id":"d","name":"build","input":{}})], "tool_use"));
        q.push_back(turn(vec![json!({"type":"tool_use","id":"e","name":"finish","input":{"name":"Lucky Loot","summary":"More loot.","how_to_use":"Play."}})], "tool_use"));
        q.push_back(turn(vec![json!({"type":"text","text":"Ready."})], "end_turn"));
    }
    let p = f.hub.maker.create(&f.hub, key, "More scrap spawns").unwrap();
    assert_eq!(p.kind, MakerKind::BepInExPlugin);
    let wait_for_question = |not: &str| {
        let start = Instant::now();
        loop {
            let v = f.hub.maker.view(&p.id).unwrap();
            if let Some(a) = v.project.approval.filter(|a| a.id != not && v.project.status == maker::Status::Waiting) {
                return a;
            }
            assert!(start.elapsed() < Duration::from_secs(20));
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    let first = wait_for_question("");
    assert!(first.what.contains("dotnet build"));
    f.hub.maker.approve(&p.id, false).unwrap();
    assert!(wait_for_question(&first.id).what.contains("dotnet build"), "Claude asks again");
    assert_eq!(*built.lock(), 0, "nothing was built without a yes");
    f.hub.maker.approve(&p.id, true).unwrap();
    let p = wait_idle(&f.hub, &p.id);
    assert_eq!(*built.lock(), 1);
    assert!(p.ready);
    let bodies = claude_bodies(&f.server);
    assert!(bodies[0]["tools"].as_array().unwrap().iter().any(|t| t["name"] == "build"));
    assert!(bodies[0]["system"][0]["text"].as_str().unwrap().contains("2022.3.9f1"), "the Unity version is read from the game");
    let declined = &bodies[1]["messages"].as_array().unwrap()[2]["content"].as_array().unwrap()[2];
    assert!(declined["content"].as_str().unwrap().contains("didn't approve") && declined["is_error"] == true);

    f.hub.maker.install(&f.hub, &p.id).unwrap();
    let dir = f.game_dir("Lethal Company");
    assert!(dir.join("BepInEx/core/BepInEx.dll").is_file(), "BepInEx came with it");
    assert_eq!(std::fs::read(dir.join("BepInEx/plugins/Lucky_Loot/LuckyLoot.dll")).unwrap(), b"plugin");
    assert!(!dir.join("BepInEx/plugins/Lucky_Loot/BepInEx.dll").exists(), "only the mod's own file");
}
