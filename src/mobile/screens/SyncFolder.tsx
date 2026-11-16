// A synced folder between the PC and this phone: pick one of the PC's
// shared folders; everything new or changed there since you last synced
// downloads in one tap, and files you pick on the phone go straight into
// it. (A web page can't watch the phone's own folders, so the phone side
// is "pick what to send".)

import { CheckCheck, CloudDownload, Folder, FolderSync, Upload } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { client, type FsEntry, uploadFile } from '../client';
import { errorMessage, vibrate } from '../lib/util';
import { toast } from '../state';
import { Empty } from '../ui/common';
import { SubHeader } from './More';

const KEY = 'omnihub.sync';

interface SyncState {
  path: string | null;
  /** File path → modified time when it was last downloaded here. */
  seen: Record<string, number>;
}

function read(): SyncState {
  try {
    return { path: null, seen: {}, ...(JSON.parse(localStorage.getItem(KEY) ?? '{}') as Partial<SyncState>) };
  } catch {
    return { path: null, seen: {} };
  }
}

function write(s: SyncState) {
  try {
    localStorage.setItem(KEY, JSON.stringify(s));
  } catch {
    /* not kept */
  }
}

/** Files in a folder and the folders inside it (two levels, at most 500 files). */
async function filesIn(path: string): Promise<FsEntry[]> {
  const out: FsEntry[] = [];
  const top = await client.list(path);
  for (const e of top.entries) {
    if (e.hidden) continue;
    if (!e.isDir) out.push(e);
    else if (out.length < 500) {
      const sub = await client.list(e.path).catch(() => null);
      sub?.entries.filter((x) => !x.isDir && !x.hidden).forEach((x) => out.length < 500 && out.push(x));
    }
  }
  return out;
}

function Picker({ onPick }: { onPick: (path: string) => void }) {
  const [here, setHere] = useState<{ path: string | null; parent: string | null; entries: FsEntry[] } | null>(null);
  const [roots, setRoots] = useState<{ name: string; path: string }[]>([]);
  useEffect(() => {
    client.roots().then((r) => setRoots(r.roots), (e) => toast.error(errorMessage(e)));
  }, []);
  const open = (p: string) => client.list(p).then((r) => setHere({ path: r.path, parent: r.parent, entries: r.entries.filter((x) => x.isDir && !x.hidden) }), (e) => toast.error(errorMessage(e)));
  return (
    <div className="px-4">
      <div className="mb-2 text-[14px] text-dim">Pick the PC folder to sync with this phone.</div>
      {!here && roots.length === 0 && <Empty icon={<Folder size={26} />} title="No shared folders" body="Share a folder on the PC first: Phone → Permissions → Shared folders." />}
      {!here &&
        roots.map((r) => (
          <button key={r.path} type="button" onClick={() => void open(r.path)} className="card press mb-2 flex w-full items-center gap-3 p-3.5 text-left">
            <Folder size={20} className="text-accent" />
            <span className="text-[15px] font-medium">{r.name}</span>
          </button>
        ))}
      {here && (
        <>
          <div className="mb-2 flex items-center justify-between gap-2">
            <span className="min-w-0 truncate text-[13px] text-faint">{here.path}</span>
            <button type="button" className="press shrink-0 rounded-full bg-accent px-4 py-2 text-sm font-semibold text-white" onClick={() => here.path && onPick(here.path)}>
              Sync this folder
            </button>
          </div>
          {here.parent && (
            <button type="button" onClick={() => void open(here.parent as string)} className="mb-2 text-[14px] text-accent">
              ← Up
            </button>
          )}
          {here.entries.map((e) => (
            <button key={e.path} type="button" onClick={() => void open(e.path)} className="card press mb-2 flex w-full items-center gap-3 p-3 text-left">
              <Folder size={18} className="text-dim" />
              <span className="truncate text-[15px]">{e.name}</span>
            </button>
          ))}
        </>
      )}
    </div>
  );
}

export function SyncFolderPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const [st, setSt] = useState<SyncState>(read);
  const [files, setFiles] = useState<FsEntry[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const save = (s: SyncState) => {
    setSt(s);
    write(s);
  };
  const load = useCallback(() => {
    if (!st.path) return;
    filesIn(st.path).then(setFiles, (e) => toast.error(errorMessage(e)));
  }, [st.path]);
  useEffect(() => {
    if (active) load();
  }, [active, load]);
  const fresh = (files ?? []).filter((f) => st.seen[f.path] !== f.modified);
  const getAll = async () => {
    setBusy('down');
    const seen = { ...st.seen };
    try {
      for (const f of fresh) {
        await client.download(f.path);
        seen[f.path] = f.modified;
        // Browsers allow a few downloads at a time: one by one.
        await new Promise((r) => setTimeout(r, 700));
      }
      save({ ...st, seen });
      vibrate(20);
      toast.success(`${fresh.length} ${fresh.length === 1 ? 'file' : 'files'} downloaded`);
    } catch (e) {
      save({ ...st, seen });
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };
  const send = async (list: FileList | null) => {
    if (!list?.length || !st.path) return;
    setBusy('up');
    let n = 0;
    try {
      for (const f of Array.from(list)) {
        await uploadFile(f, () => undefined, { dir: st.path });
        n++;
      }
      toast.success(`${n} ${n === 1 ? 'file' : 'files'} sent to the PC`);
      load();
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };
  return (
    <div className="flex min-h-full flex-col pb-6">
      <SubHeader title="Synced folder" onBack={onBack} />
      {!st.path ? (
        <Picker onPick={(p) => save({ path: p, seen: {} })} />
      ) : (
        <div className="space-y-3 px-4">
          <div className="card p-4">
            <div className="flex items-center gap-3">
              <span className="grid h-11 w-11 place-items-center rounded-2xl bg-accent-soft text-accent">
                <FolderSync size={21} />
              </span>
              <div className="min-w-0 flex-1">
                <div className="truncate text-[16px] font-semibold">{st.path.split(/[\\/]/).filter(Boolean).pop()}</div>
                <div className="truncate text-[12.5px] text-faint">{st.path}</div>
              </div>
            </div>
            <div className="mt-3 grid grid-cols-2 gap-2">
              <button type="button" disabled={!!busy || fresh.length === 0} onClick={() => void getAll()} className="press flex items-center justify-center gap-2 rounded-xl bg-accent px-3 py-2.5 text-[14px] font-semibold text-white disabled:opacity-50">
                {fresh.length === 0 ? <CheckCheck size={17} /> : <CloudDownload size={17} />}
                {busy === 'down' ? 'Downloading…' : fresh.length === 0 ? 'Up to date' : `Get ${fresh.length} new`}
              </button>
              <button type="button" disabled={!!busy} onClick={() => input.current?.click()} className="press flex items-center justify-center gap-2 rounded-xl border border-line bg-surface px-3 py-2.5 text-[14px] font-semibold disabled:opacity-50">
                <Upload size={17} /> {busy === 'up' ? 'Sending…' : 'Send files'}
              </button>
            </div>
            <input ref={input} type="file" multiple className="hidden" onChange={(e) => void send(e.target.files)} />
          </div>
          {files && files.length === 0 && <Empty icon={<Folder size={26} />} title="The folder is empty" body="Files you send from here, or put in it on the PC, show up here." />}
          {fresh.length > 0 && (
            <div className="card divide-y divide-line">
              {fresh.slice(0, 100).map((f) => (
                <div key={f.path} className="flex items-center justify-between gap-3 px-4 py-2.5">
                  <span className="min-w-0 truncate text-[14px]">{f.name}</span>
                  <span className="shrink-0 text-[12px] text-accent">{st.seen[f.path] ? 'changed' : 'new'}</span>
                </div>
              ))}
            </div>
          )}
          <button type="button" className="text-[13px] text-faint underline" onClick={() => save({ path: null, seen: {} })}>
            Sync a different folder
          </button>
        </div>
      )}
    </div>
  );
}
