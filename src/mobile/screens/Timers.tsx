// Timers and alarms on the PC, from the phone: they ring on both.

import { AlarmClock, Plus, Timer, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { client, type PhoneTimers } from '../client';
import { useEvent } from '../lib/events';
import { errorMessage, vibrate } from '../lib/util';
import { toast } from '../state';
import { SubHeader } from './More';

const left = (ms: number) => {
  const s = Math.max(0, Math.round(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
};

export function TimersPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const [st, setSt] = useState<PhoneTimers | null>(null);
  const [, tick] = useState(0);
  const load = useCallback(() => client.timers().then(setSt, (e) => toast.error(errorMessage(e))), []);
  useEffect(() => {
    if (active) void load();
  }, [active, load]);
  useEffect(() => {
    if (!active) return;
    const id = window.setInterval(() => tick((n) => n + 1), 1000);
    return () => window.clearInterval(id);
  }, [active]);
  useEvent('timer:done', () => void load());
  const run = (p: Promise<PhoneTimers>) => p.then(setSt, (e) => toast.error(errorMessage(e)));
  return (
    <div className="flex min-h-full flex-col pb-6">
      <SubHeader title="Timers" onBack={onBack} />
      <div className="space-y-3 px-4">
        <div className="card p-4">
          <div className="mb-3 flex items-center gap-2 text-[15px] font-semibold">
            <Timer size={18} className="text-accent" /> Start a timer
          </div>
          <div className="grid grid-cols-4 gap-2">
            {[1, 5, 10, 15, 20, 30, 45, 60].map((m) => (
              <button
                key={m}
                type="button"
                onClick={() => {
                  vibrate(15);
                  void run(client.startTimer(m));
                }}
                className="press rounded-xl border border-line bg-surface py-3 text-[15px] font-semibold"
              >
                {m < 60 ? `${m}m` : '1h'}
              </button>
            ))}
          </div>
        </div>
        {st?.timers.map((t) => (
          <div key={t.id} className="card flex items-center gap-3 p-4">
            <div className="min-w-0 flex-1">
              <div className="text-[13px] text-dim">{t.label}</div>
              <div className="font-display text-[30px] font-bold tabular-nums leading-tight">{left(t.endsAt - Date.now())}</div>
            </div>
            <button type="button" aria-label="Cancel" onClick={() => void run(client.cancelTimer(t.id))} className="press grid h-10 w-10 place-items-center rounded-full bg-surface-3">
              <X size={18} />
            </button>
          </div>
        ))}
        <div className="card p-4">
          <div className="mb-2 flex items-center justify-between">
            <span className="flex items-center gap-2 text-[15px] font-semibold">
              <AlarmClock size={18} className="text-accent" /> Alarms
            </span>
            <button type="button" aria-label="Add alarm" onClick={() => void run(client.saveAlarm({ id: '', label: '', time: '07:00', days: [], enabled: true, fired: '' }))} className="press grid h-9 w-9 place-items-center rounded-full bg-accent text-white">
              <Plus size={18} />
            </button>
          </div>
          {st?.alarms.length === 0 && <div className="text-[13px] text-faint">No alarms.</div>}
          {st?.alarms.map((a) => (
            <div key={a.id} className="flex items-center gap-3 border-t border-line py-2.5 first:border-t-0">
              <input type="time" value={a.time} onChange={(e) => void run(client.saveAlarm({ ...a, time: e.target.value }))} className="field w-[120px]" aria-label="Time" />
              <span className="min-w-0 flex-1 truncate text-[14px] text-dim">{a.label || (a.days.length ? 'Some days' : 'Every day')}</span>
              <button type="button" onClick={() => void run(client.saveAlarm({ ...a, enabled: !a.enabled }))} className={`press rounded-full px-3 py-1.5 text-[13px] font-semibold ${a.enabled ? 'bg-accent text-white' : 'bg-surface-3 text-dim'}`}>
                {a.enabled ? 'On' : 'Off'}
              </button>
              <button type="button" aria-label="Delete alarm" onClick={() => void run(client.deleteAlarm(a.id))} className="press text-faint">
                <X size={16} />
              </button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
