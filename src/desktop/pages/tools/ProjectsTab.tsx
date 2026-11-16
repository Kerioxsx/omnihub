// Your Git projects at a glance: branch, uncommitted changes, ahead and
// behind, last commit. Found in the folders you add; only read.

import { formatRelative } from '@shared/format';
import type { DevProject } from '@shared/types';
import { ArrowDown, ArrowUp, Code2, FolderOpen, FolderPlus, GitBranch, RefreshCw, SquareTerminal, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Skeleton } from '../../components/ui/Card';
import { SearchInput } from '../../components/ui/Form';
import { EmptyState } from '../../components/ui/States';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';

export function ProjectsTab() {
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const roots = settings?.dev.roots ?? [];
  const [list, setList] = useState<DevProject[] | null>(null);
  const [q, setQ] = useState('');
  const load = useCallback(() => {
    if (!roots.length) return setList([]);
    setList(null);
    void api.dev.projects(roots).then(setList, (e: unknown) => {
      setList([]);
      toast.error('Couldn’t read the projects', errorText(e));
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [roots.join('|')]);
  useEffect(load, [load]);
  const addRoot = async () => {
    const f = await api.app.pickFolder('Choose a folder with your projects');
    if (f && !roots.includes(f)) void update({ dev: { roots: [...roots, f] } });
  };
  const open = (p: DevProject, app: 'code' | 'terminal' | 'explorer') => void api.dev.openIn(p.path, app).catch((e: unknown) => toast.error('Couldn’t open it', errorText(e)));
  const shown = (list ?? []).filter((p) => !q || `${p.name} ${p.branch}`.toLowerCase().includes(q.toLowerCase()));

  return (
    <Card className="p-5">
      <CardHeader
        icon={GitBranch}
        title="Projects"
        subtitle="Every Git project in the folders you add: what’s changed and what isn’t pushed yet."
        actions={
          <div className="flex gap-2">
            <IconButton icon={RefreshCw} label="Refresh" onClick={load} />
            <Button size="sm" icon={FolderPlus} onClick={() => void addRoot()}>
              Add folder
            </Button>
          </div>
        }
      />
      {roots.length > 0 && (
        <div className="mt-3 flex flex-wrap gap-1.5">
          {roots.map((r) => (
            <span key={r} className="flex items-center gap-1 rounded-full bg-surface-2 py-0.5 pl-3 pr-1 font-mono text-[11.5px] text-dim">
              {r}
              <IconButton size="sm" icon={X} label={`Stop looking in ${r}`} onClick={() => void update({ dev: { roots: roots.filter((x) => x !== r) } })} />
            </span>
          ))}
        </div>
      )}
      <div className="mt-4">
        {!roots.length ? (
          <EmptyState icon={GitBranch} title="Add the folder where your projects are" description="For example C:\Users\you\source or Documents\GitHub." action={<Button icon={FolderPlus} onClick={() => void addRoot()}>Add folder</Button>} />
        ) : !list ? (
          <Skeleton className="h-48 rounded-xl" />
        ) : !list.length ? (
          <EmptyState compact icon={GitBranch} title="No Git projects found there" />
        ) : (
          <>
            <SearchInput value={q} onChange={setQ} placeholder="Find a project" className="mb-3 w-64" />
            <div className="grid gap-2 lg:grid-cols-2">
              {shown.map((p) => (
                <div key={p.path} className="rounded-xl border border-line bg-surface p-3">
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0">
                      <div className="truncate text-[14px] font-semibold text-fg">{p.name}</div>
                      <div className="truncate font-mono text-[11px] text-faint">{p.path}</div>
                    </div>
                    <div className="flex shrink-0">
                      <IconButton size="sm" icon={Code2} label="Open in VS Code" onClick={() => open(p, 'code')} />
                      <IconButton size="sm" icon={SquareTerminal} label="Open a terminal here" onClick={() => open(p, 'terminal')} />
                      <IconButton size="sm" icon={FolderOpen} label="Open the folder" onClick={() => open(p, 'explorer')} />
                    </div>
                  </div>
                  <div className="mt-2 flex flex-wrap items-center gap-1.5">
                    <Badge icon={GitBranch}>{p.branch || '—'}</Badge>
                    {p.changes > 0 ? <Badge tone="warn">{p.changes} changed</Badge> : !p.note && <Badge tone="good">clean</Badge>}
                    {p.ahead > 0 && <Badge tone="accent" icon={ArrowUp}>{p.ahead} to push</Badge>}
                    {p.behind > 0 && <Badge tone="info" icon={ArrowDown}>{p.behind} to pull</Badge>}
                    {!p.upstream && !p.note && <Badge>not on a remote</Badge>}
                  </div>
                  {p.lastMessage && (
                    <div className="mt-2 truncate text-[12px] text-dim">
                      {p.lastMessage} <span className="text-faint">· {formatRelative(p.lastCommit)}</span>
                    </div>
                  )}
                  {p.note && <div className="mt-2 text-[12px] text-faint">{p.note}</div>}
                </div>
              ))}
            </div>
          </>
        )}
      </div>
    </Card>
  );
}
