// Big Picture's building blocks: what can be focused (rows of items you move
// through with the controller), what a card stands for, and the look you pick
// in its settings (background, reduced motion).

import type { Browser, LibraryGame, Site, Watching } from '@shared/types';
import { useSyncExternalStore } from 'react';
import { padPrefs } from '../../../../lib/pads';

export type Tab = 'home' | 'library' | 'watch' | 'room' | 'settings';
export const TABS: { id: Tab; label: string }[] = [
  { id: 'home', label: 'Home' },
  { id: 'library', label: 'Library' },
  { id: 'watch', label: 'Watch' },
  { id: 'room', label: 'Room' },
  { id: 'settings', label: 'Settings' },
];

/** What a card opens. */
export type Target = { kind: 'game'; game: LibraryGame } | { kind: 'watching'; w: Watching } | { kind: 'site'; site: Site };

type Base = { key: string; run?: () => void };
export type Item = Base &
  (
    | { type: 'feature'; target: Target }
    | { type: 'wide'; target: Target }
    | { type: 'cover'; game: LibraryGame }
    | { type: 'channel'; site: Site }
    | { type: 'chip'; label: string; selected?: boolean }
    | { type: 'quick'; label: string; sub: string; icon: string }
    | { type: 'setting'; label: string; sub?: string; value: string | boolean }
    | { type: 'theme'; id: BpTheme; label: string; selected: boolean }
    | { type: 'browser'; browser: Browser | null; selected: boolean }
    | { type: 'button'; label: string; primary?: boolean }
    | { type: 'notice'; title: string; sub: string; action: string }
  );

export type RowKind = 'hero' | 'strip' | 'grid' | 'chips' | 'quick' | 'list' | 'themes' | 'browsers' | 'buttons' | 'notice';
export interface NavRow {
  id: string;
  kind: RowKind;
  /** Shown above the row (the first row of a grid only). */
  title?: string;
  sub?: string;
  items: Item[];
  /** Rows of one grid keep the column when you move between them. */
  group?: string;
  /** Columns, for grids. */
  cols?: number;
}

/** `list` cut into rows of `n`. */
export function chunk<T>(list: T[], n: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < list.length; i += n) out.push(list.slice(i, i + n));
  return out;
}

/** The list with `i` set to `v` (growing it when rows appear later). */
export const setAt = (c: number[], i: number, v: number) => {
  const n = [...c];
  while (n.length <= i) n.push(0);
  n[i] = v;
  return n;
};

export function targetTitle(t: Target): string {
  if (t.kind === 'game') return t.game.name;
  if (t.kind === 'site') return t.site.name;
  return t.w.show || t.w.service;
}

export function targetKey(t: Target): string {
  return t.kind === 'game' ? t.game.key : t.kind === 'site' ? `site:${t.site.id}` : `w:${t.w.url}`;
}

/** "40 min ago", "yesterday", "3 days ago". */
export function since(ms: number): string {
  const m = Math.max(0, Math.round((Date.now() - ms) / 60000));
  if (m < 2) return 'just now';
  if (m < 60) return `${m} min ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h} h ago`;
  const d = Math.round(h / 24);
  return d === 1 ? 'yesterday' : `${d} days ago`;
}

// ---------- look ----------

export type BpTheme = 'nebula' | 'oled' | 'aurora' | 'ember';
export const THEMES: { id: BpTheme; label: string; bg: string; swatch: string; accent: string; accentRgb: string }[] = [
  { id: 'nebula', label: 'Nebula', bg: 'radial-gradient(ellipse 70% 90% at 85% -20%, #232a52 0, transparent 75%), radial-gradient(ellipse 50% 60% at -5% 105%, #1a1638 0, transparent 70%), #080b15', swatch: 'linear-gradient(125deg,#192849,#5949a0)', accent: '#acbcff', accentRgb: '172,188,255' },
  { id: 'oled', label: 'OLED black', bg: '#000', swatch: 'linear-gradient(125deg,#111,#1b1b1b)', accent: '#d6dcff', accentRgb: '214,220,255' },
  { id: 'aurora', label: 'Aurora', bg: 'radial-gradient(ellipse 70% 90% at 85% -20%, #0f4d4b 0, transparent 72%), radial-gradient(ellipse 55% 60% at -5% 105%, #10304a 0, transparent 70%), #031016', swatch: 'linear-gradient(125deg,#082e33,#157871)', accent: '#86e8d2', accentRgb: '134,232,210' },
  { id: 'ember', label: 'Ember', bg: 'radial-gradient(ellipse 70% 90% at 85% -20%, #5b2234 0, transparent 72%), radial-gradient(ellipse 55% 60% at -5% 105%, #3a1a14 0, transparent 70%), #110a0e', swatch: 'linear-gradient(125deg,#27202e,#ae5869)', accent: '#ffb3a1', accentRgb: '255,179,161' },
];

export interface BpPrefs {
  theme: BpTheme;
  reducedMotion: boolean;
}
const KEY = 'omnihub.bigpicture';
let prefs: BpPrefs = (() => {
  const d: BpPrefs = { theme: 'nebula', reducedMotion: false };
  try {
    return { ...d, ...JSON.parse(localStorage.getItem(KEY) ?? '{}') };
  } catch {
    return d;
  }
})();
const subs = new Set<() => void>();
export function setBpPrefs(patch: Partial<BpPrefs>) {
  prefs = { ...prefs, ...patch };
  try {
    localStorage.setItem(KEY, JSON.stringify(prefs));
  } catch {
    /* kept for this session */
  }
  subs.forEach((s) => s());
}
export function useBpPrefs(): BpPrefs {
  return useSyncExternalStore(
    (s) => {
      subs.add(s);
      return () => subs.delete(s);
    },
    () => prefs,
  );
}

let audio: AudioContext | null = null;
/** A soft tick (a rising chime for "play", a falling one for "back"). */
export function blip(kind: 'move' | 'play' | 'back' | 'tab') {
  if (!padPrefs().sounds) return;
  try {
    audio ??= new AudioContext();
    const t = audio.currentTime;
    const o = audio.createOscillator();
    const g = audio.createGain();
    o.type = 'sine';
    const [f1, f2, len, vol] = kind === 'move' ? [880, 990, 0.05, 0.025] : kind === 'tab' ? [700, 1050, 0.09, 0.03] : kind === 'play' ? [520, 1040, 0.35, 0.05] : [660, 440, 0.12, 0.03];
    o.frequency.setValueAtTime(f1, t);
    o.frequency.exponentialRampToValueAtTime(f2, t + len);
    g.gain.setValueAtTime(vol, t);
    g.gain.exponentialRampToValueAtTime(0.0001, t + len);
    o.connect(g).connect(audio.destination);
    o.start(t);
    o.stop(t + len + 0.02);
  } catch {
    /* no sound */
  }
}

/** The controller inside a channel (matches couchpad.rs). `face` names the button on the pad. */
export const REMOTE_LEGEND: { face: 'bottom' | 'right' | 'left' | 'top' | 'view' | 'lbrb' | 'lt' | 'ls' | 'rs' | 'dpad' | 'start'; label: string }[] = [
  { face: 'ls', label: 'Move the pointer' },
  { face: 'bottom', label: 'Click' },
  { face: 'rs', label: 'Scroll' },
  { face: 'left', label: 'Play / pause' },
  { face: 'top', label: 'Full screen' },
  { face: 'dpad', label: 'Seek and volume' },
  { face: 'right', label: 'Esc' },
  { face: 'start', label: 'Enter' },
  { face: 'lbrb', label: 'Previous / next button' },
  { face: 'lt', label: 'Back a page' },
  { face: 'view', label: 'Close and come back here' },
];
