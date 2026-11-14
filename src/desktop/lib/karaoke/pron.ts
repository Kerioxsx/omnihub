// Pronunciation for Karaoke Studio: the lyrics re-spelled so your own
// language reads them aloud close to how they're sung. Rules only, no
// dictionary, so it only does what rules can do reliably:
//   - Cyrillic (Russian, Ukrainian), Japanese kana, Korean Hangul and Greek
//     are converted sound by sound for Spanish, Portuguese, English, French,
//     German or Italian readers (others get a standard romanization);
//   - Spanish lyrics get syllables and the stressed one (Spanish spelling
//     shows stress), and a re-spelling for English and Portuguese readers;
//   - everything else gets syllables only.
// Sounds your language doesn't have are flagged with a short tip. What
// would need a dictionary (kanji, Chinese characters, English words,
// Russian stress) is said, never guessed.

export type Reader = 'es' | 'pt' | 'en' | 'fr' | 'de' | 'it' | 'neutral';

export interface WordGuide {
  /** How to say it, in the reader's spelling ("" when nothing to add). */
  say: string;
  /** Syllables, the stressed one in capitals when known. */
  syllables: string;
  /** Tips for sounds the reader's language doesn't have. */
  tips: string[];
}

export interface SongLanguage {
  code: string;
  name: string;
  script: 'latin' | 'cyrillic' | 'kana' | 'han' | 'hangul' | 'greek' | 'arabic' | 'hebrew' | 'devanagari' | 'thai' | 'other';
}

// ---------- which language a song is in ----------

const STOP: Record<string, string[]> = {
  es: ['el', 'la', 'que', 'de', 'y', 'en', 'no', 'me', 'te', 'mi', 'tu', 'yo', 'lo', 'es', 'por', 'con', 'una', 'quiero', 'amor', 'corazón', 'nada', 'pero'],
  en: ['the', 'and', 'you', 'i', 'to', 'my', 'me', 'it', 'is', 'in', 'of', 'that', 'on', 'love', 'your', 'we', 'all', 'just', 'like', 'know', 'baby', "i'm", "don't"],
  pt: ['o', 'a', 'que', 'de', 'e', 'não', 'eu', 'você', 'meu', 'minha', 'um', 'uma', 'com', 'pra', 'amor', 'coração', 'te', 'é', 'se', 'mais', 'tudo'],
  fr: ['le', 'la', 'les', 'et', 'je', 'tu', 'de', 'que', 'pas', 'un', 'une', 'est', 'mon', 'ma', 'toi', 'moi', "c'est", 'dans', 'pour', 'amour'],
  de: ['der', 'die', 'das', 'und', 'ich', 'du', 'nicht', 'ist', 'ein', 'eine', 'mein', 'dich', 'mich', 'wir', 'liebe', 'zu', 'auf', 'mit'],
  it: ['il', 'la', 'che', 'di', 'e', 'non', 'io', 'tu', 'mi', 'ti', 'un', 'una', 'per', 'amore', 'sei', 'è', 'cuore', 'con', 'come'],
};
const LANG_NAMES: Record<string, string> = { es: 'Spanish', en: 'English', pt: 'Portuguese', fr: 'French', de: 'German', it: 'Italian', ru: 'Russian', uk: 'Ukrainian', ja: 'Japanese', ko: 'Korean', zh: 'Chinese', el: 'Greek', ar: 'Arabic', he: 'Hebrew', hi: 'Hindi', th: 'Thai' };

export function detectLanguage(text: string): SongLanguage {
  const count = (re: RegExp) => (text.match(re) ?? []).length;
  const kana = count(/[぀-ヿ]/g);
  const han = count(/[一-鿿]/g);
  const scripts: [SongLanguage['script'], string, number][] = [
    ['cyrillic', /[іїєґ]/i.test(text) ? 'uk' : 'ru', count(/[Ѐ-ӿ]/g)],
    ['kana', 'ja', kana + (kana ? han : 0)],
    ['han', 'zh', kana ? 0 : han],
    ['hangul', 'ko', count(/[가-힯]/g)],
    ['greek', 'el', count(/[Ͱ-Ͽ]/g)],
    ['arabic', 'ar', count(/[؀-ۿ]/g)],
    ['hebrew', 'he', count(/[֐-׿]/g)],
    ['devanagari', 'hi', count(/[ऀ-ॿ]/g)],
    ['thai', 'th', count(/[฀-๿]/g)],
  ];
  const latin = count(/[a-zà-ÿ]/gi);
  const best = scripts.sort((a, b) => b[2] - a[2])[0];
  if (best[2] > latin * 0.3 && best[2] > 3) return { code: best[1], name: LANG_NAMES[best[1]], script: best[0] };
  const words = text.toLowerCase().match(/[a-zà-ÿ']+/g) ?? [];
  let top = 'en';
  let topScore = -1;
  for (const [lang, list] of Object.entries(STOP)) {
    const set = new Set(list);
    const score = words.filter((w) => set.has(w)).length + (lang === 'es' && /ñ|¿|¡/.test(text) ? 5 : 0) + (lang === 'pt' && /ã|õ|ç/.test(text) ? 5 : 0) + (lang === 'de' && /ß|ü|ö|ä/.test(text) ? 3 : 0);
    if (score > topScore) {
      topScore = score;
      top = lang;
    }
  }
  return { code: top, name: LANG_NAMES[top], script: 'latin' };
}

/** Which spelling a first language reads in ("es-MX" → "es"). */
export function readerOf(lang: string): Reader {
  const base = lang.split('-')[0];
  return (['es', 'pt', 'en', 'fr', 'de', 'it'] as Reader[]).includes(base as Reader) ? (base as Reader) : base === 'gl' || base === 'ca' ? 'es' : 'neutral';
}

// ---------- tips ----------

const TIP = {
  zh: 'ж: like the “s” in “measure” (a buzzing “sh”)',
  kh: 'х: a breathy sound in the throat, like Spanish “j” or Scottish “loch”',
  sh: 'ш: “sh” as in English “shoe”',
  y: 'ы: say “i” with the tongue pulled back, a little like a muffled “i”',
  soft: 'ь: the consonant before it is soft (a tiny “y” after it)',
  v: 'в: a real “v” — upper teeth on the lower lip',
  z: 'з: a buzzing “s”, like a bee',
  shch: 'щ: a long soft “sh”',
  stress: 'Russian stress isn’t written; the stressed vowel is louder and longer',
  tsu: 'tsu: “ts” as at the end of “cats”, then “u”',
  fu: 'fu: blow softly between the lips, they don’t touch the teeth',
  jHard: 'j/y: like English “j” in “jeep”, but softer',
  hSoft: 'h: a soft breath, like a sigh',
  eo: 'eo: an open “o”, mouth relaxed',
  eu: 'eu: “u” with the lips not rounded (smile while saying “u”)',
  thGreek: 'θ: “th” as in “think”',
  dGreek: 'δ: “th” as in “this”',
  rr: 'rr: a rolled “r”',
  kanji: 'Kanji: their reading needs a dictionary OmniHub doesn’t have yet',
  hanzi: 'Chinese characters: their reading needs a dictionary OmniHub doesn’t have yet',
};

// ---------- Cyrillic ----------

const VOWELS_CY = 'аеёиоуыэюяіїє';

function cyrillic(word: string, reader: Reader, ukrainian: boolean): { say: string; tips: string[] } {
  const w = word.toLowerCase();
  const tips = new Set<string>();
  let out = '';
  for (let i = 0; i < w.length; i++) {
    const c = w[i];
    const prev = w[i - 1] ?? '';
    const next = w[i + 1] ?? '';
    const startOrAfterVowel = !prev || VOWELS_CY.includes(prev) || prev === 'ь' || prev === 'ъ';
    const front = 'еиёяюєіїь'.includes(next);
    const iot = (base: string) => {
      // е, ё, ю, я: a "y" sound at the start or after a vowel.
      if (reader === 'es' || reader === 'pt' || reader === 'it') return startOrAfterVowel ? 'i' + base : base === 'e' ? 'e' : 'i' + base;
      if (reader === 'de') return 'j' + base;
      if (reader === 'fr') return base === 'e' ? (startOrAfterVowel ? 'ié' : 'é') : 'i' + (base === 'u' ? 'ou' : base);
      return startOrAfterVowel ? 'y' + base : base === 'e' ? 'e' : 'y' + base;
    };
    const m: Record<string, () => string> = {
      а: () => 'a',
      б: () => 'b',
      в: () => {
        if (reader === 'es') tips.add(TIP.v);
        return reader === 'de' ? 'w' : 'v';
      },
      г: () => {
        if (ukrainian) {
          if (reader === 'es') tips.add(TIP.hSoft);
          return reader === 'es' ? 'j' : 'h';
        }
        return (reader === 'es' || reader === 'pt') && front ? 'gu' : reader === 'it' && front ? 'gh' : 'g';
      },
      ґ: () => 'g',
      д: () => 'd',
      е: () => (ukrainian ? 'e' : iot('e')),
      є: () => iot('e'),
      ё: () => iot('o'),
      ж: () => {
        if (reader === 'pt' || reader === 'fr') return 'j';
        tips.add(TIP.zh);
        return reader === 'es' || reader === 'it' ? 'sh' : reader === 'de' ? 'sch' : 'zh';
      },
      з: () => {
        if (reader === 'es') tips.add(TIP.z);
        return reader === 'de' ? 's' : 'z';
      },
      и: () => (ukrainian ? (reader === 'neutral' || reader === 'en' ? 'y' : 'i') : 'i'),
      і: () => 'i',
      ї: () => (reader === 'de' ? 'ji' : reader === 'es' || reader === 'pt' || reader === 'it' ? 'ii' : 'yi'),
      й: () => (reader === 'de' ? 'j' : reader === 'es' || reader === 'pt' || reader === 'it' || reader === 'fr' ? 'i' : 'y'),
      к: () => (reader === 'it' && front ? 'ch' : reader === 'it' ? 'c' : 'k'),
      л: () => 'l',
      м: () => 'm',
      н: () => 'n',
      о: () => 'o',
      п: () => 'p',
      р: () => {
        if (reader === 'en' || reader === 'fr' || reader === 'de') tips.add(TIP.rr);
        return 'r';
      },
      с: () => (reader === 'pt' && VOWELS_CY.includes(prev) && VOWELS_CY.includes(next) ? 'ss' : 's'),
      т: () => 't',
      у: () => (reader === 'fr' ? 'ou' : 'u'),
      ф: () => 'f',
      х: () => {
        if (reader === 'es') return 'j';
        if (reader === 'de') return 'ch';
        if (reader === 'pt') return 'rr';
        tips.add(TIP.kh);
        return reader === 'it' ? 'h' : 'kh';
      },
      ц: () => (reader === 'de' || reader === 'it' ? 'z' : 'ts'),
      ч: () => (reader === 'de' ? 'tsch' : reader === 'pt' || reader === 'fr' ? 'tch' : reader === 'it' ? (front ? 'c' : 'ci') : 'ch'),
      ш: () => {
        if (reader === 'pt' || reader === 'fr') return 'ch';
        if (reader === 'de') return 'sch';
        if (reader === 'es' || reader === 'it') tips.add(TIP.sh);
        return 'sh';
      },
      щ: () => {
        tips.add(TIP.shch);
        return reader === 'pt' || reader === 'fr' ? 'ch' : reader === 'de' ? 'schtsch' : reader === 'en' || reader === 'neutral' ? 'shch' : 'sh';
      },
      ъ: () => '',
      ы: () => {
        tips.add(TIP.y);
        return reader === 'en' || reader === 'neutral' ? 'y' : 'i';
      },
      ь: () => {
        tips.add(TIP.soft);
        return reader === 'en' || reader === 'neutral' ? "'" : '';
      },
      э: () => (reader === 'fr' ? 'è' : 'e'),
      ю: () => iot('u'),
      я: () => iot('a'),
    };
    out += m[c]?.() ?? c;
  }
  if (!ukrainian && /[аеёиоуыэюя].*[аеёиоуыэюя]/.test(w)) tips.add(TIP.stress);
  return { say: out, tips: [...tips] };
}

// ---------- Japanese kana ----------

const KANA: Record<string, string> = {
  あ: 'a', い: 'i', う: 'u', え: 'e', お: 'o', か: 'ka', き: 'ki', く: 'ku', け: 'ke', こ: 'ko', さ: 'sa', し: 'shi', す: 'su', せ: 'se', そ: 'so', た: 'ta', ち: 'chi', つ: 'tsu', て: 'te', と: 'to',
  な: 'na', に: 'ni', ぬ: 'nu', ね: 'ne', の: 'no', は: 'ha', ひ: 'hi', ふ: 'fu', へ: 'he', ほ: 'ho', ま: 'ma', み: 'mi', む: 'mu', め: 'me', も: 'mo', や: 'ya', ゆ: 'yu', よ: 'yo',
  ら: 'ra', り: 'ri', る: 'ru', れ: 're', ろ: 'ro', わ: 'wa', を: 'o', ん: 'n', が: 'ga', ぎ: 'gi', ぐ: 'gu', げ: 'ge', ご: 'go', ざ: 'za', じ: 'ji', ず: 'zu', ぜ: 'ze', ぞ: 'zo',
  だ: 'da', ぢ: 'ji', づ: 'zu', で: 'de', ど: 'do', ば: 'ba', び: 'bi', ぶ: 'bu', べ: 'be', ぼ: 'bo', ぱ: 'pa', ぴ: 'pi', ぷ: 'pu', ぺ: 'pe', ぽ: 'po', ゔ: 'vu',
  ぁ: 'a', ぃ: 'i', ぅ: 'u', ぇ: 'e', ぉ: 'o',
};
const SMALL_Y: Record<string, string> = { ゃ: 'a', ゅ: 'u', ょ: 'o' };

function hepburn(word: string): string {
  // Katakana as hiragana.
  const h = [...word].map((c) => (c >= 'ァ' && c <= 'ヶ' ? String.fromCharCode(c.charCodeAt(0) - 0x60) : c)).join('');
  let out = '';
  let double = false;
  for (let i = 0; i < h.length; i++) {
    const c = h[i];
    if (c === 'っ') {
      double = true;
      continue;
    }
    if (c === 'ー') {
      out += out.slice(-1);
      continue;
    }
    let syl = KANA[c];
    if (syl === undefined) {
      out += c;
      continue;
    }
    const y = SMALL_Y[h[i + 1]];
    if (y) {
      // きゃ → kya, しゃ → sha, ちゃ → cha, じゃ → ja.
      syl = syl.endsWith('hi') ? syl.slice(0, -1) + y : syl === 'ji' ? 'j' + y : syl.slice(0, -1) + 'y' + y;
      i++;
    }
    if (double) {
      out += syl.startsWith('ch') ? 't' : syl[0];
      double = false;
    }
    if (syl === 'n' && /^[aiueoy]/.test(KANA[h[i + 1]] ?? '')) syl = "n'";
    out += syl;
  }
  return out.replace(/n(?=[bpm])/g, 'm');
}

function kana(word: string, reader: Reader): { say: string; tips: string[] } {
  const tips = new Set<string>();
  if (/[一-鿿]/.test(word)) tips.add(TIP.kanji);
  let r = hepburn(word);
  if (/tsu/.test(r) && reader !== 'de' && reader !== 'it') tips.add(TIP.tsu);
  if (/fu/.test(r)) tips.add(TIP.fu);
  switch (reader) {
    case 'es':
      if (/sh/.test(r)) tips.add(TIP.sh.replace('ш: ', 'sh: '));
      if (/j/.test(r)) tips.add(TIP.jHard);
      if (/h/.test(r.replace(/[cs]h/g, ''))) tips.add(TIP.hSoft);
      r = r.replace(/(?<![cs])h/g, 'j').replace(/j(?=[aiueo])/g, (m, o, s) => (s[o - 1] === 'j' ? m : 'y')).replace(/ji/g, 'yi');
      break;
    case 'pt':
      r = r.replace(/sh/g, 'x').replace(/ch/g, 'tch').replace(/j/g, 'dj');
      if (/(?<![cstx])h/.test(r)) tips.add(TIP.hSoft);
      break;
    case 'fr':
      r = r.replace(/sh/g, 'ch').replace(/(?<!t)ch(?=i|a|u|o|e)/g, 'tch').replace(/j/g, 'dj').replace(/u/g, 'ou').replace(/y/g, 'y');
      if (/(?<![cs])h/.test(r)) tips.add(TIP.hSoft);
      break;
    case 'de':
      r = r.replace(/sh/g, 'sch').replace(/(?<!s)ch/g, 'tsch').replace(/j/g, 'dsch').replace(/ts/g, 'z').replace(/(?<![dts])z(?!u)/g, 's').replace(/y/g, 'j').replace(/w/g, 'u');
      break;
    case 'it':
      r = r
        .replace(/shi/g, 'sci')
        .replace(/sh/g, 'sci')
        .replace(/chi/g, 'ci')
        .replace(/ch/g, 'ci')
        .replace(/ji/g, 'gi')
        .replace(/j/g, 'gi')
        .replace(/ke/g, 'che')
        .replace(/ki/g, 'chi')
        .replace(/ge/g, 'ghe')
        .replace(/gi(?![aou])/g, 'ghi')
        .replace(/ts/g, 'z')
        .replace(/y/g, 'i');
      if (/(?<![cs])h/.test(r)) tips.add(TIP.hSoft);
      break;
  }
  return { say: r, tips: [...tips] };
}

// ---------- Korean Hangul (Revised Romanization) ----------

const L_RR = ['g', 'kk', 'n', 'd', 'tt', 'r', 'm', 'b', 'pp', 's', 'ss', '', 'j', 'jj', 'ch', 'k', 't', 'p', 'h'];
const V_RR = ['a', 'ae', 'ya', 'yae', 'eo', 'e', 'yeo', 'ye', 'o', 'wa', 'wae', 'oe', 'yo', 'u', 'wo', 'we', 'wi', 'yu', 'eu', 'ui', 'i'];
const T_RR = ['', 'k', 'k', 'k', 'n', 'n', 'n', 't', 'l', 'k', 'm', 'l', 'l', 'l', 'p', 'l', 'm', 'p', 'p', 't', 't', 'ng', 't', 't', 'k', 't', 'p', 't'];
/** A final consonant carried over to a following silent initial (liaison). */
const T_LINK = ['', 'g', 'kk', 'ks', 'n', 'nj', 'nh', 'd', 'r', 'lg', 'lm', 'lb', 'ls', 'lt', 'lp', 'lh', 'm', 'b', 'ps', 's', 'ss', 'ng', 'j', 'ch', 'k', 't', 'p', 'h'];

function hangul(word: string, reader: Reader): { say: string; tips: string[] } {
  const tips = new Set<string>();
  const syl = [...word].map((c) => {
    const code = c.charCodeAt(0) - 0xac00;
    return code >= 0 && code < 11172 ? { l: Math.floor(code / 588), v: Math.floor((code % 588) / 28), t: code % 28 } : null;
  });
  let out = '';
  syl.forEach((s, i) => {
    if (!s) {
      out += [...word][i];
      return;
    }
    const nxt = syl[i + 1];
    const linked = s.t && nxt && nxt.l === 11;
    out += (i > 0 && syl[i - 1] && syl[i - 1]!.t && s.l === 11 ? '' : L_RR[s.l]) + V_RR[s.v] + (linked ? T_LINK[s.t] : T_RR[s.t]);
  });
  if (/eo/.test(out)) tips.add(TIP.eo);
  if (/eu/.test(out)) tips.add(TIP.eu);
  if (reader === 'es') {
    out = out.replace(/eo/g, 'o').replace(/eu/g, 'u').replace(/ae/g, 'e').replace(/h/g, 'j').replace(/(?<!c)j(?=[aeiouwy])/g, 'y');
  } else if (reader === 'pt') {
    out = out.replace(/eo/g, 'ó').replace(/eu/g, 'u').replace(/ae/g, 'é');
  } else if (reader === 'fr') {
    out = out.replace(/eo/g, 'o').replace(/eu/g, 'eu').replace(/u(?!i)/g, 'ou').replace(/j/g, 'dj');
  } else if (reader === 'de') {
    out = out.replace(/eo/g, 'o').replace(/ch/g, 'tsch').replace(/j/g, 'dsch').replace(/y/g, 'j');
  } else if (reader === 'it') {
    out = out.replace(/eo/g, 'o').replace(/ch/g, 'c').replace(/j/g, 'gi').replace(/y/g, 'i');
  }
  return { say: out, tips: [...tips] };
}

// ---------- Greek ----------

function greek(word: string, reader: Reader): { say: string; tips: string[] } {
  const tips = new Set<string>();
  let w = word.toLowerCase().normalize('NFD');
  const stressed = /́/.test(w);
  w = w.replace(/ου/g, 'u').replace(/αι/g, 'e').replace(/ει|οι|υι/g, 'i').replace(/μπ/g, 'b').replace(/ντ/g, 'd').replace(/γκ|γγ/g, 'g').replace(/αυ/g, 'av').replace(/ευ/g, 'ev');
  const map: Record<string, string> = { α: 'a', β: 'v', γ: 'g', δ: 'd', ε: 'e', ζ: 'z', η: 'i', θ: 'th', ι: 'i', κ: 'k', λ: 'l', μ: 'm', ν: 'n', ξ: 'x', ο: 'o', π: 'p', ρ: 'r', σ: 's', ς: 's', τ: 't', υ: 'i', φ: 'f', χ: 'kh', ψ: 'ps', ω: 'o' };
  let out = '';
  for (const c of w) {
    if (c === '́') {
      out += reader === 'es' || reader === 'pt' || reader === 'it' || reader === 'fr' ? '́' : '';
      continue;
    }
    if (c === 'θ') tips.add(TIP.thGreek);
    if (c === 'δ') tips.add(TIP.dGreek);
    out += map[c] ?? c;
  }
  if (reader === 'es') out = out.replace(/kh/g, 'j').replace(/th/g, 'z');
  if (reader === 'de') out = out.replace(/kh/g, 'ch').replace(/v/g, 'w').replace(/z/g, 's');
  if (reader === 'pt') out = out.replace(/kh/g, 'rr');
  out = out.normalize('NFC');
  if (stressed && reader !== 'es' && reader !== 'pt' && reader !== 'it' && reader !== 'fr') tips.add('The written accent marks the stressed syllable');
  return { say: out, tips: [...tips] };
}

// ---------- syllables ----------

const V_LAT = 'aeiouáéíóúüàèìòùâêîôûãõäëïöÿy';

/** Spanish syllables with the stressed one in capitals (Spanish spelling shows stress). */
export function spanishSyllables(word: string): string {
  const w = word.toLowerCase().replace(/[^a-záéíóúüñ]/g, '');
  if (!w) return word;
  const strong = 'aeoáéíóú';
  const isV = (c: string) => 'aeiouáéíóúü'.includes(c);
  // Split before a single consonant between vowels, and between two consonants (keeping "pr", "bl"… and "ch", "ll", "rr" together).
  const parts: string[] = [];
  let cur = '';
  for (let i = 0; i < w.length; i++) {
    const c = w[i];
    cur += c;
    const n1 = w[i + 1];
    const n2 = w[i + 2];
    if (isV(c) && n1) {
      if (isV(n1)) {
        // Two strong vowels (or an accented weak one) are separate syllables.
        if ((strong.includes(c) && strong.includes(n1)) || 'íú'.includes(c) || 'íú'.includes(n1)) {
          parts.push(cur);
          cur = '';
        }
      } else if (n2 && isV(n2)) {
        parts.push(cur);
        cur = '';
      } else if (n2 && !isV(n2) && w[i + 3]) {
        const pair = n1 + n2;
        if (/^(pr|br|tr|dr|cr|gr|fr|pl|bl|cl|gl|fl|ch|ll|rr)$/.test(pair)) {
          parts.push(cur);
          cur = '';
        } else if (isV(w[i + 3] ?? '')) {
          parts.push(cur + n1);
          cur = '';
          i++;
        }
      }
    }
  }
  if (cur) parts.push(cur);
  // Stress: the written accent, else the last-but-one syllable for words ending in a vowel, n or s, else the last.
  let at = parts.findIndex((p) => /[áéíóú]/.test(p));
  if (at < 0) at = /[aeiouns]$/.test(w) ? Math.max(0, parts.length - 2) : parts.length - 1;
  return parts.map((p, i) => (i === at && parts.length > 1 ? p.toUpperCase() : p)).join('·');
}

/** Rough syllables by vowel groups (any Latin-script language). */
export function roughSyllables(word: string): string {
  const w = word.replace(/[^\p{L}']/gu, '');
  if (w.length < 4) return w;
  const out: string[] = [];
  let cur = '';
  for (let i = 0; i < w.length; i++) {
    cur += w[i];
    const isV = V_LAT.includes(w[i].toLowerCase());
    const n1 = w[i + 1]?.toLowerCase();
    const n2 = w[i + 2]?.toLowerCase();
    if (isV && n1 && !V_LAT.includes(n1) && n2 && V_LAT.includes(n2)) {
      out.push(cur);
      cur = '';
    }
  }
  if (cur) out.push(cur);
  return out.join('·');
}

/** Spanish re-spelled for English or Portuguese readers. */
function spanishFor(word: string, reader: Reader): { say: string; tips: string[] } {
  const tips = new Set<string>();
  let w = word.toLowerCase();
  if (reader === 'en') {
    if (/rr|^r/.test(w)) tips.add(TIP.rr);
    w = w
      .replace(/^h/, '')
      .replace(/([^c])h/g, '$1')
      .replace(/qu([ei])/g, 'k$1')
      .replace(/gu([ei])/g, 'g$1')
      .replace(/g([ei])/g, 'h$1')
      .replace(/j/g, 'h')
      .replace(/ll/g, 'y')
      .replace(/ñ/g, 'ny')
      .replace(/c([eiéí])/g, 's$1')
      .replace(/z/g, 's')
      .replace(/v/g, 'b');
    tips.add('Spanish vowels are always short and pure: a as in “father”, e as in “bet”, i as in “machine”, o as in “more”, u as in “rule”');
  } else if (reader === 'pt') {
    w = w.replace(/j/g, 'rr').replace(/g([ei])/g, 'rr$1').replace(/ll/g, 'i').replace(/ñ/g, 'nh').replace(/^h/, '');
  } else return { say: '', tips: [] };
  return { say: w, tips: [...tips] };
}

// ---------- the whole guide ----------

/** How to say one word of a song in `lang`, for someone whose first language reads as `reader`. */
export function guideWord(word: string, song: SongLanguage, reader: Reader): WordGuide {
  const clean = word.trim();
  if (!clean) return { say: '', syllables: '', tips: [] };
  switch (song.script) {
    case 'cyrillic': {
      const r = cyrillic(clean.replace(/[^\p{L}'-]/gu, ''), reader, song.code === 'uk');
      return { say: r.say, syllables: '', tips: r.tips };
    }
    case 'kana': {
      const r = kana(clean, reader);
      return { say: r.say, syllables: '', tips: r.tips };
    }
    case 'hangul': {
      const r = hangul(clean, reader);
      return { say: r.say, syllables: '', tips: r.tips };
    }
    case 'greek': {
      const r = greek(clean, reader);
      return { say: r.say, syllables: '', tips: r.tips };
    }
    case 'han':
      return { say: '', syllables: '', tips: [TIP.hanzi] };
    case 'latin': {
      if (song.code === 'es') {
        const r = spanishFor(clean, reader);
        return { say: r.say, syllables: spanishSyllables(clean), tips: r.tips };
      }
      return { say: '', syllables: roughSyllables(clean), tips: [] };
    }
    default:
      return { say: '', syllables: '', tips: [] };
  }
}

/** What the guide can and can't do for this song, in a sentence. */
export function guideNote(song: SongLanguage, reader: Reader, readerName: string): string {
  const spell = reader === 'neutral' ? 'a standard romanization' : `${readerName} spelling`;
  switch (song.script) {
    case 'cyrillic':
      return `${song.name} re-spelled in ${spell}, sound by sound. ${song.code === 'ru' ? 'Stress isn’t written in Russian, so it isn’t marked.' : ''}`.trim();
    case 'kana':
      return `Japanese kana re-spelled in ${spell}. Kanji need a dictionary OmniHub doesn’t have yet, so they stay as they are.`;
    case 'hangul':
      return `Korean re-spelled in ${spell} (Revised Romanization, adapted).`;
    case 'greek':
      return `Greek re-spelled in ${spell}; the written accent shows the stressed syllable.`;
    case 'han':
      return 'Chinese characters need a dictionary OmniHub doesn’t have yet; the lyrics are shown as they are.';
    case 'latin':
      return song.code === 'es'
        ? `Spanish syllables with the stressed one in capitals${reader === 'en' || reader === 'pt' ? `, and a re-spelling for ${readerName} readers` : ''}.`
        : `${song.name} needs a pronunciation dictionary OmniHub doesn’t have yet, so only rough syllables are shown.`;
    default:
      return 'OmniHub can’t re-spell this writing system yet; the lyrics are shown as they are.';
  }
}
