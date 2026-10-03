// Small building blocks used across the phone screens.

import type { ReactNode, ButtonHTMLAttributes } from 'react';
import { LoaderCircle, CircleAlert, RefreshCw } from 'lucide-react';
import { cx } from '../lib/util';

type Variant = 'primary' | 'soft' | 'ghost' | 'danger' | 'outline';

export function Button({
  variant = 'primary',
  size = 'md',
  loading,
  icon,
  className,
  children,
  disabled,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant; size?: 'sm' | 'md' | 'lg'; loading?: boolean; icon?: ReactNode }) {
  const v: Record<Variant, string> = {
    primary: 'grad-bg text-white shadow-[0_8px_24px_-10px_var(--accent-glow)]',
    soft: 'bg-accent-soft text-accent',
    ghost: 'bg-surface-2 text-fg',
    danger: 'bg-bad text-white shadow-[0_8px_24px_-12px_rgba(248,113,113,.7)]',
    outline: 'border border-line-strong text-fg',
  };
  const s = { sm: 'h-9 px-3.5 text-sm rounded-xl gap-1.5', md: 'h-12 px-5 text-[15px] rounded-2xl gap-2', lg: 'h-14 px-6 text-base rounded-2xl gap-2.5' }[size];
  return (
    <button
      {...rest}
      disabled={disabled || loading}
      className={cx('press inline-flex items-center justify-center font-semibold whitespace-nowrap disabled:opacity-50', v[variant], s, className)}
    >
      {loading ? <LoaderCircle className="spin" size={size === 'sm' ? 16 : 18} /> : icon}
      {children}
    </button>
  );
}

export function IconButton({ label, className, children, ...rest }: ButtonHTMLAttributes<HTMLButtonElement> & { label: string }) {
  return (
    <button
      aria-label={label}
      title={label}
      {...rest}
      className={cx('press grid h-10 w-10 shrink-0 place-items-center rounded-full bg-surface-2 text-fg disabled:opacity-40', className)}
    >
      {children}
    </button>
  );
}

export function Spinner({ size = 20, className }: { size?: number; className?: string }) {
  return <LoaderCircle size={size} className={cx('spin text-accent', className)} />;
}

export function Skeleton({ className }: { className?: string }) {
  return <div className={cx('skeleton', className)} />;
}

export function ListSkeleton({ rows = 6, thumb = true }: { rows?: number; thumb?: boolean }) {
  return (
    <div className="space-y-2">
      {Array.from({ length: rows }, (_, i) => (
        <div key={i} className="flex items-center gap-3 rounded-2xl p-2.5" style={{ opacity: 1 - i * 0.12 }}>
          {thumb && <Skeleton className="h-11 w-11 rounded-xl" />}
          <div className="flex-1 space-y-2">
            <Skeleton className="h-3.5 w-3/5" />
            <Skeleton className="h-3 w-2/5" />
          </div>
        </div>
      ))}
    </div>
  );
}

export function Empty({ icon, title, body, action }: { icon: ReactNode; title: string; body?: ReactNode; action?: ReactNode }) {
  return (
    <div className="fade-up flex flex-col items-center px-6 py-12 text-center">
      <div className="mb-4 grid h-16 w-16 place-items-center rounded-3xl bg-accent-soft text-accent">{icon}</div>
      <div className="font-display text-lg font-semibold">{title}</div>
      {body && <div className="mt-1.5 max-w-[300px] text-[15px] text-dim">{body}</div>}
      {action && <div className="mt-5">{action}</div>}
    </div>
  );
}

export function ErrorState({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <div className="fade-up flex flex-col items-center px-6 py-12 text-center">
      <div className="mb-4 grid h-16 w-16 place-items-center rounded-3xl bg-bad/15 text-bad">
        <CircleAlert size={30} />
      </div>
      <div className="font-display text-lg font-semibold">Something went wrong</div>
      <div className="mt-1.5 max-w-[300px] text-[15px] text-dim">{message}</div>
      {onRetry && (
        <Button variant="ghost" className="mt-5" icon={<RefreshCw size={17} />} onClick={onRetry}>
          Try again
        </Button>
      )}
    </div>
  );
}

export function PageHeader({ title, subtitle, left, right, className }: { title: ReactNode; subtitle?: ReactNode; left?: ReactNode; right?: ReactNode; className?: string }) {
  return (
    <header className={cx('px-safe flex items-center gap-3 pb-3 pt-[calc(var(--safe-top)+14px)]', className)}>
      {left}
      <div className="min-w-0 flex-1">
        {subtitle && <div className="truncate text-[13px] font-medium text-dim">{subtitle}</div>}
        <h1 className="truncate font-display text-[28px] font-bold leading-tight tracking-tight">{title}</h1>
      </div>
      {right && <div className="flex shrink-0 items-center gap-2">{right}</div>}
    </header>
  );
}

export function SectionTitle({ children, right }: { children: ReactNode; right?: ReactNode }) {
  return (
    <div className="mb-2.5 mt-6 flex items-center justify-between px-1">
      <h2 className="text-[13px] font-semibold uppercase tracking-[0.08em] text-faint">{children}</h2>
      {right}
    </div>
  );
}

export function Segmented<T extends string>({ value, options, onChange, className }: { value: T; options: { value: T; label: ReactNode }[]; onChange: (v: T) => void; className?: string }) {
  return (
    <div className={cx('flex rounded-2xl bg-surface-2 p-1', className)} role="tablist">
      {options.map((o) => (
        <button
          key={o.value}
          role="tab"
          aria-selected={o.value === value}
          onClick={() => onChange(o.value)}
          className={cx(
            'flex h-9 flex-1 items-center justify-center gap-1.5 rounded-xl px-2 text-sm font-semibold transition-colors',
            o.value === value ? 'bg-elev text-fg shadow-[0_2px_10px_-4px_rgba(0,0,0,.4)]' : 'text-dim',
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Switch({ on, onChange, label, disabled }: { on: boolean; onChange: (v: boolean) => void; label: string; disabled?: boolean }) {
  return <button role="switch" aria-checked={on} aria-label={label} disabled={disabled} data-on={on} onClick={() => onChange(!on)} className="switch disabled:opacity-40" />;
}

export function ProgressBar({ value, active, tone = 'accent', className }: { value: number; active?: boolean; tone?: 'accent' | 'good' | 'bad' | 'warn' | 'dim'; className?: string }) {
  const color = { accent: 'grad-bg', good: 'bg-good', bad: 'bg-bad', warn: 'bg-warn', dim: 'bg-faint' }[tone];
  return (
    <div className={cx('h-1.5 overflow-hidden rounded-full bg-surface-3', className)}>
      <div className={cx('h-full rounded-full transition-[width] duration-300 ease-out', color, active && 'bar-active')} style={{ width: `${Math.max(0, Math.min(1, value)) * 100}%` }} />
    </div>
  );
}

/** Circular gauge (CPU, RAM, countdowns). */
export function Ring({ value, size = 76, stroke = 8, children, color, track = 'var(--surface-3)' }: { value: number; size?: number; stroke?: number; children?: ReactNode; color?: string; track?: string }) {
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const v = Math.max(0, Math.min(1, value));
  const gid = `g${size}${stroke}`;
  return (
    <div className="relative grid place-items-center" style={{ width: size, height: size }}>
      <svg width={size} height={size} className="-rotate-90">
        <defs>
          <linearGradient id={gid} x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor="#8b5cf6" />
            <stop offset="1" stopColor="#22d3ee" />
          </linearGradient>
        </defs>
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke={track} strokeWidth={stroke} />
        <circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={color ?? `url(#${gid})`}
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={c}
          strokeDashoffset={c * (1 - v)}
          style={{ transition: 'stroke-dashoffset .6s cubic-bezier(.2,.8,.2,1), stroke .3s' }}
        />
      </svg>
      <div className="absolute inset-0 grid place-items-center">{children}</div>
    </div>
  );
}

export function Row({ icon, title, subtitle, right, onClick, className }: { icon?: ReactNode; title: ReactNode; subtitle?: ReactNode; right?: ReactNode; onClick?: () => void; className?: string }) {
  const Tag = onClick ? 'button' : 'div';
  return (
    <Tag onClick={onClick} className={cx('flex w-full items-center gap-3.5 px-4 py-3.5 text-left', onClick && 'press active:bg-surface-2', className)}>
      {icon && <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-surface-2 text-dim">{icon}</div>}
      <div className="min-w-0 flex-1">
        <div className="truncate text-[15px] font-semibold">{title}</div>
        {subtitle && <div className="truncate text-[13px] text-dim">{subtitle}</div>}
      </div>
      {right}
    </Tag>
  );
}
