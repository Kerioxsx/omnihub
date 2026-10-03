import type { EntryKind, GeneratorOptions, Strength } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import type { LucideIcon } from 'lucide-react';
import { Box, CreditCard, KeyRound, Mail, RefreshCw, StickyNote, Wand, Wifi } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Checkbox } from '../../components/ui/Form';
import { cx } from '../../lib/cx';
import { useDebounced, useStoredState } from '../../lib/hooks';

export const KINDS: { value: EntryKind; label: string; icon: LucideIcon }[] = [
  { value: 'login', label: 'Login', icon: KeyRound },
  { value: 'email', label: 'Email', icon: Mail },
  { value: 'note', label: 'Secure note', icon: StickyNote },
  { value: 'card', label: 'Card', icon: CreditCard },
  { value: 'wifi', label: 'Wi-Fi', icon: Wifi },
  { value: 'other', label: 'Other', icon: Box },
];

export const kindIcon = (k: EntryKind): LucideIcon => KINDS.find((x) => x.value === k)?.icon ?? Box;

const SEG = ['bg-bad', 'bg-bad', 'bg-warn', 'bg-good', 'bg-good'];
const TEXT = ['text-bad', 'text-bad', 'text-warn', 'text-good', 'text-good'];

/** Live strength estimate from the backend (debounced). */
export function useStrength(password: string): Strength | null {
  const pw = useDebounced(password, 150);
  const [s, setS] = useState<Strength | null>(null);
  useEffect(() => {
    if (!pw) {
      setS(null);
      return;
    }
    let alive = true;
    api.vault.strength(pw).then((r) => alive && setS(r), () => undefined);
    return () => {
      alive = false;
    };
  }, [pw]);
  return password ? s : null;
}

export function StrengthMeter({ strength, compact }: { strength: Strength | null; compact?: boolean }) {
  const score = strength?.score ?? -1;
  return (
    <div aria-live="polite">
      <div className="flex gap-1" role="meter" aria-label="Password strength" aria-valuemin={0} aria-valuemax={4} aria-valuenow={Math.max(0, score)}>
        {[0, 1, 2, 3, 4].map((i) => (
          <div key={i} className="h-1.5 flex-1 overflow-hidden rounded-full bg-surface-3">
            <motion.div className={cx('h-full rounded-full', score >= 0 ? SEG[score] : '')} initial={false} animate={{ width: i <= score ? '100%' : '0%' }} transition={{ type: 'spring', stiffness: 300, damping: 30, delay: i * 0.03 }} />
          </div>
        ))}
      </div>
      {strength && (
        <div className="mt-1.5 flex items-baseline justify-between gap-3 text-[12px]">
          <span className={cx('font-medium', TEXT[strength.score])}>{strength.label}</span>
          <span className="tabular text-faint">≈ {strength.bits} bits</span>
        </div>
      )}
      {!compact && strength && strength.feedback.length > 0 && (
        <ul className="mt-1 space-y-0.5 text-[12px] text-faint">
          {strength.feedback.slice(0, 3).map((f) => (
            <li key={f}>• {f}</li>
          ))}
        </ul>
      )}
    </div>
  );
}

const DEFAULT_GEN: GeneratorOptions = { length: 20, lowercase: true, uppercase: true, digits: true, symbols: true, avoidAmbiguous: true };

/** Generator popover: options, preview, "Use". */
export function GeneratorButton({ onUse }: { onUse: (pw: string) => void }) {
  const [open, setOpen] = useState(false);
  const [opts, setOpts] = useStoredState<GeneratorOptions>('omnihub.vault.generator', DEFAULT_GEN);
  const [value, setValue] = useState('');
  const [err, setErr] = useState<string | null>(null);
  const ref = useRef<HTMLDivElement>(null);
  const gen = async (o = opts) => {
    try {
      setValue(await api.vault.generate(o));
      setErr(null);
    } catch (e) {
      setErr(errorText(e));
    }
  };
  useEffect(() => {
    if (open) void gen();
  }, [open]);
  useEffect(() => {
    if (!open) return;
    const close = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener('mousedown', close);
    return () => window.removeEventListener('mousedown', close);
  }, [open]);
  const set = (p: Partial<GeneratorOptions>) => {
    const next = { ...opts, ...p };
    setOpts(next);
    void gen(next);
  };
  return (
    <div ref={ref} className="relative">
      <Button size="sm" variant="subtle" icon={Wand} onClick={() => setOpen(!open)} aria-expanded={open}>
        Generate
      </Button>
      <AnimatePresence>
        {open && (
          <motion.div initial={{ opacity: 0, y: -6, scale: 0.97 }} animate={{ opacity: 1, y: 0, scale: 1 }} exit={{ opacity: 0, scale: 0.97 }} transition={{ duration: 0.14 }} className="absolute right-0 top-10 z-30 w-[340px] rounded-2xl border border-line-strong bg-elev p-4 shadow-[0_20px_60px_-15px_rgba(0,0,0,0.6)]">
            <div className="flex items-center gap-2 rounded-xl border border-line bg-surface px-3 py-2.5">
              <code className="min-w-0 flex-1 break-all font-mono text-[13px] text-fg">{value || '…'}</code>
              <button type="button" aria-label="Generate another" onClick={() => void gen()} className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg text-dim hover:bg-surface-3 hover:text-fg">
                <RefreshCw size={14} />
              </button>
            </div>
            {err && <div className="mt-2 text-[12px] text-bad">{err}</div>}
            <label className="mt-4 block text-[12.5px] text-dim">
              <span className="flex justify-between">
                Length <span className="font-semibold tabular text-fg">{opts.length}</span>
              </span>
              <input type="range" min={8} max={64} value={opts.length} onChange={(e) => set({ length: Number(e.target.value) })} className="mt-1.5 w-full" aria-label="Length" />
            </label>
            <div className="mt-3 grid grid-cols-2 gap-2">
              <Checkbox checked={opts.lowercase} onChange={(v) => set({ lowercase: v })} label="a–z" />
              <Checkbox checked={opts.uppercase} onChange={(v) => set({ uppercase: v })} label="A–Z" />
              <Checkbox checked={opts.digits} onChange={(v) => set({ digits: v })} label="0–9" />
              <Checkbox checked={opts.symbols} onChange={(v) => set({ symbols: v })} label="!@#$" />
              <div className="col-span-2">
                <Checkbox checked={opts.avoidAmbiguous} onChange={(v) => set({ avoidAmbiguous: v })} label="Avoid look-alikes (l, 1, O, 0)" />
              </div>
            </div>
            <Button
              variant="primary"
              size="sm"
              className="mt-4 w-full"
              disabled={!value}
              onClick={() => {
                onUse(value);
                setOpen(false);
              }}
            >
              Use this password
            </Button>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
