// Govee lights without Windows: four pretend lights (a TV backlight, a pair
// of light bars, a floor lamp and a projector with a speaker), the cloud
// with a key, and live colours for the screen, the music and effects, so
// every part of the Lights page can be tried in a browser.

import type { DeepPartial, FirewallReport, GoveeChoice, GoveeExtra, GoveeLight, GoveeLightSettings, GoveeLook, GoveePreset, GoveeSettings, GoveeStatus, GoveeTimer, MonitorInfo } from '@shared/types';
import { emit } from './bus';

const LOOK: GoveeLook = {
  mode: 'music',
  color: '#00a2ff',
  kelvin: 4000,
  effect: 'wave',
  colors: ['#7c3aed', '#00d4ff', '#ff2bd6'],
  speed: 1,
  monitor: '',
  smoothness: 45,
  saturation: 130,
  skipDark: true,
  boost: 65,
  style: 'pulse',
  albumColors: true,
  sensitivity: 60,
  scene: '',
  deviceMusic: '',
};

const BUILT_IN: GoveePreset[] = [
  { id: 'movie', name: 'Movie', look: { ...LOOK, mode: 'screen', smoothness: 60, saturation: 130 }, brightness: null },
  { id: 'gaming', name: 'Gaming', look: { ...LOOK, mode: 'screen', smoothness: 15, saturation: 155 }, brightness: 100 },
  { id: 'party', name: 'Party', look: { ...LOOK, mode: 'music', style: 'pulse', sensitivity: 70 }, brightness: 100 },
  { id: 'spectrum', name: 'Spectrum', look: { ...LOOK, mode: 'music', style: 'spectrum' }, brightness: null },
  { id: 'flow', name: 'Flow', look: { ...LOOK, mode: 'music', style: 'flow', sensitivity: 55 }, brightness: null },
  { id: 'rainbow', name: 'Rainbow', look: { ...LOOK, mode: 'effect', effect: 'wave' }, brightness: null },
  { id: 'chill', name: 'Chill', look: { ...LOOK, mode: 'effect', effect: 'gradient', colors: ['#ff4d6d', '#ffb703', '#7b2cbf'], speed: 0.5 }, brightness: 60 },
  { id: 'ocean', name: 'Ocean', look: { ...LOOK, mode: 'effect', effect: 'breathing', colors: ['#0077b6', '#00b4d8', '#90e0ef'], speed: 0.6 }, brightness: null },
  { id: 'focus', name: 'Focus', look: { ...LOOK, mode: 'white', kelvin: 5600 }, brightness: 100 },
  { id: 'reading', name: 'Reading', look: { ...LOOK, mode: 'white', kelvin: 3200 }, brightness: 80 },
  { id: 'night', name: 'Night light', look: { ...LOOK, mode: 'color', color: '#ff5a00' }, brightness: 12 },
];

const SCENES: GoveeChoice[] = ['Sunrise', 'Sunset', 'Aurora', 'Ocean', 'Forest', 'Candlelight', 'Fireworks', 'Party', 'Movie', 'Rainbow', 'Starry sky', 'Romantic'].map((name, i) => ({ name, value: { paramId: 1600 + i, id: 1500 + i } }));
const MUSIC: GoveeChoice[] = [
  { name: 'Energic', value: 5 },
  { name: 'Rhythm', value: 3 },
  { name: 'Spectrum', value: 6 },
  { name: 'Rolling', value: 4 },
];
const toggle = (instance: string, label: string): GoveeExtra => ({ kind: 'toggle', type: 'devices.capabilities.toggle', instance, label, min: null, max: null, unit: null, choices: [], sound: false });

function light(id: string, sku: string, name: string, ip: string | null, segments: number | null, extras: GoveeExtra[], cloud = true): GoveeLight {
  return {
    id,
    sku,
    name,
    ip,
    lan: ip != null,
    cloud,
    on: true,
    brightness: 80,
    color: '#00a2ff',
    kelvin: null,
    segments,
    kelvinRange: [2000, 9000],
    scenes: cloud ? SCENES : [],
    diyScenes: cloud ? [{ name: 'My gradient', value: 8216567 }] : [],
    snapshots: cloud ? [{ name: 'Movie night', value: 0 }] : [],
    musicModes: cloud ? MUSIC : [],
    extras,
    values: { gradientToggle: 1, dreamViewToggle: 0, nightlightToggle: 0, volume: 35, whiteNoise: 2 },
    streaming: false,
    perSegment: false,
    message: null,
  };
}

const LIGHTS: GoveeLight[] = [
  light('1F:80:C5:32:32:36:72:4E', 'H6199', 'TV backlight', '192.168.1.41', 15, [toggle('gradientToggle', 'Gradient'), toggle('dreamViewToggle', 'DreamView')]),
  light('AA:12:C4:7B:90:31:5E:02', 'H6047', 'Desk light bars', '192.168.1.42', 10, [toggle('gradientToggle', 'Gradient')]),
  light('7C:A6:B0:E2:11:9F:43:D8', 'H6076', 'Floor lamp', '192.168.1.43', 7, [toggle('gradientToggle', 'Gradient')]),
  light('C2:05:9A:6E:7D:20:B1:57', 'H6092', 'Galaxy projector', null, null, [
    toggle('nightlightToggle', 'Night light'),
    { kind: 'range', type: 'devices.capabilities.range', instance: 'volume', label: 'Volume', min: 0, max: 100, unit: 'percent', choices: [], sound: true },
    {
      kind: 'choice',
      type: 'devices.capabilities.mode',
      instance: 'whiteNoise',
      label: 'White noise',
      min: null,
      max: null,
      unit: null,
      choices: ['Off', 'Rain', 'Ocean', 'Forest', 'Fan'].map((name, i) => ({ name, value: i })),
      sound: true,
    },
  ]),
];

const lightDefaults = (id: string): GoveeLightSettings => ({ id, name: '', control: true, follow: true, look: { ...LOOK }, on: true, brightness: 100, segments: 0, stream: 'auto', zone: 'whole', reverse: false, gradient: true });

let settings: GoveeSettings = {
  enabled: true,
  on: true,
  brightness: 85,
  look: { ...LOOK },
  lights: [
    { ...lightDefaults(LIGHTS[0].id), zone: 'around' },
    { ...lightDefaults(LIGHTS[1].id), zone: 'across' },
    { ...lightDefaults(LIGHTS[2].id), zone: 'right' },
    { ...lightDefaults(LIGHTS[3].id), follow: false, look: { ...LOOK, mode: 'scene', scene: 'Starry sky' }, brightness: 40 },
  ],
  presets: [{ id: 'my-cozy', name: 'Cozy evening', look: { ...LOOK, mode: 'effect', effect: 'breathing', colors: ['#ff7a00', '#ff3d00'], speed: 0.4 }, brightness: 45 }],
  preset: 'party',
  timers: [
    { id: 't1', name: 'Wake up', enabled: true, at: '07:00', days: [1, 2, 3, 4, 5], action: 'on', preset: '', brightness: 100, lights: [], fade: 15 },
    { id: 't2', name: 'Bedtime', enabled: true, at: '23:30', days: [], action: 'off', preset: '', brightness: 100, lights: [], fade: 10 },
  ],
  apiSecret: '',
  manual: [],
};

let hasKey = true;
let firewallOk = true;
let sleepUntil: number | null = null;
let heldUntil = 0;
let timer: ReturnType<typeof setInterval> | null = null;
const identifyUntil = new Map<string, number>();
const PALETTE = ['#ff2bd6', '#7c3aed', '#00d4ff', '#ffb703'];

function merge<T>(base: T, patch: unknown): T {
  if (!patch || typeof patch !== 'object' || Array.isArray(patch)) return patch as T;
  const out: Record<string, unknown> = { ...(base as Record<string, unknown>) };
  for (const [k, v] of Object.entries(patch)) out[k] = v && typeof v === 'object' && !Array.isArray(v) && out[k] && typeof out[k] === 'object' && !Array.isArray(out[k]) ? merge(out[k], v) : v;
  return out as T;
}

function lightSettings(id: string): GoveeLightSettings {
  return settings.lights.find((l) => l.id === id) ?? lightDefaults(id);
}

const hex = (r: number, g: number, b: number) => `#${[r, g, b].map((v) => Math.max(0, Math.min(255, Math.round(v))).toString(16).padStart(2, '0')).join('')}`;
const parse = (h: string): [number, number, number] => {
  const v = parseInt(h.replace('#', ''), 16) || 0;
  return [(v >> 16) & 255, (v >> 8) & 255, v & 255];
};
const mix = (a: string, b: string, k: number, scale = 1) => {
  const [x, y] = [parse(a), parse(b)];
  return hex((x[0] + (y[0] - x[0]) * k) * scale, (x[1] + (y[1] - x[1]) * k) * scale, (x[2] + (y[2] - x[2]) * k) * scale);
};
const along = (colors: string[], x: number, scale = 1) => {
  const c = colors.length ? colors : ['#ffffff'];
  const p = (((x % 1) + 1) % 1) * c.length;
  const i = Math.floor(p);
  return mix(c[i % c.length], c[(i + 1) % c.length], p - i, scale);
};
const hue = (h: number, scale = 1) => {
  const f = (n: number) => {
    const k = (n + h * 12) % 12;
    return 255 * scale * (0.5 - 0.5 * Math.max(-1, Math.min(k - 3, 9 - k, 1)));
  };
  return hex(f(0), f(8), f(4));
};
function kelvinHex(k: number): string {
  const t = k / 100;
  const r = t <= 66 ? 255 : 329.7 * Math.pow(t - 60, -0.1332);
  const g = t <= 66 ? 99.47 * Math.log(t) - 161.12 : 288.12 * Math.pow(t - 60, -0.0755);
  const b = t >= 66 ? 255 : t <= 19 ? 0 : 138.52 * Math.log(t - 10) - 305.04;
  return hex(r, g, b);
}

/** The colours a light shows at time `t` (seconds). */
function frame(l: GoveeLight, s: GoveeLightSettings, look: GoveeLook, t: number): string[] {
  const n = s.segments || l.segments || 10;
  const beat = Math.pow(Math.max(0, Math.sin(t * Math.PI * 2 * 2)), 6);
  const idx = (i: number) => (s.reverse ? n - 1 - i : i) / n;
  switch (look.mode) {
    case 'color':
      return [look.color];
    case 'white':
      return [kelvinHex(look.kelvin)];
    case 'effect': {
      const cols = look.colors.length ? look.colors : PALETTE;
      return Array.from({ length: n }, (_, i) => {
        const x = idx(i);
        if (look.effect === 'wave' || look.effect === 'spectrum') return hue(look.effect === 'wave' ? x * 0.8 - t * 0.25 * look.speed : t * 0.08 * look.speed);
        if (look.effect === 'breathing') return along(cols, Math.floor(t * 0.4 * look.speed), 0.06 + 0.94 * (0.5 - 0.5 * Math.cos(Math.PI * 2 * t * 0.4 * look.speed)));
        return along(cols, x * 0.5 + t * 0.05 * look.speed);
      });
    }
    case 'screen':
      return Array.from({ length: n }, (_, i) => along(['#1d4ed8', '#7c3aed', '#f97316', '#0ea5e9'], idx(i) * 0.6 + t * 0.07, 0.75 + 0.25 * Math.sin(t + i)));
    case 'music': {
      const pal = look.albumColors ? PALETTE : look.colors;
      if (look.style === 'spectrum') return Array.from({ length: n }, (_, i) => along(pal, idx(i) * 0.75, Math.min(1, 0.1 + Math.abs(Math.sin(t * 3 + i * 0.7)) * (1 - idx(i) * 0.5) + beat * 0.3)));
      if (look.style === 'flow') return Array.from({ length: n }, (_, i) => along(pal, idx(i) + t * 0.12, Math.min(1, 0.3 + beat * Math.exp(-Math.abs(Math.abs(idx(i) - 0.5) - (1 - beat) * 0.5) * 8))));
      return Array.from({ length: n }, (_, i) => along(pal, Math.floor(t * 2) / pal.length + 0.18 * idx(i), Math.min(1, 0.45 + 0.55 * beat)));
    }
    default:
      return [];
  }
}

function lights(): GoveeLight[] {
  return LIGHTS.map((l) => {
    const s = lightSettings(l.id);
    const look = s.follow ? settings.look : s.look;
    const on = settings.enabled ? s.on && (!s.follow || settings.on) : l.on;
    const live = ['effect', 'screen', 'music'].includes(look.mode);
    return {
      ...l,
      name: s.name || l.name,
      on,
      brightness: settings.enabled ? (s.follow ? settings.brightness : s.brightness) : l.brightness,
      color: look.mode === 'color' ? look.color : l.color,
      kelvin: look.mode === 'white' ? look.kelvin : null,
      streaming: !!(settings.enabled && on && live && l.lan && s.control),
      perSegment: s.stream === 'segments' || (s.stream === 'auto' && (l.segments ?? 0) > 1),
      message: settings.enabled && live && !l.lan ? 'Live colours need LAN Control: switch it on for this light in the Govee Home app.' : null,
      cloud: l.cloud && hasKey,
    };
  });
}

function status(): GoveeStatus {
  return {
    lan: 'on',
    lanMessage: null,
    cloud: hasKey ? 'on' : 'off',
    cloudMessage: null,
    hasKey,
    lights: lights(),
    screenMessage: null,
    musicMessage: null,
    palette: PALETTE,
    sleepUntil,
  };
}

function tick() {
  if (Date.now() > heldUntil) {
    if (timer) clearInterval(timer);
    timer = null;
    return;
  }
  const t = performance.now() / 1000;
  const out: Record<string, string[]> = {};
  for (const l of LIGHTS) {
    const s = lightSettings(l.id);
    const look = s.follow ? settings.look : s.look;
    const blink = (identifyUntil.get(l.id) ?? 0) > Date.now();
    if (blink) out[l.id] = [Math.floor(t * 4) % 2 ? '#000028' : '#ffffff'];
    else if (!settings.enabled || !s.control || !s.on || (s.follow && !settings.on)) out[l.id] = ['#000000'];
    else out[l.id] = frame(l, s, look, t);
  }
  emit('govee:frame', { lights: out });
}

function changed() {
  emit('govee:settings', { ...settings });
  emit('govee:status', status());
}

export const govee = {
  settings: (): GoveeSettings => ({ ...settings }),
  update(patch: DeepPartial<GoveeSettings>): GoveeSettings {
    const { apiSecret: _, ...rest } = patch as DeepPartial<GoveeSettings> & { apiSecret?: string };
    settings = merge(settings, rest);
    changed();
    return { ...settings };
  },
  updateLight(id: string, patch: DeepPartial<GoveeLightSettings>): GoveeSettings {
    const next = merge(lightSettings(id), patch);
    settings = { ...settings, lights: settings.lights.some((l) => l.id === id) ? settings.lights.map((l) => (l.id === id ? next : l)) : [...settings.lights, next] };
    changed();
    return { ...settings };
  },
  hold(): GoveeStatus {
    heldUntil = Date.now() + 8000;
    timer ??= setInterval(tick, 66);
    return status();
  },
  scan: () => setTimeout(() => emit('govee:status', status()), 600),
  setKey(key: string) {
    hasKey = key.trim().length > 0;
    changed();
  },
  control(id: string, instance: string, value: unknown) {
    const l = LIGHTS.find((x) => x.id === id);
    if (!l?.cloud || !hasKey) throw 'That needs your Govee API key (and the light in your Govee account).';
    l.values = { ...l.values, [instance]: value };
    setTimeout(() => emit('govee:status', status()), 300);
  },
  identify(id: string) {
    identifyUntil.set(id, Date.now() + 1800);
  },
  presets: (): GoveePreset[] => [...BUILT_IN, ...settings.presets],
  savePreset(p: GoveePreset): GoveePreset {
    const saved = { ...p, id: p.id && !BUILT_IN.some((b) => b.id === p.id) ? p.id : `my-${Math.random().toString(36).slice(2, 10)}`, name: p.name.trim() || 'My preset' };
    settings = { ...settings, presets: [...settings.presets.filter((x) => x.id !== saved.id), saved] };
    changed();
    return saved;
  },
  deletePreset(id: string) {
    settings = { ...settings, presets: settings.presets.filter((p) => p.id !== id) };
    changed();
  },
  applyPreset(id: string, ids: string[]): GoveeSettings {
    const p = govee.presets().find((x) => x.id === id);
    if (!p) throw 'That preset no longer exists.';
    if (ids.length === 0) settings = { ...settings, enabled: true, on: true, look: { ...p.look }, brightness: p.brightness ?? settings.brightness, preset: id };
    else for (const lid of ids) govee.updateLight(lid, { follow: false, look: { ...p.look }, on: true, ...(p.brightness != null ? { brightness: p.brightness } : {}) });
    changed();
    return { ...settings };
  },
  saveTimer(t: GoveeTimer): GoveeTimer {
    const saved = { ...t, id: t.id || `t-${Math.random().toString(36).slice(2, 8)}` };
    settings = { ...settings, timers: [...settings.timers.filter((x) => x.id !== saved.id), saved] };
    changed();
    return saved;
  },
  deleteTimer(id: string) {
    settings = { ...settings, timers: settings.timers.filter((t) => t.id !== id) };
    changed();
  },
  sleep(minutes: number | null): GoveeStatus {
    sleepUntil = minutes ? Date.now() + minutes * 60_000 : null;
    return status();
  },
  monitors: (): MonitorInfo[] => [
    { index: 0, name: 'ASUS VG27AQL1A', x: 0, y: 0, width: 2560, height: 1440, primary: true },
    { index: 1, name: 'LG TV', x: 2560, y: 0, width: 3840, height: 2160, primary: false },
  ],
  firewall(): FirewallReport {
    return {
      supported: true,
      program: 'C:\\Users\\You\\AppData\\Local\\OmniHub\\omnihub.exe',
      networks: [{ id: '{6B2C1E54-3A9D-4F1B-9C6E-2D8A7F0B4E31}', name: 'Home Wi-Fi', category: 'private' }],
      activeProfiles: 2,
      verdict: firewallOk ? 'allowed' : 'noRule',
      rules: [],
      message: firewallOk ? 'Windows Firewall lets the lights answer OmniHub.' : 'Windows Firewall has no rule for OmniHub, so it may block the lights’ answers.',
    };
  },
  fixFirewall(): FirewallReport {
    firewallOk = true;
    return govee.firewall();
  },
};
