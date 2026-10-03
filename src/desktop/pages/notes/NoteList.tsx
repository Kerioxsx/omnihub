import { formatRelative } from '@shared/format';
import type { Note, NoteKind } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { Lightbulb, Pin, Send, StickyNote } from 'lucide-react';
import { Skeleton } from '../../components/ui/Card';
import { SearchInput, Segmented } from '../../components/ui/Form';
import { EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { plainSnippet } from '../../lib/util';
import { NOTE_COLORS } from './colors';

export function NoteList({
  kind,
  onKind,
  notes,
  loading,
  selected,
  onSelect,
  query,
  onQuery,
  tags,
  tag,
  onTag,
}: {
  kind: NoteKind;
  onKind: (k: NoteKind) => void;
  notes: Note[];
  loading: boolean;
  selected: string | null;
  onSelect: (id: string) => void;
  query: string;
  onQuery: (q: string) => void;
  tags: [string, number][];
  tag: string | null;
  onTag: (t: string | null) => void;
}) {
  return (
    <div className="card flex min-h-0 flex-col overflow-hidden">
      <div className="space-y-2.5 border-b border-line p-3">
        <Segmented
          label="Kind"
          value={kind}
          onChange={onKind}
          className="!flex w-full [&>button]:flex-1"
          options={[
            { value: 'note', label: 'Notes', icon: StickyNote },
            { value: 'idea', label: 'Ideas for Claude', icon: Lightbulb },
          ]}
        />
        <SearchInput value={query} onChange={onQuery} placeholder={kind === 'idea' ? 'Search ideas' : 'Search notes'} inputSize="sm" aria-label="Search notes" />
        {tags.length > 0 && (
          <div className="fade-x -mx-3 flex gap-1 overflow-x-auto px-3 pb-0.5 [scrollbar-width:none]">
            {tags.slice(0, 14).map(([t, n]) => (
              <button key={t} type="button" onClick={() => onTag(tag === t ? null : t)} aria-pressed={tag === t} className={cx('h-6 shrink-0 whitespace-nowrap rounded-full border px-2 text-[11.5px] transition-colors', tag === t ? 'border-accent/40 bg-accent-soft text-accent' : 'border-line text-dim hover:text-fg')}>
                #{t} <span className="text-faint">{n}</span>
              </button>
            ))}
          </div>
        )}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto p-1.5" role="listbox" aria-label={kind === 'idea' ? 'Ideas' : 'Notes'}>
        {loading && !notes.length ? (
          Array.from({ length: 6 }, (_, i) => <Skeleton key={i} className="m-1.5 h-16" />)
        ) : notes.length === 0 ? (
          <EmptyState compact icon={kind === 'idea' ? Lightbulb : StickyNote} title={query || tag ? 'Nothing matches' : kind === 'idea' ? 'No ideas yet' : 'No notes yet'} description={query || tag ? undefined : kind === 'idea' ? 'Write down an idea and send it to Claude.' : 'Create your first note.'} />
        ) : (
          <AnimatePresence initial={false}>
            {notes.map((n) => {
              const active = n.id === selected;
              const color = n.color ? NOTE_COLORS[n.color] : null;
              return (
                <motion.button
                  layout="position"
                  key={n.id}
                  type="button"
                  role="option"
                  aria-selected={active}
                  initial={{ opacity: 0, y: -4 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, height: 0 }}
                  transition={{ type: 'spring', stiffness: 400, damping: 34 }}
                  onClick={() => onSelect(n.id)}
                  className={cx('relative mb-0.5 block w-full overflow-hidden rounded-xl px-3 py-2.5 text-left transition-colors', active ? 'bg-surface-3' : 'hover:bg-surface-2')}
                >
                  {color && <span className="absolute inset-y-2.5 left-0 w-[3px] rounded-r-full" style={{ background: color }} />}
                  <div className="flex items-center gap-1.5">
                    {n.pinned && <Pin size={12} className="shrink-0 text-accent" aria-label="Pinned" />}
                    <span className={cx('truncate text-[13.5px] font-medium', n.title ? 'text-fg' : 'text-faint')}>{n.title || 'Untitled'}</span>
                    {n.exportedAt && <Send size={11} className="ml-auto shrink-0 text-accent" aria-label="Sent to Claude" />}
                  </div>
                  <div className="mt-0.5 line-clamp-2 text-[12px] leading-snug text-faint">{plainSnippet(n.body, 140) || 'No content'}</div>
                  <div className="mt-1 flex items-center gap-2 text-[11px] text-faint">
                    <span>{formatRelative(n.updated)}</span>
                    {n.tags.slice(0, 2).map((t) => (
                      <span key={t} className="text-dim">
                        #{t}
                      </span>
                    ))}
                  </div>
                </motion.button>
              );
            })}
          </AnimatePresence>
        )}
      </div>
    </div>
  );
}
