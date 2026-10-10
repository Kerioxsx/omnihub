// PC-wide settings that decide how well games run, each with what it
// does, where it stands and a one-click change (or undo), what sets this
// PC's ceiling for frames per second, and every supported game's own
// settings in one place.

import type { ConfigStatus, GameConfigs, PcStatus, PcTweak, TweakId } from '@shared/types';
import { motion } from 'motion/react';
import { ArrowRight, CheckCircle2, Cpu, ExternalLink, Gauge, Keyboard, MemoryStick, Monitor, MonitorPlay, MousePointer2, Network, Power, RotateCcw, ShieldAlert, Sparkles, SquareStack, Timer, Trash2, Video, Zap } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, Skeleton } from '../../components/ui/Card';
import { ProgressRing } from '../../components/ui/Progress';
import { cx } from '../../lib/cx';
import { navigate } from '../../lib/router';
import { toast } from '../../state/toasts';
import { GameTile } from './shared';

const ICON: Record<TweakId, typeof Zap> = {
  refreshRate: Monitor,
  powerPlan: Power,
  gameDvr: Video,
  gameMode: Gauge,
  windowedGames: SquareStack,
  gpuScheduling: MonitorPlay,
  preciseTimer: Timer,
  networkThrottling: Network,
  mouseAcceleration: MousePointer2,
  stickyKeys: Keyboard,
  memoryIntegrity: ShieldAlert,
};

const IMPACT = { high: { label: 'Big gain', tone: 'accent' as const }, medium: { label: 'Helps', tone: 'info' as const }, low: { label: 'Small', tone: 'neutral' as const } };

export function PcOptimize() {
  const [status, setStatus] = useState<PcStatus | null>(null);
  const [busy, setBusy] = useState<TweakId | 'all' | null>(null);
  const [startup, setStartup] = useState<number | null>(null);

  useEffect(() => {
    void api.pc.status().then(setStatus, (e: unknown) => toast.error('Could not read the PC’s settings', errorText(e)));
    void api.apps.startup().then((l) => setStartup(l.filter((s) => s.enabled).length), () => undefined);
  }, []);

  const set = async (t: PcTweak, on: boolean) => {
    setBusy(t.id);
    try {
      setStatus(await api.pc.set(t.id, on));
      if (on && t.restart) toast.info(`${t.title}: on after a restart`, 'Windows applies this the next time the PC starts.');
    } catch (e) {
      toast.error(on ? `Could not change “${t.title}”` : 'Could not put it back', errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const todo = status?.tweaks.filter((t) => t.available && !t.optimized && !t.settingsLink) ?? [];
  const optimizeAll = async () => {
    setBusy('all');
    let failed = 0;
    let s = status;
    for (const t of todo) {
      try {
        s = await api.pc.set(t.id, true);
        setStatus(s);
      } catch (e) {
        failed++;
        toast.error(`Could not change “${t.title}”`, errorText(e));
      }
    }
    setBusy(null);
    if (!failed) toast.success('This PC is set up for games', todo.some((t) => t.restart) ? 'Some changes take effect after a restart.' : undefined);
  };

  if (!status) {
    return (
      <div className="space-y-3">
        <Skeleton className="h-44 w-full rounded-2xl" />
        <Skeleton className="h-96 w-full rounded-2xl" />
      </div>
    );
  }
  const available = status.tweaks.filter((t) => t.available);
  const done = available.filter((t) => t.optimized).length;
  const m = status.machine;
  return (
    <div className="space-y-4">
      <Card className="relative overflow-hidden p-5">
        <div className="pointer-events-none absolute -right-24 -top-24 h-72 w-72 rounded-full bg-[radial-gradient(circle,var(--accent-soft),transparent_70%)]" aria-hidden />
        <div className="relative flex flex-wrap items-center gap-6">
          <ProgressRing value={available.length ? done / available.length : 1} size={96} stroke={9} label={`${done} of ${available.length} optimized`}>
            <div className="text-center">
              <div className="font-display text-[22px] font-bold tabular text-fg">
                {done}/{available.length}
              </div>
              <div className="text-[10.5px] uppercase tracking-[0.08em] text-faint">set</div>
            </div>
          </ProgressRing>
          <div className="min-w-0 flex-1">
            <div className="font-display text-[18px] font-semibold text-fg">{todo.length ? `${todo.length} change${todo.length === 1 ? '' : 's'} can make games faster` : 'This PC is set up for games'}</div>
            <p className="mt-1 max-w-2xl text-[13px] leading-relaxed text-dim">Frames per second top out where your processor and graphics card do. These settings take away what Windows puts in the way; each one can be put back. None of them lowers picture quality, so games you play on Ultra stay on Ultra.</p>
            <div className="mt-3 flex flex-wrap gap-2 text-[12px]">
              <Spec icon={Cpu} label={m.cpu || 'Processor'} detail={m.cores ? `${m.cores} cores · ${m.threads} threads` : undefined} />
              {m.gpus.map((g) => (
                <Spec key={g} icon={MonitorPlay} label={g} />
              ))}
              {m.ramGb > 0 && <Spec icon={MemoryStick} label={`${m.ramGb.toFixed(0)} GB memory`} />}
              {m.displays.map((d) => (
                <Spec key={d.name + d.width} icon={Monitor} label={`${d.width}×${d.height} · ${d.hz} Hz`} detail={d.maxHz > d.hz ? `can do ${d.maxHz} Hz` : d.name} warn={d.maxHz > d.hz} />
              ))}
            </div>
          </div>
          {todo.length > 0 && (
            <Button icon={Sparkles} size="lg" loading={busy === 'all'} disabled={!!busy} onClick={() => void optimizeAll()}>
              Optimize everything
            </Button>
          )}
        </div>
      </Card>

      <Card className="divide-y divide-line">
        {status.tweaks.map((t, i) => {
          const Icon = ICON[t.id];
          return (
            <motion.div key={t.id} initial={{ opacity: 0, y: 4 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: i * 0.03 }} className={cx('flex items-start gap-4 px-5 py-4', !t.available && 'opacity-60')}>
              <div className={cx('mt-0.5 grid h-9 w-9 shrink-0 place-items-center rounded-xl', t.optimized ? 'bg-good/12 text-good' : 'bg-surface-2 text-dim')}>{t.optimized ? <CheckCircle2 size={18} /> : <Icon size={18} />}</div>
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="text-[14px] font-semibold text-fg">{t.title}</span>
                  <Badge tone={IMPACT[t.impact].tone}>{IMPACT[t.impact].label}</Badge>
                  {t.admin && <Badge tone="warn">Admin</Badge>}
                  {t.restart && <Badge>Restart</Badge>}
                </div>
                <p className="mt-0.5 text-[12.5px] leading-snug text-faint">{t.description}</p>
                <div className={cx('mt-1 text-[12px] font-medium', t.optimized ? 'text-good' : t.available ? 'text-warn' : 'text-faint')}>Now: {t.current}</div>
              </div>
              <div className="flex shrink-0 items-center gap-1.5 self-center">
                {t.settingsLink ? (
                  <Button variant="secondary" size="sm" icon={ExternalLink} onClick={() => void api.app.openUrl(t.settingsLink as string).catch((e: unknown) => toast.error('Could not open Windows Security', errorText(e)))}>
                    Open Windows Security
                  </Button>
                ) : t.optimized ? (
                  t.canUndo && (
                    <Button variant="ghost" size="sm" icon={RotateCcw} loading={busy === t.id} disabled={!!busy} onClick={() => void set(t, false)}>
                      Undo
                    </Button>
                  )
                ) : (
                  t.available && (
                    <Button size="sm" icon={Zap} loading={busy === t.id} disabled={!!busy} onClick={() => void set(t, true)}>
                      {t.id === 'refreshRate' ? 'Use full rate' : 'Turn on'}
                    </Button>
                  )
                )}
              </div>
            </motion.div>
          );
        })}
      </Card>

      <AllGames />

      <div className="grid gap-3 md:grid-cols-2">
        <Card interactive role="button" tabIndex={0} className="flex cursor-pointer items-center gap-4 p-4" onClick={() => navigate('apps', { filter: 'startup' })} onKeyDown={(e) => e.key === 'Enter' && navigate('apps', { filter: 'startup' })}>
          <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-surface-2 text-dim">
            <Power size={18} />
          </div>
          <div className="min-w-0 flex-1">
            <div className="text-[14px] font-semibold text-fg">Programs that start with Windows</div>
            <div className="text-[12.5px] text-faint">{startup == null ? 'Each one keeps running in the background.' : `${startup} start${startup === 1 ? 's' : ''} with Windows and keep${startup === 1 ? 's' : ''} running in the background.`}</div>
          </div>
          <ArrowRight size={16} className="text-faint" />
        </Card>
        <Card interactive role="button" tabIndex={0} className="flex cursor-pointer items-center gap-4 p-4" onClick={() => navigate('storage', { tab: 'cleanup' })} onKeyDown={(e) => e.key === 'Enter' && navigate('storage', { tab: 'cleanup' })}>
          <div className="grid h-9 w-9 shrink-0 place-items-center rounded-xl bg-surface-2 text-dim">
            <Trash2 size={18} />
          </div>
          <div className="min-w-0 flex-1">
            <div className="text-[14px] font-semibold text-fg">Clean up junk files</div>
            <div className="text-[12.5px] text-faint">Old caches, temporary files and leftovers — room for shader caches and game updates.</div>
          </div>
          <ArrowRight size={16} className="text-faint" />
        </Card>
      </div>
    </div>
  );
}

function Spec({ icon: Icon, label, detail, warn }: { icon: typeof Zap; label: string; detail?: string; warn?: boolean }) {
  return (
    <span className={cx('inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1', warn ? 'border-warn/30 bg-warn/8 text-fg' : 'border-line bg-surface-2/70 text-fg')}>
      <Icon size={13} className={warn ? 'text-warn' : 'text-dim'} />
      <span className="max-w-[260px] truncate font-medium">{label}</span>
      {detail && <span className={warn ? 'text-warn' : 'text-faint'}>· {detail}</span>}
    </span>
  );
}

const KIND: Record<ConfigStatus['game'], Parameters<typeof GameTile>[0]['kind']> = { fortnite: 'fortnite', valorant: 'valorant', cs2: 'cs2', apex: 'apex', overwatch: 'overwatch', roblox: 'roblox', minecraft: 'minecraft' };

/** Every supported game's own settings: what's left to change, and one button for all. */
function AllGames() {
  const [all, setAll] = useState<GameConfigs | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  useEffect(() => {
    void api.games.configs().then(setAll, () => undefined);
  }, []);
  if (!all) return null;
  const found = all.games.filter((g) => g.found);
  const todo = found.filter((g) => g.pending.length > 0 && !g.running);
  const apply = async (games: ConfigStatus[]) => {
    setBusy(games.length === 1 ? games[0].game : 'all');
    let failed = 0;
    for (const g of games) {
      try {
        setAll(await api.games.applyConfig(g.game));
      } catch (e) {
        failed++;
        toast.error(`Could not change ${g.label}’s settings`, errorText(e));
      }
    }
    setBusy(null);
    if (!failed) toast.success(games.length === 1 ? `${games[0].label} is set for the most FPS` : 'Every game is set for the most FPS', 'Your own settings are kept — “Put back mine” on each game’s page.');
  };
  return (
    <Card>
      <div className="flex flex-wrap items-center gap-3 px-5 pt-4">
        <div className="min-w-0 flex-1">
          <div className="font-display text-[15px] font-semibold text-fg">Each game’s own settings</div>
          <div className="text-[12.5px] text-faint">The fastest in-game graphics for competitive games. Leave out the ones you play for the looks — or use Quality mode for them.</div>
        </div>
        {todo.length > 1 && (
          <Button icon={Sparkles} loading={busy === 'all'} disabled={!!busy} onClick={() => void apply(todo)}>
            Optimize all {todo.length}
          </Button>
        )}
      </div>
      <div className="grid gap-2 p-5 sm:grid-cols-2 xl:grid-cols-3">
        {all.games.map((g) => (
          <div key={g.game} className={cx('flex items-center gap-3 rounded-xl border p-2.5', g.found ? 'border-line bg-surface-2/50' : 'border-dashed border-line opacity-60')}>
            <GameTile kind={KIND[g.game]} size={34} />
            <div className="min-w-0 flex-1">
              <div className="truncate text-[13px] font-semibold text-fg">{g.label}</div>
              <div className={cx('truncate text-[11.5px]', !g.found ? 'text-faint' : g.pending.length ? 'text-warn' : 'text-good')}>
                {!g.found ? 'Not played on this PC yet' : g.running ? 'Running — close it first' : g.pending.length ? `${g.pending.length} setting${g.pending.length === 1 ? '' : 's'} to change` : 'Set for max FPS'}
              </div>
            </div>
            {g.found && g.pending.length > 0 && (
              <Button size="sm" variant="secondary" loading={busy === g.game} disabled={!!busy || g.running} onClick={() => void apply([g])}>
                Optimize
              </Button>
            )}
            {g.found && !g.pending.length && <CheckCircle2 size={16} className="shrink-0 text-good" />}
          </div>
        ))}
      </div>
    </Card>
  );
}
