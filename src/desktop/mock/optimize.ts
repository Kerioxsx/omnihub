// The game optimizer for the mock backend: Fortnite and Minecraft
// settings files that start out slow, the games "installed" on this
// pretend PC, and the PC-wide tweaks.

import type { ConfigChange, ConfigGame, ConfigOptions, ConfigSetting, ConfigStatus, GameConfigs, GameKind, GameProfile, GameStep, InstalledGame, PcStatus, PcTweak, TweakId } from '@shared/types';
import { addProfile, list as listProfiles, newProfile } from './games';

let options: ConfigOptions = {
  fortnite: { frameLimit: 0, performanceMode: true, lowestQuality: true, viewDistance: 1, resolutionScale: 100, showFps: true, fullscreen: true },
  minecraft: { unlimitedFps: true, fastGraphics: true, minimalParticles: true, noClouds: true, renderDistance: null },
};

/** What the pretend files say now. */
const files: Record<ConfigGame, Record<string, string>> = {
  fortnite: { FrameRateLimit: '144', PreferredFeatureLevel: 'sm6', PreferredRHI: 'dx12', bUseVSync: 'True', 'sg.ShadowQuality': '3', 'sg.TextureQuality': '3', 'sg.EffectsQuality': '3', 'sg.PostProcessQuality': '3', 'sg.ViewDistanceQuality': '3', 'sg.ResolutionQuality': '100', FullscreenMode: '1', bShowFPS: 'False', bMotionBlur: 'True', bShowGrass: 'True' },
  minecraft: { maxFps: '120', enableVsync: 'true', graphicsMode: '1', renderDistance: '12', particles: '0', renderClouds: '"true"', ao: 'true', entityShadows: 'true' },
};
const originals = structuredClone(files);
const backups: Partial<Record<ConfigGame, number>> = {};

function wanted(game: ConfigGame): [string, string, string][] {
  if (game === 'fortnite') {
    const o = options.fortnite;
    const w: [string, string, string][] = [
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
  const o = options.minecraft;
  const w: [string, string, string][] = [];
  if (o.unlimitedFps) w.push(['maxFps', '260', 'Max frame rate: unlimited'], ['enableVsync', 'false', 'VSync off']);
  if (o.fastGraphics) w.push(['graphicsMode', '0', 'Graphics: fast'], ['ao', 'false', 'Smooth lighting off'], ['entityShadows', 'false', 'Entity shadows off']);
  if (o.minimalParticles) w.push(['particles', '2', 'Particles: minimal']);
  if (o.noClouds) w.push(['renderClouds', '"false"', 'Clouds off']);
  if (o.renderDistance != null) w.push(['renderDistance', String(o.renderDistance), `Render distance: ${o.renderDistance} chunks`]);
  return w;
}

function pending(game: ConfigGame): ConfigChange[] {
  return wanted(game)
    .filter(([k, v]) => files[game][k] !== v)
    .map(([key, to, label]) => ({ key, to, label, from: files[game][key] ?? null }));
}

function settings(game: ConfigGame): ConfigSetting[] {
  const f = files[game];
  if (game === 'fortnite') {
    const limit = Number(f.FrameRateLimit);
    const shadows = Number(f['sg.ShadowQuality']);
    return [
      { label: 'Frame rate limit', value: limit === 0 ? 'Unlimited' : `${limit} FPS`, good: limit === 0 || limit >= 240 },
      { label: 'Rendering mode', value: f.PreferredFeatureLevel === 'es31' ? 'Performance' : 'DirectX 12', good: f.PreferredFeatureLevel === 'es31' },
      { label: 'VSync', value: f.bUseVSync === 'True' ? 'On' : 'Off', good: f.bUseVSync !== 'True' },
      { label: 'Shadows', value: ['Off', 'Medium', 'High', 'Epic'][shadows] ?? 'High', good: shadows === 0 },
      { label: 'Window mode', value: f.FullscreenMode === '0' ? 'Fullscreen' : 'Windowed fullscreen', good: f.FullscreenMode === '0' },
      { label: 'FPS counter', value: f.bShowFPS === 'True' ? 'On' : 'Off', good: f.bShowFPS === 'True' },
    ];
  }
  const fps = Number(f.maxFps);
  return [
    { label: 'Max frame rate', value: fps >= 260 ? 'Unlimited' : `${fps} FPS`, good: fps >= 260 },
    { label: 'VSync', value: f.enableVsync === 'true' ? 'On' : 'Off', good: f.enableVsync !== 'true' },
    { label: 'Graphics', value: ['Fast', 'Fancy', 'Fabulous'][Number(f.graphicsMode)] ?? 'Fancy', good: f.graphicsMode === '0' },
    { label: 'Render distance', value: `${f.renderDistance} chunks`, good: true },
  ];
}

function status(game: ConfigGame): ConfigStatus {
  return {
    game,
    path: game === 'fortnite' ? 'C:\\Users\\You\\AppData\\Local\\FortniteGame\\Saved\\Config\\WindowsClient\\GameUserSettings.ini' : 'C:\\Users\\You\\AppData\\Roaming\\.minecraft\\options.txt',
    found: true,
    running: false,
    settings: settings(game),
    pending: pending(game),
    backupAt: backups[game] ?? null,
  };
}

export function configs(): GameConfigs {
  return { options: structuredClone(options), games: [status('fortnite'), status('minecraft')] };
}

export function setOptions(o: ConfigOptions): GameConfigs {
  options = structuredClone(o);
  return configs();
}

export function apply(game: ConfigGame): GameConfigs {
  for (const [k, v] of wanted(game)) files[game][k] = v;
  backups[game] ??= Math.floor(Date.now() / 1000);
  return configs();
}

export function restore(game: ConfigGame): GameConfigs {
  if (!backups[game]) throw new Error('There is no copy of your own settings to restore.');
  files[game] = structuredClone(originals[game]);
  delete backups[game];
  return configs();
}

/** The boost's "Game settings" step. */
export function settingsStep(kind: GameKind): GameStep {
  const game = kind as ConfigGame;
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
  mouseAcceleration: { id: 'mouseAcceleration', title: 'Mouse acceleration off', description: '“Enhance pointer precision” off, so the same hand movement always turns the same amount — for aim, not frames.', impact: 'low', available: true, admin: false, restart: false, settingsLink: null, on: 'Off', off: 'On' },
  memoryIntegrity: { id: 'memoryIntegrity', title: 'Memory Integrity', description: 'A Windows security feature that Microsoft says can lower game performance. Your choice, in Windows Security → Core isolation.', impact: 'high', available: true, admin: false, restart: true, settingsLink: 'windowsdefender://coreisolation', on: 'Off', off: 'On' },
};
const pcState: Record<TweakId, { optimized: boolean; undo: boolean }> = {
  refreshRate: { optimized: false, undo: false },
  powerPlan: { optimized: false, undo: false },
  gameDvr: { optimized: false, undo: false },
  gameMode: { optimized: true, undo: false },
  windowedGames: { optimized: false, undo: false },
  gpuScheduling: { optimized: true, undo: false },
  mouseAcceleration: { optimized: false, undo: false },
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
