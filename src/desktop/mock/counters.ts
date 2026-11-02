// Counters, the auto clicker and the Pokémon save editor for the mock
// backend: the same shapes as the real ones, kept in memory.

import type { ClickConfig, ClickStatus, Counter, CounterInput, CountersState, Found, MonEdit, MonView, PkmSave, PkmSlot, PkmTables, PkmTrainer } from '@shared/types';
import { emit } from './bus';

const now = () => Date.now();
let seq = 1;
const id = () => `m${(seq++).toString(36)}${Math.random().toString(36).slice(2, 6)}`;

const state: CountersState = {
  keysOn: true,
  counters: [
    {
      id: 'demo-hunt',
      name: 'Shiny Charizard',
      count: 3127,
      step: 1,
      image: null,
      key: { vk: 0x20, label: 'Space' },
      minusKey: null,
      timerMs: 3 * 3600_000 + 41 * 60_000,
      timerSince: null,
      hunt: { species: 6, name: 'Charizard', generation: 9, masuda: false, charm: true, customOdds: null, method: 'Soft resets' },
      created: now() - 86400_000,
      updated: now(),
    },
    { id: 'demo-deaths', name: 'Elden Ring deaths', count: 214, step: 1, image: null, key: { vk: 0x75, label: 'F6' }, minusKey: { vk: 0x74, label: 'F5' }, timerMs: 0, timerSince: null, hunt: null, created: now() - 3 * 86400_000, updated: now() },
  ],
  found: [{ id: 'f1', counter: 'demo-hunt', name: 'Ralts', species: 280, count: 1840, timerMs: 2 * 3600_000, at: now() - 5 * 86400_000, odds: '1/1365 · Masuda method' }],
};

const changed = () => emit('counters:changed', structuredClone(state));
const find = (cid: string) => {
  const c = state.counters.find((x) => x.id === cid);
  if (!c) throw new Error('No such counter');
  return c;
};
const elapsed = (c: Counter) => c.timerMs + (c.timerSince ? now() - c.timerSince : 0);

export const counters = {
  state: (): CountersState => structuredClone(state),
  save(input: CounterInput): Counter {
    const name = input.name.trim();
    if (!name) throw new Error('Give the counter a name');
    let c = input.id ? state.counters.find((x) => x.id === input.id) : undefined;
    if (!c) {
      c = { id: id(), name, count: 0, step: 1, image: null, key: null, minusKey: null, timerMs: 0, timerSince: null, hunt: null, created: now(), updated: now() };
      state.counters.push(c);
    }
    Object.assign(c, { name, step: Math.max(1, input.step || 1), key: input.key, minusKey: input.minusKey, hunt: input.hunt, updated: now() });
    changed();
    return structuredClone(c);
  },
  remove(cid: string) {
    state.counters = state.counters.filter((c) => c.id !== cid);
    changed();
  },
  bump(cid: string, times: number): Counter {
    const c = find(cid);
    c.count = Math.max(0, c.count + times * c.step);
    changed();
    return structuredClone(c);
  },
  set(cid: string, count: number): Counter {
    const c = find(cid);
    c.count = Math.max(0, Math.floor(count));
    changed();
    return structuredClone(c);
  },
  timer(cid: string, run: boolean, reset: boolean): Counter {
    const c = find(cid);
    c.timerMs = reset ? 0 : elapsed(c);
    c.timerSince = run ? now() : null;
    changed();
    return structuredClone(c);
  },
  keysOn(on: boolean) {
    state.keysOn = on;
    changed();
  },
  found(cid: string, reset: boolean): Found {
    const c = find(cid);
    const f: Found = { id: id(), counter: c.id, name: c.hunt?.name ?? c.name, species: c.hunt?.species ?? null, count: c.count, timerMs: elapsed(c), at: now(), odds: c.hunt ? `1/4096 · ${c.hunt.method}` : null };
    state.found.unshift(f);
    if (reset) {
      c.count = 0;
      c.timerMs = 0;
      c.timerSince = c.timerSince ? now() : null;
    }
    changed();
    return f;
  },
  forgetFound(fid: string) {
    state.found = state.found.filter((f) => f.id !== fid);
    changed();
  },
  setImage(cid: string, path: string | null): Counter {
    const c = find(cid);
    c.image = path ? 'demo.png' : null;
    changed();
    return structuredClone(c);
  },
  image: (): string | null => null,
  /** The page's own keys stand in for the system-wide key watcher. */
  key(vk: number) {
    if (!state.keysOn) return;
    let hit = false;
    for (const c of state.counters) {
      if (c.key?.vk === vk) c.count += c.step;
      else if (c.minusKey?.vk === vk) c.count = Math.max(0, c.count - c.step);
      else continue;
      hit = true;
    }
    if (hit) changed();
  },
};

if (typeof window !== 'undefined') {
  window.addEventListener('keydown', (e) => {
    const t = e.target as HTMLElement | null;
    if (e.repeat || (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable))) return;
    counters.key(e.keyCode);
  });
}

// ---------- auto clicker ----------

let config: ClickConfig = { button: 'left', double: false, intervalMs: 100, jitterMs: 0, maxClicks: 0, maxSeconds: 0, at: null, toggleKey: 0x75 };
let status: ClickStatus = { running: false, clicks: 0, rate: 0, seconds: 0, stopped: null };
let timer: ReturnType<typeof setInterval> | null = null;

export const autoclick = {
  config: () => structuredClone(config),
  setConfig(c: ClickConfig) {
    if (status.running) throw new Error('Stop the clicker before changing it');
    if (!(c.intervalMs >= 0.01)) throw new Error('The interval goes from 0.01 ms to an hour');
    config = structuredClone(c);
  },
  status: () => structuredClone(status),
  start() {
    if (status.running) return;
    const started = now();
    // What a fast PC reaches: about one click per interval down to ~1 ms, then Windows' ceiling.
    const target = Math.min(1000 / config.intervalMs, 20000) * (config.double ? 2 : 1);
    status = { running: true, clicks: 0, rate: 0, seconds: 0, stopped: null };
    timer = setInterval(() => {
      const s = (now() - started) / 1000;
      const rate = target * (0.94 + Math.random() * 0.05);
      status = { ...status, seconds: s, rate, clicks: Math.round(status.clicks + rate / 4) };
      if (config.maxClicks && status.clicks >= config.maxClicks) autoclick.stopWith(`Done: ${config.maxClicks} clicks`);
      else if (config.maxSeconds && s >= config.maxSeconds) autoclick.stopWith(`Done: ${config.maxSeconds} s`);
      else emit('autoclick:status', status);
    }, 250);
    emit('autoclick:status', status);
  },
  stopWith(reason: string) {
    if (timer) clearInterval(timer);
    timer = null;
    status = { ...status, running: false, stopped: reason };
    emit('autoclick:status', status);
  },
  stop: () => autoclick.stopWith('Stopped with the Stop button'),
  cursor: (): [number, number] => [960, 540],
};

// ---------- Pokémon save editor ----------

const NATURES = ['Hardy', 'Lonely', 'Brave', 'Adamant', 'Naughty', 'Bold', 'Docile', 'Relaxed', 'Impish', 'Lax', 'Timid', 'Hasty', 'Serious', 'Jolly', 'Naive', 'Modest', 'Mild', 'Quiet', 'Bashful', 'Rash', 'Calm', 'Gentle', 'Sassy', 'Careful', 'Quirky'];
const SPECIES: [number, string, number][] = [
  [1, 'Bulbasaur', 1], [4, 'Charmander', 4], [6, 'Charizard', 6], [25, 'Pikachu', 25], [150, 'Mewtwo', 150], [151, 'Mew', 151], [197, 'Umbreon', 197], [248, 'Tyranitar', 248],
  [277, 'Treecko', 252], [280, 'Sceptile', 254], [282, 'Torchic', 255], [284, 'Blaziken', 257], [285, 'Mudkip', 258], [287, 'Swampert', 260], [394, 'Ralts', 280], [396, 'Gardevoir', 282], [406, 'Rayquaza', 384], [401, 'Kyogre', 382], [405, 'Groudon', 383],
];
const MOVES = ['—', 'Pound', 'Tackle', 'Thunderbolt', 'Flamethrower', 'Surf', 'Earthquake', 'Psychic', 'Ice Beam', 'Dragon Claw', 'Leaf Blade', 'Blaze Kick', 'Calm Mind', 'Swords Dance', 'Protect', 'Hyper Beam'];
const ITEMS = ['None', 'Master Ball', 'Rare Candy', 'Leftovers', 'Choice Band', 'Lum Berry', 'Shell Bell', 'Soft Sand'];

export const pkmTables: PkmTables = {
  species: SPECIES.map(([sid, name, dex]) => ({ id: sid, name, dex })),
  moves: MOVES.map((name, i) => ({ id: i, name, dex: 15 })),
  items: ITEMS.map((name, i) => ({ id: i, name, dex: 0 })),
  natures: NATURES,
  balls: ['', 'Master Ball', 'Ultra Ball', 'Great Ball', 'Poké Ball', 'Safari Ball', 'Net Ball', 'Dive Ball', 'Nest Ball', 'Repeat Ball', 'Timer Ball', 'Luxury Ball', 'Premier Ball'],
  abilities: Array.from({ length: 412 }, () => ['Overgrow', ''] as [string, string]),
};

function mon(sid: number, level: number, extra: Partial<MonView> = {}): MonView {
  const [, name, dex] = SPECIES.find((s) => s[0] === sid) ?? SPECIES[0];
  const ivs = [31, 20, 31, 28, 31, 25];
  const base = 60 + (dex % 40);
  const stats = ivs.map((iv, i) => (i === 0 ? Math.floor(((2 * base + iv) * level) / 100) + level + 10 : Math.floor(((2 * base + iv) * level) / 100) + 5));
  return { species: sid, dex, speciesName: name, nickname: name.toUpperCase(), level, exp: level ** 3, nature: 3, shiny: false, gender: 'male', abilitySlot: 0, ability: 'Overgrow', item: 0, friendship: 70, moves: [2, 3, 0, 0], pp: [35, 15, 0, 0], ppUps: [0, 0, 0, 0], ivs, evs: [0, 0, 0, 0, 0, 0], stats, otName: 'MAY', tid: 12345, sid: 54321, ball: 4, metLevel: 5, egg: false, pid: 0x1234abcd, valid: true, ...extra };
}

let save: PkmSave | null = null;
const demoSave = (path: string): PkmSave => ({
  path,
  game: 'Emerald',
  trainer: { name: 'MAY', gender: 1, tid: 12345, sid: 54321, money: 48210, hours: 42, minutes: 17 },
  party: [mon(287, 52, { nickname: 'TIDE', item: 3 }), mon(396, 48, { shiny: true }), mon(406, 70, { moves: [9, 6, 15, 14] }), mon(25, 31, { gender: 'female' })],
  boxes: Array.from({ length: 14 }, (_, b) => ({
    name: `BOX ${b + 1}`,
    mons: Array.from({ length: 30 }, (_, i) => (b === 0 && i < 9 ? mon(SPECIES[i * 2 % SPECIES.length][0], 5 + i * 4, { shiny: i === 4 }) : null)),
  })),
  changed: false,
  checksumsOk: true,
});

function need(): PkmSave {
  if (!save) throw new Error('Open a save first');
  return save;
}

export const pkm = {
  tables: () => pkmTables,
  open: (path: string) => (save = demoSave(path)),
  view: () => save,
  close: () => void (save = null),
  edit(slot: PkmSlot, e: MonEdit): PkmSave {
    const s = need();
    const old = slot.box === null ? s.party[slot.index] : s.boxes[slot.box].mons[slot.index];
    const sid = e.species ?? old?.species ?? 277;
    const next = mon(sid, e.level ?? old?.level ?? 5, { ...(old ?? {}), ...Object.fromEntries(Object.entries(e).filter(([, v]) => v !== undefined)) });
    const sp = SPECIES.find((x) => x[0] === sid);
    if (sp) Object.assign(next, { speciesName: sp[1], dex: sp[2], nickname: e.nickname || (old && old.species === sid ? old.nickname : sp[1].toUpperCase()) });
    if (slot.box === null) {
      if (slot.index < s.party.length) s.party[slot.index] = next;
      else s.party.push(next);
    } else s.boxes[slot.box].mons[slot.index] = next;
    s.changed = true;
    return structuredClone(s);
  },
  remove(slot: PkmSlot): PkmSave {
    const s = need();
    if (slot.box === null) {
      if (s.party.length <= 1) throw new Error('The party needs at least one Pokémon');
      s.party.splice(slot.index, 1);
    } else s.boxes[slot.box].mons[slot.index] = null;
    s.changed = true;
    return structuredClone(s);
  },
  trainer(t: PkmTrainer): PkmSave {
    const s = need();
    s.trainer = { ...t };
    s.changed = true;
    return structuredClone(s);
  },
  importPk3: (slot: PkmSlot) => pkm.edit(slot, { species: 151, level: 30 }),
  exportPk3: () => undefined,
  write(): string {
    const s = need();
    s.changed = false;
    return `${s.path}.bak-20261102-120000`;
  },
};
