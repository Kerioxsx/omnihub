// In-memory caches for screenshot thumbnails and app icons, loaded lazily
// when they scroll into view.

import { useEffect, useState } from 'react';
import { api } from '../api';
import { useInView } from './hooks';

const thumbs = new Map<string, string>();
const thumbLoads = new Map<string, Promise<string>>();
const icons = new Map<string, string | null>();
const iconLoads = new Map<string, Promise<string | null>>();

function loadThumb(id: string): Promise<string> {
  let p = thumbLoads.get(id);
  if (!p) {
    p = api.shots.thumb(id).then((url) => {
      thumbs.set(id, url);
      return url;
    });
    p.catch(() => thumbLoads.delete(id));
    thumbLoads.set(id, p);
  }
  return p;
}

function loadIcon(id: string): Promise<string | null> {
  let p = iconLoads.get(id);
  if (!p) {
    p = api.apps.icon(id).then(
      (url) => {
        icons.set(id, url);
        return url;
      },
      () => {
        icons.set(id, null);
        return null;
      },
    );
    iconLoads.set(id, p);
  }
  return p;
}

/** Thumbnail data URL for a screenshot, fetched once it is near the viewport. */
export function useThumb(id: string): [(el: HTMLElement | null) => void, string | null] {
  const [ref, visible] = useInView<HTMLElement>('300px');
  const [url, setUrl] = useState<string | null>(() => thumbs.get(id) ?? null);
  useEffect(() => {
    if (!visible || url) return;
    let alive = true;
    void loadThumb(id).then((u) => alive && setUrl(u), () => undefined);
    return () => {
      alive = false;
    };
  }, [id, visible, url]);
  return [ref, url];
}

/** App icon (null while loading or when the app has none). */
export function useAppIcon(id: string): [(el: HTMLElement | null) => void, string | null, boolean] {
  const [ref, visible] = useInView<HTMLElement>('200px');
  const [state, setState] = useState<{ url: string | null; done: boolean }>(() => (icons.has(id) ? { url: icons.get(id) ?? null, done: true } : { url: null, done: false }));
  useEffect(() => {
    if (!visible || state.done) return;
    let alive = true;
    void loadIcon(id).then((u) => alive && setState({ url: u, done: true }));
    return () => {
      alive = false;
    };
  }, [id, visible, state.done]);
  return [ref, state.url, state.done];
}

export function forgetThumb(id: string): void {
  thumbs.delete(id);
  thumbLoads.delete(id);
}
