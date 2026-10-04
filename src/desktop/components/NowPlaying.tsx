// What the PC is playing: cover, title, the lyric line being sung and the
// controls. Shown on Home while something plays.

import type { LyricLine, MediaAction, MediaState } from '@shared/types';
import { motion } from 'motion/react';
import { Music2, Pause, Play, SkipBack, SkipForward } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../api';
import { useEvent } from '../lib/hooks';
import { toast } from '../state/toasts';
import { IconButton } from './ui/Button';

function fmt(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

export function NowPlayingCard() {
  const [state, setState] = useState<MediaState | null>(null);
  // Desktop and core share a clock, but keep the same maths as the phone.
  const [offset, setOffset] = useState(0);
  const [art, setArt] = useState<string | null>(null);
  const [lines, setLines] = useState<LyricLine[]>([]);
  const [now, setNow] = useState(() => Date.now());

  const load = useCallback(() => {
    void api.media.state().then((r) => {
      setOffset(r.nowMs - Date.now());
      setState(r.state);
    }, () => undefined);
  }, []);
  useEffect(load, [load]);
  useEvent<{ state: MediaState | null; nowMs: number }>('media:state', (p) => {
    setOffset(p.nowMs - Date.now());
    setState(p.state);
  });

  const key = state?.key;
  const loadLyrics = useCallback(() => {
    if (!key) return setLines([]);
    void api.media.lyrics().then((r) => setLines(r.key === key && r.lyrics.status === 'ready' ? r.lyrics.lyrics.lines : []), () => setLines([]));
  }, [key]);
  useEffect(loadLyrics, [loadLyrics]);
  useEvent<{ key: string }>('media:lyrics', (p) => p.key === key && loadLyrics());

  useEffect(() => {
    if (!state?.art) return setArt(null);
    let alive = true;
    void api.media.art(state.art).then((u) => alive && setArt(u), () => undefined);
    return () => {
      alive = false;
    };
  }, [state?.art]);

  useEffect(() => {
    if (!state?.playing) return;
    const t = setInterval(() => setNow(Date.now()), 250);
    return () => clearInterval(t);
  }, [state?.playing]);

  const pos = !state ? 0 : Math.min(state.durationMs || Infinity, state.playing ? state.positionMs + Math.max(0, now + offset - state.positionAt) * (state.rate || 1) : state.positionMs);
  const line = useMemo(() => {
    let i = -1;
    for (let k = 0; k < lines.length && lines[k].ms <= pos; k++) i = k;
    return i >= 0 ? lines[i].text : null;
  }, [lines, pos]);

  if (!state) return null;
  const act = (a: MediaAction) => () => {
    if (a === 'toggle') setState({ ...state, playing: !state.playing, positionMs: pos, positionAt: Date.now() + offset });
    void api.media.control(a).catch((e: unknown) => toast.error('The player did not respond', errorText(e)));
  };
  return (
    <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} className="card relative flex items-center gap-4 overflow-hidden p-3 pr-4">
      {art && <img src={art} alt="" className="pointer-events-none absolute inset-0 h-full w-full scale-150 object-cover opacity-25 blur-3xl" aria-hidden />}
      <div className="relative shrink-0">
        {art ? <img src={art} alt={`Cover of ${state.title}`} className="h-16 w-16 rounded-xl object-cover shadow-lg" /> : <div className="grid h-16 w-16 place-items-center rounded-xl bg-accent-soft text-accent"><Music2 size={24} /></div>}
      </div>
      <div className="relative min-w-0 flex-1">
        <div className="flex items-baseline gap-2">
          <span className="truncate text-[15px] font-semibold text-fg">{state.title}</span>
          <span className="shrink-0 text-[11.5px] text-faint">{state.appName}</span>
        </div>
        <div className="truncate text-[13px] text-dim">{state.artist}</div>
        <div className="mt-1 truncate text-[13px] font-medium text-fg/90" aria-live="off">{line ? `♪ ${line}` : ' '}</div>
        <div className="mt-1.5 flex items-center gap-2 text-[11px] tabular text-faint">
          <span>{fmt(pos)}</span>
          <div className="h-1 flex-1 overflow-hidden rounded-full bg-surface-3">
            <div className="accent-gradient h-full rounded-full" style={{ width: state.durationMs ? `${(pos / state.durationMs) * 100}%` : '0%' }} />
          </div>
          <span>{fmt(state.durationMs)}</span>
        </div>
      </div>
      <div className="relative flex items-center gap-1">
        <IconButton icon={SkipBack} label="Previous track" disabled={!state.canPrevious} onClick={act('previous')} />
        <IconButton icon={state.playing ? Pause : Play} label={state.playing ? 'Pause' : 'Play'} variant="subtle" disabled={!state.canPlayPause} onClick={act('toggle')} />
        <IconButton icon={SkipForward} label="Next track" disabled={!state.canNext} onClick={act('next')} />
      </div>
    </motion.div>
  );
}
