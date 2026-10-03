// Home: PC status, pending power action, inbox, idea composer, shortcuts.

import { useCallback, useEffect, useState } from 'react';
import { Cpu, MemoryStick, HardDrive, Download, X, Inbox, Sparkles, Send, FolderOpen, MonitorPlay, Power, Clock, Lightbulb, ChevronRight, NotebookPen } from 'lucide-react';
import { basename, formatBytes, formatRelative } from '@shared/format';
import { client, type Status } from '../client';
import { useApp, useInbox, usePower, toast, type Tab } from '../state';
import { PullToRefresh } from '../ui/PullToRefresh';
import { Logo } from '../ui/Logo';
import { Button, ProgressBar, Ring, SectionTitle, Skeleton, Switch } from '../ui/common';
import { PendingPowerCard } from './Power';
import { cx, errorMessage, formatUptime, greeting, useInterval, usePageVisible } from '../lib/util';
import { FileIcon } from './fileKinds';

export function HomeScreen({ active, openMore }: { active: boolean; openMore: (page: 'notes') => void }) {
  const { info, socket, online, setTab } = useApp();
  const visible = usePageVisible();
  const [status, setStatus] = useState<Status | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const setPending = usePower((s) => s.setPending);
  const inbox = useInbox();

  const loadStatus = useCallback(async () => {
    try {
      const s = await client.status();
      setStatus(s);
      setStatusError(null);
      setPending(s.pendingPower);
    } catch (e) {
      setStatusError(errorMessage(e));
    }
  }, [setPending]);

  useEffect(() => {
    if (active && visible) loadStatus();
  }, [active, visible, loadStatus]);
  useInterval(loadStatus, active && visible ? 5000 : null);
  useEffect(() => {
    useInbox.getState().load();
  }, []);

  const refresh = async () => {
    await Promise.all([loadStatus(), inbox.load()]);
  };

  const f = info?.features;
  const connected = socket === 'open' && online;
  const shortcuts: { tab: Tab; label: string; hint: string; icon: React.ReactNode; tone: string }[] = [
    { tab: 'files', label: 'Files', hint: 'Browse & send', icon: <FolderOpen size={22} />, tone: 'from-violet-500/25 to-violet-500/5 text-violet-300' },
    ...(f?.screen ? [{ tab: 'screen' as Tab, label: 'Screen', hint: f.control ? 'View & control' : 'View live', icon: <MonitorPlay size={22} />, tone: 'from-cyan-500/25 to-cyan-500/5 text-cyan-300' }] : []),
    ...(f?.power ? [{ tab: 'power' as Tab, label: 'Power', hint: 'Lock, sleep…', icon: <Power size={22} />, tone: 'from-rose-500/25 to-rose-500/5 text-rose-300' }] : []),
  ];

  return (
    <PullToRefresh onRefresh={refresh}>
      <div className="px-safe pb-tabbar pt-[calc(var(--safe-top)+14px)]">
        <header className="flex items-center gap-3 pb-1">
          <Logo size={42} className="drop-shadow-[0_6px_18px_rgba(91,92,240,.35)]" />
          <div className="min-w-0 flex-1">
            <div className="text-[13px] font-medium text-dim">{greeting()}</div>
            <h1 className="truncate font-display text-[26px] font-bold leading-tight tracking-tight">{info?.name || status?.hostname || 'Your PC'}</h1>
          </div>
          <div className={cx('flex items-center gap-1.5 rounded-full px-3 py-1.5 text-xs font-semibold', connected ? 'bg-good/12 text-good' : 'bg-warn/12 text-warn')}>
            <span className={cx('h-2 w-2 rounded-full', connected ? 'dot-live bg-good' : 'bg-warn')} />
            {connected ? 'Online' : online ? 'Connecting' : 'Offline'}
          </div>
        </header>

        <PendingPowerCard className="mt-4" />

        {/* PC status */}
        <section className="card mt-4 overflow-hidden p-4">
          {status ? (
            <>
              <div className="flex items-center gap-2 text-[13px] text-dim">
                <span className="truncate">
                  {status.hostname && status.hostname !== info?.name ? `${status.hostname} · ` : ''}
                  {status.os || info?.platform}
                </span>
                <span className="text-faint">·</span>
                <span className="flex shrink-0 items-center gap-1">
                  <Clock size={13} /> up {formatUptime(status.uptime)}
                </span>
              </div>
              <div className="mt-4 grid grid-cols-2 gap-3">
                <Gauge icon={<Cpu size={14} />} label="CPU" value={status.cpu / 100} text={`${Math.round(status.cpu)}%`} sub="processor load" />
                <Gauge
                  icon={<MemoryStick size={14} />}
                  label="Memory"
                  value={status.memory.total ? status.memory.used / status.memory.total : 0}
                  text={`${Math.round((status.memory.used / Math.max(1, status.memory.total)) * 100)}%`}
                  sub={`${formatBytes(status.memory.used)} of ${formatBytes(status.memory.total, 0)}`}
                />
              </div>
              {status.disks.length > 0 && (
                <div className="mt-4 space-y-3 border-t border-line pt-4">
                  {status.disks.slice(0, 4).map((d) => {
                    const used = d.total ? (d.total - d.free) / d.total : 0;
                    return (
                      <div key={d.root}>
                        <div className="mb-1.5 flex items-center gap-2 text-sm">
                          <HardDrive size={15} className="text-faint" />
                          <span className="min-w-0 flex-1 truncate font-semibold">{d.label ? `${d.label} (${d.root.replace(/\\$/, '')})` : d.root}</span>
                          <span className="num shrink-0 text-[13px] text-dim">{formatBytes(d.free)} free</span>
                        </div>
                        <ProgressBar value={used} tone={used > 0.92 ? 'bad' : used > 0.8 ? 'warn' : 'accent'} />
                      </div>
                    );
                  })}
                </div>
              )}
            </>
          ) : statusError ? (
            <div className="py-3 text-center text-sm text-dim">
              {statusError}
              <div className="mt-3">
                <Button size="sm" variant="ghost" onClick={loadStatus}>
                  Retry
                </Button>
              </div>
            </div>
          ) : (
            <div>
              <Skeleton className="h-3.5 w-1/2" />
              <div className="mt-4 grid grid-cols-2 gap-3">
                <Skeleton className="h-[108px] rounded-2xl" />
                <Skeleton className="h-[108px] rounded-2xl" />
              </div>
              <Skeleton className="mt-4 h-3 w-full" />
            </div>
          )}
        </section>

        {/* Shortcuts */}
        <div className={cx('mt-3 grid gap-3', shortcuts.length === 3 ? 'grid-cols-3' : shortcuts.length === 2 ? 'grid-cols-2' : 'grid-cols-1')}>
          {shortcuts.map((s) => (
            <button key={s.tab} onClick={() => setTab(s.tab)} className="press card flex flex-col items-start gap-3 p-3.5 text-left">
              <div className={cx('grid h-11 w-11 place-items-center rounded-2xl bg-gradient-to-br light:text-accent', s.tone)}>{s.icon}</div>
              <div>
                <div className="text-[15px] font-semibold">{s.label}</div>
                <div className="text-xs text-dim">{s.hint}</div>
              </div>
            </button>
          ))}
        </div>

        {/* Inbox */}
        <SectionTitle right={inbox.items.length > 0 && <span className="rounded-full bg-accent-soft px-2 py-0.5 text-xs font-bold text-accent">{inbox.items.length}</span>}>From your PC</SectionTitle>
        <section className="card overflow-hidden">
          {!inbox.loaded ? (
            <div className="space-y-3 p-4">
              <Skeleton className="h-10 w-full" />
            </div>
          ) : inbox.items.length === 0 ? (
            <div className="flex items-center gap-3.5 px-4 py-4">
              <div className="grid h-11 w-11 shrink-0 place-items-center rounded-2xl bg-surface-2 text-faint">
                <Inbox size={20} />
              </div>
              <div className="text-sm text-dim">
                Nothing waiting. Use <b className="text-fg">Send to phone</b> on the PC and files appear here.
              </div>
            </div>
          ) : (
            <ul className="divide-y divide-[var(--border)]">
              {inbox.items.map((it) => (
                <InboxRow key={it.id} id={it.id} name={it.name} size={it.size} created={it.created} />
              ))}
            </ul>
          )}
        </section>

        {f?.notes && <IdeaComposer onOpenNotes={() => openMore('notes')} />}

        <div className="mt-8 text-center text-xs text-faint">
          OmniHub {info?.version} · {info?.tls ? 'HTTPS' : 'HTTP'} · local network
        </div>
      </div>
    </PullToRefresh>
  );
}

function Gauge({ icon, label, value, text, sub }: { icon: React.ReactNode; label: string; value: number; text: string; sub: string }) {
  const color = value > 0.9 ? 'var(--bad)' : value > 0.75 ? 'var(--warn)' : undefined;
  return (
    <div className="rounded-2xl bg-surface p-3">
      <div className="flex items-center gap-1.5 text-[13px] font-semibold text-dim">
        {icon} {label}
      </div>
      <div className="mt-2 flex items-center gap-3">
        <Ring value={value} size={60} stroke={6.5} color={color}>
          <span className="num text-[15px] font-bold">{text}</span>
        </Ring>
        <div className="min-w-0 text-xs leading-snug text-faint">{sub}</div>
      </div>
    </div>
  );
}

function InboxRow({ id, name, size, created }: { id: string; name: string; size: number; created: number }) {
  const inbox = useInbox();
  const [busy, setBusy] = useState(false);
  const item = inbox.items.find((i) => i.id === id);
  const download = async () => {
    if (!item) return;
    setBusy(true);
    try {
      await client.receive(item);
      toast.success('Downloading', name);
    } catch (e) {
      toast.error("Couldn't download", errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  const dismiss = async () => {
    inbox.remove(id);
    try {
      await client.dismiss(id);
    } catch {
      /* already gone */
    }
  };
  return (
    <li className="flex items-center gap-3 px-4 py-3">
      <FileIcon name={name} />
      <div className="min-w-0 flex-1">
        <div className="truncate text-[15px] font-semibold">{name}</div>
        <div className="text-[13px] text-dim">
          {formatBytes(size)} · {formatRelative(created)}
        </div>
      </div>
      <button aria-label={`Download ${name}`} onClick={download} disabled={busy} className="press grid h-10 w-10 place-items-center rounded-full bg-accent text-white disabled:opacity-60">
        <Download size={18} />
      </button>
      <button aria-label={`Dismiss ${name}`} onClick={dismiss} className="press grid h-10 w-10 place-items-center rounded-full bg-surface-2 text-dim">
        <X size={18} />
      </button>
    </li>
  );
}

function IdeaComposer({ onOpenNotes }: { onOpenNotes: () => void }) {
  const [title, setTitle] = useState('');
  const [body, setBody] = useState('');
  const [toClaude, setToClaude] = useState(true);
  const [claudeFolder, setClaudeFolder] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    client.notes('idea', '').then(
      (r) => setClaudeFolder(r.claudeFolder),
      () => setClaudeFolder(null),
    );
  }, []);

  const send = async () => {
    if (!title.trim() && !body.trim()) return;
    setBusy(true);
    try {
      const n = await client.createNote({ kind: 'idea', title: title.trim() || body.trim().split('\n')[0].slice(0, 80), body, sendToClaude: toClaude && claudeFolder !== false });
      setTitle('');
      setBody('');
      if (n.exportedPath) toast.success('Idea sent to Claude', `Saved as ${basename(n.exportedPath)}`, { label: 'Notes', run: onOpenNotes });
      else toast.success('Idea saved on the PC', claudeFolder === false ? 'Set a Claude folder in OmniHub on the PC to export ideas.' : undefined, { label: 'Notes', run: onOpenNotes });
    } catch (e) {
      toast.error("Couldn't save the idea", errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <SectionTitle
        right={
          <button onClick={onOpenNotes} className="flex items-center gap-0.5 text-[13px] font-semibold text-accent">
            All notes <ChevronRight size={15} />
          </button>
        }
      >
        Idea for Claude
      </SectionTitle>
      <section className="card relative overflow-hidden p-4">
        <div className="pointer-events-none absolute -right-10 -top-10 h-32 w-32 rounded-full bg-[radial-gradient(closest-side,rgba(124,58,237,.28),transparent)]" />
        <div className="relative flex items-center gap-2.5">
          <div className="grid h-9 w-9 place-items-center rounded-xl grad-bg text-white">
            <Lightbulb size={18} />
          </div>
          <div className="text-sm text-dim">Capture it now — it lands in OmniHub notes on the PC.</div>
        </div>
        <input className="field relative mt-3.5 font-semibold" placeholder="Title" value={title} onChange={(e) => setTitle(e.target.value)} maxLength={200} enterKeyHint="next" />
        <textarea
          className="field relative mt-2.5 min-h-[96px] resize-none leading-relaxed"
          placeholder="Describe the idea… Markdown works."
          value={body}
          onChange={(e) => {
            setBody(e.target.value);
            e.target.style.height = 'auto';
            e.target.style.height = `${Math.min(320, e.target.scrollHeight + 2)}px`;
          }}
        />
        <div className="relative mt-3 flex items-center gap-3">
          <Switch on={toClaude && claudeFolder !== false} onChange={setToClaude} label="Send to Claude folder" disabled={claudeFolder === false} />
          <div className="min-w-0 flex-1 text-sm">
            <div className="flex items-center gap-1.5 font-semibold">
              <Sparkles size={14} className="text-accent" /> Send to Claude folder
            </div>
            <div className="text-xs text-faint">{claudeFolder === false ? 'No Claude folder set on the PC' : 'Exported as Markdown for Claude Code'}</div>
          </div>
        </div>
        <Button className="relative mt-4 w-full" loading={busy} disabled={!title.trim() && !body.trim()} onClick={send} icon={toClaude && claudeFolder !== false ? <Send size={18} /> : <NotebookPen size={18} />}>
          {toClaude && claudeFolder !== false ? 'Send idea' : 'Save idea'}
        </Button>
      </section>
    </>
  );
}
