import type { Note, NoteKind } from '@shared/types';
import { Bug, CalendarDays, CheckSquare, LayoutTemplate, Lightbulb, type LucideIcon, NotebookPen, Users, Wrench } from 'lucide-react';
import { api } from '../../api';
import { Modal } from '../../components/ui/Overlay';
import { useAsync } from '../../lib/hooks';
import { plainSnippet } from '../../lib/util';
import { TEMPLATES } from './templates';

const ICONS: Record<string, LucideIcon> = { meeting: Users, daily: CalendarDays, todo: CheckSquare, weekly: CalendarDays, bug: Bug, 'claude-feature': Lightbulb, 'claude-bugfix': Wrench };

export interface TemplateChoice {
  kind: NoteKind;
  title: string;
  body: string;
  tags: string[];
}

/** Start a note from a built-in template or from your own (notes tagged #template). */
export function TemplatePicker({ open, onClose, onPick }: { open: boolean; onClose: () => void; onPick: (t: TemplateChoice) => void }) {
  const mine = useAsync(() => (open ? api.notes.list({ tag: 'template' }) : Promise.resolve([] as Note[])), [open]);
  const card = (key: string, Icon: LucideIcon, name: string, description: string, choice: TemplateChoice, idea: boolean) => (
    <button key={key} type="button" onClick={() => onPick(choice)} className="card card-interactive flex items-start gap-3 p-3.5 text-left">
      <span className={idea ? 'flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-warn/15 text-warn' : 'flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-accent-soft text-accent'}>
        <Icon size={17} aria-hidden />
      </span>
      <span className="min-w-0">
        <span className="block text-[13.5px] font-semibold text-fg">{name}</span>
        <span className="mt-0.5 block text-[12px] leading-snug text-faint">{description}</span>
      </span>
    </button>
  );
  return (
    <Modal open={open} onClose={onClose} title="New from a template" description="Pick a starting point; dates fill themselves in." icon={LayoutTemplate} size="lg">
      <div className="space-y-4 pb-2">
        <div className="grid grid-cols-2 gap-2.5">
          {TEMPLATES.map((t) => card(t.id, ICONS[t.id] ?? NotebookPen, t.name, t.description, { kind: t.kind, title: t.title, body: t.body, tags: t.tags }, t.kind === 'idea'))}
        </div>
        <div>
          <div className="mb-2 text-[11px] font-semibold uppercase tracking-wider text-faint">Your templates</div>
          {mine.data && mine.data.length > 0 ? (
            <div className="grid grid-cols-2 gap-2.5">
              {mine.data.map((n) =>
                card(n.id, LayoutTemplate, n.title || 'Untitled template', plainSnippet(n.body, 90) || 'Empty note', { kind: n.kind, title: n.title.replace(/\s*\(template\)$/i, ''), body: n.body, tags: n.tags.filter((t) => t !== 'template') }, n.kind === 'idea'),
              )}
            </div>
          ) : (
            <div className="rounded-xl border border-dashed border-line px-4 py-3 text-[12.5px] text-faint">Use “Save as template” on any note (or tag it #template) to see it here.</div>
          )}
        </div>
      </div>
    </Modal>
  );
}
