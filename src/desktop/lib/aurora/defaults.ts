// Aurora's defaults (the same as the core's `VisualSettings::default()`),
// for "Reset to defaults" and the demo.

import type { Effects, VisualSettings } from '@shared/types';

export const DEFAULT_EFFECTS: Effects = { zoom: 0.5, glitch: 0.3, split: 0.35, fisheye: 0, ripple: 0.25, kaleidoscope: 0, halftone: 0, pixelate: 0, shake: 0.15, echo: 0.4, scanlines: 0, duotone: 0, twist: 0.15, bloom: 0.4 };

export const DEFAULT_VISUALS: VisualSettings = {
  view: 'aurora',
  enabled: true,
  audioReactive: true,
  sensitivity: 1,
  bass: 1,
  smoothing: 0.5,
  reducedMotion: false,
  noFlashes: false,
  speed: 1,
  keepAwake: true,
  color: { mode: 'album', primary: '#a855f7', secondary: '#ec4899', colors: ['#8b5cf6', '#ec4899', '#3b82f6', '#22d3ee'], vividness: 1 },
  glow: { enabled: true, animation: 'music', thickness: 2.5, intensity: 1, edge: 1, highlight: 0.6, glow: 0.85, glowSize: 12, bloom: 0.3, bloomSize: 48, radius: 12 },
  lyrics: {
    layout: 'stack',
    emphasis: 'glow',
    font: 'display',
    visible: true,
    autoShow: true,
    size: 1,
    weight: 800,
    wordHighlight: true,
    estimateWords: true,
    timing: 'auto',
    place: 'center',
    offsetX: 0,
    offsetY: 0,
    lines: 3,
    backing: 0,
    highlightColor: null,
    glow: 0.6,
    transition: 1,
    motion: 0.6,
  },
  scene: { enabled: true, style: 'visual', intensity: 0.8, speed: 1, blur: 0.1, saturation: 1.1, opacity: 0.9, artwork: true, waveform: false, effects: { ...DEFAULT_EFFECTS } },
  desktop: { enabled: false, lyrics: false, display: 'primary', clearTaskbar: true, hideFullscreen: true, startWithApp: false },
};
