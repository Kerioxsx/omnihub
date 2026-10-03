import type { Entry, EntryInput, EntryKind } from '@shared/types';
import { Eye, EyeOff, Save, Star } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Field, TextInput, Textarea } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { cx } from '../../lib/cx';
import { toast } from '../../state/toasts';
import { GeneratorButton, KINDS, StrengthMeter, useStrength } from './shared';

interface Form {
  kind: EntryKind;
  title: string;
  username: string;
  email: string;
  password: string;
  url: string;
  notes: string;
  tags: string;
  favorite: boolean;
}

const EMPTY: Form = { kind: 'login', title: '', username: '', email: '', password: '', url: '', notes: '', tags: '', favorite: false };

const LABELS: Partial<Record<EntryKind, { username?: string; password?: string; url?: string }>> = {
  card: { username: 'Name on card', password: 'PIN' },
  wifi: { username: 'Network name (SSID)', password: 'Wi-Fi password' },
};

export function EntryForm({ open, entry, onClose, onSaved }: { open: boolean; entry: Entry | null; onClose: () => void; onSaved: (id: string) => void }) {
  const [f, setF] = useState<Form>(EMPTY);
  const [show, setShow] = useState(false);
  const [busy, setBusy] = useState(false);
  const strength = useStrength(f.password);

  useEffect(() => {
    if (!open) return;
    setShow(false);
    setF(entry ? { kind: entry.kind, title: entry.title, username: entry.username, email: entry.email, password: entry.password, url: entry.url, notes: entry.notes, tags: entry.tags.join(', '), favorite: entry.favorite } : EMPTY);
  }, [open, entry]);

  const set = (p: Partial<Form>) => setF((x) => ({ ...x, ...p }));
  const labels = LABELS[f.kind] ?? {};
  const showLogin = f.kind !== 'note';

  const save = async () => {
    setBusy(true);
    try {
      const input: EntryInput = {
        id: entry?.id ?? null,
        kind: f.kind,
        title: f.title.trim(),
        username: f.username,
        email: f.email,
        password: entry && f.password === entry.password ? null : f.password,
        url: f.url.trim(),
        notes: f.notes,
        tags: f.tags
          .split(',')
          .map((t) => t.trim())
          .filter(Boolean),
        favorite: f.favorite,
      };
      const s = await api.vault.save(input);
      toast.success(entry ? 'Entry updated' : 'Entry added', s.primaryAccount ? 'This looks like a primary account — consider a passkey instead.' : undefined);
      onSaved(s.id);
      onClose();
    } catch (e) {
      toast.error('Could not save', errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={entry ? `Edit “${entry.title}”` : 'New entry'}
      size="lg"
      footer={
        <>
          <button type="button" onClick={() => set({ favorite: !f.favorite })} aria-pressed={f.favorite} className={cx('mr-auto inline-flex items-center gap-1.5 text-[13px]', f.favorite ? 'text-warn' : 'text-dim hover:text-fg')}>
            <Star size={15} fill={f.favorite ? 'currentColor' : 'none'} /> Favourite
          </button>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" icon={Save} loading={busy} disabled={!f.title.trim()} onClick={() => void save()}>
            {entry ? 'Save changes' : 'Add entry'}
          </Button>
        </>
      }
    >
      <form
        className="space-y-4 pb-2"
        onSubmit={(e) => {
          e.preventDefault();
          if (f.title.trim()) void save();
        }}
      >
        <div className="grid grid-cols-6 gap-1.5" role="radiogroup" aria-label="Kind">
          {KINDS.map((k) => (
            <button
              key={k.value}
              type="button"
              role="radio"
              aria-checked={f.kind === k.value}
              onClick={() => set({ kind: k.value })}
              className={cx('flex flex-col items-center gap-1 rounded-xl border px-2 py-2.5 text-[11.5px] transition-colors', f.kind === k.value ? 'border-accent/50 bg-accent-soft text-accent' : 'border-line bg-surface text-dim hover:text-fg')}
            >
              <k.icon size={17} aria-hidden />
              {k.label}
            </button>
          ))}
        </div>
        <Field label="Title" htmlFor="e-title">
          <TextInput id="e-title" value={f.title} onChange={(e) => set({ title: e.target.value })} placeholder="e.g. GitHub" data-autofocus />
        </Field>
        {showLogin && (
          <div className="grid grid-cols-2 gap-3">
            <Field label={labels.username ?? 'Username'} htmlFor="e-user">
              <TextInput id="e-user" value={f.username} onChange={(e) => set({ username: e.target.value })} autoComplete="off" />
            </Field>
            {f.kind !== 'wifi' && f.kind !== 'card' && (
              <Field label="Email" htmlFor="e-email">
                <TextInput id="e-email" type="email" value={f.email} onChange={(e) => set({ email: e.target.value })} autoComplete="off" />
              </Field>
            )}
          </div>
        )}
        {showLogin && (
          <div>
            <div className="mb-1.5 flex items-center justify-between">
              <label htmlFor="e-pw" className="text-[12.5px] font-medium text-dim">
                {labels.password ?? 'Password'}
              </label>
              <GeneratorButton
                onUse={(pw) => {
                  set({ password: pw });
                  setShow(true);
                }}
              />
            </div>
            <TextInput
              id="e-pw"
              type={show ? 'text' : 'password'}
              value={f.password}
              onChange={(e) => set({ password: e.target.value })}
              autoComplete="new-password"
              className="[&_input]:font-mono"
              right={
                <button type="button" onClick={() => setShow(!show)} aria-label={show ? 'Hide password' : 'Show password'} className="flex h-7 w-7 items-center justify-center rounded-lg text-faint hover:bg-surface-3 hover:text-fg">
                  {show ? <EyeOff size={14} /> : <Eye size={14} />}
                </button>
              }
            />
            {f.password && (
              <div className="mt-2">
                <StrengthMeter strength={strength} compact />
              </div>
            )}
          </div>
        )}
        {f.kind !== 'note' && f.kind !== 'card' && f.kind !== 'wifi' && (
          <Field label="Website" htmlFor="e-url">
            <TextInput id="e-url" value={f.url} onChange={(e) => set({ url: e.target.value })} placeholder="https://" />
          </Field>
        )}
        <Field label={f.kind === 'note' ? 'Secure note' : 'Notes'} htmlFor="e-notes" hint="Encrypted like the password.">
          <Textarea id="e-notes" value={f.notes} onChange={(e) => set({ notes: e.target.value })} rows={f.kind === 'note' ? 7 : 3} className="font-mono text-[13px]" />
        </Field>
        <Field label="Tags" htmlFor="e-tags" hint="Comma separated">
          <TextInput id="e-tags" value={f.tags} onChange={(e) => set({ tags: e.target.value })} placeholder="work, finance" />
        </Field>
      </form>
    </Modal>
  );
}
