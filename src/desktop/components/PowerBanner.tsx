import { motion } from 'motion/react';
import { Power } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../api';
import { cx } from '../lib/cx';
import { useNow } from '../lib/hooks';
import { formatCountdown } from '../lib/util';
import { useLive } from '../state/live';
import { toast } from '../state/toasts';
import { Button } from './ui/Button';

/** Countdown for a power action a phone requested, with a big Cancel. */
export function PowerBanner({ inline }: { inline?: boolean }) {
  const power = useLive((s) => s.power);
  const setPower = useLive((s) => s.setPower);
  const now = useNow(250);
  const [busy, setBusy] = useState(false);
  if (!power) return null;
  const left = power.at - now / 1000;
  const urgent = left < 15;
  const cancel = async () => {
    setBusy(true);
    try {
      await api.power.cancel();
      setPower(null);
      toast.success(`${power.label} cancelled`);
    } catch (e) {
      toast.error('Could not cancel', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <motion.div
      initial={{ opacity: 0, y: inline ? 6 : -16 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -16 }}
      role="alert"
      className={cx(
        'flex items-center gap-4 rounded-2xl border px-4 py-3',
        urgent ? 'border-bad/40 bg-bad/12' : 'border-warn/35 bg-warn/10',
        inline ? '' : 'glass pointer-events-auto w-[560px] shadow-[0_18px_60px_-16px_rgba(0,0,0,0.6)]',
      )}
    >
      <div className={cx('flex h-10 w-10 shrink-0 items-center justify-center rounded-xl', urgent ? 'pulse-ring bg-bad/20 text-bad' : 'bg-warn/15 text-warn')}>
        <Power size={19} aria-hidden />
      </div>
      <div className="min-w-0 flex-1">
        <div className="text-[14px] font-semibold text-fg">
          {power.label} in <span className="font-mono tabular">{formatCountdown(left)}</span>
        </div>
        <div className="truncate text-[12.5px] text-dim">Requested by {power.requestedBy}. Save your work or cancel it here.</div>
      </div>
      <Button variant="danger" size="lg" loading={busy} onClick={cancel} className="font-semibold">
        Cancel {power.label.toLowerCase()}
      </Button>
    </motion.div>
  );
}
