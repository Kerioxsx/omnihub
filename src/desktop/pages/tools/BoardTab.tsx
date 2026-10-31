// The phone's whiteboard: what's drawn on the phone (More → Whiteboard)
// shows up here live, and can be saved as a picture.

import type { BoardStroke } from '@shared/types';
import { Eraser, PenTool, Save, Undo2 } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Segmented } from '../../components/ui/Form';
import { useEvent } from '../../lib/hooks';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';

/** Width ÷ height of the board, the same on the phone. */
export const BOARD_ASPECT = 4 / 3;

export function drawBoard(ctx: CanvasRenderingContext2D, strokes: BoardStroke[], w: number, h: number, paper: string) {
  ctx.fillStyle = paper;
  ctx.fillRect(0, 0, w, h);
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  for (const s of strokes) {
    ctx.strokeStyle = s.eraser ? paper : s.color;
    ctx.lineWidth = Math.max(1, s.width * w);
    ctx.beginPath();
    s.points.forEach(([x, y], i) => (i ? ctx.lineTo(x * w, y * h) : ctx.moveTo(x * w, y * h)));
    if (s.points.length === 1) ctx.lineTo(s.points[0][0] * w + 0.1, s.points[0][1] * h);
    ctx.stroke();
  }
}

export function BoardTab() {
  const [strokes, setStrokes] = useState<BoardStroke[]>([]);
  const [paper, setPaper] = useState<'white' | 'dark'>('white');
  const canvas = useRef<HTMLCanvasElement>(null);
  const load = useCallback(() => void api.board.strokes().then(setStrokes, () => undefined), []);
  useEffect(load, [load]);
  useEvent<{ strokes: BoardStroke[] }>('board:strokes', (p) => setStrokes((s) => [...s, ...(p?.strokes ?? [])]));
  useEvent('board:undo', () => setStrokes((s) => s.slice(0, -1)));
  useEvent('board:clear', () => setStrokes([]));

  const color = paper === 'white' ? '#ffffff' : '#0f1115';
  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const draw = () => {
      const r = c.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      c.width = Math.round(r.width * dpr);
      c.height = Math.round(r.height * dpr);
      const ctx = c.getContext('2d');
      if (ctx) drawBoard(ctx, strokes, c.width, c.height, color);
    };
    draw();
    const ro = new ResizeObserver(draw);
    ro.observe(c);
    return () => ro.disconnect();
  }, [strokes, color]);

  const save = async () => {
    // A fixed 1600-wide picture, whatever the window's size.
    const c = document.createElement('canvas');
    c.width = 1600;
    c.height = Math.round(1600 / BOARD_ASPECT);
    const ctx = c.getContext('2d');
    if (!ctx) return;
    drawBoard(ctx, strokes, c.width, c.height, color);
    try {
      const path = await api.board.save(c.toDataURL('image/png'));
      toast.success('Saved', path, { action: { label: 'Show', run: () => void api.app.revealPath(path) } });
    } catch (e) {
      toast.error('Couldn’t save', errorText(e));
    }
  };

  const clear = async () => {
    if (await confirm({ title: 'Clear the whiteboard?', description: 'On the phone too.', confirmLabel: 'Clear', tone: 'danger' })) void api.board.clear();
  };

  return (
    <Card className="p-5">
      <CardHeader
        icon={PenTool}
        title="Whiteboard"
        subtitle="Draw on your phone (More → Whiteboard) and it shows up here as you draw. Good for sketching an idea on a call or a screen share."
        actions={
          <div className="flex items-center gap-2">
            <Segmented label="Paper" size="sm" value={paper} onChange={setPaper} options={[{ value: 'white', label: 'White' }, { value: 'dark', label: 'Dark' }]} />
            <Button size="sm" icon={Undo2} disabled={!strokes.length} onClick={() => void api.board.undo()}>
              Undo
            </Button>
            <Button size="sm" icon={Eraser} disabled={!strokes.length} onClick={() => void clear()}>
              Clear
            </Button>
            <Button size="sm" variant="primary" icon={Save} disabled={!strokes.length} onClick={() => void save()}>
              Save picture
            </Button>
          </div>
        }
      />
      <div className="mx-auto mt-4 w-full max-w-4xl overflow-hidden rounded-xl border border-line shadow-sm" style={{ aspectRatio: String(BOARD_ASPECT) }}>
        <canvas ref={canvas} className="h-full w-full" aria-label="Whiteboard drawn on the phone" />
      </div>
      {!strokes.length && <p className="mt-3 text-center text-[13px] text-faint">Nothing drawn yet. Open OmniHub on your phone → More → Whiteboard.</p>}
    </Card>
  );
}
