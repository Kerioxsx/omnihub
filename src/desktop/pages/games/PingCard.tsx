// Ping helper: measure latency, jitter and loss to the game's servers, and
// the settings on this PC that cut lag spikes.

import type { GameBoost, GameProfile, GameState, PingResult, PingTarget } from '@shared/types';
import { Activity, Cable, Wifi } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Switch, TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { toast } from '../../state/toasts';
import { AdminBadge, Row } from './shared';

function grade(ms: number | null): { tone: string; label: string } {
  if (ms == null) return { tone: 'text-bad', label: 'No reply' };
  if (ms < 40) return { tone: 'text-good', label: 'Great' };
  if (ms < 80) return { tone: 'text-accent', label: 'Good' };
  if (ms < 130) return { tone: 'text-warn', label: 'Playable' };
  return { tone: 'text-bad', label: 'High' };
}

export function PingCard({ profile, state, onBoost, onHost }: { profile: GameProfile; state: GameState | null; onBoost: (b: Partial<GameBoost>) => void; onHost: (h: string | null) => void }) {
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
          <Switch checked={b.wifiLowLatency} onChange={(v) => onBoost({ wifiLowLatency: v })} label="Wi-Fi low-latency mode" />
        </Row>
        <Row title="Prioritise the game's traffic" badge={<AdminBadge on={state?.networkPriority} />} hint="Marks its packets as real-time (DSCP 46). Routers with WMM or QoS — most Wi-Fi routers — send them ahead of downloads and streams.">
          <Switch checked={b.networkPriority} onChange={(v) => onBoost({ networkPriority: v })} label="Prioritise the game's traffic" />
        </Row>
        <Row title="Server to test" hint="Optional: the address of a server you play on (a host name or IP; add :port to test over TCP).">
          <TextInput value={host} onChange={(e) => setHost(e.target.value)} onBlur={() => onHost(host.trim() || null)} onKeyDown={(e) => e.key === 'Enter' && onHost(host.trim() || null)} placeholder={profile.kind === 'fortnite' ? 'Fortnite regions' : '1.1.1.1'} className="w-56 font-mono text-[12.5px]" />
        </Row>
      </div>
      <div className="px-5 pb-4 pt-1">
        <Callout tone="info" className="text-[12.5px]">
          No program can shorten the distance to a game server. These settings remove what adds lag on your side — closing downloaders under “Close while playing” helps the most on a busy connection.
        </Callout>
      </div>
    </Card>
  );
}
