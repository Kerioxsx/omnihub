// Karaoke Studio's lyrics: the line being sung with every word filling in
// as it's sung (Aurora's word fill, copied so Aurora stays as it is), and
// under each word how to say it in your own spelling and its syllables.
// The line before sits above, the next ones below. Breaks show how long
// until the next line. Click a line to jump to it.

import type { LyricLine } from '@shared/types';
import { Lightbulb, Mic } from 'lucide-react';
import { memo, useEffect, useMemo, useRef, useState } from 'react';
import { lineEnd, phaseAt } from '../../../components/aurora/AuroraLyrics';
import { cx } from '../../../lib/cx';
import { guideWord, type Reader, type SongLanguage, type WordGuide } from '../../../lib/karaoke/pron';

export interface ShowOptions {
  say: boolean;
  syllables: boolean;
}

type View = { kind: 'line'; index: number } | { kind: 'break'; to: number; next: number } | { kind: 'intro'; to: number } | { kind: 'end' };

const sameView = (a: View, b: View) => a.kind === b.kind && (a.kind !== 'line' || a.index === (b as { index: number }).index) && (a.kind !== 'break' || a.next === (b as { next: number }).next);

/** What to show now, re-rendering only when it changes (seeks included). */
function useView(time: () => number, lines: LyricLine[]): View {
  const pick = (): View => {
    const p = phaseAt(lines, time());
    return p.kind === 'break' ? { kind: 'break', to: p.to, next: p.next } : p;
  };
  const [view, setView] = useState<View>(pick);
  const ref = useRef(view);
  useEffect(() => {
    let raf = 0;
    const step = () => {
      const v = pick();
      if (!sameView(v, ref.current)) {
        ref.current = v;
        setView(v);
      }
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [time, lines]);
  return view;
}

const wordsOf = (l: LyricLine) => (l.words?.length ? l.words : [{ ms: l.ms, text: l.text }]);

function Guide({ g, word, show, small }: { g: WordGuide; word: string; show: ShowOptions; small?: boolean }) {
  const say = show.say && g.say && g.say !== word ? g.say : '';
  // Syllables only when a word has more than one (a single one adds nothing).
  const syl = show.syllables && g.syllables.includes('·') ? g.syllables : '';
  if (!say && !syl) return null;
  return (
    <span className={cx('mt-0.5 flex flex-col items-center leading-tight', small ? 'text-[11px]' : 'text-[14px]')}>
      {say && <span className="k-say font-medium text-dim">{g.say}</span>}
      {syl && <span className={cx('text-faint', small ? 'text-[10px]' : 'text-[12px]')}>{g.syllables}</span>}
    </span>
  );
}

/** A line not being sung: quiet, with its pronunciation small. */
const StillLine = memo(function StillLine({ line, guides, show, onClick, faded }: { line: LyricLine; guides: WordGuide[]; show: ShowOptions; onClick: () => void; faded?: boolean }) {
  return (
    <button type="button" onClick={onClick} className={cx('flex w-full flex-wrap justify-center gap-x-2.5 gap-y-1 rounded-xl px-3 py-1.5 text-center transition-opacity hover:bg-surface-2/60', faded ? 'opacity-40' : 'opacity-70')}>
      {wordsOf(line).map((w, k) =>
        w.text.trim() ? (
          <span key={k} className="flex flex-col items-center">
            <span className="text-[17px] font-medium text-fg">{w.text.trim()}</span>
            <Guide g={guides[k]} word={w.text.trim()} show={show} small />
          </span>
        ) : null,
      )}
    </button>
  );
});

/** The line being sung: each word fills as it's sung (styles set per frame, no re-render). */
function LiveLine({ lines, index, guides, show, time }: { lines: LyricLine[]; index: number; guides: WordGuide[]; show: ShowOptions; time: () => number }) {
  const line = lines[index];
  const words = useMemo(() => wordsOf(line), [line]);
  const end = useMemo(() => lineEnd(lines, index), [lines, index]);
  const spans = useRef<(HTMLSpanElement | null)[]>([]);
  const boxes = useRef<(HTMLSpanElement | null)[]>([]);
  useEffect(() => {
    let raf = 0;
    const step = () => {
      const t = time();
      words.forEach((w, i) => {
        const el = spans.current[i];
        if (!el) return;
        const stop = words[i + 1]?.ms ?? end;
        const f = Math.min(1, Math.max(0, (t - w.ms) / Math.max(1, stop - w.ms)));
        el.style.backgroundImage = `linear-gradient(90deg, var(--accent) ${f * 100}%, color-mix(in oklab, var(--text) 45%, transparent) ${f * 100}%)`;
        const box = boxes.current[i];
        if (box) box.dataset.state = f >= 1 ? 'sung' : f > 0 ? 'now' : 'next';
      });
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [words, end, time]);
  return (
    <div className="flex flex-wrap justify-center gap-x-3.5 gap-y-2 px-4 text-center">
      {words.map((w, i) =>
        w.text.trim() ? (
          <span
            key={i}
            ref={(el) => {
              boxes.current[i] = el;
            }}
            className="k-word flex flex-col items-center transition-transform duration-150 data-[state=now]:-translate-y-0.5 data-[state=now]:[&_.k-say]:text-accent data-[state=sung]:[&_.k-say]:text-fg"
          >
            <span
              ref={(el) => {
                spans.current[i] = el;
              }}
              className="bg-clip-text font-display text-[34px] font-bold leading-[1.15] text-transparent"
              style={{ backgroundImage: 'linear-gradient(90deg, var(--accent) 0%, color-mix(in oklab, var(--text) 45%, transparent) 0%)' }}
            >
              {w.text.trim()}
            </span>
            <Guide g={guides[i]} word={w.text.trim()} show={show} />
          </span>
        ) : null,
      )}
    </div>
  );
}

function Countdown({ to, time, label }: { to: number; time: () => number; label: string }) {
  const [left, setLeft] = useState(() => Math.max(0, to - time()));
  useEffect(() => {
    const id = window.setInterval(() => setLeft(Math.max(0, to - time())), 200);
    return () => window.clearInterval(id);
  }, [to, time]);
  return (
    <div className="flex flex-col items-center gap-3 py-6 text-dim">
      <div className="flex gap-2" aria-hidden>
        {[0, 1, 2].map((k) => (
          <span key={k} className={cx('h-3 w-3 rounded-full transition-colors', left < (3 - k) * 1000 ? 'bg-accent' : 'bg-surface-3')} />
        ))}
      </div>
      <span className="text-[13px]">
        {label} {left > 0 ? `in ${Math.ceil(left / 1000)} s` : 'now'}
      </span>
    </div>
  );
}

export function KaraokeLyrics({ lines, time, song, reader, show, loop, waiting, onPick }: { lines: LyricLine[]; time: () => number; song: SongLanguage; reader: Reader; show: ShowOptions; loop: number | null; waiting: boolean; onPick: (i: number) => void }) {
  const view = useView(time, lines);
  const guides = useMemo(() => lines.map((l) => wordsOf(l).map((w) => guideWord(w.text, song, reader))), [lines, song, reader]);
  const sung = (from: number, dir: 1 | -1, count: number) => {
    const out: number[] = [];
    for (let k = from; k >= 0 && k < lines.length && out.length < count; k += dir) if (lines[k].text.trim()) out.push(k);
    return out;
  };
  const cur = view.kind === 'line' ? view.index : view.kind === 'break' ? view.next : view.kind === 'intro' ? sung(0, 1, 1)[0] ?? -1 : -1;
  const before = view.kind === 'line' ? sung(cur - 1, -1, 1) : [];
  const after = view.kind === 'line' ? sung(cur + 1, 1, 2) : cur >= 0 ? sung(cur, 1, 2) : [];
  const tips = view.kind === 'line' ? [...new Set(guides[cur].flatMap((g) => g.tips))] : [];

  if (view.kind === 'end') return <div className="py-16 text-center text-[14px] text-dim">That’s the end of the lyrics.</div>;

  return (
    <div className="flex flex-col items-center gap-3 py-4">
      <div className="min-h-[52px] w-full">
        {before.map((i) => (
          <StillLine key={i} line={lines[i]} guides={guides[i]} show={show} onClick={() => onPick(i)} faded />
        ))}
      </div>
      <div className={cx('relative w-full rounded-2xl py-4', loop === cur && 'ring-1 ring-accent/40', waiting && 'bg-accent-soft/40')}>
        {waiting && (
          <div className="mb-2 flex items-center justify-center gap-1.5 text-[12.5px] font-medium text-accent">
            <Mic size={13} aria-hidden /> Your turn: start singing and the song follows
          </div>
        )}
        {view.kind === 'line' ? (
          <LiveLine lines={lines} index={cur} guides={guides[cur]} show={show} time={time} />
        ) : (
          <Countdown to={view.to} time={time} label={view.kind === 'intro' ? 'First line' : 'Next line'} />
        )}
      </div>
      <div className="w-full space-y-1">
        {after.map((i) => (
          <StillLine key={i} line={lines[i]} guides={guides[i]} show={show} onClick={() => onPick(i)} />
        ))}
      </div>
      {tips.length > 0 && show.say && (
        <div className="mt-1 w-full max-w-[640px] rounded-xl border border-line bg-surface-2/50 px-3.5 py-2.5">
          {tips.slice(0, 4).map((t) => (
            <div key={t} className="flex items-start gap-2 py-0.5 text-[12.5px] text-dim">
              <Lightbulb size={13} className="mt-0.5 shrink-0 text-warn" aria-hidden />
              {t}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
