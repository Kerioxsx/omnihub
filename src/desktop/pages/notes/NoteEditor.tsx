import { formatDateTime, formatRelative } from '@shared/format';
import type { Note } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import type { LucideIcon } from 'lucide-react';
import { Bold, Check, CircleAlert, Code, Columns2, Eye, FolderOpen, Heading, Italic, Link, List, ListChecks, LoaderCircle, Palette, PenLine, Pin, PinOff, Send, Sparkles, Tag, Trash, X } from 'lucide-react';
import { type KeyboardEvent, useEffect, useMemo, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Segmented } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useNow, useStoredState } from '../../lib/hooks';
import { renderMarkdown } from '../../lib/util';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';
import { NOTE_COLORS } from './colors';
import { type MdAction, applyMd } from './mdEdit';

export interface Draft {
  title: string;
  body: string;
  tags: string[];
  pinned: boolean;
  color: string | null;
}

type Mode = 'edit' | 'split' | 'preview';

const TOOLS: [MdAction, LucideIcon, string][] = [
  ['bold', Bold, 'Bold (Ctrl+B)'],
  ['italic', Italic, 'Italic (Ctrl+I)'],
  ['heading', Heading, 'Heading'],
  ['list', List, 'Bulleted list'],
  ['checklist', ListChecks, 'Checklist'],
  ['code', Code, 'Code'],
  ['link', Link, 'Link (Ctrl+K)'],
];

export function NoteEditor({
  note,
  draft,
  onDraft,
  saving,
  saveError,
  onDelete,
  onExported,
}: {
  note: Note;
  draft: Draft;
  onDraft: (d: Draft) => void;
  saving: 'idle' | 'pending' | 'saving' | 'saved';
  saveError: string | null;
  onDelete: () => void;
  onExported: (n: Note) => void;
}) {
  const [mode, setMode] = useStoredState<Mode>('omnihub.notes.mode', 'split');
  const [tagInput, setTagInput] = useState('');
  const [colorOpen, setColorOpen] = useState(false);
  const [sending, setSending] = useState(false);
  const area = useRef<HTMLTextAreaElement>(null);
  const titleRef = useRef<HTMLInputElement>(null);
  const claudeFolder = useSettings((s) => s.settings?.notes.claudeFolder ?? null);
  const updateSettings = useSettings((s) => s.update);
  useNow(30000);
  const html = useMemo(() => renderMarkdown(draft.body || '*Nothing here yet.*'), [draft.body]);

  useEffect(() => {
    if (!note.title && !note.body) titleRef.current?.focus();
  }, [note.id]);

  const apply = (action: MdAction) => {
    const el = area.current;
    if (!el) return;
    const r = applyMd(action, draft.body, el.selectionStart, el.selectionEnd);
    onDraft({ ...draft, body: r.value });
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(r.start, r.end);
    });
  };

  const onKey = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (!(e.ctrlKey || e.metaKey)) return;
    const k = e.key.toLowerCase();
    if (k === 'b' || k === 'i' || k === 'k') {
      e.preventDefault();
      e.stopPropagation();
      apply(k === 'b' ? 'bold' : k === 'i' ? 'italic' : 'link');
    }
  };

  const addTag = () => {
    const t = tagInput.trim().replace(/^#/, '').toLowerCase();
    setTagInput('');
    if (t && !draft.tags.includes(t)) onDraft({ ...draft, tags: [...draft.tags, t] });
  };

  const pickFolder = async () => {
    const folder = await api.app.pickFolder('Choose the Claude ideas folder');
    if (folder) await updateSettings({ notes: { claudeFolder: folder } });
    return folder;
  };

  const send = async () => {
    setSending(true);
    try {
      if (!claudeFolder && !(await pickFolder())) return;
      const n = await api.notes.export(note.id);
      onExported(n);
      toast.success(note.exportedAt ? 'Updated for Claude' : 'Sent to Claude', n.exportedPath ?? undefined);
    } catch (e) {
      toast.error('Could not send to Claude', errorText(e));
    } finally {
      setSending(false);
    }
  };

  const isIdea = note.kind === 'idea';

  return (
    <div className="card flex h-full min-h-0 flex-col overflow-hidden">
      <div className="flex items-center gap-2 px-5 pb-2 pt-4">
        <input
          ref={titleRef}
          value={draft.title}
          onChange={(e) => onDraft({ ...draft, title: e.target.value })}
          placeholder={isIdea ? 'Idea title' : 'Untitled'}
          aria-label="Title"
          className="min-w-0 flex-1 bg-transparent font-display text-[21px] font-semibold tracking-tight text-fg outline-none placeholder:text-faint"
        />
        <SaveState state={saving} error={saveError} />
        <IconButton icon={draft.pinned ? PinOff : Pin} label={draft.pinned ? 'Unpin' : 'Pin to top'} active={draft.pinned} onClick={() => onDraft({ ...draft, pinned: !draft.pinned })} />
        <div className="relative">
          <IconButton icon={Palette} label="Colour" onClick={() => setColorOpen(!colorOpen)} active={colorOpen} />
          <AnimatePresence>
            {colorOpen && (
              <motion.div initial={{ opacity: 0, y: -4, scale: 0.96 }} animate={{ opacity: 1, y: 0, scale: 1 }} exit={{ opacity: 0, scale: 0.96 }} className="glass absolute right-0 top-10 z-20 flex gap-1.5 rounded-xl border border-line-strong p-2 shadow-xl" onMouseLeave={() => setColorOpen(false)}>
                <button type="button" aria-label="No colour" onClick={() => (onDraft({ ...draft, color: null }), setColorOpen(false))} className={cx('flex h-6 w-6 items-center justify-center rounded-full border border-line-strong text-faint', !draft.color && 'ring-2 ring-accent')}>
                  <X size={12} />
                </button>
                {Object.entries(NOTE_COLORS).map(([name, c]) => (
                  <button key={name} type="button" aria-label={name} onClick={() => (onDraft({ ...draft, color: name }), setColorOpen(false))} className={cx('h-6 w-6 rounded-full transition-transform hover:scale-110', draft.color === name && 'ring-2 ring-white/80 ring-offset-2 ring-offset-[var(--bg-elev)]')} style={{ background: c }} />
                ))}
              </motion.div>
            )}
          </AnimatePresence>
        </div>
        <IconButton icon={Trash} label="Delete" variant="danger" onClick={onDelete} />
      </div>
        <div className="flex min-w-0 flex-wrap items-center gap-1.5 px-5 pb-3">
          {draft.tags.map((t) => (
            <span key={t} className="inline-flex h-6 items-center gap-0.5 rounded-full bg-surface-3 pl-2 pr-0.5 text-[11.5px] text-dim">
              #{t}
              <button type="button" aria-label={`Remove tag ${t}`} onClick={() => onDraft({ ...draft, tags: draft.tags.filter((x) => x !== t) })} className="flex h-5 w-5 items-center justify-center rounded-full hover:bg-surface-2 hover:text-fg">
                <X size={10} />
              </button>
            </span>
          ))}
          <span className="inline-flex items-center gap-1 text-faint">
            <Tag size={12} aria-hidden />
            <input
              value={tagInput}
              onChange={(e) => setTagInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ',') {
                  e.preventDefault();
                  addTag();
                } else if (e.key === 'Backspace' && !tagInput && draft.tags.length) onDraft({ ...draft, tags: draft.tags.slice(0, -1) });
              }}
              onBlur={addTag}
              placeholder="Add tag"
              aria-label="Add tag"
              className="w-24 bg-transparent text-[12px] text-fg outline-none placeholder:text-faint"
            />
          </span>
        </div>

      <div className="flex flex-wrap items-center gap-2 border-y border-line px-4 py-2">
        <div className="flex items-center gap-0.5">
          {TOOLS.map(([a, I, label]) => (
            <IconButton key={a} icon={I} label={label} size="sm" onClick={() => apply(a)} disabled={mode === 'preview'} />
          ))}
        </div>
        <div className="flex-1" />
        <Segmented
          size="sm"
          label="View"
          value={mode}
          onChange={setMode}
          options={[
            { value: 'edit', label: 'Write', icon: PenLine },
            { value: 'split', label: 'Split', icon: Columns2 },
            { value: 'preview', label: 'Preview', icon: Eye },
          ]}
        />
      </div>

      <div className={cx('grid min-h-0 flex-1', mode === 'split' ? 'grid-cols-2' : 'grid-cols-1')}>
        {mode !== 'preview' && (
          <textarea
            ref={area}
            value={draft.body}
            onChange={(e) => onDraft({ ...draft, body: e.target.value })}
            onKeyDown={onKey}
            placeholder={isIdea ? 'Describe the idea. What problem does it solve? What should Claude build or think about?' : 'Start writing… Markdown works: **bold**, - lists, - [ ] checklists, `code`'}
            aria-label="Note body (Markdown)"
            spellCheck
            className={cx('h-full min-h-0 resize-none bg-transparent px-5 py-4 text-[14px] leading-[1.7] text-fg outline-none placeholder:text-faint', mode === 'split' && 'border-r border-line')}
          />
        )}
        {mode !== 'edit' && <div className="markdown min-h-0 overflow-y-auto px-6 py-4" dangerouslySetInnerHTML={{ __html: html }} />}
      </div>

      {isIdea && (
        <div className="border-t border-line bg-surface/60 px-5 py-3">
          {!claudeFolder ? (
            <Callout tone="accent" icon={FolderOpen} title="Choose where ideas go" action={<Button size="sm" variant="primary" icon={FolderOpen} onClick={() => void pickFolder()}>Choose folder</Button>}>
              “Send to Claude” writes the idea as a Markdown file into a folder Claude can read (e.g. a project folder Claude Code works in).
            </Callout>
          ) : (
            <div className="flex items-center gap-4">
              <div className="min-w-0 flex-1">
                {note.exportedAt ? (
                  <>
                    <div className="flex items-center gap-1.5 text-[13px] text-fg">
                      <Check size={14} className="text-good" aria-hidden />
                      Sent {formatRelative(note.exportedAt)}
                    </div>
                    <div className="mt-0.5 truncate text-[11.5px] text-faint" title={note.exportedPath ?? ''}>
                      Re-sending updates the same file · <span className="font-mono">{note.exportedPath?.split('\\').pop()}</span>
                    </div>
                  </>
                ) : (
                  <>
                    <div className="text-[13px] text-dim">Not sent yet</div>
                    <div className="mt-0.5 truncate font-mono text-[11px] text-faint">→ {claudeFolder}</div>
                  </>
                )}
              </div>
              <motion.div whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.97 }}>
                <Button variant="primary" size="lg" icon={note.exportedAt ? Sparkles : Send} loading={sending} onClick={send} className="shadow-[0_10px_30px_-10px_var(--accent-glow)]" title={note.exportedAt ? `Last sent ${formatDateTime(note.exportedAt)}` : undefined}>
                  {note.exportedAt ? 'Send update to Claude' : 'Send to Claude'}
                </Button>
              </motion.div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function SaveState({ state, error }: { state: 'idle' | 'pending' | 'saving' | 'saved'; error: string | null }) {
  if (error)
    return (
      <span className="flex items-center gap-1 text-[12px] text-bad" title={error}>
        <CircleAlert size={13} /> Not saved
      </span>
    );
  return (
    <AnimatePresence mode="wait">
      {state === 'saving' || state === 'pending' ? (
        <motion.span key="saving" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="flex items-center gap-1 text-[12px] text-faint">
          <LoaderCircle size={12} className="spin" /> Saving…
        </motion.span>
      ) : state === 'saved' ? (
        <motion.span key="saved" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="flex items-center gap-1 text-[12px] text-faint">
          <Check size={12} className="text-good" /> Saved
        </motion.span>
      ) : null}
    </AnimatePresence>
  );
}
