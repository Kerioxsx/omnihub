// The app's looks: Classic (light or dark with an accent colour) and the
// animated themes. Each animated theme brings its own colours; its
// background lives in components/Backdrop.tsx and themes.css.

import type { Look } from '@shared/types';

export interface LookInfo {
  id: Look;
  name: string;
  /** What you'll see, for the picker. */
  blurb: string;
  /** Uses the light palette. */
  light: boolean;
}

export const LOOKS: LookInfo[] = [
  { id: 'classic', name: 'Classic', blurb: 'Light or dark, with your accent colour.', light: false },
  { id: 'aurora', name: 'Aurora', blurb: 'Northern lights drifting behind everything.', light: false },
  { id: 'synthwave', name: 'Synthwave', blurb: 'A neon sun over a moving grid.', light: false },
  { id: 'ocean', name: 'Ocean', blurb: 'Slow waves in deep blue.', light: false },
  { id: 'galaxy', name: 'Galaxy', blurb: 'Twinkling stars and drifting nebulas.', light: false },
  { id: 'sakura', name: 'Sakura', blurb: 'Cherry blossom petals falling.', light: true },
  { id: 'ember', name: 'Ember', blurb: 'Warm sparks rising from below.', light: false },
];

export const lookInfo = (id: string | undefined): LookInfo => LOOKS.find((l) => l.id === id) ?? LOOKS[0];
