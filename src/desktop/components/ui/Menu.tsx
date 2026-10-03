// Right-click context menu: `openMenu(event, items)` from anywhere, rendered by <MenuHost/>.

import { AnimatePresence, motion } from 'motion/react';
import type { LucideIcon } from 'lucide-react';
import { type KeyboardEvent, useEffect, useLayoutEffect, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { create } from 'zustand';
import { cx } from '../../lib/cx';

export type MenuItem =
  | { kind?: 'item'; label: string; icon?: LucideIcon; onSelect: () => void; danger?: boolean; disabled?: boolean; hint?: string }
  | { kind: 'separator' }
  | { kind: 'header'; label: string };

interface MenuState {
  menu: { x: number; y: number; items: MenuItem[] } | null;
  show: (x: number, y: number, items: MenuItem[]) => void;
  hide: () => void;
}

const useMenu = create<MenuState>((set) => ({
  menu: null,
  show: (x, y, items) => set({ menu: { x, y, items } }),
  hide: () => set({ menu: null }),
}));

export function openMenu(e: { clientX: number; clientY: number; preventDefault: () => void }, items: MenuItem[]): void {
  e.preventDefault();
  useMenu.getState().show(e.clientX, e.clientY, items);
}

export function MenuHost() {
  const menu = useMenu((s) => s.menu);
  const hide = useMenu((s) => s.hide);
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState({ x: 0, y: 0 });
  const [active, setActive] = useState(-1);

  useLayoutEffect(() => {
    if (!menu || !ref.current) return;
    const { offsetWidth: w, offsetHeight: h } = ref.current;
    setPos({ x: Math.min(menu.x, window.innerWidth - w - 8), y: Math.min(menu.y, window.innerHeight - h - 8) });
    setActive(-1);
    ref.current.focus();
  }, [menu]);

  useEffect(() => {
    if (!menu) return;
    const close = (e: Event) => {
      if (e.type === 'mousedown' && ref.current?.contains(e.target as Node)) return;
      hide();
    };
    window.addEventListener('mousedown', close);
    window.addEventListener('blur', close);
    window.addEventListener('resize', close);
    window.addEventListener('wheel', close, { passive: true });
    return () => {
      window.removeEventListener('mousedown', close);
      window.removeEventListener('blur', close);
      window.removeEventListener('resize', close);
      window.removeEventListener('wheel', close);
    };
  }, [menu, hide]);

  const selectable = menu ? menu.items.map((it, i) => ((it.kind ?? 'item') === 'item' && !('disabled' in it && it.disabled) ? i : -1)).filter((i) => i >= 0) : [];

  const onKey = (e: KeyboardEvent) => {
    if (!menu) return;
    if (e.key === 'Escape') {
      e.stopPropagation();
      hide();
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const idx = selectable.indexOf(active);
      const next = e.key === 'ArrowDown' ? selectable[(idx + 1) % selectable.length] : selectable[(idx - 1 + selectable.length) % selectable.length];
      setActive(next ?? -1);
    } else if (e.key === 'Enter' && active >= 0) {
      const it = menu.items[active];
      if ((it.kind ?? 'item') === 'item' && 'onSelect' in it) {
        hide();
        it.onSelect();
      }
    }
  };

  return createPortal(
    <AnimatePresence>
      {menu && (
        <motion.div
          ref={ref}
          role="menu"
          tabIndex={-1}
          onKeyDown={onKey}
          initial={{ opacity: 0, scale: 0.96 }}
          animate={{ opacity: 1, scale: 1 }}
          exit={{ opacity: 0, scale: 0.98, transition: { duration: 0.08 } }}
          transition={{ duration: 0.12 }}
          style={{ left: pos.x, top: pos.y, transformOrigin: 'top left' }}
          className="glass fixed z-[100] min-w-[220px] rounded-xl border border-line-strong p-1 shadow-[0_18px_50px_-12px_rgba(0,0,0,0.55)] outline-none"
        >
          {menu.items.map((it, i) => {
            if (it.kind === 'separator') return <div key={i} className="mx-2 my-1 h-px bg-line" />;
            if (it.kind === 'header') return <div key={i} className="truncate px-2.5 pb-1 pt-1.5 text-[11px] font-medium uppercase tracking-wider text-faint">{it.label}</div>;
            const Icon = it.icon;
            return (
              <button
                key={i}
                role="menuitem"
                type="button"
                disabled={it.disabled}
                onMouseEnter={() => setActive(i)}
                onClick={() => {
                  hide();
                  it.onSelect();
                }}
                className={cx(
                  'flex w-full items-center gap-2.5 rounded-lg px-2.5 py-1.5 text-left text-[13px] transition-colors',
                  it.danger ? 'text-bad' : 'text-fg',
                  active === i && (it.danger ? 'bg-bad/12' : 'bg-surface-3'),
                  it.disabled && 'cursor-not-allowed opacity-40',
                )}
              >
                {Icon && <Icon size={15} aria-hidden className={it.danger ? 'text-bad' : 'text-dim'} />}
                <span className="flex-1">{it.label}</span>
                {it.hint && <span className="font-mono text-[11px] text-faint">{it.hint}</span>}
              </button>
            );
          })}
        </motion.div>
      )}
    </AnimatePresence>,
    document.body,
  );
}
