// Theme preference (system / dark / light), kept in localStorage.

export type ThemePref = 'system' | 'dark' | 'light';
const KEY = 'omnihub.theme';

export function getThemePref(): ThemePref {
  try {
    const v = localStorage.getItem(KEY);
    if (v === 'dark' || v === 'light' || v === 'system') return v;
  } catch {
    /* private mode */
  }
  return 'system';
}

export function setThemePref(p: ThemePref) {
  try {
    localStorage.setItem(KEY, p);
  } catch {
    /* ignore */
  }
  applyTheme(p);
}

const media = typeof window !== 'undefined' && window.matchMedia ? window.matchMedia('(prefers-color-scheme: light)') : null;

export function applyTheme(p: ThemePref = getThemePref()) {
  const light = p === 'light' || (p === 'system' && !!media?.matches);
  const root = document.documentElement;
  root.classList.toggle('light', light);
  root.style.colorScheme = light ? 'light' : 'dark';
  const meta = document.querySelector('meta[name="theme-color"]');
  meta?.setAttribute('content', light ? '#f5f5fa' : '#0b0b12');
}

export function watchSystemTheme() {
  media?.addEventListener?.('change', () => {
    if (getThemePref() === 'system') applyTheme('system');
  });
}
