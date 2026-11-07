// "Ask your PC": a question in plain words, answered from what OmniHub
// measures right now (busy programs, network, temperatures, space, health).
// With a local model (Chats → Local AI) the answer can also be put in a
// sentence, from those same facts.

import type { AskReply } from '@shared/types';
import { ArrowRight, MessageCircleQuestion, Sparkles } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { Switch, TextInput } from '../../components/ui/Form';
import { useStoredState } from '../../lib/hooks';
import { navigate } from '../../lib/router';
import { toast } from '../../state/toasts';

const EXAMPLES = ['Why is my PC slow?', 'What’s using my internet?', 'Is it running hot?', 'Am I running out of space?'];

export function AskPc() {
  const [q, setQ] = useState('');
  const [busy, setBusy] = useState(false);
  const [reply, setReply] = useState<AskReply | null>(null);
  const [words, setWords] = useStoredState('home.askWords', false);
  const ask = async (question = q) => {
    if (!question.trim()) return;
    setQ(question);
    setBusy(true);
    try {
      setReply(await api.askPc(question, words));
    } catch (e) {
      toast.error('No answer', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Card className="p-4">
      <form
        className="flex items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void ask();
        }}
      >
        <MessageCircleQuestion size={18} className="shrink-0 text-accent" aria-hidden />
        <TextInput value={q} onChange={(e) => setQ(e.target.value)} placeholder="Ask your PC… (why is it slow? what’s using my internet?)" className="flex-1" aria-label="Ask your PC" maxLength={500} />
        <Button type="submit" variant="primary" loading={busy} disabled={!q.trim()}>
          Ask
        </Button>
      </form>
      {!reply && (
        <div className="mt-2.5 flex flex-wrap items-center gap-1.5">
          {EXAMPLES.map((x) => (
            <button key={x} type="button" onClick={() => void ask(x)} className="rounded-full border border-line bg-surface-2/60 px-2.5 py-1 text-[12px] text-dim hover:border-accent/50 hover:text-fg">
              {x}
            </button>
          ))}
          <span className="ml-auto flex items-center gap-2 text-[12px] text-faint">
            <Sparkles size={12} aria-hidden /> Explain with local AI
            <Switch checked={words} onChange={setWords} label="Explain with local AI" />
          </span>
        </div>
      )}
      {reply && (
        <div className="mt-3 space-y-3">
          {reply.words && <p className="rounded-xl bg-accent-soft/50 px-3.5 py-2.5 text-[13.5px] text-fg">{reply.words}</p>}
          {reply.answers.map((a) => (
            <div key={a.title} className="rounded-xl border border-line bg-surface-2/40 p-3.5">
              <div className="flex items-center justify-between gap-3">
                <div className="text-[14px] font-semibold text-fg">{a.title}</div>
                {a.route && (
                  <Button size="xs" variant="ghost" iconRight={ArrowRight} onClick={() => navigate(a.route as string)}>
                    Open
                  </Button>
                )}
              </div>
              <ul className="mt-1.5 space-y-0.5 text-[12.5px] text-dim">
                {a.points.map((p) => (
                  <li key={p}>· {p}</li>
                ))}
              </ul>
              {a.tips.length > 0 && <div className="mt-2 text-[12.5px] text-fg">{a.tips.join(' ')}</div>}
            </div>
          ))}
          <div className="flex items-center justify-between text-[11.5px] text-faint">
            <span>From what OmniHub measures right now; nothing is sent anywhere.</span>
            <button type="button" className="underline" onClick={() => setReply(null)}>
              Ask something else
            </button>
          </div>
        </div>
      )}
    </Card>
  );
}
