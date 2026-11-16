// The phone as a whiteboard: draw with a finger or stylus and each stroke
// shows up on the PC (Tools → Whiteboard), where it can be saved.

import type { BoardStroke } from '@shared/types';
import { Eraser, Trash2, Undo2 } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { client } from '../client';
import { useEvent } from '../lib/events';
import { cx, errorMessage } from '../lib/util';
import { toast } from '../state';
import { IconButton } from '../ui/common';
import { SubHeader } from './More';

const ASPECT = 4 / 3;
const COLORS = ['#111827', '#ef4444', '#3b82f6', '#22c55e', '#f59e0b', '#a855f7'];
const WIDTHS = [0.004, 0.008, 0.016];

function paint(c: HTMLCanvasElement, strokes: BoardStroke[], current: BoardStroke | null) {
  const ctx = c.getContext('2d');
  if (!ctx) return;
  const w = c.width, h = c.height;
  ctx.fillStyle = '#ffffff';
  ctx.fillRect(0, 0, w, h);
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  for (const s of current ? [...strokes, current] : strokes) {
    ctx.strokeStyle = s.eraser ? '#ffffff' : s.color;
    ctx.lineWidth = Math.max(1, s.width * w);
    ctx.beginPath();
    s.points.forEach(([x, y], i) => (i ? ctx.lineTo(x * w, y * h) : ctx.moveTo(x * w, y * h)));
    if (s.points.length === 1) ctx.lineTo(s.points[0][0] * w + 0.1, s.points[0][1] * h);
    ctx.stroke();
  }
}

export function BoardPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [strokes, setStrokes] = useState<BoardStroke[]>([]);
  const [color, setColor] = useState(COLORS[0]);
  const [width, setWidth] = useState(WIDTHS[1]);
  const [eraser, setEraser] = useState(false);
  const current = useRef<BoardStroke | null>(null);

  useEffect(() => {
    if (active) void client.board().then(setStrokes, () => undefined);
  }, [active]);
  useEvent<{ strokes: BoardStroke[] }>('board:strokes', (p) => {
    // Our own strokes come back too; only add what we haven't drawn here.
    const mine = new Set(strokes.map((s) => JSON.stringify(s.points[0]) + s.color + s.points.length));
    setStrokes((s) => [...s, ...(p?.strokes ?? []).filter((x) => !mine.has(JSON.stringify(x.points[0]) + x.color + x.points.length))]);
  });
  useEvent('board:undo', () => void client.board().then(setStrokes, () => undefined));
  useEvent('board:clear', () => setStrokes([]));

  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const fit = () => {
      const r = c.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      c.width = Math.round(r.width * dpr);
      c.height = Math.round(r.height * dpr);
      paint(c, strokes, current.current);
    };
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(c);
    return () => ro.disconnect();
  }, [strokes]);

  const at = (e: React.PointerEvent): [number, number] => {
    const r = canvas.current!.getBoundingClientRect();
    return [Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)), Math.min(1, Math.max(0, (e.clientY - r.top) / r.height))];
  };
  const down = (e: React.PointerEvent) => {
    (e.target as HTMLElement).setPointerCapture?.(e.pointerId);
    current.current = { points: [at(e)], color, width: eraser ? width * 3 : width, eraser };
    paint(canvas.current!, strokes, current.current);
  };
  const move = (e: React.PointerEvent) => {
    const s = current.current;
    if (!s) return;
    const p = at(e);
    const [lx, ly] = s.points[s.points.length - 1];
    if (Math.hypot(p[0] - lx, p[1] - ly) < 0.002) return;
    s.points.push([Math.round(p[0] * 10000) / 10000, Math.round(p[1] * 10000) / 10000]);
    paint(canvas.current!, strokes, s);
  };
  const up = () => {
    const s = current.current;
    current.current = null;
    if (!s) return;
    setStrokes((x) => [...x, s]);
    client.boardAdd([s]).catch((err: unknown) => toast.error('Not sent to the PC', errorMessage(err)));
  };

  return (
    <div className="flex h-full flex-col">
      <SubHeader
        title="Whiteboard"
        subtitle="Shows up on the PC in Tools → Whiteboard"
        onBack={onBack}
        right={
          <div className="flex gap-1">
            <IconButton label="Undo" onClick={() => void client.boardUndo().then(() => setStrokes((s) => s.slice(0, -1)))}>
              <Undo2 size={20} />
            </IconButton>
            <IconButton label="Clear" onClick={() => void client.boardClear().then(() => setStrokes([]))}>
              <Trash2 size={20} />
            </IconButton>
          </div>
        }
      />
      <div className="px-safe flex min-h-0 flex-1 flex-col items-center justify-center gap-3 pb-tabbar">
        <div className="w-full overflow-hidden rounded-2xl border border-line shadow-lg" style={{ aspectRatio: String(ASPECT), maxHeight: '100%' }}>
          <canvas ref={canvas} className="h-full w-full touch-none" onPointerDown={down} onPointerMove={move} onPointerUp={up} onPointerCancel={up} aria-label="Whiteboard" />
        </div>
        <div className="flex w-full items-center justify-between gap-2">
          <div className="flex gap-1.5">
            {COLORS.map((c) => (
              <button key={c} type="button" aria-label={`Colour ${c}`} onClick={() => (setColor(c), setEraser(false))} className={cx('h-8 w-8 rounded-full border-2', !eraser && c === color ? 'border-accent' : 'border-transparent')} style={{ background: c }} />
            ))}
          </div>
          <div className="flex items-center gap-1">
            {WIDTHS.map((w) => (
              <button key={w} type="button" aria-label={`Width ${w}`} onClick={() => setWidth(w)} className={cx('grid h-8 w-8 place-items-center rounded-lg', w === width ? 'bg-surface-3' : '')}>
                <span className="rounded-full bg-fg" style={{ width: 4 + w * 500, height: 4 + w * 500 }} />
              </button>
            ))}
            <button type="button" aria-label="Eraser" onClick={() => setEraser(!eraser)} className={cx('grid h-8 w-8 place-items-center rounded-lg', eraser ? 'bg-accent-soft text-accent' : 'text-dim')}>
              <Eraser size={18} />
            </button>
          </div>
        </div>
        <p className="text-center text-[12px] text-faint">Turn the phone sideways for a bigger board.</p>
      </div>
    </div>
  );
}
