//! The server host end to end: made from pretend Mojang, Paper and Fabric
//! servers, run as a pretend Minecraft (this test program, see
//! `process::tests::fake_server`), and — when asked — as a real one.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::*;
use crate::mods::tests::{serve, Seen};

fn sha1(b: &[u8]) -> String {
    use sha1::Digest as _;
    hex::encode(sha1::Sha1::digest(b))
}

fn sha256(b: &[u8]) -> String {
    use sha2::Digest as _;
    hex::encode(sha2::Sha256::digest(b))
}

pub struct Fix {
    pub _t: tempfile::TempDir,
    pub hub: Arc<ServerHub>,
    pub web: crate::mods::tests::Server,
    pub data: PathBuf,
}

const VANILLA: &[u8] = b"pretend vanilla 26.3 server jar";
const PAPER: &[u8] = b"pretend paper 26.3 build 207";
const FABRIC: &[u8] = b"pretend fabric launcher";

/// A hub talking to pretend Mojang, Paper, Fabric and Modrinth sites.
pub fn fix() -> Fix {
    let t = tempfile::tempdir().unwrap();
    let base: Arc<parking_lot::Mutex<String>> = Arc::default();
    let b2 = base.clone();
    let web = serve(Box::new(move |r: &Seen| {
        let b = b2.lock().clone();
        let ok = |v: Value| Some((200, "application/json", v.to_string().into_bytes()));
        let jar = |x: &[u8]| Some((200, "application/java-archive", x.to_vec()));
        match r.path.as_str() {
            "/mc/game/version_manifest_v2.json" => ok(json!({ "latest": { "release": "26.3" }, "versions": [
                { "id": "26.3", "type": "release", "url": format!("{b}/v/26.3.json"), "releaseTime": "2026-09-15T00:00:00Z" },
                { "id": "26.3-rc-1", "type": "snapshot", "url": format!("{b}/v/rc.json"), "releaseTime": "2026-09-10T00:00:00Z" },
                { "id": "1.21.11", "type": "release", "url": format!("{b}/v/1.21.11.json"), "releaseTime": "2025-12-01T00:00:00Z" },
            ] })),
            "/v/26.3.json" => ok(json!({ "id": "26.3", "javaVersion": { "majorVersion": 25 }, "downloads": { "server": { "url": format!("{b}/jars/server.jar"), "sha1": sha1(VANILLA) } } })),
            "/jars/server.jar" => jar(VANILLA),
            "/v3/projects/paper/versions/26.3/builds" => ok(json!([{ "id": 207, "channel": "STABLE", "downloads": { "server:default": { "name": "paper-26.3-207.jar", "url": format!("{b}/jars/paper.jar"), "checksums": { "sha256": sha256(PAPER) } } } }])),
            "/v3/projects/paper/versions/26.3" => ok(json!({ "version": { "id": "26.3", "java": { "version": { "minimum": 25 }, "flags": { "recommended": ["-XX:+UseG1GC", "-XX:G1HeapRegionSize=8M"] } } } })),
            "/v3/projects/paper" => ok(json!({ "versions": { "26.3": ["26.3", "26.3-rc-3"], "1.21": ["1.21.11", "1.21.10"] } })),
            "/jars/paper.jar" => jar(PAPER),
            "/v2/versions/loader/26.3" => ok(json!([{ "loader": { "version": "0.19.5", "stable": true } }, { "loader": { "version": "0.19.4", "stable": true } }])),
            "/v2/versions/installer" => ok(json!([{ "version": "1.1.2", "stable": true }])),
            "/v2/versions/loader/26.3/0.19.5/1.1.2/server/jar" => jar(FABRIC),
            "/v2/versions/game" => ok(json!([{ "version": "26.3", "stable": true }, { "version": "26.3-rc-1", "stable": false }])),
            "/users/profiles/minecraft/Alex" => ok(json!({ "id": "a1b2c3d4e5f647a8b9c0d1e2f3a4b5c6", "name": "Alex" })),
            _ => None,
        }
    }));
    *base.lock() = web.base.clone();
    let data = t.path().join("data");
    let events = EventBus::new();
    let procs = Arc::new(crate::system::procs::ProcessMonitor::new());
    let games = crate::games::GameHub::new(&data, events.clone(), procs.clone());
    let mods = crate::mods::ModHub::with(&data, events.clone(), games, procs, crate::mods::Endpoints { modrinth: web.base.clone(), fabric_meta: web.base.clone(), quilt_meta: web.base.clone(), roaming: Some(t.path().join("Roaming")), home: Some(t.path().join("home")), ..Default::default() });
    let w = &web.base;
    let ep = Endpoints {
        web: install::Endpoints { piston: w.clone(), fill: w.clone(), fabric_meta: w.clone(), quilt_meta: w.clone(), quilt_maven: w.clone(), forge_maven: w.clone(), forge_files: w.clone(), neo_maven: w.clone(), adoptium: w.clone() },
        mojang_api: w.clone(),
        ipify: format!("{w}/ip"),
        ssdp: "127.0.0.1:9".into(),
    };
    let hub = ServerHub::with(&data, events, mods, ep);
    // Java is "there" (the pretend server doesn't need it).
    let java = hub.runtimes.dir.join("25-jre").join("bin");
    std::fs::create_dir_all(&java).unwrap();
    std::fs::write(java.join(if cfg!(windows) { "java.exe" } else { "java" }), b"").unwrap();
    *hub.launcher.lock() = Some(Arc::new(|s: &Server, _java: &Path| process::tests::fake_command(Path::new(&s.dir))));
    Fix { _t: t, hub, web, data }
}

fn wait(what: &str, secs: u64, mut f: impl FnMut() -> bool) {
    let until = Instant::now() + Duration::from_secs(secs);
    while !f() {
        assert!(Instant::now() < until, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(30));
    }
}

fn state(h: &ServerHub, id: &str) -> State {
    h.view(id).unwrap().state
}

fn new(name: &str, flavor: Flavor) -> NewServer {
    NewServer { name: name.into(), flavor, mc: "26.3".into(), loader: String::new(), memory_mb: Some(1024), port: None, eula: true, online_mode: false, max_players: Some(8), motd: None, whitelist: false, seed: None, gamemode: None, difficulty: Some("hard".into()), from: None, world: None, start: false }
}

#[test]
fn makes_vanilla_paper_and_fabric_servers() {
    let f = fix();
    let h = &f.hub;
    assert!(h.create(NewServer { eula: false, ..new("No", Flavor::Vanilla) }).unwrap_err().contains("EULA"));
    assert_eq!(h.versions(Flavor::Vanilla).unwrap(), ["26.3", "1.21.11"]);
    assert_eq!(h.versions(Flavor::Paper).unwrap(), ["26.3", "1.21.11", "1.21.10"]);
    assert_eq!(h.versions(Flavor::Fabric).unwrap(), ["26.3"]);

    let v = h.create(new("Survival World", Flavor::Vanilla)).unwrap();
    assert_eq!(v.state, State::Installing);
    wait("vanilla install", 20, || state(h, &v.server.id) == State::Stopped);
    let s = h.server(&v.server.id).unwrap();
    assert_eq!((s.launch.clone(), s.java_major, s.memory_mb), (Some(Launch::Jar { jar: "server.jar".into() }), 25, 1024));
    let dir = PathBuf::from(&s.dir);
    assert_eq!(std::fs::read(dir.join("server.jar")).unwrap(), VANILLA);
    assert!(std::fs::read_to_string(dir.join("eula.txt")).unwrap().contains("eula=true"));
    let p = Props::load(&dir.join("server.properties"));
    assert_eq!((p.get("server-port"), p.get("max-players"), p.get("online-mode"), p.get("difficulty"), p.get("motd")), (Some("25565"), Some("8"), Some("false"), Some("hard"), Some("Survival World")));

    let paper = h.create(new("Plugins", Flavor::Paper)).unwrap();
    wait("paper install", 20, || state(h, &paper.server.id) == State::Stopped);
    let ps = h.server(&paper.server.id).unwrap();
    assert_eq!((ps.loader.as_str(), ps.flags.clone()), ("207", vec!["-XX:+UseG1GC".to_string(), "-XX:G1HeapRegionSize=8M".to_string()]));
    assert_eq!(std::fs::read(Path::new(&ps.dir).join("paper.jar")).unwrap(), PAPER);
    assert_eq!(Props::load(&Path::new(&ps.dir).join("server.properties")).get("server-port"), Some("25566"), "the next free port");

    let fab = h.create(new("Modded", Flavor::Fabric)).unwrap();
    wait("fabric install", 20, || state(h, &fab.server.id) == State::Stopped);
    let fs = h.server(&fab.server.id).unwrap();
    assert_eq!(fs.loader, "0.19.5");
    let fd = PathBuf::from(&fs.dir);
    assert_eq!(std::fs::read(fd.join(".fabric/server/26.3-server.jar")).unwrap(), VANILLA, "Mojang's jar, checked, where Fabric looks for it");
    assert_eq!(std::fs::read(fd.join("fabric-server-launch.jar")).unwrap(), FABRIC);
    assert_eq!(detect(&fd).map(|d| (d.0, d.1)), Some((Flavor::Fabric, "26.3".into())));
    assert_eq!(h.list().len(), 3);
    // A download that doesn't match its checksum is refused.
    let mut bad = Props::default();
    bad.set("x", "y");
    assert!(h.web.fetch(&format!("{}/jars/server.jar", f.web.base), &install::Sum::Sha1("0".repeat(40)), &f.data.join("x.jar"), &mut |_, _, _| {}).unwrap_err().contains("checksum"));
    assert!(!f.data.join("x.jar").exists() && !f.data.join("x.part").exists());
    h.delete(&fab.server.id, true).unwrap();
    assert!(!fd.exists());
}

/// A modpack Prism Launcher installed becomes a server: its loader and
/// version, its server-side mods (from this PC), settings and picture.
#[test]
fn makes_a_server_from_another_launchers_modpack() {
    let f = fix();
    let h = &f.hub;
    let inst = f._t.path().join("Roaming/PrismLauncher/instances/Friends");
    let game = inst.join(".minecraft");
    std::fs::create_dir_all(game.join("mods")).unwrap();
    std::fs::create_dir_all(game.join("config")).unwrap();
    std::fs::write(inst.join("instance.cfg"), "name=Friends pack\niconKey=friends\n").unwrap();
    std::fs::write(inst.join("mmc-pack.json"), r#"{"components":[{"uid":"net.minecraft","version":"26.3"},{"uid":"net.fabricmc.fabric-loader","version":"0.19.5"}]}"#).unwrap();
    let icons = f._t.path().join("Roaming/PrismLauncher/icons");
    std::fs::create_dir_all(&icons).unwrap();
    std::fs::write(icons.join("friends.png"), crate::mods::cover::png(&image::RgbaImage::from_pixel(32, 32, image::Rgba([1, 2, 3, 255])))).unwrap();
    let lithium = crate::mods::tests::zip_of(&[("fabric.mod.json", br#"{"id":"lithium","environment":"*"}"#)]);
    let zoom = crate::mods::tests::zip_of(&[("fabric.mod.json", br#"{"id":"zoomify","environment":"client"}"#)]);
    std::fs::write(game.join("mods/lithium.jar"), &lithium).unwrap();
    std::fs::write(game.join("mods/zoomify.jar"), &zoom).unwrap();
    std::fs::write(game.join("config/lithium.properties"), b"mixin.ai=true").unwrap();
    std::fs::write(game.join("options.txt"), b"fov:0.5").unwrap();

    let packs = h.mods.minecraft.launcher_packs();
    assert_eq!(packs.len(), 1);
    let v = h.create(NewServer { from: Some(FromPack::Launcher { id: packs[0].id.clone() }), mc: String::new(), ..new("Friends", Flavor::Vanilla) }).unwrap();
    wait("install from the pack", 20, || state(h, &v.server.id) == State::Stopped);
    let s = h.server(&v.server.id).unwrap();
    assert_eq!((s.flavor, s.mc.as_str(), s.loader.as_str(), s.from.as_deref()), (Flavor::Fabric, "26.3", "0.19.5", Some("Prism Launcher: Friends pack")));
    let d = PathBuf::from(&s.dir);
    assert_eq!(std::fs::read(d.join("mods/lithium.jar")).unwrap(), lithium);
    assert!(!d.join("mods/zoomify.jar").exists(), "client-only mods stay off");
    assert_eq!(std::fs::read(d.join("config/lithium.properties")).unwrap(), b"mixin.ai=true");
    assert!(!d.join("options.txt").exists());
    assert!(d.join("server-icon.png").is_file());
    assert!(std::fs::read(game.join("mods/zoomify.jar")).is_ok(), "the launcher's own files are left alone");
    assert!(h.create(NewServer { from: Some(FromPack::Launcher { id: "0".repeat(16) }), ..new("Gone", Flavor::Vanilla) }).unwrap_err().contains("isn't there"));

    // A world played alone in that pack becomes the server's world, and the
    // single player's things become the owner's.
    use nbt::Tag;
    let player = Tag::Compound(vec![("XpLevel".into(), Tag::Int(42)), ("Inventory".into(), Tag::List(vec![Tag::Compound(vec![("id".into(), Tag::Str("minecraft:diamond".into())), ("count".into(), Tag::Int(9))])]))]);
    let level = Tag::Compound(vec![("Data".into(), Tag::Compound(vec![("LevelName".into(), Tag::Str("Our base".into())), ("GameType".into(), Tag::Int(1)), ("Difficulty".into(), Tag::Byte(1)), ("hardcore".into(), Tag::Byte(0)), ("Player".into(), player.clone()), ("Version".into(), Tag::Compound(vec![("Name".into(), Tag::Str("26.3".into()))]))]))]);
    let save = game.join("saves").join("Base");
    std::fs::create_dir_all(save.join("region")).unwrap();
    std::fs::write(save.join("level.dat"), nbt::gzip(&nbt::write(&level))).unwrap();
    std::fs::write(save.join("region").join("r.0.0.mca"), vec![7u8; 8192]).unwrap();
    std::fs::write(save.join("session.lock"), b"x").unwrap();
    let pack = packs[0].id.clone();
    let world = |folder: &str, owner: Option<&str>| Some(WorldFrom { pack: pack.clone(), folder: folder.into(), owner: owner.map(String::from) });
    assert!(h.create(NewServer { from: Some(FromPack::Launcher { id: pack.clone() }), world: world("../Base", None), ..new("Bad", Flavor::Vanilla) }).unwrap_err().contains("isn't there"));
    assert!(h.create(NewServer { from: Some(FromPack::Launcher { id: pack.clone() }), world: world("Base", Some("not a name!")), ..new("Bad", Flavor::Vanilla) }).unwrap_err().contains("letters"));
    let v = h.create(NewServer { from: Some(FromPack::Launcher { id: pack.clone() }), world: world("Base", Some("Steve")), mc: String::new(), ..new("Base", Flavor::Vanilla) }).unwrap();
    wait("install with the world", 20, || state(h, &v.server.id) == State::Stopped);
    let s = h.server(&v.server.id).unwrap();
    assert_eq!(s.from.as_deref(), Some("Prism Launcher: Friends pack (world Our base)"));
    let d = PathBuf::from(&s.dir);
    assert_eq!(std::fs::read(d.join("world").join("region").join("r.0.0.mca")).unwrap(), vec![7u8; 8192]);
    assert!(!d.join("world").join("session.lock").exists());
    let p = Props::load(&d.join("server.properties"));
    assert_eq!((p.get("gamemode"), p.get("difficulty"), p.get("hardcore")), (Some("creative"), Some("easy"), Some("false")));
    let mine = d.join("world").join("playerdata").join(format!("{}.dat", players::offline_uuid("Steve")));
    assert_eq!(nbt::read(&std::fs::read(mine).unwrap()).unwrap(), player, "Steve gets the single player's things");
    assert!(save.join("session.lock").exists(), "the pack's own world is left as it was");
    // A world on its own (no modpack) runs vanilla on the version it was played in.
    let alone = h.create(NewServer { world: world("Base", None), mc: String::new(), ..new("Alone", Flavor::Fabric) }).unwrap();
    let a = h.server(&alone.server.id).unwrap();
    assert_eq!((a.flavor, a.mc.as_str()), (Flavor::Vanilla, "26.3"));
    wait("install the lone world", 20, || state(h, &alone.server.id) == State::Stopped);
}

#[test]
fn runs_players_backups_and_crashes() {
    let f = fix();
    let h = &f.hub;
    let v = h.create(new("Friends", Flavor::Vanilla)).unwrap();
    let id = v.server.id.clone();
    wait("install", 20, || state(h, &id) == State::Stopped);
    let dir = PathBuf::from(&h.server(&id).unwrap().dir);

    // Whitelist and operators while it's stopped go into the files (offline mode: worked-out UUIDs,
    // and OmniHub holds the operators).
    h.act(&id, "Alex", Action::Whitelist { on: true }).unwrap();
    h.act(&id, "Alex", Action::Op).unwrap();
    assert_eq!(players::read_list(&dir, List::Whitelist)[0].uuid, players::offline_uuid("Alex"));
    assert!(players::read_list(&dir, List::Ops).is_empty());
    assert_eq!(guard::load(&dir).ops[0].level, Some(4));
    assert!(h.act(&id, "Alex", Action::Heal).unwrap_err().contains("Start the server"));
    assert!(h.act(&id, "bad name!", Action::Op).is_err());
    let pv = h.players(&id).unwrap();
    assert!(pv.known.iter().any(|k| k.name == "Alex" && k.op && k.whitelisted && !k.online));
    // Settings.
    h.set_properties(&id, &[("max-players".into(), "12".into()), ("motd".into(), "Hello".into())]).unwrap();
    assert_eq!(h.view(&id).unwrap().max_players, 12);
    assert!(h.set_properties(&id, &[("server-port".into(), "80".into())]).is_err());
    assert!(h.set_properties(&id, &[("bad key".into(), "1".into())]).is_err());

    h.start(&id).unwrap();
    assert!(h.start(&id).is_err(), "only once");
    wait("start", 30, || state(h, &id) == State::Running);
    assert!(h.console(&id, None).iter().any(|l| l.text.contains("Done (")));
    h.command_line(&id, "join Steve").unwrap();
    wait("Steve online", 10, || h.view(&id).unwrap().online.iter().any(|o| o.name == "Steve"));
    // Where Steve is and what he carries, asked of the server.
    let pl = h.player(&id, "steve").unwrap();
    assert!(pl.live && pl.online);
    assert_eq!(pl.details.pos, Some([10.5, 70.0, -3.25]));
    assert_eq!(pl.details.items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(), ["Diamond Sword", "Oak Log", "Diamond Helmet"]);
    h.watch(&id);
    h.poll_positions(&id);
    let on = h.view(&id).unwrap().online;
    assert_eq!((on[0].pos, on[0].dimension.as_deref(), on[0].uuid.as_deref()), (Some([10.5, 70.0, -3.25]), Some("minecraft:overworld"), Some("5627dd98-e6be-3c21-b8a8-e92344183641")));
    assert!(!h.console(&id, None).iter().any(|l| l.text.contains("entity data")), "OmniHub's own questions stay out of the console");
    // Commands to players are checked first.
    assert!(h.act(&id, "Steve", Action::Give { item: "diamond 64; op Steve".into(), count: 1 }).is_err());
    assert!(h.act(&id, "Steve", Action::Teleport { x: f64::NAN, y: 0.0, z: 0.0, dimension: None }).is_err());
    assert!(!h.act(&id, "Steve", Action::Give { item: "minecraft:diamond".into(), count: 3 }).unwrap().is_empty());
    // A backup while it runs: saving pauses, the world is zipped, saving resumes.
    std::fs::create_dir_all(dir.join("world")).unwrap();
    std::fs::write(dir.join("world/level.dat"), b"lvl").unwrap();
    let b = h.backup_now(&id, false).unwrap();
    assert!(h.backups(&id).unwrap().iter().any(|x| x.name == b.name));
    assert!(h.restore(&id, &b.name).unwrap_err().contains("Stop"));
    // A crash: restarted (the hub's clock is turned forward here).
    h.command_line(&id, "crash").unwrap();
    wait("crashed", 10, || state(h, &id) == State::Crashed);
    assert!(h.view(&id).unwrap().online.is_empty());
    h.with_live(&id, |l| l.restart_at = Some(Instant::now()));
    h.tick(0);
    wait("restarted", 30, || state(h, &id) == State::Running);
    // Stopped cleanly.
    h.stop(&id).unwrap();
    wait("stopped", 20, || state(h, &id) == State::Stopped);
    assert!(dir.join("stopped-cleanly").exists());
    assert!(h.console(&id, None).iter().any(|l| l.text.contains("The server stopped.")));
    h.restore(&id, &b.name).unwrap();
    // Three crashes in ten minutes: no more restarts.
    h.with_live(&id, |l| l.crashes = vec![now(), now()]);
    h.start(&id).unwrap();
    wait("start again", 30, || state(h, &id) == State::Running);
    h.command_line(&id, "crash").unwrap();
    wait("crashed again", 10, || state(h, &id) == State::Crashed);
    assert!(h.live.lock().get(&id).unwrap().restart_at.is_none());
    assert!(h.view(&id).unwrap().problem.unwrap().contains("3 times"));
}

/// Players without a paid account (offline mode): an operator's name is an
/// operator only from places the owner allowed, and switching back to
/// checked names gives the operators and everyone's things back.
#[test]
fn guards_operators_when_names_arent_checked() {
    let f = fix();
    let h = &f.hub;
    let mut guards = h.events.subscribe();
    let id = h.create(new("Open", Flavor::Vanilla)).unwrap().server.id;
    wait("install", 20, || state(h, &id) == State::Stopped);
    let dir = PathBuf::from(&h.server(&id).unwrap().dir);
    let ops = || players::read_list(&dir, List::Ops).into_iter().map(|o| o.name).collect::<Vec<_>>();
    h.act(&id, "Alex", Action::Op).unwrap();
    h.start(&id).unwrap();
    wait("start", 30, || state(h, &id) == State::Running);
    assert!(h.console(&id, None).iter().any(|l| l.text.contains("without a paid Minecraft account can join")));

    // Alex's name from somewhere new: not an operator, kicked, waiting for the owner.
    h.command_line(&id, "join Alex 203.0.113.5").unwrap();
    wait("kept out", 10, || h.players(&id).unwrap().waiting.iter().any(|w| w.name == "Alex" && w.place == "203.0.113.5"));
    assert!(ops().is_empty());
    wait("kicked", 10, || h.console(&id, None).iter().any(|l| l.text.contains("Kicked Alex")));
    assert!(!guard::load(&dir).last.contains_key("alex"), "a kept-out login isn't where they play");
    let ev = std::iter::from_fn(|| guards.try_recv().ok()).find(|e| e.topic == "mcserver:guard").expect("the owner is told");
    assert_eq!((ev.payload["name"].as_str(), ev.payload["place"].as_str()), (Some("Alex"), Some("203.0.113.5")));
    // The owner says it's really Alex.
    assert!(h.act(&id, "Alex", Action::Allow { place: "203.0.113.5".into() }).unwrap().contains("can join"));
    assert!(h.players(&id).unwrap().waiting.is_empty());
    h.command_line(&id, "join Alex 203.0.113.5").unwrap();
    wait("operator from an allowed place", 10, || ops() == ["Alex"]);
    h.command_line(&id, "leave Alex").unwrap();
    wait("not an operator once gone", 10, || ops().is_empty());
    // On this PC itself.
    h.command_line(&id, "join Alex 127.0.0.1").unwrap();
    wait("operator here", 10, || ops() == ["Alex"]);
    // The same name from elsewhere replaces that session: no longer an operator, kicked.
    h.command_line(&id, "join Alex 198.51.100.9").unwrap();
    wait("replaced and kicked", 10, || ops().is_empty() && h.players(&id).unwrap().waiting.iter().any(|w| w.place == "198.51.100.9"));
    assert!(h.act(&id, "Alex", Action::Refuse { place: "198.51.100.9".into() }).is_ok());
    assert!(h.players(&id).unwrap().waiting.is_empty());

    // An operator made in the game joins the guard when they leave.
    h.command_line(&id, "join Steve 192.0.2.4").unwrap();
    wait("Steve on", 10, || h.view(&id).unwrap().online.iter().any(|o| o.name == "Steve"));
    h.command_line(&id, "player-op Steve").unwrap();
    wait("in-game operator", 10, || ops().contains(&"Steve".to_string()));
    h.command_line(&id, "leave Steve").unwrap();
    wait("held by OmniHub", 10, || !ops().contains(&"Steve".to_string()) && guard::load(&dir).is_op("Steve"));
    assert!(guard::load(&dir).may_join("Steve", "192.0.2.4"), "from where they were playing");
    assert!(h.players(&id).unwrap().known.iter().any(|k| k.name == "Steve" && k.op));
    // Operators typed into the console go through the guard too.
    h.command_line(&id, "op Bob").unwrap();
    assert!(guard::load(&dir).is_op("Bob") && !ops().contains(&"Bob".to_string()));
    h.command_line(&id, "/deop Bob").unwrap();
    assert!(!guard::load(&dir).is_op("Bob"));

    // Names checked again: operators and lists go back, with Mojang's UUIDs.
    h.stop(&id).unwrap();
    wait("stopped", 20, || state(h, &id) == State::Stopped);
    h.act(&id, "Alex", Action::Whitelist { on: true }).unwrap();
    std::fs::create_dir_all(dir.join("world/players/data")).unwrap();
    std::fs::write(dir.join(format!("world/players/data/{}.dat", players::offline_uuid("Alex"))), b"alex's things").unwrap();
    h.set_properties(&id, &[("online-mode".into(), "true".into())]).unwrap();
    h.start(&id).unwrap();
    wait("start checked", 30, || state(h, &id) == State::Running);
    let mut back = ops();
    back.sort();
    assert_eq!(back, ["Alex", "Steve"]);
    let alex = "a1b2c3d4-e5f6-47a8-b9c0-d1e2f3a4b5c6";
    assert!(players::read_list(&dir, List::Ops).iter().any(|o| o.name == "Alex" && o.uuid == alex), "looked up");
    assert_eq!(players::read_list(&dir, List::Whitelist)[0].uuid, alex);
    assert_eq!(std::fs::read(dir.join(format!("world/players/data/{alex}.dat"))).unwrap(), b"alex's things");
    // Checked names: operators are plain operators again.
    h.command_line(&id, "join Alex 198.51.100.9").unwrap();
    wait("joined", 10, || h.view(&id).unwrap().online.iter().any(|o| o.name == "Alex"));
    assert!(h.players(&id).unwrap().waiting.is_empty());
    assert!(ops().contains(&"Alex".to_string()));
    h.stop(&id).unwrap();
    wait("stopped again", 20, || state(h, &id) == State::Stopped);
}

#[test]
fn startup_flags_are_jvm_tuning_only() {
    for ok in ["-XX:+UseG1GC", "-XX:MaxGCPauseMillis=200", "-Dusing.aikars.flags=https://mcflags.emc.gs", "--add-opens=java.base/java.lang=ALL-UNNAMED", "-Xss4M"] {
        assert!(flag_ok(ok), "{ok}");
    }
    for bad in ["-XX:OnOutOfMemoryError=calc.exe", "-XX:OnError=cmd", "-XX:VMOptionsFile=x.txt", "-XX:Flags=x", "-Xbootclasspath/a:evil.jar", "-javaagent:x.jar", "-agentpath:x.dll", "@args.txt", "-Dx=@y", "-jar", "-Dx=1 -Dy=2"] {
        assert!(!flag_ok(bad), "{bad}");
    }
}

#[test]
fn shutdown_stops_every_server() {
    let f = fix();
    let h = &f.hub;
    let a = h.create(new("A", Flavor::Vanilla)).unwrap().server.id;
    let b = h.create(new("B", Flavor::Vanilla)).unwrap().server.id;
    wait("installs", 20, || state(h, &a) == State::Stopped && state(h, &b) == State::Stopped);
    h.start(&a).unwrap();
    h.start(&b).unwrap();
    wait("both running", 30, || state(h, &a) == State::Running && state(h, &b) == State::Running);
    assert_eq!(h.running_count(), 2);
    h.shutdown(Duration::from_secs(20));
    wait("both stopped", 20, || h.running_count() == 0);
    for id in [&a, &b] {
        assert!(Path::new(&h.server(id).unwrap().dir).join("stopped-cleanly").exists());
        assert_eq!(state(h, id), State::Stopped, "a stop OmniHub asked for isn't a crash");
    }
}

/// A real Minecraft server, downloaded from Mojang and run with the Java in
/// OMNIHUB_MC_JAVA (a folder with bin/java, version 25). With OMNIHUB_MC_BOT
/// (a mineflayer script), a bot joins and is looked at.
#[cfg(unix)]
#[test]
#[ignore]
fn real_server() {
    let Ok(java_home) = std::env::var("OMNIHUB_MC_JAVA") else { return };
    let t = tempfile::tempdir().unwrap();
    let data = t.path().join("data");
    let events = EventBus::new();
    let procs = Arc::new(crate::system::procs::ProcessMonitor::new());
    let games = crate::games::GameHub::new(&data, events.clone(), procs.clone());
    let mods = crate::mods::ModHub::new(&data, events.clone(), games, procs);
    let h = ServerHub::new(&data, events, mods);
    let rt = h.runtimes.dir.join("25-jre");
    std::fs::create_dir_all(rt.parent().unwrap()).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&java_home, &rt).unwrap();
    let flavor = match std::env::var("OMNIHUB_MC_FLAVOR").as_deref() {
        Ok("paper") => Flavor::Paper,
        Ok("fabric") => Flavor::Fabric,
        Ok("neoforge") => Flavor::NeoForge,
        Ok("forge") => Flavor::Forge,
        Ok("quilt") => Flavor::Quilt,
        _ => Flavor::Vanilla,
    };
    let mc = std::env::var("OMNIHUB_MC_VERSION").unwrap_or_else(|_| "26.3".into());
    let port = 25700 + flavor as u16;
    let v = h.create(NewServer { name: "Real".into(), flavor, mc, loader: String::new(), memory_mb: Some(1536), port: Some(port), eula: true, online_mode: false, max_players: Some(5), motd: None, whitelist: false, seed: Some("omnihub".into()), gamemode: None, difficulty: None, from: None, world: None, start: true }).unwrap();
    let id = v.server.id.clone();
    wait("running", 600, || {
        let v = h.view(&id).unwrap();
        if let Some(p) = &v.problem {
            if matches!(v.state, State::Stopped | State::Crashed) {
                for l in h.console(&id, None).iter().rev().take(15).collect::<Vec<_>>().into_iter().rev() {
                    println!("{}", l.text);
                }
                panic!("{p}");
            }
        }
        v.state == State::Running
    });
    println!("running: {:?}", h.view(&id).unwrap().server.launch);
    println!("pack format {:?}", pack_format(Path::new(&h.server(&id).unwrap().dir)));
    if let Ok(bot) = std::env::var("OMNIHUB_MC_BOT") {
        let mut b = std::process::Command::new("node").arg(&bot).arg(port.to_string()).arg("Steve").spawn().unwrap();
        wait("bot online", 60, || h.view(&id).unwrap().online.iter().any(|o| o.name == "Steve"));
        let r = h.act(&id, "Steve", Action::Give { item: "minecraft:diamond_pickaxe".into(), count: 1 }).unwrap();
        println!("give: {r}");
        let p = h.player(&id, "Steve").unwrap();
        println!("player: {:?} in {:?}, {} items: {:?}", p.details.pos, p.details.dimension, p.details.items.len(), p.details.items.iter().map(|i| &i.label).collect::<Vec<_>>());
        assert!(p.live && p.details.items.iter().any(|i| i.id == "minecraft:diamond_pickaxe"));
        h.watch(&id);
        h.poll_positions(&id);
        println!("online: {:?}", h.view(&id).unwrap().online);
        println!("tp: {}", h.act(&id, "Steve", Action::Teleport { x: 5.0, y: 120.0, z: 5.0, dimension: None }).unwrap());
        println!("op: {}", h.act(&id, "Steve", Action::Op).unwrap());
        println!("kick: {}", h.act(&id, "Steve", Action::Kick { reason: Some("Bye".into()) }).unwrap());
        let _ = b.kill();
        let _ = b.wait();
    }
    // A data pack, put on the running server the way Claude's are, and reloaded.
    if let Some(fmt) = pack_format(Path::new(&h.server(&id).unwrap().dir)) {
        let (maj, min) = fmt.split_once('.').unwrap_or((&fmt, "0"));
        let dp = t.path().join("datapack");
        std::fs::create_dir_all(dp.join("data/omnihub/function")).unwrap();
        std::fs::create_dir_all(dp.join("data/minecraft/tags/function")).unwrap();
        std::fs::write(dp.join("pack.mcmeta"), format!(r#"{{"pack":{{"description":"Made in a test","min_format":[{maj},{min}],"max_format":[{maj},{min}]}}}}"#)).unwrap();
        std::fs::write(dp.join("data/omnihub/function/hello.mcfunction"), "say Hello from a data pack OmniHub put on\n").unwrap();
        std::fs::write(dp.join("data/minecraft/tags/function/load.json"), r#"{"values":["omnihub:hello"]}"#).unwrap();
        let c = h.add_made(&id, Kind::Datapack, &dp, "Hello pack", "test").unwrap();
        println!("data pack at {}", c.file);
        wait("data pack loaded", 60, || h.console(&id, None).iter().any(|l| l.text.contains("Hello from a data pack OmniHub put on")));
        h.command_line(&id, "datapack list enabled").unwrap();
        std::thread::sleep(Duration::from_secs(2));
        println!("{}", h.console(&id, None).iter().rev().take(3).map(|l| l.text.clone()).collect::<Vec<_>>().join("\n"));
    }
    // A Paper plugin, built the way Claude's are (OMNIHUB_MC_JDK: a JDK 25 folder).
    if let (Ok(jdk), Flavor::Paper) = (std::env::var("OMNIHUB_MC_JDK"), flavor) {
        std::os::unix::fs::symlink(&jdk, h.runtimes.dir.join("25-jdk")).unwrap();
        let (javac, cp) = h.javac(&id).unwrap();
        println!("javac {} with {} jars", javac.display(), cp.len());
        let m = t.path().join("plugin/mod");
        std::fs::create_dir_all(m.join("src/dev/omnihub/hello")).unwrap();
        std::fs::write(m.join("plugin.yml"), "name: Hello\nversion: '1'\nmain: dev.omnihub.hello.Hello\napi-version: '26.1'\n").unwrap();
        std::fs::write(m.join("src/dev/omnihub/hello/Hello.java"), "package dev.omnihub.hello;\nimport org.bukkit.plugin.java.JavaPlugin;\npublic final class Hello extends JavaPlugin {\n  @Override public void onEnable() { getLogger().info(\"Hello from a plugin OmniHub built\"); }\n}\n").unwrap();
        let classes = t.path().join("plugin/build/classes");
        std::fs::create_dir_all(&classes).unwrap();
        let log = crate::mods::maker::javac_build(&javac, &cp, &m.join("src"), &classes, 25).unwrap();
        println!("javac: {}", log.trim());
        let jar = t.path().join("plugin/build/plugin.jar");
        crate::mods::maker::package_plugin(&classes, &m, &jar).unwrap();
        let c = h.add_made(&id, Kind::Plugin, &jar, "Hello", "test").unwrap();
        println!("installed {}", c.file);
        assert!(h.view(&id).unwrap().restart_needed);
        h.restart(&id).unwrap();
        std::thread::sleep(Duration::from_secs(2));
        wait("restarted", 300, || state(&h, &id) == State::Running);
        let said = h.console(&id, None).into_iter().filter(|l| l.text.contains("Hello")).map(|l| l.text).collect::<Vec<_>>();
        println!("{}", said.join("\n"));
        assert!(said.iter().any(|l| l.contains("Hello from a plugin OmniHub built")), "the plugin loaded");
    }
    let b = h.backup_now(&id, false).unwrap();
    println!("backup {} {} bytes", b.name, b.size);
    let info = h.map_info(&id, "overworld").unwrap();
    println!("map: {} regions, spawn {:?}", info.regions.len(), info.spawn);
    if let Some((x, z)) = info.regions.first() {
        let tile = h.map_tile(&id, "overworld", *x, *z).unwrap();
        println!("tile r.{x}.{z}: {} bytes", tile.len());
        if let Ok(out) = std::env::var("OMNIHUB_MC_MAP") {
            let png = base64::engine::general_purpose::STANDARD.decode(tile.trim_start_matches("data:image/png;base64,")).unwrap();
            std::fs::write(Path::new(&out).join(format!("{:?}-r.{x}.{z}.png", flavor)), png).unwrap();
        }
    }
    h.stop(&id).unwrap();
    wait("stopped", 120, || state(&h, &id) == State::Stopped);
    for l in h.console(&id, None).iter().rev().take(5) {
        println!("{}", l.text);
    }
}
