import { type ReactNode, useState } from 'react';
import { cx } from '../lib/cx';
import { useRoute } from '../lib/router';
import { NAV } from './nav';

/**
 * Page frame: the page's icon and title with its actions, above a
 * scrollable (or flexible) body. A hairline fades in under the header once
 * the body scrolls, so the header reads as a separate layer.
 */
export function Page({ title, subtitle, actions, children, scroll = true, className, headerExtra }: { title: ReactNode; subtitle?: ReactNode; actions?: ReactNode; children: ReactNode; scroll?: boolean; className?: string; headerExtra?: ReactNode }) {
  const route = useRoute();
  const Icon = NAV.find((n) => n.id === route.id)?.icon;
  const [scrolled, setScrolled] = useState(false);
  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className={cx('relative flex shrink-0 flex-wrap items-center justify-between gap-x-6 gap-y-3 px-8 pb-4 pt-6 transition-shadow duration-200', scrolled && 'shadow-[0_1px_0_var(--border)]')}>
        <div className="flex min-w-0 items-center gap-3.5">
          {Icon && (
            <div className="grid h-11 w-11 shrink-0 place-items-center rounded-[13px] bg-accent-soft text-accent shadow-[inset_0_0_0_1px_color-mix(in_oklab,var(--accent)_28%,transparent)]">
              <Icon size={21} aria-hidden />
            </div>
          )}
          <div className="min-w-0">
            <h1 className="font-display text-[26px] font-semibold leading-[1.1] tracking-[-0.025em] text-fg">{title}</h1>
            {subtitle && <div className="mt-1 text-[13.5px] text-dim">{subtitle}</div>}
          </div>
        </div>
        {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
        {headerExtra && <div className="w-full">{headerExtra}</div>}
      </header>
      <div onScroll={scroll ? (e) => setScrolled(e.currentTarget.scrollTop > 4) : undefined} className={cx('min-h-0 flex-1 px-8', scroll ? 'overflow-y-auto pb-10 pt-1' : 'flex flex-col pb-6', className)}>
        {children}
      </div>
    </div>
  );
}
