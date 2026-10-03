// Squarified treemap layout (Bruls, Huizing & van Wijk, 2000) for the nested
// items returned by `storage_treemap`.

import type { TreemapItem } from '@shared/types';

export interface TmRect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface TmNode extends TmRect {
  item: TreemapItem;
  depth: number;
  /** Index of the parent node in the layout array, -1 for top level. */
  parent: number;
  /** Height of the label strip for folders that show their children. */
  header: number;
}

export const FOLDED_ID = 4294967295;

/**
 * Lay `values` (sorted descending, all > 0) into `rect`, keeping rectangles
 * as close to squares as possible. Rows are added while the worst aspect
 * ratio in the row improves.
 */
export function squarify(values: number[], rect: TmRect): TmRect[] {
  const n = values.length;
  const out: TmRect[] = new Array(n);
  const total = values.reduce((a, b) => a + b, 0);
  if (total <= 0 || rect.w <= 0 || rect.h <= 0) {
    for (let k = 0; k < n; k++) out[k] = { x: rect.x, y: rect.y, w: 0, h: 0 };
    return out;
  }
  const scale = (rect.w * rect.h) / total;
  let { x, y, w, h } = rect;
  let i = 0;
  while (i < n) {
    const side = Math.min(w, h);
    const side2 = side * side;
    let rowSum = 0;
    let rowMin = Infinity;
    let rowMax = 0;
    let best = Infinity;
    let j = i;
    while (j < n) {
      const a = values[j] * scale;
      const s = rowSum + a;
      const mn = Math.min(rowMin, a);
      const mx = Math.max(rowMax, a);
      const worst = Math.max((side2 * mx) / (s * s), (s * s) / (side2 * mn));
      if (j > i && worst > best) break;
      rowSum = s;
      rowMin = mn;
      rowMax = mx;
      best = worst;
      j++;
    }
    if (w >= h) {
      // Column along the left edge.
      const cw = h > 0 ? rowSum / h : 0;
      let cy = y;
      for (let k = i; k < j; k++) {
        const hh = cw > 0 ? (values[k] * scale) / cw : 0;
        out[k] = { x, y: cy, w: cw, h: hh };
        cy += hh;
      }
      x += cw;
      w = Math.max(0, w - cw);
    } else {
      // Row along the top edge.
      const rh = w > 0 ? rowSum / w : 0;
      let cx = x;
      for (let k = i; k < j; k++) {
        const ww = rh > 0 ? (values[k] * scale) / rh : 0;
        out[k] = { x: cx, y, w: ww, h: rh };
        cx += ww;
      }
      y += rh;
      h = Math.max(0, h - rh);
    }
    i = j;
  }
  return out;
}

const PAD = 3;
const HEADER = 19;

/** Lay out the whole nested item tree; parents come before their children. */
export function layoutTreemap(root: TreemapItem, width: number, height: number): TmNode[] {
  const out: TmNode[] = [];
  const place = (item: TreemapItem, rect: TmRect, depth: number, parent: number) => {
    const kids = item.children.filter((c) => c.size > 0).sort((a, b) => b.size - a.size);
    if (!kids.length || rect.w < 2 || rect.h < 2) return;
    const rects = squarify(
      kids.map((k) => k.size),
      rect,
    );
    kids.forEach((k, i) => {
      const r = rects[i];
      const idx = out.length;
      const nest = k.isDir && k.children.length > 0 && r.w > 16 && r.h > 16;
      const header = nest && r.w > 54 && r.h > 42 ? HEADER : 0;
      out.push({ x: r.x, y: r.y, w: r.w, h: r.h, item: k, depth, parent, header });
      if (nest) {
        const top = header || PAD;
        const inner = { x: r.x + PAD, y: r.y + top, w: r.w - PAD * 2, h: r.h - top - PAD };
        if (inner.w > 4 && inner.h > 4) place(k, inner, depth + 1, idx);
      }
    });
  };
  place(root, { x: 0, y: 0, w: width, h: height }, 0, -1);
  return out;
}

export function lerpRect(a: TmRect, b: TmRect, t: number): TmRect {
  return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t, w: a.w + (b.w - a.w) * t, h: a.h + (b.h - a.h) * t };
}

/** Map a rect laid out in the full canvas (W×H) into `into`. */
export function mapInto(r: TmRect, into: TmRect, W: number, H: number): TmRect {
  return { x: into.x + (r.x * into.w) / W, y: into.y + (r.y * into.h) / H, w: (r.w * into.w) / W, h: (r.h * into.h) / H };
}

/** Inverse of mapInto: blow `from` up to the full canvas. */
export function mapOut(r: TmRect, from: TmRect, W: number, H: number): TmRect {
  const sx = W / Math.max(from.w, 1e-6);
  const sy = H / Math.max(from.h, 1e-6);
  return { x: (r.x - from.x) * sx, y: (r.y - from.y) * sy, w: r.w * sx, h: r.h * sy };
}

/** Deepest node under the point (children come after parents). */
export function hitTest(nodes: TmNode[], x: number, y: number): number {
  for (let i = nodes.length - 1; i >= 0; i--) {
    const n = nodes[i];
    if (x >= n.x && x < n.x + n.w && y >= n.y && y < n.y + n.h) return i;
  }
  return -1;
}

/** Index of the top-level ancestor of node i. */
export function topLevel(nodes: TmNode[], i: number): number {
  let cur = i;
  while (cur >= 0 && nodes[cur].parent >= 0) cur = nodes[cur].parent;
  return cur;
}

/** Names from the top level down to node i. */
export function chain(nodes: TmNode[], i: number): string[] {
  const names: string[] = [];
  let cur = i;
  while (cur >= 0) {
    names.push(nodes[cur].item.name);
    cur = nodes[cur].parent;
  }
  return names.reverse();
}
