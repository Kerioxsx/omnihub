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
  { id: 'photo', name: 'Your photo', blurb: 'Any photo, sized for 4K, with the colours taken from it.', light: false },
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
  { id: 'bankai', name: 'Bleach: Bankai', blurb: 'Ember-orange glass, Getsuga energy trails, sword cuts and rising embers.', light: false },
  { id: 'hollow', name: 'Bleach: Hollow', blurb: 'Bone white, black ink splatter, torn manga panels and red eyes in the dark.', light: true },
  { id: 'mugetsu', name: 'Bleach: Mugetsu', blurb: 'Near-black, slow dark fog and violet distortion that swallows the light.', light: false },
  { id: 'hakka', name: 'Bleach: Hakka no Togame', blurb: 'A frozen night: snowfall, floating ice crystals, frost ribbons and frosted glass.', light: false },
  { id: 'soul', name: 'Bleach: Soul Society', blurb: 'Daylight over white walls and towers, with spirit lights rising.', light: true },
  { id: 'hueco', name: 'Bleach: Hueco Mundo', blurb: 'White dunes under a black sky and a thin crescent moon.', light: false },
  { id: 'karakura', name: 'Bleach: Karakura night', blurb: 'A town skyline at night, a full moon and spirit lights.', light: false },
  { id: 'senbon', name: 'Bleach: Senbonzakura', blurb: 'A thousand pink blades drifting through the dark.', light: false },
];

/** Looks that can show a picture of your own behind the cards (besides the Bleach modes). */
export const PICTURE_LOOKS: string[] = ['soul', 'hueco', 'karakura', 'senbon'];

/** The Bleach modes, switched from the sidebar while one is on. */
export const BLEACH: { id: Look; kanji: string; name: string; sub: string; color: string; tint: string }[] = [
  { id: 'bankai', kanji: '卍解', name: 'Bankai', sub: 'Tensa Zangetsu', color: '#ff6500', tint: '#ff7a1f' },
  { id: 'hollow', kanji: '虚', name: 'Hollow', sub: 'Inner Hollow', color: '#d11f30', tint: '#17171c' },
  { id: 'mugetsu', kanji: '無月', name: 'Mugetsu', sub: 'Final Getsuga Tenshō', color: '#8e4ac9', tint: '#a066e0' },
  { id: 'hakka', kanji: '白霞罸', name: 'Hakka no Togame', sub: 'Frozen elegance', color: '#8fd3ff', tint: '#bfe6ff' },
];

export const isBleach = (id: string | undefined) => BLEACH.some((m) => m.id === id);

export const lookInfo = (id: string | undefined): LookInfo => LOOKS.find((l) => l.id === id) ?? LOOKS[0];
