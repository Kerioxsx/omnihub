// Ping helper: measure latency, jitter and loss to the game's servers, lag
// under load (what a busy connection adds), and the settings on this PC
// that cut lag spikes.

import type { GameBoost, GameProfile, GameState, LoadTest, PingResult, PingTarget } from '@shared/types';
import { motion } from 'motion/react';
import { Activity, ArrowDown, ArrowUp, Cable, Gauge, Wifi } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Switch, TextInput } from '../../components/ui/Form';
import { ProgressBar } from '../../components/ui/Progress';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useEvent } from '../../lib/hooks';
import { toast } from '../../state/toasts';
import { AdminBadge, Row } from './shared';

function grade(ms: number | null): { tone: string; label: string } {
  if (ms == null) return { tone: 'text-bad', label: 'No reply' };
  if (ms < 40) return { tone: 'text-good', label: 'Great' };
  if (ms < 80) return { tone: 'text-accent', label: 'Good' };
  if (ms < 130) return { tone: 'text-warn', label: 'Playable' };
  return { tone: 'text-bad', label: 'High' };
}

const GRADE_TONE: Record<string, string> = { 'A+': 'text-good border-good/40 bg-good/10', A: 'text-good border-good/40 bg-good/10', B: 'text-accent border-accent/40 bg-accent-soft', C: 'text-warn border-warn/40 bg-warn/10', D: 'text-bad border-bad/40 bg-bad/10', F: 'text-bad border-bad/40 bg-bad/10' };
const PHASE_LABEL: Record<string, string> = { idle: 'Measuring the ping with the line quiet…', download: 'Downloading as fast as the line allows…', upload: 'Uploading as fast as the line allows…' };

function LoadTestPanel({ profile }: { profile: GameProfile }) {
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<{ phase: string; progress: number; ms: number | null } | null>(null);
  const [result, setResult] = useState<LoadTest | null>(null);
  useEvent<{ phase: string; progress: number; ms: number | null }>('games:loadTest', (p) => running && setProgress(p));
  useEffect(() => setResult(null), [profile.id]);

  const run = async () => {
    setRunning(true);
    setResult(null);
    setProgress({ phase: 'idle', progress: 0, ms: null });
    try {
      const r = await api.games.loadTest(profile.id);
      setResult(r);
      if (r.error) toast.warn('Lag test incomplete', r.error);
    } catch (e) {
      toast.error('Lag test failed', errorText(e));
    } finally {
      setRunning(false);
      setProgress(null);
    }
  };
  const step = progress ? ['idle', 'download', 'upload'].indexOf(progress.phase) : 0;
  return (
    <div className="border-t border-line px-5 py-4">
      <div className="flex flex-wrap items-center gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2 text-[13.5px] font-medium text-fg">
            <Gauge size={15} className="text-accent" /> Lag under load
          </div>
          <div className="mt-0.5 text-[12.5px] leading-snug text-faint">Pings while the line is busy downloading, then uploading — what someone streaming or a game update does to your ping. About 20 seconds; uses up to a few hundred MB.</div>
        </div>
        <Button size="sm" variant="secondary" loading={running} onClick={() => void run()}>
          {result ? 'Test again' : 'Test lag under load'}
        </Button>
      </div>
      {progress && (
        <div className="mt-3">
          <div className="mb-1.5 flex justify-between text-[12px] text-dim">
            <span>{PHASE_LABEL[progress.phase] ?? 'Testing…'}</span>
            {progress.ms != null && <span className="tabular">{Math.round(progress.ms)} ms</span>}
          </div>
          <ProgressBar value={(step + progress.progress) / 3} />
        </div>
      )}
      {result && !result.error && result.grade && (
        <motion.div initial={{ opacity: 0, y: 4 }} animate={{ opacity: 1, y: 0 }} className="mt-3 space-y-3">
          <div className="flex flex-wrap items-center gap-4">
            <div className={cx('grid h-14 w-14 shrink-0 place-items-center rounded-2xl border font-display text-[24px] font-bold', GRADE_TONE[result.grade] ?? GRADE_TONE.C)}>{result.grade}</div>
            <div className="grid flex-1 grid-cols-3 gap-2 text-[12px]">
              <LoadStat label="Quiet" value={result.idleMs} />
              <LoadStat label="Downloading" icon={ArrowDown} value={result.downloadMs} speed={result.downloadMbps} base={result.idleMs} />
              <LoadStat label="Uploading" icon={ArrowUp} value={result.uploadMs} speed={result.uploadMbps} base={result.idleMs} />
            </div>
          </div>
          <ul className="space-y-1.5 text-[12.5px] leading-snug text-dim">
            {result.advice.map((a) => (
              <li key={a} className="flex gap-2">
                <span className="mt-[7px] h-1 w-1 shrink-0 rounded-full bg-accent" />
                {a}
              </li>
            ))}
          </ul>
          <div className="text-[11.5px] text-faint">Measured to {result.target}.</div>
        </motion.div>
      )}
    </div>
  );
}

function LoadStat({ label, value, icon: Icon, speed, base }: { label: string; value: number | null; icon?: typeof Gauge; speed?: number | null; base?: number | null }) {
  const extra = value != null && base != null ? value - base : null;
  return (
    <div className="rounded-xl border border-line bg-surface-2/60 px-3 py-2">
      <div className="flex items-center gap-1 text-[11px] font-medium uppercase tracking-[0.06em] text-faint">
        {Icon && <Icon size={11} />}
        {label}
      </div>
      <div className="text-[15px] font-semibold tabular text-fg">
        {value != null ? `${Math.round(value)} ms` : '—'}
        {extra != null && Icon && <span className={cx('ml-1 text-[11.5px] font-medium', extra >= 30 ? 'text-warn' : 'text-faint')}>+{Math.max(0, Math.round(extra))}</span>}
      </div>
      {speed != null && <div className="text-[11px] tabular text-faint">{speed >= 100 ? Math.round(speed) : speed.toFixed(1)} Mbps</div>}
    </div>
  );
}

export function PingCard({ profile, state, locked, onBoost, onHost }: { profile: GameProfile; state: GameState | null; locked?: boolean; onBoost: (b: Partial<GameBoost>) => void; onHost: (h: string | null) => void }) {
  const [targets, setTargets] = useState<PingTarget[]>([]);
  const [results, setResults] = useState<PingResult[] | null>(null);
  const [running, setRunning] = useState(false);
  const [host, setHost] = useState(profile.pingHost ?? '');

  useEffect(() => {
    void api.games.pingTargets(profile.id).then(setTargets, () => undefined);
    setResults(null);
  }, [profile.id, profile.pingHost, profile.kind]);

  const test = async () => {
    setRunning(true);
    try {
      setResults(await api.games.ping(profile.id));
    } catch (e) {
      toast.error('Ping test failed', errorText(e));
    } finally {
      setRunning(false);
    }
  };

  const best = results?.filter((r) => r.avgMs != null).sort((a, b) => (a.avgMs ?? 0) - (b.avgMs ?? 0))[0];
  const max = Math.max(60, ...(results ?? []).map((r) => r.maxMs ?? 0));
  const b = profile.boost;
  return (
    <Card>
      <CardHeader
        icon={Activity}
        title="Ping helper"
        subtitle={profile.kind === 'fortnite' ? "Fortnite's matchmaking regions" : profile.pingHost ? `Your server: ${profile.pingHost}` : 'Your connection to the nearest big networks'}
        className="px-5 pt-4"
        actions={
          <Button size="sm" loading={running} onClick={() => void test()}>
            {results ? 'Test again' : 'Test ping'}
          </Button>
        }
      />
      <div className="px-5 pb-1 pt-3">
        {results ? (
          <div className="space-y-1.5">
            {results.map((r) => {
              const g = grade(r.avgMs);
              return (
                <div key={r.id} className={cx('flex items-center gap-3 rounded-lg px-2 py-1.5', best?.id === r.id && results.length > 1 && 'bg-good/10')}>
                  <div className="w-36 shrink-0 truncate text-[13px] text-fg" title={r.host}>
                    {r.label}
                    {best?.id === r.id && results.length > 1 && <span className="ml-1.5 text-[11px] font-semibold text-good">best</span>}
                  </div>
                  <div className="relative h-2 flex-1 overflow-hidden rounded-full bg-surface-3">
                    {r.minMs != null && r.maxMs != null && <div className="absolute inset-y-0 rounded-full bg-accent/25" style={{ left: `${(r.minMs / max) * 100}%`, width: `${Math.max(1, ((r.maxMs - r.minMs) / max) * 100)}%` }} title={`${r.minMs}–${r.maxMs} ms`} />}
                    {r.avgMs != null && <div className="absolute inset-y-0 w-1 rounded-full bg-accent" style={{ left: `calc(${(r.avgMs / max) * 100}% - 2px)` }} />}
                  </div>
                  <div className={cx('w-16 text-right text-[13px] font-semibold tabular', g.tone)}>{r.avgMs != null ? `${Math.round(r.avgMs)} ms` : '—'}</div>
                  <div className="w-24 text-right text-[12px] tabular text-dim" title="Jitter: how much the ping jumps around">{r.jitterMs != null ? `±${r.jitterMs.toFixed(1)} jitter` : ''}</div>
                  <div className={cx('w-16 text-right text-[12px] tabular', r.loss > 0 ? 'text-bad' : 'text-faint')} title="Packets that never came back">{r.error ? 'failed' : `${Math.round(r.loss * 100)}% loss`}</div>
                </div>
              );
            })}
            {best && profile.kind === 'fortnite' && <div className="px-2 pt-1 text-[12px] text-dim">Pick <b className="text-fg">{best.label}</b> as the matchmaking region in Fortnite's settings for the lowest ping.</div>}
            {results.some((r) => (r.jitterMs ?? 0) > 8) && <div className="px-2 pt-1 text-[12px] text-warn">Jitter above ~8 ms is what makes a game feel laggy — usually Wi-Fi or something else downloading.</div>}
          </div>
        ) : (
          <div className="flex flex-wrap gap-1.5 pb-2">
            {targets.map((t) => (
              <span key={t.id} className="rounded-full bg-surface-2 px-2.5 py-1 text-[12px] text-dim" title={t.host}>
                {t.label}
              </span>
            ))}
          </div>
        )}
      </div>
      <div className="mt-2 divide-y divide-line border-t border-line">
        {state?.onWifi != null && (
          <div className="flex items-center gap-2.5 px-5 py-2.5 text-[12.5px] text-dim">
            {state.onWifi ? <Wifi size={15} className="text-warn" /> : <Cable size={15} className="text-good" />}
            {state.onWifi ? 'This PC is on Wi-Fi. A network cable is the single biggest fix for spikes and jitter.' : 'This PC is on a cable — the best connection for games.'}
          </div>
        )}
        <Row title="Wi-Fi low-latency mode" hint="While you play: no background Wi-Fi scans (the usual cause of a spike every minute) and Windows' streaming mode. Undone automatically afterwards.">
          <Switch checked={b.wifiLowLatency} disabled={locked} onChange={(v) => onBoost({ wifiLowLatency: v })} label="Wi-Fi low-latency mode" />
        </Row>
        <Row title="Prioritise the game's traffic" badge={<AdminBadge on={state?.networkPriority} />} hint="Marks its packets as real-time (DSCP 46). Routers with WMM or QoS — most Wi-Fi routers — send them ahead of downloads and streams.">
          <Switch checked={b.networkPriority} onChange={(v) => onBoost({ networkPriority: v })} label="Prioritise the game's traffic" />
        </Row>
        <Row title="Server to test" hint="Optional: the address of a server you play on (a host name or IP; add :port to test over TCP).">
          <TextInput value={host} onChange={(e) => setHost(e.target.value)} onBlur={() => onHost(host.trim() || null)} onKeyDown={(e) => e.key === 'Enter' && onHost(host.trim() || null)} placeholder={profile.kind === 'fortnite' ? 'Fortnite regions' : '1.1.1.1'} className="w-56 font-mono text-[12.5px]" />
        </Row>
      </div>
      <LoadTestPanel profile={profile} />
      <div className="px-5 pb-4 pt-1">
        <Callout tone="info" className="text-[12.5px]">
          No program can make the ping 0 or shorten the distance to a game server — the closest region is the floor. What OmniHub removes is everything your side adds on top: Wi-Fi scans, background downloads and apps, a busy connection.
        </Callout>
      </div>
    </Card>
  );
}
