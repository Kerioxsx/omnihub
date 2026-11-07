//! 0.30 commands: Capri Desktop and Ollama, the quiet schedule and sleep
//! timer, overheat alerts, app auto-updates, the party queue, screen time,
//! the item shop and price watchers, and voice commands.

use std::sync::Arc;

use omnihub_core::app_updates::AutoUpdateSettings;
use omnihub_core::capri_app::{self, CapriSettings, CapriStatus, Latest};
use omnihub_core::core::AppCore;
use omnihub_core::heat::HeatSettings;
use omnihub_core::party::PartyState;
use omnihub_core::quiet::{QuietSchedule, QuietStatus};
use omnihub_core::screentime::{Limit, LimitUse};
use omnihub_core::watchers::{PriceWatch, WatchStore};
use serde::Serialize;
use tauri::State;

type Core<'a> = State<'a, Arc<AppCore>>;
type Res<T> = Result<T, String>;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Res<T> + Send + 'static) -> Res<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| e.to_string())?
}

fn today() -> String {
    omnihub_core::screentime::local_today()
}

// ---------- Capri Desktop + Ollama ----------

fn capri_running(core: &AppCore) -> bool {
    !core.procs.pids_named(capri_app::PROCESS).is_empty()
}

#[tauri::command]
pub async fn capri_desktop_status(core: Core<'_>) -> Res<CapriStatus> {
    let core = core.inner().clone();
    blocking(move || Ok(core.capri_app.status(capri_running(&core)))).await
}

#[tauri::command]
pub async fn capri_desktop_latest(core: Core<'_>) -> Res<Latest> {
    let core = core.inner().clone();
    blocking(move || core.capri_app.latest()).await
}

#[tauri::command]
pub async fn capri_desktop_install(core: Core<'_>) -> Res<Latest> {
    let core = core.inner().clone();
    let r = blocking(move || {
        let r = core.capri_app.install();
        core.audit.record("desktop", "capri.install", &r.as_ref().map(|l| l.version.clone()).unwrap_or_else(|e| e.clone()), r.is_ok());
        r
    })
    .await;
    r
}

#[tauri::command]
pub async fn capri_desktop_open(core: Core<'_>) -> Res<()> {
    core.capri_app.open()
}

#[tauri::command]
pub async fn capri_desktop_close(core: Core<'_>) -> Res<()> {
    core.procs.end(capri_app::PROCESS).map(|_| ())
}

#[tauri::command]
pub async fn capri_desktop_settings_set(core: Core<'_>, settings: CapriSettings) -> Res<()> {
    core.capri_app.save(settings)
}

#[tauri::command]
pub async fn ollama_pull(core: Core<'_>) -> Res<()> {
    let core = core.inner().clone();
    blocking(move || capri_app::ollama_pull(capri_app::MODEL, &core.events)).await
}

// ---------- quiet schedule + sleep timer ----------

#[tauri::command]
pub async fn quiet_schedule_set(core: Core<'_>, schedule: QuietSchedule) -> Res<QuietStatus> {
    core.quiet.set_schedule(schedule)?;
    let s = core.quiet_status();
    core.events.emit("quiet:changed", &s);
    Ok(s)
}

#[tauri::command]
pub async fn quiet_timer_set(core: Core<'_>, minutes: Option<u32>) -> Res<QuietStatus> {
    core.quiet.set_timer(minutes);
    let s = core.quiet_status();
    core.events.emit("quiet:changed", &s);
    Ok(s)
}

// ---------- overheat alerts ----------

#[tauri::command]
pub async fn heat_get(core: Core<'_>) -> Res<HeatSettings> {
    Ok(core.heat.settings())
}

#[tauri::command]
pub async fn heat_set(core: Core<'_>, settings: HeatSettings) -> Res<HeatSettings> {
    core.heat.save(settings)
}

// ---------- app auto-updates ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateState {
    settings: AutoUpdateSettings,
    running: bool,
}

#[tauri::command]
pub async fn app_updates_get(core: Core<'_>) -> Res<AutoUpdateState> {
    Ok(AutoUpdateState { settings: core.app_updates.settings(), running: core.app_updates.running() })
}

#[tauri::command]
pub async fn app_updates_set(core: Core<'_>, settings: AutoUpdateSettings) -> Res<AutoUpdateState> {
    let settings = core.app_updates.save(settings)?;
    Ok(AutoUpdateState { settings, running: core.app_updates.running() })
}

#[tauri::command]
pub async fn app_updates_run(core: Core<'_>) -> Res<AutoUpdateState> {
    let core = core.inner().clone();
    blocking(move || {
        let (done, failed) = core.app_updates.run(omnihub_core::db::now())?;
        core.events.emit("apps:auto-updated", serde_json::json!({ "updated": done, "failed": failed }));
        Ok(AutoUpdateState { settings: core.app_updates.settings(), running: false })
    })
    .await
}

// ---------- party queue ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartyView {
    state: PartyState,
    urls: Vec<String>,
    qr_svgs: Vec<String>,
}

fn party_view(core: &AppCore) -> PartyView {
    let state = core.party.state();
    let status = core.remote.status();
    let urls: Vec<String> = match (&state.code, status.running) {
        (Some(code), true) => status.urls.iter().map(|u| format!("{u}/party/{code}")).collect(),
        _ => Vec::new(),
    };
    let qr_svgs = urls.iter().map(|u| omnihub_core::remote::net::qr_svg(u)).collect();
    PartyView { state, urls, qr_svgs }
}

#[tauri::command]
pub async fn party_get(core: Core<'_>) -> Res<PartyView> {
    Ok(party_view(&core))
}

#[tauri::command]
pub async fn party_start(core: Core<'_>) -> Res<PartyView> {
    if !core.remote.status().running {
        return Err("Turn on the phone companion first: guests open the party from it.".into());
    }
    core.party.start();
    core.audit.record("desktop", "party.start", "", true);
    let v = party_view(&core);
    core.events.emit("party:changed", &v.state);
    Ok(v)
}

#[tauri::command]
pub async fn party_stop(core: Core<'_>) -> Res<PartyView> {
    core.party.stop();
    let v = party_view(&core);
    core.events.emit("party:changed", &v.state);
    Ok(v)
}

#[tauri::command]
pub async fn party_auto(core: Core<'_>, on: bool) -> Res<PartyView> {
    core.party.set_auto(on);
    Ok(party_view(&core))
}

#[tauri::command]
pub async fn party_remove(core: Core<'_>, id: String) -> Res<PartyView> {
    core.party.remove(&id);
    let v = party_view(&core);
    core.events.emit("party:changed", &v.state);
    Ok(v)
}

#[tauri::command]
pub async fn party_bump(core: Core<'_>, id: String) -> Res<PartyView> {
    core.party.bump(&id);
    let v = party_view(&core);
    core.events.emit("party:changed", &v.state);
    Ok(v)
}

#[tauri::command]
pub async fn party_next(core: Core<'_>) -> Res<PartyView> {
    let core = core.inner().clone();
    blocking(move || {
        core.party_play_next()?;
        Ok(party_view(&core))
    })
    .await
}

// ---------- screen time ----------

#[tauri::command]
pub async fn screentime_list(core: Core<'_>) -> Res<Vec<LimitUse>> {
    Ok(core.screentime.list(&today()))
}

#[tauri::command]
pub async fn screentime_save(core: Core<'_>, limit: Limit) -> Res<Vec<LimitUse>> {
    let l = core.screentime.save(limit)?;
    core.audit.record("desktop", "screentime.save", &format!("{} {} min", l.target, l.minutes), true);
    Ok(core.screentime.list(&today()))
}

#[tauri::command]
pub async fn screentime_delete(core: Core<'_>, id: String) -> Res<Vec<LimitUse>> {
    core.screentime.delete(&id);
    Ok(core.screentime.list(&today()))
}

// ---------- item shop + prices ----------

#[tauri::command]
pub async fn watch_get(core: Core<'_>) -> Res<WatchStore> {
    Ok(core.watchers.get())
}

#[tauri::command]
pub async fn watch_shop_set(core: Core<'_>, wanted: Vec<String>, alerts: bool) -> Res<WatchStore> {
    Ok(core.watchers.set_shop(wanted, alerts))
}

/// Read today's shop now (and tell about wanted items in it).
#[tauri::command]
pub async fn watch_shop_check(core: Core<'_>) -> Res<WatchStore> {
    let core = core.inner().clone();
    blocking(move || {
        let day = omnihub_core::watchers::utc_today();
        for h in core.watchers.check_shop(&day)? {
            core.events.emit("watch:shop", serde_json::json!({ "item": h }));
        }
        Ok(core.watchers.get())
    })
    .await
}

#[tauri::command]
pub async fn watch_price_save(core: Core<'_>, watch: PriceWatch) -> Res<WatchStore> {
    let core = core.inner().clone();
    blocking(move || {
        let w = core.watchers.save_price(watch)?;
        // Read its price at once.
        let read = omnihub_core::watchers::fetch_price(&w.url);
        if let Some(d) = core.watchers.record(&w.id, read, omnihub_core::db::now()) {
            core.events.emit("watch:price", &d);
        }
        Ok(core.watchers.get())
    })
    .await
}

#[tauri::command]
pub async fn watch_price_delete(core: Core<'_>, id: String) -> Res<WatchStore> {
    core.watchers.delete_price(&id);
    Ok(core.watchers.get())
}

#[tauri::command]
pub async fn watch_prices_check(core: Core<'_>) -> Res<WatchStore> {
    let core = core.inner().clone();
    blocking(move || {
        let now = omnihub_core::db::now();
        for p in core.watchers.get().prices {
            let read = omnihub_core::watchers::fetch_price(&p.url);
            if let Some(d) = core.watchers.record(&p.id, read, now) {
                core.events.emit("watch:price", &d);
            }
        }
        Ok(core.watchers.get())
    })
    .await
}

// ---------- voice ----------

#[tauri::command]
pub async fn voice_command(core: Core<'_>, text: String) -> Res<String> {
    let core = core.inner().clone();
    blocking(move || core.voice_text(&text, true)).await
}

// ---------- controllers ----------

/// Set the light bar of the PlayStation controllers connected ("#ff6a00").
#[tauri::command]
pub async fn pads_set_light(color: String, bluetooth: bool) -> Res<omnihub_core::pads::LightResult> {
    let rgb = omnihub_core::pads::parse_color(&color).ok_or("Pick a colour")?;
    blocking(move || Ok(omnihub_core::pads::set_light(rgb, bluetooth))).await
}

// ---------- theme pictures ----------

fn art_url(core: &AppCore, p: Option<std::path::PathBuf>) -> Option<String> {
    p.map(|p| omnihub_core::games::shelf::media_url(&core.shelf.media_token(&p)))
}

/// The address of your picture behind a Bleach theme, if you set one.
#[tauri::command]
pub async fn theme_art_get(core: Core<'_>, theme: String) -> Res<Option<String>> {
    Ok(art_url(&core, core.theme_art.path(&theme)))
}

/// Use a picture (copied into OmniHub's data folder) behind a theme, or none.
#[tauri::command]
pub async fn theme_art_set(core: Core<'_>, theme: String, path: Option<String>) -> Res<Option<String>> {
    let p = core.theme_art.set(&theme, path.as_deref().map(std::path::Path::new))?;
    Ok(art_url(&core, p))
}

// ---------- quick sites, sound output, Bluetooth ----------

#[tauri::command]
pub async fn sites_list(core: Core<'_>) -> Res<Vec<omnihub_core::sites::Site>> {
    Ok(core.sites.list())
}

#[tauri::command]
pub async fn sites_save(core: Core<'_>, list: Vec<omnihub_core::sites::Site>) -> Res<Vec<omnihub_core::sites::Site>> {
    core.sites.save(list)
}

#[tauri::command]
pub async fn sites_reset(core: Core<'_>) -> Res<Vec<omnihub_core::sites::Site>> {
    core.sites.reset()
}

#[tauri::command]
pub async fn sites_open(url: String, full_screen: bool) -> Res<()> {
    blocking(move || omnihub_core::sites::open(&url, full_screen)).await
}

#[tauri::command]
pub async fn audio_outputs() -> Res<Vec<omnihub_core::media::outputs::Output>> {
    blocking(omnihub_core::media::outputs::list).await
}

#[tauri::command]
pub async fn audio_output_set(id: String) -> Res<Vec<omnihub_core::media::outputs::Output>> {
    blocking(move || {
        omnihub_core::media::outputs::set_default(&id)?;
        omnihub_core::media::outputs::list()
    })
    .await
}

#[tauri::command]
pub async fn bt_list() -> Res<Vec<omnihub_core::system::bt::BtDevice>> {
    blocking(omnihub_core::system::bt::list).await
}

#[tauri::command]
pub async fn bt_connect(core: Core<'_>, address: String, name: String, on: bool) -> Res<Vec<omnihub_core::system::bt::BtDevice>> {
    let core = core.inner().clone();
    blocking(move || {
        let r = omnihub_core::system::bt::connect(&address, &name, on);
        core.audit.record("desktop", if on { "bluetooth.connect" } else { "bluetooth.disconnect" }, &name, r.is_ok());
        r?;
        std::thread::sleep(std::time::Duration::from_millis(1200));
        omnihub_core::system::bt::list()
    })
    .await
}

#[tauri::command]
pub async fn bt_forget(core: Core<'_>, address: String, name: String) -> Res<Vec<omnihub_core::system::bt::BtDevice>> {
    let core = core.inner().clone();
    blocking(move || {
        let r = omnihub_core::system::bt::forget(&address);
        core.audit.record("desktop", "bluetooth.forget", &name, r.is_ok());
        r?;
        omnihub_core::system::bt::list()
    })
    .await
}

// ---------- Karaoke Studio ----------

#[tauri::command]
pub async fn karaoke_get(core: Core<'_>) -> Res<omnihub_core::karaoke::KaraokeData> {
    Ok(core.karaoke.get())
}

#[tauri::command]
pub async fn karaoke_profile(core: Core<'_>, lang: String) -> Res<omnihub_core::karaoke::KaraokeData> {
    core.karaoke.set_profile(&lang)
}

#[tauri::command]
pub async fn karaoke_session(core: Core<'_>, session: omnihub_core::karaoke::Session) -> Res<omnihub_core::karaoke::KaraokeData> {
    core.karaoke.add_session(session)
}

#[tauri::command]
pub async fn karaoke_clear(core: Core<'_>) -> Res<omnihub_core::karaoke::KaraokeData> {
    core.karaoke.clear()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KaraokeTrack {
    #[serde(flatten)]
    track: omnihub_core::karaoke::Track,
    url: String,
}

/// A song file from this PC, with the address the page plays it from.
#[tauri::command]
pub async fn karaoke_track(core: Core<'_>, path: String) -> Res<KaraokeTrack> {
    let p = std::path::PathBuf::from(&path);
    let track = omnihub_core::karaoke::track(&p)?;
    let url = omnihub_core::games::shelf::media_url(&core.shelf.media_token(&p));
    Ok(KaraokeTrack { track, url })
}

#[tauri::command]
pub async fn karaoke_lyrics(core: Core<'_>, song: Option<String>, lrc: Option<String>, title: String, artist: String, duration_ms: u64) -> Res<Option<omnihub_core::media::lyrics::Lyrics>> {
    let dir = core.media.lyrics_folder();
    blocking(move || omnihub_core::karaoke::lyrics_for(song.as_deref().map(std::path::Path::new), lrc.as_deref().map(std::path::Path::new), dir.as_deref(), &title, &artist, duration_ms)).await
}

// ---------- 0.31: microphones, screen modes, night light ----------

#[tauri::command]
pub async fn audio_inputs() -> Res<Vec<omnihub_core::media::outputs::Output>> {
    blocking(omnihub_core::media::outputs::inputs).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenModes {
    current: Option<omnihub_core::system::screenmode::Mode>,
    modes: Vec<omnihub_core::system::screenmode::Mode>,
    hdr: Option<bool>,
}

#[tauri::command]
pub async fn screen_modes() -> Res<ScreenModes> {
    use omnihub_core::system::screenmode;
    blocking(|| Ok(ScreenModes { current: screenmode::current(), modes: screenmode::modes(), hdr: screenmode::hdr() })).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NightInfo {
    settings: omnihub_core::nightlight::NightSettings,
    status: omnihub_core::nightlight::NightStatus,
}

fn night_info(core: &omnihub_core::core::AppCore) -> NightInfo {
    NightInfo { settings: core.nightlight.settings(), status: core.nightlight.status(omnihub_core::core::AppCore::local_minute()) }
}

#[tauri::command]
pub async fn nightlight_get(core: Core<'_>) -> Res<NightInfo> {
    Ok(night_info(&core))
}

#[tauri::command]
pub async fn nightlight_save(core: Core<'_>, settings: omnihub_core::nightlight::NightSettings) -> Res<NightInfo> {
    core.nightlight.save(settings)?;
    let c = core.inner().clone();
    blocking(move || {
        c.nightlight_tick(true);
        Ok(night_info(&c))
    })
    .await
}

/// On or off now (until the schedule next changes), or back to the schedule (null).
#[tauri::command]
pub async fn nightlight_now(core: Core<'_>, on: Option<bool>) -> Res<NightInfo> {
    core.nightlight.set_manual(on, omnihub_core::core::AppCore::local_minute());
    let c = core.inner().clone();
    blocking(move || {
        c.nightlight_tick(true);
        Ok(night_info(&c))
    })
    .await
}

#[derive(Serialize)]
pub struct LocalModels {
    models: Vec<String>,
    model: String,
}

/// Models Ollama has on this PC (an error if Ollama isn't running).
#[tauri::command]
pub async fn chats_local_models(core: Core<'_>) -> Res<LocalModels> {
    let c = core.inner().clone();
    blocking(move || Ok(LocalModels { models: c.chats.local.models()?, model: c.chats.local.settings().model })).await
}

#[tauri::command]
pub async fn chats_local_set(core: Core<'_>, model: String) -> Res<()> {
    core.chats.local.set_model(&model).map(|_| ())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AskReply {
    answers: Vec<omnihub_core::askpc::Answer>,
    /// The same, said in a sentence or two by a local model (when Ollama runs).
    words: Option<String>,
}

/// "Ask your PC": answered from what OmniHub measures now.
#[tauri::command]
pub async fn ask_pc(core: Core<'_>, question: String, words: bool) -> Res<AskReply> {
    let q: String = question.trim().chars().take(500).collect();
    if q.is_empty() {
        return Err("Ask something, like “why is my PC slow?”".into());
    }
    let c = core.inner().clone();
    blocking(move || {
        let answers = c.ask_pc(&q);
        let words = words
            .then(|| {
                let model = c.chats.local.model().ok()?;
                c.chats.local.chat(&model, "You explain a Windows PC's state to its owner in plain words.", &[("user".into(), omnihub_core::askpc::as_prompt(&q, &answers))], &std::sync::atomic::AtomicBool::new(false), &mut |_| {}).ok()
            })
            .flatten()
            .map(|w| w.trim().to_string())
            .filter(|w| !w.is_empty());
        Ok(AskReply { answers, words })
    })
    .await
}

// ---------- 0.31: creators going live, the daily recap ----------

#[tauri::command]
pub async fn watch_streamer_save(core: Core<'_>, streamer: omnihub_core::watchers::Streamer) -> Res<WatchStore> {
    core.watchers.save_streamer(streamer)
}

#[tauri::command]
pub async fn watch_streamer_delete(core: Core<'_>, id: String) -> Res<WatchStore> {
    Ok(core.watchers.delete_streamer(&id))
}

/// Look at every creator's page now.
#[tauri::command]
pub async fn watch_streamers_check(core: Core<'_>) -> Res<WatchStore> {
    let c = core.inner().clone();
    blocking(move || {
        for live in c.watchers.check_streamers() {
            c.events.emit("watch:live", &live);
        }
        Ok(c.watchers.get())
    })
    .await
}

#[tauri::command]
pub async fn watch_recap_set(core: Core<'_>, time: String) -> Res<WatchStore> {
    core.watchers.set_recap(&time)
}

/// Today's recap now (to preview it).
#[tauri::command]
pub async fn watch_recap_now(core: Core<'_>) -> Res<serde_json::Value> {
    let c = core.inner().clone();
    blocking(move || {
        let (title, body) = c.daily_recap();
        Ok(serde_json::json!({ "title": title, "body": body }))
    })
    .await
}

// ---------- 0.31: similar photos ----------

static SIMILAR_STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Pictures the last scan found: only these can be sent to the Recycle Bin from here.
static SIMILAR_FOUND: parking_lot::Mutex<Vec<String>> = parking_lot::Mutex::new(Vec::new());

#[tauri::command]
pub async fn photos_similar(core: Core<'_>, dir: String) -> Res<Vec<omnihub_core::storage::similar::SimilarGroup>> {
    let c = core.inner().clone();
    SIMILAR_STOP.store(false, std::sync::atomic::Ordering::SeqCst);
    blocking(move || {
        let events = c.events.clone();
        let groups = omnihub_core::storage::similar::find(std::path::Path::new(&dir), &SIMILAR_STOP, &|done, total| events.emit("photos:progress", serde_json::json!({ "done": done, "total": total })))?;
        *SIMILAR_FOUND.lock() = groups.iter().flat_map(|g| g.photos.iter().map(|p| p.path.clone())).collect();
        Ok(groups)
    })
    .await
}

#[tauri::command]
pub async fn photos_similar_stop() -> Res<()> {
    SIMILAR_STOP.store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

/// Send pictures from the last scan to the Recycle Bin; says how many went.
#[tauri::command]
pub async fn photos_trash(paths: Vec<String>) -> Res<usize> {
    blocking(move || {
        let found = SIMILAR_FOUND.lock().clone();
        let ok: Vec<String> = paths.into_iter().filter(|p| found.contains(p)).collect();
        Ok(omnihub_core::storage::similar::recycle(&ok))
    })
    .await
}

// ---------- 0.31: family and guest phones ----------

#[derive(Serialize)]
pub struct PhoneLimitsInfo {
    areas: Vec<(&'static str, &'static str)>,
    phones: std::collections::HashMap<String, Vec<String>>,
}

#[tauri::command]
pub async fn phone_limits_get(core: Core<'_>) -> Res<PhoneLimitsInfo> {
    Ok(PhoneLimitsInfo { areas: omnihub_core::remote::limits::AREAS.to_vec(), phones: core.phone_limits.get().phones })
}

/// Limit a paired phone to some areas (null: the whole app again).
#[tauri::command]
pub async fn phone_limits_set(core: Core<'_>, device: String, areas: Option<Vec<String>>) -> Res<PhoneLimitsInfo> {
    let l = core.phone_limits.set(&device, areas)?;
    core.audit.record("pc", "phone.limits", &device, true);
    Ok(PhoneLimitsInfo { areas: omnihub_core::remote::limits::AREAS.to_vec(), phones: l.phones })
}

// ---------- 0.32 ----------

#[tauri::command]
pub async fn wifi_qr() -> Res<omnihub_core::system::wifi::WifiQr> {
    blocking(omnihub_core::system::wifi::current).await
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

#[tauri::command]
pub async fn timers_get(core: Core<'_>) -> Res<omnihub_core::timers::TimersState> {
    Ok(core.timers.get())
}

#[tauri::command]
pub async fn timers_start(core: Core<'_>, minutes: u32, label: String) -> Res<omnihub_core::timers::TimersState> {
    core.timers.start(minutes, &label, now_ms())
}

#[tauri::command]
pub async fn timers_cancel(core: Core<'_>, id: String) -> Res<omnihub_core::timers::TimersState> {
    Ok(core.timers.cancel(&id))
}

#[tauri::command]
pub async fn alarm_save(core: Core<'_>, alarm: omnihub_core::timers::Alarm) -> Res<omnihub_core::timers::TimersState> {
    core.timers.save_alarm(alarm)
}

#[tauri::command]
pub async fn alarm_delete(core: Core<'_>, id: String) -> Res<omnihub_core::timers::TimersState> {
    Ok(core.timers.delete_alarm(&id))
}

#[tauri::command]
pub async fn bedtime_save(core: Core<'_>, bedtime: omnihub_core::timers::Bedtime) -> Res<omnihub_core::timers::TimersState> {
    core.timers.save_bedtime(bedtime)
}

#[tauri::command]
pub async fn bedtime_snooze(core: Core<'_>, minutes: u32) -> Res<omnihub_core::timers::TimersState> {
    Ok(core.timers.snooze(minutes, now_ms()))
}

#[tauri::command]
pub async fn watch_game_alerts(core: Core<'_>, free: bool, patches: bool) -> Res<WatchStore> {
    Ok(core.watchers.set_game_alerts(free, patches))
}

#[tauri::command]
pub async fn drives_health() -> Res<Vec<omnihub_core::pccheck::DriveHealth>> {
    blocking(omnihub_core::pccheck::drives).await
}

static STRESS_STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static STRESS_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Every core busy for a few minutes; `stress:progress` and `stress:done` events.
#[tauri::command]
pub async fn stress_start(core: Core<'_>, seconds: u32) -> Res<()> {
    use std::sync::atomic::Ordering;
    if STRESS_ON.swap(true, Ordering::SeqCst) {
        return Err("A test is already running".into());
    }
    STRESS_STOP.store(false, Ordering::SeqCst);
    let c = core.inner().clone();
    std::thread::spawn(move || {
        let temp = || {
            c.rig.hold();
            c.rig.status().cpu_temp
        };
        let ev = c.events.clone();
        let r = omnihub_core::pccheck::stress(seconds, &STRESS_STOP, &temp, &|s, t| ev.emit("stress:progress", serde_json::json!({ "seconds": s, "temp": t })));
        c.events.emit("stress:done", &r);
        STRESS_ON.store(false, Ordering::SeqCst);
    });
    Ok(())
}

#[tauri::command]
pub async fn stress_stop() -> Res<()> {
    STRESS_STOP.store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub async fn game_ready(core: Core<'_>) -> Res<Vec<omnihub_core::pccheck::ReadyItem>> {
    let c = core.inner().clone();
    blocking(move || Ok(c.game_ready())).await
}

#[tauri::command]
pub async fn windows_list() -> Res<Vec<omnihub_core::system::winpin::PinWindow>> {
    blocking(|| Ok(omnihub_core::system::winpin::list())).await
}

#[tauri::command]
pub async fn window_pin(handle: i64, on: bool) -> Res<bool> {
    blocking(move || omnihub_core::system::winpin::set(handle, on)).await
}

#[tauri::command]
pub async fn after_downloads_get(core: Core<'_>) -> Res<omnihub_core::timers::AfterDownloads> {
    Ok(core.after_downloads.lock().clone())
}

/// "shutdown", "sleep" or "" (off).
#[tauri::command]
pub async fn after_downloads_set(core: Core<'_>, action: String) -> Res<omnihub_core::timers::AfterDownloads> {
    if !["", "shutdown", "sleep"].contains(&action.as_str()) {
        return Err("Pick shut down or sleep".into());
    }
    let mut a = core.after_downloads.lock();
    *a = omnihub_core::timers::AfterDownloads { action, quiet_since: 0 };
    Ok(a.clone())
}

static MOVE_STOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Move a library game's folder to another drive (`games:move` progress).
#[tauri::command]
pub async fn game_move(core: Core<'_>, key: String, dest: String) -> Res<String> {
    let c = core.inner().clone();
    MOVE_STOP.store(false, std::sync::atomic::Ordering::SeqCst);
    blocking(move || {
        let g = c.games.library().into_iter().find(|g| g.key == key).ok_or("That game isn't in the library")?;
        let dir = g.install_dir.ok_or("OmniHub doesn't know where this game is installed")?;
        if !g.process.is_empty() && !c.procs.pids_named(&g.process).is_empty() {
            return Err("Close the game first".into());
        }
        let dest = std::path::PathBuf::from(&dest);
        let drive = dest.components().next().map(|p| p.as_os_str().to_string_lossy().to_uppercase()).unwrap_or_default();
        let free = omnihub_core::health::gather().disks.into_iter().find(|(d, _, _)| d.to_uppercase().starts_with(drive.trim_end_matches('\\'))).map_or(0, |d| d.1);
        let ev = c.events.clone();
        let name = g.name.clone();
        let to = omnihub_core::games::mover::move_game(std::path::Path::new(&dir), &dest, free, &MOVE_STOP, &|done, total| ev.emit("games:move", serde_json::json!({ "name": name, "done": done, "total": total })), &omnihub_core::games::mover::junction)?;
        Ok(to.to_string_lossy().into_owned())
    })
    .await
}

#[tauri::command]
pub async fn game_move_stop() -> Res<()> {
    MOVE_STOP.store(true, std::sync::atomic::Ordering::SeqCst);
    Ok(())
}
