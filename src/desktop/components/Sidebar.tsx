import { AnimatePresence, motion } from 'motion/react';
import { Command, LockKeyhole, LockOpen, PanelLeftClose, PanelLeftOpen } from 'lucide-react';
import { type ReactNode, useMemo } from 'react';
import { cx } from '../lib/cx';
import { navigate, useRoute } from '../lib/router';
import { MOD_LABEL } from '../lib/util';
import { usePalette } from '../state/dialogs';
import { connectedCount, useLive } from '../state/live';
import { useSettings } from '../state/settings';
import { jobFraction, useStorage } from '../state/storage';
import { Logo } from './Logo';
import { NAV, type NavItem } from './nav';
import { Dot, Kbd } from './ui/Card';
import { ProgressRing } from './ui/Progress';

const GROUPS: { id: NavItem['group']; label: string | null }[] = [
  { id: 'workspace', label: null },
  { id: 'security', label: 'Security' },
  { id: 'devices', label: 'Devices' },
];

function useBadges(): Partial<Record<NavItem['id'], ReactNode>> {
  const job = useStorage((s) => s.job);
  const volumes = useStorage((s) => s.volumes);
  const remote = useLive((s) => s.remote);
  const devices = useLive((s) => s.devices);
  const viewers = useLive((s) => s.viewers);
  const vault = useLive((s) => s.vault);
  return useMemo(() => {
    const out: Partial<Record<NavItem['id'], ReactNode>> = {};
    if (job) {
      const f = jobFraction(job, volumes);
      out.storage = (
        <span className="flex items-center gap-1.5 text-[11px] font-medium tabular-nums text-accent">
          <ProgressRing value={f} size={16} stroke={2.5} label="Scan progress" />
          {f != null ? `${Math.round(f * 100)}%` : ''}
        </span>
      );
    }
    if (remote?.running) {
      const n = connectedCount(remote, devices, viewers);
      out.phone = (
        <span className={cx('flex h-[18px] min-w-[18px] items-center justify-center gap-1 rounded-full px-1.5 text-[10.5px] font-semibold', n ? 'bg-good/15 text-good' : 'bg-surface-3 text-dim')} title={n ? `${n} phone${n > 1 ? 's' : ''} connected` : 'Companion running'}>
          <Dot tone={n ? 'good' : 'neutral'} pulse={n > 0} />
          {n || 'On'}
        </span>
      );
    }
    if (vault?.exists) {
      out.vault = vault.unlocked ? <LockOpen size={14} className="text-good" aria-label="Vault unlocked" /> : <LockKeyhole size={14} className="text-faint" aria-label="Vault locked" />;
    }
    return out;
  }, [job, volumes, remote, devices, viewers, vault]);
}

export function Sidebar({ collapsed, onToggle }: { collapsed: boolean; onToggle: () => void }) {
  const route = useRoute();
  const badges = useBadges();
  const setPalette = usePalette((s) => s.setOpen);
  const info = useSettings((s) => s.info);
  const remote = useLive((s) => s.remote);
  const devices = useLive((s) => s.devices);
  const viewers = useLive((s) => s.viewers);
  const phones = connectedCount(remote, devices, viewers);

  const item = (n: NavItem, index: number) => {
    const active = route.id === n.id;
    const Icon = n.icon;
    return (
      <button
        key={n.id}
        type="button"
        onClick={() => navigate(n.id)}
        aria-current={active ? 'page' : undefined}
        title={collapsed ? `${n.label} (${MOD_LABEL}+${index + 1})` : undefined}
        className={cx('group relative flex h-9 w-full items-center gap-3 rounded-[10px] px-2.5 text-[13.5px] font-medium transition-colors', active ? 'text-fg' : 'text-dim hover:bg-surface hover:text-fg', collapsed && 'justify-center px-0')}
      >
        {active && <motion.span layoutId="nav-active" transition={{ type: 'spring', stiffness: 500, damping: 40 }} className="absolute inset-0 rounded-[10px] border border-line bg-surface-2 shadow-[inset_0_1px_0_var(--card-hi)]" />}
        {active && <motion.span layoutId="nav-bar" transition={{ type: 'spring', stiffness: 500, damping: 40 }} className="accent-gradient absolute -left-3 top-2 h-5 w-[3px] rounded-r-full" />}
        <Icon size={17} aria-hidden className={cx('relative z-10 shrink-0 transition-colors', active ? 'text-accent' : 'text-faint group-hover:text-dim')} />
        {!collapsed && <span className="relative z-10 flex-1 truncate text-left">{n.label}</span>}
        {!collapsed && badges[n.id] && <span className="relative z-10">{badges[n.id]}</span>}
        {collapsed && badges[n.id] && <span className="absolute right-1.5 top-1.5 z-10 h-2 w-2 rounded-full bg-accent ring-2 ring-[var(--bg-elev)]" />}
      </button>
    );
  };

  return (
    <motion.nav
      aria-label="Main"
      initial={false}
      animate={{ width: collapsed ? 72 : 248 }}
      transition={{ type: 'spring', stiffness: 380, damping: 40 }}
      className="glass relative z-20 flex h-full shrink-0 flex-col border-r border-line"
    >
      <div className={cx('flex h-[64px] items-center gap-2.5 px-4', collapsed && 'justify-center px-0')}>
        <Logo size={30} />
        <AnimatePresence initial={false}>
          {!collapsed && (
            <motion.div initial={{ opacity: 0, x: -6 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -6 }} className="min-w-0 flex-1">
              <div className="font-display text-[16px] font-semibold tracking-[-0.01em] text-fg">OmniHub</div>
              <div className="-mt-0.5 truncate text-[11px] text-faint">{info?.hostname ?? 'PC companion'}</div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      <div className={cx('px-3 pb-2', collapsed && 'px-2')}>
        <button
          type="button"
          onClick={() => setPalette(true)}
          aria-label="Open command palette"
          title={collapsed ? `Search (${MOD_LABEL}+K)` : undefined}
          className={cx('flex h-9 w-full items-center gap-2 rounded-[10px] border border-line bg-surface px-2.5 text-[13px] text-faint transition-colors hover:border-line-strong hover:text-dim', collapsed && 'justify-center px-0')}
        >
          <Command size={15} aria-hidden />
          {!collapsed && (
            <>
              <span className="flex-1 text-left">Jump to…</span>
              <Kbd>{MOD_LABEL} K</Kbd>
            </>
          )}
        </button>
      </div>

      <div className="flex-1 overflow-y-auto overflow-x-hidden px-3 pb-3 pt-1">
        {GROUPS.map((g) => (
          <div key={g.id} className="mb-2">
            {g.label && !collapsed && <div className="px-2.5 pb-1 pt-3 text-[10.5px] font-semibold uppercase tracking-[0.1em] text-faint">{g.label}</div>}
            {g.label && collapsed && <div className="mx-auto my-2 h-px w-6 bg-line" />}
            <div className="flex flex-col gap-0.5">{NAV.map((n, i) => (n.group === g.id ? item(n, i) : null))}</div>
          </div>
        ))}
      </div>

      <div className="border-t border-line p-3">
        {!collapsed && (
          <button type="button" onClick={() => navigate('phone')} className="mb-2 flex w-full items-center gap-2.5 rounded-[10px] border border-line bg-surface px-3 py-2 text-left transition-colors hover:border-line-strong">
            <Dot tone={remote?.running ? (phones ? 'good' : 'accent') : 'neutral'} pulse={!!remote?.running && phones > 0} />
            <div className="min-w-0 flex-1">
              <div className="truncate text-[12.5px] font-medium text-fg">{remote?.running ? (phones ? `${phones} phone${phones > 1 ? 's' : ''} connected` : 'Companion running') : 'Companion off'}</div>
              <div className="truncate text-[11px] text-faint">{remote?.running ? (remote.tls ? 'HTTPS' : 'HTTP') + ` · port ${remote.port}` : 'Turn on to pair a phone'}</div>
            </div>
          </button>
        )}
        {item(NAV[NAV.length - 1], NAV.length - 1)}
        <button
          type="button"
          onClick={onToggle}
          aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
          className={cx('mt-0.5 flex h-9 w-full items-center gap-3 rounded-[10px] px-2.5 text-[13px] text-faint transition-colors hover:bg-surface hover:text-dim', collapsed && 'justify-center px-0')}
        >
          {collapsed ? <PanelLeftOpen size={17} aria-hidden /> : <PanelLeftClose size={17} aria-hidden />}
          {!collapsed && <span>Collapse</span>}
        </button>
      </div>
    </motion.nav>
  );
}
