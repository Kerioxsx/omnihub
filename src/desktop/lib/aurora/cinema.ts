// The player's side of Aurora's backdrop: the song's music video (asked of
// the core, which looks it up on YouTube once per song), the user's timing
// nudge per video (kept on this PC), and what the cover looks like.

import type { VideoStatus } from '@shared/types';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../../api';
import { useEvent } from '../hooks';
import { type CoverLook, coverLook } from './coverLook';

/** The playing song's music video (null until known, or when not wanted). */
export function useMusicVideo(trackKey: string | undefined, wanted: boolean): VideoStatus | null {
  const [st, setSt] = useState<{ key: string; video: VideoStatus } | null>(null);
  const load = useCallback(() => {
    if (!wanted) return;
    void api.media.video().then(
      (r) => r.key && setSt({ key: r.key, video: r.video }),
      () => undefined,
    );
  }, [wanted]);
  useEffect(() => load(), [trackKey, load]);
  useEvent<{ key: string }>('media:video', (p) => p.key === trackKey && load());
  return wanted && st && st.key === trackKey ? st.video : null;
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
