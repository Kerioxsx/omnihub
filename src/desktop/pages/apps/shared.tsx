import { formatBytes } from '@shared/format';
import type { AppInfo, AppSource } from '@shared/types';

export const SOURCE_LABEL: Record<AppSource, string> = { desktop: 'Desktop', store: 'Store', startMenu: 'Start menu' };

export function SizeHint({ app }: { app: AppInfo }) {
  if (app.size == null) return <span className="text-faint">—</span>;
  return (
    <span className="inline-flex items-center gap-1.5" title={app.sizeFromScan ? 'Measured from your latest storage scan' : 'Estimate reported by the installer. Scan the drive for the real size.'}>
      <span className="tabular text-fg">
        {app.sizeFromScan ? '' : '≈ '}
        {formatBytes(app.size)}
      </span>
      {app.sizeFromScan && <span className="h-1.5 w-1.5 rounded-full bg-good" aria-label="from scan" />}
    </span>
  );
}
