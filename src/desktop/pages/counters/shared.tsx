// What the Counters page and the pop-out share: odds, time, pictures and
// the key picker.

import type { Counter, Hunt, KeyBind } from '@shared/types';
import { Keyboard, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api } from '../../api';
import { cx } from '../../lib/cx';
import { claimGamepad } from '../../lib/padNav';
import { spriteUrl } from './pokemon';

/** The chance of a shiny per encounter (as the app's core works it out). */
export function shinyChance(h: Hunt): number {
  if (h.customOdds && h.customOdds > 0) return 1 / h.customOdds;
  const base = h.generation <= 5 ? 8192 : 4096;
  let rolls = 1;
  if (h.masuda && h.generation >= 4) rolls += h.generation === 4 ? 4 : 5;
  if (h.charm && h.generation >= 5) rolls += 2;
  return 1 - Math.pow(1 - 1 / base, rolls);
}

export const oddsLabel = (p: number) => `1/${Math.round(1 / p).toLocaleString()}`;

/** How likely a shiny would have shown up by now. */
export const foundByNow = (p: number, n: number) => 1 - Math.pow(1 - p, Math.max(0, n));

export function elapsedMs(c: Counter, now = Date.now()): number {
  return c.timerMs + (c.timerSince ? Math.max(0, now - c.timerSince) : 0);
}

export function formatTimer(ms: number): string {
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  return h ? `${h}:${String(m).padStart(2, '0')}:${String(sec).padStart(2, '0')}` : `${m}:${String(sec).padStart(2, '0')}`;
}

/** A counter's own picture (data URL), loaded once per picture. */
export function useCounterImage(c: Counter): string | null {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    if (!c.image) {
      setUrl(null);
      return;
    }
    void api.counters.image(c.id).then((u) => alive && setUrl(u), () => alive && setUrl(null));
    return () => {
      alive = false;
    };
  }, [c.id, c.image]);
  return url;
}

/** The counter's picture, else the hunted Pokémon in its shiny colours. */
export function CounterPicture({ counter, className }: { counter: Counter; className?: string }) {
  const own = useCounterImage(counter);
  const [failed, setFailed] = useState(false);
  const src = own ?? (counter.hunt && counter.hunt.species > 0 && !failed ? spriteUrl(counter.hunt.species, true) : null);
  if (!src) return null;
  return <img src={src} alt="" draggable={false} onError={() => setFailed(true)} className={cx('object-contain', !own && '[image-rendering:pixelated]', className)} />;
}

/** Controller buttons are PAD + n (the order browsers and XInput share). */
export const PAD = 0x1000;
export const PAD_NAMES = ['A', 'B', 'X', 'Y', 'LB', 'RB', 'LT', 'RT', 'Back', 'Start', 'Left stick', 'Right stick', 'D-pad up', 'D-pad down', 'D-pad left', 'D-pad right'];

/** Pick a key or controller button: click, then press it. */
export function KeyPicker({ value, onChange, label }: { value: KeyBind | null; onChange: (k: KeyBind | null) => void; label: string }) {
  const [listening, setListening] = useState(false);
  useEffect(() => {
    if (!listening) return;
    // Controller buttons too (standard gamepad order, as the PC reads them);
    // controller mode leaves the gamepad to this picker meanwhile.
    const release = claimGamepad();
    let frame = 0;
    const before = new Set<string>();
    for (const pad of navigator.getGamepads?.() ?? []) pad?.buttons.forEach((b, i) => b.pressed && before.add(`${pad.index}:${i}`));
    const poll = () => {
      for (const pad of navigator.getGamepads?.() ?? []) {
        const i = pad?.buttons.findIndex((b, n) => n < 16 && b.pressed && !before.has(`${pad.index}:${n}`)) ?? -1;
        if (pad && i >= 0) {
          setListening(false);
          onChange({ vk: PAD + i, label: `Pad ${PAD_NAMES[i]}` });
          return;
        }
      }
      frame = requestAnimationFrame(poll);
    };
    frame = requestAnimationFrame(poll);
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      setListening(false);
      if (e.key === 'Escape') return;
      const vk = e.keyCode;
      void api.counters.keyName(vk).then((name) => onChange({ vk, label: name }));
    };
    window.addEventListener('keydown', onKey, true);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener('keydown', onKey, true);
      release();
    };
  }, [listening, onChange]);
  return (
    <div className="flex items-center gap-1.5">
      <button
        type="button"
        onClick={() => setListening(true)}
        aria-label={`${label}: ${value?.label ?? 'none'}. Click, then press a key.`}
        className={cx('inline-flex h-9 min-w-[120px] items-center gap-2 rounded-lg border px-3 text-[13px] transition-colors', listening ? 'border-accent bg-accent-soft text-accent' : 'border-line bg-surface hover:border-line-strong')}
      >
        <Keyboard size={14} aria-hidden />
        {listening ? 'Press a key or button…' : (value?.label ?? 'No key')}
      </button>
      {value && !listening && (
        <button type="button" onClick={() => onChange(null)} aria-label={`Remove the ${label.toLowerCase()}`} className="grid h-8 w-8 place-items-center rounded-lg text-faint hover:bg-surface-2 hover:text-fg">
          <X size={14} />
        </button>
      )}
    </div>
  );
}
