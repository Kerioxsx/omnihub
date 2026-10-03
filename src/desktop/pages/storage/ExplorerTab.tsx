import { formatBytes, formatDate, formatNumber, formatPercent } from '@shared/format';
import type { ChildrenPage, NodeView, SortKey } from '@shared/types';
import { ArrowDown, ArrowUp, ArrowUpLeft, ChevronRight, Cloud, EyeOff, Link2, ShieldHalf } from 'lucide-react';
import { type CSSProperties, type KeyboardEvent, type MouseEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { FileIcon } from '../../components/FileIcon';
import { IconButton } from '../../components/ui/Button';
import { Card, Skeleton } from '../../components/ui/Card';
import { Segmented } from '../../components/ui/Form';
import { ErrorState } from '../../components/ui/States';
import { VirtualList } from '../../components/VirtualList';
import { cx } from '../../lib/cx';
import { useSettings } from '../../state/settings';
import { useStorage } from '../../state/storage';
import { deleteItems, openItemMenu } from './actions';
import { Treemap } from './Treemap';

const LEGEND: [string, string][] = [
  ['var(--tm-video)', 'Video'],
  ['var(--tm-image)', 'Images'],
  ['var(--tm-audio)', 'Audio'],
  ['var(--tm-archive)', 'Archives'],
  ['var(--tm-code)', 'Code'],
  ['var(--tm-doc)', 'Documents'],
  ['var(--tm-exe)', 'Programs'],
  ['var(--tm-data)', 'Data & logs'],
  ['var(--tm-other)', 'Other'],
];

type Col = 'name' | 'size' | 'files' | 'modified';

export function ExplorerTab({ scanId }: { scanId: string }) {
  const nodeId = useStorage((s) => s.nodeByScan[scanId] ?? 0);
  const setNode = useStorage((s) => s.setNode);
  const version = useStorage((s) => s.version);
  const metric = useSettings((s) => s.settings?.storage.sizeMetric ?? 'size');
  const reduced = useSettings((s) => s.settings?.general.reducedMotion ?? false);
  const updateSettings = useSettings((s) => s.update);
  const [col, setCol] = useState<Col>('size');
  const [desc, setDesc] = useState(true);
  const [page, setPage] = useState<ChildrenPage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loadingMore, setLoadingMore] = useState(false);
  const [hoverId, setHoverId] = useState<number | null>(null);
  const [selected, setSelected] = useState<number | null>(null);

  const sort: SortKey = col === 'size' ? metric : col;

  useEffect(() => {
    let alive = true;
    setError(null);
    api.storage
      .children(scanId, nodeId, sort, desc, 0, 500)
      .then((p) => {
        if (!alive) return;
        setPage(p);
        setSelected(null);
      })
      .catch((e: unknown) => alive && setError(errorText(e)));
    return () => {
      alive = false;
    };
  }, [scanId, nodeId, sort, desc, version]);

  const loadMore = useCallback(async () => {
    if (!page || loadingMore || page.items.length >= page.total) return;
    setLoadingMore(true);
    try {
      const next = await api.storage.children(scanId, page.node.id, sort, desc, page.items.length, 500);
      setPage((p) => (p && p.node.id === next.node.id ? { ...p, items: [...p.items, ...next.items] } : p));
    } finally {
      setLoadingMore(false);
    }
  }, [page, loadingMore, scanId, sort, desc]);

  const crumbs = page?.breadcrumbs ?? [];
  const goUp = () => {
    if (crumbs.length > 1) setNode(scanId, crumbs[crumbs.length - 2].id);
  };

  const pathOf = (n: NodeView) => `${(page?.path ?? '').replace(/\\$/, '')}\\${n.name}`;
  const parentSize = page ? (metric === 'alloc' ? page.node.alloc : page.node.size) : 0;
  const maxItem = useMemo(() => (page?.items.length ? Math.max(...page.items.slice(0, 50).map((i) => (metric === 'alloc' ? i.alloc : i.size))) : 0), [page, metric]);

  const onKey = (e: KeyboardEvent) => {
    if (!page) return;
    if (e.key === 'Backspace' || (e.key === 'ArrowUp' && e.altKey)) {
      e.preventDefault();
      goUp();
    }
    if (e.key === 'Delete' && selected != null) {
      const n = page.items.find((i) => i.id === selected);
      if (n) void deleteItems([{ path: pathOf(n), name: n.name, isDir: n.isDir, size: n.size }], e.shiftKey);
    }
  };

  const header = (key: Col, label: string, className?: string) => (
    <button
      type="button"
      onClick={() => {
        if (col === key) setDesc(!desc);
        else {
          setCol(key);
          setDesc(key !== 'name');
        }
      }}
      className={cx('flex items-center gap-1 text-[11px] font-semibold uppercase tracking-wider transition-colors', col === key ? 'text-fg' : 'text-faint hover:text-dim', className)}
      aria-sort={col === key ? (desc ? 'descending' : 'ascending') : undefined}
    >
      {label}
      {col === key && (desc ? <ArrowDown size={11} /> : <ArrowUp size={11} />)}
    </button>
  );

  const row = (n: NodeView, i: number, style: CSSProperties) => {
    const v = metric === 'alloc' ? n.alloc : n.size;
    const frac = parentSize ? v / parentSize : 0;
    const active = hoverId === n.id || selected === n.id;
    return (
      <div
        key={n.id}
        style={style}
        role="listitem"
        tabIndex={0}
        onMouseEnter={() => setHoverId(n.id)}
        onMouseLeave={() => setHoverId(null)}
        onClick={() => setSelected(n.id)}
        onDoubleClick={() => (n.isDir ? setNode(scanId, n.id) : void api.app.openPath(pathOf(n)))}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && n.isDir) setNode(scanId, n.id);
          if (e.key === 'ArrowDown') (e.currentTarget.nextElementSibling as HTMLElement | null)?.focus();
          if (e.key === 'ArrowUp' && !e.altKey) (e.currentTarget.previousElementSibling as HTMLElement | null)?.focus();
        }}
        onContextMenu={(e: MouseEvent) => {
          setSelected(n.id);
          openItemMenu(e, { path: pathOf(n), name: n.name, isDir: n.isDir, size: n.size });
        }}
        className={cx('relative grid cursor-default grid-cols-[minmax(0,1fr)_68px_46px_58px_84px] items-center gap-2.5 rounded-lg px-3 outline-none transition-colors focus-visible:bg-surface-3', active ? 'bg-surface-2' : i % 2 ? 'bg-transparent' : 'bg-surface/40', selected === n.id && 'ring-1 ring-accent/50')}
      >
        <span aria-hidden className="pointer-events-none absolute inset-y-[5px] left-1 rounded-md bg-accent/[0.13]" style={{ width: `calc(${maxItem ? Math.max(0.5, (v / maxItem) * 100) : 0}% - 8px)` }} />
        <div className="relative flex min-w-0 items-center gap-2.5">
          <FileIcon name={n.name} isDir={n.isDir} />
          <span className={cx('truncate text-[13px]', n.isDir ? 'font-medium text-fg' : 'text-fg/90')}>{n.name}</span>
          {n.hidden && <EyeOff size={12} className="shrink-0 text-faint" aria-label="Hidden" />}
          {n.system && <ShieldHalf size={12} className="shrink-0 text-faint" aria-label="System" />}
          {n.cloud && <Cloud size={12} className="shrink-0 text-info" aria-label="Cloud placeholder" />}
          {n.reparse && <Link2 size={12} className="shrink-0 text-faint" aria-label="Link" />}
        </div>
        <span className="text-right text-[12.5px] font-medium tabular text-fg">{formatBytes(v)}</span>
        <span className="relative text-right text-[12px] tabular text-dim">{frac >= 0.001 ? formatPercent(frac, frac < 0.1 ? 1 : 0) : '<0.1%'}</span>
        <span className="text-right text-[12px] tabular text-dim">{n.isDir ? formatNumber(n.files) : '—'}</span>
        <span className="text-right text-[12px] tabular text-faint">{n.modified ? formatDate(n.modified) : '—'}</span>
      </div>
    );
  };

  return (
    <div className="flex min-h-[480px] flex-1 flex-col gap-3" onKeyDown={onKey}>
      <div className="flex items-center gap-3">
        <IconButton icon={ArrowUpLeft} label="Up one level (Backspace)" disabled={crumbs.length <= 1} onClick={goUp} variant="secondary" />
        <nav aria-label="Breadcrumbs" className="flex min-w-0 flex-1 items-center gap-0.5 overflow-hidden">
          {crumbs.map((c, i) => {
            const last = i === crumbs.length - 1;
            return (
              <span key={c.id} className={cx('flex min-w-0 items-center', !last && 'shrink')}>
                {i > 0 && <ChevronRight size={14} className="mx-0.5 shrink-0 text-faint" aria-hidden />}
                <button type="button" disabled={last} onClick={() => setNode(scanId, c.id)} className={cx('truncate rounded-md px-1.5 py-1 text-[13px] transition-colors', last ? 'font-semibold text-fg' : 'text-dim hover:bg-surface-2 hover:text-fg')}>
                  {c.name.replace(/\\$/, '') || c.name}
                </button>
              </span>
            );
          })}
          {page && <span className="ml-2 shrink-0 text-[12.5px] text-faint tabular">{formatBytes(metric === 'alloc' ? page.node.alloc : page.node.size)} · {formatNumber(page.total)} items</span>}
        </nav>
        <Segmented
          size="sm"
          label="Size metric"
          value={metric}
          onChange={(m) => void updateSettings({ storage: { sizeMetric: m } }, { silent: true })}
          options={[
            { value: 'size', label: 'Size', title: 'Logical file size' },
            { value: 'alloc', label: 'On disk', title: 'Space allocated on disk (clusters)' },
          ]}
        />
      </div>

      <div className="grid min-h-0 flex-1 grid-cols-1 gap-3 xl:grid-cols-[minmax(0,1.2fr)_minmax(440px,1fr)]">
        <Card className="flex min-h-[360px] flex-col p-2.5 xl:min-h-0">
          <div className="min-h-0 flex-1">
            <Treemap
              scanId={scanId}
              nodeId={nodeId}
              version={version}
              basePath={page?.path ?? ''}
              highlightId={hoverId ?? selected}
              reducedMotion={reduced}
              onHover={setHoverId}
              onDrill={(id) => setNode(scanId, id)}
              onSelect={setSelected}
              onContextMenu={(e, t) => openItemMenu(e, { path: t.path, name: t.name, isDir: t.isDir, size: t.size })}
            />
          </div>
          <div className="flex flex-wrap items-center gap-x-3.5 gap-y-1 px-1.5 pb-0.5 pt-2.5 text-[11.5px] text-dim">
            <span className="inline-flex items-center gap-1.5">
              <span className="h-2.5 w-2.5 rounded-[3px] border border-line-strong" style={{ background: 'var(--tm-dir)' }} />
              Folders
            </span>
            {LEGEND.map(([c, l]) => (
              <span key={l} className="inline-flex items-center gap-1.5">
                <span className="h-2.5 w-2.5 rounded-[3px]" style={{ background: c }} />
                {l}
              </span>
            ))}
            <span className="ml-auto text-faint">Treemap areas use logical size</span>
          </div>
        </Card>

        <Card className="flex min-h-[380px] flex-col p-1.5 xl:min-h-0">
          <div className="grid grid-cols-[minmax(0,1fr)_68px_46px_58px_84px] items-center gap-2.5 border-b border-line px-3 pb-2 pt-1.5">
            {header('name', 'Name')}
            {header('size', metric === 'alloc' ? 'On disk' : 'Size', 'justify-end')}
            <span className="text-right text-[11px] font-semibold uppercase tracking-wider text-faint">%</span>
            {header('files', 'Files', 'justify-end')}
            {header('modified', 'Modified', 'justify-end')}
          </div>
          {error ? (
            <ErrorState error={error} />
          ) : !page ? (
            <div className="space-y-1.5 p-2">
              {Array.from({ length: 12 }, (_, i) => (
                <Skeleton key={i} className="h-8" />
              ))}
            </div>
          ) : page.items.length === 0 ? (
            <div className="flex flex-1 items-center justify-center text-[13px] text-faint">This folder is empty.</div>
          ) : (
            <VirtualList items={page.items} rowHeight={36} renderRow={row} className="flex-1 pt-1" onEndReached={loadMore} ariaLabel={`Contents of ${page.node.name}`} />
          )}
          {page && page.items.length < page.total && <div className="border-t border-line px-3 py-1.5 text-[11.5px] text-faint">Showing {formatNumber(page.items.length)} of {formatNumber(page.total)} — scroll for more</div>}
        </Card>
      </div>
    </div>
  );
}
