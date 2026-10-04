// Settings, audit log, app info and system stats for the mock backend.

import type { AppInfoDetails, AuditEntry, DeepPartial, Priority, ProcessGroup, ProcessSort, Settings, ShareState, StartupItem, StartupLocation, SystemStats, Usage, WindowInfo } from '@shared/types';
import { emit } from './bus';
import { DAY, GB, MB, NOW } from './rng';

const query = typeof window !== 'undefined' ? new URLSearchParams(window.location.search) : new URLSearchParams();
export const flags = {
  onboarding: query.get('onboarding') === '1',
  power: query.get('power') === '1',
};

export const settings: Settings = {
  general: { theme: 'dark', accent: 'violet', reducedMotion: false, launchAtLogin: false, startMinimized: false, closeToTray: true, onboarded: !flags.onboarding, displayName: '', uiScale: 100 },
  storage: { defaultMode: 'fast', exclude: [], cleanup: { largeFileMin: 1 << 30, largeFileAgeDays: 180, installerAgeDays: 30 }, showHidden: true, sizeMetric: 'size', explorerView: query.get('view') === 'grid' ? 'grid' : query.get('view') === 'sunburst' ? 'sunburst' : query.get('view') === 'list' ? 'list' : query.get('view') === 'treemap' ? 'treemap' : 'split', gridSize: 'md', gridPreviews: true, lowSpaceAlert: true, lowSpacePercent: 10 },
  notes: { claudeFolder: 'C:\\Users\\Player\\Documents\\Claude Ideas', sidecarJson: false, autoExportIdeas: false, indexFile: true },
  vault: { autoLockMinutes: 5, clipboardClearSeconds: 20, lockOnSessionLock: true, allowPhone: false, helloEnabled: false, browserAutofill: false, browserOfferSave: true },
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
    deviceName: 'GAMING-PC',
    allowClipboard: true,
    allowTasks: true,
    sendToMenu: true,
  },
  screenshots: { dir: null, format: 'png', hotkeyRegion: 'Alt+Shift+S', hotkeyFull: 'Alt+Shift+A', hotkeyWindow: 'Alt+Shift+W', copyToClipboard: true },
  screen: { preset: 'balanced', maxFps: 60, scrcpyPath: null, sunshinePath: null, airplay: { name: 'GAMING-PC (OmniHub)', quality: '1080p', fps: 60, audio: true, requirePin: true, lowLatency: true, fullscreen: false }, airplayKeepOnTop: false, airplayPip: false, airplayAutoStart: false, uxplayPath: null },
  apps: { favorites: ['spotify-music', 'visual-studio-code'] },
  media: { allowPhone: true, lyricsOnline: true, eqEnabled: false, bassDb: 0, trebleDb: 0 },
  updates: { check: true, autoInstall: true },
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
  version: '0.2.2',
  elevated: false,
  platform: 'windows',
  hostname: 'GAMING-PC',
  dpapi: true,
  dataDir: 'C:\\Users\\Player\\AppData\\Roaming\\OmniHub',
  configDir: 'C:\\Users\\Player\\AppData\\Roaming\\OmniHub',
  cacheDir: 'C:\\Users\\Player\\AppData\\Local\\OmniHub\\cache',
  screenshotDir: 'C:\\Users\\Player\\Pictures\\OmniHub',
  incomingDir: 'C:\\Users\\Player\\Downloads\\OmniHub',
  userName: '',
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
  if (t.includes('claude')) return 'C:\\Users\\Player\\Documents\\Claude Ideas';
  if (t.includes('incoming') || t.includes('phone')) return 'C:\\Users\\Player\\Downloads\\From phone';
  if (t.includes('screenshot')) return 'C:\\Users\\Player\\Pictures\\Screenshots';
  if (t.includes('browse') || t.includes('root')) return 'D:\\Media';
  return 'C:\\Users\\Player\\Documents';
}

export function pickFiles(): string[] {
  return ['C:\\Users\\Player\\Videos\\Edits\\Phoenix Trailer\\Phoenix Trailer v3 final.mp4', 'C:\\Users\\Player\\Documents\\Work\\Roadmap 2026-10.pdf'];
}

// ---------- processes ----------

const PROCS: { name: string; count: number; cpu: number; mem: number; gpu: number; vram: number; disk: number; exe: string; priority?: Priority }[] = [
  { name: 'FortniteClient-Win64-Shipping.exe', count: 1, cpu: 18.4, mem: 5.1 * GB, gpu: 87, vram: 6.2 * GB, disk: 4.2 * MB, exe: 'C:\\Program Files\\Epic Games\\Fortnite\\FortniteGame\\Binaries\\Win64\\FortniteClient-Win64-Shipping.exe', priority: 'high' },
  { name: 'brave.exe', count: 18, cpu: 7.5, mem: 2.4 * GB, gpu: 4, vram: 0.31 * GB, disk: 0.4 * MB, exe: 'C:\\Program Files\\BraveSoftware\\Brave-Browser\\Application\\brave.exe' },
  { name: 'Discord.exe', count: 6, cpu: 1.2, mem: 0.62 * GB, gpu: 1.5, vram: 0.12 * GB, disk: 0.05 * MB, exe: 'C:\\Users\\You\\AppData\\Local\\Discord\\app-1.0.9187\\Discord.exe' },
  { name: 'Code.exe', count: 11, cpu: 3.1, mem: 1.3 * GB, gpu: 0.6, vram: 0.08 * GB, disk: 0.2 * MB, exe: 'C:\\Users\\You\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe' },
  { name: 'steamwebhelper.exe', count: 7, cpu: 0.8, mem: 0.51 * GB, gpu: 0.3, vram: 0.05 * GB, disk: 0, exe: 'C:\\Program Files (x86)\\Steam\\bin\\cef\\cef.win7x64\\steamwebhelper.exe' },
  { name: 'Spotify.exe', count: 5, cpu: 0.6, mem: 0.38 * GB, gpu: 0.4, vram: 0.04 * GB, disk: 0.02 * MB, exe: 'C:\\Users\\You\\AppData\\Roaming\\Spotify\\Spotify.exe' },
  { name: 'explorer.exe', count: 1, cpu: 0.4, mem: 0.21 * GB, gpu: 0.2, vram: 0.03 * GB, disk: 0.01 * MB, exe: 'C:\\Windows\\explorer.exe' },
  { name: 'MsMpEng.exe', count: 1, cpu: 1.9, mem: 0.29 * GB, gpu: 0, vram: 0, disk: 1.1 * MB, exe: 'C:\\ProgramData\\Microsoft\\Windows Defender\\Platform\\MsMpEng.exe' },
  { name: 'omnihub.exe', count: 1, cpu: 0.9, mem: 0.16 * GB, gpu: 0.8, vram: 0.06 * GB, disk: 0.01 * MB, exe: 'C:\\Users\\You\\AppData\\Local\\OmniHub\\OmniHub.exe' },
  { name: 'dwm.exe', count: 1, cpu: 1.4, mem: 0.12 * GB, gpu: 6.5, vram: 0.42 * GB, disk: 0, exe: 'C:\\Windows\\System32\\dwm.exe', priority: 'high' },
  { name: 'OBS64.exe', count: 1, cpu: 4.2, mem: 0.44 * GB, gpu: 9.1, vram: 0.35 * GB, disk: 2.8 * MB, exe: 'C:\\Program Files\\obs-studio\\bin\\64bit\\obs64.exe', priority: 'aboveNormal' },
  { name: 'svchost.exe', count: 74, cpu: 1.1, mem: 1.05 * GB, gpu: 0, vram: 0, disk: 0.3 * MB, exe: 'C:\\Windows\\System32\\svchost.exe' },
  { name: 'EpicGamesLauncher.exe', count: 1, cpu: 0.7, mem: 0.33 * GB, gpu: 0.5, vram: 0.07 * GB, disk: 0.1 * MB, exe: 'C:\\Program Files (x86)\\Epic Games\\Launcher\\Portal\\Binaries\\Win64\\EpicGamesLauncher.exe' },
];
const PROTECTED = new Set(['msmpeng.exe', 'dwm.exe', 'omnihub.exe', 'svchost.exe']);

const jitter = (v: number, f = 0.3) => Math.max(0, v + (Math.random() - 0.5) * v * f);

function groups(): ProcessGroup[] {
  return PROCS.map((p, i) => ({
    name: p.name,
    count: p.count,
    cpu: Math.round(jitter(p.cpu, 0.6) * 10) / 10,
    memory: Math.round(p.mem * (0.97 + Math.random() * 0.06)),
    gpu: Math.round(Math.min(100, jitter(p.gpu, 0.25)) * 10) / 10,
    gpuMemory: Math.round(p.vram),
    disk: Math.round(jitter(p.disk, 0.8)),
    exe: p.exe,
    pids: Array.from({ length: p.count }, (_, k) => 1000 + i * 100 + k * 4),
    canEnd: !PROTECTED.has(p.name.toLowerCase()),
    priority: p.priority ?? 'normal',
  }));
}

function sortGroups(list: ProcessGroup[], sort: ProcessSort) {
  const by: Record<ProcessSort, (a: ProcessGroup, b: ProcessGroup) => number> = {
    cpu: (a, b) => b.cpu - a.cpu,
    memory: (a, b) => b.memory - a.memory,
    gpu: (a, b) => b.gpu - a.gpu,
    disk: (a, b) => b.disk - a.disk,
    name: (a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()),
  };
  return list.sort(by[sort]);
}

export function processes(sort: ProcessSort, limit: number): ProcessGroup[] {
  return sortGroups(groups(), sort).slice(0, limit);
}

export function usage(sort: ProcessSort, limit: number): Usage {
  const list = groups();
  const cpu = Math.min(100, list.reduce((s, g) => s + g.cpu, 0) + 4);
  const gpu = Math.min(100, Math.max(...list.map((g) => g.gpu)) + 3);
  return {
    cpu: Math.round(cpu * 10) / 10,
    cpuName: 'AMD Ryzen 7 7800X3D 8-Core Processor',
    cores: 16,
    memoryUsed: Math.round(list.reduce((s, g) => s + g.memory, 0) + 3.2 * GB),
    memoryTotal: 32 * GB,
    gpuSupported: true,
    gpus: [
      { name: 'NVIDIA GeForce RTX 4070 SUPER', percent: Math.round(gpu * 10) / 10, memoryUsed: Math.round(list.reduce((s, g) => s + g.gpuMemory, 0) + 0.4 * GB), memoryTotal: 12 * GB },
      { name: 'AMD Radeon(TM) Graphics', percent: 2.1, memoryUsed: 0.2 * GB, memoryTotal: 0.5 * GB },
    ],
    disk: list.reduce((s, g) => s + g.disk, 0),
    processCount: 286,
    processes: sortGroups(list, sort).slice(0, limit),
  };
}

export function setPriority(name: string, priority: Priority): number {
  const p = PROCS.find((x) => x.name.toLowerCase() === name.toLowerCase());
  if (!p) throw new Error(`${name} is not running`);
  if (PROTECTED.has(name.toLowerCase())) throw new Error(`${name} is part of Windows; its priority stays as Windows set it`);
  p.priority = priority;
  audit('desktop', 'process.priority', `${p.name} → ${priority}`);
  return p.count;
}

export function endProcess(name: string): number {
  const i = PROCS.findIndex((p) => p.name.toLowerCase() === name.toLowerCase());
  if (i < 0) throw new Error(`${name} is not running`);
  if (PROTECTED.has(name.toLowerCase())) throw new Error(`${name} is part of Windows and cannot be ended here`);
  const [p] = PROCS.splice(i, 1);
  audit('desktop', 'process.end', `Ended ${p.name}`);
  return p.count;
}

// ---------- startup apps ----------

const startupItems: StartupItem[] = [
  { name: 'Discord', command: '"C:\\Users\\Player\\AppData\\Local\\Discord\\Update.exe" --processStart Discord.exe', location: 'runUser', enabled: true },
  { name: 'Spotify', command: '"C:\\Users\\Player\\AppData\\Roaming\\Spotify\\Spotify.exe" --autostart --minimized', location: 'runUser', enabled: true },
  { name: 'Steam', command: '"C:\\Program Files (x86)\\Steam\\steam.exe" -silent', location: 'runUser', enabled: false },
  { name: 'OneDrive', command: '"C:\\Program Files\\Microsoft OneDrive\\OneDrive.exe" /background', location: 'runUser', enabled: true },
  { name: 'SecurityHealth', command: '%windir%\\system32\\SecurityHealthSystray.exe', location: 'runMachine', enabled: true },
  { name: 'RtkAudUService', command: '"C:\\Windows\\System32\\DriverStore\\FileRepository\\realtekservice.inf_amd64\\RtkAudUService64.exe" -background', location: 'runMachine', enabled: true },
  { name: 'Logitech Download Assistant', command: 'C:\\Windows\\system32\\rundll32.exe C:\\Windows\\System32\\LogiLDA.dll,LogiFetch', location: 'runMachine32', enabled: false },
  { name: 'OmniHub.lnk', command: 'C:\\Users\\Player\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\OmniHub.lnk', location: 'folderUser', enabled: true },
  { name: 'Send to OneNote.lnk', command: 'C:\\Users\\Player\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\Send to OneNote.lnk', location: 'folderUser', enabled: false },
].map((i) => {
  const loc = i.location as StartupLocation;
  const machine = loc !== 'runUser' && loc !== 'folderUser';
  const key = { runUser: 'run-user', runMachine: 'run-machine', runMachine32: 'run-machine32', folderUser: 'folder-user', folderCommon: 'folder-common' }[loc];
  const target = i.command.startsWith('"') ? i.command.slice(1, i.command.indexOf('"', 1)) : i.command.split(' ')[0];
  return { id: `${key}:${i.name}`, name: i.name, displayName: i.name.replace(/\.lnk$/i, ''), command: i.command, target, location: loc, enabled: i.enabled, needsAdmin: machine };
});

export function startupList(): StartupItem[] {
  return startupItems.map((i) => ({ ...i })).sort((a, b) => a.displayName.localeCompare(b.displayName));
}

export function startupSet(id: string, enabled: boolean): StartupItem[] {
  const item = startupItems.find((i) => i.id === id);
  if (!item) throw new Error('that startup entry no longer exists');
  item.enabled = enabled;
  audit('desktop', enabled ? 'startup.enable' : 'startup.disable', item.displayName);
  return startupList();
}

// ---------- screen share privacy ----------

const WINDOWS: WindowInfo[] = [
  { id: 101, title: 'Baldur’s Gate 3', app: 'bg3_dx11.exe', width: 2560, height: 1440 },
  { id: 102, title: 'App.tsx — omnihub — Visual Studio Code', app: 'Code.exe', width: 1920, height: 1080 },
  { id: 103, title: 'YouTube — Brave', app: 'brave.exe', width: 1600, height: 1000 },
  { id: 104, title: 'Spotify Premium', app: 'Spotify.exe', width: 1280, height: 800 },
];
const share: ShareState = { paused: false, windowId: null, window: null };

export function shareState(): ShareState {
  return { ...share };
}

export function shareWindows(): WindowInfo[] {
  return WINDOWS.map((w) => ({ ...w }));
}

export function setSharePaused(on: boolean): ShareState {
  share.paused = on;
  audit('desktop', on ? 'screen.pause' : 'screen.resume', '');
  emit('screen:share-state', shareState());
  return shareState();
}

export function setShareWindow(id: number | null): ShareState {
  share.windowId = id;
  share.window = WINDOWS.find((w) => w.id === id) ?? null;
  emit('screen:share-state', shareState());
  return shareState();
}
