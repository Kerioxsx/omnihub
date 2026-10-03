// Modal dialogs and side drawers with a shared Escape stack, focus handling
// and enter/exit animations.

import { AnimatePresence, motion } from 'motion/react';
import type { LucideIcon } from 'lucide-react';
import { X } from 'lucide-react';
import { type KeyboardEvent, type ReactNode, type RefObject, useEffect, useId, useRef } from 'react';
import { createPortal } from 'react-dom';
import { cx } from '../../lib/cx';
import { IconButton } from './Button';

const stack: string[] = [];
const closers = new Map<string, () => void>();

if (typeof window !== 'undefined') {
  window.addEventListener(
    'keydown',
    (e) => {
      if (e.key !== 'Escape' || !stack.length) return;
      const top = stack[stack.length - 1];
      e.preventDefault();
      e.stopPropagation();
      closers.get(top)?.();
    },
    true,
  );
}

/** True while any modal/drawer is open (page shortcuts should pause). */
export function overlayOpen(): boolean {
  return stack.length > 0;
}

function useLayer(open: boolean, onClose: () => void, panel: RefObject<HTMLElement | null>) {
  const id = useId();
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    if (!open) return;
    const prev = document.activeElement as HTMLElement | null;
    stack.push(id);
    closers.set(id, () => closeRef.current());
    const t = setTimeout(() => {
      const el = panel.current;
      if (!el) return;
      const auto = el.querySelector<HTMLElement>('[data-autofocus]') ?? el.querySelector<HTMLElement>('input:not([type=hidden]):not([disabled]), textarea, select');
      (auto ?? el).focus({ preventScroll: true });
    }, 40);
    return () => {
      clearTimeout(t);
      const i = stack.indexOf(id);
      if (i >= 0) stack.splice(i, 1);
      closers.delete(id);
      prev?.focus?.({ preventScroll: true });
    };
  }, [open, id, panel]);
}

/** Keep Tab focus inside the panel. */
function trapTab(e: KeyboardEvent<HTMLElement>) {
  if (e.key !== 'Tab') return;
  const items = Array.from(e.currentTarget.querySelectorAll<HTMLElement>('button:not([disabled]), [href], input:not([disabled]), select, textarea, [tabindex]:not([tabindex="-1"])')).filter((el) => el.offsetParent !== null);
  if (!items.length) return;
  const first = items[0];
  const last = items[items.length - 1];
  if (e.shiftKey && document.activeElement === first) {
    e.preventDefault();
    last.focus();
  } else if (!e.shiftKey && document.activeElement === last) {
    e.preventDefault();
    first.focus();
  }
}

const SIZES = { sm: 'max-w-[420px]', md: 'max-w-[520px]', lg: 'max-w-[680px]', xl: 'max-w-[900px]' };

export interface ModalProps {
  open: boolean;
  onClose: () => void;
  title?: ReactNode;
  description?: ReactNode;
  icon?: LucideIcon;
  iconTone?: 'accent' | 'bad' | 'warn' | 'good';
  size?: keyof typeof SIZES;
  children?: ReactNode;
  footer?: ReactNode;
  className?: string;
  /** Render without the card chrome (e.g. a lightbox). */
  bare?: boolean;
  labelledBy?: string;
}

export function Modal({ open, onClose, title, description, icon: Icon, iconTone = 'accent', size = 'md', children, footer, className, bare, labelledBy }: ModalProps) {
  const panel = useRef<HTMLDivElement>(null);
  const titleId = useId();
  useLayer(open, onClose, panel);
  const toneCls = { accent: 'bg-accent-soft text-accent', bad: 'bg-bad/15 text-bad', warn: 'bg-warn/15 text-warn', good: 'bg-good/15 text-good' }[iconTone];
  return createPortal(
    <AnimatePresence>
      {open && (
        <motion.div className="fixed inset-0 z-[80] flex items-center justify-center p-6" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.16 }}>
          <div className="absolute inset-0 bg-black/55 backdrop-blur-[6px]" onMouseDown={onClose} aria-hidden />
          <motion.div
            ref={panel}
            role="dialog"
            aria-modal="true"
            aria-labelledby={labelledBy ?? (title ? titleId : undefined)}
            tabIndex={-1}
            onKeyDown={trapTab}
            initial={{ opacity: 0, scale: 0.96, y: 10 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.97, y: 6 }}
            transition={{ type: 'spring', stiffness: 420, damping: 34 }}
            className={cx('relative w-full outline-none', !bare && 'max-h-[calc(100vh-48px)] overflow-hidden rounded-[20px] border border-line-strong bg-elev shadow-[0_30px_80px_-20px_rgba(0,0,0,0.6)]', !bare && SIZES[size], className)}
          >
            {bare ? (
              children
            ) : (
              <div className="flex max-h-[calc(100vh-48px)] flex-col">
                {(title || Icon) && (
                  <div className="flex items-start gap-3 px-6 pb-2 pt-5">
                    {Icon && (
                      <div className={cx('flex h-10 w-10 shrink-0 items-center justify-center rounded-xl', toneCls)}>
                        <Icon size={19} aria-hidden />
                      </div>
                    )}
                    <div className="min-w-0 flex-1 pt-0.5">
                      {title && (
                        <h2 id={titleId} className="font-display text-[17px] font-semibold leading-snug text-fg">
                          {title}
                        </h2>
                      )}
                      {description && <div className="mt-1 text-[13.5px] leading-relaxed text-dim">{description}</div>}
                    </div>
                    <IconButton icon={X} label="Close" size="sm" onClick={onClose} className="-mr-2 -mt-1" />
                  </div>
                )}
                <div className="min-h-0 flex-1 overflow-y-auto px-6 py-3">{children}</div>
                {footer && <div className="flex items-center justify-end gap-2 border-t border-line bg-surface px-6 py-3.5">{footer}</div>}
              </div>
            )}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>,
    document.body,
  );
}

export function Drawer({ open, onClose, children, width = 440, label }: { open: boolean; onClose: () => void; children: ReactNode; width?: number; label: string }) {
  const panel = useRef<HTMLDivElement>(null);
  useLayer(open, onClose, panel);
  return createPortal(
    <AnimatePresence>
      {open && (
        <motion.div className="fixed inset-0 z-[70]" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.18 }}>
          <div className="absolute inset-0 bg-black/40 backdrop-blur-[2px]" onMouseDown={onClose} aria-hidden />
          <motion.aside
            ref={panel}
            role="dialog"
            aria-modal="true"
            aria-label={label}
            tabIndex={-1}
            onKeyDown={trapTab}
            initial={{ x: width + 40 }}
            animate={{ x: 0 }}
            exit={{ x: width + 40 }}
            transition={{ type: 'spring', stiffness: 380, damping: 38 }}
            style={{ width }}
            className="absolute bottom-3 right-3 top-3 flex flex-col overflow-hidden rounded-[20px] border border-line-strong bg-elev shadow-[0_30px_80px_-20px_rgba(0,0,0,0.6)] outline-none"
          >
            {children}
          </motion.aside>
        </motion.div>
      )}
    </AnimatePresence>,
    document.body,
  );
}
