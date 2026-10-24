// Grid view of a folder: tiles with an icon (or an image preview), name,
// size and a bar for the share of the folder. Previews load lazily, a few
// at a time.

import { formatBytes, formatPercent } from '@shared/format';
import type { NodeView } from '@shared/types';
import { Cloud, EyeOff, Link2 } from 'lucide-react';
import { type MouseEvent, memo, useEffect, useRef, useState } from 'react';
import { api } from '../../api';
import { FileIcon } from '../../components/FileIcon';
import { cx } from '../../lib/cx';

const TILE = { sm: 112, md: 148, lg: 196 } as const;
const PREVIEW = /\.(jpe?g|png|gif|webp|bmp)$/i;

// Preview requests share one small queue so a big folder does not flood the backend.
let active = 0;
const waiting: (() => void)[] = [];
function queued<T>(fn: () => Promise<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const run = () => {
      active++;
      fn()
        .then(resolve, reject)
        .finally(() => {
          active--;
          waiting.shift()?.();
        });
    };
    if (active < 4) run();
    else waiting.push(run);
  });
}
const previews = new Map<string, string | null>();

function Preview({ path, size }: { path: string; size: number }) {
  const [url, setUrl] = useState<string | null | undefined>(previews.get(path));
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (url !== undefined || !ref.current) return;
    let alive = true;
    const io = new IntersectionObserver(
      ([e]) => {
        if (!e.isIntersecting) return;
        io.disconnect();
        void queued(() => api.storage.thumb(path, size * 2)).then(
          (u) => {
            previews.set(path, u);
            if (alive) setUrl(u);
          },
          () => alive && setUrl(null),
        );
      },
      { rootMargin: '200px' },
    );
    io.observe(ref.current);
    return () => {
      alive = false;
      io.disconnect();
    };
  }, [path, size, url]);
  return <div ref={ref} className="absolute inset-0">{url && <img src={url} alt="" className="h-full w-full object-cover" draggable={false} />}</div>;
}

const Tile = memo(function Tile({
  n,
  path,
  tile,
  value,
  parent,
  selected,
  previewsOn,
  onSelect,
  onOpen,
  onMenu,
}: {
  n: NodeView;
  path: string;
  tile: number;
  value: number;
  parent: number;
  selected: boolean;
  previewsOn: boolean;
  onSelect: (id: number) => void;
  onOpen: (n: NodeView) => void;
  onMenu: (e: MouseEvent, n: NodeView) => void;
}) {
  const frac = parent ? value / parent : 0;
  const canPreview = previewsOn && !n.isDir && PREVIEW.test(n.name) && !n.cloud;
  return (
    <button
      type="button"
      onClick={() => onSelect(n.id)}
      onDoubleClick={() => onOpen(n)}
      onKeyDown={(e) => e.key === 'Enter' && onOpen(n)}
      onContextMenu={(e) => {
        onSelect(n.id);
        onMenu(e, n);
      }}
      title={`${n.name}\n${formatBytes(value)}`}
      className={cx('group flex flex-col overflow-hidden rounded-xl border bg-surface text-left outline-none transition-colors focus-visible:ring-2 focus-visible:ring-accent', selected ? 'border-accent/60 bg-accent-soft' : 'border-line hover:border-line-strong hover:bg-surface-2')}
    >
      <div className="relative flex items-center justify-center bg-[color-mix(in_oklab,var(--surface-2)_70%,transparent)]" style={{ height: Math.round(tile * 0.62) }}>
        <FileIcon name={n.name} isDir={n.isDir} size={Math.round(tile * 0.3)} />
        {canPreview && <Preview path={path} size={tile} />}
        <div className="absolute right-1.5 top-1.5 flex gap-1">
          {n.hidden && <EyeOff size={12} className="text-faint" aria-label="Hidden" />}
          {n.cloud && <Cloud size={12} className="text-info" aria-label="Cloud placeholder" />}
          {n.reparse && <Link2 size={12} className="text-faint" aria-label="Link" />}
        </div>
      </div>
      <div className="flex min-h-0 flex-1 flex-col gap-1 px-2.5 pb-2 pt-1.5">
        <div className={cx('line-clamp-2 break-words text-[12.5px] leading-snug', n.isDir ? 'font-medium text-fg' : 'text-fg/90')}>{n.name}</div>
        <div className="mt-auto flex items-baseline justify-between gap-2 text-[11.5px] tabular">
          <span className="font-medium text-fg">{formatBytes(value)}</span>
          <span className="text-faint">{frac >= 0.001 ? formatPercent(frac, frac < 0.1 ? 1 : 0) : '<0.1%'}</span>
        </div>
        <div className="h-1 overflow-hidden rounded-full bg-surface-3">
          <div className="h-full rounded-full bg-accent/70" style={{ width: `${Math.max(1, Math.min(100, frac * 100))}%` }} />
        </div>
      </div>
    </button>
  );
});

export function GridView({
  items,
  basePath,
  metric,
  parentSize,
  size,
  previewsOn,
  selected,
  hasMore,
  onSelect,
  onOpen,
  onMenu,
  onEndReached,
}: {
  items: NodeView[];
  basePath: string;
  metric: 'size' | 'alloc';
  parentSize: number;
  size: keyof typeof TILE;
  previewsOn: boolean;
  selected: number | null;
  hasMore: boolean;
  onSelect: (id: number) => void;
  onOpen: (n: NodeView) => void;
  onMenu: (e: MouseEvent, n: NodeView) => void;
  onEndReached: () => void;
}) {
  const tile = TILE[size];
  const end = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!hasMore || !end.current) return;
    const io = new IntersectionObserver(([e]) => e.isIntersecting && onEndReached(), { rootMargin: '400px' });
    io.observe(end.current);
    return () => io.disconnect();
  }, [hasMore, onEndReached, items.length]);
  const pathOf = (n: NodeView) => `${basePath.replace(/\\$/, '')}\\${n.name}`;
  return (
    <div className="min-h-0 flex-1 overflow-y-auto p-1">
      <div className="grid gap-2.5" style={{ gridTemplateColumns: `repeat(auto-fill, minmax(${tile}px, 1fr))` }} role="list" aria-label="Folder contents">
        {items.map((n) => (
          <Tile key={n.id} n={n} path={pathOf(n)} tile={tile} value={metric === 'alloc' ? n.alloc : n.size} parent={parentSize} selected={selected === n.id} previewsOn={previewsOn} onSelect={onSelect} onOpen={onOpen} onMenu={onMenu} />
        ))}
      </div>
      {hasMore && <div ref={end} className="h-8" />}
    </div>
  );
}
