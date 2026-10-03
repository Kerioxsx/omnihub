import { motion } from 'motion/react';
import { House, FolderOpen, MonitorPlay, Power, Ellipsis } from 'lucide-react';
import type { ReactNode } from 'react';
import { useApp, type Tab } from '../state';
import { cx, vibrate } from '../lib/util';

const ITEMS: { tab: Tab; label: string; icon: ReactNode }[] = [
  { tab: 'home', label: 'Home', icon: <House size={22} /> },
  { tab: 'files', label: 'Files', icon: <FolderOpen size={22} /> },
  { tab: 'screen', label: 'Screen', icon: <MonitorPlay size={22} /> },
  { tab: 'power', label: 'Power', icon: <Power size={22} /> },
  { tab: 'more', label: 'More', icon: <Ellipsis size={22} /> },
];

export function TabBar({ hidden, badges }: { hidden: Tab[]; badges: Partial<Record<Tab, number>> }) {
  const { tab, setTab } = useApp();
  return (
    <nav
      className="fixed inset-x-0 bottom-0 z-40 border-t border-line bg-glass backdrop-blur-2xl backdrop-saturate-150"
      style={{ paddingBottom: 'var(--safe-bottom)' }}
      aria-label="Sections"
    >
      <div className="mx-auto flex h-[var(--tabbar-h)] max-w-[640px] items-stretch px-2">
        {ITEMS.filter((i) => !hidden.includes(i.tab)).map((i) => {
          const active = i.tab === tab;
          const badge = badges[i.tab] ?? 0;
          return (
            <button
              key={i.tab}
              onClick={() => {
                if (!active) vibrate(8);
                setTab(i.tab);
              }}
              aria-current={active ? 'page' : undefined}
              className={cx('relative flex flex-1 flex-col items-center justify-center gap-1 transition-colors', active ? 'text-fg' : 'text-faint')}
            >
              <div className="relative grid h-8 w-14 place-items-center">
                {active && (
                  <motion.div
                    layoutId="tab-pill"
                    className="absolute inset-0 rounded-full bg-accent-soft"
                    transition={{ type: 'spring', damping: 30, stiffness: 420 }}
                  />
                )}
                <span className={cx('relative transition-transform duration-200', active ? 'scale-105 text-accent' : '')}>{i.icon}</span>
                {badge > 0 && (
                  <span className="absolute -top-0.5 right-2 grid h-[18px] min-w-[18px] place-items-center rounded-full bg-accent px-1 text-[11px] font-bold text-white ring-2 ring-[var(--bg)]">
                    {badge > 9 ? '9+' : badge}
                  </span>
                )}
              </div>
              <span className={cx('text-[11px] font-semibold tracking-wide', active && 'text-fg')}>{i.label}</span>
            </button>
          );
        })}
      </div>
    </nav>
  );
}
