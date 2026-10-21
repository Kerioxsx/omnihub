// Mods without Windows: a pretend Steam library (Balatro, Lethal Company,
// GTA V Enhanced, Palworld, RimWorld…), Thunderstore lists, Workshop items,
// Claude making a mod (with streamed replies) and Minecraft packs to mix.

import type { BrowsePackage, ClaudeSettings, InstalledMod, MakerItem, MakerProject, MakerView, McEntry, McInstalled, McMix, McPackHit, McPackVersion, McSource, ModBrowse, ModGame, ModNote, ModSort, TsPackage, WorkshopItem } from '@shared/types';
import { emit } from './bus';
import { launchersMock } from './launchers';
import { blendCover, packIcon, paintedCover } from './mcCover';
import { serversMock } from './servers';

const now = Date.now();
const DAY = 86_400_000;
const steamLib = 'D:\\SteamLibrary\\steamapps\\common';
const hdr = (id: number) => `https://shared.cloudflare.steamstatic.com/store_item_assets/steam/apps/${id}/header.jpg`;
const note = (tone: ModNote['tone'], text: string, link: string | null = null): ModNote => ({ tone, text, link });

function game(p: Partial<ModGame> & Pick<ModGame, 'key' | 'name'>): ModGame {
  return {
    source: 'steam',
    appId: null,
    dir: `${steamLib}\\${p.name}`,
    exe: `${p.name}.exe`,
    engine: 'unityMono',
    engineLabel: 'Unity',
    antiCheat: [],
    onlineOnly: false,
    method: 'bepInEx',
    methodLabel: 'BepInEx plugins',
    loaders: [],
    thunderstore: null,
    tsLoader: null,
    tsInstalls: false,
    dataDir: null,
    mods: 0,
    enabledMods: 0,
    image: p.appId ? hdr(p.appId) : null,
    notes: [],
    maker: { available: true, kind: 'bepInExPlugin', label: 'a BepInEx plugin (C#)', why: null },
    ...p,
  };
}

let games: ModGame[] = [
  game({ key: 'steam:2379780', name: 'Balatro', appId: 2379780, engine: 'love', engineLabel: 'LÖVE', method: 'lovely', methodLabel: 'Lovely mods', thunderstore: 'balatro', tsLoader: 'lovely', tsInstalls: true, dataDir: 'C:\\Users\\Player\\AppData\\Roaming\\Balatro', loaders: [{ kind: 'lovely', label: 'Lovely', installed: true, managed: true, auto: true, link: null }], notes: [note('info', 'Mods live in %AppData%\\Balatro\\Mods. Most need Steamodded; OmniHub installs it (and Lovely) with the first mod that does.')], maker: { available: true, kind: 'steamodded', label: 'a Steamodded mod (Lua)', why: null } }),
  game({ key: 'steam:1966720', name: 'Lethal Company', appId: 1966720, thunderstore: 'lethal-company', tsLoader: 'bepinex', tsInstalls: true, loaders: [{ kind: 'bepInEx', label: 'BepInEx 5', installed: true, managed: true, auto: true, link: null }] }),
  game({ key: 'steam:3240220', name: 'Grand Theft Auto V Enhanced', appId: 3240220, exe: 'GTA5_Enhanced.exe', engine: 'rage', engineLabel: 'RAGE', antiCheat: ['BattlEye'], method: 'gta', methodLabel: 'Script Hook V mods', loaders: [{ kind: 'scriptHookV', label: 'Script Hook V', installed: false, managed: false, auto: false, link: 'https://www.dev-c.com/gtav/scripthookv/' }, { kind: 'scriptHookVDotNet', label: 'Script Hook V .NET', installed: false, managed: false, auto: true, link: null }], notes: [note('warn', 'Mods work in Story Mode only, with BattlEye off: in the Rockstar Games Launcher, Settings → this game → uncheck BattlEye (or add -nobattleye to the game\'s Steam launch options). GTA Online won\'t start that way: switch every mod off here and BattlEye back on to play online.'), note('info', 'Most mods need Script Hook V by Alexander Blade. Its licence doesn\'t let others hand it out: download it from his page (pick the version for your game build), then add the zip here.', 'https://www.dev-c.com/gtav/scripthookv/')], maker: { available: true, kind: 'shvdnScript', label: 'a Script Hook V .NET script (C#)', why: null } }),
  game({ key: 'steam:3241660', name: 'R.E.P.O.', appId: 3241660, exe: 'REPO.exe', thunderstore: 'repo', tsLoader: 'bepinex', tsInstalls: true, loaders: [{ kind: 'bepInEx', label: 'BepInEx 5', installed: false, managed: false, auto: true, link: null }] }),
  game({ key: 'steam:1623730', name: 'Palworld', appId: 1623730, exe: 'Palworld.exe', engine: 'unreal', engineLabel: 'Unreal Engine', method: 'unreal', methodLabel: '.pak and UE4SS mods', loaders: [{ kind: 'ue4ss', label: 'UE4SS (for Lua mods)', installed: false, managed: false, auto: true, link: null }], notes: [note('info', '.pak mods go into ~mods; Lua mods need UE4SS, which OmniHub can install. Some Unreal games check their files and refuse mods.')], maker: { available: true, kind: 'ue4ssLua', label: 'a UE4SS Lua mod', why: null } }),
  game({ key: 'steam:294100', name: 'RimWorld', appId: 294100, exe: 'RimWorldWin64.exe', notes: [note('info', 'Mods for this game are BepInEx plugins (.dll). Get them from its mod pages (Nexus Mods, GitHub…) and add the file here; OmniHub installs BepInEx with the first one.')], loaders: [{ kind: 'bepInEx', label: 'BepInEx 5', installed: false, managed: false, auto: true, link: null }] }),
  game({ key: 'steam:413150', name: 'Stardew Valley', appId: 413150, exe: 'Stardew Valley.exe', engine: 'unknown', engineLabel: 'Unknown engine', method: 'files', methodLabel: 'Files in the game folder', notes: [note('info', "OmniHub doesn't know a mod loader for this game. Mods that are files for the game folder can still be added and removed cleanly; follow the mod's own instructions.")], maker: { available: true, kind: 'files', label: 'game files (config, data, text)', why: null } }),
  game({ key: 'steam:730', name: 'Counter-Strike 2', appId: 730, exe: 'cs2.exe', engine: 'source2', engineLabel: 'Source 2', onlineOnly: true, method: 'none', methodLabel: 'No mods', notes: [note('bad', "An online competitive game: changing its files breaks its anti-cheat rules and can get the account banned, so OmniHub doesn't offer mods for it.")], maker: { available: false, kind: null, label: '', why: "Claude doesn't make mods for online competitive games: they'd count as cheating." } }),
];

function tsMod(fullName: string, version: string, downloads: number, description: string, cats: string[], deps: string[] = [], rating = 0, updatedDays = 3): TsPackage {
  const [owner, ...rest] = fullName.split('-');
  const name = rest.join('-');
  return { fullName, name, owner, description, icon: '', version, downloadUrl: '', dependencies: deps, downloads, rating, categories: cats, updated: new Date(now - updatedDays * DAY).toISOString(), deprecated: false, nsfw: false, pinned: false, website: '', size: 0, url: `https://thunderstore.io/package/${owner}/${name}/` };
}

const LISTS: Record<string, TsPackage[]> = {
  balatro: [
    { ...tsMod('Steamodded-Steamodded', '26.1002.0', 412_000, 'A Balatro modding framework', ['Mods', 'Libraries'], ['Thunderstore-lovely-0.9.0'], 9, 18), pinned: true },
    tsMod('Thunderstore-lovely', '0.10.0', 380_000, 'Lovely is a runtime lua injector for LÖVE 2d', ['Tools'], [], 14, 27),
    tsMod('spectralpack-Cryptid', '0.5.17', 151_000, 'Hundreds of absurd new jokers, decks and consumables', ['Mods', 'Content Mods'], ['just_frostice482-Amulet-3.5.4', 'Steamodded-Steamodded-1.1814.0'], 6),
    tsMod('just_frostice482-Amulet', '3.5.4', 140_000, 'Huge numbers for runs that go past the score limit', ['Libraries'], ['Steamodded-Steamodded-1.0.0'], 4),
    tsMod('JamesTheJellyfish-Handy', '2.3.0', 98_000, 'Quality-of-life controls: hold to select, quick sell and more', ['Quality of Life'], ['Steamodded-Steamodded-1.0.0'], 5, 2),
    tsMod('Mysthaps-JokerDisplay', '1.8.4', 91_000, 'Shows what each joker will add, right under it', ['Quality of Life'], ['Steamodded-Steamodded-1.0.0'], 5, 9),
    tsMod('Betmma-BetmmaVouchers', '2.2.1', 41_000, 'Fifty new vouchers to buy in the shop', ['Content Mods'], ['Steamodded-Steamodded-1.0.0'], 2, 30),
    tsMod('Larswijn-CardSleeves', '1.6.2', 38_000, 'Sleeves that change how each deck plays', ['Content Mods'], ['Steamodded-Steamodded-1.0.0'], 3, 12),
    tsMod('itayfeder-FusionJokers', '1.3.0', 22_000, 'Fuse two jokers into a stronger one', ['Content Mods', 'New Mechanics'], ['Steamodded-Steamodded-1.0.0'], 1, 44),
    tsMod('SpaceKnight-Pokermon', '2.9.1', 64_000, 'Pocket-monster jokers that evolve as you play', ['Content Mods'], ['Steamodded-Steamodded-1.0.0'], 4, 1),
  ],
  'lethal-company': [
    { ...tsMod('BepInEx-BepInExPack', '5.4.2305', 98_000_000, 'BepInEx pack for Mono Unity games. Preconfigured and ready to use.', ['Libraries'], [], 900), pinned: true },
    tsMod('notnotnotswipez-MoreCompany', '1.11.0', 41_000_000, 'Lets more than four players join a crew', ['Mods', 'Misc'], ['BepInEx-BepInExPack-5.4.2100'], 300, 40),
    tsMod('anormaltwig-LateCompany', '1.0.18', 21_000_000, 'Join a game that already started', ['Mods'], ['BepInEx-BepInExPack-5.4.2100'], 180, 60),
    tsMod('x753-More_Suits', '1.5.2', 33_000_000, 'Adds more suits to pick from', ['Suits', 'Cosmetics'], ['BepInEx-BepInExPack-5.4.2100'], 210, 20),
    tsMod('Evaisa-LethalLib', '1.1.1', 39_000_000, 'A library for adding new items, enemies and moons', ['Libraries'], ['BepInEx-BepInExPack-5.4.2100'], 200, 15),
    tsMod('Sligili-HDLethalCompany', '1.6.0', 12_000_000, 'Sharper picture and better performance options', ['Mods', 'Client-side'], ['BepInEx-BepInExPack-5.4.2100'], 120, 90),
    tsMod('ShaosilGaming-GeneralImprovements', '1.4.10', 18_000_000, 'Many small fixes and quality-of-life changes', ['Mods', 'Client-side'], ['BepInEx-BepInExPack-5.4.2100'], 160, 6),
    tsMod('TeamBMX-ReservedItemSlotCore', '2.0.38', 9_000_000, 'Slots kept for the flashlight and walkie', ['Mods'], ['BepInEx-BepInExPack-5.4.2100'], 90, 8),
  ],
};
LISTS.repo = [
  { ...tsMod('BepInEx-BepInExPack', '5.4.2305', 98_000_000, 'BepInEx pack for Mono Unity games. Preconfigured and ready to use.', ['Libraries'], [], 900), pinned: true },
  tsMod('zelofi-MorePlayers', '1.2.0', 2_400_000, 'Up to twenty players in one lobby', ['Mods'], ['BepInEx-BepInExPack-5.4.2100'], 40, 3),
  tsMod('Magic_Wesley-Wesleys_Enemies', '3.1.4', 1_900_000, 'New monsters roam the levels', ['Mods', 'Monsters'], ['BepInEx-BepInExPack-5.4.2100'], 55, 2),
  tsMod('ModdingFamily-MoreUpgrades', '1.5.6', 1_600_000, 'More upgrades in the shop', ['Mods'], ['BepInEx-BepInExPack-5.4.2100'], 30, 5),
];

const LOADERS = new Set(['Thunderstore-lovely', 'BepInEx-BepInExPack']);

function installed(p: Partial<InstalledMod> & Pick<InstalledMod, 'id' | 'game' | 'name'>): InstalledMod {
  return { version: '1.0.0', author: '', description: '', icon: '', source: { type: 'file', name: `${p.name}.zip` }, enabled: true, loader: null, dependency: false, installedAt: now - 2 * DAY, gameDir: '', dataDir: null, files: [], created: [], moved: [], update: null, ...p };
}

let mods: InstalledMod[] = [
  installed({ id: 'b0', game: 'steam:2379780', name: 'lovely', version: '0.10.0', author: 'Thunderstore', description: 'Lovely is a runtime lua injector for LÖVE 2d', loader: 'lovely', source: { type: 'thunderstore', fullName: 'Thunderstore-lovely', url: '' }, installedAt: now - 9 * DAY }),
  installed({ id: 'b1', game: 'steam:2379780', name: 'Steamodded', version: '26.0911.0', author: 'Steamodded', description: 'A Balatro modding framework', dependency: true, source: { type: 'thunderstore', fullName: 'Steamodded-Steamodded', url: '' }, installedAt: now - 9 * DAY, update: '26.1002.0' }),
  installed({ id: 'b2', game: 'steam:2379780', name: 'JokerDisplay', version: '1.8.4', author: 'Mysthaps', description: 'Shows what each joker will add, right under it', source: { type: 'thunderstore', fullName: 'Mysthaps-JokerDisplay', url: '' }, installedAt: now - 8 * DAY }),
  installed({ id: 'b3', game: 'steam:2379780', name: 'Lucky Joker', version: '2', author: 'Claude', description: 'A joker that gives +4 mult, and x2 on lucky cards', source: { type: 'claude', project: 'p1' }, installedAt: now - 3600_000 }),
  installed({ id: 'l0', game: 'steam:1966720', name: 'BepInExPack', version: '5.4.2305', author: 'BepInEx', loader: 'bepInEx', source: { type: 'thunderstore', fullName: 'BepInEx-BepInExPack', url: '' }, installedAt: now - 20 * DAY }),
  installed({ id: 'l1', game: 'steam:1966720', name: 'MoreCompany', version: '1.11.0', author: 'notnotnotswipez', description: 'Lets more than four players join a crew', source: { type: 'thunderstore', fullName: 'notnotnotswipez-MoreCompany', url: '' }, installedAt: now - 20 * DAY }),
  installed({ id: 'l2', game: 'steam:1966720', name: 'LateCompany', version: '1.0.18', author: 'anormaltwig', description: 'Join a game that already started', source: { type: 'thunderstore', fullName: 'anormaltwig-LateCompany', url: '' }, installedAt: now - 19 * DAY, enabled: false }),
  installed({ id: 'l3', game: 'steam:1966720', name: 'More Suits', version: '1.5.2', author: 'x753', description: 'Adds more suits to pick from', source: { type: 'thunderstore', fullName: 'x753-More_Suits', url: '' }, installedAt: now - 19 * DAY }),
];

function recount(): void {
  games = games.map((g) => {
    const mine = mods.filter((m) => m.game === g.key && !m.loader);
    return { ...g, mods: mine.length, enabledMods: mine.filter((m) => m.enabled).length };
  });
  games.sort((a, b) => b.mods - a.mods || a.name.localeCompare(b.name));
}
recount();

function changed(gameKey: string): void {
  recount();
  emit('mods:changed', { game: gameKey });
}

function progress(gameKey: string, steps: string[], done: () => void): Promise<void> {
  return new Promise((resolve) => {
    steps.forEach((text, i) => setTimeout(() => emit('mods:progress', { game: gameKey, text, done: i, total: steps.length }), i * 350));
    setTimeout(() => {
      emit('mods:progress', { game: gameKey, text: '', done: steps.length, total: steps.length });
      done();
      changed(gameKey);
      resolve();
    }, steps.length * 350 + 100);
  });
}

const loaded = new Set<string>();

// ---------- Claude ----------

let claude: ClaudeSettings = { apiSecret: '', model: 'claude-opus-5-5', effort: 'medium', web: true, hasKey: new URLSearchParams(window.location.search).get('claude') !== 'nokey' };

const projects: Record<string, MakerView> = {
  p1: {
    project: { id: 'p1', game: 'steam:2379780', gameName: 'Balatro', name: 'Lucky Joker', kind: 'steamodded', created: now - 2 * 3600_000, updated: now - 3600_000, status: 'idle', ready: true, version: 2, summary: 'Adds the Lucky Joker: +4 mult on every hand, and x2 mult when a Lucky card scores. Common, $5, with its own art.', howToUse: 'It shows up in the shop like any joker. Hover it to read what it does.', installed: 'b3', installedVersion: 2, usage: { input: 182_000, output: 21_400, cacheRead: 610_000, cacheWrite: 41_000, webSearches: 3 }, cost: 1.68, model: 'claude-opus-5-5', web: true, approval: null, error: null },
    items: [
      { kind: 'user', text: 'Make a joker that gives +4 mult, and doubles it when a lucky card scores', at: now - 2 * 3600_000 },
      { kind: 'note', text: 'Checking how Balatro scores Lucky cards before writing the joker.', at: now - 2 * 3600_000 + 4000 },
      { kind: 'tool', text: 'Looked in game/Balatro.exe!/', at: now - 2 * 3600_000 + 6000 },
      { kind: 'tool', text: 'Searched game/Balatro.exe!/ for “m_lucky”', at: now - 2 * 3600_000 + 9000 },
      { kind: 'tool', text: 'Read game/Balatro.exe!/card.lua', at: now - 2 * 3600_000 + 12000 },
      { kind: 'web', text: 'Searched the web: Steamodded SMODS.Joker calculate context individual', at: now - 2 * 3600_000 + 15000 },
      { kind: 'web', text: 'Read https://github.com/Steamodded/smods/wiki/SMODS.Joker', at: now - 2 * 3600_000 + 19000 },
      { kind: 'tool', text: 'Wrote mod/LuckyJoker.json (14 lines)', at: now - 2 * 3600_000 + 30000 },
      { kind: 'tool', text: 'Wrote mod/main.lua (46 lines)', at: now - 2 * 3600_000 + 34000 },
      { kind: 'tool', text: 'Wrote mod/assets/1x/lucky.png.txt (3 lines)', at: now - 2 * 3600_000 + 36000 },
      { kind: 'tool', text: 'Finished the mod', at: now - 2 * 3600_000 + 40000 },
      { kind: 'text', text: "Done! **Lucky Joker** gives **+4 mult** on every hand, and when a *Lucky* card scores it gives **x2 mult** on top.\n\nPress **Install**, then start a run: it appears in the shop like any common joker ($5).", at: now - 2 * 3600_000 + 42000 },
      { kind: 'notice', text: 'Installed version 1. Start the game to try it; tell Claude here what happened.', tone: 'good', at: now - 2 * 3600_000 + 60000 },
      { kind: 'user', text: 'It works! Can it say "Lucky!" when it doubles?', at: now - 3600_000 - 30000 },
      { kind: 'tool', text: 'Changed mod/main.lua', at: now - 3600_000 - 20000 },
      { kind: 'tool', text: 'Finished the mod', at: now - 3600_000 - 15000 },
      { kind: 'text', text: 'Now it pops up **"Lucky!"** in green over the joker each time the x2 kicks in. Press **Update** to install version 2.', at: now - 3600_000 - 14000 },
      { kind: 'notice', text: 'Installed version 2. Start the game to try it; tell Claude here what happened.', tone: 'good', at: now - 3600_000 },
    ],
    files: [
      { path: 'LuckyJoker.json', size: 412 },
      { path: 'main.lua', size: 1880 },
      { path: 'localization/en-us.lua', size: 390 },
    ],
  },
  p2: {
    project: { id: 'p2', game: 'steam:3240220', gameName: 'Grand Theft Auto V Enhanced', name: 'Slow motion on a key', kind: 'shvdnScript', created: now - DAY, updated: now - DAY + 600_000, status: 'idle', ready: true, version: 1, summary: 'Press F7 to slow time down to 30% (and again for normal speed), with an on-screen note.', howToUse: 'In Story Mode, press F7.', installed: null, installedVersion: 0, usage: { input: 64_000, output: 9_100, cacheRead: 120_000, cacheWrite: 30_000, webSearches: 2 }, cost: 0.62, model: 'claude-opus-5-5', web: true, approval: null, error: null },
    items: [
      { kind: 'user', text: 'Slow motion when I press F7', at: now - DAY },
      { kind: 'web', text: 'Searched the web: SHVDN v3 Game.TimeScale', at: now - DAY + 8000 },
      { kind: 'tool', text: 'Wrote mod/SlowMotion.cs (38 lines)', at: now - DAY + 20000 },
      { kind: 'tool', text: 'Finished the mod', at: now - DAY + 24000 },
      { kind: 'text', text: 'Ready: **F7** toggles slow motion (30% speed) in Story Mode. Press **Install**; OmniHub adds Script Hook V .NET. You still need **Script Hook V** itself from its author\'s page.', at: now - DAY + 26000 },
    ],
    files: [{ path: 'SlowMotion.cs', size: 1290 }],
  },
};

const FILES: Record<string, string> = {
  'p1/LuckyJoker.json': '{\n  "id": "lucky_joker",\n  "name": "Lucky Joker",\n  "author": ["Claude"],\n  "description": "A joker that loves Lucky cards.",\n  "prefix": "lucky",\n  "main_file": "main.lua",\n  "version": "2.0.0",\n  "dependencies": ["Steamodded (>=1.0.0~BETA)"]\n}\n',
  'p1/main.lua': "SMODS.Joker {\n  key = 'lucky_joker',\n  loc_txt = {\n    name = 'Lucky Joker',\n    text = { '{C:mult}+#1#{} Mult', '{X:mult,C:white}X#2#{} Mult when a', '{C:attention}Lucky{} card scores' },\n  },\n  config = { extra = { mult = 4, xmult = 2 } },\n  rarity = 1,\n  cost = 5,\n  loc_vars = function(self, info_queue, card)\n    return { vars = { card.ability.extra.mult, card.ability.extra.xmult } }\n  end,\n  calculate = function(self, card, context)\n    if context.joker_main then\n      return { mult = card.ability.extra.mult }\n    end\n    if context.individual and context.cardarea == G.play and SMODS.has_enhancement(context.other_card, 'm_lucky') then\n      return { xmult = card.ability.extra.xmult, message = 'Lucky!', colour = G.C.GREEN }\n    end\n  end,\n}\n",
  'p1/localization/en-us.lua': "return { misc = { dictionary = { lucky_pop = 'Lucky!' } } }\n",
  'p2/SlowMotion.cs': 'using System.Windows.Forms;\nusing GTA;\n\npublic class SlowMotion : Script\n{\n    bool slow;\n\n    public SlowMotion()\n    {\n        KeyDown += (s, e) =>\n        {\n            if (e.KeyCode != Keys.F7) return;\n            slow = !slow;\n            Game.TimeScale = slow ? 0.3f : 1f;\n            GTA.UI.Notification.Show(slow ? "Slow motion on" : "Slow motion off");\n        };\n    }\n}\n',
};

let nextProject = 3;

function streamReply(id: string, text: string): void {
  const view = projects[id];
  const p = view.project;
  p.status = 'working';
  emit('maker:changed', { project: id });
  const server = p.game.startsWith('mcserver:');
  const file = p.kind === 'datapack' ? 'mod/data/omnihub/function/load.mcfunction' : p.kind === 'paperPlugin' ? 'mod/src/dev/omnihub/demo/DemoPlugin.java' : 'mod/main.lua';
  const reply = server ? `Done: ${p.kind === 'paperPlugin' ? 'the plugin' : 'the data pack'} for “${text.slice(0, 60)}” is ready for Minecraft. Press **Install** to put version ${p.version + 1} on the server.` : `Looking at it now.\n\nDone: I updated the mod for “${text.slice(0, 60)}”. Press **Update** to install version ${p.version + 1}.`;
  const words = reply.split(/(\s+)/);
  setTimeout(() => emit('maker:delta', { project: id, kind: 'note', text: 'Reading the mod and the game files it touches.' }), 300);
  setTimeout(() => {
    view.items.push({ kind: 'note', text: server ? 'Checking this Minecraft version’s data pack format and commands.' : 'Reading the mod and the game files it touches.', at: Date.now() }, { kind: 'tool', text: server ? 'Read game/server.properties' : 'Read mod/main.lua', at: Date.now() });
    emit('maker:changed', { project: id });
  }, 900);
  words.forEach((w, i) => setTimeout(() => emit('maker:delta', { project: id, kind: 'text', text: w }), 1100 + i * 45));
  setTimeout(() => {
    view.items.push({ kind: 'tool', text: `Wrote ${file}`, at: Date.now() }, { kind: 'tool', text: 'Finished the mod', at: Date.now() }, { kind: 'text', text: reply, at: Date.now() });
    p.status = 'idle';
    p.ready = true;
    p.version += 1;
    p.updated = Date.now();
    p.cost += 0.21;
    p.usage.input += 24_000;
    p.usage.output += 2_100;
    emit('maker:changed', { project: id });
  }, 1300 + words.length * 45);
}

// ---------- Minecraft ----------

const mcHits: McPackHit[] = [
  { project: '1KVo5zza', slug: 'fabulously-optimized', title: 'Fabulously Optimized', description: 'Improved performance and graphics, still feels like vanilla', icon: null, downloads: 18_994_206, versions: ['1.20.1', '1.21.1', '1.21.4', '26.1'], loaders: ['fabric'] },
  { project: 'Jkb29YJU', slug: 'cobblemon-fabric', title: 'Cobblemon Official Modpack', description: 'The official pack for Cobblemon, with friends and the mods it plays best with', icon: null, downloads: 6_200_000, versions: ['1.21.1'], loaders: ['fabric'] },
  { project: 'NRjRiSSD', slug: 'additive', title: 'Additive', description: 'Vanilla, with more: new biomes, mobs and things to build', icon: null, downloads: 2_100_000, versions: ['1.21.1', '1.21.4'], loaders: ['fabric', 'quilt'] },
  { project: 'Q6b7C1Ma', slug: 'create-arcane', title: 'Create: Arcane Engineering', description: 'Create machines meet magic', icon: null, downloads: 940_000, versions: ['1.20.1'], loaders: ['forge'] },
  { project: 'kmqpPfAZ', slug: 'simply-optimized', title: 'Simply Optimized', description: 'Lighter and faster, nothing else', icon: null, downloads: 3_800_000, versions: ['1.20.1', '1.21.1'], loaders: ['fabric', 'quilt'] },
];

function mcFiles(names: string[]): McSource['files'] {
  return names.map((n, i) => ({ path: `mods/${n}.jar`, sha1: `${i}`.padStart(40, 'a'), sha512: '', size: 100_000 + i * 7_000, urls: [], client: true, local: null }));
}

let mcSources: McSource[] = [
  { id: 's1', name: 'Fabulously Optimized', kind: { type: 'modrinth', project: '1KVo5zza', version: 'N276l2ON' }, icon: packIcon('Fabulously Optimized'), mc: '1.21.1', loader: 'fabric', loaderVersion: '0.16.9', files: mcFiles(['sodium', 'iris', 'lithium', 'entityculling', 'modmenu', 'zoomify', 'continuity', 'cull-leaves', 'ferritecore', 'immediatelyfast']), overrides: 62 },
  { id: 's2', name: 'Cobblemon Official Modpack', kind: { type: 'modrinth', project: 'Jkb29YJU', version: 'aB12cD34' }, icon: packIcon('Cobblemon Official Modpack'), mc: '1.21.1', loader: 'fabric', loaderVersion: '0.16.5', files: mcFiles(['cobblemon', 'fabric-api', 'sodium', 'jei', 'jade', 'journeymap', 'waystones', 'balm', 'yigd', 'storagedrawers']), overrides: 31 },
];
const mcInstalled: McInstalled[] = [];
let mcMix: McMix | null = null;
const DEMO_MIX: McMix = { id: 'mixdemo', name: 'Create + Cobblemon', mc: '26.1.2', loader: 'neoforge', loaderVersion: '26.1.2.122', sources: ['Create: Arcane Engineering', 'Cobblemon Official Modpack'], entries: [], clashes: [], problems: [], installable: false, sourceIds: [], cover: 'blend' };
const mcCovers = new Map<string, string>();
const mixIcons = () => (mcMix?.sourceIds ?? []).map((id) => mcSources.find((s) => s.id === id)).map((s) => s?.icon ?? packIcon(s?.name ?? 'Pack'));

function entry(title: string, status: McEntry['status'], from: string[], version: string, note: string | null = null, folder = 'mods'): McEntry {
  return { project: title.toLowerCase(), title, icon: null, folder, version, file: status === 'missing' || status === 'conflict' ? null : { path: `${folder}/${title}.jar`, sha1: '', sha512: '', size: 0, urls: [], client: true, local: null }, from, status, note };
}

export const modsMock = {
  games: () => games,
  list: (key: string) => mods.filter((m) => m.game === key).sort((a, b) => Number(!!b.loader) - Number(!!a.loader) || b.installedAt - a.installedAt),
  browse(key: string, query: string, category: string | null, sort: ModSort, offset: number, limit: number): ModBrowse {
    const g = games.find((x) => x.key === key);
    if (!g?.thunderstore) throw 'Thunderstore has no mods for this game.';
    if (!loaded.has(g.thunderstore)) {
      loaded.add(g.thunderstore);
      [0, 1, 2].forEach((i) => setTimeout(() => emit('mods:listing', { community: g.thunderstore, done: i, total: 3 }), 250 * i));
      setTimeout(() => emit('mods:changed', { game: key }), 900);
      return { packages: [], total: 0, loading: true, progress: [0, 3], error: null, categories: [] };
    }
    const all = LISTS[g.thunderstore] ?? [];
    const words = query.toLowerCase().split(/\s+/).filter(Boolean);
    let hits = all.filter((p) => !category || p.categories.some((c) => c.toLowerCase() === category.toLowerCase())).filter((p) => words.every((w) => `${p.name} ${p.owner} ${p.description} ${p.categories.join(' ')}`.toLowerCase().includes(w)));
    hits = [...hits].sort((a, b) => (sort === 'downloads' ? Number(b.pinned) - Number(a.pinned) || b.downloads - a.downloads : sort === 'rating' ? b.rating - a.rating : sort === 'updated' ? b.updated.localeCompare(a.updated) : a.name.localeCompare(b.name)));
    const inst = new Map(mods.filter((m) => m.game === key && m.source.type === 'thunderstore').map((m) => [(m.source as { fullName: string }).fullName, m.version]));
    const packages: BrowsePackage[] = hits.slice(offset, offset + limit).map((p) => ({ ...p, installed: inst.get(p.fullName) ?? null, isLoader: LOADERS.has(p.fullName), managerLink: `ror2mm://v1/install/thunderstore.io/${p.owner}/${p.name}/${p.version}/` }));
    return { packages, total: hits.length, loading: false, progress: null, error: null, categories: [...new Set(all.flatMap((p) => p.categories))].sort() };
  },
  async installPackage(key: string, fullName: string): Promise<InstalledMod[]> {
    const g = games.find((x) => x.key === key)!;
    const p = (LISTS[g.thunderstore ?? ''] ?? []).find((x) => x.fullName === fullName);
    if (!p) throw 'Thunderstore no longer has it.';
    if (mods.some((m) => m.game === key && m.source.type === 'thunderstore' && m.source.fullName === fullName)) throw "It's already installed.";
    const deps = p.dependencies.map((d) => d.split('-').slice(0, 2).join('-')).filter((d) => !LOADERS.has(d) && !mods.some((m) => m.game === key && m.source.type === 'thunderstore' && m.source.fullName === d));
    const order = [...deps.map((d) => (LISTS[g.thunderstore ?? ''] ?? []).find((x) => x.fullName === d)!).filter(Boolean), p];
    const out: InstalledMod[] = [];
    const needLoader = g.loaders.some((l) => !l.installed && l.auto);
    const steps = [...(needLoader ? [`Installing ${g.loaders[0].label}…`] : []), ...order.map((x, i) => `Installing ${x.name.replace(/_/g, ' ')} (${i + 1} of ${order.length})…`)];
    await progress(key, steps, () => {
      if (needLoader) {
        g.loaders = g.loaders.map((l) => ({ ...l, installed: true, managed: true }));
        mods.push(installed({ id: `m${Date.now()}l`, game: key, name: g.loaders[0].label, loader: g.loaders[0].kind, version: '5.4.2305', source: { type: 'thunderstore', fullName: 'BepInEx-BepInExPack', url: '' }, installedAt: Date.now() }));
      }
      for (const x of order) {
        const m = installed({ id: `m${Date.now()}${out.length}`, game: key, name: x.name.replace(/_/g, ' '), version: x.version, author: x.owner, description: x.description, dependency: x !== p, source: { type: 'thunderstore', fullName: x.fullName, url: x.url }, installedAt: Date.now() });
        mods.push(m);
        out.push(m);
      }
    });
    return out;
  },
  async installFile(key: string, path: string): Promise<InstalledMod> {
    const name = path.split(/[\\/]/).pop()!.replace(/\.(zip|7z|rar)$/i, '').replace(/[_-]/g, ' ');
    const m = installed({ id: `f${Date.now()}`, game: key, name, version: '', source: { type: 'file', name: path.split(/[\\/]/).pop()! }, installedAt: Date.now() });
    await progress(key, ['Unpacking…'], () => mods.push(m));
    return m;
  },
  async installLoader(key: string, kind: string): Promise<InstalledMod> {
    const g = games.find((x) => x.key === key)!;
    const l = g.loaders.find((x) => x.kind === kind)!;
    const m = installed({ id: `x${Date.now()}`, game: key, name: l.label, loader: l.kind, version: 'latest', source: { type: 'github', repo: '' }, installedAt: Date.now() });
    await progress(key, [`Installing ${l.label}…`], () => {
      mods.push(m);
      g.loaders = g.loaders.map((x) => (x.kind === kind ? { ...x, installed: true, managed: true } : x));
    });
    return m;
  },
  setEnabled(id: string, on: boolean): void {
    const m = mods.find((x) => x.id === id);
    if (m) {
      m.enabled = on;
      changed(m.game);
    }
  },
  setAll(key: string, on: boolean): number {
    const list = mods.filter((m) => m.game === key && m.enabled !== on);
    list.forEach((m) => (m.enabled = on));
    changed(key);
    return list.length;
  },
  uninstall(id: string): void {
    const m = mods.find((x) => x.id === id);
    mods = mods.filter((x) => x.id !== id);
    if (m) {
      if (m.loader) games = games.map((g) => (g.key === m.game ? { ...g, loaders: g.loaders.map((l) => (l.kind === m.loader ? { ...l, installed: false, managed: false } : l)) } : g));
      Object.values(projects).forEach((v) => v.project.installed === id && ((v.project.installed = null), (v.project.installedVersion = 0)));
      changed(m.game);
    }
  },
  update(id: string): InstalledMod {
    const m = mods.find((x) => x.id === id)!;
    if (m.update) m.version = m.update;
    m.update = null;
    changed(m.game);
    return m;
  },
  workshop(key: string): WorkshopItem[] {
    if (key !== 'steam:294100') return [];
    const names = ['Harmony', 'HugsLib', 'Vanilla Expanded Framework', 'Dubs Bad Hygiene', 'RimHUD', 'Pick Up And Haul', 'Numbers', 'Allow Tool'];
    return names.map((title, i) => ({ id: `${2009463077 + i * 1137}`, title, image: '', size: (2 + i * 3) * 1_048_576, updated: Math.floor((now - i * 7 * DAY) / 1000), url: `https://steamcommunity.com/sharedfiles/filedetails/?id=${2009463077 + i * 1137}` }));
  },
  addFolder(name: string, dir: string): ModGame {
    const g = game({ key: `folder:${Date.now()}`, name: name || dir.split(/[\\/]/).pop() || 'Game', source: 'folder', dir, method: 'files', methodLabel: 'Files in the game folder', engine: 'unknown', engineLabel: 'Unknown engine', maker: { available: true, kind: 'files', label: 'game files (config, data, text)', why: null } });
    games.push(g);
    changed(g.key);
    return g;
  },
  removeFolder(key: string): void {
    games = games.filter((g) => g.key !== key);
    changed(key);
  },

  claudeSettings: () => ({ ...claude, apiSecret: '' }),
  claudeUpdate(patch: Partial<ClaudeSettings>): ClaudeSettings {
    claude = { ...claude, ...patch, apiSecret: '', hasKey: claude.hasKey };
    return claude;
  },
  claudeSetKey(key: string): void {
    if (key && !key.startsWith('sk-ant-')) throw "That doesn't look like an Anthropic API key (they start with sk-ant-).";
    claude = { ...claude, hasKey: !!key };
  },
  claudeTest: () => undefined,

  projects: (gameKey: string | null) => Object.values(projects).map((v) => v.project).filter((p) => !gameKey || p.game === gameKey).sort((a, b) => b.updated - a.updated),
  create(gameKey: string, text: string): MakerProject {
    if (!claude.hasKey) throw 'Add your Anthropic API key first (Claude settings).';
    const server = gameKey.startsWith('mcserver:');
    const g = server ? { name: serversMock.serverName(gameKey), maker: { available: true, why: null, kind: (gameKey.endsWith(':plugin') ? 'paperPlugin' : 'datapack') as MakerProject['kind'] } } : games.find((x) => x.key === gameKey)!;
    if (!g.maker.available) throw g.maker.why;
    const id = `p${nextProject++}`;
    const p: MakerProject = { id, game: gameKey, gameName: g.name, name: text.split(/[.!?\n]/)[0].slice(0, 48) || 'New mod', kind: g.maker.kind!, created: Date.now(), updated: Date.now(), status: 'working', ready: false, version: 0, summary: '', howToUse: '', installed: null, installedVersion: 0, usage: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, webSearches: 0 }, cost: 0, model: claude.model, web: claude.web, approval: null, error: null };
    const files = p.kind === 'datapack' ? [{ path: 'pack.mcmeta', size: 120 }, { path: 'data/omnihub/function/load.mcfunction', size: 340 }, { path: 'data/minecraft/tags/function/load.json', size: 60 }] : p.kind === 'paperPlugin' ? [{ path: 'plugin.yml', size: 210 }, { path: 'src/dev/omnihub/demo/DemoPlugin.java', size: 2400 }] : [{ path: 'main.lua', size: 900 }];
    projects[id] = { project: p, items: [{ kind: 'user', text, at: Date.now() }], files };
    for (const f of files) FILES[`${id}/${f.path}`] = f.path === 'pack.mcmeta' ? '{\n  "pack": { "description": "Made with Claude in OmniHub", "min_format": [121, 0], "max_format": [121, 0] }\n}\n' : '-- Written by Claude in the demo\n';
    streamReply(id, text);
    return p;
  },
  send(id: string, text: string): void {
    const v = projects[id];
    if (!v) throw 'No such mod project.';
    if (v.project.status === 'working') throw 'Claude is still working on the last message. Wait, or press Stop.';
    v.items.push({ kind: 'user', text, at: Date.now() } as MakerItem);
    streamReply(id, text);
  },
  stop(id: string): void {
    const v = projects[id];
    if (v) v.project.status = 'idle';
  },
  approve: () => undefined,
  view(id: string): MakerView {
    const v = projects[id];
    if (!v) throw 'No such mod project.';
    return v;
  },
  file: (id: string, path: string) => FILES[`${id}/${path}`] ?? '',
  remove(id: string): void {
    delete projects[id];
  },
  rename(id: string, name: string): MakerProject {
    projects[id].project.name = name;
    return projects[id].project;
  },
  async install(id: string): Promise<MakerProject> {
    const v = projects[id];
    const p = v.project;
    if (p.game.startsWith('mcserver:')) {
      p.installed = serversMock.addMade(p.game, p.name, p.kind === 'paperPlugin' ? 'paperPlugin' : 'datapack');
      p.installedVersion = p.version;
      v.items.push({ kind: 'notice', tone: 'good', text: `Installed version ${p.version} on the server. ${p.kind === 'paperPlugin' ? 'Restart the server to load it.' : 'It’s live now.'} Tell Claude here how it went.`, at: Date.now() });
      emit('maker:changed', { project: id });
      return p;
    }
    await progress(p.game, ['Installing…'], () => {
      const prev = mods.find((m) => m.id === p.installed);
      if (prev) prev.version = `${p.version}`;
      else {
        const m = installed({ id: `c${Date.now()}`, game: p.game, name: p.name, version: `${p.version}`, author: 'Claude', description: p.summary, source: { type: 'claude', project: p.id }, installedAt: Date.now() });
        mods.push(m);
        p.installed = m.id;
      }
      p.installedVersion = p.version;
      v.items.push({ kind: 'notice', tone: 'good', text: `Installed version ${p.version}. Start the game to try it; tell Claude here what happened.`, at: Date.now() });
    });
    return p;
  },

  mcMixes: (): McMix[] => [...(mcMix ? [mcMix] : []), DEMO_MIX].filter((m, i, all) => all.findIndex((x) => x.name === m.name) === i),
  mcStatus: () => ({ found: true, dir: 'C:\\Users\\Player\\AppData\\Roaming\\.minecraft', sources: mcSources, installed: mcInstalled }),
  mcSearch: (query: string) => ({ hits: mcHits.filter((h) => !query || `${h.title} ${h.description}`.toLowerCase().includes(query.toLowerCase())).map((h) => ({ ...h, icon: h.icon ?? packIcon(h.title) })), total: mcHits.length }),
  mcVersions: (project: string): McPackVersion[] => {
    const h = mcHits.find((x) => x.project === project)!;
    return h.versions.slice().reverse().map((mc, i) => ({ id: `${project}${i}`, name: `${h.title} ${6 - i}.0 for ${mc}`, number: `${6 - i}.0.0`, mc: [mc], loaders: h.loaders, date: new Date(now - i * 30 * DAY).toISOString() }));
  },
  mcAddModrinth(project: string, version: string): McSource {
    const h = mcHits.find((x) => x.project === project)!;
    const v = modsMock.mcVersions(project).find((x) => x.id === version)!;
    const s: McSource = { id: `s${Date.now()}`, name: h.title, kind: { type: 'modrinth', project, version }, icon: h.icon ?? packIcon(h.title), mc: v.mc[0], loader: h.loaders[0], loaderVersion: h.loaders[0] === 'forge' ? '47.3.0' : '0.16.9', files: mcFiles(['create', 'jei', 'jade', 'appleskin', 'farmersdelight']), overrides: 18 };
    mcSources.push(s);
    return s;
  },
  mcAddFile(path: string): McSource {
    const s: McSource = { id: `s${Date.now()}`, name: path.split(/[\\/]/).pop()!.replace(/\.mrpack$/, ''), kind: { type: 'file', path }, icon: packIcon(path), mc: '1.21.1', loader: 'fabric', loaderVersion: '0.16.9', files: mcFiles(['sodium', 'lithium']), overrides: 4 };
    mcSources.push(s);
    return s;
  },
  mcAddFolder: (path: string) => modsMock.mcAddFile(`${path}\\instance.mrpack`),
  mcLauncherPacks: () => launchersMock.list(),
  mcLauncherIcon: (id: string) => launchersMock.icon(id),
  mcAddLauncherPack(id: string): McSource {
    const p = launchersMock.get(id);
    if (!p) throw new Error('That modpack isn’t there any more.');
    if (!p.mods) throw new Error('There are no mods (.jar files) in that folder or its mods folder.');
    const names = ['create', 'jei', 'jade', 'appleskin', 'farmersdelight', 'sodium', 'waystones', 'storagedrawers', 'ftb-quests', 'mekanism'];
    const s: McSource = { id: `s${Date.now()}`, name: p.name, kind: { type: 'launcher', launcher: p.launcherName, path: p.path }, icon: launchersMock.icon(id), mc: p.mc, loader: p.loader, loaderVersion: p.loaderVersion, files: mcFiles(names.slice(0, Math.min(names.length, p.mods))), overrides: 40 };
    mcSources.push(s);
    return s;
  },
  mcAddLauncherPlace: (path: string) => launchersMock.addPlace(path),
  mcRemoveLauncherPlace: (path: string) => launchersMock.removePlace(path),
  mcRemoveSource(id: string): void {
    mcSources = mcSources.filter((s) => s.id !== id);
  },
  mcPlan(name: string, ids: string[], mc: string, loader: string): McMix {
    const src = mcSources.filter((s) => ids.includes(s.id));
    const n = src.map((s) => s.name);
    const [a, b] = [n[0] ?? 'Pack', n[1] ?? n[0] ?? 'Pack'];
    const entries: McEntry[] = [
      entry('Sodium', 'kept', [a, b], '0.6.13+mc1.21.1'),
      entry('Iris Shaders', 'kept', [a], '1.8.8+1.21.1'),
      entry('Lithium', 'kept', [a], '0.14.7+mc1.21.1'),
      entry('Cobblemon', 'kept', [b], '1.6.1+1.21.1'),
      entry('Fabric API', 'kept', [b], '0.116.6+1.21.1'),
      entry('Jade', 'updated', [b], '15.10.2+fabric', '15.9.0 → 15.10.2+fabric'),
      entry('JourneyMap', 'kept', [b], '1.21.1-6.0.0-beta.48'),
      entry('Waystones', 'kept', [b], '21.1.20+fabric'),
      entry('Balm', 'dependency', [], '21.0.46+fabric', 'Needed by Waystones.'),
      entry('Mod Menu', 'kept', [a], '11.0.3'),
      entry('Entity Culling', 'kept', [a], '1.8.2'),
      entry('Zoomify', 'kept', [a], '2.14.6+1.21.1'),
      entry('Continuity', 'kept', [a], '3.0.0+1.21.1'),
      entry('FerriteCore', 'kept', [a], '7.0.2-fabric'),
      entry('ImmediatelyFast', 'kept', [a], '1.6.5+1.21.1-fabric'),
      entry('Storage Drawers', 'updated', [b], '13.10.2', '13.9.4 → 13.10.2'),
      entry('YIGD', 'missing', [b], '', `No version for Minecraft ${mc} with ${loader}.`),
      entry('JEI', 'conflict', [b], '', "Doesn't run alongside Roughly Enough Items, which stays."),
      entry('Fresh Animations', 'kept', [a], '1.9.3', null, 'resourcepacks'),
    ];
    mcMix = { id: `mix${Date.now()}`, name: name || n.join(' + '), mc, loader, loaderVersion: '0.16.9', sources: n, entries, clashes: [`config/sodium-options.json: kept ${a}'s, not ${b}'s`, `options.txt: kept ${a}'s, not ${b}'s`], problems: [], installable: loader === 'fabric' || loader === 'quilt', sourceIds: src.map((s) => s.id), cover: null };
    mcCovers.delete(mcMix.id);
    return mcMix;
  },
  mcExport: () => undefined,
  async mcInstall(id: string): Promise<McInstalled> {
    const m = mcMix!;
    const files = m.entries.filter((e) => e.file).map((e) => e.title);
    await progress('minecraft', files.map((f) => `Downloading ${f}…`), () => undefined);
    const inst: McInstalled = { name: m.name, profile: `omnihub-${id}`, dir: `C:\\Users\\Player\\AppData\\Roaming\\.minecraft\\omnihub\\${m.name.replace(/\W+/g, '_')}`, versionId: `fabric-loader-${m.loaderVersion}-${m.mc}`, mc: m.mc, loader: m.loader, mods: files.length, at: Date.now() };
    mcInstalled.push(inst);
    return inst;
  },
  async mcCover(id: string): Promise<string> {
    const have = mcCovers.get(id);
    if (have) return have;
    return modsMock.mcRemakeCover(id);
  },
  async mcRemakeCover(id: string): Promise<string> {
    const c = await blendCover(mixIcons());
    mcCovers.set(id, c);
    if (mcMix) mcMix.cover = 'blend';
    return c;
  },
  async mcAiCover(id: string, connected: boolean): Promise<string> {
    if (!connected) throw 'Connect your Pollinations account first.';
    await new Promise((r) => setTimeout(r, 1400));
    const c = await paintedCover(mixIcons());
    mcCovers.set(id, c);
    if (mcMix) mcMix.cover = 'ai';
    return c;
  },
  mcUninstall(profile: string): void {
    const i = mcInstalled.findIndex((x) => x.profile === profile);
    if (i >= 0) mcInstalled.splice(i, 1);
  },
};
