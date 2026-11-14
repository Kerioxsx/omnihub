// Timers, alarms and bedtime. Timers and alarms ring here and on your phone;
// bedtime warns you, then locks the PC until the morning.

import type { Alarm, TimersState } from '@shared/types';
import { AlarmClock, BedDouble, Plus, Timer, Trash2, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Switch, TextInput } from '../../components/ui/Form';
import { cx } from '../../lib/cx';
import { useEvent, useInterval } from '../../lib/hooks';
import { toast } from '../../state/toasts';

const DAYS = ['M', 'T', 'W', 'T', 'F', 'S', 'S'];
const left = (ms: number) => {
  const s = Math.max(0, Math.round(ms / 1000));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return `${h ? `${h}:` : ''}${String(m).padStart(h ? 2 : 1, '0')}:${String(s % 60).padStart(2, '0')}`;
};

function Days({ value, onChange }: { value: number[]; onChange: (d: number[]) => void }) {
  return (
    <div className="flex gap-1" role="group" aria-label="Days">
      {DAYS.map((d, i) => {
        const on = value.length === 0 || value.includes(i);
        return (
          <button
            key={i}
            type="button"
            aria-pressed={on}
            onClick={() => {
              const all = value.length === 0 ? [0, 1, 2, 3, 4, 5, 6] : value;
              const next = on ? all.filter((x) => x !== i) : [...all, i].sort();
              onChange(next.length === 7 ? [] : next);
            }}
            className={cx('h-7 w-7 rounded-full text-[11.5px] font-semibold', on ? 'bg-accent text-white' : 'bg-surface-3 text-dim')}
          >
            {d}
          </button>
        );
      })}
    </div>
  );
}

export function TimersTab() {
  const [st, setSt] = useState<TimersState | null>(null);
  const [minutes, setMinutes] = useState(10);
  const [label, setLabel] = useState('');
  const [, tick] = useState(0);
  const load = () => void api.timers.get().then(setSt, () => undefined);
  useEffect(load, []);
  useInterval(() => tick((n) => n + 1), 1000);
  useEvent('timer:done', load);
  useEvent('timer:alarm', load);
  if (!st) return null;
  const run = async (p: Promise<TimersState>) => {
    try {
      setSt(await p);
    } catch (e) {
      toast.error('That didn’t work', errorText(e));
    }
  };
  const alarm = (a: Alarm) => void run(api.timers.saveAlarm(a));
  const b = st.bedtime;
  return (
    <div className="space-y-4">
      <Card className="p-5">
        <CardHeader icon={Timer} title="Timers" subtitle="They ring here and on your phone." />
        <div className="mt-4 flex flex-wrap items-center gap-2">
          {[5, 10, 15, 30, 60].map((m) => (
            <Button key={m} size="sm" variant="ghost" onClick={() => void run(api.timers.start(m, ''))}>
              {m < 60 ? `${m} min` : '1 h'}
            </Button>
          ))}
          <TextInput type="number" min={1} max={1440} value={minutes} onChange={(e) => setMinutes(Number(e.target.value))} className="w-20" aria-label="Minutes" />
          <TextInput value={label} maxLength={40} onChange={(e) => setLabel(e.target.value)} placeholder="Name (pizza, break…)" className="min-w-[180px] flex-1" aria-label="Timer name" />
          <Button
            variant="primary"
            icon={Plus}
            onClick={() => {
              void run(api.timers.start(minutes, label));
              setLabel('');
            }}
          >
            Start
          </Button>
        </div>
        {st.timers.length > 0 && (
          <div className="mt-3 grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
            {st.timers.map((t) => (
              <div key={t.id} className="flex items-center gap-3 rounded-xl border border-line bg-surface-2/50 px-3.5 py-2.5">
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[12.5px] text-dim">{t.label}</div>
                  <div className="font-display text-[24px] font-bold tabular-nums leading-tight">{left(t.endsAt - Date.now())}</div>
                </div>
                <IconButton icon={X} label="Cancel timer" onClick={() => void run(api.timers.cancel(t.id))} />
              </div>
            ))}
          </div>
        )}
      </Card>

      <Card className="p-5">
        <CardHeader icon={AlarmClock} title="Alarms" subtitle="Ring at a time on the days you pick, here and on your phone." actions={<Button size="sm" icon={Plus} onClick={() => alarm({ id: '', label: '', time: '07:00', days: [0, 1, 2, 3, 4], enabled: true, fired: '' })}>Add alarm</Button>} />
        <div className="mt-3 space-y-2">
          {st.alarms.length === 0 && <p className="text-[13px] text-faint">No alarms yet.</p>}
          {st.alarms.map((a) => (
            <div key={a.id} className="flex flex-wrap items-center gap-3 rounded-xl border border-line px-3.5 py-2.5">
              <TextInput type="time" value={a.time} onChange={(e) => alarm({ ...a, time: e.target.value })} className="w-[136px]" aria-label="Time" />
              <TextInput value={a.label} maxLength={40} onChange={(e) => setSt({ ...st, alarms: st.alarms.map((x) => (x.id === a.id ? { ...x, label: e.target.value } : x)) })} onBlur={(e) => alarm({ ...a, label: e.target.value })} placeholder="Name" className="min-w-[140px] flex-1" aria-label="Alarm name" />
              <Days value={a.days} onChange={(days) => alarm({ ...a, days })} />
              <Switch checked={a.enabled} onChange={(v) => alarm({ ...a, enabled: v })} label="Alarm on" />
              <IconButton icon={Trash2} label="Delete alarm" onClick={() => void run(api.timers.deleteAlarm(a.id))} />
            </div>
          ))}
        </div>
      </Card>

      <Card className="p-5">
        <CardHeader
          icon={BedDouble}
          title="Bedtime"
          subtitle="A warning a few minutes before, then the PC locks, and locks again if it’s used before the morning."
          actions={<Switch checked={b.enabled} onChange={(v) => void run(api.timers.saveBedtime({ ...b, enabled: v }))} label="Bedtime on" />}
        />
        <div className="mt-4 flex flex-wrap items-center gap-3 text-[13px] text-dim">
          From
          <TextInput type="time" value={b.from} onChange={(e) => void run(api.timers.saveBedtime({ ...b, from: e.target.value }))} className="w-[136px]" aria-label="From" />
          to
          <TextInput type="time" value={b.to} onChange={(e) => void run(api.timers.saveBedtime({ ...b, to: e.target.value }))} className="w-[136px]" aria-label="To" />
          warn
          <TextInput type="number" min={0} max={60} value={b.warnMinutes} onChange={(e) => void run(api.timers.saveBedtime({ ...b, warnMinutes: Number(e.target.value) }))} className="w-20" aria-label="Warning minutes" />
          min before
          <Days value={b.days} onChange={(days) => void run(api.timers.saveBedtime({ ...b, days }))} />
        </div>
        {b.enabled && (
          <div className="mt-3 flex items-center gap-2">
            <Button size="sm" variant="ghost" onClick={() => void run(api.timers.snooze(15))}>
              15 more minutes tonight
            </Button>
            {st.snoozedUntil > Date.now() && <span className="text-[12px] text-faint">Put off until {new Date(st.snoozedUntil).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })}</span>}
          </div>
        )}
      </Card>
    </div>
  );
}
