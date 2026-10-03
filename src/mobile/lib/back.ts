// Android back button / back gesture support. Screens register handlers
// (close a sheet, go up a folder, leave a sub-page); the most recent one
// runs on "back". A single history entry is kept as a trap while any
// handler is registered, so the browser never leaves the app by accident.

import { useEffect, useRef } from 'react';

interface Entry {
  fn: () => void;
}

const stack: Entry[] = [];
let trapped = false;
let ignoreNextPop = false;
let syncQueued = false;

function sync() {
  syncQueued = false;
  if (stack.length && !trapped) {
    history.pushState({ omnihubTrap: true }, '');
    trapped = true;
  } else if (!stack.length && trapped) {
    trapped = false;
    ignoreNextPop = true;
    history.back();
  }
}

function queueSync() {
  if (syncQueued) return;
  syncQueued = true;
  setTimeout(sync, 0);
}

if (typeof window !== 'undefined') {
  window.addEventListener('popstate', () => {
    if (ignoreNextPop) {
      ignoreNextPop = false;
      return;
    }
    trapped = false;
    const top = stack[stack.length - 1];
    if (top) top.fn();
    queueSync();
  });
}

/** While `active`, the back button calls `onBack` instead of leaving. */
export function useBackHandler(active: boolean, onBack: () => void) {
  const ref = useRef(onBack);
  ref.current = onBack;
  useEffect(() => {
    if (!active) return;
    const entry: Entry = { fn: () => ref.current() };
    stack.push(entry);
    queueSync();
    return () => {
      const i = stack.indexOf(entry);
      if (i >= 0) stack.splice(i, 1);
      queueSync();
    };
  }, [active]);
}
