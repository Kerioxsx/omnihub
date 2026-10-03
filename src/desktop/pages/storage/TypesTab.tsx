// File types: category share, top extensions as horizontal bars, full table.
// Colour comes from extColor (fixed palette); every mark also carries a text
// label because several palette neighbours are hard to tell apart.

import { extColor, formatBytes, formatNumber, formatPercent } from '@shared/format';
import type { ExtensionStat } from '@shared/types';
import { motion } from 'motion/react';
import { FileQuestion } from 'lucide-react';
import { type CSSProperties, useMemo, useState } from 'react';
import { api } from '../../api';
import { Card, SectionTitle, Skeleton } from '../../components/ui/Card';
import { EmptyState, ErrorState } from '../../components/ui/States';
import { VirtualList } from '../../components/VirtualList';
import { useAsync } from '../../lib/hooks';
import { useSettings } from '../../state/settings';
import { useStorage } from '../../state/storage';

const CATEGORIES: [string, string][] = [
  ['var(--tm-video)', 'Video'],
  ['var(--tm-image)', 'Images'],
  ['var(--tm-audio)', 'Audio'],
  ['var(--tm-archive)', 'Archives & disk images'],
  ['var(--tm-code)', 'Code'],
  ['var(--tm-doc)', 'Documents'],
  ['var(--tm-exe)', 'Programs & game data'],
  ['var(--tm-data)', 'Data, caches & logs'],
  ['var(--tm-other)', 'Other'],
];

export function TypesTab({ scanId }: { scanId: string }) {
  const nodeId = useStorage((s) => s.nodeByScan[scanId] ?? 0);
  const version = useStorage((s) => s.version);
  const metric = useSettings((s) => s.settings?.storage.sizeMetric ?? 'size');
  const exts = useAsync(() => api.storage.extensions(scanId, nodeId), [scanId, nodeId, version]);
  const where = useAsync(() => api.storage.path(scanId, nodeId), [scanId, nodeId]);
  const [hover, setHover] = useState<string | null>(null);

  const val = (e: ExtensionStat) => (metric === 'alloc' ? e.alloc : e.size);
  const list = useMemo(() => [...(exts.data ?? [])].sort((a, b) => val(b) - val(a)), [exts.data, metric]);
  const total = list.reduce((a, e) => a + val(e), 0);
  const cats = useMemo(() => {
    const m = new Map<string, { size: number; count: number }>();
    for (const e of list) {
      const c = extColor(e.ext || null);
      const cur = m.get(c) ?? { size: 0, count: 0 };
      cur.size += val(e);
      cur.count += e.count;
      m.set(c, cur);
    }
    return CATEGORIES.map(([color, label]) => ({ color, label, ...(m.get(color) ?? { size: 0, count: 0 }) })).filter((c) => c.size > 0);
  }, [list]);

  if (exts.error) return <ErrorState error={exts.error} onRetry={exts.reload} />;
  if (!exts.data)
    return (
      <div className="grid flex-1 grid-cols-2 gap-3">
        <Skeleton className="h-full min-h-[300px] rounded-2xl" />
        <Skeleton className="h-full min-h-[300px] rounded-2xl" />
      </div>
    );
  if (!list.length) return <EmptyState icon={FileQuestion} title="No files in this folder" />;

  const top = list.slice(0, 14);
  const max = val(top[0]) || 1;

  const row = (e: ExtensionStat, _i: number, style: CSSProperties) => (
    <div key={e.ext} style={style} role="listitem" onMouseEnter={() => setHover(e.ext)} onMouseLeave={() => setHover(null)} className={`grid grid-cols-[minmax(0,1fr)_80px_86px_56px] items-center gap-3 rounded-lg px-3 transition-colors ${hover === e.ext ? 'bg-surface-2' : ''}`}>
      <span className="flex min-w-0 items-center gap-2">
        <span className="h-2.5 w-2.5 shrink-0 rounded-[3px]" style={{ background: extColor(e.ext || null) }} />
        <span className="truncate font-mono text-[12.5px] text-fg">{e.ext ? `.${e.ext}` : '(no extension)'}</span>
      </span>
      <span className="text-right text-[12px] tabular text-dim">{formatNumber(e.count)}</span>
      <span className="text-right text-[12.5px] font-medium tabular text-fg">{formatBytes(val(e))}</span>
      <span className="text-right text-[12px] tabular text-dim">{total ? formatPercent(val(e) / total, 1) : '—'}</span>
    </div>
  );

  return (
    <div className="flex min-h-[520px] flex-1 flex-col gap-3">
      <Card className="px-5 py-4">
        <div className="mb-3 flex items-baseline justify-between">
          <div className="text-[13px] text-dim">
            <span className="font-semibold text-fg tabular">{formatBytes(total)}</span> in {formatNumber(list.reduce((a, e) => a + e.count, 0))} files under <span className="font-mono text-fg">{where.data ?? '…'}</span>
          </div>
          <div className="text-[12px] text-faint">{list.length} extensions</div>
        </div>
        <div className="flex h-3.5 w-full gap-[2px] overflow-hidden rounded-full" role="img" aria-label="Share of space by file category">
          {cats.map((c) => (
            <motion.div key={c.label} title={`${c.label}: ${formatBytes(c.size)} (${formatPercent(c.size / total)})`} className="h-full first:rounded-l-full last:rounded-r-full" style={{ background: c.color }} initial={{ flexGrow: 0 }} animate={{ flexGrow: c.size / total }} transition={{ type: 'spring', stiffness: 70, damping: 20 }} />
          ))}
        </div>
        <div className="mt-3 flex flex-wrap gap-x-5 gap-y-1.5">
          {cats.map((c) => (
            <span key={c.label} className="inline-flex items-center gap-1.5 text-[12px] text-dim">
              <span className="h-2.5 w-2.5 rounded-[3px]" style={{ background: c.color }} />
              {c.label}
              <span className="font-medium text-fg tabular">{formatBytes(c.size)}</span>
              <span className="text-faint tabular">{formatPercent(c.size / total, 0)}</span>
            </span>
          ))}
        </div>
      </Card>

      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)] gap-3">
        <Card className="flex min-h-0 flex-col px-5 py-4">
          <SectionTitle>Largest extensions</SectionTitle>
          <div className="min-h-0 flex-1 space-y-[7px] overflow-y-auto pr-1" role="list">
            {top.map((e, i) => {
              const v = val(e);
              const pct = (v / max) * 100;
              return (
                <div key={e.ext} role="listitem" className="group grid grid-cols-[72px_minmax(0,1fr)] items-center gap-3" onMouseEnter={() => setHover(e.ext)} onMouseLeave={() => setHover(null)} title={`.${e.ext || '—'} · ${formatNumber(e.count)} files · ${formatBytes(v)} · ${formatPercent(v / total)}`}>
                  <span className="truncate text-right font-mono text-[12px] text-dim">{e.ext ? `.${e.ext}` : '(none)'}</span>
                  <div className="flex items-center gap-2">
                    <div className="relative h-[18px] flex-1">
                      <motion.div
                        className="absolute inset-y-0 left-0 rounded-r-[4px] transition-[filter]"
                        style={{ background: extColor(e.ext || null), filter: hover && hover !== e.ext ? 'saturate(0.5) opacity(0.55)' : undefined }}
                        initial={{ width: 0 }}
                        animate={{ width: `${Math.max(0.8, pct)}%` }}
                        transition={{ type: 'spring', stiffness: 90, damping: 20, delay: i * 0.02 }}
                      />
                    </div>
                    <span className="w-[64px] shrink-0 text-right text-[12px] font-medium tabular text-fg">{formatBytes(v)}</span>
                  </div>
                </div>
              );
            })}
          </div>
        </Card>
        <Card className="flex min-h-0 flex-col p-1.5">
          <div className="grid grid-cols-[minmax(0,1fr)_80px_86px_56px] gap-3 border-b border-line px-3 pb-2 pt-1.5 text-[11px] font-semibold uppercase tracking-wider text-faint">
            <span>Extension</span>
            <span className="text-right">Files</span>
            <span className="text-right">{metric === 'alloc' ? 'On disk' : 'Size'}</span>
            <span className="text-right">%</span>
          </div>
          <VirtualList items={list} rowHeight={32} renderRow={row} className="flex-1 pt-1" ariaLabel="All extensions" />
        </Card>
      </div>
    </div>
  );
}
