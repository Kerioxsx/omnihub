// Timers: the sleep timer, and lights on (fading in), off (fading out), to a
// preset or a brightness at a time of day on chosen days.

import type { GoveeTimer, TimerAction } from '@shared/types';
import { AlarmClock, Moon, Plus, Trash2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Select, Switch, TextInput } from '../../components/ui/Form';
import { cx } from '../../lib/cx';
import { toast } from '../../state/toasts';
import { Slider } from '../rig/LightingTab';
import type { Govee } from './useGovee';

const DAYS = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
const ACTION: Record<TimerAction, string> = { on: 'Lights on', off: 'Lights off', preset: 'A preset', brightness: 'A brightness' };

function days(d: number[]): string {
  if (d.length === 0 || d.length === 7) return 'Every day';
  if (d.join() === '1,2,3,4,5') return 'Weekdays';
  if (d.join() === '6,7') return 'Weekends';
  return d.map((x) => DAYS[x - 1]).join(', ');
}

function left(until: number): string {
  const s = Math.max(0, Math.round((until - Date.now()) / 1000));
  const m = Math.floor(s / 60);
  return m >= 60 ? `${Math.floor(m / 60)} h ${m % 60} min` : m > 0 ? `${m} min` : `${s} s`;
}

const blank = (): GoveeTimer => ({ id: '', name: '', enabled: true, at: '07:00', days: [1, 2, 3, 4, 5], action: 'on', preset: '', brightness: 100, lights: [], fade: 10 });

export function TimersTab({ govee }: { govee: Govee }) {
  const s = govee.settings;
  const st = govee.status;
  const [draft, setDraft] = useState<GoveeTimer | null>(null);
  const [, tick] = useState(0);
  useEffect(() => {
    if (!st?.sleepUntil) return;
    const t = setInterval(() => tick((n) => n + 1), 1000);
    return () => clearInterval(t);
  }, [st?.sleepUntil]);
  if (!s) return null;

  const sleep = async (m: number | null) => {
    try {
      govee.setStatus(await api.govee.sleep(m));
      if (m) toast.info(`Lights off in ${m} minutes`, 'They fade out over the last minute.');
    } catch (e) {
      toast.error('Could not set the sleep timer', errorText(e));
    }
  };
  const save = async (t: GoveeTimer) => {
    try {
      await api.govee.saveTimer(t);
      setDraft(null);
      toast.saved();
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  };

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader icon={Moon} title="Sleep timer" subtitle={st?.sleepUntil ? `Lights off in ${left(st.sleepUntil)}, fading out over the last minute.` : 'The lights fade out and switch off after a while.'} className="px-5 pt-4" />
        <div className="flex flex-wrap gap-2 px-5 pb-4 pt-3">
          {[15, 30, 45, 60, 90].map((m) => (
            <Button key={m} size="sm" variant="secondary" onClick={() => void sleep(m)}>
              {m} min
            </Button>
          ))}
          {st?.sleepUntil && (
            <Button size="sm" variant="ghost" onClick={() => void sleep(null)}>
              Cancel
            </Button>
          )}
        </div>
      </Card>

      <Card>
        <CardHeader
          icon={AlarmClock}
          title="Timers"
          subtitle="They run while OmniHub runs (it stays in the tray), on this PC’s clock."
          className="px-5 pt-4"
          actions={
            !draft && (
              <Button size="sm" variant="secondary" icon={Plus} onClick={() => setDraft(blank())}>
                Add a timer
              </Button>
            )
          }
        />
        <div className="mt-3 divide-y divide-line border-t border-line">
          {s.timers.length === 0 && !draft && <div className="px-5 py-4 text-[12.5px] text-faint">No timers yet. Wake up to a sunrise, or have the lights fade out at bedtime.</div>}
          {s.timers.map((t) => (
            <div key={t.id} className="flex items-center gap-4 px-5 py-3">
              <div className="w-16 font-mono text-[18px] font-semibold tabular-nums text-fg">{t.at}</div>
              <div className="min-w-0 flex-1">
                <div className="text-[13.5px] font-medium text-fg">{t.name || ACTION[t.action]}</div>
                <div className="text-[12px] text-faint">
                  {days(t.days)} · {t.action === 'preset' ? (govee.presets.find((p) => p.id === t.preset)?.name ?? 'Preset') : t.action === 'brightness' ? `Brightness ${t.brightness}%` : ACTION[t.action]}
                  {t.fade > 0 && (t.action === 'on' || t.action === 'off') ? ` · fades over ${t.fade} min` : ''}
                  {t.lights.length ? ` · ${t.lights.length} light${t.lights.length === 1 ? '' : 's'}` : ''}
                </div>
              </div>
              <Button size="sm" variant="ghost" onClick={() => setDraft(t)}>
                Edit
              </Button>
              <Switch checked={t.enabled} onChange={(v) => void save({ ...t, enabled: v })} label={`${t.name || t.at} on`} />
              <IconButton icon={Trash2} label="Remove" onClick={() => void api.govee.deleteTimer(t.id).catch((e: unknown) => toast.error('Not removed', errorText(e)))} />
            </div>
          ))}
        </div>
        {draft && (
          <div className="space-y-3.5 border-t border-line bg-surface-2/30 px-5 py-4">
            <div className="flex flex-wrap items-end gap-3">
              <label className="space-y-1 text-[12px] text-faint">
                Time
                <TextInput type="time" value={draft.at} onChange={(e) => setDraft({ ...draft, at: e.target.value })} className="w-32" />
              </label>
              <label className="flex-1 space-y-1 text-[12px] text-faint">
                Name
                <TextInput value={draft.name} placeholder="Wake up" onChange={(e) => setDraft({ ...draft, name: e.target.value })} />
              </label>
            </div>
            <div className="flex flex-wrap gap-1.5" role="group" aria-label="Days">
              {DAYS.map((d, i) => {
                const on = draft.days.includes(i + 1);
                return (
                  <button key={d} type="button" aria-pressed={on} onClick={() => setDraft({ ...draft, days: on ? draft.days.filter((x) => x !== i + 1) : [...draft.days, i + 1].sort() })} className={cx('h-8 w-12 rounded-lg border text-[12.5px]', on ? 'border-accent/60 bg-accent-soft text-fg' : 'border-line text-dim')}>
                    {d}
                  </button>
                );
              })}
              <span className="self-center pl-1 text-[11.5px] text-faint">{days(draft.days)}</span>
            </div>
            <div className="flex flex-wrap items-center gap-3">
              <Select value={draft.action} onChange={(e) => setDraft({ ...draft, action: e.target.value as TimerAction })} className="w-44">
                {(Object.keys(ACTION) as TimerAction[]).map((a) => (
                  <option key={a} value={a}>
                    {ACTION[a]}
                  </option>
                ))}
              </Select>
              {draft.action === 'preset' && (
                <Select value={draft.preset} onChange={(e) => setDraft({ ...draft, preset: e.target.value })} className="w-48">
                  <option value="">Choose a preset…</option>
                  {govee.presets.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.name}
                    </option>
                  ))}
                </Select>
              )}
              {(draft.action === 'on' || draft.action === 'off') && (
                <Select value={draft.fade} onChange={(e) => setDraft({ ...draft, fade: Number(e.target.value) })} className="w-44">
                  {[0, 1, 5, 10, 15, 30].map((m) => (
                    <option key={m} value={m}>
                      {m === 0 ? 'At once' : `${draft.action === 'on' ? 'Fade in' : 'Fade out'} over ${m} min`}
                    </option>
                  ))}
                </Select>
              )}
              <Select value={draft.lights.length ? 'some' : 'all'} onChange={(e) => setDraft({ ...draft, lights: e.target.value === 'all' ? [] : (st?.lights.slice(0, 1).map((l) => l.id) ?? []) })} className="w-40">
                <option value="all">All lights</option>
                <option value="some">Some lights</option>
              </Select>
            </div>
            {draft.action === 'brightness' && <Slider label="Brightness" value={draft.brightness} min={1} max={100} step={1} onChange={(v) => setDraft({ ...draft, brightness: v })} format={(v) => `${v}%`} />}
            {draft.lights.length > 0 && (
              <div className="flex flex-wrap gap-1.5">
                {st?.lights.map((l) => {
                  const on = draft.lights.includes(l.id);
                  return (
                    <button key={l.id} type="button" aria-pressed={on} onClick={() => setDraft({ ...draft, lights: on ? draft.lights.filter((x) => x !== l.id) : [...draft.lights, l.id] })} className={cx('rounded-full border px-3 py-1 text-[12.5px]', on ? 'border-accent/60 bg-accent-soft text-fg' : 'border-line text-dim')}>
                      {l.name}
                    </button>
                  );
                })}
              </div>
            )}
            <div className="flex gap-2">
              <Button size="sm" variant="primary" disabled={draft.action === 'preset' && !draft.preset} onClick={() => void save(draft)}>
                Save timer
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setDraft(null)}>
                Cancel
              </Button>
            </div>
          </div>
        )}
      </Card>
    </div>
  );
}
