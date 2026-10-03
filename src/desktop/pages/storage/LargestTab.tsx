import { formatBytes, formatDate } from '@shared/format';
import type { PathedNode } from '@shared/types';
import { FileSearch } from 'lucide-react';
import { type CSSProperties, useState } from 'react';
import { api } from '../../api';
import { FileIcon } from '../../components/FileIcon';
import { Card, Skeleton } from '../../components/ui/Card';
import { Segmented } from '../../components/ui/Form';
import { EmptyState, ErrorState } from '../../components/ui/States';
import { VirtualList } from '../../components/VirtualList';
import { useAsync } from '../../lib/hooks';
import { useStorage } from '../../state/storage';
import { openItemMenu } from './actions';

function parentOf(path: string): string {
  const i = path.lastIndexOf('\\');
  const dir = i > 0 ? path.slice(0, i) : path;
  return /^[A-Za-z]:$/.test(dir) ? `${dir}\\` : dir;
}

export function LargestTab({ scanId }: { scanId: string }) {
  const nodeId = useStorage((s) => s.nodeByScan[scanId] ?? 0);
  const version = useStorage((s) => s.version);
  const [scope, setScope] = useState<'all' | 'here'>(nodeId ? 'here' : 'all');
  const under = scope === 'here' ? nodeId : 0;
  const files = useAsync(() => api.storage.topFiles(scanId, under, 100), [scanId, under, version]);
  const here = useAsync(() => (nodeId ? api.storage.path(scanId, nodeId) : Promise.resolve('')), [scanId, nodeId]);
  const items = files.data ?? [];
  const max = items[0]?.size ?? 1;
  const total = items.reduce((a, f) => a + f.size, 0);

  const row = (f: PathedNode, i: number, style: CSSProperties) => (
    <div
      key={f.id}
      style={style}
      role="listitem"
      tabIndex={0}
      onDoubleClick={() => void api.app.revealPath(f.path)}
      onContextMenu={(e) => openItemMenu(e, { path: f.path, name: f.name, isDir: false, size: f.size })}
      className="grid grid-cols-[34px_minmax(0,1fr)_200px_96px] items-center gap-3 rounded-lg px-3 outline-none transition-colors hover:bg-surface-2 focus-visible:bg-surface-3"
    >
      <span className="text-right font-mono text-[11.5px] text-faint tabular">{i + 1}</span>
      <div className="flex min-w-0 items-center gap-2.5">
        <FileIcon name={f.name} isDir={false} />
        <div className="min-w-0">
          <div className="truncate text-[13px] text-fg">{f.name}</div>
          <div className="truncate font-mono text-[10.5px] text-faint">{parentOf(f.path)}</div>
        </div>
      </div>
      <div className="flex items-center gap-2">
        <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-surface-3">
          <div className="accent-gradient h-full rounded-full" style={{ width: `${Math.max(2, (f.size / max) * 100)}%` }} />
        </div>
        <span className="w-[64px] text-right text-[12.5px] font-medium tabular text-fg">{formatBytes(f.size)}</span>
      </div>
      <span className="text-right text-[12px] tabular text-faint">{f.modified ? formatDate(f.modified) : '—'}</span>
    </div>
  );

  return (
    <div className="flex min-h-[460px] flex-1 flex-col gap-3">
      <div className="flex items-center justify-between gap-3">
        <div className="text-[13px] text-dim">
          The 100 largest files{scope === 'here' && here.data ? <span> under <span className="font-mono text-fg">{here.data}</span></span> : ' in this scan'} — together <span className="font-semibold text-fg tabular">{formatBytes(total)}</span>.
        </div>
        <Segmented
          size="sm"
          label="Scope"
          value={scope}
          onChange={setScope}
          options={[
            { value: 'all', label: 'Whole scan' },
            { value: 'here', label: 'Current folder', disabled: !nodeId, title: nodeId ? undefined : 'Open a folder in Explorer first' },
          ]}
        />
      </div>
      <Card className="flex min-h-0 flex-1 flex-col p-1.5">
        <div className="grid grid-cols-[34px_minmax(0,1fr)_200px_96px] gap-3 border-b border-line px-3 pb-2 pt-1.5 text-[11px] font-semibold uppercase tracking-wider text-faint">
          <span className="text-right">#</span>
          <span>File</span>
          <span className="text-right">Size</span>
          <span className="text-right">Modified</span>
        </div>
        {files.error ? (
          <ErrorState error={files.error} onRetry={files.reload} />
        ) : !files.data ? (
          <div className="space-y-1.5 p-2">
            {Array.from({ length: 10 }, (_, i) => (
              <Skeleton key={i} className="h-10" />
            ))}
          </div>
        ) : items.length === 0 ? (
          <EmptyState icon={FileSearch} title="No files here" description="This folder only contains empty folders." compact />
        ) : (
          <VirtualList items={items} rowHeight={46} renderRow={row} className="flex-1 pt-1" ariaLabel="Largest files" />
        )}
      </Card>
    </div>
  );
}
