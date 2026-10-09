//! A game boost from start to finish with stand-in programs: close a
//! background app, launch the "game", follow it until it exits, then put
//! everything back (and reopen the app).

use std::sync::Arc;
use std::time::{Duration, Instant};

use omnihub_core::events::EventBus;
use omnihub_core::games::{GameHub, GameKind, Launch, Phase, StepStatus};
use omnihub_core::system::procs::ProcessMonitor;

/// A copy of a harmless program under a short name (process names are
/// cut to 15 characters on Linux).
fn stand_in(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let src = if cfg!(windows) { std::path::PathBuf::from(r"C:\Windows\System32\PING.EXE") } else { std::path::PathBuf::from("/bin/sleep") };
    let p = dir.join(name);
    std::fs::copy(src, &p).unwrap();
    p
}

fn wait_args(secs: u32) -> String {
    if cfg!(windows) {
        format!("-n {} 127.0.0.1", secs + 1)
    } else {
        secs.to_string()
    }
}

fn wait_until(what: &str, limit: Duration, mut f: impl FnMut() -> bool) {
    let t = Instant::now();
    while !f() {
        assert!(t.elapsed() < limit, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn boost_launch_watch_restore() {
    let dir = tempfile::tempdir().unwrap();
    let events = EventBus::new();
    let mut rx = events.subscribe();
    let hub = GameHub::with_test_timing(dir.path(), events, Arc::new(ProcessMonitor::new()), Duration::from_millis(200), Duration::from_secs(10));

    let hog = stand_in(dir.path(), "oh.exe");
    let mut hog_proc = std::process::Command::new(&hog).args(omnihub_core::games::split_args(&wait_args(60))).stdout(std::process::Stdio::null()).spawn().unwrap();
    let game = stand_in(dir.path(), "og.exe");

    let mut p = hub.create(GameKind::Custom);
    p.name = "Stand-in game".into();
    p.launch = Launch::Exe { path: game.to_string_lossy().into_owned(), args: wait_args(2) };
    p.process = String::new(); // filled in from the program
    p.boost.close_apps = vec!["oh.exe".into(), "svchost.exe".into()];
    p.boost.reopen_apps = true;
    // Leave the machine running the tests alone.
    p.boost.power_plan = omnihub_core::games::tweaks::PowerPlan::Keep;
    p.boost.silence_notifications = false;
    p.boost.game_mode = false;
    p.boost.gpu_high_performance = false;
    p.boost.wifi_low_latency = false;
    let saved = hub.save(p).unwrap().profile;
    assert_eq!(saved.process, "og.exe");
    assert_eq!(saved.boost.close_apps, vec!["oh.exe".to_string()], "Windows' own processes are never on the list");

    let mut bad = saved.clone();
    bad.process = "not a program".into();
    assert!(hub.save(bad).is_err());

    hub.play(&saved.id, true).unwrap();
    assert!(hub.play(&saved.id, true).is_err(), "one boost at a time");
    wait_until("the boost to end", Duration::from_secs(30), || hub.session().is_some_and(|s| !s.active()));

    let s = hub.session().unwrap();
    let by_id = |id: &str| s.steps.iter().find(|x| x.id == id).cloned();
    let apps = by_id("apps").expect("apps step");
    assert_eq!(apps.status, StepStatus::Done, "{apps:?}");
    assert_eq!(apps.detail, "Closed oh");
    assert_eq!(by_id("launch").unwrap().status, StepStatus::Done);
    assert!(s.launched);
    assert_eq!(s.message.as_deref(), Some("Stand-in game closed"));
    // The background app was closed during the game and reopened after.
    wait_until("the hog to be closed", Duration::from_secs(5), || hog_proc.try_wait().unwrap().is_some());
    assert!(s.restored.iter().any(|x| x.id == "apps" && x.detail == "Reopened oh"), "{:?}", s.restored);
    assert!(!dir.path().join("games-session.json").exists(), "journal removed");
    assert!(hub.profile(&saved.id).unwrap().last_played.is_some());

    // The phases went by in order.
    let mut phases = Vec::new();
    while let Ok(e) = rx.try_recv() {
        if e.topic == "games:session" {
            let ph: Phase = serde_json::from_value(e.payload["phase"].clone()).unwrap();
            if phases.last() != Some(&ph) {
                phases.push(ph);
            }
        }
    }
    assert_eq!(phases, vec![Phase::Starting, Phase::Waiting, Phase::Playing, Phase::Restoring, Phase::Ended]);

    // Boost only, nothing to watch: on until stopped.
    let mut b = hub.create(GameKind::Custom);
    b.boost = omnihub_core::games::Boost { power_plan: omnihub_core::games::tweaks::PowerPlan::Keep, silence_notifications: false, game_mode: false, gpu_high_performance: false, wifi_low_latency: false, ..Default::default() };
    let b = hub.save(b).unwrap().profile;
    hub.play(&b.id, false).unwrap();
    wait_until("boosted", Duration::from_secs(5), || hub.session().is_some_and(|s| s.phase == Phase::Boosted));
    assert!(hub.stop());
    wait_until("stopped", Duration::from_secs(5), || hub.session().is_some_and(|s| !s.active()));

    // Profiles persist.
    let again = GameHub::new(dir.path(), EventBus::new(), Arc::new(ProcessMonitor::new()));
    assert_eq!(again.profiles().len(), 2);
    assert!(again.delete(&b.id).is_ok());
    assert_eq!(again.profiles().len(), 1);
    let _ = hog_proc.kill();
}

/// Fortnite found through its Epic manifest, added as a profile, its
/// settings file optimized and restored, and the boost writing it again
/// before a launch.
#[test]
fn installed_fortnite_settings_and_boost() {
    use omnihub_core::games::configs::ConfigGame;
    use omnihub_core::games::library::Roots;

    let dir = tempfile::tempdir().unwrap();
    let epic = dir.path().join("epic");
    std::fs::create_dir_all(&epic).unwrap();
    std::fs::write(epic.join("fn.item"), serde_json::json!({"AppName": "Fortnite", "DisplayName": "Fortnite", "CatalogNamespace": "fn", "CatalogItemId": "4fe75bbc5a674f4f9b356b5c90567da5", "InstallLocation": dir.path().join("Fortnite"), "AppCategories": ["games"]}).to_string()).unwrap();
    let ini = dir.path().join("GameUserSettings.ini");
    let original = "[/Script/FortniteGame.FortGameUserSettings]\nFrameRateLimit=144.000000\nbUseVSync=True\n";
    std::fs::write(&ini, original).unwrap();

    let events = EventBus::new();
    let hub = GameHub::with_test_timing(dir.path(), events, Arc::new(ProcessMonitor::new()), Duration::from_millis(100), Duration::from_secs(10));
    hub.set_test_paths(vec![(ConfigGame::Fortnite, ini.clone())], Roots { epic_manifests: Some(epic), ..Default::default() });

    let found = hub.library();
    let f = found.iter().find(|g| g.key == "epic:Fortnite").expect("Fortnite found");
    assert!(f.profile_id.is_none());
    let p = hub.add_installed("epic:Fortnite").unwrap();
    assert_eq!(p.kind, GameKind::Fortnite);
    assert_eq!(hub.library().iter().find(|g| g.key == "epic:Fortnite").unwrap().profile_id.as_deref(), Some(p.id.as_str()));
    assert_eq!(hub.add_installed("epic:Fortnite").unwrap().id, p.id, "adding twice gives the same profile");

    let s = hub.config_status(ConfigGame::Fortnite);
    assert!(s.found && !s.pending.is_empty());
    let changed = hub.config_apply(ConfigGame::Fortnite).unwrap();
    assert!(!changed.is_empty());
    assert!(hub.config_status(ConfigGame::Fortnite).pending.is_empty());
    assert!(std::fs::read_to_string(&ini).unwrap().contains("FrameRateLimit=0.000000"));
    hub.config_restore(ConfigGame::Fortnite).unwrap();
    assert_eq!(std::fs::read_to_string(&ini).unwrap(), original);

    // The boost (without touching this PC's settings) writes them again.
    let mut p = p;
    p.launch = Launch::None;
    p.boost.power_plan = omnihub_core::games::tweaks::PowerPlan::Keep;
    p.boost.silence_notifications = false;
    p.boost.game_mode = false;
    p.boost.gpu_high_performance = false;
    p.boost.wifi_low_latency = false;
    p.boost.priority = None;
    let p = hub.save(p).unwrap().profile;
    hub.play(&p.id, false).unwrap();
    wait_until("the settings step", Duration::from_secs(10), || hub.session().is_some_and(|s| s.steps.iter().any(|x| x.id == "settings")));
    let step = hub.session().unwrap().steps.into_iter().find(|x| x.id == "settings").unwrap();
    assert_eq!(step.status, StepStatus::Done, "{step:?}");
    assert!(std::fs::read_to_string(&ini).unwrap().contains("bUseVSync=False"));
    hub.stop();
    wait_until("the end of the boost", Duration::from_secs(10), || hub.session().is_some_and(|s| s.phase == Phase::Ended));
}
