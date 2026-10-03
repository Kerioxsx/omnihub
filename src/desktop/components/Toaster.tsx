import { AnimatePresence, motion } from 'motion/react';
import { CircleAlert, CircleCheck, Info, TriangleAlert, X } from 'lucide-react';
import { useEffect } from 'react';
import { cx } from '../lib/cx';
import { type Toast, useToasts } from '../state/toasts';

const ICONS = { success: CircleCheck, error: CircleAlert, info: Info, warn: TriangleAlert };
const TONES = { success: 'text-good', error: 'text-bad', info: 'text-info', warn: 'text-warn' };

function ToastView({ t }: { t: Toast }) {
  const dismiss = useToasts((s) => s.dismiss);
  useEffect(() => {
    const timer = setTimeout(() => dismiss(t.id), t.duration);
    return () => clearTimeout(timer);
  }, [t.id, t.duration, t.stamp, dismiss]);
  const Icon = ICONS[t.tone];
  const compact = !t.description && !t.action;
  return (
    <motion.div
      layout
      initial={{ opacity: 0, y: 16, scale: 0.96 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, x: 40, transition: { duration: 0.18 } }}
      transition={{ type: 'spring', stiffness: 420, damping: 32 }}
      role={t.tone === 'error' ? 'alert' : 'status'}
      className={cx('glass pointer-events-auto flex w-[360px] items-start gap-3 rounded-2xl border border-line-strong px-4 shadow-[0_18px_50px_-14px_rgba(0,0,0,0.55)]', compact ? 'py-2.5' : 'py-3')}
    >
      <Icon size={18} className={cx('mt-px shrink-0', TONES[t.tone])} aria-hidden />
      <div className="min-w-0 flex-1">
        <div className="text-[13.5px] font-medium text-fg">{t.title}</div>
        {t.description && <div className="mt-0.5 break-words text-[12.5px] leading-snug text-dim">{t.description}</div>}
        {t.action && (
          <button
            type="button"
            onClick={() => {
              t.action?.run();
              dismiss(t.id);
            }}
            className="mt-2 text-[12.5px] font-semibold text-accent hover:underline"
          >
            {t.action.label}
          </button>
        )}
      </div>
      <button type="button" aria-label="Dismiss" onClick={() => dismiss(t.id)} className="-mr-1 flex h-6 w-6 items-center justify-center rounded-md text-faint hover:bg-surface-3 hover:text-fg">
        <X size={13} />
      </button>
    </motion.div>
  );
}

export function Toaster() {
  const toasts = useToasts((s) => s.toasts);
  return (
    <div className="pointer-events-none fixed bottom-5 right-5 z-[120] flex flex-col items-end gap-2" aria-live="polite">
      <AnimatePresence initial={false}>
        {toasts.map((t) => (
          <ToastView key={t.id} t={t} />
        ))}
      </AnimatePresence>
    </div>
  );
}
