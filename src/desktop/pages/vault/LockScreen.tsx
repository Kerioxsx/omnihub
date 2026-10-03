import type { VaultStatus } from '@shared/types';
import { motion, useAnimationControls } from 'motion/react';
import { Eye, EyeOff, FingerprintPattern, LockKeyhole, LockOpen } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { TextInput } from '../../components/ui/Form';
import { useNow } from '../../lib/hooks';
import { useLive } from '../../state/live';

export function LockScreen({ status }: { status: VaultStatus }) {
  const [pw, setPw] = useState('');
  const [show, setShow] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [retryUntil, setRetryUntil] = useState(() => (status.retryAfter ? Date.now() + status.retryAfter * 1000 : 0));
  const now = useNow(250);
  const shake = useAnimationControls();
  const wait = Math.max(0, Math.ceil((retryUntil - now) / 1000));

  useEffect(() => {
    if (status.retryAfter) setRetryUntil(Date.now() + status.retryAfter * 1000);
  }, [status.retryAfter]);

  const unlock = async (via: 'password' | 'hello') => {
    setBusy(true);
    setError(null);
    try {
      if (via === 'hello') await api.vault.helloUnlock();
      else await api.vault.unlock(pw);
      setPw('');
      await useLive.getState().refreshVault();
    } catch (e) {
      setError(errorText(e));
      void shake.start({ x: [0, -10, 10, -6, 6, 0], transition: { duration: 0.4 } });
      const s = await api.vault.status().catch(() => null);
      if (s?.retryAfter) setRetryUntil(Date.now() + s.retryAfter * 1000);
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-full items-center justify-center pb-10">
      <motion.div animate={shake} className="w-full max-w-[440px]">
        <Card className="relative overflow-hidden px-8 pb-8 pt-10 text-center">
          <div className="pointer-events-none absolute inset-x-0 -top-28 mx-auto h-64 w-64 rounded-full bg-[radial-gradient(circle,var(--accent-glow),transparent_68%)] opacity-60" />
          <motion.div initial={{ scale: 0.8, opacity: 0 }} animate={{ scale: 1, opacity: 1 }} transition={{ type: 'spring', stiffness: 260, damping: 18 }} className="relative mx-auto flex h-20 w-20 items-center justify-center rounded-[26px] border border-line-strong bg-surface-2 shadow-[0_20px_50px_-20px_var(--accent-glow)]">
            {busy ? <LockOpen size={34} className="text-accent" aria-hidden /> : <LockKeyhole size={34} className="text-accent" aria-hidden />}
          </motion.div>
          <h2 className="relative mt-5 font-display text-[22px] font-semibold tracking-tight text-fg">Vault is locked</h2>
          <p className="relative mt-1 text-[13.5px] text-dim">
            {status.entries} {status.entries === 1 ? 'entry' : 'entries'} · locks after {status.autoLockMinutes} min of inactivity
          </p>
          <form
            className="relative mt-6 space-y-3 text-left"
            onSubmit={(e) => {
              e.preventDefault();
              if (pw && !wait) void unlock('password');
            }}
          >
            <TextInput
              type={show ? 'text' : 'password'}
              value={pw}
              onChange={(e) => setPw(e.target.value)}
              placeholder="Master password"
              aria-label="Master password"
              autoComplete="current-password"
              inputSize="lg"
              autoFocus
              disabled={wait > 0}
              aria-invalid={!!error}
              right={
                <button type="button" onClick={() => setShow(!show)} aria-label={show ? 'Hide password' : 'Show password'} className="flex h-8 w-8 items-center justify-center rounded-lg text-faint hover:bg-surface-3 hover:text-fg">
                  {show ? <EyeOff size={15} /> : <Eye size={15} />}
                </button>
              }
            />
            {(error || wait > 0) && (
              <div role="alert" className="text-[12.5px] text-bad">
                {wait > 0 ? `Too many attempts — try again in ${wait} s.` : error}
              </div>
            )}
            <Button type="submit" variant="primary" size="lg" className="w-full" loading={busy} disabled={!pw || wait > 0} icon={LockOpen}>
              Unlock
            </Button>
            {status.helloEnabled && (
              <Button size="lg" className="w-full" icon={FingerprintPattern} onClick={() => void unlock('hello')} disabled={busy}>
                Unlock with Windows Hello
              </Button>
            )}
          </form>
          <p className="relative mt-5 text-[12px] text-faint">Encrypted with Argon2id + AES-256-GCM, bound to your Windows account.</p>
        </Card>
      </motion.div>
    </div>
  );
}
