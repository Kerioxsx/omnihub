// Chats on the phone: every conversation on the PC (OmniHub's own with
// Claude and ChatGPT, Claude Code's and Codex's), open one full screen and
// text it like a messaging app while the answer streams in; start new ones,
// save ideas, and check usage.

import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { AnimatePresence, motion } from 'motion/react';
import { ArrowLeft, ArrowUp, Bot, Code2, Copy, FolderOpen, Gauge, Lightbulb, MessagesSquare, Plus, Search, Sparkles, Square, SquareTerminal, Wrench, X } from 'lucide-react';
import type { AgentUsage, ChatDelta, ChatFolder, ChatMessage, ChatSource, ChatSummary, ChatTools, ChatView, ClaudeCodeMode, CodexSandbox, NewChatKind, Note, UsagePeriod, UsageReport } from '@shared/types';
import { formatRelative } from '@shared/format';
import { client } from '../client';
import { useApp, toast } from '../state';
import { PullToRefresh } from '../ui/PullToRefresh';
import { Sheet } from '../ui/Sheet';
import { Button, Empty, ErrorState, IconButton, ListSkeleton, PageHeader, ProgressBar } from '../ui/common';
import { useBackHandler } from '../lib/back';
import { useEvent } from '../lib/events';
import { copyToClipboard, cx, errorMessage, useKeyboardInset } from '../lib/util';
import { renderMarkdown } from './Notes';

type Kind = { label: string; icon: React.ReactNode; tone: string };

const KINDS: Record<'claude' | 'openai' | 'claudeCode' | 'codex', Kind> = {
  claude: { label: 'Claude', icon: <Sparkles size={18} />, tone: 'bg-[#d97757]/15 text-[#e08a6c]' },
  openai: { label: 'ChatGPT', icon: <Bot size={18} />, tone: 'bg-emerald-500/15 text-emerald-400' },
  claudeCode: { label: 'Claude Code', icon: <SquareTerminal size={18} />, tone: 'bg-amber-500/15 text-amber-400' },
  codex: { label: 'Codex', icon: <Code2 size={18} />, tone: 'bg-sky-500/15 text-sky-400' },
};

function kindOf(c: Pick<ChatSummary, 'source' | 'provider'>): Kind {
  if (c.source === 'omni') return c.provider === 'openai' ? KINDS.openai : KINDS.claude;
  return KINDS[c.source];
}

function KindBadge({ c, size = 40 }: { c: Pick<ChatSummary, 'source' | 'provider'>; size?: number }) {
  const k = kindOf(c);
  return (
    <div className={cx('grid shrink-0 place-items-center rounded-2xl', k.tone)} style={{ width: size, height: size }}>
      {k.icon}
    </div>
  );
}

const folderName = (p: string | null) => (p ? (p.split(/[\\/]/).filter(Boolean).pop() ?? p) : '');

const MODES: Record<'claudeCode' | 'codex', { id: string; label: string }[]> = {
  claudeCode: [
    { id: 'plan', label: 'Plan only' },
    { id: 'default', label: 'Ask first' },
    { id: 'acceptEdits', label: 'Edit files' },
    { id: 'auto', label: 'Auto' },
  ],
  codex: [
    { id: 'read-only', label: 'Read only' },
    { id: 'workspace-write', label: 'Edit the project' },
  ],
};

function plain(md: string): string {
  return md
    .replace(/```[\s\S]*?```/g, ' ')
    .replace(/(\*\*|\*|`|~~|#+\s)/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}

type View = 'list' | 'ideas' | 'usage';
type Filter = 'all' | ChatSource;

export function ChatsScreen({ active }: { active: boolean }) {
  const info = useApp((s) => s.info);
  const [chats, setChats] = useState<ChatSummary[] | null>(null);
  const [tools, setTools] = useState<ChatTools | null>(null);
  const [defaults, setDefaults] = useState<{ claudeCode: ClaudeCodeMode; codex: CodexSandbox }>({ claudeCode: 'acceptEdits', codex: 'workspace-write' });
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<Filter>('all');
  const [q, setQ] = useState('');
  const [open, setOpen] = useState<{ source: ChatSource; id: string } | null>(null);
  const [view, setView] = useState<View>('list');
  const [creating, setCreating] = useState(false);
  const agents = !!info?.features.chatAgents;

  const load = useCallback(async () => {
    try {
      const r = await client.chats();
      setChats(r.chats);
      setTools(r.tools);
      setDefaults({ claudeCode: r.claudeMode, codex: r.codexSandbox });
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);
  useEffect(() => {
    if (active) load();
  }, [active, load]);
  useEvent('chats:changed', () => active && load());
  useBackHandler(active && view !== 'list', () => setView('list'));

  const shown = useMemo(() => {
    const s = q.trim().toLowerCase();
    return (chats ?? []).filter((c) => (filter === 'all' || c.source === filter) && (!s || `${c.title} ${c.preview} ${c.folder ?? ''}`.toLowerCase().includes(s)));
  }, [chats, filter, q]);

  return (
    <div className="relative h-full overflow-hidden">
      <PullToRefresh onRefresh={load}>
        <PageHeader
          title={view === 'ideas' ? 'Ideas' : view === 'usage' ? 'Usage' : 'Chats'}
          subtitle={view === 'list' ? 'Claude, ChatGPT, Claude Code and Codex' : 'Chats'}
          left={
            view !== 'list' ? (
              <IconButton label="Back" onClick={() => setView('list')} className="-ml-1">
                <ArrowLeft size={20} />
              </IconButton>
            ) : undefined
          }
          right={
            view === 'list' ? (
              <>
                <IconButton label="Ideas" onClick={() => setView('ideas')}>
                  <Lightbulb size={19} />
                </IconButton>
                <IconButton label="Usage" onClick={() => setView('usage')}>
                  <Gauge size={19} />
                </IconButton>
              </>
            ) : undefined
          }
        />
        <div className="px-safe pb-tabbar">
          {view === 'ideas' && <IdeasView active={active} onAsk={(c) => (setView('list'), setOpen(c))} />}
          {view === 'usage' && <UsageView active={active} />}
          {view === 'list' && (
            <>
              <div className="no-scrollbar -mx-1 flex gap-2 overflow-x-auto px-1 pb-1">
                {(['all', 'omni', 'claudeCode', 'codex'] as Filter[]).map((f) => (
                  <button key={f} onClick={() => setFilter(f)} className={cx('press h-9 shrink-0 rounded-full px-4 text-sm font-semibold', filter === f ? 'bg-accent text-white' : 'bg-surface-2 text-dim')}>
                    {f === 'all' ? 'All' : f === 'omni' ? 'OmniHub' : f === 'claudeCode' ? 'Claude Code' : 'Codex'}
                  </button>
                ))}
              </div>
              <div className="relative mt-3">
                <Search size={17} className="pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-faint" />
                <input className="field h-11 py-0 pl-10 pr-10" type="search" placeholder="Find a chat" value={q} onChange={(e) => setQ(e.target.value)} enterKeyHint="search" />
                {q && (
                  <button aria-label="Clear search" onClick={() => setQ('')} className="absolute right-2 top-1/2 grid h-8 w-8 -translate-y-1/2 place-items-center rounded-full text-faint">
                    <X size={16} />
                  </button>
                )}
              </div>
              <div className="mt-3">
                {error && !chats ? (
                  <ErrorState message={error} onRetry={load} />
                ) : !chats ? (
                  <ListSkeleton rows={6} />
                ) : shown.length === 0 ? (
                  <Empty icon={<MessagesSquare size={28} />} title={q ? 'No matches' : 'No chats yet'} body={q ? `Nothing matches “${q}”.` : 'Start one with the + button: Claude, ChatGPT, or Claude Code and Codex on your PC.'} />
                ) : (
                  <div className="space-y-2">
                    {shown.map((c) => (
                      <button key={`${c.source}:${c.id}`} onClick={() => setOpen({ source: c.source, id: c.id })} className="press card flex w-full items-start gap-3 p-3.5 text-left">
                        <KindBadge c={c} />
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <div className="min-w-0 flex-1 truncate text-[16px] font-semibold">{c.title}</div>
                            {c.running ? <span className="h-2 w-2 shrink-0 animate-pulse rounded-full bg-accent" aria-label="Answering" /> : <span className="shrink-0 text-xs text-faint">{formatRelative(c.updated)}</span>}
                          </div>
                          <div className="mt-0.5 line-clamp-2 text-sm text-dim">{plain(c.preview) || '…'}</div>
                          <div className="mt-1 truncate text-xs text-faint">
                            {kindOf(c).label}
                            {c.folder ? ` · ${folderName(c.folder)}` : ''}
                          </div>
                        </div>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </>
          )}
        </div>
      </PullToRefresh>

      {view === 'list' && (
        <button
          aria-label="New chat"
          onClick={() => setCreating(true)}
          className="press grad-bg fixed right-[max(16px,var(--safe-right))] z-30 grid h-[58px] w-[58px] place-items-center rounded-[20px] text-white shadow-[0_14px_34px_-10px_var(--accent-glow)] bottom-tabbar"
        >
          <Plus size={28} strokeWidth={2.4} />
        </button>
      )}

      <NewChatSheet
        open={creating}
        onClose={() => setCreating(false)}
        tools={tools}
        agents={agents}
        defaults={defaults}
        onCreated={(c) => {
          setCreating(false);
          setOpen({ source: c.source, id: c.id });
          load();
        }}
      />
      <ChatOverlay chat={active ? open : null} defaults={defaults} agents={agents} onClose={() => (setOpen(null), load())} />
    </div>
  );
}

function Bubble({ m, onIdea }: { m: ChatMessage; onIdea: (t: string) => void }) {
  if (m.role === 'user') {
    return (
      <div className="flex justify-end">
        <div className="selectable max-w-[85%] whitespace-pre-wrap break-words rounded-[22px] rounded-br-md bg-accent px-4 py-2.5 text-[15.5px] leading-snug text-white">{m.text}</div>
      </div>
    );
  }
  if (m.role === 'assistant') {
    return (
      <div className="flex flex-col items-start gap-1">
        <div className="selectable md max-w-[92%] break-words rounded-[22px] rounded-bl-md bg-surface px-4 py-2.5 text-[15.5px] leading-relaxed" dangerouslySetInnerHTML={{ __html: renderMarkdown(m.text) }} />
        <div className="flex gap-1 pl-1">
          <button className="press flex h-8 items-center gap-1 rounded-full px-2.5 text-xs font-semibold text-faint active:bg-surface-2" onClick={() => onIdea(m.text)}>
            <Lightbulb size={14} /> Idea
          </button>
          <button className="press flex h-8 items-center gap-1 rounded-full px-2.5 text-xs font-semibold text-faint active:bg-surface-2" onClick={() => void copyToClipboard(m.text).then((ok) => (ok ? toast.success('Copied') : toast.error("Couldn't copy")))}>
            <Copy size={14} /> Copy
          </button>
        </div>
      </div>
    );
  }
  if (m.role === 'tools') return <Steps tools={m.tools ?? []} />;
  return <div className={cx('mx-auto max-w-[90%] rounded-2xl px-3.5 py-2 text-center text-[13px]', m.tone === 'bad' ? 'bg-bad/12 text-bad' : m.tone === 'warn' ? 'bg-warn/12 text-warn' : 'bg-surface-2 text-dim')}>{m.text}</div>;
}

function Steps({ tools }: { tools: string[] }) {
  const [open, setOpen] = useState(false);
  if (!tools.length) return null;
  return (
    <button onClick={() => setOpen(!open)} className="block w-full text-left">
      <div className="flex items-center gap-1.5 pl-1 text-xs font-medium text-faint">
        <Wrench size={12} /> {tools.length === 1 ? tools[0] : open ? `${tools.length} steps` : `${tools[tools.length - 1]} (+${tools.length - 1} more)`}
      </div>
      {open && tools.length > 1 && (
        <div className="mt-1 space-y-0.5 pl-5">
          {tools.map((t, i) => (
            <div key={i} className="truncate font-mono text-[11.5px] text-faint">
              {t}
            </div>
          ))}
        </div>
      )}
    </button>
  );
}

function ChatOverlay({ chat, defaults, agents, onClose }: { chat: { source: ChatSource; id: string } | null; defaults: { claudeCode: ClaudeCodeMode; codex: CodexSandbox }; agents: boolean; onClose: () => void }) {
  const [view, setView] = useState<ChatView | null>(null);
  const [live, setLive] = useState<{ prompt: string; text: string; tools: string[]; note: string } | null>(null);
  const [text, setText] = useState('');
  const [mode, setMode] = useState('');
  const [error, setError] = useState<string | null>(null);
  const scroller = useRef<HTMLDivElement>(null);
  const kb = useKeyboardInset();
  useBackHandler(!!chat, onClose);

  const load = useCallback(async () => {
    if (!chat) return;
    try {
      const v = await client.chat(chat.source, chat.id);
      setView(v);
      setLive(v.live ? { prompt: v.live.prompt, text: v.live.text, tools: v.live.tools, note: v.live.note } : null);
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, [chat]);
  useEffect(() => {
    setView(null);
    setLive(null);
    setText('');
    if (chat) {
      setMode(chat.source === 'claudeCode' ? defaults.claudeCode : chat.source === 'codex' ? defaults.codex : '');
      load();
    }
  }, [chat, load, defaults]);
  useEffect(() => {
    const el = scroller.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [view?.messages.length, live?.text, live?.tools.length]);
  useEvent<ChatDelta>('chats:delta', (d) => {
    if (!chat || d.source !== chat.source || d.id !== chat.id) return;
    if (d.kind === 'done' || d.kind === 'error') {
      if (d.kind === 'error') toast.error('It couldn’t answer', d.text);
      setLive(null);
      load();
      return;
    }
    if (d.kind === 'notice') toast.info('Heads up', d.text);
    setLive((l) => {
      const cur = l ?? { prompt: '', text: '', tools: [], note: '' };
      if (d.kind === 'text') return { ...cur, text: cur.text + d.text, note: '' };
      if (d.kind === 'tool') return { ...cur, tools: [...cur.tools, d.text], note: '' };
      if (d.kind === 'note') return { ...cur, note: d.text };
      return cur;
    });
  });
  useEvent('chats:changed', (e: { source: ChatSource; id: string }) => {
    if (chat && e.source === chat.source && e.id === chat.id && !live) load();
  });

  const send = async () => {
    const t = text.trim();
    if (!t || !chat) return;
    try {
      await client.chatSend(chat.source, chat.id, t, mode);
      setText('');
      setLive({ prompt: t, text: '', tools: [], note: '' });
    } catch (e) {
      toast.error('Not sent', errorMessage(e));
    }
  };
  const idea = async (t: string) => {
    try {
      await client.saveIdea(t, view ? `${kindOf(view.summary).label} · ${view.summary.title}` : null);
      toast.success('Saved as an idea');
    } catch (e) {
      toast.error("Couldn't save", errorMessage(e));
    }
  };

  const c = view?.summary;
  const isAgent = chat?.source === 'claudeCode' || chat?.source === 'codex';
  const lastUser = view ? [...view.messages].reverse().find((m) => m.role === 'user') : undefined;
  return createPortal(
    <AnimatePresence>
      {chat && (
        <motion.div key="chat" className="fixed inset-0 z-[55] flex flex-col bg-bg" initial={{ x: '100%' }} animate={{ x: 0 }} exit={{ x: '100%' }} transition={{ type: 'spring', damping: 34, stiffness: 340 }}>
          <header className="px-safe flex items-center gap-3 border-b border-line pb-3 pt-[calc(var(--safe-top)+10px)]">
            <IconButton label="Back" onClick={onClose} className="-ml-1">
              <ArrowLeft size={20} />
            </IconButton>
            {c && <KindBadge c={c} size={36} />}
            <div className="min-w-0 flex-1">
              <div className="truncate text-[17px] font-bold">{c?.title ?? 'Chat'}</div>
              <div className="truncate text-xs text-dim">
                {c ? kindOf(c).label : ''}
                {c?.folder && (
                  <>
                    {' · '}
                    <FolderOpen size={11} className="-mt-0.5 inline" /> {folderName(c.folder)}
                  </>
                )}
                {c?.model ? ` · ${c.model}` : ''}
              </div>
            </div>
          </header>
          <div ref={scroller} className="scroller flex-1 space-y-3 px-4 py-4">
            {error && !view && <ErrorState message={error} onRetry={load} />}
            {!view && !error && <ListSkeleton rows={4} thumb={false} />}
            {view?.earlier && <div className="text-center text-xs text-faint">Older messages are on the PC.</div>}
            {view && view.messages.map((m, i) => <Bubble key={i} m={m} onIdea={(t) => void idea(t)} />)}
            {live && c && (
              <>
                {live.prompt && lastUser?.text.trim() !== live.prompt.trim() && <Bubble m={{ role: 'user', text: live.prompt, at: 0 }} onIdea={() => undefined} />}
                <Steps tools={live.tools} />
                {live.text && <div className="md max-w-[92%] break-words rounded-[22px] rounded-bl-md bg-surface px-4 py-2.5 text-[15.5px] leading-relaxed" dangerouslySetInnerHTML={{ __html: renderMarkdown(live.text) }} />}
                <div className="flex items-center gap-2 pl-1 text-[13px] text-dim">
                  <span className="flex gap-1">
                    {[0, 1, 2].map((i) => (
                      <span key={i} className="h-1.5 w-1.5 animate-bounce rounded-full bg-accent" style={{ animationDelay: `${i * 120}ms` }} />
                    ))}
                  </span>
                  {live.note || `${kindOf(c).label} is working…`}
                </div>
              </>
            )}
            {view?.busyElsewhere && !live && <div className="rounded-2xl bg-warn/12 px-3.5 py-2.5 text-[13px] text-warn">This conversation changed on the PC in the last two minutes, so it may be open there. A message from here runs separately.</div>}
          </div>
          <div className="border-t border-line bg-glass px-3 pt-2 backdrop-blur-2xl" style={{ paddingBottom: kb ? 8 : 'calc(var(--safe-bottom) + 8px)', marginBottom: kb }}>
            {isAgent && (
              <div className="mb-2 flex items-center gap-2 px-1 text-xs text-dim">
                <span>It may:</span>
                <select className="h-8 rounded-full bg-surface-2 px-3 text-xs font-semibold text-fg" value={mode} onChange={(e) => setMode(e.target.value)} aria-label="What it may do">
                  {MODES[chat.source as 'claudeCode' | 'codex'].map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.label}
                    </option>
                  ))}
                </select>
                {!agents && <span className="text-warn">Turned off on the PC</span>}
              </div>
            )}
            <div className="flex items-end gap-2">
              <textarea
                className="field max-h-40 min-h-[46px] flex-1 resize-none rounded-[23px] py-3 leading-snug"
                rows={1}
                placeholder={c ? `Message ${kindOf(c).label}` : 'Message'}
                value={text}
                onChange={(e) => setText(e.target.value)}
                disabled={!!live || (isAgent && !agents)}
                enterKeyHint="send"
              />
              {live ? (
                <button aria-label="Stop" onClick={() => chat && void client.chatStop(chat.source, chat.id)} className="press grid h-[46px] w-[46px] shrink-0 place-items-center rounded-full bg-surface-2 text-fg">
                  <Square size={18} fill="currentColor" />
                </button>
              ) : (
                <button aria-label="Send" disabled={!text.trim()} onClick={() => void send()} className="press grad-bg grid h-[46px] w-[46px] shrink-0 place-items-center rounded-full text-white disabled:opacity-40">
                  <ArrowUp size={22} strokeWidth={2.6} />
                </button>
              )}
            </div>
          </div>
        </motion.div>
      )}
    </AnimatePresence>,
    document.body,
  );
}

function NewChatSheet({ open, onClose, tools, agents, defaults, onCreated }: { open: boolean; onClose: () => void; tools: ChatTools | null; agents: boolean; defaults: { claudeCode: ClaudeCodeMode; codex: CodexSandbox }; onCreated: (c: ChatSummary) => void }) {
  const [kind, setKind] = useState<NewChatKind>('claude');
  const [folders, setFolders] = useState<ChatFolder[]>([]);
  const [folder, setFolder] = useState('');
  const [mode, setMode] = useState('');
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const agent = kind === 'claudeCode' || kind === 'codex';
  useEffect(() => {
    if (open) client.chatFolders().then(setFolders, () => setFolders([]));
  }, [open]);
  useEffect(() => {
    if (!agent) return;
    setMode(kind === 'claudeCode' ? defaults.claudeCode : defaults.codex);
    const fit = folders.find((f) => (kind === 'claudeCode' ? f.claudeCode : f.codex)) ?? folders[0];
    if (fit && !folders.some((f) => f.path === folder)) setFolder(fit.path);
  }, [kind, agent, folders, folder, defaults]);
  const missing = kind === 'claude' ? !tools?.claudeKey : kind === 'openai' ? !tools?.openaiKey : kind === 'claudeCode' ? !tools?.claudeCode.installed : !tools?.codex.installed;
  const start = async () => {
    setBusy(true);
    try {
      const c = await client.chatNew(kind, text.trim(), agent ? folder : null, agent ? mode : '');
      setText('');
      onCreated(c);
    } catch (e) {
      toast.error('Not started', errorMessage(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Sheet
      open={open}
      onClose={onClose}
      title="New chat"
      footer={
        <Button className="w-full" loading={busy} disabled={!text.trim() || missing || (agent && (!agents || !folder))} onClick={start} icon={<ArrowUp size={18} />}>
          Start
        </Button>
      }
    >
      <div className="space-y-3 pb-2">
        <div className="grid grid-cols-2 gap-2">
          {(['claude', 'openai', 'claudeCode', 'codex'] as NewChatKind[]).map((k) => (
            <button key={k} onClick={() => setKind(k)} className={cx('press flex items-center gap-2.5 rounded-2xl border p-3 text-left', kind === k ? 'border-accent bg-accent-soft' : 'border-line bg-surface')}>
              <KindBadge c={{ source: k === 'claudeCode' || k === 'codex' ? k : 'omni', provider: k === 'openai' ? 'openai' : k === 'claude' ? 'claude' : null }} size={34} />
              <span className="text-[15px] font-semibold">{KINDS[k].label}</span>
            </button>
          ))}
        </div>
        {missing && <div className="rounded-2xl bg-warn/12 px-3.5 py-2.5 text-[13px] text-warn">{kind === 'claude' ? 'Add your Anthropic API key in OmniHub on the PC (Chats → Keys and access).' : kind === 'openai' ? 'Add your OpenAI API key in OmniHub on the PC (Chats → Keys and access).' : `${KINDS[kind].label} isn’t installed on the PC.`}</div>}
        {agent && !agents && <div className="rounded-2xl bg-warn/12 px-3.5 py-2.5 text-[13px] text-warn">Running Claude Code and Codex from the phone is turned off on the PC (Chats → Keys and access).</div>}
        {agent && (
          <>
            <select className="field" value={folder} onChange={(e) => setFolder(e.target.value)} aria-label="Project folder">
              {folders.length === 0 && <option value="">No project folders yet</option>}
              {folders.map((f) => (
                <option key={f.path} value={f.path}>
                  {f.name}
                </option>
              ))}
            </select>
            <select className="field" value={mode} onChange={(e) => setMode(e.target.value)} aria-label="What it may do">
              {MODES[kind as 'claudeCode' | 'codex'].map((m) => (
                <option key={m.id} value={m.id}>
                  It may: {m.label}
                </option>
              ))}
            </select>
          </>
        )}
        <textarea className="field min-h-[140px] resize-none leading-relaxed" placeholder={`Message ${KINDS[kind].label}…`} value={text} onChange={(e) => setText(e.target.value)} />
      </div>
    </Sheet>
  );
}

function IdeasView({ active, onAsk }: { active: boolean; onAsk: (c: { source: ChatSource; id: string }) => void }) {
  const [ideas, setIdeas] = useState<Note[] | null>(null);
  const [text, setText] = useState('');
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(() => client.ideas().then((x) => (setIdeas(x), setError(null)), (e) => setError(errorMessage(e))), []);
  useEffect(() => {
    if (active) void load();
  }, [active, load]);
  useEvent('notes:changed', () => void load());
  const save = async () => {
    try {
      await client.saveIdea(text.trim());
      setText('');
      toast.success('Idea saved');
      void load();
    } catch (e) {
      toast.error("Couldn't save", errorMessage(e));
    }
  };
  const ask = async (n: Note) => {
    try {
      const c = await client.chatNew('claude', `Here's an idea of mine. Help me think it through: what's good about it, what could go wrong, and the first three steps.\n\n${n.title}\n\n${n.body}`, null);
      onAsk({ source: c.source, id: c.id });
    } catch (e) {
      toast.error('Not started', errorMessage(e));
    }
  };
  return (
    <div className="space-y-3">
      <div className="card flex items-end gap-2 p-2.5">
        <textarea className="field min-h-[46px] flex-1 resize-none border-0 bg-transparent" rows={2} placeholder="Write an idea down…" value={text} onChange={(e) => setText(e.target.value)} />
        <Button size="sm" disabled={!text.trim()} onClick={save} icon={<Plus size={16} />}>
          Save
        </Button>
      </div>
      {error && !ideas ? (
        <ErrorState message={error} onRetry={() => void load()} />
      ) : !ideas ? (
        <ListSkeleton rows={4} thumb={false} />
      ) : ideas.length === 0 ? (
        <Empty icon={<Lightbulb size={28} />} title="No ideas yet" body="Save any chat answer with “Idea”, or write one above." />
      ) : (
        ideas.map((n) => (
          <div key={n.id} className="card space-y-2 p-4">
            <div className="flex items-start gap-2.5">
              <div className="grid h-8 w-8 shrink-0 place-items-center rounded-xl bg-amber-500/15 text-amber-400">
                <Lightbulb size={16} />
              </div>
              <div className="min-w-0 flex-1">
                <div className="text-[16px] font-semibold">{n.title}</div>
                <div className="text-xs text-faint">{formatRelative(n.updated)}</div>
              </div>
            </div>
            {n.body.trim() && n.body.trim() !== n.title && <div className="md line-clamp-6 text-sm text-dim" dangerouslySetInnerHTML={{ __html: renderMarkdown(n.body) }} />}
            <Button size="sm" variant="soft" onClick={() => void ask(n)} icon={<Sparkles size={15} />}>
              Ask Claude about it
            </Button>
          </div>
        ))
      )}
    </div>
  );
}

const sum = (t: { input: number; output: number; cacheRead: number; cacheWrite: number }) => t.input + t.output + t.cacheRead + t.cacheWrite;
const short = (n: number) => (n >= 1e9 ? `${(n / 1e9).toFixed(1)}B` : n >= 1e6 ? `${(n / 1e6).toFixed(1)}M` : n >= 1e3 ? `${(n / 1e3).toFixed(n >= 1e4 ? 0 : 1)}k` : `${n}`);
const usd = (n: number) => (n === 0 ? '$0' : n < 0.01 ? '<$0.01' : `$${n.toFixed(2)}`);
function until(unix: number): string {
  const s = unix - Date.now() / 1000;
  if (s < 3600) return `in ${Math.max(1, Math.round(s / 60))} min`;
  if (s < 86_400) return `in ${Math.floor(s / 3600)} h ${Math.round((s % 3600) / 60)} min`;
  return `in ${Math.round(s / 86_400)} days`;
}

function Periods({ p, priced }: { p: UsagePeriod; priced: boolean }) {
  return (
    <div className="grid grid-cols-3 gap-2">
      {(
        [
          ['Today', p.today],
          ['7 days', p.week],
          ['30 days', p.month],
        ] as const
      ).map(([l, t]) => (
        <div key={l} className="rounded-2xl bg-surface-2 px-3 py-2.5">
          <div className="text-[11px] font-semibold uppercase tracking-wide text-faint">{l}</div>
          <div className="text-[17px] font-bold tabular-nums">{priced ? usd(t.cost) : short(sum(t))}</div>
          <div className="text-[11px] tabular-nums text-dim">{priced ? `${short(sum(t))} tokens` : 'tokens'}</div>
        </div>
      ))}
    </div>
  );
}

function AgentCard({ title, kind, u, priced, children }: { title: string; kind: 'claudeCode' | 'codex'; u: AgentUsage; priced: boolean; children?: React.ReactNode }) {
  return (
    <div className="card space-y-3 p-4">
      <div className="flex items-center gap-3">
        <KindBadge c={{ source: kind, provider: null }} size={36} />
        <div className="min-w-0 flex-1">
          <div className="text-[17px] font-bold">{title}</div>
          {u.plan && <div className="text-xs text-dim">{u.plan.charAt(0).toUpperCase() + u.plan.slice(1)} plan</div>}
        </div>
      </div>
      {!u.found ? (
        <div className="text-sm text-faint">Nothing on the PC yet.</div>
      ) : (
        <>
          {children}
          <Periods p={u.period} priced={priced} />
        </>
      )}
    </div>
  );
}

function UsageView({ active }: { active: boolean }) {
  const [r, setR] = useState<UsageReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (active) client.chatUsage().then((x) => (setR(x), setError(null)), (e) => setError(errorMessage(e)));
  }, [active]);
  if (error && !r) return <ErrorState message={error} />;
  if (!r) return <ListSkeleton rows={3} thumb={false} />;
  const cc = r.claudeCode;
  return (
    <div className="space-y-3">
      <AgentCard title="Claude Code" kind="claudeCode" u={cc} priced>
        {cc.limits
          .filter((l) => l.reached)
          .map((l) => (
            <div key={l.name} className="rounded-2xl bg-warn/12 px-3.5 py-2.5 text-[13px] font-medium text-warn">
              {l.name} reached{l.resetsAt ? ` · resets ${until(l.resetsAt)}` : ''}
            </div>
          ))}
        {cc.window ? (
          <div className="rounded-2xl bg-accent-soft px-3.5 py-2.5 text-[13px]">
            <span className="font-semibold text-accent">This 5-hour window</span> ends {until(cc.window.end)} · {short(sum(cc.window.tokens))} tokens · {usd(cc.window.tokens.cost)} API value
          </div>
        ) : (
          <div className="text-[13px] text-faint">No use in the last 5 hours.</div>
        )}
        <div className="text-xs text-faint">Dollars are what the same use would cost on the API; a Claude plan has limits instead.</div>
      </AgentCard>
      <AgentCard title="Codex" kind="codex" u={r.codex} priced={false}>
        {r.codex.limits.map((l) => (
          <div key={l.name} className="space-y-1">
            <div className="flex justify-between text-[13px]">
              <span className="font-semibold">{l.name}</span>
              <span className="tabular-nums text-dim">{l.usedPercent === null ? 'reset since' : `${Math.round(l.usedPercent)}% used`}</span>
            </div>
            <ProgressBar value={(l.usedPercent ?? 0) / 100} tone={l.reached || (l.usedPercent ?? 0) >= 100 ? 'bad' : (l.usedPercent ?? 0) >= 80 ? 'warn' : 'accent'} />
            {l.resetsAt && <div className="text-xs text-faint">Resets {until(l.resetsAt)}</div>}
          </div>
        ))}
      </AgentCard>
      <div className="card space-y-3 p-4">
        <div className="text-[17px] font-bold">With your API keys</div>
        <div className="flex items-center gap-2 text-sm font-semibold">
          <KindBadge c={{ source: 'omni', provider: 'claude' }} size={26} /> Claude
        </div>
        <Periods p={r.omni.claude} priced />
        <div className="flex items-center gap-2 text-sm font-semibold">
          <KindBadge c={{ source: 'omni', provider: 'openai' }} size={26} /> ChatGPT
        </div>
        <Periods p={r.omni.openai} priced={false} />
        <div className="text-[13px] text-dim">
          Mods made by Claude: <span className="font-semibold text-fg">{usd(r.omni.maker.cost)}</span> all time
        </div>
      </div>
      <p className="px-2 text-center text-xs leading-relaxed text-faint">Chats in the ChatGPT and Claude apps live in your accounts online, not on the PC, so they aren’t here.</p>
    </div>
  );
}
