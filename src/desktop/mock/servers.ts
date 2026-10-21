// Minecraft servers without the PC: a Paper server with friends on it
// (walking about, chatting, their inventories), a NeoForge server made from
// a mix, the console, a drawn world map, plugins, backups and the network.

import type { SrvAction, SrvBackup, SrvContent, SrvFlavor, SrvHit, SrvKind, SrvLine, SrvMap, SrvNet, SrvNew, SrvOnline, SrvPatch, SrvPlayer, SrvPlayers, SrvView, SrvWaiting } from '@shared/types';
import { emit } from './bus';
import { launchersMock } from './launchers';
import { packIcon } from './mcCover';

const now = () => Math.floor(Date.now() / 1000);
const T0 = now();

function icon(hue: number): string {
  const c = document.createElement('canvas');
  c.width = c.height = 16;
  const g = c.getContext('2d')!;
  for (let y = 0; y < 16; y++)
    for (let x = 0; x < 16; x++) {
      const top = y < 5;
      const n = ((x * 7 + y * 13) % 5) * 3;
      g.fillStyle = top ? `hsl(${hue} 55% ${38 + n}%)` : `hsl(28 35% ${30 + n}%)`;
      g.fillRect(x, y, 1, 1);
    }
  return c.toDataURL();
}

function base(p: Partial<SrvView> & Pick<SrvView, 'id' | 'name' | 'flavor' | 'mc'>): SrvView {
  return {
    loader: '',
    dir: `C:\\Users\\Player\\AppData\\Local\\OmniHub\\data\\mcserver\\servers\\${p.name.toLowerCase().replace(/\W+/g, '_')}`,
    launch: { type: 'jar', jar: 'server.jar' },
    javaMajor: 25,
    javaPath: null,
    flags: [],
    memoryMb: 4096,
    autoStart: false,
    autoRestart: true,
    keepAwake: true,
    backupHours: 24,
    backupKeep: 7,
    lastBackup: T0 - 5 * 3600,
    upnp: false,
    from: null,
    created: T0 - 12 * 86400,
    eulaAt: T0 - 12 * 86400,
    owned: true,
    state: 'stopped',
    since: T0 - 3600,
    online: [],
    maxPlayers: 10,
    port: 25565,
    motd: p.name,
    whitelist: true,
    onlineMode: true,
    problem: null,
    work: null,
    memoryUsedMb: 0,
    cpu: 0,
    restartNeeded: false,
    icon: null,
    lan: '192.168.1.42:25565',
    ...p,
  };
}

const servers: SrvView[] = [
  base({ id: 'smp', name: 'Friends SMP', flavor: 'paper', mc: '26.3', loader: '207', launch: { type: 'jar', jar: 'paper.jar' }, state: 'running', since: T0 - 3 * 86400 - 4 * 3600, autoStart: true, upnp: true, maxPlayers: 12, memoryMb: 6144, memoryUsedMb: 3810, cpu: 14, motd: 'Friends SMP · season 3', onlineMode: false }),
  base({ id: 'create', name: 'Create + Cobblemon', flavor: 'neoForge', mc: '26.1.2', loader: '26.1.2.122', launch: { type: 'args', args: 'libraries/net/neoforged/neoforge/26.1.2.122/win_args.txt' }, from: 'Mix: Create + Cobblemon', memoryMb: 8192, port: 25566, lan: '192.168.1.42:25566', whitelist: true }),
  base({ id: 'build', name: 'Creative builds', flavor: 'vanilla', mc: '26.3', port: 25567, lan: '192.168.1.42:25567', backupHours: 0, memoryMb: 3072, whitelist: false }),
];
let iconsMade = false;
function icons() {
  if (iconsMade || typeof document === 'undefined') return;
  iconsMade = true;
  servers[0].icon = icon(110);
  servers[1].icon = packIcon('Create');
}

// ---------- players ----------

interface Walker {
  name: string;
  uuid: string;
  pos: [number, number, number];
  dir: number;
  dim: string;
  joined: number;
}

const walkers: Walker[] = [
  { name: 'kerio', uuid: '6c5d1e1f-2f6a-4b1e-9b0a-1e6a2f3d4c5b', pos: [12.5, 72, -40.5], dir: 0.4, dim: 'minecraft:overworld', joined: T0 - 2 * 3600 },
  { name: 'Alex_Builds', uuid: '1a2b3c4d-5e6f-4a1b-8c2d-3e4f5a6b7c8d', pos: [-180.5, 66, 95.5], dir: 2.2, dim: 'minecraft:overworld', joined: T0 - 41 * 60 },
  { name: 'SteveTheMiner', uuid: '5627dd98-e6be-3c21-b8a8-e92344183641', pos: [240.5, 64, 210.5], dir: 4.1, dim: 'minecraft:overworld', joined: T0 - 12 * 60 },
];

function online(): SrvOnline[] {
  return walkers.map((w) => ({ name: w.name, uuid: w.uuid, since: w.joined, pos: [...w.pos] as [number, number, number], dimension: w.dim }));
}

setInterval(() => {
  for (const w of walkers) {
    w.dir += (Math.random() - 0.5) * 0.6;
    w.pos[0] += Math.cos(w.dir) * 2.4;
    w.pos[2] += Math.sin(w.dir) * 2.4;
    w.pos[1] = 64 + Math.round(terrain(w.pos[0], w.pos[2]) * 30);
  }
  servers[0].online = online();
}, 1000);
servers[0].online = online();

// Someone tried kerio's name (an operator) from a new place: waiting for the owner.
const waiting: Record<string, SrvWaiting[]> = { smp: [{ name: 'kerio', place: '203.0.113.77', at: T0 - 260 }] };

const known = [
  { name: 'kerio', uuid: walkers[0].uuid, op: true, whitelisted: true, banned: false, banReason: null },
  { name: 'Alex_Builds', uuid: walkers[1].uuid, op: false, whitelisted: true, banned: false, banReason: null },
  { name: 'SteveTheMiner', uuid: walkers[2].uuid, op: false, whitelisted: true, banned: false, banReason: null },
  { name: 'Maya', uuid: '9f8e7d6c-5b4a-4c3d-8e2f-1a0b9c8d7e6f', op: false, whitelisted: true, banned: false, banReason: null },
  { name: 'xX_Griefer_Xx', uuid: '0f0e0d0c-0b0a-4908-8706-050403020100', op: false, whitelisted: false, banned: true, banReason: 'Burning down the village' },
];

const it = (slot: number, id: string, count = 1, extra: Partial<SrvPlayer['items'][number]> = {}) => ({ slot, id: `minecraft:${id}`, label: id.split('_').map((w) => w[0].toUpperCase() + w.slice(1)).join(' '), count, custom: null, enchanted: false, ...extra });
const inventories: Record<string, SrvPlayer['items']> = {
  kerio: [it(0, 'netherite_sword', 1, { enchanted: true, custom: 'Lightbringer' }), it(1, 'diamond_pickaxe', 1, { enchanted: true }), it(2, 'golden_carrot', 48), it(3, 'torch', 61), it(4, 'oak_log', 32), it(5, 'ender_pearl', 12), it(8, 'firework_rocket', 64), it(9, 'cobblestone', 64), it(10, 'cobblestone', 64), it(11, 'iron_ingot', 27), it(12, 'diamond', 9), it(14, 'redstone', 40), it(20, 'shulker_box'), it(27, 'experience_bottle', 16), it(103, 'netherite_helmet', 1, { enchanted: true }), it(102, 'elytra'), it(101, 'netherite_leggings', 1, { enchanted: true }), it(100, 'netherite_boots', 1, { enchanted: true }), it(-106, 'totem_of_undying')],
  Alex_Builds: [it(0, 'stone_bricks', 64), it(1, 'spruce_planks', 64), it(2, 'glass', 40), it(3, 'lantern', 16), it(4, 'deepslate_tiles', 64), it(103, 'leather_helmet')],
  SteveTheMiner: [it(0, 'iron_pickaxe'), it(1, 'torch', 22), it(2, 'bread', 9), it(9, 'raw_iron', 31), it(10, 'coal', 44), it(102, 'iron_chestplate')],
};

// ---------- the world ----------

function hash(x: number, z: number): number {
  let h = (x * 374761393 + z * 668265263) | 0;
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967295;
}

function noise(x: number, z: number): number {
  const xi = Math.floor(x);
  const zi = Math.floor(z);
  const fx = x - xi;
  const fz = z - zi;
  const s = (t: number) => t * t * (3 - 2 * t);
  const a = hash(xi, zi);
  const b = hash(xi + 1, zi);
  const c = hash(xi, zi + 1);
  const d = hash(xi + 1, zi + 1);
  return a + (b - a) * s(fx) + (c - a) * s(fz) + (a - b - c + d) * s(fx) * s(fz);
}

/** Height of the pretend world, −1 (deep sea) to 1 (mountain tops). */
function terrain(x: number, z: number): number {
  let v = 0;
  let amp = 1;
  let f = 1 / 220;
  for (let i = 0; i < 5; i++) {
    v += (noise(x * f, z * f) - 0.5) * amp;
    amp *= 0.5;
    f *= 2;
  }
  return v * 1.6 + 0.08;
}

const tiles = new Map<string, string>();
function tile(rx: number, rz: number, dim: string): string {
  const k = `${dim}${rx},${rz}`;
  const hit = tiles.get(k);
  if (hit) return hit;
  const N = 256;
  const c = document.createElement('canvas');
  c.width = c.height = N;
  const g = c.getContext('2d')!;
  const img = g.createImageData(N, N);
  for (let pz = 0; pz < N; pz++)
    for (let px = 0; px < N; px++) {
      const x = rx * 512 + px * 2;
      const z = rz * 512 + pz * 2;
      const h = terrain(x, z);
      const north = terrain(x, z - 2);
      let col: [number, number, number];
      if (dim === 'the_nether') col = h > 0.1 ? [112, 22, 18] : h > -0.15 ? [84, 64, 51] : [230, 100, 20];
      else if (dim === 'the_end') col = h > -0.2 ? [219, 207, 163] : [0, 0, 0];
      else if (h < -0.08) col = [Math.round(52 + h * 30), Math.round(92 + h * 40), Math.round(210 + h * 30)];
      else if (h < -0.03) col = [219, 207, 163];
      else if (h < 0.3) col = hash(x >> 2, z >> 2) > 0.72 ? [56, 118, 32] : [110, 170, 60];
      else if (h < 0.45) col = [118, 118, 118];
      else col = [236, 238, 240];
      const f = h < -0.08 ? 1 : h > north + 0.004 ? 1.07 : h < north - 0.004 ? 0.8 : 0.93;
      const i = (pz * N + px) * 4;
      img.data[i] = Math.min(255, col[0] * f);
      img.data[i + 1] = Math.min(255, col[1] * f);
      img.data[i + 2] = Math.min(255, col[2] * f);
      img.data[i + 3] = dim === 'the_end' && h <= -0.2 ? 0 : 255;
    }
  // A village near spawn.
  if (dim === 'overworld' && rx === 0 && rz === -1) {
    g.putImageData(img, 0, 0);
    g.fillStyle = '#8f7748';
    for (let i = 0; i < 9; i++) g.fillRect(20 + (i % 3) * 14, 200 + Math.floor(i / 3) * 14, 8, 8);
    g.fillStyle = '#9c8a6a';
    g.fillRect(14, 222, 50, 2);
  } else g.putImageData(img, 0, 0);
  const url = c.toDataURL();
  tiles.set(k, url);
  return url;
}

// ---------- console ----------

const consoles: Record<string, SrvLine[]> = {};
let n = 1;
function line(id: string, text: string, level: SrvLine['level'] = 'info') {
  const l = (consoles[id] ??= []);
  const t = new Date();
  const stamp = `[${t.toTimeString().slice(0, 8)} INFO]: `;
  l.push({ n: n++, at: now(), text: level === 'cmd' || text.startsWith('[OmniHub]') ? text : `${stamp}${text}`, level });
  if (l.length > 2000) l.shift();
  emit('mcserver:console', { id });
}

function boot(id: string) {
  const s = servers.find((x) => x.id === id)!;
  const lines = [
    `Starting minecraft server version ${s.mc}`,
    'Loading properties',
    `Default game type: SURVIVAL`,
    `Starting Minecraft server on *:${s.port}`,
    'Preparing level "world"',
    'Preparing spawn area: 100%',
    `Done (${(4 + Math.random() * 8).toFixed(3)}s)! For help, type "help"`,
  ];
  lines.forEach((t) => line(id, t));
}
line('smp', '[OmniHub] Starting Paper 26.3 with 6144 MB of memory…');
boot('smp');
line('smp', 'kerio joined the game');
line('smp', 'Alex_Builds joined the game');
line('smp', '<Alex_Builds> the castle roof is done!', 'chat');
line('smp', '<kerio> looks amazing', 'chat');
line('smp', 'SteveTheMiner joined the game');
line('smp', 'SteveTheMiner has made the advancement [Diamonds!]');
const CHAT = ['anyone want to go to the nether?', 'found a mineshaft at 240 -12 210', 'brb', 'who took my iron', 'trading hall is open', 'gg'];
setInterval(() => {
  if (servers[0].state !== 'running') return;
  const w = walkers[Math.floor(Math.random() * walkers.length)];
  if (Math.random() < 0.4) line('smp', `<${w.name}> ${CHAT[Math.floor(Math.random() * CHAT.length)]}`, 'chat');
  else if (Math.random() < 0.3) line('smp', 'Saving the game (this may take a moment!)');
}, 7000);

// ---------- content ----------

const contents: Record<string, SrvContent[]> = {
  smp: [
    { file: 'plugins/LuckPerms-Bukkit-5.5.jar', kind: 'plugin', title: 'LuckPerms', version: '5.5.17', icon: null, project: 'Vebnzrzj', source: 'modrinth', from: null, dependency: false, enabled: true, size: 1_960_000, sha1: '' },
    { file: 'plugins/Chunky-Bukkit-1.4.jar', kind: 'plugin', title: 'Chunky', version: '1.4.40', icon: null, project: 'fALzjamp', source: 'modrinth', from: null, dependency: false, enabled: true, size: 420_000, sha1: '' },
    { file: 'plugins/voicechat-bukkit-2.6.jar', kind: 'plugin', title: 'Simple Voice Chat', version: '2.6.2', icon: null, project: '9eGKb6K1', source: 'modrinth', from: null, dependency: false, enabled: true, size: 3_100_000, sha1: '' },
    { file: 'plugins/omnihub-homes.jar', kind: 'plugin', title: 'Homes', version: '', icon: null, project: null, source: 'claude', from: 'p-homes', dependency: false, enabled: true, size: 14_200, sha1: '' },
    { file: 'world/datapacks/omnihub-starter_kit', kind: 'datapack', title: 'Starter kit', version: '', icon: null, project: null, source: 'claude', from: 'p-kit', dependency: false, enabled: true, size: 2_400, sha1: '' },
    { file: 'world/datapacks/terralith.zip', kind: 'datapack', title: 'Terralith', version: '2.6.1', icon: null, project: '8oi3bsk5', source: 'modrinth', from: null, dependency: false, enabled: true, size: 2_800_000, sha1: '' },
  ],
  create: ['Create', 'Cobblemon', 'Farmer’s Delight', 'Jade', 'Create: Steam ’n’ Rails', 'Sophisticated Backpacks', 'Waystones', 'Balm'].map((t, i) => ({ file: `mods/${t.toLowerCase().replace(/\W+/g, '-')}.jar`, kind: 'mod' as SrvKind, title: t, version: `${6 - (i % 3)}.${i}.0`, icon: null, project: null, source: 'mix' as const, from: 'Create + Cobblemon', dependency: t === 'Balm', enabled: true, size: 400_000 + i * 310_000, sha1: '' })),
  build: [],
};

const HITS: Record<SrvKind, Omit<SrvHit, 'installed'>[]> = {
  plugin: [
    { project: 'Vebnzrzj', slug: 'luckperms', title: 'LuckPerms', description: 'A permissions plugin for Minecraft servers', author: 'Luck', icon: null, downloads: 4_100_000, categories: ['paper'], serverSide: 'required', clientSide: 'unsupported' },
    { project: 'fALzjamp', slug: 'chunky', title: 'Chunky', description: 'Pre-generates chunks, quickly and efficiently', author: 'pop4959', icon: null, downloads: 3_400_000, categories: ['paper'], serverSide: 'required', clientSide: 'unsupported' },
    { project: 'swbUV1cr', slug: 'bluemap', title: 'BlueMap', description: 'A 3D map of your world in the browser', author: 'Blue', icon: null, downloads: 1_900_000, categories: ['paper'], serverSide: 'required', clientSide: 'unsupported' },
    { project: 'wKkoqHrH', slug: 'geyser', title: 'Geyser', description: 'Lets Bedrock players (phones, consoles) join Java servers', author: 'GeyserMC', icon: null, downloads: 2_300_000, categories: ['paper'], serverSide: 'required', clientSide: 'unsupported' },
    { project: 'Lu3KuzdV', slug: 'coreprotect', title: 'CoreProtect', description: 'Logs and rolls back griefing', author: 'Intelli', icon: null, downloads: 1_200_000, categories: ['paper'], serverSide: 'required', clientSide: 'unsupported' },
  ],
  mod: [
    { project: 'LNytGWDc', slug: 'create', title: 'Create', description: 'Building tools and aesthetic technology', author: 'simibubi', icon: null, downloads: 62_000_000, categories: ['neoforge'], serverSide: 'required', clientSide: 'required' },
    { project: 'gvQqBUqZ', slug: 'lithium', title: 'Lithium', description: 'No-compromises game logic optimization', author: 'jellysquid3', icon: null, downloads: 41_000_000, categories: ['fabric'], serverSide: 'optional', clientSide: 'optional' },
    { project: 'bXX9h73M', slug: 'waystones', title: 'Waystones', description: 'Teleport back to activated waystones', author: 'BlayTheNinth', icon: null, downloads: 30_000_000, categories: ['neoforge'], serverSide: 'required', clientSide: 'required' },
  ],
  datapack: [
    { project: '8oi3bsk5', slug: 'terralith', title: 'Terralith', description: 'Explore almost 100 new biomes', author: 'Stardust Labs', icon: null, downloads: 9_000_000, categories: ['datapack'], serverSide: 'required', clientSide: 'unsupported' },
    { project: 'vandb', slug: 'vanilla-tweaks-vein-miner', title: 'Vein Miner', description: 'Mine a whole ore vein at once', author: 'VanillaTweaks', icon: null, downloads: 1_600_000, categories: ['datapack'], serverSide: 'required', clientSide: 'unsupported' },
    { project: 'grave', slug: 'graves', title: 'Graves', description: 'Your items wait in a grave where you died', author: 'VanillaTweaks', icon: null, downloads: 2_100_000, categories: ['datapack'], serverSide: 'required', clientSide: 'unsupported' },
  ],
};

const backups: Record<string, SrvBackup[]> = {
  smp: [0, 1, 2, 3].map((i) => ({ name: `auto-world-2026-10-${String(20 - i).padStart(2, '0')}_04-00-00.zip`, at: T0 - 5 * 3600 - i * 86400, size: 1_480_000_000 - i * 22_000_000, auto: true })).concat([{ name: 'world-before-the-castle.zip', at: T0 - 9 * 86400, size: 1_310_000_000, auto: false }]),
  create: [],
  build: [],
};

const props: Record<string, [string, string][]> = {};
function propsOf(s: SrvView): [string, string][] {
  return (props[s.id] ??= [
    ['motd', s.motd],
    ['max-players', String(s.maxPlayers)],
    ['server-port', String(s.port)],
    ['gamemode', 'survival'],
    ['difficulty', 'normal'],
    ['pvp', 'true'],
    ['white-list', String(s.whitelist)],
    ['enforce-whitelist', String(s.whitelist)],
    ['online-mode', String(s.onlineMode)],
    ['view-distance', '12'],
    ['simulation-distance', '10'],
    ['spawn-protection', '16'],
    ['allow-flight', 'false'],
    ['enable-command-block', 'false'],
    ['hardcore', 'false'],
    ['level-name', 'world'],
    ['level-seed', ''],
    ['enable-rcon', 'false'],
    ['pause-when-empty-seconds', '60'],
  ]);
}

function find(id: string): SrvView {
  const s = servers.find((x) => x.id === id);
  if (!s) throw 'That server is gone.';
  return s;
}

function changed(id: string) {
  emit('mcserver:changed', { id });
}

function setState(s: SrvView, state: SrvView['state']) {
  s.state = state;
  s.since = now();
  if (state !== 'running') s.online = [];
  s.memoryUsedMb = state === 'running' ? Math.round(s.memoryMb * 0.55) : state === 'starting' ? Math.round(s.memoryMb * 0.2) : 0;
  s.cpu = state === 'running' ? 9 : state === 'starting' ? 70 : 0;
  changed(s.id);
}

const VERSIONS: Record<SrvFlavor, string[]> = {
  vanilla: ['26.3', '26.2', '26.1.2', '26.1.1', '1.21.11', '1.21.10', '1.21.8', '1.21.4', '1.21.1', '1.20.6', '1.20.4', '1.20.1', '1.19.4', '1.18.2', '1.16.5', '1.12.2'],
  paper: ['26.3', '26.2', '26.1.2', '1.21.11', '1.21.10', '1.21.8', '1.21.4', '1.21.1', '1.20.6', '1.20.4', '1.20.1', '1.19.4', '1.18.2', '1.16.5', '1.12.2'],
  fabric: ['26.3', '26.2', '26.1.2', '1.21.11', '1.21.10', '1.21.8', '1.21.4', '1.21.1', '1.20.1'],
  quilt: ['1.21.11', '1.21.10', '1.21.4', '1.21.1', '1.20.1'],
  forge: ['26.3', '26.2', '26.1.2', '1.21.11', '1.21.1', '1.20.1', '1.19.2', '1.18.2', '1.16.5', '1.12.2'],
  neoForge: ['26.3', '26.2', '26.1.2', '1.21.11', '1.21.10', '1.21.1', '1.20.4'],
};

export const serversMock = {
  list(): SrvView[] {
    icons();
    return servers;
  },
  get: (id: string) => (icons(), find(id)),
  versions: (flavor: SrvFlavor) => VERSIONS[flavor],
  create(req: SrvNew): SrvView {
    if (!req.eula) throw 'A Minecraft server only runs once you accept Minecraft’s EULA (aka.ms/MinecraftEULA).';
    const id = `s${Date.now()}`;
    const port = 25565 + servers.length;
    const lp = req.from?.type === 'launcher' ? launchersMock.get(req.from.id) : undefined;
    if (req.from?.type === 'launcher' && !lp) throw new Error('That modpack isn’t there any more.');
    const from = lp ? `${lp.launcherName}: ${lp.name}` : req.from?.type === 'mix' ? 'Mix: Create + Cobblemon' : req.from?.type === 'modrinth' ? 'Modpack' : req.from ? 'Modpack: from a file' : null;
    const s = base({ id, name: req.name, flavor: req.from?.type === 'mix' ? 'neoForge' : req.flavor, mc: req.mc || '26.1.2', state: 'installing', port, lan: `192.168.1.42:${port}`, memoryMb: req.memoryMb ?? 4096, maxPlayers: req.maxPlayers ?? 10, whitelist: req.whitelist, onlineMode: req.onlineMode, from, backupHours: 24, lastBackup: 0, created: now(), eulaAt: now(), launch: null });
    servers.push(s);
    contents[id] = [];
    backups[id] = [];
    const steps = [`Downloading Java ${s.javaMajor}`, 'Unpacking Java', `Downloading ${s.flavor === 'paper' ? 'Paper' : 'Minecraft'} ${s.mc}`, ...(from ? ['Adding Create', 'Adding Farmer’s Delight', 'Adding Jade'] : [])];
    steps.forEach((t, i) =>
      setTimeout(() => {
        s.work = { text: t, done: i, total: steps.length };
        emit('mcserver:work', { id, ...s.work });
        changed(id);
      }, 400 + i * 700),
    );
    setTimeout(() => {
      s.work = null;
      s.launch = { type: 'jar', jar: s.flavor === 'paper' ? 'paper.jar' : 'server.jar' };
      setState(s, 'stopped');
      line(id, '[OmniHub] Ready. Press Start.');
      if (req.start) serversMock.start(id);
    }, 600 + steps.length * 700);
    return s;
  },
  adopt(path: string): SrvView {
    const s = base({ id: `a${Date.now()}`, name: path.split(/[\\/]/).pop() || 'Server', flavor: 'paper', mc: '1.21.4', dir: path, owned: false, backupHours: 0 });
    servers.push(s);
    contents[s.id] = [];
    backups[s.id] = [];
    return s;
  },
  update(id: string, p: SrvPatch): SrvView {
    const s = find(id);
    Object.assign(s, Object.fromEntries(Object.entries(p).filter(([k, v]) => v !== undefined && k !== 'eula' && k !== 'javaPath')));
    if (p.javaPath !== undefined) s.javaPath = p.javaPath || null;
    if (p.eula) s.eulaAt = now();
    changed(id);
    return s;
  },
  remove(id: string): void {
    const i = servers.findIndex((s) => s.id === id);
    if (i >= 0) servers.splice(i, 1);
    changed(id);
  },
  start(id: string): SrvView {
    const s = find(id);
    if (s.state === 'running' || s.state === 'starting') throw 'It’s already running.';
    setState(s, 'starting');
    line(id, `[OmniHub] Starting ${s.flavor} ${s.mc} with ${s.memoryMb} MB of memory…`);
    setTimeout(() => boot(id), 300);
    setTimeout(() => {
      setState(s, 'running');
      if (id === 'smp') s.online = online();
    }, 2200);
    return s;
  },
  stop(id: string): SrvView {
    const s = find(id);
    setState(s, 'stopping');
    line(id, 'Stopping the server');
    setTimeout(() => {
      line(id, '[OmniHub] The server stopped.');
      setState(s, 'stopped');
    }, 1500);
    return s;
  },
  restart(id: string): SrvView {
    const s = serversMock.stop(id);
    setTimeout(() => serversMock.start(id), 1800);
    return s;
  },
  kill(id: string): void {
    setState(find(id), 'stopped');
  },
  console: (id: string, after: number | null) => (consoles[id] ?? []).filter((l) => after === null || l.n > after),
  command(id: string, text: string): void {
    const s = find(id);
    if (s.state !== 'running') throw 'The server isn’t running.';
    line(id, `> ${text}`, 'cmd');
    const w = text.split(/\s+/);
    setTimeout(() => {
      if (w[0] === 'list') line(id, `There are ${s.online.length} of a max of ${s.maxPlayers} players online: ${s.online.map((o) => o.name).join(', ')}`);
      else if (w[0] === 'say') line(id, `[Server] ${w.slice(1).join(' ')}`, 'chat');
      else if (w[0] === 'save-all') (line(id, 'Saving the game (this may take a moment!)'), line(id, 'Saved the game'));
      else if (w[0] === 'time') line(id, 'Set the time to 1000');
      else if (w[0] === 'weather') line(id, 'Set the weather to clear');
      else if (w[0] === 'difficulty') line(id, 'The difficulty is Normal');
      else if (w[0] === 'seed') line(id, 'Seed: [-3263230288278424330]');
      else if (w[0] === 'tps') line(id, 'TPS from last 1m, 5m, 15m: 20.0, 20.0, 20.0');
      else line(id, `Unknown or incomplete command. See below for error`);
    }, 150);
  },
  properties: (id: string) => propsOf(find(id)),
  setProperties(id: string, changes: [string, string][]): SrvView {
    const s = find(id);
    const p = propsOf(s);
    for (const [k, v] of changes) {
      const e = p.find(([key]) => key === k);
      if (e) e[1] = v;
      else p.push([k, v]);
      if (k === 'max-players') s.maxPlayers = Number(v);
      if (k === 'motd') s.motd = v;
      if (k === 'white-list') s.whitelist = v === 'true';
      if (k === 'online-mode') s.onlineMode = v !== 'false';
    }
    if (s.state === 'running' && changes.some(([k]) => !['difficulty', 'white-list', 'gamemode'].includes(k))) s.restartNeeded = true;
    changed(id);
    return s;
  },
  setIcon: (id: string) => find(id),
  players(id: string): SrvPlayers {
    const s = find(id);
    const list = id === 'smp' ? known.map((k) => ({ ...k, online: s.state === 'running' && s.online.some((o) => o.name === k.name) })) : [];
    return { online: s.online, known: list, whitelistOn: s.whitelist, enforce: s.whitelist, maxPlayers: s.maxPlayers, onlineMode: s.onlineMode, waiting: s.onlineMode ? [] : (waiting[id] ?? []) };
  },
  player(id: string, name: string): SrvPlayer {
    const s = find(id);
    const w = walkers.find((x) => x.name === name);
    const on = !!w && s.state === 'running' && id === 'smp';
    return { name, uuid: w?.uuid ?? known.find((k) => k.name === name)?.uuid ?? null, online: on, live: on, pos: w ? [...w.pos] : name === 'Maya' ? [88.5, 70, -310.5] : null, dimension: 'minecraft:overworld', health: on ? 17 : 20, food: on ? 18 : 20, xpLevel: name === 'kerio' ? 31 : 7, gamemode: 'survival', selected: 0, items: inventories[name] ?? [], ender: [] };
  },
  act(id: string, name: string, a: SrvAction): string {
    const k = known.find((x) => x.name === name);
    switch (a.type) {
      case 'allow':
      case 'refuse':
        waiting[id] = (waiting[id] ?? []).filter((w) => !(w.name === name && w.place === a.place));
        changed(id);
        return a.type === 'allow' ? `${name} can join from ${a.place} now. Ask them to join again.` : 'Kept out.';
      case 'give': {
        const inv = (inventories[name] ??= []);
        const free = Array.from({ length: 36 }, (_, i) => i).find((i) => !inv.some((x) => x.slot === i));
        if (free !== undefined) inv.push(it(free, a.item.replace('minecraft:', ''), a.count));
        return `Gave ${a.count} [${a.item.replace('minecraft:', '').replace(/_/g, ' ')}] to ${name}`;
      }
      case 'clearSlot':
        inventories[name] = (inventories[name] ?? []).filter((x) => x.slot !== a.slot);
        return `Replaced a slot on ${name} with [Air]`;
      case 'teleport': {
        const w = walkers.find((x) => x.name === name);
        if (w) w.pos = [a.x, a.y, a.z];
        return `Teleported ${name} to ${a.x.toFixed(6)}, ${a.y.toFixed(6)}, ${a.z.toFixed(6)}`;
      }
      case 'whitelist':
        if (k) k.whitelisted = a.on;
        else known.push({ name, uuid: crypto.randomUUID(), op: false, whitelisted: true, banned: false, banReason: null });
        return a.on ? `Added ${name} to the whitelist` : `Removed ${name} from the whitelist`;
      case 'op':
        if (k) k.op = true;
        return `Made ${name} a server operator`;
      case 'deop':
        if (k) k.op = false;
        return `Made ${name} no longer a server operator`;
      case 'ban':
        if (k) (k.banned = true), (k.banReason = a.reason ?? 'Banned by an operator.');
        return `Banned ${name}: ${a.reason ?? 'Banned by an operator.'}`;
      case 'pardon':
        if (k) k.banned = false;
        return `Unbanned ${name}`;
      case 'kick':
        return `Kicked ${name}: ${a.reason ?? 'Kicked by an operator.'}`;
      case 'heal':
        return `Applied effect Instant Health to ${name}`;
      case 'feed':
        return `Applied effect Saturation to ${name}`;
      case 'gamemode':
        return `Set ${name}'s game mode to ${a.mode[0].toUpperCase()}${a.mode.slice(1)} Mode`;
      case 'message':
        return `You whisper to ${name}: ${a.text}`;
      default:
        return 'Sent.';
    }
  },
  whitelist(id: string, on: boolean): SrvView {
    return serversMock.setProperties(id, [
      ['white-list', String(on)],
      ['enforce-whitelist', String(on)],
    ]);
  },
  watch: (id: string) => find(id).online,
  map(id: string, dim: string): SrvMap {
    const s = find(id);
    const r = id === 'smp' ? (dim === 'overworld' ? 2 : 1) : id === 'create' ? 1 : 0;
    const regions: [number, number][] = [];
    for (let x = -r; x < r; x++) for (let z = -r; z < r; z++) regions.push([x, z]);
    return { regions, spawn: dim === 'overworld' && r ? [0, 70, 16] : null, players: s.state === 'running' ? s.online.filter((o) => (o.dimension ?? '').endsWith(dim)) : [], supported: true };
  },
  mapTile: (_id: string, dim: string, x: number, z: number) => tile(x, z, dim),
  content: (id: string) => contents[id] ?? [],
  identify: (id: string) => contents[id] ?? [],
  search(id: string, kind: SrvKind, query: string): { hits: SrvHit[]; total: number } {
    const have = (contents[id] ?? []).map((c) => c.project);
    const list = HITS[kind].filter((h) => !query || `${h.title} ${h.description}`.toLowerCase().includes(query.toLowerCase())).map((h) => ({ ...h, icon: h.icon ?? packIcon(h.title), installed: have.includes(h.project) }));
    return { hits: list, total: list.length };
  },
  install(id: string, kind: SrvKind, project: string): SrvContent[] {
    const h = HITS[kind].find((x) => x.project === project)!;
    const folder = kind === 'mod' ? 'mods' : kind === 'plugin' ? 'plugins' : 'world/datapacks';
    const c: SrvContent = { file: `${folder}/${h.slug}.${kind === 'datapack' ? 'zip' : 'jar'}`, kind, title: h.title, version: '1.0.0', icon: packIcon(h.title), project, source: 'modrinth', from: null, dependency: false, enabled: true, size: 1_200_000, sha1: '' };
    (contents[id] ??= []).push(c);
    const s = find(id);
    if (kind !== 'datapack' && s.state === 'running') s.restartNeeded = true;
    emit('mcserver:content', { id });
    changed(id);
    return [c];
  },
  addFile(id: string, path: string): SrvContent {
    const name = path.split(/[\\/]/).pop() ?? 'file.jar';
    const kind: SrvKind = name.endsWith('.zip') ? 'datapack' : find(id).flavor === 'paper' ? 'plugin' : 'mod';
    const c: SrvContent = { file: `${kind === 'mod' ? 'mods' : kind === 'plugin' ? 'plugins' : 'world/datapacks'}/${name}`, kind, title: name.replace(/\.(jar|zip)$/, ''), version: '', icon: null, project: null, source: 'file', from: null, dependency: false, enabled: true, size: 220_000, sha1: '' };
    (contents[id] ??= []).push(c);
    return c;
  },
  setContent(id: string, file: string, on: boolean): void {
    const c = (contents[id] ?? []).find((x) => x.file === file);
    if (c) c.enabled = on;
  },
  removeContent(id: string, file: string): void {
    contents[id] = (contents[id] ?? []).filter((x) => x.file !== file);
  },
  backups: (id: string) => backups[id] ?? [],
  backup(id: string): SrvBackup {
    const s = find(id);
    const b: SrvBackup = { name: `world-${new Date().toISOString().slice(0, 19).replace('T', '_').replace(/:/g, '-')}.zip`, at: now(), size: 1_500_000_000, auto: false };
    (backups[id] ??= []).unshift(b);
    s.lastBackup = now();
    line(id, '[OmniHub] Backed up the world (1.5 GB).');
    changed(id);
    return b;
  },
  restore: (id: string, name: string) => (line(id, `[OmniHub] Put back ${name}.`), `world.before-restore-${new Date().toISOString().slice(0, 10)}`),
  removeBackup(id: string, name: string): void {
    backups[id] = (backups[id] ?? []).filter((b) => b.name !== name);
  },
  network(id: string): SrvNet {
    const s = find(id);
    return { port: s.port, lan: [`192.168.1.42:${s.port}`], publicIp: '203.0.113.24', router: 'TP-Link Archer AX55', routerIp: '203.0.113.24', upnp: s.upnp, portOpen: s.upnp && s.state === 'running', upnpError: null, cgnat: false, java: `C:\\Users\\Player\\AppData\\Local\\OmniHub\\data\\mcserver\\java\\${s.javaMajor}-jre\\bin\\java.exe`, firewall: { supported: true, program: 'java.exe', networks: [{ id: '{a}', name: 'Home Wi-Fi', category: 'private' }], activeProfiles: 2, verdict: 'allowed', rules: [], message: 'Windows Firewall allows the Minecraft server on Home Wi-Fi.' } };
  },
  setUpnp(id: string, on: boolean): SrvNet {
    find(id).upnp = on;
    changed(id);
    return serversMock.network(id);
  },
  fixFirewall: (id: string) => serversMock.network(id),
  /** A data pack or plugin Claude made, put on its server. */
  addMade(gameKey: string, name: string, kind: 'datapack' | 'paperPlugin'): string {
    const id = gameKey.split(':')[1];
    const slug = name.toLowerCase().replace(/\W+/g, '_');
    const c: SrvContent = { file: kind === 'datapack' ? `world/datapacks/omnihub-${slug}` : `plugins/omnihub-${slug}.jar`, kind: kind === 'datapack' ? 'datapack' : 'plugin', title: name, version: '', icon: null, project: null, source: 'claude', from: null, dependency: false, enabled: true, size: 3_000, sha1: '' };
    (contents[id] ??= []).push(c);
    emit('mcserver:content', { id });
    return c.file;
  },
  serverName: (gameKey: string) => servers.find((s) => s.id === gameKey.split(':')[1])?.name ?? 'Server',
};
