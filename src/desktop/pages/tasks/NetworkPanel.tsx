// Network: speed now, data used per day, and the programs with connections
// open. (Windows only tells administrators how much data each program moves,
// so that isn't counted.)

import { formatBytes } from '@shared/format';
import type { NetStatus } from '@shared/types';
import { ArrowDown, ArrowUp, Globe, Wifi } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api } from '../../api';
import { Card, CardHeader } from '../../components/ui/Card';
import { Sparkline } from '../home/Sparkline';
import { LimitControl, speed, useAppLimits } from './AppLimits';

const rate = (b: number) => `${formatBytes(b, 1)}/s`;

export function NetworkPanel({ paused }: { paused?: boolean }) {
  const [n, setN] = useState<NetStatus | null>(null);
  const limits = useAppLimits();
  useEffect(() => {
    if (paused) return;
    let alive = true;
    const load = () => void api.network().then((x) => alive && setN(x), () => undefined);
    load();
    const t = setInterval(load, 2000);
    return () => {
      alive = false;
      clearInterval(t);
    };
  }, [paused]);
  if (!n || !n.supported) return null;
  const today = n.days[n.days.length - 1];
  const month = n.days.reduce((a, d) => a + d.down + d.up, 0);
  const maxDay = Math.max(1, ...n.days.map((d) => d.down + d.up));
  const maxRate = Math.max(1_000_000, ...n.history.map((h) => h[1]));
  return (
    <div className="mt-4 grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,1fr)]">
      <Card className="p-5">
        <CardHeader icon={Wifi} title="Network" subtitle="All programs together, on this PC’s network cards." />
        <div className="mt-4 grid grid-cols-2 gap-4">
          <div>
            <div className="flex items-center gap-1.5 text-[12px] text-faint">
              <ArrowDown size={13} className="text-accent" /> Download
            </div>
            <div className="font-display text-[22px] font-semibold tabular">{rate(n.down)}</div>
          </div>
          <div>
            <div className="flex items-center gap-1.5 text-[12px] text-faint">
              <ArrowUp size={13} className="text-accent-2" /> Upload
            </div>
            <div className="font-display text-[22px] font-semibold tabular">{rate(n.up)}</div>
          </div>
        </div>
        <div className="mt-3">
          <Sparkline values={n.history.map((h) => h[1])} max={maxRate} height={56} label="Download speed, last two minutes" />
        </div>
        <div className="mt-5 flex items-end justify-between text-[12px] text-faint">
          <span>
            Today <span className="font-semibold text-fg">{today ? formatBytes(today.down + today.up, 1) : '—'}</span>
            {today && ` (${formatBytes(today.down, 1)} down, ${formatBytes(today.up, 1)} up)`}
          </span>
          <span>
            30 days <span className="font-semibold text-fg">{formatBytes(month, 1)}</span>
          </span>
        </div>
        <div className="mt-2 flex h-20 items-end gap-[3px]" role="img" aria-label="Data used per day, last 30 days">
          {n.days.map((d) => (
            <div key={d.date} title={`${d.date}: ${formatBytes(d.down, 1)} down, ${formatBytes(d.up, 1)} up`} className="flex-1 rounded-t-sm bg-accent/70 transition-colors hover:bg-accent" style={{ height: `${Math.max(3, ((d.down + d.up) / maxDay) * 100)}%` }} />
          ))}
        </div>
      </Card>
      <Card className="p-5">
        <CardHeader icon={Globe} title="Online programs" subtitle="Programs with connections open right now, and where to. The gauge caps a program’s upload (administrator approval; downloads aren’t capped)." />
        <div className="mt-3 max-h-[300px] space-y-1 overflow-y-auto">
          {n.apps.length === 0 && <div className="py-6 text-center text-[13px] text-faint">Nothing connected right now.</div>}
          {n.apps.map((a) => {
            const lim = limits.list.find((l) => l.exe.toLowerCase() === a.process.toLowerCase());
            return (
              <div key={`${a.process}-${a.pid}`} className="rounded-lg px-2 py-1.5 hover:bg-surface-2">
                <div className="flex items-center justify-between gap-3 text-[13px]">
                  <span className="truncate font-medium">{a.process.replace(/\.exe$/i, '')}</span>
                  <span className="flex shrink-0 items-center gap-2">
                    <span className="tabular text-faint">
                      {a.connections} connection{a.connections === 1 ? '' : 's'}
                    </span>
                    {/\.exe$/i.test(a.process) && <LimitControl exe={a.process} limit={lim} busy={limits.busy === a.process} onSet={(k) => void limits.set(a.process, k)} onRemove={() => void limits.remove(a.process)} />}
                  </span>
                </div>
                <div className="truncate font-mono text-[11px] text-faint">{a.remotes.join(' · ')}</div>
              </div>
            );
          })}
        </div>
        {limits.list.some((l) => !n.apps.some((a) => a.process.toLowerCase() === l.exe.toLowerCase())) && (
          <div className="mt-3 border-t border-line pt-3">
            <div className="mb-1.5 text-[11px] font-semibold uppercase tracking-wider text-faint">Limits on programs that aren’t online</div>
            {limits.list
              .filter((l) => !n.apps.some((a) => a.process.toLowerCase() === l.exe.toLowerCase()))
              .map((l) => (
                <div key={l.exe} className="flex items-center justify-between gap-3 px-2 py-1 text-[13px]">
                  <span className="truncate">
                    {l.exe.replace(/\.exe$/i, '')}{' '}
                    <span className="text-faint">
                      · upload ≤ {speed(l.kbps)}
                      {l.active ? '' : ' · not active (removed outside OmniHub?)'}
                    </span>
                  </span>
                  <LimitControl exe={l.exe} limit={l} busy={limits.busy === l.exe} onSet={(k) => void limits.set(l.exe, k)} onRemove={() => void limits.remove(l.exe)} />
                </div>
              ))}
          </div>
        )}
      </Card>
    </div>
  );
}
