// Formatting helpers shared by the desktop UI and the phone app.

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB', 'PB'];

/** 1536 → "1.5 KB" (binary units, like Explorer). */
export function formatBytes(bytes: number | null | undefined, digits = 1): string {
  if (bytes == null || !Number.isFinite(bytes)) return '—';
  if (bytes < 1024) return `${bytes} B`;
  let v = bytes;
  let u = 0;
  while (v >= 1024 && u < UNITS.length - 1) {
    v /= 1024;
    u++;
  }
  return `${v >= 100 ? v.toFixed(0) : v.toFixed(digits)} ${UNITS[u]}`;
}

export function formatNumber(n: number): string {
  return new Intl.NumberFormat().format(n);
}

export function formatPercent(fraction: number, digits = 1): string {
  return `${(fraction * 100).toFixed(digits)}%`;
}

/** Duration in ms → "850 ms", "4.2 s", "3 min 05 s". */
export function formatDuration(ms: number): string {
  if (ms < 1000) return `${Math.round(ms)} ms`;
  const s = ms / 1000;
  if (s < 60) return `${s.toFixed(1)} s`;
  const m = Math.floor(s / 60);
  return `${m} min ${String(Math.round(s % 60)).padStart(2, '0')} s`;
}

/** Unix seconds → "just now", "5 min ago", "yesterday", "12 Mar 2024". */
export function formatRelative(unixSeconds: number | null | undefined): string {
  if (!unixSeconds) return '—';
  const diff = Date.now() / 1000 - unixSeconds;
  if (diff < 45) return 'just now';
  if (diff < 3600) return `${Math.round(diff / 60)} min ago`;
  if (diff < 86400) return `${Math.round(diff / 3600)} h ago`;
  if (diff < 172800) return 'yesterday';
  if (diff < 86400 * 30) return `${Math.round(diff / 86400)} days ago`;
  return formatDate(unixSeconds);
}

export function formatDate(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' });
}

export function formatDateTime(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleString(undefined, { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' });
}

/** Last path component, for both separators. */
export function basename(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

/** Stable colour for a file extension (used by the treemap and charts). */
export function extColor(ext: string | null | undefined, isDir = false): string {
  if (isDir) return 'var(--tm-dir)';
  if (!ext) return 'var(--tm-other)';
  const groups: Record<string, string[]> = {
    video: ['mp4', 'mkv', 'mov', 'avi', 'webm', 'wmv', 'm4v', 'flv'],
    image: ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'heic', 'heif', 'avif', 'raw', 'cr2', 'cr3', 'nef', 'arw', 'dng', 'orf', 'rw2', 'raf', 'psd', 'tif', 'tiff', 'svg'],
    audio: ['mp3', 'flac', 'wav', 'ogg', 'm4a', 'aac', 'wma', 'opus'],
    archive: ['zip', '7z', 'rar', 'tar', 'gz', 'xz', 'bz2', 'iso', 'img', 'vhd', 'vhdx', 'wim', 'cab'],
    code: ['js', 'ts', 'tsx', 'rs', 'py', 'c', 'cpp', 'h', 'cs', 'java', 'go', 'json', 'xml', 'html', 'css'],
    doc: ['pdf', 'doc', 'docx', 'xls', 'xlsx', 'ppt', 'pptx', 'txt', 'md', 'odt', 'epub'],
    exe: ['exe', 'dll', 'sys', 'msi', 'msix', 'appx', 'pak', 'bin', 'dat', 'so'],
    data: ['db', 'sqlite', 'log', 'tmp', 'cache', 'etl', 'evtx', 'dmp', 'pf'],
  };
  for (const [g, exts] of Object.entries(groups)) if (exts.includes(ext)) return `var(--tm-${g})`;
  return 'var(--tm-other)';
}
