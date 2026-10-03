// Drive picker, scan controls, live progress and the results summary.

import { formatBytes, formatDuration, formatNumber, formatRelative } from '@shared/format';
import type { JobProgress, ScanSummary, VolumeInfo } from '@shared/types';
import { motion } from 'motion/react';
import { Clock, Database, FolderTree, Gauge, HardDrive, HardDriveDownload, Info, RefreshCw, ScanLine, ShieldCheck, Usb, X, Zap } from 'lucide-react';
import type { ReactNode } from 'react';
import { Button } from '../../components/ui/Button';
import { Badge, Card, Skeleton } from '../../components/ui/Card';
import { Segmented } from '../../components/ui/Form';
import { ProgressBar, ProgressRing, UsageBar } from '../../components/ui/Progress';
import { cx } from '../../lib/cx';
import { useNow } from '../../lib/hooks';
import { type ScanModeChoice, jobFraction, rootKey } from '../../state/storage';

export function DrivePicker({ volumes, root, onSelect, loaded }: { volumes: VolumeInfo[]; root: string | null; onSelect: (root: string) => void; loaded: boolean }) {
  const isFolder = root && !volumes.some((v) => rootKey(v.root) === rootKey(root));
  if (!loaded) {
    return (
      <div className="flex gap-3">
        {[0, 1, 2].map((i) => (
          <Skeleton key={i} className="h-[62px] w-[220px] rounded-2xl" />
        ))}
      </div>
    );
  }
  return (
    <div className="flex flex-wrap gap-2.5" role="radiogroup" aria-label="Drive">
      {volumes.map((v) => {
        const used = v.total - v.free;
        const f = v.total ? used / v.total : 0;
        const active = root != null && rootKey(root) === rootKey(v.root);
        const Icon = v.kind === 'removable' ? Usb : HardDrive;
        return (
          <motion.button
            key={v.root}
            type="button"
            role="radio"
            aria-checked={active}
            whileTap={{ scale: 0.98 }}
            onClick={() => onSelect(v.root)}
            className={cx('card group relative flex w-[236px] items-center gap-3 rounded-2xl px-3.5 py-2.5 text-left transition-colors', active ? 'accent-ring !border-transparent !bg-surface-2' : 'hover:border-line-strong hover:bg-surface-2')}
          >
            <div className={cx('flex h-9 w-9 shrink-0 items-center justify-center rounded-xl', active ? 'accent-gradient text-white' : 'bg-surface-3 text-dim group-hover:text-fg')}>
              <Icon size={17} aria-hidden />
            </div>
            <div className="min-w-0 flex-1">
              <div className="flex items-baseline justify-between gap-2">
                <span className="truncate text-[13.5px] font-semibold text-fg">
                  {v.root.replace(/\\$/, '')} <span className="font-normal text-dim">{v.label}</span>
                </span>
                <span className="shrink-0 text-[11px] text-faint">{v.fileSystem}</span>
              </div>
              <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-surface-3">
                <div className={cx('h-full rounded-full', f > 0.9 ? 'bg-bad' : f > 0.8 ? 'bg-warn' : 'accent-gradient')} style={{ width: `${f * 100}%` }} />
              </div>
              <div className="mt-1 text-[11px] text-faint tabular">
                {formatBytes(v.free)} free of {formatBytes(v.total)}
              </div>
            </div>
          </motion.button>
        );
      })}
      {isFolder && (
        <div className="card accent-ring flex items-center gap-3 rounded-2xl !border-transparent px-3.5 py-2.5">
          <div className="accent-gradient flex h-9 w-9 items-center justify-center rounded-xl text-white">
            <FolderTree size={17} aria-hidden />
          </div>
          <div className="min-w-0">
            <div className="text-[11px] text-faint">Folder scan</div>
            <div className="max-w-[260px] truncate font-mono text-[12px] text-fg" title={root}>
              {root}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export function ModeToggle({ volume, mode, onChange }: { volume: VolumeInfo | null; mode: ScanModeChoice; onChange: (m: ScanModeChoice) => void }) {
  const fastOk = !!volume?.mftCapable;
  const why = !volume ? 'Fast scan works on whole NTFS drives, not single folders.' : `${volume.root.replace(/\\$/, '')} is ${volume.fileSystem}. Fast scan reads the NTFS Master File Table, so it needs an NTFS drive.`;
  return (
    <Segmented
      label="Scan mode"
      value={fastOk ? mode : 'standard'}
      onChange={onChange}
      options={[
        { value: 'fast', label: 'Fast (MFT)', icon: Zap, disabled: !fastOk, title: fastOk ? 'Reads the Master File Table directly. Windows asks for administrator approval each time.' : why },
        { value: 'standard', label: 'Standard', icon: FolderTree, title: 'Walks every folder with normal permissions. Slower, no admin prompt.' },
      ]}
    />
  );
}

export function ModeHint({ volume, mode }: { volume: VolumeInfo | null; mode: ScanModeChoice }) {
  const fastOk = !!volume?.mftCapable;
  let text: ReactNode;
  if (mode === 'fast' && fastOk) text = 'Fast scan reads the NTFS Master File Table directly — millions of files in seconds. Windows asks for administrator approval (UAC) each time; nothing else runs elevated.';
  else if (mode === 'fast' && !fastOk) text = volume ? `Fast scan needs NTFS — ${volume.root.replace(/\\$/, '')} is ${volume.fileSystem}, so OmniHub walks the folders instead.` : 'Folder scans always walk the folders (fast scan reads a whole NTFS drive).';
  else text = 'Standard scan walks every folder with your normal permissions. Slower on big drives, but no admin prompt.';
  return (
    <div className="flex max-w-[360px] items-start gap-2 text-[12px] leading-snug text-faint">
      {mode === 'fast' && fastOk ? <ShieldCheck size={14} className="mt-0.5 shrink-0 text-accent" aria-hidden /> : <Info size={14} className="mt-0.5 shrink-0" aria-hidden />}
      <span>{text}</span>
    </div>
  );
}

// ---------- live progress ----------

const PHASES: Record<string, [string, string]> = {
  starting: ['Starting…', 'Preparing the scan.'],
  elevating: ['Waiting for administrator approval…', 'Approve the Windows prompt (UAC) so OmniHub can read the Master File Table. Only this scan runs elevated.'],
  opening: ['Opening the volume…', 'Getting raw read access to the drive.'],
  journal: ['Applying recent changes…', 'Reading the USN change journal since the last snapshot.'],
  refresh: ['Refreshing the snapshot…', 'Updating the cached results with recent changes.'],
  mft: ['Reading the Master File Table', 'Every file record on the drive, straight from NTFS.'],
  saving: ['Saving a snapshot…', 'So the results open instantly next time.'],
  loading: ['Loading the cached snapshot…', 'Results will appear in a moment.'],
  walk: ['Walking folders', 'Listing every folder with your normal permissions.'],
};

function Stat({ icon: Icon, label, value }: { icon: typeof Clock; label: string; value: ReactNode }) {
  return (
    <div className="rounded-xl border border-line bg-surface px-3.5 py-2.5">
      <div className="flex items-center gap-1.5 text-[11px] font-medium uppercase tracking-wider text-faint">
        <Icon size={12} aria-hidden />
        {label}
      </div>
      <div className="mt-1 font-display text-[18px] font-semibold tabular text-fg">{value}</div>
    </div>
  );
}

export function ScanProgressView({ job, volumes, onCancel }: { job: JobProgress; volumes: VolumeInfo[]; onCancel: () => void }) {
  useNow(500);
  const f = jobFraction(job, volumes);
  const [title, desc] = PHASES[job.phase] ?? [job.phase, ''];
  const elevating = job.phase === 'elevating';
  const secs = Math.max(0.001, job.elapsedMs / 1000);
  const mft = job.phase === 'mft' || job.recordsTotal > 0;
  return (
    <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} className="flex flex-1 items-center justify-center py-6">
      <Card className="relative w-full max-w-[880px] overflow-hidden p-8">
        <div className="pointer-events-none absolute -right-24 -top-24 h-72 w-72 rounded-full bg-[radial-gradient(circle,var(--accent-soft),transparent_70%)]" />
        <div className="relative flex items-center gap-8">
          <ProgressRing value={elevating ? null : f} size={168} stroke={10} label="Scan progress">
            <div className="flex flex-col items-center">
              {elevating ? (
                <div className="pulse-ring flex h-16 w-16 items-center justify-center rounded-full bg-accent-soft text-accent">
                  <ShieldCheck size={30} aria-hidden />
                </div>
              ) : f != null ? (
                <>
                  <span className="font-display text-[34px] font-semibold tabular text-fg">{Math.round(f * 100)}%</span>
                  <span className="text-[11px] uppercase tracking-wider text-faint">{job.method === 'mft' ? 'MFT' : 'Walk'}</span>
                </>
              ) : (
                <ScanLine size={30} className="text-accent" aria-hidden />
              )}
            </div>
          </ProgressRing>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <Badge tone="accent" icon={job.method === 'mft' ? Zap : FolderTree}>
                {job.method === 'mft' ? 'Fast scan' : job.method === 'walk' ? 'Standard scan' : 'Scan'} · {job.root}
              </Badge>
            </div>
            <h2 className="mt-3 font-display text-[22px] font-semibold tracking-tight text-fg">{title}</h2>
            <p className="mt-1 max-w-[520px] text-[13.5px] leading-relaxed text-dim">{desc}</p>
            <div className="mt-5 grid grid-cols-4 gap-2.5">
              {mft ? <Stat icon={Database} label="Records" value={`${formatNumber(job.recordsDone)}`} /> : <Stat icon={Database} label="Files" value={formatNumber(job.files)} />}
              <Stat icon={FolderTree} label={mft ? 'Files' : 'Folders'} value={formatNumber(mft ? job.files : job.dirs)} />
              <Stat icon={HardDriveDownload} label="Found" value={formatBytes(job.bytes)} />
              <Stat icon={Clock} label="Elapsed" value={formatDuration(job.elapsedMs)} />
            </div>
            {mft && job.recordsTotal > 0 && (
              <div className="mt-4">
                <div className="mb-1.5 flex justify-between text-[12px] text-faint tabular">
                  <span>
                    {formatNumber(job.recordsDone)} of {formatNumber(job.recordsTotal)} records
                  </span>
                  <span className="flex items-center gap-1">
                    <Gauge size={12} aria-hidden />
                    {formatNumber(Math.round(job.recordsDone / secs))} records/s
                  </span>
                </div>
                <ProgressBar value={job.recordsDone / job.recordsTotal} label="Records read" />
              </div>
            )}
            {!mft && (
              <div className="mt-4 rounded-xl border border-line bg-surface px-3 py-2">
                <div className="text-[11px] uppercase tracking-wider text-faint">Current folder</div>
                <div className="mt-0.5 truncate font-mono text-[12px] text-dim" title={job.current}>
                  {job.current || '…'}
                </div>
              </div>
            )}
            <div className="mt-5 flex items-center gap-3">
              <Button icon={X} onClick={onCancel}>
                Cancel scan
              </Button>
              {job.errors > 0 && <span className="text-[12px] text-faint">{formatNumber(job.errors)} folders could not be read (access denied)</span>}
            </div>
          </div>
        </div>
      </Card>
    </motion.div>
  );
}

// ---------- results summary ----------

export function SummaryCard({ summary, volume, onRefresh, refreshing }: { summary: ScanSummary; volume: VolumeInfo | null; onRefresh: () => void; refreshing: boolean }) {
  useNow(30000);
  const total = summary.volumeTotal ?? volume?.total ?? null;
  const free = summary.volumeFree ?? volume?.free ?? null;
  const usedOther = total != null && free != null ? Math.max(0, total - free - summary.alloc) : 0;
  const how = summary.method === 'walk' ? 'Walked the folders' : summary.method === 'mftIncremental' ? 'Updated from the change journal' : 'Read the MFT';
  return (
    <Card className="flex items-center gap-6 px-5 py-3">
      <div className="min-w-0">
        <div className="flex items-center gap-2 text-[12px] text-faint">
          <span className="font-mono text-dim">{summary.root}</span>
          {summary.fromCache ? (
            <Badge tone="warn" icon={Clock} title="Loaded from the snapshot saved after the last fast scan">
              from cache · scanned {formatRelative(summary.scannedAt)}
            </Badge>
          ) : (
            <Badge tone="good">scanned {formatRelative(summary.scannedAt)}</Badge>
          )}
        </div>
        <div className="mt-1 flex items-baseline gap-3">
          <span className="gradient-text font-display text-[26px] font-semibold leading-none tracking-tight tabular">{formatBytes(summary.size)}</span>
          <span className="text-[13px] text-dim tabular">
            {formatNumber(summary.files)} files · {formatNumber(summary.dirs)} folders
          </span>
        </div>
        <div className="mt-1 flex items-center gap-1.5 text-[12.5px] text-faint">
          {summary.method === 'walk' ? <FolderTree size={13} aria-hidden /> : <Zap size={13} className="text-accent" aria-hidden />}
          {how} in {formatDuration(summary.durationMs)}
          {summary.errors > 0 && <span>· {formatNumber(summary.errors)} unreadable</span>}
        </div>
      </div>
      {total != null && free != null && (
        <div className="min-w-[260px] flex-1">
          <UsageBar
            height={10}
            segments={[
              { value: summary.alloc, color: 'linear-gradient(90deg,var(--accent),var(--accent-2))', label: `Scanned files · ${formatBytes(summary.alloc)}` },
              { value: usedOther, color: 'var(--text-faint)', label: `System & unreadable · ${formatBytes(usedOther)}` },
              { value: free, color: 'var(--surface-3)', label: `Free · ${formatBytes(free)}` },
            ]}
          />
          <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-[11.5px] text-dim">
            <Legend color="linear-gradient(90deg,var(--accent),var(--accent-2))" label="Files" value={formatBytes(summary.alloc)} />
            <Legend color="var(--text-faint)" label="System & other" value={formatBytes(usedOther)} />
            <Legend color="var(--surface-3)" label="Free" value={formatBytes(free)} />
            <span className="ml-auto text-faint">of {formatBytes(total)}</span>
          </div>
        </div>
      )}
      <Button icon={RefreshCw} loading={refreshing} onClick={onRefresh} variant={summary.fromCache ? 'primary' : 'secondary'}>
        {summary.fromCache ? 'Refresh' : 'Rescan'}
      </Button>
    </Card>
  );
}

function Legend({ color, label, value }: { color: string; label: string; value: string }) {
  return (
    <span className="inline-flex items-center gap-1.5">
      <span className="h-2 w-2 rounded-full" style={{ background: color }} />
      {label} <span className="font-medium text-fg tabular">{value}</span>
    </span>
  );
}

export function NeverScanned({ volume, root, onScan, mode }: { volume: VolumeInfo | null; root: string; onScan: () => void; mode: ScanModeChoice }) {
  const fast = mode === 'fast' && volume?.mftCapable;
  return (
    <div className="flex flex-1 items-center justify-center">
      <div className="flex max-w-[520px] flex-col items-center text-center">
        <motion.div initial={{ scale: 0.9, opacity: 0 }} animate={{ scale: 1, opacity: 1 }} className="relative mb-5">
          <div className="absolute inset-0 rounded-full bg-[radial-gradient(circle,var(--accent-glow),transparent_70%)] blur-xl" />
          <div className="relative flex h-20 w-20 items-center justify-center rounded-3xl border border-line-strong bg-surface-2">
            <HardDrive size={34} className="text-accent" aria-hidden />
          </div>
        </motion.div>
        <h2 className="font-display text-[20px] font-semibold text-fg">{root.replace(/\\$/, '')} hasn't been scanned yet</h2>
        <p className="mt-2 text-[13.5px] leading-relaxed text-dim">
          {volume ? `${formatBytes(volume.total - volume.free)} of ${formatBytes(volume.total)} is in use. ` : ''}Scan it to see a treemap of what takes up space, the largest files, cleanup suggestions and duplicates.
        </p>
        <Button variant="primary" size="lg" icon={fast ? Zap : ScanLine} className="mt-6" onClick={onScan}>
          {fast ? 'Fast scan' : 'Scan'} {root.replace(/\\$/, '')}
        </Button>
        {fast && <p className="mt-3 text-[12px] text-faint">Windows will ask for administrator approval to read the Master File Table.</p>}
      </div>
    </div>
  );
}
