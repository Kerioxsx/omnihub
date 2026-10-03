// Keep the screen on (while uploading, while viewing the PC screen) where
// the Screen Wake Lock API exists. The lock is dropped by the browser when
// the page is hidden, so it is re-acquired when it becomes visible again.

type Sentinel = { release: () => Promise<void>; released?: boolean; addEventListener?: (t: string, f: () => void) => void };

const reasons = new Set<string>();
let sentinel: Sentinel | null = null;
let pending = false;

function supported(): boolean {
  return typeof navigator !== 'undefined' && 'wakeLock' in navigator;
}

async function acquire() {
  if (!supported() || sentinel || pending || document.visibilityState !== 'visible') return;
  pending = true;
  try {
    const s = (await (navigator as unknown as { wakeLock: { request: (t: 'screen') => Promise<Sentinel> } }).wakeLock.request('screen')) as Sentinel;
    sentinel = s;
    s.addEventListener?.('release', () => {
      if (sentinel === s) sentinel = null;
    });
    if (!reasons.size) release();
  } catch {
    sentinel = null;
  } finally {
    pending = false;
  }
}

function release() {
  const s = sentinel;
  sentinel = null;
  s?.release().catch(() => undefined);
}

export function keepAwake(reason: string, on: boolean) {
  if (on) reasons.add(reason);
  else reasons.delete(reason);
  if (reasons.size) acquire();
  else release();
}

if (typeof document !== 'undefined') {
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible' && reasons.size) acquire();
  });
}
