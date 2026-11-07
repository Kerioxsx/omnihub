// The animated background behind the app for the current theme. Only
// transforms and opacity move, so the graphics card does the work; it
// pauses while the window is hidden and holds still with Reduce motion.

import type { Look } from '@shared/types';
import { useEffect, useMemo, useRef, useState } from 'react';
import { api } from '../api';
import { useAuroraLook, useTexture } from '../lib/aurora/textures';
import { cx } from '../lib/cx';
import { BLEACH, lookInfo } from '../lib/looks';
import { useSettings } from '../state/settings';

/** The same layout every time (no jumping between renders). */
function seeded(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Stars as one layer of tiny gradients: cheap, and it scales with its box. */
function starField(count: number, seed: number): string {
  const r = seeded(seed);
  return Array.from({ length: count }, () => {
    const size = r() < 0.15 ? 2 : 1;
    return `radial-gradient(${size}px ${size}px at ${(r() * 100).toFixed(2)}% ${(r() * 100).toFixed(2)}%, rgba(255,255,255,${(0.45 + r() * 0.55).toFixed(2)}) 50%, transparent 51%)`;
  }).join(',');
}

/** Falling or rising bits: where, how big, how fast. */
function particles(count: number, seed: number, minDur: number, maxDur: number) {
  const r = seeded(seed);
  return Array.from({ length: count }, (_, i) => {
    const dur = minDur + r() * (maxDur - minDur);
    return {
      key: i,
      style: {
        '--x': `${(r() * 100).toFixed(1)}%`,
        '--y': `${(30 + r() * 60).toFixed(1)}%`,
        '--s': (0.55 + r() * 0.75).toFixed(2),
        '--sway': `${(r() * 90 - 45).toFixed(0)}px`,
        '--dur': `${dur.toFixed(1)}s`,
        '--delay': `${(-r() * dur).toFixed(1)}s`,
      } as React.CSSProperties,
    };
  });
}

// ---------- Minecraft: original pixel art, drawn here ----------

const pixelArt = new Map<string, string>();

/** Draw once into a small canvas (scaled up with pixelated rendering). */
function drawPixels(key: string, w: number, h: number, paint: (px: (x: number, y: number, c: string) => void, r: () => number) => void): string {
  const done = pixelArt.get(key);
  if (done) return done;
  const canvas = document.createElement('canvas');
  canvas.width = w;
  canvas.height = h;
  const ctx = canvas.getContext('2d');
  if (!ctx) return '';
  const r = seeded(w * 31 + h);
  paint((x, y, c) => {
    ctx.fillStyle = c;
    ctx.fillRect(x, y, 1, 1);
  }, r);
  const url = canvas.toDataURL('image/png');
  pixelArt.set(key, url);
  return url;
}

const pick = (r: () => number, list: string[]) => list[Math.floor(r() * list.length)];
const GRASS = ['#5d9a33', '#6aab3b', '#78bb45', '#538a2d'];
const DIRT = ['#7b5434', '#87613f', '#6d4729', '#93704c', '#80593a'];
const STONE = ['#7c7c7c', '#888888', '#6f6f6f', '#949494'];

/** Hills of 16-px blocks: grass on top, dirt, then stone. */
function terrain(): string {
  const cols = 40;
  const rows = 7;
  return drawPixels('terrain', cols * 16, rows * 16, (px, r) => {
    let h = 3;
    for (let c = 0; c < cols; c++) {
      // A gentle random walk that comes back to where it started (it tiles).
      const left = cols - c;
      if (left <= Math.abs(h - 3)) h += h > 3 ? -1 : 1;
      else if (r() < 0.35) h = Math.min(5, Math.max(2, h + (r() < 0.5 ? -1 : 1)));
      for (let b = 0; b < h; b++) {
        const top = (rows - h + b) * 16;
        for (let y = 0; y < 16; y++) {
          for (let x = 0; x < 16; x++) {
            let col: string;
            if (b === 0 && (y < 3 || (y === 3 && r() < 0.6) || (y === 4 && r() < 0.25))) col = pick(r, GRASS);
            else if (b >= 3) col = pick(r, STONE);
            else col = pick(r, DIRT);
            px(c * 16 + x, top + y, col);
          }
        }
      }
    }
  });
}

/** Flat blocky clouds on a transparent strip (tiles sideways). */
function clouds(): string {
  const w = 200;
  const h = 26;
  return drawPixels('clouds', w, h, (px, r) => {
    const shapes = [
      [12, 5, 34, 8],
      [20, 2, 16, 5],
      [80, 9, 44, 7],
      [92, 6, 22, 4],
      [150, 3, 30, 9],
      [158, 12, 26, 4],
    ];
    for (const [x0, y0, cw, ch] of shapes) {
      for (let y = y0; y < y0 + ch; y++) for (let x = x0; x < x0 + cw; x++) px(x, y, r() < 0.06 ? '#eef2f8' : '#ffffff');
    }
  });
}

/** Pine trees, as a dark line along the bottom. */
const TREES = `url("data:image/svg+xml,${encodeURIComponent(
  `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 400 120' preserveAspectRatio='none'><g fill='#03110a'>${[
    [20, 40],
    [55, 70],
    [90, 30],
    [130, 55],
    [170, 20],
    [205, 60],
    [245, 35],
    [285, 65],
    [320, 25],
    [360, 50],
    [395, 40],
  ]
    .map(([x, top]) => `<path d='M${x} ${top} L${x - 22} 120 L${x + 22} 120 Z'/>`)
    .join('')}<rect y='104' width='400' height='16'/></g></svg>`,
)}")`;

/** Day, dusk or night by the PC's clock (looked at every few minutes). */
function useSky(): 'day' | 'dusk' | 'night' {
  const phase = (): 'day' | 'dusk' | 'night' => {
    const h = new Date().getHours() + new Date().getMinutes() / 60;
    return h >= 7 && h < 17.5 ? 'day' : (h >= 5.5 && h < 7) || (h >= 17.5 && h < 19.5) ? 'dusk' : 'night';
  };
  const [sky, setSky] = useState(phase);
  useEffect(() => {
    const t = setInterval(() => setSky(phase()), 5 * 60_000);
    return () => clearInterval(t);
  }, []);
  return sky;
}

/** Items from the user's own pack that float like dropped items (best ones first). */
const DROPS = ['diamond', 'emerald', 'golden_apple', 'apple', 'diamond_sword', 'ender_pearl', 'nether_star', 'totem_of_undying', 'gold_ingot', 'diamond_pickaxe', 'cookie', 'experience_bottle', 'grass_block', 'torch', 'cake', 'iron_ingot', 'bread', 'redstone'];

function Drop({ name, style }: { name: string; style: React.CSSProperties }) {
  const url = useTexture(name);
  if (!url) return null;
  return (
    <i className="bd-mc-drop" style={style}>
      <img src={url} alt="" draggable={false} />
    </i>
  );
}

function MinecraftLayers({ mini }: { mini: boolean }) {
  const sky = useSky();
  const { textures } = useAuroraLook();
  const ground = useMemo(() => terrain(), []);
  const cloudStrip = useMemo(() => clouds(), []);
  const stars = useMemo(() => starField(mini ? 30 : 110, 23), [mini]);
  const drops = useMemo(() => {
    if (!textures) return [];
    const names = DROPS.filter((n) => textures.names.has(n)).slice(0, mini ? 3 : 5);
    const r = seeded(5);
    return names.map((name, i) => ({
      name,
      style: {
        '--x': `${(8 + ((i + r() * 0.6) / names.length) * 84).toFixed(1)}%`,
        '--y': `${(mini ? 30 : 40) + r() * 22}%`,
        '--delay': `${(-r() * 2.6).toFixed(1)}s`,
      } as React.CSSProperties,
    }));
  }, [textures, mini]);
  const orbs = useMemo(() => particles(mini ? 4 : 10, 41, 7, 12), [mini]);
  return (
    <>
      <i className={cx('bd-mc-sky', sky)} />
      {sky === 'night' && <i className="bd-stars twinkle" style={{ backgroundImage: stars, height: '70%' }} />}
      <i className={cx('bd-mc-sun', sky === 'night' ? 'moon' : 'sun')} />
      <i className="bd-mc-clouds" style={{ backgroundImage: `url(${cloudStrip})` }} />
      <i className="bd-mc-ground" style={{ backgroundImage: `url(${ground})` }} />
      {sky !== 'night' && <i className={cx('bd-mc-veil', sky)} />}
      {drops.map((d) => (
        <Drop key={d.name} name={d.name} style={d.style} />
      ))}
      {orbs.map((p) => (
        <i key={p.key} className="bd-mc-orb" style={p.style} />
      ))}
    </>
  );
}

const GLYPHS = 'アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン0123456789Z:.=*+<>';

function codeColumns(count: number, seed: number) {
  const r = seeded(seed);
  return particles(count, seed + 1, 6, 14).map((p) => ({
    ...p,
    text: Array.from({ length: 14 + Math.floor(r() * 18) }, () => GLYPHS[Math.floor(r() * GLYPHS.length)]).join(''),
  }));
}

// ---------- Bleach: original shapes, drawn here (no artwork from the series) ----------

/** Rough manga-ink brush strokes for Hollow, as an SVG picture. */
const INK = (() => {
  const strokes = [
    'M-40 620 C 160 560, 300 470, 520 300 S 860 40, 1040 -20',
    'M180 760 C 360 660, 520 600, 700 470 S 940 280, 1080 230',
    'M-60 420 C 80 380, 160 330, 260 250',
  ];
  const widths = [26, 14, 9];
  const paths = strokes.map((d, i) => `<path d='${d}' fill='none' stroke='%2317171c' stroke-width='${widths[i]}' stroke-linecap='round' opacity='${[0.09, 0.07, 0.06][i]}'/>`).join('');
  const svg = `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1000 700' preserveAspectRatio='xMaxYMax slice'><filter id='r'><feTurbulence type='fractalNoise' baseFrequency='0.035' numOctaves='2' seed='7'/><feDisplacementMap in='SourceGraphic' scale='14'/></filter><g filter='url(%23r)'>${paths}</g></svg>`;
  return `url("data:image/svg+xml;utf8,${svg}")`;
})();

/** Black ink splatters for Hollow (original shapes), as one SVG picture. */
const SPLAT = (() => {
  const r = seeded(91);
  const blobs = Array.from({ length: 7 }, () => {
    const cx = 40 + r() * 920;
    const cy = 40 + r() * 620;
    const rad = 8 + r() * 24;
    const drops = Array.from({ length: 5 + Math.floor(r() * 6) }, () => {
      const a = r() * Math.PI * 2;
      const d = rad * (1.2 + r() * 1.8);
      return `<circle cx='${(cx + Math.cos(a) * d).toFixed(0)}' cy='${(cy + Math.sin(a) * d).toFixed(0)}' r='${(2 + r() * rad * 0.22).toFixed(1)}'/>`;
    }).join('');
    return `<circle cx='${cx.toFixed(0)}' cy='${cy.toFixed(0)}' r='${rad.toFixed(0)}'/>${drops}`;
  }).join('');
  const svg = `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1000 700' preserveAspectRatio='xMidYMid slice'><filter id='r'><feTurbulence type='fractalNoise' baseFrequency='0.05' numOctaves='2' seed='3'/><feDisplacementMap in='SourceGraphic' scale='22'/></filter><g filter='url(%23r)' fill='%2317171c'>${blobs}</g></svg>`;
  return `url("data:image/svg+xml;utf8,${svg}")`;
})();

/** Torn manga-panel edges for Hollow. */
const TORN = (() => {
  const r = seeded(17);
  const edge = (y0: number, y1: number) => Array.from({ length: 26 }, (_, i) => `${(i * 40).toFixed(0)},${(y0 + (y1 - y0) * (i / 25) + (r() - 0.5) * 18).toFixed(0)}`).join(' ');
  const svg = `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1000 700' preserveAspectRatio='none'><polygon points='0,0 1000,0 ${edge(120, 40).split(' ').reverse().join(' ')}' fill='%2317171c' opacity='0.07'/><polyline points='${edge(150, 70)}' fill='none' stroke='%2317171c' stroke-width='3' opacity='0.12'/><polygon points='0,700 1000,700 ${edge(600, 660).split(' ').reverse().join(' ')}' fill='%2317171c' opacity='0.05'/></svg>`;
  return `url("data:image/svg+xml;utf8,${svg}")`;
})();

/** Frost in a corner for Hakka no Togame: branching ice lines. */
const FROST = (() => {
  const r = seeded(5);
  const lines: string[] = [];
  const grow = (x: number, y: number, a: number, len: number, depth: number) => {
    if (depth > 5 || len < 6) return;
    const x2 = x + Math.cos(a) * len;
    const y2 = y + Math.sin(a) * len;
    lines.push(`M${x.toFixed(0)} ${y.toFixed(0)}L${x2.toFixed(0)} ${y2.toFixed(0)}`);
    grow(x2, y2, a + (r() - 0.5) * 0.5, len * 0.78, depth + 1);
    grow(x2, y2, a + 0.9 + r() * 0.3, len * 0.5, depth + 1);
    grow(x2, y2, a - 0.9 - r() * 0.3, len * 0.5, depth + 1);
  };
  for (let i = 0; i < 6; i++) grow(0, 0, 0.12 + i * 0.25, 70 + r() * 50, 0);
  const svg = `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 400 400'><path d='${lines.join('')}' fill='none' stroke='%23dff3ff' stroke-width='1.3' stroke-linecap='round' opacity='0.55'/></svg>`;
  return `url("data:image/svg+xml;utf8,${svg}")`;
})();

/** Long frost ribbons drifting through the air (Hakka no Togame). */
const RIBBONS = (() => {
  const paths = [
    'M-50 520 C 180 380, 260 640, 520 470 S 820 160, 1060 300',
    'M-80 300 C 160 190, 300 420, 560 260 S 880 60, 1080 140',
    'M120 720 C 300 560, 420 700, 640 560 S 920 380, 1080 460',
  ];
  const svg = `<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 1000 700' preserveAspectRatio='xMidYMid slice'>${paths
    .map((d, i) => `<path d='${d}' fill='none' stroke='%23e8f6ff' stroke-width='${[5, 3, 2][i]}' stroke-linecap='round' opacity='${[0.32, 0.22, 0.18][i]}'/><path d='${d}' fill='none' stroke='%23a9d8ff' stroke-width='${[14, 9, 6][i]}' opacity='0.05'/>`)
    .join('')}</svg>`;
  return `url("data:image/svg+xml;utf8,${svg}")`;
})();

type BleachLook = 'bankai' | 'hollow' | 'mugetsu' | 'hakka';

function BleachLayers({ look, mini }: { look: BleachLook; mini: boolean }) {
  const motes = useMemo(() => particles(mini ? 5 : look === 'mugetsu' ? 12 : look === 'bankai' ? 26 : 16, look === 'mugetsu' ? 53 : 47, 9, 18), [look, mini]);
  const snow = useMemo(() => (look === 'hakka' ? particles(mini ? 14 : 60, 31, 14, 28) : []), [look, mini]);
  const shards = useMemo(() => (look === 'hakka' ? particles(mini ? 3 : 9, 77, 16, 30) : []), [look, mini]);
  const fog = useMemo(() => (look === 'mugetsu' ? particles(mini ? 2 : 5, 13, 38, 70) : []), [look, mini]);
  if (look === 'bankai') {
    return (
      <>
        <i className="bd-bk-glow" />
        <i className="bd-bk-layer" />
        <i className="bd-bk-slash s1" />
        <i className="bd-bk-slash s2" />
        <i className="bd-bk-slash s3" />
        {!mini && [1, 2, 3, 4].map((n) => <i key={n} className={`bd-bk-trail t${n}`} />)}
        <i className="bd-bk-crescent" />
        {motes.map((p) => (
          <i key={p.key} className="bd-reiatsu" style={p.style} />
        ))}
        {!mini && <ThemeArt look={look} />}
        {!mini && <i className="bd-kanji">斬月</i>}
      </>
    );
  }
  if (look === 'hollow') {
    return (
      <>
        <i className="bd-ho-paper" />
        <i className="bd-ho-dots" />
        <i className="bd-ho-torn" style={{ backgroundImage: TORN }} />
        <i className="bd-ho-ink" style={{ backgroundImage: INK }} />
        <i className="bd-ho-splat" style={{ backgroundImage: SPLAT }} />
        <i className="bd-ho-stripes" />
        {!mini && <i className="bd-ho-crack" />}
        {!mini && (
          <i className="bd-ho-eyes">
            <i />
            <i />
          </i>
        )}
        {!mini && <ThemeArt look={look} />}
        {!mini && <i className="bd-kanji">虚</i>}
      </>
    );
  }
  if (look === 'hakka') {
    return (
      <>
        <i className="bd-hk-sky" />
        <i className="bd-hk-mist m1" />
        {!mini && <i className="bd-hk-mist m2" />}
        <i className="bd-hk-ribbons" style={{ backgroundImage: RIBBONS }} />
        {!mini && <i className="bd-hk-frost tl" style={{ backgroundImage: FROST }} />}
        {!mini && <i className="bd-hk-frost br" style={{ backgroundImage: FROST }} />}
        {shards.map((p) => (
          <i key={p.key} className="bd-hk-shard" style={p.style} />
        ))}
        {snow.map((p) => (
          <i key={p.key} className="bd-flake bd-hk-snow" style={p.style} />
        ))}
        <i className="bd-hk-floor" />
        {!mini && <ThemeArt look={look} />}
        {!mini && <i className="bd-kanji">白霞罸</i>}
      </>
    );
  }
  return (
    <>
      <i className="bd-mg-void" />
      {fog.map((p) => (
        <i key={p.key} className="bd-mg-fog" style={p.style} />
      ))}
      <i className="bd-mg-eclipse" />
      {!mini && <i className="bd-mg-ripple r1" />}
      {!mini && <i className="bd-mg-ripple r2" />}
      {motes.map((p) => (
        <i key={p.key} className="bd-mg-flame" style={p.style} />
      ))}
      <i className="bd-mg-absorb" />
      {!mini && <ThemeArt look={look} />}
      {!mini && <i className="bd-kanji">無月</i>}
    </>
  );
}

// ---------- your own picture behind a Bleach theme ----------

export type ArtStyle = 'tint' | 'ink' | 'color';
export interface ArtPrefs {
  style: ArtStyle;
  opacity: number;
  side: 'left' | 'center' | 'right';
}
const ART_KEY = 'omnihub.themeArt';

export function artPrefs(look: string): ArtPrefs {
  const fallback: ArtPrefs = { style: look === 'hollow' ? 'ink' : 'tint', opacity: look === 'hollow' ? 0.22 : 0.34, side: 'right' };
  try {
    return { ...fallback, ...(JSON.parse(localStorage.getItem(ART_KEY) ?? '{}')[look] ?? {}) };
  } catch {
    return fallback;
  }
}

export function setArtPrefs(look: string, patch: Partial<ArtPrefs>) {
  try {
    const all = JSON.parse(localStorage.getItem(ART_KEY) ?? '{}');
    all[look] = { ...artPrefs(look), ...patch };
    localStorage.setItem(ART_KEY, JSON.stringify(all));
  } catch {
    /* this session only */
  }
  window.dispatchEvent(new Event('omnihub:theme-art'));
}

/** Matrix that turns dark lines into the tint colour and drops the paper. */
function inkMatrix(hex: string): string {
  const n = parseInt(hex.slice(1), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => (v / 255).toFixed(3));
  return `0 0 0 0 ${r}  0 0 0 0 ${g}  0 0 0 0 ${b}  -0.45 -0.45 -0.45 1.35 0`;
}

function ThemeArt({ look }: { look: BleachLook }) {
  const [url, setUrl] = useState<string | null>(null);
  const [prefs, setPrefs] = useState<ArtPrefs>(() => artPrefs(look));
  const ref = useRef<HTMLElement>(null);
  useEffect(() => {
    let alive = true;
    const load = () => {
      setPrefs(artPrefs(look));
      void api.themeArt.get(look).then((u) => alive && setUrl(u), () => undefined);
    };
    load();
    window.addEventListener('omnihub:theme-art', load);
    return () => {
      alive = false;
      window.removeEventListener('omnihub:theme-art', load);
    };
  }, [look]);
  // A slow parallax with the pointer (skipped with reduced motion).
  useEffect(() => {
    if (!url || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    let frame = 0;
    const on = (e: PointerEvent) => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const el = ref.current;
        if (!el) return;
        el.style.setProperty('--px', `${((e.clientX / window.innerWidth - 0.5) * -18).toFixed(1)}px`);
        el.style.setProperty('--py', `${((e.clientY / window.innerHeight - 0.5) * -10).toFixed(1)}px`);
      });
    };
    window.addEventListener('pointermove', on, { passive: true });
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener('pointermove', on);
    };
  }, [url]);
  if (!url) return null;
  const mode = BLEACH.find((m) => m.id === look);
  const tint = prefs.style === 'ink' ? '#17171c' : (mode?.tint ?? '#ffffff');
  const filterId = `bd-art-${look}`;
  return (
    <i ref={ref} className={cx('bd-art', `at-${prefs.side}`, `is-${prefs.style}`)} style={{ '--art-opacity': prefs.opacity, '--art-glow': mode?.color } as React.CSSProperties}>
      <svg width="0" height="0" style={{ position: 'absolute' }} aria-hidden>
        <filter id={filterId} colorInterpolationFilters="sRGB">
          <feColorMatrix type="matrix" values={inkMatrix(tint)} />
        </filter>
      </svg>
      <img src={url} alt="" draggable={false} style={prefs.style === 'color' ? undefined : { filter: `url(#${filterId})` }} />
    </i>
  );
}

function useStill(): boolean {
  const animate = useSettings((s) => s.settings?.general.animateBackdrop ?? true);
  const reduced = useSettings((s) => s.settings?.general.reducedMotion ?? false);
  const [osReduced, setOsReduced] = useState(() => window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  useEffect(() => {
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    const on = () => setOsReduced(mq.matches);
    mq.addEventListener('change', on);
    return () => mq.removeEventListener('change', on);
  }, []);
  return !animate || reduced || osReduced;
}

/** Minimized, hidden or in the background: decorations hold still. */
function useHidden(): boolean {
  const [hidden, setHidden] = useState(() => document.visibilityState === 'hidden');
  const [blurred, setBlurred] = useState(() => !document.hasFocus());
  useEffect(() => {
    const on = () => setHidden(document.visibilityState === 'hidden');
    const focus = () => setBlurred(false);
    // A moment's grace, so a click into a menu or dialog doesn't freeze it.
    let t = 0;
    const blur = () => {
      window.clearTimeout(t);
      t = window.setTimeout(() => setBlurred(!document.hasFocus()), 1500);
    };
    document.addEventListener('visibilitychange', on);
    window.addEventListener('focus', focus);
    window.addEventListener('blur', blur);
    return () => {
      window.clearTimeout(t);
      document.removeEventListener('visibilitychange', on);
      window.removeEventListener('focus', focus);
      window.removeEventListener('blur', blur);
    };
  }, []);
  return hidden || blurred;
}

function Layers({ look, mini }: { look: Look; mini: boolean }) {
  const stars = useMemo(() => (look === 'galaxy' ? [starField(mini ? 40 : 140, 7), starField(mini ? 25 : 90, 19)] : null), [look, mini]);
  const petals = useMemo(() => (look === 'sakura' ? particles(mini ? 7 : 22, 3, 14, 26) : []), [look, mini]);
  const embers = useMemo(() => (look === 'ember' ? particles(mini ? 10 : 30, 11, 8, 15) : []), [look, mini]);
  const code = useMemo(() => (look === 'matrix' ? codeColumns(mini ? 10 : 34, 13) : []), [look, mini]);
  const drops = useMemo(() => (look === 'rain' ? particles(mini ? 18 : 70, 17, 0.7, 1.3) : []), [look, mini]);
  const flakes = useMemo(() => (look === 'snowfall' ? particles(mini ? 14 : 45, 29, 12, 24) : []), [look, mini]);
  const flies = useMemo(() => (look === 'fireflies' ? particles(mini ? 8 : 26, 31, 9, 18) : []), [look, mini]);
  const sky = useMemo(
    () =>
      look === 'clouds'
        ? particles(mini ? 3 : 6, 37, 70, 140).map((p, i, all) => ({ ...p, style: { ...p.style, '--y': `${4 + (i / all.length) * 52}%` } as React.CSSProperties }))
        : [],
    [look, mini],
  );
  switch (look) {
    case 'aurora':
      return (
        <>
          <i className="bd-ribbon r1" />
          <i className="bd-ribbon r2" />
          <i className="bd-ribbon r3" />
          <i className="bd-veil" />
        </>
      );
    case 'synthwave':
      return (
        <>
          <i className="bd-sky" />
          <i className="bd-sun" />
          <i className="bd-floor">
            <i className="bd-grid" />
          </i>
        </>
      );
    case 'ocean':
      return (
        <>
          <i className="bd-deep" />
          {[1, 2, 3].map((n) => (
            <i key={n} className={`bd-wave w${n}`} />
          ))}
        </>
      );
    case 'galaxy':
      return (
        <>
          <i className="bd-nebula n1" />
          <i className="bd-nebula n2" />
          <i className="bd-stars" style={{ backgroundImage: stars?.[0] }} />
          <i className="bd-stars twinkle" style={{ backgroundImage: stars?.[1] }} />
          {!mini && <i className="bd-shoot" />}
        </>
      );
    case 'sakura':
      return (
        <>
          <i className="bd-blush" />
          {petals.map((p) => (
            <i key={p.key} className="bd-petal" style={p.style} />
          ))}
        </>
      );
    case 'ember':
      return (
        <>
          <i className="bd-heat" />
          {embers.map((p) => (
            <i key={p.key} className="bd-spark" style={p.style} />
          ))}
        </>
      );
    case 'minecraft':
      return <MinecraftLayers mini={mini} />;
    case 'bankai':
    case 'hollow':
    case 'mugetsu':
    case 'hakka':
      return <BleachLayers look={look} mini={mini} />;
    case 'matrix':
      return (
        <>
          <i className="bd-code-glow" />
          {code.map((c) => (
            <i key={c.key} className="bd-code" style={c.style}>
              {c.text}
            </i>
          ))}
        </>
      );
    case 'rain':
      return (
        <>
          <i className="bd-storm" />
          {drops.map((p) => (
            <i key={p.key} className="bd-rain" style={p.style} />
          ))}
          {!mini && <i className="bd-flash" />}
        </>
      );
    case 'snowfall':
      return (
        <>
          <i className="bd-night" />
          <i className="bd-drift" />
          {flakes.map((p) => (
            <i key={p.key} className="bd-flake" style={p.style} />
          ))}
        </>
      );
    case 'fireflies':
      return (
        <>
          <i className="bd-woods" />
          <i className="bd-trees" style={{ backgroundImage: TREES }} />
          {flies.map((p) => (
            <i key={p.key} className="bd-fly" style={p.style} />
          ))}
        </>
      );
    case 'lava':
      return (
        <>
          <i className="bd-lavaglow" />
          {[1, 2, 3, 4].map((n) => (
            <i key={n} className={`bd-blob b${n}`} />
          ))}
        </>
      );
    case 'clouds':
      return (
        <>
          <i className="bd-daysky" />
          <i className="bd-sunglow" />
          {sky.map((p) => (
            <i key={p.key} className="bd-cloud" style={p.style} />
          ))}
        </>
      );
    default:
      return null;
  }
}

/** The app's background. `look` overrides the setting (for previews); `contained` fills its parent instead of the window. */
export function Backdrop({ look: forced, contained = false }: { look?: Look; contained?: boolean }) {
  const setting = useSettings((s) => s.settings?.general.look);
  const look = forced ?? lookInfo(setting).id;
  const still = useStill();
  const hidden = useHidden();
  return (
    <div className={cx('app-backdrop', `look-${look}`, contained && 'is-contained', still && 'is-still', hidden && 'is-paused')} aria-hidden>
      <Layers look={look} mini={contained} />
    </div>
  );
}
