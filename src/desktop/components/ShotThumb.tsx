import { ImageOff } from 'lucide-react';
import { useThumb } from '../lib/media';
import { cx } from '../lib/cx';

/** Lazily loaded screenshot thumbnail with a shimmer placeholder. */
export function ShotThumb({ id, alt, className, exists = true }: { id: string; alt: string; className?: string; exists?: boolean }) {
  const [ref, url] = useThumb(id);
  return (
    <div ref={ref} className={cx('relative overflow-hidden bg-surface-2', className)}>
      {!exists ? (
        <div className="flex h-full w-full items-center justify-center text-faint">
          <ImageOff size={20} aria-label="File missing" />
        </div>
      ) : url ? (
        <img src={url} alt={alt} className="h-full w-full object-cover transition-transform duration-300 group-hover:scale-[1.04]" draggable={false} />
      ) : (
        <div className="skeleton absolute inset-0 rounded-none" />
      )}
    </div>
  );
}
