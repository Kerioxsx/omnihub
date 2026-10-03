// Files: shared roots, folder browser (list / photo grid), preview sheet,
// multi-select download, and uploads to the PC.

import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { create } from 'zustand';
import { AnimatePresence, motion } from 'motion/react';
import {
  ArrowLeft,
  ChevronRight,
  Search,
  LayoutGrid,
  List,
  ArrowUpDown,
  Download,
  ExternalLink,
  X,
  Check,
  Plus,
  Monitor,
  FileText,
  Image as ImageIcon,
  Film,
  Music,
  Smartphone,
  Folder,
  FolderOpen,
  Copy,
  Camera,
  Images,
  FilePlus2,
  CheckCheck,
  Eye,
  EyeOff,
  ArrowDownAZ,
  ArrowUpAZ,
} from 'lucide-react';
import { formatBytes, formatDate, formatDateTime, formatRelative } from '@shared/format';
import { client, type FsEntry } from '../client';
import { useApp, toast } from '../state';
import { useUploads } from '../uploads';
import { PullToRefresh } from '../ui/PullToRefresh';
import { Sheet } from '../ui/Sheet';
import { Button, Empty, ErrorState, IconButton, ListSkeleton, PageHeader, Segmented, Switch } from '../ui/common';
import { FileIcon, kindOf } from './fileKinds';
import { UploadsPanel } from './Uploads';
import { useBackHandler } from '../lib/back';
import { copyText } from '../lib/clipboard';
import { cx, errorMessage, limiter, sepOf, shortPath, useInView, vibrate } from '../lib/util';

type SortKey = 'name' | 'size' | 'date';
type Listing = Awaited<ReturnType<typeof client.list>>;
type RootT = { name: string; path: string };

interface FilesPrefs {
  path: string | null;
  sort: SortKey;
  desc: boolean;
  showHidden: boolean;
  /** Per-folder view choice; folders without one pick automatically. */
  views: Record<string, 'list' | 'grid'>;
  setPath: (p: string | null) => void;
  setSort: (k: SortKey, desc: boolean) => void;
  setShowHidden: (v: boolean) => void;
  setView: (path: string, v: 'list' | 'grid') => void;
}

const PREFS_KEY = 'omnihub.files';
function loadPrefs(): Partial<FilesPrefs> {
  try {
    return JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}');
  } catch {
    return {};
  }
}
function savePrefs(s: FilesPrefs) {
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify({ sort: s.sort, desc: s.desc, showHidden: s.showHidden }));
  } catch {
    /* ignore */
  }
}

export const useFiles = create<FilesPrefs>((set, get) => ({
  path: null,
  sort: 'name',
  desc: false,
  showHidden: false,
  views: {},
  ...loadPrefs(),
  setPath: (path) => set({ path }),
  setSort: (sort, desc) => {
    set({ sort, desc });
    savePrefs(get());
  },
  setShowHidden: (showHidden) => {
    set({ showHidden });
    savePrefs(get());
  },
  setView: (path, v) => set({ views: { ...get().views, [path]: v } }),
}));

const thumbs = limiter(4);
const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

function rootIcon(name: string): ReactNode {
  const n = name.toLowerCase();
  if (n.includes('desktop')) return <Monitor size={22} />;
  if (n.includes('document')) return <FileText size={22} />;
  if (n.includes('download')) return <Download size={22} />;
  if (n.includes('picture') || n.includes('photo')) return <ImageIcon size={22} />;
  if (n.includes('video')) return <Film size={22} />;
  if (n.includes('music')) return <Music size={22} />;
  if (n.includes('phone')) return <Smartphone size={22} />;
  return <Folder size={22} />;
}

function countLabel(folders: number, files: number): string {
  const parts = [folders && `${folders} folder${folders > 1 ? 's' : ''}`, files && `${files} file${files > 1 ? 's' : ''}`].filter(Boolean);
  return parts.length ? parts.join(' · ') : 'Empty';
}

const norm = (p: string) => (sepOf(p) === '\\' ? p.toLowerCase().replace(/\\+$/, '') : p.replace(/\/+$/, ''));

function rootOf(path: string, roots: RootT[]): RootT | null {
  const np = norm(path);
  let best: RootT | null = null;
  for (const r of roots) {
    const nr = norm(r.path);
    if (np === nr || np.startsWith(nr + sepOf(r.path))) if (!best || nr.length > norm(best.path).length) best = r;
  }
  return best;
}

function crumbsFor(path: string, roots: RootT[]): { name: string; path: string }[] {
  const sep = sepOf(path);
  const root = rootOf(path, roots);
  if (!root) {
    const parts = path.split(sep).filter(Boolean);
    return parts.map((p, i) => ({ name: p, path: (path.startsWith(sep) ? sep : '') + parts.slice(0, i + 1).join(sep) }));
  }
  const rest = path.slice(root.path.replace(/[\\/]+$/, '').length).split(sep).filter(Boolean);
  const out = [{ name: root.name, path: root.path }];
  let acc = root.path.replace(/[\\/]+$/, '');
  for (const r of rest) {
    acc = acc + sep + r;
    out.push({ name: r, path: acc });
  }
  return out;
}

export function FilesScreen({ active }: { active: boolean }) {
  const prefs = useFiles();
  const { path, setPath } = prefs;
  const features = useApp((s) => s.info?.features);
  const [roots, setRoots] = useState<RootT[] | null>(null);
  const [rootsError, setRootsError] = useState<string | null>(null);
  const [listing, setListing] = useState<Listing | null>(null);
  const [listError, setListError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [preview, setPreview] = useState<FsEntry | null>(null);
  const [optionsOpen, setOptionsOpen] = useState(false);
  const [uploadOpen, setUploadOpen] = useState(false);
  const cache = useRef(new Map<string, Listing>());
  const reqId = useRef(0);
  const scroller = useRef<HTMLDivElement | null>(null);

  const loadRoots = useCallback(async () => {
    try {
      const r = await client.roots();
      setRoots(r.roots);
      setRootsError(null);
    } catch (e) {
      setRootsError(errorMessage(e));
    }
  }, []);

  const loadList = useCallback(async (p: string, silent = false) => {
    const id = ++reqId.current;
    const cached = cache.current.get(p);
    if (cached) setListing(cached);
    else if (!silent) setListing(null);
    setLoading(!cached);
    setListError(null);
    try {
      const l = await client.list(p);
      if (id !== reqId.current) return;
      cache.current.set(p, l);
      setListing(l);
    } catch (e) {
      if (id !== reqId.current) return;
      setListError(errorMessage(e));
    } finally {
      if (id === reqId.current) setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (active && !roots) loadRoots();
  }, [active, roots, loadRoots]);

  useEffect(() => {
    setQuery('');
    setSelected(new Set());
    if (path) loadList(path);
    else setListing(null);
    scroller.current?.scrollTo({ top: 0 });
  }, [path, loadList]);

  // Refresh the open folder when an upload lands in it.
  const doneCount = useUploads((s) => s.items.filter((i) => i.state === 'done').length);
  useEffect(() => {
    if (doneCount && path) loadList(path, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [doneCount]);

  const goUp = () => {
    if (!path) return;
    if (listing && norm(listing.path) === norm(path)) return setPath(listing.parent);
    const root = roots ? rootOf(path, roots) : null;
    if (!root || norm(root.path) === norm(path)) return setPath(null);
    const trimmed = path.replace(/[\\/]+$/, '');
    const i = trimmed.lastIndexOf(sepOf(path));
    setPath(i > 0 ? trimmed.slice(0, i) : null);
  };
  const selecting = selected.size > 0;
  useBackHandler(active && selecting, () => setSelected(new Set()));
  useBackHandler(active && !selecting && !!path, goUp);

  const entries = useMemo(() => {
    if (!listing) return [];
    const q = query.trim().toLowerCase();
    const list = listing.entries.filter((e) => (prefs.showHidden || !e.hidden) && (!q || e.name.toLowerCase().includes(q)));
    const dir = prefs.desc ? -1 : 1;
    list.sort((a, b) => {
      if (a.isDir !== b.isDir) return a.isDir ? -1 : 1;
      let c = 0;
      if (prefs.sort === 'size') c = (a.size ?? 0) - (b.size ?? 0);
      else if (prefs.sort === 'date') c = a.modified - b.modified;
      if (c === 0) c = collator.compare(a.name, b.name);
      return c * dir;
    });
    return list;
  }, [listing, query, prefs.showHidden, prefs.sort, prefs.desc]);

  const imageShare = useMemo(() => {
    const files = listing?.entries.filter((e) => !e.isDir) ?? [];
    return files.length ? files.filter((e) => e.kind === 'image').length / files.length : 0;
  }, [listing]);
  const view: 'list' | 'grid' = (path && prefs.views[path]) || (imageShare >= 0.6 && (listing?.entries.length ?? 0) >= 4 ? 'grid' : 'list');

  const refresh = async () => {
    cache.current.clear();
    if (path) await loadList(path);
    else await loadRoots();
  };

  const openEntry = (e: FsEntry) => {
    if (selecting) {
      if (e.isDir) return;
      toggle(e.path);
      return;
    }
    if (e.isDir) setPath(e.path);
    else setPreview(e);
  };
  const toggle = (p: string) =>
    setSelected((s) => {
      const n = new Set(s);
      if (n.has(p)) n.delete(p);
      else n.add(p);
      return n;
    });
  const startSelect = (e: FsEntry) => {
    if (e.isDir) return;
    vibrate(18);
    setSelected(new Set([e.path]));
  };

  const downloadSelected = async () => {
    const paths = [...selected];
    setSelected(new Set());
    toast.info(`Downloading ${paths.length} file${paths.length > 1 ? 's' : ''}`, 'Your browser may ask to allow multiple downloads.');
    for (const p of paths) {
      try {
        await client.download(p);
      } catch (e) {
        toast.error("Couldn't download", errorMessage(e));
      }
      await new Promise((r) => setTimeout(r, 700));
    }
  };

  const crumbs = path && roots ? crumbsFor(listing?.path ?? path, roots) : [];
  const current = path ? (crumbs[crumbs.length - 1]?.name ?? path) : null;
  const inRoot = path && roots ? rootOf(path, roots) : null;
  const incoming = roots?.find((r) => r.name === 'From phone') ?? null;
  const uploadsOn = features?.uploads !== false;
  const fileCount = entries.filter((e) => !e.isDir).length;

  return (
    <div className="relative h-full">
      <PullToRefresh onRefresh={refresh} scrollRef={(el) => (scroller.current = el)}>
        {/* Header */}
        {!path ? (
          <PageHeader title="Files" subtitle={useApp.getState().info?.name} />
        ) : (
          <div className="sticky top-0 z-20 border-b border-line bg-glass pt-[calc(var(--safe-top)+8px)] backdrop-blur-2xl">
            {selecting ? (
              <div className="px-safe flex h-14 items-center gap-2">
                <IconButton label="Cancel selection" onClick={() => setSelected(new Set())}>
                  <X size={20} />
                </IconButton>
                <div className="flex-1 text-lg font-semibold">{selected.size} selected</div>
                <IconButton label="Select all files" onClick={() => setSelected(new Set(entries.filter((e) => !e.isDir).map((e) => e.path)))}>
                  <CheckCheck size={19} />
                </IconButton>
                <Button size="sm" icon={<Download size={16} />} onClick={downloadSelected}>
                  Download
                </Button>
              </div>
            ) : (
              <div className="px-safe flex h-14 items-center gap-2">
                <IconButton label="Back" onClick={goUp}>
                  <ArrowLeft size={20} />
                </IconButton>
                <div className="min-w-0 flex-1 px-1">
                  <div className="truncate font-display text-[19px] font-bold leading-tight">{current}</div>
                  <div className="text-xs text-dim">{listing ? countLabel(entries.filter((e) => e.isDir).length, fileCount) : 'Loading…'}</div>
                </div>
                <IconButton label="Sort and view options" onClick={() => setOptionsOpen(true)}>
                  <ArrowUpDown size={18} />
                </IconButton>
                <IconButton label={view === 'grid' ? 'List view' : 'Grid view'} onClick={() => path && prefs.setView(path, view === 'grid' ? 'list' : 'grid')}>
                  {view === 'grid' ? <List size={19} /> : <LayoutGrid size={18} />}
                </IconButton>
              </div>
            )}
            {crumbs.length > 1 && !selecting && (
              <div className="hscroll px-safe flex items-center gap-1 pb-2.5 text-[13px]">
                {crumbs.map((c, i) => (
                  <div key={c.path} className="flex shrink-0 items-center gap-1">
                    {i > 0 && <ChevronRight size={14} className="text-faint" />}
                    <button
                      onClick={() => i < crumbs.length - 1 && setPath(c.path)}
                      className={cx('max-w-[160px] truncate rounded-lg px-2 py-1 font-medium', i === crumbs.length - 1 ? 'bg-accent-soft text-accent' : 'text-dim active:bg-surface-2')}
                    >
                      {c.name}
                    </button>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        <div className="px-safe pb-tabbar">
          <UploadsPanel className={path ? 'mt-3' : ''} />

          {!path ? (
            rootsError && !roots ? (
              <ErrorState message={rootsError} onRetry={loadRoots} />
            ) : !roots ? (
              <ListSkeleton rows={6} />
            ) : roots.length === 0 ? (
              <Empty icon={<FolderOpen size={30} />} title="No folders shared" body="Choose what the phone can browse in OmniHub → Settings → Phone on the PC." />
            ) : (
              <div className="card mt-1 overflow-hidden">
                {roots.map((r, i) => (
                  <button key={r.path} onClick={() => setPath(r.path)} className={cx('press flex w-full items-center gap-3.5 px-4 py-3.5 text-left active:bg-surface-2', i > 0 && 'border-t border-line')}>
                    <div className={cx('grid h-11 w-11 place-items-center rounded-2xl', r.name === 'From phone' ? 'grad-bg text-white' : 'bg-violet-500/15 text-violet-400')}>{rootIcon(r.name)}</div>
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[16px] font-semibold">{r.name}</div>
                      <div className="truncate text-xs text-faint">{r.name === 'From phone' ? 'Where uploads from this phone go' : shortPath(r.path)}</div>
                    </div>
                    <ChevronRight size={18} className="text-faint" />
                  </button>
                ))}
              </div>
            )
          ) : (
            <>
              <div className="relative mt-3">
                <Search size={17} className="pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-faint" />
                <input className="field h-11 py-0 pl-10 pr-10" placeholder={`Search in ${current ?? 'folder'}`} value={query} onChange={(e) => setQuery(e.target.value)} enterKeyHint="search" type="search" />
                {query && (
                  <button aria-label="Clear search" onClick={() => setQuery('')} className="absolute right-2 top-1/2 grid h-8 w-8 -translate-y-1/2 place-items-center rounded-full text-faint">
                    <X size={16} />
                  </button>
                )}
              </div>
              {listError && !listing ? (
                <ErrorState message={listError} onRetry={() => path && loadList(path)} />
              ) : !listing || loading ? (
                <div className="mt-3">
                  <ListSkeleton rows={7} />
                </div>
              ) : entries.length === 0 ? (
                query ? (
                  <Empty icon={<Search size={28} />} title="No matches" body={`Nothing named like “${query}” here.`} />
                ) : (
                  <Empty icon={<FolderOpen size={30} />} title="This folder is empty" body={uploadsOn ? 'Tap + to send files from your phone here.' : undefined} />
                )
              ) : view === 'grid' ? (
                <div className="mt-3 grid grid-cols-3 gap-1.5">
                  {entries.map((e) => (
                    <GridTile key={e.path} entry={e} selected={selected.has(e.path)} selecting={selecting} onOpen={() => openEntry(e)} onLongPress={() => startSelect(e)} />
                  ))}
                </div>
              ) : (
                <div className="mt-2 -mx-2">
                  {entries.map((e) => (
                    <EntryRow key={e.path} entry={e} selected={selected.has(e.path)} selecting={selecting} onOpen={() => openEntry(e)} onLongPress={() => startSelect(e)} />
                  ))}
                </div>
              )}
              {listing && entries.length > 0 && !selecting && fileCount > 0 && (
                <div className="mt-4 text-center text-xs text-faint">Long-press a file to select several</div>
              )}
            </>
          )}
        </div>
      </PullToRefresh>

      {uploadsOn && !selecting && (
        <motion.button
          initial={{ scale: 0.6, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          whileTap={{ scale: 0.92 }}
          aria-label="Send files to the PC"
          onClick={() => setUploadOpen(true)}
          className="grad-bg fixed right-[max(16px,var(--safe-right))] z-30 grid h-[58px] w-[58px] place-items-center rounded-[20px] text-white shadow-[0_14px_34px_-10px_var(--accent-glow)] bottom-tabbar"
        >
          <Plus size={28} strokeWidth={2.4} />
        </motion.button>
      )}

      <PreviewSheet entry={preview} onClose={() => setPreview(null)} />
      <OptionsSheet open={optionsOpen} onClose={() => setOptionsOpen(false)} view={view} path={path} />
      <UploadSheet
        open={uploadOpen}
        onClose={() => setUploadOpen(false)}
        here={path && inRoot ? { path: listing?.path ?? path, name: current ?? path } : null}
        incoming={incoming}
        onQueued={() => scroller.current?.scrollTo({ top: 0, behavior: 'smooth' })}
      />
    </div>
  );
}

// ---------- rows & tiles ----------

function useLongPress(onLongPress: () => void, onTap: () => void) {
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const start = useRef<{ x: number; y: number } | null>(null);
  const fired = useRef(false);
  const clear = () => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
  };
  return {
    onPointerDown: (e: React.PointerEvent) => {
      fired.current = false;
      start.current = { x: e.clientX, y: e.clientY };
      clear();
      timer.current = setTimeout(() => {
        fired.current = true;
        onLongPress();
      }, 480);
    },
    onPointerMove: (e: React.PointerEvent) => {
      if (start.current && Math.hypot(e.clientX - start.current.x, e.clientY - start.current.y) > 10) clear();
    },
    onPointerUp: clear,
    onPointerCancel: clear,
    onPointerLeave: clear,
    onContextMenu: (e: React.MouseEvent) => e.preventDefault(),
    onClick: () => {
      if (fired.current) {
        fired.current = false;
        return;
      }
      onTap();
    },
  };
}

function Thumb({ path, size, className, fallback }: { path: string; size: number; className?: string; fallback: ReactNode }) {
  const [url, setUrl] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  const alive = useRef(true);
  const urlRef = useRef<string | null>(null);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      if (urlRef.current) URL.revokeObjectURL(urlRef.current);
      urlRef.current = null;
    };
  }, []);
  const ref = useInView<HTMLDivElement>(() => {
    thumbs(() => (alive.current ? client.thumbnail(path, size) : Promise.reject(new Error('gone')))).then(
      (u) => {
        if (!alive.current) return URL.revokeObjectURL(u);
        urlRef.current = u;
        setUrl(u);
      },
      () => alive.current && setFailed(true),
    );
  });
  return (
    <div ref={ref} className={cx('relative overflow-hidden', className)}>
      {url ? (
        <img src={url} alt="" draggable={false} className="h-full w-full object-cover" style={{ animation: 'fade-up .25s ease both' }} />
      ) : failed ? (
        fallback
      ) : (
        <div className="skeleton h-full w-full rounded-none" />
      )}
    </div>
  );
}

function EntryRow({ entry: e, selected, selecting, onOpen, onLongPress }: { entry: FsEntry; selected: boolean; selecting: boolean; onOpen: () => void; onLongPress: () => void }) {
  const lp = useLongPress(onLongPress, onOpen);
  const isImage = kindOf(e.name, e.kind) === 'image';
  return (
    <button {...lp} className={cx('press flex w-full items-center gap-3 rounded-2xl px-2 py-2 text-left', selected ? 'bg-accent-soft' : 'active:bg-surface-2', selecting && e.isDir && 'opacity-40')}>
      {isImage ? <Thumb path={e.path} size={128} className="h-11 w-11 shrink-0 rounded-xl" fallback={<FileIcon name={e.name} kind={e.kind} />} /> : <FileIcon name={e.name} kind={e.kind} />}
      <div className="min-w-0 flex-1">
        <div className="truncate text-[15px] font-semibold">{e.name}</div>
        <div className="truncate text-[13px] text-dim">
          {e.isDir ? (e.size != null ? `${formatBytes(e.size)} · ` : '') : `${formatBytes(e.size)} · `}
          {formatRelative(e.modified)}
        </div>
      </div>
      {selecting ? (
        !e.isDir && <div className={cx('grid h-6 w-6 place-items-center rounded-full border-2', selected ? 'border-accent bg-accent text-white' : 'border-line-strong')}>{selected && <Check size={14} strokeWidth={3} />}</div>
      ) : e.isDir ? (
        <ChevronRight size={18} className="shrink-0 text-faint" />
      ) : null}
    </button>
  );
}

function GridTile({ entry: e, selected, selecting, onOpen, onLongPress }: { entry: FsEntry; selected: boolean; selecting: boolean; onOpen: () => void; onLongPress: () => void }) {
  const lp = useLongPress(onLongPress, onOpen);
  const isImage = kindOf(e.name, e.kind) === 'image';
  return (
    <button {...lp} className={cx('press relative aspect-square overflow-hidden rounded-xl bg-surface-2 text-left', selecting && e.isDir && 'opacity-40')} aria-label={e.name}>
      {isImage ? (
        <Thumb path={e.path} size={320} className="h-full w-full" fallback={<div className="grid h-full w-full place-items-center"><FileIcon name={e.name} kind={e.kind} size={48} /></div>} />
      ) : (
        <div className="flex h-full w-full flex-col items-center justify-center gap-2 p-2">
          <FileIcon name={e.name} kind={e.kind} size={46} />
          <div className="line-clamp-2 w-full break-all text-center text-[11px] font-medium leading-tight text-dim">{e.name}</div>
        </div>
      )}
      {selected && <div className="absolute inset-0 bg-accent/25 ring-[3px] ring-inset ring-accent" />}
      {selecting && !e.isDir && (
        <div className={cx('absolute right-1.5 top-1.5 grid h-6 w-6 place-items-center rounded-full border-2', selected ? 'border-white bg-accent text-white' : 'border-white/80 bg-black/25')}>{selected && <Check size={14} strokeWidth={3} />}</div>
      )}
    </button>
  );
}

// ---------- sheets ----------

function PreviewSheet({ entry, onClose }: { entry: FsEntry | null; onClose: () => void }) {
  const [last, setLast] = useState<FsEntry | null>(entry);
  useEffect(() => {
    if (entry) setLast(entry);
  }, [entry]);
  const e = entry ?? last;
  const [busy, setBusy] = useState<'dl' | 'open' | null>(null);
  const isImage = e ? kindOf(e.name, e.kind) === 'image' : false;

  const act = async (inline: boolean) => {
    if (!e) return;
    setBusy(inline ? 'open' : 'dl');
    try {
      await client.download(e.path, inline);
      if (!inline) toast.success('Downloading', e.name);
    } catch (err) {
      toast.error(inline ? "Couldn't open" : "Couldn't download", errorMessage(err));
    } finally {
      setBusy(null);
    }
  };

  return (
    <Sheet
      open={!!entry}
      onClose={onClose}
      title={e?.name}
      subtitle={e ? `${formatBytes(e.size)} · ${formatDate(e.modified)}` : undefined}
      footer={
        e && (
          <div className="flex gap-2.5">
            <Button className="flex-1" icon={<Download size={18} />} loading={busy === 'dl'} onClick={() => act(false)}>
              Download
            </Button>
            <Button variant="ghost" className="flex-1" icon={<ExternalLink size={18} />} loading={busy === 'open'} onClick={() => act(true)}>
              Open
            </Button>
          </div>
        )
      }
    >
      {e && (
        <div>
          <div className="grid aspect-[4/3] place-items-center overflow-hidden rounded-2xl bg-surface">
            {isImage ? (
              <Thumb key={e.path} path={e.path} size={1024} className="h-full w-full [&_img]:object-contain" fallback={<FileIcon name={e.name} kind={e.kind} size={72} />} />
            ) : (
              <FileIcon name={e.name} kind={e.kind} size={84} className="rounded-3xl" />
            )}
          </div>
          <dl className="mt-4 divide-y divide-[var(--border)] rounded-2xl border border-line text-sm">
            <InfoRow label="Size" value={e.size != null ? `${formatBytes(e.size)} (${e.size.toLocaleString()} bytes)` : '—'} />
            <InfoRow label="Modified" value={formatDateTime(e.modified)} />
            <InfoRow label="Type" value={kindOf(e.name, e.kind).replace(/^\w/, (c) => c.toUpperCase())} />
            <div className="flex items-start gap-3 px-3.5 py-3">
              <dt className="w-20 shrink-0 text-dim">Location</dt>
              <dd className="selectable min-w-0 flex-1 break-all font-mono text-[12.5px] leading-relaxed">{e.path}</dd>
              <button
                aria-label="Copy path"
                className="press -my-1 grid h-8 w-8 shrink-0 place-items-center rounded-full bg-surface-2 text-dim"
                onClick={async () => ((await copyText(e.path)) ? toast.success('Path copied') : toast.error("Couldn't copy"))}
              >
                <Copy size={15} />
              </button>
            </div>
          </dl>
        </div>
      )}
    </Sheet>
  );
}

function InfoRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start gap-3 px-3.5 py-3">
      <dt className="w-20 shrink-0 text-dim">{label}</dt>
      <dd className="min-w-0 flex-1 break-words font-medium">{value}</dd>
    </div>
  );
}

function OptionsSheet({ open, onClose, view, path }: { open: boolean; onClose: () => void; view: 'list' | 'grid'; path: string | null }) {
  const p = useFiles();
  return (
    <Sheet open={open} onClose={onClose} title="Sort & view">
      <div className="space-y-5 pb-2">
        <div>
          <div className="mb-2 text-[13px] font-semibold text-dim">Sort by</div>
          <Segmented
            value={p.sort}
            onChange={(k) => p.setSort(k, k === 'name' ? false : true)}
            options={[
              { value: 'name', label: 'Name' },
              { value: 'size', label: 'Size' },
              { value: 'date', label: 'Date' },
            ]}
          />
        </div>
        <div>
          <div className="mb-2 text-[13px] font-semibold text-dim">Order</div>
          <Segmented
            value={p.desc ? 'desc' : 'asc'}
            onChange={(v) => p.setSort(p.sort, v === 'desc')}
            options={[
              { value: 'asc', label: <><ArrowDownAZ size={16} /> {p.sort === 'name' ? 'A → Z' : p.sort === 'size' ? 'Smallest' : 'Oldest'}</> },
              { value: 'desc', label: <><ArrowUpAZ size={16} /> {p.sort === 'name' ? 'Z → A' : p.sort === 'size' ? 'Largest' : 'Newest'}</> },
            ]}
          />
        </div>
        {path && (
          <div>
            <div className="mb-2 text-[13px] font-semibold text-dim">View</div>
            <Segmented
              value={view}
              onChange={(v) => p.setView(path, v)}
              options={[
                { value: 'list', label: <><List size={16} /> List</> },
                { value: 'grid', label: <><LayoutGrid size={16} /> Grid</> },
              ]}
            />
          </div>
        )}
        <div className="flex items-center gap-3 rounded-2xl bg-surface px-4 py-3">
          {p.showHidden ? <Eye size={18} className="text-dim" /> : <EyeOff size={18} className="text-dim" />}
          <div className="flex-1 text-[15px] font-medium">Show hidden files</div>
          <Switch on={p.showHidden} onChange={p.setShowHidden} label="Show hidden files" />
        </div>
      </div>
    </Sheet>
  );
}

function UploadSheet({ open, onClose, here, incoming, onQueued }: { open: boolean; onClose: () => void; here: { path: string; name: string } | null; incoming: RootT | null; onQueued: () => void }) {
  const add = useUploads((s) => s.add);
  const [dest, setDest] = useState<'here' | 'incoming'>('here');
  useEffect(() => {
    if (open) setDest(here ? 'here' : 'incoming');
  }, [open, here]);
  const filesRef = useRef<HTMLInputElement>(null);
  const mediaRef = useRef<HTMLInputElement>(null);
  const cameraRef = useRef<HTMLInputElement>(null);

  const onPick = (list: FileList | null) => {
    const files = list ? Array.from(list) : [];
    if (!files.length) return;
    const target = dest === 'here' && here ? { dir: here.path, label: here.name } : { dir: null, label: incoming?.name ?? 'From phone' };
    add(files, target.dir, target.label);
    vibrate(10);
    onClose();
    onQueued();
  };

  const picker = (ref: React.RefObject<HTMLInputElement | null>, icon: ReactNode, title: string, sub: string) => (
    <button onClick={() => ref.current?.click()} className="press flex w-full items-center gap-3.5 rounded-2xl bg-surface px-4 py-3.5 text-left active:bg-surface-2">
      <div className="grid h-11 w-11 place-items-center rounded-2xl bg-accent-soft text-accent">{icon}</div>
      <div className="min-w-0 flex-1">
        <div className="text-[15px] font-semibold">{title}</div>
        <div className="text-xs text-dim">{sub}</div>
      </div>
      <ChevronRight size={18} className="text-faint" />
    </button>
  );

  return (
    <Sheet open={open} onClose={onClose} title="Send to PC" subtitle="Resumable — pause any time, or retry if Wi-Fi drops.">
      <div className="pb-2 pt-2">
        <div className="mb-2 text-[13px] font-semibold text-dim">Save to</div>
        <div className="space-y-2">
          {here && <DestOption active={dest === 'here'} onClick={() => setDest('here')} icon={<FolderOpen size={19} />} title={here.name} sub="This folder" />}
          <DestOption active={dest === 'incoming'} onClick={() => setDest('incoming')} icon={<Smartphone size={19} />} title={incoming?.name ?? 'From phone'} sub={incoming ? shortPath(incoming.path, 40) : 'The PC’s incoming folder'} />
        </div>
        <div className="mb-2 mt-5 text-[13px] font-semibold text-dim">Choose</div>
        <div className="space-y-2">
          {picker(mediaRef, <Images size={21} />, 'Photos & videos', 'From your gallery')}
          {picker(cameraRef, <Camera size={21} />, 'Take a photo or video', 'Opens the camera')}
          {picker(filesRef, <FilePlus2 size={21} />, 'Files', 'Any file on this phone')}
        </div>
        <input ref={filesRef} type="file" multiple hidden onChange={(e) => (onPick(e.target.files), (e.target.value = ''))} />
        <input ref={mediaRef} type="file" multiple accept="image/*,video/*" hidden onChange={(e) => (onPick(e.target.files), (e.target.value = ''))} />
        <input ref={cameraRef} type="file" accept="image/*,video/*" capture="environment" hidden onChange={(e) => (onPick(e.target.files), (e.target.value = ''))} />
      </div>
    </Sheet>
  );
}

function DestOption({ active, onClick, icon, title, sub }: { active: boolean; onClick: () => void; icon: ReactNode; title: string; sub: string }) {
  return (
    <button onClick={onClick} className={cx('press flex w-full items-center gap-3 rounded-2xl border px-3.5 py-3 text-left transition-colors', active ? 'border-accent bg-accent-soft' : 'border-line bg-surface')}>
      <div className={cx('grid h-9 w-9 place-items-center rounded-xl', active ? 'bg-accent text-white' : 'bg-surface-2 text-dim')}>{icon}</div>
      <div className="min-w-0 flex-1">
        <div className="truncate text-[15px] font-semibold">{title}</div>
        <div className="truncate text-xs text-dim">{sub}</div>
      </div>
      <AnimatePresence>
        {active && (
          <motion.div initial={{ scale: 0 }} animate={{ scale: 1 }} exit={{ scale: 0 }} className="grid h-6 w-6 place-items-center rounded-full bg-accent text-white">
            <Check size={14} strokeWidth={3} />
          </motion.div>
        )}
      </AnimatePresence>
    </button>
  );
}
