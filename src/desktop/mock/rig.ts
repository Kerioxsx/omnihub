// "My PC" without Windows: the user's PC as the demo (Ryzen 7 7800X3D,
// RTX 4070 SUPER, 2×16 GB DDR5), with OpenRGB, LibreHardwareMonitor and a
// Kraken found, so every part of the page can be tried in a browser.

import type { DeepPartial, LightProfile, RigMedia, RigSettings, RigStatus, Specs } from '@shared/types';
import { emit } from './bus';

const BUILT_IN: LightProfile[] = [
  { id: 'aurora', name: 'Aurora', effect: 'music', colors: ['#7c3aed', '#00d4ff', '#ff2bd6'], speed: 1, brightness: 100 },
  { id: 'ice', name: 'Ice', effect: 'static', colors: ['#00a2ff', '#e0f7ff'], speed: 1, brightness: 100 },
  { id: 'rainbow', name: 'Rainbow wave', effect: 'wave', colors: [], speed: 1, brightness: 100 },
  { id: 'spectrum', name: 'Spectrum', effect: 'spectrum', colors: [], speed: 0.6, brightness: 100 },
  { id: 'ember', name: 'Ember', effect: 'breathing', colors: ['#ff3b00', '#ff9500'], speed: 0.7, brightness: 100 },
  { id: 'sunset', name: 'Sunset', effect: 'gradient', colors: ['#ff4d6d', '#ffb703', '#7b2cbf'], speed: 1, brightness: 100 },
  { id: 'heat', name: 'Temperature', effect: 'temperature', colors: ['#00a2ff', '#00ff9c', '#ffd000', '#ff2a00'], speed: 1, brightness: 100 },
  { id: 'white', name: 'Clean white', effect: 'static', colors: ['#ffffff'], speed: 1, brightness: 100 },
  { id: 'off', name: 'Lights off', effect: 'off', colors: [], speed: 1, brightness: 100 },
];

const specs: Specs = {
  cpu: 'AMD Ryzen 7 7800X3D 8-Core Processor',
  cores: 8,
  threads: 16,
  gpus: ['NVIDIA GeForce RTX 4070 SUPER'],
  ram: [
    { slot: 'DIMM_A2', sizeGb: 16, speed: 6000, maker: 'G.Skill', part: 'F5-6000J3038F16G', kind: 'DDR5' },
    { slot: 'DIMM_B2', sizeGb: 16, speed: 6000, maker: 'G.Skill', part: 'F5-6000J3038F16G', kind: 'DDR5' },
  ],
  ramGb: 32,
  boardMaker: 'ASUSTeK COMPUTER INC.',
  board: 'ROG STRIX B650E-F GAMING WIFI',
  boardSize: 'atx',
  system: '',
  chassis: 'desktop',
  cooler: 'aio',
  coolerName: 'NZXT Kraken Elite (2023)',
  radiatorMm: 360,
  devices: [{ vendorId: 0x1e71, productId: 0x300c, name: 'NZXT Kraken Elite (2023)', role: 'cooler', radiatorMm: null, lcd: true }],
  disks: [{ name: 'Samsung SSD 990 PRO 2TB', nvme: true }],
  displays: [{ name: 'ASUS VG27AQL1A', width: 2560, height: 1440, hz: 170, maxHz: 170 }],
};

let settings: RigSettings = {
  lighting: { enabled: true, profile: 'aurora', custom: [], port: 6742, peripherals: false, skip: [] },
  fans: { profile: 'auto', lhmUrl: 'http://127.0.0.1:8085', lhmUser: '', lhmSecret: '', skip: [], coolers: true, liquidctlPath: null },
  screens: { monitor: '', monitorShow: { kind: 'off' }, fill: true, lcd: '\\\\?\\HID#VID_1E71&PID_300C', lcdShow: { kind: 'media', id: 'demo-aurora.png' }, lcdBrightness: 80 },
  model: { caseColor: 'black', glass: true, cooler: null, radiatorMm: null, frontFans: 3, topFans: 3, rearFans: 1, bottomFans: 0, ramRgb: true, verticalGpu: false, caseScreen: true },
};

let media: RigMedia[] = [
  { id: 'demo-aurora.png', name: 'Aurora', kind: 'picture', mime: 'image/svg+xml', bytes: 182_000 },
  { id: 'demo-ember.png', name: 'Ember', kind: 'picture', mime: 'image/svg+xml', bytes: 96_000 },
];

function merge<T>(base: T, patch: unknown): T {
  if (!patch || typeof patch !== 'object' || Array.isArray(patch)) return patch as T;
  const out: Record<string, unknown> = { ...(base as Record<string, unknown>) };
  for (const [k, v] of Object.entries(patch)) out[k] = v && typeof v === 'object' && !Array.isArray(v) && out[k] && typeof out[k] === 'object' && !Array.isArray(out[k]) && !('kind' in (v as object)) ? merge(out[k], v) : v;
  return out as T;
}

function status(): RigStatus {
  const t = Date.now() / 1000;
  const cpu = 52 + Math.sin(t / 7) * 6;
  const gpu = 47 + Math.sin(t / 9) * 5;
  const fast = settings.fans.profile === 'fast' ? 1.35 : settings.fans.profile === 'silent' ? 0.72 : 1;
  // OmniHub never sets a fan under 30% (the hardware's own Auto may).
  const duty = (base: number) => (settings.fans.profile === 'auto' ? base : Math.max(30, Math.round(base * fast)));
  return {
    lighting: settings.lighting.enabled ? 'on' : 'off',
    lightingMessage: null,
    devices: [
      { key: 'ROG STRIX B650E-F|HID', name: 'ASUS ROG STRIX B650E-F GAMING WIFI', kind: 'motherboard', leds: 18, zones: ['Aura Mainboard', 'Aura Addressable 1', 'Aura Addressable 2'], on: true },
      { key: 'G.Skill Trident Z5|I2C 0x61', name: 'G.Skill Trident Z5 RGB', kind: 'memory', leds: 8, zones: ['DRAM'], on: true },
      { key: 'G.Skill Trident Z5|I2C 0x63', name: 'G.Skill Trident Z5 RGB', kind: 'memory', leds: 8, zones: ['DRAM'], on: true },
      { key: 'RTX 4070 SUPER|I2C', name: 'ASUS TUF RTX 4070 SUPER', kind: 'gpu', leds: 1, zones: ['GPU'], on: true },
      { key: 'Kraken|HID', name: 'NZXT Kraken Elite', kind: 'cooler', leds: 24, zones: ['Pump ring', 'External'], on: true },
      { key: 'Keyboard|HID', name: 'Logitech G915', kind: 'keyboard', leds: 104, zones: ['Keys'], on: settings.lighting.peripherals },
    ].map((d) => ({ ...d, on: d.on && !settings.lighting.skip.includes(d.key) })) as RigStatus['devices'],
    openrgbInstalled: 'C:\\Program Files\\OpenRGB\\OpenRGB.exe',
    fans: 'on',
    fansMessage: null,
    controls: [
      { id: '/lpc/nct6799d/0/control/1', name: 'CPU Fan', hardware: 'Nuvoton NCT6799D', role: 'case', percent: duty(38), skipped: false },
      { id: '/lpc/nct6799d/0/control/2', name: 'Chassis Fan #1', hardware: 'Nuvoton NCT6799D', role: 'case', percent: duty(34), skipped: false },
      { id: '/lpc/nct6799d/0/control/3', name: 'Chassis Fan #2', hardware: 'Nuvoton NCT6799D', role: 'case', percent: duty(34), skipped: settings.fans.skip.includes('/lpc/nct6799d/0/control/3') },
      { id: '/gpu-nvidia/0/control/1', name: 'GPU Fan', hardware: 'NVIDIA GeForce RTX 4070 SUPER', role: 'gpu', percent: duty(30), skipped: false },
    ],
    rpm: [
      ['CPU Fan', Math.round(980 * fast)],
      ['Chassis Fan #1', Math.round(860 * fast)],
      ['Chassis Fan #2', Math.round(870 * fast)],
      ['GPU Fan', Math.round(1100 * fast)],
    ],
    cpuTemp: Math.round(cpu * 10) / 10,
    gpuTemp: Math.round(gpu * 10) / 10,
    coolers: 'on',
    coolersMessage: null,
    coolerDevices: [
      {
        description: 'NZXT Kraken 2023 Elite',
        vendorId: 0x1e71,
        productId: 0x300c,
        address: '\\\\?\\HID#VID_1E71&PID_300C',
        driver: 'KrakenZ3',
        lcd: true,
        status: [
          { key: 'Liquid temperature', value: 31.4, unit: '°C' },
          { key: 'Pump speed', value: Math.round(2200 * fast), unit: 'rpm' },
          { key: 'Pump duty', value: Math.round(70 * fast), unit: '%' },
        ],
      },
    ],
    liquidTemp: 31.4,
    lcdMessage: null,
  };
}

export const rig = {
  specs: () => specs,
  settings: () => settings,
  update: (patch: DeepPartial<RigSettings>) => {
    settings = merge(settings, patch);
    emit('rig:settings', settings);
    emit('rig:status', status());
    return settings;
  },
  hold: () => status(),
  profiles: () => [...BUILT_IN, ...settings.lighting.custom],
  saveProfile: (p: LightProfile) => {
    const name = p.name.trim().slice(0, 40);
    if (!name) throw new Error('Give the profile a name.');
    const id = !p.id || BUILT_IN.some((b) => b.id === p.id) ? `my-${crypto.randomUUID()}` : p.id;
    const saved = { ...p, id, name };
    const custom = settings.lighting.custom.some((q) => q.id === id) ? settings.lighting.custom.map((q) => (q.id === id ? saved : q)) : [...settings.lighting.custom, saved];
    rig.update({ lighting: { custom, profile: id } });
    return saved;
  },
  deleteProfile: (id: string) => {
    const custom = settings.lighting.custom.filter((q) => q.id !== id);
    rig.update({ lighting: { custom, profile: settings.lighting.profile === id ? 'aurora' : settings.lighting.profile } });
  },
  media: () => media,
  addMedia: () => {
    const m: RigMedia = { id: `demo-${['aurora', 'ember'][media.length % 2]}-${media.length}.png`, name: `Picture ${media.length + 1}`, kind: 'picture', mime: 'image/svg+xml', bytes: 120_000 };
    media = [...media, m];
    return m;
  },
  removeMedia: (id: string) => {
    media = media.filter((m) => m.id !== id);
  },
  displays: () => [
    { name: '\\\\.\\DISPLAY1', label: 'Display 1 · 2560×1440 (main)', width: 2560, height: 1440, scale: 1, primary: true },
    { name: '\\\\.\\DISPLAY2', label: 'Display 2 · 1920×480', width: 1920, height: 480, scale: 1, primary: false },
  ],
};
