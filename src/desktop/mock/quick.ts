// Clipboard history, the wheel, the macro deck, batteries and PC health for
// the mock backend.

import type { BackupJob, Battery, ClipItem, ClipList, NetStatus, PcHealth, PresenceSettings, PresenceStatus, QuickAction, QuickButton, QuickState } from '@shared/types';
import { emit } from './bus';

const now = () => Date.now();
const b = (id: string, label: string, icon: string, color: string, action: QuickAction): QuickButton => ({ id, label, icon, color, action });

let state: QuickState = {
  clipOn: true,
  clipHotkey: 'Alt+Shift+V',
  clipIgnore: ['keepass.exe', 'keepassxc.exe', '1password.exe', 'bitwarden.exe'],
  clipMax: 200,
  wheelOn: true,
  wheelHotkey: 'Alt+Shift+Q',
  wheel: [
    b('w1', 'Screenshot', 'camera', '#38bdf8', { kind: 'screenshot' }),
    b('w2', 'Play / pause', 'play', '#a78bfa', { kind: 'media', what: 'playPause' }),
    b('w3', 'Clipboard', 'clipboard-list', '#34d399', { kind: 'clipboard' }),
    b('w4', 'Microphone', 'mic-off', '#f87171', { kind: 'mic' }),
    b('w5', 'Lock', 'lock', '#fbbf24', { kind: 'lock' }),
    b('w6', 'Counter-Strike 2', 'gamepad-2', '#fb923c', { kind: 'game', key: 'steam:730' }),
    b('w7', 'Discord', 'message-circle', '#818cf8', { kind: 'open', target: 'C:\\Users\\Player\\AppData\\Local\\Discord\\Update.exe' }),
  ],
  deck: [
    b('d1', 'Play / pause', 'play', '#a78bfa', { kind: 'media', what: 'playPause' }),
    b('d2', 'Previous', 'skip-back', '#a78bfa', { kind: 'media', what: 'previous' }),
    b('d3', 'Next', 'skip-forward', '#a78bfa', { kind: 'media', what: 'next' }),
    b('d4', 'Microphone', 'mic-off', '#f87171', { kind: 'mic' }),
    b('d5', 'Volume down', 'volume-1', '#60a5fa', { kind: 'media', what: 'volumeDown' }),
    b('d6', 'Volume up', 'volume-2', '#60a5fa', { kind: 'media', what: 'volumeUp' }),
    b('d7', 'Screenshot', 'camera', '#38bdf8', { kind: 'screenshot' }),
    b('d8', 'Shiny +1', 'sparkles', '#f472b6', { kind: 'counter', id: 'demo-hunt' }),
    b('d9', 'Discord mute', 'headphones', '#818cf8', { kind: 'keys', keys: 'Ctrl+Shift+M' }),
    b('d10', 'GG', 'message-square', '#34d399', { kind: 'text', text: 'gg wp' }),
    b('d11', 'Task Manager', 'activity', '#34d399', { kind: 'keys', keys: 'Ctrl+Shift+Esc' }),
    b('d12', 'Lock', 'lock', '#fbbf24', { kind: 'lock' }),
    b('d15', 'Quiet mode', 'moon', '#818cf8', { kind: 'quiet' }),
    b('d13', 'Go live', 'radio', '#f87171', { kind: 'obs', what: 'stream' }),
    b('d14', 'Gameplay scene', 'video', '#38bdf8', { kind: 'obs', what: 'scene', target: 'Gameplay' }),
  ],
  deckColumns: 4,
  obsUrl: 'ws://127.0.0.1:4455',
};
let obsPassword = '';

export const obs = {
  password: (p: string) => {
    obsPassword = p;
  },
  info: (connect: boolean) => {
    if (connect && !obsPassword) throw "OBS isn't answering: open OBS and turn on Tools → WebSocket Server Settings → Enable WebSocket server";
    return { hasPassword: !!obsPassword, scenes: connect ? ['Starting soon', 'Gameplay', 'Just chatting', 'BRB'] : [], inputs: connect ? ['Mic/Aux', 'Desktop Audio', 'Discord'] : [] };
  },
};

export const quick = {
  state: () => structuredClone(state),
  save(next: QuickState) {
    if (next.wheel.length > 8) throw 'The wheel has room for 8';
    if (next.deck.some((x) => !x.label.trim()) || next.wheel.some((x) => !x.label.trim())) throw 'Every button needs a name';
    if (next.deck.some((x) => x.action?.kind === 'clipboard')) throw "The clipboard history opens on the PC only, so it can't go on the deck";
    next.deck = next.deck.map((x) => ({ ...x, id: x.id || Math.random().toString(36).slice(2, 12) }));
    next.wheel = next.wheel.map((x) => ({ ...x, id: x.id || Math.random().toString(36).slice(2, 12) }));
    state = structuredClone(next);
    emit('quick:changed', state);
    return structuredClone(state);
  },
  run(a: QuickAction): string {
    switch (a.kind) {
      case 'open':
        return 'Opened';
      case 'game':
        return 'Starting the game';
      case 'keys':
        return `Pressed ${a.keys}`;
      case 'text':
        return 'Typed';
      case 'mic':
        return 'Microphone muted';
      case 'counter':
        return 'Shiny Charizard: 3,128';
      case 'screenshot':
        return 'Saved Screenshot 2026-11-03.png';
      case 'lock':
        return 'Locked';
      case 'quiet':
        return 'Screens off, fans at their lowest';
      case 'capriDesktop':
        return 'Capri is open';
      case 'obs':
        if (!obsPassword) throw "OBS isn't answering: open OBS and turn on Tools → WebSocket Server Settings → Enable WebSocket server";
        return a.what === 'scene' ? `Scene: ${a.target}` : a.what === 'stream' ? 'Streaming' : 'Done';
      default:
        return 'Done';
    }
  },
};

const pic = (w: number, h: number, a: string, c: string) => `data:image/svg+xml;utf8,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}"><defs><linearGradient id="g" x2="1" y2="1"><stop offset="0" stop-color="${a}"/><stop offset="1" stop-color="${c}"/></linearGradient></defs><rect width="100%" height="100%" fill="url(#g)"/><circle cx="${w * 0.7}" cy="${h * 0.35}" r="${h * 0.18}" fill="#fff8"/></svg>`)}`;

let items: ClipItem[] = [
  { id: 'c1', at: now() - 20_000, text: 'https://store.steampowered.com/app/1145360/Hades/', width: 0, height: 0, pinned: false, app: 'chrome.exe' },
  { id: 'c2', at: now() - 95_000, text: 'git commit -m "Fix the boss fight timer"', width: 0, height: 0, pinned: false, app: 'windowsterminal.exe' },
  { id: 'c3', at: now() - 4 * 60_000, text: '', thumb: pic(240, 135, '#7c3aed', '#0ea5e9'), width: 1920, height: 1080, pinned: false, app: 'omnihub.exe' },
  { id: 'c4', at: now() - 12 * 60_000, text: 'Shipping address:\n221B Baker Street\nLondon NW1 6XE', width: 0, height: 0, pinned: true, app: 'notepad.exe' },
  { id: 'c5', at: now() - 40 * 60_000, text: 'Meet at 8 for ranked?', width: 0, height: 0, pinned: false, app: 'discord.exe' },
  { id: 'c6', at: now() - 3 * 3600_000, text: '192.168.1.24:25565', width: 0, height: 0, pinned: true, app: 'chrome.exe' },
];
let paused = false;
const changed = () => emit('clipboard:changed', null);

export const clipboard = {
  list: (): ClipList => ({ items: structuredClone(items), paused }),
  copy: (id: string) => {
    const i = items.find((x) => x.id === id);
    if (!i) throw "That item isn't in the history any more";
    items = [{ ...i, at: now() }, ...items.filter((x) => x.id !== id)];
    changed();
  },
  pin: (id: string, on: boolean) => {
    if (on && items.find((x) => x.id === id)?.thumb) throw 'Only text can be pinned';
    items = items.map((x) => (x.id === id ? { ...x, pinned: on } : x));
    changed();
  },
  remove: (id: string) => {
    items = items.filter((x) => x.id !== id);
    changed();
  },
  clear: () => {
    items = items.filter((x) => x.pinned);
    changed();
  },
  pause: (on: boolean) => {
    paused = on;
    changed();
  },
};

const hist = (from: number, perHour: number, hours: number): [number, number][] =>
  Array.from({ length: hours * 6 + 1 }, (_, i) => [now() - (hours * 6 - i) * 600_000, Math.max(1, Math.round(from - (perHour * i) / 6))] as [number, number]);

export const batteries = (): Battery[] => [
  { id: 'pc', name: 'This PC', kind: 'pc', percent: 76, charging: true, wired: false, minutesLeft: null, history: hist(52, -6, 4) },
  { id: 'pad0', name: 'Controller 1', kind: 'controller', percent: 60, charging: null, wired: false, minutesLeft: null, history: hist(100, 6, 6) },
  { id: 'bt:1', name: 'Arctis Nova 7', kind: 'bluetooth', percent: 18, charging: false, wired: false, minutesLeft: 95, history: hist(43, 5, 5) },
  { id: 'bt:2', name: 'G Pro X Superlight', kind: 'bluetooth', percent: 64, charging: false, wired: false, minutesLeft: 2100, history: hist(70, 1, 6) },
  { id: 'bt:3', name: 'DualSense Wireless Controller', kind: 'bluetooth', percent: 91, charging: false, wired: false, minutesLeft: null, history: hist(93, 0.4, 5) },
];

export const health = (): PcHealth => ({
  score: 73,
  at: now(),
  checks: [
    { id: 'disk:C:', title: 'Space on C:', status: 'warn', detail: '62 GB free of 476 GB (13%)', fix: 'Free up space', route: 'storage' },
    { id: 'disk:D:', title: 'Space on D:', status: 'ok', detail: '1104 GB free of 1863 GB (59%)', fix: null, route: null },
    { id: 'memory', title: 'Memory', status: 'ok', detail: '48% in use', fix: null, route: null },
    { id: 'restart', title: 'Restarted lately', status: 'warn', detail: 'On for 9 days', fix: 'Restart when you can', route: null },
    { id: 'startup', title: 'Apps at startup', status: 'bad', detail: '17 start with Windows', fix: 'Turn some off', route: 'apps?filter=startup' },
    { id: 'temp', title: 'Temporary files', status: 'ok', detail: '0.6 GB', fix: null, route: null },
    { id: 'bin', title: 'Recycle Bin', status: 'ok', detail: '0.3 GB', fix: null, route: null },
    { id: 'downloads', title: 'Downloads folder', status: 'ok', detail: '8.2 GB in 312 files', fix: null, route: null },
  ],
});

let presence: PresenceSettings = { enabled: false, clientId: '', showGame: true, showHunt: true };
export const discord = {
  status: (): PresenceStatus => ({ settings: { ...presence }, connected: presence.enabled, showing: presence.enabled ? { details: 'Fortnite', state: 'Shiny hunting Charizard: 3127 encounters (1/1366)', start: Date.now() - 1_800_000 } : null, error: null }),
  save: (s: PresenceSettings) => {
    if (s.enabled && !/^\d{15,22}$/.test(s.clientId.trim())) throw 'Paste your Discord Application ID (a long number from the Developer Portal)';
    presence = { ...s, clientId: s.clientId.trim() };
    return discord.status();
  },
};

export const network = (): NetStatus => {
  const now = Date.now();
  const history = Array.from({ length: 60 }, (_, i) => [now - (59 - i) * 2000, Math.round(2_400_000 + Math.sin(i / 4) * 1_600_000 + Math.random() * 900_000), Math.round(180_000 + Math.random() * 120_000)] as [number, number, number]);
  const days = Array.from({ length: 30 }, (_, i) => {
    const d = new Date(now - (29 - i) * 86400_000);
    return { date: d.toISOString().slice(0, 10), down: Math.round((4 + Math.random() * 18) * 1e9), up: Math.round((0.3 + Math.random() * 1.5) * 1e9) };
  });
  return {
    down: history[59][1],
    up: history[59][2],
    history,
    days,
    supported: true,
    apps: [
      { process: 'chrome.exe', pid: 4120, connections: 38, remotes: ['142.250.180.14:443', '151.101.1.69:443', '104.18.32.47:443'] },
      { process: 'Discord.exe', pid: 8812, connections: 9, remotes: ['162.159.135.234:443', '162.159.130.232:443'] },
      { process: 'steam.exe', pid: 6604, connections: 6, remotes: ['23.62.236.33:443', '155.133.248.37:27030'] },
      { process: 'FortniteClient-Win64-Shipping.exe', pid: 10232, connections: 4, remotes: ['3.120.4.200:7777', '18.185.10.1:443'] },
      { process: 'OneDrive.exe', pid: 2208, connections: 2, remotes: ['13.107.42.12:443'] },
    ],
  };
};

let jobs: BackupJob[] = [
  { id: 'b1', name: 'Documents & photos', sources: ['C:\\Users\\Player\\Documents', 'C:\\Users\\Player\\Pictures'], dest: 'E:\\', schedule: 'whenConnected', enabled: true, lastRun: Math.floor(Date.now() / 1000) - 26 * 3600, lastResult: '214 copied, 18,302 up to date', lastOk: true },
  { id: 'b2', name: 'Minecraft worlds', sources: ['C:\\Users\\Player\\AppData\\Roaming\\.minecraft\\saves'], dest: 'D:\\Backups', schedule: 'daily', enabled: true, lastRun: Math.floor(Date.now() / 1000) - 3 * 3600, lastResult: '12 copied, 3,410 up to date, 2 couldn’t be read', lastOk: false },
];
let runningJob: string | null = null;
export const backups = {
  list: () => ({ jobs: structuredClone(jobs), running: runningJob }),
  save: (j: BackupJob) => {
    if (!j.name.trim()) throw 'Give the backup a name';
    if (!j.sources.length) throw 'Pick at least one folder to back up';
    if (!j.dest) throw 'Pick where the backup goes (a drive or folder)';
    const saved = { ...j, id: j.id || Math.random().toString(36).slice(2, 10) };
    jobs = jobs.some((x) => x.id === saved.id) ? jobs.map((x) => (x.id === saved.id ? { ...x, ...saved, lastRun: x.lastRun, lastResult: x.lastResult, lastOk: x.lastOk } : x)) : [...jobs, saved];
    emit('backup:changed', null);
    return saved;
  },
  remove: (id: string) => {
    jobs = jobs.filter((j) => j.id !== id);
    emit('backup:changed', null);
  },
  run: (id: string) => {
    runningJob = id;
    emit('backup:changed', null);
    setTimeout(() => {
      runningJob = null;
      jobs = jobs.map((j) => (j.id === id ? { ...j, lastRun: Math.floor(Date.now() / 1000), lastResult: '37 copied, 18,479 up to date', lastOk: true } : j));
      const j = jobs.find((x) => x.id === id);
      emit('backup:done', { job: id, name: j?.name ?? '', copied: 37, bytes: 412_000_000, same: 18479, failed: 0, missingDest: false });
      emit('backup:changed', null);
    }, 1800);
  },
};
