// Demo: modpacks Prism Launcher, CurseForge and the Modrinth App installed.

import type { McLauncherDetails, McLauncherPack, McLauncherPacks, McModInfo, McShared, McWorld } from '@shared/types';
import { packIcon } from './mcCover';

const H = 3600;
const at = (hoursAgo: number) => Math.floor(Date.now() / 1000) - hoursAgo * H;
const PRISM = 'C:\\Users\\Player\\AppData\\Roaming\\PrismLauncher\\instances';
const CF = 'C:\\Users\\Player\\curseforge\\minecraft\\Instances';
const MR = 'C:\\Users\\Player\\AppData\\Roaming\\ModrinthApp\\profiles';

function pack(p: Omit<McLauncherPack, 'gameDir' | 'hasIcon'> & { hasIcon?: boolean }): McLauncherPack {
  return { ...p, gameDir: p.launcher === 'prism' ? `${p.path}\\.minecraft` : p.path, hasIcon: p.hasIcon ?? true };
}

let packs: McLauncherPack[] = [
  pack({ id: 'lp01atm9', launcher: 'prism', launcherName: 'Prism Launcher', name: 'All the Mods 9', path: `${PRISM}\\All the Mods 9`, mc: '1.20.1', loader: 'forge', loaderVersion: '47.3.0', mods: 412, disabled: 3, pack: 'All the Mods 9 0.3.2', lastPlayed: at(2) }),
  pack({ id: 'lp02cobb', launcher: 'modrinth', launcherName: 'Modrinth App', name: 'Cobblemon Official Modpack', path: `${MR}\\Cobblemon Official Modpack`, mc: '1.21.1', loader: 'fabric', loaderVersion: '0.16.9', mods: 64, disabled: 0, pack: 'Cobblemon Official Modpack 1.6.1', lastPlayed: at(5) }),
  pack({ id: 'lp03bmc4', launcher: 'curseforge', launcherName: 'CurseForge', name: 'Better MC [FORGE] - BMC4', path: `${CF}\\Better MC [FORGE] - BMC4`, mc: '1.20.1', loader: 'forge', loaderVersion: '47.3.0', mods: 270, disabled: 0, pack: 'Better MC [FORGE] - BMC4 v32', lastPlayed: at(26) }),
  pack({ id: 'lp04fo64', launcher: 'prism', launcherName: 'Prism Launcher', name: 'Fabulously Optimized', path: `${PRISM}\\Fabulously Optimized`, mc: '1.21.4', loader: 'fabric', loaderVersion: '0.16.10', mods: 78, disabled: 2, pack: 'Fabulously Optimized 6.4.0', lastPlayed: at(75) }),
  pack({ id: 'lp05cae1', launcher: 'modrinth', launcherName: 'Modrinth App', name: 'Create: Arcane Engineering', path: `${MR}\\Create Arcane Engineering`, mc: '1.20.1', loader: 'forge', loaderVersion: '47.2.0', mods: 231, disabled: 0, pack: 'Create: Arcane Engineering 1.9.2', lastPlayed: at(140) }),
  pack({ id: 'lp06skyb', launcher: 'prism', launcherName: 'Prism Launcher', name: 'Skyblock with friends', path: `${PRISM}\\Skyblock with friends`, mc: '1.21.1', loader: 'neoforge', loaderVersion: '21.1.77', mods: 165, disabled: 0, pack: null, lastPlayed: at(290), hasIcon: false }),
  pack({ id: 'lp07rlcr', launcher: 'curseforge', launcherName: 'CurseForge', name: 'RLCraft', path: `${CF}\\RLCraft`, mc: '1.12.2', loader: 'forge', loaderVersion: '14.23.5.2860', mods: 152, disabled: 0, pack: 'RLCraft 2.9.3', lastPlayed: at(960) }),
  pack({ id: 'lp08vani', launcher: 'modrinth', launcherName: 'Modrinth App', name: 'Plain 1.21.10', path: `${MR}\\Plain 1.21.10`, mc: '1.21.10', loader: '', loaderVersion: '', mods: 0, disabled: 0, pack: null, lastPlayed: null, hasIcon: false }),
];
let places: string[] = [];

const MODS: [string, string, McModInfo['side']][] = [
  ['Create', '6.0.4', ''], ['Just Enough Items', '15.20.0', ''], ['Jade', '11.12.3', 'client'], ['AppleSkin', '2.5.1', 'client'], ['Farmer’s Delight', '1.2.6', ''],
  ['Sodium', '0.6.13', 'client'], ['Iris Shaders', '1.8.8', 'client'], ['Waystones', '14.1.11', ''], ['Storage Drawers', '12.9.13', ''], ['FTB Quests', '2001.4.13', ''],
  ['Mekanism', '10.4.15', ''], ['Xaero’s Minimap', '25.2.0', 'client'], ['Sophisticated Backpacks', '3.23.6', ''], ['Balm', '7.3.29', ''], ['Architectury API', '9.2.14', ''],
];
const WORLDS: Omit<McWorld, 'lastPlayed'>[] = [
  { folder: 'New World', name: 'Base with friends', version: '', gameMode: 'survival', hardcore: false, difficulty: 'normal', cheats: false, size: 412_000_000, hasPlayer: true, hasIcon: true },
  { folder: 'Creative build', name: 'Creative build', version: '', gameMode: 'creative', hardcore: false, difficulty: 'peaceful', cheats: true, size: 96_000_000, hasPlayer: true, hasIcon: false },
  { folder: 'Hardcore 3', name: 'Hardcore #3', version: '', gameMode: 'survival', hardcore: true, difficulty: 'hard', cheats: false, size: 58_000_000, hasPlayer: true, hasIcon: true },
];

export const launchersMock = {
  list: (): McLauncherPacks => ({ packs, places }),
  details(id: string): McLauncherDetails {
    const p = packs.find((x) => x.id === id);
    if (!p) throw new Error('That modpack isn’t there any more.');
    const mods: McModInfo[] = MODS.slice(0, Math.min(MODS.length, p.mods)).map(([name, version, side], i) => ({ file: `${name.toLowerCase().replace(/\W+/g, '-')}-${version}.jar`, id: name.toLowerCase().replace(/\W+/g, ''), name, version, side, disabled: i < p.disabled, size: 180_000 + i * 97_000 }));
    const worlds = p.mods ? WORLDS.map((w, i) => ({ ...w, version: p.mc, lastPlayed: (p.lastPlayed ?? at(500)) - i * 9 * H })) : [];
    return { mods, worlds, size: 1_200_000_000 + p.mods * 2_400_000 };
  },
  worldIcon: (id: string, folder: string): string | null => (WORLDS.find((w) => w.folder === folder)?.hasIcon ? packIcon(`${id}${folder}`) : null),
  updates: (id: string): Record<string, string> => {
    const d = launchersMock.details(id);
    return Object.fromEntries(d.mods.filter((_, i) => i % 4 === 1).map((m) => [m.file, m.version.replace(/\d+$/, (n) => String(Number(n) + 2))]));
  },
  share(id: string): McShared {
    const p = packs.find((x) => x.id === id)!;
    const included = Math.max(1, Math.round(p.mods * 0.08));
    return { mods: p.mods, fromModrinth: p.mods - included, included, bytes: 38_000_000 + included * 1_600_000 };
  },
  get: (id: string): McLauncherPack | undefined => packs.find((p) => p.id === id),
  icon: (id: string): string | null => {
    const p = packs.find((x) => x.id === id);
    return p?.hasIcon ? packIcon(p.name) : null;
  },
  addPlace(path: string): McLauncherPacks {
    if (!/multimc|prism|curse|modrinth|instances|launcher/i.test(path)) throw new Error('There are no Prism Launcher, CurseForge or Modrinth App modpacks in that folder.');
    if (!places.includes(path)) {
      places = [...places, path];
      packs = [...packs, pack({ id: `lp${places.length}mmc`, launcher: 'prism', launcherName: 'MultiMC', name: 'Old survival world', path: `${path}\\instances\\Old survival world`, mc: '1.16.5', loader: 'forge', loaderVersion: '36.2.39', mods: 48, disabled: 0, pack: null, lastPlayed: at(4000), hasIcon: false })];
    }
    return launchersMock.list();
  },
  removePlace(path: string): McLauncherPacks {
    places = places.filter((p) => p !== path);
    packs = packs.filter((p) => !p.path.startsWith(`${path}\\`));
    return launchersMock.list();
  },
};
