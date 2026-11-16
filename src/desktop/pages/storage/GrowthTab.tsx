import { formatBytes, formatRelative } from '@shared/format';
import type { GrowthItem } from '@shared/types';
import { FolderOpen, TrendingDown, TrendingUp } from 'lucide-react';
import { api } from '../../api';
import { FileIcon } from '../../components/FileIcon';
import { Button } from '../../components/ui/Button';
import { Badge, Card, Skeleton } from '../../components/ui/Card';
import { EmptyState, ErrorState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useAsync } from '../../lib/hooks';
import { useStorage } from '../../state/storage';
import { openItemMenu } from './actions';

function signed(n: number): string {
  return `${n >= 0 ? '+' : '−'}${formatBytes(Math.abs(n))}`;
}

function Rows({ items, max, onOpen }: { items: GrowthItem[]; max: number; onOpen: (i: GrowthItem) => void }) {
  return (
    <div className="space-y-0.5">
      {items.map((i) => {
        const up = i.delta > 0;
        const name = i.path.split(/[\\/]/).filter(Boolean).pop() ?? i.path;
        return (
          <div
            key={i.path}
            role="listitem"
            tabIndex={0}
            onDoubleClick={() => onOpen(i)}
            onContextMenu={(e) => openItemMenu(e, { path: i.path, name, isDir: true, size: i.after })}
            className="grid grid-cols-[minmax(0,1fr)_220px_110px_32px] items-center gap-3 rounded-lg px-3 py-2 outline-none transition-colors hover:bg-surface-2 focus-visible:bg-surface-3"
          >
            <div className="flex min-w-0 items-center gap-2.5">
              <FileIcon name={name} isDir />
              <div className="min-w-0">
                <div className="flex items-center gap-1.5 truncate text-[13px] text-fg">
                  {name}
                  {i.isNew && <Badge tone="accent">new</Badge>}
                  {!i.node && <Badge>gone</Badge>}
                </div>
                <div className="truncate font-mono text-[10.5px] text-faint">{i.path}</div>
              </div>
            </div>
            <div className="flex items-center gap-2">
              <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-surface-3">
                <div className={cx('h-full rounded-full', up ? 'bg-warn' : 'bg-good')} style={{ width: `${Math.max(3, (Math.abs(i.delta) / max) * 100)}%` }} />
              </div>
              <span className={cx('w-[78px] text-right text-[12.5px] font-semibold tabular', up ? 'text-warn' : 'text-good')}>{signed(i.delta)}</span>
            </div>
            <span className="text-right text-[12px] tabular text-faint">
              {formatBytes(i.before)} → {formatBytes(i.after)}
            </span>
            {i.node != null ? (
              <button type="button" onClick={() => onOpen(i)} className="text-faint hover:text-fg" aria-label={`Open ${name}`} title="Open in Explorer tab">
                <FolderOpen size={15} />
              </button>
            ) : (
              <span />
            )}
          </div>
        );
      })}
    </div>
  );
}

/** Folders that grew or shrank since the previous scan of this drive. */
export function GrowthTab({ scanId }: { scanId: string }) {
  const version = useStorage((s) => s.version);
  const report = useAsync(() => api.storage.growth(scanId, 15), [scanId, version]);
  const open = (i: GrowthItem) => {
    if (i.node == null) return;
    useStorage.getState().setNode(scanId, i.node);
    useStorage.getState().setTab('explorer');
  };
  if (report.error) return <ErrorState error={report.error} onRetry={() => void report.reload()} />;
  const r = report.data;
  if (!r) return <Skeleton className="min-h-[420px] flex-1 rounded-2xl" />;
  if (r.since == null)
    return (
      <Card className="flex flex-1 items-center justify-center">
        <EmptyState icon={TrendingUp} title="Nothing to compare with yet" description="OmniHub remembers folder sizes after each scan. Scan this drive again in a few days to see which folders grew." />
      </Card>
    );
  const net = r.totalAfter - r.totalBefore;
  const max = Math.max(1, ...r.grew.map((i) => Math.abs(i.delta)), ...r.shrank.map((i) => Math.abs(i.delta)));
  return (
    <div className="flex min-h-[460px] flex-1 flex-col gap-3">
      <div className="flex items-center gap-3 text-[13px] text-dim">
        <span>
          Since the scan {formatRelative(r.since)}, this drive's files {net >= 0 ? 'grew' : 'shrank'} by{' '}
          <span className={cx('font-semibold tabular', net >= 0 ? 'text-warn' : 'text-good')}>{signed(net)}</span> ({formatBytes(r.totalBefore)} → {formatBytes(r.totalAfter)}).
        </span>
        <div className="flex-1" />
        <Button size="sm" variant="ghost" onClick={() => void report.reload()}>
          Refresh
        </Button>
      </div>
      <Card className="min-h-0 flex-1 overflow-y-auto p-3" role="list">
        <div className="mb-1 flex items-center gap-2 px-3 text-[11px] font-semibold uppercase tracking-wider text-faint">
          <TrendingUp size={13} className="text-warn" /> Grew most
        </div>
        {r.grew.length ? <Rows items={r.grew} max={max} onOpen={open} /> : <div className="px-3 py-3 text-[12.5px] text-faint">No folder grew noticeably.</div>}
        {r.shrank.length > 0 && (
          <>
            <div className="mb-1 mt-4 flex items-center gap-2 px-3 text-[11px] font-semibold uppercase tracking-wider text-faint">
              <TrendingDown size={13} className="text-good" /> Shrank most
            </div>
            <Rows items={r.shrank} max={max} onOpen={open} />
          </>
        )}
      </Card>
    </div>
  );
}
