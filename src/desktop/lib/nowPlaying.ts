// What the PC plays, for the Home card and the full-screen player: the
// track, its position (run locally between updates), cover art, lyrics and
// the volume. Desktop and core share a clock, but the maths follows the
// phone's so a small offset doesn't matter.

import type { LyricLine, LyricsStatus, MediaAction, MediaState } from '@shared/types';
import { useCallback, useEffect, useRef, useState } from 'react';
import { create } from 'zustand';
import { api, errorText, inTauri } from '../api';
import { toast } from '../state/toasts';
import { useEvent } from './hooks';

export function positionOf(s: MediaState | null, pcNow: number): number {
  if (!s) return 0;
  const p = s.playing ? s.positionMs + Math.max(0, pcNow - s.positionAt) * (s.rate || 1) : s.positionMs;
  return Math.max(0, s.durationMs ? Math.min(p, s.durationMs) : p);
}

export function fmtTime(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

/** Index of the line being sung at `t` (−1 before the first). */
export function activeLine(lines: LyricLine[], t: number): number {
  let lo = 0;
  let hi = lines.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (lines[mid].ms <= t) lo = mid + 1;
    else hi = mid;
  }
  return lo - 1;
}

export function useNowPlaying() {
  const [state, setState] = useState<MediaState | null>(null);
  const [loaded, setLoaded] = useState(false);
  const offset = useRef(0);
  const [art, setArt] = useState<string | null>(null);
  const [lyrics, setLyrics] = useState<LyricsStatus | null>(null);

  useEffect(() => {
    void api.media.state().then(
      (r) => {
        offset.current = r.nowMs - Date.now();
        setState(r.state);
        setLoaded(true);
      },
      () => setLoaded(true),
    );
  }, []);
  useEvent<{ state: MediaState | null; nowMs: number }>('media:state', (p) => {
    offset.current = p.nowMs - Date.now();
    setState(p.state);
  });

  const key = state?.key;
  const loadLyrics = useCallback(() => {
    if (!key) return setLyrics(null);
    void api.media.lyrics().then((r) => r.key === key && setLyrics(r.lyrics), () => setLyrics(null));
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

  /** The track position right now. */
  const position = useCallback(() => positionOf(state, Date.now() + offset.current), [state]);

  const control = useCallback(
    (a: MediaAction, ms = 0) => {
      if (!state) return;
      const pcNow = Date.now() + offset.current;
      // Answer the click at once; the player's own update follows.
      if (a === 'toggle') setState({ ...state, playing: !state.playing, positionMs: positionOf(state, pcNow), positionAt: pcNow });
      if (a === 'seek') setState({ ...state, positionMs: ms, positionAt: pcNow });
      void api.media.control(a, ms).catch((e: unknown) => toast.error('The player did not respond', errorText(e)));
    },
    [state],
  );

  const lines = lyrics?.status === 'ready' ? lyrics.lyrics.lines : [];
  return { state, loaded, art, lyrics, lines, position, control };
}

/** Re-render on every animation frame (or at `fps`) while `on`. */
export function useFrame(on: boolean, fps = 60): number {
  const [t, setT] = useState(0);
  useEffect(() => {
    if (!on) return;
    let raf = 0;
    let last = 0;
    const step = (now: number) => {
      if (now - last >= 1000 / fps - 1) {
        last = now;
        setT(now);
      }
      raf = requestAnimationFrame(step);
    };
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }, [on, fps]);
  return t;
}

// ---------- the full-screen player ----------

export const usePlayer = create<{ open: boolean; show: () => void; hide: () => void }>((set) => ({
  open: false,
  show: () => set({ open: true }),
  hide: () => set({ open: false }),
}));

/** The OmniHub window itself in full screen (the browser page in the demo). */
export async function setWindowFullscreen(on: boolean): Promise<void> {
  try {
    if (inTauri) {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().setFullscreen(on);
    } else if (on && !document.fullscreenElement) {
      await document.documentElement.requestFullscreen?.();
    } else if (!on && document.fullscreenElement) {
      await document.exitFullscreen?.();
    }
  } catch {
    /* stays windowed */
  }
}
