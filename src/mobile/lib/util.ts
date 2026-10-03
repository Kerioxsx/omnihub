// Small helpers shared by the phone screens.

import { useEffect, useRef, useState } from 'react';
import { ApiError } from '../client';

export function cx(...parts: (string | false | null | undefined)[]): string {
  return parts.filter(Boolean).join(' ');
}

/** Haptic tick where supported (Android); silently ignored elsewhere. */
export function vibrate(pattern: number | number[]) {
  try {
    navigator.vibrate?.(pattern);
  } catch {
    /* unsupported */
  }
}

/** Human message for any thrown value (API, network, other). */
export function errorMessage(e: unknown): string {
  if (e instanceof ApiError) {
    const m = e.message || '';
    if (e.status === 429) return 'Too many attempts — wait a minute and try again.';
    if (e.status === 0) return "Can't reach the PC.";
    if (e.status === 502 || e.status === 503 || e.status === 504) return "The PC isn't answering right now.";
    return m.charAt(0).toUpperCase() + m.slice(1);
  }
  if (e instanceof TypeError) return "Can't reach the PC — check that you're on the same Wi-Fi.";
  if (e instanceof Error) return e.message;
  return String(e);
}

export function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d} d ${h} h`;
  if (h > 0) return `${h} h ${m} min`;
  return `${Math.max(1, m)} min`;
}

export function formatSpeed(bytesPerSec: number): string {
  if (!Number.isFinite(bytesPerSec) || bytesPerSec <= 0) return '';
  const mb = bytesPerSec / (1024 * 1024);
  if (mb >= 1) return `${mb >= 100 ? mb.toFixed(0) : mb.toFixed(1)} MB/s`;
  return `${Math.max(1, Math.round(bytesPerSec / 1024))} KB/s`;
}

export function formatEta(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return '';
  if (seconds < 1) return 'almost done';
  if (seconds < 60) return `${Math.ceil(seconds)} s left`;
  const m = Math.floor(seconds / 60);
  if (m < 60) return `${m} min ${String(Math.round(seconds % 60)).padStart(2, '0')} s left`;
  return `${Math.floor(m / 60)} h ${m % 60} min left`;
}

export function formatCountdown(seconds: number): string {
  const s = Math.max(0, Math.ceil(seconds));
  const m = Math.floor(s / 60);
  return `${m}:${String(s % 60).padStart(2, '0')}`;
}

/** Path separator used by a PC path (Windows or POSIX). */
export function sepOf(path: string): string {
  return path.includes('\\') && !path.startsWith('/') ? '\\' : '/';
}

export function greeting(): string {
  const h = new Date().getHours();
  if (h < 5) return 'Good night';
  if (h < 12) return 'Good morning';
  if (h < 18) return 'Good afternoon';
  return 'Good evening';
}

/** True while the page is visible. */
export function usePageVisible(): boolean {
  const [visible, setVisible] = useState(() => document.visibilityState !== 'hidden');
  useEffect(() => {
    const on = () => setVisible(document.visibilityState !== 'hidden');
    document.addEventListener('visibilitychange', on);
    return () => document.removeEventListener('visibilitychange', on);
  }, []);
  return visible;
}

export function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const t = setTimeout(() => setV(value), ms);
    return () => clearTimeout(t);
  }, [value, ms]);
  return v;
}

/** setInterval that always calls the latest callback; `null` pauses it. */
export function useInterval(fn: () => void, ms: number | null) {
  const ref = useRef(fn);
  ref.current = fn;
  useEffect(() => {
    if (ms == null) return;
    const t = setInterval(() => ref.current(), ms);
    return () => clearInterval(t);
  }, [ms]);
}

/** Re-render every `ms` (for countdowns and relative times). */
export function useNow(ms = 1000, enabled = true): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!enabled) return;
    setNow(Date.now());
    const t = setInterval(() => setNow(Date.now()), ms);
    return () => clearInterval(t);
  }, [ms, enabled]);
  return now;
}

/**
 * Bottom inset of the on-screen keyboard (iOS overlays it on the page; on
 * Android the viewport is resized and this stays 0).
 */
export function useKeyboardInset(): number {
  const [inset, setInset] = useState(0);
  useEffect(() => {
    const vv = window.visualViewport;
    if (!vv) return;
    const on = () => setInset(Math.max(0, Math.round(window.innerHeight - vv.height - vv.offsetTop)));
    vv.addEventListener('resize', on);
    vv.addEventListener('scroll', on);
    on();
    return () => {
      vv.removeEventListener('resize', on);
      vv.removeEventListener('scroll', on);
    };
  }, []);
  return inset;
}

/** Calls `onVisible` once the element scrolls near the viewport. */
export function useInView<T extends Element>(onVisible: () => void, rootMargin = '200px'): (el: T | null) => void {
  const cb = useRef(onVisible);
  cb.current = onVisible;
  const obs = useRef<IntersectionObserver | null>(null);
  useEffect(() => () => obs.current?.disconnect(), []);
  return (el: T | null) => {
    obs.current?.disconnect();
    obs.current = null;
    if (!el) return;
    if (!('IntersectionObserver' in window)) {
      cb.current();
      return;
    }
    obs.current = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          obs.current?.disconnect();
          obs.current = null;
          cb.current();
        }
      },
      { rootMargin },
    );
    obs.current.observe(el);
  };
}

/** Run async jobs with at most `n` at a time (thumbnails, app icons). */
export function limiter(n: number) {
  let active = 0;
  const queue: (() => void)[] = [];
  const next = () => {
    if (active >= n) return;
    const job = queue.shift();
    if (job) {
      active++;
      job();
    }
  };
  return function run<T>(fn: () => Promise<T>): Promise<T> {
    return new Promise<T>((resolve, reject) => {
      queue.push(() => {
        fn()
          .then(resolve, reject)
          .finally(() => {
            active--;
            next();
          });
      });
      next();
    });
  };
}
