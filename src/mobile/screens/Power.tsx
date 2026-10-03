// Power: lock / display off (tap), sleep … shut down (hold to confirm),
// then a countdown the PC also shows, with a big Cancel.

import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { motion, AnimatePresence } from 'motion/react';
import { Lock, MonitorOff, Moon, Snowflake, LogOut, RotateCcw, Power, ShieldAlert, Timer, X, LoaderCircle } from 'lucide-react';
import type { PowerAction } from '@shared/types';
import { client } from '../client';
import { usePower, secondsLeft, toast, useApp } from '../state';
import { PageHeader, Ring, Skeleton, ErrorState } from '../ui/common';
import { PullToRefresh } from '../ui/PullToRefresh';
import { cx, errorMessage, formatCountdown, useNow, vibrate } from '../lib/util';
import { useEvent } from '../lib/events';

type PowerInfo = Awaited<ReturnType<typeof client.power>>;

const META: Record<PowerAction, { icon: typeof Lock; desc: string; tone: string; ring: string }> = {
  lock: { icon: Lock, desc: 'Locks the PC right away', tone: 'text-sky-400 bg-sky-500/15', ring: '#38bdf8' },
  displayOff: { icon: MonitorOff, desc: 'Turns the screens off', tone: 'text-indigo-400 bg-indigo-500/15', ring: '#818cf8' },
  sleep: { icon: Moon, desc: 'Low-power sleep', tone: 'text-violet-400 bg-violet-500/15', ring: '#a78bfa' },
  hibernate: { icon: Snowflake, desc: 'Saves the session to disk', tone: 'text-cyan-400 bg-cyan-500/15', ring: '#22d3ee' },
  signOut: { icon: LogOut, desc: 'Closes your Windows session', tone: 'text-amber-400 bg-amber-500/15', ring: '#fbbf24' },
  restart: { icon: RotateCcw, desc: 'Restarts Windows', tone: 'text-orange-400 bg-orange-500/15', ring: '#fb923c' },
  shutdown: { icon: Power, desc: 'Turns the PC off', tone: 'text-rose-400 bg-rose-500/15', ring: '#fb7185' },
};

const HOLD_MS = 1200;

/** Countdown card for a pending power action (Home and Power screens). */
export function PendingPowerCard({ className }: { className?: string }) {
  const { pending, skew, setPending } = usePower();
  const now = useNow(250, !!pending);
  const [busy, setBusy] = useState(false);
  const total = useRef<{ id: string; secs: number } | null>(null);
  if (pending && total.current?.id !== pending.id) total.current = { id: pending.id, secs: Math.max(1, secondsLeft(pending, skew, now)) };

  const cancel = async () => {
    setBusy(true);
    usePower.getState().setCancelling(true);
    try {
      const r = await client.cancelPower();
      setPending(null);
      vibrate(15);
      if (r.cancelled) toast.success('Cancelled', `${pending?.label ?? 'The action'} won't run.`);
      else toast.info('Nothing to cancel', 'It already ran or was cancelled on the PC.');
    } catch (e) {
      toast.error("Couldn't cancel", errorMessage(e));
    } finally {
      setBusy(false);
      setTimeout(() => usePower.getState().setCancelling(false), 1500);
    }
  };

  return (
    <AnimatePresence initial={false}>
      {pending && (
        <motion.section
          key={pending.id}
          initial={{ opacity: 0, height: 0 }}
          animate={{ opacity: 1, height: 'auto' }}
          exit={{ opacity: 0, height: 0 }}
          transition={{ duration: 0.28, ease: [0.2, 0.8, 0.2, 1] }}
          className={cx('overflow-hidden', className)}
        >
          <PendingBody label={pending.label} action={pending.action} by={pending.requestedBy} left={secondsLeft(pending, skew, now)} total={total.current?.secs ?? 10} busy={busy} onCancel={cancel} />
        </motion.section>
      )}
    </AnimatePresence>
  );
}

function PendingBody({ label, action, by, left, total, busy, onCancel }: { label: string; action: PowerAction; by: string; left: number; total: number; busy: boolean; onCancel: () => void }) {
  const m = META[action] ?? META.shutdown;
  const Icon = m.icon;
  const running = left <= 0;
  return (
    <div className="relative overflow-hidden rounded-[22px] border border-bad/35 bg-[linear-gradient(135deg,rgba(248,113,113,.16),rgba(251,146,60,.08))] p-4">
      <div className="flex items-center gap-4">
        <Ring value={running ? 1 : left / total} size={78} stroke={7} color="var(--bad)" track="rgba(248,113,113,.18)">
          {running ? <LoaderCircle size={24} className="spin text-bad" /> : <span className="num font-display text-[22px] font-bold">{Math.ceil(left)}</span>}
        </Ring>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-1.5 text-[13px] font-semibold text-bad">
            <Icon size={14} /> {running ? 'Running now' : 'Scheduled'}
          </div>
          <div className="font-display text-xl font-bold leading-tight">{running ? `${label}…` : `${label} in ${formatCountdown(left)}`}</div>
          <div className="mt-0.5 truncate text-xs text-dim">Requested by {by.replace(/^phone:/, '')} · the PC shows a cancel banner too</div>
        </div>
      </div>
      {!running && (
        <button onClick={onCancel} disabled={busy} className="press mt-4 flex h-14 w-full items-center justify-center gap-2 rounded-2xl bg-bad text-[17px] font-bold text-white shadow-[0_10px_30px_-12px_rgba(248,113,113,.8)] disabled:opacity-60">
          {busy ? <LoaderCircle size={20} className="spin" /> : <X size={20} strokeWidth={2.6} />} Cancel {label.toLowerCase()}
        </button>
      )}
    </div>
  );
}

export function PowerScreen({ active }: { active: boolean }) {
  const [data, setData] = useState<PowerInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { setPending, setSkew } = usePower();
  const info = useApp((s) => s.info);
  const scroller = useRef<HTMLDivElement | null>(null);
  const [sent, setSent] = useState<PowerAction | null>(null);

  const load = useCallback(async () => {
    try {
      const d = await client.power();
      setData(d);
      setError(null);
      setPending(d.pending);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, [setPending]);

  useEffect(() => {
    if (active) load();
  }, [active, load]);
  useEvent('power:executed', () => setSent(null));

  const run = async (action: PowerAction, destructive: boolean) => {
    const t0 = Date.now() / 1000;
    try {
      const r = await client.requestPower(action);
      const t1 = Date.now() / 1000;
      const delay = destructive ? (data?.delaySeconds ?? 0) : 0;
      // Estimate the PC↔phone clock offset from our own request.
      const skew = r.pending.at - delay - (t0 + t1) / 2;
      setSkew(Math.abs(skew) < 2 ? 0 : skew);
      if (destructive) {
        setPending(r.pending);
        scroller.current?.scrollTo({ top: 0, behavior: 'smooth' });
      } else {
        setSent(action);
        setTimeout(() => setSent((s) => (s === action ? null : s)), 2500);
        vibrate(10);
      }
    } catch (e) {
      toast.error(`Couldn't ${action === 'displayOff' ? 'turn off the display' : 'do that'}`, errorMessage(e));
    }
  };

  const enabled = data?.enabled ?? info?.features.power ?? true;
  const quick = data?.actions.filter((a) => !a.destructive) ?? [];
  const hold = data?.actions.filter((a) => a.destructive) ?? [];

  return (
    <PullToRefresh onRefresh={load} scrollRef={(el) => (scroller.current = el)}>
      <PageHeader title="Power" subtitle={info?.name} />
      <div className="px-safe pb-tabbar">
        <PendingPowerCard className="mb-4" />
        {!enabled && (
          <div className="mb-4 flex items-start gap-3 rounded-2xl border border-warn/30 bg-warn/10 p-4 text-sm">
            <ShieldAlert size={20} className="mt-0.5 shrink-0 text-warn" />
            <div>
              <div className="font-semibold">Power control is turned off on the PC</div>
              <div className="mt-0.5 text-dim">Turn it on in OmniHub → Settings → Phone → “Allow power actions”.</div>
            </div>
          </div>
        )}
        {error && !data ? (
          <ErrorState message={error} onRetry={load} />
        ) : !data ? (
          <div className="grid grid-cols-2 gap-3">
            {Array.from({ length: 6 }, (_, i) => (
              <Skeleton key={i} className="h-[128px] rounded-[22px]" />
            ))}
          </div>
        ) : (
          <>
            <div className="grid grid-cols-2 gap-3">
              {quick.map((a) => {
                const m = META[a.action];
                const Icon = m.icon;
                const done = sent === a.action;
                return (
                  <button key={a.action} disabled={!enabled} onClick={() => run(a.action, false)} className="press card flex flex-col items-start gap-3 p-4 text-left disabled:opacity-45">
                    <div className={cx('grid h-12 w-12 place-items-center rounded-2xl transition-colors', done ? 'bg-good/20 text-good' : m.tone)}>
                      <Icon size={24} />
                    </div>
                    <div>
                      <div className="text-[16px] font-semibold">{a.label}</div>
                      <div className="text-xs text-dim">{done ? 'Sent ✓' : m.desc}</div>
                    </div>
                  </button>
                );
              })}
            </div>
            <div className="mb-2.5 mt-6 flex items-center justify-between px-1">
              <h2 className="text-[13px] font-semibold uppercase tracking-[0.08em] text-faint">Hold to confirm</h2>
              <span className="flex items-center gap-1 text-xs text-faint">
                <Timer size={13} /> {data.delaySeconds} s countdown
              </span>
            </div>
            <div className="grid grid-cols-2 gap-3">
              {hold.map((a, i) => (
                <HoldCard
                  key={a.action}
                  action={a.action}
                  label={a.label}
                  disabled={!enabled}
                  wide={i === hold.length - 1 && hold.length % 2 === 1}
                  onConfirm={() => run(a.action, true)}
                />
              ))}
            </div>
            <p className="mt-5 px-2 text-center text-xs leading-relaxed text-faint">
              Actions that end your session wait {data.delaySeconds} seconds so you (or anyone at the PC) can cancel.
            </p>
          </>
        )}
      </div>
    </PullToRefresh>
  );
}

function HoldCard({ action, label, disabled, wide, onConfirm }: { action: PowerAction; label: string; disabled: boolean; wide: boolean; onConfirm: () => void }) {
  const m = META[action];
  const Icon = m.icon;
  const [p, setP] = useState(0);
  const raf = useRef(0);
  const t0 = useRef(0);
  const holding = useRef(false);

  const tick = () => {
    if (!holding.current) return;
    const v = Math.min(1, (performance.now() - t0.current) / HOLD_MS);
    setP(v);
    if (v >= 1) {
      holding.current = false;
      vibrate([25, 50, 70]);
      onConfirm();
      setTimeout(() => setP(0), 450);
      return;
    }
    raf.current = requestAnimationFrame(tick);
  };
  const start = () => {
    if (disabled || holding.current) return;
    holding.current = true;
    t0.current = performance.now();
    vibrate(12);
    raf.current = requestAnimationFrame(tick);
  };
  const stop = () => {
    if (!holding.current) return;
    holding.current = false;
    cancelAnimationFrame(raf.current);
    setP(0);
  };
  useEffect(() => () => cancelAnimationFrame(raf.current), []);

  return (
    <button
      disabled={disabled}
      aria-label={`Hold to ${label.toLowerCase()}`}
      onPointerDown={start}
      onPointerUp={stop}
      onPointerLeave={stop}
      onPointerCancel={stop}
      onContextMenu={(e) => e.preventDefault()}
      onKeyDown={(e) => {
        if ((e.key === ' ' || e.key === 'Enter') && !e.repeat) {
          e.preventDefault();
          start();
        }
      }}
      onKeyUp={stop}
      onBlur={stop}
      className={cx('card relative flex select-none items-center gap-3.5 overflow-hidden p-4 text-left [-webkit-touch-callout:none] disabled:opacity-45', wide && 'col-span-2')}
      style={{ transform: `scale(${1 - p * 0.03})`, transition: p === 0 ? 'transform .2s' : 'none' }}
    >
      <div className="pointer-events-none absolute inset-0 origin-left" style={{ background: `linear-gradient(90deg, ${m.ring}33, ${m.ring}1a)`, transform: `scaleX(${p})` }} />
      <div className="relative">
        <Ring value={p} size={52} stroke={3.5} color={m.ring} track="transparent">
          <div className={cx('grid h-11 w-11 place-items-center rounded-full', m.tone)}>
            <Icon size={21} />
          </div>
        </Ring>
      </div>
      <div className="relative min-w-0">
        <div className="text-[16px] font-semibold">{label}</div>
        <div className="text-xs text-dim">{p >= 1 ? 'Confirmed' : p > 0 ? 'Keep holding…' : wide ? `${m.desc} · hold` : 'Press & hold'}</div>
      </div>
    </button>
  );
}

export function PowerIcon({ action, size = 20 }: { action: PowerAction; size?: number }): ReactNode {
  const Icon = META[action]?.icon ?? Power;
  return <Icon size={size} />;
}
