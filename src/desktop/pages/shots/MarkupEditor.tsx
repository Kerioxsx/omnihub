// Draw on a screenshot — pen, highlighter, arrows, boxes, text and blur —
// then save the result as a new screenshot or copy it. The original file
// is never changed.

import type { Screenshot } from '@shared/types';
import type { LucideIcon } from 'lucide-react';
import { ArrowUpRight, Clipboard, Grid3x3, Highlighter, Pencil, Redo2, Save, Square, Trash2, Type, Undo2 } from 'lucide-react';
import { type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent, useCallback, useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Spinner } from '../../components/ui/Card';
import { Modal } from '../../components/ui/Overlay';
import { cx } from '../../lib/cx';
import { toast } from '../../state/toasts';

type Tool = 'pen' | 'highlight' | 'arrow' | 'rect' | 'text' | 'blur';
type Pt = [number, number];
interface Shape {
  tool: Tool;
  color: string;
  width: number;
  points: Pt[];
  text?: string;
}

const TOOLS: { id: Tool; label: string; icon: LucideIcon; key: string }[] = [
  { id: 'pen', label: 'Pen', icon: Pencil, key: 'p' },
  { id: 'highlight', label: 'Highlighter', icon: Highlighter, key: 'h' },
  { id: 'arrow', label: 'Arrow', icon: ArrowUpRight, key: 'a' },
  { id: 'rect', label: 'Box', icon: Square, key: 'r' },
  { id: 'text', label: 'Text', icon: Type, key: 't' },
  { id: 'blur', label: 'Blur (hide)', icon: Grid3x3, key: 'b' },
];
const COLORS = ['#ff4d4f', '#ffb020', '#2ecc71', '#3b82f6', '#8b5cf6', '#ffffff', '#111111'];
const WIDTHS = [
  { label: 'Thin', k: 0.6 },
  { label: 'Medium', k: 1 },
  { label: 'Thick', k: 1.8 },
];

function drawArrow(ctx: CanvasRenderingContext2D, [x1, y1]: Pt, [x2, y2]: Pt, w: number) {
  const a = Math.atan2(y2 - y1, x2 - x1);
  const head = Math.max(10, w * 4.5);
  ctx.beginPath();
  ctx.moveTo(x1, y1);
  ctx.lineTo(x2 - Math.cos(a) * head * 0.6, y2 - Math.sin(a) * head * 0.6);
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(x2, y2);
  ctx.lineTo(x2 - head * Math.cos(a - 0.45), y2 - head * Math.sin(a - 0.45));
  ctx.lineTo(x2 - head * Math.cos(a + 0.45), y2 - head * Math.sin(a + 0.45));
  ctx.closePath();
  ctx.fill();
}

/** Pixelate a region using the original image, so blurred text cannot be recovered. */
function pixelate(ctx: CanvasRenderingContext2D, img: HTMLImageElement, [x1, y1]: Pt, [x2, y2]: Pt) {
  const x = Math.round(Math.min(x1, x2));
  const y = Math.round(Math.min(y1, y2));
  const w = Math.round(Math.abs(x2 - x1));
  const h = Math.round(Math.abs(y2 - y1));
  if (w < 2 || h < 2) return;
  const block = Math.max(8, Math.round(Math.min(img.naturalWidth, img.naturalHeight) / 90));
  const small = document.createElement('canvas');
  small.width = Math.max(1, Math.ceil(w / block));
  small.height = Math.max(1, Math.ceil(h / block));
  const s = small.getContext('2d');
  if (!s) return;
  s.drawImage(img, x, y, w, h, 0, 0, small.width, small.height);
  ctx.save();
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(small, 0, 0, small.width, small.height, x, y, w, h);
  ctx.restore();
}

function render(ctx: CanvasRenderingContext2D, img: HTMLImageElement, shapes: Shape[]) {
  ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
  ctx.drawImage(img, 0, 0);
  for (const s of shapes) {
    ctx.save();
    ctx.lineCap = 'round';
    ctx.lineJoin = 'round';
    ctx.strokeStyle = s.color;
    ctx.fillStyle = s.color;
    ctx.lineWidth = s.width;
    const [a, b] = [s.points[0], s.points[s.points.length - 1]];
    switch (s.tool) {
      case 'pen':
      case 'highlight':
        if (s.tool === 'highlight') {
          ctx.globalAlpha = 0.35;
          ctx.lineWidth = s.width * 5;
          ctx.lineCap = 'butt';
        }
        ctx.beginPath();
        s.points.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
        ctx.stroke();
        break;
      case 'arrow':
        drawArrow(ctx, a, b, s.width);
        break;
      case 'rect':
        ctx.strokeRect(Math.min(a[0], b[0]), Math.min(a[1], b[1]), Math.abs(b[0] - a[0]), Math.abs(b[1] - a[1]));
        break;
      case 'blur':
        pixelate(ctx, img, a, b);
        break;
      case 'text': {
        const size = Math.round(s.width * 7);
        ctx.font = `600 ${size}px system-ui, "Segoe UI", sans-serif`;
        ctx.textBaseline = 'top';
        // A dark outline keeps text readable on any background.
        ctx.lineWidth = Math.max(2, size / 7);
        ctx.strokeStyle = s.color === '#111111' ? '#ffffff' : 'rgba(0,0,0,0.75)';
        (s.text ?? '').split('\n').forEach((line, i) => {
          ctx.strokeText(line, a[0], a[1] + i * size * 1.2);
          ctx.fillText(line, a[0], a[1] + i * size * 1.2);
        });
        break;
      }
    }
    ctx.restore();
  }
}

export function MarkupEditor({ shot, open, onClose, onSaved }: { shot: Screenshot; open: boolean; onClose: () => void; onSaved: (s: Screenshot) => void }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [img, setImg] = useState<HTMLImageElement | null>(null);
  const [shapes, setShapes] = useState<Shape[]>([]);
  const [redo, setRedo] = useState<Shape[]>([]);
  const [tool, setTool] = useState<Tool>('arrow');
  const [color, setColor] = useState(COLORS[0]);
  const [widthK, setWidthK] = useState(1);
  const [draft, setDraft] = useState<Shape | null>(null);
  const [typing, setTyping] = useState<{ at: Pt; screen: Pt; value: string } | null>(null);
  const [busy, setBusy] = useState<'save' | 'copy' | null>(null);

  // Load the full image.
  useEffect(() => {
    if (!open) return;
    let alive = true;
    setImg(null);
    setShapes([]);
    setRedo([]);
    api.shots.image(shot.id).then(
      (url) => {
        const i = new Image();
        i.onload = () => alive && setImg(i);
        i.src = url;
      },
      (e: unknown) => toast.error('Could not open the screenshot', errorText(e)),
    );
    return () => {
      alive = false;
    };
  }, [open, shot.id]);

  const base = img ? Math.max(3, Math.round(Math.min(img.naturalWidth, img.naturalHeight) / 170)) : 4;
  const strokeW = Math.max(1, Math.round(base * widthK));

  useEffect(() => {
    const c = canvas.current;
    if (!c || !img) return;
    if (c.width !== img.naturalWidth) {
      c.width = img.naturalWidth;
      c.height = img.naturalHeight;
    }
    const ctx = c.getContext('2d');
    if (ctx) render(ctx, img, draft ? [...shapes, draft] : shapes);
  }, [img, shapes, draft]);

  const toImage = (e: { clientX: number; clientY: number }): Pt => {
    const c = canvas.current as HTMLCanvasElement;
    const r = c.getBoundingClientRect();
    return [((e.clientX - r.left) * c.width) / r.width, ((e.clientY - r.top) * c.height) / r.height];
  };

  const commit = useCallback((s: Shape) => {
    setShapes((list) => [...list, s]);
    setRedo([]);
  }, []);

  const onDown = (e: ReactPointerEvent<HTMLCanvasElement>) => {
    if (!img || e.button !== 0) return;
    // Text boxes are placed on click, after the browser has moved the focus.
    if (tool === 'text') return;
    const p = toImage(e);
    e.currentTarget.setPointerCapture(e.pointerId);
    setDraft({ tool, color, width: strokeW, points: [p, p] });
  };
  const onMove = (e: ReactPointerEvent<HTMLCanvasElement>) => {
    if (!draft) return;
    const p = toImage(e);
    setDraft((d) => (d ? { ...d, points: d.tool === 'pen' || d.tool === 'highlight' ? [...d.points, p] : [d.points[0], p] } : d));
  };
  const onClick = (e: ReactMouseEvent<HTMLCanvasElement>) => {
    if (tool !== 'text' || !img) return;
    if (typing?.value.trim()) commit({ tool: 'text', color, width: strokeW, points: [typing.at], text: typing.value });
    setTyping({ at: toImage(e), screen: [e.clientX, e.clientY], value: '' });
  };
  const onUp = () => {
    if (!draft) return;
    const [a, b] = [draft.points[0], draft.points[draft.points.length - 1]];
    const moved = Math.hypot(b[0] - a[0], b[1] - a[1]) > 3 || draft.points.length > 2;
    if (moved) commit(draft);
    setDraft(null);
  };

  const undo = useCallback(() => {
    setShapes((list) => {
      if (!list.length) return list;
      setRedo((r) => [...r, list[list.length - 1]]);
      return list.slice(0, -1);
    });
  }, []);
  const redoOne = useCallback(() => {
    setRedo((r) => {
      if (!r.length) return r;
      setShapes((list) => [...list, r[r.length - 1]]);
      return r.slice(0, -1);
    });
  }, []);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement).closest('input, textarea')) return;
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
        e.preventDefault();
        if (e.shiftKey) redoOne();
        else undo();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'y') {
        e.preventDefault();
        redoOne();
      } else if (!e.ctrlKey && !e.metaKey && !e.altKey) {
        const t = TOOLS.find((x) => x.key === e.key.toLowerCase());
        if (t) setTool(t.id);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, undo, redoOne]);

  const finishTyping = () => {
    if (typing?.value.trim()) commit({ tool: 'text', color, width: strokeW, points: [typing.at], text: typing.value });
    setTyping(null);
  };

  const save = async () => {
    const c = canvas.current;
    if (!c) return;
    setBusy('save');
    try {
      const s = await api.shots.saveEdit(shot.id, c.toDataURL('image/png'));
      toast.success('Saved as a new screenshot', 'The original is unchanged.');
      onSaved(s);
    } catch (e) {
      toast.error('Could not save', errorText(e));
    } finally {
      setBusy(null);
    }
  };
  const copy = async () => {
    const c = canvas.current;
    if (!c) return;
    setBusy('copy');
    try {
      const blob = await new Promise<Blob | null>((res) => c.toBlob(res, 'image/png'));
      if (!blob) throw new Error('could not render the image');
      await navigator.clipboard.write([new ClipboardItem({ 'image/png': blob })]);
      toast.success('Copied to the clipboard');
    } catch (e) {
      toast.error('Could not copy', errorText(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <Modal open={open} onClose={onClose} bare className="!fixed inset-3 !w-auto !max-w-none" labelledBy="markup-title">
      <div className="flex h-full flex-col overflow-hidden rounded-[20px] border border-line-strong bg-[#050508]/95 shadow-2xl">
        <div className="flex flex-wrap items-center gap-2 border-b border-line px-4 py-2.5">
          <span id="markup-title" className="mr-2 text-[13.5px] font-semibold text-fg">
            Mark up
          </span>
          <div className="flex rounded-[11px] border border-line bg-surface p-0.5" role="radiogroup" aria-label="Tool">
            {TOOLS.map((t) => (
              <IconButton key={t.id} icon={t.icon} label={`${t.label} (${t.key.toUpperCase()})`} size="sm" active={tool === t.id} onClick={() => setTool(t.id)} role="radio" aria-checked={tool === t.id} />
            ))}
          </div>
          <div className="flex items-center gap-1 rounded-[11px] border border-line bg-surface px-1.5 py-1" role="radiogroup" aria-label="Colour">
            {COLORS.map((c) => (
              <button
                key={c}
                type="button"
                role="radio"
                aria-checked={color === c}
                aria-label={`Colour ${c}`}
                onClick={() => setColor(c)}
                className={cx('h-5 w-5 rounded-full border transition-transform', color === c ? 'scale-110 border-white ring-2 ring-accent/60' : 'border-white/20 hover:scale-105')}
                style={{ background: c }}
              />
            ))}
          </div>
          <div className="flex rounded-[11px] border border-line bg-surface p-0.5" role="radiogroup" aria-label="Thickness">
            {WIDTHS.map((w) => (
              <button key={w.label} type="button" role="radio" aria-checked={widthK === w.k} onClick={() => setWidthK(w.k)} className={cx('flex h-7 w-9 items-center justify-center rounded-lg', widthK === w.k ? 'bg-surface-3' : 'hover:bg-surface-2')} title={w.label} aria-label={w.label}>
                <span className="rounded-full bg-fg" style={{ width: 16, height: Math.max(1.5, 2.2 * w.k) }} />
              </button>
            ))}
          </div>
          <IconButton icon={Undo2} label="Undo (Ctrl+Z)" size="sm" disabled={!shapes.length} onClick={undo} />
          <IconButton icon={Redo2} label="Redo (Ctrl+Y)" size="sm" disabled={!redo.length} onClick={redoOne} />
          <IconButton icon={Trash2} label="Remove all marks" size="sm" disabled={!shapes.length} onClick={() => setShapes([])} />
          <div className="flex-1" />
          <Button size="sm" variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button size="sm" icon={Clipboard} loading={busy === 'copy'} disabled={!img} onClick={() => void copy()}>
            Copy
          </Button>
          <Button size="sm" variant="primary" icon={Save} loading={busy === 'save'} disabled={!img || !shapes.length} onClick={() => void save()}>
            Save as copy
          </Button>
        </div>
        <div className="relative flex min-h-0 flex-1 items-center justify-center p-4">
          {!img && <Spinner size={22} />}
          <canvas
            ref={canvas}
            onPointerDown={onDown}
            onPointerMove={onMove}
            onPointerUp={onUp}
            onPointerCancel={onUp}
            onClick={onClick}
            className={cx('max-h-full max-w-full rounded-lg shadow-[0_30px_80px_-20px_rgba(0,0,0,0.8)]', !img && 'hidden', tool === 'text' ? 'cursor-text' : 'cursor-crosshair')}
            aria-label="Screenshot being marked up"
          />
          {typing && (
            <textarea
              autoFocus
              value={typing.value}
              onChange={(e) => setTyping({ ...typing, value: e.target.value })}
              onBlur={finishTyping}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  finishTyping();
                } else if (e.key === 'Escape') {
                  e.stopPropagation();
                  setTyping(null);
                }
              }}
              placeholder="Type, then Enter"
              rows={1}
              className="fixed z-[90] min-w-[160px] resize-none rounded-md border border-accent/60 bg-black/70 px-2 py-1 text-[15px] font-semibold outline-none"
              style={{ left: typing.screen[0], top: typing.screen[1], color }}
              aria-label="Text to add"
            />
          )}
        </div>
      </div>
    </Modal>
  );
}
