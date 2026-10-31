// Window layouts: save where your windows are ("Work", "Gaming") and put
// them back with one click, starting the programs that aren't open.

import { formatRelative } from '@shared/format';
import type { WindowLayout } from '@shared/types';
import { LayoutDashboard, Play, Plus, Trash } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader, Skeleton } from '../../components/ui/Card';
import { Checkbox, TextInput } from '../../components/ui/Form';
import { EmptyState } from '../../components/ui/States';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';

/** The windows as small boxes, so a layout is recognisable at a glance. */
function Thumb({ layout }: { layout: WindowLayout }) {
  const ws = layout.windows;
  if (!ws.length) return null;
  const minX = Math.min(...ws.map((w) => w.left));
  const minY = Math.min(...ws.map((w) => w.top));
  const maxX = Math.max(...ws.map((w) => w.right));
  const maxY = Math.max(...ws.map((w) => w.bottom));
  const W = Math.max(1, maxX - minX);
  const H = Math.max(1, maxY - minY);
  return (
    <div className="relative h-24 w-full overflow-hidden rounded-lg bg-surface-2">
      {ws.map((w, i) => (
        <div
          key={i}
          title={w.title}
          className="absolute overflow-hidden rounded border border-accent/40 bg-accent-soft px-1 text-[9.5px] leading-tight text-accent"
          style={{ left: `${((w.left - minX) / W) * 100}%`, top: `${((w.top - minY) / H) * 100}%`, width: `${((w.right - w.left) / W) * 100}%`, height: `${((w.bottom - w.top) / H) * 100}%` }}
        >
          {w.name}
        </div>
      ))}
    </div>
  );
}

export function LayoutsTab() {
  const [list, setList] = useState<WindowLayout[] | null>(null);
  const [name, setName] = useState('');
  const [launch, setLaunch] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  const load = useCallback(() => void api.layouts.list().then(setList, () => setList([])), []);
  useEffect(load, [load]);

  const save = async () => {
    setBusy('save');
    try {
      const l = await api.layouts.save(name);
      toast.success(`Saved “${l.name}”`, `${l.windows.length} windows`);
      setName('');
      load();
    } catch (e) {
      toast.error('Couldn’t save the layout', errorText(e));
    } finally {
      setBusy(null);
    }
  };
  const apply = async (l: WindowLayout) => {
    setBusy(l.name);
    try {
      const r = await api.layouts.apply(l.name, launch);
      const started = r.launched.length ? `, started ${r.launched.join(', ')}` : '';
      if (r.missing.length) toast.info(`Placed ${r.placed} window${r.placed === 1 ? '' : 's'}${started}`, `Not open: ${r.missing.join(', ')}`);
      else toast.success(`“${l.name}” is back`, `Placed ${r.placed} window${r.placed === 1 ? '' : 's'}${started}`);
    } catch (e) {
      toast.error('Couldn’t put the windows back', errorText(e));
    } finally {
      setBusy(null);
    }
  };
  const remove = async (l: WindowLayout) => {
    if (!(await confirm({ title: `Delete “${l.name}”?`, confirmLabel: 'Delete', tone: 'danger' }))) return;
    await api.layouts.remove(l.name).catch(() => undefined);
    load();
  };

  return (
    <div className="flex flex-col gap-4">
      <Card className="p-5">
        <CardHeader icon={LayoutDashboard} title="Save how your windows are arranged" subtitle="Put your windows where you like them (any monitor), name the layout, save. One click puts them back later." />
        <div className="mt-4 flex flex-wrap items-center gap-2">
          <TextInput value={name} onChange={(e) => setName(e.target.value)} placeholder="Work, Gaming, Streaming…" className="w-64" maxLength={40} onKeyDown={(e) => e.key === 'Enter' && name.trim() && void save()} />
          <Button variant="primary" icon={Plus} loading={busy === 'save'} disabled={!name.trim()} onClick={() => void save()}>
            Save layout
          </Button>
          <div className="ml-auto">
            <Checkbox checked={launch} onChange={setLaunch} label="Start programs that aren’t open" />
          </div>
        </div>
      </Card>
      {!list ? (
        <Skeleton className="h-40 rounded-2xl" />
      ) : !list.length ? (
        <EmptyState icon={LayoutDashboard} title="No layouts yet" description="Arrange your windows, then save them above." />
      ) : (
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {list.map((l) => (
            <Card key={l.name} className="flex flex-col gap-3 p-4">
              <div className="flex items-center justify-between gap-2">
                <div className="min-w-0">
                  <div className="truncate text-[14px] font-semibold text-fg">{l.name}</div>
                  <div className="text-[12px] text-faint">
                    {l.windows.length} windows · saved {formatRelative(l.savedAt)}
                  </div>
                </div>
                <IconButton size="sm" icon={Trash} label="Delete" onClick={() => void remove(l)} />
              </div>
              <Thumb layout={l} />
              <Button variant="primary" icon={Play} loading={busy === l.name} disabled={!!busy} onClick={() => void apply(l)}>
                Put windows back
              </Button>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
