// The user's own look for the lyrics: a font file and an emoji pack chosen on
// this PC and kept in OmniHub's data folder (see media/look.rs). Nothing here
// is shipped with OmniHub; these are only the names pictures are looked up by.
//
// With the user's pack as the emoji look, each emoji the lyrics earn is drawn
// as the pack's picture that matches it best, whatever kind of pack it is:
//   1. the word itself ("apple" → apple.png, "diamonds" → diamond.png);
//   2. the emoji's code points, as emoji sets name them (Twemoji 1f525,
//      Noto emoji_u1f525, U+1F525, or 🔥.png itself);
//   3. the emoji's names (red_heart, heart, joy…: CLDR names and the
//      GitHub/Discord short codes, see emojiNames.ts), with the usual
//      prefixes and suffixes of packs ignored ("Fire_3d", "emoji_fire");
//   4. a game item for it (❤️ → heart_pottery_sherd, ⏰ → clock: Minecraft's
//      item names, the most common texture pack).
// An emoji font as the pack draws every emoji in that font. With neither a
// match nor a font, the plain emoji.

import type { AuroraLook } from '@shared/types';
import { useEffect, useState } from 'react';
import { api } from '../../api';
import { EMOJI_NAMES } from './emojiNames';

/** emoji → the pictures that show it, best first ("name*" is any picture whose name starts so). */
const EMOJI_ITEMS: Record<string, string[]> = {
  '👁️': ['ender_eye'], '👀': ['spyglass', 'ender_eye'], '😶': ['player_head'], '👂': ['sculk_sensor'], '🎧': ['jukebox'], '💬': ['oak_sign', 'writable_book'], '📞': ['bell'], '📱': ['recovery_compass', 'compass'], '📢': ['goat_horn', 'bell'], '🤫': ['sculk_shrieker'], '🤥': ['fermented_spider_eye'],
  '🧠': ['brain_coral', 'sculk_catalyst'], '💭': ['echo_shard', 'phantom_membrane'], '🌙': ['clock'], '🔄': ['repeater'], '❤️': ['heart_pottery_sherd', 'red_dye', 'poppy'], '💋': ['heart_pottery_sherd', 'red_dye'], '👄': ['red_dye'], '🫂': ['heart_pottery_sherd'], '🤍': ['white_dye', 'heart_pottery_sherd'], '🤝': ['emerald'], '🙏': ['totem_of_undying'], '😇': ['enchanted_golden_apple', 'golden_apple'], '😈': ['wither_skeleton_skull'], '👻': ['ghast_spawn_egg', 'ghast_tear'], '☁️': ['white_wool'], '🔥': ['fire_charge', 'blaze_powder', 'campfire'], '✨': ['glowstone_dust', 'experience_bottle', 'nether_star'], '🌠': ['firework_star', 'nether_star'], '🌴': ['jungle_sapling'],
  '😊': ['sunflower', 'cookie'], '😂': ['slime_ball'], '😢': ['ghast_tear'], '🥀': ['wither_rose'], '💙': ['heart_of_the_sea', 'blue_dye'], '😤': ['blaze_powder', 'magma_cream'], '🤪': ['pufferfish', 'suspicious_stew'], '😨': ['creeper_head'], '😬': ['creeper_head'], '😳': ['creeper_head'], '😎': ['diamond_helmet', 'diamond'], '😴': ['red_bed'], '🛏️': ['red_bed'], '⏰': ['clock'], '🤒': ['poisonous_potato', 'spider_eye'], '🤕': ['redstone'], '🩸': ['redstone'], '💔': ['heartbreak_pottery_sherd', 'red_dye'], '🥺': ['ghast_tear'], '💀': ['skeleton_skull', 'wither_skeleton_skull'], '🪦': ['skeleton_skull'], '🌱': ['oak_sapling', 'wheat_seeds'], '🕊️': ['feather'], '🪽': ['elytra', 'feather'],
  '⬅️': ['arrow'], '🚪': ['oak_door'], '▶️': ['arrow'], '🛑': ['redstone_torch'], '🔚': ['end_portal_frame', 'end_crystal'], '🔁': ['repeater'], '♾️': ['totem_of_undying', 'ender_pearl'], '🏃': ['leather_boots', 'sugar'], '🚶': ['leather_boots'], '💃': ['music_disc_cat', 'note_block'], '🦘': ['rabbit_foot', 'slime_block'], '🚗': ['minecart'], '🛣️': ['rail'], '🚆': ['minecart', 'rail'], '✈️': ['elytra'], '⛵': ['oak_boat'], '🚢': ['oak_boat'], '🚲': ['saddle'], '🚀': ['firework_rocket'], '⚡': ['lightning_rod', 'redstone'], '🐢': ['turtle_helmet', 'turtle_spawn_egg'], '🏁': ['white_banner'], '🗺️': ['filled_map', 'map'], '🧳': ['bundle', 'chest'],
  '🏙️': ['stone_bricks', 'bricks'], '🏘️': ['oak_planks', 'bricks'], '🏠': ['oak_planks', 'crafting_table'], '🛋️': ['red_wool'], '🪟': ['glass_pane', 'glass'], '🧱': ['bricks', 'brick'], '⛪': ['bell'], '🏫': ['bookshelf'], '🏥': ['glistering_melon_slice'], '🏨': ['red_bed'], '🪩': ['amethyst_cluster', 'jukebox'], '🌍': ['grass_block'], '🪐': ['ender_pearl'], '⛰️': ['stone', 'grass_block'], '🏖️': ['sand'], '🏝️': ['sand', 'jungle_sapling'], '🏜️': ['cactus', 'sand'], '🌲': ['spruce_sapling'], '🌷': ['pink_tulip', 'red_tulip'], '🌳': ['oak_sapling'], '🌉': ['oak_planks'], '⛓️': ['iron_chain', 'chain'],
  '🌌': ['ender_eye', 'end_stone'], '⭐': ['nether_star'], '☀️': ['sunflower', 'daylight_detector'], '🌅': ['sunflower'], '🌇': ['sunflower'], '💡': ['torch', 'lantern', 'glowstone'], '🌑': ['coal', 'black_dye'], '🌧️': ['water_bucket'], '⛈️': ['lightning_rod'], '❄️': ['snowball'], '🥶': ['powder_snow_bucket', 'packed_ice'], '🧊': ['ice', 'packed_ice'], '💨': ['wind_charge', 'breeze_rod'], '🌈': ['beacon'], '🌊': ['water_bucket', 'heart_of_the_sea'], '💧': ['water_bucket'], '🏞️': ['water_bucket'], '🌸': ['pink_petals', 'cherry_sapling'], '🌹': ['rose_bush', 'poppy'], '🍃': ['oak_leaves'],
  '🎉': ['firework_rocket'], '🎵': ['music_disc_cat', 'note_block'], '🎶': ['note_block', 'music_disc_cat'], '🎤': ['goat_horn'], '📻': ['jukebox'], '🥁': ['note_block'], '🎸': ['note_block'], '🎹': ['note_block'], '🔊': ['note_block', 'goat_horn'], '🥂': ['honey_bottle', 'glass_bottle'], '👓': ['spyglass'], '🍾': ['experience_bottle'], '🍷': ['potion*', 'glass_bottle'], '🍺': ['honey_bottle'], '🥴': ['fermented_spider_eye', 'suspicious_stew'], '🍴': ['bread', 'cooked_beef'], '🍽️': ['bowl', 'cooked_beef'], '🍬': ['sugar'], '🍭': ['sugar'], '🍯': ['honey_bottle'], '🎂': ['cake'], '🍒': ['sweet_berries'], '🍑': ['sweet_berries'], '🍎': ['apple'], '🍋': ['yellow_dye'], '☕': ['mushroom_stew'], '🍵': ['mushroom_stew'], '🍕': ['bread', 'pumpkin_pie'],
  '💸': ['emerald'], '💵': ['emerald'], '💰': ['gold_ingot', 'emerald'], '🛍️': ['bundle', 'emerald'], '💎': ['diamond'], '👑': ['golden_helmet'], '💍': ['gold_nugget'], '🎁': ['chest', 'bundle'], '🔑': ['trial_key', 'tripwire_hook'], '🔒': ['iron_door', 'chest'], '🪞': ['glass_pane'], '📸': ['painting', 'item_frame'], '🎬': ['painting'], '📺': ['observer'], '💌': ['writable_book', 'paper'], '✍️': ['writable_book'], '📖': ['book'], '📄': ['paper'], '⏳': ['clock'], '⏱️': ['clock'], '🎮': ['diamond_sword'], '🏆': ['totem_of_undying'], '🏅': ['gold_nugget'], '💯': ['experience_bottle'], '🥊': ['diamond_sword'], '💣': ['tnt'], '💥': ['tnt', 'fire_charge'], '🧽': ['sponge'], '🎨': ['red_dye', 'painting'],
  '👕': ['leather_chestplate'], '👗': ['leather_chestplate'], '👟': ['leather_boots'], '👠': ['golden_boots'], '👖': ['leather_leggings'], '🧢': ['leather_helmet'], '🧥': ['leather_chestplate'], '🕶️': ['spyglass'], '💄': ['red_dye'], '💇': ['shears'], '💅': ['pink_dye'],
  '🐱': ['cat_spawn_egg'], '🐶': ['wolf_spawn_egg', 'bone'], '🦁': ['ocelot_spawn_egg'], '🐯': ['ocelot_spawn_egg'], '🐺': ['wolf_spawn_egg'], '🦊': ['fox_spawn_egg'], '🐻': ['polar_bear_spawn_egg'], '🐍': ['spider_eye'], '🐦': ['parrot_spawn_egg', 'feather'], '🦋': ['allay_spawn_egg'], '🐝': ['bee_spawn_egg', 'honeycomb'], '🐴': ['horse_spawn_egg', 'saddle'], '🐟': ['cod', 'tropical_fish'], '🦈': ['guardian_spawn_egg', 'cod'], '🐋': ['dolphin_spawn_egg'], '🐬': ['dolphin_spawn_egg'], '🐉': ['dragon_head', 'dragon_egg'], '🦄': ['horse_spawn_egg'], '🐒': ['panda_spawn_egg'], '🐰': ['rabbit_foot', 'rabbit_spawn_egg'], '🦉': ['parrot_spawn_egg', 'feather'], '🕷️': ['spider_eye', 'cobweb'],
  '😍': ['heart_pottery_sherd'], '🌬️': ['wind_charge'], '💐': ['peony', 'poppy'], '🥃': ['honey_bottle'], '🥇': ['gold_ingot'], '🏎️': ['minecart'], '🏍️': ['saddle'], '👋': ['player_head'], '🗼': ['end_rod', 'lightning_rod'], '⚔️': ['diamond_sword', 'iron_sword'],
};

/**
 * Words that are a picture in a texture pack but have no emoji of their own
 * (English and Spanish, accents dropped), with the emoji used where the pack
 * lacks the picture.
 */
export const TEXTURE_WORDS: Record<string, { items: string[]; emoji: string }> = {
  feather: { items: ['feather'], emoji: '🕊️' }, pluma: { items: ['feather'], emoji: '🕊️' }, plumas: { items: ['feather'], emoji: '🕊️' },
  sword: { items: ['diamond_sword', 'iron_sword'], emoji: '⚔️' }, espada: { items: ['diamond_sword', 'iron_sword'], emoji: '⚔️' },
  shield: { items: ['shield'], emoji: '⚔️' }, escudo: { items: ['shield'], emoji: '⚔️' },
  bone: { items: ['bone'], emoji: '💀' }, bones: { items: ['bone'], emoji: '💀' }, hueso: { items: ['bone'], emoji: '💀' }, huesos: { items: ['bone'], emoji: '💀' },
  compass: { items: ['compass'], emoji: '🗺️' }, brujula: { items: ['compass'], emoji: '🗺️' },
  bell: { items: ['bell'], emoji: '🎶' }, campana: { items: ['bell'], emoji: '🎶' },
  torch: { items: ['torch'], emoji: '🔥' }, antorcha: { items: ['torch'], emoji: '🔥' }, lantern: { items: ['lantern'], emoji: '💡' }, candle: { items: ['candle'], emoji: '🔥' }, vela: { items: ['candle'], emoji: '🔥' }, velas: { items: ['candle'], emoji: '🔥' },
  bread: { items: ['bread'], emoji: '🍽️' }, pan: { items: ['bread'], emoji: '🍽️' }, cookie: { items: ['cookie'], emoji: '🍬' }, galleta: { items: ['cookie'], emoji: '🍬' },
  emerald: { items: ['emerald'], emoji: '💎' }, esmeralda: { items: ['emerald'], emoji: '💎' }, esmeraldas: { items: ['emerald'], emoji: '💎' },
  iron: { items: ['iron_ingot'], emoji: '⛓️' }, hierro: { items: ['iron_ingot'], emoji: '⛓️' }, plata: { items: ['iron_ingot'], emoji: '💰' }, oro: { items: ['gold_ingot'], emoji: '💰' },
  arrow: { items: ['arrow'], emoji: '⬅️' }, flecha: { items: ['arrow'], emoji: '⬅️' }, bow: { items: ['bow'], emoji: '⬅️' },
  sand: { items: ['sand'], emoji: '🏖️' }, arena: { items: ['sand'], emoji: '🏖️' }, stone: { items: ['stone'], emoji: '⛰️' }, piedra: { items: ['stone'], emoji: '⛰️' }, rock: { items: ['stone'], emoji: '⛰️' },
  totem: { items: ['totem_of_undying'], emoji: '✨' }, potion: { items: ['potion*'], emoji: '🍷' }, pocion: { items: ['potion*'], emoji: '🍷' },
  creeper: { items: ['creeper_head'], emoji: '💥' }, tnt: { items: ['tnt'], emoji: '💣' }, dynamite: { items: ['tnt'], emoji: '💣' }, dinamita: { items: ['tnt'], emoji: '💣' },
  pumpkin: { items: ['carved_pumpkin', 'pumpkin'], emoji: '👻' }, calabaza: { items: ['carved_pumpkin', 'pumpkin'], emoji: '👻' },
  bucket: { items: ['bucket'], emoji: '💧' }, chest: { items: ['chest'], emoji: '🎁' }, cofre: { items: ['chest'], emoji: '🎁' },
  egg: { items: ['egg'], emoji: '🍽️' }, huevo: { items: ['egg'], emoji: '🍽️' },
  saddle: { items: ['saddle'], emoji: '🐴' }, trident: { items: ['trident'], emoji: '🌊' }, elytra: { items: ['elytra'], emoji: '🪽' },
  string: { items: ['string'], emoji: '✨' }, lightning: { items: ['lightning_rod'], emoji: '⚡' },
};

/** Lines with nothing to picture still get one now and then (see planEmojis). */
export const FILLER_ITEMS = ['nether_star', 'note_block', 'glowstone_dust', 'amethyst_shard', 'firework_star', 'experience_bottle', 'music_disc_cat', 'diamond'];

export interface TextureSet {
  names: Set<string>;
  sorted: string[];
  /** A pack's names without their usual prefixes and suffixes ("fire_3d" → fire). */
  bare: Map<string, string>;
}

// What packs put around the name: Noto's emoji_u1f525, U+1F525, Fluent's
// Fire_3d / Fire_color / Fire_flat, sizes like fire_72 or fire_128x128.
const PREFIX = /^(?:emoji_u|emoji_|u_|u(?=[0-9a-f]{4,6}(?:_|$)))/;
const SUFFIX = /_(?:3d|color|colour|flat|high_contrast|default|animated|emoji|icon|\d{2,4}(?:x\d{2,4})?|\d?x)$/;

/** A pack's picture names, ready for lookups. */
export function textureSet(list: string[]): TextureSet {
  const bare = new Map<string, string>();
  const sorted = [...list].sort();
  for (const n of sorted) {
    let b = n;
    for (let i = 0; i < 3; i++) b = b.replace(PREFIX, '').replace(SUFFIX, '');
    if (b && b !== n && !bare.has(b)) bare.set(b, n);
  }
  return { names: new Set(list), sorted, bare };
}

/** The pack's picture by one of its names (exact, else without prefix/suffix). */
function named(look: TextureSet, n: string): string | null {
  return look.names.has(n) ? n : (look.bare.get(n) ?? null);
}

function pick(candidates: string[] | undefined, look: TextureSet): string | null {
  for (const c of candidates ?? []) {
    if (c.endsWith('*')) {
      const prefix = c.slice(0, -1);
      const hit = look.sorted.find((n) => n.startsWith(prefix));
      if (hit) return hit;
    } else {
      const hit = named(look, c);
      if (hit) return hit;
    }
  }
  return null;
}

/** How emoji sets name a picture after the emoji's code points (1f525, 2764_fe0f, 2764). */
export function codeNames(emoji: string): string[] {
  const cps = [...emoji].map((c) => (c.codePointAt(0) ?? 0).toString(16));
  const full = cps.join('_');
  const plain = cps.filter((c) => c !== 'fe0f').join('_');
  return [...new Set([full, plain])];
}

/** Every name a pack could file this emoji under, best first. */
function emojiCandidates(emoji: string): string[] {
  return [...codeNames(emoji), ...(EMOJI_NAMES[emoji] ?? [])];
}

/**
 * The picture for an emoji the lyrics earned (see the top of this file).
 * `filler`: one of the now-and-then emojis for a line with nothing to
 * picture. Null when the pack has nothing for it.
 */
export function textureFor(look: TextureSet, emoji: string, word?: string, filler?: number): string | null {
  if (filler !== undefined) return pick(emojiCandidates(emoji), look) ?? pick(FILLER_ITEMS.slice(filler % FILLER_ITEMS.length).concat(FILLER_ITEMS), look);
  if (word) {
    const own = pick(TEXTURE_WORDS[word]?.items, look);
    if (own) return own;
    // The word is a picture's name ("apple", "diamond", "clock").
    if (word.length >= 3) {
      const hit = named(look, word);
      if (hit) return hit;
    }
  }
  return pick(emojiCandidates(emoji), look) ?? pick(EMOJI_ITEMS[emoji], look);
}

/** The chosen font and emoji pack (re-read when the user changes them). */
export function useAuroraLook(): { look: AuroraLook | null; textures: TextureSet | null } {
  const [look, setLook] = useState<AuroraLook | null>(null);
  useEffect(() => {
    const load = () =>
      void api.media.look().then(
        (l) => setLook(l),
        () => setLook(null),
      );
    load();
    window.addEventListener(LOOK_CHANGED, load);
    return () => window.removeEventListener(LOOK_CHANGED, load);
  }, []);
  const [textures, setTextures] = useState<TextureSet | null>(null);
  useEffect(() => {
    const list = look?.textures ?? [];
    setTextures(list.length ? textureSet(list) : null);
  }, [look]);
  return { look, textures };
}

/** Said (on window) after the font or textures change. */
export const LOOK_CHANGED = 'omnihub:aurora-look';

const pictures = new Map<string, Promise<string | null>>();

/** Forget the loaded pictures (after new textures are chosen). */
export function forgetPictures(): void {
  pictures.clear();
}

/** A texture's picture as a data URL (loaded once). */
export function useTexture(name: string | null): string | null {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    if (!name) return setUrl(null);
    let alive = true;
    let p = pictures.get(name);
    if (!p) {
      p = api.media.texture(name).catch(() => null);
      pictures.set(name, p);
    }
    void p.then((u) => alive && setUrl(u));
    return () => {
      alive = false;
    };
  }, [name]);
  return url;
}

/** The family name the chosen font is used under. */
export const CUSTOM_FONT = 'OmniHub Custom Lyrics';

/** Load the chosen font as CUSTOM_FONT while `on` (once per change). */
export function useCustomFont(on: boolean, fontName: string | null | undefined): void {
  useEffect(() => {
    if (!on || !fontName) return;
    let alive = true;
    void api.media.font().then((url) => {
      if (!alive || !url) return;
      let el = document.getElementById('omnihub-custom-font') as HTMLStyleElement | null;
      if (!el) {
        el = document.createElement('style');
        el.id = 'omnihub-custom-font';
        document.head.append(el);
      }
      el.textContent = `@font-face{font-family:"${CUSTOM_FONT}";src:url(${url});font-display:block}`;
    });
    return () => {
      alive = false;
    };
  }, [on, fontName]);
}

/** The family name the emoji pack's font is used under. */
export const CUSTOM_EMOJI_FONT = 'OmniHub Custom Emoji';

/** Load the emoji pack's font as CUSTOM_EMOJI_FONT while `on`. */
export function useCustomEmojiFont(on: boolean, fontName: string | null | undefined): void {
  useEffect(() => {
    if (!on || !fontName) return;
    let alive = true;
    void api.media.emojiFont().then((url) => {
      if (!alive || !url) return;
      let el = document.getElementById('omnihub-custom-emoji-font') as HTMLStyleElement | null;
      if (!el) {
        el = document.createElement('style');
        el.id = 'omnihub-custom-emoji-font';
        document.head.append(el);
      }
      el.textContent = `@font-face{font-family:"${CUSTOM_EMOJI_FONT}";src:url(${url});font-display:block}`;
    });
    return () => {
      alive = false;
    };
  }, [on, fontName]);
}

const pixelArt = new Map<string, boolean>();

/**
 * Whether a picture is pixel art (a game's 16×16 icon, maybe stored larger
 * in blocks): such pictures are drawn without smoothing so they stay crisp.
 */
export function isPixelArt(img: HTMLImageElement, key: string): boolean {
  const known = pixelArt.get(key);
  if (known !== undefined) return known;
  const w = img.naturalWidth;
  const h = img.naturalHeight;
  let answer = false;
  if (w > 0 && h > 0 && w <= 48 && h <= 48) answer = !img.src.startsWith('data:image/svg');
  else if (w > 0 && h > 0 && w <= 1024 && h <= 1024 && !img.src.startsWith('data:image/svg')) {
    try {
      const c = document.createElement('canvas');
      c.width = w;
      c.height = h;
      const g = c.getContext('2d', { willReadFrequently: true });
      if (g) {
        g.drawImage(img, 0, 0);
        const d = g.getImageData(0, 0, w, h).data;
        // Blocks of the size a 16-, 24- or 32-pixel icon was scaled up by.
        for (const base of [16, 24, 32]) {
          const k = w / base;
          if (!Number.isInteger(k) || k < 2 || h % k !== 0) continue;
          let same = 0;
          let n = 0;
          for (let i = 0; i < 600; i++) {
            const x = Math.floor(((i * 7919) % 997) / 997 * w);
            const y = Math.floor(((i * 104729) % 991) / 991 * h);
            const a = (y * w + x) * 4;
            const b = ((y - (y % k)) * w + (x - (x % k))) * 4;
            n++;
            if (d[a] === d[b] && d[a + 1] === d[b + 1] && d[a + 2] === d[b + 2] && d[a + 3] === d[b + 3]) same++;
          }
          if (same / n > 0.97) {
            answer = true;
            break;
          }
        }
      }
    } catch {
      answer = false;
    }
  }
  pixelArt.set(key, answer);
  return answer;
}
