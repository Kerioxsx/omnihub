// Backups of chosen folders to another drive: new and changed files are
// copied; nothing in the backup is ever deleted.

import { formatRelative } from '@shared/format';
import type { BackupJob, BackupRun, BackupSchedule } from '@shared/types';
import { Archive, CheckCircle2, FolderOpen, HardDriveDownload, Pencil, Play, Plus, Square, Trash2, TriangleAlert, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader, Spinner } from '../../components/ui/Card';
import { Field, Select, Switch, TextInput } from '../../components/ui/Form';
import { Callout, EmptyState } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';

const SCHEDULE: Record<BackupSchedule, string> = { daily: 'Every day', weekly: 'Every week', whenConnected: 'When the drive is plugged in', manual: 'Only when I click' };

const blank = (): BackupJob => ({ id: '', name: '', sources: [], dest: '', schedule: 'whenConnected', enabled: true, lastRun: null, lastResult: null, lastOk: true });

function Editor({ job, onDone }: { job: BackupJob; onDone: () => void }) {
  const [j, setJ] = useState(job);
  const [busy, setBusy] = useState(false);
  const pick = async (title: string) => (await api.app.pickFolder(title)) ?? null;
  const save = async () => {
    setBusy(true);
    try {
      await api.backups.save(j);
      toast.success(job.id ? 'Backup saved' : 'Backup made', j.schedule === 'manual' ? 'Run it whenever you like.' : `${SCHEDULE[j.schedule]}, by itself.`);
      onDone();
    } catch (e) {
      toast.error('Not saved', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Card className="space-y-4 p-5">
      <div className="flex items-center justify-between">
        <div className="font-medium">{job.id ? 'Edit backup' : 'New backup'}</div>
        <IconButton icon={X} label="Close" onClick={onDone} />
      </div>
      <Field label="Name">
        <TextInput value={j.name} onChange={(e) => setJ({ ...j, name: e.target.value })} placeholder="Documents & photos" autoFocus />
      </Field>
      <Field label="Folders to back up">
        <div className="space-y-1.5">
          {j.sources.map((s) => (
            <div key={s} className="flex items-center gap-2 rounded-lg border border-line bg-surface px-3 py-1.5 text-[13px]">
              <FolderOpen size={14} className="shrink-0 text-faint" />
              <span className="flex-1 truncate">{s}</span>
              <IconButton icon={X} size="sm" label={`Remove ${s}`} onClick={() => setJ({ ...j, sources: j.sources.filter((x) => x !== s) })} />
            </div>
          ))}
          <Button size="sm" variant="secondary" icon={Plus} onClick={() => void pick('Pick a folder to back up').then((p) => p && !j.sources.includes(p) && setJ({ ...j, sources: [...j.sources, p] }))}>
            Add a folder
          </Button>
        </div>
      </Field>
      <Field label="Back up to" hint="A USB drive, a second drive or a network folder. The copies go into “OmniHub Backups” there.">
        <div className="flex gap-2">
          <TextInput value={j.dest} onChange={(e) => setJ({ ...j, dest: e.target.value })} placeholder="E:\" className="flex-1" />
          <Button variant="secondary" icon={FolderOpen} onClick={() => void pick('Pick where the backup goes').then((p) => p && setJ({ ...j, dest: p }))}>
            Pick
          </Button>
        </div>
      </Field>
      <Field label="When">
        <Select value={j.schedule} onChange={(e) => setJ({ ...j, schedule: e.target.value as BackupSchedule })}>
          {Object.entries(SCHEDULE).map(([v, l]) => (
            <option key={v} value={v}>
              {l}
            </option>
          ))}
        </Select>
      </Field>
      <div className="flex justify-end gap-2">
        <Button variant="ghost" onClick={onDone}>
          Cancel
        </Button>
        <Button loading={busy} onClick={() => void save()}>
          Save
        </Button>
      </div>
    </Card>
  );
}

export function BackupsTab() {
  const [data, setData] = useState<{ jobs: BackupJob[]; running: string | null } | null>(null);
  const [editing, setEditing] = useState<BackupJob | null>(null);
  const load = () => void api.backups.list().then(setData, () => undefined);
  useEffect(load, []);
  useEvent('backup:changed', load);
  useEvent<BackupRun>('backup:done', (r) => {
    load();
    if (r.missingDest) toast.warn(`${r.name} didn’t run`, 'Its drive or folder isn’t there. Plug it in and it runs by itself.');
    else if (r.failed) toast.warn(`${r.name}: ${r.failed} files couldn’t be copied`, 'Usually files that were open. They’re tried again next time.');
    else toast.success(`${r.name} is backed up`, `${r.copied} new or changed files copied.`);
  });
  if (!data) return <Spinner />;
  return (
    <div className="space-y-4">
      {editing ? (
        <Editor job={editing} onDone={() => setEditing(null)} />
      ) : (
        <Card className="p-5">
          <CardHeader
            icon={Archive}
            title="Backups"
            subtitle="Your folders copied to another drive: only new and changed files each time, and nothing in the backup is ever deleted, so a file deleted or broken on the PC is still there."
            actions={
              <Button size="sm" icon={Plus} onClick={() => setEditing(blank())}>
                New backup
              </Button>
            }
          />
          <div className="mt-4 space-y-2">
            {data.jobs.length === 0 && <EmptyState compact icon={HardDriveDownload} title="No backups yet" description="For example: Documents and Pictures to a USB drive, whenever it’s plugged in." />}
            {data.jobs.map((j) => {
              const running = data.running === j.id;
              return (
                <div key={j.id} className="flex flex-wrap items-center gap-3 rounded-xl border border-line bg-surface px-4 py-3">
                  <Switch checked={j.enabled} onChange={(enabled) => void api.backups.save({ ...j, enabled }).then(load, (e: unknown) => toast.error('Couldn’t change it', errorText(e)))} label={`${j.name} on`} />
                  <div className="min-w-[200px] flex-1">
                    <div className="font-medium">{j.name}</div>
                    <div className="truncate text-[12px] text-faint">
                      {j.sources.length} folder{j.sources.length === 1 ? '' : 's'} → {j.dest} · {SCHEDULE[j.schedule]}
                    </div>
                    <div className="mt-0.5 flex items-center gap-1.5 text-[12px]">
                      {running ? (
                        <span className="flex items-center gap-1.5 text-accent">
                          <Spinner size={12} /> Backing up…
                        </span>
                      ) : j.lastRun ? (
                        <span className={j.lastOk ? 'text-dim' : 'text-warn'}>
                          {j.lastOk ? <CheckCircle2 size={12} className="mr-1 inline text-good" /> : <TriangleAlert size={12} className="mr-1 inline" />}
                          {formatRelative(j.lastRun)} · {j.lastResult}
                        </span>
                      ) : (
                        <span className="text-faint">{j.lastResult ?? 'Not run yet'}</span>
                      )}
                    </div>
                  </div>
                  {running ? (
                    <Button size="sm" variant="secondary" icon={Square} onClick={() => void api.backups.cancel()}>
                      Stop
                    </Button>
                  ) : (
                    <Button size="sm" variant="secondary" icon={Play} disabled={!!data.running} onClick={() => void api.backups.run(j.id).then(load, (e: unknown) => toast.error('Couldn’t start it', errorText(e)))}>
                      Back up now
                    </Button>
                  )}
                  <IconButton icon={Pencil} label={`Edit ${j.name}`} onClick={() => setEditing(j)} />
                  <IconButton
                    icon={Trash2}
                    label={`Delete ${j.name}`}
                    onClick={async () => {
                      if (await confirm({ title: `Delete “${j.name}”?`, description: 'Only the schedule goes; the copies already on the drive stay.', confirmLabel: 'Delete', tone: 'danger' })) void api.backups.remove(j.id).then(load);
                    }}
                  />
                </div>
              );
            })}
          </div>
        </Card>
      )}
      <Callout title="Good to know">Files open in other programs are skipped and tried again next time. Copies keep their folders, so finding a file in the backup is like finding it on the PC.</Callout>
    </div>
  );
}
