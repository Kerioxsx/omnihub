// Focus: a focus timer that keeps chosen apps closed while it runs, and a
// time tracker (off until you turn it on) of which programs you used.

import type { FocusState, TimeReport } from '@shared/types';
import { Clock, Pause, Play, Plus, Timer, Trash, X } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Segmented, Switch, type TabItem, Tabs, TextInput } from '../../components/ui/Form';
import { EmptyState } from '../../components/ui/States';
import { useEvent, useInterval } from '../../lib/hooks';
import { confirm } from '../../state/dialogs';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';

const hm = (secs: number) => {
  const m = Math.round(secs / 60);
  return m < 60 ? `${m} min` : `${Math.floor(m / 60)} h ${m % 60 ? `${m % 60} min` : ''}`.trim();
};
const PRESETS: { value: string; label: string; f: number; b: number }[] = [
  { value: '25', label: '25 / 5', f: 25, b: 5 },
  { value: '50', label: '50 / 10', f: 50, b: 10 },
  { value: '90', label: '90 / 15', f: 90, b: 15 },
];
const SUGGESTED = ['discord.exe', 'steam.exe', 'epicgameslauncher.exe', 'robloxplayerbeta.exe', 'spotify.exe', 'whatsapp.exe', 'telegram.exe'];

function Ring({ left, total, phase }: { left: number; total: number; phase: FocusState['phase'] }) {
  const r = 88;
  const c = 2 * Math.PI * r;
  const p = total ? Math.max(0, Math.min(1, left / total)) : 0;
  const m = Math.floor(left / 60_000);
  const s = Math.floor((left % 60_000) / 1000);
  return (
    <div className="relative mx-auto grid h-[220px] w-[220px] place-items-center">
      <svg viewBox="0 0 200 200" className="absolute inset-0 -rotate-90">
        <circle cx="100" cy="100" r={r} fill="none" stroke="var(--color-surface-3, #2a2a38)" strokeWidth="10" />
        <circle cx="100" cy="100" r={r} fill="none" stroke={phase === 'focus' ? 'var(--color-accent, #8b5cf6)' : '#22c55e'} strokeWidth="10" strokeLinecap="round" strokeDasharray={c} strokeDashoffset={c * (1 - p)} style={{ transition: 'stroke-dashoffset 1s linear' }} />
      </svg>
      <div className="text-center">
        <div className="font-display text-[46px] font-bold tabular-nums leading-none">
          {m}:{String(s).padStart(2, '0')}
        </div>
        <div className="mt-1 text-[13px] text-dim">{phase === 'focus' ? 'Focus' : 'Break'}</div>
      </div>
    </div>
  );
}

function FocusTab() {
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const [state, setState] = useState<FocusState | null>(null);
  const [preset, setPreset] = useState('25');
  const [app, setApp] = useState('');
  const [now, setNow] = useState(Date.now());
  const load = useCallback(() => void api.focus.state().then(setState, () => undefined), []);
  useEffect(load, [load]);
  useEvent('focus:changed', load);
  useInterval(() => setNow(Date.now()), state ? 1000 : null);
  if (!settings) return null;
  const blocked = settings.focus.blocked;
  const setBlocked = (b: string[]) => void update({ focus: { blocked: b } });
  const addApp = (a: string) => {
    const name = a.trim().toLowerCase().replace(/^.*[\\/]/, '');
    if (!name) return;
    const exe = name.endsWith('.exe') ? name : `${name}.exe`;
    if (!blocked.includes(exe)) setBlocked([...blocked, exe]);
    setApp('');
  };
  const p = PRESETS.find((x) => x.value === preset) ?? PRESETS[0];
  const start = async () => {
    try {
      setState(await api.focus.start(p.f, p.b, blocked));
    } catch (e) {
      toast.error('Couldn’t start', errorText(e));
    }
  };
  const left = state ? Math.max(0, state.endsAt - now) : 0;
  const total = state ? (state.phase === 'focus' ? state.focusMinutes : state.breakMinutes) * 60_000 : 0;

  return (
    <div className="grid gap-4 lg:grid-cols-[1fr_1fr]">
      <Card className="p-6">
        <CardHeader icon={Timer} title="Focus timer" subtitle="Work in rounds with short breaks. Apps you choose are closed if they open while you focus." />
        <div className="mt-6">
          {state ? (
            <>
              <Ring left={left} total={total} phase={state.phase} />
              <div className="mt-4 text-center text-[13px] text-dim">
                Round {state.rounds + (state.phase === 'focus' ? 1 : 0)} · {state.focusMinutes} / {state.breakMinutes}
                {state.closed.length > 0 && ` · closed ${state.closed.join(', ')}`}
              </div>
              <div className="mt-4 flex justify-center">
                <Button variant="danger" icon={Pause} onClick={() => void api.focus.stop().then(() => setState(null))}>
                  Stop
                </Button>
              </div>
            </>
          ) : (
            <div className="flex flex-col items-center gap-4 py-6">
              <Segmented label="Length" value={preset} onChange={setPreset} options={PRESETS.map((x) => ({ value: x.value, label: x.label }))} />
              <Button variant="primary" size="lg" icon={Play} onClick={() => void start()}>
                Start focusing
              </Button>
              <p className="max-w-sm text-center text-[12.5px] text-faint">OmniHub’s own notifications wait until a break. Windows’ Focus assist (Do not disturb) is in the quick settings.</p>
            </div>
          )}
        </div>
      </Card>
      <Card className="p-6">
        <CardHeader icon={X} title="Keep away while focusing" subtitle="These are closed (like clicking ×) when they open during focus; breaks are free." />
        <div className="mt-4 flex flex-wrap gap-2">
          {blocked.length === 0 && <span className="text-[13px] text-faint">Nothing yet.</span>}
          {blocked.map((b) => (
            <span key={b} className="flex items-center gap-1 rounded-full bg-surface-2 py-1 pl-3 pr-1 text-[12.5px]">
              {b}
              <IconButton size="sm" icon={X} label={`Remove ${b}`} onClick={() => setBlocked(blocked.filter((x) => x !== b))} />
            </span>
          ))}
        </div>
        <div className="mt-4 flex gap-2">
          <TextInput value={app} onChange={(e) => setApp(e.target.value)} placeholder="discord.exe" onKeyDown={(e) => e.key === 'Enter' && addApp(app)} className="w-60" />
          <Button icon={Plus} onClick={() => addApp(app)} disabled={!app.trim()}>
            Add
          </Button>
        </div>
        <div className="mt-3 flex flex-wrap gap-1.5">
          {SUGGESTED.filter((s) => !blocked.includes(s)).map((s) => (
            <button key={s} type="button" onClick={() => addApp(s)} className="rounded-full border border-line px-2.5 py-1 text-[12px] text-dim hover:bg-surface-2 hover:text-fg">
              + {s}
            </button>
          ))}
        </div>
      </Card>
    </div>
  );
}

function TimeTab() {
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const [r, setR] = useState<TimeReport | null>(null);
  const load = useCallback(() => void api.focus.report().then(setR, () => undefined), []);
  useEffect(load, [load]);
  useInterval(load, 60_000);
  const max = useMemo(() => Math.max(1, ...(r?.week.map((d) => d.seconds) ?? [1])), [r]);
  if (!settings) return null;
  const on = settings.focus.trackTime;
  const total = r?.apps.reduce((a, x) => a + x.seconds, 0) ?? 0;
  return (
    <div className="grid gap-4 lg:grid-cols-[1.2fr_1fr]">
      <Card className="p-6">
        <CardHeader
          icon={Clock}
          title="Today"
          subtitle={on ? `${hm(total)} at the PC${r?.focusMinutesToday ? ` · ${r.focusMinutesToday} min focused` : ''}` : 'Turn on to see where your time goes.'}
          actions={<Switch checked={on} onChange={(v) => void update({ focus: { trackTime: v } }).then(load)} label="Track time" />}
        />
        <p className="mt-2 text-[12px] text-faint">Adds up which program is in front while you use the PC (not when you’re away). Only program names and minutes, never window titles; kept on this PC.</p>
        <div className="mt-4 space-y-2">
          {!r?.apps.length ? (
            <EmptyState compact icon={Clock} title={on ? 'Nothing yet today' : 'Time tracking is off'} description={on ? 'Use your PC a bit and come back.' : undefined} />
          ) : (
            r.apps.slice(0, 15).map((a) => (
              <div key={a.app}>
                <div className="flex justify-between text-[13px]">
                  <span className="truncate text-fg">{a.name}</span>
                  <span className="shrink-0 tabular-nums text-dim">{hm(a.seconds)}</span>
                </div>
                <div className="mt-1 h-1.5 rounded-full bg-surface-2">
                  <div className="h-full rounded-full bg-accent" style={{ width: `${(a.seconds / Math.max(1, r.apps[0].seconds)) * 100}%` }} />
                </div>
              </div>
            ))
          )}
        </div>
      </Card>
      <Card className="p-6">
        <CardHeader
          icon={Clock}
          title="This week"
          actions={
            <Button
              size="sm"
              variant="ghost"
              icon={Trash}
              onClick={async () => {
                if (await confirm({ title: 'Forget all tracked time?', confirmLabel: 'Forget', tone: 'danger' })) await api.focus.clear().then(load);
              }}
            >
              Clear
            </Button>
          }
        />
        <div className="mt-6 flex h-40 items-end gap-2">
          {(r?.week ?? []).map((d) => (
            <div key={d.day} className="flex flex-1 flex-col items-center gap-1.5">
              <div className="w-full rounded-t-md bg-accent/80" style={{ height: `${(d.seconds / max) * 120}px`, minHeight: d.seconds ? 3 : 0 }} title={hm(d.seconds)} />
              <span className="text-[11px] text-faint">{new Date(`${d.day}T12:00:00`).toLocaleDateString(undefined, { weekday: 'short' })}</span>
            </div>
          ))}
        </div>
        {r && r.sessionsToday > 0 && (
          <div className="mt-4">
            <Badge tone="good">
              {r.sessionsToday} focus round{r.sessionsToday === 1 ? '' : 's'} today
            </Badge>
          </div>
        )}
      </Card>
    </div>
  );
}

type Tab = 'focus' | 'time';
const TABS: TabItem<Tab>[] = [
  { value: 'focus', label: 'Focus timer', icon: Timer },
  { value: 'time', label: 'Time', icon: Clock },
];

export function FocusPage() {
  const [tab, setTab] = useState<Tab>('focus');
  return (
    <Page title="Focus" subtitle="A focus timer that keeps distractions closed, and where your time goes.">
      <Tabs items={TABS} value={tab} onChange={setTab} className="mb-4" />
      {tab === 'focus' ? <FocusTab /> : <TimeTab />}
    </Page>
  );
}
