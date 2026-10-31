// Demo data for guest uploads, folder rules, focus and time, and dev tools.

import type { DevEnvironment, DevProject, FocusState, FolderRule, GuestCreated, GuestLink, LanLink, RuleDone, TimeReport } from '@shared/types';
import { emit } from './bus';

const now = () => Math.floor(Date.now() / 1000);
/** A QR-looking square for the demo (the app draws a real one). */
const DEMO_QR = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 21 21" shape-rendering="crispEdges"><rect width="21" height="21" fill="#fff"/>${[[0, 0], [14, 0], [0, 14]].map(([x, y]) => `<rect x="${x}" y="${y}" width="7" height="7"/><rect x="${x + 1}" y="${y + 1}" width="5" height="5" fill="#fff"/><rect x="${x + 2}" y="${y + 2}" width="3" height="3"/>`).join('')}${Array.from({ length: 60 }, (_, i) => `<rect x="${8 + ((i * 7) % 13)}" y="${(i * 5) % 21}" width="1" height="1"/>`).join('')}</svg>`;

let guests: GuestLink[] = [];
export function guestCreate(label: string, minutes: number, maxBytes: number): GuestCreated {
  const code = Math.random().toString(36).slice(2, 14);
  const link = { code, label: label.trim() || 'Guest', created: now(), expiresAt: now() + minutes * 60, maxBytes, usedBytes: 0, files: 0 };
  guests = [...guests, link];
  const url = `https://192.168.1.24:47800/guest/${code}`;
  return { link, urls: [url], qrSvgs: [DEMO_QR] };
}
export const guestList = () => guests;
export function guestStop(code: string) {
  guests = guests.filter((g) => g.code !== code);
}

let rules: FolderRule[] = [
  { id: 'r1', name: 'PDFs to Documents', enabled: true, folder: 'C:\\Users\\Player\\Downloads', extensions: ['pdf'], contains: [], action: { kind: 'move', to: 'C:\\Users\\Player\\Documents\\PDFs' } },
  { id: 'r2', name: 'Old installers', enabled: true, folder: 'C:\\Users\\Player\\Downloads', extensions: ['exe', 'msi'], contains: [], action: { kind: 'recycle', days: 30 } },
];
const log: RuleDone[] = [
  { at: now() - 600, rule: 'PDFs to Documents', file: 'Invoice October.pdf', what: 'Moved to C:\\Users\\Player\\Documents\\PDFs', ok: true },
  { at: now() - 7200, rule: 'Old installers', file: 'SteamSetup.exe', what: 'Sent to the Recycle Bin', ok: true },
];
export const ruleList = () => rules;
export function ruleSave(r: FolderRule): FolderRule {
  if (!r.name.trim()) throw 'Give the rule a name.';
  if (!r.folder) throw 'Pick the folder to watch.';
  const saved = { ...r, id: r.id || Math.random().toString(36).slice(2) };
  rules = rules.some((x) => x.id === saved.id) ? rules.map((x) => (x.id === saved.id ? saved : x)) : [...rules, saved];
  return saved;
}
export function ruleDelete(id: string) {
  rules = rules.filter((r) => r.id !== id);
}
export const ruleLog = () => log;

let focus: FocusState | null = null;
export const focusState = () => focus;
export function focusStart(f: number, b: number, blocked: string[]): FocusState {
  focus = { phase: 'focus', endsAt: Date.now() + f * 60_000, focusMinutes: f, breakMinutes: b, rounds: 0, blocked, closed: [] };
  emit('focus:changed', {});
  return focus;
}
export function focusStop() {
  focus = null;
  emit('focus:changed', {});
}
export function timeReport(tracking: boolean): TimeReport {
  const day = (d: number) => new Date(Date.now() - d * 86_400_000).toISOString().slice(0, 10);
  return {
    tracking,
    day: day(0),
    apps: tracking
      ? [
          { app: 'code.exe', name: 'Visual Studio Code', seconds: 3 * 3600 + 1200 },
          { app: 'chrome.exe', name: 'Google Chrome', seconds: 2 * 3600 + 300 },
          { app: 'discord.exe', name: 'Discord', seconds: 2700 },
          { app: 'spotify.exe', name: 'Spotify', seconds: 1500 },
          { app: 'fortniteclient-win64-shipping.exe', name: 'Fortnite', seconds: 1200 },
        ]
      : [],
    week: [6, 5, 4, 3, 2, 1, 0].map((d) => ({ day: day(d), seconds: tracking ? [21_000, 18_500, 25_300, 9_800, 16_200, 22_400, 24_900][6 - d] : 0 })),
    focusMinutesToday: tracking ? 75 : 0,
    sessionsToday: tracking ? 3 : 0,
  };
}

export function projects(roots: string[]): DevProject[] {
  if (!roots.length) return [];
  const base = roots[0];
  return [
    { path: `${base}\\omnihub`, name: 'omnihub', branch: 'main', changes: 3, ahead: 2, behind: 0, upstream: true, lastCommit: now() - 1800, lastMessage: 'Aurora music hub', note: null },
    { path: `${base}\\portfolio`, name: 'portfolio', branch: 'redesign', changes: 0, ahead: 0, behind: 4, upstream: true, lastCommit: now() - 3 * 86_400, lastMessage: 'New hero section', note: null },
    { path: `${base}\\mc-plugin`, name: 'mc-plugin', branch: 'main', changes: 0, ahead: 0, behind: 0, upstream: false, lastCommit: now() - 20 * 86_400, lastMessage: 'First version', note: null },
  ];
}

let env: DevEnvironment = {
  userPath: [
    { path: '%USERPROFILE%\\AppData\\Local\\Microsoft\\WindowsApps', exists: true },
    { path: 'C:\\Users\\Player\\AppData\\Local\\Programs\\Microsoft VS Code\\bin', exists: true },
    { path: 'C:\\Users\\Player\\.cargo\\bin', exists: true },
    { path: 'C:\\tools\\old-sdk\\bin', exists: false },
  ],
  systemPath: [
    { path: 'C:\\Windows\\system32', exists: true },
    { path: 'C:\\Windows', exists: true },
    { path: 'C:\\Program Files\\Git\\cmd', exists: true },
    { path: 'C:\\Program Files\\nodejs\\', exists: true },
  ],
  userVars: [
    ['JAVA_HOME', 'C:\\Program Files\\Eclipse Adoptium\\jdk-21'],
    ['ANDROID_HOME', '%LOCALAPPDATA%\\Android\\Sdk'],
  ],
};
export const environment = () => env;
export function setPath(entries: string[]): DevEnvironment {
  env = { ...env, userPath: entries.map((p) => ({ path: p, exists: !p.includes('old-sdk') })) };
  return env;
}
export function setVar(name: string, value: string): DevEnvironment {
  env = { ...env, userVars: [...env.userVars.filter(([n]) => n !== name), [name, value] as [string, string]].sort((a, b) => a[0].localeCompare(b[0])) };
  return env;
}
export function removeVar(name: string): DevEnvironment {
  env = { ...env, userVars: env.userVars.filter(([n]) => n !== name) };
  return env;
}
export function lanLink(port: number, local: string): LanLink {
  if (local.startsWith('127.')) return { urls: [], qrSvgs: [], note: 'It only listens on this PC (127.0.0.1). Start it with --host 0.0.0.0 so your phone can reach it.' };
  return { urls: [`http://192.168.1.24:${port}`], qrSvgs: [DEMO_QR], note: '' };
}
