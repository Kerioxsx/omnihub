// Emojis beside the lyrics, the way people decorate them in edits like the
// reference videos: now and then, one well-chosen emoji beside a word you can
// picture or feel ("👁 saw", "city 🏙", "let go 🕊", "broken heart 💔"),
// never one after every line.
//
// Each line is read once per song: a phrase ("let go", "fall apart") first,
// then single words by their plain form ("eating" → eat, "goin'" → go,
// "saw" → see). A planner spaces the picks over the song: by default about
// one every two or three lines (very common words like "love" less often),
// and never the same emoji twice in a row. In word-by-word lyrics the emoji
// sits beside the word that earned it.
//
// They look glossy and 3D, close to the iPhone's: Microsoft's Fluent Emoji
// (MIT licence), shipped in public/emoji/3d. Apple's own artwork cannot be
// shipped (its licence does not allow it). "system" uses Windows' emoji font.

import type { EmojiAmount } from '@shared/types';

/** word → emoji. Only words that clearly mean one picture or one feeling. */
const WORDS: Record<string, string> = {
  // seeing, saying, hearing
  see: '👁️', eye: '👀', look: '👀', watch: '👀', stare: '👀', face: '😶', hear: '👂', listen: '🎧', ear: '👂', say: '💬', talk: '💬', text: '💬', message: '💬', call: '📞', phone: '📱', scream: '📢', shout: '📢', whisper: '🤫', secret: '🤫', quiet: '🤫', silence: '🤫', lie: '🤥', liar: '🤥',
  // the head and the heart
  mind: '🧠', brain: '🧠', think: '💭', thought: '💭', dream: '🌙', remember: '💭', wonder: '💭', memory: '💭', change: '🔄', heart: '❤️', love: '❤️', lover: '❤️', kiss: '💋', lips: '👄', hug: '🫂', hold: '🫂', touch: '🤍', hand: '🤝', pray: '🙏', prayer: '🙏', god: '🙏', bless: '🙏', angel: '😇', devil: '😈', demon: '😈', ghost: '👻', heaven: '☁️', hell: '🔥', soul: '✨', magic: '✨', spark: '✨', shine: '✨', glow: '✨', gold: '✨', wish: '🌠', hope: '✨', euphoria: '✨', paradise: '🌴',
  // feelings
  smile: '😊', happy: '😊', laugh: '😂', joke: '😂', cry: '😢', tear: '😢', sad: '🥀', lonely: '🥀', alone: '🥀', lose: '🥀', loss: '🥀', gone: '🥀', fade: '🥀', blue: '💙', mad: '😤', angry: '😤', rage: '😤', crazy: '🤪', insane: '🤪', scared: '😨', afraid: '😨', fear: '😨', nervous: '😬', shy: '😳', cool: '😎', tired: '😴', sleep: '😴', bed: '🛏️', wake: '⏰', sick: '🤒', hurt: '🤕', pain: '🤕', bleed: '🩸', broken: '💔', heartbreak: '💔', heartbroken: '💔', miss: '🥺', sorry: '🥺', please: '🥺', dead: '💀', die: '💀', death: '💀', grave: '🪦', alive: '🌱', grow: '🌱', free: '🕊️', freedom: '🕊️', peace: '🕊️', fly: '🕊️', wing: '🪽', passion: '🔥', desire: '🔥', wild: '🔥',
  // going places
  left: '⬅️', leave: '🚪', door: '🚪', start: '▶️', begin: '▶️', stop: '🛑', end: '🔚', again: '🔁', forever: '♾️', run: '🏃', walk: '🚶', dance: '💃', jump: '🦘', ride: '🚗', drive: '🚗', car: '🚗', road: '🛣️', street: '🛣️', highway: '🛣️', train: '🚆', plane: '✈️', flight: '✈️', boat: '⛵', ship: '🚢', bike: '🚲', rocket: '🚀', space: '🚀', fast: '⚡', speed: '⚡', slow: '🐢', race: '🏁', map: '🗺️', travel: '🧳', away: '🧳', escape: '🏃',
  // places
  city: '🏙️', town: '🏘️', home: '🏠', house: '🏠', room: '🛋️', window: '🪟', wall: '🧱', church: '⛪', school: '🏫', hospital: '🏥', hotel: '🏨', club: '🪩', world: '🌍', earth: '🌍', globe: '🌍', planet: '🪐', mountain: '⛰️', hill: '⛰️', beach: '🏖️', island: '🏝️', desert: '🏜️', forest: '🌲', garden: '🌷', park: '🌳', tree: '🌳', bridge: '🌉', jail: '⛓️', prison: '⛓️', chain: '⛓️',
  // sky and weather
  sky: '🌌', star: '⭐', moon: '🌙', night: '🌙', tonight: '🌙', midnight: '🌙', sun: '☀️', sunshine: '☀️', summer: '☀️', morning: '🌅', sunrise: '🌅', sunset: '🌇', light: '💡', dark: '🌑', darkness: '🌑', shadow: '🌑', cloud: '☁️', rain: '🌧️', storm: '⛈️', thunder: '⚡', lightning: '⚡', snow: '❄️', winter: '❄️', cold: '🥶', freeze: '🥶', ice: '🧊', fire: '🔥', flame: '🔥', burn: '🔥', hot: '🔥', heat: '🔥', smoke: '💨', wind: '💨', rainbow: '🌈', ocean: '🌊', sea: '🌊', wave: '🌊', water: '💧', river: '🏞️', flower: '🌸', rose: '🌹', petal: '🌸', leaf: '🍃',
  // party, music, food
  party: '🎉', celebrate: '🎉', music: '🎵', song: '🎶', sing: '🎤', mic: '🎤', radio: '📻', beat: '🥁', drum: '🥁', guitar: '🎸', piano: '🎹', loud: '🔊', drink: '🥂', toast: '🥂', glass: '👓', champagne: '🍾', wine: '🍷', beer: '🍺', drunk: '🥴', eat: '🍴', food: '🍽️', hungry: '🍽️', dinner: '🍽️', sugar: '🍬', sweet: '🍭', candy: '🍬', honey: '🍯', cake: '🎂', birthday: '🎂', cherry: '🍒', peach: '🍑', apple: '🍎', lemon: '🍋', coffee: '☕', tea: '🍵', pizza: '🍕',
  // things
  money: '💸', cash: '💵', dollar: '💵', rich: '💰', million: '💰', pay: '💸', buy: '🛍️', shop: '🛍️', diamond: '💎', crown: '👑', king: '👑', queen: '👑', ring: '💍', marry: '💍', wedding: '💍', gift: '🎁', key: '🔑', lock: '🔒', mirror: '🪞', picture: '📸', photo: '📸', camera: '📸', movie: '🎬', screen: '📺', letter: '💌', write: '✍️', book: '📖', paper: '📄', clock: '⏰', time: '⏳', hour: '⏳', wait: '⏳', minute: '⏱️', game: '🎮', trophy: '🏆', win: '🏆', champion: '🏆', medal: '🏅', hundred: '💯', fight: '🥊', bomb: '💣', boom: '💥', crash: '💥', erase: '🧽', paint: '🎨', color: '🎨',
  // wearing
  wear: '👕', shirt: '👕', dress: '👗', shoe: '👟', sneaker: '👟', heel: '👠', jeans: '👖', hat: '🧢', jacket: '🧥', shades: '🕶️', sunglasses: '🕶️', makeup: '💄', lipstick: '💄', perfume: '🌸', hair: '💇', nail: '💅',
  // animals
  cat: '🐱', dog: '🐶', puppy: '🐶', lion: '🦁', tiger: '🐯', wolf: '🐺', fox: '🦊', bear: '🐻', snake: '🐍', bird: '🐦', butterfly: '🦋', bee: '🐝', horse: '🐴', fish: '🐟', shark: '🦈', whale: '🐋', dolphin: '🐬', dragon: '🐉', unicorn: '🦄', monkey: '🐒', bunny: '🐰', rabbit: '🐰', owl: '🦉', dove: '🕊️', spider: '🕷️',
};

/** Phrases that mean more than their words (checked first, by plain forms). */
const PHRASES: { words: string[]; emoji: string }[] = [
  { words: ['let', 'go'], emoji: '🕊️' },
  { words: ['broken', 'heart'], emoji: '💔' },
  { words: ['break', 'heart'], emoji: '💔' },
  { words: ['break', 'my', 'heart'], emoji: '💔' },
  { words: ['fall', 'apart'], emoji: '💔' },
  { words: ['fall', 'in', 'love'], emoji: '😍' },
  { words: ['fly', 'away'], emoji: '🕊️' },
  { words: ['set', 'free'], emoji: '🕊️' },
  { words: ['on', 'fire'], emoji: '🔥' },
  { words: ['all', 'night'], emoji: '🌙' },
  { words: ['good', 'night'], emoji: '🌙' },
  { words: ['shooting', 'star'], emoji: '🌠' },
  { words: ['dance', 'floor'], emoji: '🪩' },
  { words: ['hold', 'on'], emoji: '🫂' },
  { words: ['miss', 'you'], emoji: '🥺' },
  { words: ['say', 'goodbye'], emoji: '👋' },
  { words: ['goodbye'], emoji: '👋' },
];

/** Picture before the word ("👁 saw", "⬅️ left", "▶️ start"); the rest go after. */
const BEFORE = new Set(['see', 'left', 'start', 'begin', 'face', 'stop', 'end', 'again', 'fast', 'ride', 'drive']);
/** Very common in songs: only now and then, or every chorus would be full of hearts. */
const COMMON = new Set(['love', 'lover', 'heart', 'time', 'night', 'tonight', 'world', 'light', 'dark', 'see', 'say', 'talk', 'call', 'home', 'run', 'change', 'wait', 'think', 'dream', 'remember', 'mind', 'hand', 'face', 'look', 'eye', 'sky', 'fire', 'hot', 'cold', 'sun', 'away', 'end', 'start', 'free', 'hear', 'die', 'dead', 'hold', 'gone', 'lose', 'touch', 'wish', 'hope', 'gold', 'alone']);

const IRREGULAR: Record<string, string> = {
  saw: 'see', seen: 'see', ate: 'eat', eaten: 'eat', ran: 'run', flew: 'fly', flown: 'fly', drove: 'drive', driven: 'drive', rode: 'ride', ridden: 'ride', thought: 'think', told: 'tell', said: 'say', won: 'win', broke: 'broken', bought: 'buy', wrote: 'write', written: 'write', sang: 'sing', sung: 'sing', fell: 'fall', fallen: 'fall', woke: 'wake', slept: 'sleep', heard: 'hear', paid: 'pay', fought: 'fight', burnt: 'burn', froze: 'freeze', frozen: 'freeze', drank: 'drink', died: 'die', dying: 'die', lying: 'lie', lied: 'lie', cried: 'cry', eyes: 'eye', feet: 'shoe', wore: 'wear', worn: 'wear', glasses: 'glass', lives: 'alive', began: 'begin', begun: 'begin', grew: 'grow', grown: 'grow', held: 'hold', lost: 'lose', left: 'left', dreamt: 'dream', bled: 'bleed', knives: 'knife', leaves: 'leaf', wolves: 'wolf', jeans: 'jeans', shades: 'shades', sunglasses: 'sunglasses', lips: 'lips', news: 'news',
};

/** The plain forms a sung word may stand for ("Dancin'" → dancing → dance). */
export function lemmas(raw: string): string[] {
  let w = raw.toLowerCase().replace(/[’‘`]/g, "'").replace(/[^a-z0-9']/g, '');
  w = w.replace(/^'+/, '');
  if (w.endsWith("in'")) w = `${w.slice(0, -1)}g`;
  w = w.replace(/'s$/, '').replace(/'/g, '');
  if (!w) return [];
  const out = new Set<string>([w]);
  if (IRREGULAR[w]) {
    out.add(IRREGULAR[w]);
    return [...out];
  }
  const undouble = (s: string) => (/([b-df-hj-np-tv-z])\1$/.test(s) ? s.slice(0, -1) : s);
  if (w.endsWith('ing') && w.length > 4) {
    const b = w.slice(0, -3);
    out.add(b).add(`${b}e`).add(undouble(b));
  } else if (w.endsWith('ied') && w.length > 4) out.add(`${w.slice(0, -3)}y`);
  else if (w.endsWith('ed') && w.length > 3) {
    const b = w.slice(0, -2);
    out.add(b).add(`${b}e`).add(undouble(b));
  } else if (w.endsWith('ies') && w.length > 4) out.add(`${w.slice(0, -3)}y`);
  else if (w.endsWith('es') && w.length > 3) out.add(w.slice(0, -2)).add(w.slice(0, -1));
  else if (w.endsWith('s') && !w.endsWith('ss') && w.length > 3) out.add(w.slice(0, -1));
  return [...out];
}

/** The emoji for one word, with the plain form that matched. */
export function emojiFor(word: string): { emoji: string; lemma: string } | null {
  for (const l of lemmas(word)) {
    const e = WORDS[l];
    if (e) return { emoji: e, lemma: l };
  }
  return null;
}

/** The best emoji for a line: a phrase in it, else its strongest word. `at` is the word it belongs to. */
export function lineEmoji(words: string[]): { emoji: string; at: number; before: boolean; common: boolean } | null {
  const forms = words.map(lemmas);
  for (const p of PHRASES) {
    for (let i = 0; i + p.words.length <= words.length; i++) {
      if (p.words.every((pw, k) => forms[i + k].includes(pw))) return { emoji: p.emoji, at: i + p.words.length - 1, before: false, common: false };
    }
  }
  let best: { emoji: string; at: number; before: boolean; common: boolean } | null = null;
  words.forEach((w, i) => {
    const hit = emojiFor(w);
    if (!hit) return;
    const common = COMMON.has(hit.lemma);
    // A word you can picture beats a very common one; otherwise the later word
    // (lines tend to end on what they are about).
    if (!best || (best.common && !common) || best.common === common) best = { emoji: hit.emoji, at: i, before: BEFORE.has(hit.lemma), common };
  });
  return best;
}

export interface EmojiPick {
  emoji: string;
  /** Shown before the word (else after). */
  before: boolean;
}

/**
 * Where the emojis go in a song. `steps` are what the lyrics show one at a time
 * (words, or whole lines), each with the line it belongs to. At most one emoji
 * per line; "some" leaves at least one line between them (two after a very
 * common word), "more" may use every line; never the same emoji twice running.
 */
export function planEmojis(steps: { text: string; line: number }[], amount: EmojiAmount): (EmojiPick | null)[] {
  const out: (EmojiPick | null)[] = steps.map(() => null);
  if (amount === 'off' || !steps.length) return out;
  const more = amount === 'more';
  // The steps of each line, in order.
  const lines = new Map<number, number[]>();
  steps.forEach((s, i) => lines.set(s.line, [...(lines.get(s.line) ?? []), i]));
  let lastLine = -Infinity;
  let lastEmoji = '';
  let order = 0;
  for (const [, idx] of [...lines.entries()].sort((a, b) => a[0] - b[0])) {
    order += 1;
    // The line's words, and which step each word is in.
    const words: string[] = [];
    const stepOf: number[] = [];
    for (const i of idx)
      for (const w of steps[i].text.split(/\s+/).filter(Boolean)) {
        words.push(w);
        stepOf.push(i);
      }
    const hit = lineEmoji(words);
    if (!hit) continue;
    const need = more ? (hit.common ? 2 : 1) : hit.common ? 3 : 2;
    if (order - lastLine < need || hit.emoji === lastEmoji) continue;
    const step = stepOf[hit.at];
    // A whole line shown at once gets its emoji at the end.
    out[step] = { emoji: hit.emoji, before: idx.length > 1 && hit.before };
    lastLine = order;
    lastEmoji = hit.emoji;
  }
  return out;
}

/** The bundled 3D picture: its code points without the variation selector. */
export function emojiFile(emoji: string): string {
  const cps = [...emoji].map((c) => c.codePointAt(0) ?? 0).filter((c) => c !== 0xfe0f);
  return `/emoji/3d/${cps.map((c) => c.toString(16)).join('-')}.webp`;
}

/** Every emoji used (for the asset script and tests). */
export const ALL_EMOJIS = [...new Set([...Object.values(WORDS), ...PHRASES.map((p) => p.emoji)])];
