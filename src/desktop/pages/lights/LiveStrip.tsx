// A light's colours as it shows them now: one cell per segment, from the
// `govee:frame` events (only this strip redraws).

import { useEffect, useState } from 'react';
import { on } from '../../api';
import { cx } from '../../lib/cx';

export function LiveStrip({ id, fallback, className }: { id: string; fallback?: string[]; className?: string }) {
  const [colors, setColors] = useState<string[] | null>(null);
  useEffect(() => {
    let alive = true;
    const off = on<{ lights: Record<string, string[]> }>('govee:frame', (f) => {
      const c = f.lights[id];
      if (alive && c?.length) setColors(c);
    });
    return () => {
      alive = false;
      void off.then((u) => u());
    };
  }, [id]);
  const shown = colors ?? fallback ?? ['#1f2230'];
  const glow = shown[Math.floor(shown.length / 2)] ?? '#000000';
  return (
    <div className={cx('flex h-3 overflow-hidden rounded-full ring-1 ring-white/10', className)} style={{ boxShadow: `0 0 18px -4px ${glow}` }} aria-hidden>
      {shown.map((c, i) => (
        <div key={i} className="h-full flex-1 transition-[background-color] duration-100 ease-linear" style={{ backgroundColor: c }} />
      ))}
    </div>
  );
}
