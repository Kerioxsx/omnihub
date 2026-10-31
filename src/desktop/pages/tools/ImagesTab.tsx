// Image tools: take out location and camera details, resize, compress,
// HEIC → JPG. Each result is a new file next to the original.

import { formatBytes } from '@shared/format';
import type { ImageInfo, ImageJob, ImageResult } from '@shared/types';
import { FolderOpen, Image as ImageIcon, MapPin, X } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Field, Segmented, Select } from '../../components/ui/Form';
import { Callout, EmptyState } from '../../components/ui/States';
import { toast } from '../../state/toasts';

type Kind = ImageJob['kind'];
const KINDS: { value: Kind; label: string }[] = [
  { value: 'clean', label: 'Remove details' },
  { value: 'resize', label: 'Resize' },
  { value: 'compress', label: 'Compress' },
  { value: 'toJpg', label: 'To JPG' },
];
const HELP: Record<Kind, string> = {
  clean: 'Takes out where the photo was taken, the camera and the date, so it’s safe to share. The picture itself isn’t touched.',
  resize: 'Makes the longest side at most this many pixels.',
  compress: 'A smaller JPG. Pictures with see-through parts stay PNG.',
  toJpg: 'iPhone HEIC photos, PNG and WebP to JPG (HEIC needs Microsoft’s free HEIF extension).',
};

const name = (p: string) => p.split(/[\\/]/).pop() ?? p;

export function ImagesTab() {
  const [files, setFiles] = useState<ImageInfo[]>([]);
  const [kind, setKind] = useState<Kind>('clean');
  const [maxSide, setMaxSide] = useState(1600);
  const [quality, setQuality] = useState(75);
  const [busy, setBusy] = useState(false);
  const [results, setResults] = useState<ImageResult[] | null>(null);

  const choose = async () => {
    const picked = (await api.app.pickFiles('Choose pictures')).filter((p) => /\.(jpe?g|png|webp|heic|heif|avif|bmp|gif|tiff?)$/i.test(p));
    if (!picked.length) return;
    try {
      const info = await api.images.inspect(picked);
      setFiles((f) => [...f.filter((x) => !picked.includes(x.path)), ...info]);
      setResults(null);
    } catch (e) {
      toast.error('Couldn’t read the pictures', errorText(e));
    }
  };

  const run = async () => {
    const job: ImageJob = kind === 'resize' ? { kind, maxSide } : kind === 'compress' ? { kind, quality } : { kind };
    setBusy(true);
    try {
      const r = await api.images.run(files.map((f) => f.path), job);
      setResults(r);
      const ok = r.filter((x) => x.done).length;
      if (ok) toast.success(`${ok} ${ok === 1 ? 'picture' : 'pictures'} done`, 'Saved next to the originals.');
      if (ok < r.length) toast.error(`${r.length - ok} couldn’t be done`, r.find((x) => x.error)?.error ?? undefined);
    } catch (e) {
      toast.error('Couldn’t do it', errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const located = files.filter((f) => f.hasLocation).length;
  return (
    <Card className="p-5">
      <CardHeader icon={ImageIcon} title="Images" subtitle="Remove location and camera details, resize, compress, or turn HEIC into JPG. Originals stay as they are." actions={<Button icon={FolderOpen} onClick={() => void choose()}>Choose pictures…</Button>} />
      {!files.length ? (
        <EmptyState compact icon={ImageIcon} title="No pictures chosen" description="Pick photos to clean up before sharing, or a folder’s worth to shrink." className="mt-4" />
      ) : (
        <>
          {located > 0 && (
            <Callout tone="warn" icon={MapPin} className="mt-4">
              {located === 1 ? 'One photo says' : `${located} photos say`} where {located === 1 ? 'it was' : 'they were'} taken. “Remove details” takes that out.
            </Callout>
          )}
          <div className="mt-4 max-h-64 overflow-auto rounded-xl border border-line">
            {files.map((f) => {
              const r = results?.find((x) => x.source === f.path);
              return (
                <div key={f.path} className="flex items-center gap-3 border-b border-line px-3 py-2 text-[13px] last:border-0">
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-fg">{name(f.path)}</div>
                    <div className="truncate text-[12px] text-faint">
                      {formatBytes(f.bytes)}
                      {f.width > 0 && ` · ${f.width}×${f.height}`}
                      {f.camera && ` · ${f.camera}`}
                      {f.taken && ` · ${f.taken}`}
                    </div>
                    {r?.done && (
                      <div className="truncate text-[12px] text-good">
                        → {name(r.done.output)} ({formatBytes(r.done.after)}){r.done.note && ` · ${r.done.note}`}
                      </div>
                    )}
                    {r?.error && <div className="truncate text-[12px] text-bad">{r.error}</div>}
                  </div>
                  {f.hasLocation && (
                    <Badge tone="warn" icon={MapPin}>
                      Location
                    </Badge>
                  )}
                  {r?.done && <IconButton size="sm" icon={FolderOpen} label="Show in folder" onClick={() => void api.app.revealPath(r.done!.output)} />}
                  <IconButton size="sm" icon={X} label="Remove from the list" onClick={() => setFiles((x) => x.filter((y) => y.path !== f.path))} />
                </div>
              );
            })}
          </div>
          <div className="mt-4 flex flex-wrap items-end gap-3">
            <Segmented label="What to do" value={kind} onChange={setKind} options={KINDS} />
            {kind === 'resize' && (
              <Field label="Longest side">
                <Select value={maxSide} onChange={(e) => setMaxSide(Number(e.target.value))}>
                  {[1080, 1600, 2048, 2560, 3840].map((v) => (
                    <option key={v} value={v}>
                      {v} px
                    </option>
                  ))}
                </Select>
              </Field>
            )}
            {kind === 'compress' && (
              <Field label={`Quality ${quality}`}>
                <input type="range" min={30} max={95} step={5} value={quality} onChange={(e) => setQuality(Number(e.target.value))} className="w-48 accent-[var(--accent)]" aria-label="JPG quality" />
              </Field>
            )}
            <Button variant="primary" loading={busy} onClick={() => void run()}>
              {KINDS.find((k) => k.value === kind)?.label} · {files.length} {files.length === 1 ? 'picture' : 'pictures'}
            </Button>
          </div>
          <p className="mt-2 text-[12.5px] text-dim">{HELP[kind]}</p>
        </>
      )}
    </Card>
  );
}
