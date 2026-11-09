// The PC's photo wallpaper: one shared copy of its address and colours,
// refreshed when it changes (here or from another window).

import type { WallInfo } from '@shared/palette';
import { useSyncExternalStore } from 'react';
import { api, on } from '../api';

export type PcWall = WallInfo & { url: string };

let wall: PcWall | null | undefined;
const subs = new Set<() => void>();
let started = false;

export function refreshWallpaper() {
  void api.wallpaper.get('pc').then(
    (w) => {
      wall = w;
      subs.forEach((s) => s());
    },
    () => undefined,
  );
}

function start() {
  if (started) return;
  started = true;
  refreshWallpaper();
  void on<{ slot: string }>('wallpaper:changed', (p) => p.slot === 'pc' && refreshWallpaper());
}

/** The photo wallpaper (undefined while loading, null when there is none). */
export function useWallpaper(): PcWall | null | undefined {
  start();
  return useSyncExternalStore(
    (s) => {
      subs.add(s);
      return () => subs.delete(s);
    },
    () => wall,
  );
}

export function setWallpaperLocal(w: PcWall | null) {
  wall = w;
  subs.forEach((s) => s());
}
