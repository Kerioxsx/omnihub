// Focus sounds, made on the fly with Web Audio (no files): noises, rain,
// waves, wind and a fireplace, mixed at their own volumes. They keep playing
// while you move around the app.

export type SoundId = 'rain' | 'waves' | 'wind' | 'fire' | 'brown' | 'pink' | 'white';

export const SOUNDS: { id: SoundId; label: string; hint: string }[] = [
  { id: 'rain', label: 'Rain', hint: 'Steady rain with drops' },
  { id: 'waves', label: 'Waves', hint: 'Slow waves on a beach' },
  { id: 'wind', label: 'Wind', hint: 'Gusts through trees' },
  { id: 'fire', label: 'Fireplace', hint: 'A low fire that crackles' },
  { id: 'brown', label: 'Brown noise', hint: 'Deep and soft' },
  { id: 'pink', label: 'Pink noise', hint: 'Balanced, like a waterfall' },
  { id: 'white', label: 'White noise', hint: 'Bright, hides voices' },
];

type Playing = { gain: GainNode; stop: () => void };

let ctx: AudioContext | null = null;
let master: GainNode | null = null;
let masterLevel = 0.8;
const levels: Partial<Record<SoundId, number>> = {};
const playing = new Map<SoundId, Playing>();
const buffers = new Map<string, AudioBuffer>();
const listeners = new Set<() => void>();
const changed = () => listeners.forEach((l) => l());

function audio(): { ctx: AudioContext; master: GainNode } {
  if (!ctx || !master) {
    ctx = new AudioContext();
    master = ctx.createGain();
    master.gain.value = masterLevel;
    master.connect(ctx.destination);
  }
  if (ctx.state === 'suspended') void ctx.resume();
  return { ctx, master };
}

function noise(c: AudioContext, kind: 'white' | 'pink' | 'brown'): AudioBuffer {
  const hit = buffers.get(kind);
  if (hit) return hit;
  const len = c.sampleRate * 6;
  const buf = c.createBuffer(2, len, c.sampleRate);
  for (let ch = 0; ch < 2; ch++) {
    const d = buf.getChannelData(ch);
    let b0 = 0,
      b1 = 0,
      b2 = 0,
      b3 = 0,
      b4 = 0,
      b5 = 0,
      b6 = 0,
      last = 0;
    for (let i = 0; i < len; i++) {
      const w = Math.random() * 2 - 1;
      if (kind === 'white') d[i] = w * 0.5;
      else if (kind === 'pink') {
        // Paul Kellet's filter.
        b0 = 0.99886 * b0 + w * 0.0555179;
        b1 = 0.99332 * b1 + w * 0.0750759;
        b2 = 0.969 * b2 + w * 0.153852;
        b3 = 0.8665 * b3 + w * 0.3104856;
        b4 = 0.55 * b4 + w * 0.5329522;
        b5 = -0.7616 * b5 - w * 0.016898;
        d[i] = (b0 + b1 + b2 + b3 + b4 + b5 + b6 + w * 0.5362) * 0.11;
        b6 = w * 0.115926;
      } else {
        last = (last + 0.02 * w) / 1.02;
        d[i] = last * 3.5;
      }
    }
    // Fade the ends together so the loop doesn't click.
    const f = Math.floor(c.sampleRate * 0.05);
    for (let i = 0; i < f; i++) {
      const t = i / f;
      d[len - f + i] = d[len - f + i] * (1 - t) + d[i] * t;
    }
  }
  buffers.set(kind, buf);
  return buf;
}

function loop(c: AudioContext, kind: 'white' | 'pink' | 'brown'): AudioBufferSourceNode {
  const s = c.createBufferSource();
  s.buffer = noise(c, kind);
  s.loop = true;
  s.loopEnd = s.buffer.duration - 0.05;
  return s;
}

function lfo(c: AudioContext, hz: number, depth: number, target: AudioParam): OscillatorNode {
  const o = c.createOscillator();
  const g = c.createGain();
  o.frequency.value = hz;
  g.gain.value = depth;
  o.connect(g).connect(target);
  o.start();
  return o;
}

/** Short noise bursts at random times (raindrops, fire crackles). */
function bursts(c: AudioContext, out: AudioNode, opts: { every: [number, number]; freq: [number, number]; length: number; level: number }): () => void {
  let alive = true;
  let timer = 0;
  const one = () => {
    if (!alive) return;
    const s = loop(c, 'white');
    const bp = c.createBiquadFilter();
    bp.type = 'bandpass';
    bp.frequency.value = opts.freq[0] + Math.random() * (opts.freq[1] - opts.freq[0]);
    bp.Q.value = 2 + Math.random() * 6;
    const g = c.createGain();
    const t = c.currentTime;
    const peak = opts.level * (0.3 + Math.random() * 0.7);
    g.gain.setValueAtTime(0, t);
    g.gain.linearRampToValueAtTime(peak, t + 0.004);
    g.gain.exponentialRampToValueAtTime(0.0001, t + opts.length * (0.5 + Math.random()));
    s.connect(bp).connect(g).connect(out);
    s.start(t, Math.random() * 5);
    s.stop(t + opts.length * 2 + 0.05);
    timer = window.setTimeout(one, opts.every[0] + Math.random() * (opts.every[1] - opts.every[0]));
  };
  one();
  return () => {
    alive = false;
    clearTimeout(timer);
  };
}

function build(id: SoundId, c: AudioContext, out: GainNode): () => void {
  const nodes: AudioScheduledSourceNode[] = [];
  const stops: (() => void)[] = [];
  const src = (kind: 'white' | 'pink' | 'brown') => {
    const s = loop(c, kind);
    s.start(0, Math.random() * 5);
    nodes.push(s);
    return s;
  };
  if (id === 'white' || id === 'pink' || id === 'brown') {
    src(id).connect(out);
  } else if (id === 'rain') {
    const hp = c.createBiquadFilter();
    hp.type = 'highpass';
    hp.frequency.value = 400;
    const lp = c.createBiquadFilter();
    lp.type = 'lowpass';
    lp.frequency.value = 7000;
    const g = c.createGain();
    g.gain.value = 0.7;
    src('pink').connect(hp).connect(lp).connect(g).connect(out);
    nodes.push(lfo(c, 0.07, 0.12, g.gain));
    stops.push(bursts(c, out, { every: [15, 90], freq: [1800, 6000], length: 0.03, level: 0.25 }));
  } else if (id === 'waves') {
    const lp = c.createBiquadFilter();
    lp.type = 'lowpass';
    lp.frequency.value = 700;
    const g = c.createGain();
    g.gain.value = 0.55;
    src('brown').connect(lp).connect(g).connect(out);
    nodes.push(lfo(c, 0.11, 0.45, g.gain), lfo(c, 0.11, 450, lp.frequency));
  } else if (id === 'wind') {
    const bp = c.createBiquadFilter();
    bp.type = 'bandpass';
    bp.frequency.value = 500;
    bp.Q.value = 0.9;
    const g = c.createGain();
    g.gain.value = 0.8;
    src('pink').connect(bp).connect(g).connect(out);
    nodes.push(lfo(c, 0.05, 280, bp.frequency), lfo(c, 0.083, 0.35, g.gain), lfo(c, 0.31, 60, bp.frequency));
  } else if (id === 'fire') {
    const lp = c.createBiquadFilter();
    lp.type = 'lowpass';
    lp.frequency.value = 400;
    const g = c.createGain();
    g.gain.value = 0.6;
    src('brown').connect(lp).connect(g).connect(out);
    nodes.push(lfo(c, 0.4, 0.12, g.gain));
    stops.push(bursts(c, out, { every: [60, 700], freq: [1500, 4500], length: 0.02, level: 0.5 }));
    stops.push(bursts(c, out, { every: [400, 2500], freq: [300, 900], length: 0.08, level: 0.35 }));
  }
  return () => {
    stops.forEach((s) => s());
    nodes.forEach((n) => {
      try {
        n.stop();
      } catch {
        /* already stopped */
      }
    });
  };
}

/** Set one sound's volume (0 stops it). */
export function setSound(id: SoundId, level: number) {
  level = Math.max(0, Math.min(1, level));
  const now = playing.get(id);
  if (level === 0) {
    delete levels[id];
    if (now && ctx) {
      now.gain.gain.setTargetAtTime(0, ctx.currentTime, 0.15);
      const p = now;
      setTimeout(() => {
        p.stop();
        p.gain.disconnect();
      }, 700);
      playing.delete(id);
    }
    changed();
    return;
  }
  levels[id] = level;
  const { ctx: c, master: m } = audio();
  if (now) {
    now.gain.gain.setTargetAtTime(level * level, c.currentTime, 0.1);
  } else {
    const gain = c.createGain();
    gain.gain.value = 0;
    gain.connect(m);
    gain.gain.setTargetAtTime(level * level, c.currentTime, 0.3);
    playing.set(id, { gain, stop: build(id, c, gain) });
  }
  changed();
}

export function setMaster(level: number) {
  masterLevel = Math.max(0, Math.min(1, level));
  if (master && ctx) master.gain.setTargetAtTime(masterLevel, ctx.currentTime, 0.1);
  changed();
}

export function stopAll() {
  for (const id of [...playing.keys()]) setSound(id, 0);
}

export const soundState = () => ({ levels: { ...levels }, master: masterLevel });

export function subscribeSounds(l: () => void): () => void {
  listeners.add(l);
  return () => {
    listeners.delete(l);
  };
}
