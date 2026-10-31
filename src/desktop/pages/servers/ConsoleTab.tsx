// The server's console as it happens, and a command line (with history and
// common commands one click away).

import type { SrvLine, SrvView } from '@shared/types';
import { ArrowDown, Send } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { TextInput } from '../../components/ui/Form';
import { cx } from '../../lib/cx';
import { useEvent } from '../../lib/hooks';
import { run } from './ServerView';

const QUICK = ['list', 'save-all', 'time set day', 'weather clear', 'difficulty', 'seed', 'tps'];

const levelClass: Record<SrvLine['level'], string> = {
  info: 'text-fg/85',
  warn: 'text-warn',
  error: 'text-bad',
  chat: 'text-accent',
  cmd: 'text-info font-semibold',
};

export function ConsoleTab({ s }: { s: SrvView }) {
  const [lines, setLines] = useState<SrvLine[]>([]);
  const [text, setText] = useState('');
  const [history, setHistory] = useState<string[]>([]);
  const [hi, setHi] = useState(-1);
  const [follow, setFollow] = useState(true);
  const box = useRef<HTMLDivElement>(null);
  const last = useRef<number | null>(null);

  const load = useCallback(
    (all = false) => {
      void api.servers.console(s.id, all ? null : last.current).then((l) => {
        if (!l.length) return;
        last.current = l[l.length - 1].n;
        setLines((cur) => (all ? l : [...cur, ...l]).slice(-3000));
      });
    },
    [s.id],
  );
  useEffect(() => {
    last.current = null;
    setLines([]);
    load(true);
  }, [load]);
  useEvent<{ id: string }>('mcserver:console', (e) => e.id === s.id && load());

  useEffect(() => {
    if (follow && box.current) box.current.scrollTop = box.current.scrollHeight;
  }, [lines, follow]);

  const send = async (cmd: string) => {
    const c = cmd.trim();
    if (!c) return;
    if (await run('Not sent', () => api.servers.command(s.id, c))) {
      setHistory((h) => [c, ...h.filter((x) => x !== c)].slice(0, 50));
      setHi(-1);
      setText('');
      setFollow(true);
    }
  };

  return (
    <Card className="flex h-[calc(100vh-260px)] min-h-[360px] flex-col overflow-hidden">
      <div
        ref={box}
        onScroll={(e) => {
          const el = e.currentTarget;
          setFollow(el.scrollHeight - el.scrollTop - el.clientHeight < 40);
        }}
        className="relative flex-1 overflow-y-auto bg-[#0e1014] px-4 py-3 font-mono text-[12px] leading-[1.55]"
      >
        {lines.length === 0 && <div className="text-faint">{s.state === 'stopped' ? 'The server is off. Its console shows here when it runs.' : 'Waiting for the server…'}</div>}
        {lines.map((l) => (
          <div key={l.n} className={cx('whitespace-pre-wrap break-words', levelClass[l.level], l.text.startsWith('[OmniHub]') && 'text-accent-2')}>
            {l.text}
          </div>
        ))}
      </div>
      {!follow && (
        <div className="pointer-events-none relative">
          <div className="pointer-events-auto absolute -top-12 right-4">
            <IconButton icon={ArrowDown} label="Latest" variant="secondary" onClick={() => setFollow(true)} />
          </div>
        </div>
      )}
      <div className="space-y-2 border-t border-line px-3 py-2.5">
        <div className="flex flex-wrap gap-1.5">
          {QUICK.map((q) => (
            <Button key={q} size="xs" variant="subtle" disabled={s.state !== 'running'} onClick={() => void send(q)}>
              {q}
            </Button>
          ))}
        </div>
        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            void send(text);
          }}
        >
          <TextInput
            value={text}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'ArrowUp' && history.length) {
                e.preventDefault();
                const n = Math.min(hi + 1, history.length - 1);
                setHi(n);
                setText(history[n]);
              } else if (e.key === 'ArrowDown') {
                e.preventDefault();
                const n = hi - 1;
                setHi(Math.max(-1, n));
                setText(n >= 0 ? history[n] : '');
              }
            }}
            disabled={s.state !== 'running' && s.state !== 'starting'}
            placeholder={s.state === 'running' ? 'A command, like “give Steve diamond 5” or “time set day”' : 'Start the server to type commands'}
            className="flex-1 font-mono"
            aria-label="Command"
          />
          <Button type="submit" variant="primary" icon={Send} disabled={!text.trim() || (s.state !== 'running' && s.state !== 'starting')}>
            Send
          </Button>
        </form>
      </div>
    </Card>
  );
}
