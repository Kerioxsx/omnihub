// The game library and emulator hub for the mock backend.

import type { EmuState, RomOpts, GameDetail, GameRecap, LibraryGame, SessionRec, ShelfEntry, ShelfPatch, SteamAccount, SteamAchievement, SteamFriend, WishItem } from '@shared/types';
import { emit } from './bus';

const STEAM = 'https://shared.steamstatic.com/store_item_assets/steam/apps';
const art = (app: number) => ({ cover: `${STEAM}/${app}/library_600x900_2x.jpg`, hero: `${STEAM}/${app}/library_hero.jpg`, logo: `${STEAM}/${app}/logo.png`, header: `${STEAM}/${app}/header.jpg` });
const now = Date.now();
const H = 3600_000;

const GAMES: [number, string, number, number][] = [
  [367520, 'Hollow Knight', 4120, 2],
  [1145360, 'Hades', 2930, 30],
  [413150, 'Stardew Valley', 9800, 5],
  [1091500, 'Cyberpunk 2077', 3100, 90],
  [730, 'Counter-Strike 2', 15200, 1],
  [1245620, 'ELDEN RING', 7400, 12],
  [504230, 'Celeste', 860, 200],
  [105600, 'Terraria', 5200, 60],
  [1086940, "Baldur's Gate 3", 6100, 20],
  [632360, 'Risk of Rain 2', 1900, 45],
  [892970, 'Valheim', 2400, 150],
  [1794680, 'Vampire Survivors', 1500, 8],
];

const entries: Record<string, ShelfEntry> = {};
const blank = (): ShelfEntry => ({ status: '', favorite: false, notes: '', cover: null, hero: null, logo: null, saveDirs: [], autoBackup: false, dailyLimit: 0 });
const entry = (key: string) => (entries[key] ??= blank());
entry('steam:367520').status = 'playing';
entry('steam:367520').favorite = true;
entry('steam:1145360').status = 'finished';
entry('steam:1091500').status = 'backlog';
entry('steam:1086940').status = 'backlog';

export const library = {
  games(): LibraryGame[] {
    const steam: LibraryGame[] = GAMES.map(([app, name, minutes, daysAgo]) => {
      const e = entry(`steam:${app}`);
      return { key: `steam:${app}`, name, source: 'steam', app, process: '', art: art(app), status: e.status, favorite: e.favorite, minutes, lastPlayed: now - daysAgo * 24 * H, profileId: null, canLaunch: true };
    });
    const others: LibraryGame[] = [
      { key: 'epic:fortnite', name: 'Fortnite', source: 'epic', app: null, process: 'FortniteClient-Win64-Shipping.exe', art: { cover: null, hero: null, logo: null, header: null }, status: entry('epic:fortnite').status, favorite: false, minutes: 22_000, lastPlayed: now - 3 * H, profileId: null, canLaunch: true },
      { key: 'minecraft:java', name: 'Minecraft', source: 'minecraft', app: null, process: 'javaw.exe', art: { cover: null, hero: null, logo: null, header: null }, status: entry('minecraft:java').status, favorite: entry('minecraft:java').favorite, minutes: 12_400, lastPlayed: now - 26 * H, profileId: null, canLaunch: true },
    ];
    return [...steam, ...others];
  },
  detail(key: string): GameDetail {
    const app = Number(key.split(':')[1]) || 0;
    const sessions: SessionRec[] = [0, 1, 3].map((d) => ({ key, name: 'Game', start: now - d * 24 * H - 2 * H, end: now - d * 24 * H - 20 * 60_000 }));
    return {
      key,
      players: app ? 3000 + ((app * 7919) % 90_000) : null,
      screenshots: [],
      soundtrack: app === 367520 ? ['Enter Hallownest', 'Dirtmouth', 'Crossroads', 'Greenpath', 'City of Tears'].map((n, i) => ({ src: '', name: `${String(i + 1).padStart(2, '0')} ${n}`, at: 0 })) : [],
      sessions,
      entry: { ...entry(key) },
      suggestedDirs: app === 367520 ? ['C:\\Users\\Player\\AppData\\LocalLow\\Team Cherry\\Hollow Knight'] : [],
      backups: entry(key).saveDirs.length ? [{ id: '20261101-210500-auto', at: now - 26 * H, bytes: 1_840_000, files: 12 }] : [],
      minutesToday: 42,
      installDir: null,
    };
  },
  achievements(): SteamAchievement[] {
    const names = ['Falsehood', 'Protected', 'Test of Resolve', 'Release', 'Witness', 'Connection', 'Speedrun 1', 'Steel Soul'];
    return names.map((n, i) => ({ id: `a${i}`, name: n, description: 'Do the thing in Hallownest.', icon: '', iconGray: '', unlocked: i < 5, unlockedAt: i < 5 ? Math.floor(now / 1000) - i * 86400 : 0, percent: [71.2, 54.1, 38, 22.4, 14.9, 9.1, 3.2, 1.1][i], hidden: i === 7 }));
  },
  friends: (): SteamFriend[] => [
    { steamid: '1', name: 'Amy', avatar: '', state: 1, game: 'Celeste', gameId: 504230 },
    { steamid: '2', name: 'Kai', avatar: '', state: 1, game: null, gameId: null },
    { steamid: '3', name: 'Zed', avatar: '', state: 0, game: null, gameId: null },
  ],
  wishlist: (): WishItem[] => [
    { app: 1145350, name: 'Hades II', discount: 25, price: '$22.49', original: '$29.99' },
    { app: 1030300, name: 'Hollow Knight: Silksong', discount: 0, price: '$19.99', original: null },
  ],
  account: (): SteamAccount => ({ user: { steamid: '76561198000000002', accountId: 39734274, name: 'Player' }, hasKey: true, hasSgdb: false }),
  setKey: () => undefined,
  patch(key: string, p: ShelfPatch): ShelfEntry {
    Object.assign(entry(key), Object.fromEntries(Object.entries(p).filter(([, v]) => v !== undefined)));
    emit('library:changed', {});
    return { ...entry(key) };
  },
  setArt: (key: string) => ({ ...entry(key) }),
  findArt: (key: string) => ({ ...entry(key) }),
  play: (key: string) => {
    const g = library.games().find((x) => x.key === key);
    // Show what a recap looks like a moment after "playing".
    setTimeout(() => {
      const recap: GameRecap = { key, name: g?.name ?? 'Game', start: Date.now() - 95 * 60_000, end: Date.now(), minutes: 95, avgFps: 143.6, lowFps: 88.2, clips: [{ src: '', name: 'Clutch 1v3', at: Date.now() - 20 * 60_000 }], screenshots: [], backup: entry(key).autoBackup ? '20261102-221000-auto' : null, art: g?.art ?? { cover: null, hero: null, logo: null, header: null } };
      emit('games:recap', recap);
    }, 1500);
  },
  backup: () => ({ id: '20261102-120000-manual', at: Date.now(), bytes: 1_900_000, files: 12 }),
  restore: () => undefined,
  deleteBackup: () => undefined,
  sessions: (): SessionRec[] => GAMES.slice(0, 6).flatMap(([app, name], i) => [0, 2, 5, 9, 14, 21].slice(0, 6 - i).map((d) => ({ key: `steam:${app}`, name, start: now - d * 24 * H - (i + 2) * H, end: now - d * 24 * H - i * H }))),
};

const emu: EmuState = {
  emulators: [
    { id: 'mgba', name: 'mGBA', exe: 'C:\\Emulators\\mGBA\\mGBA.exe', platforms: ['gba', 'gb', 'gbc'], args: '"{rom}"', found: true },
    { id: 'dolphin', name: 'Dolphin', exe: 'C:\\Program Files\\Dolphin\\Dolphin.exe', platforms: ['gc', 'wii'], args: '-e "{rom}"', found: true },
  ],
  romDirs: ['C:\\Games\\ROMs'],
  chosen: {},
  romOpts: {},
  roms: [
    ['Pokemon - Emerald Version (USA, Europe)', 'gba', true],
    ['Pokemon - FireRed Version (USA, Europe) (Rev 1)', 'gba', true],
    ['Metroid Fusion (USA)', 'gba', false],
    ['Legend of Zelda, The - The Minish Cap (USA)', 'gba', false],
    ['Pokemon - Crystal Version (USA, Europe) (Rev 1)', 'gbc', false],
    ['Super Mario Sunshine (USA)', 'gc', false],
  ].map(([name, platform, save], i) => ({
    id: `rom${i}`,
    path: `C:\\Games\\ROMs\\${name}.${platform}`,
    name: name as string,
    platform: platform as string,
    cover: platform === 'gc' ? null : `https://raw.githubusercontent.com/libretro-thumbnails/${platform === 'gba' ? 'Nintendo_-_Game_Boy_Advance' : 'Nintendo_-_Game_Boy_Color'}/master/Named_Boxarts/${encodeURIComponent(name as string)}.png`,
    save: save ? `C:\\Games\\ROMs\\${name}.sav` : null,
  })),
};

export const emulators = {
  state: () => structuredClone(emu),
  refresh: () => structuredClone(emu),
  add: () => structuredClone(emu),
  remove: (id: string) => {
    emu.emulators = emu.emulators.filter((e) => e.id !== id);
    return structuredClone(emu);
  },
  setDirs: (dirs: string[]) => {
    emu.romDirs = dirs;
    return structuredClone(emu);
  },
  choose: (platform: string, emulator: string) => {
    if (emulator) emu.chosen[platform] = emulator;
    else delete emu.chosen[platform];
    return structuredClone(emu);
  },
  launch: () => undefined,
  setRomOpts: (rom: string, opts: RomOpts) => {
    const r = emu.roms.find((x) => x.id === rom);
    if (r) emu.romOpts[r.path] = opts;
    return structuredClone(emu);
  },
};
