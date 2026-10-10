// A pretend music player for the mock backend (same tracks as the Rust
// test player), so Now Playing and lyrics can be developed without Windows.

import type { LyricLine, LyricsStatus, MediaAction, MediaState } from '@shared/types';
import { emit } from './bus';

// The core's test lyrics (original, made up for testing): words timed with
// instrumental breaks, lines only, an instrumental, and a song without lyrics.
import daylight from '../../../crates/omnihub-core/src/media/testdata/daylight-drive.lrc?raw';
import nightLoop from '../../../crates/omnihub-core/src/media/testdata/night-loop.lrc?raw';

const TRACKS = [
  { title: 'Daylight Drive', artist: 'Midnight Atlas', album: 'Open Roads', durationMs: 200_000, hue: ['#ff7a3c', '#2a1440'], bpm: 118, lrc: daylight as string | null, instrumental: false },
  { title: 'Night Loop', artist: 'Midnight Atlas', album: 'Open Roads', durationMs: 180_000, hue: ['#465aff', '#140a28'], bpm: 96, lrc: nightLoop as string | null, instrumental: false },
  { title: 'Static Bloom', artist: 'Midnight Atlas', album: 'Open Roads', durationMs: 150_000, hue: ['#22c79a', '#081c2a'], bpm: 84, lrc: null, instrumental: true },
  { title: 'Unwritten Demo', artist: 'Midnight Atlas', album: 'Open Roads', durationMs: 120_000, hue: ['#ec4899', '#2a0a1e'], bpm: 108, lrc: null, instrumental: false },
];

/** The LRC subset the core reads: line stamps and `<mm:ss.xx>` word stamps. */
function parseLrc(src: string): LyricLine[] {
  const time = (t: string) => {
    const m = /^(\d+):(\d+)(?:[.:](\d+))?$/.exec(t.trim());
    return m ? Number(m[1]) * 60_000 + Number(m[2]) * 1000 + Number((m[3] ?? '0').padEnd(3, '0').slice(0, 3)) : null;
  };
  const out: LyricLine[] = [];
  for (const raw of src.split('\n')) {
    const m = /^\[([^\]]+)\](.*)$/.exec(raw.trim());
    const ms = m ? time(m[1]) : null;
    if (!m || ms == null) continue;
    const words: { ms: number; text: string }[] = [];
    const re = /<([^>]+)>([^<]*)/g;
    let w: RegExpExecArray | null;
    while ((w = re.exec(m[2]))) {
      const t = time(w[1]);
      if (t != null && w[2].trim()) words.push({ ms: t, text: w[2] });
    }
    const text = m[2].replace(/<[^>]+>/g, '').replace(/\s+/g, ' ').trim();
    out.push(words.length ? { ms, text, words } : { ms, text });
  }
  return out;
}

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
  const t = TRACKS[index];
  if (t.instrumental) return { key: s.key, lyrics: { status: 'ready', lyrics: { lines: [], plain: null, instrumental: true, source: 'OmniHub test lyrics' } } };
  if (!t.lrc) return { key: s.key, lyrics: { status: 'none' } };
  return { key: s.key, lyrics: { status: 'ready', lyrics: { lines: parseLrc(t.lrc), plain: null, instrumental: false, source: 'OmniHub test lyrics' } } };
}

/** For the pretend sound: the track, where it is and whether it plays. */
export function playback() {
  return { track: index, bpm: TRACKS[index].bpm, positionMs: position(), playing };
}

/** Original neon covers on black, one design per test track. */
export function mediaArt(id: string): string {
  const i = Number(id.split('-')[1]) || 0;
  const t = TRACKS[i];
  const [a, b] = t.hue;
  const rays = (n: number, colors: string[], inner: number, outer: number, width: number) =>
    Array.from({ length: n }, (_, k) => {
      const ang = (k / n) * Math.PI * 2;
      const [c, s] = [Math.cos(ang), Math.sin(ang)];
      const len = outer * (0.75 + 0.25 * Math.sin(k * 2.3));
      return `<line x1="${150 + c * inner}" y1="${150 + s * inner}" x2="${150 + c * len}" y2="${150 + s * len}" stroke="${colors[k % colors.length]}" stroke-width="${width}" stroke-linecap="round"/>`;
    }).join('');
  const dots = Array.from({ length: 12 }, (_, y) => Array.from({ length: 12 }, (_, x) => `<circle cx="${96 + x * 10}" cy="${96 + y * 10}" r="${1.2 + ((x + y) % 3)}" fill="#fff" opacity=".55"/>`).join('')).join('');
  const designs = [
    // A sunburst in orange, pink and yellow with a halftone heart.
    `${rays(22, ['#ff5ea8', '#ff9a3c', '#ffe14d', '#c13cff'], 34, 150, 9)}<circle cx="150" cy="150" r="46" fill="${a}"/><g clip-path="url(#c)">${dots}</g><circle cx="150" cy="150" r="20" fill="#120616"/>`,
    // Neon rings.
    [0, 1, 2, 3, 4].map((k) => `<circle cx="${150 + k * 6}" cy="${150 - k * 4}" r="${28 + k * 22}" fill="none" stroke="${['#3b82f6', '#22d3ee', '#a855f7', '#ec4899', '#60a5fa'][k]}" stroke-width="${7 - k}"/>`).join('') + `<circle cx="150" cy="150" r="18" fill="${a}"/>`,
    // Petals.
    Array.from({ length: 8 }, (_, k) => `<ellipse cx="150" cy="88" rx="22" ry="62" fill="${['#22c79a', '#a3e635', '#22d3ee', '#10b981'][k % 4]}" opacity=".85" transform="rotate(${k * 45} 150 150)"/>`).join('') + `<circle cx="150" cy="150" r="26" fill="#fde047"/>`,
    // Zigzags.
    Array.from({ length: 7 }, (_, k) => `<polyline points="${Array.from({ length: 9 }, (_, x) => `${20 + x * 32},${60 + k * 30 + (x % 2 ? -14 : 14)}`).join(' ')}" fill="none" stroke="${['#ec4899', '#f472b6', '#a855f7', '#fb7185'][k % 4]}" stroke-width="8" stroke-linejoin="round"/>`).join(''),
  ];
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 300"><defs><radialGradient id="g"><stop offset="0" stop-color="${b}"/><stop offset="1" stop-color="#050307"/></radialGradient><clipPath id="c"><circle cx="150" cy="150" r="46"/></clipPath></defs><rect width="300" height="300" fill="url(#g)"/>${designs[i % designs.length]}</svg>`;
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
