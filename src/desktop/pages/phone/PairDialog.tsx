import type { Device, PairingInfo } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { Check, Copy, QrCode, ShieldCheck, Smartphone } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Spinner } from '../../components/ui/Card';
import { Modal } from '../../components/ui/Overlay';
import { ProgressRing } from '../../components/ui/Progress';
import { ErrorState } from '../../components/ui/States';
import { useEvent, useNow } from '../../lib/hooks';
import { copyText, formatCountdown } from '../../lib/util';
import { useLive } from '../../state/live';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';

export function PairDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [info, setInfo] = useState<PairingInfo | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [paired, setPaired] = useState<Device | null>(null);
  const running = useLive((s) => !!s.remote?.running);
  const update = useSettings((s) => s.update);
  const pairedRef = useRef(false);
  const now = useNow(500);

  useEffect(() => {
    if (!open) return;
    setInfo(null);
    setError(null);
    setPaired(null);
    pairedRef.current = false;
    if (!running) return;
    let alive = true;
    api.remote
      .pairBegin()
      .then((p) => alive && setInfo(p))
      .catch((e: unknown) => alive && setError(errorText(e)));
    return () => {
      alive = false;
    };
  }, [open, running]);

  useEvent<Device>('remote:paired', (d) => {
    if (!open) return;
    pairedRef.current = true;
    setPaired(d);
    setTimeout(onClose, 2200);
  });

  const close = () => {
    if (!pairedRef.current && info) void api.remote.pairCancel().catch(() => undefined);
    onClose();
  };

  const left = info ? info.expiresAt - now / 1000 : 0;
  const expired = !!info && left <= 0;

  return (
    <Modal open={open} onClose={close} title={paired ? undefined : 'Pair a phone'} description={paired ? undefined : 'Scan the code with your phone camera, or open the address and type the PIN.'} icon={paired ? undefined : QrCode} size="xl">
      <AnimatePresence mode="wait">
        {paired ? (
          <motion.div key="ok" initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="flex flex-col items-center py-10 text-center">
            <div className="relative">
              <motion.div initial={{ scale: 0.4, opacity: 0.9 }} animate={{ scale: 2.4, opacity: 0 }} transition={{ duration: 1.1, ease: 'easeOut' }} className="absolute inset-0 rounded-full bg-good/30" />
              <motion.div initial={{ scale: 0.2, rotate: -40 }} animate={{ scale: 1, rotate: 0 }} transition={{ type: 'spring', stiffness: 260, damping: 14 }} className="relative flex h-20 w-20 items-center justify-center rounded-full bg-good text-white shadow-[0_16px_40px_-10px_var(--good)]">
                <Check size={40} strokeWidth={3} />
              </motion.div>
            </div>
            <motion.h2 initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0, transition: { delay: 0.2 } }} className="mt-6 font-display text-[22px] font-semibold text-fg">
              {paired.name} is paired
            </motion.h2>
            <motion.p initial={{ opacity: 0 }} animate={{ opacity: 1, transition: { delay: 0.3 } }} className="mt-1 text-[13.5px] text-dim">
              Connected from {paired.lastIp}. You can manage what it may do below.
            </motion.p>
          </motion.div>
        ) : !running ? (
          <motion.div key="off" className="flex flex-col items-center gap-4 py-10 text-center">
            <Smartphone size={34} className="text-accent" />
            <div className="text-[15px] font-semibold text-fg">The phone companion is off</div>
            <p className="max-w-md text-[13px] text-dim">It needs to run so your phone can connect. It only listens on your local network{useSettings.getState().settings?.remote.tls ? ', over HTTPS' : ''}.</p>
            <Button variant="primary" onClick={() => void update({ remote: { enabled: true } })}>
              Turn on and pair
            </Button>
          </motion.div>
        ) : error ? (
          <ErrorState key="err" title="Could not open pairing" error={error} />
        ) : !info ? (
          <div key="load" className="flex justify-center py-20">
            <Spinner size={26} />
          </div>
        ) : (
          <motion.div key="pair" initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} className="grid grid-cols-[300px_minmax(0,1fr)] gap-8 pb-4 pt-2">
            <div className="flex flex-col items-center">
              <div className="relative rounded-[22px] bg-white p-4 shadow-[0_20px_50px_-20px_var(--accent-glow)]">
                <div className="qr h-[244px] w-[244px]" aria-label="Pairing QR code" role="img" dangerouslySetInnerHTML={{ __html: info.qrSvg }} />
                {expired && <div className="absolute inset-0 flex items-center justify-center rounded-[22px] bg-white/90 text-[13px] font-semibold text-black">Expired</div>}
              </div>
              <div className="mt-4 flex items-center gap-2.5 text-[12.5px] text-dim">
                <ProgressRing value={Math.max(0, left / 300)} size={22} stroke={3} label="Time left" />
                {expired ? 'The code expired' : <>Expires in <span className="font-mono tabular text-fg">{formatCountdown(left)}</span></>}
              </div>
              {expired && (
                <Button size="sm" className="mt-2" onClick={() => void api.remote.pairBegin().then(setInfo, (e: unknown) => toast.error('Could not refresh', errorText(e)))}>
                  New code
                </Button>
              )}
            </div>
            <div className="min-w-0 space-y-5">
              <div>
                <div className="text-[12px] font-medium uppercase tracking-wider text-faint">PIN</div>
                <div className="mt-2 flex gap-2" aria-label={`PIN ${info.pin.split('').join(' ')}`}>
                  {info.pin.split('').map((d, i) => (
                    <motion.span key={i} initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0, transition: { delay: i * 0.05 } }} className="flex h-16 w-12 items-center justify-center rounded-xl border border-line-strong bg-surface-2 font-mono text-[30px] font-semibold text-fg">
                      {d}
                    </motion.span>
                  ))}
                </div>
              </div>
              <div>
                <div className="text-[12px] font-medium uppercase tracking-wider text-faint">Or open on your phone</div>
                <div className="mt-2 space-y-1.5">
                  {info.urls.map((u) => {
                    const base = u.split('/#')[0];
                    return (
                      <div key={u} className="flex items-center gap-2 rounded-xl border border-line bg-surface py-1 pl-3 pr-1">
                        <span className="min-w-0 flex-1 truncate font-mono text-[13px] text-fg">{base}</span>
                        <IconButton icon={Copy} label="Copy address" size="sm" onClick={() => void copyText(base).then(() => toast.success('Address copied'))} />
                      </div>
                    );
                  })}
                </div>
              </div>
              {info.fingerprint && (
                <div className="flex gap-3 rounded-xl border border-info/25 bg-info/8 px-4 py-3">
                  <ShieldCheck size={17} className="mt-0.5 shrink-0 text-info" />
                  <div className="min-w-0 text-[12.5px] leading-relaxed text-dim">
                    <span className="font-semibold text-fg">Expect a one-time certificate warning.</span> The connection is HTTPS with a certificate this PC created itself, so the phone browser asks once. Continue if the fingerprint starts with <span className="font-mono text-fg">{info.fingerprint.slice(0, 23)}</span>.
                  </div>
                </div>
              )}
              <div className="flex items-center gap-2 text-[12.5px] text-faint">
                <Spinner size={13} /> Waiting for your phone…
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </Modal>
  );
}
