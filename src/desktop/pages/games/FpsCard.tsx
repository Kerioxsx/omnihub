// FPS meter: frames per second while a game runs, measured by Intel's
// PresentMon. Setting it up (download, one approval), the live readout
// while playing, and past results to compare boosts.

import type { BoostMode, FpsLive, FpsOverview, FpsRecord, GameProfile, GameSession } from '@shared/types';
import { formatRelative } from '@shared/format';
import { motion } from 'motion/react';
import { Activity, Download, LogOut, ShieldCheck, Trophy } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Switch } from '../../components/ui/Form';
import { ProgressBar } from '../../components/ui/Progress';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useEvent } from '../../lib/hooks';
import { toast } from '../../state/toasts';
import { Row } from './shared';

export const MODE_LABEL: Record<BoostMode, string> = { competitive: 'Competitive', quality: 'Quality', custom: 'Custom' };

/** The last minute of live readings. */
export function useFpsLive(active: boolean): { live: FpsLive | null; trail: number[] } {
  const [live, setLive] = useState<FpsLive | null>(null);
  const trail = useRef<number[]>([]);
  useEvent<FpsLive>('games:fps', (l) => {
    trail.current = [...trail.current.slice(-59), l.fps];
    setLive(l);
  });
  useEffect(() => {
    if (!active) {
      setLive(null);
      trail.current = [];
    }
  }, [active]);
  return { live: active ? live : null, trail: trail.current };
}

export function Sparkline({ values, className }: { values: number[]; className?: string }) {
  if (values.length < 2) return null;
  const max = Math.max(...values) * 1.08;
  const min = Math.min(...values) * 0.92;
  const pts = values.map((v, i) => `${(i / (values.length - 1)) * 100},${100 - ((v - min) / Math.max(1, max - min)) * 100}`).join(' ');
  return (
    <svg viewBox="0 0 100 100" preserveAspectRatio="none" className={className} aria-hidden>
      <defs>
        <linearGradient id="fps-fill" x1="0" x2="0" y1="0" y2="1">
          <stop offset="0" stopColor="var(--accent)" stopOpacity="0.35" />
          <stop offset="1" stopColor="var(--accent)" stopOpacity="0" />
        </linearGradient>
      </defs>
      <polygon points={`0,100 ${pts} 100,100`} fill="url(#fps-fill)" />
      <polyline points={pts} fill="none" stroke="var(--accent)" strokeWidth="2" vectorEffect="non-scaling-stroke" strokeLinejoin="round" />
    </svg>
  );
}

export function LiveFps({ live, trail, compact }: { live: FpsLive; trail: number[]; compact?: boolean }) {
  return (
    <div className={cx('relative overflow-hidden rounded-2xl border border-accent/30 bg-accent-soft/40', compact ? 'px-4 py-3' : 'px-5 py-4')}>
      <div className="relative flex flex-wrap items-end gap-x-8 gap-y-2">
        <div>
          <div className="text-[11px] font-semibold uppercase tracking-[0.08em] text-accent">Live</div>
          <div className={cx('font-display font-bold tabular leading-none text-fg', compact ? 'text-[30px]' : 'text-[44px]')}>
            {Math.round(live.fps)}
            <span className="ml-1 text-[14px] font-semibold text-dim">FPS</span>
          </div>
        </div>
        <Stat label="1% low" value={`${Math.round(live.low1)}`} />
        <Stat label="Frame time" value={`${live.frameMs.toFixed(2)} ms`} />
        <Stat label="Slowest frame" value={`${live.worstMs.toFixed(1)} ms`} warn={live.worstMs > Math.max(live.frameMs * 2.5, 8)} />
        <Sparkline values={trail} className={cx('ml-auto min-w-[140px] flex-1 basis-40', compact ? 'h-10' : 'h-14')} />
      </div>
    </div>
  );
}

function Stat({ label, value, warn }: { label: string; value: string; warn?: boolean }) {
  return (
    <div>
      <div className="text-[11px] font-medium uppercase tracking-[0.06em] text-faint">{label}</div>
      <div className={cx('text-[16px] font-semibold tabular', warn ? 'text-warn' : 'text-fg')}>{value}</div>
    </div>
  );
}

function ResultRow({ r, best }: { r: FpsRecord; best: boolean }) {
  const s = r.summary;
  const minutes = Math.max(1, Math.round(s.seconds / 60));
  return (
    <div className={cx('grid grid-cols-[1fr_auto_auto_auto] items-center gap-x-5 rounded-lg px-2 py-1.5 text-[12.5px]', best && 'bg-good/8')}>
      <div className="flex min-w-0 items-center gap-2">
        <span className="truncate text-fg">{formatRelative(r.at)}</span>
        <Badge tone={r.mode === 'competitive' ? 'accent' : r.mode === 'quality' ? 'info' : 'neutral'}>{MODE_LABEL[r.mode]}</Badge>
        {best && <Trophy size={13} className="shrink-0 text-good" />}
        <span className="hidden text-faint sm:inline">{minutes} min</span>
      </div>
      <div className="text-right tabular">
        <span className="font-semibold text-fg">{Math.round(s.avg)}</span> <span className="text-faint">avg</span>
      </div>
      <div className="text-right tabular">
        <span className="font-semibold text-fg">{Math.round(s.low1)}</span> <span className="text-faint">1% low</span>
      </div>
      <div className={cx('w-20 text-right tabular', s.hitches > 10 ? 'text-warn' : 'text-faint')} title="Frames that took over 2.5× the average: the stutters you notice">
        {s.hitches} stutter{s.hitches === 1 ? '' : 's'}
      </div>
    </div>
  );
}

export function FpsCard({ profile, session, onMeter }: { profile: GameProfile; session: GameSession | null; onMeter: (on: boolean) => void }) {
  const [fps, setFps] = useState<FpsOverview | null>(null);
  const [busy, setBusy] = useState<'install' | 'allow' | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const active = !!session && session.profileId === profile.id && session.phase !== 'ended';
  const { live, trail } = useFpsLive(active);

  const load = () => void api.games.fps().then(setFps, () => undefined);
  useEffect(load, []);
  useEffect(() => {
    if (session?.phase === 'ended') load();
  }, [session?.phase]);
  useEvent<{ done: number; total: number }>('games:fpsInstall', (p) => setProgress(p.total ? p.done / p.total : null));

  const install = async () => {
    setBusy('install');
    setProgress(0);
    try {
      setFps(await api.games.fpsInstall());
      toast.success('FPS meter downloaded', 'Checked against the checksum published with this release.');
    } catch (e) {
      toast.error('Could not get the FPS meter', errorText(e));
    } finally {
      setBusy(null);
      setProgress(null);
    }
  };
  const allow = async () => {
    setBusy('allow');
    try {
      const r = await api.games.fpsAllow();
      setFps(r);
      if (r.status.signOutNeeded) toast.info('One more step', 'Sign out of Windows and back in once — then the FPS meter works in every game.');
    } catch (e) {
      toast.error('Not allowed', errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const st = fps?.status;
  const ready = !!st?.installed && !!st.allowed;
  const mine = fps?.history.filter((r) => r.profileId === profile.id).slice(0, 6) ?? [];
  const best = mine.reduce<FpsRecord | null>((b, r) => (!b || r.summary.low1 > b.summary.low1 ? r : b), null);
  return (
    <Card>
      <CardHeader icon={Activity} title="FPS meter" subtitle="Every frame the game shows, measured by Intel PresentMon — the tool hardware reviewers use. Works with anti-cheat; nothing touches the game." className="px-5 pt-4" />
      <div className="space-y-3 px-5 pb-4 pt-3">
        {st && !st.supported && <Callout tone="info">The FPS meter needs Windows.</Callout>}
        {st?.supported && !st.installed && (
          <Callout
            tone="accent"
            icon={Download}
            title="Get the FPS meter"
            action={
              <Button size="sm" icon={Download} loading={busy === 'install'} onClick={() => void install()}>
                Download
              </Button>
            }
          >
            PresentMon is published with each OmniHub release and checked against its checksum before it’s used.
            {progress != null && <ProgressBar value={progress} className="mt-2" />}
          </Callout>
        )}
        {st?.installed && !st.allowed && !st.signOutNeeded && (
          <Callout
            tone="warn"
            icon={ShieldCheck}
            title="Allow it once"
            action={
              <Button size="sm" icon={ShieldCheck} loading={busy === 'allow'} onClick={() => void allow()}>
                Allow
              </Button>
            }
          >
            Windows only shows frame timings to administrators and the “Performance Log Users” group. One approval adds you to that group; it counts from your next sign-in.
          </Callout>
        )}
        {st?.signOutNeeded && (
          <Callout tone="info" icon={LogOut} title="Sign out and back in once">
            You’re in “Performance Log Users” now. Windows applies it from your next sign-in; after that the meter runs in every game.
          </Callout>
        )}
        {live && <LiveFps live={live} trail={trail} />}
        {active && !live && ready && profile.boost.fpsMeter && <div className="text-[12.5px] text-faint">The readout starts once the game draws its first frames.</div>}
        {mine.length > 0 && (
          <div>
            <div className="mb-1 flex items-center justify-between text-[11px] font-semibold uppercase tracking-[0.08em] text-faint">
              <span>Past sessions</span>
              <span className="normal-case tracking-normal">Higher 1% lows feel smoother</span>
            </div>
            <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="space-y-0.5">
              {mine.map((r) => (
                <ResultRow key={r.at} r={r} best={mine.length > 1 && r === best} />
              ))}
            </motion.div>
          </div>
        )}
      </div>
      <div className="border-t border-line">
        <Row title="Measure while playing" hint={ready ? 'Starts when the game does and ends with it. Sessions over ten seconds are kept here.' : 'Turns on once the meter is set up above.'}>
          <Switch checked={profile.boost.fpsMeter} onChange={onMeter} label="Measure while playing" />
        </Row>
      </div>
    </Card>
  );
}
