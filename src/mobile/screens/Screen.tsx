// Screen: watch (and, when the PC allows it, control) the PC's display.
//
// Frames are JPEGs with a 20-byte header (capture/stream.rs). Each one is
// decoded with createImageBitmap, drawn on a canvas sized to the frame,
// and acknowledged *after* drawing — the PC keeps at most two frames in
// flight, so slow links drop frames instead of piling up delay.

import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { AnimatePresence, motion } from 'motion/react';
import {
  MonitorPlay,
  Monitor,
  X,
  Maximize,
  Minimize,
  Keyboard,
  Hand,
  MousePointer2,
  Eye,
  Gauge,
  RefreshCw,
  LoaderCircle,
  Info,
  Check,
  ZoomOut,
  EyeOff,
  Smartphone,
  ShieldAlert,
  Zap,
  CornerDownLeft,
  Delete,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  ArrowDown,
  ChevronRight,
} from 'lucide-react';
import type { MonitorInfo, Preset } from '@shared/types';
import { client, openScreen, parseFrame, type FrameHeader, type ScreenStats, type ViewerMessage } from '../client';
import { useApp } from '../state';
import { Button, Empty, ErrorState, PageHeader, Skeleton } from '../ui/common';
import { Sheet } from '../ui/Sheet';
import { useBackHandler } from '../lib/back';
import { keepAwake } from '../lib/wakelock';
import { cx, errorMessage, keyboardInset, vibrate, usePageVisible } from '../lib/util';

type ScreenInfo = Awaited<ReturnType<typeof client.screenInfo>>;
type Mode = 'view' | 'trackpad' | 'touch';
type Conn = 'connecting' | 'live' | 'paused' | 'closed' | 'error';

const PRESET_KEY = 'omnihub.screenPreset';
const MODE_KEY = 'omnihub.screenMode';

function savedPreset(): string | null {
  try {
    return localStorage.getItem(PRESET_KEY);
  } catch {
    return null;
  }
}

export function ScreenScreen({ active }: { active: boolean }) {
  const info = useApp((s) => s.info);
  const [si, setSi] = useState<ScreenInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [monitor, setMonitor] = useState(0);
  const [preset, setPreset] = useState<string>('balanced');
  const [viewing, setViewing] = useState(false);

  const load = useCallback(async () => {
    try {
      const r = await client.screenInfo();
      setSi(r);
      setError(null);
      const saved = savedPreset();
      setPreset(saved && r.presets.some((p) => p.id === saved) ? saved : r.defaultPreset || 'balanced');
      const primary = r.monitors.find((m) => m.primary) ?? r.monitors[0];
      if (primary) setMonitor((cur) => (r.monitors.some((m) => m.index === cur) ? cur : primary.index));
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);

  useEffect(() => {
    if (active) load();
  }, [active, load]);
  // Leaving the tab ends the stream.
  useEffect(() => {
    if (!active) setViewing(false);
  }, [active]);

  const choosePreset = (id: string) => {
    setPreset(id);
    try {
      localStorage.setItem(PRESET_KEY, id);
    } catch {
      /* ignore */
    }
  };

  if (info && !info.features.screen) {
    return (
      <div className="scroller h-full">
        <PageHeader title="Screen" subtitle={info.name} />
        <Empty icon={<MonitorPlay size={30} />} title="Screen sharing is off" body="Turn it on in OmniHub → Settings → Phone on the PC." />
      </div>
    );
  }

  const control = si?.allowControl ?? info?.features.control ?? false;

  return (
    <div className="scroller h-full">
      <PageHeader title="Screen" subtitle={info?.name} />
      <div className="px-safe pb-tabbar">
        {error && !si ? (
          <ErrorState message={error} onRetry={load} />
        ) : !si ? (
          <div className="space-y-3">
            <Skeleton className="h-[180px] rounded-[22px]" />
            <Skeleton className="h-16 rounded-2xl" />
            <Skeleton className="h-40 rounded-2xl" />
          </div>
        ) : (
          <>
            <section className="relative overflow-hidden rounded-[24px] border border-line bg-[linear-gradient(140deg,rgba(124,58,237,.28),rgba(6,182,212,.14))] p-5">
              <div className="pointer-events-none absolute -right-8 -top-12 h-44 w-44 rounded-full bg-[radial-gradient(closest-side,rgba(34,211,238,.25),transparent)]" />
              <div className="relative flex items-center gap-4">
                <div className="grid h-14 w-14 place-items-center rounded-2xl bg-white/15 text-white backdrop-blur">
                  <MonitorPlay size={28} />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="font-display text-xl font-bold leading-tight text-fg">Watch your PC live</div>
                  <div className="mt-0.5 text-sm text-dim">{control ? 'View, trackpad or touch control' : 'View only'}</div>
                </div>
              </div>
              <Button size="lg" className="relative mt-5 w-full" icon={<Zap size={19} />} onClick={() => setViewing(true)} disabled={!si.monitors.length}>
                Start viewing
              </Button>
            </section>

            <div className="mb-2.5 mt-6 px-1 text-[13px] font-semibold uppercase tracking-[0.08em] text-faint">Display</div>
            {si.monitors.length === 0 ? (
              <div className="card p-4 text-sm text-dim">No display found on the PC.</div>
            ) : (
              <div className="space-y-2">
                {si.monitors.map((m) => (
                  <Choice key={m.index} active={m.index === monitor} onClick={() => setMonitor(m.index)} icon={<Monitor size={19} />} title={m.name || `Display ${m.index + 1}`} sub={`${m.width} × ${m.height}${m.primary ? ' · primary' : ''}`} />
                ))}
              </div>
            )}

            <div className="mb-2.5 mt-6 px-1 text-[13px] font-semibold uppercase tracking-[0.08em] text-faint">Quality</div>
            <div className="space-y-2">
              {si.presets.map((p) => (
                <Choice key={p.id} active={p.id === preset} onClick={() => choosePreset(p.id)} icon={<Gauge size={19} />} title={p.label} sub={`up to ${p.fps} fps · JPEG quality ${p.quality}`} />
              ))}
            </div>

            {!control && (
              <div className="mt-4 flex items-start gap-3 rounded-2xl border border-line bg-surface p-4 text-sm">
                <ShieldAlert size={18} className="mt-0.5 shrink-0 text-dim" />
                <div className="text-dim">
                  Remote control is off. To use the phone as a trackpad or touch screen, turn on <b className="text-fg">Allow control</b> in OmniHub → Settings → Phone.
                </div>
              </div>
            )}
            <HonestNote className="mt-4" />
          </>
        )}
      </div>
      <AnimatePresence>
        {viewing && si && (
          <Viewer
            key="viewer"
            monitor={si.monitors.find((m) => m.index === monitor) ?? si.monitors[0]}
            presets={si.presets}
            preset={preset}
            onPreset={choosePreset}
            allowControl={control}
            onExit={() => setViewing(false)}
          />
        )}
      </AnimatePresence>
    </div>
  );
}

function HonestNote({ className }: { className?: string }) {
  return (
    <div className={cx('flex items-start gap-3 rounded-2xl border border-line bg-surface p-4 text-sm', className)}>
      <Info size={18} className="mt-0.5 shrink-0 text-accent" />
      <div className="text-dim">
        Expect ~40–90 ms on good Wi-Fi. For 4K60 gaming-grade streaming use <b className="text-fg">Moonlight</b> with <b className="text-fg">Sunshine</b> on the PC.
      </div>
    </div>
  );
}

function Choice({ active, onClick, icon, title, sub }: { active: boolean; onClick: () => void; icon: ReactNode; title: string; sub: string }) {
  return (
    <button onClick={onClick} className={cx('press flex w-full items-center gap-3 rounded-2xl border px-3.5 py-3 text-left transition-colors', active ? 'border-accent bg-accent-soft' : 'border-line bg-card')}>
      <div className={cx('grid h-10 w-10 place-items-center rounded-xl', active ? 'bg-accent text-white' : 'bg-surface-2 text-dim')}>{icon}</div>
      <div className="min-w-0 flex-1">
        <div className="truncate text-[15px] font-semibold">{title}</div>
        <div className="truncate text-xs text-dim">{sub}</div>
      </div>
      {active && (
        <div className="grid h-6 w-6 place-items-center rounded-full bg-accent text-white">
          <Check size={14} strokeWidth={3} />
        </div>
      )}
    </button>
  );
}

// ---------- the viewer ----------

interface Pt {
  x: number;
  y: number;
  sx: number;
  sy: number;
}

const clamp01 = (v: number) => Math.max(0, Math.min(1, v));

/** Draw the pointer; `s` = frame pixels per screen pixel, so it stays ~20 px tall on the phone. */
function drawCursor(ctx: CanvasRenderingContext2D, x: number, y: number, s: number) {
  ctx.save();
  ctx.translate(x, y);
  ctx.scale(s, s);
  ctx.beginPath();
  ctx.moveTo(0, 0);
  ctx.lineTo(0, 17);
  ctx.lineTo(4.2, 13);
  ctx.lineTo(7.2, 19.6);
  ctx.lineTo(10, 18.4);
  ctx.lineTo(7.1, 11.9);
  ctx.lineTo(12.6, 11.9);
  ctx.closePath();
  ctx.fillStyle = '#ffffff';
  ctx.strokeStyle = '#000000';
  ctx.lineWidth = 1.3;
  ctx.lineJoin = 'round';
  ctx.fill();
  ctx.stroke();
  ctx.restore();
}

async function decodeJpeg(jpeg: Uint8Array): Promise<CanvasImageSource & { close?: () => void }> {
  const blob = new Blob([jpeg as Uint8Array<ArrayBuffer>], { type: 'image/jpeg' });
  if (typeof createImageBitmap === 'function') return createImageBitmap(blob);
  // Older Safari: decode through an <img>.
  const url = URL.createObjectURL(blob);
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    return img;
  } finally {
    URL.revokeObjectURL(url);
  }
}

const fsEnabled = () => !!(document.fullscreenEnabled || (document as unknown as { webkitFullscreenEnabled?: boolean }).webkitFullscreenEnabled);
const fsElement = () => document.fullscreenElement ?? (document as unknown as { webkitFullscreenElement?: Element }).webkitFullscreenElement ?? null;

function Viewer({
  monitor,
  presets,
  preset,
  onPreset,
  allowControl,
  onExit,
}: {
  monitor: MonitorInfo;
  presets: Preset[];
  preset: string;
  onPreset: (id: string) => void;
  allowControl: boolean;
  onExit: () => void;
}) {
  const rootRef = useRef<HTMLDivElement>(null);
  const boxRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const ctxRef = useRef<CanvasRenderingContext2D | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const presetRef = useRef(preset);
  presetRef.current = preset;
  const pipe = useRef<{ decoding: boolean; queued: ReturnType<typeof parseFrame> }>({ decoding: false, queued: null });
  const decodeMs = useRef(0);
  const lastCursor = useRef<{ x: number; y: number } | null>(null);
  const cursorScale = useRef(1);
  const visible = usePageVisible();

  const [conn, setConn] = useState<Conn>('connecting');
  const [error, setError] = useState<string | null>(null);
  const [stats, setStats] = useState<ScreenStats | null>(null);
  const [frame, setFrame] = useState<{ w: number; h: number } | null>(null);
  const [box, setBox] = useState({ w: 0, h: 0 });
  const [hud, setHud] = useState(true);
  const [mode, setModeState] = useState<Mode>(() => {
    try {
      const m = localStorage.getItem(MODE_KEY) as Mode | null;
      return m === 'trackpad' || m === 'touch' ? m : 'view';
    } catch {
      return 'view';
    }
  });
  const [kbOpen, setKbOpen] = useState(false);
  const [presetOpen, setPresetOpen] = useState(false);
  const [zoom, setZoom] = useState({ z: 1, x: 0, y: 0 });
  const [vp, setVp] = useState({ x: 0.5, y: 0.5 });
  const [isFs, setIsFs] = useState(false);
  const [rotateHint, setRotateHint] = useState(true);
  const [ripple, setRipple] = useState<{ x: number; y: number; id: number; right?: boolean } | null>(null);

  const control = allowControl && (stats ? stats.control : true);
  const effMode: Mode = control ? mode : 'view';
  const setMode = (m: Mode) => {
    setModeState(m);
    try {
      localStorage.setItem(MODE_KEY, m);
    } catch {
      /* ignore */
    }
    if (m === 'trackpad' && lastCursor.current && frame) setVp({ x: clamp01(lastCursor.current.x / frame.w), y: clamp01(lastCursor.current.y / frame.h) });
  };

  useBackHandler(true, onExit);

  // ----- socket -----
  const send = useCallback((m: ViewerMessage) => {
    const ws = wsRef.current;
    if (ws && ws.readyState === WebSocket.OPEN) ws.send(JSON.stringify(m));
  }, []);

  const ack = (ws: WebSocket, h: FrameHeader) => {
    if (ws.readyState === WebSocket.OPEN) ws.send(JSON.stringify({ t: 'ack', seq: h.seq, ts: h.ts } satisfies ViewerMessage));
  };

  const draw = (img: CanvasImageSource, h: FrameHeader) => {
    const c = canvasRef.current;
    if (!c) return;
    if (c.width !== h.width || c.height !== h.height || !ctxRef.current) {
      c.width = h.width;
      c.height = h.height;
      ctxRef.current = c.getContext('2d', { alpha: false });
      setFrame({ w: h.width, h: h.height });
    }
    const ctx = ctxRef.current;
    if (!ctx) return;
    ctx.drawImage(img, 0, 0, h.width, h.height);
    if (h.cursor) {
      lastCursor.current = h.cursor;
      drawCursor(ctx, h.cursor.x, h.cursor.y, cursorScale.current);
    }
  };

  const decode = async (f: NonNullable<ReturnType<typeof parseFrame>>, ws: WebSocket) => {
    pipe.current.decoding = true;
    const t0 = performance.now();
    try {
      const img = await decodeJpeg(f.jpeg);
      if (wsRef.current === ws) draw(img, f.header);
      img.close?.();
      const dt = performance.now() - t0;
      decodeMs.current = decodeMs.current ? decodeMs.current * 0.85 + dt * 0.15 : dt;
      if (wsRef.current === ws) setConn((c) => (c === 'connecting' ? 'live' : c));
    } catch {
      /* a corrupt frame: skip it */
    }
    ack(ws, f.header);
    pipe.current.decoding = false;
    const q = pipe.current.queued;
    pipe.current.queued = null;
    if (q && wsRef.current === ws) decode(q, ws);
  };

  const closeSocket = () => {
    const ws = wsRef.current;
    wsRef.current = null;
    pipe.current = { decoding: false, queued: null };
    if (ws) {
      ws.onclose = null;
      ws.onmessage = null;
      try {
        ws.close();
      } catch {
        /* ignore */
      }
    }
  };

  const connect = useCallback(async () => {
    closeSocket();
    setConn('connecting');
    setError(null);
    try {
      const ws = await openScreen(monitor.index, presetRef.current);
      wsRef.current = ws;
      ws.onmessage = (m) => {
        if (typeof m.data === 'string') {
          try {
            const msg = JSON.parse(m.data);
            if (msg.t === 'stats') setStats(msg as ScreenStats);
          } catch {
            /* ignore */
          }
          return;
        }
        const f = parseFrame(m.data as ArrayBuffer);
        if (!f) return;
        if (pipe.current.decoding) {
          // Keep only the newest waiting frame; acknowledge the one it replaces.
          if (pipe.current.queued) ack(ws, pipe.current.queued.header);
          pipe.current.queued = f;
          return;
        }
        decode(f, ws);
      };
      ws.onclose = (ev) => {
        if (wsRef.current !== ws) return;
        wsRef.current = null;
        setConn('closed');
        setError(ev.reason || (ev.code === 1006 ? 'The connection dropped.' : null));
      };
    } catch (e) {
      setConn('error');
      setError(errorMessage(e));
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [monitor.index]);

  useEffect(() => {
    connect();
    return closeSocket;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [connect]);

  // Pause while the page is hidden (saves battery and the PC's CPU), resume after.
  const wasLive = useRef(false);
  useEffect(() => {
    if (!visible) {
      if (wsRef.current) {
        wasLive.current = true;
        closeSocket();
        setConn('paused');
      }
    } else if (wasLive.current) {
      wasLive.current = false;
      connect();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible]);

  useEffect(() => {
    keepAwake('screen', conn === 'live');
    return () => keepAwake('screen', false);
  }, [conn]);

  // ----- layout -----
  useEffect(() => {
    const el = boxRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setBox({ w: el.clientWidth, h: el.clientHeight }));
    ro.observe(el);
    setBox({ w: el.clientWidth, h: el.clientHeight });
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    const on = () => setIsFs(!!fsElement());
    document.addEventListener('fullscreenchange', on);
    document.addEventListener('webkitfullscreenchange', on);
    return () => {
      document.removeEventListener('fullscreenchange', on);
      document.removeEventListener('webkitfullscreenchange', on);
    };
  }, []);
  useEffect(
    () => () => {
      if (fsElement()) document.exitFullscreen?.().catch(() => undefined);
    },
    [],
  );

  const fit = frame && box.w ? Math.min(box.w / frame.w, box.h / frame.h) : 1;
  const dispW = frame ? frame.w * fit : 0;
  const dispH = frame ? frame.h * fit : 0;
  if (frame && dispW) cursorScale.current = Math.max(1, frame.w / (dispW * zoom.z));
  const ox = (box.w - dispW) / 2;
  const oy = (box.h - dispH) / 2;

  const clampZoom = useCallback(
    (z: number, x: number, y: number) => {
      z = Math.max(1, Math.min(8, z));
      const W = dispW * z;
      const H = dispH * z;
      const cx = W >= box.w ? Math.max(box.w - W - ox, Math.min(-ox, x)) : (box.w - W) / 2 - ox;
      const cy = H >= box.h ? Math.max(box.h - H - oy, Math.min(-oy, y)) : (box.h - H) / 2 - oy;
      return { z, x: z === 1 ? 0 : cx, y: z === 1 ? 0 : cy };
    },
    [dispW, dispH, box.w, box.h, ox, oy],
  );
  const zoomRef = useRef(zoom);
  zoomRef.current = zoom;

  // ----- input -----
  const sender = useRef<{ move: { x: number; y: number } | null; mt: ReturnType<typeof setTimeout> | null; wdx: number; wdy: number; wt: ReturnType<typeof setTimeout> | null }>({ move: null, mt: null, wdx: 0, wdy: 0, wt: null });
  const flushMove = () => {
    const s = sender.current;
    if (s.mt) clearTimeout(s.mt);
    s.mt = null;
    if (s.move) {
      send({ t: 'pointer', kind: 'move', x: s.move.x, y: s.move.y });
      s.move = null;
    }
  };
  const queueMove = (x: number, y: number) => {
    const s = sender.current;
    s.move = { x, y };
    if (!s.mt) s.mt = setTimeout(flushMove, 16);
  };
  const flushWheel = () => {
    const s = sender.current;
    if (s.wt) clearTimeout(s.wt);
    s.wt = null;
    if (Math.abs(s.wdy) >= 1 || Math.abs(s.wdx) >= 1) {
      send({ t: 'wheel', dy: Math.round(s.wdy), dx: Math.round(s.wdx) });
      s.wdx = 0;
      s.wdy = 0;
    }
  };
  const queueWheel = (dx: number, dy: number) => {
    const s = sender.current;
    s.wdx += dx;
    s.wdy += dy;
    if (!s.wt) s.wt = setTimeout(flushWheel, 16);
  };
  const pointerAt = (clientX: number, clientY: number) => {
    const r = canvasRef.current?.getBoundingClientRect();
    if (!r || !r.width) return null;
    return { x: clamp01((clientX - r.left) / r.width), y: clamp01((clientY - r.top) / r.height) };
  };
  const click = (p: { x: number; y: number }, button = 0, screen?: { x: number; y: number }) => {
    flushMove();
    send({ t: 'pointer', kind: 'click', x: p.x, y: p.y, button });
    vibrate(button === 2 ? [15, 30, 15] : 8);
    if (screen) setRipple({ x: screen.x, y: screen.y, id: Date.now(), right: button === 2 });
  };

  const g = useRef({
    pts: new Map<number, Pt>(),
    max: 0,
    t0: 0,
    moved: false,
    mode: '' as '' | 'drag' | 'done' | 'two' | 'pinch' | 'scroll' | 'after',
    long: null as ReturnType<typeof setTimeout> | null,
    two: { cx: 0, cy: 0, d: 1, z: 1, x: 0, y: 0, lcx: 0, lcy: 0 },
    lastT: 0,
    lastTap: { t: 0, x: 0, y: 0 },
    downAt: null as { x: number; y: number } | null,
    dragAt: null as { x: number; y: number } | null,
  });

  const local = (clientX: number, clientY: number) => {
    const r = boxRef.current!.getBoundingClientRect();
    return { x: clientX - r.left, y: clientY - r.top };
  };
  const centroid = () => {
    const a = [...g.current.pts.values()];
    const cx = (a[0].x + a[1].x) / 2;
    const cy = (a[0].y + a[1].y) / 2;
    const d = Math.hypot(a[0].x - a[1].x, a[0].y - a[1].y) || 1;
    return { cx, cy, d };
  };

  const onDown = (e: React.PointerEvent) => {
    if (!frame) return;
    (e.currentTarget as Element).setPointerCapture?.(e.pointerId);
    const s = g.current;
    const p = local(e.clientX, e.clientY);
    s.pts.set(e.pointerId, { x: p.x, y: p.y, sx: p.x, sy: p.y });
    if (s.pts.size === 1) {
      s.max = 1;
      s.t0 = performance.now();
      s.lastT = s.t0;
      s.moved = false;
      s.mode = '';
      if (effMode === 'touch') {
        const at = pointerAt(e.clientX, e.clientY);
        s.downAt = at;
        s.long = setTimeout(() => {
          if (s.pts.size === 1 && !s.moved && at) {
            click(at, 2, p);
            s.mode = 'done';
          }
        }, 550);
      }
    } else if (s.pts.size === 2) {
      if (s.long) clearTimeout(s.long);
      if (s.mode === 'drag' && s.dragAt) {
        flushMove();
        send({ t: 'pointer', kind: 'up', x: s.dragAt.x, y: s.dragAt.y, button: 0 });
      }
      s.max = 2;
      s.t0 = s.mode === '' ? s.t0 : performance.now();
      s.mode = 'two';
      const c = centroid();
      const z = zoomRef.current;
      s.two = { cx: c.cx, cy: c.cy, d: c.d, z: z.z, x: z.x, y: z.y, lcx: c.cx, lcy: c.cy };
    } else s.max = Math.max(s.max, s.pts.size);
  };

  const onMove = (e: React.PointerEvent) => {
    const s = g.current;
    const pt = s.pts.get(e.pointerId);
    if (!pt) return;
    const p = local(e.clientX, e.clientY);
    const dx = p.x - pt.x;
    const dy = p.y - pt.y;
    pt.x = p.x;
    pt.y = p.y;
    const now = performance.now();
    const dt = Math.max(1, now - s.lastT);
    s.lastT = now;

    if (s.pts.size === 1) {
      if (!s.moved && Math.hypot(p.x - pt.sx, p.y - pt.sy) > 9) {
        s.moved = true;
        if (s.long) clearTimeout(s.long);
      }
      if (!s.moved) return;
      if (s.mode === 'after' || s.mode === 'done') {
        if (effMode === 'view' && zoomRef.current.z > 1) setZoom((z) => clampZoom(z.z, z.x + dx, z.y + dy));
        return;
      }
      if (effMode === 'view') {
        if (zoomRef.current.z > 1) setZoom((z) => clampZoom(z.z, z.x + dx, z.y + dy));
      } else if (effMode === 'touch') {
        const at = pointerAt(e.clientX, e.clientY);
        if (!at) return;
        if (s.mode !== 'drag') {
          s.mode = 'drag';
          const start = s.downAt ?? at;
          send({ t: 'pointer', kind: 'move', x: start.x, y: start.y });
          send({ t: 'pointer', kind: 'down', x: start.x, y: start.y, button: 0 });
        }
        s.dragAt = at;
        queueMove(at.x, at.y);
      } else {
        // Trackpad: relative motion with acceleration.
        const r = canvasRef.current?.getBoundingClientRect();
        if (!r?.width) return;
        const speed = Math.hypot(dx, dy) / dt;
        const gain = 1.25 * (1 + Math.min(2.6, speed * 1.4));
        setVp((v) => {
          const n = { x: clamp01(v.x + (dx * gain) / r.width), y: clamp01(v.y + (dy * gain) / r.height) };
          queueMove(n.x, n.y);
          return n;
        });
      }
    } else if (s.pts.size === 2) {
      const c = centroid();
      if (s.mode === 'two') {
        const dd = Math.abs(c.d - s.two.d);
        const dc = Math.hypot(c.cx - s.two.cx, c.cy - s.two.cy);
        if (effMode === 'view') {
          if (dd > 6 || dc > 6) s.mode = 'pinch';
        } else if (dd > 28 && dd > dc) s.mode = 'pinch';
        else if (dc > 14) s.mode = 'scroll';
      }
      if (s.mode === 'pinch') {
        const t = s.two;
        const z = Math.max(1, Math.min(8, (t.z * c.d) / t.d));
        const ux = (t.cx - ox - t.x) / t.z;
        const uy = (t.cy - oy - t.y) / t.z;
        setZoom(clampZoom(z, c.cx - ox - ux * z, c.cy - oy - uy * z));
      } else if (s.mode === 'scroll') {
        const sdx = c.cx - s.two.lcx;
        const sdy = c.cy - s.two.lcy;
        // Natural scrolling: content follows the fingers.
        queueWheel(-sdx * 3, -sdy * 3);
      }
      s.two.lcx = c.cx;
      s.two.lcy = c.cy;
    }
  };

  const onUp = (e: React.PointerEvent) => {
    const s = g.current;
    const pt = s.pts.get(e.pointerId);
    if (!pt) return;
    s.pts.delete(e.pointerId);
    if (s.pts.size === 1) {
      // One finger of two lifted: the rest of this gesture is ignored (or pans).
      if (s.mode === 'two' && s.max === 2 && performance.now() - s.t0 < 280) {
        // wait for the second finger: handled when it lifts
        s.mode = 'two';
      } else s.mode = 'after';
      return;
    }
    if (s.pts.size > 0) return;
    if (s.long) clearTimeout(s.long);
    const dur = performance.now() - s.t0;
    const screen = { x: pt.x, y: pt.y };
    if (s.max === 1 && !s.moved && s.mode === '' && dur < 450) {
      if (effMode === 'view') {
        const now = performance.now();
        const lt = s.lastTap;
        if (now - lt.t < 320 && Math.hypot(lt.x - pt.x, lt.y - pt.y) < 40) {
          s.lastTap = { t: 0, x: 0, y: 0 };
          const z = zoomRef.current;
          if (z.z > 1) setZoom({ z: 1, x: 0, y: 0 });
          else {
            const nz = 2.5;
            const ux = (pt.x - ox - z.x) / z.z;
            const uy = (pt.y - oy - z.y) / z.z;
            setZoom(clampZoom(nz, pt.x - ox - ux * nz, pt.y - oy - uy * nz));
          }
          setHud((h) => !h); // undo the first tap's toggle
        } else {
          s.lastTap = { t: now, x: pt.x, y: pt.y };
          setHud((h) => !h);
        }
      } else if (effMode === 'touch') {
        const at = pointerAt(e.clientX, e.clientY);
        if (at) click(at, 0, screen);
      } else {
        const v = vpRef.current;
        click(v, 0, vpScreen(v));
      }
    } else if (s.max === 2 && s.mode === 'two' && dur < 320) {
      // Two-finger tap.
      if (effMode === 'trackpad') click(vpRef.current, 2, vpScreen(vpRef.current));
      else if (effMode === 'touch') {
        const at = pointerAt(e.clientX, e.clientY);
        if (at) click(at, 2, screen);
      } else setZoom({ z: 1, x: 0, y: 0 });
    } else if (s.mode === 'drag') {
      flushMove();
      const at = pointerAt(e.clientX, e.clientY);
      if (at) send({ t: 'pointer', kind: 'up', x: at.x, y: at.y, button: 0 });
    }
    flushWheel();
    s.mode = '';
    s.max = 0;
  };

  const vpRef = useRef(vp);
  vpRef.current = vp;
  const vpScreen = (v: { x: number; y: number }) => ({ x: ox + zoom.x + v.x * dispW * zoom.z, y: oy + zoom.y + v.y * dispH * zoom.z });

  // ----- HUD values -----
  const latency = stats ? Math.round(stats.rtt / 2 + stats.encodeMs + decodeMs.current) : null;
  const toggleFs = async () => {
    try {
      if (fsElement()) await document.exitFullscreen?.();
      else {
        const el = rootRef.current as (HTMLElement & { webkitRequestFullscreen?: () => void }) | null;
        if (el?.requestFullscreen) await el.requestFullscreen({ navigationUI: 'hide' });
        else el?.webkitRequestFullscreen?.();
        if (frame && frame.w > frame.h) {
          const o = screen.orientation as ScreenOrientation & { lock?: (o: string) => Promise<void> };
          await o?.lock?.('landscape').catch(() => undefined);
        }
      }
    } catch {
      /* not allowed */
    }
  };
  const portraitForLandscape = !!frame && frame.w > frame.h && box.h > box.w;
  useEffect(() => {
    if (!rotateHint) return;
    const t = setTimeout(() => setRotateHint(false), 7000);
    return () => clearTimeout(t);
  }, [rotateHint]);

  const presetLabel = presets.find((p) => p.id === preset)?.label ?? preset;

  return (
    <motion.div
      ref={rootRef}
      className="fixed inset-0 z-50 bg-black text-white"
      initial={{ opacity: 0, scale: 1.02 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 1.02 }}
      transition={{ duration: 0.22 }}
    >
      <div
        ref={boxRef}
        className="viewer-surface absolute inset-0 overflow-hidden"
        onPointerDown={onDown}
        onPointerMove={onMove}
        onPointerUp={onUp}
        onPointerCancel={onUp}
        onContextMenu={(e) => e.preventDefault()}
        onWheel={(e) => {
          if (effMode !== 'view') queueWheel(e.deltaX, e.deltaY);
          else {
            const p = local(e.clientX, e.clientY);
            const z = zoomRef.current;
            const nz = Math.max(1, Math.min(8, z.z * (e.deltaY < 0 ? 1.15 : 1 / 1.15)));
            const ux = (p.x - ox - z.x) / z.z;
            const uy = (p.y - oy - z.y) / z.z;
            setZoom(clampZoom(nz, p.x - ox - ux * nz, p.y - oy - uy * nz));
          }
        }}
      >
        <canvas
          ref={canvasRef}
          aria-label={`Live view of ${monitor.name}`}
          className="absolute block bg-black"
          style={{ left: ox, top: oy, width: dispW || '100%', height: dispH || '100%', transform: `translate(${zoom.x}px, ${zoom.y}px) scale(${zoom.z})`, transformOrigin: '0 0', imageRendering: zoom.z > 2.5 ? 'pixelated' : 'auto' }}
        />
        {effMode === 'trackpad' && frame && (
          <div className="pointer-events-none absolute z-10" style={{ left: vpScreen(vp).x, top: vpScreen(vp).y }}>
            <div className="-ml-[14px] -mt-[14px] h-7 w-7 rounded-full border-2 border-white/90 bg-violet-500/35 shadow-[0_0_0_4px_rgba(139,92,246,.25)]" />
          </div>
        )}
        <AnimatePresence>
          {ripple && (
            <motion.div
              key={ripple.id}
              className={cx('pointer-events-none absolute z-10 h-12 w-12 rounded-full border-2', ripple.right ? 'border-cyan-300' : 'border-white')}
              style={{ left: ripple.x - 24, top: ripple.y - 24 }}
              initial={{ scale: 0.3, opacity: 0.9 }}
              animate={{ scale: 1.3, opacity: 0 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.45 }}
              onAnimationComplete={() => setRipple(null)}
            />
          )}
        </AnimatePresence>
      </div>

      {/* Connection overlays */}
      {conn !== 'live' && (
        <div className="pointer-events-none absolute inset-0 z-20 grid place-items-center p-6">
          {conn === 'connecting' || conn === 'paused' ? (
            <div className="hud-glass flex items-center gap-3 rounded-2xl px-5 py-4 text-[15px] font-semibold">
              <LoaderCircle size={20} className="spin text-cyan-300" /> {conn === 'paused' ? 'Paused while in background…' : `Connecting to ${monitor.name}…`}
            </div>
          ) : (
            <div className="hud-glass pointer-events-auto w-full max-w-[320px] rounded-3xl p-5 text-center">
              <div className="mx-auto mb-3 grid h-12 w-12 place-items-center rounded-2xl bg-white/10">
                <MonitorPlay size={24} />
              </div>
              <div className="font-display text-lg font-bold">Disconnected</div>
              <div className="mt-1 text-sm text-white/65">{error || 'The stream stopped. The PC may have ended it or the network dropped.'}</div>
              <div className="mt-4 flex gap-2">
                <button onClick={onExit} className="press h-12 flex-1 rounded-2xl bg-white/10 font-semibold">
                  Exit
                </button>
                <button onClick={connect} className="press grad-bg flex h-12 flex-1 items-center justify-center gap-2 rounded-2xl font-semibold">
                  <RefreshCw size={17} /> Reconnect
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {/* HUD */}
      <AnimatePresence>
        {hud ? (
          <motion.div key="hud" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.18 }} className="pointer-events-none absolute inset-0 z-30">
            <div className="pointer-events-auto absolute left-0 right-0 top-0 flex items-center gap-2 px-[max(12px,var(--safe-left))] pt-[calc(var(--safe-top)+10px)]">
              <HudButton label="Exit viewer" onClick={onExit}>
                <X size={20} />
              </HudButton>
              <div className="hud-glass flex min-w-0 flex-1 items-center gap-2 rounded-full px-3.5 py-2 text-[13px] font-semibold">
                <span className={cx('h-2 w-2 shrink-0 rounded-full', conn === 'live' ? 'dot-live bg-emerald-400' : conn === 'connecting' || conn === 'paused' ? 'bg-amber-400' : 'bg-rose-400')} />
                <span className="truncate">
                  {conn === 'live' ? 'Live' : conn === 'connecting' ? 'Connecting' : conn === 'paused' ? 'Paused' : 'Offline'} · {monitor.name}
                </span>
              </div>
              {fsEnabled() && (
                <HudButton label={isFs ? 'Exit full screen' : 'Full screen'} onClick={toggleFs}>
                  {isFs ? <Minimize size={18} /> : <Maximize size={18} />}
                </HudButton>
              )}
              <HudButton label="Hide controls" onClick={() => setHud(false)}>
                <EyeOff size={18} />
              </HudButton>
            </div>

            <div className="pointer-events-auto hud-glass absolute left-[max(12px,var(--safe-left))] top-[calc(var(--safe-top)+64px)] rounded-2xl px-3.5 py-2.5" data-testid="hud-stats">
              <div className="grid grid-cols-[auto_auto] gap-x-4 gap-y-0.5 font-mono text-[11.5px] leading-[1.55]">
                <Stat k="fps" v={stats ? `${stats.fps}` : '—'} />
                <Stat k="rtt" v={stats ? `${stats.rtt} ms` : '—'} />
                <Stat k="latency" v={latency != null ? `≈${latency} ms` : '—'} hl />
                <Stat k="rate" v={stats ? (stats.kbps >= 1000 ? `${(stats.kbps / 1000).toFixed(1)} Mbps` : `${stats.kbps} kbps`) : '—'} />
                <Stat k="quality" v={stats ? `${stats.quality}` : '—'} />
                <Stat k="size" v={frame ? `${frame.w}×${frame.h}` : '—'} />
                <Stat k="decode" v={decodeMs.current ? `${decodeMs.current.toFixed(1)} ms` : '—'} />
              </div>
            </div>

            {portraitForLandscape && rotateHint && !isFs && !kbOpen && (
              <div className="pointer-events-auto absolute inset-x-0 bottom-[calc(var(--safe-bottom)+112px)] flex justify-center px-4">
                <button onClick={() => setRotateHint(false)} className="hud-glass flex items-center gap-2 rounded-full px-3.5 py-2 text-xs font-semibold">
                  <Smartphone size={15} className="rotate-90" /> Turn sideways for a bigger view
                </button>
              </div>
            )}

            <div className="pointer-events-auto absolute inset-x-0 bottom-0 px-[max(12px,var(--safe-left))] pb-[calc(var(--safe-bottom)+12px)]" style={{ display: kbOpen ? 'none' : undefined }}>
              <div className="mx-auto flex max-w-[560px] items-center gap-2">
                {control ? (
                  <div className="hud-glass flex flex-1 rounded-2xl p-1">
                    {(
                      [
                        ['view', <Eye key="v" size={15} />, 'View'],
                        ['trackpad', <MousePointer2 key="t" size={15} />, 'Trackpad'],
                        ['touch', <Hand key="h" size={15} />, 'Touch'],
                      ] as [Mode, ReactNode, string][]
                    ).map(([m, icon, label]) => (
                      <button key={m} onClick={() => setMode(m)} aria-pressed={effMode === m} className={cx('flex h-10 min-w-0 flex-1 items-center justify-center gap-1 rounded-xl px-1 text-[12.5px] font-semibold transition-colors [&>svg]:shrink-0', effMode === m ? 'bg-white text-black' : 'text-white/80')}>
                        {icon}
                        {label}
                      </button>
                    ))}
                  </div>
                ) : (
                  <div className="hud-glass flex h-12 flex-1 items-center gap-2 rounded-2xl px-3.5 text-[12.5px] text-white/75">
                    <Eye size={16} className="shrink-0" /> View only · control is off on the PC
                  </div>
                )}
                {control && (
                  <HudButton label="Keyboard" onClick={() => setKbOpen(true)} big>
                    <Keyboard size={20} />
                  </HudButton>
                )}
                <HudButton label="Quality" onClick={() => setPresetOpen(true)} big>
                  <Gauge size={20} />
                </HudButton>
                {zoom.z > 1 && (
                  <HudButton label="Reset zoom" onClick={() => setZoom({ z: 1, x: 0, y: 0 })} big>
                    <ZoomOut size={20} />
                  </HudButton>
                )}
              </div>
              <div className="mt-2 text-center text-[11px] text-white/45">
                {effMode === 'view' ? 'Pinch to zoom · drag to pan · double-tap to reset · tap to hide controls' : effMode === 'trackpad' ? 'Drag to move · tap to click · two-finger tap = right click · two fingers to scroll' : 'Tap to click · long-press = right click · drag to drag · two fingers to scroll'}
              </div>
            </div>
          </motion.div>
        ) : (
          <motion.button
            key="show"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={() => setHud(true)}
            aria-label="Show controls"
            className="hud-glass absolute right-[max(12px,var(--safe-right))] top-[calc(var(--safe-top)+10px)] z-30 grid h-10 w-10 place-items-center rounded-full opacity-70"
          >
            <Eye size={18} />
          </motion.button>
        )}
      </AnimatePresence>

      {kbOpen && control && <KeyboardPanel send={send} onClose={() => setKbOpen(false)} />}

      <Sheet open={presetOpen} onClose={() => setPresetOpen(false)} title="Stream quality" subtitle={`Now: ${presetLabel}`}>
        <div className="space-y-2 pb-2">
          {presets.map((p) => (
            <Choice
              key={p.id}
              active={p.id === preset}
              onClick={() => {
                onPreset(p.id);
                send({ t: 'preset', id: p.id });
                setPresetOpen(false);
              }}
              icon={<Gauge size={19} />}
              title={p.label}
              sub={`up to ${p.fps} fps · quality ${p.quality} · ${p.maxWidth}px wide`}
            />
          ))}
          <HonestNote className="mt-3" />
        </div>
      </Sheet>
    </motion.div>
  );
}

function Stat({ k, v, hl }: { k: string; v: string; hl?: boolean }) {
  return (
    <>
      <span className="text-white/50">{k}</span>
      <span className={cx('text-right tabular-nums', hl ? 'font-semibold text-cyan-300' : 'text-white')}>{v}</span>
    </>
  );
}

function HudButton({ label, onClick, children, big }: { label: string; onClick: () => void; children: ReactNode; big?: boolean }) {
  return (
    <button aria-label={label} title={label} onClick={onClick} className={cx('hud-glass press grid shrink-0 place-items-center', big ? 'h-12 w-12 rounded-2xl' : 'h-10 w-10 rounded-full')}>
      {children}
    </button>
  );
}

// ---------- keyboard ----------

const KEYS: { label: ReactNode; key: string; aria: string }[] = [
  { label: 'Esc', key: 'Escape', aria: 'Escape' },
  { label: 'Tab', key: 'Tab', aria: 'Tab' },
  { label: <CornerDownLeft size={16} />, key: 'Enter', aria: 'Enter' },
  { label: <Delete size={16} />, key: 'Backspace', aria: 'Backspace' },
  { label: <ArrowLeft size={16} />, key: 'Left', aria: 'Left arrow' },
  { label: <ArrowUp size={16} />, key: 'Up', aria: 'Up arrow' },
  { label: <ArrowDown size={16} />, key: 'Down', aria: 'Down arrow' },
  { label: <ArrowRight size={16} />, key: 'Right', aria: 'Right arrow' },
  { label: 'Win', key: 'Win', aria: 'Windows key' },
  { label: 'Ctrl+C', key: 'Ctrl+C', aria: 'Copy' },
  { label: 'Ctrl+V', key: 'Ctrl+V', aria: 'Paste' },
  { label: 'Ctrl+Z', key: 'Ctrl+Z', aria: 'Undo' },
  { label: 'Alt+Tab', key: 'Alt+Tab', aria: 'Switch windows' },
  { label: 'Del', key: 'Delete', aria: 'Delete' },
];

function KeyboardPanel({ send, onClose }: { send: (m: ViewerMessage) => void; onClose: () => void }) {
  const [val, setVal] = useState('');
  const last = useRef('');
  const composing = useRef(false);
  const input = useRef<HTMLInputElement>(null);
  const [kbInset, setKbInset] = useState(0);
  useEffect(() => {
    input.current?.focus();
    const vv = window.visualViewport;
    if (!vv) return;
    const on = () => setKbInset(keyboardInset(vv));
    vv.addEventListener('resize', on);
    on();
    return () => vv.removeEventListener('resize', on);
  }, []);

  // Send the difference between what was typed before and now, so IME
  // composition and autocorrect turn into backspaces + text on the PC.
  const apply = (v: string) => {
    const prev = last.current;
    let i = 0;
    while (i < prev.length && i < v.length && prev[i] === v[i]) i++;
    for (let k = 0; k < prev.length - i; k++) send({ t: 'key', key: 'Backspace' });
    const added = v.slice(i);
    if (added) send({ t: 'text', text: added });
    last.current = v;
    if (!composing.current && v.length > 48) {
      last.current = '';
      setVal('');
    } else setVal(v);
  };

  const keys = useMemo(() => KEYS, []);
  return (
    <div className="absolute inset-x-0 z-40 border-t border-white/10 bg-[rgba(10,10,16,.92)] backdrop-blur-xl" style={{ bottom: kbInset }} onPointerDown={(e) => e.stopPropagation()}>
      <div className="hscroll flex gap-1.5 px-3 pt-3">
        {keys.map((k) => (
          <button
            key={k.key}
            aria-label={k.aria}
            onPointerDown={(e) => e.preventDefault()}
            onClick={() => {
              send({ t: 'key', key: k.key });
              vibrate(6);
            }}
            className="press grid h-10 min-w-[48px] shrink-0 place-items-center rounded-xl bg-white/10 px-3 font-mono text-[13px] font-semibold text-white active:bg-white/20"
          >
            {k.label}
          </button>
        ))}
      </div>
      <form
        className="flex items-center gap-2 px-3 pb-[calc(var(--safe-bottom)+10px)] pt-2.5"
        onSubmit={(e) => {
          e.preventDefault();
          send({ t: 'key', key: 'Enter' });
          last.current = '';
          setVal('');
        }}
      >
        <input
          ref={input}
          value={val}
          onChange={(e) => apply(e.target.value)}
          onCompositionStart={() => (composing.current = true)}
          onCompositionEnd={(e) => {
            composing.current = false;
            apply((e.target as HTMLInputElement).value);
          }}
          onKeyDown={(e) => {
            if (e.key === 'Backspace' && !val) send({ t: 'key', key: 'Backspace' });
          }}
          placeholder="Type to send to the PC…"
          autoCapitalize="off"
          autoCorrect="off"
          spellCheck={false}
          enterKeyHint="send"
          className="h-11 min-w-0 flex-1 rounded-xl border border-white/15 bg-white/10 px-3.5 text-white outline-none placeholder:text-white/40 focus:border-violet-400"
          aria-label="Text to type on the PC"
        />
        <button type="submit" aria-label="Enter" className="press grid h-11 w-11 place-items-center rounded-xl bg-white/10 text-white">
          <CornerDownLeft size={18} />
        </button>
        <button type="button" onClick={onClose} className="press flex h-11 items-center gap-1 rounded-xl bg-white px-3.5 text-sm font-semibold text-black">
          Done <ChevronRight size={15} />
        </button>
      </form>
    </div>
  );
}
