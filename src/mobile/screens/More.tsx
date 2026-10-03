// More: menu of secondary pages (Notes, Apps, Vault, Settings) with a
// slide-in sub-page stack.

import { AnimatePresence, motion } from 'motion/react';
import { NotebookPen, AppWindow, KeyRound, Settings as SettingsIcon, ChevronRight, ShieldCheck, ShieldAlert, ArrowLeft } from 'lucide-react';
import type { ReactNode } from 'react';
import { useApp } from '../state';
import { Logo } from '../ui/Logo';
import { IconButton, PageHeader } from '../ui/common';
import { useBackHandler } from '../lib/back';
import { cx } from '../lib/util';
import { NotesPage } from './Notes';
import { AppsPage } from './Apps';
import { VaultPage } from './Vault';
import { SettingsPage } from './Settings';

export type MorePage = 'notes' | 'apps' | 'vault' | 'settings';

export function MoreScreen({ active, page, setPage, onUnpaired }: { active: boolean; page: MorePage | null; setPage: (p: MorePage | null) => void; onUnpaired: () => void }) {
  const info = useApp((s) => s.info);
  const f = info?.features;
  useBackHandler(active && page !== null, () => setPage(null));

  const items: { id: MorePage; title: string; sub: string; icon: ReactNode; tone: string; show: boolean }[] = [
    { id: 'notes', title: 'Notes & ideas', sub: 'Read, search and capture ideas for Claude', icon: <NotebookPen size={21} />, tone: 'bg-violet-500/15 text-violet-400', show: !!f?.notes },
    { id: 'apps', title: 'Apps', sub: 'Launch programs on the PC', icon: <AppWindow size={21} />, tone: 'bg-cyan-500/15 text-cyan-400', show: !!f?.apps },
    { id: 'vault', title: 'Vault', sub: 'Look up a password (secure connection)', icon: <KeyRound size={21} />, tone: 'bg-amber-500/15 text-amber-400', show: !!f?.vault },
    { id: 'settings', title: 'Settings', sub: 'Theme, PC info, unpair', icon: <SettingsIcon size={21} />, tone: 'bg-slate-400/15 text-slate-400', show: true },
  ];
  const off = [!f?.notes && 'Notes', !f?.apps && 'Apps', !f?.vault && 'Vault'].filter(Boolean) as string[];

  return (
    <div className="relative h-full overflow-hidden">
      <div className="scroller h-full">
        <PageHeader title="More" subtitle={info?.name} />
        <div className="px-safe pb-tabbar">
          <div className="card flex items-center gap-3.5 p-4">
            <Logo size={48} />
            <div className="min-w-0 flex-1">
              <div className="truncate font-display text-lg font-bold">{info?.name ?? 'Your PC'}</div>
              <div className={cx('flex items-center gap-1.5 text-[13px]', info?.tls ? 'text-good' : 'text-dim')}>
                {info?.tls ? <ShieldCheck size={14} /> : <ShieldAlert size={14} />}
                {info?.tls ? 'Encrypted (HTTPS)' : 'Local network (HTTP)'} · OmniHub {info?.version}
              </div>
            </div>
          </div>
          <div className="card mt-4 overflow-hidden">
            {items
              .filter((i) => i.show)
              .map((i, n) => (
                <button key={i.id} onClick={() => setPage(i.id)} className={cx('press flex w-full items-center gap-3.5 px-4 py-3.5 text-left active:bg-surface-2', n > 0 && 'border-t border-line')}>
                  <div className={cx('grid h-11 w-11 place-items-center rounded-2xl', i.tone)}>{i.icon}</div>
                  <div className="min-w-0 flex-1">
                    <div className="text-[16px] font-semibold">{i.title}</div>
                    <div className="truncate text-[13px] text-dim">{i.sub}</div>
                  </div>
                  <ChevronRight size={18} className="text-faint" />
                </button>
              ))}
          </div>
          {off.length > 0 && (
            <p className="mt-4 px-2 text-center text-xs leading-relaxed text-faint">
              Turned off on the PC: {off.join(', ')}.{!f?.vault && ' The vault also needs the secure (HTTPS) connection.'}
            </p>
          )}
        </div>
      </div>

      <AnimatePresence>
        {page && (
          <motion.div
            key={page}
            className="absolute inset-0 z-10 bg-bg"
            initial={{ x: '100%' }}
            animate={{ x: 0 }}
            exit={{ x: '100%' }}
            transition={{ type: 'spring', damping: 34, stiffness: 340 }}
          >
            {page === 'notes' && <NotesPage active={active} onBack={() => setPage(null)} />}
            {page === 'apps' && <AppsPage onBack={() => setPage(null)} />}
            {page === 'vault' && <VaultPage active={active} onBack={() => setPage(null)} />}
            {page === 'settings' && <SettingsPage onBack={() => setPage(null)} onUnpaired={onUnpaired} />}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

/** Header for sub-pages: back button, title, optional actions. */
export function SubHeader({ title, subtitle, onBack, right }: { title: string; subtitle?: string; onBack: () => void; right?: ReactNode }) {
  return (
    <PageHeader
      title={title}
      subtitle={subtitle}
      left={
        <IconButton label="Back" onClick={onBack} className="-ml-1">
          <ArrowLeft size={20} />
        </IconButton>
      }
      right={right}
    />
  );
}
