// Games: start a game on the PC with its boost profile, follow the boost,
// stop it, and test the ping — from the couch.

import { useCallback, useEffect, useState } from 'react';
import { Activity, CheckCircle2, CircleSlash, Gamepad2, LoaderCircle, Rocket, Square, XCircle, Zap } from 'lucide-react';
import { formatRelative } from '@shared/format';
import { client, type FpsLive, type GameKind, type GameSession, type PhoneGame, type PingResult } from '../client';
import { useEvent } from '../lib/events';
import { toast } from '../state';
import { PullToRefresh } from '../ui/PullToRefresh';
import { Sheet } from '../ui/Sheet';
import { Button, Empty, ErrorState, Skeleton } from '../ui/common';
import { SubHeader } from './More';
import { cx, errorMessage, vibrate } from '../lib/util';

const TILE: Record<GameKind, { short: string; from: string; to: string }> = {
  fortnite: { short: 'FN', from: '#7c3aed', to: '#0ea5e9' },
  roblox: { short: 'RB', from: '#334155', to: '#0f172a' },
  valorant: { short: 'VA', from: '#ff4655', to: '#7f1d1d' },
  cs2: { short: 'CS', from: '#f59e0b', to: '#78350f' },
  apex: { short: 'AP', from: '#dc2626', to: '#450a0a' },
  overwatch: { short: 'OW', from: '#f97316', to: '#1e293b' },
  rocketLeague: { short: 'RL', from: '#2563eb', to: '#ea580c' },
  gta5: { short: 'V', from: '#16a34a', to: '#14532d' },
  callOfDuty: { short: 'CoD', from: '#57534e', to: '#1c1917' },
  league: { short: 'LoL', from: '#c8aa6e', to: '#0a1428' },
  minecraft: { short: 'MC', from: '#65a30d', to: '#3f2a14' },
  cyberpunk: { short: '77', from: '#facc15', to: '#0e7490' },
  custom: { short: '★', from: '#8b5cf6', to: '#22d3ee' },
};

const PHASE: Record<GameSession['phase'], string> = {
  starting: 'Boosting…',
  waiting: 'Starting the game on the PC',
  playing: 'Playing — boosted',
  boosted: 'Boosted — start the game',
  restoring: 'Putting everything back…',
  ended: 'Boost ended',
};

function Tile({ kind, size = 48 }: { kind: GameKind; size?: number }) {
  const t = TILE[kind] ?? TILE.custom;
  return (
    <div className="grid shrink-0 place-items-center rounded-2xl font-display font-bold text-white" style={{ width: size, height: size, fontSize: size * (t.short.length > 2 ? 0.28 : 0.36), background: `linear-gradient(135deg, ${t.from}, ${t.to})` }} aria-hidden>
      {t.short}
    </div>
  );
}

export function GamesPage({ onBack }: { onBack: () => void }) {
  const [games, setGames] = useState<PhoneGame[] | null>(null);
  const [session, setSession] = useState<GameSession | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [ping, setPing] = useState<{ game: PhoneGame; results: PingResult[] | null } | null>(null);
  const [fps, setFps] = useState<FpsLive | null>(null);

  const load = useCallback(async () => {
    try {
      const r = await client.games();
      setGames(r.profiles);
      setSession(r.session);
      setFps(r.fps ?? null);
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);
  useEffect(() => {
    void load();
  }, [load]);
  useEvent<GameSession | null>('games:session', (s) => {
    setSession(s);
    if (s?.phase === 'ended') {
      setFps(null);
      void load();
    }
  });
  // The PC's FPS meter, live: the phone as a second screen.
  useEvent<FpsLive>('games:fps', setFps);

  const play = async (g: PhoneGame, launch: boolean) => {
    setBusy(g.id);
    try {
      const r = await client.playGame(g.id, launch);
      setSession(r.session);
      vibrate(15);
      toast.success(launch ? `Starting ${g.name} on the PC` : 'PC boosted', launch ? 'Boost first, then the game.' : 'Start the game when you like.');
    } catch (e) {
      toast.error(launch ? `Couldn't start ${g.name}` : "Couldn't boost", errorMessage(e));
    } finally {
      setBusy(null);
    }
  };
  const stop = async () => {
    try {
      await client.stopGame();
      vibrate(10);
    } catch (e) {
      toast.error("Couldn't stop the boost", errorMessage(e));
    }
  };
  const testPing = async (g: PhoneGame) => {
    setPing({ game: g, results: null });
    try {
      const r = await client.pingGame(g.id);
      setPing({ game: g, results: r.results });
    } catch (e) {
      setPing(null);
      toast.error('Ping test failed', errorMessage(e));
    }
  };

  const active = session && session.phase !== 'ended' ? session : null;
  return (
    <div className="flex h-full flex-col">
      <SubHeader title="Games" subtitle="Boost the PC and start a game" onBack={onBack} />
      <PullToRefresh onRefresh={load} className="min-h-0 flex-1">
        <div className="px-safe pb-tabbar">
          {session && (
            <section className={cx('card mb-4 p-4', active && 'border-accent/40')}>
              <div className="flex items-center gap-3">
                <div className={cx('grid h-10 w-10 place-items-center rounded-xl', active ? 'bg-accent-soft text-accent' : 'bg-surface-2 text-dim')}>
                  {session.phase === 'starting' || session.phase === 'restoring' ? <LoaderCircle size={19} className="spin" /> : active ? <Zap size={19} /> : <CheckCircle2 size={19} />}
                </div>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[15px] font-semibold">{session.name}</div>
                  <div className={cx('truncate text-[13px]', active ? 'text-accent' : 'text-dim')}>{session.message ?? PHASE[session.phase]}</div>
                </div>
              </div>
              {active && fps && (
                <div className="mt-3 flex items-end justify-between rounded-2xl bg-accent-soft px-4 py-3">
                  <div>
                    <div className="text-[11px] font-semibold uppercase tracking-wider text-accent">Live on the PC</div>
                    <div className="num font-display text-[40px] font-bold leading-none">
                      {Math.round(fps.fps)}
                      <span className="ml-1 text-[14px] font-semibold text-dim">FPS</span>
                    </div>
                  </div>
                  <div className="text-right text-[12.5px] text-dim">
                    <div>
                      <b className="num text-[15px] text-fg">{Math.round(fps.low1)}</b> 1% low
                    </div>
                    <div>
                      <b className="num text-[15px] text-fg">{fps.frameMs.toFixed(2)}</b> ms/frame
                    </div>
                  </div>
                </div>
              )}
              {session.phase === 'ended' && session.fps && (
                <div className="mt-3 grid grid-cols-3 gap-2 text-center">
                  {[
                    ['FPS avg', session.fps.avg],
                    ['1% low', session.fps.low1],
                    ['0.1% low', session.fps.low01],
                  ].map(([l, v]) => (
                    <div key={l as string} className="rounded-xl bg-surface-2 px-2 py-2">
                      <div className="num text-[18px] font-bold">{Math.round(v as number)}</div>
                      <div className="text-[11px] text-dim">{l}</div>
                    </div>
                  ))}
                </div>
              )}
              <ul className="mt-3 space-y-1.5">
                {(session.phase === 'ended' && session.restored.length ? session.restored : session.steps).map((s) => {
                  const Icon = s.status === 'done' ? CheckCircle2 : s.status === 'failed' ? XCircle : CircleSlash;
                  return (
                    <li key={s.id} className="flex items-start gap-2 text-[13px]">
                      <Icon size={15} className={cx('mt-0.5 shrink-0', s.status === 'done' ? 'text-good' : s.status === 'failed' ? 'text-bad' : 'text-faint')} />
                      <span className="min-w-0">
                        <span className="font-semibold">{s.label}</span> <span className="text-dim">{s.detail}</span>
                      </span>
                    </li>
                  );
                })}
              </ul>
              {active && (
                <Button className="mt-4 w-full" variant="ghost" icon={<Square size={17} />} loading={session.phase === 'restoring'} onClick={() => void stop()}>
                  Stop boost
                </Button>
              )}
            </section>
          )}

          {error && !games ? (
            <ErrorState message={error} onRetry={() => void load()} />
          ) : !games ? (
            <div className="space-y-3">
              {[0, 1].map((i) => (
                <Skeleton key={i} className="h-[132px] rounded-3xl" />
              ))}
            </div>
          ) : games.length === 0 ? (
            <Empty icon={<Gamepad2 size={28} />} title="No game profiles yet" body="Add games on the PC in OmniHub → Games; they show up here to start from your phone." />
          ) : (
            <div className="space-y-3">
              {games.map((g) => (
                <section key={g.id} className="card p-4">
                  <div className="flex items-center gap-3.5">
                    <Tile kind={g.kind} />
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[17px] font-semibold">{g.name}</div>
                      <div className="truncate text-[13px] text-dim">{active?.profileId === g.id ? <span className="text-accent">{PHASE[active.phase]}</span> : g.lastPlayed ? `Played ${formatRelative(g.lastPlayed)}` : 'Not played yet'}</div>
                    </div>
                  </div>
                  <div className="mt-3.5 grid grid-cols-2 gap-2">
                    {g.canLaunch ? (
                      <Button size="md" className="col-span-2" icon={<Rocket size={18} />} disabled={!!active} loading={busy === g.id} onClick={() => void play(g, true)}>
                        Play on PC
                      </Button>
                    ) : (
                      <Button size="md" icon={<Zap size={18} />} disabled={!!active} loading={busy === g.id} onClick={() => void play(g, false)}>
                        Boost PC
                      </Button>
                    )}
                    {g.canLaunch && (
                      <Button size="md" variant="ghost" icon={<Zap size={18} />} disabled={!!active || busy === g.id} onClick={() => void play(g, false)} aria-label="Boost only">
                        Boost
                      </Button>
                    )}
                    <Button size="md" variant="ghost" icon={<Activity size={18} />} onClick={() => void testPing(g)} aria-label={`Test ping for ${g.name}`}>
                      Ping
                    </Button>
                  </div>
                </section>
              ))}
            </div>
          )}
          <p className="mt-4 px-2 text-center text-xs leading-relaxed text-faint">Everything the boost changes is put back when the game closes. Set up what each boost does on the PC.</p>
        </div>
      </PullToRefresh>

      <Sheet open={!!ping} onClose={() => setPing(null)} title={ping ? `Ping · ${ping.game.name}` : ''} subtitle="Measured from the PC">
        {ping &&
          (ping.results ? (
            <div className="space-y-2 pb-2">
              {ping.results.map((r) => (
                <div key={r.id} className="flex items-center gap-3 rounded-xl bg-surface-2 px-3.5 py-3">
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[15px] font-semibold">{r.label}</div>
                    <div className="num text-[12.5px] text-dim">
                      {r.jitterMs != null ? `±${r.jitterMs.toFixed(1)} ms jitter · ` : ''}
                      {Math.round(r.loss * 100)}% loss
                    </div>
                  </div>
                  <div className={cx('num text-[20px] font-bold', r.avgMs == null ? 'text-bad' : r.avgMs < 40 ? 'text-good' : r.avgMs < 80 ? 'text-accent' : r.avgMs < 130 ? 'text-warn' : 'text-bad')}>{r.avgMs != null ? `${Math.round(r.avgMs)} ms` : '—'}</div>
                </div>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center gap-3 py-10 text-dim">
              <LoaderCircle size={28} className="spin" />
              Pinging from the PC…
            </div>
          ))}
      </Sheet>
    </div>
  );
}
