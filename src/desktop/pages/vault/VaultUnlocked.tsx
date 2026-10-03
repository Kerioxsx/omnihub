import { formatDate, formatRelative } from '@shared/format';
import type { Entry, EntryKind, EntrySummary, VaultStatus } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { ClipboardCheck, Copy, ExternalLink, Eye, EyeOff, KeyRound, LockKeyhole, Pencil, Plus, Settings2, ShieldAlert, Star, Timer, Trash } from 'lucide-react';
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, Skeleton, Spinner } from '../../components/ui/Card';
import { SearchInput, Select } from '../../components/ui/Form';
import { Callout, EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useEvent, useNow } from '../../lib/hooks';
import { formatCountdown, prettyUrl } from '../../lib/util';
import { confirm } from '../../state/dialogs';
import { useLive } from '../../state/live';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';
import { EntryForm } from './EntryForm';
import { KINDS, StrengthMeter, kindIcon, useStrength } from './shared';
import { VaultSettings } from './VaultSettings';

type Field = 'password' | 'username' | 'email' | 'url' | 'notes';

const SCORE_TONE = ['bg-bad', 'bg-bad', 'bg-warn', 'bg-good', 'bg-good'];
const PROVIDER_HELP: [RegExp, string, string][] = [
  [/google|gmail/i, 'Google', 'https://myaccount.google.com/signinoptions/passkeys'],
  [/microsoft|outlook|live\.com|hotmail/i, 'Microsoft', 'https://account.live.com/proofs/manage/additional'],
  [/apple|icloud/i, 'Apple', 'https://support.apple.com/en-us/102195'],
];

function FieldRow({ label, children, onCopy, copied, mono }: { label: string; children: ReactNode; onCopy?: () => void; copied?: number | null; mono?: boolean }) {
  return (
    <div className="group flex items-center gap-3 rounded-xl border border-line bg-surface px-4 py-2.5">
      <div className="min-w-0 flex-1">
        <div className="text-[11px] font-medium uppercase tracking-wider text-faint">{label}</div>
        <div className={cx('mt-0.5 min-h-5 break-all text-[14px] text-fg', mono && 'font-mono text-[13.5px]')}>{children}</div>
      </div>
      {copied != null ? (
        <motion.span initial={{ opacity: 0, scale: 0.9 }} animate={{ opacity: 1, scale: 1 }} className="flex items-center gap-1.5 rounded-full bg-good/12 px-2.5 py-1 text-[11.5px] font-medium text-good tabular" role="status">
          <ClipboardCheck size={13} /> Clears in {copied}s
        </motion.span>
      ) : (
        onCopy && <IconButton icon={Copy} label={`Copy ${label.toLowerCase()}`} size="sm" onClick={onCopy} className="opacity-70 group-hover:opacity-100" />
      )}
    </div>
  );
}

function Detail({ id, onEdit, onDeleted, onChanged }: { id: string; onEdit: (e: Entry) => void; onDeleted: () => void; onChanged: () => void }) {
  const [entry, setEntry] = useState<Entry | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reveal, setReveal] = useState(false);
  const [revealNotes, setRevealNotes] = useState(false);
  const [copied, setCopied] = useState<{ field: Field; until: number } | null>(null);
  const clearSecs = useSettings((s) => s.settings?.vault.clipboardClearSeconds ?? 20);
  const now = useNow(500);
  const strength = useStrength(reveal && entry ? entry.password : '');

  useEffect(() => {
    let alive = true;
    setEntry(null);
    setReveal(false);
    setRevealNotes(false);
    api.vault
      .get(id)
      .then((e) => alive && setEntry(e))
      .catch((e: unknown) => alive && setError(errorText(e)));
    return () => {
      alive = false;
    };
  }, [id]);

  const copy = async (field: Field) => {
    try {
      await api.vault.copy(id, field);
      setCopied({ field, until: Date.now() + clearSecs * 1000 });
    } catch (e) {
      toast.error('Could not copy', errorText(e));
    }
  };
  const left = (f: Field) => (copied && copied.field === f && copied.until > now ? Math.ceil((copied.until - now) / 1000) : null);

  if (error) return <EmptyState icon={ShieldAlert} title="Could not open the entry" description={error} />;
  if (!entry)
    return (
      <div className="space-y-3 p-6">
        <Skeleton className="h-14 w-2/3" />
        <Skeleton className="h-16" />
        <Skeleton className="h-16" />
        <Skeleton className="h-16" />
      </div>
    );

  const Icon = kindIcon(entry.kind);
  const primary = PROVIDER_HELP.find(([re]) => re.test(`${entry.url} ${entry.email} ${entry.username}`));
  const toggleFav = async () => {
    try {
      await api.vault.save({ id: entry.id, kind: entry.kind, title: entry.title, username: entry.username, email: entry.email, password: null, url: entry.url, notes: null, tags: entry.tags, favorite: !entry.favorite });
      setEntry({ ...entry, favorite: !entry.favorite });
      onChanged();
    } catch (e) {
      toast.error('Could not update', errorText(e));
    }
  };
  const remove = async () => {
    const ok = await confirm({ title: `Delete “${entry.title}”?`, description: 'The entry is removed from the vault. This cannot be undone.', tone: 'danger', confirmLabel: 'Delete entry' });
    if (!ok) return;
    try {
      await api.vault.delete(entry.id);
      toast.success('Entry deleted');
      onDeleted();
    } catch (e) {
      toast.error('Could not delete', errorText(e));
    }
  };

  return (
    <motion.div key={entry.id} initial={{ opacity: 0, x: 8 }} animate={{ opacity: 1, x: 0 }} transition={{ duration: 0.18 }} className="flex min-h-0 flex-1 flex-col">
      <div className="flex items-start gap-4 border-b border-line px-6 py-5">
        <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-accent-soft text-accent">
          <Icon size={22} aria-hidden />
        </div>
        <div className="min-w-0 flex-1">
          <h2 className="truncate font-display text-[20px] font-semibold tracking-tight text-fg">{entry.title}</h2>
          <div className="mt-1 flex flex-wrap items-center gap-1.5">
            <Badge>{KINDS.find((k) => k.value === entry.kind)?.label}</Badge>
            {entry.tags.map((t) => (
              <Badge key={t} tone="accent">
                #{t}
              </Badge>
            ))}
          </div>
        </div>
        <IconButton icon={Star} label={entry.favorite ? 'Remove from favourites' : 'Add to favourites'} onClick={() => void toggleFav()} className={entry.favorite ? 'text-warn [&_svg]:fill-current' : undefined} />
        <Button size="sm" icon={Pencil} onClick={() => onEdit(entry)}>
          Edit
        </Button>
        <IconButton icon={Trash} label="Delete entry" variant="danger" onClick={() => void remove()} />
      </div>
      <div className="min-h-0 flex-1 space-y-3 overflow-y-auto px-6 py-5">
        {primary && (
          <Callout
            tone="warn"
            icon={ShieldAlert}
            title={`This looks like your primary ${primary[1]} account`}
            action={
              <Button size="sm" variant="secondary" iconRight={ExternalLink} onClick={() => void api.app.openUrl(primary[2])}>
                Set up a passkey
              </Button>
            }
          >
            Whoever gets this password can reset your other accounts. Use a passkey with two-factor authentication, and an app password for apps that need one.
          </Callout>
        )}
        {entry.username && (
          <FieldRow label={entry.kind === 'wifi' ? 'Network' : entry.kind === 'card' ? 'Name on card' : 'Username'} onCopy={() => void copy('username')} copied={left('username')}>
            {entry.username}
          </FieldRow>
        )}
        {entry.email && (
          <FieldRow label="Email" onCopy={() => void copy('email')} copied={left('email')}>
            {entry.email}
          </FieldRow>
        )}
        {entry.password && (
          <div className="space-y-2">
            <FieldRow label={entry.kind === 'card' ? 'PIN' : 'Password'} onCopy={() => void copy('password')} copied={left('password')} mono>
              <span className="flex items-center gap-2">
                <button type="button" onClick={() => setReveal(!reveal)} className="text-left" aria-label={reveal ? 'Hide password' : 'Reveal password'}>
                  {reveal ? entry.password : '•'.repeat(Math.min(18, Math.max(10, entry.password.length)))}
                </button>
                <button type="button" onClick={() => setReveal(!reveal)} aria-label={reveal ? 'Hide password' : 'Reveal password'} className="text-faint hover:text-fg">
                  {reveal ? <EyeOff size={14} /> : <Eye size={14} />}
                </button>
              </span>
            </FieldRow>
            {reveal && (
              <div className="px-1">
                <StrengthMeter strength={strength} compact />
              </div>
            )}
          </div>
        )}
        {entry.url && (
          <FieldRow label="Website" onCopy={() => void copy('url')} copied={left('url')}>
            <button type="button" onClick={() => void api.app.openUrl(entry.url)} className="inline-flex items-center gap-1.5 text-accent hover:underline">
              {prettyUrl(entry.url)} <ExternalLink size={12} />
            </button>
          </FieldRow>
        )}
        {entry.notes && (
          <FieldRow label={entry.kind === 'note' ? 'Secure note' : 'Notes'} onCopy={() => void copy('notes')} copied={left('notes')} mono>
            {revealNotes ? (
              <span className="whitespace-pre-wrap">{entry.notes}</span>
            ) : (
              <button type="button" onClick={() => setRevealNotes(true)} className="flex items-center gap-2 text-faint hover:text-fg">
                <Eye size={14} /> Click to reveal
              </button>
            )}
          </FieldRow>
        )}
        <div className="grid grid-cols-3 gap-3 pt-2 text-[12px]">
          <div>
            <div className="text-faint">Updated</div>
            <div className="text-dim">{formatRelative(entry.updated)}</div>
          </div>
          <div>
            <div className="text-faint">Password changed</div>
            <div className="text-dim">{formatRelative(entry.passwordChanged)}</div>
          </div>
          <div>
            <div className="text-faint">Created</div>
            <div className="text-dim">{formatDate(entry.created)}</div>
          </div>
        </div>
      </div>
    </motion.div>
  );
}

export function VaultUnlocked({ status }: { status: VaultStatus }) {
  const [entries, setEntries] = useState<EntrySummary[] | null>(null);
  const [q, setQ] = useState('');
  const [kind, setKind] = useState<'' | EntryKind>('');
  const [favs, setFavs] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [form, setForm] = useState<{ open: boolean; entry: Entry | null }>({ open: false, entry: null });
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [locksAt, setLocksAt] = useState(() => (status.locksIn != null ? Date.now() + status.locksIn * 1000 : null));
  const now = useNow(1000);
  const lastTouch = useRef(0);

  const load = useCallback(async () => {
    try {
      const list = await api.vault.list();
      setEntries(list);
      setSelected((s) => s ?? list[0]?.id ?? null);
    } catch (e) {
      toast.error('Could not list entries', errorText(e));
    }
  }, []);
  useEffect(() => {
    void load();
  }, [load]);
  useEffect(() => {
    if (status.locksIn != null) setLocksAt(Date.now() + status.locksIn * 1000);
  }, [status.locksIn]);
  useEvent('vault:unlocked', () => void load());

  // Keep the vault open while the user is active here (throttled).
  const touch = () => {
    if (Date.now() - lastTouch.current < 15000) return;
    lastTouch.current = Date.now();
    void api.vault.touch().then(() => setLocksAt(Date.now() + status.autoLockMinutes * 60000), () => undefined);
  };

  const list = useMemo(() => {
    const needle = q.trim().toLowerCase();
    return (entries ?? []).filter((e) => (!kind || e.kind === kind) && (!favs || e.favorite) && (!needle || `${e.title} ${e.username} ${e.email} ${e.url} ${e.tags.join(' ')}`.toLowerCase().includes(needle)));
  }, [entries, q, kind, favs]);

  const lock = async () => {
    await api.vault.lock().catch(() => undefined);
    await useLive.getState().refreshVault();
  };

  const secs = locksAt ? Math.max(0, (locksAt - now) / 1000) : null;
  const weak = (entries ?? []).filter((e) => e.hasPassword && e.passwordScore <= 1).length;

  return (
    <div className="flex min-h-0 flex-1 flex-col gap-3" onMouseMove={touch} onKeyDown={touch}>
      <div className="flex items-center gap-2">
        <SearchInput value={q} onChange={setQ} placeholder="Search entries" className="w-72" aria-label="Search entries" />
        <Select value={kind} onChange={(e) => setKind(e.target.value as '' | EntryKind)} aria-label="Kind" className="w-[150px]">
          <option value="">All kinds</option>
          {KINDS.map((k) => (
            <option key={k.value} value={k.value}>
              {k.label}
            </option>
          ))}
        </Select>
        <button type="button" onClick={() => setFavs(!favs)} aria-pressed={favs} className={cx('inline-flex h-9 items-center gap-1.5 rounded-[10px] border px-3 text-[13px] transition-colors', favs ? 'border-warn/40 bg-warn/12 text-warn' : 'border-line bg-surface text-dim hover:text-fg')}>
          <Star size={14} fill={favs ? 'currentColor' : 'none'} aria-hidden /> Favourites
        </button>
        {weak > 0 && (
          <Badge tone="bad" icon={ShieldAlert} title="Entries with a weak or reused-looking password">
            {weak} weak password{weak > 1 ? 's' : ''}
          </Badge>
        )}
        <div className="flex-1" />
        {secs != null && (
          <span className={cx('flex items-center gap-1.5 rounded-full border px-3 py-1 text-[12px] tabular', secs < 60 ? 'border-warn/40 bg-warn/10 text-warn' : 'border-line bg-surface text-dim')} title="The vault locks itself after inactivity">
            <Timer size={13} /> Locks in {formatCountdown(secs)}
          </span>
        )}
        <IconButton icon={Settings2} label="Vault settings" variant="secondary" onClick={() => setSettingsOpen(true)} />
        <Button icon={LockKeyhole} onClick={() => void lock()}>
          Lock
        </Button>
        <Button variant="primary" icon={Plus} onClick={() => setForm({ open: true, entry: null })}>
          New entry
        </Button>
      </div>
      <div className="grid min-h-0 flex-1 grid-cols-[340px_minmax(0,1fr)] gap-3">
        <Card className="flex min-h-0 flex-col overflow-hidden p-1.5">
          {!entries ? (
            <div className="space-y-1.5 p-1.5">
              {[0, 1, 2, 3, 4, 5].map((i) => (
                <Skeleton key={i} className="h-14" />
              ))}
            </div>
          ) : list.length === 0 ? (
            <EmptyState compact icon={KeyRound} title={entries.length ? 'No matches' : 'The vault is empty'} description={entries.length ? undefined : 'Add your first login, card or secure note.'} />
          ) : (
            <div className="min-h-0 flex-1 overflow-y-auto" role="listbox" aria-label="Vault entries">
              <AnimatePresence initial={false}>
                {list.map((e) => {
                  const I = kindIcon(e.kind);
                  const active = selected === e.id;
                  return (
                    <motion.button layout="position" key={e.id} type="button" role="option" aria-selected={active} onClick={() => setSelected(e.id)} className={cx('relative mb-0.5 flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition-colors', active ? 'bg-surface-3' : 'hover:bg-surface-2')}>
                      <span className={cx('flex h-9 w-9 shrink-0 items-center justify-center rounded-xl', active ? 'accent-gradient text-white' : 'bg-surface-3 text-dim')}>
                        <I size={16} aria-hidden />
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="flex items-center gap-1.5">
                          <span className="truncate text-[13.5px] font-medium text-fg">{e.title}</span>
                          {e.favorite && <Star size={11} className="shrink-0 fill-current text-warn" aria-label="Favourite" />}
                        </span>
                        <span className="block truncate text-[12px] text-faint">{e.username || e.email || prettyUrl(e.url) || (e.hasNotes ? 'Secure note' : '—')}</span>
                      </span>
                      {e.primaryAccount && <span className="h-2 w-2 shrink-0 rounded-full bg-warn" title="Primary account — consider a passkey" />}
                      {e.hasPassword && <span className={cx('h-1.5 w-5 shrink-0 rounded-full', SCORE_TONE[e.passwordScore])} title={`Password strength ${e.passwordScore}/4`} />}
                    </motion.button>
                  );
                })}
              </AnimatePresence>
            </div>
          )}
          {entries && <div className="border-t border-line px-3 py-2 text-[11.5px] text-faint">{entries.length} entries · encrypted on this PC</div>}
        </Card>
        <Card className="flex min-h-0 flex-col overflow-hidden">
          {selected ? (
            <Detail
              key={selected}
              id={selected}
              onEdit={(entry) => setForm({ open: true, entry })}
              onDeleted={() => {
                setSelected(null);
                void load();
              }}
              onChanged={() => void load()}
            />
          ) : entries ? (
            <EmptyState icon={KeyRound} title="Select an entry" />
          ) : (
            <div className="flex flex-1 items-center justify-center">
              <Spinner />
            </div>
          )}
        </Card>
      </div>
      <EntryForm
        open={form.open}
        entry={form.entry}
        onClose={() => setForm({ open: false, entry: null })}
        onSaved={(id) => {
          void load();
          setSelected(null);
          setTimeout(() => setSelected(id), 0);
        }}
      />
      <VaultSettings open={settingsOpen} onClose={() => setSettingsOpen(false)} status={status} />
    </div>
  );
}
