// Region capture overlay, shown in its own fullscreen borderless window.
// The frozen screen is the background; the user drags a rectangle and
// commits with Enter / double-click (rect is sent in IMAGE pixels).

import type { PendingRegion, Rect } from '@shared/types';
import { Check, X } from 'lucide-react';
import { type MouseEvent, useCallback, useEffect, useRef, useState } from 'react';
import { api, errorText, inTauri } from '../../api';
import { useEvent } from '../../lib/hooks';
import { closeCurrentWindow } from '../../lib/util';

interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

function isPending(v: unknown): v is PendingRegion {
  return typeof v === 'object' && v !== null && 'id' in v && 'image' in v;
}

export function RegionOverlay() {
  const [pending, setPending] = useState<PendingRegion | null>(null);
  const [missing, setMissing] = useState(false);
  const [box, setBox] = useState<Box | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [dragging, setDragging] = useState(false);
  const img = useRef<HTMLImageElement>(null);
  const drag = useRef<{ mode: 'draw' | 'move'; sx: number; sy: number; box: Box | null } | null>(null);

  const load = useCallback(async () => {
    try {
      let p = await api.shots.regionPending();
      // Browser preview: start a capture so the overlay has something to show.
      if (!p && !inTauri) {
        await api.shots.regionBegin();
        p = await api.shots.regionPending();
      }
      setPending(p);
      setMissing(!p);
    } catch (e) {
      setError(errorText(e));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  useEvent<unknown>('region:pending', (p) => {
    if (isPending(p)) {
      setPending(p);
      setMissing(false);
      setBox(null);
    } else void load();
  });

  const close = useCallback(() => void closeCurrentWindow('#/screenshots'), []);

  const cancel = useCallback(async () => {
    await api.shots.regionCancel().catch(() => undefined);
    close();
  }, [close]);

  const commit = useCallback(async () => {
    if (!pending || !box || box.w < 4 || box.h < 4 || busy) return;
    const el = img.current;
    const scale = el && el.naturalWidth ? el.naturalWidth / window.innerWidth : pending.width / window.innerWidth;
    const scaleY = el && el.naturalHeight ? el.naturalHeight / window.innerHeight : scale;
    const rect: Rect = { x: Math.round(box.x * scale), y: Math.round(box.y * scaleY), width: Math.round(box.w * scale), height: Math.round(box.h * scaleY) };
    setBusy(true);
    try {
      await api.shots.regionCommit(pending.id, rect);
      close();
    } catch (e) {
      setError(errorText(e));
      setBusy(false);
    }
  }, [pending, box, busy, close]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        void cancel();
      } else if (e.key === 'Enter') {
        e.preventDefault();
        void commit();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [cancel, commit]);

  const inside = (x: number, y: number) => !!box && x >= box.x && x <= box.x + box.w && y >= box.y && y <= box.y + box.h;

  const onDown = (e: MouseEvent) => {
    if (e.button !== 0) return;
    const x = e.clientX;
    const y = e.clientY;
    drag.current = { mode: inside(x, y) ? 'move' : 'draw', sx: x, sy: y, box };
    setDragging(true);
    if (!inside(x, y)) setBox({ x, y, w: 0, h: 0 });
  };
  const onMove = (e: MouseEvent) => {
    const d = drag.current;
    if (!d) return;
    if (d.mode === 'draw') {
      const x = Math.min(d.sx, e.clientX);
      const y = Math.min(d.sy, e.clientY);
      setBox({ x, y, w: Math.abs(e.clientX - d.sx), h: Math.abs(e.clientY - d.sy) });
    } else if (d.box) {
      const nx = Math.max(0, Math.min(window.innerWidth - d.box.w, d.box.x + e.clientX - d.sx));
      const ny = Math.max(0, Math.min(window.innerHeight - d.box.h, d.box.y + e.clientY - d.sy));
      setBox({ ...d.box, x: nx, y: ny });
    }
  };
  const onUp = () => {
    drag.current = null;
    setDragging(false);
    setBox((b) => (b && (b.w < 3 || b.h < 3) ? null : b));
  };

  const scale = img.current?.naturalWidth ? img.current.naturalWidth / window.innerWidth : (pending?.width ?? window.innerWidth) / window.innerWidth;
  const label = box ? `${Math.round(box.w * scale)} × ${Math.round(box.h * scale)}` : '';
  const toolbarBelow = box ? box.y + box.h + 52 < window.innerHeight : true;

  if (error && !pending)
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 bg-black text-white">
        <div className="text-[15px]">Could not start the capture</div>
        <div className="font-mono text-[12px] text-white/60">{error}</div>
        <button type="button" onClick={close} className="rounded-lg bg-white/10 px-4 py-2 text-[13px] hover:bg-white/20">
          Close
        </button>
      </div>
    );
  if (missing)
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 bg-black text-white">
        <div className="text-[15px]">No capture in progress</div>
        <button type="button" onClick={close} className="rounded-lg bg-white/10 px-4 py-2 text-[13px] hover:bg-white/20">
          Close
        </button>
      </div>
    );

  return (
    <div
      className="fixed inset-0 select-none overflow-hidden bg-black"
      style={{ cursor: dragging && drag.current?.mode === 'move' ? 'grabbing' : 'crosshair' }}
      onMouseDown={onDown}
      onMouseMove={onMove}
      onMouseUp={onUp}
      onDoubleClick={() => void commit()}
      onContextMenu={(e) => {
        e.preventDefault();
        void cancel();
      }}
      role="application"
      aria-label="Drag to select a region. Enter or double-click to capture, Escape to cancel."
    >
      {pending && <img ref={img} src={pending.image} alt="" draggable={false} className="pointer-events-none absolute inset-0 h-full w-full object-fill" />}
      {!box && <div className="pointer-events-none absolute inset-0 bg-black/45" />}
      {box && (
        <div className="pointer-events-none absolute border border-white/90" style={{ left: box.x, top: box.y, width: box.w, height: box.h, boxShadow: '0 0 0 100vmax rgba(0,0,0,0.5)', cursor: 'move' }}>
          {(['nw', 'ne', 'sw', 'se'] as const).map((c) => (
            <span key={c} className="absolute h-2.5 w-2.5 rounded-full border border-black/40 bg-white" style={{ [c.includes('n') ? 'top' : 'bottom']: -5, [c.includes('w') ? 'left' : 'right']: -5 }} />
          ))}
          <span className="absolute -top-7 left-0 whitespace-nowrap rounded-md bg-black/75 px-2 py-0.5 font-mono text-[12px] text-white" style={box.y < 32 ? { top: 6, left: 6 } : undefined}>
            {label}
          </span>
        </div>
      )}
      {box && box.w > 3 && !dragging && (
        <div className="absolute flex items-center gap-1 rounded-xl border border-white/15 bg-black/80 p-1 shadow-xl backdrop-blur" style={{ left: Math.min(box.x + box.w - 196, window.innerWidth - 204), top: toolbarBelow ? box.y + box.h + 10 : box.y - 46 }} onMouseDown={(e) => e.stopPropagation()}>
          <button type="button" onClick={() => void cancel()} className="flex h-8 items-center gap-1.5 rounded-lg px-3 text-[12.5px] text-white/80 hover:bg-white/10 hover:text-white">
            <X size={14} /> Cancel
          </button>
          <button type="button" onClick={() => void commit()} disabled={busy} className="flex h-8 items-center gap-1.5 rounded-lg bg-white px-3 text-[12.5px] font-semibold text-black hover:bg-white/90">
            <Check size={14} /> Capture <span className="font-mono text-[11px] text-black/50">Enter</span>
          </button>
        </div>
      )}
      {!box && (
        <div className="pointer-events-none absolute left-1/2 top-8 -translate-x-1/2 rounded-full border border-white/15 bg-black/70 px-4 py-2 text-[13px] text-white/90 shadow-xl backdrop-blur">
          Drag to select · <b>Enter</b> or double-click to capture · <b>Esc</b> or right-click to cancel
        </div>
      )}
      {error && <div className="absolute bottom-6 left-1/2 -translate-x-1/2 rounded-lg bg-red-600/90 px-4 py-2 text-[13px] text-white">{error}</div>}
    </div>
  );
}
