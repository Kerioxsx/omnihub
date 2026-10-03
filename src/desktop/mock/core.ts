// Settings, audit log, app info and system stats for the mock backend.

import type { AppInfoDetails, AuditEntry, DeepPartial, Settings, SystemStats } from '@shared/types';
import { emit } from './bus';
import { DAY, GB, NOW } from './rng';

const query = typeof window !== 'undefined' ? new URLSearchParams(window.location.search) : new URLSearchParams();
export const flags = {
  onboarding: query.get('onboarding') === '1',
  power: query.get('power') === '1',
};

export const settings: Settings = {
  general: { theme: 'dark', accent: 'violet', reducedMotion: false, launchAtLogin: false, startMinimized: false, closeToTray: true, onboarded: !flags.onboarding },
  storage: { defaultMode: 'fast', exclude: [], cleanup: { largeFileMin: 1 << 30, largeFileAgeDays: 180, installerAgeDays: 30 }, showHidden: true, sizeMetric: 'size' },
  notes: { claudeFolder: 'C:\\Users\\Alex\\Documents\\Claude Ideas', sidecarJson: false, autoExportIdeas: false, indexFile: true },
  vault: { autoLockMinutes: 5, clipboardClearSeconds: 20, lockOnSessionLock: true, allowPhone: false, helloEnabled: false },
  remote: {
    enabled: false,
    port: 47800,
    bind: 'lan',
    tls: true,
    allowTailscale: false,
    browseScope: 'userFolders',
    customRoots: [],
    allowUploads: true,
    allowPower: true,
    powerDelaySeconds: 10,
    allowScreen: true,
    allowControl: false,
    allowAppLaunch: true,
    allowNotes: true,
    incomingDir: null,
    deviceName: 'ALEX-DESKTOP',
  },
  screenshots: { dir: null, format: 'png', hotkeyRegion: 'Alt+Shift+S', hotkeyFull: 'Alt+Shift+A', hotkeyWindow: 'Alt+Shift+W', copyToClipboard: true },
  screen: { preset: 'balanced', maxFps: 60, scrcpyPath: null, sunshinePath: null },
};

function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

/** RFC 7396 merge patch (null clears a value). */
function mergePatch(target: Record<string, unknown>, patch: Record<string, unknown>): void {
  for (const [k, v] of Object.entries(patch)) {
    if (v === null) target[k] = null;
    else if (isRecord(v) && isRecord(target[k])) mergePatch(target[k] as Record<string, unknown>, v);
    else target[k] = Array.isArray(v) ? [...v] : v;
  }
}

type SettingsHook = (before: Settings, after: Settings) => void;
const hooks: SettingsHook[] = [];
export function onSettingsChange(h: SettingsHook): void {
  hooks.push(h);
}

export function updateSettings(patch: DeepPartial<Settings>): Settings {
  const before = structuredClone(settings);
  mergePatch(settings as unknown as Record<string, unknown>, patch as Record<string, unknown>);
  if (settings.remote.port < 1024 || settings.remote.port > 65535) {
    Object.assign(settings, before);
    throw new Error('port must be between 1024 and 65535');
  }
  const after = structuredClone(settings);
  hooks.forEach((h) => h(before, after));
  emit('settings:changed', after);
  return after;
}

// ---------- audit ----------

let auditId = 0;
export const auditLog: AuditEntry[] = [];

const seedAudit: [number, string, string, string, boolean][] = [
  [DAY * 6.2, 'desktop', 'settings', 'Phone companion enabled', true],
  [DAY * 6.1, 'Pixel 8 Pro', 'pair', 'Paired from 192.168.1.57', true],
  [DAY * 5.4, 'Pixel 8 Pro', 'files.download', 'Downloads\\invoice_2401.pdf', true],
  [DAY * 4.9, 'iPhone 15', 'pair', 'Paired from 192.168.1.63', true],
  [DAY * 4.2, 'iPhone 15', 'files.upload', 'IMG_4471.MOV (1.9 GB) → Downloads\\OmniHub', true],
  [DAY * 3.3, 'unknown', 'pair', 'Wrong PIN from 192.168.1.80', false],
  [DAY * 2.6, 'Pixel 8 Pro', 'screen.view', 'Watched DELL U2723QE for 4 min', true],
  [DAY * 2.1, 'Pixel 8 Pro', 'power.schedule', 'Sleep in 10 s', true],
  [DAY * 2.1, 'desktop', 'power.cancel', 'Sleep cancelled from the desktop', true],
  [DAY * 1.4, 'Pixel 8 Pro', 'apps.launch', 'Spotify', true],
  [DAY * 0.9, 'iPhone 15', 'notes.create', 'Idea: “Treemap keyboard navigation”', true],
  [DAY * 0.3, 'desktop', 'vault.unlock', 'Unlocked with the master password', true],
  [3600 * 2, 'Pixel 8 Pro', 'files.list', 'Pictures\\Camera Roll', true],
];
for (const [ago, actor, action, detail, ok] of seedAudit) auditLog.unshift({ id: ++auditId, at: NOW - Math.round(ago), actor, action, detail, ok });

export function audit(actor: string, action: string, detail: string, ok = true): void {
  const entry: AuditEntry = { id: ++auditId, at: Math.floor(Date.now() / 1000), actor, action, detail, ok };
  auditLog.unshift(entry);
  emit('audit:new', entry);
}

// ---------- app info & stats ----------

export const appInfo: AppInfoDetails = {
  version: '0.1.0',
  elevated: false,
  platform: 'windows',
  hostname: 'ALEX-DESKTOP',
  dpapi: true,
  dataDir: 'C:\\Users\\Alex\\AppData\\Roaming\\OmniHub',
  configDir: 'C:\\Users\\Alex\\AppData\\Roaming\\OmniHub',
  cacheDir: 'C:\\Users\\Alex\\AppData\\Local\\OmniHub\\cache',
  screenshotDir: 'C:\\Users\\Alex\\Pictures\\OmniHub',
  incomingDir: 'C:\\Users\\Alex\\Downloads\\OmniHub',
};

const bootedAt = NOW - Math.round(DAY * 2.2);
let cpuPhase = 0;
export function systemStats(): SystemStats {
  cpuPhase += 1;
  const cpu = Math.max(2, Math.min(96, 14 + 9 * Math.sin(cpuPhase / 2.3) + 6 * Math.sin(cpuPhase / 0.9) + Math.random() * 6));
  const used = 14.6 * GB + Math.sin(cpuPhase / 4) * 0.6 * GB + Math.random() * 0.2 * GB;
  return { cpu, memory: { used, total: 32 * GB }, uptime: Math.floor(Date.now() / 1000) - bootedAt, os: 'Windows 11 Pro 24H2 (build 26100.2033)' };
}

export function pickFolder(title: string | null): string | null {
  const t = (title ?? '').toLowerCase();
  if (t.includes('claude')) return 'C:\\Users\\Alex\\Documents\\Claude Ideas';
  if (t.includes('incoming') || t.includes('phone')) return 'C:\\Users\\Alex\\Downloads\\From phone';
  if (t.includes('screenshot')) return 'C:\\Users\\Alex\\Pictures\\Screenshots';
  if (t.includes('browse') || t.includes('root')) return 'D:\\Media';
  return 'C:\\Users\\Alex\\Documents';
}

export function pickFiles(): string[] {
  return ['C:\\Users\\Alex\\Videos\\Edits\\Phoenix Trailer\\Phoenix Trailer v3 final.mp4', 'C:\\Users\\Alex\\Documents\\Work\\Roadmap 2026-10.pdf'];
}
