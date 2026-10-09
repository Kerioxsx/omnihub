// A game's own settings, set for frames per second and low delay:
// Fortnite, VALORANT, Counter-Strike 2, Apex Legends, Overwatch 2, Roblox
// and Minecraft. Shows what the file says now, what Optimize changes, and
// puts your own settings back.

import type { ConfigGame, ConfigOptions, ConfigStatus, FortniteOptions, GameConfigs, GameSession, MinecraftOptions, ProOptions } from '@shared/types';
import { formatRelative } from '@shared/format';
import { AnimatePresence, motion } from 'motion/react';
import { CheckCircle2, ChevronDown, Gauge, Info, Lightbulb, RotateCcw, Sparkles, XCircle } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Segmented, Select, Switch } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useEvent } from '../../lib/hooks';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { Row } from './shared';

const FPS_CAPS = [0, 1000, 540, 480, 360, 240, 165, 144, 120, 60];

type ProGame = 'valorant' | 'cs2' | 'apex' | 'overwatch' | 'roblox';

const SUBTITLE: Record<ConfigGame, string> = {
  fortnite: 'Written into Fortnite’s own settings file — the same values its menu would save.',
  valorant: 'Written into VALORANT’s settings for every account that has played on this PC.',
  cs2: 'Written into cs2_video.txt for every Steam account on this PC.',
  apex: 'Written into Apex Legends’ videoconfig.txt.',
  overwatch: 'Written into Overwatch’s Settings_v0.ini.',
  roblox: 'Written into Roblox’s own settings (the in-game menu’s choices).',
  minecraft: 'Written into Minecraft’s options.txt (Java edition).',
};

/** The switches each game has. */
const PRO_ROWS: Record<ProGame, { key: keyof ProOptions; title: string; hint: string }[]> = {
  valorant: [
    { key: 'uncapped', title: 'No frame limit, VSync off', hint: 'Unlimited frames, no VSync and no dynamic resolution.' },
    { key: 'lowestQuality', title: 'Lowest quality', hint: 'Shadows, effects, anti-aliasing, post-processing and textures at their lowest — what most pros play with.' },
  ],
  cs2: [
    { key: 'uncapped', title: 'VSync off', hint: 'The frame limit itself is the console’s fps_max (see below).' },
    { key: 'lowLatency', title: 'NVIDIA Reflex on', hint: 'Lower input delay on NVIDIA graphics cards; ignored on others.' },
    { key: 'lowestQuality', title: 'Lowest quality', hint: 'Shader detail, shadows, multisampling, textures, particles and ambient occlusion at their lowest.' },
  ],
  apex: [
    { key: 'uncapped', title: 'VSync off, no adaptive resolution', hint: 'The frame limit itself is a launch option (see below).' },
    { key: 'lowestQuality', title: 'Lowest quality', hint: 'Anti-aliasing, shadows, ambient occlusion, volumetric lighting and effects at their lowest.' },
  ],
  overwatch: [
    { key: 'uncapped', title: 'Frame limit 600, VSync off', hint: 'Overwatch’s highest frame limit, with VSync and triple buffering off.' },
    { key: 'lowLatency', title: 'Reduce buffering', hint: 'One frame less queued up: lower input delay.' },
  ],
  roblox: [{ key: 'uncapped', title: 'Maximum frame rate 240', hint: 'The highest frame limit in Roblox’s own menu.' }],
};

const isPro = (g: ConfigGame): g is ProGame => g in PRO_ROWS;

export function GameSettingsCard({ game, beforeLaunch, onBeforeLaunch, locked }: { game: ConfigGame; beforeLaunch: boolean; onBeforeLaunch: (v: boolean) => void; locked?: string }) {
  const [all, setAll] = useState<GameConfigs | null>(null);
  const [busy, setBusy] = useState<'apply' | 'restore' | null>(null);
  const [showChanges, setShowChanges] = useState(false);
  const saving = useRef<ReturnType<typeof setTimeout>>(undefined);

  const load = () => void api.games.configs().then(setAll, (e: unknown) => toast.error('Could not read the game’s settings', errorText(e)));
  useEffect(load, []);
  useEffect(() => () => clearTimeout(saving.current), []);
  // A boost writes the settings before launch: show what it wrote.
  const [stepSeen, setStepSeen] = useState(false);
  useEvent<GameSession | null>('games:session', (s) => {
    const done = !!s?.steps.some((x) => x.id === 'settings');
    if (done && !stepSeen) load();
    setStepSeen(done);
  });

  const status: ConfigStatus | undefined = all?.games.find((g) => g.game === game);
  const label = status?.label ?? game;

  const setOptions = (o: ConfigOptions) => {
    setAll((a) => (a ? { ...a, options: o } : a));
    clearTimeout(saving.current);
    saving.current = setTimeout(() => void api.games.setConfigOptions(o).then(setAll, (e: unknown) => toast.error('Not saved', errorText(e))), 250);
  };
  const fn = (p: Partial<FortniteOptions>) => all && setOptions({ ...all.options, fortnite: { ...all.options.fortnite, ...p } });
  const mc = (p: Partial<MinecraftOptions>) => all && setOptions({ ...all.options, minecraft: { ...all.options.minecraft, ...p } });
  const pro = (g: ProGame, p: Partial<ProOptions>) => all && setOptions({ ...all.options, [g]: { ...all.options[g], ...p } });

  const apply = async () => {
    setBusy('apply');
    try {
      clearTimeout(saving.current);
      if (all) await api.games.setConfigOptions(all.options);
      const n = status?.pending.length ?? 0;
      setAll(await api.games.applyConfig(game));
      toast.success(`${label} is set for the most FPS`, `${n} setting${n === 1 ? '' : 's'} changed. Your own settings are kept, so you can put them back.`);
    } catch (e) {
      toast.error(`Could not change ${label}’s settings`, errorText(e));
    } finally {
      setBusy(null);
    }
  };
  const restore = async () => {
    const ok = await confirm({ title: `Put back your own ${label} settings?`, description: `${label} gets the settings it had before OmniHub first changed them.`, confirmLabel: 'Put back' });
    if (!ok) return;
    setBusy('restore');
    try {
      setAll(await api.games.restoreConfig(game));
      toast.success('Your settings are back');
    } catch (e) {
      toast.error('Could not put them back', errorText(e));
    } finally {
      setBusy(null);
    }
  };

  const optimized = status?.found && status.pending.length === 0;
  const o = all?.options;
  const accounts = status && status.paths.length > 1 ? status.paths.length : 0;
  return (
    <Card className="overflow-hidden">
      <div className="relative px-5 pb-4 pt-4">
        <div className="pointer-events-none absolute inset-0 bg-[radial-gradient(120%_120%_at_0%_0%,var(--accent-soft),transparent_60%)]" aria-hidden />
        <CardHeader
          className="relative"
          icon={Gauge}
          title={`${label} settings for max FPS`}
          subtitle={SUBTITLE[game] + (accounts ? ` (${accounts} accounts)` : '')}
          actions={
            status?.backupAt ? (
              <Button variant="ghost" size="sm" icon={RotateCcw} loading={busy === 'restore'} disabled={!!busy || status.running} onClick={() => void restore()} title={`Your settings from ${formatRelative(status.backupAt)}`}>
                Put back mine
              </Button>
            ) : undefined
          }
        />
        {status && status.found && status.settings.length > 0 && (
          <div className="relative mt-4 grid grid-cols-2 gap-2 sm:grid-cols-3">
            {status.settings.map((s) => (
              <div key={s.label} className={cx('rounded-xl border px-3 py-2', s.good ? 'border-good/25 bg-good/6' : 'border-line bg-surface-2/70')}>
                <div className="text-[11px] font-medium uppercase tracking-[0.06em] text-faint">{s.label}</div>
                <div className={cx('mt-0.5 flex items-center gap-1.5 text-[13.5px] font-semibold', s.good ? 'text-good' : 'text-fg')}>
                  {s.good ? <CheckCircle2 size={13} /> : <XCircle size={13} className="text-warn" />}
                  {s.value}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {status && !status.found && (
        <div className="px-5 pb-4">
          <Callout tone="info" icon={Info} title={`${label} hasn’t saved settings on this PC yet`}>
            Start {label} once, close it, and the settings show up here.
          </Callout>
        </div>
      )}
      {status?.running && (
        <div className="px-5 pb-4">
          <Callout tone="warn" icon={Info} title={`${label} is running`}>
            Close it first — it saves its own settings when it closes and would undo these.
          </Callout>
        </div>
      )}

      {o && (
        <div className="divide-y divide-line border-t border-line">
          {game === 'fortnite' && (
            <>
              <Row title="Frame rate limit" hint="Unlimited lets Fortnite draw as many frames as your PC can. A cap a little above your monitor’s refresh rate gives steadier frame times.">
                <Select value={o.fortnite.frameLimit} onChange={(e) => fn({ frameLimit: Number(e.target.value) })} className="w-36">
                  {FPS_CAPS.map((c) => (
                    <option key={c} value={c}>
                      {c === 0 ? 'Unlimited' : `${c} FPS`}
                    </option>
                  ))}
                </Select>
              </Row>
              <Row title="Performance rendering mode" hint="Fortnite’s lightest renderer, made for high frame rates. The biggest single gain on most PCs.">
                <Switch checked={o.fortnite.performanceMode} onChange={(v) => fn({ performanceMode: v })} label="Performance mode" />
              </Row>
              <Row title="Lowest quality everywhere" hint="Shadows, effects, post-processing, textures, foliage and grass at their lowest — what most pros play with.">
                <Switch checked={o.fortnite.lowestQuality} onChange={(v) => fn({ lowestQuality: v })} label="Lowest quality" />
              </Row>
              {o.fortnite.lowestQuality && (
                <Row title="View distance">
                  <Segmented size="sm" label="View distance" value={String(o.fortnite.viewDistance)} onChange={(v) => fn({ viewDistance: Number(v) })} options={[{ value: '0', label: 'Near' }, { value: '1', label: 'Medium' }, { value: '2', label: 'Far' }, { value: '3', label: 'Epic' }]} />
                </Row>
              )}
              <Row title="3D resolution" hint="Below 100% renders fewer pixels: more FPS, softer picture.">
                <Select value={o.fortnite.resolutionScale} onChange={(e) => fn({ resolutionScale: Number(e.target.value) })} className="w-28">
                  {[100, 90, 85, 75, 67, 50].map((r) => (
                    <option key={r} value={r}>
                      {r}%
                    </option>
                  ))}
                </Select>
              </Row>
              <Row title="Fullscreen" hint="Exclusive fullscreen has the lowest input delay.">
                <Switch checked={o.fortnite.fullscreen} onChange={(v) => fn({ fullscreen: v })} label="Fullscreen" />
              </Row>
              <Row title="Show the FPS counter" hint="Fortnite’s own counter in the top corner, so you can see what you get.">
                <Switch checked={o.fortnite.showFps} onChange={(v) => fn({ showFps: v })} label="FPS counter" />
              </Row>
            </>
          )}
          {game === 'minecraft' && (
            <>
              <Row title="Unlimited frame rate, VSync off">
                <Switch checked={o.minecraft.unlimitedFps} onChange={(v) => mc({ unlimitedFps: v })} label="Unlimited frame rate" />
              </Row>
              <Row title="Fast graphics" hint="Fast graphics, no smooth lighting, entity shadows or mipmaps, and chunk updates off the main thread.">
                <Switch checked={o.minecraft.fastGraphics} onChange={(v) => mc({ fastGraphics: v })} label="Fast graphics" />
              </Row>
              <Row title="Minimal particles">
                <Switch checked={o.minecraft.minimalParticles} onChange={(v) => mc({ minimalParticles: v })} label="Minimal particles" />
              </Row>
              <Row title="No clouds">
                <Switch checked={o.minecraft.noClouds} onChange={(v) => mc({ noClouds: v })} label="No clouds" />
              </Row>
              <Row title="Render distance" hint="Fewer chunks is the biggest FPS gain in Minecraft.">
                <Select value={o.minecraft.renderDistance ?? ''} onChange={(e) => mc({ renderDistance: e.target.value ? Number(e.target.value) : null })} className="w-36">
                  <option value="">Keep mine</option>
                  {[6, 8, 10, 12, 16].map((d) => (
                    <option key={d} value={d}>
                      {d} chunks
                    </option>
                  ))}
                </Select>
              </Row>
              <Row title="Simulation distance" hint="How far the world keeps moving around you; lower is lighter on the processor.">
                <Select value={o.minecraft.simulationDistance ?? ''} onChange={(e) => mc({ simulationDistance: e.target.value ? Number(e.target.value) : null })} className="w-36">
                  <option value="">Keep mine</option>
                  {[5, 6, 8, 10, 12].map((d) => (
                    <option key={d} value={d}>
                      {d} chunks
                    </option>
                  ))}
                </Select>
              </Row>
            </>
          )}
          {isPro(game) &&
            PRO_ROWS[game].map((r) => (
              <Row key={r.key} title={r.title} hint={r.hint}>
                <Switch checked={o[game][r.key]} onChange={(v) => pro(game, { [r.key]: v })} label={r.title} />
              </Row>
            ))}
          <Row title="Before every launch" hint={locked ?? `When you press Play, OmniHub sets these again (${label} sometimes resets them after updates).`}>
            <Switch checked={beforeLaunch} disabled={!!locked} onChange={onBeforeLaunch} label="Apply before every launch" />
          </Row>
        </div>
      )}

      {status?.found && (
        <div className="border-t border-line px-5 py-4">
          <div className="flex flex-wrap items-center gap-3">
            <Button icon={Sparkles} loading={busy === 'apply'} disabled={!!busy || status.running || optimized} onClick={() => void apply()}>
              {optimized ? 'Set for max FPS' : `Optimize ${label}`}
            </Button>
            {status.pending.length > 0 && (
              <button type="button" onClick={() => setShowChanges((v) => !v)} className="inline-flex items-center gap-1 text-[12.5px] font-medium text-dim hover:text-fg">
                {status.pending.length} setting{status.pending.length === 1 ? '' : 's'} to change
                <ChevronDown size={14} className={cx('transition-transform', showChanges && 'rotate-180')} />
              </button>
            )}
            {optimized && (
              <span className="inline-flex items-center gap-1.5 text-[12.5px] font-medium text-good">
                <CheckCircle2 size={14} /> Nothing left to change
              </span>
            )}
          </div>
          <AnimatePresence initial={false}>
            {showChanges && status.pending.length > 0 && (
              <motion.ul initial={{ height: 0, opacity: 0 }} animate={{ height: 'auto', opacity: 1 }} exit={{ height: 0, opacity: 0 }} className="mt-3 grid gap-x-6 gap-y-1 overflow-hidden text-[12.5px] sm:grid-cols-2">
                {status.pending.map((c) => (
                  <li key={c.key} className="flex items-baseline justify-between gap-3 border-b border-line/60 py-1">
                    <span className="text-fg">{c.label}</span>
                    <span className="shrink-0 font-mono text-[11px] text-faint">
                      {c.from ?? '—'} → {c.to}
                    </span>
                  </li>
                ))}
              </motion.ul>
            )}
          </AnimatePresence>
          {status.note && (
            <p className="mt-3 flex items-start gap-2 text-[12px] leading-relaxed text-faint">
              <Lightbulb size={14} className="mt-px shrink-0 text-accent" />
              <span>{status.note} Frame rates top out where your processor and graphics card do; these settings remove everything else in the way.</span>
            </p>
          )}
        </div>
      )}
    </Card>
  );
}
