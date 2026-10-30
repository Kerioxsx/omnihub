// App updates through winget: every installed app with a newer version,
// updated one by one or all at once.

import type { AppUpdate } from '@shared/types';
import { ArrowUpCircle, CheckCircle2, Download, RefreshCw } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card, Skeleton } from '../../components/ui/Card';
import { Callout, EmptyState, ErrorState } from '../../components/ui/States';
import { useAsync } from '../../lib/hooks';
import { toast } from '../../state/toasts';

export function UpdatesPanel({ q }: { q: string }) {
  const list = useAsync(() => api.apps.updates(), []);
  const [busy, setBusy] = useState<Set<string>>(new Set());
  const [done, setDone] = useState<Set<string>>(new Set());

  const update = async (u: AppUpdate) => {
    setBusy((b) => new Set(b).add(u.id));
    try {
      await api.apps.update(u.id);
      setDone((d) => new Set(d).add(u.id));
      return true;
    } catch (e) {
      toast.error(`${u.name} wasn’t updated`, errorText(e));
      return false;
    } finally {
      setBusy((b) => {
        const n = new Set(b);
        n.delete(u.id);
        return n;
      });
    }
  };
  const updateAll = async (items: AppUpdate[]) => {
    let ok = 0;
    for (const u of items) if (await update(u)) ok++;
    if (ok) toast.success(`${ok} app${ok === 1 ? '' : 's'} updated`);
  };

  if (list.error) return <ErrorState title="Couldn’t check for updates" error={list.error} onRetry={list.reload} />;
  if (!list.data)
    return (
      <div className="space-y-2">
        <div className="text-[12.5px] text-faint">Asking winget which apps have a newer version… (takes a few seconds)</div>
        {[0, 1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-14 rounded-xl" />
        ))}
      </div>
    );
  const needle = q.trim().toLowerCase();
  const items = list.data.filter((u) => !needle || `${u.name} ${u.id}`.toLowerCase().includes(needle));
  const pending = items.filter((u) => !done.has(u.id));
  if (list.data.length === 0) return <EmptyState icon={CheckCircle2} title="Everything is up to date" description="winget found no newer version of your apps." />;

  return (
    <div className="space-y-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="text-[13px] text-dim">
          {pending.length} update{pending.length === 1 ? '' : 's'} available
        </div>
        <div className="flex gap-2">
          <Button size="sm" variant="ghost" icon={RefreshCw} onClick={list.reload}>
            Check again
          </Button>
          <Button size="sm" variant="primary" icon={Download} disabled={!pending.length || busy.size > 0} onClick={() => void updateAll(pending)}>
            Update all
          </Button>
        </div>
      </div>
      <Card className="divide-y divide-line p-0">
        {items.map((u) => (
          <div key={u.id} className="flex items-center gap-3 px-4 py-3">
            <ArrowUpCircle size={20} className={done.has(u.id) ? 'text-good' : 'text-accent'} />
            <div className="min-w-0 flex-1">
              <div className="truncate text-[13.5px] font-medium text-fg">{u.name}</div>
              <div className="truncate text-[12px] text-faint">
                {u.id} · {u.version} → <span className="text-fg">{u.available}</span>
              </div>
            </div>
            {done.has(u.id) ? (
              <span className="text-[12.5px] text-good">Updated</span>
            ) : (
              <Button size="sm" variant="secondary" icon={Download} loading={busy.has(u.id)} disabled={busy.has(u.id)} onClick={() => void update(u)}>
                Update
              </Button>
            )}
          </div>
        ))}
      </Card>
      <Callout tone="info" className="text-[12.5px]">
        Updates come from winget, Windows’ own package manager. Some installers ask for administrator rights or close the app while they update it.
      </Callout>
    </div>
  );
}
