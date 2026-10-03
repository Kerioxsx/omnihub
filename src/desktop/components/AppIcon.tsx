import { useAppIcon } from '../lib/media';
import { cx } from '../lib/cx';

const GRADS = ['from-violet-500 to-cyan-400', 'from-rose-500 to-orange-400', 'from-emerald-500 to-sky-400', 'from-amber-500 to-pink-500', 'from-indigo-500 to-fuchsia-500', 'from-sky-500 to-teal-400'];

function monogram(name: string): string {
  const words = name.replace(/[^A-Za-z0-9 ]/g, ' ').split(/\s+/).filter(Boolean);
  if (!words.length) return '?';
  return words.length === 1 ? words[0].slice(0, 2) : (words[0][0] + words[1][0]).toUpperCase();
}

function hash(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) | 0;
  return Math.abs(h);
}

/** App icon with a monogram placeholder while loading (or when there is none). */
export function AppIcon({ id, name, size = 40, className }: { id: string; name: string; size?: number; className?: string }) {
  const [ref, url, done] = useAppIcon(id);
  return (
    <div ref={ref} className={cx('relative shrink-0', className)} style={{ width: size, height: size }}>
      {url ? (
        <img src={url} alt="" width={size} height={size} className="h-full w-full rounded-[22%] object-cover" draggable={false} />
      ) : (
        <div className={cx('flex h-full w-full items-center justify-center rounded-[22%] bg-gradient-to-br font-semibold text-white/90', GRADS[hash(name) % GRADS.length], !done && 'animate-pulse opacity-60')} style={{ fontSize: size * 0.36 }}>
          {monogram(name)}
        </div>
      )}
    </div>
  );
}
