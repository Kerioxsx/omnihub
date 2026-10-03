import { motion } from 'motion/react';
import { Cpu, Eye, EyeOff, HardDrive, KeyRound, LockKeyhole, ShieldAlert, ShieldCheck } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { Field, TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { useLive } from '../../state/live';
import { toast } from '../../state/toasts';
import { StrengthMeter, useStrength } from './shared';

const LAYERS = [
  { icon: Cpu, title: 'Argon2id key derivation', text: 'Your master password is stretched with a memory-hard function, so guessing it offline is slow and expensive.' },
  { icon: LockKeyhole, title: 'AES-256-GCM encryption', text: 'Every entry is encrypted and authenticated; any tampering is detected.' },
  { icon: ShieldCheck, title: 'Windows DPAPI', text: 'The vault key is additionally bound to your Windows account on this PC.' },
  { icon: HardDrive, title: 'Local only', text: 'Nothing is uploaded. Phones can only read entries if you allow it, over HTTPS, with the master password.' },
];

export function CreateVault() {
  const [pw, setPw] = useState('');
  const [confirmPw, setConfirmPw] = useState('');
  const [show, setShow] = useState(false);
  const [busy, setBusy] = useState(false);
  const strength = useStrength(pw);
  const mismatch = confirmPw.length > 0 && pw !== confirmPw;
  const ok = pw.length >= 8 && pw === confirmPw && (strength?.score ?? 0) >= 2;

  const create = async () => {
    setBusy(true);
    try {
      await api.vault.create(pw);
      setPw('');
      setConfirmPw('');
      await useLive.getState().refreshVault();
      toast.success('Vault created', 'It locks automatically when you are away.');
    } catch (e) {
      toast.error('Could not create the vault', errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} className="mx-auto grid max-w-[1040px] grid-cols-[minmax(0,1fr)_minmax(0,1.05fr)] gap-4 pb-6">
      <Card className="relative overflow-hidden p-7">
        <div className="pointer-events-none absolute -left-20 -top-24 h-64 w-64 rounded-full bg-[radial-gradient(circle,var(--accent-soft),transparent_70%)]" />
        <div className="relative">
          <div className="accent-gradient flex h-12 w-12 items-center justify-center rounded-2xl text-white shadow-[0_10px_30px_-10px_var(--accent-glow)]">
            <KeyRound size={22} aria-hidden />
          </div>
          <h2 className="mt-4 font-display text-[22px] font-semibold tracking-tight text-fg">Create your vault</h2>
          <p className="mt-1 text-[13.5px] leading-relaxed text-dim">Choose a master password you can remember. It is never stored — without it, the vault cannot be opened, not even by OmniHub.</p>
          <form
            className="mt-6 space-y-4"
            onSubmit={(e) => {
              e.preventDefault();
              if (ok) void create();
            }}
          >
            <Field label="Master password" htmlFor="vault-new">
              <TextInput
                id="vault-new"
                type={show ? 'text' : 'password'}
                value={pw}
                onChange={(e) => setPw(e.target.value)}
                autoComplete="new-password"
                inputSize="lg"
                placeholder="A long passphrase works best"
                autoFocus
                right={
                  <button type="button" onClick={() => setShow(!show)} aria-label={show ? 'Hide password' : 'Show password'} className="flex h-8 w-8 items-center justify-center rounded-lg text-faint hover:bg-surface-3 hover:text-fg">
                    {show ? <EyeOff size={15} /> : <Eye size={15} />}
                  </button>
                }
              />
            </Field>
            <StrengthMeter strength={strength} />
            <Field label="Confirm" htmlFor="vault-confirm" hint={mismatch ? <span className="text-bad">The passwords don't match.</span> : undefined}>
              <TextInput id="vault-confirm" type={show ? 'text' : 'password'} value={confirmPw} onChange={(e) => setConfirmPw(e.target.value)} autoComplete="new-password" inputSize="lg" aria-invalid={mismatch} />
            </Field>
            <Button type="submit" variant="primary" size="lg" className="w-full" disabled={!ok} loading={busy} icon={LockKeyhole}>
              Create vault
            </Button>
            <p className="text-center text-[12px] text-faint">Tip: four random words beat a short complex password.</p>
          </form>
        </div>
      </Card>
      <div className="space-y-4">
        <Card className="p-6">
          <h3 className="font-display text-[15px] font-semibold text-fg">How your secrets are protected</h3>
          <div className="mt-4 space-y-4">
            {LAYERS.map((l, i) => (
              <motion.div key={l.title} initial={{ opacity: 0, x: 8 }} animate={{ opacity: 1, x: 0, transition: { delay: 0.08 + i * 0.06 } }} className="flex gap-3">
                <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-accent-soft text-accent">
                  <l.icon size={17} aria-hidden />
                </div>
                <div>
                  <div className="text-[13.5px] font-semibold text-fg">{l.title}</div>
                  <div className="text-[12.5px] leading-relaxed text-dim">{l.text}</div>
                </div>
              </motion.div>
            ))}
          </div>
        </Card>
        <Callout tone="warn" icon={ShieldAlert} title="Don't keep your primary Google, Microsoft or Apple passwords here">
          Those accounts can reset every other password you own. Protect them with a <strong className="text-fg">passkey</strong> and <strong className="text-fg">two-factor authentication</strong> instead, and use <strong className="text-fg">app passwords</strong> for apps that need them. OmniHub flags entries that look like primary accounts.
        </Callout>
        <Callout tone="bad" icon={KeyRound} title="There is no password reset">
          If you forget the master password, the vault cannot be recovered. Export an encrypted backup from the vault settings once you've added entries.
        </Callout>
      </div>
    </motion.div>
  );
}
