// The animated background behind the app for the current theme. Only
// transforms and opacity move, so the graphics card does the work; it
// pauses while the window is hidden and holds still with Reduce motion.

import type { Look } from '@shared/types';
import { useEffect, useMemo, useState } from 'react';
import { cx } from '../lib/cx';
import { lookInfo } from '../lib/looks';
import { useSettings } from '../state/settings';

/** The same layout every time (no jumping between renders). */
function seeded(seed: number) {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Stars as one layer of tiny gradients: cheap, and it scales with its box. */
function starField(count: number, seed: number): string {
  const r = seeded(seed);
  return Array.from({ length: count }, () => {
    const size = r() < 0.15 ? 2 : 1;
    return `radial-gradient(${size}px ${size}px at ${(r() * 100).toFixed(2)}% ${(r() * 100).toFixed(2)}%, rgba(255,255,255,${(0.45 + r() * 0.55).toFixed(2)}) 50%, transparent 51%)`;
  }).join(',');
}

/** Falling or rising bits: where, how big, how fast. */
function particles(count: number, seed: number, minDur: number, maxDur: number) {
  const r = seeded(seed);
  return Array.from({ length: count }, (_, i) => {
    const dur = minDur + r() * (maxDur - minDur);
    return {
      key: i,
      style: {
        '--x': `${(r() * 100).toFixed(1)}%`,
        '--s': (0.55 + r() * 0.75).toFixed(2),
        '--sway': `${(r() * 90 - 45).toFixed(0)}px`,
        '--dur': `${dur.toFixed(1)}s`,
        '--delay': `${(-r() * dur).toFixed(1)}s`,
      } as React.CSSProperties,
    };
  });
}

function useStill(): boolean {
  const animate = useSettings((s) => s.settings?.general.animateBackdrop ?? true);
  const reduced = useSettings((s) => s.settings?.general.reducedMotion ?? false);
  const [osReduced, setOsReduced] = useState(() => window.matchMedia('(prefers-reduced-motion: reduce)').matches);
  useEffect(() => {
    const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
    const on = () => setOsReduced(mq.matches);
    mq.addEventListener('change', on);
    return () => mq.removeEventListener('change', on);
  }, []);
  return !animate || reduced || osReduced;
}

function useHidden(): boolean {
  const [hidden, setHidden] = useState(() => document.visibilityState === 'hidden');
  useEffect(() => {
    const on = () => setHidden(document.visibilityState === 'hidden');
    document.addEventListener('visibilitychange', on);
    return () => document.removeEventListener('visibilitychange', on);
  }, []);
  return hidden;
}

function Layers({ look, mini }: { look: Look; mini: boolean }) {
  const stars = useMemo(() => (look === 'galaxy' ? [starField(mini ? 40 : 140, 7), starField(mini ? 25 : 90, 19)] : null), [look, mini]);
  const petals = useMemo(() => (look === 'sakura' ? particles(mini ? 7 : 22, 3, 14, 26) : []), [look, mini]);
  const embers = useMemo(() => (look === 'ember' ? particles(mini ? 10 : 30, 11, 8, 15) : []), [look, mini]);
  switch (look) {
    case 'aurora':
      return (
        <>
          <i className="bd-ribbon r1" />
          <i className="bd-ribbon r2" />
          <i className="bd-ribbon r3" />
          <i className="bd-veil" />
        </>
      );
    case 'synthwave':
      return (
        <>
          <i className="bd-sky" />
          <i className="bd-sun" />
          <i className="bd-floor">
            <i className="bd-grid" />
          </i>
        </>
      );
    case 'ocean':
      return (
        <>
          <i className="bd-deep" />
          {[1, 2, 3].map((n) => (
            <i key={n} className={`bd-wave w${n}`} />
          ))}
        </>
      );
    case 'galaxy':
      return (
        <>
          <i className="bd-nebula n1" />
          <i className="bd-nebula n2" />
          <i className="bd-stars" style={{ backgroundImage: stars?.[0] }} />
          <i className="bd-stars twinkle" style={{ backgroundImage: stars?.[1] }} />
          {!mini && <i className="bd-shoot" />}
        </>
      );
    case 'sakura':
      return (
        <>
          <i className="bd-blush" />
          {petals.map((p) => (
            <i key={p.key} className="bd-petal" style={p.style} />
          ))}
        </>
      );
    case 'ember':
      return (
        <>
          <i className="bd-heat" />
          {embers.map((p) => (
            <i key={p.key} className="bd-spark" style={p.style} />
          ))}
        </>
      );
    default:
      return null;
  }
}

/** The app's background. `look` overrides the setting (for previews); `contained` fills its parent instead of the window. */
export function Backdrop({ look: forced, contained = false }: { look?: Look; contained?: boolean }) {
  const setting = useSettings((s) => s.settings?.general.look);
  const look = forced ?? lookInfo(setting).id;
  const still = useStill();
  const hidden = useHidden();
  return (
    <div className={cx('app-backdrop', `bd-${look}`, contained && 'is-contained', still && 'is-still', hidden && 'is-paused')} aria-hidden>
      <Layers look={look} mini={contained} />
    </div>
  );
}
