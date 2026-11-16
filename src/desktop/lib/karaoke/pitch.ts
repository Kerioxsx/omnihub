// Karaoke Studio's ears: the microphone, analysed here on the PC and never
// recorded, kept or sent anywhere. Each frame (about 40 a second) gives:
//   - the voice's pitch (F0) by YIN, with how sure it is (periodicity);
//   - whether you're singing: voice activity from loudness over a noise
//     floor that adapts to the room AND periodicity, with hysteresis
//     (on after ~75 ms of voice, off after ~200 ms without), so a breath, a
//     consonant or background noise doesn't flicker it;
//   - note and syllable starts: when the voice starts, when the pitch moves
//     to a new note and holds there, or when the loudness dips and rises
//     again (a new syllable sung legato).
// `Coach` turns the frames into a practice summary from what was really
// measured: steadiness of held notes in cents, entrances against the
// lyrics' timing, and your comfortable range.

export interface VoiceFrame {
  /** performance.now() of the frame. */
  at: number;
  /** Loudness, dBFS. */
  db: number;
  /** The room's noise floor, dBFS. */
  floor: number;
  /** Pitch, Hz, when the voice is periodic enough to have one. */
  hz: number | null;
  /** The same as a MIDI note number (fractional: 69.5 = A4 + 50 cents). */
  midi: number | null;
  /** Periodicity 0–1 (YIN): how much of a clear pitch there is. */
  confidence: number;
  /** Singing now (after hysteresis). */
  voiced: boolean;
  /** A note or syllable started in this frame. */
  onset: boolean;
}

// ---------- pitch ----------

/** YIN pitch of `x` at `rate` Hz, between `min` and `max` Hz. */
export function yin(x: Float32Array, rate: number, min = 70, max = 1050, threshold = 0.15): { hz: number | null; confidence: number } {
  const tauMin = Math.max(2, Math.floor(rate / max));
  const tauMax = Math.min(Math.floor(x.length / 2), Math.ceil(rate / min));
  const w = x.length - tauMax;
  if (w < 32) return { hz: null, confidence: 0 };
  const d = new Float32Array(tauMax + 1);
  for (let tau = 1; tau <= tauMax; tau++) {
    let s = 0;
    for (let j = 0; j < w; j++) {
      const diff = x[j] - x[j + tau];
      s += diff * diff;
    }
    d[tau] = s;
  }
  // Cumulative mean normalized difference.
  const c = new Float32Array(tauMax + 1);
  c[0] = 1;
  let sum = 0;
  for (let tau = 1; tau <= tauMax; tau++) {
    sum += d[tau];
    c[tau] = sum > 0 ? (d[tau] * tau) / sum : 1;
  }
  let best = -1;
  for (let tau = tauMin; tau <= tauMax; tau++) {
    if (c[tau] < threshold) {
      while (tau + 1 <= tauMax && c[tau + 1] < c[tau]) tau++;
      best = tau;
      break;
    }
  }
  if (best < 0) {
    let lo = tauMin;
    for (let tau = tauMin; tau <= tauMax; tau++) if (c[tau] < c[lo]) lo = tau;
    // Not periodic enough to call it a pitch.
    return { hz: null, confidence: Math.max(0, 1 - c[lo]) };
  }
  // Parabolic interpolation around the dip.
  let t = best;
  if (best > 1 && best < tauMax) {
    const a = c[best - 1];
    const b = c[best];
    const e = c[best + 1];
    const den = a + e - 2 * b;
    if (den !== 0) t = best + (a - e) / (2 * den);
  }
  return { hz: rate / t, confidence: Math.max(0, Math.min(1, 1 - c[best])) };
}

export const hzToMidi = (hz: number) => 69 + 12 * Math.log2(hz / 440);

const NAMES = ['C', 'C♯', 'D', 'D♯', 'E', 'F', 'F♯', 'G', 'G♯', 'A', 'A♯', 'B'];
const SOLFEGE = ['Do', 'Do♯', 'Re', 'Re♯', 'Mi', 'Fa', 'Fa♯', 'Sol', 'Sol♯', 'La', 'La♯', 'Si'];

/** "A3" (or "La3" in solfège). */
export function noteName(midi: number, solfege = false): string {
  const n = Math.round(midi);
  return `${(solfege ? SOLFEGE : NAMES)[((n % 12) + 12) % 12]}${Math.floor(n / 12) - 1}`;
}

/** Cents away from the nearest note (−50…50). */
export const centsOff = (midi: number) => Math.round((midi - Math.round(midi)) * 100);

const dbOf = (x: Float32Array) => {
  let s = 0;
  for (let i = 0; i < x.length; i++) s += x[i] * x[i];
  return 10 * Math.log10(s / x.length + 1e-12);
};

// ---------- voice activity, notes and syllables ----------

/** Frames in, frames with voice activity and onsets out (no audio APIs: testable). */
export class Detector {
  private floor: number | null = null;
  private on = 0;
  private off = 0;
  private voiced = false;
  private lastOnset = -1e9;
  private recent: number[] = [];
  private segMidi: number | null = null;
  private moved = 0;
  private dbs: number[] = [];

  /** One frame of samples at `rate` Hz, taken at `at` ms. */
  push(samples: Float32Array, rate: number, at: number): VoiceFrame {
    // Half the rate (averaging pairs): voices don't need more, YIN costs a quarter.
    const half = new Float32Array(samples.length >> 1);
    for (let i = 0; i < half.length; i++) half[i] = (samples[2 * i] + samples[2 * i + 1]) / 2;
    const db = dbOf(samples);
    const p = yin(half, rate / 2);
    return this.frame(db, p.hz, p.confidence, at);
  }

  frame(db: number, hz: number | null, confidence: number, at: number): VoiceFrame {
    // The floor follows quiet quickly and a noisier room in a few seconds
    // (never while you sing, or your voice would become the floor).
    let floor = this.floor ?? Math.min(db, -45);
    floor = db < floor ? floor + (db - floor) * 0.3 : this.voiced ? floor : Math.min(floor + 0.05, -30);
    floor = Math.max(-90, floor);
    this.floor = floor;
    const above = db - floor;
    const pitched = hz !== null && confidence > 0.72;
    const speechy = db > -58 && above > 9 && (pitched || above > 20);
    if (speechy) {
      this.on++;
      this.off = 0;
    } else {
      this.off++;
      this.on = 0;
    }
    let onset = false;
    if (!this.voiced && this.on >= 3) {
      this.voiced = true;
      onset = true;
      this.segMidi = null;
    } else if (this.voiced && this.off >= 8) {
      this.voiced = false;
      this.segMidi = null;
      this.recent = [];
    }
    const midi = pitched && hz ? hzToMidi(hz) : null;
    if (this.voiced && midi !== null) {
      this.recent.push(midi);
      if (this.recent.length > 3) this.recent.shift();
      const med = [...this.recent].sort((a, b) => a - b)[this.recent.length >> 1];
      if (this.segMidi === null) this.segMidi = med;
      else if (Math.abs(med - this.segMidi) > 0.8) {
        // A new note once the pitch holds somewhere new.
        if (++this.moved >= 3) {
          this.segMidi = med;
          this.moved = 0;
          onset = true;
        }
      } else this.moved = 0;
    }
    // A dip and rise in loudness while singing: a new syllable.
    this.dbs.push(db);
    if (this.dbs.length > 6) this.dbs.shift();
    if (this.voiced && this.dbs.length === 6) {
      const dip = Math.min(...this.dbs.slice(1, 5));
      if (this.dbs[0] - dip > 6 && db - dip > 6) onset = true;
    }
    if (onset) {
      // One onset per syllable at most (~110 ms apart).
      if (at - this.lastOnset < 110) onset = false;
      else this.lastOnset = at;
    }
    return { at, db, floor, hz: midi !== null ? hz : null, midi, confidence, voiced: this.voiced, onset };
  }
}

// ---------- the microphone ----------

export type MicError = 'denied' | 'missing' | 'busy' | 'unsupported';

export function micErrorText(e: MicError): string {
  switch (e) {
    case 'denied':
      return 'The microphone was blocked. Allow it for OmniHub (Windows Settings → Privacy & security → Microphone) and try again.';
    case 'missing':
      return 'No microphone found. Plug one in (or connect a headset) and try again.';
    case 'busy':
      return 'Another app is using the microphone right now.';
    default:
      return 'This window can’t use a microphone.';
  }
}

export interface MicInput {
  id: string;
  label: string;
}

/** Microphones (names only appear once one has been allowed). */
export async function listMics(): Promise<MicInput[]> {
  try {
    const all = await navigator.mediaDevices.enumerateDevices();
    return all.filter((d) => d.kind === 'audioinput').map((d, i) => ({ id: d.deviceId, label: d.label || `Microphone ${i + 1}` }));
  } catch {
    return [];
  }
}

/** A live microphone feeding a Detector. Nothing is recorded. */
export class Mic {
  private stream: MediaStream | null = null;
  private ctx: AudioContext | null = null;
  private timer = 0;
  private detector = new Detector();
  private listeners = new Set<(f: VoiceFrame) => void>();
  /** Called if the microphone goes away (unplugged). */
  onEnded: (() => void) | null = null;
  label = '';

  static async start(deviceId?: string): Promise<Mic> {
    if (!navigator.mediaDevices?.getUserMedia) throw 'unsupported' satisfies MicError;
    const m = new Mic();
    try {
      m.stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          deviceId: deviceId ? { exact: deviceId } : undefined,
          // Echo cancelling keeps the song from speakers out of the analysis;
          // the rest would bend the voice it's measuring.
          echoCancellation: true,
          noiseSuppression: false,
          autoGainControl: false,
        },
      });
    } catch (e) {
      const name = (e as { name?: string })?.name;
      throw (name === 'NotAllowedError' || name === 'SecurityError' ? 'denied' : name === 'NotFoundError' || name === 'OverconstrainedError' ? 'missing' : name === 'NotReadableError' ? 'busy' : 'unsupported') satisfies MicError;
    }
    const track = m.stream.getAudioTracks()[0];
    m.label = track?.label ?? '';
    track?.addEventListener('ended', () => {
      m.stop();
      m.onEnded?.();
    });
    m.ctx = new AudioContext();
    const src = m.ctx.createMediaStreamSource(m.stream);
    const an = m.ctx.createAnalyser();
    an.fftSize = 2048;
    src.connect(an);
    const buf = new Float32Array(an.fftSize);
    const rate = m.ctx.sampleRate;
    m.timer = window.setInterval(() => {
      an.getFloatTimeDomainData(buf);
      const f = m.detector.push(buf, rate, performance.now());
      m.listeners.forEach((l) => l(f));
    }, 25);
    return m;
  }

  listen(l: (f: VoiceFrame) => void): () => void {
    this.listeners.add(l);
    return () => this.listeners.delete(l);
  }

  stop(): void {
    window.clearInterval(this.timer);
    this.stream?.getTracks().forEach((t) => t.stop());
    void this.ctx?.close().catch(() => undefined);
    this.stream = null;
    this.ctx = null;
    this.listeners.clear();
  }
}

// ---------- the practice summary ----------

export interface CoachSummary {
  seconds: number;
  /** Lines with voice in them. */
  lines: number;
  steadinessCents: number | null;
  timingMs: number | null;
  lowNote: number | null;
  highNote: number | null;
  /** Lines worth another go, by index, worst first. */
  retry: number[];
  /** Per line: entrance offset (ms, negative = early) and steadiness. */
  perLine: Map<number, { entrance?: number; steadiness?: number; voiced: number }>;
}

const percentile = (sorted: number[], p: number) => sorted[Math.min(sorted.length - 1, Math.max(0, Math.round((sorted.length - 1) * p)))];

/**
 * Collects frames during a practice. Timing is only measured when the song
 * keeps its own time (Follow the song); held notes are judged against their
 * own centre, because there is no reference melody to compare with.
 */
export class Coach {
  private started = performance.now();
  private active = 0;
  private notes: number[] = [];
  private seg: { line: number; midis: number[]; start: number } | null = null;
  private held: { line: number; cents: number }[] = [];
  private entered = new Set<number>();
  private wasVoiced = false;
  readonly perLine = new Map<number, { entrance?: number; steadiness?: number; voiced: number }>();

  constructor(public measureTiming: boolean) {}

  private lineInfo(i: number) {
    let v = this.perLine.get(i);
    if (!v) this.perLine.set(i, (v = { voiced: 0 }));
    return v;
  }

  /** A frame, with the line being sung (−1 for none), when it starts, and the song's time. */
  add(f: VoiceFrame, line: number, lineStart: number, songMs: number): void {
    if (f.voiced) this.active += 25;
    if (f.voiced && f.midi !== null && f.confidence > 0.85) {
      this.notes.push(f.midi);
      if (this.notes.length > 30000) this.notes.splice(0, 10000);
    }
    if (line >= 0 && f.voiced) this.lineInfo(line).voiced += 1;
    // An entrance: the voice starting near a line's start.
    if (this.measureTiming && line >= 0 && f.voiced && !this.wasVoiced && !this.entered.has(line)) {
      // The voice is confirmed ~60 ms after it starts (hysteresis and the analysis window).
      const off = songMs - lineStart - 60;
      if (off > -700 && off < 1500) {
        this.entered.add(line);
        this.lineInfo(line).entrance = Math.round(off);
      }
    }
    this.wasVoiced = f.voiced;
    // Held notes: from one onset to the next while singing.
    if (f.onset || !f.voiced) this.closeSegment();
    if (f.voiced && f.midi !== null && f.confidence > 0.8) {
      this.seg ??= { line, midis: [], start: f.at };
      this.seg.midis.push(f.midi);
    }
  }

  private closeSegment(): void {
    const s = this.seg;
    this.seg = null;
    // At least ~350 ms of clear pitch: a held note. Its attack and release
    // are left out, and the last frames too (they can already be the next
    // note, which is only confirmed after it holds for a few frames).
    if (!s || s.midis.length < 14) return;
    const cut = Math.floor(s.midis.length * 0.15);
    const mid = s.midis.slice(cut, s.midis.length - Math.max(cut, 3));
    const centre = [...mid].sort((a, b) => a - b)[mid.length >> 1];
    const dev = mid.reduce((a, m) => a + Math.abs(m - centre) * 100, 0) / mid.length;
    this.held.push({ line: s.line, cents: dev });
    if (s.line >= 0) {
      const li = this.lineInfo(s.line);
      li.steadiness = li.steadiness === undefined ? dev : (li.steadiness + dev) / 2;
    }
  }

  summary(): CoachSummary {
    this.closeSegment();
    const sorted = [...this.notes].sort((a, b) => a - b);
    const entrances = [...this.perLine.values()].map((v) => v.entrance).filter((x): x is number => x !== undefined);
    const lines = [...this.perLine.values()].filter((v) => v.voiced >= 8).length;
    // Worth another go: lines with a late or early entrance, or wobbly held notes.
    const retry = [...this.perLine.entries()]
      .map(([i, v]) => ({ i, bad: Math.max(v.entrance !== undefined ? Math.abs(v.entrance) / 300 : 0, v.steadiness !== undefined ? v.steadiness / 35 : 0) }))
      .filter((x) => x.bad >= 1)
      .sort((a, b) => b.bad - a.bad)
      .slice(0, 5)
      .map((x) => x.i);
    return {
      seconds: Math.round((performance.now() - this.started) / 1000),
      lines,
      steadinessCents: this.held.length >= 3 ? Math.round(this.held.reduce((a, h) => a + h.cents, 0) / this.held.length) : null,
      timingMs: entrances.length >= 3 ? Math.round(entrances.reduce((a, b) => a + b, 0) / entrances.length) : null,
      // The comfortable range: what you sang clearly, not the odd squeak.
      lowNote: sorted.length >= 80 ? Math.round(percentile(sorted, 0.05)) : null,
      highNote: sorted.length >= 80 ? Math.round(percentile(sorted, 0.95)) : null,
      retry,
      perLine: this.perLine,
    };
  }

  /** Seconds of singing so far. */
  get singingSeconds(): number {
    return Math.round(this.active / 1000);
  }
}
