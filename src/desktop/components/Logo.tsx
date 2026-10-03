import { cx } from '../lib/cx';

/** The official OmniHub mark (public/omnihub.svg). */
export function Logo({ size = 30, className }: { size?: number; className?: string }) {
  return <img src={`${import.meta.env.BASE_URL}omnihub.svg`} width={size} height={size} alt="" aria-hidden className={cx('shrink-0 select-none drop-shadow-[0_6px_16px_var(--accent-glow)]', className)} draggable={false} />;
}
