import type { LucideIcon } from 'lucide-react';
import { LoaderCircle } from 'lucide-react';
import type { HTMLAttributes, ReactNode } from 'react';
import { cx } from '../../lib/cx';

export function Card({ className, children, interactive, ...rest }: HTMLAttributes<HTMLDivElement> & { interactive?: boolean }) {
  return (
    <div className={cx('card', interactive && 'card-interactive', className)} {...rest}>
      {children}
    </div>
  );
}

export function CardHeader({ icon: Icon, title, subtitle, actions, className }: { icon?: LucideIcon; title: ReactNode; subtitle?: ReactNode; actions?: ReactNode; className?: string }) {
  return (
    <div className={cx('flex items-start gap-3', className)}>
      {Icon && (
        <div className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] bg-accent-soft text-accent">
          <Icon size={16} aria-hidden />
        </div>
      )}
      <div className="min-w-0 flex-1">
        <h3 className="font-display text-[15px] font-semibold leading-tight text-fg">{title}</h3>
        {subtitle && <div className="mt-0.5 text-[13px] text-dim">{subtitle}</div>}
      </div>
      {actions && <div className="flex shrink-0 items-center gap-1.5">{actions}</div>}
    </div>
  );
}

export type Tone = 'neutral' | 'accent' | 'good' | 'warn' | 'bad' | 'info';

const TONES: Record<Tone, string> = {
  neutral: 'bg-surface-2 text-dim border-line',
  accent: 'bg-accent-soft text-accent border-accent/25',
  good: 'bg-good/12 text-good border-good/25',
  warn: 'bg-warn/12 text-warn border-warn/25',
  bad: 'bg-bad/12 text-bad border-bad/25',
  info: 'bg-info/12 text-info border-info/25',
};

export function Badge({ tone = 'neutral', icon: Icon, children, className, title }: { tone?: Tone; icon?: LucideIcon; children: ReactNode; className?: string; title?: string }) {
  return (
    <span title={title} className={cx('inline-flex h-[22px] shrink-0 items-center gap-1 whitespace-nowrap rounded-full border px-2 text-[11.5px] font-medium', TONES[tone], className)}>
      {Icon && <Icon size={12} aria-hidden />}
      {children}
    </span>
  );
}

export function Dot({ tone = 'good', pulse }: { tone?: Tone; pulse?: boolean }) {
  const color = { neutral: 'bg-faint', accent: 'bg-accent', good: 'bg-good', warn: 'bg-warn', bad: 'bg-bad', info: 'bg-info' }[tone];
  return (
    <span className="relative inline-flex h-2 w-2 shrink-0">
      {pulse && <span className={cx('absolute inset-0 animate-ping rounded-full opacity-60', color)} />}
      <span className={cx('relative inline-flex h-2 w-2 rounded-full', color)} />
    </span>
  );
}

export function Kbd({ children, className }: { children: ReactNode; className?: string }) {
  return <kbd className={cx('inline-flex h-5 min-w-5 items-center justify-center rounded-md border border-line bg-surface-2 px-1.5 font-mono text-[10.5px] text-dim', className)}>{children}</kbd>;
}

export function Skeleton({ className }: { className?: string }) {
  return <div className={cx('skeleton', className)} aria-hidden />;
}

export function Spinner({ size = 16, className }: { size?: number; className?: string }) {
  return <LoaderCircle size={size} className={cx('spin text-accent', className)} aria-label="Loading" />;
}

/** A label/value pair used in detail panels. */
export function Meta({ label, children, mono }: { label: string; children: ReactNode; mono?: boolean }) {
  return (
    <div className="min-w-0">
      <div className="text-[11px] font-medium uppercase tracking-wider text-faint">{label}</div>
      <div className={cx('mt-0.5 truncate text-[13px] text-fg', mono && 'font-mono text-[12px]')}>{children}</div>
    </div>
  );
}

export function SectionTitle({ children, actions, className }: { children: ReactNode; actions?: ReactNode; className?: string }) {
  return (
    <div className={cx('mb-3 flex items-center justify-between gap-3', className)}>
      <h2 className="font-display text-[13px] font-semibold uppercase tracking-[0.08em] text-faint">{children}</h2>
      {actions}
    </div>
  );
}
