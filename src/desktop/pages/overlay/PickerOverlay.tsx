// The colour picker and ruler, on a frozen copy of the screen (the same
// overlay window as region screenshots). Click to pick a colour (copied as
// hex), drag to measure in screen pixels (Shift keeps the line straight),
// arrow keys nudge one pixel, Esc or right-click closes.

import type { PendingRegion } from '@shared/types';
import { Check, Pipette, X } from 'lucide-react';
import { type MouseEvent, useCallback, useEffect, useRef, useState } from 'react';
import { api, errorText, inTauri } from '../../api';
import { useEvent } from '../../lib/hooks';
import { closeCurrentWindow } from '../../lib/util';
import { RegionOverlay } from './RegionOverlay';
import { TranslateOverlay } from './TranslateOverlay';

export const PICKED_KEY = 'omnihub.pickedColors';
const ZOOM = 14;
const CELLS = 11;

export const hex = (r: number, g: number, b: number) => `#${[r, g, b].map((v) => v.toString(16).padStart(2, '0')).join('')}`.toUpperCase();

export function hsl(r: number, g: number, b: number): string {
  const [R, G, B] = [r / 255, g / 255, b / 255];
  const max = Math.max(R, G, B);
  const min = Math.min(R, G, B);
  const l = (max + min) / 2;
  const d = max - min;
  let h = 0;
  const s = d === 0 ? 0 : d / (1 - Math.abs(2 * l - 1));
  if (d !== 0) h = max === R ? ((G - B) / d) % 6 : max === G ? (B - R) / d + 2 : (R - G) / d + 4;
  return `hsl(${Math.round((h * 60 + 360) % 360)}, ${Math.round(s * 100)}%, ${Math.round(l * 100)}%)`;
}

export function readPicked(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(PICKED_KEY) ?? '[]') as unknown;
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string' && /^#[0-9A-F]{6}$/.test(x)).slice(0, 12) : [];
  } catch {
    return [];
  }
}

function remember(c: string) {
  try {
    localStorage.setItem(PICKED_KEY, JSON.stringify([c, ...readPicked().filter((x) => x !== c)].slice(0, 12)));
  } catch {
    /* private window */
  }
}

/** The overlay window shows the picker or the region capture, as asked. */
export function ScreenOverlay() {
  const [mode, setMode] = useState<'region' | 'pick' | 'translate' | null>(null);
  const load = useCallback(() => void api.shots.overlayMode().then(setMode, () => setMode('region')), []);
  useEffect(load, [load]);
  useEvent<unknown>('region:pending', load);
  if (!mode) return null;
  return mode === 'pick' ? <PickerOverlay /> : mode === 'translate' ? <TranslateOverlay /> : <RegionOverlay />;
}

export function PickerOverlay() {
  const [pending, setPending] = useState<PendingRegion | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [pos, setPos] = useState<{ x: number; y: number } | null>(null);
  const [line, setLine] = useState<{
    x1: number;
    y1: number;
    x2: number;
    y2: number;
  } | null>(null);
  const [picked, setPicked] = useState<string[]>([]);
  const [flash, setFlash] = useState<string | null>(null);
  const img = useRef<HTMLImageElement>(null);
  const pixels = useRef<CanvasRenderingContext2D | null>(null);
  const loupe = useRef<HTMLCanvasElement>(null);
  const drag = useRef<{ x: number; y: number; moved: boolean } | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        let p = await api.shots.regionPending();
        if (!p && !inTauri) {
          await api.shots.toolBegin('pick');
          p = await api.shots.regionPending();
        }
        setPending(p);
        if (!p) setError('No frozen screen to pick from');
      } catch (e) {
        setError(errorText(e));
      }
    })();
  }, []);

  const close = useCallback(async () => {
    await api.shots.regionCancel().catch(() => undefined);
    void closeCurrentWindow('#/tools?tab=images');
  }, []);

  const scale = () => {
    const el = img.current;
    return {
      sx: el?.naturalWidth ? el.naturalWidth / window.innerWidth : 1,
      sy: el?.naturalHeight ? el.naturalHeight / window.innerHeight : 1,
    };
  };
  const at = (x: number, y: number) => {
    const { sx, sy } = scale();
    return { px: Math.floor(x * sx), py: Math.floor(y * sy) };
  };
  // The preview is a JPEG, so colours shown and copied come from the exact
  // capture (asked for one pixel at a time); the preview only fills in while
  // that answer is on its way, and in the browser preview.
  const exact = useRef(new Map<string, [number, number, number]>());
  const asking = useRef(false);
  const [, setTick] = useState(0);
  const colour = (x: number, y: number) => {
    const { px, py } = at(x, y);
    const e = exact.current.get(`${px},${py}`);
    if (e) return { r: e[0], g: e[1], b: e[2] };
    const ctx = pixels.current;
    if (!ctx) return null;
    const d = ctx.getImageData(px, py, 1, 1).data;
    return { r: d[0], g: d[1], b: d[2] };
  };
  const latest = useRef<{ px: number; py: number } | null>(null);
  const fetchExact = useCallback(async () => {
    if (asking.current || !inTauri) return;
    asking.current = true;
    try {
      for (let i = 0; i < 4 && latest.current; i++) {
        const { px, py } = latest.current;
        const key = `${px},${py}`;
        if (exact.current.has(key)) break;
        const v = await api.shots.regionPixel(px, py).catch(() => null);
        if (!v) break;
        if (exact.current.size > 5000) exact.current.clear();
        exact.current.set(key, v);
        setTick((t) => t + 1);
      }
    } finally {
      asking.current = false;
    }
  }, []);
  useEffect(() => {
    if (!pos) return;
    latest.current = at(pos.x, pos.y);
    void fetchExact();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pos, fetchExact]);

  const onLoad = () => {
    const el = img.current;
    if (!el) return;
    const c = document.createElement('canvas');
    c.width = el.naturalWidth;
    c.height = el.naturalHeight;
    const ctx = c.getContext('2d', { willReadFrequently: true });
    ctx?.drawImage(el, 0, 0);
    pixels.current = ctx;
  };

  // Redraw the magnifier around the pointer.
  useEffect(() => {
    const cv = loupe.current;
    const src = pixels.current?.canvas;
    if (!cv || !src || !pos) return;
    const ctx = cv.getContext('2d');
    if (!ctx) return;
    const { px, py } = at(pos.x, pos.y);
    const half = Math.floor(CELLS / 2);
    ctx.imageSmoothingEnabled = false;
    ctx.fillStyle = '#000';
    ctx.fillRect(0, 0, cv.width, cv.height);
    ctx.drawImage(src, px - half, py - half, CELLS, CELLS, 0, 0, CELLS * ZOOM, CELLS * ZOOM);
    ctx.strokeStyle = 'rgba(255,255,255,0.12)';
    for (let i = 1; i < CELLS; i++) {
      ctx.beginPath();
      ctx.moveTo(i * ZOOM + 0.5, 0);
      ctx.lineTo(i * ZOOM + 0.5, CELLS * ZOOM);
      ctx.moveTo(0, i * ZOOM + 0.5);
      ctx.lineTo(CELLS * ZOOM, i * ZOOM + 0.5);
      ctx.stroke();
    }
    ctx.strokeStyle = '#fff';
    ctx.lineWidth = 2;
    ctx.strokeRect(half * ZOOM, half * ZOOM, ZOOM, ZOOM);
    ctx.lineWidth = 1;
  });

  const pick = useCallback(
    async (x: number, y: number) => {
      const { px, py } = at(x, y);
      const v = inTauri ? await api.shots.regionPixel(px, py).catch(() => null) : null;
      const c = v ? { r: v[0], g: v[1], b: v[2] } : colour(x, y);
      if (!c) return;
      const h = hex(c.r, c.g, c.b);
      remember(h);
      setPicked((p) => [h, ...p.filter((v) => v !== h)].slice(0, 8));
      void navigator.clipboard?.writeText(h).catch(() => undefined);
      setFlash(h);
      setTimeout(() => setFlash((f) => (f === h ? null : f)), 1400);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        void close();
        return;
      }
      const { sx, sy } = scale();
      const step = {
        ArrowLeft: [-1, 0],
        ArrowRight: [1, 0],
        ArrowUp: [0, -1],
        ArrowDown: [0, 1],
      }[e.key];
      if (step && pos) {
        e.preventDefault();
        const n = e.shiftKey ? 10 : 1;
        setPos({
          x: Math.max(0, Math.min(window.innerWidth - 1, pos.x + (step[0] * n) / sx)),
          y: Math.max(0, Math.min(window.innerHeight - 1, pos.y + (step[1] * n) / sy)),
        });
      } else if ((e.key === 'Enter' || e.key === ' ' || e.key.toLowerCase() === 'c') && pos) {
        e.preventDefault();
        void pick(pos.x, pos.y);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  const onDown = (e: MouseEvent) => {
    if (e.button !== 0) return;
    drag.current = { x: e.clientX, y: e.clientY, moved: false };
  };
  const onMove = (e: MouseEvent) => {
    setPos({ x: e.clientX, y: e.clientY });
    const d = drag.current;
    if (!d) return;
    let x2 = e.clientX;
    let y2 = e.clientY;
    if (!d.moved && Math.hypot(x2 - d.x, y2 - d.y) < 4) return;
    d.moved = true;
    if (e.shiftKey) {
      if (Math.abs(x2 - d.x) > Math.abs(y2 - d.y)) y2 = d.y;
      else x2 = d.x;
    }
    setLine({ x1: d.x, y1: d.y, x2, y2 });
  };
  const onUp = (e: MouseEvent) => {
    const d = drag.current;
    drag.current = null;
    if (d && !d.moved) {
      setLine(null);
      void pick(e.clientX, e.clientY);
    }
  };

  if (error && !pending)
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 bg-black text-white">
        <div className="text-[15px]">Couldn’t freeze the screen</div>
        <div className="font-mono text-[12px] text-white/60">{error}</div>
        <button type="button" onClick={() => void close()} className="rounded-lg bg-white/10 px-4 py-2 text-[13px] hover:bg-white/20">
          Close
        </button>
      </div>
    );

  const c = pos ? colour(pos.x, pos.y) : null;
  const { sx, sy } = scale();
  const ruler = line
    ? {
        dx: Math.round(Math.abs(line.x2 - line.x1) * sx),
        dy: Math.round(Math.abs(line.y2 - line.y1) * sy),
      }
    : null;
  const loupeSize = CELLS * ZOOM;
  const lx = pos ? (pos.x + 24 + loupeSize > window.innerWidth ? pos.x - 24 - loupeSize : pos.x + 24) : 0;
  const ly = pos ? (pos.y + 24 + loupeSize + 52 > window.innerHeight ? pos.y - 24 - loupeSize - 52 : pos.y + 24) : 0;

  return (
    <div
      className="fixed inset-0 select-none overflow-hidden bg-black"
      style={{ cursor: 'crosshair' }}
      onMouseDown={onDown}
      onMouseMove={onMove}
      onMouseUp={onUp}
      onContextMenu={(e) => {
        e.preventDefault();
        void close();
      }}
      role="application"
      aria-label="Click to pick a colour, drag to measure. Escape closes."
    >
      {pending && <img ref={img} src={pending.image} onLoad={onLoad} alt="" draggable={false} className="pointer-events-none absolute inset-0 h-full w-full object-fill" />}
      {line && ruler && (
        <svg className="pointer-events-none absolute inset-0 h-full w-full">
          <line x1={line.x1} y1={line.y1} x2={line.x2} y2={line.y2} stroke="#000" strokeOpacity={0.6} strokeWidth={3} />
          <line x1={line.x1} y1={line.y1} x2={line.x2} y2={line.y2} stroke="#fff" strokeWidth={1.5} strokeDasharray="6 4" />
          {[
            [line.x1, line.y1],
            [line.x2, line.y2],
          ].map(([x, y], i) => (
            <circle key={i} cx={x} cy={y} r={4} fill="#fff" stroke="#000" strokeOpacity={0.5} />
          ))}
        </svg>
      )}
      {line && ruler && (
        <div
          className="pointer-events-none absolute rounded-md bg-black/80 px-2 py-1 font-mono text-[12px] text-white"
          style={{
            left: (line.x1 + line.x2) / 2 + 10,
            top: (line.y1 + line.y2) / 2 + 10,
          }}
        >
          {Math.round(Math.hypot(ruler.dx, ruler.dy))} px{' '}
          <span className="text-white/60">
            · {ruler.dx} × {ruler.dy}
          </span>
        </div>
      )}
      {pos && c && (
        <div className="pointer-events-none absolute overflow-hidden rounded-2xl border border-white/25 bg-black/85 shadow-2xl" style={{ left: lx, top: ly, width: loupeSize }}>
          <canvas ref={loupe} width={loupeSize} height={loupeSize} className="block" />
          <div className="flex items-center gap-2 px-2 py-1.5">
            <span className="h-5 w-5 shrink-0 rounded-md border border-white/30" style={{ background: hex(c.r, c.g, c.b) }} />
            <span className="min-w-0 font-mono text-[11.5px] leading-tight text-white">
              {hex(c.r, c.g, c.b)}
              <span className="block whitespace-nowrap text-white/55">
                rgb {c.r} {c.g} {c.b}
              </span>
              <span className="block whitespace-nowrap text-white/40">
                x {at(pos.x, pos.y).px} · y {at(pos.x, pos.y).py}
              </span>
            </span>
          </div>
        </div>
      )}
      <div className="absolute left-4 top-4 w-[230px] rounded-2xl border border-white/15 bg-black/80 p-3 text-white shadow-xl backdrop-blur" onMouseDown={(e) => e.stopPropagation()} onMouseUp={(e) => e.stopPropagation()}>
        <div className="flex items-center gap-2 text-[13px] font-semibold">
          <Pipette size={14} /> Colour picker &amp; ruler
          <button type="button" onClick={() => void close()} className="ml-auto rounded-md p-1 text-white/70 hover:bg-white/10 hover:text-white" aria-label="Close">
            <X size={14} />
          </button>
        </div>
        <div className="mt-1 text-[11.5px] leading-snug text-white/60">Click: pick (copied) · Drag: measure (Shift = straight) · Arrows: 1 px · Esc: close</div>
        {picked.length > 0 && (
          <div className="mt-2.5 space-y-1">
            {picked.map((h) => {
              const [r, g, b] = [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
              return (
                <button
                  key={h}
                  type="button"
                  onClick={() =>
                    void navigator.clipboard?.writeText(h).then(
                      () => setFlash(h),
                      () => undefined,
                    )
                  }
                  className="flex w-full items-center gap-2 rounded-lg px-1.5 py-1 text-left hover:bg-white/10"
                  title={`rgb(${r}, ${g}, ${b}) · ${hsl(r, g, b)}`}
                >
                  <span className="h-5 w-5 shrink-0 rounded-md border border-white/25" style={{ background: h }} />
                  <span className="font-mono text-[12px]">{h}</span>
                  <span className="ml-auto truncate font-mono text-[10.5px] text-white/45">{hsl(r, g, b).replace('hsl', '')}</span>
                </button>
              );
            })}
          </div>
        )}
      </div>
      {flash && (
        <div className="pointer-events-none absolute bottom-8 left-1/2 flex -translate-x-1/2 items-center gap-2 rounded-full bg-white px-4 py-2 text-[13px] font-semibold text-black shadow-xl">
          <Check size={14} /> Copied {flash}
        </div>
      )}
    </div>
  );
}
