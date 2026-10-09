// The music player, Apple Music style: the cover art blurred into drifting
// light behind everything, the artwork and controls on the left, and the
// lyrics on the right in big type that glides up line by line (each line a
// moment after the one above it), fills word by word when the lyrics time
// words, breathes three dots through instrumental breaks, and jumps to a
// line when you click it.
//
// `mode="fullscreen"` covers the window (and puts the window in full
// screen); `mode="page"` fills the Music page.

import type { LyricLine, MediaState } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { ChevronDown, Maximize2, Minimize2, Minus, Music2, Pause, Play, Plus, SkipBack, SkipForward, Volume1, Volume2, VolumeX } from 'lucide-react';
import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { api } from '../../api';
import { cx } from '../../lib/cx';
import { activeLine, fmtTime, setWindowFullscreen, useFrame, useNowPlaying } from '../../lib/nowPlaying';

const OFFSET_KEY = 'omnihub.desktopLyricsOffset';
const IDLE_MS = 3000;

function readOffset(): number {
  try {
    return Number(localStorage.getItem(OFFSET_KEY)) || 0;
  } catch {
    return 0;
  }
}

// ---------- background ----------

function Ambient({ url }: { url: string | null }) {
  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden bg-[#0b0b10]" aria-hidden>
      <AnimatePresence>
        <motion.div key={url ?? 'none'} className="absolute inset-0" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 1.2 }}>
          {url ? (
            ['a', 'b', 'c', 'd'].map((k) => <img key={k} src={url} alt="" className={`ambient-blob ${k}`} draggable={false} />)
          ) : (
            <div className="absolute inset-0 bg-[radial-gradient(60%_60%_at_20%_20%,rgba(139,92,246,.55),transparent),radial-gradient(60%_60%_at_80%_70%,rgba(34,211,238,.35),transparent)]" />
          )}
        </motion.div>
      </AnimatePresence>
      <div className="absolute inset-0 bg-black/35" />
      <div className="absolute inset-0 bg-[radial-gradient(120%_90%_at_50%_40%,transparent,rgba(0,0,0,.45))]" />
    </div>
  );
}

// ---------- controls ----------

function Scrubber({ pos, duration, canSeek, onSeek }: { pos: number; duration: number; canSeek: boolean; onSeek: (ms: number) => void }) {
  const [drag, setDrag] = useState<number | null>(null);
  const bar = useRef<HTMLDivElement>(null);
  const shown = drag ?? pos;
  const at = (x: number) => {
    const r = bar.current?.getBoundingClientRect();
    return r && duration ? Math.min(1, Math.max(0, (x - r.left) / r.width)) * duration : 0;
  };
  const pct = duration ? (shown / duration) * 100 : 0;
  return (
    <div className="select-none">
      <div
        ref={bar}
        role="slider"
        aria-label="Position"
        aria-valuemin={0}
        aria-valuemax={Math.round(duration / 1000)}
        aria-valuenow={Math.round(shown / 1000)}
        tabIndex={canSeek ? 0 : -1}
        className={cx('group relative flex h-5 items-center', canSeek && 'cursor-pointer')}
        onPointerDown={(e) => {
          if (!canSeek || !duration) return;
          e.currentTarget.setPointerCapture(e.pointerId);
          setDrag(at(e.clientX));
        }}
        onPointerMove={(e) => drag != null && setDrag(at(e.clientX))}
        onPointerUp={() => {
          if (drag != null) onSeek(drag);
          setDrag(null);
        }}
      >
        <div className={cx('relative w-full overflow-hidden rounded-full bg-white/20 transition-[height]', drag != null ? 'h-2' : 'h-1.5 group-hover:h-2')}>
          <div className="absolute inset-y-0 left-0 rounded-full bg-white/90" style={{ width: `${pct}%` }} />
        </div>
      </div>
      <div className="mt-1 flex justify-between font-mono text-[11.5px] tabular-nums text-white/55">
        <span>{fmtTime(shown)}</span>
        <span>-{fmtTime(Math.max(0, duration - shown))}</span>
      </div>
    </div>
  );
}

function VolumeSlider({ volume, onChange }: { volume: { level: number; muted: boolean }; onChange: (level: number | null, muted: boolean | null) => void }) {
  const Icon = volume.muted || volume.level === 0 ? VolumeX : volume.level < 0.5 ? Volume1 : Volume2;
  return (
    <div className="flex items-center gap-3 text-white/60">
      <button type="button" onClick={() => onChange(null, !volume.muted)} className="grid h-8 w-8 place-items-center rounded-full hover:bg-white/10 hover:text-white" aria-label={volume.muted ? 'Unmute' : 'Mute'}>
        <Icon size={17} />
      </button>
      <input
        type="range"
        min={0}
        max={1}
        step={0.01}
        value={volume.muted ? 0 : volume.level}
        aria-label="Volume"
        onChange={(e) => onChange(Number(e.target.value), false)}
        className="h-1.5 flex-1 cursor-pointer appearance-none rounded-full bg-white/20 accent-white [&::-webkit-slider-thumb]:h-3.5 [&::-webkit-slider-thumb]:w-3.5 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-white"
        style={{ background: `linear-gradient(90deg, rgba(255,255,255,.85) ${(volume.muted ? 0 : volume.level) * 100}%, rgba(255,255,255,.2) 0)` }}
      />
    </div>
  );
}

// ---------- lyrics ----------

/** Words of the line being sung, filled as they are sung (no React render per frame). */
function SungWords({ line, end, time }: { line: LyricLine; end: number; time: () => number }) {
  const spans = useRef<(HTMLSpanElement | null)[]>([]);
  const words = line.words ?? [];
  useEffect(() => {
    let raf = 0;
    const step = () => {
      const t = time();
      words.forEach((w, i) => {
        const el = spans.current[i];
        if (!el) return;
        // The last word lasts about as long as the others, not into a break.
        const avg = words.length > 1 ? (words[words.length - 1].ms - words[0].ms) / (words.length - 1) : 500;
        const stop = words[i + 1]?.ms ?? Math.min(end, w.ms + Math.max(400, avg * 1.4));
        const f = Math.min(1, Math.max(0, (t - w.ms) / Math.max(1, stop - w.ms)));
        el.style.backgroundImage = `linear-gradient(90deg, #fff ${f * 100}%, rgba(255,255,255,.38) ${f * 100}%)`;
        // The word being sung lifts a little and glows.
        el.style.transform = `translateY(${f > 0 && f < 1 ? -2 : f >= 1 ? -1 : 0}px)`;
        el.style.textShadow = f > 0 && f < 1 ? '0 0 18px rgba(255,255,255,.35)' : 'none';
      });
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [words, end, time]);
  return (
    <>
      {words.map((w, i) => (
        <span
          key={i}
          ref={(el) => {
            spans.current[i] = el;
          }}
          className="inline-block whitespace-pre bg-clip-text text-transparent transition-[transform,text-shadow] duration-200"
          style={{ backgroundImage: 'linear-gradient(90deg, #fff 0%, rgba(255,255,255,.38) 0%)' }}
        >
          {w.text}
        </span>
      ))}
    </>
  );
}

/** Three dots through an instrumental break, lighting up as it passes. */
function Interlude({ from, to, time }: { from: number; to: number; time: () => number }) {
  const dots = useRef<(HTMLSpanElement | null)[]>([]);
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    let raf = 0;
    const step = () => {
      const t = time();
      const p = Math.min(1, Math.max(0, (t - from) / Math.max(1, to - from)));
      dots.current.forEach((d, i) => {
        if (d) d.style.opacity = String(0.22 + 0.78 * Math.min(1, Math.max(0, p * 3 - i)));
      });
      if (box.current) box.current.style.transform = `scale(${to - t < 500 ? Math.max(0, (to - t) / 500) : 1})`;
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [from, to, time]);
  return (
    <div ref={box} className="origin-left py-4" aria-hidden>
      <div className="flex gap-3" style={{ animation: 'lyric-breathe 2.4s ease-in-out infinite' }}>
        {[0, 1, 2].map((i) => (
          <span
            key={i}
            ref={(el) => {
              dots.current[i] = el;
            }}
            className="h-4 w-4 rounded-full bg-white"
            style={{ opacity: 0.22 }}
          />
        ))}
      </div>
    </div>
  );
}

const Lyrics = memo(function Lyrics({ lines, active, time, canSeek, onSeek, compact }: { lines: LyricLine[]; active: number; time: () => number; canSeek: boolean; onSeek: (ms: number) => void; compact: boolean }) {
  const box = useRef<HTMLDivElement>(null);
  const refs = useRef<(HTMLDivElement | null)[]>([]);
  const [shift, setShift] = useState(0);
  const [scroll, setScroll] = useState(0);
  const [scrolledAt, setScrolledAt] = useState(0);
  const userScrolling = Date.now() - scrolledAt < 3500;

  // Keep the line being sung about a third of the way down.
  const measure = useCallback(() => {
    const b = box.current;
    const el = refs.current[Math.max(0, active)];
    if (!b || !el) return;
    setShift(el.offsetTop - b.clientHeight * (compact ? 0.3 : 0.32));
  }, [active, compact]);
  useLayoutEffect(measure, [measure, lines]);
  useEffect(() => {
    const ro = new ResizeObserver(measure);
    if (box.current) ro.observe(box.current);
    return () => ro.disconnect();
  }, [measure]);
  // Back to the song a while after scrolling.
  useEffect(() => {
    if (!scrolledAt) return;
    const t = setTimeout(() => {
      setScroll(0);
      setScrolledAt(0);
    }, 3500);
    return () => clearTimeout(t);
  }, [scrolledAt]);

  const next = lines[active + 1];
  const t = time();
  const gapFrom = active < 0 ? 0 : lines[active].ms;
  const showGap = !!next && next.ms - gapFrom > 6500 && next.ms - t > 1200 && (active < 0 || t - lines[active].ms > 3500);

  return (
    <div
      ref={box}
      className="relative h-full overflow-hidden [mask-image:linear-gradient(to_bottom,transparent,black_14%,black_78%,transparent)]"
      onWheel={(e) => {
        setScroll((s) => s + e.deltaY);
        setScrolledAt(Date.now());
      }}
    >
      {lines.map((l, i) => {
        const dist = Math.abs(i - active);
        // In a break the dots take the focus and the last line rests.
        const isActive = i === active && !showGap;
        // Lines below the one being sung follow a moment later: the wave.
        const delay = userScrolling ? 0 : Math.min(0.5, Math.max(0, i - active) * 0.045);
        return (
          <motion.div
            key={`${l.ms}-${i}`}
            ref={(el) => {
              refs.current[i] = el;
            }}
            animate={{ y: -(shift + scroll), opacity: isActive ? 1 : userScrolling ? 0.6 : 0.34, filter: !userScrolling && dist > 1 ? `blur(${Math.min(2.6, (dist - 1) * 0.75)}px)` : 'blur(0px)', scale: isActive ? 1 : 0.97 }}
            transition={userScrolling ? { duration: 0.12 } : { y: { type: 'spring', stiffness: 85, damping: 17, mass: 1, delay }, default: { duration: 0.45, ease: 'easeOut' } }}
            className="origin-left will-change-transform"
          >
            <button
              type="button"
              disabled={!canSeek}
              onClick={() => {
                setScroll(0);
                setScrolledAt(0);
                onSeek(l.ms);
              }}
              className={cx('-mx-4 block w-[calc(100%+2rem)] rounded-2xl px-4 text-left font-display font-extrabold leading-[1.16] tracking-[-0.02em] text-white transition-colors', compact ? 'py-2 text-[clamp(22px,2.4vw,34px)]' : 'py-3 text-[clamp(28px,3.1vw,52px)]', canSeek && 'hover:bg-white/[0.07]')}
            >
              {isActive && l.words?.length ? <SungWords line={l} end={next?.ms ?? l.ms + 4000} time={time} /> : l.text || '♪'}
            </button>
            {i === active && showGap && <Interlude from={gapFrom} to={next.ms} time={time} />}
          </motion.div>
        );
      })}
      {active < 0 && showGap && next && (
        <motion.div className="absolute left-0" style={{ top: (refs.current[0]?.offsetTop ?? 0) - 64 }} animate={{ y: -(shift + scroll) }}>
          <Interlude from={0} to={next.ms} time={time} />
        </motion.div>
      )}
      <div style={{ height: '60%' }} />
    </div>
  );
});

// ---------- the player ----------

function Artwork({ url, state, big }: { url: string | null; state: MediaState; big: boolean }) {
  return (
    <motion.div animate={{ scale: state.playing ? 1 : 0.86 }} transition={{ type: 'spring', stiffness: 160, damping: 20 }} className={cx('aspect-square w-full', big ? 'max-w-[440px]' : 'max-w-[300px]')}>
      {url ? (
        <img src={url} alt={`Cover of ${state.title}`} className="h-full w-full rounded-[18px] object-cover shadow-[0_30px_80px_-20px_rgba(0,0,0,.75)]" draggable={false} />
      ) : (
        <div className="grid h-full w-full place-items-center rounded-[18px] bg-white/10 shadow-[0_30px_80px_-20px_rgba(0,0,0,.75)]">
          <Music2 size={72} className="text-white/50" />
        </div>
      )}
    </motion.div>
  );
}

export function FullPlayer({ mode, onClose }: { mode: 'fullscreen' | 'page'; onClose?: () => void }) {
  const np = useNowPlaying();
  const { state, art, lines, lyrics, control } = np;
  const full = mode === 'fullscreen';
  const [offset, setOffsetState] = useState(readOffset);
  const [volume, setVolume] = useState<{ level: number; muted: boolean } | null>(null);
  const [idle, setIdle] = useState(false);
  const [windowFull, setWindowFull] = useState(full);
  const idleTimer = useRef<ReturnType<typeof setTimeout>>(undefined);

  useFrame(!!state?.playing, 24);
  const setOffset = (o: number) => {
    setOffsetState(o);
    try {
      localStorage.setItem(OFFSET_KEY, String(o));
    } catch {
      /* per session then */
    }
  };
  const positionRef = useRef(np.position);
  positionRef.current = np.position;
  const offsetRef = useRef(offset);
  offsetRef.current = offset;
  // Lyrics time: the track position plus the user's timing nudge.
  const time = useCallback(() => positionRef.current() + offsetRef.current, []);
  const pos = np.position();
  const active = useMemo(() => activeLine(lines, pos + offset), [lines, pos, offset]);

  useEffect(() => {
    void api.media.audio().then((a) => setVolume(a.volume), () => undefined);
  }, []);
  const changeVolume = (level: number | null, muted: boolean | null) => {
    setVolume((v) => (v ? { level: level ?? v.level, muted: muted ?? v.muted } : v));
    void api.media.setVolume(level, muted).then(setVolume, () => undefined);
  };

  // Full screen: the window too; controls and the pointer rest when idle.
  useEffect(() => {
    if (!full) return;
    void setWindowFullscreen(true);
    return () => void setWindowFullscreen(false);
  }, [full]);
  const wake = useCallback(() => {
    setIdle(false);
    clearTimeout(idleTimer.current);
    if (full) idleTimer.current = setTimeout(() => setIdle(true), IDLE_MS);
  }, [full]);
  useEffect(() => {
    wake();
    return () => clearTimeout(idleTimer.current);
  }, [wake]);

  // Keys: space plays/pauses, arrows seek and change the volume, Esc leaves.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) && (target as HTMLInputElement).type !== 'range') return;
      if (!full && document.querySelector('[role="dialog"]')) return;
      wake();
      if (e.key === 'Escape' && full) {
        e.preventDefault();
        onClose?.();
      } else if (e.key === ' ' || e.key === 'k') {
        e.preventDefault();
        control('toggle');
      } else if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') {
        if (!state?.canSeek) return;
        e.preventDefault();
        control('seek', Math.max(0, np.position() + (e.key === 'ArrowRight' ? 5000 : -5000)));
      } else if ((e.key === 'ArrowUp' || e.key === 'ArrowDown') && volume) {
        e.preventDefault();
        changeVolume(Math.min(1, Math.max(0, volume.level + (e.key === 'ArrowUp' ? 0.05 : -0.05))), false);
      } else if (e.key === 'f' && full) {
        const on = !windowFull;
        setWindowFull(on);
        void setWindowFullscreen(on);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  const synced = lines.length > 0;
  const plain = lyrics?.status === 'ready' ? lyrics.lyrics.plain : null;
  const hide = full && idle;
  const fade = { opacity: hide ? 0 : 1, transition: 'opacity .6s ease' };

  return (
    <div className={cx('relative isolate flex h-full w-full overflow-hidden text-white', hide && 'cursor-none')} onMouseMove={wake} onPointerDown={wake}>
      <Ambient url={art} />

      {/* Top bar */}
      <div className="absolute inset-x-0 top-0 z-10 flex items-center justify-end gap-2 p-4" style={fade}>
        {synced && (
          <div className="flex items-center gap-1 rounded-full bg-white/10 px-1.5 py-1 text-[11.5px] backdrop-blur" aria-label="Lyrics timing">
            <button type="button" className="grid h-7 w-7 place-items-center rounded-full hover:bg-white/10" onClick={() => setOffset(offset - 250)} aria-label="Lyrics later" title="Lyrics later">
              <Minus size={14} />
            </button>
            <span className="w-14 text-center font-mono tabular-nums text-white/80">{offset === 0 ? 'in sync' : `${offset > 0 ? '+' : ''}${(offset / 1000).toFixed(2)}s`}</span>
            <button type="button" className="grid h-7 w-7 place-items-center rounded-full hover:bg-white/10" onClick={() => setOffset(offset + 250)} aria-label="Lyrics earlier" title="Lyrics earlier">
              <Plus size={14} />
            </button>
          </div>
        )}
        {full && (
          <button
            type="button"
            onClick={() => {
              const on = !windowFull;
              setWindowFull(on);
              void setWindowFullscreen(on);
            }}
            className="grid h-9 w-9 place-items-center rounded-full bg-white/10 backdrop-blur hover:bg-white/20"
            aria-label={windowFull ? 'Leave full screen' : 'Full screen'}
            title={windowFull ? 'Leave full screen (F)' : 'Full screen (F)'}
          >
            {windowFull ? <Minimize2 size={16} /> : <Maximize2 size={16} />}
          </button>
        )}
        {onClose && (
          <button type="button" onClick={onClose} className="grid h-9 w-9 place-items-center rounded-full bg-white/10 backdrop-blur hover:bg-white/20" aria-label="Close player" title="Close (Esc)">
            <ChevronDown size={18} />
          </button>
        )}
      </div>

      {!state ? (
        <div className="relative m-auto flex flex-col items-center gap-3 text-center">
          <div className="grid h-20 w-20 place-items-center rounded-3xl bg-white/10">
            <Music2 size={34} className="text-white/70" />
          </div>
          <div className="font-display text-[22px] font-bold">{np.loaded ? 'Nothing is playing' : 'Looking for music…'}</div>
          <div className="max-w-sm text-[13.5px] text-white/60">Play something in Spotify, Apple Music, your browser or any player — it shows up here with its lyrics.</div>
        </div>
      ) : (
        <div className={cx('relative grid h-full w-full min-w-0', synced ? (full ? 'grid-cols-[minmax(340px,0.85fr)_1.15fr] gap-[4vw] px-[6vw]' : 'grid-cols-[minmax(260px,0.8fr)_1.2fr] gap-10 px-10') : 'place-items-center')}>
          {/* Artwork and controls */}
          <div className={cx('flex min-w-0 flex-col justify-center', synced ? 'items-center py-10' : 'w-full max-w-[460px] items-center')}>
            <Artwork url={art} state={state} big={full} />
            <div className={cx('mt-7 w-full', full ? 'max-w-[440px]' : 'max-w-[300px]')}>
              <div className="flex items-end justify-between gap-3">
                <div className="min-w-0">
                  <div className="truncate font-display text-[22px] font-bold leading-tight">{state.title}</div>
                  <div className="truncate text-[16px] text-white/60">{state.artist}</div>
                </div>
                <span className="mb-1 shrink-0 rounded-full bg-white/10 px-2 py-0.5 text-[11px] text-white/60">{state.appName}</span>
              </div>
              <div className="mt-4" style={fade}>
                <Scrubber pos={pos} duration={state.durationMs} canSeek={state.canSeek} onSeek={(ms) => control('seek', ms)} />
                <div className="mt-2 flex items-center justify-center gap-6">
                  <button type="button" disabled={!state.canPrevious} onClick={() => control('previous')} className="grid h-12 w-12 place-items-center rounded-full text-white/90 hover:bg-white/10 disabled:opacity-30" aria-label="Previous track">
                    <SkipBack size={28} fill="currentColor" />
                  </button>
                  <button type="button" disabled={!state.canPlayPause} onClick={() => control('toggle')} className="grid h-16 w-16 place-items-center rounded-full text-white hover:bg-white/10 disabled:opacity-30" aria-label={state.playing ? 'Pause' : 'Play'}>
                    {state.playing ? <Pause size={40} fill="currentColor" /> : <Play size={40} fill="currentColor" className="ml-1" />}
                  </button>
                  <button type="button" disabled={!state.canNext} onClick={() => control('next')} className="grid h-12 w-12 place-items-center rounded-full text-white/90 hover:bg-white/10 disabled:opacity-30" aria-label="Next track">
                    <SkipForward size={28} fill="currentColor" />
                  </button>
                </div>
                {volume && (
                  <div className="mt-3">
                    <VolumeSlider volume={volume} onChange={changeVolume} />
                  </div>
                )}
              </div>
            </div>
            {!synced && (
              <div className="mt-8 max-w-[460px] text-center text-[13.5px] text-white/55">
                {lyrics?.status === 'searching' ? 'Looking for lyrics…' : plain ? null : 'No lyrics for this song.'}
                {plain && <div className="mt-2 max-h-[30vh] overflow-y-auto whitespace-pre-line text-left text-[15px] leading-relaxed text-white/75">{plain}</div>}
              </div>
            )}
          </div>

          {/* Lyrics */}
          {synced && (
            <div className="min-h-0 min-w-0 pb-6 pr-[2vw] pt-16">
              <Lyrics lines={lines} active={active} time={time} canSeek={state.canSeek} onSeek={(ms) => control('seek', Math.max(0, ms - offset))} compact={!full} />
            </div>
          )}
        </div>
      )}
    </div>
  );
}
