// What Aurora needs to know about a cover to stage it: where the interesting
// part is (so a cover cropped to fill a wide screen keeps it), which bands are
// calm enough to carry lyrics, how bright they are (how much shadow the text
// needs), and the lyric colours, taken from the cover itself.

import { albumPalette, type Lab, labToRgb, rgbToHex } from './palette';

export interface CoverLook {
  /** 0 (top) – 1 (bottom): where the detail is, for cropping. */
  focusY: number;
  /** Per row of the cover (48 rows), 0–1: how busy it is. */
  busy: number[];
  /** Per row, 0–1: how bright it is. */
  light: number[];
  /** The lyrics' colour: a light tone of the cover's own colour. */
  ink: string;
  /** A deep tone of it, for the glow and the text's shadow. */
  inkDeep: string;
}

const ROWS = 48;
const chroma = ([, a, b]: Lab) => Math.hypot(a, b);

/** The lyric colours from the cover's palette (most striking colours first). */
export function lyricInk(palette: Lab[]): { ink: string; inkDeep: string } {
  if (!palette.length) return { ink: '#ffffff', inkDeep: '#1a1030' };
  // The most colourful of the cover's main colours, if any has real colour:
  // a pink cover gives pink lyrics, a sepia one cream, a grey one white.
  const top = palette.slice(0, 3);
  const pick = top.reduce((a, b) => (chroma(b) > chroma(a) ? b : a));
  const c = chroma(pick);
  const hue = c > 1e-4 ? [pick[1] / c, pick[2] / c] : [0, 0];
  const at = (L: number, ch: number) => rgbToHex(labToRgb([L, hue[0] * ch, hue[1] * ch]));
  return { ink: at(0.9, Math.min(0.11, Math.max(c > 0.02 ? 0.035 : 0, c * 0.8))), inkDeep: at(0.36, Math.min(0.15, Math.max(0.03, c))) };
}

function load(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.decoding = 'async';
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error('cover did not load'));
    img.src = url;
  });
}

const cache = new Map<string, Promise<CoverLook>>();

/** Study a cover (cached per picture). */
export function coverLook(url: string): Promise<CoverLook> {
  let p = cache.get(url);
  if (!p) {
    p = study(url);
    if (cache.size > 20) cache.clear();
    cache.set(url, p);
  }
  return p;
}

async function study(url: string): Promise<CoverLook> {
  const [img, palette] = await Promise.all([load(url), albumPalette(url).catch(() => [] as Lab[])]);
  const n = ROWS;
  const canvas = document.createElement('canvas');
  canvas.width = canvas.height = n;
  const g = canvas.getContext('2d', { willReadFrequently: true });
  if (!g) return { focusY: 0.45, busy: Array(n).fill(0.5), light: Array(n).fill(0.5), ...lyricInk(palette) };
  g.drawImage(img, 0, 0, n, n);
  const d = g.getImageData(0, 0, n, n).data;
  const lum = new Float32Array(n * n);
  for (let i = 0; i < n * n; i++) lum[i] = (0.2126 * d[i * 4] + 0.7152 * d[i * 4 + 1] + 0.0722 * d[i * 4 + 2]) / 255;
  const busy: number[] = [];
  const light: number[] = [];
  for (let y = 0; y < n; y++) {
    let e = 0;
    let l = 0;
    for (let x = 0; x < n; x++) {
      const v = lum[y * n + x];
      l += v;
      if (x > 0) e += Math.abs(v - lum[y * n + x - 1]);
      if (y > 0) e += Math.abs(v - lum[(y - 1) * n + x]);
    }
    busy.push(e / n);
    light.push(l / n);
  }
  const top = Math.max(1e-6, ...busy);
  const norm = busy.map((b) => b / top);
  // Where the detail is: the centre of the busiest rows, never far from the middle.
  let w = 0;
  let sum = 0;
  norm.forEach((b, y) => {
    const k = b * b;
    w += k;
    sum += k * ((y + 0.5) / n);
  });
  const focusY = Math.min(0.7, Math.max(0.3, w > 0 ? sum / w : 0.5));
  return { focusY, busy: norm, light, ...lyricInk(palette) };
}

export type Place = 'upper' | 'center' | 'lower';
/** Where each place puts the lyrics' middle, as a share of the height. */
export const PLACE_TOP: Record<Place, number> = { upper: 0.28, center: 0.48, lower: 0.7 };

/**
 * The calmest place for lyrics over a cover that fills a screen `aspect`
 * (width / height) wide, cropped around its focus: the band with the least
 * detail and glare, the middle preferred when it is close.
 */
export function calmPlace(look: CoverLook, aspect: number): Place {
  const n = look.busy.length;
  // The share of the cover's height that shows, and where it starts.
  const shown = aspect >= 1 ? Math.min(1, 1 / aspect) : 1;
  const start = (1 - shown) * look.focusY;
  const cost = (p: Place) => {
    const mid = start + PLACE_TOP[p] * shown;
    const half = 0.09 * shown;
    let b = 0;
    let l = 0;
    let k = 0;
    for (let y = Math.max(0, Math.floor((mid - half) * n)); y <= Math.min(n - 1, Math.ceil((mid + half) * n)); y++) {
      b += look.busy[y];
      l += look.light[y];
      k += 1;
    }
    return (k ? b / k + 0.45 * (l / k) : 1) - (p === 'center' ? 0.08 : 0);
  };
  return (['center', 'lower', 'upper'] as Place[]).reduce((a, b) => (cost(b) < cost(a) ? b : a));
}

/** How much shadow the text needs where it sits (0 dark cover – 1 bright). */
export function glareAt(look: CoverLook, aspect: number, place: Place): number {
  const shown = aspect >= 1 ? Math.min(1, 1 / aspect) : 1;
  const mid = (1 - shown) * look.focusY + PLACE_TOP[place] * shown;
  const y = Math.min(look.light.length - 1, Math.max(0, Math.round(mid * look.light.length)));
  return look.light[y];
}
