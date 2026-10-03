import { type HTMLMotionProps, motion } from 'motion/react';
import type { LucideIcon } from 'lucide-react';
import { LoaderCircle } from 'lucide-react';
import { type ReactNode, forwardRef } from 'react';
import { cx } from '../../lib/cx';

export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'subtle' | 'outline';
export type ButtonSize = 'xs' | 'sm' | 'md' | 'lg';

const VARIANTS: Record<ButtonVariant, string> = {
  primary: 'accent-gradient text-white shadow-[0_8px_24px_-12px_var(--accent-glow)] hover:brightness-110 border border-white/10',
  secondary: 'bg-surface-2 border border-line text-fg hover:bg-surface-3 hover:border-line-strong',
  outline: 'border border-line-strong text-fg hover:bg-surface-2',
  ghost: 'text-dim hover:text-fg hover:bg-surface-2',
  danger: 'bg-bad/12 text-bad border border-bad/30 hover:bg-bad/20',
  subtle: 'bg-accent-soft text-accent border border-accent/20 hover:bg-accent/25',
};

const SIZES: Record<ButtonSize, string> = {
  xs: 'h-7 px-2.5 text-xs gap-1.5 rounded-lg',
  sm: 'h-8 px-3 text-[13px] gap-1.5 rounded-lg',
  md: 'h-9 px-3.5 text-sm gap-2 rounded-[10px]',
  lg: 'h-11 px-5 text-[15px] gap-2 rounded-xl',
};

const ICON: Record<ButtonSize, number> = { xs: 13, sm: 14, md: 16, lg: 18 };

export interface ButtonProps extends Omit<HTMLMotionProps<'button'>, 'children'> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: LucideIcon;
  iconRight?: LucideIcon;
  loading?: boolean;
  children?: ReactNode;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = 'secondary', size = 'md', icon: Icon, iconRight: IconRight, loading, disabled, className, children, type = 'button', ...rest },
  ref,
) {
  const off = disabled || loading;
  return (
    <motion.button
      ref={ref}
      type={type}
      disabled={off}
      whileTap={off ? undefined : { scale: 0.97 }}
      transition={{ type: 'spring', stiffness: 600, damping: 30 }}
      className={cx(
        'inline-flex shrink-0 select-none items-center justify-center whitespace-nowrap font-medium transition-[background-color,border-color,color,filter,opacity] duration-150',
        VARIANTS[variant],
        SIZES[size],
        off && 'cursor-not-allowed opacity-50',
        className,
      )}
      {...rest}
    >
      {loading ? <LoaderCircle size={ICON[size]} className="spin" aria-hidden /> : Icon ? <Icon size={ICON[size]} aria-hidden /> : null}
      {children}
      {IconRight && <IconRight size={ICON[size]} aria-hidden className="opacity-80" />}
    </motion.button>
  );
});

export interface IconButtonProps extends Omit<HTMLMotionProps<'button'>, 'children'> {
  icon: LucideIcon;
  label: string;
  size?: 'sm' | 'md' | 'lg';
  variant?: 'ghost' | 'secondary' | 'subtle' | 'danger';
  active?: boolean;
}

const ICON_BTN: Record<NonNullable<IconButtonProps['size']>, [string, number]> = {
  sm: ['h-7 w-7 rounded-lg', 14],
  md: ['h-8 w-8 rounded-[10px]', 16],
  lg: ['h-10 w-10 rounded-xl', 18],
};

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton({ icon: Icon, label, size = 'md', variant = 'ghost', active, className, disabled, type = 'button', ...rest }, ref) {
  const [box, px] = ICON_BTN[size];
  return (
    <motion.button
      ref={ref}
      type={type}
      aria-label={label}
      title={label}
      disabled={disabled}
      whileTap={disabled ? undefined : { scale: 0.92 }}
      transition={{ type: 'spring', stiffness: 600, damping: 30 }}
      className={cx(
        'inline-flex shrink-0 items-center justify-center transition-colors duration-150',
        box,
        variant === 'ghost' && (active ? 'bg-accent-soft text-accent' : 'text-dim hover:bg-surface-2 hover:text-fg'),
        variant === 'secondary' && 'border border-line bg-surface-2 text-fg hover:bg-surface-3',
        variant === 'subtle' && 'bg-accent-soft text-accent hover:bg-accent/25',
        variant === 'danger' && 'text-dim hover:bg-bad/15 hover:text-bad',
        disabled && 'cursor-not-allowed opacity-40',
        className,
      )}
      {...rest}
    >
      <Icon size={px} aria-hidden />
    </motion.button>
  );
});
