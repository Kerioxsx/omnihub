// My PC → Batteries and Health: every battery Windows knows about (the PC,
// controllers, Bluetooth headsets and mice) and a quick health score.

import type { Battery, PcHealth } from '@shared/types';
import { BatteryCharging, BatteryFull, BatteryLow, BatteryMedium, Bluetooth, Gamepad2, Laptop, RefreshCw, ShieldCheck, Plug } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, Spinner } from '../../components/ui/Card';
import { ProgressRing } from '../../components/ui/Progress';
import { EmptyState, ErrorState } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { navigate } from '../../lib/router';
import { Sparkline } from '../home/Sparkline';

function left(min: number): string {
  if (min >= 60 * 24) {
    const d = Math.round(min / 60 / 24);
    return `${d} day${d === 1 ? '' : 's'} left`;
  }
  if (min >= 60) return `${Math.floor(min / 60)} h ${min % 60 ? `${min % 60} min ` : ''}left`;
  return `${min} min left`;
}

const KIND_ICON = { pc: Laptop, controller: Gamepad2, bluetooth: Bluetooth };

function tone(p: number | null) {
  if (p == null) return 'var(--text-faint)';
  return p <= 15 ? 'var(--bad)' : p <= 30 ? 'var(--warn)' : 'var(--good)';
}

export function BatteriesTab() {
  const [list, setList] = useState<Battery[] | null>(null);
  const [busy, setBusy] = useState(false);
  const load = async (fresh: boolean) => {
    setBusy(true);
    try {
      setList(await api.batteries(fresh));
    } finally {
      setBusy(false);
    }
  };
  useEffect(() => {
    void load(false);
    const t = setInterval(() => void load(false), 60_000);
    return () => clearInterval(t);
  }, []);
  useEvent('battery:low', () => void load(false));
  if (!list) return <Spinner />;
  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-3">
        <p className="text-[12.5px] text-dim">Checked every two minutes. You get a notification at 20% and 10%.</p>
        <Button size="sm" variant="secondary" icon={RefreshCw} loading={busy} onClick={() => void load(true)}>
          Check now
        </Button>
      </div>
      {list.length === 0 ? (
        <EmptyState compact icon={BatteryMedium} title="No batteries found" description="Laptops, Xbox controllers, and Bluetooth devices that tell Windows their battery (Settings → Bluetooth & devices) show here." />
      ) : (
        list.map((b) => {
          const Icon = KIND_ICON[b.kind];
          const Bat = b.charging ? BatteryCharging : b.percent == null ? BatteryMedium : b.percent <= 20 ? BatteryLow : b.percent >= 80 ? BatteryFull : BatteryMedium;
          return (
            <Card key={b.id} className="p-4">
              <div className="flex items-center gap-3">
                <span className="grid h-10 w-10 place-items-center rounded-xl bg-surface-2 text-dim">
                  <Icon size={18} />
                </span>
                <div className="min-w-0 flex-1">
                  <div className="truncate font-medium">{b.name}</div>
                  <div className="flex flex-wrap items-center gap-1.5 text-[12px] text-dim">
                    {b.wired ? (
                      <Badge icon={Plug}>Wired</Badge>
                    ) : b.charging ? (
                      <Badge tone="good" icon={BatteryCharging}>
                        Charging
                      </Badge>
                    ) : b.minutesLeft != null ? (
                      <span>{left(b.minutesLeft)}</span>
                    ) : null}
                    {b.kind === 'controller' && !b.wired && <span className="text-faint">Rough level (Xbox controllers report four steps)</span>}
                  </div>
                </div>
                {b.percent != null && (
                  <div className="flex items-center gap-2 text-right">
                    <Bat size={20} style={{ color: tone(b.percent) }} aria-hidden />
                    <span className="font-display text-[22px] font-bold tabular-nums">{b.percent}%</span>
                  </div>
                )}
              </div>
              {b.percent != null && (
                <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-surface-3">
                  <div className="h-full rounded-full transition-all" style={{ width: `${b.percent}%`, background: tone(b.percent) }} />
                </div>
              )}
              {b.history.length > 2 && (
                <div className="mt-3">
                  <Sparkline values={b.history.map((h) => h[1])} max={100} height={36} label={`${b.name} battery today`} />
                </div>
              )}
            </Card>
          );
        })
      )}
    </div>
  );
}

const STATUS = {
  ok: { tone: 'good', label: 'Good' },
  warn: { tone: 'warn', label: 'Look at it' },
  bad: { tone: 'bad', label: 'Fix it' },
} as const;

export function HealthTab() {
  const [h, setH] = useState<PcHealth | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const run = async () => {
    setBusy(true);
    setError(null);
    try {
      setH(await api.pcHealth());
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  useEffect(() => {
    void run();
  }, []);
  if (error) return <ErrorState error={error} onRetry={() => void run()} />;
  if (!h) {
    return (
      <div className="flex items-center gap-3 text-[13px] text-dim">
        <Spinner /> Looking over your PC…
      </div>
    );
  }
  const color = h.score >= 85 ? 'var(--good)' : h.score >= 60 ? 'var(--warn)' : 'var(--bad)';
  const bad = h.checks.filter((c) => c.status !== 'ok');
  return (
    <div className="space-y-4">
      <Card className="flex items-center gap-5 p-5">
        <ProgressRing value={h.score} size={96} stroke={9} color={color} label={`Health score ${h.score} out of 100`}>
          <span className="font-display text-[28px] font-bold tabular-nums">{h.score}</span>
        </ProgressRing>
        <div className="min-w-0 flex-1">
          <div className="font-display text-[18px] font-semibold">{h.score >= 85 ? 'Your PC is in good shape' : h.score >= 60 ? 'A few things to look at' : 'Your PC needs some care'}</div>
          <div className="text-[13px] text-dim">{bad.length ? `${bad.length} of ${h.checks.length} checks want attention.` : 'Every check passed.'}</div>
        </div>
        <Button size="sm" variant="secondary" icon={RefreshCw} loading={busy} onClick={() => void run()}>
          Check again
        </Button>
      </Card>
      <div className="divide-y divide-line rounded-2xl border border-line">
        {[...bad, ...h.checks.filter((c) => c.status === 'ok')].map((c) => (
          <div key={c.id} className="flex items-center gap-3 px-4 py-3">
            <span className="h-2.5 w-2.5 shrink-0 rounded-full" style={{ background: `var(--${STATUS[c.status].tone})` }} aria-hidden />
            <div className="min-w-0 flex-1">
              <div className="text-[13.5px] font-medium">{c.title}</div>
              <div className="text-[12.5px] text-dim">{c.detail}</div>
            </div>
            <Badge tone={STATUS[c.status].tone}>{STATUS[c.status].label}</Badge>
            {c.fix && c.route && (
              <Button size="xs" variant="subtle" onClick={() => navigate(c.route!)}>
                {c.fix}
              </Button>
            )}
            {c.fix && !c.route && <span className="text-[12px] text-faint">{c.fix}</span>}
          </div>
        ))}
      </div>
      <p className="flex items-center gap-1.5 text-[12px] text-faint">
        <ShieldCheck size={13} /> Only looks; nothing is changed until you choose to fix something.
      </p>
    </div>
  );
}
