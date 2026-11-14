// Forgot the master password. Nothing can decrypt the vault without it or
// Windows Hello (that is the point of the encryption), so this offers what
// really works: Windows Hello sets a new password with nothing lost, or the
// vault is set aside (kept, not deleted) so a new one can be made, and put
// back if the password comes back to you.

import type { SetAside, VaultStatus } from '@shared/types';
import { ArchiveRestore, ChevronRight, FingerprintPattern, KeyRound, LifeBuoy, PackagePlus } from 'lucide-react';
import { useEffect, useState } from 'react';
import type { LucideIcon } from 'lucide-react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Checkbox, Field, TextInput } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { Callout } from '../../components/ui/States';
import { useLive } from '../../state/live';
import { toast } from '../../state/toasts';
import { StrengthMeter, useStrength } from './shared';

const MIN_LENGTH = 10;

function Option({ icon: Icon, title, text, onClick }: { icon: LucideIcon; title: string; text: string; onClick: () => void }) {
  return (
    <button type="button" onClick={onClick} className="group flex w-full items-center gap-3 rounded-xl border border-line bg-surface-1 px-4 py-3 text-left transition-colors hover:border-line-strong hover:bg-surface-2 focus-visible:outline-2 focus-visible:outline-accent">
      <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-accent-soft text-accent">
        <Icon size={19} aria-hidden />
      </span>
      <span className="min-w-0 flex-1">
        <span className="block text-[14px] font-semibold text-fg">{title}</span>
        <span className="mt-0.5 block text-[12.5px] leading-snug text-dim">{text}</span>
      </span>
      <ChevronRight size={16} className="shrink-0 text-faint transition-transform group-hover:translate-x-0.5" aria-hidden />
    </button>
  );
}

type Step = 'menu' | 'hello' | 'start-over';

export function ForgotPasswordDialog({ open, onClose, onDone, status }: { open: boolean; onClose: () => void; onDone?: () => void; status: VaultStatus }) {
  const [step, setStep] = useState<Step>('menu');
  const [next, setNext] = useState('');
  const [repeat, setRepeat] = useState('');
  const [understood, setUnderstood] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const strength = useStrength(next);

  useEffect(() => {
    if (!open) {
      setStep('menu');
      setNext('');
      setRepeat('');
      setUnderstood(false);
      setError(null);
    }
  }, [open]);

  // Open but Hello not on yet: it can be turned on now (no password needed while open), then used.
  const enableFirst = status.unlocked && !status.helloEnabled && status.helloAvailable;
  const canHello = status.helloEnabled || enableFirst;
  const tooShort = next.length > 0 && next.length < MIN_LENGTH;
  const mismatch = repeat.length > 0 && repeat !== next;
  const ready = next.length >= MIN_LENGTH && repeat === next && (strength?.score ?? 0) >= 2;

  const resetWithHello = async () => {
    if (!ready) return;
    setBusy(true);
    setError(null);
    try {
      if (enableFirst) await api.vault.helloEnable();
      await api.vault.resetPasswordWithHello(next);
      await useLive.getState().refreshVault();
      toast.success('New master password set', 'Everything in the vault is still there. Use the new password from now on.');
      onClose();
      onDone?.();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const startOver = async () => {
    setBusy(true);
    setError(null);
    try {
      await api.vault.setAside();
      await useLive.getState().refreshVault();
      toast.success('Old vault set aside', 'Create your new vault now. If the old password comes back to you, put the old vault back from the Vault page.');
      onClose();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const footer =
    step === 'hello' ? (
      <>
        <Button variant="ghost" onClick={() => setStep('menu')}>
          Back
        </Button>
        <Button variant="primary" icon={FingerprintPattern} loading={busy} disabled={!ready} onClick={() => void resetWithHello()}>
          Confirm with Windows Hello
        </Button>
      </>
    ) : step === 'start-over' ? (
      <>
        <Button variant="ghost" onClick={() => setStep('menu')}>
          Back
        </Button>
        <Button variant="danger" icon={PackagePlus} loading={busy} disabled={!understood} onClick={() => void startOver()}>
          Set aside and start over
        </Button>
      </>
    ) : (
      <Button variant="ghost" onClick={onClose}>
        Close
      </Button>
    );

  return (
    <Modal open={open} onClose={onClose} icon={LifeBuoy} title="Forgot your master password?" description="No one can read the vault without its password, including OmniHub. Here's what still works." footer={footer}>
      {step === 'menu' && (
        <div className="space-y-3">
          <Callout tone="info" title="Try these first">
            Caps Lock, the keyboard language (Windows key + Space switches it), and older passwords or small spelling changes you often use.
          </Callout>
          {canHello && (
            <Option
              icon={FingerprintPattern}
              title={enableFirst ? 'Turn on Windows Hello, then set a new password' : 'Use Windows Hello'}
              text={
                enableFirst
                  ? 'The vault is open, so Windows Hello can be turned on now. Then confirm with your face, fingerprint or PIN and choose a new password. Nothing is lost.'
                  : 'Confirm it’s you with your face, fingerprint or PIN, then choose a new master password. Logins, hidden files and locked notes stay as they are.'
              }
              onClick={() => setStep('hello')}
            />
          )}
          {!canHello && status.unlocked && (
            <Callout tone="warn">Windows Hello isn’t set up on this PC, so a new password needs the current one. While the vault is open you can still see and copy every login, and you can set up a Windows PIN (Settings → Accounts → Sign-in options) to use Windows Hello here.</Callout>
          )}
          {!status.unlocked && !status.helloEnabled && (
            <p className="text-[12.5px] leading-relaxed text-dim">Windows Hello wasn’t turned on for this vault, so it can’t open it this time. (Once you’re back in, turn it on in Vault settings so this can’t lock you out again.)</p>
          )}
          {!status.unlocked && (
            <Option icon={PackagePlus} title="Start a new vault" text="The old vault is set aside on this PC, not deleted. If you remember its password later, you can put it back with everything in it." onClick={() => setStep('start-over')} />
          )}
        </div>
      )}

      {step === 'hello' && (
        <form
          className="space-y-3"
          onSubmit={(e) => {
            e.preventDefault();
            void resetWithHello();
          }}
        >
          <Field label="New master password" htmlFor="fp-new" hint={tooShort ? `At least ${MIN_LENGTH} characters.` : undefined}>
            <TextInput id="fp-new" type="password" value={next} onChange={(e) => setNext(e.target.value)} autoComplete="new-password" autoFocus aria-invalid={tooShort} />
          </Field>
          {next && <StrengthMeter strength={strength} compact />}
          <Field label="Type it again" htmlFor="fp-repeat" hint={mismatch ? "The two passwords don't match." : undefined}>
            <TextInput id="fp-repeat" type="password" value={repeat} onChange={(e) => setRepeat(e.target.value)} autoComplete="new-password" aria-invalid={mismatch} />
          </Field>
          {error && <Callout tone="bad">{error}</Callout>}
          <p className="text-[12px] text-faint">{enableFirst ? 'Windows asks twice: once to turn Windows Hello on, once to confirm the new password.' : 'Windows will ask for your face, fingerprint or PIN.'} If you approve on your phone, it asks there too.</p>
          <button type="submit" hidden />
        </form>
      )}

      {step === 'start-over' && (
        <div className="space-y-3">
          <Callout tone="warn" title="The new vault starts empty">
            Your saved logins, hidden files and locked notes stay locked with the old password. The old vault is kept in OmniHub’s folder, so nothing is deleted: Vault → <b>Put back</b> returns it, and everything opens again with its password.
          </Callout>
          <Checkbox checked={understood} onChange={setUnderstood} label="I understand the new vault starts empty" />
          {error && <Callout tone="bad">{error}</Callout>}
        </div>
      )}
    </Modal>
  );
}

const formatDate = (unix: number) => new Date(unix * 1000).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });

/** Vaults set aside earlier, with Put back. `current`: a vault exists now (it gets set aside in exchange). */
export function SetAsideList({ current, className }: { current: boolean; className?: string }) {
  const [list, setList] = useState<SetAside[]>([]);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    api.vault
      .setAsideList()
      .then((l) => live && setList(l))
      .catch(() => live && setList([]));
    return () => {
      live = false;
    };
  }, [current]);

  if (list.length === 0) return null;

  const putBack = async (id: string) => {
    setBusy(id);
    try {
      const swapped = await api.vault.putBack(id);
      await useLive.getState().refreshVault();
      setList(await api.vault.setAsideList());
      toast.success('Old vault put back', swapped ? 'Unlock it with its password. The vault that was here is set aside, so you can switch back.' : 'Unlock it with its password.');
    } catch (e) {
      toast.error('Couldn’t put it back', errorText(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <Callout tone="accent" icon={ArchiveRestore} title={list.length === 1 ? 'A vault you set aside' : `${list.length} vaults you set aside`} className={className}>
      <p>Remember its password? Put it back and unlock it as before.{current ? ' The vault here now is set aside in exchange, so nothing is lost.' : ''}</p>
      <ul className="mt-2 space-y-1.5">
        {list.map((a) => (
          <li key={a.id} className="flex items-center justify-between gap-3">
            <span className="flex items-center gap-2 text-[12.5px] text-dim">
              <KeyRound size={13} aria-hidden />
              Set aside {formatDate(a.setAsideAt)}
              {a.hello && ' · has Windows Hello'}
            </span>
            <Button size="sm" loading={busy === a.id} disabled={busy !== null} onClick={() => void putBack(a.id)}>
              Put back
            </Button>
          </li>
        ))}
      </ul>
    </Callout>
  );
}
