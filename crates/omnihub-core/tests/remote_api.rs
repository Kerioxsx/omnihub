//! End-to-end test of the phone companion server over real sockets:
//! pairing, browsing, ranged downloads, resumable uploads, power with a
//! countdown, notes, the event socket, screen frames and unpairing.

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use omnihub_core::core::{AppCore, CoreOptions};
use omnihub_core::paths::AppPaths;
use omnihub_core::vault::crypto::KdfParams;
use reqwest::StatusCode;
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

struct Fixture {
    _dir: tempfile::TempDir,
    core: Arc<AppCore>,
    base: String,
    share: std::path::PathBuf,
    incoming: std::path::PathBuf,
}

fn setup(tls: bool) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let paths = AppPaths::at(&dir.path().join("home")).unwrap();
    let core = AppCore::new(
        paths,
        CoreOptions { dry_run_power: true, vault_kdf: Some(KdfParams { m_cost: 256, t_cost: 1, p_cost: 1 }), vault_dpapi: Some(false), ..Default::default() },
    )
    .unwrap();
    let share = dir.path().join("share");
    let incoming = dir.path().join("incoming");
    std::fs::create_dir_all(share.join("photos")).unwrap();
    std::fs::write(share.join("hello.txt"), "hello from the PC, this is a test file").unwrap();
    image::RgbImage::from_pixel(800, 600, image::Rgb([200, 30, 30])).save(share.join("photos/red.png")).unwrap();
    std::fs::write(dir.path().join("secret.txt"), "not shared").unwrap();
    let port = free_port();
    core.update_settings(&json!({
        "remote": {
            "enabled": true, "port": port, "tls": tls, "bind": "localhost",
            "browseScope": "custom", "customRoots": [share.to_string_lossy()],
            "incomingDir": incoming.to_string_lossy(), "powerDelaySeconds": 30, "allowControl": false
        },
        "vault": { "allowPhone": true },
        "notes": { "claudeFolder": dir.path().join("claude").to_string_lossy() }
    }))
    .unwrap();
    assert!(core.remote.is_running(), "{:?}", core.remote.status());
    let scheme = if tls { "https" } else { "http" };
    Fixture { base: format!("{scheme}://127.0.0.1:{port}"), core, share, incoming, _dir: dir }
}

fn client() -> reqwest::Client {
    let _ = rustls::crypto::ring::default_provider().install_default();
    reqwest::Client::builder().danger_accept_invalid_certs(true).no_proxy().timeout(Duration::from_secs(20)).build().unwrap()
}

#[test]
fn companion_server_end_to_end() {
    let fx = setup(false);
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let c = client();
        let base = &fx.base;

        let info: Value = c.get(format!("{base}/api/info")).send().await.unwrap().json().await.unwrap();
        assert_eq!(info["paired"], false);
        assert_eq!(info["features"]["vault"], false, "no vault yet and no TLS");

        // The phone app shell is served.
        let page = c.get(format!("{base}/")).send().await.unwrap();
        assert_eq!(page.status(), StatusCode::OK);
        assert!(page.headers()["content-security-policy"].to_str().unwrap().contains("default-src 'self'"));

        // Pairing is closed until the desktop opens it.
        let r = c.post(format!("{base}/api/pair")).json(&json!({ "pin": "123456", "deviceName": "Pixel" })).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        let pairing = fx.core.remote.begin_pairing().unwrap();
        assert!(pairing.qr_svg.contains("<svg"));
        let r = c.post(format!("{base}/api/pair")).json(&json!({ "pin": "000000x", "deviceName": "Pixel" })).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        let r: Value = c.post(format!("{base}/api/pair")).json(&json!({ "pin": pairing.pin, "deviceName": "Pixel 9" })).send().await.unwrap().json().await.unwrap();
        let token = r["token"].as_str().unwrap().to_string();
        let auth = format!("Bearer {token}");

        // Unauthenticated API calls are refused.
        assert_eq!(c.get(format!("{base}/api/fs/roots")).send().await.unwrap().status(), StatusCode::UNAUTHORIZED);
        let me: Value = c.get(format!("{base}/api/me")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        assert_eq!(me["name"], "Pixel 9");

        // Browsing stays inside the shared folders.
        let roots: Value = c.get(format!("{base}/api/fs/roots")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        let root_paths: Vec<String> = roots["roots"].as_array().unwrap().iter().map(|r| r["path"].as_str().unwrap().to_string()).collect();
        assert!(root_paths.iter().any(|p| p == &fx.share.to_string_lossy()));
        let list: Value = c.get(format!("{base}/api/fs/list")).query(&[("path", fx.share.to_string_lossy().to_string())]).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        let names: Vec<&str> = list["entries"].as_array().unwrap().iter().map(|e| e["name"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["photos", "hello.txt"], "folders first");
        let outside = fx.share.parent().unwrap().join("secret.txt");
        let r = c.get(format!("{base}/api/fs/list")).query(&[("path", fx.share.join("..").to_string_lossy().to_string())]).header("authorization", &auth).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        let r = c.post(format!("{base}/api/files/ticket")).json(&json!({ "path": outside })).header("authorization", &auth).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);

        // Thumbnails for images.
        let thumb = c.get(format!("{base}/api/fs/thumb")).query(&[("path", fx.share.join("photos/red.png").to_string_lossy().to_string())]).header("authorization", &auth).send().await.unwrap();
        assert_eq!(thumb.headers()["content-type"], "image/jpeg");
        let img = image::load_from_memory(&thumb.bytes().await.unwrap()).unwrap();
        assert!(img.width() <= 256);

        // Downloads support Range (browsers resume with it).
        let t: Value = c.post(format!("{base}/api/files/ticket")).json(&json!({ "path": fx.share.join("hello.txt") })).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        let url = format!("{base}{}", t["url"].as_str().unwrap());
        let full = c.get(&url).send().await.unwrap();
        assert!(full.headers()["content-disposition"].to_str().unwrap().contains("hello.txt"));
        assert_eq!(full.text().await.unwrap(), "hello from the PC, this is a test file");
        let part = c.get(&url).header("range", "bytes=6-9").send().await.unwrap();
        assert_eq!(part.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(part.text().await.unwrap(), "from");
        assert_eq!(c.get(format!("{base}/dl/not-a-ticket")).send().await.unwrap().status(), StatusCode::NOT_FOUND);

        // Event socket.
        let ticket: Value = c.post(format!("{base}/api/ticket")).json(&json!({ "purpose": "socket" })).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        let ws_url = format!("{}/api/ws?ticket={}", base.replacen("http", "ws", 1), ticket["ticket"].as_str().unwrap());
        let (mut events, _) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

        // Resumable upload with checksums.
        let data: Vec<u8> = (0..2_500_000u32).map(|i| (i % 251) as u8).collect();
        let st: Value = c.post(format!("{base}/api/upload")).json(&json!({ "name": "video.mp4", "size": data.len() })).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        let id = st["id"].as_str().unwrap().to_string();
        let chunk = |off: usize, len: usize| data[off..(off + len).min(data.len())].to_vec();
        let first = chunk(0, 1_000_000);
        let r = c.put(format!("{base}/api/upload/{id}/chunk?offset=0")).header("authorization", &auth).header("x-chunk-crc32", format!("{:08x}", crc32fast::hash(&first))).body(first).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let bad = chunk(1_000_000, 1_000_000);
        let r = c.put(format!("{base}/api/upload/{id}/chunk?offset=1000000")).header("authorization", &auth).header("x-chunk-crc32", "deadbeef").body(bad).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::BAD_REQUEST, "corrupted chunk rejected");
        // "Reconnect": ask where to resume.
        let st: Value = c.get(format!("{base}/api/upload/{id}")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        let mut off = st["offset"].as_u64().unwrap() as usize;
        assert_eq!(off, 1_000_000);
        while off < data.len() {
            let ch = chunk(off, 1_000_000);
            let r: Value = c.put(format!("{base}/api/upload/{id}/chunk?offset={off}")).header("authorization", &auth).header("x-chunk-crc32", format!("{:08x}", crc32fast::hash(&ch))).body(ch).send().await.unwrap().json().await.unwrap();
            off = r["offset"].as_u64().unwrap() as usize;
        }
        let done: Value = c.post(format!("{base}/api/upload/{id}/finish")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        use sha2::Digest;
        assert_eq!(done["sha256"], hex::encode(sha2::Sha256::digest(&data)));
        assert_eq!(std::fs::read(fx.incoming.join("video.mp4")).unwrap(), data);

        // PC -> phone inbox, announced on the event socket.
        let offered = fx.core.remote.send_to_phone(&[fx.share.join("hello.txt").to_string_lossy().to_string()], None, &fx.core.outbox_dir()).unwrap();
        let mut saw = std::collections::HashSet::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !(saw.contains("inbox:new") && saw.contains("transfer:done")) && tokio::time::Instant::now() < deadline {
            if let Ok(Some(Ok(Message::Text(t)))) = tokio::time::timeout(Duration::from_millis(500), events.next()).await {
                let v: Value = serde_json::from_str(&t).unwrap();
                saw.insert(v["topic"].as_str().unwrap().to_string());
            }
        }
        assert!(saw.contains("inbox:new"), "{saw:?}");
        assert!(saw.contains("transfer:done"), "{saw:?}");
        let inbox: Value = c.get(format!("{base}/api/inbox")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        assert_eq!(inbox["items"][0]["id"], offered[0].id);

        // Text and links are listed with their text and have nothing to download.
        let text = fx.core.remote.send_text("https://example.com/x", None).unwrap();
        let inbox: Value = c.get(format!("{base}/api/inbox")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        assert_eq!(inbox["items"][0]["kind"], "text");
        assert_eq!(inbox["items"][0]["text"], "https://example.com/x");
        let r = c.post(format!("{base}/api/inbox/{}/ticket", text.id)).header("authorization", &auth).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        fx.core.remote.unsend(&text.id);

        // Folders are zipped before they are offered; the zip goes when the offer does.
        let folder = fx.core.remote.send_to_phone(&[fx.share.to_string_lossy().to_string()], None, &fx.core.outbox_dir()).unwrap();
        assert!(folder[0].name.ends_with(".zip"));
        let t: Value = c.post(format!("{base}/api/inbox/{}/ticket", folder[0].id)).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        let zip = c.get(format!("{base}{}", t["url"].as_str().unwrap())).send().await.unwrap().bytes().await.unwrap();
        assert_eq!(&zip[..2], b"PK");
        fx.core.remote.unsend(&folder[0].id);
        assert!(!folder[0].path.exists());

        // Power: destructive actions need confirmation and get a countdown.
        let r = c.post(format!("{base}/api/power")).json(&json!({ "action": "shutdown" })).header("authorization", &auth).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        let r: Value = c.post(format!("{base}/api/power")).json(&json!({ "action": "shutdown", "confirm": true })).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        assert_eq!(r["pending"]["action"], "shutdown");
        assert!(fx.core.power.pending().is_some());
        let r: Value = c.post(format!("{base}/api/power/cancel")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        assert_eq!(r["cancelled"], true);
        assert!(fx.core.power.pending().is_none());

        // Notes from the phone, exported to the Claude folder.
        let n: Value = c.post(format!("{base}/api/notes")).json(&json!({ "title": "Idea from the bus", "body": "Treemap zoom", "kind": "idea", "sendToClaude": true })).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        assert!(n["tags"].as_array().unwrap().iter().any(|t| t == "from-phone"));
        assert!(std::path::Path::new(n["exportedPath"].as_str().unwrap()).exists());

        // The vault needs HTTPS.
        fx.core.vault.create("a long master password").unwrap();
        let r = c.post(format!("{base}/api/vault/session")).json(&json!({ "password": "a long master password" })).header("authorization", &auth).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);

        // Screen sharing: a frame arrives, acks keep it flowing, stats follow.
        let ticket: Value = c.post(format!("{base}/api/ticket")).json(&json!({ "purpose": "screen" })).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        let url = format!("{}/api/screen/ws?ticket={}&preset=saver", base.replacen("http", "ws", 1), ticket["ticket"].as_str().unwrap());
        let (mut screen, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        let mut frames = 0;
        let mut stats = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
        while (frames < 5 || !stats) && tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(Duration::from_secs(2), screen.next()).await {
                Ok(Some(Ok(Message::Binary(b)))) => {
                    assert_eq!(b[0], 1);
                    let w = u16::from_le_bytes([b[6], b[7]]);
                    assert!(w <= 1280);
                    let ts = u32::from_le_bytes(b[16..20].try_into().unwrap());
                    let seq = u32::from_le_bytes(b[2..6].try_into().unwrap());
                    if frames == 0 {
                        let img = image::load_from_memory(&b[20..]).unwrap();
                        assert_eq!(img.width() as u16, w);
                    }
                    frames += 1;
                    screen.send(Message::Text(json!({ "t": "ack", "seq": seq, "ts": ts }).to_string().into())).await.unwrap();
                }
                Ok(Some(Ok(Message::Text(t)))) => {
                    let v: Value = serde_json::from_str(&t).unwrap();
                    if v["t"] == "stats" {
                        stats = true;
                    }
                }
                _ => {}
            }
        }
        assert!(frames >= 5, "got {frames} frames");
        assert!(stats);
        assert_eq!(fx.core.remote.status().viewers.len(), 1);
        screen.close(None).await.ok();
        drop(screen);

        // Unpairing revokes the token.
        let r = c.post(format!("{base}/api/unpair")).header("authorization", &auth).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::NO_CONTENT);
        assert_eq!(c.get(format!("{base}/api/me")).header("authorization", &auth).send().await.unwrap().status(), StatusCode::UNAUTHORIZED);

        let audit: Vec<String> = fx.core.audit.list(100, 0).into_iter().map(|e| e.action).collect();
        for a in ["remote.pair", "files.download", "files.upload", "power.shutdown", "power.cancel", "notes.create", "screen.start", "remote.unpair"] {
            assert!(audit.iter().any(|x| x == a), "missing audit entry {a}: {audit:?}");
        }
    });
    drop(rt);
    fx.core.remote.stop();
}

#[test]
fn https_and_phone_vault() {
    let fx = setup(true);
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let c = client();
        let base = &fx.base;
        let info: Value = c.get(format!("{base}/api/info")).send().await.unwrap().json().await.unwrap();
        assert_eq!(info["tls"], true);
        let pairing = fx.core.remote.begin_pairing().unwrap();
        assert!(pairing.urls[0].starts_with("https://"));
        let r: Value = c.post(format!("{base}/api/pair")).json(&json!({ "secret": pairing.secret, "deviceName": "iPhone" })).send().await.unwrap().json().await.unwrap();
        let auth = format!("Bearer {}", r["token"].as_str().unwrap());

        fx.core.vault.create("a long master password").unwrap();
        fx.core
            .vault
            .save(omnihub_core::vault::EntryInput { title: "Router".into(), password: Some("wifi-secret-123".into()), ..Default::default() })
            .unwrap();
        fx.core.vault.lock();

        let r = c.post(format!("{base}/api/vault/session")).json(&json!({ "password": "wrong" })).header("authorization", &auth).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        let s: Value = c.post(format!("{base}/api/vault/session")).json(&json!({ "password": "a long master password" })).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        let session = s["session"].as_str().unwrap().to_string();
        assert!(!fx.core.vault.is_unlocked(), "the phone session does not unlock the vault on the PC");
        let list: Value = c.get(format!("{base}/api/vault/entries")).header("authorization", &auth).header("x-vault-session", &session).send().await.unwrap().json().await.unwrap();
        assert_eq!(list["entries"][0]["title"], "Router");
        assert!(list["entries"][0].get("password").is_none(), "listing never includes secrets");
        let id = list["entries"][0]["id"].as_str().unwrap();
        let v: Value = c.post(format!("{base}/api/vault/reveal")).json(&json!({ "id": id, "field": "password" })).header("authorization", &auth).header("x-vault-session", &session).send().await.unwrap().json().await.unwrap();
        assert_eq!(v["value"], "wifi-secret-123");
        c.delete(format!("{base}/api/vault/session")).header("authorization", &auth).header("x-vault-session", &session).send().await.unwrap();
        let r = c.get(format!("{base}/api/vault/entries")).header("authorization", &auth).header("x-vault-session", &session).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::LOCKED);
    });
    drop(rt);
    fx.core.remote.stop();
}

/// Music from the phone: what's playing (the pretend player), artwork,
/// controls, synced lyrics and the live `media:state` event.
#[test]
fn music_from_the_phone() {
    let dir = tempfile::tempdir().unwrap();
    let core = AppCore::new(AppPaths::at(&dir.path().join("home")).unwrap(), CoreOptions { fake_media: true, vault_dpapi: Some(false), ..Default::default() }).unwrap();
    let port = free_port();
    core.update_settings(&json!({ "remote": { "enabled": true, "port": port, "tls": false, "bind": "localhost" } })).unwrap();
    core.media.start(|| false);
    let base = format!("http://127.0.0.1:{port}");
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let c = client();
        let pairing = core.remote.begin_pairing().unwrap();
        let r: Value = c.post(format!("{base}/api/pair")).json(&json!({ "pin": pairing.pin, "deviceName": "iPhone" })).send().await.unwrap().json().await.unwrap();
        let auth = format!("Bearer {}", r["token"].as_str().unwrap());
        let info: Value = c.get(format!("{base}/api/info")).send().await.unwrap().json().await.unwrap();
        assert_eq!(info["features"]["media"], true);

        tokio::time::sleep(Duration::from_millis(700)).await;
        let m: Value = c.get(format!("{base}/api/media")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        let st = &m["state"];
        assert_eq!((st["title"].as_str(), st["artist"].as_str(), st["playing"].as_bool()), (Some("Daylight Drive"), Some("The Test Signals"), Some(true)));
        assert_eq!(st["positionSource"], "player");
        assert!(m["nowMs"].as_i64().unwrap() > 0);
        assert!(m["audio"]["eq"]["status"]["available"].is_boolean());

        // Artwork by id.
        let art = c.get(format!("{base}/api/media/art")).query(&[("id", st["art"].as_str().unwrap())]).header("authorization", &auth).send().await.unwrap();
        assert_eq!(art.status(), StatusCode::OK);
        assert_eq!(art.headers()["content-type"], "image/png");
        assert!(art.bytes().await.unwrap().starts_with(b"\x89PNG"));
        assert_eq!(c.get(format!("{base}/api/media/art?id=nope")).header("authorization", &auth).send().await.unwrap().status(), StatusCode::NOT_FOUND);

        // Lyrics, timed.
        let l: Value = c.get(format!("{base}/api/media/lyrics")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        assert_eq!(l["lyrics"]["status"], "ready");
        assert_eq!(l["lyrics"]["lyrics"]["lines"][0]["ms"], 2000);

        // Pausing reaches the phone as an event.
        let t: Value = c.post(format!("{base}/api/ticket")).header("authorization", &auth).json(&json!({ "purpose": "socket" })).send().await.unwrap().json().await.unwrap();
        let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://127.0.0.1:{port}/api/ws?ticket={}", t["ticket"].as_str().unwrap())).await.unwrap();
        let r = c.post(format!("{base}/api/media/control")).header("authorization", &auth).json(&json!({ "action": "pause" })).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let paused = tokio::time::timeout(Duration::from_secs(5), async {
            while let Some(Ok(msg)) = ws.next().await {
                if let Message::Text(t) = msg {
                    let v: Value = serde_json::from_str(&t).unwrap();
                    if v["topic"] == "media:state" && v["payload"]["state"]["playing"] == false {
                        return v;
                    }
                }
            }
            panic!("socket closed");
        })
        .await
        .expect("a media:state event");
        assert!(paused["payload"]["nowMs"].as_i64().is_some());
        // Seek, next track.
        c.post(format!("{base}/api/media/control")).header("authorization", &auth).json(&json!({ "action": "seek", "positionMs": 60000 })).send().await.unwrap();
        c.post(format!("{base}/api/media/control")).header("authorization", &auth).json(&json!({ "action": "next" })).send().await.unwrap();
        tokio::time::sleep(Duration::from_millis(600)).await;
        let m: Value = c.get(format!("{base}/api/media")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        assert_eq!(m["state"]["title"], "Night Loop");
        assert_eq!(c.post(format!("{base}/api/media/control")).header("authorization", &auth).json(&json!({ "action": "explode" })).send().await.unwrap().status(), StatusCode::UNPROCESSABLE_ENTITY);

        // Turned off on the PC: refused.
        core.update_settings(&json!({ "media": { "allowPhone": false } })).unwrap();
        assert_eq!(c.get(format!("{base}/api/media")).header("authorization", &auth).send().await.unwrap().status(), StatusCode::FORBIDDEN);
        let _ = ws.close(None).await;
    });
    core.remote.stop();
}

/// Tasks from the phone: CPU/memory of the whole PC and per program,
/// priority and "End task" on a program started for the test.
#[test]
fn tasks_from_the_phone() {
    let f = setup(false);
    // A harmless program with a name nothing else uses.
    let (src, args): (std::path::PathBuf, Vec<&str>) = if cfg!(windows) { (r"C:\Windows\System32\PING.EXE".into(), vec!["-n", "120", "127.0.0.1"]) } else { (std::path::PathBuf::from("/bin/sleep"), vec!["120"]) };
    let exe = f._dir.path().join(if cfg!(windows) { "omnihub-sleeper.exe" } else { "omnihub-sleeper" });
    std::fs::copy(&src, &exe).unwrap();
    let mut child = std::process::Command::new(&exe).args(&args).stdout(std::process::Stdio::null()).spawn().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let c = client();
        let base = &f.base;
        let pairing = f.core.remote.begin_pairing().unwrap();
        let r: Value = c.post(format!("{base}/api/pair")).json(&json!({ "pin": pairing.pin, "deviceName": "iPhone" })).send().await.unwrap().json().await.unwrap();
        let auth = format!("Bearer {}", r["token"].as_str().unwrap());
        let info: Value = c.get(format!("{base}/api/info")).send().await.unwrap().json().await.unwrap();
        assert_eq!(info["features"]["tasks"], true);

        let u: Value = c.get(format!("{base}/api/tasks?sort=memory&limit=5")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        let list = u["processes"].as_array().unwrap();
        assert!(!list.is_empty() && list.len() <= 5);
        assert!(list.windows(2).all(|w| w[0]["memory"].as_u64() >= w[1]["memory"].as_u64()));
        assert!(u["memoryTotal"].as_u64().unwrap() > 0 && u["cores"].as_u64().unwrap() > 0);
        assert!(u["gpuSupported"].is_boolean() && u["gpus"].is_array());
        for k in ["cpu", "gpu", "gpuMemory", "disk", "count", "canEnd"] {
            assert!(!list[0][k].is_null(), "{k} missing");
        }

        let u: Value = c.get(format!("{base}/api/tasks?sort=name&limit=500")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        let sleeper = u["processes"].as_array().unwrap().iter().find(|g| g["name"].as_str().unwrap().starts_with("omnihub-sleeper")).expect("test program listed").clone();
        assert_eq!(sleeper["canEnd"], true);
        let name = sleeper["name"].as_str().unwrap();

        let r = c.post(format!("{base}/api/tasks/priority")).header("authorization", &auth).json(&json!({ "name": name, "priority": "belowNormal" })).send().await.unwrap();
        assert_eq!(r.status(), if cfg!(windows) { StatusCode::OK } else { StatusCode::BAD_REQUEST });
        // Windows' own processes are never touched.
        let r = c.post(format!("{base}/api/tasks/end")).header("authorization", &auth).json(&json!({ "name": "csrss.exe" })).send().await.unwrap();
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);

        let r: Value = c.post(format!("{base}/api/tasks/end")).header("authorization", &auth).json(&json!({ "name": name })).send().await.unwrap().json().await.unwrap();
        assert_eq!(r["ended"], 1);

        f.core.update_settings(&json!({ "remote": { "allowTasks": false } })).unwrap();
        assert_eq!(c.get(format!("{base}/api/tasks")).header("authorization", &auth).send().await.unwrap().status(), StatusCode::FORBIDDEN);
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while child.try_wait().unwrap().is_none() {
        assert!(std::time::Instant::now() < deadline, "the program was not ended");
        std::thread::sleep(Duration::from_millis(50));
    }
    f.core.remote.stop();
}

/// Games from the phone: list profiles, boost (without launching), stop,
/// and a ping test against the profile's own host.
#[test]
fn games_from_the_phone() {
    let f = setup(false);
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let ping_port = l.local_addr().unwrap().port();
    std::thread::spawn(move || for _ in l.incoming() {});
    let mut p = f.core.games.create(omnihub_core::games::GameKind::Custom);
    p.name = "Phone game".into();
    p.ping_host = Some(format!("127.0.0.1:{ping_port}"));
    // Leave the machine running the tests alone.
    p.boost = omnihub_core::games::Boost { power_plan: omnihub_core::games::tweaks::PowerPlan::Keep, silence_notifications: false, game_mode: false, gpu_high_performance: false, wifi_low_latency: false, ..Default::default() };
    let p = f.core.games.save(p).unwrap().profile;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let c = client();
        let base = &f.base;
        let pairing = f.core.remote.begin_pairing().unwrap();
        let r: Value = c.post(format!("{base}/api/pair")).json(&json!({ "pin": pairing.pin, "deviceName": "iPhone" })).send().await.unwrap().json().await.unwrap();
        let auth = format!("Bearer {}", r["token"].as_str().unwrap());
        let info: Value = c.get(format!("{base}/api/info")).send().await.unwrap().json().await.unwrap();
        assert_eq!(info["features"]["games"], true);

        let g: Value = c.get(format!("{base}/api/games")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        assert_eq!(g["profiles"][0]["name"], "Phone game");
        assert_eq!(g["profiles"][0]["canLaunch"], false);
        assert!(g["session"].is_null());

        let r: Value = c.post(format!("{base}/api/games/{}/play", p.id)).header("authorization", &auth).json(&json!({ "launch": false })).send().await.unwrap().json().await.unwrap();
        assert_eq!(r["session"]["name"], "Phone game");
        let mut phase = Value::Null;
        for _ in 0..50 {
            let g: Value = c.get(format!("{base}/api/games")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
            phase = g["session"]["phase"].clone();
            if phase == "boosted" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert_eq!(phase, "boosted");
        // A second boost is refused while one runs.
        assert_eq!(c.post(format!("{base}/api/games/{}/play", p.id)).header("authorization", &auth).send().await.unwrap().status(), StatusCode::BAD_REQUEST);
        let r: Value = c.post(format!("{base}/api/games/stop")).header("authorization", &auth).send().await.unwrap().json().await.unwrap();
        assert_eq!(r["stopped"], true);

        let r: Value = c.post(format!("{base}/api/games/ping")).header("authorization", &auth).json(&json!({ "id": p.id })).send().await.unwrap().json().await.unwrap();
        assert_eq!(r["results"][0]["received"], 10, "{r}");
        assert!(r["results"][0]["avgMs"].as_f64().is_some());

        f.core.update_settings(&json!({ "remote": { "allowAppLaunch": false } })).unwrap();
        assert_eq!(c.get(format!("{base}/api/games")).header("authorization", &auth).send().await.unwrap().status(), StatusCode::FORBIDDEN);
    });
    f.core.remote.stop();
}
