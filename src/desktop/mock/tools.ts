// Demo data for the file tools, ports, window layouts and privacy pages.

import type { HashKind, LayoutApplied, PortUse, PrivacyUse, RecentFile, RenameItem, RenameRule, WindowLayout } from '@shared/types';
import { emit } from './bus';

const now = () => Math.floor(Date.now() / 1000);
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function hash(path: string, kind: HashKind, job: string): Promise<string> {
  const total = 734_003_200;
  for (let done = 0; done <= total; done += total / 8) {
    emit('tools:progress', { job, name: path.split('\\').pop(), done, total });
    await sleep(80);
  }
  const len = kind === 'sha1' ? 40 : kind === 'sha512' ? 128 : 64;
  let h = '';
  for (let i = 0; h.length < len; i++) h += ((path.charCodeAt(i % path.length) * 2654435761 + i) >>> 0).toString(16).padStart(8, '0');
  return h.slice(0, len);
}

export async function shred(_path: string, job: string): Promise<number> {
  emit('tools:progress', { job, name: 'shredding', done: 1, total: 1 });
  await sleep(300);
  return 3;
}

function newName(old: string, rule: RenameRule, i: number): string {
  const dot = old.lastIndexOf('.');
  let stem = dot > 0 ? old.slice(0, dot) : old;
  let ext = dot > 0 ? old.slice(dot) : '';
  if (rule.find) stem = stem.split(rule.find).join(rule.replace);
  if (rule.pattern) {
    const out = rule.pattern
      .replace(/\{n:(\d)\}/g, (_, w) => String(rule.start + i).padStart(Number(w), '0'))
      .replace(/\{n\}/g, String(rule.start + i))
      .replace(/\{name\}/g, stem)
      .replace(/\{date\}/g, '2026-10-30')
      .replace(/\{time\}/g, '09-05-07')
      .replace(/\{ext\}/g, ext.slice(1));
    if (rule.pattern.includes('{ext}') || /\.[^}]*$/.test(rule.pattern)) ext = '';
    stem = out;
  }
  if (rule.case === 'lower') stem = stem.toLowerCase();
  if (rule.case === 'upper') stem = stem.toUpperCase();
  if (rule.case === 'title') stem = stem.replace(/\b\w/g, (c) => c.toUpperCase());
  return stem + ext;
}

export function renamePlan(files: string[], rule: RenameRule): RenameItem[] {
  const items = files.map((f, i) => {
    const name = f.split('\\').pop() ?? f;
    return { from: f, to: newName(name, rule, i), problem: '' };
  });
  for (const it of items) {
    if (/[<>:"/\\|?*]/.test(it.to)) it.problem = 'A name can’t have the character ' + (it.to.match(/[<>:"/\\|?*]/)?.[0] ?? '');
    else if (items.filter((x) => x.to.toLowerCase() === it.to.toLowerCase()).length > 1) it.problem = 'Two files would get this name.';
  }
  return items;
}

export async function rename(files: string[], rule: RenameRule): Promise<number> {
  const plan = renamePlan(files, rule);
  const bad = plan.find((p) => p.problem);
  if (bad) throw bad.problem;
  await sleep(200);
  return plan.length;
}

export function recent(): RecentFile[] {
  const base = 'C:\\Users\\Player\\';
  return [
    ['Documents\\Budget 2026.xlsx', 48_210, 600],
    ['Downloads\\OmniHub_0.20.0_x64-setup.exe', 18_400_000, 1_800],
    ['Pictures\\Screenshots\\Fortnite win.png', 2_310_000, 5_400],
    ['Documents\\Essay draft.docx', 92_400, 9_000],
    ['Videos\\OmniHub Clips\\Counter-Strike 2 2026-10-28 18-02-51.mp4', 61_900_000, 86_400],
    ['Desktop\\notes.txt', 1_204, 172_800],
  ].map(([p, size, ago]) => ({ path: base + p, name: String(p).split('\\').pop() ?? '', size: Number(size), used: now() - Number(ago) }));
}

const portList: PortUse[] = [
  { proto: 'TCP', local: '0.0.0.0', port: 3000, remote: null, remotePort: null, state: 'Listening', pid: 18244, process: 'node.exe' },
  { proto: 'TCP', local: '127.0.0.1', port: 5173, remote: null, remotePort: null, state: 'Listening', pid: 18312, process: 'node.exe' },
  { proto: 'TCP', local: '0.0.0.0', port: 47800, remote: null, remotePort: null, state: 'Listening', pid: 9120, process: 'omnihub.exe' },
  { proto: 'TCP', local: '0.0.0.0', port: 25565, remote: null, remotePort: null, state: 'Listening', pid: 22016, process: 'java.exe' },
  { proto: 'TCP', local: '0.0.0.0', port: 445, remote: null, remotePort: null, state: 'Listening', pid: 4, process: 'System' },
  { proto: 'TCP', local: '192.168.1.24', port: 52144, remote: '140.82.112.25', remotePort: 443, state: 'Connected', pid: 7720, process: 'chrome.exe' },
  { proto: 'TCP', local: '192.168.1.24', port: 52188, remote: '162.159.130.234', remotePort: 443, state: 'Connected', pid: 11840, process: 'Discord.exe' },
  { proto: 'UDP', local: '0.0.0.0', port: 5353, remote: null, remotePort: null, state: '', pid: 7720, process: 'chrome.exe' },
  { proto: 'UDP', local: '0.0.0.0', port: 27015, remote: null, remotePort: null, state: '', pid: 24100, process: 'cs2.exe' },
];

export function ports(): PortUse[] {
  return portList;
}

export function endPort(pid: number): void {
  if (pid <= 4) throw 'That’s Windows itself.';
  for (let i = portList.length - 1; i >= 0; i--) if (portList[i].pid === pid) portList.splice(i, 1);
}

const win = (key: string, name: string, title: string, l: number, t: number, r: number, b: number, state: 'normal' | 'maximized' = 'normal') => ({ key, name, title, path: `C:\\Program Files\\${name}\\${key}`, left: l, top: t, right: r, bottom: b, state });
let layoutList: WindowLayout[] = [
  { name: 'Gaming', savedAt: now() - 3 * 86_400, windows: [win('discord.exe', 'Discord', '#general - Discord', 1920, 0, 2880, 1040), win('spotify.exe', 'Spotify', 'Spotify Premium', 2880, 0, 3840, 1040)] },
  { name: 'Work', savedAt: now() - 86_400, windows: [win('code.exe', 'Visual Studio Code', 'omnihub - Visual Studio Code', 0, 0, 1920, 1040, 'maximized'), win('chrome.exe', 'Google Chrome', 'Docs - Google Chrome', 1920, 0, 3840, 1040), win('slack.exe', 'Slack', 'Slack', 1920, 540, 2880, 1040)] },
];

export function layouts(): WindowLayout[] {
  return layoutList;
}

export function saveLayout(name: string): WindowLayout {
  const n = name.trim();
  if (!n) throw 'Give the layout a name (up to 40 letters).';
  const l = { name: n, savedAt: now(), windows: [win('code.exe', 'Visual Studio Code', 'omnihub - Visual Studio Code', 0, 0, 1920, 1040, 'maximized'), win('spotify.exe', 'Spotify', 'Spotify Premium', 1920, 0, 3840, 1040)] };
  layoutList = [...layoutList.filter((x) => x.name.toLowerCase() !== n.toLowerCase()), l].sort((a, b) => a.name.localeCompare(b.name));
  return l;
}

export async function applyLayout(name: string, launch: boolean): Promise<LayoutApplied> {
  const l = layoutList.find((x) => x.name === name);
  if (!l) throw 'That layout is gone.';
  await sleep(launch ? 900 : 200);
  const [first, ...rest] = l.windows;
  return launch ? { placed: l.windows.length, launched: rest.map((w) => w.name), missing: [] } : { placed: 1, launched: [], missing: first ? rest.map((w) => w.name) : [] };
}

export function removeLayout(name: string): void {
  layoutList = layoutList.filter((x) => x.name !== name);
}

export function privacy(): PrivacyUse[] {
  return [
    { what: 'microphone', app: 'Discord', id: 'C:\\Users\\Player\\AppData\\Local\\Discord\\app-1.0.9200\\Discord.exe', started: now() - 1_260, stopped: null, inUse: true },
    { what: 'webcam', app: 'Microsoft Teams', id: 'MSTeams_8wekyb3d8bbwe', started: now() - 7_200, stopped: now() - 3_600, inUse: false },
    { what: 'microphone', app: 'Google Chrome', id: 'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe', started: now() - 86_400, stopped: now() - 85_000, inUse: false },
    { what: 'location', app: 'Weather', id: 'Microsoft.BingWeather_8wekyb3d8bbwe', started: now() - 3 * 86_400, stopped: now() - 3 * 86_400 + 4, inUse: false },
    { what: 'webcam', app: 'Windows Camera', id: 'Microsoft.WindowsCamera_8wekyb3d8bbwe', started: now() - 9 * 86_400, stopped: now() - 9 * 86_400 + 300, inUse: false },
  ];
}
