import { AnimatePresence, motion } from 'motion/react';
import { CircleCheck, CircleAlert, Info } from 'lucide-react';
import { useToasts } from '../state';
import { cx } from '../lib/util';

export function Toasts({ aboveTabs }: { aboveTabs: boolean }) {
  const { toasts, dismiss } = useToasts();
  return (
    <div
      className={cx('pointer-events-none fixed inset-x-0 z-[70] flex flex-col items-center gap-2 px-3', aboveTabs ? 'bottom-tabbar' : 'bottom-[calc(var(--safe-bottom)+16px)]')}
      aria-live="polite"
    >
      <AnimatePresence initial={false}>
        {toasts.map((t) => (
          <motion.div
            key={t.id}
            layout
            initial={{ opacity: 0, y: 24, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 12, scale: 0.96, transition: { duration: 0.18 } }}
            transition={{ type: 'spring', damping: 26, stiffness: 340 }}
            drag="x"
            dragConstraints={{ left: 0, right: 0 }}
            dragElastic={0.8}
            onDragEnd={(_, i) => Math.abs(i.offset.x) > 80 && dismiss(t.id)}
            className="pointer-events-auto flex w-full max-w-[460px] items-start gap-3 rounded-2xl border border-line-strong bg-sheet/95 px-4 py-3 shadow-[0_16px_40px_-14px_rgba(0,0,0,.55)] backdrop-blur-xl"
            onClick={() => !t.action && dismiss(t.id)}
          >
            <div className={cx('mt-0.5 shrink-0', t.kind === 'success' ? 'text-good' : t.kind === 'error' ? 'text-bad' : 'text-accent')}>
              {t.kind === 'success' ? <CircleCheck size={20} /> : t.kind === 'error' ? <CircleAlert size={20} /> : <Info size={20} />}
            </div>
            <div className="min-w-0 flex-1">
              <div className="text-[15px] font-semibold leading-snug">{t.title}</div>
              {t.body && <div className="mt-0.5 line-clamp-2 break-all text-[13px] text-dim">{t.body}</div>}
            </div>
            {t.action && (
              <button
                className="press -my-1 shrink-0 rounded-xl bg-accent-soft px-3 py-2 text-sm font-semibold text-accent"
                onClick={(e) => {
                  e.stopPropagation();
                  t.action?.run();
                  dismiss(t.id);
                }}
              >
                {t.action.label}
              </button>
            )}
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}
