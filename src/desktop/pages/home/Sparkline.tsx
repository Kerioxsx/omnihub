import { useId } from 'react';

/** Single-series sparkline: 2px line, 10% area wash, end dot with a surface ring. */
export function Sparkline({ values, max, height = 40, label }: { values: number[]; max: number; height?: number; label: string }) {
  const id = useId().replace(/:/g, '');
  const w = 300;
  if (values.length < 2) return <div style={{ height }} className="rounded-lg bg-surface" aria-label={label} />;
  const step = w / (values.length - 1);
  const pts = values.map((v, i) => [i * step, height - 3 - (Math.min(max, v) / max) * (height - 8)] as const);
  const line = pts.map(([x, y], i) => `${i ? 'L' : 'M'}${x.toFixed(1)},${y.toFixed(1)}`).join(' ');
  const [lx, ly] = pts[pts.length - 1];
  return (
    <div className="relative" style={{ height }}>
    <svg viewBox={`0 0 ${w} ${height}`} preserveAspectRatio="none" className="block w-full overflow-visible" style={{ height }} role="img" aria-label={`${label}: now ${Math.round(values[values.length - 1])}%`}>
      <defs>
        <linearGradient id={`spark-${id}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="var(--accent)" stopOpacity="0.22" />
          <stop offset="1" stopColor="var(--accent)" stopOpacity="0" />
        </linearGradient>
      </defs>
      <line x1="0" y1={height - 0.5} x2={w} y2={height - 0.5} stroke="var(--border)" strokeWidth="1" vectorEffect="non-scaling-stroke" />
      <path d={`${line} L${w},${height} L0,${height} Z`} fill={`url(#spark-${id})`} />
      <path d={line} fill="none" stroke="var(--accent)" strokeWidth="2" strokeLinejoin="round" strokeLinecap="round" vectorEffect="non-scaling-stroke" />
    </svg>
    <span className="absolute h-2.5 w-2.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-[var(--bg-elev)] bg-accent" style={{ left: `${(lx / w) * 100}%`, top: ly }} />
    </div>
  );
}
