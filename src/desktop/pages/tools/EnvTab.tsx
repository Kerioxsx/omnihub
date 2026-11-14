// Your environment variables and PATH, the same ones System Properties
// edits. The system PATH is shown; only an administrator can change it.

import type { DevEnvironment } from '@shared/types';
import { AlertTriangle, ArrowDown, ArrowUp, FolderPlus, Plus, Save, Terminal, Trash, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader, Skeleton } from '../../components/ui/Card';
import { TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';

export function EnvTab() {
  const [env, setEnv] = useState<DevEnvironment | null>(null);
  const [path, setPath] = useState<string[]>([]);
  const [dirty, setDirty] = useState(false);
  const [name, setName] = useState('');
  const [value, setValue] = useState('');
  const take = useCallback((e: DevEnvironment) => {
    setEnv(e);
    setPath(e.userPath.map((p) => p.path));
    setDirty(false);
  }, []);
  const load = useCallback(() => void api.dev.environment().then(take, (e: unknown) => toast.error('Couldn’t read the environment', errorText(e))), [take]);
  useEffect(load, [load]);
  if (!env) return <Skeleton className="h-64 rounded-2xl" />;
  const exists = (p: string) => env.userPath.find((x) => x.path === p)?.exists ?? true;
  const edit = (next: string[]) => {
    setPath(next);
    setDirty(true);
  };
  const move = (i: number, d: number) => {
    const n = [...path];
    const [x] = n.splice(i, 1);
    n.splice(i + d, 0, x);
    edit(n);
  };
  const savePath = async () => {
    try {
      take(await api.dev.setPath(path));
      toast.success('PATH saved', 'Terminals and programs you open from now on see it.');
    } catch (e) {
      toast.error('Couldn’t save PATH', errorText(e));
    }
  };

  return (
    <div className="grid gap-4 xl:grid-cols-2">
      <Card className="p-5">
        <CardHeader
          icon={Terminal}
          title="Your PATH"
          subtitle="Folders Windows looks in for commands, in this order."
          actions={
            <div className="flex gap-2">
              <Button size="sm" icon={FolderPlus} onClick={() => void api.app.pickFolder('Add a folder to PATH').then((f) => f && !path.includes(f) && edit([...path, f]))}>
                Add folder
              </Button>
              <Button size="sm" variant="primary" icon={Save} disabled={!dirty} onClick={() => void savePath()}>
                Save
              </Button>
            </div>
          }
        />
        <div className="mt-3 max-h-[420px] overflow-auto rounded-xl border border-line">
          {path.map((p, i) => (
            <div key={`${p}-${i}`} className="flex items-center gap-2 border-b border-line px-3 py-1.5 last:border-0">
              {!exists(p) && <AlertTriangle size={14} className="shrink-0 text-warn" aria-label="This folder doesn’t exist" />}
              <span className={exists(p) ? 'min-w-0 flex-1 truncate font-mono text-[12px] text-fg' : 'min-w-0 flex-1 truncate font-mono text-[12px] text-faint line-through'} title={p}>
                {p}
              </span>
              <IconButton size="sm" icon={ArrowUp} label="Move up" disabled={i === 0} onClick={() => move(i, -1)} />
              <IconButton size="sm" icon={ArrowDown} label="Move down" disabled={i === path.length - 1} onClick={() => move(i, 1)} />
              <IconButton size="sm" icon={X} label="Remove" onClick={() => edit(path.filter((_, j) => j !== i))} />
            </div>
          ))}
        </div>
        {path.some((p) => !exists(p)) && <p className="mt-2 text-[12px] text-faint">Crossed-out folders don’t exist anymore; you can remove them.</p>}
        <details className="mt-4">
          <summary className="cursor-pointer text-[12.5px] text-dim">System PATH ({env.systemPath.length}, read-only)</summary>
          <div className="mt-2 max-h-56 overflow-auto rounded-xl border border-line">
            {env.systemPath.map((p, i) => (
              <div key={`${p.path}-${i}`} className="truncate border-b border-line px-3 py-1 font-mono text-[11.5px] text-dim last:border-0" title={p.path}>
                {p.path}
              </div>
            ))}
          </div>
        </details>
      </Card>
      <Card className="p-5">
        <CardHeader icon={Terminal} title="Your variables" subtitle="JAVA_HOME, ANDROID_HOME and the like. %OTHER_VARIABLES% inside work." />
        <div className="mt-3 max-h-[360px] overflow-auto rounded-xl border border-line">
          {env.userVars.map(([n, v]) => (
            <div key={n} className="flex items-center gap-2 border-b border-line px-3 py-1.5 last:border-0">
              <span className="w-40 shrink-0 truncate font-mono text-[12px] font-semibold text-fg">{n}</span>
              <button type="button" className="min-w-0 flex-1 truncate text-left font-mono text-[12px] text-dim hover:text-fg" title="Edit" onClick={() => { setName(n); setValue(v); }}>
                {v}
              </button>
              <IconButton
                size="sm"
                icon={Trash}
                label={`Remove ${n}`}
                onClick={async () => {
                  if (!(await confirm({ title: `Remove ${n}?`, confirmLabel: 'Remove', tone: 'danger' }))) return;
                  await api.dev.removeVar(n).then(take, (e: unknown) => toast.error('Couldn’t remove it', errorText(e)));
                }}
              />
            </div>
          ))}
        </div>
        <div className="mt-3 grid gap-2 sm:grid-cols-[180px_1fr_auto]">
          <TextInput value={name} onChange={(e) => setName(e.target.value)} placeholder="NAME" className="font-mono" />
          <TextInput value={value} onChange={(e) => setValue(e.target.value)} placeholder="Value" className="font-mono" />
          <Button
            icon={Plus}
            disabled={!name.trim()}
            onClick={() =>
              void api.dev.setVar(name.trim(), value).then(
                (e) => {
                  take(e);
                  setName('');
                  setValue('');
                  toast.success('Saved', 'New terminals see it.');
                },
                (e: unknown) => toast.error('Couldn’t save it', errorText(e)),
              )
            }
          >
            Save
          </Button>
        </div>
        <Callout tone="info" className="mt-3">
          Programs already open keep the old values until you restart them.
        </Callout>
      </Card>
    </div>
  );
}
