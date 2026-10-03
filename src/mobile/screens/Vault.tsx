// Vault: read-only password lookup over the secure connection. The session
// token lives in memory only (never in storage) and ends after the PC's
// idle timeout, when you lock it, or when the page stays hidden.

import { useEffect, useMemo, useRef, useState } from 'react';
import { create } from 'zustand';
import { KeyRound, Lock, Search, X, Eye, EyeOff, Copy, Star, Globe, AtSign, User, ShieldCheck, LockOpen, FileText, CreditCard, Wifi, Mail, KeySquare } from 'lucide-react';
import type { EntrySummary } from '@shared/types';
import { ApiError, client } from '../client';
import { toast } from '../state';
import { Sheet } from '../ui/Sheet';
import { Button, Empty, ErrorState, IconButton, ListSkeleton, Ring } from '../ui/common';
import { SubHeader } from './More';
import { canCopy, copyText } from '../lib/clipboard';
import { cx, errorMessage, useNow, usePageVisible, vibrate } from '../lib/util';

interface VaultSession {
  session: string | null;
  idleSeconds: number;
  lastUsed: number;
  lockedReason: string | null;
  set: (s: string | null, idleSeconds?: number, reason?: string | null) => void;
  touch: () => void;
}

// Memory only: a reload or closing the tab forgets the session.
const useVault = create<VaultSession>((set) => ({
  session: null,
  idleSeconds: 300,
  lastUsed: 0,
  lockedReason: null,
  set: (session, idleSeconds = 300, reason = null) => set({ session, idleSeconds, lastUsed: Date.now(), lockedReason: reason }),
  touch: () => set({ lastUsed: Date.now() }),
}));

const REVEAL_MS = 15000;
const HIDDEN_LOCK_MS = 60000;

const KIND_ICON = { login: KeySquare, email: Mail, note: FileText, card: CreditCard, wifi: Wifi, other: KeyRound } as const;

export function VaultPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const v = useVault();
  const visible = usePageVisible();
  const hiddenAt = useRef<number | null>(null);

  const lock = async (reason: string | null = null) => {
    const s = useVault.getState().session;
    useVault.getState().set(null, 300, reason);
    if (s) client.vaultLock(s).catch(() => undefined);
  };

  // Lock after the PC's idle timeout, or after a minute in the background.
  const now = useNow(5000, !!v.session);
  useEffect(() => {
    if (v.session && now - v.lastUsed > v.idleSeconds * 1000) lock('Locked after inactivity.');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [now]);
  useEffect(() => {
    if (!visible) hiddenAt.current = Date.now();
    else if (hiddenAt.current && Date.now() - hiddenAt.current > HIDDEN_LOCK_MS && useVault.getState().session) lock('Locked while the app was in the background.');
    if (visible) hiddenAt.current = null;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [visible]);

  return (
    <div className="h-full">
      <div className="scroller h-full">
        <SubHeader
          title="Vault"
          subtitle={v.session ? 'Unlocked · read-only' : 'Locked'}
          onBack={onBack}
          right={
            v.session && (
              <Button size="sm" variant="ghost" icon={<Lock size={15} />} onClick={() => lock(null)}>
                Lock
              </Button>
            )
          }
        />
        <div className="px-safe pb-tabbar">{v.session ? <Entries session={v.session} active={active} onExpired={() => lock('The vault locked on the PC. Unlock it again.')} /> : <Unlock reason={v.lockedReason} />}</div>
      </div>
    </div>
  );
}

function Unlock({ reason }: { reason: string | null }) {
  const [pw, setPw] = useState('');
  const [show, setShow] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!pw) return;
    setBusy(true);
    setError(null);
    try {
      const r = await client.vaultUnlock(pw);
      setPw('');
      vibrate(12);
      useVault.getState().set(r.session, r.idleSeconds);
    } catch (err) {
      setError(err instanceof ApiError && err.status === 403 && /password|decrypt|wrong/i.test(err.message) ? 'Wrong master password.' : errorMessage(err));
      vibrate([30, 60, 30]);
    } finally {
      setBusy(false);
    }
  };
  return (
    <div className="fade-up">
      <div className="flex flex-col items-center pt-4 text-center">
        <div className="grid h-20 w-20 place-items-center rounded-[26px] bg-[linear-gradient(135deg,rgba(251,191,36,.25),rgba(251,146,60,.12))] text-amber-400">
          <KeyRound size={36} />
        </div>
        <div className="mt-4 font-display text-xl font-bold">Unlock your vault</div>
        <div className="mt-1 max-w-[300px] text-sm text-dim">Enter the master password you use in OmniHub on the PC.</div>
      </div>
      {reason && <div className="mt-5 rounded-xl bg-warn/12 px-3.5 py-2.5 text-center text-sm font-medium text-warn">{reason}</div>}
      <form onSubmit={submit} className="card mt-5 p-4">
        <div className="relative">
          <input
            className="field pr-12"
            type={show ? 'text' : 'password'}
            value={pw}
            onChange={(e) => setPw(e.target.value)}
            placeholder="Master password"
            autoComplete="current-password"
            autoCapitalize="off"
            autoCorrect="off"
            spellCheck={false}
            enterKeyHint="go"
            aria-label="Master password"
          />
          <button type="button" aria-label={show ? 'Hide password' : 'Show password'} onClick={() => setShow((s) => !s)} className="absolute right-1.5 top-1/2 grid h-10 w-10 -translate-y-1/2 place-items-center rounded-xl text-faint">
            {show ? <EyeOff size={18} /> : <Eye size={18} />}
          </button>
        </div>
        {error && <div className="mt-3 rounded-xl bg-bad/12 px-3.5 py-2.5 text-sm font-medium text-bad">{error}</div>}
        <Button type="submit" className="mt-4 w-full" loading={busy} disabled={!pw} icon={<LockOpen size={18} />}>
          Unlock
        </Button>
      </form>
      <div className="mt-5 space-y-2.5 rounded-2xl border border-line bg-surface p-4 text-[13px] leading-relaxed text-dim">
        <div className="flex items-center gap-2 font-semibold text-fg">
          <ShieldCheck size={16} className="text-good" /> How this stays safe
        </div>
        <p>Your vault stays encrypted on the PC. This phone only gets a short-lived, read-only session that is kept in memory — never saved — and locks after 5 minutes of inactivity.</p>
        <p>Only unlock on your own phone and your own Wi-Fi. Passwords are hidden until you tap Reveal and hide again after 15 seconds. Lock the vault when you're done.</p>
      </div>
    </div>
  );
}

function Entries({ session, active, onExpired }: { session: string; active: boolean; onExpired: () => void }) {
  const [entries, setEntries] = useState<EntrySummary[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [q, setQ] = useState('');
  const [open, setOpen] = useState<EntrySummary | null>(null);

  const load = async () => {
    try {
      const r = await client.vaultEntries(session);
      useVault.getState().touch();
      setEntries(r.entries);
      setError(null);
    } catch (e) {
      if (e instanceof ApiError && e.status === 423) return onExpired();
      setError(errorMessage(e));
    }
  };
  useEffect(() => {
    if (active) load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [session, active]);

  const shown = useMemo(() => {
    const s = q.trim().toLowerCase();
    return (entries ?? [])
      .filter((e) => !s || [e.title, e.username, e.email, e.url, ...e.tags].some((x) => x?.toLowerCase().includes(s)))
      .sort((a, b) => Number(b.favorite) - Number(a.favorite) || a.title.localeCompare(b.title));
  }, [entries, q]);

  return (
    <>
      <div className="relative">
        <Search size={17} className="pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-faint" />
        <input className="field h-11 py-0 pl-10 pr-10" type="search" placeholder="Search vault" value={q} onChange={(e) => setQ(e.target.value)} autoCapitalize="off" />
        {q && (
          <button aria-label="Clear search" onClick={() => setQ('')} className="absolute right-2 top-1/2 grid h-8 w-8 -translate-y-1/2 place-items-center rounded-full text-faint">
            <X size={16} />
          </button>
        )}
      </div>
      <div className="mt-3">
        {error && !entries ? (
          <ErrorState message={error} onRetry={load} />
        ) : !entries ? (
          <ListSkeleton rows={6} />
        ) : shown.length === 0 ? (
          <Empty icon={<KeyRound size={28} />} title={q ? 'No matches' : 'Vault is empty'} body={q ? `Nothing matches “${q}”.` : 'Add entries in OmniHub on the PC.'} />
        ) : (
          <div className="card overflow-hidden">
            {shown.map((e, i) => {
              const Icon = KIND_ICON[e.kind] ?? KeyRound;
              return (
                <button key={e.id} onClick={() => setOpen(e)} className={cx('press flex w-full items-center gap-3 px-4 py-3 text-left active:bg-surface-2', i > 0 && 'border-t border-line')}>
                  <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-amber-500/12 text-amber-400">
                    <Icon size={19} />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-1.5">
                      <span className="truncate text-[15px] font-semibold">{e.title}</span>
                      {e.favorite && <Star size={13} className="shrink-0 fill-amber-400 text-amber-400" />}
                    </div>
                    <div className="truncate text-[13px] text-dim">{e.username || e.email || e.url || '—'}</div>
                  </div>
                </button>
              );
            })}
          </div>
        )}
      </div>
      <EntrySheet session={session} entry={open} onClose={() => setOpen(null)} onExpired={onExpired} />
    </>
  );
}

function EntrySheet({ session, entry, onClose, onExpired }: { session: string; entry: EntrySummary | null; onClose: () => void; onExpired: () => void }) {
  const [last, setLast] = useState(entry);
  useEffect(() => {
    if (entry) setLast(entry);
  }, [entry]);
  const e = entry ?? last;
  const [revealed, setRevealed] = useState<{ value: string; until: number } | null>(null);
  const [notes, setNotes] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const now = useNow(250, !!revealed);
  const visible = usePageVisible();

  useEffect(() => {
    setRevealed(null);
    setNotes(null);
  }, [entry?.id, session]);
  useEffect(() => {
    if (revealed && now > revealed.until) setRevealed(null);
  }, [now, revealed]);
  useEffect(() => {
    if (!visible) {
      setRevealed(null);
      setNotes(null);
    }
  }, [visible]);
  useEffect(() => {
    if (!entry) {
      setRevealed(null);
      setNotes(null);
    }
  }, [entry]);

  const fetchField = async (field: 'password' | 'notes' | 'username' | 'email') => {
    if (!e) return null;
    try {
      const r = await client.vaultReveal(session, e.id, field);
      useVault.getState().touch();
      return r.value;
    } catch (err) {
      if (err instanceof ApiError && err.status === 423) {
        onClose();
        onExpired();
      } else toast.error("Couldn't read the vault", errorMessage(err));
      return null;
    }
  };

  const reveal = async () => {
    if (revealed) return setRevealed(null);
    setBusy(true);
    const v = await fetchField('password');
    setBusy(false);
    if (v != null) setRevealed({ value: v, until: Date.now() + REVEAL_MS });
  };
  const copy = async (text: string | null | undefined, what: string) => {
    if (text == null) return;
    const ok = await copyText(text);
    if (ok) {
      vibrate(10);
      toast.success(`${what} copied`, what === 'Password' ? 'Clear your clipboard when you are done.' : undefined);
    } else toast.error("Couldn't copy", 'Copying is blocked here — tap Reveal and copy it by hand.');
  };
  const copyPassword = async () => {
    // Use the revealed value if we have it (keeps the tap's user activation for the copy).
    if (revealed) return copy(revealed.value, 'Password');
    const v = await fetchField('password');
    await copy(v, 'Password');
  };

  return (
    <Sheet open={!!entry} onClose={onClose} title={e?.title} subtitle={e?.url || undefined}>
      {e && (
        <div className="space-y-2.5 pb-3">
          {e.username && <Field icon={<User size={17} />} label="Username" value={e.username} onCopy={canCopy() ? () => copy(e.username, 'Username') : undefined} />}
          {e.email && <Field icon={<AtSign size={17} />} label="Email" value={e.email} onCopy={canCopy() ? () => copy(e.email, 'Email') : undefined} />}
          {e.url && (
            <Field
              icon={<Globe size={17} />}
              label="Website"
              value={
                <a href={/^https?:\/\//i.test(e.url) ? e.url : `https://${e.url}`} target="_blank" rel="noopener noreferrer" className="text-accent underline-offset-2">
                  {e.url}
                </a>
              }
            />
          )}
          {e.hasPassword && (
            <div className="rounded-2xl border border-line bg-surface p-3.5">
              <div className="flex items-center gap-2 text-[12px] font-semibold uppercase tracking-wide text-faint">
                <KeyRound size={14} /> Password
              </div>
              <div className="mt-2 flex items-center gap-2">
                <div className={cx('selectable min-w-0 flex-1 break-all font-mono text-[17px]', revealed ? 'text-fg' : 'tracking-[0.25em] text-dim')}>{revealed ? revealed.value : '••••••••••••'}</div>
                {revealed && (
                  <Ring value={Math.max(0, revealed.until - now) / REVEAL_MS} size={30} stroke={3}>
                    <span className="num text-[10px] font-bold text-dim">{Math.ceil(Math.max(0, revealed.until - now) / 1000)}</span>
                  </Ring>
                )}
              </div>
              <div className="mt-3 flex gap-2">
                <Button size="sm" variant={revealed ? 'ghost' : 'soft'} className="flex-1" loading={busy} onClick={reveal} icon={revealed ? <EyeOff size={16} /> : <Eye size={16} />}>
                  {revealed ? 'Hide' : 'Reveal'}
                </Button>
                {canCopy() && (
                  <Button size="sm" variant="ghost" className="flex-1" onClick={copyPassword} icon={<Copy size={16} />}>
                    Copy
                  </Button>
                )}
              </div>
            </div>
          )}
          {e.hasNotes && (
            <div className="rounded-2xl border border-line bg-surface p-3.5">
              <div className="flex items-center gap-2 text-[12px] font-semibold uppercase tracking-wide text-faint">
                <FileText size={14} /> Notes
              </div>
              {notes != null ? (
                <div className="selectable mt-2 whitespace-pre-wrap text-sm">{notes}</div>
              ) : (
                <Button size="sm" variant="ghost" className="mt-2.5" onClick={async () => setNotes(await fetchField('notes'))}>
                  Show notes
                </Button>
              )}
            </div>
          )}
          {!e.hasPassword && !e.username && !e.email && !e.hasNotes && <div className="py-6 text-center text-sm text-dim">Nothing else stored for this entry.</div>}
        </div>
      )}
    </Sheet>
  );
}

function Field({ icon, label, value, onCopy }: { icon: React.ReactNode; label: string; value: React.ReactNode; onCopy?: () => void }) {
  return (
    <div className="flex items-center gap-3 rounded-2xl border border-line bg-surface px-3.5 py-3">
      <div className="text-faint">{icon}</div>
      <div className="min-w-0 flex-1">
        <div className="text-[12px] font-semibold uppercase tracking-wide text-faint">{label}</div>
        <div className="selectable truncate text-[15px] font-medium">{value}</div>
      </div>
      {onCopy && (
        <IconButton label={`Copy ${label.toLowerCase()}`} onClick={onCopy} className="h-9 w-9">
          <Copy size={15} />
        </IconButton>
      )}
    </div>
  );
}
