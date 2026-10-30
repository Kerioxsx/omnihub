// Hidden files: any file, of any size, moved into the vault encrypted —
// gone from where it was until you unhide it with the vault unlocked.

import { basename, extColor, formatBytes, formatRelative } from '@shared/format';
import type { HiddenFile, HiddenProgress } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { ExternalLink, EyeOff, FileArchive, FileText, Film, FolderInput, FolderOutput, Image, type LucideIcon, Music, Save, Square, Trash, Undo2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { useDropClaim } from '../../components/DropOverlay';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, Skeleton } from '../../components/ui/Card';
import { SearchInput } from '../../components/ui/Form';
import { EmptyState } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';

interface Job {
  id: string;
  label: string;
  done: number;
  total: number;
}

const ext = (name: string) => (name.includes('.') ? name.split('.').pop()!.toLowerCase() : '');

function fileIcon(name: string): LucideIcon {
  const e = ext(name);
  if (['zip', '7z', 'rar', 'tar', 'gz', 'iso'].includes(e)) return FileArchive;
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'heic', 'bmp'].includes(e)) return Image;
  if (['mp4', 'mkv', 'mov', 'avi', 'webm'].includes(e)) return Film;
  if (['mp3', 'wav', 'flac', 'm4a', 'ogg'].includes(e)) return Music;
  return FileText;
}

const newJob = () => (crypto.randomUUID?.() ?? `${Date.now()}-${Math.random()}`).replace(/-/g, '');

export function HiddenFiles() {
  const [files, setFiles] = useState<HiddenFile[] | null>(null);
  const [q, setQ] = useState('');
  const [jobs, setJobs] = useState<Job[]>([]);

  const load = useCallback(async () => {
    try {
      setFiles(await api.hidden.list());
    } catch (e) {
      toast.error('Couldn’t list hidden files', errorText(e));
    }
  }, []);
  useEffect(() => void load(), [load]);

  useEvent<HiddenProgress>('hidden:progress', (p) => setJobs((js) => js.map((j) => (j.id === p.job ? { ...j, done: p.done, total: p.total } : j))));

  /** Run one job with a progress row; returns null when it failed or was stopped. */
  const run = async <T,>(label: string, f: (job: string) => Promise<T>): Promise<T | null> => {
    const id = newJob();
    setJobs((js) => [...js, { id, label, done: 0, total: 0 }]);
    try {
      return await f(id);
    } catch (e) {
      const msg = errorText(e);
      if (msg !== 'Stopped.') toast.error(`${label} didn’t finish`, msg);
      return null;
    } finally {
      setJobs((js) => js.filter((j) => j.id !== id));
    }
  };

  const hideMany = async (paths: string[]) => {
    if (!paths.length) return;
    const what = paths.length === 1 ? `“${basename(paths[0])}”` : `${paths.length} files`;
    const ok = await confirm({ title: `Hide ${what}?`, description: 'It’s encrypted into OmniHub and removed from where it is now. Only this vault, unlocked, brings it back — with Unhide.', confirmLabel: 'Hide' });
    if (!ok) return;
    let hidden = 0;
    for (const path of paths) {
      const r = await run(`Hiding ${basename(path)}`, (job) => api.hidden.hide(path, job));
      if (!r) continue;
      hidden++;
      setFiles((fs) => [r.file, ...(fs ?? [])]);
      if (r.originalLeft) toast.warn(`${r.file.name} is hidden, but the original is still there`, 'Another app has it open. Close that app and delete the original yourself.');
    }
    if (hidden) toast.success(hidden === 1 ? 'Hidden' : `${hidden} files hidden`, 'Encrypted with your vault key.');
  };

  const pick = async () => hideMany(await api.app.pickFiles('Choose files to hide'));

  useDropClaim({ title: 'Drop to hide in OmniHub', detail: 'Encrypted with your vault key and removed from where it is now.', icon: EyeOff, onDrop: (paths) => void hideMany(paths) });

  const unhide = async (f: HiddenFile, choose: boolean) => {
    const folder = choose ? await api.app.pickFolder(`Where should ${f.name} go?`) : null;
    if (choose && !folder) return;
    const to = await run(`Unhiding ${f.name}`, (job) => api.hidden.unhide(f.id, folder, job));
    if (to == null) return;
    setFiles((fs) => fs?.filter((x) => x.id !== f.id) ?? fs);
    toast.success('Back where it was', to);
  };

  const saveCopy = async (f: HiddenFile) => {
    const path = await api.app.saveFile('Save a copy', f.name);
    if (!path) return;
    if ((await run(`Copying ${f.name}`, (job) => api.hidden.saveCopy(f.id, path, job))) !== null) toast.success('Copy saved — the file stays hidden', path);
  };

  const open = async (f: HiddenFile) => {
    if ((await run(`Opening ${f.name}`, (job) => api.hidden.open(f.id, job))) !== null) toast.info('Opened a temporary copy', 'It’s deleted when the vault locks. Changes to it aren’t saved back.');
  };

  const remove = async (f: HiddenFile) => {
    const ok = await confirm({ title: `Delete “${f.name}” for good?`, description: 'It can’t be recovered, not even with the vault password.', tone: 'danger', confirmLabel: 'Delete' });
    if (!ok) return;
    try {
      await api.hidden.remove(f.id);
      setFiles((fs) => fs?.filter((x) => x.id !== f.id) ?? fs);
    } catch (e) {
      toast.error('Couldn’t delete it', errorText(e));
    }
  };

  const list = useMemo(() => {
    const needle = q.trim().toLowerCase();
    return (files ?? []).filter((f) => !needle || `${f.name} ${f.folder}`.toLowerCase().includes(needle));
  }, [files, q]);
  const total = (files ?? []).reduce((n, f) => n + f.size, 0);

  return (
    <Card className="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div className="flex items-center gap-2 border-b border-line px-4 py-3">
        <EyeOff size={17} className="text-accent" aria-hidden />
        <div className="min-w-0 flex-1">
          <div className="text-[14px] font-semibold text-fg">Hidden files</div>
          <div className="truncate text-[12px] text-faint">Any file, any size — encrypted, and gone from its folder until you unhide it.</div>
        </div>
        {!!files?.length && <SearchInput value={q} onChange={setQ} placeholder="Search hidden files" className="w-56" aria-label="Search hidden files" />}
        <Button variant="primary" icon={FolderInput} onClick={() => void pick()}>
          Hide files…
        </Button>
      </div>

      <AnimatePresence initial={false}>
        {jobs.map((j) => (
          <motion.div key={j.id} initial={{ height: 0, opacity: 0 }} animate={{ height: 'auto', opacity: 1 }} exit={{ height: 0, opacity: 0 }} className="border-b border-line bg-surface-2/40">
            <div className="flex items-center gap-3 px-4 py-2.5">
              <div className="min-w-0 flex-1">
                <div className="flex justify-between gap-3 text-[12.5px]">
                  <span className="truncate text-fg">{j.label}</span>
                  <span className="shrink-0 tabular text-faint">{j.total ? `${formatBytes(j.done)} of ${formatBytes(j.total)}` : 'Starting…'}</span>
                </div>
                <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-surface-3">
                  <div className="h-full rounded-full accent-gradient transition-[width] duration-200" style={{ width: `${j.total ? Math.min(100, (j.done / j.total) * 100) : 2}%` }} />
                </div>
              </div>
              <IconButton icon={Square} label="Stop" size="sm" onClick={() => void api.hidden.cancel(j.id)} />
            </div>
          </motion.div>
        ))}
      </AnimatePresence>

      <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
        {!files ? (
          <div className="space-y-1.5 p-1.5">
            {[0, 1, 2].map((i) => (
              <Skeleton key={i} className="h-14" />
            ))}
          </div>
        ) : files.length === 0 ? (
          <EmptyState
            icon={EyeOff}
            title="Nothing hidden yet"
            description="Drop any file on this window — a zip, videos, documents, however big — or choose Hide files. It’s encrypted with your vault key and removed from where it was."
            action={
              <Button variant="primary" icon={FolderInput} onClick={() => void pick()}>
                Hide files…
              </Button>
            }
          />
        ) : list.length === 0 ? (
          <EmptyState compact icon={EyeOff} title="No matches" />
        ) : (
          list.map((f) => {
            const I = fileIcon(f.name);
            return (
              <div key={f.id} className="group flex items-center gap-3 rounded-xl px-3 py-2.5 hover:bg-surface-2">
                <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-surface-3" style={{ color: extColor(ext(f.name)) }}>
                  <I size={17} aria-hidden />
                </span>
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[13.5px] font-medium text-fg">{f.name}</div>
                  <div className="truncate text-[12px] text-faint" title={f.folder}>
                    {formatBytes(f.size)} · hidden {formatRelative(f.hiddenAt)} · from {f.folder || 'unknown'}
                  </div>
                </div>
                <div className="flex shrink-0 items-center gap-0.5 opacity-70 transition-opacity group-hover:opacity-100">
                  <IconButton icon={ExternalLink} label="Open a temporary copy" size="sm" onClick={() => void open(f)} />
                  <IconButton icon={Save} label="Save a copy (stays hidden)" size="sm" onClick={() => void saveCopy(f)} />
                  <IconButton icon={FolderOutput} label="Unhide into another folder" size="sm" onClick={() => void unhide(f, true)} />
                  <Button size="sm" icon={Undo2} onClick={() => void unhide(f, false)} title={`Put it back in ${f.folder}`}>
                    Unhide
                  </Button>
                  <IconButton icon={Trash} label="Delete for good" size="sm" variant="danger" onClick={() => void remove(f)} />
                </div>
              </div>
            );
          })
        )}
      </div>
      {!!files?.length && (
        <div className="border-t border-line px-4 py-2 text-[11.5px] text-faint">
          {files.length} {files.length === 1 ? 'file' : 'files'} · {formatBytes(total)} · each with its own key, wrapped by your vault key · drag files here to hide them
        </div>
      )}
    </Card>
  );
}
