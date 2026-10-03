// Generates believable C:\, D:\ and E:\ trees for the storage mock.
// Everything is seeded, so sizes and names are the same on every load.

import { DAY, GB, KB, MB, NOW, Rng } from './rng';
import { FLAG_CLOUD, FLAG_HIDDEN, FLAG_SYSTEM, FakeTree } from './tree';
import type { VolumeInfo } from '@shared/types';

type Age = number | [number, number];
type Namer = (i: number, r: Rng) => string;

const HS = FLAG_HIDDEN | FLAG_SYSTEM;

class Builder {
  constructor(
    readonly t: FakeTree,
    readonly r: Rng,
  ) {}

  private when(age: Age): number {
    const days = typeof age === 'number' ? age : this.r.range(age[0], age[1]);
    return Math.floor(NOW - days * DAY - this.r.range(0, 6 * 3600));
  }

  /** Create (or reuse) nested folders, e.g. mk(root, 'Users\\Alex\\AppData'). */
  mk(parent: number, rel: string, flags = 0, age: Age = [20, 400]): number {
    let cur = parent;
    for (const name of rel.split('\\').filter(Boolean)) {
      const existing = this.t.node(cur).children.find((c) => this.t.nodes[c].name === name && this.t.nodes[c].isDir);
      cur = existing ?? this.t.add(cur, name, true, 0, this.when(age), flags);
    }
    return cur;
  }

  file(parent: number, name: string, size: number, age: Age = [10, 300], flags = 0): number {
    return this.t.add(parent, name, false, Math.round(size), this.when(age), flags);
  }

  /** `count` files whose sizes follow a log-normal spread summing to `total`. */
  spread(parent: number, count: number, total: number, namer: Namer, age: Age = [10, 300], sigma = 1.1, flags = 0): void {
    const weights: number[] = [];
    let sum = 0;
    for (let i = 0; i < count; i++) {
      const w = Math.exp(this.r.normal() * sigma);
      weights.push(w);
      sum += w;
    }
    const used = new Set(this.t.node(parent).children.map((c) => this.t.nodes[c].name.toLowerCase()));
    for (let i = 0; i < count; i++) {
      let name = namer(i, this.r);
      if (used.has(name.toLowerCase())) {
        const dot = name.lastIndexOf('.');
        const base = dot > 0 ? name.slice(0, dot) : name;
        const ext = dot > 0 ? name.slice(dot) : '';
        let k = 2;
        while (used.has(`${base}_${k}${ext}`.toLowerCase())) k++;
        name = `${base}_${k}${ext}`;
      }
      used.add(name.toLowerCase());
      this.file(parent, name, Math.max(1, (total * weights[i]) / sum), age, flags);
    }
  }

  /** Several sub-folders, each with a spread of files. */
  folders(parent: number, names: string[], filesEach: [number, number], total: number, namer: Namer, age: Age = [10, 300], flags = 0): void {
    const shares = names.map(() => Math.exp(this.r.normal() * 0.9));
    const sum = shares.reduce((a, b) => a + b, 0);
    names.forEach((n, i) => {
      const d = this.mk(parent, n, flags, age);
      this.spread(d, this.r.int(filesEach[0], filesEach[1]), (total * shares[i]) / sum, namer, age, 1.0, flags);
    });
  }

  copy(id: number, parent: number, name?: string, age: Age = [2, 60]): number {
    const n = this.t.node(id);
    return this.file(parent, name ?? n.name, n.size, age);
  }
}

// ---------- name generators ----------

const DLL_A = ['api-ms-win-', 'd3d', 'msvcp', 'vcruntime', 'lib', 'Qt6', 'ucrt', 'nv', 'Windows.', 'System.', 'mf', 'dx', 'ole', 'shell', 'crypt', 'ws2_', 'kernel', 'gdi', 'comctl', 'icu', 'av', 'cef', 'v8_', 'ffmpeg', 'skia', 'vulkan-', 'cuda', 'onnx', 'Microsoft.', 'win', 'Media.', 'xaudio'];
const DLL_B = ['core', 'base', 'ui', 'media', 'render', 'net', 'audio', 'video', 'io', 'gfx', 'compat', 'shell', 'storage', 'crypto', 'sync', 'input', 'xml', 'runtime', 'host', 'svc', 'data', 'graphics', 'ext', 'codec', 'font', 'gpu'];
const DLL_C = ['', '32', '64', '140', '_1', '2', 'w', 'ex', '-l1-1-0', '_win', 'helper', 'api', '12', '_x64'];

const dll =
  (exts: string[] = ['dll', 'dll', 'dll', 'exe', 'sys', 'mui', 'dat']): Namer =>
  (_, r) =>
    `${r.pick(DLL_A)}${r.pick(DLL_B)}${r.pick(DLL_C)}.${r.pick(exts)}`;

const seq =
  (prefix: string, ext: string, pad = 4, start = 0): Namer =>
  (i) =>
    `${prefix}${String(i + start).padStart(pad, '0')}.${ext}`;

const hexName =
  (len: number, ext?: string): Namer =>
  (_, r) =>
    r.hex(len) + (ext ? `.${ext}` : '');

const names =
  (list: string[]): Namer =>
  (i) =>
    list[i % list.length];

const fromList =
  (bases: string[], exts: string[]): Namer =>
  (i, r) =>
    `${bases[i % bases.length]}${i >= bases.length ? ` ${Math.floor(i / bases.length) + 1}` : ''}.${r.pick(exts)}`;

const PHOTO = (year: number): Namer => (i, r) => {
  const m = String(r.int(1, 12)).padStart(2, '0');
  const d = String(r.int(1, 28)).padStart(2, '0');
  return r.chance(0.25) ? `IMG_${year}${m}${d}_${String(100000 + i * 37).slice(-6)}.heic` : `IMG_${year}${m}${d}_${String(120000 + i * 53).slice(-6)}.jpg`;
};

const PKGS = ['react', 'react-dom', 'typescript', 'vite', 'esbuild', '@babel', 'lodash', 'rollup', 'postcss', 'tailwindcss', 'zod', 'zustand', 'motion', 'lucide-react', 'marked', 'dompurify', 'eslint', 'prettier', 'chalk', 'semver', 'picomatch', 'fast-glob', 'nanoid', 'immer', 'date-fns', 'axios', 'ws', 'undici', 'source-map', 'acorn', 'browserslist', 'caniuse-lite', 'electron-to-chromium', 'debug', 'ms', 'yaml', 'commander', 'glob', 'minimatch', 'tslib', '@types', '@tauri-apps', '@tailwindcss', 'lightningcss', 'sharp', 'playwright-core', 'jiti', 'magic-string', 'estree-walker', 'cross-spawn'];
const SRC_FILES = ['index.js', 'index.d.ts', 'package.json', 'README.md', 'LICENSE', 'cjs.js', 'esm.mjs', 'utils.js', 'types.d.ts', 'CHANGELOG.md'];

function nodeModules(b: Builder, parent: number, packages: number, total: number): void {
  const nm = b.mk(parent, 'node_modules', 0, [5, 90]);
  const per = total / packages;
  for (let i = 0; i < packages; i++) {
    const name = i < PKGS.length ? PKGS[i] : `${b.r.pick(PKGS).replace('@', '')}-${b.r.pick(['utils', 'core', 'plugin', 'cli', 'compat', 'parser'])}`;
    const pkg = b.mk(nm, name, 0, [5, 90]);
    const dist = b.mk(pkg, 'dist', 0, [5, 90]);
    b.spread(pkg, b.r.int(3, 6), per * 0.25, (k) => SRC_FILES[k % SRC_FILES.length], [5, 90], 0.8);
    b.spread(dist, b.r.int(3, 12), per * 0.75, (k, r) => `${r.pick(['index', 'chunk', 'bundle', 'core', 'runtime', 'server', 'client'])}-${r.hex(6)}.${k % 3 === 0 ? 'js.map' : 'js'}`, [5, 90], 1.0);
  }
}

const ARTISTS: [string, string[]][] = [
  ['Daft Punk', ['Random Access Memories', 'Discovery']],
  ['Radiohead', ['In Rainbows', 'OK Computer']],
  ['Tame Impala', ['Currents', 'The Slow Rush']],
  ['Massive Attack', ['Mezzanine']],
  ['Bonobo', ['Migration', 'Fragments']],
  ['Fleetwood Mac', ['Rumours']],
  ['Kendrick Lamar', ['DAMN.', 'To Pimp a Butterfly']],
  ['Hans Zimmer', ['Interstellar OST', 'Dune OST']],
  ['ODESZA', ['A Moment Apart']],
  ['Pink Floyd', ['The Dark Side of the Moon', 'Wish You Were Here']],
  ['Billie Eilish', ['Hit Me Hard and Soft']],
  ['Justice', ['Hyperdrama']],
];
const TRACK_WORDS = ['Intro', 'Midnight', 'Echoes', 'Gravity', 'Lights', 'Runaway', 'Horizon', 'Static', 'Golden', 'Afterglow', 'Signal', 'Northern', 'Satellite', 'Outro', 'Reverie', 'Pulse'];

function music(b: Builder, parent: number, albumLimit: number, ext: 'flac' | 'mp3'): void {
  let albums = 0;
  for (const [artist, list] of ARTISTS) {
    for (const album of list) {
      if (albums++ >= albumLimit) return;
      const d = b.mk(parent, `${artist}\\${album}`, 0, [200, 1500]);
      const n = b.r.int(9, 14);
      for (let i = 0; i < n; i++) {
        const title = `${b.r.pick(TRACK_WORDS)}${b.r.chance(0.4) ? ` ${b.r.pick(TRACK_WORDS)}` : ''}`;
        b.file(d, `${String(i + 1).padStart(2, '0')} - ${title}.${ext}`, ext === 'flac' ? b.r.range(22, 64) * MB : b.r.range(5, 12) * MB, [200, 1500]);
      }
      b.file(d, 'cover.jpg', b.r.range(0.3, 2.4) * MB, [200, 1500]);
    }
  }
}

const GUID = (r: Rng) => `{${r.hex(8)}-${r.hex(4)}-${r.hex(4)}-${r.hex(4)}-${r.hex(12)}}`.toUpperCase();

// ---------- C:\ ----------

function buildC(): FakeTree {
  const t = new FakeTree('C:\\', NOW - 3 * DAY);
  const r = new Rng(0xc0ffee);
  const b = new Builder(t, r);
  const root = 0;

  // Root system files
  b.file(root, 'pagefile.sys', 24 * GB, 0, HS);
  b.file(root, 'hiberfil.sys', 12.7 * GB, 1, HS);
  b.file(root, 'swapfile.sys', 16 * MB, 1, HS);
  b.file(root, 'DumpStack.log.tmp', 8 * KB, 1, HS);

  // $Recycle.Bin
  const bin = b.mk(root, '$Recycle.Bin\\S-1-5-21-3623811015-3361044348-30300820-1001', HS, [1, 20]);
  b.file(bin, '$RXK2M1A.mp4', 4.2 * GB, [6, 9], HS);
  b.file(bin, '$R8QW2LZ.zip', 2.1 * GB, [12, 14], HS);
  b.file(bin, '$RJ3K9PL.iso', 1.9 * GB, [20, 22], HS);
  b.spread(b.mk(bin, '$RT7Y2KD', HS, [3, 4]), 140, 1.6 * GB, (i, rr) => `${rr.pick(['render', 'frame', 'clip', 'asset', 'cache'])}_${i}.${rr.pick(['png', 'exr', 'tmp', 'mp4'])}`, [3, 4], 1.0, HS);
  b.spread(bin, 12, 6 * KB, (_, rr) => `$I${rr.hex(6).toUpperCase()}`, [1, 20], 0.2, HS);

  // Games (custom Steam library on C:)
  const common = b.mk(root, 'Games\\Steam\\steamapps\\common', 0, [30, 300]);
  const steamapps = b.mk(root, 'Games\\Steam\\steamapps');
  const bg3 = b.mk(common, "Baldurs Gate 3", 0, [40, 60]);
  b.spread(b.mk(bg3, 'Data'), 48, 112 * GB, (i) => (i === 0 ? 'Gustav.pak' : i === 1 ? 'Textures.pak' : i === 2 ? 'VirtualTextures.pak' : `Patch${i}_Hotfix${i % 4}.pak`), [40, 60], 1.3);
  b.spread(b.mk(bg3, 'bin'), 70, 1.2 * GB, dll(['dll', 'exe', 'pdb']), [40, 60]);
  const cp = b.mk(common, 'Cyberpunk 2077', 0, [20, 40]);
  b.spread(b.mk(cp, 'archive\\pc\\content'), 36, 54 * GB, (i) => `basegame_${i + 1}_${['engine', 'gamedata', 'audio', 'textures', 'lang', 'movies', 'world'][i % 7]}.archive`, [20, 200], 1.0);
  b.spread(b.mk(cp, 'archive\\pc\\ep1'), 10, 9.4 * GB, (i) => `ep1_${i + 1}_${['textures', 'gamedata', 'audio', 'world'][i % 4]}.archive`, [20, 200], 0.9);
  b.spread(b.mk(cp, 'bin\\x64'), 110, 1.4 * GB, dll(), [20, 40]);
  b.spread(b.mk(cp, 'r6\\cache'), 14, 2.1 * GB, (i) => `${['final', 'tweakdb', 'inkatlas', 'shader'][i % 4]}_${i}.redscripts`, [20, 40]);
  const er = b.mk(common, 'ELDEN RING\\Game', 0, [90, 200]);
  [['Data0.bdt', 15.8], ['Data1.bdt', 9.6], ['Data2.bdt', 7.1], ['Data3.bdt', 11.4], ['Data0.bhd', 0.0004], ['Data1.bhd', 0.0003], ['Data2.bhd', 0.0003], ['Data3.bhd', 0.0002], ['eldenring.exe', 0.08], ['oo2core_6_win64.dll', 0.001]].forEach(([n, s]) => b.file(er, String(n), Number(s) * GB, [90, 200]));
  b.spread(b.mk(er, 'sd'), 4, 2.6 * GB, (i) => `sd${i ? `_dlc0${i}` : ''}.bdt`, [90, 200]);
  const cs = b.mk(common, 'Counter-Strike Global Offensive', 0, [2, 10]);
  b.spread(b.mk(cs, 'game\\csgo'), 180, 31 * GB, seq('pak01_', 'vpk', 3), [2, 10], 0.5);
  b.spread(b.mk(cs, 'game\\bin\\win64'), 140, 0.9 * GB, dll(['dll', 'exe']), [2, 10]);
  b.spread(b.mk(common, 'Monster Hunter Wilds'), 64, 71 * GB, (i) => `re_chunk_000.pak.sub_${String(i).padStart(3, '0')}.pak`, [10, 40], 1.2);
  b.spread(b.mk(common, 'Hades II'), 260, 8.6 * GB, (i, rr) => `${rr.pick(['Content', 'Audio', 'Packages', 'Maps'])}_${i}.${rr.pick(['pkg', 'bank', 'map_text', 'fsb'])}`, [5, 30]);
  b.folders(b.mk(steamapps, 'shadercache'), ['1086940', '1091500', '1245620', '730', '1145350', '431960', '1172470', '892970'], [4, 12], 3.1 * GB, (i) => `steamapp_pipeline_cache_${i}.foz`, [0, 30]);
  b.spread(b.mk(steamapps, 'workshop\\content\\431960'), 60, 1.8 * GB, (i) => `${2800000000 + i * 7919}.pkg`, [30, 300]);
  b.spread(steamapps, 6, 40 * KB, (i) => `appmanifest_${['1086940', '1091500', '1245620', '730', '1145350', '431960'][i]}.acf`, [2, 30]);

  // Program Files
  const pf = b.mk(root, 'Program Files', 0, [60, 500]);
  b.spread(b.mk(pf, 'Adobe\\Adobe Premiere Pro 2025'), 600, 9.8 * GB, dll(['dll', 'dll', 'exe', 'zip', 'pak', 'aex', 'prm']), [40, 120]);
  b.spread(b.mk(pf, 'Adobe\\Adobe Photoshop 2025'), 400, 5.4 * GB, dll(['dll', 'dll', 'exe', '8bi', 'pak']), [40, 120]);
  b.spread(b.mk(pf, 'Adobe\\Adobe Creative Cloud'), 200, 1.1 * GB, dll(), [40, 120]);
  b.spread(b.mk(pf, 'Blender Foundation\\Blender 4.2'), 520, 1.4 * GB, (i, rr) => (i % 3 === 0 ? `${rr.pick(['bpy', 'mathutils', 'cycles', 'io_scene', 'node', 'rigify', 'bl_ui'])}_${i}.py` : dll(['dll', 'pyd', 'py', 'blend', 'exe'])(i, rr)), [60, 90]);
  b.spread(b.mk(pf, 'Docker\\Docker'), 220, 3.1 * GB, dll(['dll', 'exe', 'tar', 'iso', 'vhdx']), [10, 40], 1.5);
  b.spread(b.mk(pf, 'Epic Games\\Fortnite\\FortniteGame\\Content\\Paks'), 54, 34 * GB, (i) => `pakchunk${i}-WindowsClient.${['pak', 'ucas', 'utoc'][i % 3]}`, [3, 20], 1.4);
  b.spread(b.mk(pf, 'Git'), 420, 410 * MB, dll(['exe', 'dll', 'sh', 'pl', 'txt']), [100, 140]);
  b.spread(b.mk(pf, 'Google\\Chrome\\Application\\129.0.6668.90'), 150, 610 * MB, dll(['dll', 'pak', 'exe', 'bin', 'dat']), [5, 12]);
  b.spread(b.mk(pf, 'Microsoft Office\\root\\Office16'), 480, 4.4 * GB, dll(['dll', 'exe', 'dll', 'olb', 'xll']), [20, 60]);
  b.spread(b.mk(pf, 'Microsoft Visual Studio\\2022\\Community\\Common7\\IDE'), 900, 8.4 * GB, dll(['dll', 'dll', 'exe', 'pkgdef', 'json', 'xml']), [30, 90]);
  b.spread(b.mk(pf, 'Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.41.34120'), 320, 2.8 * GB, (i, rr) => `${rr.pick(['lib', 'include', 'bin'])}_${rr.pick(DLL_B)}${i}.${rr.pick(['lib', 'h', 'exe', 'dll', 'pdb'])}`, [30, 90]);
  b.spread(b.mk(pf, 'Mozilla Firefox'), 110, 270 * MB, dll(['dll', 'exe', 'ja', 'json']), [3, 20]);
  b.folders(b.mk(pf, 'NVIDIA Corporation'), ['NVIDIA app', 'Display.NvContainer', 'PhysX', 'NvBackend', 'Installer2', 'FrameViewSDK'], [30, 90], 2.6 * GB, dll(), [10, 60]);
  b.spread(b.mk(pf, 'nodejs\\node_modules\\npm'), 360, 96 * MB, (i, rr) => `${rr.pick(PKGS).replace('@', '')}_${i}.${rr.pick(['js', 'json', 'md'])}`, [40, 60]);
  b.file(b.mk(pf, 'nodejs'), 'node.exe', 78 * MB, [40, 60]);
  b.spread(b.mk(pf, 'obs-studio\\bin\\64bit'), 160, 300 * MB, dll(['dll', 'exe']), [30, 60]);
  b.spread(b.mk(pf, 'obs-studio\\obs-plugins\\64bit'), 70, 180 * MB, dll(['dll']), [30, 60]);
  b.spread(b.mk(pf, 'PowerShell\\7'), 260, 210 * MB, dll(['dll', 'psd1', 'psm1', 'exe']), [30, 60]);
  b.spread(b.mk(pf, '7-Zip'), 14, 5.7 * MB, fromList(['7z', '7zFM', '7zG', '7-zip', '7-zip32', 'Uninstall', 'descript', 'History', 'License', 'readme', '7zCon', 'Lang', 'Codecs', 'Formats'], ['exe', 'dll', 'txt']), [200, 400]);
  b.spread(b.mk(pf, 'VideoLAN\\VLC'), 260, 180 * MB, dll(['dll', 'exe', 'lua']), [100, 300]);
  b.spread(b.mk(pf, 'Common Files'), 180, 1.4 * GB, dll(), [60, 400]);
  b.spread(b.mk(pf, 'Windows Defender'), 60, 120 * MB, dll(['dll', 'exe', 'vdm']), [1, 10]);
  const wapps = b.mk(pf, 'WindowsApps', FLAG_HIDDEN);
  const storeApps = ['Microsoft.WindowsCalculator_11.2405.2.0_x64__8wekyb3d8bbwe', 'Microsoft.Windows.Photos_2024.11070.15005.0_x64__8wekyb3d8bbwe', 'Microsoft.WindowsTerminal_1.21.2361.0_x64__8wekyb3d8bbwe', 'SpotifyAB.SpotifyMusic_1.247.462.0_x64__zpdnekdrzrea0', 'Microsoft.MinecraftUWP_1.21.3101.0_x64__8wekyb3d8bbwe', 'Microsoft.XboxGamingOverlay_7.324.9122.0_x64__8wekyb3d8bbwe', 'Microsoft.GamingApp_2409.1001.5.0_x64__8wekyb3d8bbwe', 'Microsoft.WindowsNotepad_11.2408.12.0_x64__8wekyb3d8bbwe', 'Microsoft.Paint_11.2408.30.0_x64__8wekyb3d8bbwe', 'Microsoft.ScreenSketch_11.2408.13.0_x64__8wekyb3d8bbwe', 'Microsoft.Todos_2.114.7122.0_x64__8wekyb3d8bbwe', 'Microsoft.WindowsStore_22409.1401.5.0_x64__8wekyb3d8bbwe', 'Microsoft.YourPhone_1.24082.142.0_x64__8wekyb3d8bbwe', 'Microsoft.ZuneMusic_11.2408.8.0_x64__8wekyb3d8bbwe', 'Microsoft.Clipchamp_3.1.11920.0_x64__yxz26nhyzhsrt', 'Microsoft.OutlookForWindows_1.2024.925.300_x64__8wekyb3d8bbwe', 'Microsoft.WindowsCamera_2024.2408.1.0_x64__8wekyb3d8bbwe', 'Microsoft.VP9VideoExtensions_1.2.1.0_x64__8wekyb3d8bbwe', 'Microsoft.HEIFImageExtension_1.2.3.0_x64__8wekyb3d8bbwe', 'Microsoft.DesktopAppInstaller_1.23.1911.0_x64__8wekyb3d8bbwe', 'Microsoft.Copilot_1.24092.202.0_x64__8wekyb3d8bbwe', 'Microsoft.PowerAutomateDesktop_11.2409.226.0_x64__8wekyb3d8bbwe'];
  b.folders(wapps, storeApps, [8, 26], 19 * GB, dll(['dll', 'exe', 'pri', 'xml', 'png', 'winmd']), [3, 90], FLAG_HIDDEN);

  // Program Files (x86)
  const pf86 = b.mk(root, 'Program Files (x86)', 0, [60, 500]);
  const steam = b.mk(pf86, 'Steam', 0, [1, 30]);
  b.file(steam, 'steam.exe', 4.1 * MB, [1, 5]);
  b.spread(b.mk(steam, 'bin'), 220, 420 * MB, dll(), [1, 20]);
  b.spread(b.mk(steam, 'package'), 60, 610 * MB, (i) => `steam_client_win64_${i}.zip.vz`, [1, 5]);
  b.spread(b.mk(steam, 'steamapps\\common\\Steamworks Shared'), 30, 900 * MB, (i) => `_CommonRedist_${i}.exe`, [100, 400]);
  b.spread(b.mk(steam, 'steamapps\\common\\Wallpaper Engine'), 140, 1.1 * GB, dll(['dll', 'exe', 'pkg', 'json']), [20, 90]);
  b.spread(b.mk(steam, 'steamapps\\common\\Hollow Knight'), 120, 8.9 * GB, (i) => `hollow_knight_Data_${['resources', 'sharedassets', 'level'][i % 3]}${i}.assets`, [300, 600]);
  b.spread(b.mk(steam, 'dumps'), 6, 160 * MB, (i) => `crash_steam_${20260800 + i}.dmp`, [5, 60]);
  b.spread(b.mk(steam, 'logs'), 40, 30 * MB, (i) => `${['content_log', 'connection_log', 'bootstrap_log', 'shader_log', 'webhelper'][i % 5]}_${i}.txt`, [0, 20]);
  b.spread(b.mk(pf86, 'Microsoft\\Edge\\Application\\129.0.2792.65'), 170, 620 * MB, dll(['dll', 'pak', 'exe']), [5, 15]);
  b.spread(b.mk(pf86, 'Windows Kits\\10'), 700, 3.1 * GB, (i, rr) => `${rr.pick(['um', 'shared', 'ucrt', 'winrt', 'km'])}_${rr.pick(DLL_B)}${i}.${rr.pick(['h', 'lib', 'idl', 'winmd', 'exe'])}`, [100, 300]);
  b.spread(b.mk(pf86, 'Microsoft Visual Studio\\Installer'), 200, 280 * MB, dll(), [20, 60]);
  b.spread(b.mk(pf86, 'Battle.net'), 120, 460 * MB, dll(), [10, 60]);

  // ProgramData
  const pd = b.mk(root, 'ProgramData', FLAG_HIDDEN, [60, 500]);
  const wer = b.mk(pd, 'Microsoft\\Windows\\WER');
  for (let i = 0; i < 20; i++) {
    const d = b.mk(wer, `ReportArchive\\AppCrash_${r.pick(['Cyberpunk2077.ex', 'obs64.exe', 'Discord.exe', 'explorer.exe', 'msedge.exe', 'steamwebhelper'])}_${r.hex(40)}_${r.hex(8)}_${r.hex(8)}`, 0, [10, 200]);
    b.spread(d, 3, r.range(4, 90) * MB, (k) => ['Report.wer', `memory.hdmp`, 'WERInternalMetadata.xml'][k], [10, 200]);
  }
  b.folders(b.mk(wer, 'ReportQueue'), [`Critical_${r.hex(40)}`, `NonCritical_x64_${r.hex(40)}`, `Kernel_141_${r.hex(40)}`], [2, 4], 210 * MB, (k) => ['Report.wer', 'memory.hdmp', 'minidump.mdmp', 'WERInternalMetadata.xml'][k % 4], [2, 30]);
  b.spread(b.mk(pd, 'Microsoft\\Windows Defender\\Definition Updates\\Backup'), 12, 1.8 * GB, (i) => `mpasbase${i}.vdm`, [1, 10]);
  b.spread(b.mk(pd, 'Microsoft\\Search\\Data\\Applications\\Windows'), 20, 1.4 * GB, (i) => `Windows${i}.edb`, [0, 10]);
  b.folders(b.mk(pd, 'Package Cache'), Array.from({ length: 60 }, () => GUID(r)), [1, 3], 6.2 * GB, (_, rr) => `${rr.pick(['vc_runtime', 'dotnet', 'windowsdesktop', 'vs_setup', 'sdk'])}_x64.${rr.pick(['msi', 'cab', 'exe'])}`, [30, 600]);
  b.spread(b.mk(pd, 'NVIDIA Corporation\\Downloader'), 8, 1.3 * GB, (i) => `${['560.94', '560.81', '556.12', '555.85'][i % 4]}-desktop-win10-win11-64bit-international-dch-whql_${i}.exe`, [20, 200]);
  b.spread(b.mk(pd, 'chocolatey\\lib'), 260, 650 * MB, (i, rr) => `${rr.pick(['git', 'nodejs', 'python', '7zip', 'ffmpeg', 'yt-dlp', 'curl'])}_${i}.${rr.pick(['nupkg', 'exe', 'ps1', 'zip'])}`, [30, 300]);

  // System folders
  b.file(b.mk(root, 'Recovery\\WindowsRE', HS), 'Winre.wim', 680 * MB, [100, 120], HS);
  const svi = b.mk(root, 'System Volume Information', HS, [0, 5]);
  for (let i = 0; i < 4; i++) b.file(svi, `{${r.hex(8)}-${r.hex(4)}-${r.hex(4)}-${r.hex(4)}-${r.hex(12)}}{3808876b-c176-4e48-b7ae-04046e6cc752}`, r.range(1.8, 2.9) * GB, [i * 7, i * 7 + 2], HS);
  b.spread(svi, 10, 30 * MB, (i) => (i === 0 ? 'tracking.log' : `IndexerVolumeGuid_${i}`), [0, 30], 1, HS);
  b.spread(b.mk(root, 'Intel\\Logs'), 8, 3 * MB, (i) => `IntelGFX_${i}.log`, [100, 400]);
  b.mk(root, 'PerfLogs', 0, [700, 800]);

  // Windows
  const win = b.mk(root, 'Windows', 0, [0, 30]);
  ['explorer.exe:5.4', 'notepad.exe:0.36', 'regedit.exe:0.36', 'HelpPane.exe:1.1', 'bfsvc.exe:0.11', 'splwow64.exe:0.16', 'hh.exe:0.02', 'win.ini:0.0001', 'system.ini:0.0001', 'WindowsUpdate.log:0.27'].forEach((s) => {
    const [n, m] = s.split(':');
    b.file(win, n, Number(m) * MB, [5, 200]);
  });
  const s32 = b.mk(win, 'System32', 0, [0, 30]);
  b.spread(s32, 1300, 7.4 * GB, dll(['dll', 'dll', 'dll', 'exe', 'mui', 'cpl', 'msc', 'dat', 'tlb']), [5, 400]);
  b.folders(b.mk(s32, 'DriverStore\\FileRepository'), Array.from({ length: 160 }, (_, i) => `${r.pick(['nv_dispig', 'iaahcic', 'netrtwlane', 'hdaudio', 'usbxhci', 'rt640x64', 'intcaudiobus', 'bthleenum', 'wvhdmi', 'machine'])}.inf_amd64_${r.hex(16)}${i}`), [3, 9], 4.8 * GB, dll(['sys', 'dll', 'cat', 'inf', 'exe']), [5, 300]);
  b.spread(b.mk(s32, 'drivers'), 300, 180 * MB, dll(['sys']), [5, 300]);
  b.spread(b.mk(s32, 'config', HS), 12, 620 * MB, names(['SYSTEM', 'SOFTWARE', 'SAM', 'SECURITY', 'DEFAULT', 'COMPONENTS', 'DRIVERS', 'BBI', 'ELAM', 'SOFTWARE.LOG1', 'SYSTEM.LOG1', 'COMPONENTS.LOG1']), [0, 2], 1.3, HS);
  b.spread(b.mk(s32, 'LogFiles\\WMI'), 60, 140 * MB, (i) => `LwtNetLog_${i}.etl`, [0, 30]);
  b.spread(b.mk(win, 'SysWOW64'), 800, 2.1 * GB, dll(), [5, 400]);
  const sxs = b.mk(win, 'WinSxS', 0, [5, 300]);
  b.folders(sxs, Array.from({ length: 360 }, (_, i) => `amd64_microsoft-windows-${r.pick(DLL_B)}-${r.pick(DLL_B)}_31bf3856ad364e35_10.0.26100.${r.int(1, 2033)}_none_${r.hex(16)}${i % 10}`), [1, 4], 9.6 * GB, dll(['dll', 'exe', 'mui', 'manifest']), [5, 400]);
  b.spread(b.mk(sxs, 'ManifestCache'), 20, 380 * MB, hexName(16, 'bin'), [0, 30]);
  b.folders(b.mk(win, 'SoftwareDistribution\\Download'), Array.from({ length: 24 }, () => r.hex(32)), [1, 5], 3.6 * GB, (_, rr) => `windows11.0-kb50${rr.int(40000, 44999)}-x64_${rr.hex(40)}.${rr.pick(['cab', 'psf', 'msu', 'esd'])}`, [3, 70]);
  b.file(b.mk(win, 'SoftwareDistribution\\DataStore'), 'DataStore.edb', 420 * MB, 0);
  b.spread(b.mk(win, 'Temp'), 140, 1.2 * GB, (i, rr) => `${rr.pick(['MpCmdRun', 'TrustedInstaller', 'DISM', 'msi', 'CBS', 'WinSAT'])}_${rr.hex(6)}${i}.${rr.pick(['log', 'tmp', 'cab', 'etl', 'dmp'])}`, [0, 90]);
  b.spread(b.mk(win, 'Installer', FLAG_HIDDEN), 220, 6.8 * GB, (_, rr) => `${rr.hex(5)}.${rr.pick(['msi', 'msi', 'msp'])}`, [30, 900], 1.2, FLAG_HIDDEN);
  b.spread(b.mk(win, 'Fonts'), 300, 640 * MB, (i, rr) => `${rr.pick(['segoe', 'arial', 'calibri', 'consola', 'cambria', 'times', 'verdana', 'tahoma', 'georgia', 'malgun', 'msyh', 'yugoth', 'seguiemj', 'CascadiaCode', 'Inter'])}${rr.pick(['', 'b', 'i', 'z', 'l', 'sb', 'bd'])}${i > 100 ? i : ''}.${rr.pick(['ttf', 'ttf', 'otf', 'ttc'])}`, [100, 800]);
  b.spread(b.mk(win, 'Minidump'), 5, 7.4 * MB, (i) => `0${9 - i}2${i}26-${12000 + i * 731}-01.dmp`, [12, 200]);
  b.spread(b.mk(win, 'LiveKernelReports\\WATCHDOG'), 2, 1.2 * GB, (i) => `WATCHDOG-20260${8 + i}1${i}-2214.dmp`, [20, 60]);
  b.file(win, 'MEMORY.DMP', 2.6 * GB, 12);
  b.spread(b.mk(win, 'assembly\\NativeImages_v4.0.30319_64'), 380, 1.9 * GB, (i, rr) => `System.${rr.pick(DLL_B)}.${rr.pick(['ni', 'resources'])}${i}.dll`, [30, 400]);
  b.spread(b.mk(win, 'Microsoft.NET\\Framework64\\v4.0.30319'), 420, 1.1 * GB, dll(), [30, 400]);
  b.spread(b.mk(win, 'Logs\\CBS'), 60, 520 * MB, (i) => (i === 0 ? 'CBS.log' : `CbsPersist_2026${String(900 + i).slice(-3)}.cab`), [0, 120], 1.5);
  b.spread(b.mk(win, 'Prefetch'), 200, 48 * MB, (_, rr) => `${rr.pick(['CHROME', 'DISCORD', 'STEAM', 'CODE', 'OBS64', 'EXPLORER', 'SVCHOST', 'MSEDGE', 'SPOTIFY', 'CYBERPUNK2077'])}.EXE-${rr.hex(8).toUpperCase()}.pf`, [0, 60]);
  b.spread(b.mk(win, 'ServiceProfiles\\NetworkService\\AppData\\Local\\Microsoft\\Windows\\DeliveryOptimization\\Cache'), 50, 2.3 * GB, hexName(40), [1, 40]);
  b.folders(b.mk(win, 'SystemApps'), ['Microsoft.Windows.StartMenuExperienceHost_cw5n1h2txyewy', 'MicrosoftWindows.Client.CBS_cw5n1h2txyewy', 'Microsoft.Windows.Search_cw5n1h2txyewy', 'ShellExperienceHost_cw5n1h2txyewy', 'Microsoft.LockApp_cw5n1h2txyewy'], [10, 30], 900 * MB, dll(['dll', 'exe', 'pri', 'winmd']), [10, 60]);
  b.spread(b.mk(win, 'servicing\\Packages'), 120, 420 * MB, (i) => `Package_for_RollupFix~31bf3856ad364e35~amd64~~26100.${1700 + i}.1.${['mum', 'cat'][i % 2]}`, [10, 200]);
  b.spread(b.mk(win, 'Web\\Wallpaper\\Windows'), 6, 60 * MB, (i) => `img${i}.jpg`, [200, 400]);

  // Windows.old
  const wold = b.mk(root, 'Windows.old', 0, [120, 125]);
  b.spread(b.mk(wold, 'Windows\\System32'), 300, 6.2 * GB, dll(), [120, 900]);
  b.spread(b.mk(wold, 'Windows\\WinSxS'), 200, 7.6 * GB, dll(['dll', 'manifest']), [120, 900]);
  b.spread(b.mk(wold, 'Program Files'), 160, 4.1 * GB, dll(), [120, 900]);
  b.spread(b.mk(wold, 'Users\\Alex\\AppData'), 140, 2.4 * GB, dll(['dat', 'db', 'log', 'tmp']), [120, 900]);

  // Users
  const users = b.mk(root, 'Users', 0, [0, 30]);
  b.spread(b.mk(users, 'Default', FLAG_HIDDEN), 40, 12 * MB, (i) => (i === 0 ? 'NTUSER.DAT' : `ntuser_${i}.ini`), [300, 600], 1, FLAG_HIDDEN);
  b.spread(b.mk(users, 'Public\\Desktop'), 6, 20 * KB, fromList(['Steam', 'Discord', 'OBS Studio', 'Blender 4.2', 'Battle.net', 'VLC media player'], ['lnk']), [10, 300]);
  const alex = b.mk(users, 'Alex', 0, [0, 5]);
  b.file(alex, 'NTUSER.DAT', 18 * MB, 0, HS);
  b.file(alex, 'ntuser.dat.LOG1', 2.1 * MB, 0, HS);
  b.spread(b.mk(alex, '.cargo\\registry\\cache\\index.crates.io-6f17d22bba15001f'), 340, 1.6 * GB, (i, rr) => `${rr.pick(['tokio', 'serde', 'windows', 'rustls', 'axum', 'hyper', 'regex', 'rayon', 'image', 'tauri', 'wry', 'syn', 'quote', 'clap', 'anyhow'])}-${rr.int(0, 2)}.${rr.int(0, 40)}.${i}.crate`, [5, 200]);
  b.spread(b.mk(alex, '.gradle\\caches\\modules-2\\files-2.1'), 160, 2.2 * GB, (i, rr) => `${rr.pick(['kotlin-stdlib', 'okhttp', 'androidx.core', 'gson', 'guava', 'compose-ui'])}-${rr.int(1, 9)}.${i}.jar`, [60, 400]);
  b.folders(b.mk(alex, '.vscode\\extensions'), ['rust-lang.rust-analyzer-0.3.2129-win32-x64', 'ms-python.python-2024.14.1', 'esbenp.prettier-vscode-11.0.0', 'dbaeumer.vscode-eslint-3.0.10', 'tauri-apps.tauri-vscode-0.2.9', 'bradlc.vscode-tailwindcss-0.12.11', 'github.copilot-1.234.0', 'ms-vscode.cpptools-1.21.6-win32-x64', 'anthropic.claude-code-2.0.12'], [6, 18], 1.1 * GB, (i, rr) => `${rr.pick(['extension', 'server', 'main', 'bundle', 'worker'])}${i}.${rr.pick(['js', 'json', 'exe', 'wasm', 'node'])}`, [5, 40]);
  b.spread(b.mk(alex, '.ollama\\models\\blobs'), 5, 17.2 * GB, (_, rr) => `sha256-${rr.hex(64)}`, [20, 90], 0.9);

  // AppData
  const local = b.mk(alex, 'AppData\\Local', 0, [0, 2]);
  t.node(b.mk(alex, 'AppData')).flags = FLAG_HIDDEN;
  const temp = b.mk(local, 'Temp', 0, [0, 1]);
  b.spread(temp, 380, 4.4 * GB, (i, rr) => `${rr.pick(['tmp', '~DF', 'chrome_', 'msedge_', 'Discord_', 'vs_setup_', 'nsis', 'wct', 'pip-', 'rust_'])}${rr.hex(6).toUpperCase()}${i}.${rr.pick(['tmp', 'tmp', 'log', 'etl', 'cab', 'msi', 'zip', 'dmp', 'json'])}`, [0, 140]);
  b.folders(temp, Array.from({ length: 10 }, () => GUID(r)), [3, 20], 1.1 * GB, (i, rr) => `${rr.pick(['setup', 'payload', 'cab', 'data'])}${i}.${rr.pick(['cab', 'msi', 'exe', 'dat'])}`, [2, 60]);
  const chrome = b.mk(local, 'Google\\Chrome\\User Data', 0, [0, 1]);
  b.spread(b.mk(chrome, 'Default\\Cache\\Cache_Data'), 1100, 1.5 * GB, (i) => (i < 4 ? `data_${i}` : i === 4 ? 'index' : `f_${(0x1a00 + i).toString(16).padStart(6, '0')}`), [0, 25]);
  b.spread(b.mk(chrome, 'Default\\Code Cache\\js'), 400, 410 * MB, (_, rr) => `${rr.hex(16)}_0`, [0, 25]);
  b.spread(b.mk(chrome, 'Default\\GPUCache'), 6, 52 * MB, (i) => ['data_0', 'data_1', 'data_2', 'data_3', 'index', 'f_000001'][i], [0, 3]);
  b.spread(b.mk(chrome, 'Profile 1\\Cache\\Cache_Data'), 500, 680 * MB, (i) => `f_${(0x2b00 + i).toString(16).padStart(6, '0')}`, [0, 40]);
  b.spread(b.mk(chrome, 'Default\\Extensions'), 220, 210 * MB, (i, rr) => `${rr.hex(12)}_${i}.${rr.pick(['js', 'json', 'png', 'css'])}`, [5, 90]);
  b.spread(b.mk(chrome, 'Default'), 8, 160 * MB, names(['History', 'Favicons', 'Web Data', 'Login Data', 'Cookies', 'Top Sites', 'Shortcuts', 'Visited Links']), [0, 1]);
  b.spread(b.mk(local, 'Microsoft\\Edge\\User Data\\Default\\Cache\\Cache_Data'), 420, 590 * MB, (i) => `f_${(0x0900 + i).toString(16).padStart(6, '0')}`, [0, 30]);
  b.spread(b.mk(local, 'Microsoft\\Edge\\User Data\\Default\\Code Cache\\js'), 160, 140 * MB, (_, rr) => `${rr.hex(16)}_0`, [0, 30]);
  b.spread(b.mk(local, 'Mozilla\\Firefox\\Profiles\\x8k2v1qp.default-release\\cache2\\entries'), 600, 760 * MB, (_, rr) => rr.hex(40).toUpperCase(), [0, 30]);
  b.folders(b.mk(local, 'D3DSCache'), Array.from({ length: 30 }, () => r.hex(16)), [2, 5], 1.7 * GB, (i) => ['index.idx', 'data.val', 'tmp.lock', 'meta.bin', 'shader.bin'][i % 5], [0, 60]);
  b.spread(b.mk(local, 'NVIDIA\\DXCache'), 220, 2.4 * GB, (_, rr) => `${rr.hex(16)}_${rr.hex(8)}.${rr.pick(['nvph', 'toc'])}`, [0, 45]);
  b.folders(b.mk(local, 'NVIDIA\\GLCache'), Array.from({ length: 24 }, () => r.hex(32)), [3, 7], 850 * MB, (_, rr) => `${rr.hex(16)}.${rr.pick(['bin', 'toc'])}`, [0, 45]);
  const dumps = b.mk(local, 'CrashDumps', 0, [3, 40]);
  b.file(dumps, 'Discord.exe.18244.dmp', 612 * MB, 9);
  b.file(dumps, 'Cyberpunk2077.exe.20560.dmp', 2.1 * GB, 23);
  b.file(dumps, 'obs64.exe.9932.dmp', 410 * MB, 41);
  b.file(dumps, 'explorer.exe.7712.dmp', 96 * MB, 3);
  b.file(dumps, 'Code.exe.15388.dmp', 380 * MB, 17);
  b.folders(b.mk(local, 'Microsoft\\Windows\\WER\\ReportArchive'), Array.from({ length: 12 }, () => `AppHang_${r.pick(['steamwebhelper', 'Discord.exe', 'msedge.exe', 'Spotify.exe'])}_${r.hex(40)}`), [2, 3], 240 * MB, (k) => ['Report.wer', 'memory.hdmp', 'WERInternalMetadata.xml'][k % 3], [5, 120]);
  const expl = b.mk(local, 'Microsoft\\Windows\\Explorer', 0, [0, 1]);
  [['thumbcache_16.db', 4], ['thumbcache_32.db', 22], ['thumbcache_48.db', 18], ['thumbcache_96.db', 64], ['thumbcache_256.db', 410], ['thumbcache_1280.db', 820], ['thumbcache_idx.db', 1.2], ['iconcache_16.db', 3], ['iconcache_32.db', 6], ['iconcache_256.db', 41]].forEach(([n, s]) => b.file(expl, String(n), Number(s) * MB, [0, 3]));
  b.spread(b.mk(local, 'Microsoft\\Windows\\INetCache\\IE'), 240, 240 * MB, (_, rr) => `${rr.hex(8).toUpperCase()}.${rr.pick(['htm', 'js', 'png', 'css'])}`, [0, 90]);
  b.spread(b.mk(local, 'Microsoft\\OneDrive'), 200, 380 * MB, dll(['dll', 'exe', 'dat']), [5, 30]);
  const npm = b.mk(local, 'npm-cache\\_cacache\\content-v2\\sha512', 0, [1, 120]);
  for (let i = 0; i < 96; i++) {
    const d = b.mk(npm, `${r.hex(2)}${i}`.slice(0, 2) + (i >= 64 ? r.hex(1) : ''), 0, [1, 120]);
    b.spread(b.mk(d, r.hex(2)), r.int(4, 14), r.range(5, 40) * MB, () => r.hex(124), [1, 120]);
  }
  b.spread(b.mk(local, 'npm-cache\\_cacache\\index-v5'), 120, 14 * MB, (_, rr) => rr.hex(56), [1, 120]);
  b.spread(b.mk(local, 'pip\\cache\\http-v2'), 220, 1.1 * GB, (_, rr) => `${rr.hex(56)}.body`, [5, 200]);
  b.spread(b.mk(local, 'Yarn\\Cache\\v6'), 300, 700 * MB, (i, rr) => `npm-${rr.pick(PKGS).replace('@', '')}-${rr.int(1, 9)}.${i}.0-${rr.hex(40)}-integrity`, [10, 300]);
  b.spread(b.mk(local, 'go-build'), 160, 900 * MB, (_, rr) => `${rr.hex(64)}-${rr.pick(['a', 'd'])}`, [10, 200]);
  b.spread(b.mk(local, 'Discord\\app-1.0.9163'), 110, 420 * MB, dll(['dll', 'exe', 'pak', 'node', 'asar']), [3, 10]);
  b.spread(b.mk(local, 'Discord\\packages'), 2, 180 * MB, (i) => `Discord-1.0.916${2 + i}-full.nupkg`, [3, 20]);
  const docker = b.mk(local, 'Docker\\wsl', 0, [0, 1]);
  b.file(b.mk(docker, 'disk'), 'docker_data.vhdx', 46.4 * GB, 0);
  b.file(b.mk(docker, 'main'), 'ext4.vhdx', 1.3 * GB, 0);
  const pkgs = b.mk(local, 'Packages', 0, [0, 30]);
  b.spread(b.mk(pkgs, 'SpotifyAB.SpotifyMusic_zpdnekdrzrea0\\LocalCache\\Spotify\\Data'), 420, 5.6 * GB, (_, rr) => `${rr.hex(2)}\\${rr.hex(32)}.file`.replace('\\', '_'), [0, 60]);
  b.spread(b.mk(pkgs, 'Microsoft.MinecraftUWP_8wekyb3d8bbwe\\LocalState\\games\\com.mojang\\minecraftWorlds'), 60, 2.1 * GB, (i) => `world_${i}.ldb`, [10, 200]);
  b.folders(pkgs, ['Microsoft.WindowsTerminal_8wekyb3d8bbwe', 'Microsoft.Windows.Photos_8wekyb3d8bbwe', 'Microsoft.WindowsCalculator_8wekyb3d8bbwe', 'Microsoft.XboxGamingOverlay_8wekyb3d8bbwe', 'Microsoft.YourPhone_8wekyb3d8bbwe', 'Microsoft.Todos_8wekyb3d8bbwe', 'Microsoft.WindowsNotepad_8wekyb3d8bbwe', 'Microsoft.ScreenSketch_8wekyb3d8bbwe', 'Microsoft.Copilot_8wekyb3d8bbwe', 'Microsoft.OutlookForWindows_8wekyb3d8bbwe'], [4, 14], 1.4 * GB, (i, rr) => `${rr.pick(['settings', 'state', 'cache', 'thumbs', 'db'])}${i}.${rr.pick(['dat', 'db', 'json', 'bin'])}`, [0, 60]);
  b.spread(b.mk(local, 'Programs\\Microsoft VS Code'), 260, 440 * MB, dll(['dll', 'exe', 'pak', 'js', 'node', 'asar']), [3, 20]);
  b.spread(b.mk(local, 'Programs\\Python\\Python312\\Lib'), 600, 190 * MB, (i, rr) => `${rr.pick(['asyncio', 'json', 'email', 'http', 'urllib', 'xml', 'sqlite3', 'tkinter', 'unittest', 'importlib'])}_${i}.${rr.pick(['py', 'pyc', 'pyd'])}`, [100, 300]);
  b.spread(b.mk(local, 'JetBrains\\Toolbox\\cache'), 120, 820 * MB, hexName(24, 'cache'), [5, 90]);
  b.spread(b.mk(local, 'EpicGamesLauncher\\Saved\\webcache_4430'), 200, 410 * MB, (i) => `f_${(0x400 + i).toString(16).padStart(6, '0')}`, [0, 60]);
  b.spread(b.mk(local, 'Unity\\cache\\packages'), 90, 1.2 * GB, (i, rr) => `com.unity.${rr.pick(['render-pipelines', 'textmeshpro', 'inputsystem', 'cinemachine'])}-${i}.tgz`, [30, 300]);
  const roaming = b.mk(alex, 'AppData\\Roaming', 0, [0, 2]);
  b.spread(b.mk(roaming, 'Code\\User\\workspaceStorage'), 220, 520 * MB, (_, rr) => `${rr.hex(32)}_state.vscdb`, [0, 120]);
  b.spread(b.mk(roaming, 'Code\\Cache\\Cache_Data'), 240, 230 * MB, (i) => `f_${(0x300 + i).toString(16).padStart(6, '0')}`, [0, 30]);
  b.spread(b.mk(roaming, 'Code\\CachedData'), 60, 400 * MB, hexName(40, 'code'), [0, 60]);
  b.spread(b.mk(roaming, 'discord\\Cache\\Cache_Data'), 600, 640 * MB, (i) => `f_${(0x700 + i).toString(16).padStart(6, '0')}`, [0, 20]);
  b.spread(b.mk(roaming, 'discord\\Code Cache\\js'), 120, 120 * MB, (_, rr) => `${rr.hex(16)}_0`, [0, 20]);
  b.spread(b.mk(roaming, 'Spotify'), 30, 120 * MB, dll(['dll', 'exe', 'spa']), [5, 30]);
  b.spread(b.mk(roaming, 'obs-studio\\logs'), 50, 12 * MB, (i) => `2026-09-${String((i % 28) + 1).padStart(2, '0')} 2${i % 4}-1${i % 6}-0${i % 9}.txt`, [0, 60]);
  b.spread(b.mk(roaming, 'Microsoft\\Windows\\Recent'), 220, 300 * KB, (i, rr) => `${rr.pick(['report', 'IMG', 'notes', 'budget', 'trailer', 'design', 'invoice'])}_${i}.lnk`, [0, 120]);
  b.spread(b.mk(roaming, 'Adobe\\Common\\Media Cache Files'), 180, 2.4 * GB, (i) => `trailer_v${i}.cfa`, [5, 120], 1.3);
  b.spread(b.mk(alex, 'AppData\\LocalLow\\Unity'), 30, 200 * MB, (i) => `Player_${i}.log`, [5, 200]);

  // Desktop
  const desktop = b.mk(alex, 'Desktop', 0, [0, 3]);
  [['Steam.lnk', 2 * KB, 200], ['Discord.lnk', 2 * KB, 180], ['todo.txt', 3 * KB, 1], ['Screenshot 2026-09-12 203311.png', 3.4 * MB, 21], ['budget-2026.xlsx', 88 * KB, 5], ['Project Phoenix.pptx', 24 * MB, 9], ['ubuntu-24.04.1-desktop-amd64.iso', 5.8 * GB, 214], ['render_final_v3.mp4', 1.1 * GB, 33]].forEach(([n, s, a]) => b.file(desktop, String(n), Number(s), Number(a)));

  // Documents
  const docs = b.mk(alex, 'Documents', 0, [0, 3]);
  const claude = b.mk(docs, 'Claude Ideas', 0, [0, 1]);
  [['INDEX.md', 3 * KB, 0], ['2026-09-21-storage-treemap-zoom.md', 4 * KB, 12], ['2026-09-28-phone-companion-onboarding.md', 6 * KB, 5], ['2026-09-29-vault-passkeys-plan.md', 5 * KB, 4], ['REPLY-storage-treemap-zoom.md', 9 * KB, 11], ['claude-notes.md', 2 * KB, 2]].forEach(([n, s, a]) => b.file(claude, String(n), Number(s), Number(a)));
  const omni = b.mk(docs, 'Projects\\omnihub', 0, [0, 1]);
  nodeModules(b, omni, 90, 420 * MB);
  b.spread(b.mk(omni, 'src\\desktop'), 70, 1.6 * MB, (i, rr) => `${rr.pick(['App', 'Sidebar', 'Treemap', 'Storage', 'Vault', 'Notes', 'Phone', 'Settings', 'api', 'mock', 'router', 'format'])}${i}.${rr.pick(['tsx', 'ts', 'css'])}`, [0, 10]);
  b.spread(b.mk(omni, 'crates\\omnihub-core\\src'), 40, 900 * KB, (i, rr) => `${rr.pick(['tree', 'engine', 'mft', 'walk', 'vault', 'notes', 'remote', 'auth'])}${i}.rs`, [0, 10]);
  b.spread(b.mk(omni, 'target\\debug\\deps'), 700, 6.8 * GB, (i, rr) => `${rr.pick(['libtokio', 'libserde', 'libwindows', 'librustls', 'libaxum', 'libtauri', 'libwry', 'omnihub_core'])}-${rr.hex(16)}${i}.${rr.pick(['rlib', 'rmeta', 'pdb', 'd', 'dll'])}`, [0, 20], 1.3);
  b.spread(b.mk(omni, 'target\\release'), 180, 2.1 * GB, (i, rr) => `${rr.pick(['omnihub', 'build', 'deps', 'incremental'])}-${rr.hex(12)}${i}.${rr.pick(['rlib', 'exe', 'pdb', 'o'])}`, [0, 20], 1.3);
  const site = b.mk(docs, 'Projects\\portfolio-site', 0, [10, 60]);
  nodeModules(b, site, 50, 260 * MB);
  b.spread(b.mk(site, '.next\\cache'), 160, 410 * MB, hexName(20, 'pack'), [10, 60]);
  b.spread(b.mk(docs, "My Games\\Baldur's Gate 3\\PlayerProfiles\\Public\\Savegames\\Story"), 48, 1.1 * GB, (i) => `Alex-${1000 + i}__QuickSave_${i}.lsv`, [0, 60], 0.3);
  b.spread(b.mk(docs, 'My Games\\Cyberpunk 2077'), 30, 260 * MB, (i) => `ManualSave-${i}.dat`, [20, 200], 0.3);
  b.spread(b.mk(docs, 'Work'), 180, 1.4 * GB, (i, rr) => `${rr.pick(['Q3 Review', 'Roadmap', 'Invoice', 'Contract', 'Meeting notes', 'Budget', 'Design spec', 'Pitch deck', 'Onboarding'])} ${2024 + (i % 3)}-${String((i % 12) + 1).padStart(2, '0')}.${rr.pick(['docx', 'xlsx', 'pptx', 'pdf', 'pdf'])}`, [3, 700]);
  b.spread(b.mk(docs, 'Taxes'), 30, 120 * MB, (i) => `${2019 + (i % 7)} ${['W-2', '1099', 'Return', 'Receipts', 'Deductions'][i % 5]}.pdf`, [100, 2000], 0.6);
  b.spread(b.mk(docs, 'Zoom'), 12, 3.4 * GB, (i) => `2025-0${(i % 9) + 1}-1${i % 9} Team sync ${i}.mp4`, [300, 500], 0.5);
  const backups = b.mk(docs, 'Backups', 0, [100, 300]);
  b.spread(b.mk(docs, 'Visual Studio 2022\\Projects\\HelloWin32'), 20, 40 * MB, (i, rr) => `${rr.pick(['main', 'resource', 'HelloWin32', 'stdafx'])}${i}.${rr.pick(['cpp', 'h', 'vcxproj', 'pdb', 'obj'])}`, [300, 500]);

  // Downloads
  const dl = b.mk(alex, 'Downloads', 0, [0, 1]);
  const big: [string, number, number][] = [
    ['Win11_24H2_English_x64.iso', 5.84 * GB, 160],
    ['ubuntu-24.04-desktop-amd64.iso', 5.7 * GB, 240],
    ['debian-12.7.0-amd64-DVD-1.iso', 3.9 * GB, 120],
    ['DaVinci_Resolve_19.0_Windows.zip', 3.3 * GB, 95],
    ['photos-2024.zip', 6.4 * GB, 210],
    ['Assets_Megapack_2025.zip', 12.1 * GB, 280],
    ['Blender_Splash_Files.zip', 900 * MB, 140],
    ['cuda_12.6.1_560.94_windows.exe', 3.2 * GB, 88],
    ['Docker Desktop Installer.exe', 520 * MB, 75],
    ['NVIDIA-app-v10.0.3.exe', 160 * MB, 44],
    ['OBS-Studio-30.2.3-Windows-Installer.exe', 140 * MB, 63],
    ['VSCodeUserSetup-x64-1.93.1.exe', 96 * MB, 30],
    ['python-3.12.5-amd64.exe', 26 * MB, 54],
    ['Git-2.46.0-64-bit.exe', 66 * MB, 70],
    ['node-v20.17.0-x64.msi', 30 * MB, 33],
    ['SteamSetup.exe', 2.3 * MB, 400],
    ['ChromeSetup.exe', 1.4 * MB, 380],
    ['discord_setup.exe', 112 * MB, 160],
    ['Spotify Setup.exe', 1.3 * MB, 300],
    ['vs_community__2022.exe', 4.2 * MB, 210],
    ['Kdenlive-24.08.1.msi', 120 * MB, 36],
    ['IMG_4471.MOV', 1.9 * GB, 18],
    ['Interstellar.Soundtrack.flac.zip', 1.1 * GB, 600],
    ['GodotEngine_4.3-stable_win64.zip', 70 * MB, 48],
  ];
  const dlIds = new Map<string, number>();
  big.forEach(([n, s, a]) => dlIds.set(n, b.file(dl, n, s, a)));
  b.spread(dl, 180, 6.8 * GB, (i, rr) => `${rr.pick(['invoice', 'boarding-pass', 'IMG', 'Screenshot', 'manual', 'paper', 'wallpaper', 'resume', 'statement', 'export', 'archive', 'clip', 'presentation'])}_${2000 + i * 7}.${rr.pick(['pdf', 'pdf', 'jpg', 'png', 'zip', 'docx', 'mp4', 'xlsx', 'csv', 'webp', 'epub'])}`, [0, 700], 1.4);
  b.spread(b.mk(dl, 'Telegram Desktop'), 160, 2.1 * GB, (i, rr) => `${rr.pick(['photo', 'video', 'file', 'sticker', 'voice'])}_${i}@${rr.int(10, 28)}-0${rr.int(1, 9)}-2026.${rr.pick(['jpg', 'mp4', 'pdf', 'ogg', 'webp'])}`, [0, 300], 1.3);
  // Deliberate duplicates
  b.copy(dlIds.get('Win11_24H2_English_x64.iso')!, dl, 'Win11_24H2_English_x64 (1).iso', 158);
  b.copy(dlIds.get('VSCodeUserSetup-x64-1.93.1.exe')!, dl, 'VSCodeUserSetup-x64-1.93.1 (1).exe', 29);
  b.copy(dlIds.get('photos-2024.zip')!, backups, 'photos-2024.zip', 205);
  b.copy(dlIds.get('DaVinci_Resolve_19.0_Windows.zip')!, backups, 'DaVinci_Resolve_19.0_Windows.zip', 90);

  // Music
  music(b, b.mk(alex, 'Music', 0, [30, 300]), 20, 'flac');

  // Pictures
  const pics = b.mk(alex, 'Pictures', 0, [0, 2]);
  const roll = b.mk(pics, 'Camera Roll', 0, [0, 30]);
  b.spread(b.mk(roll, '2024'), 520, 2.6 * GB, PHOTO(2024), [280, 640], 0.5);
  b.spread(b.mk(roll, '2025'), 640, 3.3 * GB, PHOTO(2025), [30, 280], 0.5);
  const roll26 = b.mk(roll, '2026');
  b.spread(roll26, 300, 1.6 * GB, PHOTO(2026), [0, 270], 0.5);
  b.spread(b.mk(pics, 'Screenshots'), 320, 1.1 * GB, (i) => `Screenshot ${2025 + (i % 2)}-${String((i % 12) + 1).padStart(2, '0')}-${String((i % 28) + 1).padStart(2, '0')} ${String(100000 + i * 317).slice(-6)}.png`, [0, 500], 0.7);
  b.spread(b.mk(pics, 'OmniHub'), 40, 160 * MB, (i) => `2026-09-${String((i % 28) + 1).padStart(2, '0')}_${String(10 + (i % 13)).padStart(2, '0')}-${String(i * 7 % 60).padStart(2, '0')}-${String(i * 13 % 60).padStart(2, '0')}.png`, [0, 28], 0.6);
  b.spread(b.mk(pics, 'RAW\\2025'), 640, 19.5 * GB, seq('_DSC', 'CR3', 4, 1200), [30, 300], 0.15);
  b.spread(b.mk(pics, 'RAW\\2026'), 260, 8.1 * GB, seq('_DSC', 'CR3', 4, 4100), [0, 120], 0.15);
  const lr = b.mk(pics, 'Lightroom', 0, [0, 5]);
  b.file(lr, 'Lightroom Catalog-v13.lrcat', 1.2 * GB, 1);
  b.spread(b.mk(lr, 'Lightroom Catalog-v13 Previews.lrdata'), 400, 3.1 * GB, (_, rr) => `${rr.hex(4).toUpperCase()}\\${rr.hex(32)}.lrprev`.replace('\\', '_'), [0, 120]);
  b.spread(b.mk(pics, 'Wallpapers'), 60, 310 * MB, (i, rr) => `${rr.pick(['aurora', 'mountains', 'city-night', 'abstract', 'forest', 'nebula', 'desert'])}-${i}-4k.jpg`, [30, 900]);
  const imports = b.mk(pics, 'Imports', 0, [20, 25]);
  t.node(roll26).children.slice(0, 26).forEach((id) => b.copy(id, imports, undefined, 22));

  // Videos
  const vids = b.mk(alex, 'Videos', 0, [0, 3]);
  b.spread(b.mk(vids, 'OBS'), 24, 156 * GB, (i) => `2026-0${(i % 9) + 1}-${String((i % 27) + 1).padStart(2, '0')} 2${i % 3}-${String(i * 7 % 60).padStart(2, '0')}-${String(i * 11 % 60).padStart(2, '0')}.mkv`, [0, 270], 0.6);
  b.spread(b.mk(vids, 'OBS\\Replays'), 40, 22 * GB, (i) => `Replay ${2026}-09-${String((i % 28) + 1).padStart(2, '0')} 21-${String(i % 60).padStart(2, '0')}-12.mp4`, [0, 40], 0.5);
  b.spread(b.mk(vids, 'Captures'), 60, 38 * GB, (i, rr) => `${rr.pick(['Cyberpunk 2077', "Baldur's Gate 3", 'ELDEN RING', 'Counter-Strike 2', 'Hades II'])} ${2026}-0${(i % 9) + 1}-${String((i % 28) + 1).padStart(2, '0')} ${String(100000 + i * 911).slice(-6)}.mp4`, [0, 270], 0.9);
  b.spread(b.mk(vids, 'Edits\\Phoenix Trailer'), 30, 12 * GB, (i, rr) => (i === 0 ? 'Phoenix Trailer.prproj' : `${rr.pick(['A001', 'B002', 'drone', 'interview', 'broll'])}_C0${i}.${rr.pick(['mov', 'mp4', 'wav'])}`), [5, 60], 1.0);
  b.copy(dlIds.get('IMG_4471.MOV')!, vids, 'IMG_4471.MOV', 17);

  // source\repos
  const repos = b.mk(alex, 'source\\repos', 0, [0, 30]);
  for (const name of ['dotfiles', 'game-jam-2026', 'rust-raytracer', 'discord-bot', 'home-assistant-config', 'ml-experiments']) {
    const repo = b.mk(repos, name, 0, [0, 120]);
    b.spread(b.mk(repo, '.git\\objects\\pack'), r.int(2, 6), r.range(20, 900) * MB, (_, rr) => `pack-${rr.hex(40)}.${rr.pick(['pack', 'idx', 'rev'])}`, [0, 120], 1.2);
    b.spread(b.mk(repo, 'src'), r.int(8, 30), r.range(0.2, 3) * MB, (i, rr) => `${rr.pick(['main', 'lib', 'utils', 'scene', 'model', 'bot', 'config'])}${i}.${rr.pick(['rs', 'py', 'ts', 'js', 'yaml'])}`, [0, 120]);
    if (name === 'ml-experiments') b.spread(b.mk(repo, 'checkpoints'), 8, 7.4 * GB, (i) => `epoch_${(i + 1) * 5}.safetensors`, [40, 200], 0.4);
    if (name === 'rust-raytracer') b.spread(b.mk(repo, 'target\\release\\deps'), 120, 1.4 * GB, (i, rr) => `lib${rr.pick(['image', 'rayon', 'glam', 'png'])}-${rr.hex(16)}${i}.rlib`, [10, 60]);
  }

  // OneDrive (cloud placeholders — size but nothing on disk)
  const od = b.mk(alex, 'OneDrive', 0, [0, 10]);
  b.spread(b.mk(od, 'Documents'), 90, 3.4 * GB, (i, rr) => `${rr.pick(['Thesis', 'Notes', 'Plan', 'Recipe', 'Statement'])} ${i}.${rr.pick(['docx', 'pdf', 'xlsx'])}`, [10, 900], 1.2, FLAG_CLOUD);
  b.spread(b.mk(od, 'Pictures'), 120, 2.2 * GB, PHOTO(2023), [500, 900], 0.5, FLAG_CLOUD);

  t.finish();
  // Cloud placeholders take no space on disk.
  for (const n of t.nodes) if (!n.isDir && n.flags & FLAG_CLOUD) n.alloc = 0;
  t.finish();
  return t;
}

// ---------- D:\ (games and media) ----------

function buildD(): FakeTree {
  const t = new FakeTree('D:\\', NOW - 1 * DAY);
  const r = new Rng(0xd15c);
  const b = new Builder(t, r);
  const common = b.mk(0, 'SteamLibrary\\steamapps\\common', 0, [5, 200]);
  const games: [string, number, number, string, Age][] = [
    ['Starfield', 128, 44, 'ba2', [60, 300]],
    ['Red Dead Redemption 2', 118, 34, 'rpf', [200, 500]],
    ['Microsoft Flight Simulator 2024', 162, 180, 'fspackage', [10, 60]],
    ['ForzaHorizon5', 112, 210, 'zip', [100, 400]],
    ['Black Myth Wukong', 128, 52, 'pak', [30, 90]],
    ['Alan Wake 2', 88, 70, 'bin', [60, 300]],
    ['The Witcher 3', 72, 110, 'bundle', [300, 700]],
    ['Call of Duty HQ', 214, 260, 'xpak', [2, 20]],
    ['Helldivers 2', 72, 100, 'pkg', [10, 60]],
    ['Stardew Valley', 0.6, 400, 'xnb', [100, 800]],
  ];
  for (const [name, gb, files, ext, age] of games) {
    const g = b.mk(common, name, 0, age);
    b.spread(b.mk(g, 'Content'), files, gb * GB * 0.97, (i) => `${name.replace(/\W/g, '').slice(0, 10).toLowerCase()}_${String(i).padStart(3, '0')}.${ext}`, age, 1.1);
    b.spread(g, r.int(20, 60), gb * GB * 0.03, dll(['dll', 'exe']), age);
  }
  const sa = b.mk(0, 'SteamLibrary\\steamapps');
  b.folders(b.mk(sa, 'shadercache'), ['1716740', '1174180', '2537590', '1551360', '2358720', '2215430', '292030', '1938090'], [3, 8], 3.2 * GB, (i) => `steamapp_pipeline_cache_${i}.foz`, [0, 40]);
  b.spread(b.mk(sa, 'workshop\\content\\294100'), 80, 8.2 * GB, (i) => `${2900000000 + i * 31}.zip`, [20, 400]);
  const movies = b.mk(0, 'Media\\Movies', 0, [30, 900]);
  const titles = ['Dune Part Two (2024)', 'Oppenheimer (2023)', 'Blade Runner 2049 (2017)', 'Interstellar (2014)', 'The Batman (2022)', 'Mad Max Fury Road (2015)', 'Arrival (2016)', 'Everything Everywhere All at Once (2022)', 'Spider-Man Across the Spider-Verse (2023)', 'Parasite (2019)', 'The Dark Knight (2008)', 'Inception (2010)', 'Top Gun Maverick (2022)', 'Past Lives (2023)', 'Poor Things (2023)', 'Godzilla Minus One (2023)', 'The Grand Budapest Hotel (2014)', 'Whiplash (2014)', 'Her (2013)', 'Ex Machina (2014)', 'Sicario (2015)', 'Prisoners (2013)', 'Knives Out (2019)', 'Joker (2019)', 'Drive (2011)', 'The Social Network (2010)', 'Gravity (2013)', 'Annihilation (2018)', 'The Martian (2015)', 'Alien Romulus (2024)', 'Furiosa (2024)', 'Civil War (2024)', 'The Holdovers (2023)', 'Killers of the Flower Moon (2023)', 'Tenet (2020)', 'No Country for Old Men (2007)', 'Spirited Away (2001)', 'Your Name (2016)', 'Akira (1988)', 'Princess Mononoke (1997)'];
  titles.forEach((m) => {
    const d = b.mk(movies, m, 0, [30, 900]);
    b.file(d, `${m.replace(/ \(\d+\)/, '')} ${r.chance(0.6) ? '2160p UHD' : '1080p'} BluRay.mkv`, r.range(4, 24) * GB, [30, 900]);
    b.file(d, `${m.replace(/ \(\d+\)/, '')}.en.srt`, r.range(60, 140) * KB, [30, 900]);
    b.file(d, 'poster.jpg', r.range(0.2, 1.4) * MB, [30, 900]);
  });
  const tv = b.mk(0, 'Media\\TV Shows', 0, [30, 900]);
  for (const show of ['Severance', 'The Bear', 'Shogun', 'Arcane', 'The Last of Us', 'Andor', 'Succession', 'Silo', 'Fallout']) {
    const seasons = r.int(1, 4);
    for (let s = 1; s <= seasons; s++) {
      const d = b.mk(tv, `${show}\\Season ${s}`, 0, [30, 900]);
      const eps = r.int(8, 10);
      for (let e = 1; e <= eps; e++) b.file(d, `${show} S${String(s).padStart(2, '0')}E${String(e).padStart(2, '0')} 2160p WEB-DL.mkv`, r.range(1.2, 3.6) * GB, [30, 900]);
    }
  }
  music(b, b.mk(0, 'Media\\Music', 0, [100, 1500]), 22, 'flac');
  b.spread(b.mk(0, 'Recordings'), 48, 262 * GB, (i) => `2026-0${(i % 9) + 1}-${String((i % 28) + 1).padStart(2, '0')} stream ${i + 1}.mkv`, [1, 270], 0.6);
  b.file(b.mk(0, 'Backups\\Macrium', 0, [40, 60]), 'C-Drive-2026-08-14-Full.mrimg', 412 * GB, 50);
  b.file(b.mk(0, 'Backups\\Macrium'), 'C-Drive-2026-09-20-Diff.mrimg', 38 * GB, 13);
  for (const y of [2019, 2020, 2021, 2022, 2023]) b.spread(b.mk(0, `Photos Archive\\${y}`), 220, r.range(5, 9) * GB, seq(`IMG_${y}_`, r.chance(0.5) ? 'CR2' : 'NEF', 4), [365 * (2026 - y) - 300, 365 * (2026 - y)], 0.25);
  const bin = b.mk(0, '$RECYCLE.BIN\\S-1-5-21-3623811015-3361044348-30300820-1001', HS, [2, 30]);
  b.spread(bin, 10, 3.1 * GB, (_, rr) => `$R${rr.hex(6).toUpperCase()}.${rr.pick(['mkv', 'zip', 'mp4'])}`, [2, 30], 1, HS);
  b.spread(b.mk(0, 'System Volume Information', HS, [0, 2]), 4, 60 * MB, (i) => (i === 0 ? 'tracking.log' : `{${r.hex(8)}}`), [0, 2], 1, HS);
  t.finish();
  return t;
}

// ---------- E:\ (removable, exFAT) ----------

function buildE(): FakeTree {
  const t = new FakeTree('E:\\', NOW - 6 * DAY);
  const b = new Builder(t, new Rng(0xe5));
  b.spread(b.mk(0, 'DCIM\\100GOPRO', 0, [6, 30]), 28, 46 * GB, seq('GX01', 'MP4', 4, 2210), [6, 30], 0.6);
  b.spread(b.mk(0, 'DCIM\\100GOPRO'), 28, 120 * MB, seq('GX01', 'THM', 4, 2210), [6, 30], 0.3);
  b.spread(b.mk(0, 'Installers'), 9, 14 * GB, fromList(['Win11_23H2_x64', 'Office_2024_Pro', 'drivers-backup', 'Ventoy-1.0.99', 'memtest86-usb', 'Rufus-4.5', 'GParted-live', 'Hirens-BootCD', 'Clonezilla'], ['iso', 'zip', 'exe']), [60, 400], 1.3);
  music(b, b.mk(0, 'Music', 0, [200, 900]), 6, 'mp3');
  t.finish();
  return t;
}

// ---------- volumes ----------

interface Drive {
  info: Omit<VolumeInfo, 'free'>;
  build: () => FakeTree;
  tree: FakeTree | null;
  /** Bytes in use that a scan cannot see ($MFT, shadow copies…). */
  overhead: number;
}

const drives: Drive[] = [
  { info: { root: 'C:\\', label: 'Windows', fileSystem: 'NTFS', kind: 'fixed', total: 2000396746752, clusterSize: 4096, serial: 0x5c2a91f4, mftCapable: true }, build: buildC, tree: null, overhead: 3.4 * GB },
  { info: { root: 'D:\\', label: 'Data', fileSystem: 'NTFS', kind: 'fixed', total: 4000785100800, clusterSize: 4096, serial: 0x8e13b027, mftCapable: true }, build: buildD, tree: null, overhead: 5.1 * GB },
  { info: { root: 'E:\\', label: 'SANDISK', fileSystem: 'exFAT', kind: 'removable', total: 128043712512, clusterSize: 131072, serial: 0x2f04c1aa, mftCapable: false }, build: buildE, tree: null, overhead: 40 * MB },
];

export function driveTree(root: string): FakeTree | null {
  const letter = root.slice(0, 2).toUpperCase();
  const d = drives.find((x) => x.info.root.startsWith(letter));
  if (!d) return null;
  if (!d.tree) d.tree = d.build();
  return d.tree;
}

export function volumes(): VolumeInfo[] {
  return drives.map((d) => {
    const tree = driveTree(d.info.root)!;
    const used = tree.node(0).alloc + d.overhead;
    return { ...d.info, free: Math.max(0, d.info.total - used) };
  });
}

export function volumeFor(root: string): VolumeInfo | null {
  const letter = root.slice(0, 2).toUpperCase();
  return volumes().find((v) => v.root.startsWith(letter)) ?? null;
}

/** Environment tokens used by the cleanup rules. */
export const TOKENS: Record<string, string> = {
  TEMP: 'C:\\Users\\Alex\\AppData\\Local\\Temp',
  LOCALAPPDATA: 'C:\\Users\\Alex\\AppData\\Local',
  APPDATA: 'C:\\Users\\Alex\\AppData\\Roaming',
  USERPROFILE: 'C:\\Users\\Alex',
  WINDIR: 'C:\\Windows',
  PROGRAMDATA: 'C:\\ProgramData',
  SYSTEMDRIVE: 'C:',
  PROGRAMFILES: 'C:\\Program Files',
  PROGRAMFILESX86: 'C:\\Program Files (x86)',
};
