// Music: what the PC is playing — cover art, controls, volume, bass and
// treble — and time-synced lyrics that follow the song.
//
// Sync: every update from the PC carries the track position, the PC time
// it was valid at and the PC clock, so the phone runs the clock itself
// (corrected for the clock difference measured on each request).

import { AnimatePresence, motion } from 'motion/react';
import { ChevronDown, FastForward, Info, ListMusic, Minus, Music2, Pause, Play, Plus, Rewind, SkipBack, SkipForward, SlidersHorizontal, SlidersVertical, Volume1, Volume2, VolumeX } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { type AudioInfo, client, type LyricLine, type LyricsStatus, type MediaAction, type MediaState } from '../client';
import { useEvent } from '../lib/events';
import { keepAwake } from '../lib/wakelock';
import { cx, errorMessage, useInterval, usePageVisible, vibrate } from '../lib/util';
import { toast } from '../state';
import { PageHeader } from '../ui/common';
import { Sheet } from '../ui/Sheet';
import { SoundPanel, useMixer } from './Sound';

const OFFSET_KEY = 'omnihub.lyricsOffset';

function fmt(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

/** PC time = phone time + offset. Keeps the sample with the least network delay. */
function useClock() {
  const best = useRef<{ offset: number; rtt: number } | null>(null);
  const [offset, setOffset] = useState(0);
  const sample = useCallback((pcNow: number, sentAt: number, receivedAt: number) => {
    const rtt = Math.max(0, receivedAt - sentAt);
    const o = pcNow - (sentAt + receivedAt) / 2;
    const b = best.current;
    // A clearly better (faster) sample wins; old samples fade so clock changes are followed.
    if (!b || rtt <= b.rtt + 15 || Math.abs(o - b.offset) > 2000) {
      best.current = { offset: o, rtt };
      setOffset(o);
    } else {
      best.current = { offset: b.offset, rtt: b.rtt + 5 };
    }
  }, []);
  return { offset, sample };
}

function positionOf(s: MediaState | null, pcNow: number): number {
  if (!s) return 0;
  const p = s.playing ? s.positionMs + Math.max(0, pcNow - s.positionAt) * (s.rate || 1) : s.positionMs;
  return Math.max(0, s.durationMs ? Math.min(p, s.durationMs) : p);
}

/** Re-render while playing (for the progress bar and lyrics). */
function useTicker(on: boolean, fps = 20): number {
  const [t, setT] = useState(() => Date.now());
  useEffect(() => {
    if (!on) return;
    let raf = 0;
    let last = 0;
    const step = (now: number) => {
      if (now - last >= 1000 / fps) {
        last = now;
        setT(Date.now());
      }
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [on, fps]);
  return t;
}

function useArt(id: string | null | undefined): string | null {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    if (!id) {
      setUrl(null);
      return;
    }
    let alive = true;
    let made: string | null = null;
    client.mediaArt(id).then(
      (u) => {
        made = u;
        if (alive) setUrl(u);
        else URL.revokeObjectURL(u);
      },
      () => alive && setUrl(null),
    );
    return () => {
      alive = false;
      if (made) URL.revokeObjectURL(made);
    };
  }, [id]);
  return url;
}

function Artwork({ url, title, className }: { url: string | null; title: string; className?: string }) {
  return url ? (
    <img src={url} alt={`Cover of ${title}`} className={cx('aspect-square w-full rounded-2xl object-cover shadow-[0_24px_60px_-18px_rgba(0,0,0,.8)]', className)} draggable={false} />
  ) : (
    <div className={cx('grid aspect-square w-full place-items-center rounded-2xl bg-gradient-to-br from-violet-500/40 to-cyan-500/20 shadow-[0_24px_60px_-18px_rgba(0,0,0,.8)]', className)}>
      <Music2 size={64} className="text-white/60" />
    </div>
  );
}

function Backdrop({ url }: { url: string | null }) {
  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden>
      {url && <img src={url} alt="" className="absolute inset-0 h-full w-full scale-150 object-cover opacity-70 blur-[70px] saturate-150" />}
      <div className="absolute inset-0 bg-gradient-to-b from-black/30 via-black/55 to-black/85" />
    </div>
  );
}

/** Seekable progress bar. */
function Progress({ pos, duration, canSeek, onSeek }: { pos: number; duration: number; canSeek: boolean; onSeek: (ms: number) => void }) {
  const [drag, setDrag] = useState<number | null>(null);
  const bar = useRef<HTMLDivElement>(null);
  const shown = drag ?? pos;
  const at = (clientX: number) => {
    const r = bar.current?.getBoundingClientRect();
    if (!r || !duration) return 0;
    return Math.min(1, Math.max(0, (clientX - r.left) / r.width)) * duration;
  };
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
        className={cx('relative h-6 touch-none', canSeek && 'cursor-pointer')}
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
        onPointerCancel={() => setDrag(null)}
      >
        <div className="absolute inset-x-0 top-1/2 h-1.5 -translate-y-1/2 overflow-hidden rounded-full bg-white/20">
          <div className="h-full rounded-full bg-white/90" style={{ width: duration ? `${(shown / duration) * 100}%` : '0%' }} />
        </div>
        {canSeek && duration > 0 && <div className={cx('absolute top-1/2 h-4 w-4 -translate-x-1/2 -translate-y-1/2 rounded-full bg-white shadow transition-transform', drag != null ? 'scale-125' : 'scale-100')} style={{ left: `${(shown / duration) * 100}%` }} />}
      </div>
      <div className="mt-0.5 flex justify-between font-mono text-[12px] tabular-nums text-white/60">
        <span>{fmt(shown)}</span>
        <span>-{fmt(Math.max(0, duration - shown))}</span>
      </div>
    </div>
  );
}

function Slider({ value, min, max, step, onChange, label, className }: { value: number; min: number; max: number; step: number; onChange: (v: number) => void; label: string; className?: string }) {
  return (
    <input
      type="range"
      aria-label={label}
      min={min}
      max={max}
      step={step}
      value={value}
      onChange={(e) => onChange(Number(e.target.value))}
      className={cx('h-1.5 w-full cursor-pointer appearance-none rounded-full bg-white/20 accent-white', className)}
    />
  );
}

/** A phone turned sideways (not a tablet, which has room for the normal layout). */
const LANDSCAPE = '(orientation: landscape) and (max-height: 540px)';
function useLandscape(): boolean {
  const [on, setOn] = useState(() => typeof window !== 'undefined' && !!window.matchMedia?.(LANDSCAPE).matches);
  useEffect(() => {
    const m = window.matchMedia?.(LANDSCAPE);
    if (!m) return;
    const f = () => setOn(m.matches);
    f();
    m.addEventListener('change', f);
    return () => m.removeEventListener('change', f);
  }, []);
  return on;
}

type Act = (action: MediaAction, positionMs?: number) => void;
type VolumePatch = { level?: number; muted?: boolean };

/** Back 10 s, previous, play/pause, next, forward 10 s. */
function Controls({ state, pos, act, compact }: { state: MediaState; pos: number; act: Act; compact?: boolean }) {
  const play = compact ? 'h-14 w-14' : 'h-[72px] w-[72px]';
  const skip = compact ? 'h-12 w-12' : 'h-14 w-14';
  const big = compact ? 26 : 30;
  return (
    <div className="flex items-center justify-between px-1">
      <button type="button" aria-label="Back 10 seconds" disabled={!state.canSeek} onClick={() => act('seek', Math.max(0, pos - 10000))} className="grid h-11 w-11 place-items-center rounded-full text-white/80 disabled:opacity-30">
        <Rewind size={compact ? 20 : 22} fill="currentColor" />
      </button>
      <button type="button" aria-label="Previous track" disabled={!state.canPrevious} onClick={() => act('previous')} className={cx('grid place-items-center rounded-full disabled:opacity-30', skip)}>
        <SkipBack size={big} fill="currentColor" />
      </button>
      <button type="button" aria-label={state.playing ? 'Pause' : 'Play'} disabled={!state.canPlayPause} onClick={() => act('toggle')} className={cx('grid place-items-center rounded-full bg-white text-black shadow-lg transition-transform active:scale-95 disabled:opacity-40', play)}>
        {state.playing ? <Pause size={compact ? 26 : 34} fill="currentColor" /> : <Play size={compact ? 26 : 34} fill="currentColor" className="translate-x-0.5" />}
      </button>
      <button type="button" aria-label="Next track" disabled={!state.canNext} onClick={() => act('next')} className={cx('grid place-items-center rounded-full disabled:opacity-30', skip)}>
        <SkipForward size={big} fill="currentColor" />
      </button>
      <button type="button" aria-label="Forward 10 seconds" disabled={!state.canSeek} onClick={() => act('seek', Math.min(state.durationMs || Infinity, pos + 10000))} className="grid h-11 w-11 place-items-center rounded-full text-white/80 disabled:opacity-30">
        <FastForward size={compact ? 20 : 22} fill="currentColor" />
      </button>
    </div>
  );
}

/** The PC's volume: mute button, slider, percentage (and optionally the mixer). */
function VolumeRow({ volume, onChange, onMixer }: { volume: { level: number; muted: boolean }; onChange: (p: VolumePatch) => void; onMixer?: () => void }) {
  const shown = volume.muted ? 0 : volume.level;
  return (
    <div className="flex items-center gap-3">
      <button type="button" aria-label={volume.muted ? 'Unmute' : 'Mute'} onClick={() => onChange({ muted: !volume.muted })} className="text-white/70">
        {volume.muted || volume.level === 0 ? <VolumeX size={20} /> : volume.level < 0.5 ? <Volume1 size={20} /> : <Volume2 size={20} />}
      </button>
      <Slider label="Volume" min={0} max={1} step={0.01} value={shown} onChange={(level) => onChange({ level, muted: false })} />
      <span className="w-9 text-right font-mono text-[12px] tabular-nums text-white/60">{Math.round(shown * 100)}</span>
      {onMixer && (
        <button type="button" onClick={onMixer} className="grid h-9 w-9 shrink-0 place-items-center rounded-full bg-white/10 text-white/80" aria-label="Volume of each app, microphone and calls">
          <SlidersVertical size={17} />
        </button>
      )}
    </div>
  );
}

/** Full-screen lyrics, Apple Music style: the line being sung bright, the rest dim, following the song. */
function LyricsView({ lines, pos, act, onClose, art, state, offset, setOffset, volume, onVolume, landscape }: { lines: LyricLine[]; pos: number; act: Act; onClose: () => void; art: string | null; state: MediaState; offset: number; setOffset: (o: number) => void; volume: { level: number; muted: boolean } | null; onVolume: (p: VolumePatch) => void; landscape: boolean }) {
  const canSeek = state.canSeek;
  const onSeek = (ms: number) => act('seek', ms);
  const t = pos + offset;
  const active = useMemo(() => {
    let lo = 0;
    let hi = lines.length;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (lines[mid].ms <= t) lo = mid + 1;
      else hi = mid;
    }
    return lo - 1;
  }, [lines, t]);
  const box = useRef<HTMLDivElement>(null);
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const [userScroll, setUserScroll] = useState(0);
  // Keep the active line about a third of the way down.
  useEffect(() => {
    if (Date.now() - userScroll < 4000) return;
    const el = refs.current[Math.max(0, active)];
    const b = box.current;
    if (!el || !b) return;
    b.scrollTo({ top: el.offsetTop - b.clientHeight * 0.32, behavior: 'smooth' });
  }, [active, userScroll]);
  useEffect(() => {
    keepAwake('lyrics', true);
    return () => keepAwake('lyrics', false);
  }, []);
  // An instrumental break: dots before the next line.
  const next = lines[active + 1];
  const gap = next && (active < 0 ? next.ms : next.ms - lines[active].ms) > 7000 && next.ms - t > 2500 && (active < 0 || t - lines[active].ms > 4000);
  const closeButton = (
    <button type="button" onClick={onClose} className="grid h-10 w-10 shrink-0 place-items-center rounded-full bg-white/10" aria-label="Close lyrics">
      <ChevronDown size={22} />
    </button>
  );
  const timing = (
    <div className="flex shrink-0 items-center gap-1 rounded-full bg-white/10 px-1.5 py-1 text-[11px]" aria-label="Lyrics timing">
      <button type="button" className="grid h-7 w-7 place-items-center" onClick={() => setOffset(offset - 250)} aria-label="Lyrics later">
        <Minus size={14} />
      </button>
      <span className="w-12 text-center font-mono tabular-nums">{offset === 0 ? 'sync' : `${offset > 0 ? '+' : ''}${(offset / 1000).toFixed(2)}`}</span>
      <button type="button" className="grid h-7 w-7 place-items-center" onClick={() => setOffset(offset + 250)} aria-label="Lyrics earlier">
        <Plus size={14} />
      </button>
    </div>
  );
  const player = (
    <>
      <Progress pos={pos} duration={state.durationMs} canSeek={canSeek} onSeek={onSeek} />
      <div className="mt-1">
        <Controls state={state} pos={pos} act={act} compact />
      </div>
      {volume && (
        <div className="mt-1">
          <VolumeRow volume={volume} onChange={onVolume} />
        </div>
      )}
    </>
  );
  return (
    <motion.div initial={{ opacity: 0, y: 40 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: 40 }} transition={{ type: 'spring', damping: 30, stiffness: 320 }} className={cx('fixed inset-0 z-50 flex bg-black text-white', landscape ? 'flex-row' : 'flex-col')}>
      <Backdrop url={art} />
      {landscape ? (
        <aside className="relative flex w-[42%] max-w-[420px] shrink-0 flex-col justify-center gap-2.5 pr-6" style={{ paddingLeft: 'max(20px, var(--safe-left))', paddingTop: 'calc(var(--safe-top) + 10px)', paddingBottom: 'calc(var(--safe-bottom) + 10px)' }}>
          <div className="flex items-center justify-between gap-2">
            {closeButton}
            {timing}
          </div>
          <div className="flex items-center gap-3.5">
            <div className="w-[min(28vh,112px)] shrink-0">
              <Artwork url={art} title={state.title} className="rounded-xl" />
            </div>
            <div className="min-w-0">
              <div className="line-clamp-2 font-display text-[18px] font-bold leading-tight">{state.title}</div>
              <div className="truncate text-[14px] text-white/60">{state.artist}</div>
            </div>
          </div>
          <div>{player}</div>
        </aside>
      ) : (
        <div className="relative flex items-center gap-3 px-5 pb-2" style={{ paddingTop: 'calc(var(--safe-top) + 10px)' }}>
          {closeButton}
          <div className="min-w-0 flex-1">
            <div className="truncate text-[15px] font-semibold">{state.title}</div>
            <div className="truncate text-[13px] text-white/60">{state.artist}</div>
          </div>
          {timing}
        </div>
      )}
      <div className="relative flex min-h-0 min-w-0 flex-1 flex-col">
      <div
        ref={box}
        className={cx('relative flex-1 overflow-y-auto [mask-image:linear-gradient(to_bottom,transparent,black_12%,black_80%,transparent)]', landscape ? 'pb-[50vh] pl-2 pt-[24vh]' : 'px-6 pb-[40vh] pt-[16vh]')}
        style={landscape ? { paddingRight: 'max(24px, var(--safe-right))' } : undefined}
        onTouchMove={() => setUserScroll(Date.now())}
        onWheel={() => setUserScroll(Date.now())}
      >
        {lines.map((l, i) => {
          const isActive = i === active;
          const dist = Math.abs(i - active);
          // Word-by-word fill when the file times words.
          const words = isActive && l.words?.length ? l.words : null;
          return (
            <button
              key={`${l.ms}-${i}`}
              ref={(el) => {
                refs.current[i] = el;
              }}
              type="button"
              onClick={() => {
                if (!canSeek) return;
                vibrate(6);
                setUserScroll(0);
                onSeek(Math.max(0, l.ms - offset));
              }}
              className={cx('block w-full origin-left py-2.5 text-left font-display font-extrabold leading-[1.18] tracking-tight transition-all duration-500 ease-out', landscape ? 'text-[27px]' : 'text-[30px]', isActive ? 'scale-100 text-white' : 'scale-[0.97] text-white/30')}
              style={{ filter: !isActive && dist > 1 ? `blur(${Math.min(2.4, (dist - 1) * 0.7)}px)` : undefined }}
            >
              {words
                ? words.map((w, wi) => {
                    const end = l.words?.[wi + 1]?.ms ?? next?.ms ?? w.ms + 600;
                    const f = Math.min(1, Math.max(0, (t - w.ms) / Math.max(1, end - w.ms)));
                    return (
                      <span key={wi} className="bg-clip-text text-transparent" style={{ backgroundImage: `linear-gradient(90deg, #fff ${f * 100}%, rgba(255,255,255,.35) ${f * 100}%)` }}>
                        {w.text}
                      </span>
                    );
                  })
                : l.text || '♪'}
            </button>
          );
        })}
        {gap && (
          <div className={cx('pointer-events-none absolute flex gap-2', landscape ? 'left-2' : 'left-6')} style={{ top: (refs.current[Math.max(0, active)]?.offsetTop ?? 0) + 64 }} aria-hidden>
            {[0, 1, 2].map((d) => (
              <motion.span key={d} className="h-3 w-3 rounded-full bg-white" animate={{ opacity: [0.25, 1, 0.25], scale: [0.85, 1.1, 0.85] }} transition={{ duration: 1.6, repeat: Infinity, delay: d * 0.25 }} />
            ))}
          </div>
        )}
      </div>
      {Date.now() - userScroll < 4000 && (
        <button type="button" onClick={() => setUserScroll(0)} className={cx('absolute left-1/2 -translate-x-1/2 rounded-full bg-white/15 px-4 py-2 text-[13px] font-semibold backdrop-blur', landscape ? 'bottom-[calc(var(--safe-bottom)+16px)]' : 'bottom-4')}>
          Back to the song
        </button>
      )}
      </div>
      {!landscape && (
        <div className="relative px-6 pt-1" style={{ paddingBottom: 'calc(var(--safe-bottom) + 12px)' }}>
          {player}
        </div>
      )}
    </motion.div>
  );
}

export function MusicScreen({ active }: { active: boolean }) {
  const [state, setState] = useState<MediaState | null>(null);
  const [audio, setAudio] = useState<AudioInfo | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [lyrics, setLyrics] = useState<LyricsStatus | null>(null);
  const [showLyrics, setShowLyrics] = useState(false);
  const [showEq, setShowEq] = useState(false);
  const [showMixer, setShowMixer] = useState(false);
  const landscape = useLandscape();
  const mixer = useMixer(active && showMixer);
  const [offset, setOffsetState] = useState(() => Number(localStorage.getItem(OFFSET_KEY) ?? 0) || 0);
  const clock = useClock();
  const visible = usePageVisible();
  const setOffset = (o: number) => {
    const v = Math.max(-5000, Math.min(5000, o));
    setOffsetState(v);
    try {
      localStorage.setItem(OFFSET_KEY, String(v));
    } catch {
      /* private mode */
    }
  };

  const refresh = useCallback(async () => {
    const sent = Date.now();
    try {
      const r = await client.media();
      clock.sample(r.nowMs, sent, Date.now());
      setState(r.state);
      setAudio(r.audio);
    } catch (e) {
      if (!loaded) toast.error('Could not reach the music on the PC', errorMessage(e));
    } finally {
      setLoaded(true);
    }
  }, [clock, loaded]);

  useEffect(() => {
    if (active && visible) void refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, visible]);
  useInterval(() => void refresh(), active && visible ? 30000 : null);
  useEvent<{ state: MediaState | null; nowMs: number }>('media:state', (p) => {
    // Pushed events have no round trip to measure: only take them as clock
    // samples when nothing better is known yet.
    if (!loaded) clock.sample(p.nowMs, Date.now(), Date.now());
    setState(p.state);
  });

  // Lyrics for the current track.
  const key = state?.key ?? null;
  const loadLyrics = useCallback(async () => {
    if (!key) return setLyrics(null);
    try {
      const r = await client.mediaLyrics();
      if (r.key === key) setLyrics(r.lyrics);
    } catch {
      setLyrics({ status: 'none' });
    }
  }, [key]);
  useEffect(() => {
    setLyrics(null);
    void loadLyrics();
  }, [loadLyrics]);
  useEvent<{ key: string }>('media:lyrics', (p) => p.key === key && void loadLyrics());
  useInterval(() => void loadLyrics(), lyrics?.status === 'searching' ? 1500 : null);

  const now = useTicker(active && visible && !!state?.playing, showLyrics ? 30 : 8);
  const pos = positionOf(state, now + clock.offset);
  const art = useArt(state?.art);
  const lines = lyrics?.status === 'ready' ? lyrics.lyrics.lines : [];
  const current = useMemo(() => {
    const t = pos + offset;
    let idx = -1;
    for (let i = 0; i < lines.length && lines[i].ms <= t; i++) idx = i;
    return idx;
  }, [lines, pos, offset]);

  const act = async (action: MediaAction, positionMs = 0) => {
    vibrate(8);
    // Show it at once; the PC confirms with an update.
    if (state) {
      const p = positionOf(state, Date.now() + clock.offset);
      const pcNow = Date.now() + clock.offset;
      if (action === 'toggle') setState({ ...state, playing: !state.playing, positionMs: p, positionAt: pcNow });
      if (action === 'seek') setState({ ...state, positionMs, positionAt: pcNow });
    }
    try {
      await client.mediaControl(action, positionMs);
    } catch (e) {
      toast.error('The player did not respond', errorMessage(e));
      void refresh();
    }
  };

  const setAudioPart = async (patch: Parameters<typeof client.mediaAudio>[0]) => {
    if (audio) setAudio({ ...audio, volume: audio.volume && { level: patch.level ?? audio.volume.level, muted: patch.muted ?? audio.volume.muted }, eq: { ...audio.eq, bass: patch.bass ?? audio.eq.bass, treble: patch.treble ?? audio.eq.treble, enabled: patch.eqEnabled ?? audio.eq.enabled } });
    try {
      setAudio(await client.mediaAudio(patch));
    } catch (e) {
      toast.error('Could not change the sound', errorMessage(e));
    }
  };
  // Volume drags send at most a few requests a second.
  const volTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const eqTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const debounced = (timer: typeof volTimer, patch: Parameters<typeof client.mediaAudio>[0], apply: () => void) => {
    apply();
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => void setAudioPart(patch), 180);
  };

  const v = audio?.volume;
  const eq = audio?.eq;
  const onVolume = (p: VolumePatch) => {
    if (p.level != null) {
      const level = p.level;
      debounced(volTimer, { level, muted: false }, () => setAudio(audio && { ...audio, volume: { level, muted: false } }));
    } else if (p.muted != null) void setAudioPart({ muted: p.muted });
  };
  const doAct: Act = (a, ms) => void act(a, ms);

  // Lyrics preview: tap for the full view.
  const lyricsCard = state && (
    <>
      <button type="button" onClick={() => setShowLyrics(true)} disabled={!lines.length} className="mt-6 block w-full rounded-3xl bg-white/10 p-5 text-left backdrop-blur-xl transition-colors active:bg-white/15 disabled:active:bg-white/10">
        <div className="mb-2 flex items-center justify-between text-[12px] font-semibold uppercase tracking-wider text-white/55">
          <span>Lyrics</span>
          {lines.length > 0 && <span className="normal-case tracking-normal">Tap to follow along</span>}
        </div>
        {lyrics?.status === 'ready' && lines.length ? (
          <div className="space-y-1.5">
            {[current, current + 1].map((i, k) =>
              i >= 0 && i < lines.length ? (
                <p key={`${i}-${k}`} className={cx('font-display text-[21px] font-bold leading-snug', k === 0 ? 'text-white' : 'text-white/40')}>
                  {lines[i].text || '♪'}
                </p>
              ) : k === 0 ? (
                <p key="pre" className="font-display text-[21px] font-bold text-white/60">♪</p>
              ) : null,
            )}
          </div>
        ) : (
          <p className="text-[15px] text-white/60">
            {lyrics == null || lyrics.status === 'searching'
              ? 'Finding lyrics…'
              : lyrics.status === 'off'
                ? 'Online lyrics are turned off on the PC (Settings → Music).'
                : lyrics.status === 'ready' && lyrics.lyrics.instrumental
                  ? 'Instrumental — no lyrics.'
                  : lyrics.status === 'ready' && lyrics.lyrics.plain
                    ? lyrics.lyrics.plain.split('\n').slice(0, 3).join(' / ') + ' … (not time-synced)'
                    : 'No lyrics found for this song.'}
          </p>
        )}
        {state.positionSource === 'estimated' && lines.length > 0 && (
          <p className="mt-3 flex items-start gap-1.5 text-[12px] text-white/50">
            <Info size={13} className="mt-px shrink-0" /> {state.appName} doesn't share its exact position, so timing is estimated from when the song started. Use the ± buttons in the lyrics view if it drifts.
          </p>
        )}
      </button>
    </>
  );
  // Bass and treble.
  const eqCard = (
    <>
      <div className="mt-4 rounded-3xl bg-white/10 p-5 backdrop-blur-xl">
        <button type="button" onClick={() => setShowEq(!showEq)} className="flex w-full items-center gap-3 text-left">
          <SlidersHorizontal size={18} className="text-white/70" />
          <span className="flex-1 text-[15px] font-semibold">Bass &amp; treble</span>
          {eq?.status.available && <span className={cx('rounded-full px-2.5 py-0.5 text-[12px] font-semibold', eq.enabled ? 'bg-white text-black' : 'bg-white/15 text-white/70')}>{eq.enabled ? 'On' : 'Off'}</span>}
          <ChevronDown size={18} className={cx('text-white/60 transition-transform', showEq && 'rotate-180')} />
        </button>
        <AnimatePresence initial={false}>
          {showEq && (
            <motion.div initial={{ height: 0, opacity: 0 }} animate={{ height: 'auto', opacity: 1 }} exit={{ height: 0, opacity: 0 }} className="overflow-hidden">
              {eq?.status.available ? (
                <div className="space-y-4 pt-4">
                  <label className="flex items-center justify-between text-[14px]">
                    Equaliser
                    <input type="checkbox" checked={eq.enabled} onChange={(e) => void setAudioPart({ eqEnabled: e.target.checked })} className="h-5 w-5 accent-white" />
                  </label>
                  {(['bass', 'treble'] as const).map((band) => (
                    <div key={band}>
                      <div className="mb-1 flex justify-between text-[13px] text-white/70">
                        <span className="capitalize">{band}</span>
                        <span className="font-mono tabular-nums">{eq[band] > 0 ? '+' : ''}{eq[band].toFixed(1)} dB</span>
                      </div>
                      <Slider label={band === 'bass' ? 'Bass' : 'Treble'} min={-eq.maxDb} max={eq.maxDb} step={0.5} value={eq[band]} onChange={(val) => debounced(eqTimer, { [band]: val, eqEnabled: true }, () => setAudio(audio && { ...audio, eq: { ...audio.eq, [band]: val, enabled: true } }))} />
                    </div>
                  ))}
                  <button type="button" onClick={() => void setAudioPart({ bass: 0, treble: 0 })} className="text-[13px] font-semibold text-white/70 underline-offset-2 active:underline">
                    Reset to flat
                  </button>
                </div>
              ) : (
                <p className="pt-3 text-[14px] leading-relaxed text-white/65">Windows has no bass control of its own. Install the free <b className="text-white">Equalizer APO</b> on the PC (choose your speakers or headphones during setup), then come back — the sliders appear here.</p>
              )}
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </>
  );

  const details = state && (
    <>
      <div className={cx('flex items-end gap-3', !landscape && 'mt-6')}>
        <div className="min-w-0 flex-1">
          {landscape && <div className="truncate text-[13px] text-white/55">Playing in {state.appName}</div>}
          <h2 className="truncate font-display text-[23px] font-bold leading-tight">{state.title}</h2>
          <p className="truncate text-[17px] text-white/65">{[state.artist, state.album].filter(Boolean).join(' — ')}</p>
        </div>
      </div>
      <div className="mt-4">
        <Progress pos={pos} duration={state.durationMs} canSeek={state.canSeek} onSeek={(ms) => void act('seek', ms)} />
      </div>
      <div className="mt-3">
        <Controls state={state} pos={pos} act={doAct} compact={landscape} />
      </div>
      {v ? (
        <div className="mt-5">
          <VolumeRow volume={v} onChange={onVolume} onMixer={() => setShowMixer(true)} />
        </div>
      ) : (
        <button type="button" onClick={() => setShowMixer(true)} className="mt-5 flex items-center gap-2 text-[14px] font-semibold text-white/75">
          <SlidersVertical size={17} /> Volume of each app &amp; calls
        </button>
      )}
      {lyricsCard}
      {eqCard}
    </>
  );

  return (
    <div className="relative h-full overflow-hidden bg-black text-white">
      <Backdrop url={art} />
      {state && landscape ? (
        <div className="relative flex h-full items-center gap-7" style={{ paddingLeft: 'max(20px, var(--safe-left))', paddingRight: 'max(20px, var(--safe-right))', paddingTop: 'calc(var(--safe-top) + 10px)', paddingBottom: 'calc(var(--tabbar-h) + var(--safe-bottom) + 10px)' }}>
          <motion.div key={state.key} initial={{ opacity: 0, scale: 0.96 }} animate={{ opacity: 1, scale: state.playing ? 1 : 0.92 }} transition={{ type: 'spring', damping: 22, stiffness: 220 }} className="aspect-square h-full max-h-[320px] shrink-0">
            <Artwork url={art} title={state.title} className="h-full" />
          </motion.div>
          <div className="h-full min-w-0 flex-1 overflow-y-auto py-1 pr-1">{details}</div>
        </div>
      ) : (
        <div className="relative h-full overflow-y-auto pb-[calc(var(--tabbar-h)+var(--safe-bottom)+24px)]">
          <PageHeader title="Music" subtitle={state ? `Playing in ${state.appName}` : 'From your PC'} />
          {!state ? (
            <div className="flex flex-col items-center px-8 pt-16 text-center">
              <div className="grid h-24 w-24 place-items-center rounded-3xl bg-white/10">
                <ListMusic size={40} className="text-white/70" />
              </div>
              <h2 className="mt-6 font-display text-[22px] font-bold">{loaded ? 'Nothing is playing' : 'Looking for music…'}</h2>
              <p className="mt-2 text-[15px] text-white/60">Play something on the PC — Spotify, Apple Music, YouTube Music in a browser, or any app that shows in Windows' media controls.</p>
              {loaded && (
                <button type="button" onClick={() => void act('play')} className="mt-6 flex items-center gap-2 rounded-full bg-white px-6 py-3 text-[15px] font-semibold text-black">
                  <Play size={18} fill="currentColor" /> Resume on the PC
                </button>
              )}
            </div>
          ) : (
            <div className="mx-auto max-w-[460px] px-6">
              <motion.div key={state.key} initial={{ opacity: 0, scale: 0.96 }} animate={{ opacity: 1, scale: state.playing ? 1 : 0.92 }} transition={{ type: 'spring', damping: 22, stiffness: 220 }} className="mx-auto mt-2 w-[min(100%,340px)]">
                <Artwork url={art} title={state.title} />
              </motion.div>
              {details}
            </div>
          )}
        </div>
      )}
      <AnimatePresence>
        {showLyrics && state && lines.length > 0 && <LyricsView lines={lines} pos={pos} act={doAct} onClose={() => setShowLyrics(false)} art={art} state={state} offset={offset} setOffset={setOffset} volume={v ?? null} onVolume={onVolume} landscape={landscape} />}
      </AnimatePresence>
      <Sheet open={showMixer} onClose={() => setShowMixer(false)} title="Volume & calls" subtitle="Each app on the PC, your microphone and calls">
        <SoundPanel api={mixer} />
      </Sheet>
    </div>
  );
}
