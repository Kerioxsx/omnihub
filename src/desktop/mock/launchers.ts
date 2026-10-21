// Demo: modpacks Prism Launcher, CurseForge and the Modrinth App installed.

import type { McLauncherPack, McLauncherPacks } from '@shared/types';
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

export const launchersMock = {
  list: (): McLauncherPacks => ({ packs, places }),
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
