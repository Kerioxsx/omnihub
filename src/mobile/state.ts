// App-wide state (zustand). Nothing secret is persisted here: the vault
// session lives in memory only (see screens/Vault.tsx).

import { create } from 'zustand';
import type { InboxItem, PendingPower } from '@shared/types';
import { client, type ServerInfo, type SocketStatus } from './client';

export type Tab = 'home' | 'files' | 'screen' | 'power' | 'more';
export const TAB_ORDER: Tab[] = ['home', 'files', 'screen', 'power', 'more'];

interface AppState {
  info: ServerInfo | null;
  socket: SocketStatus;
  online: boolean;
  tab: Tab;
  /** Previous tab, for the slide direction of the transition. */
  prevTab: Tab;
  setInfo: (i: ServerInfo | null) => void;
  setSocket: (s: SocketStatus) => void;
  setOnline: (o: boolean) => void;
  setTab: (t: Tab) => void;
  refreshInfo: () => Promise<void>;
}

export const useApp = create<AppState>((set, get) => ({
  info: null,
  socket: 'connecting',
  online: typeof navigator === 'undefined' ? true : navigator.onLine,
  tab: 'home',
  prevTab: 'home',
  setInfo: (info) => set({ info }),
  setSocket: (socket) => set({ socket }),
  setOnline: (online) => set({ online }),
  setTab: (tab) => {
    if (tab !== get().tab) set({ prevTab: get().tab, tab });
  },
  refreshInfo: async () => {
    try {
      set({ info: await client.info() });
    } catch {
      /* keep the last known info */
    }
  },
}));

// ---------- toasts ----------

export interface Toast {
  id: number;
  kind: 'success' | 'error' | 'info';
  title: string;
  body?: string;
  action?: { label: string; run: () => void };
  duration: number;
}

interface ToastState {
  toasts: Toast[];
  push: (t: Omit<Toast, 'id' | 'duration'> & { duration?: number }) => number;
  dismiss: (id: number) => void;
}

let toastSeq = 0;
export const useToasts = create<ToastState>((set) => ({
  toasts: [],
  push: (t) => {
    const id = ++toastSeq;
    const toast: Toast = { duration: t.kind === 'error' ? 5000 : 3200, ...t, id };
    set((s) => ({ toasts: [...s.toasts.slice(-2), toast] }));
    if (toast.duration > 0) setTimeout(() => useToasts.getState().dismiss(id), toast.duration);
    return id;
  },
  dismiss: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));

export const toast = {
  success: (title: string, body?: string, action?: Toast['action']) => useToasts.getState().push({ kind: 'success', title, body, action }),
  error: (title: string, body?: string, action?: Toast['action']) => useToasts.getState().push({ kind: 'error', title, body, action }),
  info: (title: string, body?: string, action?: Toast['action']) => useToasts.getState().push({ kind: 'info', title, body, action }),
};

// ---------- power ----------

interface PowerState {
  pending: PendingPower | null;
  /** PC clock minus phone clock, in seconds (estimated from our own requests). */
  skew: number;
  /** Set while this phone's own cancel request is in flight (avoids a duplicate toast). */
  cancelling: boolean;
  setPending: (p: PendingPower | null) => void;
  setSkew: (s: number) => void;
  setCancelling: (c: boolean) => void;
}

/** Lock and display off run immediately; only the others get a countdown. */
export const hasCountdown = (p: PendingPower | null) => !!p && p.action !== 'lock' && p.action !== 'displayOff';

export const usePower = create<PowerState>((set) => ({
  pending: null,
  skew: 0,
  cancelling: false,
  setPending: (pending) => set({ pending: hasCountdown(pending) ? pending : null }),
  setSkew: (skew) => set({ skew }),
  setCancelling: (cancelling) => set({ cancelling }),
}));

/** Seconds until the pending action fires, on the phone's clock. */
export function secondsLeft(p: PendingPower, skew: number, now = Date.now()): number {
  return p.at - skew - now / 1000;
}

// ---------- inbox (files the PC sent) ----------

interface InboxState {
  items: InboxItem[];
  loaded: boolean;
  error: string | null;
  load: () => Promise<void>;
  add: (i: InboxItem) => void;
  remove: (id: string) => void;
}

export const useInbox = create<InboxState>((set, get) => ({
  items: [],
  loaded: false,
  error: null,
  load: async () => {
    try {
      const { items } = await client.inbox();
      set({ items, loaded: true, error: null });
    } catch (e) {
      set({ loaded: true, error: e instanceof Error ? e.message : String(e) });
    }
  },
  add: (i) => {
    if (get().items.some((x) => x.id === i.id)) return;
    set({ items: [i, ...get().items] });
  },
  remove: (id) => set({ items: get().items.filter((i) => i.id !== id) }),
}));
