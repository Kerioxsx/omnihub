// The uploads list shown at the top of Files while anything is queued,
// running, paused, failed or recently finished.

import { useState } from 'react';
import { AnimatePresence, motion } from 'motion/react';
import { Pause, Play, X, RotateCcw, ChevronDown, CloudUpload, CircleCheck, CircleAlert, Copy, Trash } from 'lucide-react';
import { formatBytes } from '@shared/format';
import { useUploads, isActive, type UploadItem } from '../uploads';
import { ProgressBar } from '../ui/common';
import { FileIcon } from './fileKinds';
import { copyText } from '../lib/clipboard';
import { toast } from '../state';
import { cx, formatEta, formatSpeed, shortPath } from '../lib/util';

export function UploadsPanel({ className }: { className?: string }) {
  const { items, clearFinished } = useUploads();
  const [open, setOpen] = useState(true);
  if (!items.length) return null;
  const active = items.filter((i) => isActive(i.state));
  const total = items.reduce((a, i) => a + i.size, 0);
  const sent = items.reduce((a, i) => a + (i.state === 'done' ? i.size : i.sent), 0);
  const speed = active.reduce((a, i) => a + (i.state === 'uploading' ? i.speed : 0), 0);
  const finished = items.filter((i) => i.state === 'done' || i.state === 'cancelled').length;
  const failed = items.filter((i) => i.state === 'error').length;
  return (
    <section className={cx('card mb-4 overflow-hidden', className)} aria-label="Uploads">
      <button onClick={() => setOpen((o) => !o)} className="flex w-full items-center gap-3 px-4 pb-3 pt-3.5 text-left">
        <div className={cx('grid h-10 w-10 place-items-center rounded-2xl', active.length ? 'grad-bg text-white' : failed ? 'bg-bad/15 text-bad' : 'bg-good/15 text-good')}>
          {active.length ? <CloudUpload size={20} /> : failed ? <CircleAlert size={20} /> : <CircleCheck size={20} />}
        </div>
        <div className="min-w-0 flex-1">
          <div className="text-[15px] font-semibold">
            {active.length ? `Sending ${active.length} file${active.length > 1 ? 's' : ''}` : failed ? `${failed} upload${failed > 1 ? 's' : ''} need attention` : 'Uploads finished'}
          </div>
          <div className="num truncate text-xs text-dim">
            {formatBytes(sent)} of {formatBytes(total)}
            {speed > 0 && ` · ${formatSpeed(speed)}`}
          </div>
        </div>
        <ChevronDown size={18} className={cx('text-faint transition-transform', open && 'rotate-180')} />
      </button>
      {active.length > 0 && <ProgressBar value={total ? sent / total : 0} active className="mx-4 mb-3" />}
      <AnimatePresence initial={false}>
        {open && (
          <motion.div initial={{ height: 0, opacity: 0 }} animate={{ height: 'auto', opacity: 1 }} exit={{ height: 0, opacity: 0 }} transition={{ duration: 0.22 }} className="overflow-hidden">
            <ul className="border-t border-line">
              <AnimatePresence initial={false}>
                {items.map((i) => (
                  <motion.li key={i.key} layout initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} exit={{ opacity: 0, height: 0 }} transition={{ duration: 0.2 }}>
                    <UploadRow item={i} />
                  </motion.li>
                ))}
              </AnimatePresence>
            </ul>
            {finished > 0 && (
              <button onClick={clearFinished} className="flex w-full items-center justify-center gap-1.5 border-t border-line py-3 text-sm font-semibold text-dim active:bg-surface-2">
                <Trash size={15} /> Clear finished
              </button>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </section>
  );
}

function UploadRow({ item: i }: { item: UploadItem }) {
  const u = useUploads();
  const frac = i.size ? (i.state === 'done' ? 1 : i.sent / i.size) : i.state === 'done' ? 1 : 0;
  const tone = i.state === 'done' ? 'good' : i.state === 'error' ? 'bad' : i.state === 'paused' || i.state === 'cancelled' ? 'dim' : 'accent';
  let status: React.ReactNode;
  switch (i.state) {
    case 'queued':
      status = `Waiting · ${formatBytes(i.size)} → ${i.dirLabel}`;
      break;
    case 'uploading':
      status = `Sending to ${i.dirLabel}`;
      break;
    case 'paused':
      status = 'Paused — tap ▶ to resume';
      break;
    case 'error':
      status = <span className="text-bad">{i.error ?? 'Failed'}</span>;
      break;
    case 'cancelled':
      status = 'Cancelled';
      break;
    case 'done':
      status = (
        <span className="selectable" title={i.path}>
          Saved to {shortPath(i.path ?? '', 44)}
        </span>
      );
      break;
  }
  const showBar = i.state !== 'done' && i.state !== 'cancelled';
  return (
    <div className="px-4 py-3">
      <div className="flex gap-3">
        <FileIcon name={i.name} size={40} />
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <div className="min-w-0 flex-1 truncate text-[15px] font-semibold">{i.name}</div>
            {i.state === 'done' && <CircleCheck size={16} className="shrink-0 text-good" />}
          </div>
          <div className={cx('mt-0.5 text-xs text-dim', i.state !== 'done' && 'truncate')}>{status}</div>
          {i.state === 'done' && i.sha256 && (
            <button
              className="mt-1.5 inline-flex max-w-full items-center gap-1.5 rounded-lg bg-surface-2 px-2 py-1 font-mono text-[11px] text-dim active:bg-surface-3"
              onClick={async () => ((await copyText(i.sha256!)) ? toast.success('SHA-256 copied') : toast.error("Couldn't copy", i.sha256))}
              aria-label="Copy SHA-256"
            >
              SHA-256 {i.sha256.slice(0, 10)}…{i.sha256.slice(-6)} <Copy size={11} />
            </button>
          )}
        </div>
        <div className="flex shrink-0 items-start gap-1.5">
          {i.state === 'uploading' || i.state === 'queued' ? (
            <RowBtn label="Pause" onClick={() => u.pause(i.key)}>
              <Pause size={16} />
            </RowBtn>
          ) : i.state === 'paused' ? (
            <RowBtn label="Resume" onClick={() => u.resume(i.key)} accent>
              <Play size={16} />
            </RowBtn>
          ) : i.state === 'error' ? (
            <RowBtn label="Retry" onClick={() => u.retry(i.key)} accent>
              <RotateCcw size={16} />
            </RowBtn>
          ) : null}
          {i.state === 'done' || i.state === 'cancelled' || i.state === 'error' ? (
            <RowBtn label="Remove from list" onClick={() => u.remove(i.key)}>
              <X size={16} />
            </RowBtn>
          ) : (
            <RowBtn label="Cancel upload" onClick={() => u.cancel(i.key)}>
              <X size={16} />
            </RowBtn>
          )}
        </div>
      </div>
      {showBar && (
        <div className="mt-2.5 pl-[52px]">
          <ProgressBar value={frac} tone={tone} active={i.state === 'uploading'} />
          <div className="num mt-1.5 flex justify-between gap-3 whitespace-nowrap text-[11.5px] text-faint">
            <span>
              {formatBytes(i.state === 'paused' ? i.offset : i.sent)} of {formatBytes(i.size)} · {Math.floor(frac * 100)}%
            </span>
            {i.state === 'uploading' && <span>{i.speed > 0 ? `${formatSpeed(i.speed)}${Number.isFinite(i.eta) ? ` · ${formatEta(i.eta)}` : ''}` : 'starting…'}</span>}
          </div>
        </div>
      )}
    </div>
  );
}

function RowBtn({ label, onClick, children, accent }: { label: string; onClick: () => void; children: React.ReactNode; accent?: boolean }) {
  return (
    <button aria-label={label} title={label} onClick={onClick} className={cx('press grid h-9 w-9 place-items-center rounded-full', accent ? 'bg-accent text-white' : 'bg-surface-2 text-dim')}>
      {children}
    </button>
  );
}
