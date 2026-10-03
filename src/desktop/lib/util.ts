// Misc helpers for the desktop UI.

import DOMPurify from 'dompurify';
import { marked } from 'marked';
import { inTauri } from '../api';

marked.setOptions({ gfm: true, breaks: false });

/** Markdown → sanitized HTML. */
export function renderMarkdown(md: string): string {
  const html = marked.parse(md, { async: false });
  return DOMPurify.sanitize(html, { USE_PROFILES: { html: true } });
}

export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    try {
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.position = 'fixed';
      ta.style.opacity = '0';
      document.body.appendChild(ta);
      ta.select();
      const ok = document.execCommand('copy');
      ta.remove();
      return ok;
    } catch {
      return false;
    }
  }
}

/** Close the current window (Tauri) or fall back to history/back in a browser. */
export async function closeCurrentWindow(fallbackHash = '#/screenshots'): Promise<void> {
  if (inTauri) {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().close();
      return;
    } catch {
      /* fall through */
    }
  }
  window.location.hash = fallbackHash;
}

/** "02:05" style countdown from seconds. */
export function formatCountdown(seconds: number): string {
  const s = Math.max(0, Math.ceil(seconds));
  const m = Math.floor(s / 60);
  return `${m}:${String(s % 60).padStart(2, '0')}`;
}

export function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

export function greeting(date = new Date()): string {
  const h = date.getHours();
  if (h < 5) return 'Good night';
  if (h < 12) return 'Good morning';
  if (h < 18) return 'Good afternoon';
  return 'Good evening';
}

/** "Today", "Yesterday", "Monday", "28 Sep" for grouping by day. */
export function dayLabel(unixSeconds: number): string {
  const d = new Date(unixSeconds * 1000);
  const today = new Date();
  const start = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = Math.round((start(today) - start(d)) / 86400000);
  if (diff === 0) return 'Today';
  if (diff === 1) return 'Yesterday';
  if (diff < 7) return d.toLocaleDateString(undefined, { weekday: 'long' });
  return d.toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short' });
}

export function dayKey(unixSeconds: number): string {
  const d = new Date(unixSeconds * 1000);
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
}

export function clamp(v: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, v));
}

/** Short host + path for showing URLs. */
export function prettyUrl(url: string): string {
  try {
    const u = new URL(url);
    return u.host + (u.pathname === '/' ? '' : u.pathname);
  } catch {
    return url;
  }
}

export function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

/** Merge a JSON merge patch into a copy of `target`. */
export function mergeDeep<T>(target: T, patch: unknown): T {
  if (!isRecord(patch) || !isRecord(target)) return (patch === undefined ? target : patch) as T;
  const out: Record<string, unknown> = { ...target };
  for (const [k, v] of Object.entries(patch)) {
    out[k] = isRecord(v) && isRecord(out[k]) ? mergeDeep(out[k], v) : v;
  }
  return out as T;
}

export const isMac = typeof navigator !== 'undefined' && /Mac/.test(navigator.platform);
export const MOD_LABEL = isMac ? '⌘' : 'Ctrl';

/** Plain-text preview of Markdown (for list snippets). */
export function plainSnippet(md: string, max = 160): string {
  return md
    .replace(/```[\s\S]*?```/g, ' ')
    .replace(/^\s{0,3}#{1,6}\s+/gm, '')
    .replace(/^\s*[-*+]\s+\[[ xX]\]\s+/gm, '')
    .replace(/^\s*([-*+]|\d+\.)\s+/gm, '')
    .replace(/^\s*>\s?/gm, '')
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/[*_~`|]/g, '')
    .replace(/^-{3,}$/gm, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, max);
}
