// Installed apps and screenshots for the mock backend.

import type { AppInfo, CaptureKind, PendingRegion, Rect, Screenshot, ShotFilter } from '@shared/types';
import { emit } from './bus';
import { appIcon, scene, type SceneKind } from './art';
import { audit } from './core';
import { driveTree } from './drives';
import { DAY, MB, NOW, Rng, sleep } from './rng';
import { scanList } from './storage';

// ---------- apps ----------

type AppSeed = [name: string, publisher: string, version: string, source: AppInfo['source'], location: string | null, ageDays: number | null, estMb: number | null, exe: string | null, colors: [string, string] | null];

const PF = 'C:\\Program Files';
const PF86 = 'C:\\Program Files (x86)';
const LOCAL = 'C:\\Users\\Alex\\AppData\\Local';
const GAMES = 'C:\\Games\\Steam\\steamapps\\common';
const WA = 'C:\\Program Files\\WindowsApps';

const APPS: AppSeed[] = [
  ['Steam', 'Valve Corporation', '2.10.91.91', 'desktop', `${PF86}\\Steam`, 690, 1900, 'steam.exe', ['#1b2838', '#2a6b9e']],
  ['Discord', 'Discord Inc.', '1.0.9163', 'desktop', `${LOCAL}\\Discord`, 540, 610, 'Discord.exe', ['#5865f2', '#8b93ff']],
  ['Visual Studio Code', 'Microsoft Corporation', '1.93.1', 'desktop', `${LOCAL}\\Programs\\Microsoft VS Code`, 420, 450, 'Code.exe', ['#0065a9', '#2bb4f5']],
  ['Mozilla Firefox', 'Mozilla', '131.0', 'desktop', `${PF}\\Mozilla Firefox`, 610, 280, 'firefox.exe', ['#ff7139', '#9059ff']],
  ['Google Chrome', 'Google LLC', '129.0.6668.90', 'desktop', `${PF}\\Google\\Chrome\\Application`, 820, 640, 'chrome.exe', ['#ea4335', '#fbbc05']],
  ['OBS Studio', 'OBS Project', '30.2.3', 'desktop', `${PF}\\obs-studio`, 380, 500, 'obs64.exe', ['#302e31', '#6b6870']],
  ['Blender 4.2', 'Blender Foundation', '4.2.2', 'desktop', `${PF}\\Blender Foundation\\Blender 4.2`, 88, 1450, 'blender.exe', ['#e87d0d', '#265787']],
  ['7-Zip 24.08 (x64)', 'Igor Pavlov', '24.08', 'desktop', `${PF}\\7-Zip`, 300, 6, '7zFM.exe', ['#1f2937', '#4b5563']],
  ['Adobe Premiere Pro 2025', 'Adobe Inc.', '25.0', 'desktop', `${PF}\\Adobe\\Adobe Premiere Pro 2025`, 70, 10200, 'Adobe Premiere Pro.exe', ['#2a0a54', '#9999ff']],
  ['Adobe Photoshop 2025', 'Adobe Inc.', '26.0', 'desktop', `${PF}\\Adobe\\Adobe Photoshop 2025`, 70, 5600, 'Photoshop.exe', ['#001e36', '#31a8ff']],
  ['Adobe Creative Cloud', 'Adobe Inc.', '6.4.0.361', 'desktop', `${PF}\\Adobe\\Adobe Creative Cloud`, 70, 1200, 'Creative Cloud.exe', ['#da1f26', '#ff6b6b']],
  ['Docker Desktop', 'Docker Inc.', '4.34.2', 'desktop', `${PF}\\Docker\\Docker`, 160, 3300, 'Docker Desktop.exe', ['#1d63ed', '#4f9bff']],
  ['Git', 'The Git Development Community', '2.46.0', 'desktop', `${PF}\\Git`, 120, 420, null, ['#f05133', '#c2410c']],
  ['Microsoft 365 Apps for enterprise', 'Microsoft Corporation', '16.0.18025.20104', 'desktop', `${PF}\\Microsoft Office\\root`, 900, 4600, 'WINWORD.EXE', ['#d83b01', '#ff8c00']],
  ['Microsoft Visual Studio Community 2022', 'Microsoft Corporation', '17.11.4', 'desktop', `${PF}\\Microsoft Visual Studio\\2022\\Community`, 260, 11800, 'devenv.exe', ['#5c2d91', '#a77bdb']],
  ['Node.js', 'Node.js Foundation', '20.17.0', 'desktop', `${PF}\\nodejs`, 33, 180, null, ['#215732', '#6cc24a']],
  ['NVIDIA App', 'NVIDIA Corporation', '10.0.3.210', 'desktop', `${PF}\\NVIDIA Corporation\\NVIDIA app`, 44, 600, 'NVIDIA app.exe', ['#76b900', '#3d6b00']],
  ['NVIDIA Graphics Driver 560.94', 'NVIDIA Corporation', '560.94', 'desktop', null, 44, 1400, null, null],
  ['PowerShell 7-x64', 'Microsoft Corporation', '7.4.5', 'desktop', `${PF}\\PowerShell\\7`, 150, 220, 'pwsh.exe', ['#012456', '#2671be']],
  ['VLC media player', 'VideoLAN', '3.0.21', 'desktop', `${PF}\\VideoLAN\\VLC`, 400, 190, 'vlc.exe', ['#ff8800', '#e85e00']],
  ['Epic Games Launcher', 'Epic Games, Inc.', '17.2.0', 'desktop', `${PF86}\\Epic Games\\Launcher`, 500, 900, 'EpicGamesLauncher.exe', ['#2a2a2a', '#5a5a5a']],
  ['Fortnite', 'Epic Games, Inc.', '31.30', 'desktop', `${PF}\\Epic Games\\Fortnite`, 18, 36000, 'FortniteClient-Win64-Shipping.exe', ['#7c3aed', '#38bdf8']],
  ['Battle.net', 'Blizzard Entertainment', '2.36.0', 'desktop', `${PF86}\\Battle.net`, 380, 480, 'Battle.net.exe', ['#148eff', '#0a4b8c']],
  ['Microsoft Edge', 'Microsoft Corporation', '129.0.2792.65', 'desktop', `${PF86}\\Microsoft\\Edge\\Application`, 900, 650, 'msedge.exe', ['#0c59a4', '#2fd6a8']],
  ['Windows Software Development Kit', 'Microsoft Corporation', '10.1.26100.1', 'desktop', `${PF86}\\Windows Kits\\10`, 260, 3300, null, null],
  ['Python 3.12.5 (64-bit)', 'Python Software Foundation', '3.12.5150.0', 'desktop', `${LOCAL}\\Programs\\Python\\Python312`, 54, 200, 'python.exe', ['#306998', '#ffd43b']],
  ['Cyberpunk 2077', 'CD PROJEKT RED', '2.13', 'desktop', `${GAMES}\\Cyberpunk 2077`, 30, 70000, 'Cyberpunk2077.exe', ['#fcee0a', '#00f0ff']],
  ["Baldur's Gate 3", 'Larian Studios', '4.1.1.5022896', 'desktop', `${GAMES}\\Baldurs Gate 3`, 52, 118000, 'bg3.exe', ['#7a1e1e', '#d4a24c']],
  ['ELDEN RING', 'FromSoftware Inc.', '1.14', 'desktop', `${GAMES}\\ELDEN RING`, 140, 50000, 'eldenring.exe', ['#3f3a2a', '#c8a85a']],
  ['Counter-Strike 2', 'Valve', '1.40.3.4', 'desktop', `${GAMES}\\Counter-Strike Global Offensive`, 6, 33000, 'cs2.exe', ['#de9b35', '#1b1b1b']],
  ['Monster Hunter Wilds', 'CAPCOM Co., Ltd.', '1.0.2', 'desktop', `${GAMES}\\Monster Hunter Wilds`, 22, 74000, 'MonsterHunterWilds.exe', ['#5b3a1a', '#e3b04b']],
  ['Hades II', 'Supergiant Games', '0.95.2', 'desktop', `${GAMES}\\Hades II`, 16, 9000, 'Hades2.exe', ['#4c1d95', '#22d3ee']],
  ['Wallpaper Engine', 'Wallpaper Engine Team', '2.5.28', 'desktop', `${PF86}\\Steam\\steamapps\\common\\Wallpaper Engine`, 70, 1100, 'wallpaper64.exe', ['#1e88e5', '#00bcd4']],
  ['JetBrains Toolbox', 'JetBrains s.r.o.', '2.4.2', 'desktop', `${LOCAL}\\JetBrains\\Toolbox`, 200, 300, 'jetbrains-toolbox.exe', ['#000000', '#fe2857']],
  ['Ollama', 'Ollama', '0.3.12', 'startMenu', `${LOCAL}\\Programs\\Ollama`, 60, null, 'ollama app.exe', ['#111827', '#e5e7eb']],
  ['Obsidian', 'Obsidian', '1.6.7', 'startMenu', `${LOCAL}\\Programs\\Obsidian`, 230, null, 'Obsidian.exe', ['#483699', '#a88bfa']],
  ['Telegram Desktop', 'Telegram FZ-LLC', '5.5.5', 'startMenu', 'C:\\Users\\Alex\\AppData\\Roaming\\Telegram Desktop', 330, null, 'Telegram.exe', ['#229ed9', '#2aabee']],
  ['Calculator', 'Microsoft Corporation', '11.2405.2.0', 'store', `${WA}\\Microsoft.WindowsCalculator_11.2405.2.0_x64__8wekyb3d8bbwe`, 120, null, null, ['#3b3b3b', '#6b6b6b']],
  ['Photos', 'Microsoft Corporation', '2024.11070.15005.0', 'store', `${WA}\\Microsoft.Windows.Photos_2024.11070.15005.0_x64__8wekyb3d8bbwe`, 120, null, null, ['#0078d4', '#50e6ff']],
  ['Windows Terminal', 'Microsoft Corporation', '1.21.2361.0', 'store', `${WA}\\Microsoft.WindowsTerminal_1.21.2361.0_x64__8wekyb3d8bbwe`, 300, null, 'WindowsTerminal.exe', ['#1f1f1f', '#4cc2ff']],
  ['Spotify Music', 'Spotify AB', '1.247.462.0', 'store', `${WA}\\SpotifyAB.SpotifyMusic_1.247.462.0_x64__zpdnekdrzrea0`, 700, null, 'Spotify.exe', ['#1db954', '#0c7d36']],
  ['Minecraft for Windows', 'Microsoft Studios', '1.21.3101.0', 'store', `${WA}\\Microsoft.MinecraftUWP_1.21.3101.0_x64__8wekyb3d8bbwe`, 400, null, 'Minecraft.Windows.exe', ['#5a8f29', '#8b5a2b']],
  ['Xbox', 'Microsoft Corporation', '2409.1001.5.0', 'store', `${WA}\\Microsoft.GamingApp_2409.1001.5.0_x64__8wekyb3d8bbwe`, 300, null, null, ['#107c10', '#5dc21e']],
  ['Notepad', 'Microsoft Corporation', '11.2408.12.0', 'store', `${WA}\\Microsoft.WindowsNotepad_11.2408.12.0_x64__8wekyb3d8bbwe`, 300, null, 'Notepad.exe', ['#2b88d8', '#73c1ff']],
  ['Snipping Tool', 'Microsoft Corporation', '11.2408.13.0', 'store', `${WA}\\Microsoft.ScreenSketch_11.2408.13.0_x64__8wekyb3d8bbwe`, 300, null, null, ['#c239b3', '#ff7eb6']],
  ['Microsoft To Do', 'Microsoft Corporation', '2.114.7122.0', 'store', `${WA}\\Microsoft.Todos_2.114.7122.0_x64__8wekyb3d8bbwe`, 280, null, null, ['#185abd', '#4a90e2']],
  ['Phone Link', 'Microsoft Corporation', '1.24082.142.0', 'store', `${WA}\\Microsoft.YourPhone_1.24082.142.0_x64__8wekyb3d8bbwe`, 280, null, null, ['#0f6cbd', '#62abf5']],
  ['Clipchamp', 'Microsoft Corp.', '3.1.11920.0', 'store', `${WA}\\Microsoft.Clipchamp_3.1.11920.0_x64__yxz26nhyzhsrt`, 200, null, null, ['#6f2da8', '#c13584']],
];

function appId(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

const apps: AppInfo[] = APPS.map(([name, publisher, version, source, location, ageDays, estMb]) => ({
  id: appId(name),
  name,
  publisher,
  version,
  source,
  installLocation: location,
  installDate: ageDays == null ? null : NOW - ageDays * DAY,
  size: estMb == null ? null : estMb * MB,
  sizeFromScan: false,
  aumid: source === 'store' ? `${(location ?? '').split('\\').pop()?.split('_')[0]}_8wekyb3d8bbwe!App` : null,
  launchable: !name.startsWith('NVIDIA Graphics') && !name.startsWith('Windows Software') && name !== 'Git' && name !== 'Node.js',
  uninstallable: source !== 'startMenu' && name !== 'Calculator',
}));

function sizeFromScans(path: string): number | null {
  const loaded = scanList().some((s) => path.toLowerCase().startsWith(s.root.toLowerCase().replace(/\\$/, '')));
  if (!loaded) return null;
  const tree = driveTree(path);
  const id = tree?.findPath(path);
  return tree && id != null ? tree.node(id).size : null;
}

export function appsList(): AppInfo[] {
  return apps.map((a) => {
    const scanned = a.installLocation ? sizeFromScans(a.installLocation) : null;
    return scanned != null && scanned > 0 ? { ...a, size: scanned, sizeFromScan: true } : a;
  });
}

const NO_ICON = new Set(['nvidia-graphics-driver-560-94', 'windows-software-development-kit']);
export async function appIconFor(id: string): Promise<string | null> {
  await sleep(40 + Math.random() * 260);
  if (NO_ICON.has(id)) return null;
  const seed = APPS.find((a) => appId(a[0]) === id);
  if (!seed) throw new Error(`unknown app ${id}`);
  return appIcon(seed[0], seed[8] ?? undefined);
}

export function appLaunch(id: string): void {
  const a = apps.find((x) => x.id === id);
  if (!a) throw new Error(`unknown app ${id}`);
  if (!a.launchable) throw new Error(`${a.name} has no program to start`);
  audit('desktop', 'apps.launch', a.name);
}

export function appUninstall(id: string): void {
  const a = apps.find((x) => x.id === id);
  if (!a) throw new Error(`unknown app ${id}`);
  if (!a.uninstallable) throw new Error(`${a.name} cannot be uninstalled from OmniHub`);
  audit('desktop', 'apps.uninstall', `Started the uninstaller for ${a.name}`);
}

function exeFor(id: string): string | null {
  return APPS.find((a) => appId(a[0]) === id)?.[7] ?? null;
}

// ---------- screenshots ----------

const SHOT_APPS: [exe: string, title: string, kind: SceneKind, tags: string[]][] = [
  ['Cyberpunk2077.exe', 'Cyberpunk 2077 (C) 2020 by CD Projekt RED', 'game', ['game', 'photo-mode']],
  ['bg3.exe', "Baldur's Gate 3", 'game', ['game']],
  ['eldenring.exe', 'ELDEN RING™', 'game', ['game', 'boss']],
  ['cs2.exe', 'Counter-Strike 2', 'game', ['game']],
  ['MonsterHunterWilds.exe', 'Monster Hunter Wilds', 'game', ['game']],
  ['Code.exe', 'treemap.ts — omnihub — Visual Studio Code', 'code', ['dev', 'bug']],
  ['firefox.exe', 'Linear – Roadmap — Mozilla Firefox', 'browser', ['inspiration', 'ui-idea']],
  ['chrome.exe', 'Order confirmation – Google Chrome', 'browser', ['receipt']],
  ['Discord.exe', '#general | Game Night - Discord', 'chat', ['meme']],
  ['blender.exe', 'Blender [C:\\Users\\Alex\\Documents\\scene_v4.blend]', 'blender', ['3d', 'wip']],
];

const NOTES = ['', '', '', 'Photo mode — use for wallpaper', 'Repro: crash when zooming out twice quickly', 'Love this sidebar density', 'Return window ends Oct 30', 'Boss phase 2 positioning', 'Lighting pass v2'];

const r = new Rng(0x5ca7);
const shots: Screenshot[] = [];
const kinds = new Map<string, SceneKind>();
for (let i = 0; i < 46; i++) {
  const ageH = Math.pow(r.next(), 1.4) * 27 * 24 + 0.4;
  const created = Math.floor(NOW - ageH * 3600);
  const full = r.chance(0.16);
  const [exe, title, kind, tagPool] = full ? (['', '', 'desktop', ['desktop']] as const) : r.pick(SHOT_APPS);
  const big = r.chance(0.3);
  const width = kind === 'code' || kind === 'chat' ? 1920 : big ? 3840 : 2560;
  const height = Math.round((width * 9) / 16);
  const d = new Date(created * 1000);
  const p2 = (n: number) => String(n).padStart(2, '0');
  const name = `${d.getFullYear()}-${p2(d.getMonth() + 1)}-${p2(d.getDate())}_${p2(d.getHours())}-${p2(d.getMinutes())}-${p2(d.getSeconds())}.png`;
  const id = `shot-${(1000 + i).toString(36)}${r.hex(4)}`;
  kinds.set(id, kind);
  const tags = r.chance(0.55) ? [...tagPool].slice(0, r.int(1, tagPool.length)) : [];
  shots.push({
    id,
    path: `C:\\Users\\Alex\\Pictures\\OmniHub\\${name}`,
    created,
    width,
    height,
    bytes: Math.round(width * height * r.range(0.45, 1.1)),
    appExe: exe || null,
    appTitle: title || null,
    tags,
    note: r.chance(0.2) ? r.pick(NOTES) : '',
    favorite: r.chance(0.14),
    exists: true,
  });
}
shots.sort((a, b) => b.created - a.created);

export function shotsList(f: ShotFilter): Screenshot[] {
  const q = (f.query ?? '').trim().toLowerCase();
  let out = shots.filter((s) => {
    if (f.favorites && !s.favorite) return false;
    if (f.tag && !s.tags.includes(f.tag)) return false;
    if (f.app && s.appExe !== f.app) return false;
    if (q && !`${s.appTitle ?? ''} ${s.appExe ?? ''} ${s.note} ${s.tags.join(' ')} ${s.path}`.toLowerCase().includes(q)) return false;
    return true;
  });
  if (f.limit) out = out.slice(0, f.limit);
  return out.map((s) => ({ ...s, tags: [...s.tags] }));
}

export function appScreenshots(id: string): Screenshot[] {
  const exe = exeFor(id);
  return exe ? shots.filter((s) => s.appExe?.toLowerCase() === exe.toLowerCase()) : [];
}

function find(id: string): Screenshot {
  const s = shots.find((x) => x.id === id);
  if (!s) throw new Error('screenshot not found');
  return s;
}

export async function shotThumb(id: string): Promise<string> {
  await sleep(20 + Math.random() * 180);
  const s = find(id);
  return scene(id, kinds.get(id) ?? 'desktop', 320, Math.round((320 * s.height) / s.width));
}

export function shotImage(id: string): string {
  const s = find(id);
  return scene(id, kinds.get(id) ?? 'desktop', 1600, Math.round((1600 * s.height) / s.width), true);
}

function addShot(kind: SceneKind, width: number, height: number, exe: string | null, title: string | null): Screenshot {
  const created = Math.floor(Date.now() / 1000);
  const d = new Date();
  const p2 = (n: number) => String(n).padStart(2, '0');
  const id = `shot-new-${Date.now().toString(36)}`;
  kinds.set(id, kind);
  const shot: Screenshot = {
    id,
    path: `C:\\Users\\Alex\\Pictures\\OmniHub\\${d.getFullYear()}-${p2(d.getMonth() + 1)}-${p2(d.getDate())}_${p2(d.getHours())}-${p2(d.getMinutes())}-${p2(d.getSeconds())}.png`,
    created,
    width,
    height,
    bytes: Math.round(width * height * 0.7),
    appExe: exe,
    appTitle: title,
    tags: [],
    note: '',
    favorite: false,
    exists: true,
  };
  shots.unshift(shot);
  emit('screenshots:new', shot);
  return shot;
}

export async function shotCapture(kind: CaptureKind, delaySeconds: number): Promise<Screenshot> {
  await sleep(delaySeconds * 1000 + 250);
  if (kind === 'window') return addShot('code', 1920, 1080, 'Code.exe', 'App.tsx — omnihub — Visual Studio Code');
  if (kind === 'allScreens') return addShot('desktop', 6400, 2160, null, null);
  return addShot('desktop', 3840, 2160, null, null);
}

let pending: PendingRegion | null = null;

export function regionBegin(): void {
  pending = { id: `region-${Date.now().toString(36)}`, x: 0, y: 0, width: 2560, height: 1440, scale: 1, image: scene('frozen-desktop', 'desktop', 2560, 1440) };
  emit('region:pending', pending);
}

export function regionPending(): PendingRegion | null {
  return pending;
}

export function regionCommit(id: string, rect: Rect): Screenshot {
  if (!pending || pending.id !== id) throw new Error('no region capture in progress');
  pending = null;
  return addShot('desktop', Math.max(1, Math.round(rect.width)), Math.max(1, Math.round(rect.height)), null, null);
}

export function regionCancel(): void {
  pending = null;
}

export function shotUpdate(id: string, patch: { tags: string[] | null; note: string | null; favorite: boolean | null }): Screenshot {
  const s = find(id);
  if (patch.tags) s.tags = [...new Set(patch.tags.map((t) => t.trim()).filter(Boolean))];
  if (patch.note != null) s.note = patch.note;
  if (patch.favorite != null) s.favorite = patch.favorite;
  return { ...s, tags: [...s.tags] };
}

export function shotDelete(id: string): void {
  const i = shots.findIndex((s) => s.id === id);
  if (i < 0) throw new Error('screenshot not found');
  shots.splice(i, 1);
  emit('screenshots:deleted', { id });
}

export function shotsSync(): number {
  emit('screenshots:synced', { added: 0 });
  return 0;
}
