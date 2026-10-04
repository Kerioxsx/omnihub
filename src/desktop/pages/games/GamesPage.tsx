// Games: a profile per game. "Play" gets the PC out of the game's way,
// starts it, and puts everything back when it closes.

import { formatRelative } from '@shared/format';
import type { GameKind, GameProfile, GameSession, GameStep } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { CheckCircle2, CircleSlash, Gamepad2, LoaderCircle, Plus, Rocket, Square, XCircle, Zap } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { type MenuItem, openMenu } from '../../components/ui/Menu';
import { EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useEvent } from '../../lib/hooks';
import { navigate, useRoute } from '../../lib/router';
import { toast } from '../../state/toasts';
import { ProfileEditor } from './ProfileEditor';
import { GAMES, GameTile, launchSummary } from './shared';

const ORDER: GameKind[] = ['fortnite', 'roblox', 'valorant', 'cs2', 'apex', 'rocketLeague', 'league', 'gta5', 'callOfDuty', 'custom'];

const PHASE: Record<GameSession['phase'], string> = {
  starting: 'Boosting…',
  waiting: 'Waiting for the game to start',
  playing: 'Playing — boosted',
  boosted: 'Boosted — start your game',
  restoring: 'Putting everything back…',
  ended: 'Boost ended',
};

export function GamesPage() {
  const route = useRoute();
  const [profiles, setProfiles] = useState<GameProfile[] | null>(null);
  const [session, setSession] = useState<GameSession | null>(null);
  const selected = route.params.get('id') ?? profiles?.[0]?.id ?? null;

  const load = () =>
    void api.games.list().then(
      (r) => {
        setProfiles(r.profiles);
        setSession(r.session);
      },
      (e: unknown) => toast.error('Could not load game profiles', errorText(e)),
    );
  useEffect(load, []);
  useEvent<GameSession | null>('games:session', (s) => {
    setSession(s);
    if (s?.phase === 'ended') load();
  });

  const add = async (kind: GameKind) => {
    try {
      const p = await api.games.create(kind);
      setProfiles((l) => [...(l ?? []), p]);
      navigate('games', { id: p.id });
    } catch (e) {
      toast.error('Could not add the game', errorText(e));
    }
  };
  const addMenu = (e: React.MouseEvent) => {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const items: MenuItem[] = [{ kind: 'header', label: 'Add a game' }, ...ORDER.map((k): MenuItem => ({ label: GAMES[k].label, hint: GAMES[k].blurb, onSelect: () => void add(k) }))];
    openMenu({ clientX: r.left, clientY: r.bottom + 6, preventDefault: () => undefined }, items);
  };

  const play = async (p: GameProfile, launch: boolean) => {
    try {
      setSession(await api.games.play(p.id, launch));
    } catch (e) {
      toast.error(launch ? `Could not start ${p.name}` : 'Could not boost', errorText(e));
    }
  };

  const active = session && session.phase !== 'ended' ? session : null;
  const current = profiles?.find((p) => p.id === selected) ?? null;

  return (
    <Page
      title="Games"
      subtitle="Boost the PC for a game, start it, and get everything back when you're done."
      actions={
        <Button icon={Plus} onClick={addMenu}>
          Add game
        </Button>
      }
    >
      <AnimatePresence>{session && <SessionCard key="session" session={session} onDismiss={() => setSession(null)} />}</AnimatePresence>

      {profiles && profiles.length === 0 ? (
        <Card className="mt-2 p-8">
          <EmptyState icon={Gamepad2} title="Add your first game" description="Pick a game and OmniHub sets up a boost profile for it: power plan, background apps, GPU, Wi-Fi and more — all put back when you stop playing." />
          <div className="mx-auto mt-6 grid max-w-3xl grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-5">
            {ORDER.map((k) => (
              <button key={k} type="button" onClick={() => void add(k)} className="flex flex-col items-center gap-2 rounded-xl border border-line bg-surface p-3 text-[12.5px] font-medium text-fg transition-colors hover:border-line-strong hover:bg-surface-2">
                <GameTile kind={k} size={48} />
                {GAMES[k].label}
              </button>
            ))}
          </div>
        </Card>
      ) : (
        <div className="mt-2 flex min-h-0 gap-5">
          <div className="sticky top-0 w-64 shrink-0 space-y-1.5 self-start">
            {profiles?.map((p) => {
              const sel = p.id === selected;
              const running = active?.profileId === p.id;
              return (
                <div
                  key={p.id}
                  role="button"
                  tabIndex={0}
                  onClick={() => navigate('games', { id: p.id })}
                  onKeyDown={(e) => e.key === 'Enter' && navigate('games', { id: p.id })}
                  className={cx('group flex cursor-pointer items-center gap-3 rounded-xl border p-2.5 transition-colors', sel ? 'border-accent/50 bg-accent-soft' : 'border-transparent hover:bg-surface')}
                >
                  <GameTile kind={p.kind} size={40} />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[13.5px] font-semibold text-fg">{p.name}</div>
                    <div className="truncate text-[11.5px] text-faint">{running ? <span className="text-accent">{PHASE[active.phase]}</span> : p.lastPlayed ? `Played ${formatRelative(p.lastPlayed)}` : launchSummary(p)}</div>
                  </div>
                  {!active && p.launch.type !== 'none' && (
                    <button type="button" onClick={(e) => (e.stopPropagation(), void play(p, true))} className="grid h-8 w-8 place-items-center rounded-lg text-accent opacity-0 transition-opacity hover:bg-accent/15 focus-visible:opacity-100 group-hover:opacity-100" aria-label={`Play ${p.name}`} title="Play with boost">
                      <Rocket size={16} />
                    </button>
                  )}
                </div>
              );
            })}
          </div>
          <div className="min-w-0 flex-1">
            {current && (
              <ProfileEditor
                key={current.id}
                profile={current}
                busy={!!active}
                activeHere={active?.profileId === current.id}
                onSaved={(p) => setProfiles((l) => l?.map((q) => (q.id === p.id ? p : q)) ?? null)}
                onDeleted={() => {
                  setProfiles((l) => l?.filter((q) => q.id !== current.id) ?? null);
                  navigate('games');
                }}
                onPlay={(launch) => void play(current, launch)}
              />
            )}
          </div>
        </div>
      )}
    </Page>
  );
}

function StepRow({ s }: { s: GameStep }) {
  const Icon = s.status === 'done' ? CheckCircle2 : s.status === 'failed' ? XCircle : CircleSlash;
  return (
    <motion.li initial={{ opacity: 0, x: -6 }} animate={{ opacity: 1, x: 0 }} className="flex items-start gap-2 text-[12.5px]">
      <Icon size={14} className={cx('mt-px shrink-0', s.status === 'done' ? 'text-good' : s.status === 'failed' ? 'text-bad' : 'text-faint')} />
      <span className="shrink-0 font-medium text-fg">{s.label}</span>
      <span className={cx('min-w-0', s.status === 'failed' ? 'text-bad' : 'text-dim')}>{s.detail}</span>
    </motion.li>
  );
}

function SessionCard({ session, onDismiss }: { session: GameSession; onDismiss: () => void }) {
  const [stopping, setStopping] = useState(false);
  const ended = session.phase === 'ended';
  useEffect(() => setStopping(false), [session.phase]);
  const stop = async () => {
    setStopping(true);
    try {
      await api.games.stop();
    } catch (e) {
      toast.error('Could not stop the boost', errorText(e));
      setStopping(false);
    }
  };
  return (
    <motion.div initial={{ opacity: 0, y: -8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -8 }} className="mb-5">
      <Card className={cx('relative overflow-hidden p-5', !ended && 'border-accent/40')}>
        {!ended && <div className="accent-gradient pointer-events-none absolute inset-x-0 top-0 h-0.5" />}
        <div className="flex items-start gap-4">
          <div className={cx('grid h-11 w-11 shrink-0 place-items-center rounded-xl', ended ? 'bg-surface-2 text-dim' : 'bg-accent-soft text-accent')}>
            {session.phase === 'starting' || session.phase === 'restoring' ? <LoaderCircle size={20} className="animate-spin" /> : ended ? <CheckCircle2 size={20} /> : <Zap size={20} />}
          </div>
          <div className="min-w-0 flex-1">
            <div className="font-display text-[16px] font-semibold text-fg">
              {session.name} · {PHASE[session.phase]}
            </div>
            {session.message && <div className="text-[13px] text-dim">{session.message}</div>}
            <div className="mt-3 grid gap-x-8 gap-y-3 lg:grid-cols-2">
              <div>
                <div className="mb-1.5 text-[11px] font-semibold uppercase tracking-[0.08em] text-faint">Boost</div>
                <ul className="space-y-1">
                  {session.steps.map((s) => (
                    <StepRow key={s.id} s={s} />
                  ))}
                  {session.steps.length === 0 && <li className="text-[12.5px] text-faint">Getting started…</li>}
                </ul>
              </div>
              {session.restored.length > 0 && (
                <div>
                  <div className="mb-1.5 text-[11px] font-semibold uppercase tracking-[0.08em] text-faint">Put back</div>
                  <ul className="space-y-1">
                    {session.restored.map((s) => (
                      <StepRow key={s.id} s={s} />
                    ))}
                  </ul>
                </div>
              )}
            </div>
          </div>
          {ended ? (
            <Button variant="ghost" size="sm" onClick={onDismiss}>
              Dismiss
            </Button>
          ) : (
            <Button variant="secondary" icon={Square} loading={stopping || session.phase === 'restoring'} onClick={() => void stop()} title="Ends the boost; the game keeps running">
              Stop boost
            </Button>
          )}
        </div>
      </Card>
    </motion.div>
  );
}
