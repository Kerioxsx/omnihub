// The player's side of Aurora's backdrop: the song's music video (asked of
// the core, which looks it up on YouTube once per song), the user's timing
// nudge per video (kept on this PC), and what the cover looks like.

import type { MirrorStatus, MotionStatus, VideoStatus, VideoSync } from '@shared/types';
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

/** The music app's own Canvas or video, mirrored from its window (asked every 1.5 s while wanted). */
export function useMirror(trackKey: string | undefined, wanted: boolean): MirrorStatus | null {
  const [st, setSt] = useState<MirrorStatus | null>(null);
  useEffect(() => {
    if (!wanted) return setSt(null);
    let alive = true;
    // Only a change re-renders the backdrop.
    const ask = () =>
      void api.media.mirror().then(
        (s) => alive && setSt((old) => (old && JSON.stringify(old) === JSON.stringify(s) ? old : s)),
        () => alive && setSt(null),
      );
    ask();
    const t = setInterval(ask, 1500);
    return () => {
      alive = false;
      clearInterval(t);
    };
  }, [wanted, trackKey]);
  return wanted ? st : null;
}

/**
 * Matching the playing music video to the song by ear (media/align.rs):
 * asked once, then again every 3 s while it works (the core stops a job
 * nobody asks about).
 */
export function useVideoSync(trackKey: string | undefined, id: string | undefined, wanted: boolean): { id: string; sync: VideoSync } | null {
  const [st, setSt] = useState<{ key: string; id: string; sync: VideoSync } | null>(null);
  const ask = useCallback(() => {
    if (!wanted || !id || !trackKey) return;
    void api.media.videoSync(id).then(
      (r) => r.key && setSt({ key: r.key, id, sync: r.sync }),
      () => undefined,
    );
  }, [wanted, id, trackKey]);
  useEffect(() => ask(), [ask]);
  useEvent<{ id: string }>('media:videoSync', (p) => p.id === id && ask());
  const working = !!st && st.id === id && st.sync.state === 'matching';
  useEffect(() => {
    if (!working) return;
    const t = setInterval(ask, 3000);
    return () => clearInterval(t);
  }, [working, ask]);
  return wanted && id && st && st.key === trackKey && st.id === id ? { id, sync: st.sync } : null;
}

/** How much each video is nudged by the user (ms; undefined: not nudged), kept on this PC (the last 300 videos). */
export function useVideoOffsets(): [(id: string) => number | undefined, (id: string, ms: number | null) => void, number] {
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
  const get = useCallback((id: string): number | undefined => map.current[id], []);
  // null: back to automatic (the matched offset, else none).
  const set = useCallback((id: string, ms: number | null) => {
    const next = { ...map.current };
    delete next[id];
    if (ms !== null) next[id] = Math.round(ms);
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
