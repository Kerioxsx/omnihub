// Storage page state shared with the sidebar badge, Home and the command palette.

import { create } from 'zustand';
import type { JobProgress, ScanSummary, VolumeInfo } from '@shared/types';
import { api, errorText } from '../api';
import { toast } from './toasts';
import { formatBytes, formatDuration } from '@shared/format';

export type StorageTab = 'explorer' | 'largest' | 'types' | 'cleanup' | 'duplicates' | 'search';
export type ScanModeChoice = 'fast' | 'standard';

export function rootKey(root: string): string {
  const r = root.replace(/\//g, '\\');
  return (/^[a-z]:\\?$/i.test(r) ? `${r.slice(0, 2)}\\` : r.replace(/\\+$/, '')).toLowerCase();
}

interface StorageState {
  volumes: VolumeInfo[];
  volumesLoaded: boolean;
  volumesError: string | null;
  root: string | null;
  mode: ScanModeChoice;
  summaries: Record<string, ScanSummary>;
  cacheChecked: Record<string, boolean>;
  checkingCache: boolean;
  job: JobProgress | null;
  failure: { root: string; error: string } | null;
  nodeByScan: Record<string, number>;
  tab: StorageTab;
  /** Bumped after deletions so open views refetch. */
  version: number;

  loadVolumes: () => Promise<void>;
  select: (root: string) => Promise<void>;
  setMode: (m: ScanModeChoice) => void;
  scan: (root?: string, mode?: ScanModeChoice) => Promise<void>;
  cancel: () => Promise<void>;
  setNode: (scanId: string, node: number) => void;
  setTab: (t: StorageTab) => void;
  bump: () => void;
  onProgress: (p: JobProgress) => void;
  onDone: (jobId: string, summary: ScanSummary) => void;
  onFailed: (jobId: string, error: string, cancelled: boolean) => void;
}

export const useStorage = create<StorageState>((set, get) => ({
  volumes: [],
  volumesLoaded: false,
  volumesError: null,
  root: null,
  mode: 'fast',
  summaries: {},
  cacheChecked: {},
  checkingCache: false,
  job: null,
  failure: null,
  nodeByScan: {},
  tab: 'explorer',
  version: 0,

  loadVolumes: async () => {
    try {
      const volumes = await api.storage.volumes();
      set({ volumes, volumesLoaded: true, volumesError: null });
    } catch (e) {
      set({ volumesError: errorText(e), volumesLoaded: true });
    }
  },

  select: async (root) => {
    const key = rootKey(root);
    set({ root, failure: null });
    const { summaries, cacheChecked } = get();
    if (summaries[key] || cacheChecked[key]) return;
    set({ checkingCache: true });
    try {
      const cached = await api.storage.openCached(root);
      set((s) => ({ summaries: cached ? { ...s.summaries, [key]: cached } : s.summaries, cacheChecked: { ...s.cacheChecked, [key]: true } }));
    } catch {
      set((s) => ({ cacheChecked: { ...s.cacheChecked, [key]: true } }));
    } finally {
      set({ checkingCache: false });
    }
  },

  setMode: (mode) => set({ mode }),

  scan: async (rootArg, modeArg) => {
    const root = rootArg ?? get().root;
    if (!root) return;
    if (get().job?.state === 'running') {
      toast.warn('A scan is already running', 'Cancel it first or wait for it to finish.');
      return;
    }
    const vol = get().volumes.find((v) => rootKey(v.root) === rootKey(root));
    const isVolume = !!vol;
    const mode = modeArg ?? get().mode;
    const effective = mode === 'fast' && isVolume && vol.mftCapable ? 'fast' : 'standard';
    set({ root, failure: null });
    try {
      const jobId = await api.storage.scan({ root, mode: effective, exclude: [] });
      set((s) => ({
        job: s.job?.jobId === jobId ? s.job : { jobId, root, method: '', phase: 'starting', state: 'running', recordsDone: 0, recordsTotal: 0, files: 0, dirs: 0, bytes: 0, errors: 0, current: '', elapsedMs: 0, scanId: null, error: null },
      }));
    } catch (e) {
      set({ failure: { root, error: errorText(e) } });
      toast.error('Could not start the scan', errorText(e));
    }
  },

  cancel: async () => {
    const job = get().job;
    if (!job) return;
    try {
      await api.storage.cancel(job.jobId);
    } catch (e) {
      toast.error('Could not cancel', errorText(e));
    }
  },

  setNode: (scanId, node) => set((s) => ({ nodeByScan: { ...s.nodeByScan, [scanId]: node } })),
  setTab: (tab) => set({ tab }),
  bump: () => set((s) => ({ version: s.version + 1 })),

  onProgress: (p) => {
    const job = get().job;
    if (job && job.jobId !== p.jobId) return;
    if (p.state === 'running') set({ job: p });
  },

  onDone: (jobId, summary) => {
    const job = get().job;
    if (job && job.jobId !== jobId) return;
    const key = rootKey(summary.root);
    set((s) => ({ job: null, summaries: { ...s.summaries, [key]: summary }, nodeByScan: { ...s.nodeByScan, [summary.scanId]: 0 }, cacheChecked: { ...s.cacheChecked, [key]: true }, version: s.version + 1 }));
    void get().loadVolumes();
    const how = summary.method === 'walk' ? 'Walked the folders' : 'Read the MFT';
    toast.success(`Scan of ${summary.root} finished`, `${formatBytes(summary.size)} · ${how} in ${formatDuration(summary.durationMs)}`);
  },

  onFailed: (jobId, error, cancelled) => {
    const job = get().job;
    if (job && job.jobId !== jobId) return;
    set({ job: null, failure: cancelled ? null : { root: job?.root ?? get().root ?? '', error } });
    if (cancelled) toast.info('Scan cancelled');
    else toast.error('Scan failed', error);
  },
}));

/** 0..1 progress estimate for the running job (MFT records, or bytes vs used space). */
export function jobFraction(job: JobProgress, volumes: VolumeInfo[]): number | null {
  if (job.phase === 'mft' && job.recordsTotal > 0) return Math.min(1, job.recordsDone / job.recordsTotal);
  if (job.phase === 'saving' || job.phase === 'done') return 1;
  if (job.phase === 'walk') {
    const vol = volumes.find((v) => rootKey(v.root) === rootKey(job.root));
    if (vol && vol.total > vol.free) return Math.min(0.99, job.bytes / (vol.total - vol.free));
    return null;
  }
  return null;
}
