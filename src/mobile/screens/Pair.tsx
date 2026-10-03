// Pairing: QR link (#pair=SECRET) or the 6-digit PIN shown on the PC.

import { useEffect, useRef, useState } from 'react';
import { motion } from 'motion/react';
import { ShieldCheck, Smartphone, Hash, Info, X, MonitorSmartphone, LoaderCircle } from 'lucide-react';
import { ApiError, client, pairSecretFromUrl, savedDeviceName, type ServerInfo } from '../client';
import { Logo } from '../ui/Logo';
import { Button } from '../ui/common';
import { cx, vibrate, useInterval } from '../lib/util';

const TLS_NOTE_KEY = 'omnihub.tlsNoteSeen';

function pairError(e: unknown, viaLink: boolean): string {
  if (e instanceof ApiError) {
    if (e.status === 429) return 'Too many attempts. Wait a minute, then try again.';
    if (e.status === 403 && /not open/i.test(e.message)) return "Pairing isn't open — on the PC click “Pair a phone”, then try again.";
    if (e.status === 403 && /wrong/i.test(e.message) && viaLink) return 'This pairing link has expired or was already used. Enter the 6-digit code from the PC instead, or scan a fresh QR code.';
    if (e.status === 403 && /wrong/i.test(e.message)) return "That code didn't match. Check the 6 digits shown on the PC.";
    return e.message;
  }
  return "Can't reach the PC. Make sure your phone is on the same Wi-Fi.";
}

function clearHash() {
  try {
    history.replaceState(history.state, '', location.pathname + location.search);
  } catch {
    location.hash = '';
  }
}

export function PairScreen({ info, onPaired }: { info: ServerInfo | null; onPaired: (serverName: string) => void }) {
  const [secret, setSecret] = useState<string | null>(() => pairSecretFromUrl());
  const [name, setName] = useState(() => savedDeviceName());
  const [pin, setPin] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [shake, setShake] = useState(0);
  const [focused, setFocused] = useState(false);
  const [live, setLive] = useState<ServerInfo | null>(info);
  const [tlsSeen, setTlsSeen] = useState(() => {
    try {
      return localStorage.getItem(TLS_NOTE_KEY) === '1';
    } catch {
      return false;
    }
  });
  const pinRef = useRef<HTMLInputElement>(null);
  const pcName = live?.name || 'your PC';

  // Keep "pairing open" fresh while waiting on this screen.
  useInterval(() => {
    client.info().then(setLive, () => undefined);
  }, 4000);
  useEffect(() => setLive(info), [info]);

  const submit = async (code?: string) => {
    if (busy) return;
    const deviceName = name.trim() || 'Phone';
    setBusy(true);
    setError(null);
    try {
      const r = await client.pair(secret ? { secret, deviceName } : { pin: code ?? pin, deviceName });
      clearHash();
      vibrate([12, 40, 18]);
      onPaired(r.serverName);
    } catch (e) {
      setError(pairError(e, !!secret));
      vibrate([30, 60, 30]);
      setShake((s) => s + 1);
      if (!secret) {
        setPin('');
        setTimeout(() => pinRef.current?.focus(), 50);
      }
      // A used or expired QR secret will never work again; fall back to the PIN.
      if (secret && e instanceof ApiError && e.status === 403) {
        clearHash();
        setSecret(null);
      }
    } finally {
      setBusy(false);
    }
  };

  const onPin = (v: string) => {
    const digits = v.replace(/\D/g, '').slice(0, 6);
    setPin(digits);
    setError(null);
    if (digits.length === 6) {
      pinRef.current?.blur();
      submit(digits);
    }
  };

  const dismissTls = () => {
    setTlsSeen(true);
    try {
      localStorage.setItem(TLS_NOTE_KEY, '1');
    } catch {
      /* ignore */
    }
  };

  return (
    <div className="scroller h-full">
      <div className="px-safe mx-auto flex min-h-full max-w-[460px] flex-col pb-[calc(var(--safe-bottom)+24px)] pt-[calc(var(--safe-top)+28px)]">
        <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.45, ease: [0.2, 0.8, 0.2, 1] }} className="flex flex-col items-center text-center">
          <div className="relative">
            <div className="absolute inset-[-18px] rounded-[40px] bg-[radial-gradient(closest-side,rgba(124,58,237,.45),transparent)] blur-xl" />
            <Logo size={84} className="relative drop-shadow-[0_12px_30px_rgba(91,92,240,.45)]" />
          </div>
          <div className="mt-6 text-[13px] font-semibold uppercase tracking-[0.2em] text-faint">OmniHub</div>
          <h1 className="mt-1.5 font-display text-[28px] font-bold leading-tight tracking-tight">
            {secret ? (
              <>
                Pair with <span className="grad-text">{pcName}</span>
              </>
            ) : (
              'Connect to your PC'
            )}
          </h1>
          <p className="mt-2 max-w-[330px] text-[15px] text-dim">
            {secret ? 'This phone will be able to browse files, send uploads and control the PC you allow.' : `Enter the 6-digit code shown in OmniHub on ${pcName}.`}
          </p>
        </motion.div>

        {live && !live.pairingOpen && (
          <div className="fade-up mt-6 flex items-start gap-3 rounded-2xl border border-warn/30 bg-warn/10 px-4 py-3 text-sm">
            <MonitorSmartphone size={18} className="mt-0.5 shrink-0 text-warn" />
            <div>
              <div className="font-semibold text-fg">Pairing isn't open yet</div>
              <div className="text-dim">On the PC, open OmniHub → Phone and click “Pair a phone”.</div>
            </div>
          </div>
        )}

        <motion.div key={shake} animate={shake ? { x: [0, -10, 10, -6, 6, 0] } : {}} transition={{ duration: 0.4 }} className="card mt-6 p-5">
          {!secret && (
            <div className="mb-5">
              <label className="mb-2.5 flex items-center gap-2 text-[13px] font-semibold text-dim">
                <Hash size={15} /> Pairing code
              </label>
              <div className="relative" onClick={() => pinRef.current?.focus()}>
                <input
                  ref={pinRef}
                  value={pin}
                  onChange={(e) => onPin(e.target.value)}
                  onFocus={() => setFocused(true)}
                  onBlur={() => setFocused(false)}
                  inputMode="numeric"
                  autoComplete="one-time-code"
                  pattern="[0-9]*"
                  maxLength={6}
                  aria-label="6-digit pairing code"
                  className="absolute inset-0 z-10 h-full w-full cursor-text opacity-0"
                  autoFocus={!secret}
                  disabled={busy}
                />
                <div className="flex items-center gap-2">
                  {Array.from({ length: 6 }, (_, i) => {
                    const ch = pin[i];
                    const isCaret = focused && i === Math.min(pin.length, 5) && !busy;
                    return (
                      <div
                        key={i}
                        className={cx(
                          'num grid aspect-[4/5] min-w-0 flex-1 place-items-center rounded-2xl border text-[26px] font-bold transition-all',
                          ch ? 'border-accent/60 bg-accent-soft text-fg' : 'border-line bg-surface',
                          isCaret && 'border-accent shadow-[0_0_0_3px_var(--accent-soft)]',
                          i === 3 && 'ml-2',
                        )}
                      >
                        {ch ?? (isCaret ? <span className="h-7 w-0.5 animate-pulse rounded bg-accent" /> : '')}
                      </div>
                    );
                  })}
                </div>
              </div>
            </div>
          )}

          <label className="mb-2 flex items-center gap-2 text-[13px] font-semibold text-dim" htmlFor="devname">
            <Smartphone size={15} /> This phone's name
          </label>
          <input id="devname" className="field" value={name} maxLength={60} onChange={(e) => setName(e.target.value)} placeholder="e.g. Pixel 9" autoComplete="off" enterKeyHint="go" />
          <div className="mt-1.5 px-1 text-xs text-faint">Shown on the PC in the list of paired phones.</div>

          {error && (
            <div role="alert" className="fade-up mt-4 rounded-xl bg-bad/12 px-3.5 py-2.5 text-sm font-medium text-bad">
              {error}
            </div>
          )}

          {secret ? (
            <Button size="lg" className="mt-5 w-full" loading={busy} onClick={() => submit()} icon={<ShieldCheck size={20} />}>
              Pair this phone
            </Button>
          ) : (
            <Button size="lg" className="mt-5 w-full" loading={busy} disabled={pin.length !== 6} onClick={() => submit()}>
              {busy ? 'Pairing…' : 'Pair'}
            </Button>
          )}
          {secret && (
            <button className="mx-auto mt-3 block text-sm font-semibold text-dim underline-offset-2 active:underline" onClick={() => setSecret(null)}>
              Use the 6-digit code instead
            </button>
          )}
        </motion.div>

        {live?.tls && !tlsSeen && (
          <div className="fade-up card relative mt-4 p-4 pr-11 text-sm">
            <button aria-label="Dismiss" onClick={dismissTls} className="absolute right-2.5 top-2.5 grid h-8 w-8 place-items-center rounded-full text-faint active:bg-surface-2">
              <X size={16} />
            </button>
            <div className="flex items-center gap-2 font-semibold">
              <Info size={16} className="text-accent" /> About the certificate warning
            </div>
            <p className="mt-1.5 text-dim">
              Your browser warned you because the PC uses its <b className="text-fg">own self-signed certificate</b> — there's no public website involved. The connection is still encrypted. You can compare the fingerprint shown in OmniHub on the PC.
            </p>
          </div>
        )}

        <div className="mt-auto flex items-center justify-center gap-2 pt-8 text-xs text-faint">
          {busy ? <LoaderCircle size={13} className="spin" /> : <ShieldCheck size={13} />}
          {live?.tls ? 'Encrypted connection on your local network' : 'Local network only · pairing needs the PC'}
        </div>
      </div>
    </div>
  );
}
