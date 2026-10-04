import { formatBytes, formatDateTime } from '@shared/format';
import type { Screenshot } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { ChevronLeft, ChevronRight, Clipboard, ExternalLink, FolderSearch, PenTool, ScanText, Smartphone, Star, Tag, Trash, X, ZoomIn, ZoomOut } from 'lucide-react';
import { type PointerEvent, type WheelEvent, useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Meta, Spinner } from '../../components/ui/Card';
import { Textarea } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { cx } from '../../lib/cx';
import { forgetThumb } from '../../lib/media';
import { clamp } from '../../lib/util';
import { confirm } from '../../state/dialogs';
import { useSend } from '../../state/send';
import { copyText } from '../../lib/util';
import { toast } from '../../state/toasts';
import { MarkupEditor } from './MarkupEditor';

const images = new Map<string, string>();

function useImage(id: string | null): string | null {
  const [url, setUrl] = useState<string | null>(id ? (images.get(id) ?? null) : null);
  useEffect(() => {
    if (!id) return;
    const cached = images.get(id);
    if (cached) {
      setUrl(cached);
      return;
    }
    setUrl(null);
    let alive = true;
    api.shots
      .image(id)
      .then((u) => {
        images.set(id, u);
        if (alive) setUrl(u);
      })
      .catch((e: unknown) => toast.error('Could not load the image', errorText(e)));
    return () => {
      alive = false;
    };
  }, [id]);
  return url;
}

function Viewer({ shot }: { shot: Screenshot }) {
  const url = useImage(shot.id);
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const drag = useRef<{ x: number; y: number; px: number; py: number } | null>(null);

  useEffect(() => {
    setZoom(1);
    setPan({ x: 0, y: 0 });
  }, [shot.id]);

  const onWheel = (e: WheelEvent) => {
    const next = clamp(zoom * (e.deltaY < 0 ? 1.15 : 1 / 1.15), 1, 6);
    setZoom(next);
    if (next === 1) setPan({ x: 0, y: 0 });
  };
  const onDown = (e: PointerEvent) => {
    if (zoom === 1) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = { x: e.clientX, y: e.clientY, px: pan.x, py: pan.y };
  };
  const onMove = (e: PointerEvent) => {
    const d = drag.current;
    if (d) setPan({ x: d.px + (e.clientX - d.x) / zoom, y: d.py + (e.clientY - d.y) / zoom });
  };

  return (
    <div className="relative flex h-full min-w-0 flex-1 items-center justify-center overflow-hidden" onWheel={onWheel}>
      {!url && <Spinner size={28} />}
      {url && (
        <motion.img
          key={shot.id}
          src={url}
          alt={shot.appTitle ?? 'Screenshot'}
          draggable={false}
          initial={{ opacity: 0, scale: 0.98 }}
          animate={{ opacity: 1, scale: zoom, x: pan.x * zoom, y: pan.y * zoom }}
          transition={{ type: 'spring', stiffness: 300, damping: 32 }}
          onDoubleClick={() => {
            setZoom(zoom > 1 ? 1 : 2.2);
            setPan({ x: 0, y: 0 });
          }}
          onPointerDown={onDown}
          onPointerMove={onMove}
          onPointerUp={() => (drag.current = null)}
          className={cx('max-h-[calc(100vh-140px)] max-w-full select-none rounded-lg shadow-[0_30px_80px_-20px_rgba(0,0,0,0.8)]', zoom > 1 ? 'cursor-grab active:cursor-grabbing' : 'cursor-zoom-in')}
        />
      )}
      <div className="glass absolute bottom-4 left-1/2 flex -translate-x-1/2 items-center gap-1 rounded-full border border-line-strong px-1.5 py-1">
        <IconButton icon={ZoomOut} label="Zoom out" size="sm" onClick={() => setZoom((z) => Math.max(1, z / 1.4))} />
        <span className="w-12 text-center text-[12px] tabular text-dim">{Math.round(zoom * 100)}%</span>
        <IconButton icon={ZoomIn} label="Zoom in" size="sm" onClick={() => setZoom((z) => Math.min(6, z * 1.4))} />
      </div>
    </div>
  );
}

/** "Copy text": the words Windows' OCR finds in the screenshot. */
function TextDialog({ shot, open, onClose, onRead }: { shot: Screenshot; open: boolean; onClose: () => void; onRead: () => void }) {
  const [text, setText] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!open) return;
    let alive = true;
    setText(null);
    setError(null);
    api.shots.text(shot.id).then(
      (t) => {
        if (!alive) return;
        setText(t);
        onRead();
      },
      (e: unknown) => alive && setError(errorText(e)),
    );
    return () => {
      alive = false;
    };
  }, [open, shot.id]);
  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Text in this screenshot"
      description="Read on this PC with the text recognition built into Windows."
      icon={ScanText}
      size="lg"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Close
          </Button>
          <Button variant="primary" icon={Clipboard} disabled={!text} onClick={() => text && void copyText(text).then(() => toast.success('Text copied'))}>
            Copy all
          </Button>
        </>
      }
    >
      <div className="pb-2">
        {error ? (
          <div className="rounded-xl border border-bad/30 bg-bad/8 px-4 py-3 text-[13px] text-bad">{error}</div>
        ) : text == null ? (
          <div className="flex items-center gap-2 py-8 text-[13px] text-dim">
            <Spinner /> Reading the text…
          </div>
        ) : text.trim() ? (
          <Textarea value={text} onChange={(e) => setText(e.target.value)} rows={12} className="font-mono text-[13px]" aria-label="Recognised text" />
        ) : (
          <div className="py-6 text-[13px] text-faint">No text found in this screenshot.</div>
        )}
        {text?.trim() && <p className="mt-2 text-[12px] text-faint">You can now find this screenshot by searching for any of these words.</p>}
      </div>
    </Modal>
  );
}

function Details({ shot, onChange, onDeleted, onCreated }: { shot: Screenshot; onChange: (s: Screenshot) => void; onDeleted: () => void; onCreated: (s: Screenshot) => void }) {
  const [note, setNote] = useState(shot.note);
  const [tagInput, setTagInput] = useState('');
  const [editing, setEditing] = useState(false);
  const [reading, setReading] = useState(false);
  useEffect(() => setNote(shot.note), [shot.id, shot.note]);

  const patch = async (p: { tags?: string[]; note?: string; favorite?: boolean }) => {
    try {
      onChange(await api.shots.update(shot.id, p));
    } catch (e) {
      toast.error('Could not save', errorText(e));
    }
  };
  const addTag = () => {
    const t = tagInput.trim().toLowerCase();
    if (!t || shot.tags.includes(t)) return setTagInput('');
    void patch({ tags: [...shot.tags, t] });
    setTagInput('');
  };
  const remove = async () => {
    const ok = await confirm({ title: 'Delete this screenshot?', description: 'The file moves to the Recycle Bin.', tone: 'danger', confirmLabel: 'Delete' });
    if (!ok) return;
    try {
      await api.shots.delete(shot.id, true);
      forgetThumb(shot.id);
      toast.success('Screenshot deleted');
      onDeleted();
    } catch (e) {
      toast.error('Could not delete', errorText(e));
    }
  };
  const act = (label: string, fn: () => Promise<void>, ok?: string) => () =>
    void fn()
      .then(() => ok && toast.success(ok))
      .catch((e: unknown) => toast.error(label, errorText(e)));

  return (
    <aside className="flex w-[330px] shrink-0 flex-col border-l border-line bg-elev/90">
      <div className="flex items-start gap-2 border-b border-line px-5 py-4">
        <div className="min-w-0 flex-1">
          <div className="truncate text-[14px] font-semibold text-fg">{shot.appTitle ?? 'Full screen'}</div>
          <div className="text-[12px] text-faint">{formatDateTime(shot.created)}</div>
        </div>
        <IconButton icon={Star} label={shot.favorite ? 'Remove from favourites' : 'Add to favourites'} active={shot.favorite} onClick={() => void patch({ favorite: !shot.favorite })} className={shot.favorite ? '[&_svg]:fill-current text-warn' : undefined} />
      </div>
      <div className="flex-1 space-y-5 overflow-y-auto px-5 py-4">
        <div className="grid grid-cols-2 gap-3">
          <Meta label="Size">
            {shot.width} × {shot.height}
          </Meta>
          <Meta label="File">{formatBytes(shot.bytes)}</Meta>
          <Meta label="App">{shot.appExe ?? '—'}</Meta>
          <Meta label="Format">{shot.path.split('.').pop()?.toUpperCase()}</Meta>
        </div>
        <div>
          <div className="mb-1.5 text-[11px] font-medium uppercase tracking-wider text-faint">Tags</div>
          <div className="flex flex-wrap gap-1.5">
            {shot.tags.map((t) => (
              <span key={t} className="inline-flex h-7 items-center gap-1 rounded-full border border-accent/30 bg-accent-soft pl-2.5 pr-1 text-[12px] text-accent">
                {t}
                <button type="button" aria-label={`Remove tag ${t}`} onClick={() => void patch({ tags: shot.tags.filter((x) => x !== t) })} className="flex h-5 w-5 items-center justify-center rounded-full hover:bg-accent/20">
                  <X size={11} />
                </button>
              </span>
            ))}
            <form
              onSubmit={(e) => {
                e.preventDefault();
                addTag();
              }}
              className="inline-flex h-7 items-center gap-1 rounded-full border border-dashed border-line-strong px-2.5"
            >
              <Tag size={11} className="text-faint" aria-hidden />
              <input value={tagInput} onChange={(e) => setTagInput(e.target.value)} onBlur={addTag} placeholder="Add tag" aria-label="Add tag" className="w-20 bg-transparent text-[12px] text-fg outline-none placeholder:text-faint" />
            </form>
          </div>
        </div>
        <div>
          <label htmlFor="shot-note" className="mb-1.5 block text-[11px] font-medium uppercase tracking-wider text-faint">
            Note
          </label>
          <Textarea id="shot-note" value={note} onChange={(e) => setNote(e.target.value)} onBlur={() => note !== shot.note && void patch({ note })} placeholder="What's this about?" rows={3} />
        </div>
        <div className="break-all rounded-xl border border-line bg-surface px-3 py-2 font-mono text-[11px] text-faint">{shot.path}</div>
      </div>
      <div className="grid grid-cols-2 gap-2 border-t border-line p-4">
        <Button size="sm" variant="primary" icon={PenTool} onClick={() => setEditing(true)}>
          Mark up
        </Button>
        <Button size="sm" icon={ScanText} onClick={() => setReading(true)}>
          Copy text
        </Button>
        <Button size="sm" icon={Clipboard} onClick={act('Could not copy', () => api.shots.copy(shot.id), 'Copied to the clipboard')}>
          Copy
        </Button>
        <Button size="sm" icon={Smartphone} onClick={() => useSend.getState().openFiles([shot.path])}>
          Send to phone
        </Button>
        <Button size="sm" icon={FolderSearch} onClick={act('Could not open Explorer', () => api.app.revealPath(shot.path))}>
          Show in folder
        </Button>
        <Button size="sm" icon={ExternalLink} onClick={act('Could not open', () => api.app.openPath(shot.path))}>
          Open
        </Button>
        <Button size="sm" variant="danger" icon={Trash} onClick={() => void remove()}>
          Delete
        </Button>
      </div>
      <MarkupEditor
        shot={shot}
        open={editing}
        onClose={() => setEditing(false)}
        onSaved={(s) => {
          setEditing(false);
          onCreated(s);
        }}
      />
      <TextDialog shot={shot} open={reading} onClose={() => setReading(false)} onRead={() => !shot.hasText && onChange({ ...shot, hasText: true })} />
    </aside>
  );
}

export function Lightbox({ shots, openId, onClose, onNavigate, onChange, onCreated }: { shots: Screenshot[]; openId: string | null; onClose: () => void; onNavigate: (id: string) => void; onChange: (s: Screenshot) => void; onCreated?: (s: Screenshot) => void }) {
  const idx = openId ? shots.findIndex((s) => s.id === openId) : -1;
  const shot = idx >= 0 ? shots[idx] : null;
  const prev = idx > 0 ? shots[idx - 1] : null;
  const next = idx >= 0 && idx < shots.length - 1 ? shots[idx + 1] : null;

  useEffect(() => {
    if (!shot) return;
    const onKey = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement).closest('input, textarea')) return;
      if (e.key === 'ArrowLeft' && prev) onNavigate(prev.id);
      if (e.key === 'ArrowRight' && next) onNavigate(next.id);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [shot, prev, next, onNavigate]);

  return (
    <Modal open={!!shot} onClose={onClose} bare className="!fixed inset-3 !w-auto !max-w-none" labelledBy="lightbox-title">
      {shot && (
        <div className="flex h-full overflow-hidden rounded-[20px] border border-line-strong bg-[#050508]/95 shadow-2xl">
          <div className="relative flex min-w-0 flex-1 flex-col">
            <div className="flex items-center gap-2 px-4 py-3">
              <span id="lightbox-title" className="text-[13px] text-dim">
                {idx + 1} / {shots.length}
              </span>
              <div className="flex-1" />
              <IconButton icon={X} label="Close (Esc)" onClick={onClose} data-autofocus />
            </div>
            <div className="relative flex min-h-0 flex-1 px-14 pb-4">
              <AnimatePresence mode="wait">
                <Viewer key={shot.id} shot={shot} />
              </AnimatePresence>
              {prev && (
                <button type="button" aria-label="Previous screenshot" onClick={() => onNavigate(prev.id)} className="absolute left-3 top-1/2 flex h-11 w-11 -translate-y-1/2 items-center justify-center rounded-full border border-line-strong bg-black/40 text-white backdrop-blur transition hover:bg-black/60">
                  <ChevronLeft size={20} />
                </button>
              )}
              {next && (
                <button type="button" aria-label="Next screenshot" onClick={() => onNavigate(next.id)} className="absolute right-3 top-1/2 flex h-11 w-11 -translate-y-1/2 items-center justify-center rounded-full border border-line-strong bg-black/40 text-white backdrop-blur transition hover:bg-black/60">
                  <ChevronRight size={20} />
                </button>
              )}
            </div>
          </div>
          <Details
            shot={shot}
            onChange={onChange}
            onCreated={(s) => (onCreated ? onCreated(s) : onNavigate(s.id))}
            onDeleted={() => {
              if (next) onNavigate(next.id);
              else if (prev) onNavigate(prev.id);
              else onClose();
            }}
          />
        </div>
      )}
    </Modal>
  );
}
