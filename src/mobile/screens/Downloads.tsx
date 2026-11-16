// Download on the PC: paste a link (or share one to OmniHub) and the PC
// downloads it into Downloads\OmniHub. Video pages go through yt-dlp.

import type { DownloadItem, DownloadMode } from '@shared/types';
import { CloudDownload, ClipboardPaste, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { client } from '../client';
import { useEvent } from '../lib/events';
import { errorMessage } from '../lib/util';
import { toast, useApp } from '../state';
import { Button, Empty, IconButton, ProgressBar, Segmented } from '../ui/common';
import { SubHeader } from './More';

const size = (b: number) => (b > 1e9 ? `${(b / 1e9).toFixed(1)} GB` : b > 1e6 ? `${(b / 1e6).toFixed(1)} MB` : `${Math.round(b / 1e3)} KB`);

export function DownloadsPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const f = useApp((s) => s.info?.features);
  const [url, setUrl] = useState('');
  const [mode, setMode] = useState<DownloadMode>('auto');
  const [list, setList] = useState<DownloadItem[]>([]);
  const [busy, setBusy] = useState(false);
  const load = useCallback(() => void client.downloads().then(setList, () => undefined), []);
  useEffect(() => {
    if (active) load();
  }, [active, load]);
  useEvent<DownloadItem>('download:changed', (d) => {
    if (!d?.id) return load();
    setList((l) => (l.some((x) => x.id === d.id) ? l.map((x) => (x.id === d.id ? d : x)) : [d, ...l]));
  });

  const start = async () => {
    setBusy(true);
    try {
      await client.downloadOnPc(url, mode);
      setUrl('');
      toast.success('Downloading on the PC');
    } catch (e) {
      toast.error('Couldn’t download it', errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  const paste = async () => {
    try {
      setUrl(await navigator.clipboard.readText());
    } catch {
      toast.error('Couldn’t paste', 'Long-press the box and paste instead.');
    }
  };

  return (
    <div className="scroller h-full">
      <SubHeader title="Download on the PC" subtitle="Into Downloads\OmniHub" onBack={onBack} />
      <div className="px-safe space-y-4 pb-tabbar">
        {!f?.uploads ? (
          <Empty icon={<CloudDownload size={28} />} title="Sending files to the PC is off" body="Turn it on in OmniHub on the PC (Phone → Permissions)." />
        ) : (
          <div className="card space-y-3 p-3">
            <div className="flex gap-2">
              <input value={url} onChange={(e) => setUrl(e.target.value)} inputMode="url" placeholder="https://…" className="h-12 min-w-0 flex-1 rounded-2xl bg-surface-2 px-4 text-[15px] outline-none" />
              <IconButton label="Paste" onClick={() => void paste()}>
                <ClipboardPaste size={20} />
              </IconButton>
            </div>
            <Segmented value={mode} onChange={setMode} options={[{ value: 'auto', label: 'Video or file' }, { value: 'audio', label: 'Sound only' }]} />
            <Button className="w-full" size="lg" icon={<CloudDownload size={18} />} loading={busy} disabled={!url.trim()} onClick={() => void start()}>
              Download on the PC
            </Button>
            <p className="px-1 text-[12.5px] text-faint">Files download as they are. YouTube, TikTok, X and Instagram pages save the video. Tip: in any app, Share → OmniHub → Download on the PC.</p>
          </div>
        )}
        {list.map((d) => (
          <div key={d.id} className="card space-y-2 p-3">
            <div className="flex items-center gap-2">
              <div className="min-w-0 flex-1">
                <div className="truncate text-[15px] font-semibold">{d.name ?? d.url}</div>
                <div className="truncate text-[12.5px] text-dim">
                  {d.state === 'done' ? `Done · ${size(d.bytes)}` : d.state === 'running' ? (d.percent != null ? `${Math.round(d.percent)}%` : d.bytes ? size(d.bytes) : 'Starting…') : d.state === 'failed' ? 'Failed' : 'Cancelled'}
                  {' · '}
                  {d.from}
                </div>
                {d.error && <div className="text-[12.5px] text-bad">{d.error}</div>}
              </div>
              {d.state === 'running' && (
                <IconButton label="Cancel" onClick={() => void client.cancelDownload(d.id)}>
                  <X size={18} />
                </IconButton>
              )}
            </div>
            {d.state === 'running' && <ProgressBar value={(d.percent ?? 0) / 100} active={d.percent == null} />}
          </div>
        ))}
      </div>
    </div>
  );
}
