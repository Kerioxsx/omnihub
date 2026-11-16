// Video tools: fit a size (Discord's 10 MB), compress, convert, cut, or take
// the sound out. FFmpeg (the one instant replay uses) does it on this PC.

import { formatBytes } from '@shared/format';
import type { VideoJob, VideoMedia, VideoSize, VideoTask } from '@shared/types';
import { Download, Film, FolderOpen, Play, Trash2, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Field, Segmented, Select, TextInput } from '../../components/ui/Form';
import { ProgressBar } from '../../components/ui/Progress';
import { Callout, EmptyState } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { toast } from '../../state/toasts';

type Kind = VideoJob['kind'] | 'fit';
const KINDS: { value: Kind; label: string }[] = [
  { value: 'fit', label: 'Fit a size' },
  { value: 'compress', label: 'Compress' },
  { value: 'convert', label: 'Convert' },
  { value: 'trim', label: 'Cut' },
  { value: 'audio', label: 'Sound only' },
];

const name = (p: string) => p.split(/[\\/]/).pop() ?? p;
const clock = (s: number) => `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}`;
/** "1:05", "65" or "1:05.5" → seconds. */
const seconds = (t: string) => t.split(':').reduce((a, p) => a * 60 + (Number(p) || 0), 0);

export function VideoTab() {
  const [status, setStatus] = useState<{ ready: boolean; installing: boolean } | null>(null);
  const [install, setInstall] = useState<{ done: number; total: number } | null>(null);
  const [file, setFile] = useState<string | null>(null);
  const [media, setMedia] = useState<VideoMedia | null>(null);
  const [kind, setKind] = useState<Kind>('fit');
  const [mb, setMb] = useState(10);
  const [size, setSize] = useState<VideoSize>('balanced');
  const [height, setHeight] = useState<number | null>(null);
  const [to, setTo] = useState<'mp4' | 'webm' | 'gif'>('mp4');
  const [audio, setAudio] = useState<'mp3' | 'm4a' | 'wav'>('mp3');
  const [start, setStart] = useState('0:00');
  const [end, setEnd] = useState('');
  const [exact, setExact] = useState(false);
  const [tasks, setTasks] = useState<VideoTask[]>([]);

  const load = useCallback(() => {
    void api.video.status().then(setStatus, () => setStatus({ ready: false, installing: false }));
    void api.video.tasks().then(setTasks, () => undefined);
  }, []);
  useEffect(load, [load]);
  useEvent<VideoTask>('video:changed', (t) => {
    if (!t?.id) return void api.video.tasks().then(setTasks);
    setTasks((l) => (l.some((x) => x.id === t.id) ? l.map((x) => (x.id === t.id ? t : x)) : [t, ...l]));
  });
  useEvent<{ done: number; total: number }>('play:install', setInstall);

  const getFfmpeg = async () => {
    try {
      await api.play.install();
      toast.success('FFmpeg is ready');
    } catch (e) {
      toast.error('Couldn’t download FFmpeg', errorText(e));
    } finally {
      setInstall(null);
      load();
    }
  };

  const choose = async () => {
    const [p] = await api.app.pickFiles('Choose a video');
    if (!p) return;
    setFile(p);
    setMedia(null);
    try {
      const m = await api.video.probe(p);
      setMedia(m);
      setEnd(clock(m.duration));
    } catch (e) {
      toast.error('Couldn’t read the video', errorText(e));
    }
  };

  const go = async () => {
    if (!file) return;
    const job: VideoJob =
      kind === 'fit'
        ? { kind: 'compress', size: 'balanced', maxHeight: null, targetMb: mb }
        : kind === 'compress'
          ? { kind, size, maxHeight: height, targetMb: null }
          : kind === 'convert'
            ? { kind, to }
            : kind === 'trim'
              ? { kind, start: seconds(start), end: seconds(end), exact }
              : { kind, to: audio };
    try {
      await api.video.start(file, job);
    } catch (e) {
      toast.error('Couldn’t start', errorText(e));
    }
  };

  if (status && !status.ready) {
    return (
      <Card className="p-5">
        <CardHeader icon={Film} title="Video" subtitle="Fit a video in 10 MB for Discord, compress, convert, cut, or keep only the sound." />
        <Callout tone="info" icon={Download} className="mt-4" action={<Button variant="primary" loading={!!install || status.installing} onClick={() => void getFfmpeg()}>Download FFmpeg</Button>}>
          The video tools use FFmpeg, the same free tool instant replay uses. It’s downloaded once from this release and checked.
          {install && install.total > 0 && <ProgressBar value={install.done / install.total} className="mt-2" />}
        </Callout>
      </Card>
    );
  }

  const running = tasks.filter((t) => t.state === 'running').length;
  return (
    <div className="space-y-4">
      <Card className="p-5">
        <CardHeader icon={Film} title="Video" subtitle="Fit a video in 10 MB for Discord, compress, convert, cut, or keep only the sound. A new file goes next to the original." actions={<Button icon={FolderOpen} onClick={() => void choose()}>Choose a video…</Button>} />
        {!file ? (
          <EmptyState compact icon={Film} title="No video chosen" description="Clips from instant replay are in Videos\OmniHub Clips." className="mt-4" />
        ) : (
          <>
            <div className="mt-4 flex items-center gap-3 rounded-xl border border-line bg-surface px-3 py-2.5 text-[13px]">
              <Play size={16} className="text-accent" />
              <div className="min-w-0 flex-1">
                <div className="truncate font-medium text-fg">{name(file)}</div>
                <div className="text-[12px] text-faint">{media ? `${clock(media.duration)} · ${media.width}×${media.height} · ${Math.round(media.fps)} fps · ${media.videoCodec ?? 'no picture'} / ${media.audioCodec ?? 'no sound'}` : 'Reading…'}</div>
              </div>
            </div>
            <div className="mt-4 flex flex-wrap items-end gap-3">
              <Segmented label="What to do" value={kind} onChange={setKind} options={KINDS} />
              {kind === 'fit' && (
                <Field label="Size">
                  <Select value={mb} onChange={(e) => setMb(Number(e.target.value))}>
                    <option value={10}>10 MB (Discord)</option>
                    <option value={25}>25 MB (email)</option>
                    <option value={50}>50 MB (Discord Nitro Basic)</option>
                    <option value={100}>100 MB</option>
                  </Select>
                </Field>
              )}
              {kind === 'compress' && (
                <>
                  <Segmented label="Quality" size="sm" value={size} onChange={setSize} options={[{ value: 'small', label: 'Smallest' }, { value: 'balanced', label: 'Balanced' }, { value: 'high', label: 'Best' }]} />
                  <Field label="Picture">
                    <Select value={height ?? ''} onChange={(e) => setHeight(e.target.value ? Number(e.target.value) : null)}>
                      <option value="">Keep the size</option>
                      {[2160, 1440, 1080, 720, 480].filter((h) => !media || h < media.height).map((h) => (
                        <option key={h} value={h}>
                          {h}p
                        </option>
                      ))}
                    </Select>
                  </Field>
                </>
              )}
              {kind === 'convert' && <Segmented label="To" size="sm" value={to} onChange={setTo} options={[{ value: 'mp4', label: 'MP4' }, { value: 'webm', label: 'WebM' }, { value: 'gif', label: 'GIF' }]} />}
              {kind === 'audio' && <Segmented label="As" size="sm" value={audio} onChange={setAudio} options={[{ value: 'mp3', label: 'MP3' }, { value: 'm4a', label: 'M4A' }, { value: 'wav', label: 'WAV' }]} />}
              {kind === 'trim' && (
                <>
                  <Field label="From">
                    <TextInput value={start} onChange={(e) => setStart(e.target.value)} className="w-24" aria-label="Start time" />
                  </Field>
                  <Field label="To">
                    <TextInput value={end} onChange={(e) => setEnd(e.target.value)} className="w-24" aria-label="End time" />
                  </Field>
                  <label className="flex h-9 items-center gap-2 text-[13px] text-dim">
                    <input type="checkbox" checked={exact} onChange={(e) => setExact(e.target.checked)} className="accent-[var(--accent)]" /> Exact frame (slower)
                  </label>
                </>
              )}
              <Button variant="primary" disabled={!media} onClick={() => void go()}>
                Start
              </Button>
            </div>
            <p className="mt-2 text-[12.5px] text-dim">
              {kind === 'fit' && 'Works out the quality (and a smaller picture if it must) so the file fits.'}
              {kind === 'compress' && 'An MP4 made with your graphics card’s encoder when it has one. Never bigger than the original.'}
              {kind === 'convert' && (to === 'gif' ? 'GIFs are 12 frames a second, 480 wide: best for short clips.' : to === 'mp4' ? 'MKV or MOV from OBS and phones: usually just repackaged, so it’s quick and lossless.' : 'WebM (VP9 + Opus), for the web.')}
              {kind === 'trim' && 'Without “Exact frame” the cut starts at the nearest keyframe and nothing is re-encoded.'}
              {kind === 'audio' && 'Only the soundtrack, as a music file.'}
            </p>
          </>
        )}
      </Card>
      {tasks.length > 0 && (
        <Card className="p-5">
          <CardHeader title="Jobs" subtitle={running ? `${running} running` : 'All done'} actions={<IconButton icon={Trash2} label="Clear finished" onClick={() => void api.video.clear().then(load)} />} />
          <div className="mt-3 space-y-2">
            {tasks.map((t) => (
              <div key={t.id} className="rounded-xl border border-line px-3 py-2.5 text-[13px]">
                <div className="flex items-center gap-2">
                  <div className="min-w-0 flex-1">
                    <div className="truncate font-medium text-fg">{name(t.output)}</div>
                    <div className="truncate text-[12px] text-faint">
                      {t.label} · from {name(t.source)}
                      {t.state === 'done' && ` · ${formatBytes(t.before)} → ${formatBytes(t.after)}`}
                    </div>
                    {t.error && <div className="text-[12px] text-bad">{t.error}</div>}
                  </div>
                  {t.state === 'running' && <IconButton size="sm" icon={X} label="Cancel" onClick={() => void api.video.cancel(t.id)} />}
                  {t.state === 'done' && <IconButton size="sm" icon={FolderOpen} label="Show in folder" onClick={() => void api.app.revealPath(t.output)} />}
                  {t.state === 'done' && <IconButton size="sm" icon={Play} label="Play" onClick={() => void api.app.openPath(t.output)} />}
                  {t.state !== 'running' && t.state !== 'done' && <Badge tone={t.state === 'failed' ? 'bad' : 'neutral'}>{t.state === 'failed' ? 'Failed' : 'Cancelled'}</Badge>}
                </div>
                {t.state === 'running' && <ProgressBar value={t.percent / 100} className="mt-2" />}
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}
