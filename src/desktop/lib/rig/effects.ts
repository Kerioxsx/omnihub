// The lighting effects, as crates/omnihub-core/src/rig/lighting.rs runs them
// for the real LEDs: the 3D view draws with the same maths, so what it shows
// is what the PC shows.

import type { LightEffect, LightProfile } from '@shared/types';

export type Rgb = [number, number, number];

export interface Inputs {
  /** Seconds since the effect started. */
  t: number;
  level: number;
  beat: number;
  temp: number | null;
}

export const ANIMATED: Record<LightEffect, boolean> = { off: false, static: false, breathing: true, spectrum: true, wave: true, gradient: true, music: true, temperature: false, govee: true };

export function parseHex(s: string): Rgb | null {
  const h = s.trim().replace(/^#/, '');
  if (!/^[0-9a-fA-F]{6}$/.test(h)) return null;
  const v = parseInt(h, 16);
  return [(v >> 16) & 255, (v >> 8) & 255, v & 255];
}

export const hex = (c: Rgb) => `#${c.map((x) => x.toString(16).padStart(2, '0')).join('')}`;

const clamp01 = (k: number) => Math.min(1, Math.max(0, k));
/** Rust's f32::round: halves away from zero. */
const round = (x: number) => Math.sign(x) * Math.round(Math.abs(x));

function lerp(a: Rgb, b: Rgb, k: number): Rgb {
  const t = clamp01(k);
  return [0, 1, 2].map((i) => round(a[i] + (b[i] - a[i]) * t)) as Rgb;
}

function scale(c: Rgb, k: number): Rgb {
  const t = clamp01(k);
  return c.map((x) => round(x * t)) as Rgb;
}

const rem = (x: number, m: number) => ((x % m) + m) % m;

export function hue(h: number): Rgb {
  const p = rem(h, 1) * 6;
  const x = 1 - Math.abs((p % 2) - 1);
  const seg = Math.floor(p);
  const [r, g, b] = seg === 0 ? [1, x, 0] : seg === 1 ? [x, 1, 0] : seg === 2 ? [0, 1, x] : seg === 3 ? [0, x, 1] : seg === 4 ? [x, 0, 1] : [1, 0, x];
  return [round(r * 255), round(g * 255), round(b * 255)];
}

function along(colors: Rgb[], x: number, wrap: boolean): Rgb {
  const n = colors.length;
  if (n === 0) return [255, 255, 255];
  if (n === 1) return colors[0];
  const stops = wrap ? n : n - 1;
  const p = (wrap ? rem(x, 1) : clamp01(x)) * stops;
  const i = Math.min(Math.floor(p), stops - 1);
  return lerp(colors[i], colors[(i + 1) % n], p - i);
}

export function palette(p: LightProfile): Rgb[] {
  return p.colors.map(parseHex).filter((c): c is Rgb => !!c);
}

/** The colour of the LED at `x` (0–1 along its device) on device `dev`. */
export function colorAt(p: LightProfile, colors: Rgb[], x: number, dev: number, inp: Inputs): Rgb {
  const speed = Math.min(3, Math.max(0.2, p.speed));
  const t = inp.t * speed;
  let c: Rgb;
  switch (p.effect) {
    case 'off':
      c = [0, 0, 0];
      break;
    case 'static':
      c = along(colors, x, false);
      break;
    case 'breathing': {
      const phase = t * 0.4;
      const k = 0.06 + 0.94 * (0.5 - 0.5 * Math.cos(2 * Math.PI * phase));
      const n = Math.max(1, colors.length);
      c = scale(colors[Math.floor(phase) % n] ?? [255, 255, 255], k);
      break;
    }
    case 'spectrum':
      c = hue(t * 0.08);
      break;
    case 'wave':
      c = hue(x * 0.8 + dev * 0.13 - t * 0.25);
      break;
    case 'gradient':
      c = along(colors, x * 0.5 + dev * 0.17 + t * 0.05, true);
      break;
    case 'music': {
      const k = Math.min(1, 0.12 + inp.level * 1.1 + inp.beat * 0.6);
      const base = along(colors, x * 0.6 + dev * 0.2 + t * 0.03, true);
      const flash = along(colors, x * 0.6 + dev * 0.2 + t * 0.03 + 0.5, true);
      c = scale(lerp(base, flash, inp.beat), k);
      break;
    }
    case 'temperature':
      c = along(colors, ((inp.temp ?? 40) - 35) / 50, false);
      break;
    case 'govee':
      c = along(colors, x, false);
      break;
  }
  return scale(c, Math.min(100, p.brightness) / 100);
}

export const EFFECTS: { value: LightEffect; label: string; hint: string; colors: boolean }[] = [
  { value: 'static', label: 'Static', hint: 'One colour, or your colours spread across each part', colors: true },
  { value: 'breathing', label: 'Breathing', hint: 'Fades in and out, a colour per breath', colors: true },
  { value: 'spectrum', label: 'Spectrum', hint: 'The whole PC through every colour', colors: false },
  { value: 'wave', label: 'Rainbow wave', hint: 'A rainbow running along the lights', colors: false },
  { value: 'gradient', label: 'Flow', hint: 'Your colours flowing slowly', colors: true },
  { value: 'music', label: 'Music', hint: 'Pulses with what your PC plays', colors: true },
  { value: 'govee', label: 'Like the Govee lights', hint: 'Shows what your Govee lights show: the screen, the music or their effect (your colours when they’re off)', colors: true },
  { value: 'temperature', label: 'Temperature', hint: 'Cool to hot with the processor (needs LibreHardwareMonitor)', colors: true },
  { value: 'off', label: 'Off', hint: 'Lights off', colors: false },
];
