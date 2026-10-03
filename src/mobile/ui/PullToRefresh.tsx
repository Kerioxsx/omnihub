// A scroll container with simple pull-to-refresh: drag down from the top,
// release past the threshold to refresh. The browser's own pull-to-refresh
// is disabled by overscroll-behavior on the container.

import { useRef, useState, type ReactNode } from 'react';
import { RefreshCw } from 'lucide-react';
import { cx, vibrate } from '../lib/util';

const THRESHOLD = 72;

export function PullToRefresh({
  onRefresh,
  children,
  className,
  scrollRef,
  disabled,
}: {
  onRefresh: () => Promise<unknown> | void;
  children: ReactNode;
  className?: string;
  scrollRef?: (el: HTMLDivElement | null) => void;
  disabled?: boolean;
}) {
  const el = useRef<HTMLDivElement | null>(null);
  const start = useRef<{ y: number; x: number; active: boolean } | null>(null);
  const [pull, setPull] = useState(0);
  const [refreshing, setRefreshing] = useState(false);
  const armed = useRef(false);

  const onTouchStart = (e: React.TouchEvent) => {
    if (disabled || refreshing || !el.current || el.current.scrollTop > 0 || e.touches.length !== 1) {
      start.current = null;
      return;
    }
    start.current = { y: e.touches[0].clientY, x: e.touches[0].clientX, active: false };
  };
  const onTouchMove = (e: React.TouchEvent) => {
    const s = start.current;
    if (!s || !el.current) return;
    const dy = e.touches[0].clientY - s.y;
    const dx = e.touches[0].clientX - s.x;
    if (!s.active) {
      if (dy > 8 && Math.abs(dy) > Math.abs(dx) * 1.3 && el.current.scrollTop <= 0) s.active = true;
      else if (dy < -4 || Math.abs(dx) > 12) {
        start.current = null;
        return;
      } else return;
    }
    const p = Math.max(0, Math.min(140, (dy - 8) * 0.5));
    if (p >= THRESHOLD && !armed.current) {
      armed.current = true;
      vibrate(10);
    } else if (p < THRESHOLD) armed.current = false;
    setPull(p);
  };
  const onTouchEnd = async () => {
    const s = start.current;
    start.current = null;
    if (!s?.active) return;
    if (pull >= THRESHOLD) {
      setRefreshing(true);
      setPull(56);
      try {
        await onRefresh();
      } finally {
        setRefreshing(false);
        setPull(0);
        armed.current = false;
      }
    } else setPull(0);
  };

  const progress = Math.min(1, pull / THRESHOLD);
  return (
    <div
      ref={(n) => {
        el.current = n;
        scrollRef?.(n);
      }}
      className={cx('scroller relative h-full', className)}
      onTouchStart={onTouchStart}
      onTouchMove={onTouchMove}
      onTouchEnd={onTouchEnd}
      onTouchCancel={onTouchEnd}
    >
      <div
        className="pointer-events-none absolute inset-x-0 top-0 z-10 flex justify-center"
        style={{ transform: `translateY(${pull - 44}px)`, opacity: pull > 4 ? 1 : 0, transition: start.current ? 'none' : 'transform .25s ease, opacity .2s' }}
      >
        <div className="mt-[calc(var(--safe-top)+6px)] grid h-10 w-10 place-items-center rounded-full border border-line bg-sheet text-accent shadow-lg">
          <RefreshCw size={18} className={refreshing ? 'spin' : ''} style={{ transform: refreshing ? undefined : `rotate(${progress * 270}deg)`, opacity: 0.4 + progress * 0.6 }} />
        </div>
      </div>
      <div style={{ transform: pull ? `translateY(${pull}px)` : undefined, transition: start.current ? 'none' : 'transform .25s ease' }}>{children}</div>
    </div>
  );
}
