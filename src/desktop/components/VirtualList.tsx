// A minimal windowed list: only the rows in view (plus overscan) are rendered.

import { type CSSProperties, type ReactNode, type UIEvent, useCallback, useEffect, useRef, useState } from 'react';
import { cx } from '../lib/cx';

export interface VirtualListProps<T> {
  items: T[];
  rowHeight: number;
  renderRow: (item: T, index: number, style: CSSProperties) => ReactNode;
  overscan?: number;
  className?: string;
  /** Called when the user scrolls near the end (for paging). */
  onEndReached?: () => void;
  ariaLabel?: string;
}

export function VirtualList<T>({ items, rowHeight, renderRow, overscan = 8, className, onEndReached, ariaLabel }: VirtualListProps<T>) {
  const ref = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [height, setHeight] = useState(600);
  const endRef = useRef(onEndReached);
  endRef.current = onEndReached;

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setHeight(el.clientHeight));
    ro.observe(el);
    setHeight(el.clientHeight);
    return () => ro.disconnect();
  }, []);

  const onScroll = useCallback(
    (e: UIEvent<HTMLDivElement>) => {
      const el = e.currentTarget;
      setScrollTop(el.scrollTop);
      if (el.scrollTop + el.clientHeight > el.scrollHeight - rowHeight * 10) endRef.current?.();
    },
    [rowHeight],
  );

  const start = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan);
  const end = Math.min(items.length, Math.ceil((scrollTop + height) / rowHeight) + overscan);
  const rows: ReactNode[] = [];
  for (let i = start; i < end; i++) rows.push(renderRow(items[i], i, { position: 'absolute', top: i * rowHeight, left: 0, right: 0, height: rowHeight }));

  return (
    <div ref={ref} onScroll={onScroll} className={cx('relative min-h-0 overflow-y-auto', className)} role="list" aria-label={ariaLabel}>
      <div style={{ height: items.length * rowHeight, position: 'relative' }}>{rows}</div>
    </div>
  );
}
