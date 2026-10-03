import { formatBytes, formatDate, formatNumber, formatPercent } from '@shared/format';
import type { ChildrenPage, ExplorerView, NodeView, SortKey } from '@shared/types';
import { ArrowDown, ArrowUp, ArrowUpLeft, ChevronRight, CircleDot, Cloud, Columns2, EyeOff, Image, LayoutDashboard, LayoutGrid, Link2, List, ShieldHalf } from 'lucide-react';
import { type CSSProperties, type KeyboardEvent, type MouseEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { FileIcon } from '../../components/FileIcon';
import { IconButton } from '../../components/ui/Button';
import { extOf } from '../../components/FileIcon';
import { Card, Skeleton } from '../../components/ui/Card';
import { Segmented } from '../../components/ui/Form';
import { ErrorState } from '../../components/ui/States';
import { VirtualList } from '../../components/VirtualList';
import { cx } from '../../lib/cx';
import { useSettings } from '../../state/settings';
import { useStorage } from '../../state/storage';
import { deleteItems, openItemMenu } from './actions';
import { GridView } from './GridView';
import { Sunburst } from './Sunburst';
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

const COLS = 'grid-cols-[minmax(0,1fr)_68px_46px_58px_84px]';
const COLS_WIDE = 'grid-cols-[minmax(0,1fr)_120px_84px_56px_70px_70px_96px]';

function typeLabel(n: NodeView): string {
  if (n.isDir) return 'Folder';
  const ext = extOf(n.name);
  return ext ? `${ext.toUpperCase()} file` : 'File';
}

const VIEWS: { value: ExplorerView; label: string; icon: typeof List; title: string }[] = [
  { value: 'split', label: 'Split', icon: Columns2, title: 'Treemap and list side by side' },
  { value: 'list', label: 'List', icon: List, title: 'Details list' },
  { value: 'grid', label: 'Grid', icon: LayoutGrid, title: 'Tiles with image previews' },
  { value: 'treemap', label: 'Treemap', icon: LayoutDashboard, title: 'Treemap only' },
  { value: 'sunburst', label: 'Rings', icon: CircleDot, title: 'Sunburst: folders as rings' },
];

export function ExplorerTab({ scanId }: { scanId: string }) {
  const nodeId = useStorage((s) => s.nodeByScan[scanId] ?? 0);
  const setNode = useStorage((s) => s.setNode);
  const version = useStorage((s) => s.version);
  const metric = useSettings((s) => s.settings?.storage.sizeMetric ?? 'size');
  const reduced = useSettings((s) => s.settings?.general.reducedMotion ?? false);
  const updateSettings = useSettings((s) => s.update);
  const view: ExplorerView = useSettings((s) => s.settings?.storage.explorerView ?? 'split');
  const gridSize = useSettings((s) => s.settings?.storage.gridSize ?? 'md');
  const previewsOn = useSettings((s) => s.settings?.storage.gridPreviews ?? true);
  const setView = (v: ExplorerView) => void updateSettings({ storage: { explorerView: v } }, { silent: true });
  const wide = view === 'list';
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
        className={cx('relative grid cursor-default items-center gap-2.5 rounded-lg px-3 outline-none transition-colors focus-visible:bg-surface-3', wide ? COLS_WIDE : COLS, active ? 'bg-surface-2' : i % 2 ? 'bg-transparent' : 'bg-surface/40', selected === n.id && 'ring-1 ring-accent/50')}
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
        {wide && <span className="truncate text-[12px] text-dim">{typeLabel(n)}</span>}
        <span className="text-right text-[12.5px] font-medium tabular text-fg">{formatBytes(v)}</span>
        <span className="relative text-right text-[12px] tabular text-dim">{frac >= 0.001 ? formatPercent(frac, frac < 0.1 ? 1 : 0) : '<0.1%'}</span>
        {wide && <span className="text-right text-[12px] tabular text-dim">{n.isDir ? formatNumber(n.dirs) : '—'}</span>}
        <span className="text-right text-[12px] tabular text-dim">{n.isDir ? formatNumber(n.files) : '—'}</span>
        <span className="text-right text-[12px] tabular text-faint">{n.modified ? formatDate(n.modified) : '—'}</span>
      </div>
    );
  };

  const open = (n: NodeView) => (n.isDir ? setNode(scanId, n.id) : void api.app.openPath(pathOf(n)));
  const menu = (e: MouseEvent, n: NodeView) => openItemMenu(e, { path: pathOf(n), name: n.name, isDir: n.isDir, size: n.size });

  const treemap = (
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
  );

  const legend = (
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
      <span className="ml-auto text-faint">{view === 'sunburst' ? 'Each ring is one level deeper; angles use logical size' : 'Treemap areas use logical size'}</span>
    </div>
  );

  const list = (
    <Card className={cx('flex min-h-[380px] flex-col p-1.5', view === 'split' && 'xl:min-h-0', view === 'list' && 'flex-1')}>
      <div className={cx('grid items-center gap-2.5 border-b border-line px-3 pb-2 pt-1.5', wide ? COLS_WIDE : COLS)}>
        {header('name', 'Name')}
        {wide && <span className="text-[11px] font-semibold uppercase tracking-wider text-faint">Type</span>}
        {header('size', metric === 'alloc' ? 'On disk' : 'Size', 'justify-end')}
        <span className="text-right text-[11px] font-semibold uppercase tracking-wider text-faint">%</span>
        {wide && <span className="text-right text-[11px] font-semibold uppercase tracking-wider text-faint">Folders</span>}
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
  );

  let body;
  if (view === 'split') {
    body = (
      <div className="grid min-h-0 flex-1 grid-cols-1 gap-3 xl:grid-cols-[minmax(0,1.2fr)_minmax(440px,1fr)]">
        <Card className="flex min-h-[360px] flex-col p-2.5 xl:min-h-0">
          <div className="min-h-0 flex-1">{treemap}</div>
          {legend}
        </Card>
        {list}
      </div>
    );
  } else if (view === 'list') {
    body = list;
  } else if (view === 'treemap') {
    body = (
      <Card className="flex min-h-[520px] flex-1 flex-col p-2.5">
        <div className="min-h-0 flex-1">{treemap}</div>
        {legend}
      </Card>
    );
  } else if (view === 'sunburst') {
    body = (
      <Card className="flex min-h-[520px] flex-1 flex-col p-2.5">
        <div className="min-h-0 flex-1">
          <Sunburst
            scanId={scanId}
            nodeId={nodeId}
            version={version}
            basePath={page?.path ?? ''}
            highlightId={hoverId ?? selected}
            onHover={setHoverId}
            onDrill={(id) => setNode(scanId, id)}
            onUp={crumbs.length > 1 ? goUp : null}
            onSelect={setSelected}
            onContextMenu={(e, t) => openItemMenu(e, { path: t.path, name: t.name, isDir: t.isDir, size: t.size })}
          />
        </div>
        {legend}
      </Card>
    );
  } else {
    body = (
      <Card className="flex min-h-[520px] flex-1 flex-col p-2">
        {error ? (
          <ErrorState error={error} />
        ) : !page ? (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(148px,1fr))] gap-2.5 p-1">
            {Array.from({ length: 18 }, (_, i) => (
              <Skeleton key={i} className="h-[150px] rounded-xl" />
            ))}
          </div>
        ) : page.items.length === 0 ? (
          <div className="flex flex-1 items-center justify-center text-[13px] text-faint">This folder is empty.</div>
        ) : (
          <GridView
            items={page.items}
            basePath={page.path}
            metric={metric}
            parentSize={parentSize}
            size={gridSize}
            previewsOn={previewsOn}
            selected={selected}
            hasMore={page.items.length < page.total}
            onSelect={setSelected}
            onOpen={open}
            onMenu={menu}
            onEndReached={() => void loadMore()}
          />
        )}
      </Card>
    );
  }

  return (
    <div className="flex min-h-[480px] flex-1 flex-col gap-3" onKeyDown={onKey}>
      <div className="flex flex-wrap items-center gap-3">
        <IconButton icon={ArrowUpLeft} label="Up one level (Backspace)" disabled={crumbs.length <= 1} onClick={goUp} variant="secondary" />
        <nav aria-label="Breadcrumbs" className="flex min-w-[260px] flex-1 items-center gap-0.5 overflow-hidden">
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
        {view === 'grid' && (
          <>
            <IconButton icon={Image} label={previewsOn ? 'Hide image previews' : 'Show image previews'} size="sm" active={previewsOn} onClick={() => void updateSettings({ storage: { gridPreviews: !previewsOn } }, { silent: true })} />
            <Segmented
              size="sm"
              label="Tile size"
              value={gridSize}
              onChange={(g) => void updateSettings({ storage: { gridSize: g } }, { silent: true })}
              options={[
                { value: 'sm', label: 'S', title: 'Small tiles' },
                { value: 'md', label: 'M', title: 'Medium tiles' },
                { value: 'lg', label: 'L', title: 'Large tiles' },
              ]}
            />
          </>
        )}
        <Segmented size="sm" label="View" value={view} onChange={setView} options={VIEWS.map((v) => ({ value: v.value, label: v.label, icon: v.icon, title: v.title }))} />
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
      {body}
    </div>
  );
}
