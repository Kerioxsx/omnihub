// Modpacks other launchers installed on this PC (Prism Launcher and the
// ones it grew from, CurseForge, the Modrinth App): listed with their
// version, loader and mods, added to a mix, or hosted as a server.

import type { McLauncher, McLauncherPack, McLauncherPacks, McSource } from '@shared/types';
import { Blocks, Check, FolderOpen, FolderSearch, HardDrive, Plus, RefreshCw, Server, X } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Spinner } from '../../components/ui/Card';
import { SearchInput, Segmented } from '../../components/ui/Form';
import { EmptyState } from '../../components/ui/States';
import { navigate } from '../../lib/router';
import { toast } from '../../state/toasts';

const LAUNCHERS: { id: McLauncher; label: string; tone: 'good' | 'warn' | 'info' }[] = [
  { id: 'prism', label: 'Prism Launcher', tone: 'good' },
  { id: 'curseforge', label: 'CurseForge', tone: 'warn' },
  { id: 'modrinth', label: 'Modrinth App', tone: 'info' },
];

const LOADER_LABEL: Record<string, string> = { fabric: 'Fabric', quilt: 'Quilt', forge: 'Forge', neoforge: 'NeoForge' };
/** Loaders a server can run. */
const SERVER_LOADERS = ['', 'fabric', 'quilt', 'forge', 'neoforge'];

/** "Played 3 days ago", rounded the way people say it. */
function played(secs: number | null): string {
  if (!secs) return 'Not played yet';
  const m = Math.max(0, Math.floor(Date.now() / 1000 - secs) / 60);
  const n = (v: number, unit: string) => `Played ${Math.floor(v)} ${unit}${Math.floor(v) === 1 ? '' : 's'} ago`;
  if (m < 2) return 'Played just now';
  if (m < 60) return n(m, 'minute');
  if (m < 24 * 60) return n(m / 60, 'hour');
  if (m < 48 * 60) return 'Played yesterday';
  if (m < 60 * 24 * 60) return n(m / 1440, 'day');
  return n(m / 43200, 'month');
}

/** Where it came from, without repeating its name ("Version 0.3.2"). */
function origin(p: McLauncherPack): string | null {
  if (!p.pack) return null;
  const rest = p.pack.startsWith(p.name) ? p.pack.slice(p.name.length).trim() : null;
  return rest === null ? `From ${p.pack}` : rest ? `Version ${rest}` : null;
}

export const loaderText = (p: { loader: string; loaderVersion: string }) => (p.loader ? `${LOADER_LABEL[p.loader] ?? p.loader}${p.loaderVersion ? ` ${p.loaderVersion}` : ''}` : 'No mod loader');

export function LauncherBadge({ p }: { p: McLauncherPack }) {
  const l = LAUNCHERS.find((x) => x.id === p.launcher);
  return <Badge tone={l?.tone ?? 'neutral'}>{p.launcherName}</Badge>;
}

/** The modpack's own picture, as its launcher shows it. */
export function LauncherIcon({ p, size = 44 }: { p: McLauncherPack; size?: number }) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    setSrc(null);
    if (p.hasIcon) void api.minecraft.launcherIcon(p.id).then(setSrc, () => undefined);
  }, [p.id, p.hasIcon]);
  return (
    <div className="grid shrink-0 place-items-center overflow-hidden rounded-xl border border-line bg-surface-2" style={{ width: size, height: size }}>
      {src ? <img src={src} alt="" className="h-full w-full object-cover [image-rendering:pixelated]" /> : <Blocks size={size * 0.42} className="text-faint" aria-hidden />}
    </div>
  );
}

export function InstalledPacks({ sources, onAdded }: { sources: McSource[]; onAdded: (s: McSource) => void }) {
  const [data, setData] = useState<McLauncherPacks | null>(null);
  const [filter, setFilter] = useState<McLauncher | 'all'>('all');
  const [query, setQuery] = useState('');
  const [busy, setBusy] = useState<string | null>(null);
  const [all, setAll] = useState(false);

  const load = useCallback(() => {
    setData(null);
    void api.minecraft.launcherPacks().then(setData, (e: unknown) => {
      setData({ packs: [], places: [] });
      toast.error('Couldn’t look for installed modpacks', errorText(e));
    });
  }, []);
  useEffect(load, [load]);

  const packs = useMemo(() => data?.packs ?? [], [data]);
  const counts = useMemo(() => Object.fromEntries(LAUNCHERS.map((l) => [l.id, packs.filter((p) => p.launcher === l.id).length])), [packs]);
  const list = useMemo(() => {
    const q = query.trim().toLowerCase();
    return packs.filter((p) => (filter === 'all' || p.launcher === filter) && (!q || `${p.name} ${p.pack ?? ''} ${p.mc} ${p.loader}`.toLowerCase().includes(q)));
  }, [packs, filter, query]);
  const shown = all || query ? list : list.slice(0, 6);
  const added = (p: McLauncherPack) => sources.some((s) => s.kind.type === 'launcher' && s.kind.path === p.path);

  const lookIn = async () => {
    const dir = await api.app.pickFolder('A launcher’s folder, its instances folder, or one modpack’s folder');
    if (!dir) return;
    try {
      const before = packs.length;
      const d = await api.minecraft.addLauncherPlace(dir);
      setData(d);
      toast.success('Folder added', `${d.packs.length - before || 'No'} more modpacks found. OmniHub looks there every time now.`);
    } catch (e) {
      toast.error('Nothing found there', errorText(e));
    }
  };
  const forget = async (path: string) => {
    try {
      setData(await api.minecraft.removeLauncherPlace(path));
    } catch (e) {
      toast.error('Not removed', errorText(e));
    }
  };
  const mix = async (p: McLauncherPack) => {
    setBusy(p.id);
    try {
      const s = await api.minecraft.addLauncherPack(p.id);
      toast.success(`${p.name} is ready to mix`, `${s.files.length} mods and ${s.overrides} configs, for Minecraft ${s.mc} with ${loaderText(s)}.`);
      onAdded(s);
    } catch (e) {
      toast.error('Not added', errorText(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <Card>
      <CardHeader
        icon={HardDrive}
        title="Installed on this PC"
        subtitle="Modpacks from Prism Launcher, CurseForge and the Modrinth App. OmniHub only reads them; nothing in their folders changes."
        className="px-5 pt-4"
        actions={
          <div className="flex gap-2">
            <Button size="sm" variant="secondary" icon={FolderSearch} onClick={() => void lookIn()}>
              Look in a folder
            </Button>
            <IconButton icon={RefreshCw} label="Look again" disabled={!data} onClick={load} />
          </div>
        }
      />
      <div className="space-y-3 px-5 pb-4 pt-3">
        {!data ? (
          <div className="flex items-center gap-2 py-6 text-[12.5px] text-dim">
            <Spinner size={14} /> Looking for modpacks…
          </div>
        ) : packs.length === 0 ? (
          <EmptyState
            compact
            icon={HardDrive}
            title="No installed modpacks found"
            description="OmniHub looks where Prism Launcher, CurseForge and the Modrinth App keep them. If yours are somewhere else (another drive, a portable launcher), point it at that folder."
            action={
              <Button size="sm" icon={FolderSearch} onClick={() => void lookIn()}>
                Look in a folder
              </Button>
            }
          />
        ) : (
          <>
            <div className="flex flex-wrap items-center gap-2">
              <Segmented
                size="sm"
                label="Launcher"
                value={filter}
                onChange={setFilter}
                options={[{ value: 'all' as const, label: `All ${packs.length}` }, ...LAUNCHERS.filter((l) => counts[l.id]).map((l) => ({ value: l.id, label: `${l.label} ${counts[l.id]}` }))]}
              />
              {packs.length > 6 && <SearchInput value={query} onChange={setQuery} placeholder="Find a modpack" className="ml-auto w-56" />}
            </div>
            <div className="grid gap-2 lg:grid-cols-2">
              {shown.map((p) => (
                <PackRow key={p.id} p={p} added={added(p)} busy={busy === p.id} onMix={() => void mix(p)} />
              ))}
            </div>
            {!query && list.length > 6 && (
              <Button size="sm" variant="ghost" onClick={() => setAll((a) => !a)}>
                {all ? 'Show fewer' : `Show all ${list.length}`}
              </Button>
            )}
            {list.length === 0 && <div className="py-3 text-center text-[12.5px] text-dim">No modpack matches.</div>}
          </>
        )}
        {data && data.places.length > 0 && (
          <div className="flex flex-wrap items-center gap-1.5 text-[11.5px] text-faint">
            Also looking in
            {data.places.map((p) => (
              <span key={p} className="inline-flex max-w-[22rem] items-center gap-1 rounded-full border border-line bg-surface-2 py-0.5 pl-2.5 pr-1 font-mono text-[11px] text-dim">
                <span className="truncate">{p}</span>
                <button type="button" aria-label={`Stop looking in ${p}`} onClick={() => void forget(p)} className="rounded-full p-0.5 hover:bg-surface-3 hover:text-fg">
                  <X size={11} />
                </button>
              </span>
            ))}
          </div>
        )}
      </div>
    </Card>
  );
}

function PackRow({ p, added, busy, onMix }: { p: McLauncherPack; added: boolean; busy: boolean; onMix: () => void }) {
  const hostable = !!p.mc && SERVER_LOADERS.includes(p.loader);
  return (
    <div className="flex items-center gap-3 rounded-xl border border-line px-3 py-2.5">
      <LauncherIcon p={p} />
      <div className="min-w-0 flex-1">
        <div className="flex min-w-0 items-center gap-2">
          <span className="truncate text-[13.5px] font-semibold text-fg" title={p.name}>
            {p.name}
          </span>
          <LauncherBadge p={p} />
        </div>
        <div className="truncate text-[11.5px] text-dim">
          {p.mc ? `Minecraft ${p.mc}` : 'Version unknown'} · {loaderText(p)} · {p.mods} mods{p.disabled ? ` (${p.disabled} turned off)` : ''}
        </div>
        <div className="truncate text-[11.5px] text-faint">{[origin(p), played(p.lastPlayed)].filter(Boolean).join(' · ')}</div>
      </div>
      <div className="flex shrink-0 items-center gap-1">
        <Button size="sm" variant={added ? 'ghost' : 'secondary'} icon={added ? Check : Plus} disabled={added || busy || !p.mods} loading={busy} onClick={onMix} title={!p.mods ? 'It has no mods to mix.' : added ? 'It’s in Packs to mix.' : 'Add it to Packs to mix (its mods and configs are copied; the instance stays as it is).'}>
          {added ? 'Added' : 'Mix'}
        </Button>
        <IconButton icon={Server} label={hostable ? `Host ${p.name} as a server` : 'Its version or loader can’t run as a server'} disabled={!hostable} onClick={() => navigate('servers', { from: `launcher:${p.id}` })} />
        <IconButton icon={FolderOpen} label={`Open ${p.name}’s folder`} onClick={() => void api.app.openPath(p.path)} />
      </div>
    </div>
  );
}
