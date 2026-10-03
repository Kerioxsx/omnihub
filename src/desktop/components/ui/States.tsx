import { motion } from 'motion/react';
import type { LucideIcon } from 'lucide-react';
import { CircleAlert, RefreshCw } from 'lucide-react';
import type { ReactNode } from 'react';
import { cx } from '../../lib/cx';
import { Button } from './Button';

export function EmptyState({ icon: Icon, title, description, action, className, compact }: { icon: LucideIcon; title: ReactNode; description?: ReactNode; action?: ReactNode; className?: string; compact?: boolean }) {
  return (
    <motion.div initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.25 }} className={cx('flex flex-col items-center justify-center text-center', compact ? 'gap-2 py-8' : 'gap-3 py-16', className)}>
      <div className={cx('relative flex items-center justify-center rounded-2xl border border-line bg-surface-2', compact ? 'h-11 w-11' : 'h-16 w-16')}>
        <div className="absolute inset-0 rounded-2xl bg-[radial-gradient(circle_at_30%_20%,var(--accent-soft),transparent_70%)]" />
        <Icon size={compact ? 20 : 28} className="relative text-accent" aria-hidden />
      </div>
      <div className={cx('font-display font-semibold text-fg', compact ? 'text-sm' : 'text-[17px]')}>{title}</div>
      {description && <div className={cx('max-w-md text-dim', compact ? 'text-[12.5px]' : 'text-[13.5px] leading-relaxed')}>{description}</div>}
      {action && <div className="mt-2 flex items-center gap-2">{action}</div>}
    </motion.div>
  );
}

export function ErrorState({ title = 'Something went wrong', error, onRetry, className }: { title?: string; error: string; onRetry?: () => void; className?: string }) {
  return (
    <div role="alert" className={cx('flex flex-col items-center justify-center gap-3 py-12 text-center', className)}>
      <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-bad/12 text-bad">
        <CircleAlert size={22} aria-hidden />
      </div>
      <div className="font-display text-[15px] font-semibold text-fg">{title}</div>
      <div className="max-w-lg font-mono text-[12px] text-dim">{error}</div>
      {onRetry && (
        <Button size="sm" icon={RefreshCw} onClick={onRetry}>
          Try again
        </Button>
      )}
    </div>
  );
}

/** Inline callout box (info / warning / danger). */
export function Callout({ tone = 'info', icon: Icon, title, children, className, action }: { tone?: 'info' | 'warn' | 'bad' | 'good' | 'accent'; icon?: LucideIcon; title?: ReactNode; children?: ReactNode; className?: string; action?: ReactNode }) {
  const tones = {
    info: 'border-info/25 bg-info/8 [&_.ci]:text-info',
    warn: 'border-warn/30 bg-warn/8 [&_.ci]:text-warn',
    bad: 'border-bad/30 bg-bad/8 [&_.ci]:text-bad',
    good: 'border-good/25 bg-good/8 [&_.ci]:text-good',
    accent: 'border-accent/25 bg-accent-soft [&_.ci]:text-accent',
  }[tone];
  return (
    <div className={cx('flex gap-3 rounded-xl border px-4 py-3', tones, className)}>
      {Icon && <Icon size={17} className="ci mt-0.5 shrink-0" aria-hidden />}
      <div className="min-w-0 flex-1 text-[13px] leading-relaxed text-dim">
        {title && <div className="mb-0.5 font-semibold text-fg">{title}</div>}
        {children}
      </div>
      {action && <div className="shrink-0 self-center">{action}</div>}
    </div>
  );
}
