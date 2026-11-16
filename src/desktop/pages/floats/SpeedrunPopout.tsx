// The speedrun timer window, above the game: your splits with the time
// against your personal best (green ahead, red behind, gold for a best
// segment), the running time, and buttons for when the keys aren't handy.

import type { SpeedrunState } from '@shared/types';
import { Pause, Play, RotateCcw, Undo2, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api } from '../../api';
import { useEvent } from '../../lib/hooks';
import { useFrame } from '../../lib/nowPlaying';
import { cx } from '../../lib/cx';

/** "1:02.34", or "−0:01.2" for a difference. */
export function runTime(ms: number, tenths = false): string {
  const neg = ms < 0;
  const a = Math.abs(ms);
  const h = Math.floor(a / 3_600_000);
  const m = Math.floor((a % 3_600_000) / 60_000);
  const s = Math.floor((a % 60_000) / 1000);
  const f = tenths ? `.${Math.floor((a % 1000) / 100)}` : `.${String(Math.floor((a % 1000) / 10)).padStart(2, '0')}`;
  const body = h ? `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}` : `${m}:${String(s).padStart(2, '0')}`;
  return `${neg ? '−' : ''}${body}${f}`;
}

export function SpeedrunPopout() {
  const [st, setSt] = useState<SpeedrunState | null>(null);
  const [offset, setOffset] = useState(0);
  useEffect(() => {
    void api.speedrun.state().then((s) => {
      setOffset(s.now - Date.now());
      setSt(s);
    });
  }, []);
  useEvent<SpeedrunState>('speedrun:changed', (s) => {
    setOffset(s.now - Date.now());
    setSt(s);
  });
  useFrame(!!st?.run.since, 30);
  if (!st) return null;
  const { settings: set, run } = st;
  const now = Date.now() + offset;
  const elapsed = run.banked + (run.since ? now - run.since : 0);
  const cur = run.times.length;
  const press = (what: 'split' | 'reset' | 'pause' | 'undo') => void api.speedrun.press(what).then(setSt);
  let prev = 0;
  return (
    <div data-tauri-drag-region className="m-1 flex h-[calc(100vh-8px)] flex-col rounded-2xl border border-white/10 bg-black/80 text-white shadow-xl">
      <div data-tauri-drag-region className="flex items-start justify-between px-3.5 pt-3">
        <div data-tauri-drag-region className="min-w-0">
          <div className="truncate text-[14px] font-semibold">{set.game || 'Speedrun'}</div>
          <div className="truncate text-[11.5px] text-white/60">
            {set.category} · attempt {set.attempts}
          </div>
        </div>
        <button type="button" aria-label="Close" onClick={() => void api.speedrun.window(false)} className="rounded-md p-1 text-white/50 hover:bg-white/10 hover:text-white">
          <X size={14} />
        </button>
      </div>
      <div className="mt-2 min-h-0 flex-1 overflow-y-auto px-2">
        {set.splits.map((sp, i) => {
          const t = run.times[i];
          const seg = t != null ? t - prev : null;
          const delta = t != null && sp.pb != null ? t - sp.pb : i === cur && sp.pb != null && elapsed > sp.pb ? elapsed - sp.pb : null;
          const gold = seg != null && sp.gold != null && seg <= sp.gold;
          if (t != null) prev = t;
          return (
            <div key={i} className={cx('flex items-center gap-2 rounded-lg px-1.5 py-1 text-[13px]', i === cur && !run.finished && 'bg-white/10')}>
              <span className="min-w-0 flex-1 truncate">{sp.name}</span>
              <span className={cx('w-[64px] text-right tabular-nums text-[12px]', delta == null ? '' : gold ? 'text-amber-300' : delta < 0 ? 'text-emerald-400' : 'text-red-400')}>{delta != null ? `${delta > 0 ? '+' : ''}${runTime(delta, true)}` : ''}</span>
              <span className="w-[68px] text-right tabular-nums text-white/80">{t != null ? runTime(t) : sp.pb != null ? runTime(sp.pb) : '—'}</span>
            </div>
          );
        })}
      </div>
      <div className={cx('px-3.5 py-1 text-right font-display text-[38px] font-bold tabular-nums leading-tight', run.finished ? (run.newPb ? 'text-amber-300' : 'text-white') : run.since ? 'text-emerald-400' : 'text-white/80')}>{runTime(elapsed)}</div>
      {run.newPb && <div className="px-3.5 text-right text-[12px] font-medium text-amber-300">New personal best!</div>}
      <div className="flex items-center gap-1 border-t border-white/10 px-2 py-2">
        <button type="button" onClick={() => press('split')} className="flex-1 rounded-lg bg-white/10 py-1.5 text-[12.5px] font-medium hover:bg-white/20">
          {cur === 0 && !run.since && !run.banked ? 'Start' : 'Split'}
        </button>
        <button type="button" aria-label={run.since ? 'Pause' : 'Resume'} onClick={() => press('pause')} className="rounded-lg p-2 hover:bg-white/10">
          {run.since ? <Pause size={14} /> : <Play size={14} />}
        </button>
        <button type="button" aria-label="Undo split" onClick={() => press('undo')} className="rounded-lg p-2 hover:bg-white/10">
          <Undo2 size={14} />
        </button>
        <button type="button" aria-label="Reset" onClick={() => press('reset')} className="rounded-lg p-2 hover:bg-white/10">
          <RotateCcw size={14} />
        </button>
      </div>
    </div>
  );
}
