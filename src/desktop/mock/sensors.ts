// Demo data for image and video tools, downloads, and the phone's whiteboard.

import type { BoardStroke, DownloadItem, DownloadMode, ImageInfo, ImageJob, ImageResult, VideoJob, VideoMedia, VideoTask } from '@shared/types';
import { emit } from './bus';

const now = () => Math.floor(Date.now() / 1000);
const id = () => Math.random().toString(36).slice(2, 10);
const base = (p: string) => p.split(/[\\/]/).pop() ?? p;
const dir = (p: string) => p.slice(0, p.length - base(p).length);
const stem = (p: string) => base(p).replace(/\.[^.]+$/, '');

export function imageInspect(paths: string[]): ImageInfo[] {
  return paths.map((path, i) => ({ path, bytes: 3_400_000 + i * 410_000, width: 4032, height: 3024, hasLocation: i % 2 === 0, taken: '2026-10-12 17:42:03', camera: i % 2 ? 'Apple iPhone 15 Pro' : 'Google Pixel 9' }));
}

export function imageRun(paths: string[], job: ImageJob): ImageResult[] {
  return paths.map((source, i) => {
    const before = 3_400_000 + i * 410_000;
    const [suffix, ext, after] =
      job.kind === 'clean' ? [' (no details)', /\.(png|webp)$/i.test(source) ? source.split('.').pop()! : 'jpg', before - 48_000] : job.kind === 'resize' ? [` (${job.maxSide}px)`, 'jpg', Math.round(before / 5)] : job.kind === 'compress' ? [' (small)', 'jpg', Math.round(before / 3)] : ['', 'jpg', Math.round(before * 0.8)];
    return { source, done: { source, output: `${dir(source)}${stem(source)}${suffix}.${ext}`, before, after, note: null }, error: null };
  });
}

export const videoMedia = (): VideoMedia => ({ duration: 94.2, width: 1920, height: 1080, fps: 60, videoCodec: 'h264', audioCodec: 'aac', bitrate: 12_400 });

let tasks: VideoTask[] = [
  { id: 'v0', source: 'C:\\Users\\Player\\Videos\\OmniHub Clips\\Fortnite 2026-10-30 21-14.mp4', output: 'C:\\Users\\Player\\Videos\\OmniHub Clips\\Fortnite 2026-10-30 21-14 (10 MB).mp4', label: 'Fit in 10 MB', percent: 100, state: 'done', error: null, before: 61_200_000, after: 9_640_000, startedAt: now() - 3600 },
];
export const videoTasks = () => [...tasks].reverse();
export function videoStart(source: string, job: VideoJob): string {
  const t: VideoTask = {
    id: id(),
    source,
    output: `${dir(source)}${stem(source)}${job.kind === 'compress' ? (job.targetMb ? ` (${job.targetMb} MB)` : ' (small)') : job.kind === 'trim' ? ' (cut)' : ''}.${job.kind === 'audio' ? job.to : job.kind === 'convert' ? job.to : 'mp4'}`,
    label: job.kind === 'compress' ? (job.targetMb ? `Fit in ${job.targetMb} MB` : 'Compress') : job.kind === 'audio' ? `Sound as ${job.to.toUpperCase()}` : job.kind === 'trim' ? 'Cut' : `To ${job.to.toUpperCase()}`,
    percent: 0,
    state: 'running',
    error: null,
    before: 61_200_000,
    after: 0,
    startedAt: now(),
  };
  tasks = [...tasks, t];
  const tick = setInterval(() => {
    t.percent = Math.min(100, t.percent + 9 + Math.random() * 8);
    if (t.percent >= 100) {
      t.state = 'done';
      t.after = Math.round(t.before / 6);
      clearInterval(tick);
    }
    emit('video:changed', { ...t });
  }, 500);
  return t.id;
}
export function videoCancel(vid: string) {
  const t = tasks.find((x) => x.id === vid);
  if (t && t.state === 'running') {
    t.state = 'cancelled';
    emit('video:changed', { ...t });
  }
}
export function videoClear() {
  tasks = tasks.filter((t) => t.state === 'running');
}

let downloads: DownloadItem[] = [
  { id: 'd0', url: 'https://www.youtube.com/watch?v=dQw4w9WgXcQ', name: 'Never Gonna Give You Up [dQw4w9WgXcQ].mp4', path: 'C:\\Users\\Player\\Downloads\\OmniHub\\Never Gonna Give You Up [dQw4w9WgXcQ].mp4', state: 'done', percent: 100, bytes: 31_800_000, error: null, from: 'Pixel 9', startedAt: now() - 900 },
];
export const downloadList = () => [...downloads].reverse();
export function downloadStart(url: string, mode: DownloadMode): string {
  const link = url.split(/\s+/).find((w) => /^https?:\/\//.test(w));
  if (!link) throw 'That isn’t a web link (it must start with https://).';
  const d: DownloadItem = { id: id(), url: link, name: null, path: null, state: 'running', percent: 0, bytes: 0, error: null, from: 'This PC', startedAt: now() };
  downloads = [...downloads, d];
  const name = (base(link.split('?')[0]) || 'download') + (mode === 'audio' ? '.m4a' : '');
  const tick = setInterval(() => {
    d.percent = Math.min(100, (d.percent ?? 0) + 12);
    d.bytes += 2_400_000;
    d.name = name;
    if (d.percent >= 100) {
      d.state = 'done';
      d.path = `C:\\Users\\Player\\Downloads\\OmniHub\\${name}`;
      clearInterval(tick);
    }
    emit('download:changed', { ...d });
  }, 400);
  return d.id;
}
export function downloadCancel(did: string) {
  const d = downloads.find((x) => x.id === did);
  if (d && d.state === 'running') {
    d.state = 'cancelled';
    emit('download:changed', { ...d });
  }
}
export function downloadClear() {
  downloads = downloads.filter((d) => d.state === 'running');
}

/** A little house and sun, as if drawn on a phone. */
let strokes: BoardStroke[] = [
  { points: [[0.3, 0.7], [0.3, 0.45], [0.45, 0.3], [0.6, 0.45], [0.6, 0.7], [0.3, 0.7]], color: '#60a5fa', width: 0.008 },
  { points: [[0.42, 0.7], [0.42, 0.58], [0.48, 0.58], [0.48, 0.7]], color: '#f472b6', width: 0.006 },
  { points: Array.from({ length: 25 }, (_, i) => [0.8 + 0.06 * Math.cos((i / 24) * Math.PI * 2), 0.22 + 0.08 * Math.sin((i / 24) * Math.PI * 2)] as [number, number]), color: '#facc15', width: 0.008 },
];
export const boardStrokes = () => strokes;
export function boardClear() {
  strokes = [];
  emit('board:clear', {});
}
export function boardUndo() {
  strokes = strokes.slice(0, -1);
  emit('board:undo', {});
}
