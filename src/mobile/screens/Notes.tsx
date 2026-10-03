// Notes & ideas: list with tabs and search, Markdown reader, composer.

import { useCallback, useEffect, useMemo, useState } from 'react';
import { marked } from 'marked';
import DOMPurify from 'dompurify';
import { Plus, Search, X, Lightbulb, NotebookPen, Sparkles, Pin, Tag, Send } from 'lucide-react';
import type { Note, NoteKind } from '@shared/types';
import { formatDateTime, formatRelative } from '@shared/format';
import { client } from '../client';
import { toast } from '../state';
import { PullToRefresh } from '../ui/PullToRefresh';
import { Sheet } from '../ui/Sheet';
import { Button, Empty, ErrorState, ListSkeleton, Segmented, Switch } from '../ui/common';
import { SubHeader } from './More';
import { useEvent } from '../lib/events';
import { cx, errorMessage, useDebounced } from '../lib/util';

DOMPurify.addHook('afterSanitizeAttributes', (node) => {
  if (node.tagName === 'A') {
    node.setAttribute('target', '_blank');
    node.setAttribute('rel', 'noopener noreferrer');
  }
});

export function renderMarkdown(md: string): string {
  const html = marked.parse(md, { async: false, gfm: true, breaks: true }) as string;
  return DOMPurify.sanitize(html, { USE_PROFILES: { html: true } });
}

function excerpt(md: string): string {
  return md
    .replace(/```[\s\S]*?```/g, ' ')
    .replace(/[#>*_`~\-[\]()!]/g, '')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, 160);
}

type Filter = 'all' | NoteKind;

export function NotesPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const [filter, setFilter] = useState<Filter>('all');
  const [q, setQ] = useState('');
  const dq = useDebounced(q, 250);
  const [notes, setNotes] = useState<Note[] | null>(null);
  const [claudeFolder, setClaudeFolder] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState<Note | null>(null);
  const [composing, setComposing] = useState(false);

  const load = useCallback(async () => {
    try {
      const r = await client.notes(filter === 'all' ? undefined : filter, dq);
      setNotes(r.notes);
      setClaudeFolder(r.claudeFolder);
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, [filter, dq]);

  useEffect(() => {
    if (active) load();
  }, [active, load]);
  useEvent('notes:changed', () => load());

  const sorted = useMemo(() => (notes ? [...notes].sort((a, b) => Number(b.pinned) - Number(a.pinned) || b.updated - a.updated) : null), [notes]);

  return (
    <div className="relative h-full">
      <PullToRefresh onRefresh={load}>
        <SubHeader title="Notes" subtitle="OmniHub on the PC" onBack={onBack} />
        <div className="px-safe pb-tabbar">
          <Segmented
            value={filter}
            onChange={setFilter}
            options={[
              { value: 'all', label: 'All' },
              { value: 'note', label: <><NotebookPen size={15} /> Notes</> },
              { value: 'idea', label: <><Lightbulb size={15} /> Ideas</> },
            ]}
          />
          <div className="relative mt-3">
            <Search size={17} className="pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-faint" />
            <input className="field h-11 py-0 pl-10 pr-10" type="search" placeholder="Search notes" value={q} onChange={(e) => setQ(e.target.value)} enterKeyHint="search" />
            {q && (
              <button aria-label="Clear search" onClick={() => setQ('')} className="absolute right-2 top-1/2 grid h-8 w-8 -translate-y-1/2 place-items-center rounded-full text-faint">
                <X size={16} />
              </button>
            )}
          </div>
          <div className="mt-3">
            {error && !notes ? (
              <ErrorState message={error} onRetry={load} />
            ) : !sorted ? (
              <ListSkeleton rows={5} thumb={false} />
            ) : sorted.length === 0 ? (
              <Empty
                icon={filter === 'idea' ? <Lightbulb size={28} /> : <NotebookPen size={28} />}
                title={q ? 'No matches' : filter === 'idea' ? 'No ideas yet' : 'No notes yet'}
                body={q ? `Nothing matches “${q}”.` : 'Capture one with the + button — it syncs to OmniHub on the PC.'}
              />
            ) : (
              <div className="space-y-2.5">
                {sorted.map((n) => (
                  <button key={n.id} onClick={() => setOpen(n)} className="press card block w-full p-4 text-left">
                    <div className="flex items-start gap-2">
                      <div className={cx('mt-0.5 grid h-7 w-7 shrink-0 place-items-center rounded-lg', n.kind === 'idea' ? 'bg-amber-500/15 text-amber-400' : 'bg-violet-500/15 text-violet-400')}>
                        {n.kind === 'idea' ? <Lightbulb size={15} /> : <NotebookPen size={15} />}
                      </div>
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-1.5">
                          <div className="truncate text-[16px] font-semibold">{n.title || 'Untitled'}</div>
                          {n.pinned && <Pin size={13} className="shrink-0 text-accent" />}
                        </div>
                        {n.body && <div className="mt-0.5 line-clamp-2 text-sm text-dim">{excerpt(n.body)}</div>}
                        <div className="mt-2 flex flex-wrap items-center gap-1.5 text-xs text-faint">
                          <span>{formatRelative(n.updated)}</span>
                          {n.exportedPath && (
                            <span className="inline-flex items-center gap-1 rounded-full bg-accent-soft px-2 py-0.5 font-semibold text-accent">
                              <Sparkles size={11} /> Claude
                            </span>
                          )}
                          {n.tags.slice(0, 3).map((t) => (
                            <span key={t} className="rounded-full bg-surface-2 px-2 py-0.5 font-medium text-dim">
                              #{t}
                            </span>
                          ))}
                        </div>
                      </div>
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      </PullToRefresh>

      <button
        aria-label="New note"
        onClick={() => setComposing(true)}
        className="press grad-bg fixed right-[max(16px,var(--safe-right))] z-30 grid h-[58px] w-[58px] place-items-center rounded-[20px] text-white shadow-[0_14px_34px_-10px_var(--accent-glow)] bottom-tabbar"
      >
        <Plus size={28} strokeWidth={2.4} />
      </button>

      <NoteSheet note={open} onClose={() => setOpen(null)} />
      <ComposeSheet
        open={composing}
        onClose={() => setComposing(false)}
        defaultKind={filter === 'note' ? 'note' : 'idea'}
        claudeFolder={claudeFolder}
        onSaved={() => {
          setComposing(false);
          load();
        }}
      />
    </div>
  );
}

function NoteSheet({ note, onClose }: { note: Note | null; onClose: () => void }) {
  const [full, setFull] = useState<Note | null>(note);
  useEffect(() => {
    if (!note) return;
    setFull(note);
    client.note(note.id).then(setFull, () => undefined);
  }, [note]);
  const n = full;
  const html = useMemo(() => (n?.body ? renderMarkdown(n.body) : ''), [n?.body]);
  return (
    <Sheet open={!!note} onClose={onClose} title={n?.title || 'Untitled'} subtitle={n ? `${n.kind === 'idea' ? 'Idea' : 'Note'} · updated ${formatDateTime(n.updated)}` : undefined}>
      {n && (
        <div className="pb-4">
          {(n.tags.length > 0 || n.exportedPath) && (
            <div className="mb-4 flex flex-wrap gap-1.5">
              {n.exportedPath && (
                <span className="inline-flex items-center gap-1 rounded-full bg-accent-soft px-2.5 py-1 text-xs font-semibold text-accent">
                  <Sparkles size={12} /> In Claude folder
                </span>
              )}
              {n.tags.map((t) => (
                <span key={t} className="inline-flex items-center gap-1 rounded-full bg-surface-2 px-2.5 py-1 text-xs font-medium text-dim">
                  <Tag size={11} /> {t}
                </span>
              ))}
            </div>
          )}
          {html ? <div className="md" dangerouslySetInnerHTML={{ __html: html }} /> : <div className="text-dim">This note is empty.</div>}
          {n.exportedPath && <div className="selectable mt-5 break-all rounded-xl bg-surface px-3 py-2 font-mono text-xs text-faint">{n.exportedPath}</div>}
        </div>
      )}
    </Sheet>
  );
}

function ComposeSheet({ open, onClose, defaultKind, claudeFolder, onSaved }: { open: boolean; onClose: () => void; defaultKind: NoteKind; claudeFolder: boolean; onSaved: () => void }) {
  const [kind, setKind] = useState<NoteKind>(defaultKind);
  const [title, setTitle] = useState('');
  const [body, setBody] = useState('');
  const [tags, setTags] = useState('');
  const [toClaude, setToClaude] = useState(true);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (open) setKind(defaultKind);
  }, [open, defaultKind]);

  const save = async () => {
    setBusy(true);
    try {
      const n = await client.createNote({
        kind,
        title: title.trim(),
        body,
        tags: tags
          .split(/[,\s]+/)
          .map((t) => t.replace(/^#/, '').trim())
          .filter(Boolean),
        sendToClaude: claudeFolder && toClaude,
      });
      setTitle('');
      setBody('');
      setTags('');
      toast.success(n.exportedPath ? 'Saved and sent to Claude' : kind === 'idea' ? 'Idea saved' : 'Note saved', n.exportedPath ?? undefined);
      onSaved();
    } catch (e) {
      toast.error("Couldn't save", errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Sheet
      open={open}
      onClose={onClose}
      title={kind === 'idea' ? 'New idea' : 'New note'}
      footer={
        <Button className="w-full" loading={busy} disabled={!title.trim() && !body.trim()} onClick={save} icon={claudeFolder && toClaude ? <Send size={18} /> : <NotebookPen size={18} />}>
          {claudeFolder && toClaude ? 'Save & send to Claude' : 'Save'}
        </Button>
      }
    >
      <div className="space-y-3 pb-2">
        <Segmented
          value={kind}
          onChange={setKind}
          options={[
            { value: 'idea', label: <><Lightbulb size={15} /> Idea</> },
            { value: 'note', label: <><NotebookPen size={15} /> Note</> },
          ]}
        />
        <input className="field font-semibold" placeholder="Title" value={title} onChange={(e) => setTitle(e.target.value)} maxLength={200} />
        <textarea className="field min-h-[160px] resize-none leading-relaxed" placeholder="Write in Markdown…" value={body} onChange={(e) => setBody(e.target.value)} />
        <input className="field" placeholder="Tags (comma separated)" value={tags} onChange={(e) => setTags(e.target.value)} autoCapitalize="off" />
        <div className="flex items-center gap-3 rounded-2xl bg-surface px-4 py-3">
          <Sparkles size={18} className="shrink-0 text-accent" />
          <div className="min-w-0 flex-1">
            <div className="text-[15px] font-medium">Send to Claude folder</div>
            <div className="text-xs text-faint">{claudeFolder ? 'Exported as Markdown for Claude Code' : 'Set a Claude folder in OmniHub on the PC first'}</div>
          </div>
          <Switch on={claudeFolder && toClaude} onChange={setToClaude} disabled={!claudeFolder} label="Send to Claude folder" />
        </div>
      </div>
    </Sheet>
  );
}

