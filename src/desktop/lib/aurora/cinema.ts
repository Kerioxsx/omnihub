// The player's side of Aurora's backdrop: the song's music video (asked of
// the core, which looks it up on YouTube once per song), the user's timing
// nudge per video (kept on this PC), and what the cover looks like.

import type { MotionStatus, VideoStatus } from '@shared/types';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../api';
import { useEvent } from '../hooks';
import { type CoverLook, coverLook } from './coverLook';

/**
 * Asks the core about the playing song (`ask`) until it has an answer for
 * that song: when told it is ready (`topic`), and again every 1.5 s while the
 * answer is "searching" or for another song, so a missed event or a song
 * that changed mid-question never leaves it waiting.
 */
function useSongLookup<S extends { status: string }>(trackKey: string | undefined, wanted: boolean, topic: string, ask: () => Promise<{ key: string | null; value: S }>): S | null {
  const [st, setSt] = useState<{ key: string; value: S } | null>(null);
  const asker = useRef(ask);
  asker.current = ask;
  const load = useCallback(() => {
    if (!wanted || !trackKey) return;
    void asker.current().then(
      (r) => r.key && setSt({ key: r.key, value: r.value }),
      () => undefined,
    );
  }, [wanted, trackKey]);
  useEffect(() => load(), [load]);
  useEvent<{ key: string }>(topic, (p) => p.key === trackKey && load());
  const settled = !!st && st.key === trackKey && st.value.status !== 'searching';
  useEffect(() => {
    if (!wanted || !trackKey || settled) return;
    let tries = 0;
    const timer = setInterval(() => (++tries > 80 ? clearInterval(timer) : load()), 1500);
    return () => clearInterval(timer);
  }, [wanted, trackKey, settled, load]);
  return wanted && st && st.key === trackKey ? st.value : null;
}

/** The playing song's music video (null until known, or when not wanted). */
export function useMusicVideo(trackKey: string | undefined, wanted: boolean): VideoStatus | null {
  return useSongLookup(trackKey, wanted, 'media:video', () => api.media.video().then((r) => ({ key: r.key, value: r.video })));
}

/** The playing song's animated album cover (null until known, or when not wanted). */
export function useMotionCover(trackKey: string | undefined, wanted: boolean): MotionStatus | null {
  return useSongLookup(trackKey, wanted, 'media:motion', () => api.media.motion().then((r) => ({ key: r.key, value: r.motion })));
}

const OFFSETS = 'omnihub.videoOffsets';

/** How much each video is nudged (ms), kept on this PC (the last 300 videos). */
export function useVideoOffsets(): [(id: string) => number, (id: string, ms: number) => void, number] {
  const map = useRef<Record<string, number>>({});
  const [version, setVersion] = useState(0);
  useEffect(() => {
    try {
      map.current = JSON.parse(localStorage.getItem(OFFSETS) ?? '{}') ?? {};
    } catch {
      map.current = {};
    }
    setVersion((v) => v + 1);
  }, []);
  const get = useCallback((id: string) => map.current[id] ?? 0, []);
  const set = useCallback((id: string, ms: number) => {
    const next = { ...map.current };
    delete next[id];
    if (ms !== 0) next[id] = Math.round(ms);
    const keys = Object.keys(next);
    for (const k of keys.slice(0, Math.max(0, keys.length - 300))) delete next[k];
    map.current = next;
    try {
      localStorage.setItem(OFFSETS, JSON.stringify(next));
    } catch {
      /* this session only */
    }
    setVersion((v) => v + 1);
  }, []);
  return [get, set, version];
}

/** What the cover looks like (null until studied, or with no cover). */
export function useCoverLook(art: string | null): CoverLook | null {
  const [look, setLook] = useState<{ art: string; look: CoverLook } | null>(null);
  useEffect(() => {
    if (!art) return;
    let alive = true;
    coverLook(art).then(
      (l) => alive && setLook({ art, look: l }),
      () => undefined,
    );
    return () => {
      alive = false;
    };
  }, [art]);
  // The last cover's look until the new one is studied (no flash of defaults).
  return art ? (look?.look ?? null) : null;
}
