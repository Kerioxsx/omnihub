// Gaming dashboard: FPS, temperatures, load and ping of the PC, live, so
// the PC's screen stays clean. FPS needs a game being measured (Games →
// boost with the FPS meter on).

import { Activity, Wifi } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { client, type LiveStats, type PingResult } from '../client';
import { cx, errorMessage, useInterval } from '../lib/util';
import { Ring } from '../ui/common';
import { SubHeader } from './More';

const tempColor = (t: number | null, hot: number) => (t == null ? 'var(--surface-3)' : t >= hot ? '#f87171' : t >= hot - 12 ? '#fbbf24' : '#34d399');

export function LivePage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const [s, setS] = useState<LiveStats | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [ping, setPing] = useState<PingResult | null>(null);
  const [profile, setProfile] = useState<string | null>(null);
  const load = useCallback(() => {
    client.live().then(
      (v) => (setS(v), setErr(null)),
      (e) => setErr(errorMessage(e)),
    );
  }, []);
  useEffect(() => {
    if (!active) return;
    load();
    client.games().then((g) => setProfile(g.session?.profileId ?? g.profiles[0]?.id ?? null), () => undefined);
  }, [active, load]);
  useInterval(load, active ? 1500 : null);
  const doPing = useCallback(() => {
    if (!profile) return;
    client.pingGame(profile).then((r) => setPing([...r.results].filter((x) => x.avgMs != null).sort((a, b) => (a.avgMs ?? 1e9) - (b.avgMs ?? 1e9))[0] ?? null), () => undefined);
  }, [profile]);
  useEffect(doPing, [doPing]);
  useInterval(doPing, active && profile ? 20_000 : null);

  const fps = s?.fps;
  const mem = s ? s.memoryUsed / Math.max(1, s.memoryTotal) : 0;
  return (
    <div className="scroller h-full">
      <SubHeader title="Gaming dashboard" subtitle={s?.game ? `Playing ${s.game}` : 'No game boosted'} onBack={onBack} />
      <div className="px-safe pb-tabbar space-y-4">
        {err && <div className="rounded-xl bg-bad/15 px-3 py-2 text-[13px] text-bad">{err}</div>}
        <div className="card flex flex-col items-center py-6">
          <div className="text-[13px] uppercase tracking-[0.2em] text-faint">FPS</div>
          <div className={cx('font-display text-[84px] font-extrabold leading-none tabular-nums', !fps && 'text-faint')}>{fps ? Math.round(fps.fps) : '—'}</div>
          <div className="mt-2 flex gap-5 text-[14px] text-dim">
            <span>1% low {fps ? Math.round(fps.low1) : '—'}</span>
            <span>{fps ? `${fps.frameMs.toFixed(1)} ms` : 'frame time —'}</span>
          </div>
          {!fps && <p className="mt-3 px-6 text-center text-[13px] text-faint">FPS shows while a boosted game is measured (OmniHub → Games, FPS meter on).</p>}
        </div>
        <div className="grid grid-cols-2 gap-3">
          {(
            [
              ['Processor', s?.cpuTemp ?? null, 90, s?.cpu ?? null],
              ['Graphics', s?.gpuTemp ?? null, 85, s?.gpu ?? null],
            ] as const
          ).map(([label, t, hot, load]) => (
            <div key={label} className="card flex flex-col items-center gap-2 py-4">
              <Ring value={t == null ? 0 : t / 100} size={96} stroke={9} color={tempColor(t, hot)}>
                <div className="text-center">
                  <div className="font-display text-[24px] font-bold tabular-nums">{t == null ? '—' : `${Math.round(t)}°`}</div>
                </div>
              </Ring>
              <div className="text-[14px] font-semibold">{label}</div>
              <div className="text-[12.5px] text-faint">{load == null ? 'Load —' : `Load ${Math.round(load)}%`}</div>
            </div>
          ))}
        </div>
        <div className="card grid grid-cols-2 divide-x divide-line">
          <div className="flex items-center gap-3 p-4">
            <Activity size={20} className="text-accent" />
            <div>
              <div className="text-[12px] text-faint">Memory</div>
              <div className="font-semibold tabular-nums">{s ? `${Math.round(mem * 100)}%` : '—'}</div>
            </div>
          </div>
          <div className="flex items-center gap-3 p-4">
            <Wifi size={20} className="text-accent" />
            <div className="min-w-0">
              <div className="text-[12px] text-faint">Ping{ping ? ` · ${ping.label}` : ''}</div>
              <div className="font-semibold tabular-nums">{ping?.avgMs != null ? `${Math.round(ping.avgMs)} ms` : '—'}</div>
            </div>
          </div>
        </div>
        {s && s.cpuTemp == null && <p className="px-2 text-center text-[12.5px] text-faint">Temperatures need LibreHardwareMonitor on the PC (OmniHub → My PC → Fans).</p>}
      </div>
    </div>
  );
}
