import { useId } from 'react';

/** The official OmniHub mark (assets/logo.svg), inlined so it needs no request. */
export function Logo({ size = 40, className }: { size?: number; className?: string }) {
  const id = useId().replace(/:/g, '');
  const tile = `t${id}`;
  const sheen = `s${id}`;
  const ring = `r${id}`;
  return (
    <svg width={size} height={size} viewBox="48 48 928 928" className={className} aria-hidden="true">
      <defs>
        <linearGradient id={tile} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#7c3aed" />
          <stop offset="0.55" stopColor="#5b5cf0" />
          <stop offset="1" stopColor="#06b6d4" />
        </linearGradient>
        <radialGradient id={sheen} cx="0.28" cy="0.18" r="0.95">
          <stop offset="0" stopColor="#ffffff" stopOpacity="0.38" />
          <stop offset="0.55" stopColor="#ffffff" stopOpacity="0" />
        </radialGradient>
        <linearGradient id={ring} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#ffffff" />
          <stop offset="1" stopColor="#e0f7ff" />
        </linearGradient>
      </defs>
      <rect x="48" y="48" width="928" height="928" rx="232" fill={`url(#${tile})`} />
      <rect x="48" y="48" width="928" height="928" rx="232" fill={`url(#${sheen})`} />
      <circle cx="512" cy="512" r="236" fill="none" stroke={`url(#${ring})`} strokeWidth="64" />
      <circle cx="512" cy="512" r="92" fill="#ffffff" />
      <g fill="#ffffff" stroke="#5b5cf0" strokeWidth="22">
        <circle cx="630" cy="308" r="64" />
        <circle cx="630" cy="716" r="64" />
        <circle cx="276" cy="512" r="64" />
      </g>
    </svg>
  );
}
