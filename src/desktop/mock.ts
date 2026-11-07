// In-browser mock of the Rust backend, used when the UI runs outside Tauri
// (`npm run dev` in a normal browser, screenshots, UI development). Every
// command in api.ts is implemented against realistic, seeded in-memory data.
// Like Tauri, failures reject with a plain string.

import type { DownloadMode, ImageJob, VideoJob, FolderRule, HashKind, MusicSource, PlayItem, RenameRule, CaptureKind, ChatSource, ConfigGame, ConfigOptions, DeepPartial, DupeOptions, EntryInput, GameKind, GameProfile, GeneratorOptions, MediaAction, ModSort, NewChatKind, NoteFilter, NoteInput, PowerAction, Priority, ProcessSort, Rect, RobloxFlags, RobloxShaders, RomOpts, ScanRequest, ScrcpyOptions, SearchQuery, Settings, ShotFilter, SortKey, SrvAction, SrvFlavor, SrvKind, SrvNew, SrvPatch, TweakId } from '@shared/types';
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
import * as prod from './mock/productivity';
import * as sensors from './mock/sensors';

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
import { autoclick, counters as ctr, pkm } from './mock/counters';
import { emulators as emus, library as lib } from './mock/library';
import { quiet as quietMock } from './mock/quiet';
import { karaoke as karaokeMock } from './mock/karaoke';
// A plain original shape standing in for a picture you'd pick (black ink on white).
const SAMPLE_ART = `data:image/svg+xml;utf8,${encodeURIComponent("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 300 500'><rect width='300' height='500' fill='white'/><path d='M150 40 l26 60 -10 4 22 120 40 260 h-156 l40 -260 22 -120 -10 -4z' fill='black'/><path d='M60 470 L250 120' stroke='black' stroke-width='6'/></svg>")}`;
const mockArt: Record<string, string | null> = {};
let mockSites = [
  { id: 'netflix', name: 'Netflix', url: 'https://www.netflix.com/browse', color: '#e50914' },
  { id: 'youtube', name: 'YouTube', url: 'https://www.youtube.com', color: '#ff0033' },
  { id: 'crunchyroll', name: 'Crunchyroll', url: 'https://www.crunchyroll.com', color: '#f47521' },
  { id: 'disney', name: 'Disney+', url: 'https://www.disneyplus.com', color: '#113ccf' },
  { id: 'hbomax', name: 'HBO Max', url: 'https://play.hbomax.com', color: '#5b2ee5' },
  { id: 'prime', name: 'Prime Video', url: 'https://www.primevideo.com', color: '#1399ff' },
  { id: 'twitch', name: 'Twitch', url: 'https://www.twitch.tv', color: '#9146ff' },
];
let mockOutputs = [
  { id: 'spk', name: 'Speakers (Realtek(R) Audio)', default: true },
  { id: 'hp', name: 'Headphones (WH-1000XM5)', default: false },
  { id: 'tv', name: 'LG TV (NVIDIA High Definition Audio)', default: false },
];
let mockNight = { settings: { enabled: false, from: '21:30', to: '07:00', kelvin: 3400, dim: 100, fade: 30, screenBrightness: null as number | null }, status: { level: 0, kelvin: 6500, manual: null as boolean | null } };
let mockRun = {
  settings: { game: 'Celeste', category: 'Any%', splits: [{ name: 'Forsaken City', pb: 132000, gold: 128500 }, { name: 'Old Site', pb: 301000, gold: 162000 }, { name: 'Celestial Resort', pb: 512000, gold: 199000 }, { name: 'Golden Ridge', pb: 703000, gold: 185000 }], keySplit: 0x61, keyReset: 0x63, keyPause: 0x62, keyUndo: 0x68, attempts: 41 },
  run: { since: null as number | null, banked: 0, times: [] as number[], finished: false, newPb: false },
};
let mockFloats = { widgets: [] as { id: string; kind: 'clock' | 'temps' | 'music' | 'counter' | 'battery' | 'network'; x: number; y: number }[], crosshair: false, wallpaper: false };
const mockLimits: Record<string, string[]> = {};
let mockTimers = { timers: [] as { id: string; label: string; endsAt: number; minutes: number }[], alarms: [{ id: 'a1', label: 'School', time: '07:00', days: [0, 1, 2, 3, 4], enabled: true, fired: '' }], bedtime: { enabled: false, from: '23:30', to: '06:30', warnMinutes: 10, days: [] as number[] }, snoozedUntil: 0 };
let mockWins = [
  { handle: 1, title: 'Hollow Knight speedrun guide - YouTube', app: 'chrome.exe', pinned: false },
  { handle: 2, title: 'Discord', app: 'discord.exe', pinned: false },
  { handle: 3, title: 'Notes.txt - Notepad', app: 'notepad.exe', pinned: true },
];
let mockAfter = { action: '', quietSince: 0 };
let mockSnippets = { keysOn: true, snippets: [{ id: 'a1', name: 'Good game', text: 'gg wp, thanks for the games!', key: 0x7c }] };
let emuOffers = [
  { id: 'duckstation', name: 'DuckStation', platforms: ['psx'], installed: null as string | null, latest: null as string | null },
  { id: 'pcsx2', name: 'PCSX2', platforms: ['ps2'], installed: null, latest: null },
  { id: 'melonds', name: 'melonDS', platforms: ['nds'], installed: '1.0', latest: null },
  { id: 'mgba', name: 'mGBA', platforms: ['gba', 'gb', 'gbc'], installed: null, latest: null },
  { id: 'cemu', name: 'Cemu', platforms: ['wiiu'], installed: null, latest: null },
  { id: 'azahar', name: 'Azahar', platforms: ['3ds'], installed: null, latest: null },
  { id: 'rpcs3', name: 'RPCS3', platforms: ['ps3'], installed: null, latest: null },
];
let mockRaUser: string | null = null;
let mockHome = { url: '', connected: false, shelly: [] as [string, string][] };
let mockPeers = [{ id: 'p1', name: 'LIVING-ROOM-PC', url: 'https://192.168.1.31:7878', fingerprint: '3F:A2:91:0C' }];
let mockBench = [{ at: Math.floor(Date.now() / 1000) - 86400 * 7, single: 1834.2, multi: 21877, memory: 24.35, threads: 16, cpuTemp: 68 }];
let mockAppLimits = [{ exe: 'OneDrive.exe', kbps: 2000, active: true }];
let mockBoard = { sounds: [{ id: 's1', name: 'Air horn', path: 'C:\\Sounds\\air horn.mp3', key: 0x61, volume: 80 }, { id: 's2', name: 'Bruh', path: 'C:\\Sounds\\bruh.wav', key: 0, volume: 100 }], output: '', keysOn: true };
let mockBt = [
  { address: 'B8:F6:53:CA:1B:2E', name: 'WH-1000XM5', connected: true, kind: 'audio' as const, audio: true },
  { address: '7C:96:D2:11:22:33', name: 'JBL Flip 6', connected: false, kind: 'audio' as const, audio: true },
  { address: 'E4:17:D8:44:55:66', name: 'DualSense Wireless Controller', connected: false, kind: 'gamepad' as const, audio: false },
  { address: 'C8:3F:26:77:88:99', name: 'MX Master 3S', connected: true, kind: 'mouse' as const, audio: false },
];
import { appUpdatesMock, capriDesktop, heatMock, partyMock, screenTimeMock, voiceMock, watchMock } from './mock/extras';
import { backups as bk, batteries as bats, clipboard as clip, discord, health as pcHealth, network, obs, quick as qk } from './mock/quick';

/** What a key is called (the same names the app uses). */
function keyLabel(vk: number): string {
  const named: Record<number, string> = { 8: 'Backspace', 9: 'Tab', 13: 'Enter', 27: 'Esc', 32: 'Space', 33: 'Page Up', 34: 'Page Down', 35: 'End', 36: 'Home', 37: 'Left', 38: 'Up', 39: 'Right', 40: 'Down', 45: 'Insert', 46: 'Delete' };
  if (named[vk]) return named[vk];
  if ((vk >= 48 && vk <= 57) || (vk >= 65 && vk <= 90)) return String.fromCharCode(vk);
  if (vk >= 96 && vk <= 105) return `Num ${vk - 96}`;
  if (vk >= 112 && vk <= 135) return `F${vk - 111}`;
  return `Key ${vk}`;
}

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
  games_fps_cap: (a) => games.setFpsCap(str(a, 'id'), num(a, 'fps')),
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
  games_gpu_drivers: () => [{ name: 'NVIDIA GeForce RTX 4070', version: '32.0.15.6094', date: Math.floor(Date.now() / 1000) - 140 * 86400, daysOld: 140, updateUrl: 'https://www.nvidia.com/en-us/geforce/drivers/', changed: true }],
  games_fortnite_replays: () => ({ dir: 'C:\\Users\\Player\\AppData\\Local\\FortniteGame\\Saved\\Demos', bytes: 1_920_000_000, list: Array.from({ length: 18 }, (_, i) => ({ name: `UnsavedReplay-2026.10.${String(30 - i).padStart(2, '0')}-21.14.07.replay`, bytes: 60_000_000 + i * 9_000_000, at: Math.floor(Date.now() / 1000) - i * 86400 * 2 })) }),
  games_fortnite_clean_replays: () => [12, 1_310_000_000],
  games_net_check: () => ({ onWifi: true, wifiSaving: 2 }),
  games_set_wifi_saving: (a) => ({ onWifi: true, wifiSaving: a?.off ? 0 : 2 }),
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
  pick_files: (a) => core.pickFiles(optStr(a, 'title')),
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
  shots_tool_begin: () => media.regionBegin(),
  shots_overlay_mode: () => (location.hash.includes('mode=pick') ? 'pick' : location.hash.includes('mode=translate') ? 'translate' : 'region'),
  shots_region_pixel: () => null,
  shots_region_translate: async () => {
    await new Promise((r) => setTimeout(r, 700));
    return { text: 'Appuyez sur A pour continuer\nSauvegarde en cours…', translated: 'Press A to continue\nSaving…', model: 'llama3.2' };
  },
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

  // guests, folder rules, focus and time, dev tools
  guests_create: (a) => prod.guestCreate(str(a, 'label'), num(a, 'minutes'), num(a, 'maxBytes')),
  guests_list: () => prod.guestList(),
  guests_stop: (a) => prod.guestStop(str(a, 'code')),
  rules_list: () => prod.ruleList(),
  rules_save: (a) => prod.ruleSave(obj<FolderRule>(a, 'rule')),
  rules_delete: (a) => prod.ruleDelete(str(a, 'id')),
  rules_log: () => prod.ruleLog(),
  rules_run: () => [],
  focus_state: () => prod.focusState(),
  focus_start: (a) => prod.focusStart(num(a, 'focusMinutes'), num(a, 'breakMinutes'), strList(a, 'blocked')),
  focus_stop: () => prod.focusStop(),
  time_report: () => prod.timeReport(core.settings.focus.trackTime),
  time_clear: () => undefined,
  dev_projects: (a) => prod.projects(strList(a, 'roots')),
  dev_environment: () => prod.environment(),
  dev_set_path: (a) => prod.setPath(strList(a, 'entries')),
  dev_set_var: (a) => prod.setVar(str(a, 'name'), str(a, 'value')),
  dev_remove_var: (a) => prod.removeVar(str(a, 'name')),
  dev_lan_link: (a) => prod.lanLink(num(a, 'port'), str(a, 'local')),
  dev_open: () => undefined,

  // image and video tools, downloads, the phone's whiteboard
  images_inspect: (a) => sensors.imageInspect(strList(a, 'paths')),
  images_run: (a) => sensors.imageRun(strList(a, 'paths'), obj<ImageJob>(a, 'job')),
  video_status: () => ({ ready: true, installing: false }),
  video_probe: () => sensors.videoMedia(),
  video_start: (a) => sensors.videoStart(str(a, 'path'), obj<VideoJob>(a, 'job')),
  video_tasks: () => sensors.videoTasks(),
  video_cancel: (a) => sensors.videoCancel(str(a, 'id')),
  video_clear: () => sensors.videoClear(),
  downloads_start: (a) => sensors.downloadStart(str(a, 'url'), str(a, 'mode') as DownloadMode),
  downloads_list: () => sensors.downloadList(),
  downloads_cancel: (a) => sensors.downloadCancel(str(a, 'id')),
  downloads_clear: () => sensors.downloadClear(),
  downloads_folder: () => 'C:\\Users\\Player\\Downloads\\OmniHub',
  board_strokes: () => sensors.boardStrokes(),
  board_clear: () => sensors.boardClear(),
  board_undo: () => sensors.boardUndo(),
  board_save: () => 'C:\\Users\\Player\\Pictures\\OmniHub Whiteboard\\Whiteboard 2026-10-31 14-02.png',
  scans_folder: () => 'C:\\Users\\Player\\Documents\\OmniHub Scans',

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
  hidden_folder_files: (a) => [`${str(a, 'path')}\\photo1.jpg`, `${str(a, 'path')}\\notes.txt`],
  hidden_tidy_folder: () => undefined,
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
  vault_reset_password_hello: (a) => vault.resetPasswordWithHello(str(a, 'newPassword')),
  counters_state: () => ctr.state(),
  counters_event: (a) => ctr.event(str(a, 'id'), str(a, 'event')),
  counters_undo: (a) => ctr.undo(str(a, 'id')),
  counters_clear_log: (a) => ctr.clearLog(str(a, 'id')),
  counters_overlay: (a) => ctr.overlay(bool(a, 'on'), num(a, 'port')),
  library_games: () => lib.games(),
  library_detail: (a) => lib.detail(str(a, 'key')),
  library_achievements: () => lib.achievements(),
  library_friends: () => lib.friends(),
  library_wishlist: () => lib.wishlist(),
  library_account: () => lib.account(),
  library_set_key: () => lib.setKey(),
  library_patch: (a) => lib.patch(str(a, 'key'), obj(a, 'patch')),
  library_set_art: (a) => lib.setArt(str(a, 'key')),
  library_find_art: (a) => lib.findArt(str(a, 'key')),
  library_play: (a) => lib.play(str(a, 'key')),
  library_backup: () => lib.backup(),
  library_restore: () => lib.restore(),
  library_delete_backup: () => lib.deleteBackup(),
  library_sessions: () => lib.sessions(),
  emulators_state: () => emus.state(),
  ra_user: () => mockRaUser,
  ra_connect: (a) => (mockRaUser = str(a, 'key') ? str(a, 'user') : null),
  ra_summary: () => ({
    user: mockRaUser ?? 'Player',
    points: 1234,
    rank: 5678,
    games: [
      { id: 1, title: 'Pokémon Emerald Version', console: 'Game Boy Advance', icon: '', lastPlayed: '2026-11-06 20:00', achieved: 12, total: 60 },
      { id: 2, title: 'Metroid Fusion', console: 'Game Boy Advance', icon: '', lastPlayed: '2026-11-02 18:30', achieved: 41, total: 41 },
    ],
    recent: [{ title: 'Stone Badge', description: 'Beat Roxanne', game: 'Pokémon Emerald Version', badge: '', points: 5, date: '2026-11-06 20:01', hardcore: true }],
  }),
  emulators_pad_setup: () => [
    { emulator: 'DuckStation', status: 'set', file: 'C:\\Users\\you\\Documents\\DuckStation\\settings.ini' },
    { emulator: 'PCSX2', status: 'already', file: 'C:\\Users\\you\\Documents\\PCSX2\\inis\\PCSX2.ini' },
  ],
  emulator_saves: () => [
    { emulator: 'PCSX2', what: 'Memory cards', path: 'C:\\Users\\you\\Documents\\PCSX2\\memcards', files: 2, bytes: 16_777_216 },
    { emulator: 'DuckStation', what: 'Save states', path: 'C:\\Users\\you\\Documents\\DuckStation\\savestates', files: 7, bytes: 41_000_000 },
  ],
  emulator_saves_backup: () => 'C:\\Users\\you\\Documents\\OmniHub Backups\\Emulator saves 2026-11-07 10-30.zip',
  emulators_rom_opts: (a) => emus.setRomOpts(str(a, 'rom'), obj<RomOpts>(a, 'opts')),
  bios_check: () => [
    { platform: 'gba', name: 'gba_bios.bin', status: 'good', detail: 'Game Boy Advance: matches the known dump.' },
    { platform: 'psx', name: 'scph1001.bin', status: 'missing', detail: 'PlayStation (USA, SCPH-1001): not in this folder.' },
    { platform: 'nds', name: 'bios7.bin', status: 'different', detail: 'Nintendo DS ARM7 BIOS: bios7.bin isn’t the known dump (a bad dump, or another revision). It may still work.' },
  ],
  romdat_check: () => ({ datRoms: 3021, checked: [{ path: 'D:\\ROMs\\GBA\\emerald.gba', status: 'rename', proper: 'Pokemon - Emerald Version (USA, Europe).gba' }, { path: 'D:\\ROMs\\GBA\\Metroid Fusion (USA).gba', status: 'ok', proper: 'Metroid Fusion (USA).gba' }, { path: 'D:\\ROMs\\GBA\\hack.gba', status: 'unknown', proper: null }], duplicates: [['D:\\ROMs\\GBA\\emerald.gba', 'D:\\ROMs\\GBA\\copy of emerald.gba']] }),
  romdat_apply: () => ({ renamed: 1, skipped: [], state: emus.state() }),
  emulators_offers: (a) => emuOffers.map((o) => (a?.check ? { ...o, latest: o.latest ?? '2.2.0' } : o)),
  emulators_install: async (a) => {
    const id = str(a, 'id');
    for (const d of [10, 40, 80, 100]) {
      emit('emu:install', { id, done: d, total: 100 });
      await new Promise((r) => setTimeout(r, 250));
    }
    emuOffers = emuOffers.map((o) => (o.id === id ? { ...o, installed: o.latest ?? '2.2.0' } : o));
    return emus.state();
  },
  emulators_refresh: () => emus.refresh(),
  emulators_add: () => emus.add(),
  emulators_remove: (a) => emus.remove(str(a, 'id')),
  emulators_set_dirs: (a) => emus.setDirs(obj(a, 'dirs')),
  emulators_choose: (a) => emus.choose(str(a, 'platform'), str(a, 'emulator')),
  emulators_launch: () => emus.launch(),
  quick_state: () => qk.state(),
  quick_save: (a) => qk.save(obj(a, 'state')),
  quick_run: (a) => qk.run(obj(a, 'action')),
  quick_hide: () => undefined,
  quick_open: () => undefined,
  quick_obs_password: (a) => obs.password(str(a, 'password')),
  quick_obs_info: (a) => obs.info(Boolean(a?.connect)),
  presence_status: () => discord.status(),
  presence_save: (a) => discord.save(obj(a, 'settings')),
  network_status: () => network(),
  wake_targets: () => [{ name: 'Ethernet', mac: '0C-9D-92-AA-BB-CC', ip: '192.168.1.24', broadcast: '192.168.1.255', wifi: false }],
  open_device_manager: () => undefined,
  quiet_status: () => quietMock.status(),
  quiet_set: (a) => quietMock.set(Boolean(a?.on)),
  quiet_schedule_set: (a) => quietMock.setSchedule(obj(a, 'schedule')),
  quiet_timer_set: (a) => quietMock.setTimer((a?.minutes as number | null | undefined) ?? null),
  heat_get: () => heatMock.get(),
  heat_set: (a) => heatMock.set(obj(a, 'settings')),
  capri_desktop_status: () => capriDesktop.status(),
  capri_desktop_latest: () => capriDesktop.latest(),
  capri_desktop_install: () => capriDesktop.install(),
  capri_desktop_open: () => capriDesktop.open(),
  capri_desktop_close: () => capriDesktop.close(),
  capri_desktop_settings_set: (a) => capriDesktop.setSettings(obj(a, 'settings')),
  ollama_pull: () => capriDesktop.pull(),
  app_updates_get: () => appUpdatesMock.get(),
  app_updates_set: (a) => appUpdatesMock.set(obj(a, 'settings')),
  app_updates_run: () => appUpdatesMock.run(),
  party_get: () => partyMock.get(),
  party_start: () => partyMock.start(),
  party_stop: () => partyMock.stop(),
  party_auto: (a) => partyMock.auto(Boolean(a?.on)),
  party_remove: (a) => partyMock.remove(str(a, 'id')),
  party_bump: (a) => partyMock.bump(str(a, 'id')),
  party_next: () => partyMock.next(),
  screentime_list: () => screenTimeMock.list(),
  screentime_save: (a) => screenTimeMock.save(obj(a, 'limit')),
  screentime_delete: (a) => screenTimeMock.remove(str(a, 'id')),
  watch_get: () => watchMock.get(),
  watch_streamer_save: (a) => watchMock.saveStreamer(a?.streamer as { platform: 'twitch' | 'youtube'; handle: string }),
  watch_streamer_delete: (a) => watchMock.removeStreamer(str(a, 'id')),
  watch_streamers_check: () => watchMock.get(),
  watch_recap_set: (a) => watchMock.setRecap(str(a, 'time')),
  watch_recap_now: () => ({ title: 'Your day on the PC', body: 'Played 1 h 55 min: Fortnite 1 h 35 min, Minecraft 20 min. 3.4 GB of internet used. PC health 88/100.' }),
  watch_shop_set: (a) => watchMock.setShop((a?.wanted as string[]) ?? [], Boolean(a?.alerts)),
  watch_shop_check: () => watchMock.checkShop(),
  watch_price_save: (a) => watchMock.savePrice(obj(a, 'watch')),
  watch_price_delete: (a) => watchMock.removePrice(str(a, 'id')),
  watch_prices_check: () => watchMock.checkPrices(),
  voice_command: (a) => voiceMock(str(a, 'text')),
  sites_list: () => mockSites,
  sites_save: (a) => (mockSites = (a?.list as typeof mockSites) ?? mockSites),
  sites_reset: () => mockSites,
  sites_open: () => undefined,
  audio_outputs: () => mockOutputs,
  audio_output_set: (a) => (mockOutputs = mockOutputs.map((o) => ({ ...o, default: o.id === str(a, 'id') }))),
  ask_pc: (a) => {
    const q = str(a, 'question').toLowerCase();
    const slow = { title: 'Something is keeping the PC busy', points: ['The processor is 78% busy right now.', 'chrome.exe is using 41% of the processor.', 'Steam.exe is using 12% of the processor.'], tips: ['Close chrome.exe if you don’t need it (Tasks page).'], route: 'tasks' };
    const net = { title: 'Your internet is busy', points: ['Right now: 48.2 Mbit/s down, 1.1 Mbit/s up.', 'steam.exe has 14 connections open.', 'chrome.exe has 9 connections open.'], tips: ['Pause downloads and updates (Steam, Windows Update, cloud sync) while you play.'], route: 'tools?tab=network' };
    const answers = /internet|network|wifi|download/.test(q) ? [net] : /slow|lag/.test(q) ? [slow] : [{ title: 'Health score: 86/100', points: ['Space on C:: 41 GB free of 512 GB (8%)'], tips: ['Free up space'], route: 'home' }];
    return { answers, words: a?.words ? 'Chrome is the main thing keeping your PC busy right now. Closing a few tabs should make it feel faster.' : null };
  },
  photos_similar: async () => {
    for (let n = 0; n <= 240; n += 40) {
      emit('photos:progress', { done: n, total: 240 });
      await new Promise((r) => setTimeout(r, 120));
    }
    const p = (name: string, w: number, h: number, bytes: number) => ({ path: `C:\\Users\\Player\\Pictures\\Phone\\${name}`, bytes, width: w, height: h, modified: Date.now() - 86400000 });
    return [
      { photos: [p('IMG_2041.jpg', 4032, 3024, 3_900_000), p('IMG_2041 (1).jpg', 4032, 3024, 3_900_000), p('IMG_2041-small.jpg', 1280, 960, 410_000)], keep: 0 },
      { photos: [p('Screenshot 2026-10-12 174203.png', 2560, 1440, 1_200_000), p('Screenshot 2026-10-12 174205.png', 2560, 1440, 1_190_000)], keep: 0 },
    ];
  },
  photos_similar_stop: () => undefined,
  photos_trash: (a) => ((a?.paths as string[]) ?? []).length,
  speedrun_state: () => ({ ...mockRun, now: Date.now() }),
  speedrun_save: (a) => ((mockRun = { ...mockRun, settings: a?.settings as typeof mockRun.settings }), { ...mockRun, now: Date.now() }),
  speedrun_press: () => ({ ...mockRun, now: Date.now() }),
  speedrun_window: () => undefined,
  floats_get: () => mockFloats,
  widget_add: (a) => (mockFloats = { ...mockFloats, widgets: [...mockFloats.widgets, { id: String(Date.now()), kind: str(a, 'kind') as 'clock', x: 0, y: 0 }] }),
  widget_remove: (a) => (mockFloats = { ...mockFloats, widgets: mockFloats.widgets.filter((w) => w.id !== str(a, 'id')) }),
  crosshair_set: (a) => ((mockFloats = { ...mockFloats, crosshair: Boolean(a?.on) }), Boolean(a?.on)),
  wallpaper_set: (a) => ((mockFloats = { ...mockFloats, wallpaper: Boolean(a?.on) }), Boolean(a?.on)),
  phone_limits_get: () => ({ areas: [['deck', 'Deck and scenes'], ['music', 'Music, volume and the party queue'], ['files', 'Files, photos and transfers'], ['games', 'Games and counters'], ['notes', 'Notes, ideas and tasks'], ['chats', 'Chats'], ['screen', 'Screen sharing and control'], ['power', 'Power and quiet mode']], phones: mockLimits }),
  phone_limits_set: (a) => {
    const d = str(a, 'device');
    const areas = a?.areas as string[] | null;
    if (areas) mockLimits[d] = areas;
    else delete mockLimits[d];
    return { areas: [], phones: mockLimits };
  },
  soundboard_get: () => mockBoard,
  soundboard_save: (a) => (mockBoard = a?.settings as typeof mockBoard),
  soundboard_play: () => undefined,
  soundboard_stop: () => undefined,
  wifi_qr: () => ({ ssid: 'Casa Lopez 5G', security: 'WPA', password: 'demo-password', svg: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 29 29"><rect width="29" height="29" fill="#fff"/><path d="M1 1h7v7H1zM21 1h7v7h-7zM1 21h7v7H1zM10 10h3v3h-3zM15 12h4v2h-4zM12 16h2v4h-2zM17 17h3v3h-3z" fill="#0b0b12"/></svg>' }),
  timers_get: () => mockTimers,
  timers_start: (a) => (mockTimers = { ...mockTimers, timers: [...mockTimers.timers, { id: String(Date.now()), label: str(a, 'label') || `${num(a, 'minutes')} min`, endsAt: Date.now() + num(a, 'minutes') * 60000, minutes: num(a, 'minutes') }] }),
  timers_cancel: (a) => (mockTimers = { ...mockTimers, timers: mockTimers.timers.filter((t) => t.id !== str(a, 'id')) }),
  alarm_save: (a) => {
    const al = a?.alarm as (typeof mockTimers.alarms)[number];
    const withId = { ...al, id: al.id || String(Date.now()) };
    return (mockTimers = { ...mockTimers, alarms: [...mockTimers.alarms.filter((x) => x.id !== withId.id), withId] });
  },
  alarm_delete: (a) => (mockTimers = { ...mockTimers, alarms: mockTimers.alarms.filter((x) => x.id !== str(a, 'id')) }),
  bedtime_save: (a) => (mockTimers = { ...mockTimers, bedtime: a?.bedtime as typeof mockTimers.bedtime }),
  bedtime_snooze: () => mockTimers,
  watch_game_alerts: (a) => ({ ...watchMock.get(), freeAlerts: Boolean(a?.free), patchAlerts: Boolean(a?.patches), fortniteAlerts: Boolean(a?.fortnite) }),
  drives_health: () => [
    { name: 'Samsung SSD 990 PRO 2TB', media: 'SSD', bus: 'NVMe', health: 'Healthy', size: 2_000_398_934_016, temperature: 41, wear: 3, powerOnHours: 1520, readErrors: 0, writeErrors: 0, verdict: 'Healthy.' },
    { name: 'WDC WD40EZAZ', media: 'HDD', bus: 'SATA', health: 'Healthy', size: 4_000_787_030_016, temperature: 36, wear: null, powerOnHours: 21980, readErrors: 4, writeErrors: 0, verdict: 'Worth watching: back up what matters on it.' },
  ],
  stress_start: async (a) => {
    const total = num(a, 'seconds');
    for (let s = 1; s <= Math.min(total, 6); s++) {
      await new Promise((r) => setTimeout(r, 300));
      emit('stress:progress', { seconds: s, temp: 48 + s * 6 });
    }
    emit('stress:done', { seconds: total, startTemp: 54, maxTemp: 84, endTemp: 83, throttled: false, verdict: 'It reached 84 °C: warm but fine for most processors under full load.' });
  },
  stress_stop: () => undefined,
  game_ready: () => [
    { title: 'Windows updates', status: 'ok', detail: 'No restart waiting.', route: null },
    { title: 'Free space', status: 'ok', detail: 'Enough room for updates and shaders.', route: null },
    { title: 'Background programs', status: 'warn', detail: 'Busy: chrome.exe (22%). Close them or use the game’s boost.', route: 'tasks' },
    { title: 'Memory', status: 'ok', detail: '46% in use.', route: null },
    { title: 'Temperatures', status: 'ok', detail: '52 °C.', route: null },
  ],
  windows_list: () => mockWins,
  window_pin: (a) => {
    mockWins = mockWins.map((w) => (w.handle === num(a, 'handle') ? { ...w, pinned: Boolean(a?.on) } : w));
    return Boolean(a?.on);
  },
  snippets_get: () => mockSnippets,
  snippets_save: (a) => {
    const s = (a?.settings ?? mockSnippets) as typeof mockSnippets;
    mockSnippets = { ...s, snippets: s.snippets.map((x, i) => ({ ...x, id: x.id || `s${Date.now()}${i}`, name: x.name.trim() || x.text.split('\n')[0].slice(0, 24) })) };
    return mockSnippets;
  },
  home_status: () => mockHome,
  home_connect: (a) => (mockHome = { ...mockHome, url: str(a, 'url'), connected: !!str(a, 'token') }),
  home_set_shelly: (a) => (mockHome = { ...mockHome, shelly: (a?.plugs ?? []) as [string, string][] }),
  home_devices: () => [
    ...(mockHome.connected
      ? [
          { id: 'ha:light.desk', name: 'Desk lamp', kind: 'light', state: 'on' },
          { id: 'ha:switch.monitor_backlight', name: 'Monitor backlight', kind: 'switch', state: 'off' },
          { id: 'ha:scene.movie', name: 'Movie night', kind: 'scene', state: '' },
        ]
      : []),
    ...mockHome.shelly.map(([name, ip]) => ({ id: `shelly:${ip}`, name, kind: 'plug', state: '' })),
  ],
  home_run: (a) => `${str(a, 'device')}: ${str(a, 'action')}`,
  peers_list: () => mockPeers,
  peers_pair: (a) => {
    const pc = { id: `p${mockPeers.length + 1}`, name: 'LIVING-ROOM-PC', url: `https://${str(a, 'url')}`, fingerprint: '3F:A2:91:0C:77:E1:5B:D4:...' };
    mockPeers = [...mockPeers, pc];
    return pc;
  },
  peers_forget: (a) => (mockPeers = mockPeers.filter((p) => p.id !== str(a, 'id'))),
  peers_status: () => ({ hostname: 'LIVING-ROOM-PC', cpu: 7.5, memory: { used: 6_000_000_000, total: 16_000_000_000 }, uptime: 52_000 }),
  peers_action: () => undefined,
  deals_search: (a) => [
    { id: '612', title: `${str(a, 'title')}: Game of the Year`, thumb: '', cheapest: 3.99 },
    { id: '613', title: str(a, 'title'), thumb: '', cheapest: 9.99 },
  ],
  deals_game: () => ({
    title: 'BioShock Infinite',
    lowestEver: [2.99, 1543028665],
    deals: [
      { store: 'GOG', price: 3.99, retail: 29.99, savings: 86.7, url: 'https://www.cheapshark.com/redirect?dealID=def' },
      { store: 'Fanatical', price: 4.49, retail: 29.99, savings: 85, url: 'https://www.cheapshark.com/redirect?dealID=ghi' },
      { store: 'Steam', price: 29.99, retail: 29.99, savings: 0, url: 'https://www.cheapshark.com/redirect?dealID=abc' },
    ],
  }),
  standby_purge: async () => new Promise((r) => setTimeout(r, 600)),
  bench_results: () => mockBench,
  bench_run: async () => {
    for (const p of [5, 35, 75, 100]) {
      emit('bench:progress', { percent: p });
      await new Promise((r) => setTimeout(r, 300));
    }
    mockBench = [...mockBench, { at: Math.floor(Date.now() / 1000), single: 1820 + Math.random() * 40, multi: 21400 + Math.random() * 500, memory: 24.1, threads: 16, cpuTemp: 71 }];
    return mockBench;
  },
  net_limits: () => mockAppLimits,
  net_limit_set: (a) => {
    const exe = str(a, 'exe');
    mockAppLimits = [...mockAppLimits.filter((x) => x.exe.toLowerCase() !== exe.toLowerCase()), { exe, kbps: num(a, 'kbps'), active: true }].sort((x, y) => x.exe.localeCompare(y.exe));
    return mockAppLimits;
  },
  net_limit_remove: (a) => (mockAppLimits = mockAppLimits.filter((x) => x.exe !== str(a, 'exe'))),
  after_downloads_get: () => mockAfter,
  after_downloads_set: (a) => (mockAfter = { action: str(a, 'action'), quietSince: 0 }),
  game_move: async () => {
    for (let n = 0; n <= 10; n++) {
      emit('games:move', { name: 'Hollow Knight', done: n * 1e8, total: 1e9 });
      await new Promise((r) => setTimeout(r, 150));
    }
    return 'D:\\Games\\Hollow Knight';
  },
  game_move_stop: () => undefined,
  audio_inputs: () => [{ id: 'mic1', name: 'Microphone (HyperX QuadCast)', default: true }, { id: 'mic2', name: 'Headset Microphone (WH-1000XM5)', default: false }],
  screen_modes: () => ({ current: { width: 2560, height: 1440, hz: 165 }, modes: [{ width: 2560, height: 1440, hz: 165 }, { width: 2560, height: 1440, hz: 144 }, { width: 2560, height: 1440, hz: 60 }, { width: 1920, height: 1080, hz: 165 }, { width: 1920, height: 1080, hz: 144 }, { width: 1920, height: 1080, hz: 60 }, { width: 1280, height: 720, hz: 60 }], hdr: false }),
  nightlight_get: () => mockNight,
  nightlight_save: (a) => (mockNight = { ...mockNight, settings: a?.settings as typeof mockNight.settings }),
  nightlight_now: (a) => (mockNight = { ...mockNight, status: { ...mockNight.status, manual: (a?.on as boolean | null) ?? null, level: a?.on ? 1 : 0, kelvin: a?.on ? mockNight.settings.kelvin : 6500 } }),
  karaoke_get: () => karaokeMock.get(),
  karaoke_profile: (a) => karaokeMock.profile(str(a, 'lang')),
  karaoke_session: (a) => karaokeMock.session(a?.session as never),
  karaoke_clear: () => karaokeMock.clear(),
  karaoke_track: (a) => karaokeMock.track(str(a, 'path')),
  karaoke_lyrics: () => karaokeMock.lyrics(),
  bt_list: () => mockBt,
  bt_connect: (a) => (mockBt = mockBt.map((d) => (d.address === str(a, 'address') ? { ...d, connected: bool(a, 'on') } : d))),
  bt_forget: (a) => (mockBt = mockBt.filter((d) => d.address !== str(a, 'address'))),
  theme_art_get: (a) => mockArt[str(a, 'theme')] ?? null,
  theme_art_set: (a) => {
    const t = str(a, 'theme');
    mockArt[t] = a?.path ? SAMPLE_ART : null;
    return mockArt[t];
  },
  pads_set_light: (a) => ({ set: 1, skippedBluetooth: bool(a, 'bluetooth') ? 0 : 1, failed: 0 }),
  backups_list: () => bk.list(),
  backups_save: (a) => bk.save(obj(a, 'job')),
  backups_delete: (a) => bk.remove(str(a, 'id')),
  backups_run: (a) => bk.run(str(a, 'id')),
  backups_cancel: () => undefined,
  clipboard_list: () => clip.list(),
  clipboard_copy: (a) => clip.copy(str(a, 'id')),
  clipboard_paste: (a) => clip.copy(str(a, 'id')),
  clipboard_pin: (a) => clip.pin(str(a, 'id'), Boolean(a?.on)),
  clipboard_delete: (a) => clip.remove(str(a, 'id')),
  clipboard_clear: () => clip.clear(),
  clipboard_pause: (a) => clip.pause(Boolean(a?.on)),
  batteries: () => bats(),
  pc_health: () => pcHealth(),
  rules_tidy_downloads: (a) => prod.ruleTidy((a?.oldInstallers as number | null) ?? null),
  counters_save: (a) => ctr.save(obj(a, 'input')),
  counters_delete: (a) => ctr.remove(str(a, 'id')),
  counters_bump: (a) => ctr.bump(str(a, 'id'), num(a, 'times')),
  counters_set: (a) => ctr.set(str(a, 'id'), num(a, 'count')),
  counters_timer: (a) => ctr.timer(str(a, 'id'), bool(a, 'run'), bool(a, 'reset')),
  counters_keys_on: (a) => ctr.keysOn(bool(a, 'on')),
  counters_found: (a) => ctr.found(str(a, 'id'), bool(a, 'reset')),
  counters_forget_found: (a) => ctr.forgetFound(str(a, 'id')),
  counters_set_image: (a) => ctr.setImage(str(a, 'id'), optStr(a, 'path')),
  counters_image: () => ctr.image(),
  key_name: (a) => keyLabel(num(a, 'vk')),
  counters_popout: (a) => void window.open(`#/counter?id=${encodeURIComponent(str(a, 'id'))}`, '_blank', 'width=260,height=300'),
  autoclick_config: () => autoclick.config(),
  autoclick_set_config: (a) => autoclick.setConfig(obj(a, 'config')),
  autoclick_status: () => autoclick.status(),
  autoclick_start: () => autoclick.start(),
  autoclick_stop: () => autoclick.stop(),
  autoclick_cursor: () => autoclick.cursor(),
  pkm_tables: () => pkm.tables(),
  pkm_open: (a) => pkm.open(str(a, 'path')),
  pkm_view: () => pkm.view(),
  pkm_close: () => pkm.close(),
  pkm_edit: (a) => pkm.edit(obj(a, 'slot'), obj(a, 'edit')),
  pkm_delete: (a) => pkm.remove(obj(a, 'slot')),
  pkm_trainer: (a) => pkm.trainer(obj(a, 'trainer')),
  pkm_import: (a) => pkm.importPk3(obj(a, 'slot')),
  pkm_export: () => pkm.exportPk3(),
  pkm_write: () => pkm.write(),
  vault_set_aside: () => vault.setAside(),
  vault_set_aside_list: () => vault.setAsideList(),
  vault_put_back: (a) => vault.putBack(str(a, 'id')),
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
  chats_local_models: () => ({ models: ['gemma3:4b', 'llama3.2:latest'], model: 'llama3.2:latest' }),
  chats_local_set: () => undefined,
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
