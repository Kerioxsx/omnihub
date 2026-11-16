// A counter in its own small window above everything (label "counter-…",
// route #/counter?id=…): drag it anywhere, resize it, close it with × or Esc.
// Keys keep counting while it's open; it updates when they do.

import type { Counter, CountersState } from '@shared/types';
import { Minus, Pause, Play, Plus, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, inTauri } from '../../api';
import { useEvent, useNow } from '../../lib/hooks';
import { useRoute } from '../../lib/router';
import { CounterPicture, elapsedMs, foundByNow, formatTimer, oddsLabel, shinyChance } from './shared';

async function closeWindow() {
  if (!inTauri) return window.close();
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  void getCurrentWindow().close();
}

export function CounterPopout() {
  const id = useRoute().params.get('id') ?? '';
  const [counter, setCounter] = useState<Counter | null>(null);
  const now = useNow(counter?.timerSince ? 500 : 60_000);
  useEffect(() => {
    document.documentElement.classList.add('overlay-route', 'popout-route');
    void api.counters.state().then((s) => setCounter(s.counters.find((c) => c.id === id) ?? null));
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && void closeWindow();
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [id]);
  useEvent<CountersState>('counters:changed', (s) => {
    const c = s.counters.find((x) => x.id === id);
    if (c) setCounter(c);
    else void closeWindow();
  });
  if (!counter) return <div className="h-screen w-screen bg-[var(--bg)]" data-tauri-drag-region />;
  const hunt = counter.hunt;
  const p = hunt ? shinyChance(hunt) : 0;
  const bump = (n: number) => void api.counters.bump(counter.id, n);
  return (
    <div className="group relative flex h-screen w-screen select-none flex-col items-center justify-center gap-1 overflow-hidden border border-line-strong bg-[var(--bg)] px-3 pb-3 pt-6 text-fg" data-tauri-drag-region>
      <button type="button" onClick={() => void closeWindow()} aria-label="Close" className="absolute right-1.5 top-1.5 grid h-6 w-6 place-items-center rounded-full text-faint opacity-60 hover:bg-surface-2 hover:text-fg group-hover:opacity-100">
        <X size={14} />
      </button>
      <CounterPicture counter={counter} className="pointer-events-none h-[28vh] max-h-28 w-auto" />
      <div className="pointer-events-none max-w-full truncate text-[13px] font-medium text-dim">{counter.name}</div>
      <div className="pointer-events-none font-display text-[min(18vw,64px)] font-bold tabular-nums leading-none">{counter.count.toLocaleString()}</div>
      {hunt && (
        <div className="pointer-events-none text-[11.5px] text-faint">
          {oddsLabel(p)} · {(foundByNow(p, counter.count) * 100).toFixed(1)}% by now
        </div>
      )}
      <div className="mt-1 flex items-center gap-1.5">
        <button type="button" onClick={() => bump(-1)} aria-label="Take one away" className="grid h-8 w-8 place-items-center rounded-lg bg-surface-2 hover:bg-surface-3">
          <Minus size={15} />
        </button>
        <button type="button" onClick={() => void api.counters.timer(counter.id, !counter.timerSince)} aria-label={counter.timerSince ? 'Pause the timer' : 'Start the timer'} className="flex h-8 items-center gap-1.5 rounded-lg bg-surface-2 px-2.5 font-mono text-[13px] tabular-nums hover:bg-surface-3">
          {counter.timerSince ? <Pause size={13} /> : <Play size={13} />}
          {formatTimer(elapsedMs(counter, now))}
        </button>
        <button type="button" onClick={() => bump(1)} aria-label="Add one" className="accent-gradient grid h-8 w-8 place-items-center rounded-lg text-white">
          <Plus size={15} />
        </button>
      </div>
    </div>
  );
}
