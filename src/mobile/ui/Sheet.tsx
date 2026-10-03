// Bottom sheet: slides up, drag the handle down (or tap outside, or press
// back) to close. Content scrolls inside; the sheet rides above the
// on-screen keyboard.

import type { ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { AnimatePresence, motion, useDragControls } from 'motion/react';
import { X } from 'lucide-react';
import { useBackHandler } from '../lib/back';
import { useKeyboardInset, cx } from '../lib/util';

export function Sheet({
  open,
  onClose,
  title,
  subtitle,
  children,
  footer,
  className,
}: {
  open: boolean;
  onClose: () => void;
  title?: ReactNode;
  subtitle?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  className?: string;
}) {
  useBackHandler(open, onClose);
  const kb = useKeyboardInset();
  const drag = useDragControls();
  return createPortal(
    <AnimatePresence>
      {open && (
        <div className="fixed inset-0 z-[60]" key="sheet">
          <motion.div
            className="absolute inset-0 bg-black/60"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.2 }}
            onClick={onClose}
          />
          <motion.div
            role="dialog"
            aria-modal="true"
            className={cx(
              'absolute inset-x-0 bottom-0 mx-auto flex max-w-[640px] flex-col rounded-t-[28px] border-t border-line bg-sheet shadow-[0_-20px_60px_-20px_rgba(0,0,0,.6)]',
              className,
            )}
            style={{ bottom: kb, maxHeight: `calc(100dvh - ${kb}px - var(--safe-top) - 24px)` }}
            initial={{ y: '100%' }}
            animate={{ y: 0 }}
            exit={{ y: '100%' }}
            transition={{ type: 'spring', damping: 34, stiffness: 380, mass: 0.9 }}
            drag="y"
            dragListener={false}
            dragControls={drag}
            dragConstraints={{ top: 0, bottom: 0 }}
            dragElastic={{ top: 0, bottom: 0.7 }}
            onDragEnd={(_, info) => {
              if (info.offset.y > 90 || info.velocity.y > 600) onClose();
            }}
          >
            <div className="shrink-0 touch-none px-5 pb-2 pt-2.5" onPointerDown={(e) => drag.start(e)}>
              <div className="mx-auto mb-3 h-1.5 w-11 rounded-full bg-surface-3" />
              {(title || subtitle) && (
                <div className="flex items-start gap-3">
                  <div className="min-w-0 flex-1">
                    {title && <div className="truncate font-display text-xl font-bold">{title}</div>}
                    {subtitle && <div className="mt-0.5 text-sm text-dim">{subtitle}</div>}
                  </div>
                  <button aria-label="Close" onClick={onClose} className="press -mr-1 grid h-9 w-9 shrink-0 place-items-center rounded-full bg-surface-2 text-dim">
                    <X size={18} />
                  </button>
                </div>
              )}
            </div>
            <div className="scroller min-h-0 flex-1 px-5 pb-4">{children}</div>
            {footer && <div className="shrink-0 border-t border-line px-5 pb-[calc(var(--safe-bottom)+14px)] pt-3">{footer}</div>}
            {!footer && <div className="shrink-0" style={{ height: 'calc(var(--safe-bottom) + 8px)' }} />}
          </motion.div>
        </div>
      )}
    </AnimatePresence>,
    document.body,
  );
}
