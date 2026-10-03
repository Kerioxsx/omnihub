import { formatBytes, formatDate } from '@shared/format';
import type { AppInfo, AppSource } from '@shared/types';
import { motion } from 'motion/react';
import { ArrowUpDown, LayoutGrid, LayoutList, PackageSearch, RefreshCw } from 'lucide-react';
import { type CSSProperties, useMemo, useState } from 'react';
import { api } from '../../api';
import { AppIcon } from '../../components/AppIcon';
import { Page } from '../../components/Page';
import { IconButton } from '../../components/ui/Button';
import { Badge, Card, Skeleton } from '../../components/ui/Card';
import { SearchInput, Segmented, Select } from '../../components/ui/Form';
import { EmptyState, ErrorState } from '../../components/ui/States';
import { VirtualList } from '../../components/VirtualList';
import { cx } from '../../lib/cx';
import { useAsync, useStoredState } from '../../lib/hooks';
import { AppDrawer } from './AppDrawer';
import { SOURCE_LABEL, SizeHint } from './shared';

type Sort = 'name' | 'size' | 'date' | 'publisher';
type Filter = 'all' | AppSource;

export function AppsPage() {
  const apps = useAsync(() => api.apps.list(false), []);
  const [q, setQ] = useState('');
  const [sort, setSort] = useStoredState<Sort>('omnihub.apps.sort', 'name');
  const [view, setView] = useStoredState<'grid' | 'list'>('omnihub.apps.view', 'grid');
  const [filter, setFilter] = useState<Filter>('all');
  const [open, setOpen] = useState<AppInfo | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  const all = apps.data ?? [];
  const counts = useMemo(() => ({ all: all.length, desktop: all.filter((a) => a.source === 'desktop').length, store: all.filter((a) => a.source === 'store').length, startMenu: all.filter((a) => a.source === 'startMenu').length }), [all]);
  const list = useMemo(() => {
    const needle = q.trim().toLowerCase();
    const out = all.filter((a) => (filter === 'all' || a.source === filter) && (!needle || `${a.name} ${a.publisher}`.toLowerCase().includes(needle)));
    const by: Record<Sort, (a: AppInfo, b: AppInfo) => number> = {
      name: (a, b) => a.name.localeCompare(b.name),
      size: (a, b) => (b.size ?? -1) - (a.size ?? -1),
      date: (a, b) => (b.installDate ?? 0) - (a.installDate ?? 0),
      publisher: (a, b) => a.publisher.localeCompare(b.publisher) || a.name.localeCompare(b.name),
    };
    return out.sort(by[sort]);
  }, [all, q, filter, sort]);
  const totalSize = list.reduce((a, x) => a + (x.size ?? 0), 0);

  const refresh = async () => {
    setRefreshing(true);
    try {
      apps.setData(await api.apps.list(true));
    } finally {
      setRefreshing(false);
    }
  };

  const row = (a: AppInfo, _i: number, style: CSSProperties) => (
    <button key={a.id} type="button" style={style} onClick={() => setOpen(a)} className="grid grid-cols-[minmax(0,1fr)_150px_96px_110px_100px] items-center gap-4 rounded-xl px-3 text-left transition-colors hover:bg-surface-2">
      <span className="flex min-w-0 items-center gap-3">
        <AppIcon id={a.id} name={a.name} size={30} />
        <span className="min-w-0">
          <span className="block truncate text-[13.5px] font-medium text-fg">{a.name}</span>
          <span className="block truncate text-[12px] text-faint">{a.publisher}</span>
        </span>
      </span>
      <span className="truncate font-mono text-[12px] text-dim">{a.version}</span>
      <span>
        <Badge tone={a.source === 'store' ? 'info' : a.source === 'startMenu' ? 'neutral' : 'accent'}>{SOURCE_LABEL[a.source]}</Badge>
      </span>
      <span className="text-[12.5px] text-dim tabular">{a.installDate ? formatDate(a.installDate) : '—'}</span>
      <span className="text-right text-[12.5px]">
        <SizeHint app={a} />
      </span>
    </button>
  );

  return (
    <Page
      title="Apps"
      subtitle={apps.data ? `${counts.all} installed · ${counts.store} from the Microsoft Store · ${formatBytes(totalSize)} shown` : 'Reading installed programs…'}
      scroll={view === 'grid'}
      actions={
        <>
          <SearchInput value={q} onChange={setQ} placeholder="Search apps or publishers" className="w-64" aria-label="Search apps" />
          <div className="flex items-center gap-1.5">
            <ArrowUpDown size={14} className="text-faint" aria-hidden />
            <Select value={sort} onChange={(e) => setSort(e.target.value as Sort)} aria-label="Sort by" className="w-[150px]">
              <option value="name">Name</option>
              <option value="size">Size</option>
              <option value="date">Install date</option>
              <option value="publisher">Publisher</option>
            </Select>
          </div>
          <div className="flex rounded-[11px] border border-line bg-surface p-0.5">
            <IconButton icon={LayoutGrid} label="Grid view" size="sm" active={view === 'grid'} onClick={() => setView('grid')} />
            <IconButton icon={LayoutList} label="List view" size="sm" active={view === 'list'} onClick={() => setView('list')} />
          </div>
          <IconButton icon={RefreshCw} label="Refresh the list" variant="secondary" onClick={refresh} className={refreshing ? '[&_svg]:spin' : undefined} />
        </>
      }
      headerExtra={
        <Segmented
          label="Filter"
          value={filter}
          onChange={setFilter}
          options={[
            { value: 'all', label: <span>All <span className="text-faint">{counts.all}</span></span> },
            { value: 'desktop', label: <span>Desktop <span className="text-faint">{counts.desktop}</span></span> },
            { value: 'store', label: <span>Store <span className="text-faint">{counts.store}</span></span> },
            { value: 'startMenu', label: <span>Start menu <span className="text-faint">{counts.startMenu}</span></span> },
          ]}
        />
      }
    >
      {apps.error ? (
        <ErrorState title="Could not list apps" error={apps.error} onRetry={apps.reload} />
      ) : !apps.data ? (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] gap-3">
          {Array.from({ length: 12 }, (_, i) => (
            <Skeleton key={i} className="h-[92px] rounded-2xl" />
          ))}
        </div>
      ) : list.length === 0 ? (
        <EmptyState icon={PackageSearch} title="No apps match" description={q ? `Nothing named “${q}”.` : 'No apps in this category.'} />
      ) : view === 'grid' ? (
        <motion.div layout className="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] gap-3">
          {list.map((a, i) => (
            <motion.button
              layout
              key={a.id}
              type="button"
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0, transition: { delay: Math.min(i, 24) * 0.012 } }}
              whileHover={{ y: -2 }}
              transition={{ type: 'spring', stiffness: 400, damping: 30 }}
              onClick={() => setOpen(a)}
              className="card card-interactive flex items-center gap-3 px-3.5 py-3 text-left"
            >
              <AppIcon id={a.id} name={a.name} size={44} />
              <span className="min-w-0 flex-1">
                <span className="block truncate text-[13.5px] font-semibold text-fg">{a.name}</span>
                <span className="block truncate text-[12px] text-faint">{a.publisher}</span>
                <span className="mt-1 flex items-center justify-between gap-2 text-[12px]">
                  <SizeHint app={a} />
                  {a.source !== 'desktop' && <span className={cx('rounded-md px-1.5 py-px text-[10.5px] font-medium', a.source === 'store' ? 'bg-info/12 text-info' : 'bg-surface-3 text-dim')}>{SOURCE_LABEL[a.source]}</span>}
                </span>
              </span>
            </motion.button>
          ))}
        </motion.div>
      ) : (
        <Card className="flex min-h-0 flex-1 flex-col p-1.5">
          <div className="grid grid-cols-[minmax(0,1fr)_150px_96px_110px_100px] gap-4 border-b border-line px-3 pb-2 pt-1.5 text-[11px] font-semibold uppercase tracking-wider text-faint">
            <span>Name</span>
            <span>Version</span>
            <span>Source</span>
            <span>Installed</span>
            <span className="text-right">Size</span>
          </div>
          <VirtualList items={list} rowHeight={52} renderRow={row} className="flex-1 pt-1" ariaLabel="Installed apps" />
        </Card>
      )}
      <AppDrawer app={open} onClose={() => setOpen(null)} />
    </Page>
  );
}
