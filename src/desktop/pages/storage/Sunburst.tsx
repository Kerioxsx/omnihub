// Sunburst: the current folder in the middle, its contents as rings around
// it. Each ring is one level deeper; a segment's angle is its share of the
// parent. Click a folder to zoom in, the middle to go up.

import { extColor, formatBytes, formatPercent } from '@shared/format';
import type { TreemapItem } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { type MouseEvent as ReactMouseEvent, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Skeleton } from '../../components/ui/Card';
import { ErrorState } from '../../components/ui/States';
import { useSize } from '../../lib/hooks';
import type { TreemapTarget } from './Treemap';

const DEPTH = 4;
const MIN_ANGLE = 0.004; // radians; thinner slices are not drawn
const FOLDED = 4294967295;
// Distinct hues for the top-level folders; files use their type colour.
const HUES = [262, 199, 152, 32, 340, 48, 222, 12, 172, 290];

interface Seg {
  item: TreemapItem;
  depth: number;
  a0: number;
  a1: number;
  hue: number;
  path: string;
}

function layout(root: TreemapItem, basePath: string): Seg[] {
  const out: Seg[] = [];
  const walk = (node: TreemapItem, depth: number, a0: number, a1: number, hue: number, path: string) => {
    const total = node.children.reduce((s, c) => s + c.size, 0) || 1;
    let a = a0;
    node.children.forEach((c, i) => {
      const span = ((a1 - a0) * c.size) / total;
      const h = depth === 1 ? HUES[i % HUES.length] : hue;
      const p = c.id === FOLDED ? path : `${path.replace(/\\$/, '')}\\${c.name}`;
      if (span >= MIN_ANGLE) {
        out.push({ item: c, depth, a0: a, a1: a + span, hue: h, path: p });
        if (c.isDir && depth < DEPTH && c.children.length) walk(c, depth + 1, a, a + span, h, p);
      }
      a += span;
    });
  };
  walk(root, 1, 0, Math.PI * 2, 0, basePath);
  return out;
}

function arc(cx: number, cy: number, r0: number, r1: number, a0: number, a1: number): string {
  // A full circle needs two arcs.
  if (a1 - a0 >= Math.PI * 2 - 1e-6) {
    return `M ${cx + r1} ${cy} A ${r1} ${r1} 0 1 1 ${cx - r1} ${cy} A ${r1} ${r1} 0 1 1 ${cx + r1} ${cy} M ${cx + r0} ${cy} A ${r0} ${r0} 0 1 0 ${cx - r0} ${cy} A ${r0} ${r0} 0 1 0 ${cx + r0} ${cy} Z`;
  }
  const p = (r: number, a: number) => `${cx + r * Math.sin(a)} ${cy - r * Math.cos(a)}`;
  const large = a1 - a0 > Math.PI ? 1 : 0;
  return `M ${p(r1, a0)} A ${r1} ${r1} 0 ${large} 1 ${p(r1, a1)} L ${p(r0, a1)} A ${r0} ${r0} 0 ${large} 0 ${p(r0, a0)} Z`;
}

function fill(s: Seg, light: boolean): string {
  if (s.item.id === FOLDED) return light ? 'hsl(230 8% 80%)' : 'hsl(230 8% 28%)';
  if (!s.item.isDir) return extColor(s.item.ext);
  const l = light ? 58 + s.depth * 7 : 52 - s.depth * 6;
  return `hsl(${s.hue} ${light ? 62 : 58}% ${l}%)`;
}

export function Sunburst({
  scanId,
  nodeId,
  version,
  basePath,
  highlightId,
  onHover,
  onDrill,
  onUp,
  onSelect,
  onContextMenu,
}: {
  scanId: string;
  nodeId: number;
  version: number;
  basePath: string;
  highlightId: number | null;
  onHover: (id: number | null) => void;
  onDrill: (id: number) => void;
  onUp: (() => void) | null;
  onSelect: (id: number) => void;
  onContextMenu: (e: ReactMouseEvent, t: TreemapTarget) => void;
}) {
  const [ref, size] = useSize<HTMLDivElement>();
  const [data, setData] = useState<TreemapItem | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [hover, setHover] = useState<Seg | null>(null);
  const light = typeof document !== 'undefined' && document.documentElement.classList.contains('light');

  useEffect(() => {
    let alive = true;
    setError(null);
    api.storage
      .treemap(scanId, nodeId, DEPTH, 1200)
      .then((d) => alive && setData(d))
      .catch((e: unknown) => alive && setError(errorText(e)));
    return () => {
      alive = false;
    };
  }, [scanId, nodeId, version]);

  const segs = useMemo(() => (data ? layout(data, basePath) : []), [data, basePath]);
  const d = Math.max(0, Math.min(size.width, size.height));
  const cx = size.width / 2;
  const cy = size.height / 2;
  const R = d / 2 - 6;
  const r0 = R * 0.24;
  const ring = (R - r0) / DEPTH;
  const shown = hover?.item ?? null;

  if (error) return <ErrorState error={error} />;
  return (
    <div ref={ref} className="relative h-full min-h-[340px] w-full">
      {!data || d === 0 ? (
        <Skeleton className="absolute inset-6 rounded-full" />
      ) : (
        <AnimatePresence mode="wait">
          <motion.svg key={`${nodeId}-${version}`} width={size.width} height={size.height} initial={{ opacity: 0, scale: 0.92 }} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.25 }} role="img" aria-label={`Sunburst of ${data.name}`} onMouseLeave={() => (setHover(null), onHover(null))}>
            {segs.map((s) => {
              const active = hover === s || highlightId === s.item.id;
              const target: TreemapTarget = { id: s.item.id, name: s.item.name, isDir: s.item.isDir, size: s.item.size, path: s.path };
              return (
                <path
                  key={`${s.depth}-${s.item.id}-${s.a0}`}
                  d={arc(cx, cy, r0 + (s.depth - 1) * ring + 1, r0 + s.depth * ring - 1, s.a0 + 0.002, s.a1 - 0.002)}
                  fill={fill(s, light)}
                  opacity={hover && !active ? 0.55 : 1}
                  stroke={active ? 'var(--text)' : 'none'}
                  strokeWidth={active ? 1.5 : 0}
                  className="cursor-pointer transition-opacity duration-150"
                  onMouseEnter={() => {
                    setHover(s);
                    if (s.depth === 1 && s.item.id !== FOLDED) onHover(s.item.id);
                  }}
                  onClick={() => (s.item.isDir && s.item.id !== FOLDED ? onDrill(s.item.id) : s.depth === 1 && onSelect(s.item.id))}
                  onContextMenu={(e) => {
                    if (s.item.id === FOLDED) return;
                    e.preventDefault();
                    onContextMenu(e, target);
                  }}
                />
              );
            })}
            <circle cx={cx} cy={cy} r={r0 - 3} fill="var(--surface-2)" className={onUp ? 'cursor-pointer' : undefined} onClick={() => onUp?.()} onMouseEnter={() => setHover(null)}>
              <title>{onUp ? 'Up one level' : data.name}</title>
            </circle>
          </motion.svg>
        </AnimatePresence>
      )}
      {data && d > 0 && (
        <div className="pointer-events-none absolute flex flex-col items-center justify-center text-center" style={{ left: cx - r0 * 0.8, top: cy - r0 * 0.8, width: r0 * 1.6, height: r0 * 1.6 }}>
          <div className="line-clamp-2 w-full break-all px-1 text-[12px] font-semibold leading-tight text-fg">{shown ? (shown.id === FOLDED ? `${shown.folded} smaller items` : shown.name) : data.name.replace(/\\$/, '') || data.name}</div>
          <div className="mt-0.5 text-[15px] font-bold tabular text-fg">{formatBytes(shown ? shown.size : data.size)}</div>
          {shown && data.size > 0 && <div className="text-[11px] tabular text-dim">{formatPercent(shown.size / data.size, 1)} of this folder</div>}
          {!shown && onUp && <div className="text-[10.5px] text-faint">click to go up</div>}
        </div>
      )}
    </div>
  );
}
