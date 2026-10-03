// Live state fed by backend events: companion server, phones, vault lock
// state, pending power actions and file transfers.

import { create } from 'zustand';
import type { Device, PendingPower, ServerStatus, TransferEvent, VaultStatus, ViewerInfo } from '@shared/types';
import { api } from '../api';

export interface Transfer {
  id: string;
  direction: 'upload' | 'download';
  name: string;
  size: number;
  done: number;
  device: string | null;
  path: string | null;
  state: 'running' | 'done' | 'cancelled';
  startedAt: number;
  updatedAt: number;
  /** bytes/second, smoothed */
  speed: number;
}

interface LiveState {
  remote: ServerStatus | null;
  devices: Device[];
  viewers: ViewerInfo[];
  vault: VaultStatus | null;
  power: PendingPower | null;
  transfers: Transfer[];
  refreshRemote: () => Promise<void>;
  refreshDevices: () => Promise<void>;
  refreshVault: () => Promise<void>;
  refreshPower: () => Promise<void>;
  setRemote: (s: ServerStatus) => void;
  setDevices: (d: Device[]) => void;
  setViewers: (v: ViewerInfo[]) => void;
  setPower: (p: PendingPower | null) => void;
  transferEvent: (kind: 'started' | 'progress' | 'done' | 'cancelled', e: TransferEvent) => void;
  clearFinished: () => void;
}

export const useLive = create<LiveState>((set, get) => ({
  remote: null,
  devices: [],
  viewers: [],
  vault: null,
  power: null,
  transfers: [],
  refreshRemote: async () => {
    const remote = await api.remote.status();
    set({ remote, viewers: remote.viewers });
  },
  refreshDevices: async () => set({ devices: await api.remote.devices() }),
  refreshVault: async () => set({ vault: await api.vault.status() }),
  refreshPower: async () => set({ power: await api.power.pending() }),
  setRemote: (remote) => set({ remote, viewers: remote.viewers }),
  setDevices: (devices) => set({ devices }),
  setViewers: (viewers) => set({ viewers }),
  setPower: (power) => set({ power }),
  transferEvent: (kind, e) => {
    const now = Date.now();
    const list = [...get().transfers];
    const i = list.findIndex((t) => t.id === e.id);
    const prev = i >= 0 ? list[i] : null;
    const size = e.size ?? prev?.size ?? 0;
    const done = kind === 'done' ? size : (e.done ?? prev?.done ?? 0);
    let speed = prev?.speed ?? 0;
    if (prev && now > prev.updatedAt) {
      const inst = ((done - prev.done) * 1000) / (now - prev.updatedAt);
      speed = speed ? speed * 0.6 + inst * 0.4 : inst;
    }
    const next: Transfer = {
      id: e.id,
      direction: e.direction,
      name: e.name ?? prev?.name ?? '',
      size,
      done,
      device: e.device ?? prev?.device ?? null,
      path: e.path ?? prev?.path ?? null,
      state: kind === 'done' ? 'done' : kind === 'cancelled' ? 'cancelled' : 'running',
      startedAt: prev?.startedAt ?? now,
      updatedAt: now,
      speed: kind === 'done' ? (done * 1000) / Math.max(1, now - (prev?.startedAt ?? now)) : speed,
    };
    if (i >= 0) list[i] = next;
    else list.unshift(next);
    set({ transfers: list.slice(0, 30) });
  },
  clearFinished: () => set({ transfers: get().transfers.filter((t) => t.state === 'running') }),
}));

/** Phones seen in the last 10 minutes while the server is running. */
export function connectedCount(remote: ServerStatus | null, devices: Device[], viewers: ViewerInfo[]): number {
  if (!remote?.running) return 0;
  const cutoff = Date.now() / 1000 - 600;
  const names = new Set(devices.filter((d) => !d.revoked && d.lastSeen >= cutoff).map((d) => d.name));
  viewers.forEach((v) => names.add(v.device));
  return names.size;
}
