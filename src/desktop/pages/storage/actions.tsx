// Shared actions for files and folders in scan results.

import { formatBytes, formatNumber, basename } from '@shared/format';
import type { DeleteResult } from '@shared/types';
import { Copy, ExternalLink, FolderOpen, FolderSearch, ScanSearch, Trash, TrashOff } from 'lucide-react';
import type { MouseEvent } from 'react';
import { api, errorText } from '../../api';
import { type MenuItem, openMenu } from '../../components/ui/Menu';
import { copyText } from '../../lib/util';
import { confirm } from '../../state/dialogs';
import { useStorage } from '../../state/storage';
import { toast } from '../../state/toasts';

export interface ItemRef {
  path: string;
  name?: string;
  isDir: boolean;
  size: number;
}

function summarize(results: DeleteResult[], permanent: boolean, freed: number) {
  const ok = results.filter((r) => r.ok);
  const failed = results.filter((r) => !r.ok);
  if (failed.length === 0) {
    toast.success(permanent ? `Deleted ${ok.length === 1 ? basename(ok[0].path) : `${formatNumber(ok.length)} items`}` : `Moved ${ok.length === 1 ? basename(ok[0].path) : `${formatNumber(ok.length)} items`} to the Recycle Bin`, `${formatBytes(freed)}${permanent ? ' freed' : ' — restore it from the Recycle Bin if needed'}`);
  } else if (ok.length) {
    toast.warn(`${formatNumber(ok.length)} removed, ${formatNumber(failed.length)} skipped`, failed[0].error ?? undefined);
  } else {
    toast.error('Nothing was removed', failed[0]?.error ?? undefined);
  }
}

/** Confirm and delete; returns the results (null when cancelled). */
export async function deleteItems(items: ItemRef[], permanent: boolean, opts: { title?: string; description?: string } = {}): Promise<DeleteResult[] | null> {
  if (!items.length) return null;
  const total = items.reduce((a, i) => a + i.size, 0);
  const single = items.length === 1 ? (items[0].name ?? basename(items[0].path)) : null;
  const typed = permanent ? (single && single.length <= 32 ? single : 'delete') : undefined;
  const ok = await confirm({
    title: opts.title ?? (permanent ? `Permanently delete ${single ? `“${single}”` : `${formatNumber(items.length)} items`}?` : `Move ${single ? `“${single}”` : `${formatNumber(items.length)} items`} to the Recycle Bin?`),
    description:
      opts.description ??
      (permanent ? `This frees ${formatBytes(total)} immediately and cannot be undone — the files skip the Recycle Bin.` : `${formatBytes(total)} will move to the Recycle Bin. Space is freed once you empty it.`),
    tone: 'danger',
    typed,
    confirmLabel: permanent ? 'Delete permanently' : 'Move to Recycle Bin',
    details:
      items.length > 1 ? (
        <ul className="space-y-1">
          {items.slice(0, 40).map((i) => (
            <li key={i.path} className="flex items-center justify-between gap-3">
              <span className="truncate font-mono text-[11.5px] text-dim">{i.path}</span>
              <span className="shrink-0 tabular text-[12px] text-fg">{formatBytes(i.size)}</span>
            </li>
          ))}
          {items.length > 40 && <li className="pt-1 text-[12px] text-faint">…and {formatNumber(items.length - 40)} more</li>}
        </ul>
      ) : (
        <div className="break-all font-mono text-[12px] text-dim">{items[0].path}</div>
      ),
  });
  if (!ok) return null;
  try {
    const results = await api.storage.delete(
      items.map((i) => i.path),
      permanent,
    );
    const freed = items.filter((i) => results.find((r) => r.path === i.path)?.ok).reduce((a, i) => a + i.size, 0);
    summarize(results, permanent, freed);
    useStorage.getState().bump();
    return results;
  } catch (e) {
    toast.error('Delete failed', errorText(e));
    return null;
  }
}

async function run(label: string, fn: () => Promise<unknown>) {
  try {
    await fn();
  } catch (e) {
    toast.error(label, errorText(e));
  }
}

export function itemMenuItems(item: ItemRef, extra: MenuItem[] = []): MenuItem[] {
  const name = item.name ?? basename(item.path);
  return [
    { kind: 'header', label: name },
    { label: 'Open', icon: item.isDir ? FolderOpen : ExternalLink, onSelect: () => void run('Could not open', () => api.app.openPath(item.path)) },
    { label: 'Show in Explorer', icon: FolderSearch, onSelect: () => void run('Could not open Explorer', () => api.app.revealPath(item.path)) },
    {
      label: 'Copy path',
      icon: Copy,
      onSelect: () =>
        void copyText(item.path).then((ok) => {
          if (ok) toast.success('Path copied', item.path);
          else toast.error('Could not copy');
        }),
    },
    ...extra,
    ...(item.isDir ? ([{ kind: 'separator' }, { label: 'Scan from here', icon: ScanSearch, onSelect: () => void useStorage.getState().scan(item.path, 'standard') }] as MenuItem[]) : []),
    { kind: 'separator' },
    { label: 'Delete (to Recycle Bin)', icon: Trash, danger: true, hint: 'Del', onSelect: () => void deleteItems([item], false) },
    { label: 'Delete permanently…', icon: TrashOff, danger: true, hint: 'Shift+Del', onSelect: () => void deleteItems([item], true) },
  ];
}

export function openItemMenu(e: MouseEvent, item: ItemRef, extra: MenuItem[] = []): void {
  openMenu(e, itemMenuItems(item, extra));
}
