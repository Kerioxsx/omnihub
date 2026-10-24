// One game's mods: how it's modded (and any warnings), its loader, the
// mods installed (on, off, update, remove), Thunderstore, the Steam
// Workshop, and Claude making a mod for it.

import type { BrowsePackage, InstalledMod, ModBrowse, ModGame, ModSort, WorkshopItem } from '@shared/types';
import { formatBytes } from '@shared/format';
import { ArrowLeft, Bot, Download, ExternalLink, FilePlus2, FolderOpen, Package, Power, RefreshCw, Search, Store, Trash2, Wrench } from 'lucide-react';
import { type ReactNode, useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Spinner } from '../../components/ui/Card';
import { SearchInput, Select, Switch, Tabs } from '../../components/ui/Form';
import { ProgressBar } from '../../components/ui/Progress';
import { Callout, EmptyState } from '../../components/ui/States';
import { useDebounced, useEvent } from '../../lib/hooks';
import { navigate, useRoute } from '../../lib/router';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { MakerTab } from './MakerTab';
import { count, GameArt, ProgressLine, useModProgress } from './shared';

type Tab = 'installed' | 'browse' | 'workshop' | 'claude';

function sourceBadge(m: InstalledMod): ReactNode {
  switch (m.source.type) {
    case 'thunderstore':
      return <Badge tone="info">Thunderstore</Badge>;
    case 'claude':
      return (
        <Badge tone="accent" icon={Bot}>
          Made by Claude
        </Badge>
      );
    case 'github':
      return <Badge tone="neutral">GitHub</Badge>;
    default:
      return <Badge tone="neutral">From a file</Badge>;
  }
}

function InstalledTab({ game, mods, reload }: { game: ModGame; mods: InstalledMod[]; reload: () => void }) {
  const run = async (what: string, f: () => Promise<unknown>) => {
    try {
      await f();
      reload();
    } catch (e) {
      toast.error(what, errorText(e));
    }
  };
  const remove = async (m: InstalledMod) => {
    const ok = await confirm({ title: `Remove ${m.name}?`, description: m.loader ? 'Mods that need this loader stop loading until it’s back.' : 'Its files go; any game files it replaced come back.', confirmLabel: 'Remove', tone: 'danger' });
    if (ok) await run('Not removed', () => api.mods.uninstall(m.id));
  };
  if (!mods.length) {
    return (
      <Card>
        <EmptyState icon={Package} title="No mods yet" description={game.tsInstalls ? 'Find some on Thunderstore, add a mod file you downloaded, or ask Claude to make one.' : 'Add a mod file you downloaded (zip, 7z or a folder), or ask Claude to make one.'} />
      </Card>
    );
  }
  return (
    <Card className="divide-y divide-line">
      {mods.map((m) => (
        <div key={m.id} className="flex items-center gap-3 px-5 py-3.5">
          <div className="grid h-10 w-10 shrink-0 place-items-center overflow-hidden rounded-xl bg-surface-2 text-dim">{m.icon ? <img src={m.icon} alt="" className="h-full w-full object-cover" /> : m.loader ? <Wrench size={18} /> : <Package size={18} />}</div>
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-1.5">
              <span className="truncate text-[13.5px] font-semibold text-fg">{m.name}</span>
              {m.version && <span className="font-mono text-[11.5px] text-faint">{m.version}</span>}
              {m.loader && <Badge tone="accent">Mod loader</Badge>}
              {m.dependency && <Badge tone="neutral">Needed by another mod</Badge>}
              {sourceBadge(m)}
            </div>
            <div className="truncate text-[12px] text-dim">{m.description || (m.author ? `by ${m.author}` : `${m.files.length} file${m.files.length === 1 ? '' : 's'}`)}</div>
          </div>
          {m.update && (
            <Button size="sm" variant="secondary" icon={RefreshCw} onClick={() => void run('Not updated', () => api.mods.update(m.id))}>
              Update to {m.update}
            </Button>
          )}
          <Switch checked={m.enabled} onChange={(v) => void run(v ? 'Not switched on' : 'Not switched off', () => api.mods.setEnabled(m.id, v))} label={`${m.name} on`} />
          <IconButton icon={Trash2} label={`Remove ${m.name}`} onClick={() => void remove(m)} />
        </div>
      ))}
    </Card>
  );
}

const SORTS: { value: ModSort; label: string }[] = [
  { value: 'downloads', label: 'Most downloaded' },
  { value: 'rating', label: 'Best rated' },
  { value: 'updated', label: 'Recently updated' },
  { value: 'name', label: 'Name' },
];

function BrowseTab({ game, reload }: { game: ModGame; reload: () => void }) {
  const [query, setQuery] = useState('');
  const [category, setCategory] = useState('');
  const [sort, setSort] = useState<ModSort>('downloads');
  const [data, setData] = useState<ModBrowse | null>(null);
  const [limit, setLimit] = useState(30);
  const [busy, setBusy] = useState<string | null>(null);
  const [progress, setProgress] = useState<[number, number] | null>(null);
  const q = useDebounced(query, 250);
  const load = useCallback(() => void api.mods.browse(game.key, q, category || null, sort, 0, limit).then(setData, (e: unknown) => setData({ packages: [], total: 0, loading: false, progress: null, error: errorText(e), categories: [] })), [game.key, q, category, sort, limit]);
  useEffect(load, [load]);
  useEvent<{ game: string }>('mods:changed', (e) => (e.game === game.key || e.game === '') && load());
  useEvent<{ community: string; done: number; total: number }>('mods:listing', (e) => e.community === game.thunderstore && setProgress([e.done, e.total]));

  const install = async (p: BrowsePackage) => {
    if (!game.tsInstalls) {
      void api.app.openUrl(p.managerLink);
      return;
    }
    setBusy(p.fullName);
    try {
      const done = await api.mods.installPackage(game.key, p.fullName);
      toast.success(`${p.name.replace(/_/g, ' ')} installed`, done.length > 1 ? `With ${done.length - 1} mod${done.length > 2 ? 's' : ''} it needs.` : undefined);
      reload();
      load();
    } catch (e) {
      toast.error('Not installed', errorText(e));
    } finally {
      setBusy(null);
    }
  };

  if (data?.loading || !data) {
    const pr = data?.progress ?? progress;
    return (
      <Card className="space-y-3 px-5 py-6">
        <div className="flex items-center gap-2 text-[13px] text-dim">
          <Spinner /> Getting {game.name}’s mod list from Thunderstore…
        </div>
        <ProgressBar value={pr && pr[1] ? (pr[0] / pr[1]) * 100 : null} />
        <div className="text-[11.5px] text-faint">Big games have tens of thousands of mods; the list is kept for a few hours after this.</div>
      </Card>
    );
  }
  return (
    <div className="space-y-3">
      {!game.tsInstalls && (
        <Callout tone="info" title="Installed with r2modman">
          OmniHub doesn’t install mods for this game’s loader ({game.tsLoader}). Install opens r2modman or the Thunderstore Mod Manager, if you have one.
        </Callout>
      )}
      <div className="flex flex-wrap items-center gap-2">
        <SearchInput value={query} onChange={setQuery} placeholder="Search Thunderstore…" className="w-72" />
        <Select value={category} onChange={(e) => setCategory(e.target.value)} className="w-48" aria-label="Category">
          <option value="">Every category</option>
          {data.categories.map((c) => (
            <option key={c} value={c}>
              {c}
            </option>
          ))}
        </Select>
        <Select value={sort} onChange={(e) => setSort(e.target.value as ModSort)} className="w-48" aria-label="Sort">
          {SORTS.map((s) => (
            <option key={s.value} value={s.value}>
              {s.label}
            </option>
          ))}
        </Select>
        <span className="text-[12px] text-faint">{count(data.total)} mods</span>
      </div>
      {data.error && <Callout tone="bad">{data.error}</Callout>}
      <div className="grid gap-3 xl:grid-cols-2">
        {data.packages.map((p) => (
          <Card key={p.fullName} className="flex gap-3 px-4 py-3.5">
            <div className="grid h-14 w-14 shrink-0 place-items-center overflow-hidden rounded-xl bg-surface-2 text-dim">{p.icon ? <img src={p.icon} alt="" className="h-full w-full object-cover" loading="lazy" /> : <Package size={22} />}</div>
            <div className="min-w-0 flex-1">
              <div className="flex flex-wrap items-center gap-1.5">
                <span className="truncate text-[13.5px] font-semibold text-fg">{p.name.replace(/_/g, ' ')}</span>
                <span className="text-[11.5px] text-faint">by {p.owner}</span>
                {p.isLoader && <Badge tone="accent">Mod loader</Badge>}
              </div>
              <div className="line-clamp-2 text-[12px] text-dim">{p.description}</div>
              <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11.5px] text-faint">
                <span className="inline-flex items-center gap-1">
                  <Download size={11} /> {count(p.downloads)}
                </span>
                <span className="font-mono">v{p.version}</span>
                {p.dependencies.length > 0 && <span>needs {p.dependencies.length}</span>}
                <button type="button" className="inline-flex items-center gap-0.5 hover:text-accent" onClick={() => void api.app.openUrl(p.url)}>
                  Page <ExternalLink size={10} />
                </button>
              </div>
            </div>
            <div className="flex shrink-0 flex-col items-end justify-center gap-1">
              {p.isLoader ? (
                <span className="text-[11.5px] text-faint">Comes with mods</span>
              ) : p.installed ? (
                <Badge tone="good">{p.installed === p.version ? 'Installed' : `Installed ${p.installed}`}</Badge>
              ) : (
                <Button size="sm" variant="primary" icon={Download} disabled={busy !== null} onClick={() => void install(p)}>
                  {busy === p.fullName ? 'Installing…' : 'Install'}
                </Button>
              )}
            </div>
          </Card>
        ))}
      </div>
      {data.total > data.packages.length && (
        <div className="flex justify-center">
          <Button variant="secondary" onClick={() => setLimit((l) => l + 30)}>
            Show more
          </Button>
        </div>
      )}
    </div>
  );
}

function WorkshopTab({ game }: { game: ModGame }) {
  const [items, setItems] = useState<WorkshopItem[] | null>(null);
  useEffect(() => void api.mods.workshop(game.key).then(setItems, () => setItems([])), [game.key]);
  const browse = () => void api.app.openUrl(`steam://url/SteamWorkshopPage/${game.appId}`);
  return (
    <div className="space-y-3">
      <Callout
        tone="info"
        action={
          <Button size="sm" variant="secondary" icon={Store} onClick={browse}>
            Open the Workshop
          </Button>
        }
      >
        Steam installs and updates Workshop items itself: subscribe on an item’s page and Steam downloads it.
      </Callout>
      {items === null ? (
        <Spinner />
      ) : items.length === 0 ? (
        <Card>
          <EmptyState icon={Store} title="No Workshop items" description="Nothing from the Steam Workshop is downloaded for this game (or it has no Workshop)." />
        </Card>
      ) : (
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {items.map((it) => (
            <Card key={it.id} interactive className="overflow-hidden" onClick={() => void api.app.openUrl(it.url)}>
              <GameArt game={{ name: it.title, image: it.image || null }} className="aspect-square w-full" />
              <div className="px-3 py-2.5">
                <div className="truncate text-[13px] font-medium text-fg">{it.title}</div>
                <div className="text-[11.5px] text-faint">{it.size ? formatBytes(it.size) : 'Workshop item'}</div>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

export function GameView({ game, onClaude, claudeModal }: { game: ModGame; onClaude: () => void; claudeModal: ReactNode }) {
  const route = useRoute();
  const tabs: Tab[] = ['installed', ...(game.thunderstore ? (['browse'] as Tab[]) : []), ...(game.appId ? (['workshop'] as Tab[]) : []), ...(game.maker.available ? (['claude'] as Tab[]) : [])];
  const tab = (tabs.find((t) => t === route.params.get('tab')) ?? 'installed') as Tab;
  const [mods, setMods] = useState<InstalledMod[]>([]);
  const progress = useModProgress(game.key);
  const reload = useCallback(() => void api.mods.list(game.key).then(setMods, () => undefined), [game.key]);
  useEffect(reload, [reload]);
  useEvent<{ game: string }>('mods:changed', (e) => (e.game === game.key || !e.game) && reload());

  const addFile = async () => {
    const files = await api.app.pickFiles('A mod (zip, 7z, or one file)');
    for (const f of files) {
      try {
        const m = await api.mods.installFile(game.key, f);
        toast.success(`${m.name} installed`);
      } catch (e) {
        toast.error('Not installed', errorText(e));
      }
    }
    reload();
  };
  const addModFolder = async () => {
    const dir = await api.app.pickFolder('A mod’s folder');
    if (!dir) return;
    try {
      const m = await api.mods.installFile(game.key, dir);
      toast.success(`${m.name} installed`);
      reload();
    } catch (e) {
      toast.error('Not installed', errorText(e));
    }
  };
  const allOff = async (on: boolean) => {
    try {
      const n = await api.mods.setAll(game.key, on);
      toast.info(on ? `${n} switched on` : `${n} switched off`, on ? undefined : 'The game is as it was without mods.');
      reload();
    } catch (e) {
      toast.error('Not changed', errorText(e));
    }
  };
  const installLoader = async (kind: (typeof game.loaders)[number]['kind']) => {
    try {
      await api.mods.installLoader(game.key, kind);
      reload();
    } catch (e) {
      toast.error('Not installed', errorText(e));
    }
  };
  const anyOn = mods.some((m) => m.enabled);

  return (
    <Page
      title={
        <span className="flex items-center gap-2">
          <IconButton icon={ArrowLeft} label="All games" onClick={() => navigate('mods')} />
          {game.name}
        </span>
      }
      subtitle={`${game.engineLabel} · ${game.methodLabel}`}
      actions={
        !game.onlineOnly && (
          <>
            <Button variant="secondary" icon={FilePlus2} onClick={() => void addFile()}>
              Add a file
            </Button>
            <Button variant="secondary" icon={FolderOpen} onClick={() => void addModFolder()}>
              Add a folder
            </Button>
            {mods.length > 0 && (
              <Button variant="secondary" icon={Power} onClick={() => void allOff(!anyOn)}>
                {anyOn ? 'Switch all off' : 'Switch all on'}
              </Button>
            )}
          </>
        )
      }
    >
      {claudeModal}
      <div className="space-y-4">
        <Card className="overflow-hidden">
          <div className="flex flex-col gap-4 sm:flex-row">
            <GameArt game={game} className="aspect-[460/215] w-full shrink-0 sm:w-72" />
            <div className="min-w-0 flex-1 space-y-2.5 px-5 py-4 sm:pl-0">
              <div className="flex flex-wrap gap-1.5">
                <Badge tone="neutral">{game.engineLabel}</Badge>
                {game.thunderstore && <Badge tone="info">On Thunderstore</Badge>}
                {game.antiCheat.map((a) => (
                  <Badge key={a} tone="warn">
                    {a}
                  </Badge>
                ))}
              </div>
              {game.loaders.map((l) => (
                <div key={l.kind} className="flex flex-wrap items-center gap-2 text-[12.5px]">
                  <Wrench size={13} className="text-dim" />
                  <span className="font-medium text-fg">{l.label}</span>
                  {l.installed ? (
                    <Badge tone="good">Installed</Badge>
                  ) : l.auto ? (
                    <>
                      <Badge tone="neutral">Comes with the first mod</Badge>
                      <Button size="sm" variant="ghost" onClick={() => void installLoader(l.kind)}>
                        Install now
                      </Button>
                    </>
                  ) : (
                    <>
                      <Badge tone="warn">Get it yourself</Badge>
                      {l.link && (
                        <Button size="sm" variant="ghost" icon={ExternalLink} onClick={() => void api.app.openUrl(l.link!)}>
                          Its page
                        </Button>
                      )}
                    </>
                  )}
                </div>
              ))}
              <div className="flex flex-wrap gap-2 pt-1">
                <Button size="sm" variant="ghost" icon={FolderOpen} onClick={() => void api.app.openPath(game.dir)}>
                  Game folder
                </Button>
                {game.dataDir && (
                  <Button size="sm" variant="ghost" icon={FolderOpen} onClick={() => void api.app.openPath(game.dataDir!)}>
                    Mods folder
                  </Button>
                )}
              </div>
            </div>
          </div>
        </Card>

        {game.notes.map((n, i) => (
          <Callout
            key={i}
            tone={n.tone}
            action={
              n.link && (
                <Button size="sm" variant="secondary" icon={ExternalLink} onClick={() => void api.app.openUrl(n.link!)}>
                  Open
                </Button>
              )
            }
          >
            {n.text}
          </Callout>
        ))}
        <ProgressLine p={progress} />

        {!game.onlineOnly && (
          <>
            <Tabs
              value={tab}
              onChange={(t) => navigate('mods', { game: game.key, tab: t })}
              items={tabs.map((t) =>
                t === 'installed'
                  ? { value: t, label: `Installed${mods.length ? ` (${mods.filter((m) => !m.loader).length})` : ''}`, icon: Package }
                  : t === 'browse'
                    ? { value: t, label: 'Thunderstore', icon: Search }
                    : t === 'workshop'
                      ? { value: t, label: 'Steam Workshop', icon: Store }
                      : { value: t, label: 'Make with Claude', icon: Bot },
              )}
            />
            {tab === 'installed' && <InstalledTab game={game} mods={mods} reload={reload} />}
            {tab === 'browse' && <BrowseTab game={game} reload={reload} />}
            {tab === 'workshop' && <WorkshopTab game={game} />}
            {tab === 'claude' && <MakerTab game={game} onSettings={onClaude} />}
          </>
        )}
        {game.onlineOnly && (
          <Card>
            <CardHeader title="Why there are no mods here" subtitle="Online competitive games ban accounts whose game files change, and their anti-cheat can't tell a harmless mod from a cheat." className="px-5 py-4" />
          </Card>
        )}
      </div>
    </Page>
  );
}
