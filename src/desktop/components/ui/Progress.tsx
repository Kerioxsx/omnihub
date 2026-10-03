import { motion } from 'motion/react';
import { type ReactNode, useId } from 'react';
import { cx } from '../../lib/cx';

/** Circular progress (0..1). `null` spins as indeterminate. */
export function ProgressRing({
  value,
  size = 64,
  stroke = 6,
  children,
  className,
  track = 'var(--surface-3)',
  color,
  label,
}: {
  value: number | null;
  size?: number;
  stroke?: number;
  children?: ReactNode;
  className?: string;
  track?: string;
  /** Solid colour instead of the accent gradient (e.g. warn when a drive is full). */
  color?: string;
  label?: string;
}) {
  const id = useId().replace(/:/g, '');
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const v = value == null ? 0.28 : Math.max(0, Math.min(1, value));
  return (
    <div className={cx('relative inline-flex shrink-0 items-center justify-center', className)} style={{ width: size, height: size }} role="progressbar" aria-label={label} aria-valuemin={0} aria-valuemax={100} aria-valuenow={value == null ? undefined : Math.round(v * 100)}>
      <svg width={size} height={size} className={cx('-rotate-90', value == null && 'spin')}>
        <defs>
          <linearGradient id={`ring-${id}`} x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor="var(--accent)" />
            <stop offset="1" stopColor="var(--accent-2)" />
          </linearGradient>
        </defs>
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke={track} strokeWidth={stroke} />
        <motion.circle
          cx={size / 2}
          cy={size / 2}
          r={r}
          fill="none"
          stroke={color ?? `url(#ring-${id})`}
          strokeWidth={stroke}
          strokeLinecap="round"
          strokeDasharray={c}
          initial={{ strokeDashoffset: c }}
          animate={{ strokeDashoffset: c * (1 - v) }}
          transition={{ type: 'spring', stiffness: 60, damping: 18 }}
        />
      </svg>
      {children && <div className="absolute inset-0 flex items-center justify-center">{children}</div>}
    </div>
  );
}

/** Horizontal progress bar (0..1). `null` shows an indeterminate sweep. */
export function ProgressBar({ value, className, tone = 'accent', height = 6, label }: { value: number | null; className?: string; tone?: 'accent' | 'good' | 'warn' | 'bad'; height?: number; label?: string }) {
  const fill = tone === 'accent' ? 'accent-gradient' : { good: 'bg-good', warn: 'bg-warn', bad: 'bg-bad' }[tone];
  return (
    <div
      className={cx('relative w-full overflow-hidden rounded-full bg-surface-3', value == null && 'indeterminate', className)}
      style={{ height }}
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={value == null ? undefined : Math.round(value * 100)}
    >
      {value != null && <motion.div className={cx('h-full rounded-full', fill)} initial={{ width: 0 }} animate={{ width: `${Math.max(0, Math.min(1, value)) * 100}%` }} transition={{ type: 'spring', stiffness: 80, damping: 20 }} />}
    </div>
  );
}

/** Stacked usage bar: segments with a 2px surface gap between them. */
export function UsageBar({ segments, height = 8, className }: { segments: { value: number; color: string; label: string }[]; height?: number; className?: string }) {
  const total = segments.reduce((a, s) => a + s.value, 0) || 1;
  return (
    <div className={cx('flex w-full gap-[2px] overflow-hidden rounded-full', className)} style={{ height }}>
      {segments
        .filter((s) => s.value > 0)
        .map((s) => (
          <motion.div
            key={s.label}
            title={s.label}
            className="h-full first:rounded-l-full last:rounded-r-full"
            style={{ background: s.color }}
            initial={{ flexGrow: 0 }}
            animate={{ flexGrow: s.value / total }}
            transition={{ type: 'spring', stiffness: 70, damping: 20 }}
          />
        ))}
    </div>
  );
}
