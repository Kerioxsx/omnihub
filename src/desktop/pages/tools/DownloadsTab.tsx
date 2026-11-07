// Downloads: a link (from a phone, or pasted here) downloaded into
// Downloads\OmniHub. Pages with a video go through yt-dlp.

import { formatBytes } from '@shared/format';
import type { DownloadItem, DownloadMode } from '@shared/types';
import { CloudDownload, ExternalLink, FolderOpen, Link, Trash2, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Segmented, TextInput } from '../../components/ui/Form';
import { ProgressBar } from '../../components/ui/Progress';
import { EmptyState } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { toast } from '../../state/toasts';

export function DownloadsTab() {
  return (
    <div className="space-y-4">
      <AfterDownloads />
      <DownloadsCard />
    </div>
  );
}

/** Shut down or sleep once every download (OmniHub's, Steam's, Epic's, a browser's) is done. */
function AfterDownloads() {
  const [action, setAction] = useState<'' | 'shutdown' | 'sleep'>('');
  useEffect(() => {
    void api.afterDownloads.get().then((a) => setAction(a.action as '' | 'shutdown' | 'sleep'), () => undefined);
  }, []);
  useEvent('downloads:after', () => setAction(''));
  const set = (v: '' | 'shutdown' | 'sleep') => void api.afterDownloads.set(v).then((a) => setAction(a.action as '' | 'shutdown' | 'sleep'), (e: unknown) => toast.error('Couldn’t set it', errorText(e)));
  return (
    <Card className="flex flex-wrap items-center gap-4 p-5">
      <div className="min-w-0 flex-1">
        <div className="text-[14.5px] font-semibold text-fg">When downloads finish</div>
        <div className="text-[12.5px] text-dim">Once nothing has been downloading for 5 minutes (Steam, Epic, browsers and OmniHub), the PC shuts down or sleeps, with a minute to cancel.</div>
      </div>
      <Segmented label="When downloads finish" value={action} onChange={set} options={[{ value: '', label: 'Do nothing' }, { value: 'sleep', label: 'Sleep' }, { value: 'shutdown', label: 'Shut down' }]} />
    </Card>
  );
}

function DownloadsCard() {
  const [url, setUrl] = useState('');
  const [mode, setMode] = useState<DownloadMode>('auto');
  const [list, setList] = useState<DownloadItem[]>([]);
  const [folder, setFolder] = useState('');
  const load = useCallback(() => void api.downloads.list().then(setList, () => undefined), []);
  useEffect(() => {
    load();
    void api.downloads.folder().then(setFolder, () => undefined);
  }, [load]);
  useEvent<DownloadItem>('download:changed', (d) => {
    if (!d?.id) return load();
    setList((l) => (l.some((x) => x.id === d.id) ? l.map((x) => (x.id === d.id ? d : x)) : [d, ...l]));
  });

  const start = async () => {
    try {
      await api.downloads.start(url, mode);
      setUrl('');
    } catch (e) {
      toast.error('Couldn’t download it', errorText(e));
    }
  };

  return (
    <Card className="p-5">
      <CardHeader
        icon={CloudDownload}
        title="Downloads"
        subtitle="Paste a link here, or send one from your phone (More → Download on the PC). Files download as they are; YouTube, TikTok, X and Instagram videos go through yt-dlp."
        actions={folder && <Button size="sm" icon={FolderOpen} onClick={() => void api.app.openPath(folder)}>Open folder</Button>}
      />
      <form
        className="mt-4 flex flex-wrap items-center gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void start();
        }}
      >
        <TextInput icon={Link} value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://…" className="min-w-72 flex-1" aria-label="Link" />
        <Segmented label="What" size="sm" value={mode} onChange={setMode} options={[{ value: 'auto', label: 'Video or file' }, { value: 'audio', label: 'Sound only' }]} />
        <Button type="submit" variant="primary" disabled={!url.trim()}>
          Download
        </Button>
      </form>
      <div className="mt-4 space-y-2">
        {!list.length ? (
          <EmptyState compact icon={CloudDownload} title="Nothing downloaded yet" description={folder ? `Downloads go to ${folder}.` : undefined} />
        ) : (
          list.map((d) => (
            <div key={d.id} className="rounded-xl border border-line px-3 py-2.5 text-[13px]">
              <div className="flex items-center gap-2">
                <div className="min-w-0 flex-1">
                  <div className="truncate font-medium text-fg">{d.name ?? d.url}</div>
                  <div className="truncate text-[12px] text-faint">
                    {d.from} · {d.state === 'running' ? (d.bytes ? formatBytes(d.bytes) : 'Starting…') : formatBytes(d.bytes)}
                    {d.name && ` · ${new URL(d.url).hostname}`}
                  </div>
                  {d.error && <div className="text-[12px] text-bad">{d.error}</div>}
                </div>
                {d.state === 'running' && <IconButton size="sm" icon={X} label="Cancel" onClick={() => void api.downloads.cancel(d.id)} />}
                {d.state === 'done' && d.path && <IconButton size="sm" icon={FolderOpen} label="Show in folder" onClick={() => void api.app.revealPath(d.path!)} />}
                {d.state === 'done' && d.path && <IconButton size="sm" icon={ExternalLink} label="Open" onClick={() => void api.app.openPath(d.path!)} />}
                {(d.state === 'failed' || d.state === 'cancelled') && <Badge tone={d.state === 'failed' ? 'bad' : 'neutral'}>{d.state === 'failed' ? 'Failed' : 'Cancelled'}</Badge>}
              </div>
              {d.state === 'running' && <ProgressBar value={d.percent == null ? null : d.percent / 100} className="mt-2" />}
            </div>
          ))
        )}
      </div>
      {list.some((d) => d.state !== 'running') && (
        <Button size="sm" variant="ghost" icon={Trash2} className="mt-2" onClick={() => void api.downloads.clear().then(load)}>
          Clear finished
        </Button>
      )}
    </Card>
  );
}
