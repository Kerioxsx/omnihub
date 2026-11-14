// A game's own settings, set for frames per second and low delay:
// Fortnite, VALORANT, Counter-Strike 2, Apex Legends, Overwatch 2, Roblox
// and Minecraft. Shows what the file says now, what Optimize changes, and
// puts your own settings back.

import type { ConfigGame, ConfigOptions, ConfigStatus, FortniteAdvice, FortniteOptions, FortniteRenderMode, GameConfigs, GameSession, GpuDriver, MinecraftOptions, NetCheck, ProOptions, Replays } from '@shared/types';
import { formatBytes, formatRelative } from '@shared/format';
import { AnimatePresence, motion } from 'motion/react';
import { CheckCircle2, ChevronDown, Eraser, ExternalLink, Film, Gauge, Info, Lightbulb, MonitorCog, RotateCcw, Sparkles, XCircle, Zap, Cpu, Sun } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
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
  roblox: [
    { key: 'uncapped', title: 'Maximum frame rate 240', hint: 'The highest frame limit in Roblox’s own menu.' },
    { key: 'lowestQuality', title: 'Lowest graphics quality', hint: 'Roblox’s graphics slider at 1 instead of Automatic, which picks near the top on a fast PC: shorter view distance, simpler shadows and details.' },
  ],
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
            status?.backupAt || status?.locked ? (
              <div className="flex items-center gap-2">
                {status?.locked && <Badge tone="warn">File locked</Badge>}
                {status?.backupAt ? (
                  <Button variant="ghost" size="sm" icon={RotateCcw} loading={busy === 'restore'} disabled={!!busy || status.running} onClick={() => void restore()} title={`Your settings from ${formatRelative(status.backupAt)}`}>
                    Put back mine
                  </Button>
                ) : null}
              </div>
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
              <Row title="Preset" hint="Clean & bright: the bright, flat pastel look from the clips (Performance mode, everything low, sharp 100% 3D resolution, brightness up). Potato: the blurry low-poly look from the clips, for weak PCs (Performance mode, everything lowest, near view, 3D resolution as low as 10%). Competitive: the most frames (Performance mode, lowest quality, no cap). Balanced: DirectX 12 at medium quality. Pretty: DirectX 12 at epic quality. Press Apply after; your own settings stay backed up.">
                <div className="flex flex-wrap gap-1.5">
                  {PRESETS.map((p) => (
                    <Button key={p.id} size="sm" variant="secondary" icon={p.icon} onClick={() => fn(p.set)}>
                      {p.label}
                    </Button>
                  ))}
                </div>
              </Row>
              <Row title="Rendering mode" hint="Performance is Fortnite’s lightest renderer, made for high frame rates: the biggest single gain on most PCs. DirectX 12 can be smoother on fast graphics cards; DirectX 11 suits older ones.">
                <Segmented
                  size="sm"
                  label="Rendering mode"
                  value={o.fortnite.renderMode ?? (o.fortnite.performanceMode ? 'performance' : 'keep')}
                  onChange={(v) => fn({ renderMode: v as FortniteRenderMode, performanceMode: v === 'performance' })}
                  options={[{ value: 'performance', label: 'Performance' }, { value: 'dx12', label: 'DirectX 12' }, { value: 'dx11', label: 'DirectX 11' }, { value: 'keep', label: 'Keep' }]}
                />
              </Row>
              <Row title="Lowest quality everywhere" hint="Shadows, effects, post-processing, textures, foliage and grass at their lowest, and DirectX 12’s Nanite, Lumen and anti-aliasing off — what most pros play with.">
                <Switch checked={o.fortnite.lowestQuality} onChange={(v) => fn({ lowestQuality: v, quality: v ? null : o.fortnite.quality })} label="Lowest quality" />
              </Row>
              {!o.fortnite.lowestQuality && (
                <Row title="Quality" hint="Every quality setting at one level. Keep leaves Fortnite’s own choices alone.">
                  <Segmented size="sm" label="Quality" value={String(o.fortnite.quality ?? 0)} onChange={(v) => fn({ quality: Number(v) || null })} options={[{ value: '0', label: 'Keep' }, { value: '1', label: 'Medium' }, { value: '2', label: 'High' }, { value: '3', label: 'Epic' }]} />
                </Row>
              )}
              {(o.fortnite.lowestQuality || !!o.fortnite.quality) && (
                <Row title="View distance">
                  <Segmented size="sm" label="View distance" value={String(o.fortnite.viewDistance)} onChange={(v) => fn({ viewDistance: Number(v) })} options={[{ value: '0', label: 'Near' }, { value: '1', label: 'Medium' }, { value: '2', label: 'Far' }, { value: '3', label: 'Epic' }]} />
                </Row>
              )}
              {o.fortnite.potato && (
                <Row title="Potato graphics" hint="On: Performance mode, everything at its lowest, near view distance and a 3D resolution below Fortnite’s 25% minimum. Fortnite may raise a value it doesn’t accept. Turn off for your other choices.">
                  <Switch checked={!!o.fortnite.potato} onChange={(v) => fn({ potato: v })} label="Potato graphics" />
                </Row>
              )}
              <PotatoTextures />
              <Row title="Brightness" hint="Fortnite’s Brightness (100% is its default). Higher gives the light, washed pastel look. Saved as DisplayGamma in its settings; Keep leaves yours.">
                <Select value={o.fortnite.brightness ?? 0} onChange={(e) => fn({ brightness: Number(e.target.value) })} className="w-28">
                  {[0, 100, 110, 120, 130, 140, 150, 160].map((b) => (
                    <option key={b} value={b}>
                      {b === 0 ? 'Keep' : `${b}%`}
                    </option>
                  ))}
                </Select>
              </Row>
              <Row title="Lock the settings file" hint="Makes GameUserSettings.ini read-only after Apply so Fortnite can’t put its own values back (what the potato guides do). While locked, Fortnite’s menu can’t change video settings or switch Performance mode; untick and Apply, or Restore, to unlock.">
                <Switch checked={!!o.fortnite.lockFile} onChange={(v) => fn({ lockFile: v })} label="Lock settings file" />
              </Row>
              <ResolutionAdvice o={o.fortnite} set={fn} />
              <Row title="3D resolution" hint="Below 100% renders fewer pixels: more FPS, softer picture.">
                <Select value={o.fortnite.resolutionScale} onChange={(e) => fn({ resolutionScale: Number(e.target.value) })} className="w-28">
                  {(o.fortnite.potato ? [100, 75, 50, 40, 33, 25, 20, 15, 10] : [100, 90, 85, 80, 75, 67, 50, 25]).map((r) => (
                    <option key={r} value={r}>
                      {r}%
                    </option>
                  ))}
                </Select>
              </Row>
              <Row title="NVIDIA Reflex Low Latency" hint="Lower input delay on NVIDIA graphics cards. Boost also keeps the card at full clocks when the processor holds it back. Set once Fortnite has saved it.">
                <Segmented size="sm" label="NVIDIA Reflex" value={String(o.fortnite.reflex)} onChange={(v) => fn({ reflex: Number(v) })} options={[{ value: '0', label: 'Off' }, { value: '1', label: 'On' }, { value: '2', label: 'On + Boost' }]} />
              </Row>
              <Row title="Fullscreen" hint="Exclusive fullscreen has the lowest input delay.">
                <Switch checked={o.fortnite.fullscreen} onChange={(v) => fn({ fullscreen: v })} label="Fullscreen" />
              </Row>
              <Row title="Show the FPS counter" hint="Fortnite’s own counter in the top corner, so you can see what you get.">
                <Switch checked={o.fortnite.showFps} onChange={(v) => fn({ showFps: v })} label="FPS counter" />
              </Row>
              <Row title="Clean Fortnite caches" hint="Empties Fortnite’s logs, crash reports and web cache, and your graphics driver’s shader caches — old, bloated caches cause stutter. The first match after rebuilds its shaders, so it may hitch for a minute.">
                <CleanCaches />
              </Row>
              <DriverRow />
              <ReplaysRow />
              <WifiRow />
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

function CleanCaches() {
  const [busy, setBusy] = useState(false);
  const run = async () => {
    setBusy(true);
    try {
      const r = await api.games.fortniteCleanCaches();
      toast.success(r.freedBytes > 0 ? `Freed ${formatBytes(r.freedBytes)}` : 'Already clean', `${r.files.toLocaleString()} files removed${r.skipped ? `, ${r.skipped} in use left alone` : ''}.`);
    } catch (e) {
      toast.error('Couldn’t clean the caches', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Button size="sm" variant="secondary" icon={Eraser} loading={busy} onClick={() => void run()}>
      Clean
    </Button>
  );
}

const PRESETS: { id: string; label: string; icon: typeof Zap; set: Partial<FortniteOptions> }[] = [
  { id: 'bright', label: 'Clean & bright', icon: Sun, set: { potato: false, frameLimit: 0, renderMode: 'performance', performanceMode: true, lowestQuality: true, quality: null, viewDistance: 0, fullscreen: true, reflex: 2, showFps: true, resolutionScale: 100, brightness: 140 } },
  { id: 'potato', label: 'Potato', icon: Cpu, set: { potato: true, brightness: 0, frameLimit: 0, renderMode: 'performance', performanceMode: true, lowestQuality: true, quality: null, viewDistance: 0, fullscreen: true, reflex: 2, showFps: true, resolutionScale: 25 } },
  { id: 'competitive', label: 'Competitive', icon: Zap, set: { potato: false, brightness: 0, frameLimit: 0, renderMode: 'performance', performanceMode: true, lowestQuality: true, quality: null, viewDistance: 0, fullscreen: true, reflex: 2, showFps: true, resolutionScale: 100 } },
  { id: 'balanced', label: 'Balanced', icon: Gauge, set: { potato: false, lockFile: false, renderMode: 'dx12', performanceMode: false, lowestQuality: false, quality: 1, viewDistance: 2, fullscreen: true, reflex: 2, resolutionScale: 100 } },
  { id: 'pretty', label: 'Pretty', icon: Sparkles, set: { potato: false, lockFile: false, frameLimit: 0, renderMode: 'dx12', performanceMode: false, lowestQuality: false, quality: 3, viewDistance: 3, fullscreen: true, reflex: 1, resolutionScale: 100 } },
];

/** The NVIDIA driver's texture LOD bias on Fortnite's profile: the flat "potato textures" look from the clips. */
function PotatoTextures() {
  const [v, setV] = useState<number | null | undefined>(undefined);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    api.games.potatoTextures().then(setV, () => setV(null));
  }, []);
  const set = async (n: number) => {
    setBusy(true);
    try {
      setV(await api.games.potatoTextures(n));
    } catch (e) {
      toast.error('Couldn’t change the driver setting', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  if (v === undefined) return null;
  return (
    <Row
      title="Potato textures (NVIDIA)"
      hint={
        v === null
          ? 'The flat, untextured look from the potato clips is an NVIDIA driver setting (texture LOD bias), so it needs an NVIDIA card. On AMD or Intel, the closest you get is removing Fortnite’s High Resolution Textures in the Epic Games Launcher (Fortnite → Options).'
          : 'Raises the NVIDIA driver’s texture LOD bias for Fortnite only, the setting the potato apps change: Max turns almost every surface into flat colour. Windows asks for administrator rights. Restart Fortnite to see it; Off gives it back to the driver.'
      }
    >
      {v === null ? (
        <span className="text-[12.5px] text-faint">Needs an NVIDIA card</span>
      ) : (
        <Segmented size="sm" label="Potato textures" value={String([0, 16, 64, 120].includes(v) ? v : v > 0 ? 64 : 0)} onChange={(x) => void set(Number(x))} options={[{ value: '0', label: 'Off', disabled: busy }, { value: '16', label: 'Light', disabled: busy }, { value: '64', label: 'Strong', disabled: busy }, { value: '120', label: 'Max', disabled: busy }]} />
      )}
    </Row>
  );
}

/** How old the graphics driver is; after it changes, clean the shader caches. */
/** Display resolution, 3D resolution and frame cap for this monitor and these specs. */
function ResolutionAdvice({ o, set }: { o: FortniteOptions; set: (p: Partial<FortniteOptions>) => void }) {
  const [a, setA] = useState<FortniteAdvice | null>(null);
  const [why, setWhy] = useState(false);
  useEffect(() => {
    void api.games.fortniteAdvice().then(setA, () => setA(null));
  }, []);
  const res = o.width && o.height ? `${o.width}x${o.height}` : '';
  const options = a ? a.resolutions : [];
  const using = !!a && (o.width || a.nativeWidth) === a.width && (o.height || a.nativeHeight) === a.height && o.resolutionScale === a.resolutionScale && o.frameLimit === a.frameLimit;
  return (
    <>
      {a && (
        <div className="px-5 py-4">
          <div className="rounded-[var(--r-md,14px)] border border-line bg-surface-2 p-4">
            <div className="flex flex-wrap items-start gap-3">
              <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-accent-soft text-accent">
                <MonitorCog size={20} aria-hidden />
              </div>
              <div className="min-w-0 flex-1">
                <div className="text-[14px] font-semibold text-fg">Recommended for your PC</div>
                <div className="mt-0.5 text-[12.5px] text-dim">
                  {a.gpu ? `${a.gpu}${a.vramGb ? ` (${a.vramGb} GB)` : ''} · ${a.tierLabel}` : 'Graphics card unknown'} · monitor {a.nativeWidth}×{a.nativeHeight} at {a.hz} Hz
                </div>
                <div className="mt-2.5 flex flex-wrap gap-1.5">
                  <Badge tone="accent">
                    {a.width}×{a.height}
                  </Badge>
                  <Badge tone="accent">3D {a.resolutionScale}%</Badge>
                  <Badge tone="accent">{a.frameLimit ? `Cap ${a.frameLimit} FPS` : 'No cap'}</Badge>
                  <Badge tone="good">≈ {a.expectedFps} FPS</Badge>
                </div>
              </div>
              <Button
                size="sm"
                variant={using ? 'secondary' : 'primary'}
                icon={using ? CheckCircle2 : Zap}
                disabled={using}
                onClick={() =>
                  set({
                    width: a.width === a.nativeWidth && a.height === a.nativeHeight ? 0 : a.width,
                    height: a.width === a.nativeWidth && a.height === a.nativeHeight ? 0 : a.height,
                    resolutionScale: a.resolutionScale,
                    frameLimit: a.frameLimit,
                    fullscreen: true,
                    renderMode: 'performance',
                    performanceMode: true,
                    lowestQuality: a.lowestQuality,
                    quality: a.lowestQuality ? null : 1,
                  })
                }
              >
                {using ? 'Chosen' : 'Use this'}
              </Button>
            </div>
            <button type="button" onClick={() => setWhy(!why)} className="mt-3 flex items-center gap-1 text-[12.5px] font-medium text-dim hover:text-fg" aria-expanded={why}>
              <ChevronDown size={14} className={cx('transition-transform', why && 'rotate-180')} aria-hidden /> Why
            </button>
            {why && (
              <ul className="mt-2 space-y-1 text-[12.5px] leading-relaxed text-dim">
                {a.reasons.map((r) => (
                  <li key={r} className="flex gap-2">
                    <span className="mt-[7px] h-1 w-1 shrink-0 rounded-full bg-faint" />
                    {r}
                  </li>
                ))}
                <li className="flex gap-2">
                  <span className="mt-[7px] h-1 w-1 shrink-0 rounded-full bg-faint" />A rough estimate: check the FPS counter in a match and move the 3D resolution up or down from there.
                </li>
              </ul>
            )}
          </div>
        </div>
      )}
      <Row title="Display resolution" hint="Fewer pixels for the whole screen: the most FPS, but menus and text get softer too. Only works in Fullscreen; windowed fullscreen uses your desktop’s resolution.">
        <Select
          value={res}
          onChange={(e) => {
            const [w, h] = e.target.value.split('x').map(Number);
            set(e.target.value ? { width: w, height: h, fullscreen: true } : { width: 0, height: 0 });
          }}
          className="w-40"
        >
          <option value="">Keep the game’s</option>
          {options.map(([w, h]) => (
            <option key={`${w}x${h}`} value={`${w}x${h}`}>
              {w}×{h}
              {a && w === a.nativeWidth && h === a.nativeHeight ? ' (native)' : ''}
            </option>
          ))}
          {res && !options.some(([w, h]) => `${w}x${h}` === res) && <option value={res}>{res.replace('x', '×')}</option>}
        </Select>
      </Row>
    </>
  );
}

function DriverRow() {
  const [d, setD] = useState<GpuDriver | null | undefined>(undefined);
  useEffect(() => {
    void api.games.gpuDrivers().then((l) => setD(l[0] ?? null), () => setD(null));
  }, []);
  if (!d) return null;
  const old = (d.daysOld ?? 0) > 180;
  return (
    <Row
      title="Graphics driver"
      hint={
        <>
          {d.name} · {d.version}
          {d.daysOld != null && ` · ${d.daysOld} days old`}
          {d.changed && ' · Changed since the caches were last cleaned: clean them above so old shaders don’t cause stutter.'}
        </>
      }
    >
      <div className="flex items-center gap-2">
        {d.changed && <Badge tone="warn">Clean caches</Badge>}
        {old && <Badge tone="warn">Old</Badge>}
        {d.updateUrl && (
          <Button size="sm" variant="secondary" icon={ExternalLink} onClick={() => void api.app.openUrl(d.updateUrl!)}>
            Check for updates
          </Button>
        )}
      </div>
    </Row>
  );
}

/** Fortnite's saved replays pile up: send old ones to the Recycle Bin. */
function ReplaysRow() {
  const [r, setR] = useState<Replays | null>(null);
  const [busy, setBusy] = useState(false);
  const load = () => void api.games.fortniteReplays().then(setR, () => setR(null));
  useEffect(load, []);
  if (!r || !r.list.length) return null;
  const clean = async () => {
    setBusy(true);
    try {
      const [n, bytes] = await api.games.fortniteCleanReplays(5, 7);
      toast.success(n ? `${n} replay${n === 1 ? '' : 's'} sent to the Recycle Bin` : 'Nothing old enough', n ? `${formatBytes(bytes)} freed. The 5 newest are kept.` : 'Replays from the last week and the 5 newest are kept.');
      load();
    } catch (e) {
      toast.error('Couldn’t clean the replays', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Row title="Replays" hint={`${r.list.length} replay${r.list.length === 1 ? '' : 's'}, ${formatBytes(r.bytes)}. Cleaning keeps the 5 newest and anything from the last week; the rest go to the Recycle Bin.`}>
      <Button size="sm" variant="secondary" icon={Film} loading={busy} onClick={() => void clean()}>
        Clean old
      </Button>
    </Row>
  );
}

/** Wi-Fi power saving adds lag spikes; and a cable beats Wi-Fi. */
function WifiRow() {
  const [n, setN] = useState<NetCheck | null>(null);
  useEffect(() => {
    void api.games.netCheck().then(setN, () => setN(null));
  }, []);
  if (!n || (n.onWifi === null && n.wifiSaving === null)) return null;
  const saving = n.wifiSaving != null && n.wifiSaving > 0;
  const set = (off: boolean) => void api.games.setWifiSaving(off).then(setN, (e: unknown) => toast.error('Couldn’t change it', errorText(e)));
  return (
    <Row
      title="Wi-Fi power saving off"
      hint={
        n.onWifi
          ? `You’re on Wi-Fi${saving ? ', and its power saving is on — it naps between packets, which shows up as ping spikes' : ''}. A network cable is steadier still.`
          : 'You’re on a cable — the best for ping. This only matters on Wi-Fi.'
      }
    >
      <Switch checked={n.wifiSaving === 0} onChange={(v) => set(v)} label="Wi-Fi power saving off" disabled={n.wifiSaving == null} />
    </Row>
  );
}
