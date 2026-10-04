// A pretend music player for the mock backend (same tracks as the Rust
// test player), so Now Playing and lyrics can be developed without Windows.

import type { LyricsStatus, MediaAction, MediaState } from '@shared/types';
import { emit } from './bus';

// Made-up songs with original lyrics, for the demo only.
const LYRICS = [
  'City lights are fading into blue',
  'I keep the engine running just for you',
  'Every mile a little closer to the sun',
  'We were never made to be the only one',
  'Turn it up, the night is ours to keep',
  "Echoes on the highway, we don't sleep",
  'Hold the moment, let the chorus fall',
  "Daylight's coming, but we've got it all",
  'Paper planes above the parking lot',
  'Every promise that we never bought',
  'Radio is singing what we mean',
  'Somewhere in the static, in between',
];

const TRACKS = [
  { title: 'Neon Afterglow', artist: 'Midnight Atlas', album: 'Open Roads', durationMs: 200_000, hue: ['#ff7a3c', '#2a1440'] },
  { title: 'Night Loop', artist: 'Midnight Atlas', album: 'Open Roads', durationMs: 180_000, hue: ['#465aff', '#140a28'] },
];
let index = 0;
let base = 0;
let at = Date.now();
let playing = true;

function position(): number {
  const p = playing ? base + (Date.now() - at) : base;
  return Math.min(p, TRACKS[index].durationMs);
}

function state(): MediaState {
  if (position() >= TRACKS[index].durationMs) {
    index = (index + 1) % TRACKS.length;
    base = 0;
    at = Date.now();
  }
  const t = TRACKS[index];
  return {
    key: `mock\u001f${t.title}`,
    app: 'Spotify.exe',
    appName: 'Spotify',
    title: t.title,
    artist: t.artist,
    album: t.album,
    durationMs: t.durationMs,
    positionMs: position(),
    positionAt: Date.now(),
    playing,
    rate: 1,
    positionSource: 'player',
    canPlayPause: true,
    canNext: true,
    canPrevious: true,
    canSeek: true,
    art: `art-${index}`,
  };
}

const announce = () => emit('media:state', { state: state(), nowMs: Date.now() });

export function mediaState() {
  return { state: state(), nowMs: Date.now() };
}

export function mediaControl(action: MediaAction, positionMs: number): void {
  const p = position();
  if (action === 'toggle') {
    base = p;
    at = Date.now();
    playing = !playing;
  } else if (action === 'play' || action === 'pause') {
    base = p;
    at = Date.now();
    playing = action === 'play';
  } else if (action === 'next' || action === 'previous') {
    index = action === 'next' || p < 3000 ? (index + (action === 'next' ? 1 : TRACKS.length - 1)) % TRACKS.length : index;
    base = 0;
    at = Date.now();
  } else if (action === 'seek') {
    base = Math.min(positionMs, TRACKS[index].durationMs);
    at = Date.now();
  }
  announce();
}

export function mediaLyrics(): { key: string; lyrics: LyricsStatus } {
  const s = state();
  const lines = Array.from({ length: 40 }, (_, i) => ({ ms: i * 4000 + 2000, text: LYRICS[i % LYRICS.length] }));
  return { key: s.key, lyrics: { status: 'ready', lyrics: { lines, plain: null, instrumental: false, source: 'mock' } } };
}

export function mediaArt(id: string): string {
  const t = TRACKS[Number(id.split('-')[1]) || 0];
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 300"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${t.hue[0]}"/><stop offset="1" stop-color="${t.hue[1]}"/></linearGradient></defs><rect width="300" height="300" fill="url(#g)"/><circle cx="210" cy="90" r="46" fill="rgba(255,255,255,.18)"/></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

let volume = { level: 0.42, muted: false };
export function mediaAudio() {
  return { volume, eq: { available: false, hooked: false, configDir: null } };
}
export function mediaSetVolume(level: number | null, muted: boolean | null) {
  volume = { level: level ?? volume.level, muted: muted ?? volume.muted };
  return volume;
}
