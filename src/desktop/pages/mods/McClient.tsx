// OmniHub Client: Minecraft 1.8.9 for PvP in the official launcher — Forge,
// a curated set of the best open-source mods (hitboxes, crosshair, hit
// colour, 1.7 animations, FPS fixes, in-game resource pack browser, HUD),
// OmniHub's own kill effects, and resource packs.

import type { KillEffect, KillFx, McClientStatus, McPackSearchHit } from '@shared/types';
import { formatRelative } from '@shared/format';
import { Check, Download, Flame, Heart, PackagePlus, Skull, Sparkles, Swords, Trash2, Wind, Zap } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Spinner } from '../../components/ui/Card';
import { SearchInput, Segmented, Select, Switch } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useDebounced } from '../../lib/hooks';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { ProgressLine, useModProgress } from './shared';

const GROUPS: { id: string; label: string }[] = [
  { id: 'visuals', label: 'PvP visuals' },
  { id: 'hud', label: 'HUD' },
  { id: 'performance', label: 'Performance' },
  { id: 'packs', label: 'Resource packs' },
  { id: 'movement', label: 'Movement' },
];

const EFFECTS: { id: KillEffect; label: string; icon: LucideIcon; what: string }[] = [
  { id: 'blood', label: 'Blood', icon: Swords, what: 'A burst of red, block-crack style' },
  { id: 'lightning', label: 'Lightning', icon: Zap, what: 'A bolt strikes where they fell' },
  { id: 'flames', label: 'Flames', icon: Flame, what: 'A rising ring of fire' },
  { id: 'firework', label: 'Firework', icon: Sparkles, what: 'A burst in your colour' },
  { id: 'souls', label: 'Souls', icon: Wind, what: 'Spirits spiral up out of the body' },
  { id: 'hearts', label: 'Hearts', icon: Heart, what: 'A shower of hearts' },
  { id: 'explosion', label: 'Explosion', icon: Skull, what: 'A blast with smoke' },
  { id: 'random', label: 'Random', icon: Sparkles, what: 'A different one each kill' },
];

const SOUNDS: { value: string; label: string }[] = [
  { value: 'default', label: 'The effect’s own' },
  { value: 'none', label: 'No sound' },
  { value: 'levelup', label: 'Level up' },
  { value: 'orb', label: 'XP orb' },
  { value: 'ding', label: 'Ding' },
  { value: 'anvil', label: 'Anvil' },
  { value: 'explode', label: 'Explosion' },
  { value: 'thunder', label: 'Thunder' },
  { value: 'wither', label: 'Wither' },
];

export function McClientCard() {
  const [st, setSt] = useState<McClientStatus | null>(null);
  const [chosen, setChosen] = useState<string[] | null>(null);
  const [killfx, setKillfx] = useState(false);
  const [vesper, setVesper] = useState(true);
  const [busy, setBusy] = useState(false);
  const progress = useModProgress('mcclient');

  const load = useCallback(
    () =>
      void api.minecraft.client.status().then((s) => {
        setSt(s);
        setChosen((c) => c ?? (s.installed ? s.mods.filter((m) => m.installed).map((m) => m.slug) : s.mods.filter((m) => m.defaultOn).map((m) => m.slug)));
        if (s.installed) {
          setKillfx(s.killfxInstalled);
          setVesper(s.vesperInstalled);
        }
      }, () => setSt(null)),
    [],
  );
  useEffect(load, [load]);
  if (!st || !chosen) return null;

  const changed = st.installed && (st.killfxInstalled !== killfx || st.vesperInstalled !== vesper || st.mods.some((m) => m.installed !== chosen.includes(m.slug)));
  const install = async () => {
    setBusy(true);
    try {
      const s = await api.minecraft.client.install(chosen, killfx, vesper);
      setSt(s);
      toast.success(st.installed ? 'Client updated' : 'OmniHub Client is in the Minecraft Launcher', 'Pick “OmniHub Client 1.8.9” next to Play. Mod settings: Right Shift in-game (OneConfig).');
    } catch (e) {
      toast.error('Not installed', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const remove = async () => {
    if (!(await confirm({ title: 'Remove OmniHub Client?', description: 'Its launcher profile and its folder (mods, settings, resource packs, screenshots) go. Forge’s shared files stay.', confirmLabel: 'Remove', tone: 'danger' }))) return;
    try {
      await api.minecraft.client.uninstall();
      load();
    } catch (e) {
      toast.error('Not removed', errorText(e));
    }
  };
  const toggle = (slug: string, on: boolean) => setChosen(on ? [...chosen, slug] : chosen.filter((s) => s !== slug));

  return (
    <Card className="overflow-hidden">
      <div className="relative border-b border-line bg-gradient-to-br from-accent-soft via-transparent to-transparent px-5 py-5">
        <div className="flex flex-wrap items-start gap-4">
          <div className="grid h-12 w-12 shrink-0 place-items-center rounded-2xl bg-accent text-white shadow-lg shadow-accent/30">
            <Swords size={24} aria-hidden />
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="font-display text-[19px] font-bold text-fg">OmniHub Client</h2>
              <Badge tone="accent">1.8.9 PvP</Badge>
              {st.installed && (
                <Badge tone="good" icon={Check}>
                  Installed {st.installedAt ? formatRelative(st.installedAt) : ''}
                </Badge>
              )}
            </div>
            <p className="mt-1 max-w-[62ch] text-[13px] leading-relaxed text-dim">
              Forge 1.8.9 with the best open-source PvP mods, your own kill effects and resource packs — as a profile in the official Minecraft Launcher, so you sign in with Microsoft as usual and can play on Hypixel and any server.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            {st.installed && <Button size="sm" variant="ghost" icon={Trash2} onClick={() => void remove()} aria-label="Remove OmniHub Client" />}
            <Button variant="primary" icon={Download} loading={busy} disabled={busy || !st.minecraftFound || (st.installed && !changed)} onClick={() => void install()}>
              {st.installed ? (changed ? 'Update' : 'Up to date') : 'Install'}
            </Button>
          </div>
        </div>
        {!st.minecraftFound && (
          <div className="mt-3">
            <Callout tone="info">Minecraft (Java Edition) wasn’t found. Install the Minecraft Launcher and open it once, then come back.</Callout>
          </div>
        )}
        {busy && progress && (
          <div className="mt-3">
            <ProgressLine p={progress} />
          </div>
        )}
      </div>

      <div className="grid gap-px bg-line lg:grid-cols-2">
        <div className="bg-surface px-5 py-4">
          <div className="mb-2 flex items-center justify-between">
            <div className="text-[13.5px] font-semibold text-fg">Mods</div>
            <span className="text-[12px] text-faint">{chosen.length} chosen · from Modrinth</span>
          </div>
          <div className="space-y-3">
            {GROUPS.map((g) => {
              const list = st.mods.filter((m) => m.group === g.id);
              if (!list.length) return null;
              return (
                <div key={g.id}>
                  <div className="mb-1 text-[11px] font-semibold uppercase tracking-[0.08em] text-faint">{g.label}</div>
                  <div className="space-y-1">
                    {list.map((m) => (
                      <label key={m.slug} className="flex cursor-pointer items-start gap-3 rounded-xl px-2 py-1.5 hover:bg-surface-2">
                        <input type="checkbox" className="mt-1 h-4 w-4 accent-[var(--accent)]" checked={chosen.includes(m.slug)} onChange={(e) => toggle(m.slug, e.target.checked)} />
                        <span className="min-w-0 flex-1">
                          <span className="flex items-center gap-1.5 text-[13.5px] font-medium text-fg">
                            {m.name}
                            {m.installed && <Check size={13} className="text-good" aria-label="installed" />}
                          </span>
                          <span className="block text-[12px] leading-snug text-dim">{m.what}</span>
                        </span>
                      </label>
                    ))}
                  </div>
                </div>
              );
            })}
            <label className="flex cursor-pointer items-start gap-3 rounded-xl px-2 py-1.5 hover:bg-surface-2">
              <input type="checkbox" className="mt-1 h-4 w-4 accent-[var(--accent)]" checked={vesper} onChange={(e) => setVesper(e.target.checked)} />
              <span className="min-w-0 flex-1">
                <span className="flex items-center gap-1.5 text-[13.5px] font-medium text-fg">
                  Vesper Client <Badge tone="accent">ours</Badge>
                </span>
                <span className="block text-[12px] leading-snug text-dim">Our own PvP client: an animated title and menus, celestial wings and other cosmetics, five kill effects, a HUD you can drag around, a showroom and FPS savers. Its settings are in-game (Client on the title screen).</span>
              </span>
            </label>
            <label className="flex cursor-pointer items-start gap-3 rounded-xl px-2 py-1.5 hover:bg-surface-2">
              <input type="checkbox" className="mt-1 h-4 w-4 accent-[var(--accent)]" checked={killfx} onChange={(e) => setKillfx(e.target.checked)} />
              <span className="min-w-0 flex-1">
                <span className="flex items-center gap-1.5 text-[13.5px] font-medium text-fg">
                  OmniHub Kill Effects <Badge tone="accent">ours</Badge>
                </span>
                <span className="block text-[12px] leading-snug text-dim">The small kill-effects mod on its own (Vesper already has kill effects). Set it up on the right; changes apply in-game within seconds.</span>
              </span>
            </label>
          </div>
        </div>
        <div className="space-y-px bg-line">
          <KillFxEditor value={st.killfx} disabled={!st.installed || !st.killfxInstalled} onSaved={(k) => setSt({ ...st, killfx: k })} />
          <Packs st={st} onChange={load} />
        </div>
      </div>
    </Card>
  );
}

function KillFxEditor({ value, disabled, onSaved }: { value: KillFx; disabled: boolean; onSaved: (k: KillFx) => void }) {
  const [k, setK] = useState(value);
  const first = useRef(true);
  const debounced = useDebounced(k, 400);
  useEffect(() => setK(value), [value]);
  useEffect(() => {
    if (first.current) {
      first.current = false;
      return;
    }
    if (disabled || JSON.stringify(debounced) === JSON.stringify(value)) return;
    void api.minecraft.client.setKillfx(debounced).then(onSaved, (e) => toast.error('Not saved', errorText(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [debounced]);
  const set = (p: Partial<KillFx>) => setK({ ...k, ...p });
  return (
    <div className={cx('bg-surface px-5 py-4', disabled && 'opacity-60')}>
      <div className="mb-2 flex items-center justify-between gap-2">
        <div className="text-[13.5px] font-semibold text-fg">Kill effects</div>
        <Switch checked={k.enabled} onChange={(v) => set({ enabled: v })} label="Kill effects on" disabled={disabled} />
      </div>
      {disabled && <p className="mb-2 text-[12px] text-faint">Install the client with OmniHub Kill Effects to set these.</p>}
      <div className="grid grid-cols-4 gap-1.5">
        {EFFECTS.map((e) => {
          const on = k.effect === e.id;
          const Icon = e.icon;
          return (
            <button key={e.id} type="button" disabled={disabled} title={e.what} aria-pressed={on} onClick={() => set({ effect: e.id })} className={cx('flex flex-col items-center gap-1 rounded-xl border px-1 py-2 text-[11.5px] font-medium transition-colors', on ? 'border-accent bg-accent-soft text-accent' : 'border-line text-dim hover:border-line-strong hover:text-fg')}>
              <Icon size={17} aria-hidden />
              {e.label}
            </button>
          );
        })}
      </div>
      <p className="mt-1.5 text-[12px] text-faint">{EFFECTS.find((e) => e.id === k.effect)?.what}</p>
      <div className="mt-3 grid grid-cols-2 gap-3 text-[12.5px]">
        <label className="space-y-1">
          <span className="text-dim">Sound</span>
          <Select value={k.sound} disabled={disabled} onChange={(e) => set({ sound: e.target.value })} className="w-full">
            {SOUNDS.map((s) => (
              <option key={s.value} value={s.value}>
                {s.label}
              </option>
            ))}
          </Select>
        </label>
        <label className="space-y-1">
          <span className="text-dim">Colour (firework, kill mark)</span>
          <span className="flex items-center gap-2">
            <input type="color" disabled={disabled} value={k.color} onChange={(e) => set({ color: e.target.value.toUpperCase() })} className="h-8 w-10 cursor-pointer rounded-lg border border-line bg-transparent" aria-label="Effect colour" />
            <span className="font-mono text-faint">{k.color}</span>
          </span>
        </label>
      </div>
      <div className="mt-3">
        <Segmented size="sm" label="Size" value={String(k.intensity)} onChange={(v) => set({ intensity: Number(v) })} options={[{ value: '1', label: 'Subtle' }, { value: '2', label: 'Normal' }, { value: '3', label: 'Big' }]} />
      </div>
      <div className="mt-3 space-y-2 text-[13px]">
        <label className="flex items-center justify-between gap-3">
          <span className="text-dim">Only players (not mobs)</span>
          <Switch checked={k.playersOnly} onChange={(v) => set({ playersOnly: v })} label="Only players" disabled={disabled} />
        </label>
        <label className="flex items-center justify-between gap-3">
          <span className="text-dim">“☠ Kill” under the crosshair, with streaks</span>
          <Switch checked={k.killMark} onChange={(v) => set({ killMark: v })} label="Kill mark" disabled={disabled} />
        </label>
      </div>
    </div>
  );
}

function Packs({ st, onChange }: { st: McClientStatus; onChange: () => void }) {
  const [q, setQ] = useState('');
  const query = useDebounced(q, 300);
  const [hits, setHits] = useState<McPackSearchHit[] | null>(null);
  const [adding, setAdding] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    setHits(null);
    void api.minecraft.client.packs(query).then(([h]) => live && setHits(h), () => live && setHits([]));
    return () => {
      live = false;
    };
  }, [query]);
  const add = async (h: McPackSearchHit) => {
    setAdding(h.project);
    try {
      const name = await api.minecraft.client.addPack(h.project);
      toast.success(`${h.title} added`, `${name} · pick it in Options → Resource Packs.`);
      onChange();
    } catch (e) {
      toast.error('Not added', errorText(e));
    } finally {
      setAdding(null);
    }
  };
  const remove = async (name: string) => {
    try {
      await api.minecraft.client.removePack(name);
      onChange();
    } catch (e) {
      toast.error('Not removed', errorText(e));
    }
  };
  return (
    <div className={cx('bg-surface px-5 py-4', !st.installed && 'opacity-60')}>
      <CardHeader icon={PackagePlus} title="Resource packs" subtitle="1.8.9 packs from Modrinth. In-game, Resourcify browses Modrinth and CurseForge too." className="mb-2" />
      {st.packs.length > 0 && (
        <div className="mb-2 flex flex-wrap gap-1.5">
          {st.packs.map((p) => (
            <span key={p} className="inline-flex items-center gap-1 rounded-full border border-line bg-surface-2 py-0.5 pl-2.5 pr-1 text-[12px] text-fg">
              {p.replace(/\.zip$/i, '')}
              <button type="button" onClick={() => void remove(p)} className="grid h-5 w-5 place-items-center rounded-full text-faint hover:bg-surface-3 hover:text-bad" aria-label={`Remove ${p}`}>
                <Trash2 size={12} />
              </button>
            </span>
          ))}
        </div>
      )}
      <SearchInput value={q} onChange={setQ} placeholder="Search packs (PvP, 16x, Faithful…)" />
      <div className="mt-2 max-h-[260px] space-y-1 overflow-y-auto">
        {hits === null ? (
          <div className="grid place-items-center py-6">
            <Spinner />
          </div>
        ) : hits.length === 0 ? (
          <p className="py-4 text-center text-[12.5px] text-faint">No 1.8.9 packs found.</p>
        ) : (
          hits.map((h) => (
            <div key={h.project} className="flex items-center gap-3 rounded-xl px-2 py-1.5 hover:bg-surface-2">
              {h.icon ? <img src={h.icon} alt="" className="h-9 w-9 shrink-0 rounded-lg object-cover" /> : <div className="h-9 w-9 shrink-0 rounded-lg bg-surface-3" />}
              <div className="min-w-0 flex-1">
                <div className="truncate text-[13px] font-medium text-fg">{h.title}</div>
                <div className="truncate text-[11.5px] text-dim">{h.description}</div>
              </div>
              <Button size="sm" variant="secondary" icon={Download} loading={adding === h.project} disabled={!st.installed || !!adding} onClick={() => void add(h)} aria-label={`Add ${h.title}`} />
            </div>
          ))
        )}
      </div>
    </div>
  );
}
