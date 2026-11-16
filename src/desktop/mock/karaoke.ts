// Karaoke Studio for the mock backend: language and history in memory, and
// a demo song from "this PC": an original made-up Russian song (words and
// melody written for testing) with a synthesized backing track, so both
// modes can be tried in a browser.

import type { KaraokeData, KaraokeLyrics, KaraokeSession, KaraokeTrack } from '@shared/types';

let data: KaraokeData = { profile: null, sessions: [] };

const DEMO_LRC = `[00:04.00]Тихий вечер над рекой
[00:08.00]Звёзды падают в ладони
[00:12.00]Я пою, и ветер мой
[00:16.00]Тёплым светом нас догонит
[00:21.00]
[00:24.00]Тихий вечер, город спит
[00:28.00]Только сердце не молчит`;

function parse(src: string) {
  return src.split('\n').flatMap((raw) => {
    const m = /^\[(\d+):(\d+)\.(\d+)\](.*)$/.exec(raw.trim());
    return m ? [{ ms: Number(m[1]) * 60000 + Number(m[2]) * 1000 + Number(m[3].padEnd(3, '0')), text: m[4].trim() }] : [];
  });
}

let demoUrl: string | null = null;

/** A 34-second backing track: a soft pad and a simple tune (made up here). */
function demoAudio(): string {
  if (demoUrl) return demoUrl;
  const rate = 22050;
  const secs = 34;
  const n = rate * secs;
  const pcm = new Int16Array(n);
  const chords = [57, 53, 60, 55];
  const tune = [69, 67, 65, 64, 65, 67, 69, 72, 71, 69, 67, 65, 64, 62, 64, 65];
  const hz = (m: number) => 440 * 2 ** ((m - 69) / 12);
  for (let i = 0; i < n; i++) {
    const t = i / rate;
    const bar = Math.floor(t / 2) % 4;
    const root = hz(chords[bar]);
    let v = 0.12 * (Math.sin(2 * Math.PI * root * t) + 0.6 * Math.sin(2 * Math.PI * root * 1.5 * t) + 0.4 * Math.sin(2 * Math.PI * root * 2 * t));
    const beat = t % 0.5;
    v += 0.25 * Math.exp(-beat * 30) * Math.sin(2 * Math.PI * 60 * beat);
    if (t > 4 && t < 20) {
      const k = Math.floor((t - 4) / 1) % tune.length;
      const local = (t - 4) % 1;
      v += 0.14 * Math.min(1, local * 20) * Math.exp(-local * 2) * Math.sin(2 * Math.PI * hz(tune[k]) * t);
    }
    pcm[i] = Math.max(-1, Math.min(1, v)) * 32000;
  }
  const head = new DataView(new ArrayBuffer(44));
  const w = (o: number, s: string) => [...s].forEach((c, k) => head.setUint8(o + k, c.charCodeAt(0)));
  w(0, 'RIFF');
  head.setUint32(4, 36 + n * 2, true);
  w(8, 'WAVE');
  w(12, 'fmt ');
  head.setUint32(16, 16, true);
  head.setUint16(20, 1, true);
  head.setUint16(22, 1, true);
  head.setUint32(24, rate, true);
  head.setUint32(28, rate * 2, true);
  head.setUint16(32, 2, true);
  head.setUint16(34, 16, true);
  w(36, 'data');
  head.setUint32(40, n * 2, true);
  demoUrl = URL.createObjectURL(new Blob([head.buffer, pcm.buffer], { type: 'audio/wav' }));
  return demoUrl;
}

export const karaoke = {
  get: (): KaraokeData => data,
  profile: (lang: string): KaraokeData => (data = { ...data, profile: { lang } }),
  session: (s: KaraokeSession): KaraokeData => (data = { ...data, sessions: [s, ...data.sessions].slice(0, 300) }),
  clear: (): KaraokeData => (data = { ...data, sessions: [] }),
  track: (path: string): KaraokeTrack => ({ path, title: 'Тихий вечер (demo)', artist: 'OmniHub', url: demoAudio() }),
  lyrics: (): KaraokeLyrics => ({ lines: parse(DEMO_LRC), plain: null, instrumental: false, source: 'Your file' }),
};
