// Ideas: notes of the Idea kind, saved from any chat message or typed here,
// with a quick way to ask Claude about one.

import type { Note } from '@shared/types';
import { formatRelative } from '@shared/format';
import { Lightbulb, MessageSquarePlus, NotebookPen, Plus } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { Textarea } from '../../components/ui/Form';
import { EmptyState } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { navigate } from '../../lib/router';
import { renderMarkdown } from '../../lib/util';
import { toast } from '../../state/toasts';

export function IdeasTab() {
  const [ideas, setIdeas] = useState<Note[] | null>(null);
  const [text, setText] = useState('');
  const load = useCallback(() => void api.ideas.list().then(setIdeas, (e: unknown) => toast.error('Can’t list the ideas', errorText(e))), []);
  useEffect(load, [load]);
  useEvent('notes:changed', load);

  const save = async () => {
    const t = text.trim();
    if (!t) return;
    try {
      await api.ideas.save(t);
      setText('');
      load();
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  };
  const ask = async (n: Note) => {
    try {
      const c = await api.chats.create('claude', `Here's an idea of mine. Help me think it through: what's good about it, what could go wrong, and the first three steps.\n\n${n.title}\n\n${n.body}`);
      navigate('chats', { source: c.source, id: c.id });
    } catch (e) {
      toast.error('Not started', errorText(e));
    }
  };

  return (
    <div className="mx-auto max-w-3xl space-y-4">
      <Card className="flex items-end gap-2 px-4 py-3">
        <Textarea
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) void save();
          }}
          rows={2}
          placeholder="Write an idea down… (Ctrl+Enter saves it)"
          className="flex-1 resize-none"
          aria-label="New idea"
        />
        <Button variant="primary" icon={Plus} disabled={!text.trim()} onClick={() => void save()}>
          Save idea
        </Button>
      </Card>
      {ideas && ideas.length === 0 && (
        <Card>
          <EmptyState icon={Lightbulb} title="No ideas yet" description="Save any chat message as an idea with the bulb next to it, or write one above. They’re notes, so they also show in Notes (and in your Claude folder if ideas go there)." />
        </Card>
      )}
      <div className="space-y-3">
        {(ideas ?? []).map((n) => (
          <Card key={n.id} className="space-y-2 px-5 py-4">
            <div className="flex items-start gap-3">
              <div className="mt-0.5 grid h-8 w-8 shrink-0 place-items-center rounded-full bg-warn/12 text-warn">
                <Lightbulb size={16} />
              </div>
              <div className="min-w-0 flex-1">
                <div className="text-[14px] font-semibold text-fg">{n.title}</div>
                <div className="text-[11.5px] text-faint">{formatRelative(n.updated)}</div>
              </div>
              <Button size="sm" variant="ghost" icon={MessageSquarePlus} onClick={() => void ask(n)}>
                Ask Claude
              </Button>
              <Button size="sm" variant="ghost" icon={NotebookPen} onClick={() => navigate('notes', { tab: 'idea', id: n.id })}>
                Open in Notes
              </Button>
            </div>
            {n.body.trim() && n.body.trim() !== n.title && <div className="markdown pl-11 text-[13px] text-dim" dangerouslySetInnerHTML={{ __html: renderMarkdown(n.body) }} />}
          </Card>
        ))}
      </div>
    </div>
  );
}
