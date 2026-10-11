// Word timings for lyrics that only time lines (most lyrics online do).
// Each word gets a share of the line by its syllables, spread over the time
// the line has until the next one (singers fill it), unless that is so long
// a pause must follow (then at most a slow singing pace), so the highlight
// moves word by word with the voice. It is an estimate, close but not
// exact, and only used when the setting allows; real word timings always win.

import type { LyricLine } from '@shared/types';

/** Vowel groups, roughly a word's syllables (at least one). */
export function syllables(word: string): number {
  const w = word.toLowerCase().replace(/[^a-zà-ÿ']/g, '');
  if (!w) return 1;
  const groups = w.match(/[aeiouyà-ÿ]+/g)?.length ?? 1;
  // A silent final "e" ("rock", "wave", "fire").
  const silentE = w.length > 2 && w.endsWith('e') && !/[aeiouy]e$/.test(w) && !w.endsWith('le') ? 1 : 0;
  return Math.max(1, groups - silentE);
}

/** The slowest a syllable is sung, ms: a line with more room than that is followed by a pause, not sung slower. */
const MAX_PER_SYLLABLE = 480;

/** Lines with word timings: the file's own, or estimated (`estimated: true`). */
export function estimateWords(lines: LyricLine[]): LyricLine[] {
  return lines.map((l, i) => {
    if (l.words?.length || !l.text.trim()) return l;
    const parts = l.text.split(/(\s+)/).reduce<string[]>((out, p) => {
      if (/^\s+$/.test(p) && out.length) out[out.length - 1] += p;
      else if (p) out.push(p);
      return out;
    }, []);
    if (parts.length < 2) return l;
    const next = lines.slice(i + 1).find((n) => n.ms > l.ms)?.ms ?? l.ms + 6000;
    const room = Math.max(400, next - l.ms);
    const weights = parts.map((p) => syllables(p) + 0.35);
    const total = weights.reduce((a, b) => a + b, 0);
    // Over the line's time (a breath before the next one), at most a slow pace.
    const span = Math.min(room * 0.88, total * MAX_PER_SYLLABLE);
    let at = 0;
    const words = parts.map((text, k) => {
      const w = { ms: Math.round(l.ms + (span * at) / total), text };
      at += weights[k];
      return w;
    });
    return { ...l, words, estimated: true };
  });
}
