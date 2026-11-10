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

// ---------- how the photo is shown ----------

export interface PhotoPrefs {
  /** Darkening over the photo, 0–70 (%). */
  dim: number;
  /** Blur, 0–24 px. */
  blur: number;
  /** A slow drift (off with Reduce motion). */
  drift: boolean;
  /** Change to the next gallery photo every N minutes (0 = never). */
  rotate: number;
}
const PREFS_KEY = 'omnihub.photoLook';
const DEFAULT_PREFS: PhotoPrefs = { dim: 30, blur: 0, drift: false, rotate: 0 };
let photo: PhotoPrefs = (() => {
  try {
    return { ...DEFAULT_PREFS, ...JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}') };
  } catch {
    return DEFAULT_PREFS;
  }
})();
const photoSubs = new Set<() => void>();

export function setPhotoPrefs(patch: Partial<PhotoPrefs>) {
  photo = { ...photo, ...patch };
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(photo));
  } catch {
    /* this session only */
  }
  photoSubs.forEach((s) => s());
}

export function usePhotoPrefs(): PhotoPrefs {
  return useSyncExternalStore(
    (s) => {
      photoSubs.add(s);
      return () => photoSubs.delete(s);
    },
    () => photo,
  );
}

const ROTATED_KEY = 'omnihub.photoRotatedAt';
/** Move on to the next gallery photo when it's time (call now and then while the photo look is on). */
export async function rotateIfDue() {
  if (!photo.rotate) return;
  const last = Number(localStorage.getItem(ROTATED_KEY) ?? 0);
  if (Date.now() - last < photo.rotate * 60_000) return;
  localStorage.setItem(ROTATED_KEY, String(Date.now()));
  if (!last) return; // start counting from now
  const list = await api.wallpaper.gallery().catch(() => []);
  if (list.length < 2) return;
  const at = list.findIndex((g) => g.source === wall?.source);
  const next = list[(at + 1) % list.length];
  if (next?.source) setWallpaperLocal(await api.wallpaper.select(next.source).catch(() => wall ?? null));
}

