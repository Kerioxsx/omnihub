// Every conversation in one list (OmniHub's own with Claude and ChatGPT,
// and Claude Code's and Codex's on this PC), one open at a time with its
// answer streaming in, and a composer that picks how much Claude Code or
// Codex may do.

import type { ChatDelta, ChatFolder, ChatMessage, ChatSource, ChatSummary, ChatTools, ChatView, NewChatKind, Settings } from '@shared/types';
import { formatRelative } from '@shared/format';
import { AlertTriangle, CircleStop, Copy, FolderOpen, GitBranch, KeyRound, Lightbulb, MessagesSquare, Pencil, Plus, Send, Trash2, Wrench } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, Spinner } from '../../components/ui/Card';
import { SearchInput, Segmented, Select, Textarea, TextInput } from '../../components/ui/Form';
import { Callout, EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useEvent } from '../../lib/hooks';
import { navigate, useRoute } from '../../lib/router';
import { plainSnippet, renderMarkdown } from '../../lib/util';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { CLAUDE_MODES, CODEX_SANDBOXES, folderName, KINDS, KindIcon, kindOf, sourceOfKind } from './shared';

type Filter = 'all' | ChatSource;

function MessageLine({ m, chat, onIdea }: { m: ChatMessage; chat: ChatSummary; onIdea: (text: string) => void }) {
  if (m.role === 'user') {
    return (
      <div className="group flex justify-end gap-1.5">
        <MessageActions text={m.text} onIdea={onIdea} />
        <div className="max-w-[80%] whitespace-pre-wrap break-words rounded-2xl rounded-br-md bg-accent px-4 py-2.5 text-[13.5px] text-white shadow-sm">{m.text}</div>
      </div>
    );
  }
  if (m.role === 'assistant') {
    return (
      <div className="group flex gap-2.5">
        <KindIcon s={chat} size={28} className="mt-0.5" />
        <div className="markdown min-w-0 max-w-[85%] break-words text-[13.5px] text-fg" dangerouslySetInnerHTML={{ __html: renderMarkdown(m.text) }} />
        <MessageActions text={m.text} onIdea={onIdea} />
      </div>
    );
  }
  if (m.role === 'tools') return <ToolLines tools={m.tools ?? []} />;
  return (
    <Callout tone={m.tone === 'bad' ? 'bad' : m.tone === 'warn' ? 'warn' : 'info'} className="ml-10">
      {m.text}
    </Callout>
  );
}

function ToolLines({ tools }: { tools: string[] }) {
  const [open, setOpen] = useState(false);
  const shown = open ? tools : tools.slice(-3);
  return (
    <div className="space-y-0.5 pl-10">
      {tools.length > 3 && !open && (
        <button type="button" className="text-[11.5px] text-faint hover:text-dim" onClick={() => setOpen(true)}>
          {tools.length - 3} more step{tools.length - 3 === 1 ? '' : 's'}…
        </button>
      )}
      {shown.map((t, i) => (
        <div key={i} className="flex items-center gap-2 text-[12px] text-faint">
          <Wrench size={12} className="shrink-0" />
          <span className="truncate font-mono">{t}</span>
        </div>
      ))}
    </div>
  );
}

function MessageActions({ text, onIdea }: { text: string; onIdea: (text: string) => void }) {
  return (
    <div className="flex shrink-0 items-start gap-0.5 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
      <IconButton icon={Lightbulb} label="Save as an idea" size="sm" onClick={() => onIdea(text)} />
      <IconButton icon={Copy} label="Copy" size="sm" onClick={() => void navigator.clipboard.writeText(text).then(() => toast.info('Copied'))} />
    </div>
  );
}

function ChatRow({ c, active, onPick }: { c: ChatSummary; active: boolean; onPick: () => void }) {
  const k = kindOf(c);
  return (
    <button type="button" onClick={onPick} className={cx('flex w-full gap-2.5 rounded-xl border px-3 py-2.5 text-left transition-colors', active ? 'border-accent/50 bg-accent-soft' : 'border-transparent hover:bg-surface-2/60')}>
      <KindIcon s={c} size={30} className="mt-0.5" />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5">
          <span className="min-w-0 flex-1 truncate text-[13px] font-medium text-fg">{c.title}</span>
          {c.running ? <Spinner size={12} className="text-accent" /> : <span className="shrink-0 text-[11px] text-faint">{formatRelative(c.updated)}</span>}
        </div>
        <div className="truncate text-[12px] text-dim">{c.preview ? plainSnippet(c.preview, 140) : '…'}</div>
        <div className="mt-0.5 truncate text-[11px] text-faint">
          {k.label}
          {c.folder ? ` · ${folderName(c.folder)}` : ''}
        </div>
      </div>
    </button>
  );
}

function Conversation({ view, live, onIdea }: { view: ChatView; live: { prompt: string; text: string; tools: string[]; note: string } | null; onIdea: (text: string) => void }) {
  const chat = view.summary;
  const lastUser = [...view.messages].reverse().find((m) => m.role === 'user');
  const showPrompt = live && live.prompt && lastUser?.text.trim() !== live.prompt.trim();
  return (
    <div className="space-y-3">
      {view.earlier && <div className="text-center text-[11.5px] text-faint">Older messages are on the PC; the newest are shown.</div>}
      {view.messages.map((m, i) => (
        <MessageLine key={i} m={m} chat={chat} onIdea={onIdea} />
      ))}
      {live && (
        <>
          {showPrompt && <MessageLine m={{ role: 'user', text: live.prompt, at: 0 }} chat={chat} onIdea={onIdea} />}
          {live.tools.length > 0 && <ToolLines tools={live.tools} />}
          {live.text && (
            <div className="flex gap-2.5">
              <KindIcon s={chat} size={28} className="mt-0.5" />
              <div className="markdown min-w-0 max-w-[85%] break-words text-[13.5px] text-fg" dangerouslySetInnerHTML={{ __html: renderMarkdown(live.text) }} />
            </div>
          )}
          <div className="flex items-center gap-2 pl-10 text-[12px] text-dim">
            <Spinner size={13} /> {live.note || `${kindOf(chat).label} is working…`}
          </div>
        </>
      )}
    </div>
  );
}

function ModePicker({ source, value, onChange }: { source: ChatSource; value: string; onChange: (v: string) => void }) {
  if (source === 'omni') return null;
  const options = source === 'claudeCode' ? CLAUDE_MODES : CODEX_SANDBOXES;
  const hint = options.find((o) => o.id === value)?.hint;
  return (
    <Select value={value} onChange={(e) => onChange(e.target.value)} className="w-40" aria-label="What it may do" title={hint}>
      {options.map((o) => (
        <option key={o.id} value={o.id}>
          {o.label}
        </option>
      ))}
    </Select>
  );
}

const STARTERS: Record<NewChatKind, string[]> = {
  claude: ['Plan a cheap weekend trip for me', 'Explain how my PC’s RAM speed affects games', 'Help me write a message to my team'],
  openai: ['Summarize the news about GPUs this week', 'Give me 5 names for a gaming channel', 'Quiz me on Spanish verbs'],
  claudeCode: ['Find and fix the failing tests', 'Explain how this project is organized', 'Add a dark mode toggle'],
  codex: ['Review my last commit for bugs', 'Write tests for the checkout code', 'Make the build faster'],
};

function NewChat({ tools, folders, settings, onCreated }: { tools: ChatTools | null; folders: ChatFolder[]; settings: Settings['chats'] | null; onCreated: (c: ChatSummary) => void }) {
  const [kind, setKind] = useState<NewChatKind>('claude');
  const [folder, setFolder] = useState<string>('');
  const [mode, setMode] = useState<string>('');
  const [text, setText] = useState('');
  const [busy, setBusy] = useState(false);
  const source = sourceOfKind(kind);
  useEffect(() => {
    if (source === 'claudeCode') setMode(settings?.claudeMode ?? 'acceptEdits');
    else if (source === 'codex') setMode(settings?.codexSandbox ?? 'workspace-write');
    const fit = folders.find((f) => (source === 'claudeCode' ? f.claudeCode : f.codex)) ?? folders[0];
    if (source !== 'omni' && !folder && fit) setFolder(fit.path);
  }, [source, settings, folders, folder]);
  const missing = kind === 'claude' ? !tools?.claudeKey : kind === 'openai' ? !tools?.openaiKey : kind === 'claudeCode' ? !tools?.claudeCode.installed : !tools?.codex.installed;
  const pick = async () => {
    const dir = await api.app.pickFolder('The project folder');
    if (dir) setFolder(dir);
  };
  const start = async () => {
    const t = text.trim();
    if (!t) return;
    setBusy(true);
    try {
      const c = await api.chats.create(kind, t, source === 'omni' ? null : folder || null, mode);
      setText('');
      onCreated(c);
    } catch (e) {
      toast.error('Not started', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const choices: NewChatKind[] = ['claude', 'openai', 'claudeCode', 'codex'];
  const folderList = folders.some((f) => f.path === folder) || !folder ? folders : [{ path: folder, name: folderName(folder), last: 0, claudeCode: false, codex: false }, ...folders];
  return (
    <div className="mx-auto max-w-2xl space-y-5 py-6">
      <div className="text-center">
        <div className="text-[16px] font-semibold text-fg">Start a chat</div>
        <div className="mt-1 text-[12.5px] text-dim">Claude and ChatGPT use your own API keys. Claude Code and Codex run on this PC in a project folder, with your own sign-in.</div>
      </div>
      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        {choices.map((c) => {
          const k = KINDS[c];
          const Icon = k.icon;
          return (
            <button key={c} type="button" onClick={() => setKind(c)} className={cx('flex flex-col items-center gap-2 rounded-2xl border px-3 py-3.5 text-[12.5px] font-medium transition-colors', kind === c ? 'border-accent/60 bg-accent-soft text-fg' : 'border-line bg-surface-2/40 text-dim hover:text-fg')}>
              <span className={cx('grid h-9 w-9 place-items-center rounded-full', k.tint)}>
                <Icon size={18} />
              </span>
              {k.label}
            </button>
          );
        })}
      </div>
      {missing && (
        <Callout tone="warn" icon={KeyRound}>
          {kind === 'claude' ? 'Add your Anthropic API key under Keys first.' : kind === 'openai' ? 'Add your OpenAI API key under Keys first.' : `${KINDS[kind].label} isn’t installed on this PC (OmniHub looks for it on PATH).`}
        </Callout>
      )}
      {source !== 'omni' && (
        <div className="flex flex-wrap items-center gap-2">
          <Select value={folder} onChange={(e) => setFolder(e.target.value)} className="min-w-0 flex-1" aria-label="Project folder">
            {!folder && <option value="">Choose a project folder…</option>}
            {folderList.map((f) => (
              <option key={f.path} value={f.path}>
                {f.name} — {f.path}
              </option>
            ))}
          </Select>
          <Button variant="secondary" icon={FolderOpen} onClick={() => void pick()}>
            Other folder
          </Button>
          <ModePicker source={source} value={mode} onChange={setMode} />
        </div>
      )}
      <div className="flex flex-wrap justify-center gap-2">
        {STARTERS[kind].map((s) => (
          <button key={s} type="button" className="rounded-full border border-line bg-surface-2/60 px-3 py-1.5 text-[12px] text-dim hover:border-accent/50 hover:text-fg" onClick={() => setText(s)}>
            {s}
          </button>
        ))}
      </div>
      <div className="flex items-end gap-2">
        <Textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
              e.preventDefault();
              void start();
            }
          }}
          rows={3}
          placeholder={`Message ${KINDS[kind].label}…`}
          className="flex-1 resize-none"
          aria-label="First message"
        />
        <Button variant="primary" icon={Send} disabled={!text.trim() || busy || missing || (source !== 'omni' && !folder)} onClick={() => void start()}>
          Start
        </Button>
      </div>
    </div>
  );
}

export function ChatsTab({ onIdea }: { onIdea: (text: string, from: string) => void }) {
  const route = useRoute();
  const source = route.params.get('source') as ChatSource | null;
  const id = route.params.get('id');
  const [chats, setChats] = useState<ChatSummary[] | null>(null);
  const [view, setView] = useState<ChatView | null>(null);
  const [tools, setTools] = useState<ChatTools | null>(null);
  const [folders, setFolders] = useState<ChatFolder[]>([]);
  const [settings, setSettings] = useState<Settings['chats'] | null>(null);
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState<Filter>('all');
  const [text, setText] = useState('');
  const [mode, setMode] = useState('');
  const [live, setLive] = useState<{ prompt: string; text: string; tools: string[]; note: string } | null>(null);
  const [renaming, setRenaming] = useState(false);
  const scroller = useRef<HTMLDivElement>(null);
  const open = (c: { source: ChatSource; id: string } | null) => navigate('chats', c ? { source: c.source, id: c.id } : {});

  const loadList = useCallback(() => void api.chats.list().then(setChats, (e: unknown) => toast.error('Can’t list the chats', errorText(e))), []);
  const loadView = useCallback(() => {
    if (!source || !id) {
      setView(null);
      return;
    }
    void api.chats.view(source, id).then(
      (v) => {
        setView(v);
        setLive(v.live ? { prompt: v.live.prompt, text: v.live.text, tools: v.live.tools, note: v.live.note } : null);
      },
      () => setView(null),
    );
  }, [source, id]);
  useEffect(loadList, [loadList]);
  useEffect(loadView, [loadView]);
  useEffect(() => {
    void api.chats.tools().then(setTools, () => undefined);
    void api.chats.folders().then(setFolders, () => undefined);
    void api.app.settings().then((s) => setSettings(s.chats), () => undefined);
  }, []);
  useEffect(() => {
    if (source === 'claudeCode') setMode(settings?.claudeMode ?? 'acceptEdits');
    else if (source === 'codex') setMode(settings?.codexSandbox ?? 'workspace-write');
  }, [source, settings]);
  useEffect(() => {
    const el = scroller.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [view?.messages.length, live?.text, live?.tools.length, id]);

  useEvent<{ source: ChatSource; id: string }>('chats:changed', (e) => {
    loadList();
    if (e.source === source && e.id === id && !live) loadView();
  });
  useEvent<ChatDelta>('chats:delta', (d) => {
    if (d.source !== source || d.id !== id) return;
    if (d.kind === 'done' || d.kind === 'error') {
      if (d.kind === 'error') toast.error(`${view ? kindOf(view.summary).label : 'It'} couldn’t answer`, d.text);
      setLive(null);
      loadView();
      loadList();
      return;
    }
    setLive((l) => {
      const cur = l ?? { prompt: '', text: '', tools: [], note: '' };
      if (d.kind === 'text') return { ...cur, text: cur.text + d.text, note: '' };
      if (d.kind === 'tool') return { ...cur, tools: [...cur.tools, d.text], note: '' };
      if (d.kind === 'note') return { ...cur, note: d.text };
      return cur;
    });
    if (d.kind === 'notice') toast.warn('Heads up', d.text);
  });

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (chats ?? []).filter((c) => (filter === 'all' || c.source === filter) && (!q || `${c.title}\n${c.preview}\n${c.folder ?? ''}`.toLowerCase().includes(q)));
  }, [chats, query, filter]);

  const send = async () => {
    const t = text.trim();
    if (!t || !source || !id) return;
    try {
      await api.chats.send(source, id, t, mode);
      setText('');
      setLive({ prompt: t, text: '', tools: [], note: '' });
    } catch (e) {
      toast.error('Not sent', errorText(e));
    }
  };
  const remove = async (c: ChatSummary) => {
    if (!(await confirm({ title: `Delete “${c.title}”?`, description: 'The conversation is deleted from this PC.', confirmLabel: 'Delete', tone: 'danger' }))) return;
    await api.chats.remove(c.id).catch((e: unknown) => toast.error('Not deleted', errorText(e)));
    open(null);
    loadList();
  };
  const c = view?.summary;
  const counts = (s: ChatSource) => (chats ?? []).filter((x) => x.source === s).length;
  return (
    <div className="grid gap-4 lg:grid-cols-[300px_minmax(0,1fr)]">
      <div className="flex min-h-0 flex-col gap-2.5">
        <Button variant={!id ? 'primary' : 'secondary'} icon={Plus} className="w-full" onClick={() => open(null)}>
          New chat
        </Button>
        <SearchInput value={query} onChange={setQuery} placeholder="Find a chat…" />
        <Segmented
          label="Show"
          size="sm"
          value={filter}
          onChange={setFilter}
          options={[
            { value: 'all', label: 'All' },
            { value: 'omni', label: `OmniHub${chats ? ` ${counts('omni')}` : ''}` },
            { value: 'claudeCode', label: `Claude Code${chats ? ` ${counts('claudeCode')}` : ''}` },
            { value: 'codex', label: `Codex${chats ? ` ${counts('codex')}` : ''}` },
          ]}
        />
        <div className="max-h-[calc(100vh-300px)] min-h-[200px] space-y-1 overflow-y-auto pr-1">
          {chats === null && (
            <div className="flex justify-center py-8">
              <Spinner />
            </div>
          )}
          {chats && shown.length === 0 && <div className="px-2 py-6 text-center text-[12.5px] text-faint">{chats.length ? 'No chat matches.' : 'No chats yet.'}</div>}
          {shown.map((x) => (
            <ChatRow key={`${x.source}:${x.id}`} c={x} active={x.source === source && x.id === id} onPick={() => open(x)} />
          ))}
        </div>
      </div>

      <Card className="flex h-[min(78vh,860px)] min-h-[480px] flex-col">
        {!id || !source ? (
          <div className="min-h-0 flex-1 overflow-y-auto px-5">
            <NewChat tools={tools} folders={folders} settings={settings} onCreated={(x) => (open(x), loadList())} />
          </div>
        ) : !view || !c ? (
          <EmptyState icon={MessagesSquare} title="Opening the chat…" className="flex-1" />
        ) : (
          <>
            <div className="flex items-center gap-3 border-b border-line px-5 py-3">
              <KindIcon s={c} size={34} />
              <div className="min-w-0 flex-1">
                {renaming ? (
                  <TextInput
                    autoFocus
                    defaultValue={c.title}
                    inputSize="sm"
                    aria-label="Name"
                    onBlur={(e) => {
                      setRenaming(false);
                      const t = e.target.value.trim();
                      if (t && t !== c.title) void api.chats.rename(c.id, t).then(loadView);
                    }}
                    onKeyDown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()}
                  />
                ) : (
                  <div className="truncate text-[14.5px] font-semibold text-fg">{c.title}</div>
                )}
                <div className="flex flex-wrap items-center gap-1.5 text-[11.5px] text-faint">
                  <span>{kindOf(c).label}</span>
                  {c.model && <span>· {c.model}</span>}
                  {c.folder && (
                    <span className="inline-flex items-center gap-1" title={c.folder}>
                      · <FolderOpen size={11} /> {folderName(c.folder)}
                    </span>
                  )}
                  {c.branch && (
                    <span className="inline-flex items-center gap-1">
                      · <GitBranch size={11} /> {c.branch}
                    </span>
                  )}
                </div>
              </div>
              {c.running && <Badge tone="info">Answering</Badge>}
              {c.source === 'omni' && (
                <>
                  <IconButton icon={Pencil} label="Rename" size="sm" onClick={() => setRenaming(true)} />
                  <IconButton icon={Trash2} label="Delete" size="sm" onClick={() => void remove(c)} />
                </>
              )}
            </div>
            <div ref={scroller} className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
              <Conversation view={view} live={live} onIdea={(t) => onIdea(t, `${kindOf(c).label} · ${c.title}`)} />
            </div>
            {view.busyElsewhere && !live && (
              <Callout tone="warn" icon={AlertTriangle} className="mx-4 mb-2">
                This conversation changed on the PC in the last two minutes, so it may be open in a terminal. A message from here runs separately; the terminal won’t show it until you reopen the conversation there.
              </Callout>
            )}
            <div className="flex items-end gap-2 border-t border-line px-4 py-3">
              <Textarea
                value={text}
                onChange={(e) => setText(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter' && !e.shiftKey) {
                    e.preventDefault();
                    void send();
                  }
                }}
                rows={2}
                placeholder={`Message ${kindOf(c).label}…`}
                className="min-h-[44px] flex-1 resize-none"
                aria-label="Message"
                disabled={!!live}
              />
              <ModePicker source={c.source} value={mode} onChange={setMode} />
              {live ? (
                <Button variant="secondary" icon={CircleStop} onClick={() => void api.chats.stop(c.source, c.id)}>
                  Stop
                </Button>
              ) : (
                <Button variant="primary" icon={Send} disabled={!text.trim()} onClick={() => void send()}>
                  Send
                </Button>
              )}
            </div>
          </>
        )}
      </Card>
    </div>
  );
}

