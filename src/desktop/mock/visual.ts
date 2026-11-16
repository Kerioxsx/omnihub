// Pretend sound analysis for the demo: frames that follow the pretend
// player's tempo, pause and track changes, like the core's test signal.

import type { AudioFrame, VisualStatus } from '@shared/types';
import { emit } from './bus';
import { playback } from './music';

const leases = new Map<string, number>();
let timer: ReturnType<typeof setInterval> | null = null;
let status: VisualStatus = { state: 'off', source: null, sampleRate: 0, error: null };
let quietSince = 0;
let beats = 0;
let lastBeat = -1;
let lastTrack = -1;

function setStatus(s: VisualStatus) {
  status = s;
  emit('audio:status', s);
}

function frame(): AudioFrame | null {
  const p = playback();
  if (p.track !== lastTrack) {
    lastTrack = p.track;
    beats = 0;
    lastBeat = -1;
  }
  const t = p.positionMs / 1000;
  if (!p.playing) {
    if (!quietSince) quietSince = Date.now();
    // A heartbeat, then nothing while paused (the core goes quiet too).
    if (Date.now() - quietSince > 600) return null;
    return { t: Date.now(), level: 0, low: 0, mid: 0, high: 0, beat: 0, flux: 0, bands: Array(16).fill(0), active: false, beats, bpm: null };
  }
  quietSince = 0;
  const period = 60 / p.bpm;
  const n = Math.floor(t / period);
  const phase = t - n * period;
  if (n !== lastBeat) {
    lastBeat = n;
    beats += 1;
  }
  const kick = Math.exp(-phase / 0.11);
  const bar = Math.floor(n / 4);
  // Quieter breaks every few bars, so the light breathes with the song.
  const section = 0.75 + 0.25 * Math.sin(bar * 0.9);
  const hat = Math.exp(-((t % (period / 2)) / 0.03));
  const low = Math.min(1, (0.35 + 0.6 * kick) * section);
  const mid = 0.35 + 0.2 * Math.sin(t * 1.7) * section + 0.1 * kick;
  const high = 0.2 + 0.35 * hat * section;
  const bands = Array.from({ length: 16 }, (_, i) => {
    const w = i / 15;
    const v = (1 - w) * low * 0.9 + Math.exp(-((w - 0.45) ** 2) / 0.03) * mid + w * w * high + 0.06 * Math.sin(t * 3 + i);
    return Math.round(Math.max(0, Math.min(1, v)) * 1000) / 1000;
  });
  const r = (v: number) => Math.round(Math.max(0, Math.min(1, v)) * 1000) / 1000;
  return { t: Date.now(), level: r(0.45 + 0.35 * kick * section), low: r(low), mid: r(mid), high: r(high), beat: r(phase < 0.016 ? 1 : kick), flux: r(kick * 0.8), bands, active: true, beats, bpm: p.bpm };
}

function tick() {
  const now = Date.now();
  for (const [who, at] of leases) if (now - at > 10_000) leases.delete(who);
  if (!leases.size) {
    if (timer) clearInterval(timer);
    timer = null;
    setStatus({ state: 'off', source: null, sampleRate: 0, error: null });
    return;
  }
  const f = frame();
  if (f) emit('audio:frame', f);
}

export function visualHold(who: string): VisualStatus {
  leases.set(who, Date.now());
  if (!timer) {
    timer = setInterval(tick, 1000 / 60);
    setStatus({ state: 'listening', source: 'Test signal (demo)', sampleRate: 48_000, error: null });
  }
  return status;
}

export function visualRelease(who: string): void {
  leases.delete(who);
}

export function visualStatus(): VisualStatus {
  return status;
}
