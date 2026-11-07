// A crosshair drawn in SVG (the same drawing for the preview and the overlay).

import type { CrosshairLook } from './crosshair';

export function CrosshairShape({ look, box = 120 }: { look: CrosshairLook; box?: number }) {
  const c = box / 2;
  const { size: s, thickness: t, gap: g, color } = look;
  const stroke = { stroke: color, strokeWidth: t, strokeLinecap: 'butt' as const };
  const lines =
    look.style === 'cross' || look.style === 'tcross'
      ? [
          look.style === 'cross' ? [c, c - g - s, c, c - g] : null,
          [c, c + g, c, c + g + s],
          [c - g - s, c, c - g, c],
          [c + g, c, c + g + s, c],
        ].filter(Boolean)
      : look.style === 'chevron'
        ? [
            [c - s, c + s * 0.7, c, c],
            [c, c, c + s, c + s * 0.7],
          ]
        : [];
  const shape = (extra: Record<string, unknown>) => (
    <g {...extra}>
      {lines.map((l, i) => {
        const [x1, y1, x2, y2] = l as number[];
        return <line key={i} x1={x1} y1={y1} x2={x2} y2={y2} />;
      })}
      {look.style === 'circle' && <circle cx={c} cy={c} r={s} fill="none" />}
      {(look.style === 'dot' || look.dot) && <circle cx={c} cy={c} r={Math.max(1, t)} fill={extra.stroke as string} stroke="none" />}
    </g>
  );
  return (
    <svg width={box} height={box} viewBox={`0 0 ${box} ${box}`} style={{ opacity: look.opacity }} aria-hidden>
      {look.outline && shape({ stroke: '#000', strokeWidth: t + 2, strokeLinecap: 'butt', opacity: 0.8 })}
      {shape(stroke)}
    </svg>
  );
}
