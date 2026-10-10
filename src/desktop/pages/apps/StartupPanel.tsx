import type { StartupItem, StartupLocation } from '@shared/types';
import { FolderOpen, Power, ShieldCheck } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../../api';
import { IconButton } from '../../components/ui/Button';
import { Badge, Card, Skeleton } from '../../components/ui/Card';
import { Switch } from '../../components/ui/Form';
import { Callout, EmptyState, ErrorState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useAsync } from '../../lib/hooks';
import { toast } from '../../state/toasts';

const WHERE: Record<StartupLocation, string> = {
  runUser: 'Registry · you',
  runMachine: 'Registry · all users',
  runMachine32: 'Registry · all users (32-bit)',
  folderUser: 'Startup folder · you',
  folderCommon: 'Startup folder · all users',
};

/** Apps that start with Windows, switched on and off like Task Manager does. */
export function StartupPanel({ q }: { q: string }) {
  const items = useAsync(() => api.apps.startup(), []);
  const [busy, setBusy] = useState<string | null>(null);
  const toggle = async (i: StartupItem, on: boolean) => {
    setBusy(i.id);
    try {
      items.setData(await api.apps.setStartup(i.id, on));
      toast.success(on ? `${i.displayName} will start with Windows` : `${i.displayName} won't start with Windows`, 'Takes effect the next time you sign in.');
    } catch (e) {
      toast.error(`Could not change ${i.displayName}`, errorText(e));
    } finally {
      setBusy(null);
    }
  };
  if (items.error) return <ErrorState title="Could not read the startup apps" error={items.error} onRetry={items.reload} />;
  if (!items.data)
    return (
      <div className="space-y-2">
        {[0, 1, 2, 3, 4].map((i) => (
          <Skeleton key={i} className="h-14 rounded-xl" />
        ))}
      </div>
    );
  const needle = q.trim().toLowerCase();
  const list = items.data.filter((i) => !needle || `${i.displayName} ${i.command}`.toLowerCase().includes(needle));
  const on = items.data.filter((i) => i.enabled).length;
  return (
    <div className="space-y-3">
      <Callout tone="info" icon={Power} title={`${on} of ${items.data.length} apps start when you sign in`}>
        Fewer startup apps means Windows is ready sooner. Switching one off doesn't uninstall it — you can still open it from the Start menu. Entries for all users ask for administrator approval.
      </Callout>
      {list.length === 0 ? (
        <EmptyState icon={Power} title="No startup apps match" />
      ) : (
        <Card className="divide-y divide-line p-1.5">
          {list.map((i) => (
            <div key={i.id} className="flex items-center gap-3 px-3 py-2.5">
              <span className={cx('flex h-9 w-9 shrink-0 items-center justify-center rounded-xl', i.enabled ? 'bg-accent-soft text-accent' : 'bg-surface-3 text-faint')}>
                <Power size={16} aria-hidden />
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className={cx('truncate text-[13.5px] font-medium', i.enabled ? 'text-fg' : 'text-dim')}>{i.displayName}</span>
                  <Badge>{WHERE[i.location]}</Badge>
                  {i.needsAdmin && <ShieldCheck size={13} className="shrink-0 text-faint" aria-label="Asks for administrator approval" />}
                </div>
                <div className="truncate font-mono text-[11px] text-faint" title={i.command}>
                  {i.command}
                </div>
              </div>
              {i.target && <IconButton icon={FolderOpen} label="Show the file" size="sm" onClick={() => i.target && void api.app.revealPath(i.target).catch((e: unknown) => toast.error('Could not show the file', errorText(e)))} />}
              <Switch checked={i.enabled} disabled={busy === i.id} onChange={(v) => void toggle(i, v)} label={`Start ${i.displayName} with Windows`} />
            </div>
          ))}
        </Card>
      )}
    </div>
  );
}
