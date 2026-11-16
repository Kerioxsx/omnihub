// The soundboard's hidden player window: plays sounds as they're asked for,
// to the output you picked (when this window can see it).

import { useEffect, useRef } from 'react';
import { useEvent } from '../../lib/hooks';

type WithSink = HTMLAudioElement & { setSinkId?: (id: string) => Promise<void> };

async function sinkFor(name: string): Promise<string | null> {
  if (!name) return null;
  try {
    const devs = await navigator.mediaDevices.enumerateDevices();
    return devs.find((d) => d.kind === 'audiooutput' && d.label === name)?.deviceId ?? null;
  } catch {
    return null;
  }
}

export function SoundboardPlayer() {
  const playing = useRef(new Set<HTMLAudioElement>());
  useEffect(() => {
    document.title = 'OmniHub soundboard';
  }, []);
  useEvent<{ url: string; volume: number; output: string }>('soundboard:play', (p) => {
    const a = new Audio(p.url) as WithSink;
    a.volume = Math.max(0, Math.min(1, (p.volume || 100) / 100));
    playing.current.add(a);
    a.onended = () => playing.current.delete(a);
    void sinkFor(p.output).then(async (id) => {
      if (id && a.setSinkId) await a.setSinkId(id).catch(() => undefined);
      await a.play().catch(() => undefined);
    });
  });
  useEvent('soundboard:stop', () => {
    playing.current.forEach((a) => a.pause());
    playing.current.clear();
  });
  return null;
}
