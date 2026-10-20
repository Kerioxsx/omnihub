// Claude making a mod for one game: the game's mod projects, the
// conversation (streamed as Claude works: its notes, what it reads and
// writes, its answer), approvals for builds, the mod's files, and Install.

import type { MakerDelta, MakerItem, MakerProject, MakerView, ModGame } from '@shared/types';
import { formatBytes } from '@shared/format';
import { Bot, Check, CircleStop, Download, FileCode2, Globe, KeyRound, Pencil, Plus, Send, Sparkles, Trash2, Wrench, X } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, Spinner } from '../../components/ui/Card';
import { Textarea, TextInput } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { Callout, EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useEvent } from '../../lib/hooks';
import { renderMarkdown } from '../../lib/util';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { modelName, money } from './shared';

const IDEAS: Record<string, string[]> = {
  steamodded: ['A joker that gives +4 mult, and x2 when a Lucky card scores', 'A new deck that starts with two extra jokers slots', 'A tarot card that turns a card into a Gold card'],
  shvdnScript: ['Press F7 for slow motion', 'A key that fixes and cleans my car', 'Spawn my favourite car at my position'],
  ue4ssLua: ['Make day time last twice as long', 'A key that fills my stamina', 'Show my coordinates on screen'],
  bepInExPlugin: ['A key to toggle a brighter flashlight', 'Show the time on screen', 'Make items weigh less'],
  melonMod: ['Show my FPS on screen', 'A key to skip the intro', 'Make the day longer'],
  files: ['Raise the max stack size of items', 'Make the camera field of view wider', 'Turn off the screen shake'],
  lovely: ['Speed up the game’s animations', 'Change the colours of the menu', 'Show a counter on screen'],
};

function ItemLine({ it }: { it: MakerItem }) {
  if (it.kind === 'user') {
    return (
      <div className="flex justify-end">
        <div className="max-w-[80%] whitespace-pre-wrap rounded-2xl rounded-br-md bg-accent px-4 py-2.5 text-[13.5px] text-white shadow-sm">{it.text}</div>
      </div>
    );
  }
  if (it.kind === 'text') {
    return (
      <div className="flex gap-2.5">
        <div className="mt-0.5 grid h-7 w-7 shrink-0 place-items-center rounded-full bg-accent-soft text-accent">
          <Bot size={15} />
        </div>
        <div className="markdown min-w-0 max-w-[85%] text-[13.5px] text-fg" dangerouslySetInnerHTML={{ __html: renderMarkdown(it.text) }} />
      </div>
    );
  }
  if (it.kind === 'note') return <div className="pl-10 text-[12.5px] italic text-dim">{it.text}</div>;
  if (it.kind === 'tool' || it.kind === 'web') {
    const Icon = it.kind === 'web' ? Globe : it.text.startsWith('Wrote') || it.text.startsWith('Changed') ? FileCode2 : it.text.startsWith('Finished') ? Check : Wrench;
    return (
      <div className="flex items-center gap-2 pl-10 text-[12px] text-faint">
        <Icon size={12} className="shrink-0" />
        <span className="truncate font-mono">{it.text}</span>
      </div>
    );
  }
  return (
    <Callout tone={it.tone === 'bad' ? 'bad' : it.tone === 'warn' ? 'warn' : it.tone === 'good' ? 'good' : 'info'} className="ml-10">
      {it.text}
    </Callout>
  );
}

function Conversation({ view, live, note }: { view: MakerView; live: string; note: string }) {
  const working = view.project.status === 'working';
  return (
    <div className="space-y-3">
      {view.items.map((it, i) => (
        <ItemLine key={i} it={it} />
      ))}
      {working && note && <div className="pl-10 text-[12.5px] italic text-dim">{note}</div>}
      {working && live && (
        <div className="flex gap-2.5">
          <div className="mt-0.5 grid h-7 w-7 shrink-0 place-items-center rounded-full bg-accent-soft text-accent">
            <Bot size={15} />
          </div>
          <div className="markdown min-w-0 max-w-[85%] text-[13.5px] text-fg" dangerouslySetInnerHTML={{ __html: renderMarkdown(live) }} />
        </div>
      )}
      {working && (
        <div className="flex items-center gap-2 pl-10 text-[12px] text-dim">
          <Spinner size={13} /> Claude is working…
        </div>
      )}
    </div>
  );
}

function ProjectSide({ view, onChanged }: { view: MakerView; onChanged: () => void }) {
  const p = view.project;
  const [file, setFile] = useState<{ path: string; text: string } | null>(null);
  const [renaming, setRenaming] = useState(false);
  const [installing, setInstalling] = useState(false);
  const install = async () => {
    setInstalling(true);
    try {
      await api.maker.install(p.id);
      toast.success(p.installed ? 'Updated' : 'Installed', 'Start the game to try it.');
      onChanged();
    } catch (e) {
      toast.error('Not installed', errorText(e));
    } finally {
      setInstalling(false);
    }
  };
  const open = async (path: string) => {
    try {
      setFile({ path, text: await api.maker.file(p.id, path) });
    } catch (e) {
      toast.error('Can’t show it', errorText(e));
    }
  };
  const tokens = p.usage.input + p.usage.output + p.usage.cacheRead + p.usage.cacheWrite;
  const update = p.installed && p.installedVersion < p.version;
  return (
    <div className="space-y-3">
      <Card className="space-y-3 px-4 py-4">
        {renaming ? (
          <TextInput
            autoFocus
            defaultValue={p.name}
            inputSize="sm"
            aria-label="Name"
            onBlur={(e) => {
              setRenaming(false);
              if (e.target.value.trim() && e.target.value.trim() !== p.name) void api.maker.rename(p.id, e.target.value.trim()).then(onChanged);
            }}
            onKeyDown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()}
          />
        ) : (
          <div className="flex items-center gap-1.5">
            <span className="truncate text-[14.5px] font-semibold text-fg">{p.name}</span>
            <IconButton icon={Pencil} label="Rename" size="sm" onClick={() => setRenaming(true)} />
          </div>
        )}
        <div className="flex flex-wrap gap-1.5">
          {p.ready ? <Badge tone="good">Ready · version {p.version}</Badge> : <Badge tone="neutral">In progress</Badge>}
          {p.installed && <Badge tone={update ? 'warn' : 'info'}>{update ? `Installed: version ${p.installedVersion}` : 'Installed'}</Badge>}
        </div>
        {p.summary && <div className="text-[12.5px] text-fg">{p.summary}</div>}
        {p.howToUse && (
          <div className="rounded-lg bg-surface-2/60 px-3 py-2 text-[12px] text-dim">
            <span className="font-medium text-fg">In the game: </span>
            {p.howToUse}
          </div>
        )}
        <Button variant="primary" icon={Download} className="w-full" disabled={!p.ready || installing || p.status === 'working' || (!!p.installed && !update)} onClick={() => void install()}>
          {installing ? 'Installing…' : update ? `Update to version ${p.version}` : p.installed ? 'Installed' : 'Install'}
        </Button>
      </Card>
      <Card className="px-4 py-3">
        <div className="mb-2 text-[11.5px] font-semibold uppercase tracking-wide text-faint">Its files</div>
        {view.files.length === 0 ? (
          <div className="text-[12px] text-faint">None yet.</div>
        ) : (
          <div className="space-y-0.5">
            {view.files.map((f) => (
              <button key={f.path} type="button" onClick={() => void open(f.path)} className="flex w-full items-center justify-between gap-2 rounded-md px-2 py-1 text-left font-mono text-[12px] text-dim hover:bg-surface-2 hover:text-fg">
                <span className="truncate">{f.path}</span>
                <span className="shrink-0 text-faint">{formatBytes(f.size)}</span>
              </button>
            ))}
          </div>
        )}
      </Card>
      <div className="px-1 text-[11.5px] text-faint" title={`${tokens.toLocaleString()} tokens, ${p.usage.webSearches} web searches`}>
        About {money(p.cost)} with your API key so far · {modelName(p.model)}
      </div>
      <Modal open={!!file} onClose={() => setFile(null)} title={file?.path ?? ''} icon={FileCode2} size="lg">
        <pre className="max-h-[60vh] overflow-auto rounded-xl bg-surface-2 p-4 font-mono text-[12px] leading-relaxed text-fg">{file?.text}</pre>
      </Modal>
    </div>
  );
}

export function MakerTab({ game, onSettings }: { game: ModGame; onSettings: () => void }) {
  const [projects, setProjects] = useState<MakerProject[]>([]);
  const [current, setCurrent] = useState<string | null>(null);
  const [view, setView] = useState<MakerView | null>(null);
  const [text, setText] = useState('');
  const [live, setLive] = useState('');
  const [note, setNote] = useState('');
  const [hasKey, setHasKey] = useState<boolean | null>(null);
  const [sending, setSending] = useState(false);
  const scroller = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = scroller.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [view?.items.length, live, note, current]);

  const loadProjects = useCallback(() => void api.maker.projects(game.key).then((p) => {
    setProjects(p);
    setCurrent((c) => (c && p.some((x) => x.id === c) ? c : c === 'new' ? 'new' : (p[0]?.id ?? 'new')));
  }), [game.key]);
  const loadView = useCallback(() => {
    if (current && current !== 'new') void api.maker.view(current).then((v) => {
      setView(v);
      if (v.project.status !== 'working') {
        setLive('');
        setNote('');
      }
    }, () => setView(null));
    else setView(null);
  }, [current]);
  useEffect(() => void api.claude.settings().then((s) => setHasKey(s.hasKey), () => setHasKey(false)), []);
  useEffect(loadProjects, [loadProjects]);
  useEffect(loadView, [loadView]);
  useEvent<{ project: string }>('maker:changed', (e) => {
    if (e.project === current) {
      setLive('');
      loadView();
    }
    loadProjects();
  });
  useEvent<MakerDelta>('maker:delta', (d) => {
    if (d.project !== current) return;
    if (d.kind === 'text') setLive((l) => l + d.text);
    else if (d.kind === 'note') setNote((n) => n + d.text);
    else if (d.kind === 'tool' || d.kind === 'web') setNote('');
  });

  const send = async () => {
    const t = text.trim();
    if (!t) return;
    setSending(true);
    try {
      if (current === 'new' || !current) {
        const p = await api.maker.create(game.key, t);
        setCurrent(p.id);
      } else {
        await api.maker.send(current, t);
      }
      setText('');
      setLive('');
      setNote('');
      loadProjects();
    } catch (e) {
      toast.error('Not sent', errorText(e));
    } finally {
      setSending(false);
    }
  };
  const remove = async (p: MakerProject) => {
    if (!(await confirm({ title: `Delete “${p.name}”?`, description: 'The conversation and its files go. An installed copy stays installed (remove it in Installed).', confirmLabel: 'Delete', tone: 'danger' }))) return;
    await api.maker.remove(p.id);
    setCurrent(null);
    loadProjects();
  };

  if (hasKey === false) {
    return (
      <Card>
        <EmptyState
          icon={KeyRound}
          title="Add your Anthropic API key"
          description="Claude makes mods with your own key from console.anthropic.com. It's kept encrypted for your Windows account and sent only to Anthropic."
          action={
            <Button variant="primary" icon={Sparkles} onClick={onSettings}>
              Claude settings
            </Button>
          }
        />
      </Card>
    );
  }
  const p = view?.project;
  const working = p?.status === 'working';
  return (
    <div className="grid gap-4 lg:grid-cols-[220px_minmax(0,1fr)_280px]">
      <div className="space-y-1.5">
        <Button variant={current === 'new' ? 'primary' : 'secondary'} icon={Plus} className="w-full" onClick={() => setCurrent('new')}>
          New mod
        </Button>
        {projects.map((x) => (
          <div key={x.id} className={cx('group flex items-center gap-1 rounded-xl border px-3 py-2', current === x.id ? 'border-accent/50 bg-accent-soft' : 'border-transparent hover:bg-surface-2/60')}>
            <button type="button" className="min-w-0 flex-1 text-left" onClick={() => setCurrent(x.id)}>
              <div className="truncate text-[13px] font-medium text-fg">{x.name}</div>
              <div className="text-[11px] text-faint">{x.status === 'working' ? 'Working…' : x.status === 'waiting' ? 'Waiting for you' : x.ready ? `Version ${x.version}${x.installed ? ' · installed' : ''}` : 'In progress'}</div>
            </button>
            <IconButton icon={Trash2} label={`Delete ${x.name}`} size="sm" className="opacity-0 group-hover:opacity-100" onClick={() => void remove(x)} />
          </div>
        ))}
      </div>

      <Card className="flex h-[min(70vh,780px)] min-h-[460px] flex-col">
        <div ref={scroller} className="min-h-0 flex-1 overflow-y-auto px-5 py-4">
          {current === 'new' || !view ? (
            <div className="space-y-4 py-6">
              <div className="text-center">
                <div className="mx-auto mb-3 grid h-12 w-12 place-items-center rounded-2xl bg-accent-soft text-accent">
                  <Bot size={24} />
                </div>
                <div className="text-[15px] font-semibold text-fg">What should the mod do?</div>
                <div className="mx-auto mt-1 max-w-md text-[12.5px] text-dim">
                  Claude makes {game.maker.label} for {game.name}: it reads the game’s files, looks up how its mods work, writes the mod, and you install it with one click.
                </div>
              </div>
              <div className="flex flex-wrap justify-center gap-2">
                {(IDEAS[game.maker.kind ?? 'files'] ?? []).map((i) => (
                  <button key={i} type="button" className="rounded-full border border-line bg-surface-2/60 px-3 py-1.5 text-[12px] text-dim hover:border-accent/50 hover:text-fg" onClick={() => setText(i)}>
                    {i}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            <Conversation view={view} live={live} note={note} />
          )}
        </div>
        {p?.status === 'waiting' && p.approval && (
          <div className="mx-4 mb-3">
            <Callout
              tone="warn"
              title="Claude asks"
              action={
                <div className="flex gap-2">
                  <Button size="sm" variant="primary" icon={Check} onClick={() => void api.maker.approve(p.id, true)}>
                    Allow
                  </Button>
                  <Button size="sm" variant="ghost" icon={X} onClick={() => void api.maker.approve(p.id, false)}>
                    No
                  </Button>
                </div>
              }
            >
              {p.approval.what}
            </Callout>
          </div>
        )}
        {p?.error && p.status === 'error' && <Callout tone="bad" className="mx-4 mb-3">{p.error}</Callout>}
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
            placeholder={current === 'new' || !view ? 'Describe the mod…' : 'Tell Claude what happened, or what to change…'}
            className="min-h-[44px] flex-1 resize-none"
            aria-label="Message to Claude"
            disabled={working || sending}
          />
          {working ? (
            <Button variant="secondary" icon={CircleStop} onClick={() => current && void api.maker.stop(current)}>
              Stop
            </Button>
          ) : (
            <Button variant="primary" icon={Send} disabled={!text.trim() || sending} onClick={() => void send()}>
              Send
            </Button>
          )}
        </div>
      </Card>

      <div>{view && current !== 'new' ? <ProjectSide view={view} onChanged={loadView} /> : <Callout tone="info">Claude only writes in the mod’s own folder and reads the game without changing it. Mods it makes are for single-player and offline play.</Callout>}</div>
    </div>
  );
}
