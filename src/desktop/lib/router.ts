// A tiny hash router: `#/storage?tab=cleanup`.

import { useSyncExternalStore } from 'react';

export type RouteId = 'home' | 'storage' | 'apps' | 'screenshots' | 'notes' | 'vault' | 'phone' | 'screen' | 'settings' | 'overlay';

export const PAGE_IDS: readonly RouteId[] = ['home', 'storage', 'apps', 'screenshots', 'notes', 'vault', 'phone', 'screen', 'settings'];

export interface Route {
  id: RouteId;
  params: URLSearchParams;
}

function parse(hash: string): Route {
  const raw = hash.replace(/^#/, '').replace(/^\/+/, '');
  const [path, query = ''] = raw.split('?');
  const seg = (path.split('/')[0] || 'home').toLowerCase();
  const id = (seg === 'overlay' || PAGE_IDS.includes(seg as RouteId) ? seg : 'home') as RouteId;
  return { id, params: new URLSearchParams(query) };
}

let current = parse(typeof window !== 'undefined' ? window.location.hash : '');
const listeners = new Set<() => void>();

if (typeof window !== 'undefined') {
  window.addEventListener('hashchange', () => {
    current = parse(window.location.hash);
    listeners.forEach((l) => l());
  });
}

function subscribe(l: () => void) {
  listeners.add(l);
  return () => {
    listeners.delete(l);
  };
}

export function useRoute(): Route {
  return useSyncExternalStore(
    subscribe,
    () => current,
    () => current,
  );
}

export function currentRoute(): Route {
  return current;
}

/** Navigate to a page, optionally with query params. Accepts "storage", "/storage", "#/storage?x=1". */
export function navigate(to: string, params?: Record<string, string>): void {
  const clean = to.replace(/^#/, '').replace(/^\/+/, '');
  const [path, query] = clean.split('?');
  const qs = new URLSearchParams(query);
  if (params) for (const [k, v] of Object.entries(params)) qs.set(k, v);
  const q = qs.toString();
  const next = `#/${path}${q ? `?${q}` : ''}`;
  if (window.location.hash !== next) window.location.hash = next;
}
