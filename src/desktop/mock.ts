// In-browser mock of the Rust backend, used when the UI runs outside Tauri
// (`npm run dev` in a normal browser, screenshots, UI development). Every
// command in api.ts is implemented against realistic, seeded in-memory data.
// Like Tauri, failures reject with a plain string.

import type { HashKind, MusicSource, PlayItem, RenameRule, CaptureKind, ChatSource, ConfigGame, ConfigOptions, DeepPartial, DupeOptions, EntryInput, GameKind, GameProfile, GeneratorOptions, MediaAction, ModSort, NewChatKind, NoteFilter, NoteInput, PowerAction, Priority, ProcessSort, Rect, RobloxFlags, RobloxShaders, ScanRequest, ScrcpyOptions, SearchQuery, Settings, ShotFilter, SortKey, SrvAction, SrvFlavor, SrvKind, SrvNew, SrvPatch, TweakId } from '@shared/types';
import { type Args, bool, emit, listen, num, obj, optStr, str, strList } from './mock/bus';
import { scene } from './mock/art';
import * as core from './mock/core';
import * as games from './mock/games';
import { rig } from './mock/rig';
import { govee } from './mock/govee';
import { modsMock as mm } from './mock/mods';
import { chatsMock as cm, openaiMock as om } from './mock/chats';
import { pollMock } from './mock/mcCover';
import { serversMock as sv } from './mock/servers';
import * as optimize from './mock/optimize';
import * as updates from './mock/update';
import * as tools from './mock/tools';
import * as catalog from './mock/catalog';

let playInstalled = false;
let playRecording = false;
let playClips: { path: string; name: string; size: number; created: number }[] = [
  { path: 'C:\\Users\\Player\\Videos\\OmniHub Clips\\Fortnite highlight 2026-10-29 21-14-07.mp4', name: 'Fortnite highlight 2026-10-29 21-14-07', size: 48_200_000, created: Math.floor(Date.now() / 1000) - 86_400 },
  { path: 'C:\\Users\\Player\\Videos\\OmniHub Clips\\Counter-Strike 2 2026-10-28 18-02-51.mp4', name: 'Counter-Strike 2 2026-10-28 18-02-51', size: 61_900_000, created: Math.floor(Date.now() / 1000) - 2 * 86_400 },
];
let capriOn = false;
let capriInstalled = false;
const mockScanTask = { on: false };
import { volumes } from './mock/drives';
import * as media from './mock/media';
import * as music from './mock/music';
import * as visual from './mock/visual';
import * as notes from './mock/notes';
import * as remote from './mock/remote';
import { latency } from './mock/rng';
import * as storage from './mock/storage';
import * as vault from './mock/vault';

type Handler = (args: Args) => unknown;

const SORT_KEYS: readonly SortKey[] = ['size', 'alloc', 'name', 'modified', 'files'];
const sortKey = (args: Args): SortKey => {
  const v = args.sort;
  return SORT_KEYS.find((k) => k === v) ?? 'size';
};

const handlers: Record<string, Handler> = {
  // app
  app_info: () => core.appInfo,
  app_last_crash: () => (new URLSearchParams(window.location.search).get('crash') ? 'OmniHub 0.2.3 did not close normally last time.\n\nWindows crash reports (newest first):\n- uxplay.exe crashed in libgstd3d12.dll (exception c0000005)\n' : null),
  app_log_error: (args) => console.error(args.message),
  system_stats: () => core.systemStats(),
  storage_scan_task_status: () => mockScanTask.on,
  storage_scan_task_set: (a) => ((mockScanTask.on = bool(a, 'on')), mockScanTask.on),
  update_state: () => updates.info(),
  update_check: () => updates.check(),
  update_install: () => updates.install(),
  rig_specs: () => rig.specs(),
  rig_settings: () => rig.settings(),
  rig_update: (a) => rig.update(obj(a, 'patch')),
  rig_hold: () => rig.hold(),
  rig_profiles: () => rig.profiles(),
  rig_save_profile: (a) => rig.saveProfile(obj(a, 'profile')),
  rig_delete_profile: (a) => rig.deleteProfile(str(a, 'id')),
  rig_lhm_login: () => undefined,
  rig_start_openrgb: () => undefined,
  rig_media: () => rig.media(),
  rig_media_add: () => rig.addMedia(),
  rig_media_remove: (a) => rig.removeMedia(str(a, 'id')),
  rig_displays: () => rig.displays(),
  govee_settings: () => govee.settings(),
  govee_update: (a) => govee.update(obj(a, 'patch')),
  govee_update_light: (a) => govee.updateLight(str(a, 'id'), obj(a, 'patch')),
  govee_hold: () => govee.hold(),
  govee_scan: () => govee.scan(),
  govee_set_key: (a) => govee.setKey(str(a, 'key')),
  govee_control: (a) => govee.control(str(a, 'id'), str(a, 'instance'), a.value),
  govee_identify: (a) => govee.identify(str(a, 'id')),
  govee_presets: () => govee.presets(),
  govee_save_preset: (a) => govee.savePreset(obj(a, 'preset')),
  govee_delete_preset: (a) => govee.deletePreset(str(a, 'id')),
  govee_apply_preset: (a) => govee.applyPreset(str(a, 'id'), strList(a, 'lights')),
  govee_save_timer: (a) => govee.saveTimer(obj(a, 'timer')),
  govee_delete_timer: (a) => govee.deleteTimer(str(a, 'id')),
  govee_sleep: (a) => govee.sleep(typeof a.minutes === 'number' ? a.minutes : null),
  govee_monitors: () => govee.monitors(),
  govee_firewall: () => govee.firewall(),
  govee_fix_firewall: () => govee.fixFirewall(),
  games_list: () => games.list(),
  games_create: (a) => games.create(str(a, 'kind') as GameKind),
  games_save: (a) => games.save(obj<GameProfile>(a, 'profile')),
  games_delete: (a) => games.remove(str(a, 'id')),
  games_state: (a) => games.state(str(a, 'id')),
  games_play: (a) => games.play(str(a, 'id'), bool(a, 'launch')),
  games_stop: () => games.stop(),
  games_ping: (a) => games.ping(optStr(a, 'id'), optStr(a, 'host')),
  games_ping_targets: (a) => games.pingTargets(optStr(a, 'id')),
  games_roblox_status: () => games.robloxStatus(),
  games_roblox_write: (a) => games.robloxWrite(obj<RobloxFlags>(a, 'flags')),
  games_roblox_preview: (a) => games.preview(obj<RobloxFlags>(a, 'flags')),
  games_roblox_shaders: () => games.robloxShaders(),
  games_roblox_shaders_apply: (a) => games.robloxShadersApply(obj<RobloxShaders>(a, 'shaders')),
  games_roblox_flag_list: () => games.robloxFlagList(),
  games_configs: () => optimize.configs(),
  games_config_options: (a) => optimize.setOptions(obj<ConfigOptions>(a, 'options')),
  games_config_apply: (a) => optimize.apply(str(a, 'game') as ConfigGame),
  games_config_restore: (a) => optimize.restore(str(a, 'game') as ConfigGame),
  games_fortnite_clean: () => ({ freedBytes: 1_284_000_000, files: 2316, skipped: 3 }),
  displays_list: () => [
    { id: '\\\\.\\DISPLAY1#0', name: 'LG ULTRAGEAR', primary: true, builtin: false, brightness: 70, contrast: 75 },
    { id: '\\\\.\\DISPLAY2#0', name: 'DELL S2421H', primary: false, builtin: false, brightness: 55, contrast: 50 },
  ],
  displays_set: () => null,
  capri_status: (a) => ({ installed: capriOn || capriInstalled, version: capriOn || capriInstalled ? '0.4.4' : null, running: capriOn, latest: a.check ? '0.4.4' : null }),
  capri_set: (a) => {
    capriOn = !!a.on;
    if (capriOn) capriInstalled = true;
    return { installed: capriInstalled, version: capriInstalled ? '0.4.4' : null, running: capriOn, latest: null };
  },
  capri_update: () => ({ installed: true, version: '0.4.4', running: capriOn, latest: '0.4.4' }),
  play_status: () => ({
    supported: true,
    game: core.settings.play.replay || core.settings.play.voice || core.settings.play.capriReacts ? 'Fortnite' : null,
    since: Math.floor(Date.now() / 1000) - 754,
    recorderInstalled: playInstalled,
    installing: false,
    replaying: core.settings.play.replay && playInstalled,
    buffered: Math.min(core.settings.play.replayMinutes * 60, 754),
    recording: playRecording,
    encoder: core.settings.play.replay && playInstalled ? 'NVIDIA NVENC' : null,
    replayError: null,
    listening: core.settings.play.voice,
    voiceError: null,
    capriRunning: capriOn,
    folder: core.settings.play.folder ?? 'C:\\Users\\Player\\Videos\\OmniHub Clips',
    heard: core.settings.play.voice ? 'capri clip that' : null,
  }),
  play_install: () =>
    new Promise((r) => {
      let done = 0;
      const t = setInterval(() => {
        done += 9_000_000;
        emit('play:install', { done: Math.min(done, 92_000_000), total: 92_000_000 });
        if (done >= 92_000_000) {
          clearInterval(t);
          playInstalled = true;
          emit('play:changed', {});
          r(null);
        }
      }, 120);
    }),
  play_clip: (a) => {
    const name = `Fortnite ${new Date().toISOString().slice(0, 19).replace('T', ' ').replace(/:/g, '-')}`;
    playClips.unshift({ path: `C:\\Users\\Player\\Videos\\OmniHub Clips\\${name}.mp4`, name, size: num(a, 'seconds') * 1_900_000, created: Math.floor(Date.now() / 1000) });
    emit('play:clip', { path: playClips[0].path, seconds: num(a, 'seconds') });
    return playClips[0].path;
  },
  play_record: (a) => {
    playRecording = bool(a, 'start');
    emit('play:changed', {});
    return null;
  },
  play_clips: () => playClips,
  play_clip_delete: (a) => {
    playClips = playClips.filter((c) => c.path !== str(a, 'path'));
    return null;
  },
  play_hello: () => {
    if (!capriOn) throw new Error("Capri isn't running.");
    return null;
  },
  games_library: () => optimize.library(),
  games_add_installed: (a) => optimize.addInstalled(str(a, 'key')),
  games_load_test: (a) => games.loadTest(optStr(a, 'id')),
  games_fps: () => games.fpsOverview(),
  games_fps_install: () => games.fpsInstall(),
  games_fps_allow: () => games.fpsAllow(),
  pc_status: () => optimize.pcStatus(),
  pc_set: (a) => optimize.pcSet(str(a, 'id') as TweakId, bool(a, 'on')),
  media_state: () => music.mediaState(),
  media_control: (a) => music.mediaControl(str(a, 'action') as MediaAction, num(a, 'positionMs', 0)),
  media_lyrics: () => music.mediaLyrics(),
  media_video: () => music.mediaVideo(),
  // The demo has no yt-dlp: a test can hand it a stream (localStorage "omnihub.demo.stream").
  media_video_stream: () => {
    const url = localStorage.getItem('omnihub.demo.stream');
    if (!url) throw new Error('Playing videos directly needs the Windows app.');
    return url;
  },
  // A test can hand the demo an animated cover (localStorage "omnihub.demo.motion").
  media_motion: () => {
    const url = localStorage.getItem('omnihub.demo.motion');
    return { key: music.mediaVideo().key, motion: url ? { status: 'found', url } : { status: 'none' } };
  },
  // A test can hand the demo a font and an emoji pack (localStorage "omnihub.demo.font" and
  // "omnihub.demo.emojiFont", data URLs, and "omnihub.demo.textures", a JSON map of name →
  // data URL); nothing is shipped.
  aurora_look: () => {
    const textures = demoTextures();
    return { font: localStorage.getItem('omnihub.demo.font') ? 'Demo font' : null, textures: Object.keys(textures).sort(), emojiFont: localStorage.getItem('omnihub.demo.emojiFont') ? 'Demo emoji font' : null };
  },
  aurora_emoji_font: () => localStorage.getItem('omnihub.demo.emojiFont'),
  aurora_choose_font: () => {
    throw new Error('Choosing a font needs the Windows app.');
  },
  aurora_choose_emoji_pack: () => {
    throw new Error('Choosing an emoji pack needs the Windows app.');
  },
  aurora_forget_look: (a) => {
    if (bool(a, 'font')) localStorage.removeItem('omnihub.demo.font');
    if (bool(a, 'textures')) {
      localStorage.removeItem('omnihub.demo.textures');
      localStorage.removeItem('omnihub.demo.emojiFont');
    }
  },
  aurora_font: () => localStorage.getItem('omnihub.demo.font'),
  aurora_texture: (a) => demoTextures()[str(a, 'name')] ?? null,
  media_mirror: () => ({ state: 'unavailable', kind: null, app: null, width: 0, height: 0 }),
  media_mirror_frame: () => null,
  // A test can hand the demo a matched offset (localStorage "omnihub.demo.sync", in ms).
  media_video_sync: () => {
    const ms = localStorage.getItem('omnihub.demo.sync');
    return { key: music.mediaVideo().key, sync: ms === null ? { state: 'off' } : { state: 'matched', offsetMs: Number(ms), score: 0.8 } };
  },
  media_stream_status: () => ({ installed: !!localStorage.getItem('omnihub.demo.stream'), installing: false }),
  media_art: (a) => music.mediaArt(str(a, 'id')),
  media_audio: () => music.mediaAudio(),
  media_set_volume: (a) => music.mediaSetVolume(a.level == null ? null : num(a, 'level'), a.muted == null ? null : bool(a, 'muted')),
  visual_hold: (a) => visual.visualHold(str(a, 'who')),
  visual_release: (a) => visual.visualRelease(str(a, 'who')),
  visual_status: () => visual.visualStatus(),
  visual_keep_awake: () => undefined,
  ambient_displays: () => [
    { name: '\\\\.\\DISPLAY1', label: 'Display 1 · 2560×1440 (main)', width: 2560, height: 1440, scale: 1.25, primary: true },
    { name: '\\\\.\\DISPLAY2', label: 'Display 2 · 1920×1080', width: 1920, height: 1080, scale: 1, primary: false },
  ],
  screen_share_state: () => core.shareState(),
  screen_windows: () => core.shareWindows(),
  screen_set_paused: (a) => core.setSharePaused(bool(a, 'on')),
  screen_set_window: (a) => core.setShareWindow(a.id == null ? null : num(a, 'id')),
  startup_list: () => core.startupList(),
  startup_set: (a) => core.startupSet(str(a, 'id'), bool(a, 'enabled')),
  system_processes: (a) => core.processes((optStr(a, 'sort') ?? 'cpu') as ProcessSort, num(a, 'limit') || 8),
  system_end_process: (a) => core.endProcess(str(a, 'name')),
  system_usage: (a) => core.usage((optStr(a, 'sort') ?? 'cpu') as ProcessSort, num(a, 'limit') || 200),
  system_set_priority: (a) => core.setPriority(str(a, 'name'), str(a, 'priority') as Priority),
  settings_get: () => core.settings,
  settings_update: (a) => core.updateSettings(obj<DeepPartial<Settings>>(a, 'patch')),
  settings_export: (a) => void str(a, 'path'),
  settings_import: (a) => (void str(a, 'path'), core.settings),
  open_path: (a) => void str(a, 'path'),
  reveal_path: (a) => void str(a, 'path'),
  open_url: (a) => void str(a, 'url'),
  restart_elevated: () => {
    core.audit('desktop', 'app.elevate', 'Restart as administrator requested');
  },
  audit_list: (a) => core.auditLog.slice(num(a, 'offset', 0), num(a, 'offset', 0) + num(a, 'limit', 100)),
  audit_clear: () => {
    core.auditLog.length = 0;
  },
  pick_folder: (a) => core.pickFolder(optStr(a, 'title')),
  pick_files: () => core.pickFiles(),
  pick_save_file: (a) => `C:\\Users\\Player\\Documents\\${str(a, 'defaultName')}`,
  set_autostart: (a) => {
    core.updateSettings({ general: { launchAtLogin: bool(a, 'enabled') } });
  },
  hide_window: () => undefined,

  // storage
  storage_volumes: () => volumes(),
  storage_scan: (a) => storage.scanStart(obj<ScanRequest>(a, 'request')),
  storage_progress: (a) => storage.scanProgress(str(a, 'jobId')),
  storage_cancel: (a) => storage.scanCancel(str(a, 'jobId')),
  storage_scans: () => storage.scanList(),
  storage_summary: (a) => storage.scanSummary(str(a, 'scanId')),
  storage_open_cached: (a) => storage.openCached(str(a, 'root')),
  storage_children: (a) => storage.children(str(a, 'scanId'), num(a, 'node'), sortKey(a), bool(a, 'descending', true), num(a, 'offset', 0), num(a, 'limit', 500)),
  storage_treemap: (a) => storage.treemap(str(a, 'scanId'), num(a, 'node'), num(a, 'depth', 3), num(a, 'maxItems', 1500)),
  storage_thumb: (a) => {
    const path = str(a, 'path');
    return /\.(jpe?g|png|gif|webp|bmp)$/i.test(path) ? scene(path, (['game', 'desktop', 'browser', 'code'] as const)[path.length % 4], 320, 200) : null;
  },
  storage_top_files: (a) => storage.topFiles(str(a, 'scanId'), num(a, 'node'), num(a, 'n', 100)),
  storage_growth: (a) => storage.growth(str(a, 'scanId'), num(a, 'limit', 15)),
  storage_export_csv: (a) => storage.exportCsv(str(a, 'scanId'), num(a, 'node'), str(a, 'kind'), str(a, 'path')),
  storage_extensions: (a) => storage.extensions(str(a, 'scanId'), num(a, 'node')),
  storage_search: (a) => storage.search(str(a, 'scanId'), obj<SearchQuery>(a, 'query')),
  storage_path: (a) => storage.nodePath(str(a, 'scanId'), num(a, 'node')),
  storage_cleanup: (a) => storage.cleanup(str(a, 'scanId')),
  storage_delete: (a) => storage.deletePaths(strList(a, 'paths'), bool(a, 'permanent')),
  storage_empty_recycle_bin: () => storage.emptyRecycleBin(),
  storage_duplicates_start: (a) => storage.dupesStart(str(a, 'scanId'), obj<DupeOptions>(a, 'options')),
  storage_duplicates_progress: (a) => storage.dupesProgress(str(a, 'jobId')),
  storage_duplicates_result: (a) => storage.dupesResult(str(a, 'jobId')),
  storage_duplicates_cancel: (a) => storage.dupesCancel(str(a, 'jobId')),

  // apps
  apps_list: () => media.appsList(),
  apps_icon: (a) => media.appIconFor(str(a, 'id')),
  apps_launch: (a) => media.appLaunch(str(a, 'id')),
  apps_uninstall: (a) => media.appUninstall(str(a, 'id')),
  apps_screenshots: (a) => media.appScreenshots(str(a, 'id')),

  // screenshots
  shots_list: (a) => media.shotsList(obj<ShotFilter>(a, 'filter')),
  shots_capture: (a) => media.shotCapture(str(a, 'kind') as CaptureKind, num(a, 'delaySeconds', 0)),
  shots_region_begin: () => media.regionBegin(),
  shots_region_pending: () => media.regionPending(),
  shots_region_commit: (a) => media.regionCommit(str(a, 'id'), obj<Rect>(a, 'rect')),
  shots_region_cancel: () => media.regionCancel(),
  shots_thumb: (a) => media.shotThumb(str(a, 'id')),
  shots_image: (a) => media.shotImage(str(a, 'id')),
  shots_pin: () => null,
  vault_import_file: () => [42, 45],
  vault_wifi_qr: () => '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 21 21"><rect width="21" height="21" fill="#fff"/><path d="M0 0h7v7H0zM14 0h7v7h-7zM0 14h7v7H0zM9 9h3v3H9z"/></svg>',
  apps_updates: () => [
    { name: 'Mozilla Firefox (x64)', id: 'Mozilla.Firefox', version: '127.0', available: '128.0.3', source: 'winget' },
    { name: 'Discord', id: 'Discord.Discord', version: '1.0.9150', available: '1.0.9160', source: 'winget' },
    { name: '7-Zip', id: '7zip.7zip', version: '23.01', available: '24.08', source: 'winget' },
  ],
  apps_update: () => new Promise((r) => setTimeout(() => r(null), 900)),
  shots_update: (a) => media.shotUpdate(str(a, 'id'), { tags: Array.isArray(a.tags) ? strList(a, 'tags') : null, note: optStr(a, 'note'), favorite: typeof a.favorite === 'boolean' ? a.favorite : null }),
  shots_delete: (a) => media.shotDelete(str(a, 'id')),
  shots_copy: (a) => void str(a, 'id'),
  shots_text: (a) => media.shotText(str(a, 'id')),
  shots_save_edit: (a) => media.shotSaveEdit(str(a, 'id'), str(a, 'png')),
  shots_sync: () => media.shotsSync(),

  // notes
  notes_list: (a) => notes.notesList(obj<NoteFilter>(a, 'filter')),
  notes_get: (a) => notes.noteGet(str(a, 'id')),
  notes_save: (a) => notes.noteSave(obj<NoteInput>(a, 'input')),
  notes_read_locked: (a) => notes.noteReadLocked(str(a, 'id')),
  notes_set_locked: (a) => notes.noteSetLocked(str(a, 'id'), bool(a, 'locked')),
  notes_delete: (a) => notes.noteDelete(str(a, 'id')),
  notes_remind: (a) => notes.noteRemind(str(a, 'id'), a.at == null ? null : num(a, 'at')),
  notes_tags: () => notes.noteTags(),
  notes_export: (a) => notes.noteExport(str(a, 'id')),
  notes_folder_files: () => notes.folderFiles(),
  notes_folder_read: (a) => notes.folderRead(str(a, 'path')),

  // Aurora's music hub
  music_search: (a) => catalog.search(str(a, 'query')),
  music_artist: (a) => catalog.artist(str(a, 'id')),
  music_album: (a) => catalog.albumPage(str(a, 'id')),
  music_up_next: (a) => catalog.upNext(str(a, 'title'), str(a, 'artist')),
  music_play: (a) => catalog.play(obj<PlayItem>(a, 'item'), str(a, 'source') as MusicSource),
  music_installed: () => ({ spotify: true, appleMusic: false }),

  // file tools, ports, layouts, privacy
  files_hash: (a) => tools.hash(str(a, 'path'), str(a, 'kind') as HashKind, str(a, 'job')),
  files_shred: (a) => tools.shred(str(a, 'path'), str(a, 'job')),
  files_rename_plan: (a) => tools.renamePlan(strList(a, 'files'), obj<RenameRule>(a, 'rule')),
  files_rename: (a) => tools.rename(strList(a, 'files'), obj<RenameRule>(a, 'rule')),
  files_recent: () => tools.recent(),
  ports_list: () => tools.ports(),
  ports_end: (a) => tools.endPort(num(a, 'pid')),
  layouts_list: () => tools.layouts(),
  layouts_save: (a) => tools.saveLayout(str(a, 'name')),
  layouts_apply: (a) => tools.applyLayout(str(a, 'name'), bool(a, 'launch')),
  layouts_delete: (a) => tools.removeLayout(str(a, 'name')),
  privacy_list: () => tools.privacy(),
  panic_now: () => {
    vault.lock('manual');
    emit('app:panic', { by: 'desktop' });
  },

  // vault
  vault_status: () => vault.status(),
  hidden_list: () => vault.hiddenList(),
  hidden_hide: (a) => vault.hide(str(a, 'path'), str(a, 'job')),
  hidden_unhide: (a) => vault.unhide(str(a, 'id'), optStr(a, 'folder'), str(a, 'job')),
  hidden_save_copy: (a) => vault.hiddenCopy(str(a, 'id'), str(a, 'job')),
  hidden_open: (a) => vault.hiddenCopy(str(a, 'id'), str(a, 'job')),
  hidden_delete: (a) => vault.hiddenRemove(str(a, 'id')),
  hidden_cancel: (a) => vault.hiddenCancel(str(a, 'job')),
  vault_create: (a) => vault.create(str(a, 'password')),
  vault_unlock: (a) => vault.unlock(str(a, 'password')),
  vault_lock: () => vault.lock('manual'),
  vault_touch: () => vault.touch(),
  vault_list: () => vault.list(),
  vault_get: (a) => vault.get(str(a, 'id')),
  vault_save: (a) => vault.save(obj<EntryInput>(a, 'input')),
  vault_delete: (a) => vault.remove(str(a, 'id')),
  vault_copy: (a) => vault.copy(str(a, 'id'), str(a, 'field')),
  vault_generate: (a) => vault.generate(obj<GeneratorOptions>(a, 'options')),
  vault_strength: (a) => vault.strength(str(a, 'password')),
  vault_change_password: (a) => vault.changePassword(str(a, 'oldPassword'), str(a, 'newPassword')),
  vault_hello_enable: () => vault.helloEnable(),
  vault_hello_disable: () => vault.helloDisable(),
  vault_hello_unlock: () => vault.helloUnlock(),
  vault_export: (a) => void str(a, 'path'),
  vault_import: (a) => vault.importBackup(str(a, 'password')),
  vault_totp: (a) => vault.totp(str(a, 'id')),
  vault_health: () => vault.health(),
  vault_breach_check: () => vault.breachCheck(),

  // browser autofill
  browser_status: () => vault.browserStatus(),
  browser_repair: () => vault.browserStatus().browsers,
  browser_open_page: () => null,
  browser_pair_respond: (a) => vault.browserRespond(str(a, 'id'), bool(a, 'allow')),
  browser_revoke: (a) => vault.browserRevoke(str(a, 'id')),

  // phone companion
  remote_status: () => remote.status(),
  remote_start: () => {
    core.updateSettings({ remote: { enabled: true } });
    return remote.status();
  },
  remote_stop: () => {
    core.updateSettings({ remote: { enabled: false } });
  },
  remote_pair_begin: () => remote.pairBegin(),
  remote_pair_cancel: () => remote.pairCancel(),
  remote_devices: () => remote.deviceList(),
  remote_device_rename: (a) => remote.renameDevice(str(a, 'id'), str(a, 'name')),
  remote_device_revoke: (a) => remote.revokeDevice(str(a, 'id')),
  remote_send: (a) => remote.send(strList(a, 'paths'), optStr(a, 'deviceId')),
  remote_inbox: () => remote.inboxList(),
  remote_send_text: (a) => remote.sendText(str(a, 'text'), optStr(a, 'deviceId')),
  remote_inbox_remove: (a) => remote.inboxRemove(str(a, 'id')),
  clipboard_text: () => 'https://www.youtube.com/watch?v=dQw4w9WgXcQ',
  take_pending_send: () => [],
  remote_stop_viewer: (a) => remote.stopViewer(str(a, 'id')),
  remote_stop_all_viewers: () => remote.stopAllViewers(),
  remote_diagnostics: () => remote.diagnostics(),
  remote_fix_firewall: (a) => remote.fixFirewall(bool(a, 'includePublic')),
  network_make_private: (a) => remote.makeNetworkPrivate(str(a, 'id')),

  // power
  power_schedule: (a) => remote.powerSchedule(str(a, 'action') as PowerAction, num(a, 'delaySeconds', 10)),
  power_cancel: () => remote.powerCancel(),
  power_pending: () => remote.powerPending(),

  // screen sharing
  screen_monitors: () => remote.monitors,
  screen_presets: () => remote.presets,
  scrcpy_status: () => remote.scrcpyStatus(),
  scrcpy_launch: (a) => remote.scrcpyLaunch(obj<ScrcpyOptions>(a, 'options')),
  scrcpy_stop: () => remote.scrcpyStop(),
  scrcpy_wireless: (a) => remote.scrcpyWireless(str(a, 'serial')),
  scrcpy_connect: (a) => remote.scrcpyConnect(str(a, 'addr')),
  scrcpy_pair: (a) => remote.scrcpyPair(str(a, 'addr'), str(a, 'code')),
  sunshine_status: () => remote.sunshineStatus(),
  airplay_status: () => remote.airplayStatus(),
  airplay_install: () => remote.airplayInstall(),
  airplay_uninstall: () => remote.airplayUninstall(),
  airplay_start: () => remote.airplayStart(),
  airplay_stop: () => remote.airplayStop(),
  airplay_keep_on_top: (a) => void core.updateSettings({ screen: { airplayKeepOnTop: bool(a, 'on') } }),
  airplay_place: () => undefined,
  airplay_firewall: () => remote.airplayFirewall(),
  airplay_fix_firewall: () => remote.airplayFixFirewall(),

  // mods
  mods_games: () => mm.games(),
  mods_list: (a) => mm.list(str(a, 'key')),
  mods_browse: (a) => mm.browse(str(a, 'key'), str(a, 'query'), optStr(a, 'category'), str(a, 'sort') as ModSort, num(a, 'offset', 0), num(a, 'limit', 30)),
  mods_install_package: (a) => mm.installPackage(str(a, 'key'), str(a, 'fullName')),
  mods_install_file: (a) => mm.installFile(str(a, 'key'), str(a, 'path')),
  mods_install_loader: (a) => mm.installLoader(str(a, 'key'), str(a, 'kind')),
  mods_set_enabled: (a) => mm.setEnabled(str(a, 'id'), bool(a, 'on')),
  mods_set_all: (a) => mm.setAll(str(a, 'key'), bool(a, 'on')),
  mods_uninstall: (a) => mm.uninstall(str(a, 'id')),
  mods_update: (a) => mm.update(str(a, 'id')),
  mods_workshop: (a) => mm.workshop(str(a, 'key')),
  mods_add_folder: (a) => mm.addFolder(str(a, 'name'), str(a, 'dir')),
  mods_remove_folder: (a) => mm.removeFolder(str(a, 'key')),
  chats_list: () => cm.list(),
  chats_view: (a) => cm.view(str(a, 'source') as ChatSource, str(a, 'id')),
  chats_send: (a) => cm.send(str(a, 'source') as ChatSource, str(a, 'id'), str(a, 'text')),
  chats_create: (a) => cm.create(str(a, 'kind') as NewChatKind, str(a, 'text'), optStr(a, 'folder')),
  chats_stop: (a) => cm.stop(str(a, 'source') as ChatSource, str(a, 'id')),
  chats_rename: (a) => cm.rename(str(a, 'id'), str(a, 'title')),
  chats_delete: (a) => cm.remove(str(a, 'id')),
  chats_folders: () => cm.folders(),
  chats_tools: () => cm.tools(),
  chats_usage: () => cm.usage(),
  ideas_list: () => cm.ideas(),
  ideas_save: (a) => cm.saveIdea(str(a, 'text'), optStr(a, 'from')),
  openai_settings: () => om.settings(),
  openai_update: (a) => om.update(obj(a, 'patch')),
  openai_set_key: (a) => om.setKey(str(a, 'key')),
  openai_models: () => om.models(),
  claude_settings: () => mm.claudeSettings(),
  claude_update: (a) => mm.claudeUpdate(obj(a, 'patch')),
  claude_set_key: (a) => mm.claudeSetKey(str(a, 'key')),
  claude_test: () => mm.claudeTest(),
  maker_projects: (a) => mm.projects(optStr(a, 'game')),
  maker_create: (a) => mm.create(str(a, 'game'), str(a, 'text')),
  maker_send: (a) => mm.send(str(a, 'id'), str(a, 'text')),
  maker_stop: (a) => mm.stop(str(a, 'id')),
  maker_approve: () => mm.approve(),
  maker_view: (a) => mm.view(str(a, 'id')),
  maker_file: (a) => mm.file(str(a, 'id'), str(a, 'path')),
  maker_delete: (a) => mm.remove(str(a, 'id')),
  maker_rename: (a) => mm.rename(str(a, 'id'), str(a, 'name')),
  maker_install: (a) => mm.install(str(a, 'id')),
  mc_status: () => mm.mcStatus(),
  mc_search: (a) => mm.mcSearch(str(a, 'query')),
  mc_versions: (a) => mm.mcVersions(str(a, 'project')),
  mc_add_modrinth: (a) => mm.mcAddModrinth(str(a, 'project'), str(a, 'version')),
  mc_add_file: (a) => mm.mcAddFile(str(a, 'path')),
  mc_add_folder: (a) => mm.mcAddFolder(str(a, 'path')),
  mc_launcher_packs: () => mm.mcLauncherPacks(),
  mc_launcher_icon: (a) => mm.mcLauncherIcon(str(a, 'id')),
  mc_add_launcher_pack: (a) => mm.mcAddLauncherPack(str(a, 'id')),
  mc_add_launcher_place: (a) => mm.mcAddLauncherPlace(str(a, 'path')),
  mc_remove_launcher_place: (a) => mm.mcRemoveLauncherPlace(str(a, 'path')),
  mc_remove_source: (a) => mm.mcRemoveSource(str(a, 'id')),
  mc_plan: (a) => mm.mcPlan(str(a, 'name'), strList(a, 'ids'), str(a, 'mc'), str(a, 'loader')),
  mc_export: () => mm.mcExport(),
  mc_import_shared: (a) => mm.mcImportShared(str(a, 'path')),
  mc_launcher_details: (a) => mm.mcLauncherDetails(str(a, 'id')),
  mc_launcher_world_icon: (a) => mm.mcLauncherWorldIcon(str(a, 'id'), str(a, 'folder')),
  mc_launcher_updates: (a) => mm.mcLauncherUpdates(str(a, 'id')),
  mc_share_launcher_pack: (a) => mm.mcShareLauncherPack(str(a, 'id')),
  mc_install: (a) => mm.mcInstall(str(a, 'id')),
  mc_uninstall: (a) => mm.mcUninstall(str(a, 'profile')),
  mc_cover: (a) => mm.mcCover(str(a, 'id')),
  mc_remake_cover: (a) => mm.mcRemakeCover(str(a, 'id')),
  mc_ai_cover: (a) => mm.mcAiCover(str(a, 'id'), pollMock.status().connected),
  mc_save_cover: () => undefined,
  mc_mixes: () => mm.mcMixes(),
  pollinations_status: () => pollMock.status(),
  pollinations_sign_in: () => pollMock.signIn(),
  pollinations_check: () => pollMock.check(),
  pollinations_set_key: (a) => pollMock.setKey(str(a, 'key')),
  pollinations_forget: () => pollMock.forget(),
  srv_list: () => sv.list(),
  srv_get: (a) => sv.get(str(a, 'id')),
  srv_versions: (a) => sv.versions(str(a, 'flavor') as SrvFlavor),
  srv_create: (a) => sv.create(obj<SrvNew>(a, 'server')),
  srv_adopt: (a) => sv.adopt(str(a, 'path')),
  srv_update: (a) => sv.update(str(a, 'id'), obj<SrvPatch>(a, 'patch')),
  srv_delete: (a) => sv.remove(str(a, 'id')),
  srv_start: (a) => sv.start(str(a, 'id')),
  srv_stop: (a) => sv.stop(str(a, 'id')),
  srv_restart: (a) => sv.restart(str(a, 'id')),
  srv_kill: (a) => sv.kill(str(a, 'id')),
  srv_console: (a) => sv.console(str(a, 'id'), a.after === null || a.after === undefined ? null : num(a, 'after')),
  srv_command: (a) => sv.command(str(a, 'id'), str(a, 'text')),
  srv_properties: (a) => sv.properties(str(a, 'id')),
  srv_set_properties: (a) => sv.setProperties(str(a, 'id'), obj<[string, string][]>(a, 'changes')),
  srv_set_icon: (a) => sv.setIcon(str(a, 'id')),
  srv_players: (a) => sv.players(str(a, 'id')),
  srv_player: (a) => sv.player(str(a, 'id'), str(a, 'name')),
  srv_act: (a) => sv.act(str(a, 'id'), str(a, 'name'), obj<SrvAction>(a, 'action')),
  srv_whitelist: (a) => sv.whitelist(str(a, 'id'), bool(a, 'on')),
  srv_watch: (a) => sv.watch(str(a, 'id')),
  srv_map: (a) => sv.map(str(a, 'id'), str(a, 'dim')),
  srv_map_tile: (a) => sv.mapTile(str(a, 'id'), str(a, 'dim'), num(a, 'x'), num(a, 'z')),
  srv_content: (a) => sv.content(str(a, 'id')),
  srv_identify: (a) => sv.identify(str(a, 'id')),
  srv_search: (a) => sv.search(str(a, 'id'), str(a, 'kind') as SrvKind, str(a, 'query')),
  srv_install: (a) => sv.install(str(a, 'id'), str(a, 'kind') as SrvKind, str(a, 'project')),
  srv_add_file: (a) => sv.addFile(str(a, 'id'), str(a, 'path')),
  srv_set_content: (a) => sv.setContent(str(a, 'id'), str(a, 'file'), bool(a, 'on')),
  srv_remove_content: (a) => sv.removeContent(str(a, 'id'), str(a, 'file')),
  srv_backups: (a) => sv.backups(str(a, 'id')),
  srv_backup: (a) => sv.backup(str(a, 'id')),
  srv_restore: (a) => sv.restore(str(a, 'id'), str(a, 'name')),
  srv_delete_backup: (a) => sv.removeBackup(str(a, 'id'), str(a, 'name')),
  srv_network: (a) => sv.network(str(a, 'id')),
  srv_set_upnp: (a) => sv.setUpnp(str(a, 'id'), bool(a, 'on')),
  srv_fix_firewall: (a) => sv.fixFirewall(str(a, 'id')),
};

const FAST = new Set(['vault_strength', 'vault_touch', 'vault_status', 'system_stats', 'vault_generate', 'storage_progress', 'storage_duplicates_progress', 'remote_status']);

function demoTextures(): Record<string, string> {
  try {
    return JSON.parse(localStorage.getItem('omnihub.demo.textures') ?? '{}') ?? {};
  } catch {
    return {};
  }
}

export async function mockInvoke(cmd: string, args: Record<string, unknown>): Promise<unknown> {
  const handler = handlers[cmd];
  if (!handler) throw `mock: unknown command ${cmd}`;
  await (FAST.has(cmd) ? latency(5, 25) : latency());
  try {
    const result = await handler(args);
    return result === undefined ? null : structuredClone(result);
  } catch (e) {
    // Tauri rejects with the error string.
    throw e instanceof Error ? e.message : String(e);
  }
}

export function mockListen(topic: string, handler: (payload: unknown) => void): () => void {
  return listen(topic, handler);
}

export function mockEmit(topic: string, payload: unknown): void {
  emit(topic, payload);
}
