// Home: PC status, pending power action, inbox, idea composer, shortcuts.

import { useCallback, useEffect, useState } from 'react';
import { Cpu, MemoryStick, HardDrive, Download, X, Inbox, Sparkles, Send, FolderOpen, MonitorPlay, Power, Clock, Lightbulb, ChevronRight, NotebookPen, Copy, ExternalLink, Link2, Type, Share, ClipboardPaste, Music2, Pause, Play, SkipForward, SlidersVertical, AppWindow } from 'lucide-react';
import { basename, formatBytes, formatRelative } from '@shared/format';
import type { InboxItem } from '@shared/types';
import { client, type MediaState, type Status } from '../client';
import { useEvent } from '../lib/events';
import { useApp, useInbox, usePower, toast, type Tab } from '../state';
import { PullToRefresh } from '../ui/PullToRefresh';
import { Logo } from '../ui/Logo';
import { Button, ProgressBar, Ring, SectionTitle, Skeleton, Switch } from '../ui/common';
import { PendingPowerCard } from './Power';
import { CallBar } from './Sound';
import type { MorePage } from './More';
import { copyToClipboard, cx, errorMessage, formatUptime, greeting, linkOf, useInterval, usePageVisible } from '../lib/util';
import { FileIcon } from './fileKinds';

export function HomeScreen({ active, openMore }: { active: boolean; openMore: (page: MorePage) => void }) {
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
        <CallBar active={active} className="mt-4" />

        {(f?.media || f?.tasks) && (
          <div className="mt-4 grid grid-cols-2 gap-3">
            {f?.media && (
              <button onClick={() => openMore('sound')} className="card press flex items-center gap-3 p-3.5 text-left">
                <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-accent-soft text-accent">
                  <SlidersVertical size={20} />
                </div>
                <div className="min-w-0">
                  <div className="truncate text-[14.5px] font-semibold">Volume</div>
                  <div className="truncate text-[12px] text-dim">Apps, mic &amp; calls</div>
                </div>
              </button>
            )}
            {f?.tasks && (
              <button onClick={() => openMore('open')} className="card press flex items-center gap-3 p-3.5 text-left">
                <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-rose-500/15 text-rose-400">
                  <AppWindow size={20} />
                </div>
                <div className="min-w-0">
                  <div className="truncate text-[14.5px] font-semibold">Open apps</div>
                  <div className="truncate text-[12px] text-dim">Close any app</div>
                </div>
              </button>
            )}
          </div>
        )}

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
              {f?.tasks && (
                <button onClick={() => openMore('tasks')} className="press mt-3 flex w-full items-center justify-center gap-1 rounded-xl bg-surface-2 py-2.5 text-[13.5px] font-semibold text-accent">
                  What's using it — every program <ChevronRight size={15} />
                </button>
              )}
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

        {f?.media && <NowPlayingCard onOpen={() => setTab('music')} />}

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
                <InboxRow key={it.id} item={it} />
              ))}
            </ul>
          )}
        </section>

        {f?.clipboard && <ClipboardComposer />}

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

const SHARE_LIMIT = 200 * 1024 * 1024;

function InboxRow({ item }: { item: InboxItem }) {
  const inbox = useInbox();
  const [busy, setBusy] = useState(false);
  const [shareFile, setShareFile] = useState<File | null>(null);
  const canShareFiles = typeof navigator !== 'undefined' && !!navigator.canShare && item.kind === 'file' && item.size <= SHARE_LIMIT;
  const { id, name, size, created } = item;

  const download = async () => {
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
  // Two taps: fetch first, then open the share sheet while the tap still counts as a user gesture.
  const prepareShare = async () => {
    setBusy(true);
    try {
      const f = await client.fetchForShare(item);
      if (!navigator.canShare?.({ files: [f] })) {
        toast.info("This phone can't share that file type", 'Use Download instead.');
        return;
      }
      setShareFile(f);
    } catch (e) {
      toast.error("Couldn't get the file", errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  const share = async () => {
    if (!shareFile) return;
    try {
      await navigator.share({ files: [shareFile], title: name });
    } catch (e) {
      if ((e as DOMException)?.name !== 'AbortError') toast.error("Couldn't open the share sheet", errorMessage(e));
    }
  };
  const copy = async () => {
    const ok = await copyToClipboard(item.text ?? '');
    if (ok) toast.success('Copied');
    else toast.error("Couldn't copy");
  };
  const dismiss = async () => {
    inbox.remove(id);
    try {
      await client.dismiss(id);
    } catch {
      /* already gone */
    }
  };

  if (item.kind === 'text') {
    const link = linkOf(item.text ?? '');
    return (
      <li className="px-4 py-3">
        <div className="flex items-start gap-3">
          <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-accent-soft text-accent">{link ? <Link2 size={19} /> : <Type size={19} />}</div>
          <div className="min-w-0 flex-1">
            <div className="line-clamp-3 whitespace-pre-wrap break-words text-[15px]">{item.text}</div>
            <div className="mt-0.5 text-[13px] text-dim">From your PC · {formatRelative(created)}</div>
          </div>
          <button aria-label="Dismiss" onClick={dismiss} className="press grid h-9 w-9 shrink-0 place-items-center rounded-full bg-surface-2 text-dim">
            <X size={17} />
          </button>
        </div>
        <div className="mt-2.5 flex gap-2 pl-[52px]">
          <Button size="sm" onClick={copy} icon={<Copy size={16} />}>
            Copy
          </Button>
          {link && (
            <a href={link} target="_blank" rel="noopener noreferrer" className="press inline-flex h-9 items-center gap-1.5 rounded-full bg-surface-2 px-3.5 text-[14px] font-semibold">
              <ExternalLink size={15} /> Open
            </a>
          )}
        </div>
      </li>
    );
  }

  return (
    <li className="flex items-center gap-3 px-4 py-3">
      <FileIcon name={name} />
      <div className="min-w-0 flex-1">
        <div className="truncate text-[15px] font-semibold">{name}</div>
        <div className="text-[13px] text-dim">
          {formatBytes(size)}
          {item.folder ? ' · zipped folder' : ''} · {formatRelative(created)}
        </div>
      </div>
      {canShareFiles &&
        (shareFile ? (
          <button aria-label={`Share or save ${name}`} onClick={share} className="press grid h-10 min-w-10 place-items-center rounded-full bg-good px-3 text-[13px] font-bold text-white">
            Save
          </button>
        ) : (
          <button aria-label={`Get ${name} for saving to Photos or Files`} onClick={prepareShare} disabled={busy} className="press grid h-10 w-10 place-items-center rounded-full bg-surface-2 text-fg disabled:opacity-60">
            <Share size={18} />
          </button>
        ))}
      <button aria-label={`Download ${name}`} onClick={download} disabled={busy} className="press grid h-10 w-10 place-items-center rounded-full bg-accent text-white disabled:opacity-60">
        <Download size={18} />
      </button>
      <button aria-label={`Dismiss ${name}`} onClick={dismiss} className="press grid h-10 w-10 place-items-center rounded-full bg-surface-2 text-dim">
        <X size={18} />
      </button>
    </li>
  );
}

/** Put text (a link, a code…) on the PC's clipboard. */
function ClipboardComposer() {
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const paste = async () => {
    try {
      const t = await navigator.clipboard.readText();
      if (t) setText(t);
    } catch {
      toast.info('Paste into the box', 'This browser does not let the page read the clipboard.');
    }
  };
  const send = async () => {
    if (!text.trim()) return;
    setBusy(true);
    try {
      await client.sendClipboard(text);
      setText('');
      toast.success('On the PC clipboard', 'Paste it there with Ctrl+V.');
    } catch (e) {
      toast.error("Couldn't send", errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <>
      <SectionTitle>Send to PC clipboard</SectionTitle>
      <section className="card p-4">
        <textarea className="field min-h-[72px] resize-none leading-relaxed" placeholder="A link, a code, an address…" value={text} onChange={(e) => setText(e.target.value)} aria-label="Text for the PC clipboard" />
        <div className="mt-3 flex gap-2">
          {typeof navigator !== 'undefined' && !!navigator.clipboard?.readText && (
            <Button variant="soft" onClick={paste} icon={<ClipboardPaste size={17} />}>
              Paste
            </Button>
          )}
          <Button className="flex-1" loading={busy} disabled={!text.trim()} onClick={send} icon={<Send size={17} />}>
            Copy on PC
          </Button>
        </div>
      </section>
    </>
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

/** What the PC is playing, with play/pause and next; tap for the Music tab. */
function NowPlayingCard({ onOpen }: { onOpen: () => void }) {
  const [state, setState] = useState<MediaState | null>(null);
  const [art, setArt] = useState<string | null>(null);
  useEffect(() => {
    void client.media().then((r) => setState(r.state), () => undefined);
  }, []);
  useEvent<{ state: MediaState | null }>('media:state', (p) => setState(p.state));
  useEffect(() => {
    if (!state?.art) return setArt(null);
    let alive = true;
    let made: string | null = null;
    client.mediaArt(state.art).then(
      (u) => {
        made = u;
        if (alive) setArt(u);
        else URL.revokeObjectURL(u);
      },
      () => undefined,
    );
    return () => {
      alive = false;
      if (made) URL.revokeObjectURL(made);
    };
  }, [state?.art]);
  if (!state) return null;
  const act = (a: 'toggle' | 'next') => (e: React.MouseEvent) => {
    e.stopPropagation();
    if (a === 'toggle') setState({ ...state, playing: !state.playing });
    void client.mediaControl(a).catch((err: unknown) => toast.error('The player did not respond', errorMessage(err)));
  };
  return (
    <div role="button" tabIndex={0} onClick={onOpen} className="press card mt-3 flex items-center gap-3 p-3">
      {art ? <img src={art} alt="" className="h-12 w-12 shrink-0 rounded-xl object-cover" /> : <div className="grid h-12 w-12 shrink-0 place-items-center rounded-xl bg-accent-soft text-accent"><Music2 size={20} /></div>}
      <div className="min-w-0 flex-1">
        <div className="truncate text-[15px] font-semibold">{state.title}</div>
        <div className="truncate text-xs text-dim">{state.artist || state.appName}</div>
      </div>
      {state.canPlayPause && (
        <button type="button" onClick={act('toggle')} aria-label={state.playing ? 'Pause' : 'Play'} className="grid h-10 w-10 place-items-center rounded-full bg-[var(--fg)] text-[var(--bg)]">
          {state.playing ? <Pause size={18} fill="currentColor" /> : <Play size={18} fill="currentColor" />}
        </button>
      )}
      {state.canNext && (
        <button type="button" onClick={act('next')} aria-label="Next track" className="grid h-10 w-10 place-items-center rounded-full text-dim">
          <SkipForward size={20} fill="currentColor" />
        </button>
      )}
    </div>
  );
}
