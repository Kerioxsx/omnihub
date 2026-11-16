// Scan documents: take a photo, the page's corners are found (drag them if
// they're off), it's straightened and cleaned up, and the pages go to the PC
// as a PDF with their text — or into a note. Receipts and business cards are
// read into fields.

import type { Scanned, ScanKind, ScanTarget } from '@shared/types';
import { Camera, Check, FileText, ImagePlus, Receipt, RotateCcw, Send, Trash2, UserSquare } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { client } from '../client';
import { findPage, jpegBase64, loadPhoto, straighten, type Look, type Quad } from '../lib/scan';
import { cx, errorMessage, vibrate } from '../lib/util';
import { toast, useApp } from '../state';
import { Button, Empty, Segmented } from '../ui/common';
import { SubHeader } from './More';

interface Page {
  id: number;
  canvas: HTMLCanvasElement;
  url: string;
}

let seq = 0;

/** Drag the four corners of the page on the photo. */
function Corners({ photo, quad, onChange }: { photo: HTMLCanvasElement; quad: Quad; onChange: (q: Quad) => void }) {
  const box = useRef<HTMLDivElement>(null);
  const [url] = useState(() => photo.toDataURL('image/jpeg', 0.8));
  const drag = useRef<number | null>(null);
  const move = (e: React.PointerEvent) => {
    if (drag.current == null || !box.current) return;
    const r = box.current.getBoundingClientRect();
    const x = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
    const y = Math.min(1, Math.max(0, (e.clientY - r.top) / r.height));
    const q = quad.map((p) => [...p]) as Quad;
    q[drag.current] = [x, y];
    onChange(q);
  };
  return (
    <div className="flex justify-center">
      <div
        ref={box}
        className="relative touch-none select-none"
        style={{ aspectRatio: `${photo.width} / ${photo.height}`, maxHeight: '58vh', width: photo.width >= photo.height ? '100%' : 'auto', height: photo.width >= photo.height ? 'auto' : '58vh' }}
        onPointerMove={move}
        onPointerUp={() => (drag.current = null)}
        onPointerCancel={() => (drag.current = null)}
      >
        <img src={url} alt="" className="h-full w-full rounded-xl" draggable={false} />
        <svg className="pointer-events-none absolute inset-0 h-full w-full" viewBox="0 0 1 1" preserveAspectRatio="none">
          <polygon points={quad.map((p) => p.join(',')).join(' ')} fill="rgba(99,102,241,.18)" stroke="rgb(129,140,248)" strokeWidth="0.006" vectorEffect="non-scaling-stroke" />
        </svg>
        {quad.map((p, i) => (
          <button
            key={i}
            type="button"
            aria-label={['Top left', 'Top right', 'Bottom right', 'Bottom left'][i] + ' corner'}
            className="absolute -ml-5 -mt-5 grid h-10 w-10 place-items-center"
            style={{ left: `${p[0] * 100}%`, top: `${p[1] * 100}%` }}
            onPointerDown={(e) => {
              (e.target as HTMLElement).setPointerCapture?.(e.pointerId);
              drag.current = i;
            }}
          >
            <span className="h-5 w-5 rounded-full border-2 border-white bg-indigo-500 shadow-lg" />
          </button>
        ))}
      </div>
    </div>
  );
}

export function ScanPage({ onBack }: { onBack: () => void }) {
  const f = useApp((s) => s.info?.features);
  const input = useRef<HTMLInputElement>(null);
  const [photo, setPhoto] = useState<HTMLCanvasElement | null>(null);
  const [quad, setQuad] = useState<Quad | null>(null);
  const [look, setLook] = useState<Look>('color');
  const [pages, setPages] = useState<Page[]>([]);
  const [kind, setKind] = useState<ScanKind>('document');
  const [target, setTarget] = useState<ScanTarget>('folder');
  const [title, setTitle] = useState('');
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<Scanned | null>(null);

  useEffect(() => () => pages.forEach((p) => URL.revokeObjectURL(p.url)), [pages]);

  const take = async (file: File | undefined) => {
    if (!file) return;
    try {
      const c = await loadPhoto(file);
      setPhoto(c);
      setQuad(findPage(c));
      setDone(null);
    } catch (e) {
      toast.error('Couldn’t read the photo', errorMessage(e));
    }
  };

  const keep = async () => {
    if (!photo || !quad) return;
    setBusy(true);
    await new Promise((r) => setTimeout(r, 20));
    const c = straighten(photo, quad, look);
    const blob = await new Promise<Blob | null>((r) => c.toBlob(r, 'image/jpeg', 0.7));
    setPages((p) => [...p, { id: ++seq, canvas: c, url: blob ? URL.createObjectURL(blob) : '' }]);
    setPhoto(null);
    setQuad(null);
    setBusy(false);
    vibrate(15);
  };

  const send = async () => {
    setBusy(true);
    try {
      const r = await client.scan({ kind, title: title.trim(), target, pages: pages.map((p) => jpegBase64(p.canvas)) });
      setDone(r);
      setPages([]);
      setTitle('');
      toast.success(target === 'notes' ? 'Saved as a note on the PC' : 'Saved on the PC', r.files[0]?.split(/[\\/]/).pop());
    } catch (e) {
      toast.error('Couldn’t send it', errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const pick = (capture: boolean) => {
    if (!input.current) return;
    if (capture) input.current.setAttribute('capture', 'environment');
    else input.current.removeAttribute('capture');
    input.current.click();
  };

  return (
    <div className="scroller h-full">
      <SubHeader title="Scan" subtitle="Documents, receipts and business cards" onBack={onBack} />
      <input ref={input} type="file" accept="image/*" className="hidden" onChange={(e) => void take(e.target.files?.[0]).finally(() => (e.target.value = ''))} />
      <div className="px-safe space-y-4 pb-tabbar">
        {!f?.uploads ? (
          <Empty icon={<FileText size={28} />} title="Sending files to the PC is off" body="Turn it on in OmniHub on the PC (Phone → Permissions)." />
        ) : photo && quad ? (
          <div className="card space-y-3 p-3">
            <p className="px-1 text-[13px] text-dim">Drag the corners onto the page’s corners.</p>
            <Corners photo={photo} quad={quad} onChange={setQuad} />
            <Segmented value={look} onChange={setLook} options={[{ value: 'color', label: 'Colour' }, { value: 'gray', label: 'Gray' }, { value: 'bw', label: 'Black & white' }]} />
            <div className="flex gap-2.5">
              <Button variant="ghost" className="flex-1" icon={<RotateCcw size={18} />} onClick={() => (setPhoto(null), setQuad(null))}>
                Retake
              </Button>
              <Button className="flex-1" icon={<Check size={18} />} loading={busy} onClick={() => void keep()}>
                Keep page
              </Button>
            </div>
          </div>
        ) : (
          <>
            <div className="grid grid-cols-2 gap-2.5">
              <Button size="lg" icon={<Camera size={20} />} onClick={() => pick(true)}>
                {pages.length ? 'Next page' : 'Take a photo'}
              </Button>
              <Button size="lg" variant="ghost" icon={<ImagePlus size={20} />} onClick={() => pick(false)}>
                From photos
              </Button>
            </div>
            {pages.length > 0 && (
              <div className="card space-y-3 p-3">
                <div className="flex gap-2 overflow-x-auto pb-1">
                  {pages.map((p, i) => (
                    <div key={p.id} className="relative shrink-0">
                      <img src={p.url} alt={`Page ${i + 1}`} className="h-28 rounded-lg border border-line object-contain" />
                      <button type="button" aria-label={`Remove page ${i + 1}`} className="absolute right-1 top-1 grid h-7 w-7 place-items-center rounded-full bg-black/60 text-white" onClick={() => setPages((x) => x.filter((y) => y.id !== p.id))}>
                        <Trash2 size={14} />
                      </button>
                      <span className="absolute bottom-1 left-1 rounded bg-black/60 px-1.5 text-[11px] text-white">{i + 1}</span>
                    </div>
                  ))}
                </div>
                <Segmented
                  value={kind}
                  onChange={setKind}
                  options={[
                    { value: 'document', label: <><FileText size={15} /> Document</> },
                    { value: 'receipt', label: <><Receipt size={15} /> Receipt</> },
                    { value: 'card', label: <><UserSquare size={15} /> Card</> },
                  ]}
                />
                {f?.notes && <Segmented value={target} onChange={setTarget} options={[{ value: 'folder', label: 'Scans folder' }, { value: 'notes', label: 'A note' }]} />}
                <input value={title} onChange={(e) => setTitle(e.target.value)} placeholder={kind === 'receipt' ? 'Name (optional), e.g. Groceries' : kind === 'card' ? 'Name (optional)' : 'Name (optional), e.g. Lease'} className="h-12 w-full rounded-2xl bg-surface-2 px-4 text-[15px] outline-none" />
                <Button className="w-full" size="lg" icon={<Send size={18} />} loading={busy} onClick={() => void send()}>
                  Send {pages.length} {pages.length === 1 ? 'page' : 'pages'} to the PC
                </Button>
              </div>
            )}
            {!pages.length && !done && (
              <p className="px-2 text-center text-[13px] leading-relaxed text-faint">
                Put the page on a darker surface for the best edges. Several photos become one PDF; the PC reads the text with Windows’ own text recognition, nothing goes online.
              </p>
            )}
            {done && (
              <div className="card space-y-2 p-4">
                <div className="flex items-center gap-2 text-[15px] font-semibold text-good">
                  <Check size={18} /> Saved on the PC
                </div>
                <div className="text-[13px] text-dim">{done.files.map((p) => p.split(/[\\/]/).pop()).join(' · ')}</div>
                {done.fields.length > 0 && (
                  <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-[14px]">
                    {done.fields.map(([k, v]) => (
                      <div key={k} className="contents">
                        <dt className="text-dim">{k}</dt>
                        <dd className="min-w-0 truncate font-medium">{v}</dd>
                      </div>
                    ))}
                  </dl>
                )}
                {done.text ? <pre className={cx('max-h-48 overflow-auto whitespace-pre-wrap rounded-xl bg-surface-2 p-3 text-[12.5px] text-dim')}>{done.text}</pre> : <p className="text-[13px] text-faint">No text was read (on Windows, text recognition needs a language with OCR installed).</p>}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
