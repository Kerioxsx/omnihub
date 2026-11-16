// What a server runs: its mods, plugins and data packs (on, off, removed),
// more from Modrinth (with what they need), a file from this PC, or made by
// Claude (data packs for any server, plugins for Paper).

import type { SrvContent, SrvHit, SrvKind, SrvView } from '@shared/types';
import { formatBytes } from '@shared/format';
import { Bot, Box, Download, FilePlus2, Package, Puzzle, Search, Trash2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, Spinner } from '../../components/ui/Card';
import { SearchInput, Segmented, Switch } from '../../components/ui/Form';
import { Callout, EmptyState } from '../../components/ui/States';
import { useDebounced, useEvent } from '../../lib/hooks';
import { navigate, useRoute } from '../../lib/router';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { MakerTab } from '../mods/MakerTab';
import { ClaudeSettingsModal } from '../mods/shared';
import { run } from './ServerView';

type View = 'installed' | 'browse' | 'claude';

const KIND_LABEL: Record<SrvKind, string> = { mod: 'Mod', plugin: 'Plugin', datapack: 'Data pack' };

function kinds(s: SrvView): SrvKind[] {
  const out: SrvKind[] = [];
  if (s.flavor === 'paper') out.push('plugin');
  if (s.flavor === 'fabric' || s.flavor === 'quilt' || s.flavor === 'forge' || s.flavor === 'neoForge') out.push('mod');
  out.push('datapack');
  return out;
}

function sourceBadge(c: SrvContent) {
  if (c.source === 'claude') return <Badge tone="accent" icon={Bot}>Made by Claude</Badge>;
  if (c.source === 'modrinth') return <Badge tone="info">Modrinth</Badge>;
  if (c.source === 'pack' || c.source === 'mix') return <Badge tone="neutral">{c.from ?? 'Modpack'}</Badge>;
  return <Badge tone="neutral">Added by hand</Badge>;
}

function Installed({ s, list, reload }: { s: SrvView; list: SrvContent[]; reload: () => void }) {
  const running = s.state === 'running' || s.state === 'starting';
  const remove = async (c: SrvContent) => {
    if (await confirm({ title: `Remove ${c.title}?`, description: c.kind === 'datapack' ? 'The world keeps anything it already made.' : 'The server stops loading it.', confirmLabel: 'Remove', tone: 'danger' })) {
      await run('Not removed', () => api.servers.removeContent(s.id, c.file));
      reload();
    }
  };
  if (!list.length) {
    return (
      <Card>
        <EmptyState icon={Package} title="Nothing added yet" description={`Find ${kinds(s).map((k) => KIND_LABEL[k].toLowerCase() + 's').join(' and ')} on Modrinth, add a file, or ask Claude to make one.`} />
      </Card>
    );
  }
  return (
    <div className="space-y-3">
      {running && kinds(s).some((k) => k !== 'datapack') && <Callout tone="info">Mods and plugins can be switched on, off or removed while the server is stopped (Windows keeps them open while it runs). Data packs change right away.</Callout>}
      <Card className="divide-y divide-line">
        {list.map((c) => (
          <div key={c.file} className="flex items-center gap-3 px-5 py-3">
            <div className="grid h-10 w-10 shrink-0 place-items-center overflow-hidden rounded-xl bg-surface-2 text-dim">{c.icon ? <img src={c.icon} alt="" className="h-full w-full object-cover" /> : c.kind === 'datapack' ? <Box size={18} /> : <Puzzle size={18} />}</div>
            <div className="min-w-0 flex-1">
              <div className="flex flex-wrap items-center gap-1.5">
                <span className="truncate text-[13.5px] font-semibold text-fg">{c.title}</span>
                {c.version && <span className="font-mono text-[11.5px] text-faint">{c.version}</span>}
                <Badge tone="neutral">{KIND_LABEL[c.kind]}</Badge>
                {c.dependency && <Badge tone="neutral">Needed by another</Badge>}
                {sourceBadge(c)}
              </div>
              <div className="truncate font-mono text-[11.5px] text-faint">
                {c.file} · {formatBytes(c.size)}
              </div>
            </div>
            <Switch checked={c.enabled} disabled={running && c.kind !== 'datapack'} onChange={(v) => void run('Not changed', () => api.servers.setContent(s.id, c.file, v)).then(reload)} label={`${c.title} on`} />
            <IconButton icon={Trash2} label={`Remove ${c.title}`} disabled={running && c.kind !== 'datapack'} onClick={() => void remove(c)} />
          </div>
        ))}
      </Card>
    </div>
  );
}

function Browse({ s, onInstalled }: { s: SrvView; onInstalled: () => void }) {
  const ks = kinds(s);
  const [kind, setKind] = useState<SrvKind>(ks[0]);
  const [query, setQuery] = useState('');
  const q = useDebounced(query, 300);
  const [hits, setHits] = useState<SrvHit[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const load = useCallback(() => {
    setHits(null);
    void api.servers.search(s.id, kind, q).then((r) => setHits(r.hits), (e: unknown) => (setHits([]), toast.error('Modrinth didn’t answer', errorText(e))));
  }, [s.id, kind, q]);
  useEffect(load, [load]);
  const install = async (h: SrvHit) => {
    setBusy(h.project);
    try {
      const got = await api.servers.install(s.id, kind, h.project);
      toast.success(`${h.title} added`, got.length > 1 ? `With ${got.length - 1} it needs.` : kind === 'datapack' ? 'It’s live (the server reloaded its data packs).' : s.state === 'running' ? 'Restart the server to load it.' : undefined);
      onInstalled();
      load();
    } catch (e) {
      toast.error('Not added', errorText(e));
    } finally {
      setBusy(null);
    }
  };
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center gap-2">
        {ks.length > 1 && <Segmented label="Kind" value={kind} onChange={setKind} options={ks.map((k) => ({ value: k, label: `${KIND_LABEL[k]}s` }))} />}
        <SearchInput value={query} onChange={setQuery} placeholder={`Search Modrinth’s ${KIND_LABEL[kind].toLowerCase()}s for Minecraft ${s.mc}`} className="min-w-[260px] flex-1" />
      </div>
      <Card className="divide-y divide-line">
        {hits === null && (
          <div className="flex items-center gap-2 px-5 py-6 text-[13px] text-dim">
            <Spinner /> Searching…
          </div>
        )}
        {hits?.length === 0 && <EmptyState compact icon={Search} title="Nothing found" description={`Nothing for ${s.flavor === 'paper' ? 'Paper' : s.flavor} ${s.mc} matches. Try other words.`} />}
        {hits?.map((h) => (
          <div key={h.project} className="flex items-center gap-3 px-5 py-3">
            <div className="h-10 w-10 shrink-0 overflow-hidden rounded-xl bg-surface-2">{h.icon && <img src={h.icon} alt="" className="h-full w-full object-cover" />}</div>
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-1.5">
                <span className="truncate text-[13.5px] font-semibold text-fg">{h.title}</span>
                <span className="text-[11.5px] text-faint">by {h.author}</span>
              </div>
              <div className="truncate text-[12px] text-dim">{h.description}</div>
            </div>
            <span className="hidden text-[11.5px] text-faint sm:block">{h.downloads >= 1e6 ? `${(h.downloads / 1e6).toFixed(1)}M` : h.downloads >= 1e3 ? `${Math.round(h.downloads / 1e3)}k` : h.downloads}</span>
            <Button size="sm" variant={h.installed ? 'ghost' : 'secondary'} icon={Download} disabled={h.installed || !!busy} loading={busy === h.project} onClick={() => void install(h)}>
              {h.installed ? 'Added' : 'Add'}
            </Button>
          </div>
        ))}
      </Card>
    </div>
  );
}

export function ContentTab({ s }: { s: SrvView }) {
  const route = useRoute();
  const view = (route.params.get('view') as View | null) ?? 'installed';
  const setView = (v: View) => navigate('servers', { id: s.id, tab: 'content', ...(v === 'installed' ? {} : { view: v }) });
  const [list, setList] = useState<SrvContent[] | null>(null);
  const [made, setMade] = useState<'datapack' | 'plugin'>('datapack');
  const [claude, setClaude] = useState(false);
  const load = useCallback(() => void api.servers.content(s.id).then(setList, () => setList([])), [s.id]);
  useEffect(load, [load]);
  useEvent<{ id: string }>('mcserver:content', (e) => e.id === s.id && load());
  // Files dropped in by hand are looked up on Modrinth once.
  useEffect(() => {
    if (list?.some((c) => !c.project && c.source === 'file')) void api.servers.identify(s.id).then(setList, () => undefined);
  }, [s.id, list === null]); // eslint-disable-line react-hooks/exhaustive-deps

  const target = useMemo(
    () => ({ key: `mcserver:${s.id}:${made}`, name: s.name, server: true, maker: made === 'plugin' ? { kind: 'paperPlugin' as const, label: 'a Paper plugin (Java)' } : { kind: 'datapack' as const, label: 'a data pack' } }),
    [s.id, s.name, made],
  );
  const addFile = async () => {
    const files = await api.app.pickFiles('Add mods, plugins (.jar) or data packs (.zip)');
    for (const f of files ?? []) await run('Not added', () => api.servers.addFile(s.id, f), 'Added');
    load();
  };
  const count = (k: SrvKind) => list?.filter((c) => c.kind === k).length ?? 0;

  return (
    <div className="space-y-4">
      <ClaudeSettingsModal open={claude} onClose={() => setClaude(false)} />
      <div className="flex flex-wrap items-center gap-2">
        <Segmented
          label="Show"
          value={view}
          onChange={setView}
          options={[
            { value: 'installed', label: `On this server${list ? ` (${list.length})` : ''}`, icon: Package },
            { value: 'browse', label: 'Find on Modrinth', icon: Search },
            { value: 'claude', label: 'Make with Claude', icon: Bot },
          ]}
        />
        <div className="flex-1" />
        <Button variant="secondary" icon={FilePlus2} onClick={() => void addFile()}>
          Add a file
        </Button>
      </div>
      {view === 'installed' && list && (
        <>
          <div className="flex flex-wrap gap-1.5 text-[12px] text-dim">
            {kinds(s).map((k) => (
              <Badge key={k} tone="neutral">
                {count(k)} {KIND_LABEL[k].toLowerCase()}
                {count(k) === 1 ? '' : 's'}
              </Badge>
            ))}
          </div>
          <Installed s={s} list={list} reload={load} />
        </>
      )}
      {view === 'browse' && <Browse s={s} onInstalled={load} />}
      {view === 'claude' && (
        <div className="space-y-3">
          {s.flavor === 'paper' && (
            <Segmented
              label="Make"
              value={made}
              onChange={setMade}
              options={[
                { value: 'datapack', label: 'A data pack' },
                { value: 'plugin', label: 'A Paper plugin (Java)' },
              ]}
            />
          )}
          <MakerTab game={target} onSettings={() => setClaude(true)} />
        </div>
      )}
    </div>
  );
}
