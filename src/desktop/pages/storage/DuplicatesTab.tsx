import { basename, formatBytes, formatDate, formatNumber } from '@shared/format';
import type { DupeGroup, DupeJobProgress } from '@shared/types';
import { motion } from 'motion/react';
import { CopyCheck, Files, Play, Trash, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { FileIcon } from '../../components/FileIcon';
import { Button } from '../../components/ui/Button';
import { Badge, Card } from '../../components/ui/Card';
import { Segmented } from '../../components/ui/Form';
import { ProgressBar } from '../../components/ui/Progress';
import { EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useEvent } from '../../lib/hooks';
import { useStorage } from '../../state/storage';
import { toast } from '../../state/toasts';
import { deleteItems } from './actions';

const MIN_SIZES = [
  { value: '1048576', label: '1 MB' },
  { value: '10485760', label: '10 MB' },
  { value: '104857600', label: '100 MB' },
  { value: '1073741824', label: '1 GB' },
] as const;
type MinSize = (typeof MIN_SIZES)[number]['value'];

export function DuplicatesTab({ scanId }: { scanId: string }) {
  const nodeId = useStorage((s) => s.nodeByScan[scanId] ?? 0);
  const [minSize, setMinSize] = useState<MinSize>('10485760');
  const [scope, setScope] = useState<'all' | 'here'>('all');
  const [jobId, setJobId] = useState<string | null>(null);
  const [progress, setProgress] = useState<DupeJobProgress | null>(null);
  const [groups, setGroups] = useState<DupeGroup[] | null>(null);
  const [remove, setRemove] = useState<Set<string>>(new Set());
  const [shown, setShown] = useState(40);
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);

  const stopPolling = () => {
    if (timer.current) clearInterval(timer.current);
    timer.current = null;
  };
  useEffect(() => stopPolling, []);

  const finish = async (id: string) => {
    stopPolling();
    const result = await api.storage.duplicatesResult(id);
    if (!result) return;
    setGroups(result);
    setJobId(null);
    setProgress(null);
    // Keep the oldest copy of each group, mark the rest for removal.
    setRemove(new Set(result.flatMap((g) => g.files.slice(1).map((f) => f.path))));
    setShown(40);
  };

  useEvent<{ jobId: string }>('storage:dupes-done', (p) => {
    if (p.jobId === jobId) void finish(p.jobId);
  });

  const start = async () => {
    setGroups(null);
    try {
      const id = await api.storage.duplicatesStart(scanId, { minSize: Number(minSize), under: scope === 'here' ? nodeId : 0, maxGroups: 500 });
      setJobId(id);
      stopPolling();
      timer.current = setInterval(async () => {
        const p = await api.storage.duplicatesProgress(id);
        if (!p) return;
        setProgress(p);
        if (p.done) void finish(id);
      }, 250);
    } catch (e) {
      toast.error('Could not look for duplicates', errorText(e));
    }
  };

  const cancel = async () => {
    if (!jobId) return;
    stopPolling();
    await api.storage.duplicatesCancel(jobId).catch(() => undefined);
    setJobId(null);
    setProgress(null);
  };

  const toggle = (g: DupeGroup, path: string) => {
    const next = new Set(remove);
    if (next.has(path)) next.delete(path);
    else {
      const kept = g.files.filter((f) => !next.has(f.path) && f.path !== path);
      if (!kept.length) {
        toast.warn('Keep at least one copy', 'Every file in a group would be deleted.');
        return;
      }
      next.add(path);
    }
    setRemove(next);
  };

  const selected = useMemo(() => (groups ?? []).flatMap((g) => g.files.filter((f) => remove.has(f.path)).map((f) => ({ path: f.path, isDir: false, size: g.size }))), [groups, remove]);
  const selectedSize = selected.reduce((a, s) => a + s.size, 0);
  const wasted = (groups ?? []).reduce((a, g) => a + g.wasted, 0);

  const deleteSelected = async () => {
    const res = await deleteItems(selected, false, { title: `Move ${formatNumber(selected.length)} duplicate${selected.length === 1 ? '' : 's'} to the Recycle Bin?`, description: `${formatBytes(selectedSize)} of extra copies. One copy of every file is kept.` });
    if (!res) return;
    const gone = new Set(res.filter((r) => r.ok).map((r) => r.path));
    setGroups((gs) => (gs ?? []).map((g) => ({ ...g, files: g.files.filter((f) => !gone.has(f.path)) })).filter((g) => g.files.length > 1).map((g) => ({ ...g, wasted: g.size * (g.files.length - 1) })));
    setRemove((r) => new Set([...r].filter((p) => !gone.has(p))));
  };

  const phaseLabel = progress?.phase === 2 ? 'Comparing full contents' : progress?.phase === 3 ? 'Finishing' : 'Quick check of first & last 64 KB';
  const frac = progress ? (progress.phase === 1 ? progress.filesDone / Math.max(1, progress.filesTotal) * 0.4 : progress.phase === 2 ? 0.4 + (progress.bytesDone / Math.max(1, progress.bytesTotal)) * 0.6 : 1) : 0;

  return (
    <div className="flex min-h-[460px] flex-1 flex-col gap-3">
      <Card className="flex flex-wrap items-center gap-4 px-5 py-4">
        <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-accent-soft text-accent">
          <Files size={19} aria-hidden />
        </div>
        <div className="min-w-[220px] flex-1">
          <div className="text-[14px] font-semibold text-fg">Find identical files</div>
          <div className="text-[12.5px] text-dim">Files are grouped by size, then compared by content hash. Nothing is deleted without your confirmation.</div>
        </div>
        <div className="flex items-center gap-2 text-[12.5px] text-dim">
          At least
          <Segmented size="sm" label="Minimum size" value={minSize} onChange={setMinSize} options={MIN_SIZES.map((m) => ({ value: m.value, label: m.label }))} />
        </div>
        <Segmented
          size="sm"
          label="Scope"
          value={scope}
          onChange={setScope}
          options={[
            { value: 'all', label: 'Whole scan' },
            { value: 'here', label: 'Current folder', disabled: !nodeId },
          ]}
        />
        {jobId ? (
          <Button icon={X} onClick={cancel}>
            Cancel
          </Button>
        ) : (
          <Button variant="primary" icon={Play} onClick={start}>
            {groups ? 'Search again' : 'Find duplicates'}
          </Button>
        )}
      </Card>

      {jobId && (
        <motion.div initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }}>
          <Card className="px-5 py-4">
            <div className="mb-2 flex justify-between text-[12.5px]">
              <span className="font-medium text-fg">{phaseLabel}…</span>
              <span className="tabular text-faint">
                {progress ? (progress.phase === 1 ? `${formatNumber(progress.filesDone)} / ${formatNumber(progress.filesTotal)} candidates` : `${formatBytes(progress.bytesDone)} / ${formatBytes(progress.bytesTotal)}`) : 'Starting…'}
              </span>
            </div>
            <ProgressBar value={progress ? frac : null} label="Duplicate search progress" />
          </Card>
        </motion.div>
      )}

      {!jobId && !groups && <EmptyState icon={CopyCheck} title="Look for wasted space" description="Duplicate downloads, photos imported twice and copied ISO files add up. Pick a minimum size and start the search." compact />}

      {groups && groups.length === 0 && <EmptyState icon={CopyCheck} title="No duplicates found" description={`No identical files over ${MIN_SIZES.find((m) => m.value === minSize)?.label} in this scan.`} />}

      {groups && groups.length > 0 && (
        <>
          <div className="flex items-center justify-between gap-3">
            <div className="text-[13px] text-dim">
              <span className="font-semibold text-fg tabular">{formatNumber(groups.length)}</span> groups · <span className="font-semibold text-fg tabular">{formatBytes(wasted)}</span> in extra copies
            </div>
            <Button variant="danger" icon={Trash} disabled={!selected.length} onClick={deleteSelected}>
              Delete selected ({formatNumber(selected.length)} · {formatBytes(selectedSize)})
            </Button>
          </div>
          <div className="min-h-0 flex-1 space-y-2.5 overflow-y-auto pb-6 pr-1">
            {groups.slice(0, shown).map((g) => (
              <Card key={g.hash + g.size} className="px-4 py-3">
                <div className="mb-2 flex items-center gap-2.5">
                  <FileIcon name={basename(g.files[0].path)} isDir={false} />
                  <span className="truncate text-[13.5px] font-semibold text-fg">{basename(g.files[0].path)}</span>
                  <span className="text-[12px] text-faint tabular">
                    {formatBytes(g.size)} × {g.files.length}
                  </span>
                  <Badge tone="warn" className="ml-auto">
                    {formatBytes(g.wasted)} wasted
                  </Badge>
                </div>
                <div className="space-y-1">
                  {g.files.map((f) => {
                    const del = remove.has(f.path);
                    return (
                      <div key={f.path} className={cx('flex items-center gap-3 rounded-lg px-2 py-1.5 transition-colors', del ? 'bg-bad/6' : 'bg-surface')}>
                        <Segmented
                          size="sm"
                          label={`Keep or delete ${f.path}`}
                          value={del ? 'delete' : 'keep'}
                          onChange={() => toggle(g, f.path)}
                          options={[
                            { value: 'keep', label: 'Keep' },
                            { value: 'delete', label: 'Delete' },
                          ]}
                        />
                        <span className={cx('min-w-0 flex-1 truncate font-mono text-[11.5px]', del ? 'text-faint line-through decoration-bad/50' : 'text-dim')} title={f.path}>
                          {f.path}
                        </span>
                        <span className="shrink-0 text-[11.5px] text-faint tabular">{formatDate(f.modified)}</span>
                      </div>
                    );
                  })}
                </div>
              </Card>
            ))}
            {groups.length > shown && (
              <div className="flex justify-center pt-1">
                <Button size="sm" onClick={() => setShown(shown + 40)}>
                  Show {Math.min(40, groups.length - shown)} more groups
                </Button>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
