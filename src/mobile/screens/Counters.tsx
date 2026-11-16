// Tap the phone to count: the PC's counters (shiny hunts included), with
// their events and timer. Changes show on the PC (and its pop-outs) at once.

import { Minus, Pause, Play, Plus, Sparkles } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { client, type PhoneCounter } from '../client';
import { errorMessage, vibrate } from '../lib/util';
import { toast } from '../state';
import { Empty } from '../ui/common';
import { SubHeader } from './More';

const fmt = (ms: number) => {
  const s = Math.floor(ms / 1000);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return `${h ? `${h}:${String(m).padStart(2, '0')}` : m}:${String(s % 60).padStart(2, '0')}`;
};

export function CountersPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const [list, setList] = useState<PhoneCounter[] | null>(null);
  const [pick, setPick] = useState<string | null>(null);
  const [, tick] = useState(0);
  const load = useCallback(() => {
    client.counters().then(
      (r) => setList(r.counters),
      (e) => toast.error(errorMessage(e)),
    );
  }, []);
  useEffect(() => {
    if (!active) return;
    load();
    const t = setInterval(load, 3000);
    const c = setInterval(() => tick((n) => n + 1), 1000);
    return () => {
      clearInterval(t);
      clearInterval(c);
    };
  }, [active, load]);
  const c = list?.find((x) => x.id === pick) ?? list?.[0] ?? null;
  const act = async (p: Promise<unknown>) => {
    vibrate(15);
    try {
      await p;
      load();
    } catch (e) {
      toast.error(errorMessage(e));
    }
  };
  return (
    <div className="flex min-h-full flex-col">
      <SubHeader title="Counters" onBack={onBack} />
      {list && list.length === 0 && <Empty icon={<Sparkles size={28} />} title="No counters yet" body="Make one on the PC (Counters page), then count from here." />}
      {list && list.length > 1 && (
        <div className="flex gap-2 overflow-x-auto px-4 pb-2">
          {list.map((x) => (
            <button key={x.id} type="button" onClick={() => setPick(x.id)} className={`shrink-0 rounded-full border px-3 py-1.5 text-[13px] ${x.id === c?.id ? 'border-accent bg-accent/15 text-fg' : 'border-white/10 text-dim'}`}>
              {x.name}
            </button>
          ))}
        </div>
      )}
      {c && (
        <div className="flex flex-1 flex-col items-center gap-3 px-4 pb-6">
          {c.sprite && <img src={c.sprite} alt="" className="h-28 w-28 [image-rendering:pixelated]" />}
          <div className="text-center text-[15px] font-semibold text-dim">{c.name}</div>
          {c.odds && (
            <div className="flex items-center gap-1.5 text-[12.5px] text-faint">
              <Sparkles size={13} /> {c.odds} · {((c.chance ?? 0) * 100).toFixed(1)}% by now
            </div>
          )}
          {c.phases > 0 && <div className="text-[12.5px] text-faint">Phase {c.phases + 1}: {c.phaseCount.toLocaleString()}</div>}
          <button type="button" onClick={() => void act(client.bumpCounter(c.id, 1))} aria-label="Add one" className="grid aspect-square w-full max-w-[320px] place-items-center rounded-[36px] bg-accent text-white shadow-[0_20px_60px_-20px_var(--accent)] active:scale-[0.98]">
            <span className="text-[64px] font-extrabold tabular-nums leading-none">{c.count.toLocaleString()}</span>
          </button>
          <div className="flex w-full max-w-[320px] gap-2">
            <button type="button" onClick={() => void act(client.bumpCounter(c.id, -1))} aria-label="Take one away" className="grid h-14 flex-1 place-items-center rounded-2xl bg-white/8">
              <Minus size={22} />
            </button>
            <button type="button" onClick={() => void act(client.counterTimer(c.id, !c.running))} className="flex h-14 flex-1 items-center justify-center gap-2 rounded-2xl bg-white/8 font-mono tabular-nums">
              {c.running ? <Pause size={18} /> : <Play size={18} />} {fmt(c.timerMs)}
            </button>
            <button type="button" onClick={() => void act(client.bumpCounter(c.id, 1))} aria-label="Add one" className="grid h-14 flex-1 place-items-center rounded-2xl bg-white/8">
              <Plus size={22} />
            </button>
          </div>
          {c.events.length > 0 && (
            <div className="flex w-full max-w-[320px] flex-wrap gap-2">
              {c.events.map((e) => (
                <button key={e.id} type="button" onClick={() => void act(client.counterEvent(c.id, e.id))} className="rounded-xl border border-white/10 px-3 py-2 text-[13px]">
                  {e.name}
                </button>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
