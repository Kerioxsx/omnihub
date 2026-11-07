// Colours for Aurora: picked from the cover art (or the user's own), in
// OKLab, a colour space where equal steps look equal, so palettes blend
// from one song to the next without muddy or flashing in-between colours.

import type { VisualSettings } from '@shared/types';

export type Rgb = [number, number, number]; // sRGB 0–1
export type Lab = [number, number, number]; // OKLab

const toLinear = (c: number) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4);
const toSrgb = (c: number) => (c <= 0.0031308 ? 12.92 * c : 1.055 * c ** (1 / 2.4) - 0.055);

export function rgbToLab([r, g, b]: Rgb): Lab {
  const [lr, lg, lb] = [toLinear(r), toLinear(g), toLinear(b)];
  const l = Math.cbrt(0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb);
  const m = Math.cbrt(0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb);
  const s = Math.cbrt(0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb);
  return [0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s, 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s, 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s];
}

export function labToRgb([L, a, b]: Lab): Rgb {
  const l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const c = (v: number) => Math.min(1, Math.max(0, toSrgb(v)));
  return [c(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s), c(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s), c(-0.0041960863 * l - 0.7034186147 * m + 1.707614701 * s)];
}

export function hexToRgb(hex: string): Rgb {
  const h = hex.replace('#', '');
  const full = h.length === 3 ? h.replace(/./g, (c) => c + c) : h.padEnd(6, '0').slice(0, 6);
  const n = parseInt(full, 16);
  return Number.isNaN(n) ? [1, 1, 1] : [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}

export function rgbToHex([r, g, b]: Rgb): string {
  return `#${[r, g, b]
    .map((v) =>
      Math.round(Math.min(1, Math.max(0, v)) * 255)
        .toString(16)
        .padStart(2, '0'),
    )
    .join('')}`;
}

const chroma = ([, a, b]: Lab) => Math.hypot(a, b);

/** Scale the chroma (vividness) and keep the colour bright enough to glow. */
export function tune(lab: Lab, vividness: number, minL = 0.5): Lab {
  const [L, a, b] = lab;
  return [Math.min(0.92, Math.max(minL, L)), a * vividness, b * vividness];
}

// ---------- from the cover art ----------

const cache = new Map<string, Promise<Lab[]>>();

/** The cover's main colours, most striking first (3–5 of them). */
export function albumPalette(url: string): Promise<Lab[]> {
  let p = cache.get(url);
  if (!p) {
    p = extract(url);
    if (cache.size > 30) cache.clear();
    cache.set(url, p);
  }
  return p;
}

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.decoding = 'async';
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error('cover art did not load'));
    img.src = url;
  });
}

async function extract(url: string): Promise<Lab[]> {
  const img = await loadImage(url);
  const size = 48;
  const canvas = document.createElement('canvas');
  canvas.width = size;
  canvas.height = size;
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  if (!ctx) return [];
  ctx.drawImage(img, 0, 0, size, size);
  const data = ctx.getImageData(0, 0, size, size).data;
  const px: Lab[] = [];
  for (let i = 0; i < data.length; i += 4) {
    if (data[i + 3] < 128) continue;
    px.push(rgbToLab([data[i] / 255, data[i + 1] / 255, data[i + 2] / 255]));
  }
  return paletteOf(px);
}

/** k-means in OKLab, then the clusters ranked by size and colourfulness. */
export function paletteOf(px: Lab[], k = 6): Lab[] {
  if (!px.length) return [];
  // Start from pixels spread through the image in order of chroma (stable).
  const sorted = [...px].sort((a, b) => chroma(b) - chroma(a));
  let centers: Lab[] = Array.from({ length: Math.min(k, sorted.length) }, (_, i) => sorted[Math.floor((i * (sorted.length - 1)) / Math.max(1, k - 1))]);
  let counts: number[] = [];
  for (let iter = 0; iter < 10; iter++) {
    const sums = centers.map(() => [0, 0, 0]);
    counts = centers.map(() => 0);
    for (const p of px) {
      let best = 0;
      let bd = Infinity;
      centers.forEach((c, i) => {
        const d = (p[0] - c[0]) ** 2 + (p[1] - c[1]) ** 2 + (p[2] - c[2]) ** 2;
        if (d < bd) {
          bd = d;
          best = i;
        }
      });
      sums[best][0] += p[0];
      sums[best][1] += p[1];
      sums[best][2] += p[2];
      counts[best] += 1;
    }
    centers = centers.map((c, i) => (counts[i] ? [sums[i][0] / counts[i], sums[i][1] / counts[i], sums[i][2] / counts[i]] : c));
  }
  const total = px.length;
  const ranked = centers
    .map((c, i) => ({ c, share: counts[i] / total }))
    .filter((x) => x.share > 0.015)
    // Colourful clusters glow better than grey ones; very dark or white ones least.
    .map((x) => ({
      ...x,
      score: Math.sqrt(x.share) * (0.15 + chroma(x.c) * 6) * (x.c[0] < 0.18 || x.c[0] > 0.95 ? 0.35 : 1),
    }))
    .sort((a, b) => b.score - a.score);
  // Drop near-duplicates.
  const out: Lab[] = [];
  for (const r of ranked) {
    if (out.every((o) => Math.hypot(o[0] - r.c[0], o[1] - r.c[1], o[2] - r.c[2]) > 0.08)) out.push(r.c);
    if (out.length === 5) break;
  }
  return out.length ? out : [ranked[0]?.c ?? [0.6, 0.1, -0.1]];
}

// ---------- what to draw ----------

/** Four colours for the renderers, from the colour mode. */
export function targetPalette(color: VisualSettings['color'], album: Lab[] | null): Lab[] {
  const user = (hexes: string[]) => hexes.map((h) => rgbToLab(hexToRgb(h)));
  let base: Lab[];
  switch (color.mode) {
    case 'single':
      base = user([color.primary]);
      break;
    case 'duo':
      base = user([color.primary, color.secondary]);
      break;
    case 'multi':
      base = user(color.colors.length ? color.colors : [color.primary]);
      break;
    default:
      base = album?.length ? album : user(color.colors);
  }
  // Grey covers still glow: lift their chroma a little toward the accent.
  const lifted = base.map((c) => (color.mode === 'album' && chroma(c) < 0.03 ? ([c[0], c[1] + 0.04, c[2] - 0.06] as Lab) : c));
  const four = [0, 1, 2, 3].map((i) => lifted[i % lifted.length]);
  return four.map((c) => tune(c, color.vividness));
}

const mixLab = (a: Lab, b: Lab, t: number): Lab => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];

/** At least `min` chroma (neon edges never go grey), bright enough to glow. */
function neon(c: Lab, vividness: number, min = 0.16): Lab {
  const ch = chroma(c);
  const k = ch > 1e-4 ? Math.max(1, min / ch) : 1;
  const base: Lab = ch > 1e-4 ? [c[0], c[1] * k, c[2] * k] : [c[0], 0.12, -0.12];
  return tune(base, vividness, 0.6);
}

/**
 * The edge light's colours. Album mode: the playing song's cover, its most
 * colourful colours made vivid enough to glow (purple–magenta, the user's
 * primary and secondary, when a song has no cover). Other modes: the
 * user's colours.
 */
export function glowPalette(color: VisualSettings['color'], album: Lab[] | null): Lab[] {
  const user = [color.primary, color.secondary].map((h) => rgbToLab(hexToRgb(h)));
  let pair: Lab[];
  if (color.mode === 'album') {
    const vivid = album?.length ? [...album].sort((a, b) => chroma(b) - chroma(a)) : [];
    pair = vivid.length ? [vivid[0], vivid[1] ?? mixLab(vivid[0], user[1], 0.35), vivid[2] ?? vivid[0]] : user;
  } else {
    pair = targetPalette(color, album).slice(0, color.mode === 'multi' ? 4 : 2);
  }
  return [0, 1, 2, 3].map((i) => neon(pair[i % pair.length], color.vividness));
}

/** Eases the drawn palette toward the target, in OKLab (about a second). */
export class PaletteBlender {
  current: Lab[] | null = null;
  target: Lab[] = [];

  set(target: Lab[]) {
    this.target = target;
    if (!this.current) this.current = target.map((c) => [...c] as Lab);
  }

  /** `dt` seconds; returns sRGB colours as a flat array of 4×3 numbers. */
  step(dt: number, seconds = 1.2): Float32Array {
    const out = new Float32Array(12);
    if (!this.current) return out;
    const k = 1 - Math.exp(-dt / Math.max(0.05, seconds / 3));
    this.current = this.current.map((c, i) => {
      const t = this.target[i] ?? c;
      return [c[0] + (t[0] - c[0]) * k, c[1] + (t[1] - c[1]) * k, c[2] + (t[2] - c[2]) * k] as Lab;
    });
    this.current.forEach((c, i) => out.set(labToRgb(c), i * 3));
    return out;
  }

  /** The most vivid colour, bright enough to carry white text (word box, pill, glow). */
  accent(): string {
    const cs = this.current ?? this.target;
    if (!cs.length) return '#c026d3';
    const best = cs.reduce((a, b) => (chroma(b) > chroma(a) ? b : a));
    return rgbToHex(labToRgb([Math.min(0.72, Math.max(0.55, best[0])), best[1], best[2]]));
  }
}
