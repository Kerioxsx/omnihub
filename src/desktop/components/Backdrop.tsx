// The animated background behind the app for the current theme. Only
// transforms and opacity move, so the graphics card does the work; it
// pauses while the window is hidden and holds still with Reduce motion.

import type { Look } from '@shared/types';
import { useEffect, useMemo, useState } from 'react';
import { useAuroraLook, useTexture } from '../lib/aurora/textures';
import { cx } from '../lib/cx';
import { lookInfo } from '../lib/looks';
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

function useHidden(): boolean {
  const [hidden, setHidden] = useState(() => document.visibilityState === 'hidden');
  useEffect(() => {
    const on = () => setHidden(document.visibilityState === 'hidden');
    document.addEventListener('visibilitychange', on);
    return () => document.removeEventListener('visibilitychange', on);
  }, []);
  return hidden;
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
