// Change the vault's master password: the current one, the new one twice.
// Only the key that unlocks the vault is re-wrapped, so entries, hidden
// files, locked notes and Windows Hello all keep working.

import { KeyRound } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Field, TextInput } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { Callout } from '../../components/ui/States';
import { toast } from '../../state/toasts';
import { StrengthMeter, useStrength } from './shared';

/** The vault's own minimum (MIN_MASTER_LEN in the core). */
const MIN_LENGTH = 10;

export function ChangePasswordDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const [current, setCurrent] = useState('');
  const [next, setNext] = useState('');
  const [repeat, setRepeat] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const strength = useStrength(next);

  useEffect(() => {
    if (!open) {
      setCurrent('');
      setNext('');
      setRepeat('');
      setError(null);
    }
  }, [open]);

  const tooShort = next.length > 0 && next.length < MIN_LENGTH;
  const mismatch = repeat.length > 0 && repeat !== next;
  const same = next.length > 0 && next === current;
  const ready = current.length > 0 && next.length >= MIN_LENGTH && repeat === next && !same && (strength?.score ?? 0) >= 2;

  const submit = async () => {
    if (!ready) return;
    setBusy(true);
    setError(null);
    try {
      await api.vault.changePassword(current, next);
      toast.success('Master password changed', 'Use the new one next time you unlock the vault.');
      onClose();
    } catch (e) {
      const msg = errorText(e);
      setError(/wrong (master )?password|password is wrong/i.test(msg) ? "That isn't your current master password." : msg);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      icon={KeyRound}
      title="Change master password"
      description="Your logins, hidden files and locked notes stay as they are, and Windows Hello keeps working."
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" loading={busy} disabled={!ready} onClick={() => void submit()}>
            Change password
          </Button>
        </>
      }
    >
      <form
        className="space-y-3"
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <Field label="Current master password" htmlFor="cp-current">
          <TextInput id="cp-current" type="password" value={current} onChange={(e) => setCurrent(e.target.value)} autoComplete="current-password" autoFocus />
        </Field>
        <Field label="New master password" htmlFor="cp-new" hint={tooShort ? `At least ${MIN_LENGTH} characters.` : same ? "That's the same as the current one." : undefined}>
          <TextInput id="cp-new" type="password" value={next} onChange={(e) => setNext(e.target.value)} autoComplete="new-password" aria-invalid={tooShort || same} />
        </Field>
        {next && <StrengthMeter strength={strength} compact />}
        <Field label="Type the new password again" htmlFor="cp-repeat" hint={mismatch ? "The two new passwords don't match." : undefined}>
          <TextInput id="cp-repeat" type="password" value={repeat} onChange={(e) => setRepeat(e.target.value)} autoComplete="new-password" aria-invalid={mismatch} />
        </Field>
        {error && <Callout tone="bad">{error}</Callout>}
        <p className="text-[12px] text-faint">No one can recover a forgotten master password, including OmniHub. Write the new one down somewhere safe until you know it by heart.</p>
        <button type="submit" hidden />
      </form>
    </Modal>
  );
}
