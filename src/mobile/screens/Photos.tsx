// Photo import: pick photos and videos from the camera roll (select all is
// fine) and only the ones not on the PC yet are sent, into
// Pictures\Phone\<month>. A web page can't read the camera roll by itself,
// so it's a tap whenever you want them copied.

import { Images, LoaderCircle } from 'lucide-react';
import { useRef, useState } from 'react';
import { client, PHOTOS_DIR } from '../client';
import { errorMessage, vibrate } from '../lib/util';
import { toast } from '../state';
import { useUploads } from '../uploads';
import { Button } from '../ui/common';
import { UploadsPanel } from './Uploads';
import { SubHeader } from './More';

export function PhotosPage({ onBack }: { onBack: () => void }) {
  const input = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState(false);
  const [last, setLast] = useState<{ sent: number; had: number; dest: string } | null>(null);
  const add = useUploads((s) => s.add);
  const picked = async (files: File[]) => {
    if (!files.length) return;
    setBusy(true);
    try {
      const r = await client.photosCheck(files.map((f) => ({ name: f.name, size: f.size })));
      const fresh = files.filter((_, i) => !r.have[i]);
      if (fresh.length) add(fresh, PHOTOS_DIR, 'Pictures › Phone');
      setLast({ sent: fresh.length, had: files.length - fresh.length, dest: r.dest });
      vibrate(20);
    } catch (e) {
      toast.error("Couldn't check the photos", errorMessage(e));
    } finally {
      setBusy(false);
      if (input.current) input.current.value = '';
    }
  };
  return (
    <div className="scroller h-full">
      <SubHeader title="Import photos" subtitle="Camera roll → PC" onBack={onBack} />
      <div className="px-safe pb-tabbar space-y-4">
        <div className="card flex flex-col items-center gap-4 p-6 text-center">
          <div className="grid h-16 w-16 place-items-center rounded-3xl bg-pink-500/15 text-pink-400">{busy ? <LoaderCircle size={30} className="spin" /> : <Images size={30} />}</div>
          <div>
            <div className="font-display text-xl font-bold">Copy new photos to the PC</div>
            <p className="mt-1 text-[14px] text-dim">Select as many as you like; photos already on the PC are skipped. They go to Pictures › Phone, in a folder for each month.</p>
          </div>
          <input ref={input} type="file" accept="image/*,video/*" multiple hidden onChange={(e) => void picked([...(e.target.files ?? [])])} />
          <Button icon={<Images size={18} />} disabled={busy} onClick={() => input.current?.click()}>
            Choose photos
          </Button>
          {last && (
            <p className="text-[13px] text-dim">
              {last.sent ? `Sending ${last.sent} new` : 'Nothing new'}
              {last.had ? ` · ${last.had} already on the PC` : ''}
            </p>
          )}
        </div>
        <UploadsPanel />
      </div>
    </div>
  );
}
