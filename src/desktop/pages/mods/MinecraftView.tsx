// Minecraft modpacks, mixed: packs from Modrinth, .mrpack files or
// installed instances; one Minecraft version and loader; the mix with every
// mod once (updated to fit, dependencies added, clashes sorted), saved as a
// .mrpack or installed as a Minecraft Launcher profile.

import type { McEntry, McEntryStatus, McLoader, McMix, McPackHit, McPackVersion, McStatus } from '@shared/types';
import { ArrowLeft, Blocks, Download, FilePlus2, FolderOpen, FolderPlus, Play, Plus, Save, Search, Trash2, Wand2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Spinner } from '../../components/ui/Card';
import { Field, SearchInput, Select, TextInput } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { Callout, EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useDebounced } from '../../lib/hooks';
import { navigate } from '../../lib/router';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { count, ProgressLine, useModProgress } from './shared';

const LOADERS: { value: McLoader; label: string }[] = [
  { value: 'fabric', label: 'Fabric' },
  { value: 'quilt', label: 'Quilt' },
  { value: 'neoforge', label: 'NeoForge' },
  { value: 'forge', label: 'Forge' },
];

const STATUS: Record<McEntryStatus, { label: string; tone: 'good' | 'info' | 'accent' | 'neutral' | 'warn' | 'bad' }> = {
  kept: { label: 'In', tone: 'good' },
  updated: { label: 'Updated to fit', tone: 'info' },
  dependency: { label: 'Added: needed', tone: 'accent' },
  asIs: { label: 'Kept as it is', tone: 'neutral' },
  missing: { label: 'Left out', tone: 'warn' },
  conflict: { label: 'Left out: clashes', tone: 'bad' },
};

function versionCmp(a: string, b: string): number {
  const p = (s: string) => s.split(/[.-]/).map((x) => parseInt(x, 10) || 0);
  const [x, y] = [p(a), p(b)];
  for (let i = 0; i < Math.max(x.length, y.length); i++) if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) - (y[i] ?? 0);
  return 0;
}

function AddFromModrinth({ open, onClose, onAdded }: { open: boolean; onClose: () => void; onAdded: () => void }) {
  const [query, setQuery] = useState('');
  const [hits, setHits] = useState<McPackHit[] | null>(null);
  const [pick, setPick] = useState<McPackHit | null>(null);
  const [versions, setVersions] = useState<McPackVersion[] | null>(null);
  const [busy, setBusy] = useState(false);
  const q = useDebounced(query, 300);
  useEffect(() => {
    if (open) void api.minecraft.search(q).then((r) => setHits(r.hits), (e: unknown) => (toast.error('Modrinth didn’t answer', errorText(e)), setHits([])));
  }, [q, open]);
  useEffect(() => {
    setVersions(null);
    if (pick) void api.minecraft.versions(pick.project).then(setVersions, () => setVersions([]));
  }, [pick]);
  const add = async (v: McPackVersion) => {
    if (!pick) return;
    setBusy(true);
    try {
      await api.minecraft.addModrinth(pick.project, v.id);
      toast.success(`${pick.title} added`);
      onAdded();
      setPick(null);
      onClose();
    } catch (e) {
      toast.error('Not added', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal open={open} onClose={onClose} title={pick ? pick.title : 'Add a modpack from Modrinth'} icon={Search} size="lg">
      {!pick ? (
        <div className="space-y-3">
          <SearchInput value={query} onChange={setQuery} placeholder="Search modpacks…" />
          {hits === null ? (
            <Spinner />
          ) : (
            <div className="max-h-[55vh] space-y-1.5 overflow-y-auto">
              {hits.map((h) => (
                <button key={h.project} type="button" onClick={() => setPick(h)} className="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left hover:bg-surface-2">
                  <div className="grid h-11 w-11 shrink-0 place-items-center overflow-hidden rounded-xl bg-surface-2 text-dim">{h.icon ? <img src={h.icon} alt="" className="h-full w-full object-cover" /> : <Blocks size={20} />}</div>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[13.5px] font-semibold text-fg">{h.title}</div>
                    <div className="truncate text-[12px] text-dim">{h.description}</div>
                    <div className="text-[11.5px] text-faint">
                      {count(h.downloads)} downloads · {h.loaders.join(', ')} · {h.versions.slice().sort(versionCmp).slice(-3).join(', ')}
                    </div>
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>
      ) : (
        <div className="space-y-3">
          <Button size="sm" variant="ghost" icon={ArrowLeft} onClick={() => setPick(null)}>
            Back
          </Button>
          <div className="text-[12.5px] text-dim">Pick the version to add:</div>
          {versions === null ? (
            <Spinner />
          ) : (
            <div className="max-h-[50vh] space-y-1 overflow-y-auto">
              {versions.slice(0, 40).map((v) => (
                <div key={v.id} className="flex items-center gap-3 rounded-lg px-3 py-2 hover:bg-surface-2">
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[13px] text-fg">{v.name}</div>
                    <div className="text-[11.5px] text-faint">
                      Minecraft {v.mc.join(', ')} · {v.loaders.join(', ')}
                    </div>
                  </div>
                  <Button size="sm" variant="primary" icon={Plus} disabled={busy} onClick={() => void add(v)}>
                    Add
                  </Button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </Modal>
  );
}

function EntryRow({ e }: { e: McEntry }) {
  const s = STATUS[e.status];
  return (
    <div className={cx('flex items-center gap-3 px-4 py-2.5', (e.status === 'missing' || e.status === 'conflict') && 'opacity-75')}>
      <div className="grid h-8 w-8 shrink-0 place-items-center overflow-hidden rounded-lg bg-surface-2 text-dim">{e.icon ? <img src={e.icon} alt="" className="h-full w-full object-cover" /> : <Blocks size={15} />}</div>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="truncate text-[13px] font-medium text-fg">{e.title}</span>
          {e.version && <span className="truncate font-mono text-[11px] text-faint">{e.version}</span>}
          {e.folder !== 'mods' && <Badge tone="neutral">{e.folder === 'resourcepacks' ? 'Resource pack' : e.folder === 'shaderpacks' ? 'Shader pack' : e.folder}</Badge>}
        </div>
        <div className="truncate text-[11.5px] text-faint">{e.note ?? (e.from.length ? `From ${e.from.join(' and ')}` : '')}</div>
      </div>
      <Badge tone={s.tone}>{s.label}</Badge>
    </div>
  );
}

export function MinecraftView() {
  const [st, setSt] = useState<McStatus | null>(null);
  const [chosen, setChosen] = useState<string[]>([]);
  const [mc, setMc] = useState('');
  const [loader, setLoader] = useState<McLoader>('fabric');
  const [name, setName] = useState('');
  const [mix, setMix] = useState<McMix | null>(null);
  const [planning, setPlanning] = useState(false);
  const [adding, setAdding] = useState(false);
  const [installing, setInstalling] = useState(false);
  const progress = useModProgress('minecraft');

  const load = useCallback(() => void api.minecraft.status().then((s) => {
    setSt(s);
    setChosen((c) => (c.length ? c.filter((id) => s.sources.some((x) => x.id === id)) : s.sources.map((x) => x.id)));
  }), []);
  useEffect(load, [load]);

  const picked = useMemo(() => (st?.sources ?? []).filter((s) => chosen.includes(s.id)), [st, chosen]);
  const versions = useMemo(() => [...new Set(picked.map((s) => s.mc))].sort(versionCmp).reverse(), [picked]);
  useEffect(() => {
    if (!versions.includes(mc) && versions.length) setMc(versions[0]);
    const l = picked[0]?.loader as McLoader | undefined;
    if (l && LOADERS.some((x) => x.value === l) && !picked.some((s) => s.loader === loader)) setLoader(l);
  }, [versions, mc, picked, loader]);

  const addFile = async () => {
    const files = await api.app.pickFiles('A Modrinth pack (.mrpack)');
    for (const f of files) {
      try {
        const s = await api.minecraft.addFile(f);
        setChosen((c) => [...c, s.id]);
      } catch (e) {
        toast.error('Not added', errorText(e));
      }
    }
    load();
  };
  const addFolder = async () => {
    const dir = await api.app.pickFolder('An installed instance (Prism, CurseForge, Modrinth App…)');
    if (!dir) return;
    try {
      const s = await api.minecraft.addFolder(dir);
      setChosen((c) => [...c, s.id]);
      load();
    } catch (e) {
      toast.error('Not added', errorText(e));
    }
  };
  const plan = async () => {
    setPlanning(true);
    try {
      setMix(await api.minecraft.plan(name, chosen, mc, loader));
    } catch (e) {
      toast.error('Couldn’t mix them', errorText(e));
    } finally {
      setPlanning(false);
    }
  };
  const save = async () => {
    if (!mix) return;
    const path = await api.app.saveFile('Save the mix', `${mix.name.replace(/[^\w\s+-]/g, '').trim() || 'Mix'}.mrpack`);
    if (!path) return;
    try {
      await api.minecraft.exportPack(mix.id, path);
      toast.success('Saved', 'Open it in Modrinth App, Prism Launcher or any launcher that reads .mrpack.');
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  };
  const install = async () => {
    if (!mix) return;
    setInstalling(true);
    try {
      const inst = await api.minecraft.install(mix.id);
      toast.success(`${inst.name} is in the Minecraft Launcher`, `Pick it from the profiles next to Play. ${inst.mods} mods.`);
      load();
    } catch (e) {
      toast.error('Not installed', errorText(e));
    } finally {
      setInstalling(false);
    }
  };
  const removeInstall = async (profile: string, n: string) => {
    if (!(await confirm({ title: `Remove ${n}?`, description: 'Its folder (mods, configs, worlds made in it) and its launcher profile go.', confirmLabel: 'Remove', tone: 'danger' }))) return;
    try {
      await api.minecraft.uninstall(profile);
      load();
    } catch (e) {
      toast.error('Not removed', errorText(e));
    }
  };

  const tally = (f: (e: McEntry) => boolean) => mix?.entries.filter(f).length ?? 0;
  const groups: { title: string; list: McEntry[] }[] = mix
    ? [
        { title: 'In the mix', list: mix.entries.filter((e) => !['missing', 'conflict'].includes(e.status)) },
        { title: 'Left out', list: mix.entries.filter((e) => ['missing', 'conflict'].includes(e.status)) },
      ]
    : [];

  return (
    <Page
      title={
        <span className="flex items-center gap-2">
          <IconButton icon={ArrowLeft} label="All games" onClick={() => navigate('mods')} />
          Minecraft modpacks
        </span>
      }
      subtitle="Mix two or more packs into one: every mod once, each at the version made for the Minecraft and loader you choose."
    >
      <AddFromModrinth open={adding} onClose={() => setAdding(false)} onAdded={load} />
      <div className="space-y-4">
        {st && !st.found && <Callout tone="info">Minecraft (Java Edition) wasn’t found on this PC. You can still mix packs and save them as a .mrpack.</Callout>}
        <Card>
          <CardHeader
            icon={Blocks}
            title="Packs to mix"
            subtitle="The first pack wins when two packs set the same config."
            className="px-5 pt-4"
            actions={
              <div className="flex flex-wrap gap-2">
                <Button size="sm" variant="primary" icon={Search} onClick={() => setAdding(true)}>
                  From Modrinth
                </Button>
                <Button size="sm" variant="secondary" icon={FilePlus2} onClick={() => void addFile()}>
                  .mrpack file
                </Button>
                <Button size="sm" variant="secondary" icon={FolderPlus} onClick={() => void addFolder()}>
                  Installed instance
                </Button>
              </div>
            }
          />
          <div className="space-y-1.5 px-5 pb-4 pt-3">
            {!st ? (
              <Spinner />
            ) : st.sources.length === 0 ? (
              <EmptyState compact icon={Blocks} title="No packs yet" description="Add packs from Modrinth, a .mrpack file, or an instance you play (Prism Launcher, CurseForge, Modrinth App)." />
            ) : (
              st.sources.map((s, i) => (
                <div key={s.id} className={cx('flex items-center gap-3 rounded-xl border px-3 py-2.5', chosen.includes(s.id) ? 'border-accent/50 bg-accent-soft/40' : 'border-line')}>
                  <input type="checkbox" checked={chosen.includes(s.id)} onChange={(e) => setChosen((c) => (e.target.checked ? [...c, s.id] : c.filter((x) => x !== s.id)))} aria-label={`Mix ${s.name}`} className="h-4 w-4 accent-[var(--accent)]" />
                  <span className="w-5 text-center font-mono text-[11.5px] text-faint">{chosen.includes(s.id) ? chosen.indexOf(s.id) + 1 : i + 1}</span>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[13.5px] font-semibold text-fg">{s.name}</div>
                    <div className="text-[11.5px] text-faint">
                      Minecraft {s.mc} · {s.loader}
                      {s.loaderVersion ? ` ${s.loaderVersion}` : ''} · {s.files.length} files · {s.overrides} configs and extras
                    </div>
                  </div>
                  <IconButton icon={Trash2} label={`Remove ${s.name}`} onClick={() => void api.minecraft.removeSource(s.id).then(load)} />
                </div>
              ))
            )}
          </div>
        </Card>

        <Card className="px-5 py-4">
          <div className="grid gap-3 md:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)_minmax(0,1fr)_auto] md:items-end">
            <Field label="Name">
              <TextInput value={name} onChange={(e) => setName(e.target.value)} placeholder={picked.map((s) => s.name).join(' + ') || 'My mix'} />
            </Field>
            <Field label="Minecraft version">
              <TextInput value={mc} onChange={(e) => setMc(e.target.value.trim())} list="mc-versions" placeholder="1.21.1" />
              <datalist id="mc-versions">
                {versions.map((v) => (
                  <option key={v} value={v} />
                ))}
              </datalist>
            </Field>
            <Field label="Mod loader">
              <Select value={loader} onChange={(e) => setLoader(e.target.value as McLoader)}>
                {LOADERS.map((l) => (
                  <option key={l.value} value={l.value}>
                    {l.label}
                  </option>
                ))}
              </Select>
            </Field>
            <Button variant="primary" icon={Wand2} disabled={chosen.length === 0 || !mc || planning} onClick={() => void plan()}>
              {planning ? 'Mixing…' : 'Mix them'}
            </Button>
          </div>
        </Card>

        <ProgressLine p={progress} />

        {mix && (
          <Card className="overflow-hidden">
            <div className="flex flex-wrap items-center gap-3 border-b border-line px-5 py-4">
              <div className="min-w-0 flex-1">
                <div className="text-[15px] font-semibold text-fg">{mix.name}</div>
                <div className="text-[12.5px] text-dim">
                  Minecraft {mix.mc} · {mix.loader} {mix.loaderVersion} · {tally((e) => !!e.file || e.status === 'asIs')} mods and packs · {tally((e) => e.status === 'updated')} updated · {tally((e) => e.status === 'dependency')} added · {tally((e) => e.status === 'missing' || e.status === 'conflict')} left out
                </div>
              </div>
              <Button variant="secondary" icon={Save} onClick={() => void save()}>
                Save as .mrpack
              </Button>
              <Button variant="primary" icon={Play} disabled={!mix.installable || !st?.found || installing} onClick={() => void install()} title={mix.installable ? undefined : 'Forge and NeoForge mixes: save as .mrpack and open it in Modrinth App or Prism Launcher.'}>
                {installing ? 'Installing…' : 'Install in the Minecraft Launcher'}
              </Button>
            </div>
            {mix.problems.map((p, i) => (
              <Callout key={i} tone="warn" className="mx-5 mt-3">
                {p}
              </Callout>
            ))}
            {!mix.installable && (
              <Callout tone="info" className="mx-5 mt-3">
                {mix.loader === 'forge' ? 'Forge' : 'NeoForge'} needs its own installer, so this mix goes into a launcher that installs it: save it as a .mrpack and open it in Modrinth App, Prism Launcher or CurseForge.
              </Callout>
            )}
            {groups.map((g) =>
              g.list.length ? (
                <div key={g.title}>
                  <div className="px-5 pb-1 pt-4 text-[11.5px] font-semibold uppercase tracking-wide text-faint">
                    {g.title} ({g.list.length})
                  </div>
                  <div className="divide-y divide-line">
                    {g.list.map((e, i) => (
                      <EntryRow key={`${e.title}${i}`} e={e} />
                    ))}
                  </div>
                </div>
              ) : null,
            )}
            {mix.clashes.length > 0 && (
              <div className="px-5 pb-4 pt-4">
                <div className="mb-1 text-[11.5px] font-semibold uppercase tracking-wide text-faint">Configs both packs had</div>
                <ul className="space-y-0.5 font-mono text-[11.5px] text-dim">
                  {mix.clashes.slice(0, 30).map((c) => (
                    <li key={c}>{c}</li>
                  ))}
                  {mix.clashes.length > 30 && <li>… and {mix.clashes.length - 30} more</li>}
                </ul>
              </div>
            )}
          </Card>
        )}

        {st && st.installed.length > 0 && (
          <Card>
            <CardHeader icon={Download} title="Installed mixes" subtitle="Profiles OmniHub added to the Minecraft Launcher." className="px-5 pt-4" />
            <div className="divide-y divide-line px-2 pb-2 pt-2">
              {st.installed.map((i) => (
                <div key={i.profile} className="flex items-center gap-3 px-3 py-2.5">
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[13.5px] font-medium text-fg">{i.name}</div>
                    <div className="text-[11.5px] text-faint">
                      Minecraft {i.mc} · {i.loader} · {i.mods} mods
                    </div>
                  </div>
                  <Button size="sm" variant="ghost" icon={FolderOpen} onClick={() => void api.app.openPath(i.dir)}>
                    Folder
                  </Button>
                  <IconButton icon={Trash2} label={`Remove ${i.name}`} onClick={() => void removeInstall(i.profile, i.name)} />
                </div>
              ))}
            </div>
          </Card>
        )}
      </div>
    </Page>
  );
}
