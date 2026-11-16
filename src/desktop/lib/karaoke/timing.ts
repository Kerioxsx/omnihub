// Word timings for Karaoke Studio. A copy of Aurora's estimate
// (lib/aurora/timing.ts) that counts syllables in any writing system —
// Cyrillic, Greek, kana, Hangul, Chinese characters — so a Russian or
// Japanese line is spread by its real syllables. Kept separate so Aurora
// doesn't change. Real word timings from the lyrics always win; estimated
// lines carry `estimated: true` and the page says they're approximate.

import type { LyricLine } from '@shared/types';
import { syllables as latinSyllables } from '../aurora/timing';

const MAX_PER_SYLLABLE = 480;

/** Roughly how many syllables (or morae) a word has, in any script (at least one). */
export function syllableCount(word: string): number {
  const w = word.toLowerCase();
  const count = (re: RegExp) => (w.match(re) ?? []).length;
  if (/[а-яёіїєґў]/.test(w)) return Math.max(1, count(/[аеёиоуыэюяіїєў]/g));
  if (/[α-ωάέήίόύώϊϋΐΰ]/.test(w)) return Math.max(1, count(/[αεηιουωάέήίόύώϊϋΐΰ]+/g));
  // Kana: one mora each, small ゃゅょぁ… join the one before.
  const kana = count(/[ぁ-ゖァ-ヺー]/g) - count(/[ゃゅょぁぃぅぇぉャュョァィゥェォ]/g);
  const blocks = count(/[가-힯]/g) + count(/[一-鿿]/g);
  if (kana + blocks > 0) return Math.max(1, kana + blocks);
  return latinSyllables(word);
}

/** Lines with word timings: the file's own, or estimated (`estimated: true`). */
export function estimateWords(lines: LyricLine[]): LyricLine[] {
  return lines.map((l, i) => {
    if (l.words?.length || !l.text.trim()) return l;
    const parts = l.text.split(/(\s+)/).reduce<string[]>((out, p) => {
      if (/^\s+$/.test(p) && out.length) out[out.length - 1] += p;
      else if (p) out.push(p);
      return out;
    }, []);
    if (parts.length < 1) return l;
    const next = lines.slice(i + 1).find((n) => n.ms > l.ms)?.ms ?? l.ms + 6000;
    const room = Math.max(400, next - l.ms);
    const weights = parts.map((p) => syllableCount(p) + 0.35);
    const total = weights.reduce((a, b) => a + b, 0);
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

/** Shift every timing by `ms` (positive = later), for lyrics that run early or late. */
export function shiftLines(lines: LyricLine[], ms: number): LyricLine[] {
  if (!ms) return lines;
  return lines.map((l) => ({ ...l, ms: l.ms + ms, words: l.words?.map((w) => ({ ...w, ms: w.ms + ms })) }));
}
