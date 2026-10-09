// The game optimizer for the mock backend: every supported game's
// settings file, starting out slow (Apex and Overwatch not installed),
// the games "installed" on this pretend PC, and the PC-wide tweaks.

import type { ConfigChange, ConfigGame, ConfigOptions, ConfigSetting, ConfigStatus, GameConfigs, GameKind, GameProfile, GameStep, InstalledGame, PcStatus, PcTweak, ProOptions, TweakId } from '@shared/types';
import { addProfile, list as listProfiles, newProfile } from './games';

const PRO: ProOptions = { uncapped: true, lowestQuality: true, lowLatency: true };
let options: ConfigOptions = {
  fortnite: { frameLimit: 0, performanceMode: true, lowestQuality: true, viewDistance: 1, resolutionScale: 100, showFps: true, fullscreen: true },
  minecraft: { unlimitedFps: true, fastGraphics: true, minimalParticles: true, noClouds: true, renderDistance: null, simulationDistance: null },
  valorant: { ...PRO },
  cs2: { ...PRO },
  apex: { ...PRO },
  overwatch: { ...PRO },
  roblox: { ...PRO },
};

const ORDER: ConfigGame[] = ['fortnite', 'valorant', 'cs2', 'apex', 'overwatch', 'roblox', 'minecraft'];
const LABEL: Record<ConfigGame, string> = { fortnite: 'Fortnite', valorant: 'VALORANT', cs2: 'Counter-Strike 2', apex: 'Apex Legends', overwatch: 'Overwatch 2', roblox: 'Roblox', minecraft: 'Minecraft' };
const NOTE: Record<ConfigGame, string | null> = {
  fortnite: 'Turn on NVIDIA Reflex (On + Boost) in Fortnite’s graphics settings if you have it.',
  valorant: 'Turn on NVIDIA Reflex (On + Boost) in VALORANT’s video settings if you have it.',
  cs2: 'CS2 keeps its frame limit in the console: type fps_max 0 for unlimited (or fps_max 540).',
  apex: 'Apex’s frame limit is a launch option: add +fps_max 0 in Steam (Properties → Launch options) or the EA app.',
  overwatch: 'Turn on NVIDIA Reflex (Enabled + Boost) in Overwatch’s video settings if you have it.',
  roblox: 'Roblox’s own menu goes up to 240 FPS; the rest of its speed comes from the Fast Flags below.',
  minecraft: null,
};
const PATHS: Record<ConfigGame, string[]> = {
  fortnite: ['C:\\Users\\You\\AppData\\Local\\FortniteGame\\Saved\\Config\\WindowsClient\\GameUserSettings.ini'],
  valorant: ['C:\\Users\\You\\AppData\\Local\\VALORANT\\Saved\\Config\\4f1c2a9e-eu\\Windows\\GameUserSettings.ini'],
  cs2: ['C:\\Program Files (x86)\\Steam\\userdata\\81234567\\730\\local\\cfg\\cs2_video.txt', 'C:\\Program Files (x86)\\Steam\\userdata\\99887766\\730\\local\\cfg\\cs2_video.txt'],
  apex: [],
  overwatch: [],
  roblox: ['C:\\Users\\You\\AppData\\Local\\Roblox\\GlobalBasicSettings_13.xml'],
  minecraft: ['C:\\Users\\You\\AppData\\Roaming\\.minecraft\\options.txt'],
};

/** What the pretend files say now. */
const files: Record<ConfigGame, Record<string, string>> = {
  fortnite: { FrameRateLimit: '144', PreferredFeatureLevel: 'sm6', PreferredRHI: 'dx12', bUseVSync: 'True', 'sg.ShadowQuality': '3', 'sg.TextureQuality': '3', 'sg.EffectsQuality': '3', 'sg.PostProcessQuality': '3', 'sg.ViewDistanceQuality': '3', 'sg.ResolutionQuality': '100', FullscreenMode: '1', bShowFPS: 'False', bMotionBlur: 'True', bShowGrass: 'True' },
  minecraft: { maxFps: '120', enableVsync: 'true', graphicsMode: '1', renderDistance: '12', particles: '0', renderClouds: 'true', ao: 'true', entityShadows: 'true' },
  valorant: { bUseVSync: 'True', FrameRateLimit: '240', 'sg.ShadowQuality': '2', 'sg.EffectsQuality': '2', 'sg.TextureQuality': '3', 'sg.AntiAliasingQuality': '2', 'sg.PostProcessQuality': '1' },
  cs2: { 'setting.mat_vsync': '1', 'setting.r_low_latency': '0', 'setting.shaderquality': '1', 'setting.videocfg_shadow_quality': '2', 'setting.msaa_samples': '4', 'setting.videocfg_texture_detail': '2', 'setting.videocfg_ao_detail': '1' },
  apex: { 'setting.mat_vsync_mode': '1' },
  overwatch: { FrameRateCap: '300', VerticalSyncEnabled: '1', TripleBufferingEnabled: '0', ReduceBuffering: '0' },
  roblox: { FramerateCap: '60' },
};
const originals = structuredClone(files);
const backups: Partial<Record<ConfigGame, number>> = {};

type Want = [key: string, to: string, label: string];

function pro(game: 'valorant' | 'cs2' | 'apex' | 'overwatch' | 'roblox'): Want[] {
  const o = options[game];
  const w: Want[] = [];
  const low = (pairs: [string, string][]) => o.lowestQuality && pairs.forEach(([k, l]) => w.push([k, '0', l]));
  switch (game) {
    case 'valorant':
      if (o.uncapped) w.push(['bUseVSync', 'False', 'VSync off'], ['FrameRateLimit', '0', 'Frame rate limit: unlimited']);
      low([['sg.ShadowQuality', 'Shadows: low'], ['sg.EffectsQuality', 'Effects: low'], ['sg.TextureQuality', 'Textures: low'], ['sg.AntiAliasingQuality', 'Anti-aliasing: low'], ['sg.PostProcessQuality', 'Post-processing: low']]);
      break;
    case 'cs2':
      if (o.uncapped) w.push(['setting.mat_vsync', '0', 'VSync off']);
      if (o.lowLatency) w.push(['setting.r_low_latency', '1', 'NVIDIA Reflex: on']);
      low([['setting.shaderquality', 'Shader detail: low'], ['setting.videocfg_shadow_quality', 'Shadows: low'], ['setting.msaa_samples', 'Multisampling anti-aliasing: off'], ['setting.videocfg_texture_detail', 'Textures: low'], ['setting.videocfg_ao_detail', 'Ambient occlusion: off']]);
      break;
    case 'apex':
      if (o.uncapped) w.push(['setting.mat_vsync_mode', '0', 'VSync off']);
      break;
    case 'overwatch':
      if (o.uncapped) w.push(['FrameRateCap', '600', 'Frame rate limit: 600'], ['VerticalSyncEnabled', '0', 'VSync off'], ['TripleBufferingEnabled', '0', 'Triple buffering off']);
      if (o.lowLatency) w.push(['ReduceBuffering', '1', 'Reduce buffering: on']);
      break;
    case 'roblox':
      if (o.uncapped) w.push(['FramerateCap', '240', 'Maximum frame rate: 240']);
      break;
  }
  return w.filter(([k]) => k in files[game]);
}

function wanted(game: ConfigGame): Want[] {
  if (game === 'fortnite') {
    const o = options.fortnite;
    const w: Want[] = [
      ['FrameRateLimit', String(o.frameLimit), o.frameLimit ? `Frame rate limit: ${o.frameLimit} FPS` : 'Frame rate limit: unlimited'],
      ['bUseVSync', 'False', 'VSync off'],
      ['bMotionBlur', 'False', 'Motion blur off'],
      ['bShowFPS', o.showFps ? 'True' : 'False', o.showFps ? 'FPS counter on' : 'FPS counter off'],
      ['sg.ResolutionQuality', String(o.resolutionScale), `3D resolution: ${o.resolutionScale}%`],
    ];
    if (o.fullscreen) w.push(['FullscreenMode', '0', 'Window mode: fullscreen']);
    if (o.performanceMode) w.push(['PreferredFeatureLevel', 'es31', 'Rendering mode: Performance']);
    if (o.lowestQuality) {
      w.push(['bShowGrass', 'False', 'Grass off'], ['sg.ViewDistanceQuality', String(o.viewDistance), `View distance: ${['near', 'medium', 'far', 'epic'][o.viewDistance]}`]);
      for (const [k, what] of [['sg.ShadowQuality', 'Shadows'], ['sg.TextureQuality', 'Textures'], ['sg.EffectsQuality', 'Effects'], ['sg.PostProcessQuality', 'Post-processing']]) w.push([k, '0', `${what}: low`]);
    }
    return w;
  }
  if (game === 'minecraft') {
    const o = options.minecraft;
    const w: Want[] = [];
    if (o.unlimitedFps) w.push(['maxFps', '260', 'Max frame rate: unlimited'], ['enableVsync', 'false', 'VSync off']);
    if (o.fastGraphics) w.push(['graphicsMode', '0', 'Graphics: fast'], ['ao', 'false', 'Smooth lighting off'], ['entityShadows', 'false', 'Entity shadows off']);
    if (o.minimalParticles) w.push(['particles', '2', 'Particles: minimal']);
    if (o.noClouds) w.push(['renderClouds', 'false', 'Clouds off']);
    if (o.renderDistance != null) w.push(['renderDistance', String(o.renderDistance), `Render distance: ${o.renderDistance} chunks`]);
    return w;
  }
  return pro(game);
}

function pending(game: ConfigGame): ConfigChange[] {
  if (!PATHS[game].length) return [];
  return wanted(game)
    .filter(([k, v]) => files[game][k] !== v)
    .map(([key, to, label]) => ({ key, to, label, from: files[game][key] ?? null }));
}

const onOff = (v: string | undefined) => (v == null ? 'Game’s choice' : v === '0' || v.toLowerCase() === 'false' ? 'Off' : 'On');
const level = (v: string | undefined): [string, boolean] => (v == null ? ['Game’s choice', false] : Number(v) <= 0 ? ['Low', true] : Number(v) <= 1 ? ['Medium', false] : ['High', false]);

function settings(game: ConfigGame): ConfigSetting[] {
  const f = files[game];
  const row = (label: string, value: string, good: boolean): ConfigSetting => ({ label, value, good });
  switch (game) {
    case 'fortnite': {
      const limit = Number(f.FrameRateLimit);
      const shadows = Number(f['sg.ShadowQuality']);
      return [
        row('Frame rate limit', limit === 0 ? 'Unlimited' : `${limit} FPS`, limit === 0 || limit >= 240),
        row('Rendering mode', f.PreferredFeatureLevel === 'es31' ? 'Performance' : 'DirectX 12', f.PreferredFeatureLevel === 'es31'),
        row('VSync', f.bUseVSync === 'True' ? 'On' : 'Off', f.bUseVSync !== 'True'),
        row('Shadows', ['Off', 'Medium', 'High', 'Epic'][shadows] ?? 'High', shadows === 0),
        row('Window mode', f.FullscreenMode === '0' ? 'Fullscreen' : 'Windowed fullscreen', f.FullscreenMode === '0'),
        row('FPS counter', f.bShowFPS === 'True' ? 'On' : 'Off', f.bShowFPS === 'True'),
      ];
    }
    case 'minecraft': {
      const fps = Number(f.maxFps);
      return [
        row('Max frame rate', fps >= 260 ? 'Unlimited' : `${fps} FPS`, fps >= 260),
        row('VSync', f.enableVsync === 'true' ? 'On' : 'Off', f.enableVsync !== 'true'),
        row('Graphics', ['Fast', 'Fancy', 'Fabulous'][Number(f.graphicsMode)] ?? 'Fancy', f.graphicsMode === '0'),
        row('Render distance', `${f.renderDistance} chunks`, true),
      ];
    }
    case 'valorant': {
      const [sh, sg] = level(f['sg.ShadowQuality']);
      const [fx, fg] = level(f['sg.EffectsQuality']);
      return [row('VSync', onOff(f.bUseVSync), f.bUseVSync === 'False'), row('Shadows', sh, sg), row('Effects', fx, fg)];
    }
    case 'cs2': {
      const [sd, sdg] = level(f['setting.shaderquality']);
      const [sh, shg] = level(f['setting.videocfg_shadow_quality']);
      const reflex = Number(f['setting.r_low_latency']);
      return [row('VSync', onOff(f['setting.mat_vsync']), f['setting.mat_vsync'] === '0'), row('NVIDIA Reflex', reflex >= 2 ? 'On + Boost' : reflex >= 1 ? 'On' : 'Off', reflex >= 1), row('Shader detail', sd, sdg), row('Shadows', sh, shg)];
    }
    case 'apex':
      return [row('VSync', onOff(f['setting.mat_vsync_mode']), f['setting.mat_vsync_mode'] === '0')];
    case 'overwatch':
      return [row('Frame rate limit', `${f.FrameRateCap} FPS`, Number(f.FrameRateCap) >= 400), row('VSync', onOff(f.VerticalSyncEnabled), f.VerticalSyncEnabled === '0'), row('Reduce buffering', onOff(f.ReduceBuffering), f.ReduceBuffering === '1')];
    case 'roblox':
      return [row('Maximum frame rate', `${f.FramerateCap} FPS`, Number(f.FramerateCap) >= 240)];
  }
}

function status(game: ConfigGame): ConfigStatus {
  const found = PATHS[game].length > 0;
  return { game, label: LABEL[game], paths: PATHS[game], found, running: false, settings: found ? settings(game) : [], pending: pending(game), backupAt: backups[game] ?? null, note: NOTE[game] };
}

export function configs(): GameConfigs {
  return { options: structuredClone(options), games: ORDER.map(status) };
}

export function setOptions(o: ConfigOptions): GameConfigs {
  options = structuredClone(o);
  return configs();
}

export function apply(game: ConfigGame): GameConfigs {
  if (!PATHS[game].length) throw new Error(`${LABEL[game]} has no settings file yet. Start it once, close it, then try again.`);
  if (pending(game).length) backups[game] ??= Math.floor(Date.now() / 1000);
  for (const [k, v] of wanted(game)) files[game][k] = v;
  return configs();
}

export function restore(game: ConfigGame): GameConfigs {
  if (!backups[game]) throw new Error('There is no copy of your own settings to restore.');
  files[game] = structuredClone(originals[game]);
  delete backups[game];
  return configs();
}

const KIND_GAME: Partial<Record<GameKind, ConfigGame>> = { fortnite: 'fortnite', minecraft: 'minecraft', valorant: 'valorant', cs2: 'cs2', apex: 'apex', overwatch: 'overwatch', roblox: 'roblox' };

export function configGame(kind: GameKind): ConfigGame | null {
  return KIND_GAME[kind] ?? null;
}

/** The boost's "Game settings" step. */
export function settingsStep(game: ConfigGame): GameStep {
  if (!PATHS[game].length) return { id: 'settings', label: 'Game settings', status: 'skipped', detail: `${LABEL[game]} has no settings file yet. Start it once, close it, then try again.` };
  const n = pending(game).length;
  apply(game);
  return { id: 'settings', label: 'Game settings', status: n ? 'done' : 'skipped', detail: n ? `${n} settings set for the most FPS` : 'already set for the most FPS' };
}

// ---------- installed games ----------

const INSTALLED: Omit<InstalledGame, 'profileId'>[] = [
  { key: 'epic:Fortnite', name: 'Fortnite', source: 'epic', kind: 'fortnite', launch: { type: 'epic', app: 'fn%3A4fe75bbc5a674f4f9b356b5c90567da5%3AFortnite' }, process: 'FortniteClient-Win64-Shipping.exe', exePath: null, installDir: 'C:\\Program Files\\Epic Games\\Fortnite' },
  { key: 'steam:730', name: 'Counter-Strike 2', source: 'steam', kind: 'cs2', launch: { type: 'steam', appId: 730 }, process: 'cs2.exe', exePath: null, installDir: 'D:\\SteamLibrary\\steamapps\\common\\Counter-Strike Global Offensive' },
  { key: 'minecraft:java', name: 'Minecraft', source: 'minecraft', kind: 'minecraft', launch: { type: 'none' }, process: 'javaw.exe', exePath: null, installDir: 'C:\\Users\\You\\AppData\\Roaming\\.minecraft' },
  { key: 'roblox:player', name: 'Roblox', source: 'roblox', kind: 'roblox', launch: { type: 'roblox', placeId: null }, process: 'RobloxPlayerBeta.exe', exePath: null, installDir: null },
  { key: 'epic:Sugar', name: 'Rocket League', source: 'epic', kind: 'rocketLeague', launch: { type: 'epic', app: 'Sugar' }, process: 'RocketLeague.exe', exePath: null, installDir: 'C:\\Program Files\\Epic Games\\rocketleague' },
  { key: 'steam:3354750', name: 'skate.', source: 'steam', kind: 'custom', launch: { type: 'steam', appId: 3354750 }, process: 'skate.exe', exePath: 'D:\\SteamLibrary\\steamapps\\common\\skate\\skate.exe', installDir: 'D:\\SteamLibrary\\steamapps\\common\\skate' },
  { key: 'steam:1091500', name: 'Cyberpunk 2077', source: 'steam', kind: 'cyberpunk', launch: { type: 'steam', appId: 1091500 }, process: 'Cyberpunk2077.exe', exePath: null, installDir: 'D:\\SteamLibrary\\steamapps\\common\\Cyberpunk 2077' },
  { key: 'riot:valorant', name: 'VALORANT', source: 'riot', kind: 'valorant', launch: { type: 'riot', product: 'valorant' }, process: 'VALORANT-Win64-Shipping.exe', exePath: null, installDir: 'C:\\Riot Games\\VALORANT\\live' },
];

function matchProfile(g: Omit<InstalledGame, 'profileId'>, profiles: GameProfile[]): string | null {
  return profiles.find((p) => (g.kind !== 'custom' && p.kind === g.kind) || (g.process && p.process.toLowerCase() === g.process.toLowerCase()))?.id ?? null;
}

export function library(): InstalledGame[] {
  const { profiles } = listProfiles();
  return INSTALLED.map((g) => ({ ...g, profileId: matchProfile(g, profiles) })).sort((a, b) => a.name.localeCompare(b.name));
}

export function addInstalled(key: string): GameProfile {
  const g = library().find((x) => x.key === key);
  if (!g) throw new Error('That game is no longer installed.');
  if (g.profileId) return listProfiles().profiles.find((p) => p.id === g.profileId) as GameProfile;
  const p = newProfile(g.kind);
  if (g.kind === 'custom') Object.assign(p, { name: g.name, launch: g.launch, process: g.process, exePath: g.exePath });
  return addProfile(p);
}

// ---------- PC tweaks ----------

const TWEAKS: Record<TweakId, Omit<PcTweak, 'optimized' | 'current' | 'canUndo'> & { on: string; off: string }> = {
  refreshRate: { id: 'refreshRate', title: 'Monitor at its full refresh rate', description: 'Windows often leaves a 144 or 240 Hz monitor at 60 Hz. Every frame above that is drawn but never shown.', impact: 'high', available: true, admin: false, restart: false, settingsLink: null, on: '170 Hz', off: 'ASUS VG27AQL1A at 60 Hz — can do 170 Hz' },
  powerPlan: { id: 'powerPlan', title: 'Ultimate Performance power plan', description: 'Keeps the processor at full speed instead of waiting to ramp up — fewer stutters and higher lows. Uses more power (on a laptop, keep it plugged in).', impact: 'medium', available: true, admin: false, restart: false, settingsLink: null, on: 'High or Ultimate Performance', off: 'Balanced or power saving' },
  gameDvr: { id: 'gameDvr', title: 'Background game recording off', description: 'Xbox Game Bar can record games in the background (Game DVR), which costs frames. Screenshots with Win+Shift+S still work.', impact: 'medium', available: true, admin: false, restart: false, settingsLink: null, on: 'Off', off: 'On' },
  gameMode: { id: 'gameMode', title: 'Game Mode', description: 'Windows holds back updates and background work while a game runs.', impact: 'low', available: true, admin: false, restart: false, settingsLink: null, on: 'On', off: 'Off' },
  windowedGames: { id: 'windowedGames', title: 'Optimizations for windowed games', description: 'Windows 11 runs games in borderless windows with the same low input delay as fullscreen.', impact: 'medium', available: true, admin: false, restart: false, settingsLink: null, on: 'On', off: 'Off' },
  gpuScheduling: { id: 'gpuScheduling', title: 'Hardware-accelerated GPU scheduling', description: 'The graphics card schedules its own work: lower delay, and needed for DLSS frame generation. Takes effect after a restart.', impact: 'medium', available: true, admin: true, restart: true, settingsLink: null, on: 'On', off: 'Off' },
  preciseTimer: { id: 'preciseTimer', title: 'Precise timer for games', description: 'Lets a boost’s 0.5 ms Windows timer reach the game (Windows 11 otherwise keeps it to the program that asked). Games that pace frames with short waits hold steadier frame times. Takes effect after a restart.', impact: 'low', available: true, admin: true, restart: true, settingsLink: null, on: 'Shared with games', off: 'Kept per program' },
  networkThrottling: { id: 'networkThrottling', title: 'No network throttling during media', description: 'Windows slows network handling while audio or video plays and keeps 20% of the processor for background work. This lifts the limit and keeps 10%. Takes effect after a restart.', impact: 'low', available: true, admin: true, restart: true, settingsLink: null, on: 'No network limit · 10% kept for background work', off: 'Network limited during media · 20% kept for background work' },
  mouseAcceleration: { id: 'mouseAcceleration', title: 'Mouse acceleration off', description: '“Enhance pointer precision” off, so the same hand movement always turns the same amount — for aim, not frames.', impact: 'low', available: true, admin: false, restart: false, settingsLink: null, on: 'Off', off: 'On' },
  stickyKeys: { id: 'stickyKeys', title: 'Sticky Keys shortcut off', description: 'Pressing Shift five times no longer pops the Sticky Keys box over your game. Sticky Keys itself stays in Settings → Accessibility.', impact: 'low', available: true, admin: false, restart: false, settingsLink: null, on: 'Shortcut off', off: 'Shift ×5 opens Sticky Keys' },
  memoryIntegrity: { id: 'memoryIntegrity', title: 'Memory Integrity', description: 'A Windows security feature that Microsoft says can lower game performance. Your choice, in Windows Security → Core isolation.', impact: 'high', available: true, admin: false, restart: true, settingsLink: 'windowsdefender://coreisolation', on: 'Off', off: 'On' },
};
const pcState: Record<TweakId, { optimized: boolean; undo: boolean }> = {
  refreshRate: { optimized: false, undo: false },
  powerPlan: { optimized: false, undo: false },
  gameDvr: { optimized: false, undo: false },
  gameMode: { optimized: true, undo: false },
  windowedGames: { optimized: false, undo: false },
  gpuScheduling: { optimized: true, undo: false },
  preciseTimer: { optimized: false, undo: false },
  networkThrottling: { optimized: false, undo: false },
  mouseAcceleration: { optimized: false, undo: false },
  stickyKeys: { optimized: false, undo: false },
  memoryIntegrity: { optimized: false, undo: false },
};

export function pcStatus(): PcStatus {
  const refresh = pcState.refreshRate.optimized ? 170 : 60;
  return {
    tweaks: Object.values(TWEAKS).map(({ on, off, ...t }) => ({ ...t, optimized: pcState[t.id].optimized, current: pcState[t.id].optimized ? on : off, canUndo: pcState[t.id].undo })),
    machine: { cpu: 'AMD Ryzen 7 7800X3D 8-Core Processor', cores: 8, threads: 16, ramGb: 31.9, gpus: ['NVIDIA GeForce RTX 4070 SUPER'], displays: [{ name: 'ASUS VG27AQL1A', width: 2560, height: 1440, hz: refresh, maxHz: 170 }] },
  };
}

export async function pcSet(id: TweakId, on: boolean): Promise<PcStatus> {
  if (id === 'memoryIntegrity') throw new Error('Memory Integrity is changed in Windows Security → Core isolation.');
  await new Promise((r) => setTimeout(r, TWEAKS[id].admin ? 900 : 300));
  pcState[id] = { optimized: on, undo: on };
  return pcStatus();
}
