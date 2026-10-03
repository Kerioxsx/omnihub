// Canvas treemap: devicePixelRatio-aware drawing, cushion shading, folder
// groups with labels, hover tooltip, hit-testing and an animated zoom when
// drilling in or out.

import { formatBytes, formatPercent, extColor } from '@shared/format';
import type { TreemapItem } from '@shared/types';
import { type MouseEvent as ReactMouseEvent, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Skeleton } from '../../components/ui/Card';
import { ErrorState } from '../../components/ui/States';
import { useSize } from '../../lib/hooks';
import { FOLDED_ID, type TmNode, type TmRect, chain, hitTest, layoutTreemap, lerpRect, mapInto, mapOut, topLevel } from './treemapLayout';

export interface TreemapTarget {
  id: number;
  name: string;
  isDir: boolean;
  size: number;
  path: string;
}

interface Props {
  scanId: string;
  nodeId: number;
  version: number;
  /** Full path of `nodeId` (for tooltips and actions). */
  basePath: string;
  highlightId: number | null;
  reducedMotion: boolean;
  onHover: (id: number | null) => void;
  onDrill: (id: number) => void;
  onSelect: (id: number) => void;
  onContextMenu: (e: ReactMouseEvent, target: TreemapTarget) => void;
}

// ---------- colours ----------

interface Palette {
  light: boolean;
  bg: string;
  dir: string;
  text: string;
  dim: string;
  accent: string;
  font: string;
  byVar: Map<string, string>;
}

function readPalette(): Palette {
  const root = document.documentElement;
  const cs = getComputedStyle(root);
  const get = (v: string) => cs.getPropertyValue(v).trim();
  const byVar = new Map<string, string>();
  for (const k of ['dir', 'video', 'image', 'audio', 'archive', 'code', 'doc', 'exe', 'data', 'other']) byVar.set(`var(--tm-${k})`, get(`--tm-${k}`));
  return { light: root.classList.contains('light'), bg: get('--bg-elev') || '#0d0d16', dir: get('--tm-dir'), text: get('--text'), dim: get('--text-dim'), accent: get('--accent'), font: getComputedStyle(document.body).fontFamily, byVar };
}

function parseHex(c: string): [number, number, number] {
  const m = /^#?([0-9a-f]{6}|[0-9a-f]{3})$/i.exec(c.trim());
  if (!m) return [120, 120, 140];
  const h = m[1].length === 3 ? m[1].replace(/./g, (x) => x + x) : m[1];
  return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
}

function mix(c: string, target: [number, number, number], t: number): string {
  const [r, g, b] = parseHex(c);
  return `rgb(${Math.round(r + (target[0] - r) * t)},${Math.round(g + (target[1] - g) * t)},${Math.round(b + (target[2] - b) * t)})`;
}

const WHITE: [number, number, number] = [255, 255, 255];
const BLACK: [number, number, number] = [0, 0, 0];

// ---------- drawing ----------

function ellipsize(ctx: CanvasRenderingContext2D, text: string, max: number): string {
  if (max <= 8) return '';
  if (ctx.measureText(text).width <= max) return text;
  let lo = 0;
  let hi = text.length;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (ctx.measureText(`${text.slice(0, mid)}…`).width <= max) lo = mid;
    else hi = mid - 1;
  }
  return lo > 0 ? `${text.slice(0, lo)}…` : '';
}

interface DrawOpts {
  alpha: number;
  labels: boolean;
  rect: (n: TmNode) => TmRect;
}

function drawNodes(ctx: CanvasRenderingContext2D, nodes: TmNode[], pal: Palette, cushions: Map<string, [string, string, string]>, o: DrawOpts) {
  ctx.globalAlpha = o.alpha;
  const ink = 'rgba(8,8,18,0.86)';
  const inkDim = 'rgba(8,8,18,0.6)';
  for (const n of nodes) {
    const r = o.rect(n);
    if (r.w < 0.6 || r.h < 0.6) continue;
    const it = n.item;
    if (it.id === FOLDED_ID) {
      ctx.fillStyle = pal.light ? 'rgba(15,15,40,0.06)' : 'rgba(255,255,255,0.045)';
      ctx.fillRect(r.x + 0.5, r.y + 0.5, r.w - 1, r.h - 1);
      ctx.save();
      ctx.beginPath();
      ctx.rect(r.x + 0.5, r.y + 0.5, r.w - 1, r.h - 1);
      ctx.clip();
      ctx.strokeStyle = pal.light ? 'rgba(15,15,40,0.09)' : 'rgba(255,255,255,0.07)';
      ctx.lineWidth = 1;
      ctx.beginPath();
      for (let d = -r.h; d < r.w; d += 7) {
        ctx.moveTo(r.x + d, r.y + r.h);
        ctx.lineTo(r.x + d + r.h, r.y);
      }
      ctx.stroke();
      ctx.restore();
      if (o.labels && r.w > 70 && r.h > 22) {
        ctx.font = `500 11px ${pal.font}`;
        ctx.fillStyle = pal.dim;
        ctx.fillText(ellipsize(ctx, it.name, r.w - 12), r.x + 6, r.y + 15);
      }
      continue;
    }
    if (it.isDir) {
      const shade = Math.min(0.5, n.depth * 0.12);
      ctx.fillStyle = pal.light ? mix(pal.dir, WHITE, shade) : mix(pal.dir, BLACK, 0.25 + shade * 0.5);
      ctx.fillRect(r.x + 0.5, r.y + 0.5, r.w - 1, r.h - 1);
      ctx.strokeStyle = pal.light ? 'rgba(30,30,80,0.18)' : 'rgba(255,255,255,0.09)';
      ctx.lineWidth = 1;
      ctx.strokeRect(r.x + 0.5, r.y + 0.5, r.w - 1, r.h - 1);
      if (o.labels && n.header > 0 && r.w > 40) {
        ctx.font = `600 11px ${pal.font}`;
        const size = formatBytes(it.size);
        const sizeW = ctx.measureText(size).width;
        const nameMax = r.w - 14 - (r.w > 120 ? sizeW + 10 : 0);
        ctx.fillStyle = pal.light ? '#1d1d33' : pal.text;
        ctx.fillText(ellipsize(ctx, it.name, nameMax), r.x + 6, r.y + 13.5);
        if (r.w > 120) {
          ctx.font = `500 10.5px ${pal.font}`;
          ctx.fillStyle = pal.light ? 'rgba(29,29,51,0.6)' : pal.dim;
          ctx.fillText(size, r.x + r.w - sizeW - 7, r.y + 13.5);
        }
      } else if (it.children.length === 0 && r.w > 6 && r.h > 6) {
        // A folder whose contents were not expanded (too small or too deep).
        const g = ctx.createLinearGradient(r.x, r.y, r.x + r.w, r.y + r.h);
        g.addColorStop(0, pal.light ? 'rgba(255,255,255,0.5)' : 'rgba(255,255,255,0.07)');
        g.addColorStop(1, 'rgba(0,0,0,0)');
        ctx.fillStyle = g;
        ctx.fillRect(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
        if (o.labels && r.w > 60 && r.h > 26) {
          ctx.font = `600 11px ${pal.font}`;
          ctx.fillStyle = pal.light ? '#1d1d33' : pal.text;
          ctx.fillText(ellipsize(ctx, it.name, r.w - 12), r.x + 6, r.y + 15);
          if (r.h > 40) {
            ctx.font = `500 10.5px ${pal.font}`;
            ctx.fillStyle = pal.light ? 'rgba(29,29,51,0.6)' : pal.dim;
            ctx.fillText(formatBytes(it.size), r.x + 6, r.y + 29);
          }
        }
      }
      continue;
    }
    const key = extColor(it.ext);
    let cushion = cushions.get(key);
    if (!cushion) {
      const base = pal.byVar.get(key) ?? '#64748b';
      cushion = [mix(base, WHITE, 0.22), base, mix(base, BLACK, 0.28)];
      cushions.set(key, cushion);
    }
    if (r.w * r.h > 300) {
      const g = ctx.createLinearGradient(r.x, r.y, r.x + r.w, r.y + r.h);
      g.addColorStop(0, cushion[0]);
      g.addColorStop(0.45, cushion[1]);
      g.addColorStop(1, cushion[2]);
      ctx.fillStyle = g;
    } else ctx.fillStyle = cushion[1];
    if (r.w > 2.5 && r.h > 2.5) ctx.fillRect(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
    else ctx.fillRect(r.x, r.y, r.w, r.h);
    if (o.labels && r.w > 58 && r.h > 24) {
      ctx.font = `600 11px ${pal.font}`;
      ctx.fillStyle = ink;
      ctx.fillText(ellipsize(ctx, it.name, r.w - 12), r.x + 6, r.y + 15);
      if (r.h > 40) {
        ctx.font = `500 10.5px ${pal.font}`;
        ctx.fillStyle = inkDim;
        ctx.fillText(formatBytes(it.size), r.x + 6, r.y + 29);
      }
    }
  }
  ctx.globalAlpha = 1;
}

const ease = (t: number) => 1 - Math.pow(1 - t, 3);

interface Anim {
  kind: 'in' | 'out' | 'fade';
  focus: TmRect | null;
  prev: TmNode[];
  start: number;
}

export function Treemap({ scanId, nodeId, version, basePath, highlightId, reducedMotion, onHover, onDrill, onSelect, onContextMenu }: Props) {
  const [boxRef, size] = useSize<HTMLDivElement>();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const tipRef = useRef<HTMLDivElement>(null);
  const [data, setData] = useState<{ nodeId: number; root: TreemapItem } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [hover, setHover] = useState(-1);
  const [palette, setPalette] = useState<Palette | null>(null);
  const cushions = useRef(new Map<string, [string, string, string]>());
  const anim = useRef<Anim | null>(null);
  const prevLayout = useRef<{ nodeId: number; nodes: TmNode[] } | null>(null);
  const raf = useRef(0);

  // Fetch the item tree for the current folder.
  useEffect(() => {
    let alive = true;
    setError(null);
    api.storage
      .treemap(scanId, nodeId, 8, 1500)
      .then((root) => alive && setData({ nodeId, root }))
      .catch((e: unknown) => alive && setError(errorText(e)));
    return () => {
      alive = false;
    };
  }, [scanId, nodeId, version]);

  // Re-read colours when the theme or accent changes.
  useEffect(() => {
    const update = () => {
      cushions.current.clear();
      setPalette(readPalette());
    };
    update();
    const mo = new MutationObserver(update);
    mo.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'data-accent'] });
    return () => mo.disconnect();
  }, []);

  const W = Math.floor(size.width);
  const H = Math.floor(size.height);
  const nodes = useMemo(() => (data && W > 0 && H > 0 ? layoutTreemap(data.root, W, H) : []), [data, W, H]);

  // Decide how to animate when the folder changes.
  useEffect(() => {
    if (!data || !nodes.length) return;
    const prev = prevLayout.current;
    if (prev && prev.nodeId !== data.nodeId && !reducedMotion) {
      const from = prev.nodes.find((n) => n.item.id === data.nodeId && n.parent === -1);
      const focus = from ? null : nodes.find((n) => n.item.id === prev.nodeId);
      anim.current = from ? { kind: 'in', focus: from, prev: prev.nodes, start: performance.now() } : focus ? { kind: 'out', focus, prev: prev.nodes, start: performance.now() } : { kind: 'fade', focus: null, prev: prev.nodes, start: performance.now() };
    } else if (prev?.nodeId !== data.nodeId) {
      anim.current = null;
    }
    prevLayout.current = { nodeId: data.nodeId, nodes };
    setHover(-1);
  }, [nodes, data, reducedMotion]);

  const highlightIndex = useMemo(() => (highlightId == null ? -1 : nodes.findIndex((n) => n.item.id === highlightId && n.parent === -1)), [nodes, highlightId]);

  const draw = useCallback(
    (now: number) => {
      const canvas = canvasRef.current;
      if (!canvas || !palette || W <= 0 || H <= 0) return false;
      const dpr = window.devicePixelRatio || 1;
      if (canvas.width !== Math.round(W * dpr) || canvas.height !== Math.round(H * dpr)) {
        canvas.width = Math.round(W * dpr);
        canvas.height = Math.round(H * dpr);
        canvas.style.width = `${W}px`;
        canvas.style.height = `${H}px`;
      }
      const ctx = canvas.getContext('2d');
      if (!ctx) return false;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, W, H);
      ctx.textBaseline = 'alphabetic';
      const a = anim.current;
      let running = false;
      if (a) {
        const t = Math.min(1, (now - a.start) / 280);
        const e = ease(t);
        running = t < 1;
        if (a.kind === 'in' && a.focus) {
          const f = a.focus;
          drawNodes(ctx, a.prev, palette, cushions.current, { alpha: 1 - e, labels: false, rect: (n) => lerpRect(n, mapOut(n, f, W, H), e) });
          drawNodes(ctx, nodes, palette, cushions.current, { alpha: Math.min(1, e * 1.5), labels: t > 0.7, rect: (n) => lerpRect(mapInto(n, f, W, H), n, e) });
        } else if (a.kind === 'out' && a.focus) {
          const f = a.focus;
          drawNodes(ctx, nodes, palette, cushions.current, { alpha: Math.min(1, 0.3 + e), labels: t > 0.7, rect: (n) => lerpRect(mapOut(n, f, W, H), n, e) });
          drawNodes(ctx, a.prev, palette, cushions.current, { alpha: 1 - e, labels: false, rect: (n) => lerpRect(n, mapInto(n, f, W, H), e) });
        } else {
          drawNodes(ctx, a.prev, palette, cushions.current, { alpha: 1 - e, labels: false, rect: (n) => n });
          drawNodes(ctx, nodes, palette, cushions.current, { alpha: e, labels: t > 0.6, rect: (n) => n });
        }
        if (!running) anim.current = null;
      } else {
        drawNodes(ctx, nodes, palette, cushions.current, { alpha: 1, labels: true, rect: (n) => n });
      }
      // Hover / highlight outlines.
      if (!running) {
        const outline = (i: number, color: string, width: number) => {
          const n = nodes[i];
          if (!n) return;
          ctx.fillStyle = palette.light ? 'rgba(255,255,255,0.18)' : 'rgba(255,255,255,0.09)';
          ctx.fillRect(n.x + 1, n.y + 1, n.w - 2, n.h - 2);
          ctx.strokeStyle = color;
          ctx.lineWidth = width;
          ctx.strokeRect(n.x + width / 2, n.y + width / 2, n.w - width, n.h - width);
        };
        if (highlightIndex >= 0) outline(highlightIndex, palette.accent, 2);
        if (hover >= 0 && hover !== highlightIndex) outline(hover, palette.light ? '#15151f' : '#ffffff', 1.5);
      }
      return running;
    },
    [nodes, palette, W, H, hover, highlightIndex],
  );

  useEffect(() => {
    cancelAnimationFrame(raf.current);
    const loop = (now: number) => {
      if (draw(now)) raf.current = requestAnimationFrame(loop);
    };
    raf.current = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf.current);
  }, [draw]);

  const pathOf = (i: number) => {
    const names = chain(nodes, i);
    const base = basePath.replace(/\\$/, '');
    return `${base}\\${names.join('\\')}`;
  };

  const onMove = (e: ReactMouseEvent<HTMLCanvasElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const i = anim.current ? -1 : hitTest(nodes, x, y);
    if (i !== hover) {
      setHover(i);
      const top = i >= 0 ? topLevel(nodes, i) : -1;
      onHover(top >= 0 && nodes[top].item.id !== FOLDED_ID ? nodes[top].item.id : null);
    }
    const tip = tipRef.current;
    if (tip) {
      const tw = tip.offsetWidth;
      const th = tip.offsetHeight;
      const left = x + 16 + tw > W ? x - tw - 12 : x + 16;
      const topPx = y + 18 + th > H ? y - th - 10 : y + 18;
      tip.style.transform = `translate(${Math.max(4, left)}px, ${Math.max(4, topPx)}px)`;
    }
  };

  const onLeave = () => {
    setHover(-1);
    onHover(null);
  };

  const onClick = (e: ReactMouseEvent<HTMLCanvasElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const i = hitTest(nodes, e.clientX - rect.left, e.clientY - rect.top);
    if (i < 0) return;
    const top = topLevel(nodes, i);
    const t = nodes[top];
    if (t.item.id === FOLDED_ID) return;
    if (t.item.isDir) onDrill(t.item.id);
    else onSelect(t.item.id);
  };

  const onContext = (e: ReactMouseEvent<HTMLCanvasElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    let i = hitTest(nodes, e.clientX - rect.left, e.clientY - rect.top);
    while (i >= 0 && nodes[i].item.id === FOLDED_ID) i = nodes[i].parent;
    if (i < 0) return;
    const it = nodes[i].item;
    onContextMenu(e, { id: it.id, name: it.name, isDir: it.isDir, size: it.size, path: pathOf(i) });
  };

  const hovered = hover >= 0 ? nodes[hover] : null;
  const total = data?.root.size ?? 0;

  return (
    <div ref={boxRef} className="relative h-full w-full overflow-hidden rounded-xl">
      {error ? (
        <ErrorState title="Could not draw the treemap" error={error} />
      ) : !data ? (
        <Skeleton className="absolute inset-0 rounded-xl" />
      ) : nodes.length === 0 && W > 0 ? (
        <div className="absolute inset-0 flex items-center justify-center text-[13px] text-faint">This folder is empty.</div>
      ) : null}
      <canvas
        ref={canvasRef}
        className="absolute inset-0 cursor-pointer"
        role="img"
        aria-label={`Treemap of ${basePath}: ${nodes.filter((n) => n.parent === -1).length} items, largest ${nodes[0]?.item.name ?? 'none'}`}
        onMouseMove={onMove}
        onMouseLeave={onLeave}
        onClick={onClick}
        onContextMenu={onContext}
      />
      <div ref={tipRef} className="pointer-events-none absolute left-0 top-0 z-10 max-w-[340px] rounded-xl border border-line-strong bg-elev/95 px-3 py-2 shadow-[0_12px_32px_-10px_rgba(0,0,0,0.6)] backdrop-blur-md transition-opacity duration-100" style={{ opacity: hovered ? 1 : 0 }}>
        {hovered && (
          <>
            <div className="flex items-center gap-2">
              <span className="h-2.5 w-2.5 shrink-0 rounded-[3px]" style={{ background: hovered.item.id === FOLDED_ID ? 'var(--surface-3)' : extColor(hovered.item.ext, hovered.item.isDir) }} />
              <span className="truncate text-[13px] font-semibold text-fg">{hovered.item.name}</span>
            </div>
            <div className="mt-1 flex items-baseline gap-2 text-[12px]">
              <span className="font-semibold tabular text-fg">{formatBytes(hovered.item.size)}</span>
              <span className="text-dim">{total ? formatPercent(hovered.item.size / total) : ''} of this folder</span>
            </div>
            {hovered.item.id !== FOLDED_ID ? <div className="mt-1 break-all font-mono text-[10.5px] leading-snug text-faint">{pathOf(hover)}</div> : <div className="mt-1 text-[11px] text-faint">Items under 0.1% are grouped. Use the list to see them.</div>}
            {hovered.item.id !== FOLDED_ID && <div className="mt-1.5 text-[10.5px] text-faint">{topLevel(nodes, hover) >= 0 && nodes[topLevel(nodes, hover)].item.isDir ? 'Click to open · right-click for actions' : 'Right-click for actions'}</div>}
          </>
        )}
      </div>
    </div>
  );
}
