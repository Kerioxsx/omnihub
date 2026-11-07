// Follow my voice: the song keeps pace with the singer instead of the other
// way round. It only works on audio OmniHub plays itself (a song file from
// this PC), because it has to slow, hold and resume the music smoothly.
//
// It isn't a loudness gate. Each tick it knows, from the microphone (see
// pitch.ts), whether you're singing (voice activity with hysteresis and
// periodicity) and where notes and syllables start, and from the lyrics
// where each line and word falls. With that:
//   - before a line, if you're not singing, the music eases off and waits at
//     the line's start; it goes on the moment you start the line;
//   - while you sing, it plays at your practice speed, a little slower if
//     the song's syllables run ahead of yours, a little faster if you lead
//     (with hysteresis, so it never wobbles);
//   - a breath (under ~0.45 s) changes nothing; a longer stop slows and
//     softens the music, and after ~1.1 s it pauses and remembers the word
//     you stopped on; when you sing again it picks up from that word;
//   - between lines (solos, intros) the music plays on.
// Rate changes keep the pitch (the browser's pitch-preserving stretch) and
// volume changes are faded, never cut.

import type { LyricLine } from '@shared/types';
import { lineEnd } from '../../components/aurora/AuroraLyrics';
import { syllableCount } from './timing';

export interface FollowLine {
  index: number;
  start: number;
  end: number;
  /** Word starts, ms. */
  words: number[];
  /** Syllables per word. */
  weights: number[];
}

/** The lines to follow (lines with words; estimated words are fine). */
export function followLines(lines: LyricLine[]): FollowLine[] {
  const out: FollowLine[] = [];
  lines.forEach((l, i) => {
    if (!l.text.trim()) return;
    const words = l.words?.length ? l.words : [{ ms: l.ms, text: l.text }];
    out.push({ index: i, start: l.ms, end: lineEnd(lines, i), words: words.map((w) => w.ms), weights: words.map((w) => (w.text.trim() ? syllableCount(w.text) : 0)) });
  });
  return out;
}

export type FollowState = 'free' | 'waiting' | 'singing' | 'breath' | 'slowing' | 'held';

export interface FollowIn {
  now: number;
  song: number;
  voiced: boolean;
  onset: boolean;
  /** The practice speed (0.5–1). */
  speed: number;
  /** Not paused by the person. */
  playing: boolean;
}

export interface FollowOut {
  state: FollowState;
  rate: number;
  volume: number;
  pause: boolean;
  /** Jump here (ms) once, when resuming at the word you stopped on. */
  seek: number | null;
}

export const BREATH_MS = 450;
export const HOLD_MS = 1100;

export class Follower {
  private state: FollowState = 'free';
  private lastVoiced = -1e9;
  private wasVoiced = false;
  private songAtStop = 0;
  private resumeAt: number | null = null;
  private rate = 1;
  private volume = 1;
  private lastNow = 0;
  private lineIdx = -1;
  private onsets = 0;
  private adjust = 0;

  constructor(private lines: FollowLine[]) {}

  setLines(lines: FollowLine[]): void {
    this.lines = lines;
    this.reset();
  }

  reset(): void {
    this.state = 'free';
    this.resumeAt = null;
    this.lineIdx = -1;
    this.onsets = 0;
    this.adjust = 0;
  }

  /** Go on without waiting for the voice (the Continue button). */
  release(): number | null {
    const at = this.resumeAt;
    this.resumeAt = null;
    this.state = 'free';
    this.lastVoiced = this.lastNow;
    return at;
  }

  private lineAt(song: number): FollowLine | null {
    return this.lines.find((l) => song >= l.start - 40 && song < l.end) ?? null;
  }

  private nextLine(song: number): FollowLine | null {
    return this.lines.find((l) => l.start - 40 > song) ?? null;
  }

  /** Syllables of `l` the song has reached by `song`. */
  private songSyllables(l: FollowLine, song: number): number {
    let s = 0;
    for (let k = 0; k < l.words.length; k++) {
      if (l.words[k] > song) break;
      const stop = l.words[k + 1] ?? l.end;
      s += l.weights[k] * Math.min(1, (song - l.words[k]) / Math.max(1, stop - l.words[k]));
    }
    return s;
  }

  step(i: FollowIn): FollowOut {
    const dt = this.lastNow ? Math.min(200, i.now - this.lastNow) : 25;
    this.lastNow = i.now;
    if (i.voiced) this.lastVoiced = i.now;
    if (this.wasVoiced && !i.voiced) this.songAtStop = i.song;
    this.wasVoiced = i.voiced;
    const silence = i.now - this.lastVoiced;
    let seek: number | null = null;
    let target = i.speed;
    let vol = 1;
    let pause = false;

    const line = this.lineAt(i.song);
    if (line && line.index !== this.lineIdx) {
      this.lineIdx = line.index;
      this.onsets = 0;
      this.adjust = 0;
    }
    if (line && i.onset) this.onsets++;

    if (!i.playing) {
      pause = true;
    } else if (this.state === 'held') {
      if (i.voiced) {
        // Back in, from the word you stopped on.
        seek = this.resumeAt;
        this.resumeAt = null;
        this.state = 'singing';
        this.volume = 0.25;
        this.rate = i.speed;
        if (seek !== null && this.lineAt(seek)?.index !== this.lineIdx) {
          this.lineIdx = this.lineAt(seek)?.index ?? -1;
          this.onsets = 1;
        }
      } else pause = true;
    } else {
      const next = this.nextLine(i.song);
      // The last word has begun: stopping now is just the end of the line.
      const finishing = !!line && i.song >= line.words[line.words.length - 1];
      const yourLineNext = !!next && next.start - i.song < 320 && silence > 250;
      if (line && !(finishing && yourLineNext)) {
        if (i.voiced || silence < BREATH_MS || finishing) {
          this.state = i.voiced ? 'singing' : 'breath';
          // Lead or lag by syllables, once you've clearly started the line.
          if (this.onsets >= 2 && !finishing) {
            const lag = this.songSyllables(line, i.song) - this.onsets;
            if (this.adjust === 0 && lag > 3) this.adjust = -1;
            else if (this.adjust === 0 && lag < -2) this.adjust = 1;
            else if (this.adjust === -1 && lag < 1.5) this.adjust = 0;
            else if (this.adjust === 1 && lag > -0.5) this.adjust = 0;
          }
          target = i.speed * (this.adjust < 0 ? 0.85 : this.adjust > 0 ? 1.08 : 1);
        } else if (silence < HOLD_MS) {
          this.state = 'slowing';
          target = i.speed * 0.6;
          vol = 0.35;
        } else {
          // Wait at the start of the word you stopped on.
          const k = Math.max(0, line.words.findLastIndex((w) => w <= this.songAtStop));
          this.resumeAt = Math.max(line.start, line.words[k]) - 60;
          this.state = 'held';
          pause = true;
        }
      } else if (next && yourLineNext) {
        // Your line is next: ease off and wait for you at its start.
        this.state = 'waiting';
        target = i.speed * 0.7;
        if (i.song >= next.start - 70) {
          this.resumeAt = next.start - 50;
          this.state = 'held';
          pause = true;
        }
      } else {
        this.state = 'free';
      }
    }

    // Smooth: rate at most ~1.2× a second, volume ~2.5× a second.
    const k = dt / 1000;
    this.rate += Math.max(-1.2 * k, Math.min(1.2 * k, target - this.rate));
    this.volume += Math.max(-2.5 * k, Math.min(2.5 * k, vol - this.volume));
    return { state: this.state, rate: Math.max(0.5, Math.min(1.25, this.rate)), volume: Math.max(0, Math.min(1, this.volume)), pause, seek };
  }

  get current(): FollowState {
    return this.state;
  }
}
