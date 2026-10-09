// Game profiles for the mock backend: templates, a pretend boost session
// whose steps tick in, ping results and a found Roblox install.

import type { GameKind, GameLaunch, GameProfile, GameSession, GameState, GameStep, PingResult, PingTarget, RobloxFlags, RobloxInstall } from '@shared/types';
import { emit } from './bus';
import { settingsStep } from './optimize';

const FORTNITE_APP = 'fn%3A4fe75bbc5a674f4f9b356b5c90567da5%3AFortnite';

const TEMPLATES: Record<GameKind, { name: string; launch: GameLaunch; process: string }> = {
  fortnite: { name: 'Fortnite', launch: { type: 'epic', app: FORTNITE_APP }, process: 'FortniteClient-Win64-Shipping.exe' },
  roblox: { name: 'Roblox', launch: { type: 'roblox', placeId: null }, process: 'RobloxPlayerBeta.exe' },
  valorant: { name: 'VALORANT', launch: { type: 'riot', product: 'valorant' }, process: 'VALORANT-Win64-Shipping.exe' },
  cs2: { name: 'Counter-Strike 2', launch: { type: 'steam', appId: 730 }, process: 'cs2.exe' },
  apex: { name: 'Apex Legends', launch: { type: 'steam', appId: 1172470 }, process: 'r5apex.exe' },
  rocketLeague: { name: 'Rocket League', launch: { type: 'epic', app: 'Sugar' }, process: 'RocketLeague.exe' },
  gta5: { name: 'GTA V', launch: { type: 'steam', appId: 271590 }, process: 'GTA5.exe' },
  callOfDuty: { name: 'Call of Duty', launch: { type: 'steam', appId: 1938090 }, process: 'cod.exe' },
  league: { name: 'League of Legends', launch: { type: 'riot', product: 'league_of_legends' }, process: 'League of Legends.exe' },
  minecraft: { name: 'Minecraft', launch: { type: 'none' }, process: 'javaw.exe' },
  custom: { name: 'My game', launch: { type: 'none' }, process: '' },
};

const ROBLOX_DEFAULT: RobloxFlags = { enabled: true, preset: 'balanced', renderer: 'auto', msaa: null, textureQuality: null, noGrass: false, graySky: false, lowDetailDistance: false, qualityLevel: null, exclusiveFullscreen: false, ignoreDisplayScaling: false, custom: {} };

function template(kind: GameKind): GameProfile {
  const t = TEMPLATES[kind];
  return {
    id: crypto.randomUUID(),
    name: t.name,
    kind,
    launch: t.launch,
    process: t.process,
    exePath: kind === 'fortnite' ? 'C:\\Program Files\\Epic Games\\Fortnite\\FortniteGame\\Binaries\\Win64\\FortniteClient-Win64-Shipping.exe' : kind === 'roblox' ? 'C:\\Users\\You\\AppData\\Local\\Roblox\\Versions\\version-6d3a1b2c4e5f4a1b\\RobloxPlayerBeta.exe' : null,
    boost: { powerPlan: 'ultimate', priority: 'high', closeApps: [], reopenApps: true, silenceNotifications: true, gameMode: true, gpuHighPerformance: true, fullscreenOptimizationsOff: false, wifiLowLatency: true, networkPriority: false, startHighPriority: false, gameSettings: true },
    pingHost: null,
    roblox: { ...ROBLOX_DEFAULT },
    lastPlayed: null,
  };
}

const profiles: GameProfile[] = (() => {
  const f = template('fortnite');
  f.lastPlayed = Math.floor(Date.now() / 1000) - 3 * 3600;
  f.boost.closeApps = ['brave.exe', 'OneDrive.exe', 'Spotify.exe'];
  f.boost.startHighPriority = true;
  const r = template('roblox');
  r.roblox.preset = 'maxFps';
  r.lastPlayed = Math.floor(Date.now() / 1000) - 2 * 86400;
  return [f, r];
})();
const applied = new Map<string, { qos: boolean; ifeo: boolean }>([[profiles[0].id, { qos: false, ifeo: true }]]);

let session: GameSession | null = null;
let timers: ReturnType<typeof setTimeout>[] = [];

export function list() {
  return { profiles: structuredClone(profiles), session };
}

export function addProfile(p: GameProfile): GameProfile {
  profiles.push(p);
  return structuredClone(p);
}

export function newProfile(kind: GameKind): GameProfile {
  return template(kind);
}

export function create(kind: GameKind): GameProfile {
  const p = template(kind);
  const n = profiles.filter((q) => q.kind === kind && kind !== 'custom').length;
  if (n) p.name = `${p.name} ${n + 1}`;
  profiles.push(p);
  return structuredClone(p);
}

export function save(p: GameProfile) {
  const i = profiles.findIndex((q) => q.id === p.id);
  if (i < 0) throw new Error('That profile no longer exists.');
  if (!p.name.trim()) throw new Error('Give the profile a name.');
  if (p.process && !/^[A-Za-z0-9][A-Za-z0-9 ._()-]*\.exe$/i.test(p.process)) throw new Error(`“${p.process}” is not a program name like Game.exe.`);
  profiles[i] = { ...structuredClone(p), name: p.name.trim(), lastPlayed: profiles[i].lastPlayed };
  return { profile: structuredClone(profiles[i]), warnings: [] };
}

export function remove(id: string) {
  const i = profiles.findIndex((q) => q.id === id);
  if (i >= 0) profiles.splice(i, 1);
}

export function state(id: string): GameState | null {
  const p = profiles.find((q) => q.id === id);
  if (!p) return null;
  const a = applied.get(id) ?? { qos: false, ifeo: false };
  return { networkPriority: a.qos, startHighPriority: a.ifeo, onWifi: true, running: false };
}

function push(f: (s: GameSession) => void) {
  if (!session) return;
  f(session);
  session = { ...session, steps: [...session.steps], restored: [...session.restored] };
  emit('games:session', session);
}

export function play(id: string, launch: boolean): GameSession {
  const p = profiles.find((q) => q.id === id);
  if (!p) throw new Error('That profile no longer exists.');
  if (session && session.phase !== 'ended') throw new Error(`${session.name} is already boosted — stop that first.`);
  timers.forEach(clearTimeout);
  timers = [];
  session = { profileId: id, name: p.name, phase: 'starting', startedAt: Math.floor(Date.now() / 1000), endedAt: null, launched: false, steps: [], restored: [], message: null };
  emit('games:session', session);
  const b = p.boost;
  const steps: GameStep[] = [];
  if (b.closeApps.length) steps.push({ id: 'apps', label: 'Background apps', status: 'done', detail: `Closed ${b.closeApps.map((a) => a.replace(/\.exe$/i, '')).join(', ')}` });
  if (b.powerPlan !== 'keep') steps.push({ id: 'power', label: 'Power plan', status: 'done', detail: b.powerPlan === 'ultimate' ? 'Ultimate Performance' : 'High performance' });
  if (b.silenceNotifications) steps.push({ id: 'notifications', label: 'Notifications', status: 'done', detail: 'Pop-ups paused' });
  if (b.gameMode) steps.push({ id: 'gamemode', label: 'Game Mode', status: 'skipped', detail: 'already on' });
  if (b.gpuHighPerformance || b.fullscreenOptimizationsOff) steps.push({ id: 'pergame', label: 'GPU & fullscreen', status: 'done', detail: [b.gpuHighPerformance && 'high-performance GPU', b.fullscreenOptimizationsOff && 'fullscreen optimizations off'].filter(Boolean).join(', ') });
  if (b.networkPriority || b.startHighPriority) {
    const a = applied.get(id) ?? { qos: false, ifeo: false };
    const need = (b.networkPriority && !a.qos) || (b.startHighPriority && !a.ifeo);
    steps.push({ id: 'admin', label: 'Network & start priority', status: need ? 'done' : 'skipped', detail: need ? 'Turned on (kept for next time)' : 'already on' });
    applied.set(id, { qos: b.networkPriority || a.qos, ifeo: b.startHighPriority || a.ifeo });
  }
  if (b.wifiLowLatency) steps.push({ id: 'wifi', label: 'Wi-Fi', status: 'done', detail: 'Low-latency mode on Intel(R) Wi-Fi 6E AX211 160MHz' });
  if ((p.kind === 'fortnite' || p.kind === 'minecraft') && b.gameSettings) steps.push(settingsStep(p.kind));
  if (p.kind === 'roblox' && p.roblox.enabled) steps.push({ id: 'roblox', label: 'Roblox flags', status: 'done', detail: `${Object.keys(preview(p.roblox).flags).length} flags written (1 file)` });
  if (launch) steps.push({ id: 'launch', label: 'Launch', status: p.launch.type === 'none' ? 'skipped' : 'done', detail: p.launch.type === 'none' ? 'no launcher set — start the game yourself' : `Started ${p.name}` });
  steps.forEach((s, i) => timers.push(setTimeout(() => push((x) => x.steps.push(s)), 250 + i * 220)));
  const t0 = 300 + steps.length * 220;
  if (launch && p.launch.type !== 'none') {
    p.lastPlayed = Math.floor(Date.now() / 1000);
    timers.push(setTimeout(() => push((x) => ((x.launched = true), (x.phase = 'waiting'))), t0));
    timers.push(
      setTimeout(() => {
        push((x) => (x.phase = 'playing'));
        if (b.priority) push((x) => x.steps.push({ id: 'priority', label: 'Game priority', status: 'done', detail: 'High priority' }));
      }, t0 + 2500),
    );
  } else {
    timers.push(setTimeout(() => push((x) => (x.phase = p.process ? 'waiting' : 'boosted')), t0));
  }
  return session;
}

export function stop(): boolean {
  if (!session || session.phase === 'ended') return false;
  timers.forEach(clearTimeout);
  timers = [];
  push((x) => (x.phase = 'restoring'));
  const p = profiles.find((q) => q.id === session?.profileId);
  setTimeout(() => {
    push((x) => {
      x.restored = [
        ...(p?.boost.wifiLowLatency ? [{ id: 'wifi', label: 'Wi-Fi', status: 'done' as const, detail: 'Back to normal' }] : []),
        ...(p?.boost.powerPlan !== 'keep' ? [{ id: 'power', label: 'Power plan', status: 'done' as const, detail: 'Back to the previous plan' }] : []),
        ...(p?.boost.silenceNotifications ? [{ id: 'notifications', label: 'Notifications', status: 'done' as const, detail: 'Pop-ups back on' }] : []),
        ...(p?.boost.closeApps.length && p.boost.reopenApps ? [{ id: 'apps', label: 'Background apps', status: 'done' as const, detail: `Reopened ${p.boost.closeApps.map((a) => a.replace(/\.exe$/i, '')).join(', ')}` }] : []),
      ];
      x.phase = 'ended';
      x.endedAt = Math.floor(Date.now() / 1000);
    });
  }, 700);
  return true;
}

const FORTNITE: PingTarget[] = [
  ['nae', 'NA-East', 24],
  ['nac', 'NA-Central', 38],
  ['naw', 'NA-West', 71],
  ['eu', 'Europe', 96],
  ['br', 'Brazil', 138],
  ['me', 'Middle East', 182],
  ['asia', 'Asia', 196],
  ['oce', 'Oceania', 214],
].map(([id, label]) => ({ id: id as string, label: label as string, host: `ping-${id}.ds.on.epicgames.com` }));
const BASE: Record<string, number> = { nae: 24, nac: 38, naw: 71, eu: 96, br: 138, me: 182, asia: 196, oce: 214, cloudflare: 9, google: 12, custom: 31 };

export function pingTargets(id: string | null): PingTarget[] {
  const p = profiles.find((q) => q.id === id);
  if (p?.pingHost) return [{ id: 'custom', label: p.pingHost, host: p.pingHost }];
  if (p?.kind === 'fortnite') return FORTNITE;
  return [
    { id: 'cloudflare', label: 'Nearest Cloudflare', host: '1.1.1.1' },
    { id: 'google', label: 'Nearest Google', host: '8.8.8.8' },
  ];
}

export async function ping(id: string | null, host: string | null): Promise<PingResult[]> {
  await new Promise((r) => setTimeout(r, 1600));
  const targets = host ? [{ id: 'custom', label: host, host }] : pingTargets(id);
  return targets.map((t) => {
    const base = BASE[t.id] ?? 40;
    const samples = Array.from({ length: 10 }, () => (Math.random() < 0.03 ? null : Math.round((base + Math.random() * base * 0.25 + (Math.random() < 0.1 ? base * 0.6 : 0)) * 10) / 10));
    const got = samples.filter((s): s is number => s != null);
    const diffs = got.slice(1).map((v, i) => Math.abs(v - got[i]));
    const r1 = (v: number) => Math.round(v * 10) / 10;
    return {
      ...t,
      address: t.host.match(/^\d/) ? t.host : `3.${Math.floor(Math.random() * 200)}.${Math.floor(Math.random() * 200)}.${Math.floor(Math.random() * 200)}`,
      sent: 10,
      received: got.length,
      avgMs: got.length ? r1(got.reduce((a, b) => a + b, 0) / got.length) : null,
      minMs: got.length ? Math.min(...got) : null,
      maxMs: got.length ? Math.max(...got) : null,
      jitterMs: diffs.length ? r1(diffs.reduce((a, b) => a + b, 0) / diffs.length) : null,
      loss: 1 - got.length / 10,
      samples,
      error: null,
    };
  });
}

export function robloxStatus(): RobloxInstall {
  return { found: true, player: 'C:\\Users\\You\\AppData\\Local\\Roblox\\Versions\\version-6d3a1b2c4e5f4a1b\\RobloxPlayerBeta.exe', version: 'version-6d3a1b2c4e5f4a1b', versionDirs: ['C:\\Users\\You\\AppData\\Local\\Roblox\\Versions\\version-6d3a1b2c4e5f4a1b'], bootstrapper: null, running: false };
}

const ALLOW = new Set([
  'DFIntCSGLevelOfDetailSwitchingDistance', 'DFIntCSGLevelOfDetailSwitchingDistanceL12', 'DFIntCSGLevelOfDetailSwitchingDistanceL23', 'DFIntCSGLevelOfDetailSwitchingDistanceL34',
  'FFlagHandleAltEnterFullscreenManually', 'DFFlagTextureQualityOverrideEnabled', 'DFIntTextureQualityOverride', 'FIntDebugForceMSAASamples', 'DFFlagDisableDPIScale',
  'FFlagDebugGraphicsPreferD3D11', 'FFlagDebugGraphicsPreferVulkan', 'FFlagDebugGraphicsPreferOpenGL', 'FFlagDebugSkyGray', 'DFFlagDebugPauseVoxelizer',
  'DFIntDebugFRMQualityLevelOverride', 'FIntFRMMaxGrassDistance', 'FIntFRMMinGrassDistance', 'FIntGrassMovementReducedMotionFactor',
]);

export function withPreset(f: RobloxFlags): RobloxFlags {
  const base: RobloxFlags = { ...ROBLOX_DEFAULT, enabled: f.enabled, preset: f.preset, custom: f.custom };
  switch (f.preset) {
    case 'maxFps':
      return { ...base, renderer: 'd3d11', msaa: 0, textureQuality: 0, noGrass: true, lowDetailDistance: true, qualityLevel: 1, exclusiveFullscreen: true };
    case 'balanced':
      return { ...base, renderer: 'd3d11', noGrass: true, exclusiveFullscreen: true };
    case 'quality':
      return { ...base, msaa: 4, textureQuality: 3 };
    default:
      return f;
  }
}

export function preview(flags: RobloxFlags) {
  const f = withPreset(flags);
  const m: Record<string, unknown> = {};
  if (f.renderer === 'd3d11') m.FFlagDebugGraphicsPreferD3D11 = 'True';
  if (f.renderer === 'vulkan') m.FFlagDebugGraphicsPreferVulkan = 'True';
  if (f.renderer === 'openGl') m.FFlagDebugGraphicsPreferOpenGL = 'True';
  if (f.msaa != null) m.FIntDebugForceMSAASamples = String(f.msaa);
  if (f.textureQuality != null) Object.assign(m, { DFFlagTextureQualityOverrideEnabled: 'True', DFIntTextureQualityOverride: String(f.textureQuality) });
  if (f.noGrass) Object.assign(m, { FIntFRMMinGrassDistance: '0', FIntFRMMaxGrassDistance: '0' });
  if (f.graySky) m.FFlagDebugSkyGray = 'True';
  if (f.lowDetailDistance) for (const k of ['', 'L12', 'L23', 'L34']) m[`DFIntCSGLevelOfDetailSwitchingDistance${k}`] = '0';
  if (f.qualityLevel != null) m.DFIntDebugFRMQualityLevelOverride = String(f.qualityLevel);
  if (f.exclusiveFullscreen) m.FFlagHandleAltEnterFullscreenManually = 'False';
  if (f.ignoreDisplayScaling) m.DFFlagDisableDPIScale = 'True';
  Object.assign(m, flags.custom);
  return { flags: m, ignored: Object.keys(m).filter((k) => !ALLOW.has(k)) };
}

export function robloxWrite(flags: RobloxFlags): string[] {
  void flags;
  return ['C:\\Users\\You\\AppData\\Local\\Roblox\\Versions\\version-6d3a1b2c4e5f4a1b\\ClientSettings\\ClientAppSettings.json'];
}
