// The new layout: a floating top bar with the app's sections (Workspace,
// Play, Connect), search, status and the clock, and a slim rail with the
// pages of the section you're in. Settings → Appearance → Layout switches
// back to the classic sidebar.

import { Command, Gamepad2, LayoutGrid, Settings, Smartphone } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { motion } from 'motion/react';
import { useEffect, useState } from 'react';
import { cx } from '../lib/cx';
import { useStoredState } from '../lib/hooks';
import { type RouteId, navigate, useRoute } from '../lib/router';
import { MOD_LABEL } from '../lib/util';
import { usePalette } from '../state/dialogs';
import { connectedCount, useLive } from '../state/live';
import { useSettings } from '../state/settings';
import { Logo } from './Logo';
import { NAV, type NavItem } from './nav';
import { Dot, Kbd } from './ui/Card';

type SectionId = 'workspace' | 'play' | 'connect';
const SECTIONS: { id: SectionId; label: string; icon: LucideIcon; pages: NavItem['id'][] }[] = [
  { id: 'workspace', label: 'Workspace', icon: LayoutGrid, pages: ['home', 'storage', 'apps', 'screenshots', 'notes', 'chats', 'focus', 'counters', 'quick', 'music'] },
  { id: 'play', label: 'Play', icon: Gamepad2, pages: ['games', 'mods', 'client', 'servers', 'tasks', 'tools', 'rig'] },
  { id: 'connect', label: 'Connect', icon: Smartphone, pages: ['phone', 'screen', 'lights', 'vault'] },
];
const byId = new Map(NAV.map((n) => [n.id, n]));
/** Shorter names where the rail is narrow. */
const SHORT: Partial<Record<string, string>> = { screenshots: 'Shots', quick: 'Actions', screen: 'Share' };
const sectionOf = (id: string): SectionId | null => SECTIONS.find((s) => s.pages.includes(id as NavItem['id']))?.id ?? null;

function Clock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const t = setInterval(() => setNow(new Date()), 15_000);
    return () => clearInterval(t);
  }, []);
  return <span className="tabular-nums">{now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>;
}

export function TopBar() {
  const route = useRoute();
  const setPalette = usePalette((s) => s.setOpen);
  const info = useSettings((s) => s.info);
  const remote = useLive((s) => s.remote);
  const devices = useLive((s) => s.devices);
  const viewers = useLive((s) => s.viewers);
  const phones = connectedCount(remote, devices, viewers);
  // The page you were on last in each section (a section tab takes you back there).
  const [last, setLast] = useStoredState<Partial<Record<SectionId, string>>>('omnihub.sectionLast', {});
  const current = sectionOf(route.id);
  useEffect(() => {
    if (current && last[current] !== route.id) setLast({ ...last, [current]: route.id });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [route.id]);
  return (
    <header className="float relative z-20 mx-2.5 mt-2.5 grid h-[56px] shrink-0 grid-cols-[1fr_auto_1fr] items-center gap-4 rounded-[18px] px-3.5">
      <button type="button" onClick={() => navigate('home')} className="flex min-w-0 items-center gap-2.5 justify-self-start rounded-[12px] py-1 pr-2 transition-colors hover:bg-surface" aria-label="Home">
        <Logo size={30} />
        <span className="hidden min-w-0 text-left lg:block">
          <span className="block font-display text-[15.5px] font-semibold tracking-[-0.015em] text-fg">OmniHub</span>
          <span className="-mt-0.5 block truncate text-[11px] text-faint">{info?.hostname ?? 'PC companion'}</span>
        </span>
      </button>
      <nav aria-label="Sections" className="flex items-center gap-0.5 rounded-[14px] bg-surface p-1 shadow-[inset_0_0_0_1px_var(--border)]">
        {SECTIONS.map((s) => {
          const on = current === s.id;
          const Icon = s.icon;
          return (
            <button
              key={s.id}
              type="button"
              aria-current={on ? 'true' : undefined}
              onClick={() => navigate((last[s.id] && s.pages.includes(last[s.id] as NavItem['id']) ? last[s.id] : s.pages[0]) as RouteId)}
              className={cx('relative flex items-center gap-2 rounded-[10px] px-3.5 py-1.5 text-[13.5px] font-semibold transition-colors duration-150', on ? 'text-fg' : 'text-dim hover:text-fg')}
            >
              {on && <motion.span layoutId="section-on" transition={{ type: 'spring', stiffness: 520, damping: 42 }} className="absolute inset-0 rounded-[10px] bg-elev shadow-[0_1px_2px_rgba(0,0,0,0.35),0_0_0_1px_var(--border-strong)]" />}
              <Icon size={15} aria-hidden className={cx('relative transition-colors', on ? 'text-accent' : 'text-faint')} />
              <span className="relative">{s.label}</span>
            </button>
          );
        })}
      </nav>
      <div className="flex items-center gap-2.5 justify-self-end">
        <button type="button" onClick={() => setPalette(true)} className="flex h-9 items-center gap-2 rounded-[11px] bg-surface pl-3 pr-1.5 text-[13px] text-faint shadow-[inset_0_0_0_1px_var(--border)] transition-[color,box-shadow] duration-150 hover:text-dim hover:shadow-[inset_0_0_0_1px_var(--border-strong)] xl:w-56" aria-label="Search">
          <Command size={15} aria-hidden />
          <span className="hidden flex-1 text-left xl:inline">Search OmniHub</span>
          <Kbd className="hidden xl:inline-flex">{MOD_LABEL} K</Kbd>
        </button>
        <button type="button" onClick={() => navigate('phone')} title={remote?.running ? (phones ? `${phones} phone${phones > 1 ? 's' : ''} connected` : 'Phone companion on') : 'Phone companion off'} className="hidden items-center gap-1.5 text-[12px] text-dim hover:text-fg md:flex">
          <Dot tone={remote?.running ? (phones ? 'good' : 'accent') : 'neutral'} pulse={!!remote?.running && phones > 0} />
          {remote?.running ? (phones ? `${phones} phone${phones > 1 ? 's' : ''}` : 'Companion on') : 'Companion off'}
        </button>
        <span className="text-[14px] font-semibold text-fg">
          <Clock />
        </span>
        <button type="button" onClick={() => navigate('settings')} aria-current={route.id === 'settings' ? 'page' : undefined} aria-label="Settings" title="Settings" className={cx('grid h-9 w-9 place-items-center rounded-[10px] border transition-colors', route.id === 'settings' ? 'border-transparent bg-accent-soft text-accent' : 'border-line bg-surface text-dim hover:text-fg')}>
          <Settings size={17} />
        </button>
      </div>
    </header>
  );
}

export function Rail() {
  const route = useRoute();
  const sec = SECTIONS.find((s) => s.id === (sectionOf(route.id) ?? 'workspace'))!;
  return (
    <nav aria-label={`${sec.label} pages`} className="float relative z-20 my-2.5 ml-2.5 flex w-[80px] shrink-0 flex-col items-center gap-0.5 overflow-y-auto overflow-x-hidden rounded-[18px] px-1.5 py-2 [scrollbar-width:none]">
      {sec.pages.map((id) => {
        const n = byId.get(id);
        if (!n) return null;
        const on = route.id === id;
        const Icon = n.icon;
        return (
          <button key={id} type="button" onClick={() => navigate(id)} aria-current={on ? 'page' : undefined} title={`${n.label}: ${n.description}`} className={cx('group relative flex w-full flex-col items-center gap-1 rounded-[12px] px-1 pb-1.5 pt-1 transition-colors duration-150', on ? 'text-fg' : 'text-faint hover:text-dim')}>
            {on && <motion.span layoutId="rail-bar" transition={{ type: 'spring', stiffness: 520, damping: 42 }} className="absolute -left-1.5 top-2.5 h-6 w-[3px] rounded-r-full bg-accent" />}
            <span className={cx('relative grid h-10 w-10 place-items-center rounded-[12px] transition-[background-color,transform,box-shadow] duration-150 ease-out group-active:scale-95', on ? '' : 'group-hover:bg-surface-2')}>
              {on && <motion.span layoutId="rail-on" transition={{ type: 'spring', stiffness: 520, damping: 42 }} className="absolute inset-0 rounded-[12px] bg-accent-soft shadow-[inset_0_0_0_1px_color-mix(in_oklab,var(--accent)_30%,transparent)]" />}
              <Icon size={19} className={cx('relative transition-colors', on && 'text-accent')} aria-hidden />
            </span>
            <span className={cx('relative w-full truncate text-center text-[10.5px] leading-tight', on ? 'font-semibold' : 'font-medium')}>{SHORT[id] ?? n.label}</span>
          </button>
        );
      })}
    </nav>
  );
}
