// The phone's look: match the PC's theme (its animated looks become still,
// matching colours), or pick one of a few elegant palettes; a photo
// wallpaper (the PC's, or one just for the phone) that fills the screen and
// sets the colours; and frosted glass over it. Applied at once from what was
// saved last time, then refreshed from the PC.

import { applyPalette, type Palette, type WallInfo } from '@shared/palette';
import { useSyncExternalStore } from 'react';
import { getToken } from '../client';
import { getThemePref } from './theme';

export type Preset = 'classic' | 'midnight' | 'aurora' | 'rose' | 'ocean' | 'graphite' | 'ember' | 'pearl';
export interface PhoneLook {
  /** 'pc' follows the PC's theme. */
  theme: 'pc' | Preset;
  wallpaper: 'none' | 'pc' | 'mine';
  /** The colours come from the wallpaper when there is one. */
  fromWallpaper: boolean;
  glass: boolean;
}
export interface PcLook {
  look: string;
  accent: string;
  theme: 'system' | 'dark' | 'light';
  glass: boolean;
  wallpapers: { pc: WallInfo | null; phone: WallInfo | null };
}

type Swatch = { accent: string; accent2: string; light: boolean; bg: string };

const g = (a: string, b: string, base: string) => `radial-gradient(120% 70% at 0% -10%, ${a}, transparent 62%), radial-gradient(100% 60% at 110% 105%, ${b}, transparent 60%), ${base}`;

export const PRESETS: Record<Preset, Swatch & { name: string }> = {
  classic: { name: 'Classic', accent: '#8b5cf6', accent2: '#22d3ee', light: false, bg: g('rgba(124,58,237,.22)', 'rgba(6,182,212,.14)', '#0b0b12') },
  midnight: { name: 'Midnight', accent: '#a5b4fc', accent2: '#c4b5fd', light: false, bg: g('rgba(99,102,241,.32)', 'rgba(147,51,234,.22)', '#070812') },
  aurora: { name: 'Aurora', accent: '#5eead4', accent2: '#86efac', light: false, bg: g('rgba(20,184,166,.32)', 'rgba(34,197,94,.18)', '#04110f') },
  rose: { name: 'Rose gold', accent: '#f9a8d4', accent2: '#fdba74', light: false, bg: g('rgba(236,72,153,.28)', 'rgba(251,146,60,.2)', '#140a10') },
  ocean: { name: 'Ocean', accent: '#7dd3fc', accent2: '#818cf8', light: false, bg: g('rgba(14,165,233,.3)', 'rgba(79,70,229,.22)', '#050c18') },
  graphite: { name: 'Graphite', accent: '#e2e8f0', accent2: '#94a3b8', light: false, bg: g('rgba(148,163,184,.18)', 'rgba(100,116,139,.14)', '#0a0b0d') },
  ember: { name: 'Ember', accent: '#fdba74', accent2: '#fca5a5', light: false, bg: g('rgba(249,115,22,.3)', 'rgba(239,68,68,.18)', '#120905') },
  pearl: { name: 'Pearl', accent: '#7c3aed', accent2: '#0891b2', light: true, bg: g('rgba(196,181,253,.45)', 'rgba(165,243,252,.4)', '#f7f6fb') },
};

/** The PC's themes, as still colours for the phone. */
const PC_LOOKS: Record<string, Swatch> = {
  aurora: PRESETS.aurora,
  synthwave: { accent: '#f472b6', accent2: '#a78bfa', light: false, bg: g('rgba(236,72,153,.3)', 'rgba(124,58,237,.3)', '#0d0618') },
  ocean: PRESETS.ocean,
  galaxy: { accent: '#c4b5fd', accent2: '#93c5fd', light: false, bg: g('rgba(139,92,246,.3)', 'rgba(59,130,246,.2)', '#05040e') },
  sakura: { accent: '#db2777', accent2: '#9333ea', light: true, bg: g('rgba(251,207,232,.7)', 'rgba(233,213,255,.6)', '#fff7fb') },
  ember: PRESETS.ember,
  minecraft: { accent: '#4ade80', accent2: '#60a5fa', light: false, bg: g('rgba(74,222,128,.22)', 'rgba(96,165,250,.25)', '#0a1410') },
  matrix: { accent: '#4ade80', accent2: '#22c55e', light: false, bg: g('rgba(34,197,94,.22)', 'rgba(21,128,61,.2)', '#020803') },
  rain: { accent: '#93c5fd', accent2: '#a5b4fc', light: false, bg: g('rgba(71,85,105,.4)', 'rgba(59,130,246,.18)', '#070a10') },
  snowfall: { accent: '#bae6fd', accent2: '#c7d2fe', light: false, bg: g('rgba(148,163,184,.3)', 'rgba(125,211,252,.2)', '#070b14') },
  fireflies: { accent: '#fde047', accent2: '#86efac', light: false, bg: g('rgba(132,204,22,.2)', 'rgba(250,204,21,.14)', '#050a05') },
  lava: { accent: '#fb923c', accent2: '#f472b6', light: false, bg: g('rgba(249,115,22,.32)', 'rgba(236,72,153,.26)', '#140608') },
  clouds: { accent: '#2563eb', accent2: '#0ea5e9', light: true, bg: g('rgba(191,219,254,.8)', 'rgba(224,242,254,.9)', '#f0f7ff') },
  bankai: { accent: '#ff7a1f', accent2: '#ffb36b', light: false, bg: g('rgba(255,101,0,.32)', 'rgba(255,60,0,.16)', '#0b0604') },
  hollow: { accent: '#d11f30', accent2: '#17171c', light: true, bg: g('rgba(209,31,48,.12)', 'rgba(23,23,28,.08)', '#f4f1ea') },
  mugetsu: { accent: '#a066e0', accent2: '#6d28d9', light: false, bg: g('rgba(142,74,201,.3)', 'rgba(30,10,50,.5)', '#050307') },
  soul: { accent: '#2f6fd8', accent2: '#c98a1e', light: true, bg: g('rgba(191,224,255,.9)', 'rgba(255,240,214,.9)', '#f3f8ff') },
  hueco: { accent: '#9fd0c8', accent2: '#e8e4d6', light: false, bg: g('rgba(232,228,214,.16)', 'rgba(159,208,200,.14)', '#07080c') },
  karakura: { accent: '#6ea8ff', accent2: '#ffd27a', light: false, bg: g('rgba(60,80,180,.34)', 'rgba(255,210,122,.12)', '#080b1d') },
  senbon: { accent: '#ff7ab8', accent2: '#c9a0ff', light: false, bg: g('rgba(255,122,184,.26)', 'rgba(140,60,160,.24)', '#0c0711') },
  hakka: { accent: '#8fd3ff', accent2: '#bfe6ff', light: false, bg: g('rgba(143,211,255,.26)', 'rgba(191,230,255,.14)', '#060b12') },
};
const NAMED: Record<string, [string, string]> = { violet: ['#8b5cf6', '#22d3ee'], cyan: ['#06b6d4', '#a78bfa'], rose: ['#f43f5e', '#fb923c'], emerald: ['#10b981', '#38bdf8'], amber: ['#f59e0b', '#f472b6'] };

const PREFS_KEY = 'omnihub.look';
const PC_KEY = 'omnihub.pcLook';
const DEFAULT: PhoneLook = { theme: 'pc', wallpaper: 'pc', fromWallpaper: true, glass: true };

function read<T>(key: string, fallback: T): T {
  try {
    const v = localStorage.getItem(key);
    return v ? { ...fallback, ...JSON.parse(v) } : fallback;
  } catch {
    return fallback;
  }
}

let prefs: PhoneLook = read(PREFS_KEY, DEFAULT);
let pc: PcLook | null = read<PcLook | null>(PC_KEY, null);
let wallUrl: string | null = null;
let wallKey = '';
const subs = new Set<() => void>();
const notify = () => subs.forEach((s) => s());

export interface Resolved {
  palette: Palette;
  bg: string;
  /** Which wallpaper to show (null: none). */
  wall: WallInfo | null;
}

const media = typeof window !== 'undefined' && window.matchMedia ? window.matchMedia('(prefers-color-scheme: light)') : null;

export function resolve(p: PhoneLook = prefs, l: PcLook | null = pc): Resolved {
  const wall = p.wallpaper === 'mine' ? (l?.wallpapers.phone ?? null) : p.wallpaper === 'pc' ? (l?.wallpapers.pc ?? null) : null;
  let s: Swatch;
  if (p.theme !== 'pc') {
    s = PRESETS[p.theme];
    if (p.theme === 'classic') {
      const t = getThemePref();
      const light = t === 'light' || (t === 'system' && !!media?.matches);
      s = light ? { ...s, light, accent: '#7c3aed', bg: g('rgba(124,58,237,.1)', 'rgba(6,182,212,.08)', '#f5f5fa') } : s;
    }
  } else if (!l) {
    s = PRESETS.classic;
  } else if (l.look === 'photo' && l.wallpapers.pc) {
    const w = l.wallpapers.pc.palette;
    s = { accent: w.accent, accent2: w.accent2, light: w.light, bg: g(`${w.accent}44`, `${w.accent2}33`, w.tint) };
  } else if (l.look === 'classic' || !PC_LOOKS[l.look]) {
    const [a, b] = NAMED[l.accent] ?? NAMED.violet;
    const light = l.theme === 'light' || (l.theme === 'system' && !!media?.matches);
    s = { accent: a, accent2: b, light, bg: light ? g(`${a}22`, `${b}1c`, '#f5f5fa') : g(`${a}38`, `${b}24`, '#0b0b12') };
  } else {
    s = PC_LOOKS[l.look];
  }
  if (wall && p.fromWallpaper) {
    const w = wall.palette;
    s = { accent: w.accent, accent2: w.accent2, light: w.light, bg: w.tint };
  }
  const tint = s.light ? '#f5f5fa' : '#0b0b12';
  return { palette: { accent: s.accent, accent2: s.accent2, light: s.light, tint }, bg: s.bg, wall };
}

/** Put the look on the page. */
export function applyLook() {
  const r = resolve();
  const root = document.documentElement;
  root.classList.toggle('light', r.palette.light);
  root.style.colorScheme = r.palette.light ? 'light' : 'dark';
  applyPalette(r.palette);
  root.style.setProperty('--m-bg', r.bg);
  // Text on the gradient: dark when the colours are pale.
  const lum = (h: string) => {
    const n = parseInt(h.slice(1), 16);
    return (0.2126 * ((n >> 16) & 255) + 0.7152 * ((n >> 8) & 255) + 0.0722 * (n & 255)) / 255;
  };
  root.style.setProperty('--on-grad', (lum(r.palette.accent) + lum(r.palette.accent2)) / 2 > 0.62 ? '#10101a' : '#ffffff');
  // Buttons and bars use the look's two colours (Classic keeps the OmniHub gradient).
  if (prefs.theme === 'classic' && !r.wall) root.style.removeProperty('--grad');
  else root.style.setProperty('--grad', `linear-gradient(135deg, ${r.palette.accent} 0%, ${r.palette.accent2} 100%)`);
  root.toggleAttribute('data-glass', prefs.glass);
  root.toggleAttribute('data-wall', !!r.wall);
  document.querySelector('meta[name="theme-color"]')?.setAttribute('content', r.palette.light ? '#f5f5fa' : '#0b0b12');
  void loadWall(r.wall);
}

export function getLook(): PhoneLook {
  return prefs;
}

export function setLook(patch: Partial<PhoneLook>) {
  prefs = { ...prefs, ...patch };
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
  } catch {
    /* this session only */
  }
  applyLook();
  notify();
}

/** Ask the PC for its theme and wallpapers (quietly keeps the last one). */
export async function refreshLook() {
  if (!getToken()) return;
  try {
    const res = await fetch('/api/look', { headers: { authorization: `Bearer ${getToken()}` } });
    if (!res.ok) return;
    pc = (await res.json()) as PcLook;
    try {
      localStorage.setItem(PC_KEY, JSON.stringify(pc));
    } catch {
      /* fine */
    }
    applyLook();
    notify();
  } catch {
    /* offline: keep the saved look */
  }
}

/** The wallpaper picture, kept in the browser's cache so it shows instantly next time. */
async function loadWall(w: WallInfo | null) {
  const key = w ? `${w.slot}:${w.version}` : '';
  if (key === wallKey) return;
  wallKey = key;
  if (!w) {
    setWall(null);
    return;
  }
  const cacheKey = `/__wall/${w.slot}/${w.version}`;
  try {
    const cache = 'caches' in window ? await caches.open('omnihub-wall') : null;
    let res = cache ? await cache.match(cacheKey) : undefined;
    if (!res) {
      const got = await fetch(`/api/wallpaper/${w.slot}?v=${w.version}`, { headers: { authorization: `Bearer ${getToken()}` } });
      if (!got.ok) return;
      if (cache) {
        // Only the newest picture of each slot is kept.
        for (const k of await cache.keys()) if (new URL(k.url).pathname.startsWith(`/__wall/${w.slot}/`)) await cache.delete(k);
        await cache.put(cacheKey, got.clone());
      }
      res = got;
    }
    if (wallKey !== key) return;
    setWall(URL.createObjectURL(await res.blob()));
  } catch {
    /* no picture this time */
  }
}

function setWall(url: string | null) {
  if (wallUrl) URL.revokeObjectURL(wallUrl);
  wallUrl = url;
  notify();
}

export function useLook(): { prefs: PhoneLook; pc: PcLook | null; wallUrl: string | null; resolved: Resolved } {
  useSyncExternalStore(
    (s) => {
      subs.add(s);
      return () => subs.delete(s);
    },
    () => `${JSON.stringify(prefs)}|${pc ? JSON.stringify(pc) : ''}|${wallUrl ?? ''}`,
  );
  return { prefs, pc, wallUrl, resolved: resolve() };
}

/** A photo from this phone as its own wallpaper (iPhone HEIC photos are turned into JPEG first). */
export async function uploadPhoneWallpaper(file: File): Promise<WallInfo> {
  let body: Blob = file;
  if (!/^image\/(jpeg|png|webp|gif)$/.test(file.type)) body = await toJpeg(file);
  const res = await fetch('/api/wallpaper/phone', { method: 'POST', headers: { authorization: `Bearer ${getToken()}`, 'content-type': body.type || 'application/octet-stream' }, body });
  const j = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error((j as { error?: string }).error ?? `HTTP ${res.status}`);
  await refreshLook();
  return j as WallInfo;
}

export async function removePhoneWallpaper() {
  await fetch('/api/wallpaper/phone', { method: 'DELETE', headers: { authorization: `Bearer ${getToken()}` } });
  await refreshLook();
}

async function toJpeg(file: File): Promise<Blob> {
  const url = URL.createObjectURL(file);
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    const k = Math.min(1, 8000 / Math.max(img.naturalWidth, img.naturalHeight));
    const c = document.createElement('canvas');
    c.width = Math.round(img.naturalWidth * k);
    c.height = Math.round(img.naturalHeight * k);
    c.getContext('2d')?.drawImage(img, 0, 0, c.width, c.height);
    return await new Promise<Blob>((ok, bad) => c.toBlob((b) => (b ? ok(b) : bad(new Error('This phone can’t read that photo'))), 'image/jpeg', 0.95));
  } finally {
    URL.revokeObjectURL(url);
  }
}

export function watchLook() {
  media?.addEventListener?.('change', () => applyLook());
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') void refreshLook();
  });
}
