// A desktop widget: a small card sitting on the desktop (under other
// windows). Drag it by its body. Temperatures, now playing, a counter,
// batteries, network speed or a clock.

import type { Battery, Counter, MediaState, NetStatus, RigStatus } from '@shared/types';
import { Battery as BatteryIcon, Clock, Gauge, Music2, Sparkles, Thermometer, X } from 'lucide-react';
import { type ReactNode, useEffect, useState } from 'react';
import { api } from '../../api';
import { ThemeController } from '../../components/GlobalEvents';
import { useEvent, useInterval } from '../../lib/hooks';
import { useRoute } from '../../lib/router';
import { useSettings } from '../../state/settings';

function Shell({ icon: Icon, title, children, id }: { icon: typeof Clock; title: string; children: ReactNode; id: string }) {
  return (
    <div data-tauri-drag-region className="group relative m-1 flex h-[calc(100vh-8px)] flex-col rounded-2xl border border-white/10 bg-black/55 p-3.5 text-white shadow-lg backdrop-blur-md">
      <div data-tauri-drag-region className="flex items-center gap-1.5 text-[11.5px] font-medium uppercase tracking-wide text-white/60">
        <Icon size={13} aria-hidden /> {title}
      </div>
      <button type="button" aria-label="Remove widget" onClick={() => void api.widgets.remove(id)} className="absolute right-2 top-2 hidden rounded-md p-1 text-white/60 hover:bg-white/10 hover:text-white group-hover:block">
        <X size={13} />
      </button>
      <div data-tauri-drag-region className="mt-2 min-h-0 flex-1">
        {children}
      </div>
    </div>
  );
}

const speed = (b: number) => (b * 8 >= 1e6 ? `${((b * 8) / 1e6).toFixed(1)} Mbit/s` : `${Math.round((b * 8) / 1e3)} kbit/s`);

function Temps({ id }: { id: string }) {
  const [s, setS] = useState<RigStatus | null>(null);
  const hold = () => void api.rig.hold().then(setS, () => undefined);
  useEffect(hold, []);
  useInterval(hold, 4000);
  useEvent<RigStatus>('rig:status', setS);
  const row = (label: string, t: number | null | undefined) => (
    <div className="flex items-baseline justify-between">
      <span className="text-[13px] text-white/70">{label}</span>
      <span className={`font-display text-[24px] font-bold tabular-nums ${t != null && t >= 85 ? 'text-red-400' : ''}`}>{t != null ? `${Math.round(t)}°` : '—'}</span>
    </div>
  );
  return (
    <Shell icon={Thermometer} title="Temperatures" id={id}>
      {row('Processor', s?.cpuTemp)}
      {row('Graphics', s?.gpuTemp)}
    </Shell>
  );
}

function Music({ id }: { id: string }) {
  const [m, setM] = useState<MediaState | null>(null);
  useEffect(() => void api.media.state().then((r) => setM(r.state), () => undefined), []);
  useEvent<{ state: MediaState | null }>('media:state', (p) => setM(p.state));
  return (
    <Shell icon={Music2} title="Now playing" id={id}>
      {m ? (
        <>
          <div className="truncate text-[15px] font-semibold">{m.title}</div>
          <div className="truncate text-[12.5px] text-white/70">{m.artist}</div>
        </>
      ) : (
        <div className="text-[13px] text-white/60">Nothing playing</div>
      )}
    </Shell>
  );
}

function CounterW({ id }: { id: string }) {
  const [c, setC] = useState<Counter | null>(null);
  const load = () => void api.counters.state().then((s) => setC(s.counters[0] ?? null), () => undefined);
  useEffect(load, []);
  useEvent('counters:changed', load);
  return (
    <Shell icon={Sparkles} title={c?.name ?? 'Counter'} id={id}>
      <div className="font-display text-[40px] font-bold leading-none tabular-nums">{c ? c.count.toLocaleString() : '—'}</div>
    </Shell>
  );
}

function Batteries({ id }: { id: string }) {
  const [b, setB] = useState<Battery[]>([]);
  const load = () => void api.batteries().then(setB, () => undefined);
  useEffect(load, []);
  useInterval(load, 60000);
  return (
    <Shell icon={BatteryIcon} title="Batteries" id={id}>
      {b.length === 0 && <div className="text-[13px] text-white/60">No batteries found</div>}
      {b.slice(0, 3).map((x) => (
        <div key={x.id} className="flex items-center justify-between text-[13px]">
          <span className="truncate text-white/80">{x.name}</span>
          <span className="tabular-nums">{x.percent != null ? `${x.percent}%` : '—'}</span>
        </div>
      ))}
    </Shell>
  );
}

function Network({ id }: { id: string }) {
  const [n, setN] = useState<NetStatus | null>(null);
  const load = () => void api.network().then(setN, () => undefined);
  useEffect(load, []);
  useInterval(load, 2000);
  return (
    <Shell icon={Gauge} title="Network" id={id}>
      <div className="text-[13px] text-white/70">Down</div>
      <div className="font-display text-[20px] font-bold tabular-nums">{n ? speed(n.down) : '—'}</div>
      <div className="mt-1 text-[12px] text-white/60">Up {n ? speed(n.up) : '—'}</div>
    </Shell>
  );
}

function ClockW({ id }: { id: string }) {
  const [t, setT] = useState(new Date());
  useInterval(() => setT(new Date()), 1000);
  return (
    <Shell icon={Clock} title={t.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'short' })} id={id}>
      <div className="font-display text-[42px] font-bold leading-none tabular-nums">{t.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })}</div>
    </Shell>
  );
}

export function WidgetView() {
  const params = useRoute().params;
  const kind = params.get('kind') ?? 'clock';
  const id = params.get('id') ?? '';
  useEffect(() => {
    void useSettings.getState().load();
  }, []);
  const W = { temps: Temps, music: Music, counter: CounterW, battery: Batteries, network: Network, clock: ClockW }[kind] ?? ClockW;
  return (
    <>
      <ThemeController />
      <W id={id} />
    </>
  );
}
