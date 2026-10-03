import { motion } from 'motion/react';
import type { LucideIcon } from 'lucide-react';
import { ChevronDown, Search, X } from 'lucide-react';
import { type InputHTMLAttributes, type ReactNode, type SelectHTMLAttributes, type TextareaHTMLAttributes, forwardRef, useId } from 'react';
import { cx } from '../../lib/cx';

// ---------- Switch ----------

export function Switch({ checked, onChange, label, disabled, size = 'md' }: { checked: boolean; onChange: (v: boolean) => void; label: string; disabled?: boolean; size?: 'md' | 'lg' }) {
  const lg = size === 'lg';
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cx(
        'relative inline-flex shrink-0 items-center rounded-full border transition-colors duration-200',
        lg ? 'h-8 w-14' : 'h-[22px] w-[38px]',
        checked ? 'accent-gradient border-white/10' : 'border-line-strong bg-surface-3',
        disabled && 'cursor-not-allowed opacity-50',
      )}
    >
      <motion.span
        layout
        transition={{ type: 'spring', stiffness: 700, damping: 35 }}
        className={cx('block rounded-full bg-white shadow-[0_2px_6px_rgba(0,0,0,0.35)]', lg ? 'h-6 w-6' : 'h-4 w-4')}
        style={{ marginLeft: checked ? (lg ? 28 : 18) : 3 }}
      />
    </button>
  );
}

// ---------- Segmented ----------

export interface SegOption<T extends string> {
  value: T;
  label: ReactNode;
  icon?: LucideIcon;
  disabled?: boolean;
  title?: string;
}

export function Segmented<T extends string>({ options, value, onChange, size = 'md', className, label }: { options: SegOption<T>[]; value: T; onChange: (v: T) => void; size?: 'sm' | 'md'; className?: string; label: string }) {
  const id = useId();
  return (
    <div role="radiogroup" aria-label={label} className={cx('inline-flex items-center gap-0.5 rounded-[11px] border border-line bg-surface p-0.5', className)}>
      {options.map((o) => {
        const active = o.value === value;
        const Icon = o.icon;
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={active}
            disabled={o.disabled}
            title={o.title}
            onClick={() => onChange(o.value)}
            className={cx(
              'relative inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-[9px] font-medium transition-colors',
              size === 'sm' ? 'h-7 px-2.5 text-xs' : 'h-8 px-3 text-[13px]',
              active ? 'text-fg' : 'text-dim hover:text-fg',
              o.disabled && 'cursor-not-allowed opacity-40 hover:text-dim',
            )}
          >
            {active && <motion.span layoutId={`seg-${id}`} transition={{ type: 'spring', stiffness: 500, damping: 38 }} className="absolute inset-0 rounded-[9px] border border-line-strong bg-surface-3 shadow-sm" />}
            <span className="relative z-10 inline-flex items-center gap-1.5">
              {Icon && <Icon size={size === 'sm' ? 13 : 14} aria-hidden />}
              {o.label}
            </span>
          </button>
        );
      })}
    </div>
  );
}

// ---------- Tabs ----------

export interface TabItem<T extends string> {
  value: T;
  label: string;
  icon?: LucideIcon;
  badge?: ReactNode;
}

export function Tabs<T extends string>({ items, value, onChange, className }: { items: TabItem<T>[]; value: T; onChange: (v: T) => void; className?: string }) {
  const id = useId();
  return (
    <div role="tablist" className={cx('flex items-center gap-1 overflow-x-auto border-b border-line', className)}>
      {items.map((t) => {
        const active = t.value === value;
        const Icon = t.icon;
        return (
          <button
            key={t.value}
            role="tab"
            type="button"
            aria-selected={active}
            onClick={() => onChange(t.value)}
            className={cx('relative inline-flex h-10 items-center gap-2 whitespace-nowrap px-3 text-[13px] font-medium transition-colors', active ? 'text-fg' : 'text-dim hover:text-fg')}
          >
            {Icon && <Icon size={15} aria-hidden className={active ? 'text-accent' : undefined} />}
            {t.label}
            {t.badge}
            {active && <motion.span layoutId={`tab-${id}`} transition={{ type: 'spring', stiffness: 500, damping: 40 }} className="accent-gradient absolute inset-x-2 -bottom-px h-[2px] rounded-full" />}
          </button>
        );
      })}
    </div>
  );
}

// ---------- Inputs ----------

const FIELD = 'w-full rounded-[10px] border border-line bg-surface text-fg placeholder:text-faint outline-none transition-[border-color,box-shadow,background-color] focus:border-accent/60 focus:bg-surface-2 focus:shadow-[0_0_0_3px_var(--accent-soft)]';

export interface TextInputProps extends InputHTMLAttributes<HTMLInputElement> {
  icon?: LucideIcon;
  right?: ReactNode;
  inputSize?: 'sm' | 'md' | 'lg';
}

export const TextInput = forwardRef<HTMLInputElement, TextInputProps>(function TextInput({ icon: Icon, right, className, inputSize = 'md', ...rest }, ref) {
  const h = inputSize === 'sm' ? 'h-8 text-[13px]' : inputSize === 'lg' ? 'h-11 text-[15px]' : 'h-9 text-sm';
  return (
    <div className={cx('relative flex items-center', className)}>
      {Icon && <Icon size={15} aria-hidden className="pointer-events-none absolute left-3 text-faint" />}
      <input ref={ref} className={cx(FIELD, h, Icon ? 'pl-9' : 'pl-3', right ? 'pr-10' : 'pr-3', 'disabled:opacity-50')} {...rest} />
      {right && <div className="absolute right-1.5 flex items-center">{right}</div>}
    </div>
  );
});

export const SearchInput = forwardRef<HTMLInputElement, Omit<TextInputProps, 'onChange' | 'value'> & { value: string; onChange: (v: string) => void }>(function SearchInput({ value, onChange, placeholder = 'Search…', ...rest }, ref) {
  return (
    <TextInput
      ref={ref}
      icon={Search}
      value={value}
      placeholder={placeholder}
      onChange={(e) => onChange(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === 'Escape' && value) {
          e.stopPropagation();
          onChange('');
        }
      }}
      right={
        value ? (
          <button type="button" aria-label="Clear search" onClick={() => onChange('')} className="flex h-6 w-6 items-center justify-center rounded-md text-faint hover:bg-surface-3 hover:text-fg">
            <X size={13} />
          </button>
        ) : undefined
      }
      {...rest}
    />
  );
});

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaHTMLAttributes<HTMLTextAreaElement>>(function Textarea({ className, ...rest }, ref) {
  return <textarea ref={ref} className={cx(FIELD, 'min-h-20 resize-y px-3 py-2 text-sm leading-relaxed', className)} {...rest} />;
});

export function Select({ className, children, ...rest }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <div className={cx('relative', className)}>
      <select className={cx(FIELD, 'h-9 cursor-pointer appearance-none pl-3 pr-8 text-sm [&>option]:bg-elev')} {...rest}>
        {children}
      </select>
      <ChevronDown size={14} aria-hidden className="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-faint" />
    </div>
  );
}

export function Field({ label, hint, children, htmlFor, className }: { label: string; hint?: ReactNode; children: ReactNode; htmlFor?: string; className?: string }) {
  return (
    <div className={cx('flex flex-col gap-1.5', className)}>
      <label htmlFor={htmlFor} className="text-[12.5px] font-medium text-dim">
        {label}
      </label>
      {children}
      {hint && <div className="text-xs text-faint">{hint}</div>}
    </div>
  );
}

export function Checkbox({ checked, onChange, label, disabled }: { checked: boolean; onChange: (v: boolean) => void; label: ReactNode; disabled?: boolean }) {
  return (
    <label className={cx('inline-flex cursor-pointer select-none items-center gap-2 text-[13px] text-fg', disabled && 'cursor-not-allowed opacity-50')}>
      <input type="checkbox" className="h-4 w-4 cursor-pointer rounded" checked={checked} disabled={disabled} onChange={(e) => onChange(e.target.checked)} />
      {label}
    </label>
  );
}
