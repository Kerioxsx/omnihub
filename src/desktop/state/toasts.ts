import { create } from 'zustand';

export type ToastTone = 'success' | 'error' | 'info' | 'warn';

export interface Toast {
  id: number;
  tone: ToastTone;
  title: string;
  description?: string;
  action?: { label: string; run: () => void };
  /** Toasts with the same key replace each other (e.g. "Saved"). */
  key?: string;
  duration: number;
  /** Bumped when a keyed toast is refreshed, to restart its timer. */
  stamp: number;
}

interface ToastState {
  toasts: Toast[];
  push: (t: Omit<Toast, 'id' | 'stamp' | 'duration'> & { duration?: number }) => number;
  dismiss: (id: number) => void;
}

let seq = 0;

export const useToasts = create<ToastState>((set, get) => ({
  toasts: [],
  push: (t) => {
    const duration = t.duration ?? (t.tone === 'error' ? 6500 : 3800);
    const existing = t.key ? get().toasts.find((x) => x.key === t.key) : undefined;
    if (existing) {
      set({ toasts: get().toasts.map((x) => (x.id === existing.id ? { ...x, ...t, duration, stamp: Date.now() } : x)) });
      return existing.id;
    }
    const id = ++seq;
    set({ toasts: [...get().toasts.slice(-4), { ...t, id, duration, stamp: Date.now() }] });
    return id;
  },
  dismiss: (id) => set({ toasts: get().toasts.filter((x) => x.id !== id) }),
}));

type Extra = Partial<Pick<Toast, 'description' | 'action' | 'key' | 'duration'>>;

const push = (tone: ToastTone) => (title: string, description?: string, extra: Extra = {}) => useToasts.getState().push({ tone, title, description, ...extra });

export const toast = {
  success: push('success'),
  error: push('error'),
  info: push('info'),
  warn: push('warn'),
  saved: () => useToasts.getState().push({ tone: 'success', title: 'Saved', key: 'saved', duration: 1600 }),
};
