import { formatBytes, formatDate, formatNumber } from '@shared/format';
import type { CleanupCategory, Risk, Suggestion } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import type { LucideIcon } from 'lucide-react';
import { Archive, Bug, ChevronDown, Code, Copy, Download, Info, Layers, Monitor, Recycle, ShieldAlert, Sparkles, Timer, Trash, CircleCheckBig } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, Skeleton } from '../../components/ui/Card';
import { EmptyState, ErrorState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useAsync } from '../../lib/hooks';
import { copyText } from '../../lib/util';
import { confirm } from '../../state/dialogs';
import { useStorage } from '../../state/storage';
import { toast } from '../../state/toasts';
import { deleteItems, openItemMenu } from './actions';

const ICONS: Record<CleanupCategory, LucideIcon> = { temp: Timer, cache: Layers, crashDumps: Bug, downloads: Download, largeOld: Archive, recycleBin: Recycle, developer: Code, system: Monitor };
const RISK: Record<Risk, { tone: 'good' | 'warn' | 'info'; label: string }> = { safe: { tone: 'good', label: 'Safe to remove' }, review: { tone: 'warn', label: 'Review first' }, info: { tone: 'info', label: 'Info' } };

function SuggestionCard({ s, onCleaned }: { s: Suggestion; onCleaned: () => void }) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const Icon = ICONS[s.category] ?? Sparkles;
  const risk = RISK[s.risk];
  const isBin = s.id === 'recycle-bin' || s.actionHint === 'empty-recycle-bin';

  const clean = async () => {
    setBusy(true);
    try {
      if (isBin) {
        const ok = await confirm({ title: 'Empty the Recycle Bin?', description: `This permanently removes ${formatNumber(s.files)} deleted files and frees ${formatBytes(s.size)}. It cannot be undone.`, tone: 'danger', confirmLabel: 'Empty Recycle Bin' });
        if (!ok) return;
        await api.storage.emptyRecycleBin();
        toast.success('Recycle Bin emptied', `${formatBytes(s.size)} freed`);
        useStorage.getState().bump();
        onCleaned();
        return;
      }
      const items = s.items.length === s.paths.length ? s.items.map((i) => ({ path: i.path, isDir: i.isDir, size: i.size })) : s.paths.map((p) => ({ path: p, isDir: false, size: s.items.find((i) => i.path === p)?.size ?? 0 }));
      const res = await deleteItems(items, false, {
        title: `Clean “${s.title}”?`,
        description: `${formatNumber(s.paths.length)} item${s.paths.length === 1 ? '' : 's'} (${formatNumber(s.files)} files, ${formatBytes(s.size)}) will move to the Recycle Bin.${s.needsAdmin ? ' Some may need administrator rights and will be skipped.' : ''}`,
      });
      if (res) onCleaned();
    } catch (e) {
      toast.error('Cleanup failed', errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <motion.div layout initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, scale: 0.98 }}>
      <Card className="overflow-hidden">
        <div className="flex items-start gap-4 px-5 py-4">
          <div className={cx('flex h-10 w-10 shrink-0 items-center justify-center rounded-xl', s.risk === 'safe' ? 'bg-good/12 text-good' : s.risk === 'review' ? 'bg-warn/12 text-warn' : 'bg-info/12 text-info')}>
            <Icon size={19} aria-hidden />
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-[14.5px] font-semibold text-fg">{s.title}</h3>
              <Badge tone={risk.tone}>{risk.label}</Badge>
              {s.needsAdmin && (
                <Badge tone="neutral" icon={ShieldAlert} title="Some items need administrator rights; anything OmniHub can't remove is skipped.">
                  May need admin
                </Badge>
              )}
            </div>
            <p className="mt-1 max-w-[720px] text-[13px] leading-relaxed text-dim">{s.description}</p>
            <div className="mt-2 flex items-center gap-3 text-[12px] text-faint">
              <span className="tabular">{formatNumber(s.files)} files</span>
              {s.items.length > 0 && (
                <button type="button" onClick={() => setOpen(!open)} className="inline-flex items-center gap-1 font-medium text-dim hover:text-fg" aria-expanded={open}>
                  {open ? 'Hide' : 'Show'} {s.items.length === 1 ? 'item' : `${Math.min(50, s.items.length)} largest items`}
                  <ChevronDown size={13} className={cx('transition-transform', open && 'rotate-180')} />
                </button>
              )}
            </div>
          </div>
          <div className="flex shrink-0 flex-col items-end gap-2">
            <div className="font-display text-[20px] font-semibold tabular text-fg">{formatBytes(s.size)}</div>
            {s.risk === 'info' ? (
              s.actionHint && (
                <div className="flex items-center gap-1 rounded-lg border border-line bg-surface-2 py-0.5 pl-2.5 pr-0.5">
                  <code className="font-mono text-[11.5px] text-dim">{s.actionHint}</code>
                  <IconButton icon={Copy} label="Copy command" size="sm" onClick={() => void copyText(s.actionHint ?? '').then(() => toast.success('Command copied', s.actionHint ?? ''))} />
                </div>
              )
            ) : (
              <Button size="sm" variant={s.risk === 'safe' ? 'primary' : 'secondary'} icon={isBin ? Recycle : Trash} loading={busy} onClick={clean} disabled={!isBin && s.paths.length === 0}>
                {isBin ? 'Empty' : 'Clean'}
              </Button>
            )}
          </div>
        </div>
        <AnimatePresence initial={false}>
          {open && (
            <motion.div initial={{ height: 0 }} animate={{ height: 'auto' }} exit={{ height: 0 }} transition={{ type: 'spring', stiffness: 300, damping: 34 }} className="overflow-hidden border-t border-line bg-surface/50">
              <ul className="max-h-64 overflow-y-auto px-5 py-2">
                {s.items.map((it) => (
                  <li key={it.id} onContextMenu={(e) => openItemMenu(e, { path: it.path, isDir: it.isDir, size: it.size })} className="flex items-center gap-3 rounded-md px-2 py-1 hover:bg-surface-2">
                    <span className="min-w-0 flex-1 truncate font-mono text-[11.5px] text-dim" title={it.path}>
                      {it.path}
                    </span>
                    <span className="shrink-0 text-[11.5px] text-faint tabular">{formatDate(it.modified)}</span>
                    <span className="w-[70px] shrink-0 text-right text-[12px] font-medium tabular text-fg">{formatBytes(it.size)}</span>
                  </li>
                ))}
                {s.paths.length > s.items.length && <li className="px-2 py-1 text-[12px] text-faint">…and {formatNumber(s.paths.length - s.items.length)} smaller items</li>}
              </ul>
            </motion.div>
          )}
        </AnimatePresence>
      </Card>
    </motion.div>
  );
}

export function CleanupTab({ scanId }: { scanId: string }) {
  const version = useStorage((s) => s.version);
  const res = useAsync(() => api.storage.cleanup(scanId), [scanId, version]);
  const list = res.data ?? [];
  const safe = list.filter((s) => s.risk === 'safe');
  const safeSize = safe.reduce((a, s) => a + s.size, 0);
  const reviewSize = list.filter((s) => s.risk === 'review').reduce((a, s) => a + s.size, 0);
  const [busyAll, setBusyAll] = useState(false);

  const cleanAllSafe = async () => {
    const del = safe.filter((s) => s.id !== 'recycle-bin' && s.paths.length);
    const bin = safe.find((s) => s.id === 'recycle-bin');
    const ok = await confirm({
      title: `Clean ${formatBytes(safeSize)} of safe items?`,
      description: 'Temporary files, caches and crash dumps move to the Recycle Bin' + (bin ? ', and the Recycle Bin is emptied first.' : '.'),
      confirmLabel: 'Clean safe items',
      details: (
        <ul className="space-y-1">
          {safe.map((s) => (
            <li key={s.id} className="flex justify-between gap-3">
              <span className="text-dim">{s.title}</span>
              <span className="tabular text-fg">{formatBytes(s.size)}</span>
            </li>
          ))}
        </ul>
      ),
    });
    if (!ok) return;
    setBusyAll(true);
    try {
      if (bin) await api.storage.emptyRecycleBin();
      const paths = del.flatMap((s) => s.paths);
      const results = paths.length ? await api.storage.delete(paths, false) : [];
      const failed = results.filter((r) => !r.ok).length;
      if (failed) toast.warn(`Cleaned with ${formatNumber(failed)} skipped`, results.find((r) => !r.ok)?.error ?? undefined);
      else toast.success('Safe items cleaned', `${formatBytes(safeSize)} freed or moved to the Recycle Bin`);
      useStorage.getState().bump();
    } catch (e) {
      toast.error('Cleanup failed', errorText(e));
    } finally {
      setBusyAll(false);
    }
  };

  if (res.error) return <ErrorState error={res.error} onRetry={res.reload} />;
  if (!res.data)
    return (
      <div className="space-y-3">
        <Skeleton className="h-24 rounded-2xl" />
        {[0, 1, 2, 3].map((i) => (
          <Skeleton key={i} className="h-28 rounded-2xl" />
        ))}
      </div>
    );
  if (!list.length) return <EmptyState icon={CircleCheckBig} title="Nothing to clean" description="No temporary files, caches or leftovers worth removing were found in this scan." />;

  return (
    <div className="min-h-0 flex-1 space-y-3 overflow-y-auto pb-6 pr-1">
      <Card className="relative flex items-center gap-6 overflow-hidden px-6 py-5">
        <div className="pointer-events-none absolute -left-10 -top-16 h-48 w-48 rounded-full bg-[radial-gradient(circle,color-mix(in_oklab,var(--good)_22%,transparent),transparent_70%)]" />
        <div className="relative flex h-12 w-12 items-center justify-center rounded-2xl bg-good/12 text-good">
          <Sparkles size={22} aria-hidden />
        </div>
        <div className="relative flex-1">
          <div className="text-[13px] text-dim">You can safely free</div>
          <div className="font-display text-[28px] font-semibold leading-tight tabular text-fg">{formatBytes(safeSize)}</div>
          <div className="text-[12.5px] text-faint">
            plus {formatBytes(reviewSize)} worth reviewing · {list.length} suggestions
          </div>
        </div>
        <div className="relative flex items-center gap-2 text-[12px] text-faint">
          <Info size={14} aria-hidden />
          Everything goes to the Recycle Bin first.
        </div>
        <Button variant="primary" size="lg" icon={Sparkles} loading={busyAll} disabled={!safe.length} onClick={cleanAllSafe} className="relative">
          Clean all safe items
        </Button>
      </Card>
      <AnimatePresence initial={false}>
        {list.map((s) => (
          <SuggestionCard key={s.id} s={s} onCleaned={() => void res.reload()} />
        ))}
      </AnimatePresence>
    </div>
  );
}
