// Aurora's lyrics, over the music.
//
// Stack (the default, like the reference): the word being sung, huge, in
// the middle and highlighted; the one before above and the one after below,
// small and quiet. Words come from the lyrics' own word timings, or — for
// lyrics that only time lines, when "estimate words" is on — from
// lib/aurora/timing.ts, which spreads each line's words by syllables. With
// that off, a whole line lights up at a time. Lines: the current line with
// the next ones under it, words filling in as they are sung.
//
// Instrumental breaks show three breathing dots that fill up toward the next
// line, which appears a moment before it is sung. Seeking jumps straight to
// the right place; a new track never shows the last one's lyrics; songs
// without lyrics, instrumentals and unsynced lyrics each say so.

import type { LyricLine, VisualSettings } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { type CSSProperties, forwardRef, memo, type ReactNode, useEffect, useMemo, useRef, useState } from 'react';
import { LevelFollower } from '../../lib/aurora/audio';
import { estimateWords } from '../../lib/aurora/timing';
import { activeLine } from '../../lib/nowPlaying';

export type LyricsState = 'ready' | 'searching' | 'none' | 'off' | 'instrumental' | 'plain' | 'loading';

type Phase = { kind: 'line'; index: number } | { kind: 'break'; from: number; to: number; next: number } | { kind: 'intro'; to: number } | { kind: 'end' };

/** About when a line has been sung (its last word, or a guess from its length). */
export function lineEnd(lines: LyricLine[], i: number): number {
  const l = lines[i];
  const next = lines[i + 1]?.ms ?? Infinity;
  if (l.words?.length) {
    const w = l.words;
    const avg = w.length > 1 ? (w[w.length - 1].ms - w[0].ms) / (w.length - 1) : 500;
    return Math.min(next, w[w.length - 1].ms + Math.max(500, avg * 1.6));
  }
  const words = l.text.split(/\s+/).filter(Boolean).length;
  return Math.min(next, l.ms + Math.max(2500, words * 380 + 1400));
}

/** What to show at `t`: a line, a break (with what follows), the intro, or nothing after the end. */
export function phaseAt(lines: LyricLine[], t: number): Phase {
  if (!lines.length) return { kind: 'end' };
  const i = activeLine(lines, t);
  const nextSung = (from: number) => {
    for (let k = from; k < lines.length; k++) if (lines[k].text) return k;
    return -1;
  };
  if (i < 0) {
    const first = nextSung(0);
    return first >= 0 && lines[first].ms > 3500 ? { kind: 'intro', to: lines[first].ms } : { kind: 'line', index: Math.max(0, first) };
  }
  const n = nextSung(i + 1);
  if (!lines[i].text) {
    // An empty line marks a break.
    return n >= 0 ? { kind: 'break', from: lines[i].ms, to: lines[n].ms, next: n } : { kind: 'end' };
  }
  const end = lineEnd(lines, i);
  if (n < 0) return t > end + 4000 ? { kind: 'end' } : { kind: 'line', index: i };
  // A long wait for the next line after this one is sung: a break.
  if (lines[n].ms - end > 6000 && t > end + 800) return { kind: 'break', from: end, to: lines[n].ms, next: n };
  return { kind: 'line', index: i };
}

/** One step of the stack: a word (when words are timed) or a whole line. */
export interface StackItem {
  ms: number;
  text: string;
  line: number;
  word: boolean;
}

/** The stack's steps. Syllables timed separately ("beau" "tiful") make one word. */
export function stackItems(lines: LyricLine[], words: boolean): StackItem[] {
  const out: StackItem[] = [];
  lines.forEach((l, i) => {
    if (!l.text) return;
    if (!words || !l.words?.length) {
      out.push({ ms: l.ms, text: l.text, line: i, word: false });
      return;
    }
    let cur: { ms: number; text: string } | null = null;
    for (const w of l.words) {
      if (cur && !/\s$/.test(cur.text) && !/^\s/.test(w.text)) cur.text += w.text;
      else {
        if (cur) out.push({ ms: cur.ms, text: cur.text.trim(), line: i, word: true });
        cur = { ms: w.ms, text: w.text };
      }
    }
    if (cur) out.push({ ms: cur.ms, text: cur.text.trim(), line: i, word: true });
  });
  return out;
}

/** The stack at `t`: the step being sung (or a break) and its neighbours. */
export function stackAt(items: StackItem[], lines: LyricLine[], t: number): { cur: number; dots: { from: number; to: number } | null; next: number; prev: number } | null {
  const phase = phaseAt(lines, t);
  if (phase.kind === 'end' || !items.length) return null;
  const firstOf = (line: number) => items.findIndex((x) => x.line >= line);
  if (phase.kind === 'line') {
    let i = -1;
    for (let k = 0; k < items.length && items[k].ms <= t; k++) i = k;
    i = Math.max(i, firstOf(phase.index));
    return { cur: i, dots: null, next: i + 1, prev: i - 1 };
  }
  const next = phase.kind === 'break' ? firstOf(phase.next) : 0;
  return { cur: -1, dots: { from: phase.kind === 'break' ? phase.from : 0, to: phase.to }, next: phase.to - t < 3000 ? next : -1, prev: next - 1 };
}

const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** Follow the song on every frame; React renders only when `pick` changes. */
function useFollow<T>(time: () => number, pick: (t: number) => T, deps: unknown[]): { value: T; jump: boolean } {
  const [state, setState] = useState(() => ({ value: pick(time()), jump: false }));
  const pickRef = useRef(pick);
  pickRef.current = pick;
  useEffect(() => {
    let raf = 0;
    let cur = pickRef.current(time());
    setState({ value: cur, jump: false });
    let lastT = time();
    let lastNow = performance.now();
    const step = (now: number) => {
      const t = time();
      // A seek: change without the usual animation.
      const jumped = Math.abs(t - lastT - (now - lastNow)) > 1500;
      lastT = t;
      lastNow = now;
      const p = pickRef.current(t);
      if (!same(p, cur)) {
        cur = p;
        setState({ value: p, jump: jumped });
      }
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
    // `deps` say when `pick` means something new.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [time, ...deps]);
  return state;
}

export const FONTS: Record<VisualSettings['lyrics']['font'], { label: string; css: string; stretch?: string }> = {
  display: { label: 'OmniHub', css: 'var(--font-display)' },
  condensed: { label: 'Condensed', css: 'Bahnschrift, "Arial Narrow", var(--font-display)', stretch: 'condensed' },
  heavy: { label: 'Heavy', css: '"Arial Black", "Segoe UI Black", var(--font-display)' },
  serif: { label: 'Serif', css: 'Georgia, "Times New Roman", serif' },
  impact: { label: 'Impact', css: 'Impact, Haettenschweiler, var(--font-display)' },
};

// ---------- pieces ----------

/** The words of the line being sung, filled as they are sung (no React render per frame). */
function KaraokeWords({ line, end, time, color, glow }: { line: LyricLine; end: number; time: () => number; color: string; glow: number }) {
  const spans = useRef<(HTMLSpanElement | null)[]>([]);
  const words = useMemo(() => line.words ?? [], [line.words]);
  useEffect(() => {
    let raf = 0;
    const step = () => {
      const t = time();
      words.forEach((w, i) => {
        const el = spans.current[i];
        if (!el) return;
        const stop = words[i + 1]?.ms ?? end;
        const f = Math.min(1, Math.max(0, (t - w.ms) / Math.max(1, stop - w.ms)));
        el.style.backgroundImage = `linear-gradient(90deg, ${color} ${f * 100}%, rgba(255,255,255,.36) ${f * 100}%)`;
        const singing = f > 0 && f < 1;
        el.style.transform = `translateY(${singing ? -0.04 : 0}em)`;
        el.style.filter = singing && glow > 0 ? `drop-shadow(0 0 ${0.25 * glow}em color-mix(in oklab, var(--aurora-glow) ${Math.round(70 * glow)}%, transparent))` : 'none';
      });
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [words, end, time, color, glow]);
  return (
    <>
      {words.map((w, i) => (
        <span
          key={i}
          ref={(el) => {
            spans.current[i] = el;
          }}
          className="inline-block whitespace-pre bg-clip-text text-transparent transition-[transform] duration-150"
          style={{ backgroundImage: 'linear-gradient(90deg, #fff 0%, rgba(255,255,255,.36) 0%)' }}
        >
          {w.text}
        </span>
      ))}
    </>
  );
}

/** Three dots through a break, filling up toward the next line. */
function BreakDots({ from, to, time, reduced }: { from: number; to: number; time: () => number; reduced: boolean }) {
  const dots = useRef<(HTMLSpanElement | null)[]>([]);
  useEffect(() => {
    let raf = 0;
    const step = () => {
      const p = Math.min(1, Math.max(0, (time() - from) / Math.max(1, to - from)));
      dots.current.forEach((d, i) => {
        if (d) d.style.opacity = String(0.25 + 0.75 * Math.min(1, Math.max(0, p * 3 - i)));
      });
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [from, to, time]);
  return (
    <div className="flex justify-center gap-[0.35em] py-[0.2em]" style={{ animation: reduced ? undefined : 'lyric-breathe 2.4s ease-in-out infinite' }} role="img" aria-label="Instrumental">
      {[0, 1, 2].map((i) => (
        <span
          key={i}
          ref={(el) => {
            dots.current[i] = el;
          }}
          className="h-[0.32em] w-[0.32em] rounded-full"
          style={{ opacity: 0.25, background: 'var(--aurora-hl)', boxShadow: '0 0 0.4em var(--aurora-glow)' }}
        />
      ))}
    </div>
  );
}

const still = () => 0;

// ---------- layouts ----------

interface LayoutProps {
  lines: LyricLine[];
  time: () => number;
  s: VisualSettings['lyrics'];
  reduced: boolean;
  /** Width of the overlay, px. */
  width: number;
  full: boolean;
}

/** The reference look: the step being sung, huge, between its neighbours. */
function StackLayout({ lines, time, s, reduced, width, full }: LayoutProps) {
  const wordMode = s.timing === 'auto' && s.wordHighlight;
  const items = useMemo(() => stackItems(lines, wordMode), [lines, wordMode]);
  const { value: at, jump } = useFollow(time, (t) => stackAt(items, lines, t), [items, lines]);
  if (!at) return null;

  const base = Math.min(full ? 230 : 150, Math.max(full ? 56 : 38, width * 0.115)) * s.size;
  const avail = width * 0.88;
  const sizeOf = (it: StackItem) => (it.word ? Math.min(base, avail / (0.62 * Math.max(3, it.text.length))) : Math.min(base * 0.58, Math.max(base * 0.26, (1.8 * avail) / (0.6 * Math.max(10, it.text.length)))));
  const small = (it: StackItem) => Math.max(15, Math.min(sizeOf(it) * 0.36, base * (it.word ? 0.3 : 0.2)));
  const dur = reduced ? 0.25 : jump ? 0.1 : 0.34 / Math.max(0.25, s.transition);
  const ease = [0.22, 1, 0.36, 1] as const;
  // Movement: each word gets its own slight tilt, more with more movement.
  const mv = reduced ? 0 : Math.max(0, s.motion);
  const tiltOf = (key: string) => {
    let h = 0;
    for (let i = 0; i < key.length; i++) h = (h * 31 + key.charCodeAt(i)) | 0;
    return (((h >>> 0) % 1000) / 1000 - 0.5) * 6 * mv;
  };
  const hl = s.highlightColor ?? '#ffffff';
  const g = Math.min(1, s.glow);
  const glowShadow =
    s.glow > 0
      ? `0 0 ${0.12 + 0.1 * s.glow}em color-mix(in oklab, var(--aurora-glow) ${Math.round(75 * g)}%, transparent), 0 0 ${0.5 * s.glow}em color-mix(in oklab, var(--aurora-glow) ${Math.round(40 * g)}%, transparent), 0 0.04em 0.3em rgba(0,0,0,.45)`
      : '0 0.04em 0.3em rgba(0,0,0,.5)';

  type Row = { key: string; role: 'prev' | 'cur' | 'next'; node: ReactNode; size: number };
  const rows: Row[] = [];
  if (s.lines >= 3 && at.prev >= 0 && items[at.prev]) rows.push({ key: `i${at.prev}`, role: 'prev', node: items[at.prev].text, size: small(items[at.prev]) });
  if (at.dots) rows.push({ key: `gap${at.dots.from}`, role: 'cur', node: <BreakDots from={at.dots.from} to={at.dots.to} time={time} reduced={reduced} />, size: base * 0.55 });
  else if (items[at.cur]) rows.push({ key: `i${at.cur}`, role: 'cur', node: items[at.cur].text, size: sizeOf(items[at.cur]) });
  if (s.lines >= 2 && at.next >= 0 && items[at.next]) rows.push({ key: `i${at.next}`, role: 'next', node: items[at.next].text, size: small(items[at.next]) });

  const look = (role: Row['role'], dots: boolean): CSSProperties => {
    if (role === 'cur' && !dots) {
      if (s.emphasis === 'box')
        return {
          color: '#fff',
          background: 'var(--aurora-box)',
          padding: '0.02em 0.22em 0.06em',
          borderRadius: '0.14em',
          textShadow: '0 0.03em 0.12em rgba(0,0,0,.35)',
          boxShadow: s.glow > 0 ? `0 0 ${0.6 * s.glow}em color-mix(in oklab, var(--aurora-glow) 45%, transparent)` : undefined,
        };
      if (s.emphasis === 'color') return { color: 'var(--aurora-hl-accent)', textShadow: glowShadow };
      return { color: hl, textShadow: glowShadow };
    }
    // Only what is being sung is highlighted; what comes next stays quiet.
    if (role === 'next') return { color: 'rgba(255,255,255,.7)', textShadow: '0 0.04em 0.3em rgba(0,0,0,.55)' };
    return { color: 'rgba(255,255,255,.55)', textShadow: '0 0.04em 0.3em rgba(0,0,0,.5)' };
  };

  return (
    // Sizes animate and the column re-centres every frame, so words glide
    // between places; a step leaving fades where it was.
    <div className="relative flex flex-col items-center justify-center text-center">
      <AnimatePresence initial={false} mode="popLayout">
        {rows.map((r) => (
          <motion.div
            key={r.key}
            initial={reduced ? { opacity: 0, fontSize: r.size } : { opacity: 0, y: `${0.4 + 0.5 * mv}em`, scale: 1 - 0.35 * mv, rotate: tiltOf(r.key) * 2, filter: 'blur(6px)', fontSize: r.size }}
            animate={{ opacity: r.role === 'prev' ? 0.6 : 1, y: 0, scale: 1, rotate: r.role === 'cur' ? tiltOf(r.key) * 0.5 : 0, filter: 'blur(0px)', fontSize: r.size }}
            exit={reduced ? { opacity: 0 } : { opacity: 0, y: `-${0.4 + 0.4 * mv}em`, scale: 1 - 0.2 * mv, rotate: -tiltOf(r.key), filter: 'blur(6px)' }}
            // With movement, words pop in with a little overshoot.
            transition={
              mv > 0.02 && !jump
                ? {
                    default: { duration: dur, ease },
                    scale: { type: 'spring', stiffness: 520, damping: 12 + (1.5 - mv) * 9 },
                    y: { type: 'spring', stiffness: 380, damping: 14 + (1.5 - mv) * 8 },
                    rotate: { type: 'spring', stiffness: 300, damping: 16 },
                  }
                : { duration: dur, ease }
            }
            className="max-w-full leading-[1.08] tracking-[-0.02em] [text-wrap:balance]"
            style={{ fontWeight: r.role === 'cur' ? s.weight : Math.max(600, s.weight - 100), marginTop: r.role === 'next' ? '0.35em' : r.role === 'cur' ? '0.04em' : 0 }}
            data-lyric-role={r.role}
            {...(r.role === 'cur' ? { 'data-lyric-current': '' } : r.role === 'next' ? { 'data-lyric-next': '' } : {})}
          >
            <span className="inline-block transition-[background-color,color,padding,border-radius] duration-300 [box-decoration-break:clone]" style={look(r.role, r.key.startsWith('gap'))}>
              {r.node}
            </span>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}

/** The current line with the next lines under it. */
function LinesLayout({ lines, time, s, reduced, full }: LayoutProps) {
  const { value: phase, jump } = useFollow(time, (t) => phaseAt(lines, t), [lines]);
  const wordMode = s.timing === 'auto' && s.wordHighlight;
  const hl = s.highlightColor ?? '#ffffff';
  const dur = reduced ? 0.3 : jump ? 0.12 : 0.55 / Math.max(0.25, s.transition);
  const enter = reduced ? { opacity: 0 } : { opacity: 0, y: '0.6em', scale: 0.96, filter: 'blur(8px)' };
  const exit = reduced ? { opacity: 0 } : { opacity: 0, y: '-0.5em', scale: 0.98, filter: 'blur(6px)' };
  const settle = reduced ? { opacity: 1 } : { opacity: 1, y: 0, scale: 1, filter: 'blur(0px)' };
  const fontSize = `calc(${full ? 'clamp(30px, 4.6cqw, 88px)' : 'clamp(22px, 4.2cqw, 50px)'} * ${s.size})`;
  const glowCss = s.glow > 0 ? `0 0 ${0.5 * s.glow}em color-mix(in oklab, var(--aurora-glow) ${Math.round(55 * Math.min(1, s.glow))}%, transparent)` : 'none';
  const box: CSSProperties | undefined = s.emphasis === 'box' ? { background: 'var(--aurora-box)', padding: '0 0.2em', borderRadius: '0.14em', boxDecorationBreak: 'clone', WebkitBoxDecorationBreak: 'clone' } : undefined;
  const lineColor = s.emphasis === 'color' ? 'var(--aurora-hl-accent)' : hl;

  const upcoming = (from: number) => {
    const out: { i: number; l: LyricLine }[] = [];
    for (let k = from; k < lines.length && out.length < Math.max(0, s.lines - 1); k++) if (lines[k].text) out.push({ i: k, l: lines[k] });
    return out;
  };
  let body: { key: string; node: ReactNode; after: { i: number; l: LyricLine }[] } | null = null;
  const t = time();
  if (phase.kind === 'line') {
    const l = lines[phase.index];
    const words = wordMode && l.words?.length ? l.words : null;
    body = {
      key: `l${phase.index}`,
      node: words ? (
        <KaraokeWords line={l} end={lineEnd(lines, phase.index)} time={time} color={s.emphasis === 'color' ? 'var(--aurora-hl-accent)' : hl} glow={s.glow} />
      ) : (
        <span style={{ color: lineColor, textShadow: glowCss, ...box }}>{l.text}</span>
      ),
      after: upcoming(phase.index + 1),
    };
  } else if (phase.kind === 'break' || phase.kind === 'intro') {
    const from = phase.kind === 'break' ? phase.from : 0;
    const nextIdx = phase.kind === 'break' ? phase.next : lines.findIndex((l) => l.text);
    body = { key: `b${from}`, node: <BreakDots from={from} to={phase.to} time={time} reduced={reduced} />, after: phase.to - t < 3000 && nextIdx >= 0 ? upcoming(nextIdx) : [] };
  }
  if (!body) return null;
  return (
    <div style={{ fontSize }}>
      <AnimatePresence mode="popLayout" initial={false}>
        <motion.div
          key={body.key}
          initial={enter}
          animate={settle}
          exit={exit}
          transition={{ duration: dur, ease: [0.22, 1, 0.36, 1] }}
          className="leading-[1.12] tracking-[-0.02em] [text-wrap:balance]"
          style={{ fontWeight: s.weight }}
          data-lyric-current
        >
          {body.node}
        </motion.div>
      </AnimatePresence>
      <AnimatePresence initial={false}>
        {body.after.map(({ i, l }, k) => (
          <motion.div
            key={`n${i}`}
            layout={!reduced}
            initial={{ opacity: 0, y: reduced ? 0 : '0.4em' }}
            animate={{ opacity: Math.max(0.16, 0.42 - k * 0.14), y: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: dur * 0.9, ease: [0.22, 1, 0.36, 1] }}
            className="mt-[0.35em] leading-[1.15] tracking-[-0.015em] text-white [text-wrap:balance]"
            style={{ fontSize: '0.62em', fontWeight: Math.max(500, s.weight - 100) }}
            data-lyric-next
          >
            {l.text}
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  );
}

// ---------- the overlay ----------

export interface AuroraLyricsProps {
  lines: LyricLine[];
  state: LyricsState;
  /** Lyrics time: the track position with the user's timing nudge. */
  time: () => number;
  trackKey: string;
  settings: VisualSettings['lyrics'];
  reduced: boolean;
  full: boolean;
  shown: boolean;
  onShowPlain?: () => void;
}

export const AuroraLyrics = memo(
  forwardRef<HTMLDivElement, AuroraLyricsProps>(function AuroraLyrics({ lines, state, time, trackKey, settings: s, reduced, full, shown, onShowPlain }, boxRef) {
    const root = useRef<HTMLDivElement>(null);
    const [width, setWidth] = useState(1200);
    // Movement with the music: a gentle sway, a lift with the bass and a
    // bounce on beats, applied to the whole block on every frame.
    const [moveEl, setMoveEl] = useState<HTMLDivElement | null>(null);
    const mv = reduced ? 0 : Math.max(0, s.motion);
    useEffect(() => {
      if (!moveEl) return;
      if (mv < 0.005) {
        moveEl.style.transform = '';
        return;
      }
      const follow = new LevelFollower();
      let raf = 0;
      let last = performance.now();
      let t = Math.random() * 100;
      const step = (now: number) => {
        raf = requestAnimationFrame(step);
        const dt = Math.min(0.1, (now - last) / 1000);
        last = now;
        t += dt;
        const lv = follow.step(dt);
        const k = mv * (lv.active ? 1 : 0.4);
        const x = Math.sin(t * 0.55) * 7 * k + Math.sin(t * 1.7) * 2 * k;
        const y = Math.cos(t * 0.75) * 5 * k - lv.low * 10 * mv;
        const rot = Math.sin(t * 0.4) * 1.2 * k;
        const scale = 1 + lv.beat * 0.075 * mv + lv.level * 0.02 * mv;
        const jolt = lv.beat > 0.8 ? (Math.random() - 0.5) * 4 * mv : 0;
        moveEl.style.transform = `translate(${(x + jolt).toFixed(2)}px, ${y.toFixed(2)}px) rotate(${rot.toFixed(3)}deg) scale(${scale.toFixed(4)})`;
      };
      raf = requestAnimationFrame(step);
      return () => cancelAnimationFrame(raf);
    }, [moveEl, mv]);
    useEffect(() => {
      const el = root.current;
      if (!el) return;
      const ro = new ResizeObserver(() => setWidth(el.clientWidth || 1200));
      ro.observe(el);
      setWidth(el.clientWidth || 1200);
      return () => ro.disconnect();
    }, []);

    const message: { title: string; detail?: string; action?: { label: string; run: () => void } } | null =
      state === 'searching' || state === 'loading'
        ? { title: 'Looking for lyrics…' }
        : state === 'none'
          ? { title: 'No lyrics for this song', detail: 'The light keeps following the music.' }
          : state === 'off'
            ? { title: 'Online lyrics are off', detail: 'Turn on “Look lyrics up online” in Aurora’s settings, or choose a folder of .lrc files.' }
            : state === 'instrumental'
              ? { title: 'Instrumental' }
              : state === 'plain'
                ? { title: 'These lyrics aren’t time-synced', detail: 'Aurora shows lyrics in time with the song.', action: onShowPlain ? { label: 'Read them in the Lyrics view', run: onShowPlain } : undefined }
                : null;

    // Messages fade out after a while (instrumental stays, quietly).
    const [messageGone, setMessageGone] = useState(false);
    useEffect(() => {
      setMessageGone(false);
      if (state === 'none' || state === 'off') {
        const timer = setTimeout(() => setMessageGone(true), 7000);
        return () => clearTimeout(timer);
      }
    }, [state, trackKey]);

    // Lyrics that only time lines: words spread over each line (an estimate, when allowed).
    const timed = useMemo(() => (s.estimateWords && s.timing === 'auto' && s.wordHighlight ? estimateWords(lines) : lines), [lines, s.estimateWords, s.timing, s.wordHighlight]);
    const ready = state === 'ready' && lines.length > 0;
    const top = s.place === 'upper' ? 28 : s.place === 'lower' ? 70 : 48;
    const font = FONTS[s.font] ?? FONTS.display;
    const plate = s.backing > 0.02;
    const vars = {
      '--aurora-hl': s.highlightColor ?? '#ffffff',
      '--aurora-glow': s.highlightColor ?? 'color-mix(in oklab, var(--aurora-accent, #c026d3) 80%, white)',
      '--aurora-hl-accent': s.highlightColor ?? 'color-mix(in oklab, var(--aurora-accent, #c026d3) 70%, white)',
      '--aurora-box': s.highlightColor ?? 'color-mix(in oklab, var(--aurora-accent, #c026d3) 88%, black)',
      '--aurora-pill': 'color-mix(in oklab, var(--aurora-accent, #c026d3) 72%, transparent)',
      fontFamily: font.css,
      fontStretch: font.stretch,
    } as CSSProperties;
    const layout = { lines: timed, time, s, reduced, width, full };

    return (
      <div ref={root} className="pointer-events-none absolute inset-0 z-[5] [container-type:size]" aria-live="off">
        <AnimatePresence>
          {shown && (ready || (message && !messageGone)) && (
            <motion.div key={`${trackKey}-box`} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.45 }} className="absolute inset-0">
              <div ref={boxRef} className="absolute left-1/2 w-[90%] max-w-[1500px] text-center" style={{ top: `${top + s.offsetY}%`, transform: `translate(calc(-50% + ${s.offsetX}cqw), -50%)`, ...vars }} data-aurora-lyrics>
                <div ref={setMoveEl} className="will-change-transform">
                  <div
                    className={plate ? 'inline-block max-w-full rounded-[0.6em] px-[1.2em] py-[0.6em] backdrop-blur-xl' : undefined}
                    style={{ background: plate ? `rgba(4,4,10,${s.backing})` : undefined, fontSize: plate ? 'clamp(14px, 1.6cqw, 26px)' : undefined }}
                  >
                    {ready ? (
                      s.layout === 'lines' ? (
                        <LinesLayout {...layout} />
                      ) : (
                        <StackLayout {...layout} />
                      )
                    ) : message ? (
                      <div className="text-white" style={{ fontSize: full ? 'clamp(20px, 2.2cqw, 40px)' : 'clamp(16px, 2.4cqw, 26px)' }} data-lyric-message>
                        {state === 'instrumental' && <BreakDots from={0} to={1} time={still} reduced={reduced} />}
                        <div className={state === 'searching' || state === 'loading' ? 'animate-pulse font-semibold opacity-70' : 'font-bold opacity-85'}>{message.title}</div>
                        {message.detail && <div className="mx-auto mt-2 max-w-[28em] text-[0.55em] font-medium leading-snug text-white/60">{message.detail}</div>}
                        {message.action && (
                          <button type="button" onClick={message.action.run} className="pointer-events-auto mt-3 rounded-full bg-white/12 px-4 py-1.5 text-[0.5em] font-semibold text-white backdrop-blur hover:bg-white/20">
                            {message.action.label}
                          </button>
                        )}
                      </div>
                    ) : null}
                  </div>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    );
  }),
);
