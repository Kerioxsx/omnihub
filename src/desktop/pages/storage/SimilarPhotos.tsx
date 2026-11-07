// Similar photos: look-alike pictures in a folder (resized copies, re-saved
// JPEGs, burst shots, the same screenshot twice). The biggest of each group
// is kept by default; the rest can go to the Recycle Bin.

import { formatBytes } from '@shared/format';
import type { SimilarGroup } from '@shared/types';
import { Check, FolderOpen, Images, Trash2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card } from '../../components/ui/Card';
import { ProgressBar } from '../../components/ui/Progress';
import { cx } from '../../lib/cx';
import { useEvent } from '../../lib/hooks';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';

function Thumb({ path }: { path: string }) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    let alive = true;
    void api.storage.thumb(path, 200).then((u) => alive && setSrc(u), () => undefined);
    return () => {
      alive = false;
    };
  }, [path]);
  return src ? <img src={src} alt="" className="h-full w-full object-cover" /> : <div className="h-full w-full animate-pulse bg-surface-3" />;
}

export function SimilarPhotos() {
  const [dir, setDir] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [groups, setGroups] = useState<SimilarGroup[] | null>(null);
  const [remove, setRemove] = useState<Set<string>>(new Set());
  useEvent<{ done: number; total: number }>('photos:progress', setProgress);

  const scan = async (d: string) => {
    setDir(d);
    setBusy(true);
    setGroups(null);
    setProgress(null);
    try {
      const g = await api.similarPhotos.find(d);
      setGroups(g);
      setRemove(new Set(g.flatMap((x) => x.photos.filter((_, i) => i !== x.keep).map((p) => p.path))));
    } catch (e) {
      toast.error('Couldn’t look through that folder', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const pick = async () => {
    const d = await api.app.pickFolder('A folder of photos or screenshots');
    if (d) void scan(d);
  };
  const toggle = (p: string) => {
    const next = new Set(remove);
    if (next.has(p)) next.delete(p);
    else next.add(p);
    setRemove(next);
  };
  const size = groups?.flatMap((g) => g.photos).filter((p) => remove.has(p.path)).reduce((a, p) => a + p.bytes, 0) ?? 0;
  const recycle = async () => {
    const paths = [...remove];
    if (!(await confirm({ title: `Send ${paths.length} pictures to the Recycle Bin?`, description: `Frees ${formatBytes(size)}. You can restore them from the Recycle Bin.`, confirmLabel: 'Recycle', tone: 'danger' }))) return;
    try {
      const n = await api.similarPhotos.recycle(paths);
      toast.success(`${n} pictures sent to the Recycle Bin`);
      setGroups((gs) => (gs ?? []).map((g) => ({ ...g, photos: g.photos.filter((p) => !remove.has(p.path)) })).filter((g) => g.photos.length > 1).map((g) => ({ ...g, keep: 0 })));
      setRemove(new Set());
    } catch (e) {
      toast.error('Couldn’t recycle them', errorText(e));
    }
  };

  return (
    <Card className="p-5">
      <div className="flex flex-wrap items-center gap-4">
        <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-accent-soft text-accent">
          <Images size={19} aria-hidden />
        </div>
        <div className="min-w-0 flex-1">
          <div className="text-[14.5px] font-semibold text-fg">Similar photos</div>
          <div className="text-[12.5px] text-dim">{dir ? dir : 'Look-alike pictures, even when they aren’t the same file: resized copies, burst shots, the same screenshot twice.'}</div>
        </div>
        {busy ? (
          <Button variant="ghost" onClick={() => void api.similarPhotos.stop()}>
            Stop
          </Button>
        ) : (
          <Button icon={FolderOpen} onClick={() => void pick()}>
            {dir ? 'Another folder' : 'Pick a folder'}
          </Button>
        )}
      </div>
      {busy && (
        <div className="mt-4">
          <ProgressBar value={progress && progress.total ? progress.done / progress.total : null} />
          <div className="mt-1.5 text-[12px] text-faint">{progress ? `Looked at ${progress.done.toLocaleString()} of ${progress.total.toLocaleString()} pictures` : 'Finding pictures…'}</div>
        </div>
      )}
      {groups && groups.length === 0 && <p className="mt-4 text-[13px] text-dim">No look-alikes in this folder.</p>}
      {groups && groups.length > 0 && (
        <>
          <div className="mt-4 flex items-center justify-between gap-3">
            <span className="text-[12.5px] text-dim">
              {groups.length} groups · the one with a tick is kept · click a picture to change
            </span>
            <Button variant="danger" icon={Trash2} disabled={remove.size === 0} onClick={() => void recycle()}>
              {`Recycle ${remove.size} (${formatBytes(size)})`}
            </Button>
          </div>
          <div className="mt-3 max-h-[520px] space-y-3 overflow-y-auto pr-1">
            {groups.slice(0, 200).map((g, gi) => (
              <div key={gi} className="flex gap-2 overflow-x-auto rounded-xl border border-line p-2">
                {g.photos.map((p) => {
                  const kept = !remove.has(p.path);
                  return (
                    <button key={p.path} type="button" onClick={() => toggle(p.path)} title={p.path} className={cx('relative h-[120px] w-[160px] shrink-0 overflow-hidden rounded-lg border-2 transition-opacity', kept ? 'border-good' : 'border-transparent opacity-60')}>
                      <Thumb path={p.path} />
                      <span className="absolute inset-x-0 bottom-0 bg-black/60 px-1.5 py-0.5 text-left text-[10.5px] text-white">
                        {p.width}×{p.height} · {formatBytes(p.bytes)}
                      </span>
                      {kept && (
                        <span className="absolute right-1 top-1 grid h-5 w-5 place-items-center rounded-full bg-good text-white">
                          <Check size={12} />
                        </span>
                      )}
                    </button>
                  );
                })}
              </div>
            ))}
          </div>
          {groups.length > 200 && <Badge tone="neutral" className="mt-2">Showing the 200 biggest groups</Badge>}
        </>
      )}
    </Card>
  );
}
