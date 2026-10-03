// Phone → PC uploads: a queue in a zustand store so it survives tab
// switches. Two files at a time, pause/resume (abort + resumeId), cancel,
// retry, speed and ETA, and the screen is kept awake while sending.

import { create } from 'zustand';
import { ApiError, cancelUpload, uploadFile, type UploadHandle } from './client';
import { keepAwake } from './lib/wakelock';
import { errorMessage } from './lib/util';
import { toast } from './state';

export type UploadState = 'queued' | 'uploading' | 'paused' | 'done' | 'error' | 'cancelled';

export interface UploadItem {
  key: string;
  file: File;
  name: string;
  size: number;
  /** Destination folder on the PC (null = the "From phone" folder). */
  dir: string | null;
  dirLabel: string;
  /** Server upload id once started. */
  id?: string;
  offset: number;
  sent: number;
  state: UploadState;
  error?: string;
  path?: string;
  sha256?: string;
  speed: number;
  eta: number;
  addedAt: number;
}

interface UploadsStore {
  items: UploadItem[];
  add: (files: File[], dir: string | null, dirLabel: string) => void;
  pause: (key: string) => void;
  resume: (key: string) => void;
  cancel: (key: string) => void;
  retry: (key: string) => void;
  remove: (key: string) => void;
  clearFinished: () => void;
}

const MAX_PARALLEL = 2;
const controllers = new Map<string, AbortController>();
const samples = new Map<string, { t: number; b: number }[]>();
let seq = 0;
let batchDone = 0;

export const isActive = (s: UploadState) => s === 'queued' || s === 'uploading';

export const useUploads = create<UploadsStore>((set, get) => {
  const patch = (key: string, p: Partial<UploadItem>) =>
    set((s) => ({ items: s.items.map((i) => (i.key === key ? { ...i, ...p } : i)) }));
  const find = (key: string) => get().items.find((i) => i.key === key);

  const onProgress = (key: string, h: UploadHandle) => {
    const cur = find(key);
    if (!cur || cur.state === 'cancelled') return;
    const sent = h.sent ?? h.offset;
    const now = performance.now();
    const list = samples.get(key) ?? [];
    list.push({ t: now, b: sent });
    while (list.length > 2 && now - list[0].t > 4000) list.shift();
    samples.set(key, list);
    let speed = cur.speed;
    if (list.length >= 2) {
      const span = (list[list.length - 1].t - list[0].t) / 1000;
      if (span > 0.4) speed = (list[list.length - 1].b - list[0].b) / span;
    }
    const eta = speed > 0 ? (h.size - sent) / speed : NaN;
    patch(key, {
      id: h.id,
      offset: h.offset,
      sent,
      speed,
      eta,
      state: h.state === 'uploading' ? 'uploading' : h.state,
      error: h.error,
      path: h.path,
      sha256: h.sha256,
    });
  };

  const run = async (item: UploadItem) => {
    const ctrl = new AbortController();
    controllers.set(item.key, ctrl);
    samples.delete(item.key);
    patch(item.key, { state: 'uploading', error: undefined, speed: 0, eta: NaN });
    try {
      let h: UploadHandle;
      try {
        h = await uploadFile(item.file, (u) => onProgress(item.key, u), { dir: item.dir ?? undefined, signal: ctrl.signal, resumeId: item.id });
      } catch (e) {
        // The PC forgot a paused upload (cleaned up, or restarted elsewhere): start over.
        if (item.id && e instanceof ApiError && e.status === 404 && !ctrl.signal.aborted) {
          patch(item.key, { id: undefined, offset: 0, sent: 0 });
          h = await uploadFile(item.file, (u) => onProgress(item.key, u), { dir: item.dir ?? undefined, signal: ctrl.signal });
        } else throw e;
      }
      const cur = find(item.key);
      if (cur?.state === 'cancelled') return;
      if (h.state === 'done') batchDone++;
    } catch (e) {
      const cur = find(item.key);
      if (cur && cur.state !== 'cancelled') patch(item.key, { state: 'error', error: errorMessage(e) });
    } finally {
      controllers.delete(item.key);
      pump();
    }
  };

  const pump = () => {
    const items = get().items;
    let active = items.filter((i) => i.state === 'uploading').length;
    for (const i of items) {
      if (active >= MAX_PARALLEL) break;
      if (i.state === 'queued' && !controllers.has(i.key)) {
        active++;
        run(i);
      }
    }
    const busy = get().items.some((i) => isActive(i.state));
    keepAwake('uploads', busy);
    if (!busy && batchDone > 0) {
      const n = batchDone;
      batchDone = 0;
      const last = [...get().items].reverse().find((i) => i.state === 'done');
      toast.success(n === 1 ? `Sent ${last?.name ?? 'file'} to the PC` : `${n} files sent to the PC`, n === 1 ? last?.path : undefined);
    }
  };

  return {
    items: [],
    add: (files, dir, dirLabel) => {
      const now = Date.now();
      const items: UploadItem[] = files.map((file) => ({
        key: `u${++seq}`,
        file,
        name: file.name || 'file',
        size: file.size,
        dir,
        dirLabel,
        offset: 0,
        sent: 0,
        state: 'queued',
        speed: 0,
        eta: NaN,
        addedAt: now,
      }));
      set((s) => ({ items: [...s.items, ...items] }));
      pump();
    },
    pause: (key) => {
      const i = find(key);
      if (!i) return;
      if (i.state === 'queued') patch(key, { state: 'paused' });
      controllers.get(key)?.abort();
      setTimeout(pump, 0);
    },
    resume: (key) => {
      patch(key, { state: 'queued', error: undefined });
      pump();
    },
    retry: (key) => {
      patch(key, { state: 'queued', error: undefined });
      pump();
    },
    cancel: (key) => {
      const i = find(key);
      if (!i) return;
      patch(key, { state: 'cancelled', speed: 0 });
      controllers.get(key)?.abort();
      if (i.id) cancelUpload(i.id).catch(() => undefined);
      setTimeout(pump, 0);
    },
    remove: (key) => set((s) => ({ items: s.items.filter((i) => i.key !== key) })),
    clearFinished: () => set((s) => ({ items: s.items.filter((i) => isActive(i.state) || i.state === 'paused' || i.state === 'error') })),
  };
});

if (typeof window !== 'undefined') {
  window.addEventListener('beforeunload', (e) => {
    if (useUploads.getState().items.some((i) => isActive(i.state))) {
      e.preventDefault();
      e.returnValue = '';
    }
  });
}
