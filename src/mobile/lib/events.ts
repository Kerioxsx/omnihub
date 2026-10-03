// In-page bus for the live events coming from the PC's event socket.

import { useEffect, useRef } from 'react';

const bus = new EventTarget();

export function emitEvent(topic: string, payload: unknown) {
  bus.dispatchEvent(new CustomEvent(topic, { detail: payload }));
}

export function useEvent<T = any>(topic: string, fn: (payload: T) => void) {
  const ref = useRef(fn);
  ref.current = fn;
  useEffect(() => {
    const on = (e: Event) => ref.current((e as CustomEvent).detail as T);
    bus.addEventListener(topic, on);
    return () => bus.removeEventListener(topic, on);
  }, [topic]);
}
