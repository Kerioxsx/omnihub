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
  { id: 'minecraft', name: 'Minecraft', blurb: 'Blocky hills and clouds, a sky that follows your clock, your item pack floating by.', light: false },
  { id: 'matrix', name: 'Matrix', blurb: 'Green code raining down.', light: false },
  { id: 'rain', name: 'Rain', blurb: 'A rainy night with far-off lightning.', light: false },
  { id: 'snowfall', name: 'Snowfall', blurb: 'Soft snow on a winter night.', light: false },
  { id: 'fireflies', name: 'Fireflies', blurb: 'Glowing fireflies over a dark forest.', light: false },
  { id: 'lava', name: 'Lava lamp', blurb: 'Warm blobs rising and sinking.', light: false },
  { id: 'clouds', name: 'Clouds', blurb: 'A bright sky with clouds sailing by.', light: true },
  { id: 'bankai', name: 'Bleach: Bankai', blurb: 'Black and orange, a Getsuga crescent and rising reiatsu.', light: false },
  { id: 'hollow', name: 'Bleach: Hollow', blurb: 'Bone white, manga ink and crimson mask stripes.', light: true },
  { id: 'mugetsu', name: 'Bleach: Mugetsu', blurb: 'Ink black with a purple spiritual glow.', light: false },
];

/** The three Bleach modes, switched from the sidebar while one is on. */
export const BLEACH: { id: Look; kanji: string; name: string }[] = [
  { id: 'bankai', kanji: '卍解', name: 'Bankai' },
  { id: 'hollow', kanji: '虚', name: 'Hollow' },
  { id: 'mugetsu', kanji: '無月', name: 'Mugetsu' },
];

export const lookInfo = (id: string | undefined): LookInfo => LOOKS.find((l) => l.id === id) ?? LOOKS[0];
