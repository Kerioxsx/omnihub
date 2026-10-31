// Modpacks other launchers installed on this PC (Prism Launcher and the
// ones it grew from, CurseForge, the Modrinth App): listed with their
// version, loader and mods, added to a mix, or hosted as a server.

import type { McLauncher, McLauncherDetails, McLauncherPack, McLauncherPacks, McSource, McWorld } from '@shared/types';
import { ArrowUpCircle, Blocks, Check, FolderOpen, FolderSearch, Gift, Globe, HardDrive, Info, Plus, RefreshCw, Server, X } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Spinner } from '../../components/ui/Card';
import { SearchInput, Segmented } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { EmptyState } from '../../components/ui/States';
import { ShareWithFriends } from './ShareWithFriends';
import { navigate } from '../../lib/router';
import { toast } from '../../state/toasts';

const LAUNCHERS: { id: McLauncher; label: string; tone: 'good' | 'warn' | 'info' | 'accent' }[] = [
  { id: 'prism', label: 'Prism Launcher', tone: 'good' },
  { id: 'curseforge', label: 'CurseForge', tone: 'warn' },
  { id: 'modrinth', label: 'Modrinth App', tone: 'info' },
  { id: 'minecraft', label: 'Minecraft Launcher', tone: 'accent' },
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
  const [openId, setOpenId] = useState<string | null>(null);

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
      <PackDetails p={packs.find((x) => x.id === openId) ?? null} onClose={() => setOpenId(null)} />
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
                <PackRow key={p.id} p={p} added={added(p)} busy={busy === p.id} onMix={() => void mix(p)} onOpen={() => setOpenId(p.id)} />
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

function PackRow({ p, added, busy, onMix, onOpen }: { p: McLauncherPack; added: boolean; busy: boolean; onMix: () => void; onOpen: () => void }) {
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
        <IconButton icon={Info} label={`What’s in ${p.name}: mods, worlds, sharing`} onClick={onOpen} />
        <IconButton icon={FolderOpen} label={`Open ${p.name}’s folder`} onClick={() => void api.app.openPath(p.path)} />
      </div>
    </div>
  );
}

const gb = (b: number) => (b >= 1_073_741_824 ? `${(b / 1_073_741_824).toFixed(1)} GB` : `${Math.max(1, Math.round(b / 1_048_576))} MB`);

function WorldIcon({ pack, w }: { pack: string; w: McWorld }) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    if (w.hasIcon) void api.minecraft.launcherWorldIcon(pack, w.folder).then(setSrc, () => undefined);
  }, [pack, w.folder, w.hasIcon]);
  return <div className="grid h-12 w-12 shrink-0 place-items-center overflow-hidden rounded-lg border border-line bg-surface-2">{src ? <img src={src} alt="" className="h-full w-full object-cover" /> : <Globe size={18} className="text-faint" aria-hidden />}</div>;
}

/** What's in an installed modpack: its mods (with updates on Modrinth), its worlds, and sharing it. */
export function PackDetails({ p, onClose }: { p: McLauncherPack | null; onClose: () => void }) {
  const [d, setD] = useState<McLauncherDetails | null>(null);
  const [tab, setTab] = useState<'mods' | 'worlds'>('mods');
  const [updates, setUpdates] = useState<Record<string, string> | null>(null);
  const [checking, setChecking] = useState(false);
  const [sharing, setSharing] = useState(false);
  useEffect(() => {
    setD(null);
    setUpdates(null);
    if (p) void api.minecraft.launcherDetails(p.id).then((x) => (setD(x), setTab(x.mods.length || !x.worlds.length ? 'mods' : 'worlds')), (e: unknown) => toast.error('Couldn’t read it', errorText(e)));
  }, [p]);
  if (!p) return null;
  const check = async () => {
    setChecking(true);
    try {
      const u = await api.minecraft.launcherUpdates(p.id);
      setUpdates(u);
      toast.info(Object.keys(u).length ? `${Object.keys(u).length} mods have a newer version` : 'Every mod is up to date', 'Nothing in the pack was changed.');
    } catch (e) {
      toast.error('Couldn’t check', errorText(e));
    } finally {
      setChecking(false);
    }
  };
  const hostable = !!p.mc && SERVER_LOADERS.includes(p.loader);
  return (
    <Modal open={!!p} onClose={onClose} title={p.name} description={`${p.launcherName} · ${p.mc ? `Minecraft ${p.mc}` : 'version unknown'} · ${loaderText(p)}${d ? ` · ${gb(d.size)}` : ''}`} icon={HardDrive} size="lg"
      footer={
        <>
          <Button variant="ghost" icon={FolderOpen} onClick={() => void api.app.openPath(p.path)}>
            Folder
          </Button>
          <Button variant="secondary" icon={Server} disabled={!hostable} onClick={() => navigate('servers', { from: `launcher:${p.id}` })}>
            Host as a server
          </Button>
          <Button variant="primary" icon={Gift} disabled={!p.mc || !p.mods} onClick={() => setSharing(true)}>
            Share with friends
          </Button>
        </>
      }
    >
      <ShareWithFriends open={sharing} onClose={() => setSharing(false)} name={p.name} formats save={(path, format, server) => api.minecraft.shareLauncherPack(p.id, path, format, server)} />
      {!d ? (
        <Spinner />
      ) : (
        <div className="space-y-3">
          <div className="flex flex-wrap items-center gap-2">
            <Segmented size="sm" label="Show" value={tab} onChange={setTab} options={[{ value: 'mods', label: `Mods ${d.mods.length}` }, { value: 'worlds', label: `Worlds ${d.worlds.length}` }]} />
            {tab === 'mods' && d.mods.length > 0 && (
              <Button size="sm" variant="ghost" icon={ArrowUpCircle} loading={checking} disabled={checking || !p.loader} className="ml-auto" onClick={() => void check()}>
                Check for updates
              </Button>
            )}
          </div>
          {tab === 'mods' ? (
            d.mods.length === 0 ? (
              <div className="py-6 text-center text-[12.5px] text-dim">No mods: plain Minecraft.</div>
            ) : (
              <div className="max-h-[50vh] divide-y divide-line overflow-y-auto rounded-xl border border-line">
                {d.mods.map((m) => (
                  <div key={m.file} className="flex items-center gap-3 px-3 py-2">
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[13px] font-medium text-fg">{m.name}</div>
                      <div className="truncate font-mono text-[11px] text-faint">{m.version ? `${m.version} · ` : ''}{m.file}</div>
                    </div>
                    {updates?.[m.file] && <Badge tone="accent" icon={ArrowUpCircle}>{updates[m.file]}</Badge>}
                    {m.side === 'client' && <Badge tone="neutral" title="Left off servers">Game only</Badge>}
                    {m.disabled && <Badge tone="warn">Off</Badge>}
                  </div>
                ))}
              </div>
            )
          ) : d.worlds.length === 0 ? (
            <div className="py-6 text-center text-[12.5px] text-dim">No single-player worlds in it yet.</div>
          ) : (
            <div className="grid max-h-[50vh] gap-2 overflow-y-auto sm:grid-cols-2">
              {d.worlds.map((w) => (
                <div key={w.folder} className="flex items-center gap-3 rounded-xl border border-line px-3 py-2.5">
                  <WorldIcon pack={p.id} w={w} />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[13px] font-semibold text-fg">{w.name}</div>
                    <div className="truncate text-[11.5px] text-dim">
                      {w.hardcore ? 'Hardcore' : w.gameMode[0].toUpperCase() + w.gameMode.slice(1)}{w.version ? ` · ${w.version}` : ''} · {gb(w.size)}
                    </div>
                    <div className="truncate text-[11.5px] text-faint">{played(w.lastPlayed)}</div>
                  </div>
                  <IconButton icon={Server} label={hostable ? `Host ${w.name} as a server` : 'This pack can’t run as a server'} disabled={!hostable} onClick={() => navigate('servers', { from: `world:${p.id}:${w.folder}` })} />
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </Modal>
  );
}
