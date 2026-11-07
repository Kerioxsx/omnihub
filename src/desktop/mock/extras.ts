// 0.30 features for the mock backend: Capri Desktop, heat alerts, app
// auto-updates, the party queue, screen time and the watchers.

import type { AutoUpdateSettings, CapriDesktopStatus, HeatSettings, LimitUse, PartySong, PartyView, PriceWatch, ScreenLimit, WatchStore } from '@shared/types';
import { emit } from './bus';

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
const id = () => Math.random().toString(36).slice(2, 12);

let capri: CapriDesktopStatus = {
  installed: false,
  exe: null,
  version: null,
  running: false,
  settings: { startWithOmnihub: false },
  ollama: { installed: true, running: true, models: ['llama3.2:3b'], hasModel: false },
  supported: true,
};
export const capriDesktop = {
  status: () => capri,
  latest: () => ({ version: '1.4.2', url: 'https://github.com/lachydotmcg/capri-desktop/releases/download/v1.4.2/Capri-Setup.exe', size: 98_000_000, sha256: null, notes: 'Bug fixes.' }),
  install: async () => {
    for (let p = 0; p <= 100; p += 20) {
      emit('capri:download', { done: p * 980_000, total: 98_000_000 });
      await wait(200);
    }
    capri = { ...capri, installed: true, version: '1.4.2', exe: 'C:\\Users\\you\\AppData\\Local\\Programs\\Capri\\Capri.exe' };
    return capriDesktop.latest();
  },
  open: () => {
    capri = { ...capri, running: true };
  },
  close: () => {
    capri = { ...capri, running: false };
  },
  setSettings: (s: CapriDesktopStatus['settings']) => {
    capri = { ...capri, settings: s };
  },
  pull: async () => {
    for (let p = 0; p <= 100; p += 10) {
      emit('capri:pull', { status: p < 100 ? 'downloading' : 'success', done: p * 98_000_000, total: 9_800_000_000 });
      await wait(150);
    }
    capri = { ...capri, ollama: { ...capri.ollama, models: [...capri.ollama.models, 'gemma4:e4b'], hasModel: true } };
  },
};

let heat: HeatSettings = { enabled: true, cpu: 90, gpu: 85 };
export const heatMock = {
  get: () => heat,
  set: (s: HeatSettings) => (heat = { ...s, cpu: Math.min(105, Math.max(60, s.cpu)), gpu: Math.min(100, Math.max(60, s.gpu)) }),
};

let updates: AutoUpdateSettings = { enabled: false, time: '03:30', skip: [], lastRun: Date.now() / 1000 - 86_400, lastResult: '3 updated' };
export const appUpdatesMock = {
  get: () => ({ settings: updates, running: false }),
  set: (s: AutoUpdateSettings) => {
    updates = { ...s, lastRun: updates.lastRun, lastResult: updates.lastResult };
    return appUpdatesMock.get();
  },
  run: async () => {
    await wait(1200);
    updates = { ...updates, lastRun: Date.now() / 1000, lastResult: '2 updated' };
    return appUpdatesMock.get();
  },
};

const song = (title: string, artist: string, by: string): PartySong => ({ id: id(), title, artist, album: '', cover: '', by, at: Date.now() });
let party: PartyView['state'] = { code: null, queue: [], playing: null, auto: true, played: 0 };
const view = (): PartyView => {
  const url = party.code ? `https://192.168.1.24:8765/party/${party.code}` : '';
  return { state: party, urls: url ? [url] : [], qrSvgs: url ? ['<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 21 21"><rect width="21" height="21" fill="#fff"/><path d="M1 1h7v7H1zM13 1h7v7h-7zM1 13h7v7H1zM10 10h2v2h-2zM14 14h3v3h-3z" fill="#000"/></svg>'] : [] };
};
const changed = () => {
  emit('party:changed', party);
  return view();
};
export const partyMock = {
  get: view,
  start: () => {
    party = { ...party, code: party.code ?? id(), queue: party.queue.length ? party.queue : [song('Blinding Lights', 'The Weeknd', 'Sam'), song('Levitating', 'Dua Lipa', 'Alex'), song('As It Was', 'Harry Styles', 'Mia')] };
    return changed();
  },
  stop: () => {
    party = { code: null, queue: [], playing: null, auto: true, played: 0 };
    return changed();
  },
  auto: (on: boolean) => {
    party = { ...party, auto: on };
    return view();
  },
  remove: (sid: string) => {
    party = { ...party, queue: party.queue.filter((s) => s.id !== sid) };
    return changed();
  },
  bump: (sid: string) => {
    const s = party.queue.find((x) => x.id === sid);
    if (s) party = { ...party, queue: [s, ...party.queue.filter((x) => x.id !== sid)] };
    return changed();
  },
  next: () => {
    const [first, ...rest] = party.queue;
    party = { ...party, playing: first ?? null, queue: rest, played: party.played + (first ? 1 : 0) };
    return changed();
  },
};

let limits: ScreenLimit[] = [
  { id: 'l1', kind: 'app', target: 'tiktok.exe', minutes: 60, enabled: true },
  { id: 'l2', kind: 'site', target: 'YouTube', minutes: 90, enabled: true },
];
const used: Record<string, number> = { l1: 42, l2: 95 };
const uses = (): LimitUse[] => limits.map((l) => ({ limit: l, usedMinutes: used[l.id] ?? 0, over: (used[l.id] ?? 0) >= l.minutes }));
export const screenTimeMock = {
  list: uses,
  save: (l: ScreenLimit) => {
    if (!l.target.trim()) throw new Error('Pick an app or type a website word');
    let t = l.target.trim();
    if (l.kind === 'app') {
      t = t.toLowerCase();
      if (!t.endsWith('.exe')) t += '.exe';
    }
    const next = { ...l, target: t, minutes: Math.min(1440, Math.max(5, l.minutes)), id: l.id || id() };
    limits = limits.some((x) => x.id === next.id) ? limits.map((x) => (x.id === next.id ? next : x)) : [...limits, next];
    return uses();
  },
  remove: (lid: string) => {
    limits = limits.filter((l) => l.id !== lid);
    return uses();
  },
};

let watch: WatchStore = {
  streamers: [
    { id: 's1', platform: 'twitch', handle: 'shroud', live: true, title: 'Ranked grind' },
    { id: 's2', platform: 'youtube', handle: '@LinusTechTips', live: false, title: '' },
  ],
  recapTime: '21:00',
  recapDate: '',
  shopWanted: ['Peely', 'Renegade Raider'],
  shopAlerts: true,
  shopSeen: [],
  shopDate: '',
  shopNow: [],
  prices: [
    { id: 'p1', name: 'RTX 5070', url: 'https://www.example-shop.com/rtx-5070', target: 549, price: 579.99, currency: 'USD', lowest: 569.99, history: [[Date.now() / 1000 - 86_400 * 3, 599.99], [Date.now() / 1000 - 86_400, 569.99], [Date.now() / 1000, 579.99]], checked: Date.now() / 1000, error: null },
  ],
};
export const watchMock = {
  get: () => watch,
  saveStreamer: (st: { platform: 'twitch' | 'youtube'; handle: string }) => {
    if (!/^@?[A-Za-z0-9_.-]{2,40}$/.test(st.handle)) throw st.platform === 'youtube' ? 'Use the channel’s handle, like @MrBeast' : 'Use the Twitch name, like shroud';
    return (watch = { ...watch, streamers: [...watch.streamers, { id: id(), platform: st.platform, handle: st.handle, live: false, title: '' }] });
  },
  removeStreamer: (sid: string) => (watch = { ...watch, streamers: watch.streamers.filter((x) => x.id !== sid) }),
  setRecap: (time: string) => (watch = { ...watch, recapTime: time }),
  setShop: (wanted: string[], alerts: boolean) => (watch = { ...watch, shopWanted: wanted, shopAlerts: alerts }),
  checkShop: async () => {
    await wait(600);
    watch = { ...watch, shopDate: new Date().toISOString().slice(0, 10), shopNow: ['Peely', 'Fishstick', 'Drift', 'Midas', 'Raven', 'Skull Trooper', 'Aura', 'Jonesy'] };
    emit('watch:shop', { item: 'Peely' });
    return watch;
  },
  savePrice: async (w: PriceWatch) => {
    if (!/^https:\/\//.test(w.url)) throw new Error('Paste the product page’s address (https://…)');
    await wait(500);
    const next: PriceWatch = { ...w, id: w.id || id(), name: w.name || new URL(w.url).hostname, price: w.price ?? 129.99, currency: 'USD', lowest: w.lowest ?? 129.99, history: w.history.length ? w.history : [[Date.now() / 1000, 129.99]], checked: Date.now() / 1000 };
    watch = { ...watch, prices: watch.prices.some((p) => p.id === next.id) ? watch.prices.map((p) => (p.id === next.id ? next : p)) : [...watch.prices, next] };
    return watch;
  },
  removePrice: (pid: string) => (watch = { ...watch, prices: watch.prices.filter((p) => p.id !== pid) }),
  checkPrices: async () => {
    await wait(700);
    return watch;
  },
};

export function voiceMock(text: string): string {
  const t = text.toLowerCase().replace(/^(hey |ok )?capri,? /, '');
  if (/quiet|good night/.test(t)) return 'Quiet mode on';
  const m = t.match(/^(start|launch|play) (.+)/);
  if (m) return `Starting ${m[2].replace(/\b\w/g, (c) => c.toUpperCase())}`;
  if (/clip|screenshot|open /.test(t)) return 'Done';
  throw new Error(`I didn't understand “${text}”.`);
}
