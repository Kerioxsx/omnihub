// The PC's sound as the visuals see it. While a view is on screen it holds
// a lease on the core's analysis (renewed every few seconds) and the latest
// `audio:frame` lands in a shared object that renderers read on every
// animation frame, so sound never causes React renders. Without frames
// (paused, nothing playing, capture off or unavailable) the values fall
// back to zero and the visuals idle on their own.

import type { AudioFrame, VisualStatus } from '@shared/types';
import { useEffect, useState } from 'react';
import { api, on } from '../../api';

export interface Levels {
  level: number;
  low: number;
  mid: number;
  high: number;
  beat: number;
  bands: Float32Array;
  /** sound is playing */
  active: boolean;
  bpm: number | null;
}

const latest: { frame: AudioFrame | null; at: number } = { frame: null, at: 0 };
let subscribers = 0;
let unlisten: Promise<() => void> | null = null;

function subscribe(): () => void {
  subscribers += 1;
  if (subscribers === 1) {
    unlisten = on<AudioFrame>('audio:frame', (f) => {
      latest.frame = f;
      latest.at = performance.now();
    });
  }
  return () => {
    subscribers -= 1;
    if (subscribers === 0) {
      void unlisten?.then((u) => u());
      unlisten = null;
      latest.frame = null;
    }
  };
}

/** Listen while `on` (the lease is named `who`); returns the capture status. */
export function useAudioFeed(enabled: boolean, who: string): VisualStatus | null {
  const [status, setStatus] = useState<VisualStatus | null>(null);
  useEffect(() => {
    if (!enabled) return;
    const off = subscribe();
    let alive = true;
    const hold = () =>
      void api.visual.hold(who).then(
        (s) => alive && setStatus(s),
        () => undefined,
      );
    hold();
    const timer = setInterval(hold, 4000);
    const stop = on<VisualStatus>('audio:status', (s) => alive && setStatus(s));
    return () => {
      alive = false;
      clearInterval(timer);
      void stop.then((u) => u());
      void api.visual.release(who).catch(() => undefined);
      off();
    };
  }, [enabled, who]);
  useEffect(() => {
    if (!enabled) setStatus(null);
  }, [enabled]);
  return status;
}

/**
 * Smooths the 60 Hz frames for whatever rate the display runs at, and lets
 * everything settle to zero when frames stop (paused or silent).
 */
export class LevelFollower {
  readonly v: Levels = {
    level: 0,
    low: 0,
    mid: 0,
    high: 0,
    beat: 0,
    bands: new Float32Array(16),
    active: false,
    bpm: null,
  };

  /** `dt` in seconds; `calm` 0–1 holds the response back (reduced motion, paused). */
  step(dt: number, calm = 0): Levels {
    const f = latest.frame;
    const fresh = f != null && performance.now() - latest.at < 400;
    const k = 1 - Math.exp(-dt * (fresh ? 28 : 3));
    const scale = 1 - calm;
    const to = (cur: number, target: number) => cur + (target * scale - cur) * k;
    const v = this.v;
    v.level = to(v.level, fresh ? f.level : 0);
    v.low = to(v.low, fresh ? f.low : 0);
    v.mid = to(v.mid, fresh ? f.mid : 0);
    v.high = to(v.high, fresh ? f.high : 0);
    // Beats arrive as a decaying pulse already; follow it closely.
    v.beat = fresh ? Math.max(f.beat * scale, v.beat * Math.exp(-dt * 7)) : v.beat * Math.exp(-dt * 5);
    for (let i = 0; i < 16; i++) v.bands[i] = to(v.bands[i], fresh ? (f.bands[i] ?? 0) : 0);
    v.active = fresh && f.active;
    v.bpm = fresh ? f.bpm : null;
    return v;
  }
}
