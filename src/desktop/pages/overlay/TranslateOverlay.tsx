// The screen translator, on a frozen copy of the screen (the same overlay
// window as region screenshots): drag a box over text, Windows' OCR reads
// it and the AI model on this PC (Ollama) translates it. Esc closes.

import type { PendingRegion, Translation } from '@shared/types';
import { Copy, Languages, X } from 'lucide-react';
import { type MouseEvent, useCallback, useEffect, useRef, useState } from 'react';
import { api, errorText, inTauri } from '../../api';
import { closeCurrentWindow } from '../../lib/util';

export const TRANSLATE_LANGS = [
  'English',
  'Spanish',
  'French',
  'German',
  'Italian',
  'Portuguese',
  'Dutch',
  'Polish',
  'Russian',
  'Ukrainian',
  'Turkish',
  'Arabic',
  'Hindi',
  'Japanese',
  'Korean',
  'Chinese (Simplified)',
  'Chinese (Traditional)',
  'Vietnamese',
  'Thai',
  'Indonesian',
];
const LANG_KEY = 'omnihub.translateTo';

function storedLang(): string {
  try {
    const v = localStorage.getItem(LANG_KEY);
    return v && TRANSLATE_LANGS.includes(v) ? v : 'English';
  } catch {
    return 'English';
  }
}

interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

export function TranslateOverlay() {
  const [pending, setPending] = useState<PendingRegion | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [box, setBox] = useState<Box | null>(null);
  const [to, setTo] = useState(storedLang);
  const [result, setResult] = useState<Translation | null>(null);
  const [busy, setBusy] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);
  const [showOriginal, setShowOriginal] = useState(false);
  const img = useRef<HTMLImageElement>(null);
  const drag = useRef<{ x: number; y: number } | null>(null);
  const ask = useRef(0);

  useEffect(() => {
    void (async () => {
      try {
        let p = await api.shots.regionPending();
        if (!p && !inTauri) {
          await api.shots.toolBegin('translate');
          p = await api.shots.regionPending();
        }
        setPending(p);
        if (!p) setError('No frozen screen to read');
      } catch (e) {
        setError(errorText(e));
      }
    })();
  }, []);

  const close = useCallback(async () => {
    await api.shots.regionCancel().catch(() => undefined);
    void closeCurrentWindow('#/tools?tab=images');
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        void close();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [close]);

  const translate = async (b: Box, lang: string) => {
    if (!pending) return;
    const el = img.current;
    const sx = el?.naturalWidth ? el.naturalWidth / window.innerWidth : pending.width / window.innerWidth;
    const sy = el?.naturalHeight ? el.naturalHeight / window.innerHeight : sx;
    const n = ++ask.current;
    setBusy(true);
    setFailed(null);
    setResult(null);
    setShowOriginal(false);
    try {
      const r = await api.shots.regionTranslate(pending.id, { x: Math.round(b.x * sx), y: Math.round(b.y * sy), width: Math.round(b.w * sx), height: Math.round(b.h * sy) }, lang);
      if (n === ask.current) setResult(r);
    } catch (e) {
      if (n === ask.current) setFailed(errorText(e));
    } finally {
      if (n === ask.current) setBusy(false);
    }
  };

  const onDown = (e: MouseEvent) => {
    if (e.button !== 0) return;
    drag.current = { x: e.clientX, y: e.clientY };
    setBox({ x: e.clientX, y: e.clientY, w: 0, h: 0 });
  };
  const onMove = (e: MouseEvent) => {
    const d = drag.current;
    if (!d) return;
    setBox({ x: Math.min(d.x, e.clientX), y: Math.min(d.y, e.clientY), w: Math.abs(e.clientX - d.x), h: Math.abs(e.clientY - d.y) });
  };
  const onUp = () => {
    drag.current = null;
    if (box && box.w > 8 && box.h > 8) void translate(box, to);
    else setBox(null);
  };

  const pickLang = (lang: string) => {
    setTo(lang);
    try {
      localStorage.setItem(LANG_KEY, lang);
    } catch {
      /* private window */
    }
    if (box && box.w > 8) void translate(box, lang);
  };

  if (error && !pending)
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 bg-black text-white">
        <div className="text-[15px]">Couldn’t freeze the screen</div>
        <div className="font-mono text-[12px] text-white/60">{error}</div>
        <button type="button" onClick={() => void close()} className="rounded-lg bg-white/10 px-4 py-2 text-[13px] hover:bg-white/20">
          Close
        </button>
      </div>
    );

  const cardW = 360;
  const cardLeft = box ? Math.max(12, Math.min(box.x, window.innerWidth - cardW - 12)) : 0;
  const below = box ? box.y + box.h + 220 < window.innerHeight : true;

  return (
    <div
      className="fixed inset-0 select-none overflow-hidden bg-black"
      style={{ cursor: 'crosshair' }}
      onMouseDown={onDown}
      onMouseMove={onMove}
      onMouseUp={onUp}
      onContextMenu={(e) => {
        e.preventDefault();
        void close();
      }}
      role="application"
      aria-label="Drag a box over text to translate it. Escape closes."
    >
      {pending && <img ref={img} src={pending.image} alt="" draggable={false} className="pointer-events-none absolute inset-0 h-full w-full object-fill" />}
      {!box && <div className="pointer-events-none absolute inset-0 bg-black/35" />}
      {box && <div className="pointer-events-none absolute border-2 border-white/90" style={{ left: box.x, top: box.y, width: box.w, height: box.h, boxShadow: '0 0 0 100vmax rgba(0,0,0,0.45)' }} />}
      <div
        className="absolute left-1/2 top-6 flex -translate-x-1/2 items-center gap-2 rounded-full border border-white/15 bg-black/80 py-1.5 pl-4 pr-1.5 text-[13px] text-white shadow-xl backdrop-blur"
        onMouseDown={(e) => e.stopPropagation()}
        onMouseUp={(e) => e.stopPropagation()}
      >
        <Languages size={14} />
        Drag over text to translate into
        <select value={to} onChange={(e) => pickLang(e.target.value)} className="rounded-full border border-white/20 bg-white/10 px-2 py-1 text-[12.5px] text-white outline-none" aria-label="Translate into">
          {TRANSLATE_LANGS.map((l) => (
            <option key={l} value={l} className="text-black">
              {l}
            </option>
          ))}
        </select>
        <button type="button" onClick={() => void close()} className="rounded-full p-1.5 text-white/70 hover:bg-white/10 hover:text-white" aria-label="Close">
          <X size={14} />
        </button>
      </div>
      {box && box.w > 8 && !drag.current && (busy || result || failed) && (
        <div
          className="absolute select-text rounded-2xl border border-white/15 bg-black/90 p-3.5 text-white shadow-2xl backdrop-blur"
          style={{ left: cardLeft, width: cardW, ...(below ? { top: box.y + box.h + 10 } : { top: Math.max(12, box.y - 230) }) }}
          onMouseDown={(e) => e.stopPropagation()}
          onMouseUp={(e) => e.stopPropagation()}
        >
          {busy && <div className="text-[13px] text-white/70">Reading and translating…</div>}
          {failed && <div className="text-[13px] text-red-300">{failed}</div>}
          {result && (
            <>
              <div className="max-h-[160px] overflow-y-auto whitespace-pre-wrap text-[14px] leading-relaxed">{result.translated}</div>
              {showOriginal && <div className="mt-2 max-h-[100px] overflow-y-auto whitespace-pre-wrap border-t border-white/10 pt-2 text-[12px] text-white/55">{result.text}</div>}
              <div className="mt-2.5 flex items-center gap-2 text-[11.5px] text-white/50">
                <span className="truncate">{result.model} · on this PC</span>
                <button type="button" onClick={() => setShowOriginal((v) => !v)} className="ml-auto rounded-md px-2 py-1 hover:bg-white/10 hover:text-white">
                  {showOriginal ? 'Hide original' : 'Original'}
                </button>
                <button type="button" onClick={() => void navigator.clipboard?.writeText(result.translated).catch(() => undefined)} className="flex items-center gap-1 rounded-md px-2 py-1 hover:bg-white/10 hover:text-white">
                  <Copy size={12} /> Copy
                </button>
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
