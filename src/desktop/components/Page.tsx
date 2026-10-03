import type { ReactNode } from 'react';
import { cx } from '../lib/cx';

/** Page frame: title bar with contextual actions above a scrollable (or flexible) body. */
export function Page({ title, subtitle, actions, children, scroll = true, className, headerExtra }: { title: ReactNode; subtitle?: ReactNode; actions?: ReactNode; children: ReactNode; scroll?: boolean; className?: string; headerExtra?: ReactNode }) {
  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="flex shrink-0 flex-wrap items-end justify-between gap-x-6 gap-y-3 px-8 pb-4 pt-7">
        <div className="min-w-0">
          <h1 className="font-display text-[24px] font-semibold leading-tight tracking-[-0.02em] text-fg">{title}</h1>
          {subtitle && <div className="mt-1 text-[13.5px] text-dim">{subtitle}</div>}
        </div>
        {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
        {headerExtra && <div className="w-full">{headerExtra}</div>}
      </header>
      <div className={cx('min-h-0 flex-1 px-8', scroll ? 'overflow-y-auto pb-10' : 'flex flex-col pb-6', className)}>{children}</div>
    </div>
  );
}
