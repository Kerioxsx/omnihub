// Tasks: what is using the PC right now — CPU, memory and GPU for the whole
// PC and for every program, with priority and "End task".

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Activity, Cpu, HardDrive, MemoryStick, Monitor, Search, X, XCircle } from 'lucide-react';
import { formatBytes } from '@shared/format';
import { client, type Priority, type ProcessGroup, type ProcessSort, type Usage } from '../client';
import { toast } from '../state';
import { PullToRefresh } from '../ui/PullToRefresh';
import { Sheet } from '../ui/Sheet';
import { Button, Empty, ErrorState, Ring, Segmented, Skeleton } from '../ui/common';
import { SubHeader } from './More';
import { cx, errorMessage, useInterval, usePageVisible, vibrate } from '../lib/util';

const PRIORITIES: { value: Priority; label: string }[] = [
  { value: 'low', label: 'Low' },
  { value: 'belowNormal', label: 'Below' },
  { value: 'normal', label: 'Normal' },
  { value: 'aboveNormal', label: 'Above' },
  { value: 'high', label: 'High' },
];
const PRIORITY_NAME: Record<Priority, string> = { low: 'Low', belowNormal: 'Below normal', normal: 'Normal', aboveNormal: 'Above normal', high: 'High' };

const short = (n: string) => n.replace(/\.exe$/i, '');
const rate = (b: number) => (b < 1024 ? '0 MB/s' : `${formatBytes(b, 1)}/s`);

const HUES = [262, 199, 330, 152, 28, 220, 285, 175];
function hueFor(name: string) {
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return HUES[h % HUES.length];
}

function Avatar({ name, size = 40 }: { name: string; size?: number }) {
  const hue = hueFor(name);
  return (
    <div className="grid shrink-0 place-items-center rounded-xl font-display font-bold text-white" style={{ width: size, height: size, fontSize: size * 0.42, background: `linear-gradient(135deg, hsl(${hue} 70% 55%), hsl(${(hue + 40) % 360} 70% 45%))` }} aria-hidden>
      {short(name).charAt(0).toUpperCase()}
    </div>
  );
}

export function TasksPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const [usage, setUsage] = useState<Usage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [sort, setSort] = useState<ProcessSort>('cpu');
  const [q, setQ] = useState('');
  const [pick, setPick] = useState<string | null>(null);
  const visible = usePageVisible();

  const load = useCallback(async () => {
    try {
      setUsage(await client.tasks(sort, 120));
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, [sort]);
  useEffect(() => {
    void load();
  }, [load]);
  useInterval(() => void load(), active && visible ? 2000 : null);

  const shown = useMemo(() => {
    const list = usage?.processes ?? [];
    const n = q.trim().toLowerCase();
    return n ? list.filter((p) => p.name.toLowerCase().includes(n)) : list;
  }, [usage, q]);
  const picked = usage?.processes.find((p) => p.name === pick) ?? null;
  const gpu = usage?.gpus[0];
  const value = (p: ProcessGroup) => (sort === 'memory' ? p.memory : sort === 'gpu' ? p.gpu : p.cpu);
  const max = Math.max(sort === 'memory' ? 1 : 10, ...shown.slice(0, 20).map(value));

  return (
    <div className="flex h-full flex-col">
      <SubHeader title="Tasks" subtitle={usage ? `${usage.processCount} processes running` : 'What is using the PC'} onBack={onBack} />
      <PullToRefresh onRefresh={load} className="min-h-0 flex-1">
        <div className="px-safe pb-tabbar">
          {error && !usage ? (
            <ErrorState message={error} onRetry={() => void load()} />
          ) : (
            <>
              <section className="card p-4">
                <div className="grid grid-cols-3 gap-2">
                  <Meter icon={<Cpu size={13} />} label="CPU" value={usage ? usage.cpu / 100 : null} text={usage ? `${Math.round(usage.cpu)}%` : ''} />
                  <Meter icon={<MemoryStick size={13} />} label="Memory" value={usage ? usage.memoryUsed / Math.max(1, usage.memoryTotal) : null} text={usage ? formatBytes(usage.memoryUsed, 1) : ''} />
                  <Meter icon={<Monitor size={13} />} label="GPU" value={usage ? (usage.gpuSupported && gpu ? gpu.percent / 100 : 0) : null} text={usage ? (usage.gpuSupported && gpu ? `${Math.round(gpu.percent)}%` : '—') : ''} />
                </div>
                {usage && (
                  <div className="mt-3 space-y-1 border-t border-line pt-3 text-[12.5px] text-dim">
                    <div className="truncate">
                      <Cpu size={12} className="mr-1.5 inline text-faint" />
                      {usage.cpuName || `${usage.cores} threads`}
                    </div>
                    <div className="truncate">
                      <MemoryStick size={12} className="mr-1.5 inline text-faint" />
                      {formatBytes(usage.memoryUsed, 1)} of {formatBytes(usage.memoryTotal, 0)} memory in use
                    </div>
                    {usage.gpuSupported && gpu && (
                      <div className="truncate">
                        <Monitor size={12} className="mr-1.5 inline text-faint" />
                        {gpu.name}
                        {gpu.memoryTotal > 0 && ` · ${formatBytes(gpu.memoryUsed, 1)} of ${formatBytes(gpu.memoryTotal, 0)} video memory`}
                      </div>
                    )}
                    <div className="truncate">
                      <HardDrive size={12} className="mr-1.5 inline text-faint" />
                      Disk {rate(usage.disk)}
                    </div>
                  </div>
                )}
              </section>

              <Segmented
                className="mt-4"
                value={sort}
                onChange={(v) => (setSort(v), vibrate(8))}
                options={[
                  { value: 'cpu', label: 'CPU' },
                  { value: 'memory', label: 'Memory' },
                  ...(usage?.gpuSupported ? [{ value: 'gpu' as const, label: 'GPU' }] : []),
                ]}
              />
              <div className="relative mt-3">
                <Search size={16} className="pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-faint" />
                <input className="field h-10 py-0 pl-10 pr-10 text-[15px]" type="search" placeholder="Find a program" value={q} onChange={(e) => setQ(e.target.value)} enterKeyHint="search" />
                {q && (
                  <button aria-label="Clear search" onClick={() => setQ('')} className="absolute right-2 top-1/2 grid h-8 w-8 -translate-y-1/2 place-items-center rounded-full text-faint">
                    <X size={16} />
                  </button>
                )}
              </div>

              <div className="card mt-3 overflow-hidden">
                {!usage ? (
                  Array.from({ length: 7 }, (_, i) => (
                    <div key={i} className="flex items-center gap-3 px-4 py-3">
                      <Skeleton className="h-10 w-10 rounded-xl" />
                      <div className="flex-1 space-y-2">
                        <Skeleton className="h-3.5 w-1/2" />
                        <Skeleton className="h-2.5 w-3/4" />
                      </div>
                    </div>
                  ))
                ) : shown.length === 0 ? (
                  <Empty icon={<Activity size={26} />} title="No matches" body={`Nothing running named like “${q}”.`} />
                ) : (
                  shown.map((p, i) => (
                    <button key={p.name} onClick={() => setPick(p.name)} className={cx('press relative flex w-full items-center gap-3 px-4 py-3 text-left active:bg-surface-2', i > 0 && 'border-t border-line')}>
                      <Avatar name={p.name} />
                      <div className="min-w-0 flex-1">
                        <div className="flex items-baseline gap-1.5">
                          <span className="truncate text-[15px] font-semibold">{short(p.name)}</span>
                          {p.count > 1 && <span className="shrink-0 text-[11.5px] text-faint">×{p.count}</span>}
                        </div>
                        <div className="num mt-0.5 truncate text-[12.5px] text-dim">
                          {p.cpu.toFixed(1)}% CPU · {formatBytes(p.memory, 1)}
                          {usage.gpuSupported && p.gpu > 0 ? ` · GPU ${p.gpu.toFixed(0)}%` : ''}
                        </div>
                        <div className="mt-1.5 h-1 overflow-hidden rounded-full bg-surface-3">
                          <div className="grad-bg h-full rounded-full transition-[width] duration-500" style={{ width: `${Math.min(100, (value(p) / max) * 100)}%` }} />
                        </div>
                      </div>
                    </button>
                  ))
                )}
              </div>
              <p className="mt-3 px-2 text-center text-xs text-faint">Tap a program for details, priority and End task.</p>
            </>
          )}
        </div>
      </PullToRefresh>
      <TaskSheet task={picked} gpuSupported={!!usage?.gpuSupported} onClose={() => setPick(null)} onChanged={() => void load()} />
    </div>
  );
}

function Meter({ icon, label, value, text }: { icon: React.ReactNode; label: string; value: number | null; text: string }) {
  const color = value !== null && value > 0.9 ? '#f87171' : value !== null && value > 0.75 ? '#fbbf24' : undefined;
  return (
    <div className="flex flex-col items-center gap-1.5">
      {value === null ? (
        <Skeleton className="h-[72px] w-[72px] rounded-full" />
      ) : (
        <Ring value={value} size={72} stroke={7} color={color}>
          <span className="num text-[14px] font-bold">{text}</span>
        </Ring>
      )}
      <div className="flex items-center gap-1 text-[12px] font-semibold text-dim">
        {icon}
        {label}
      </div>
    </div>
  );
}

function TaskSheet({ task, gpuSupported, onClose, onChanged }: { task: ProcessGroup | null; gpuSupported: boolean; onClose: () => void; onChanged: () => void }) {
  const [arming, setArming] = useState(false);
  const [busy, setBusy] = useState(false);
  useEffect(() => setArming(false), [task?.name]);

  const end = async () => {
    if (!task) return;
    if (!arming) {
      setArming(true);
      vibrate(10);
      return;
    }
    setBusy(true);
    try {
      const r = await client.endTask(task.name);
      vibrate([12, 40, 12]);
      toast.success(`Ended ${short(task.name)}`, `${r.ended} process${r.ended > 1 ? 'es' : ''} closed on the PC.`);
      onClose();
      onChanged();
    } catch (e) {
      toast.error(`Couldn't end ${short(task.name)}`, errorMessage(e));
    } finally {
      setBusy(false);
      setArming(false);
    }
  };

  const priority = async (p: Priority) => {
    if (!task) return;
    try {
      await client.setTaskPriority(task.name, p);
      vibrate(8);
      toast.success(`${short(task.name)}: ${PRIORITY_NAME[p]}`, 'Until the program restarts.');
      onChanged();
    } catch (e) {
      toast.error("Couldn't change the priority", errorMessage(e));
    }
  };

  return (
    <Sheet
      open={!!task}
      onClose={onClose}
      footer={
        task?.canEnd ? (
          <Button className="w-full" size="lg" variant={arming ? 'danger' : 'ghost'} loading={busy} onClick={() => void end()} icon={<XCircle size={19} />}>
            {arming ? 'Tap again to end it' : 'End task'}
          </Button>
        ) : (
          <div className="rounded-xl bg-surface px-3 py-2.5 text-center text-xs text-faint">Part of Windows (or OmniHub itself) — it can't be ended from here.</div>
        )
      }
    >
      {task && (
        <div className="pb-1">
          <div className="flex items-center gap-3.5">
            <Avatar name={task.name} size={56} />
            <div className="min-w-0 flex-1">
              <div className="truncate font-display text-xl font-bold">{short(task.name)}</div>
              <div className="truncate text-[13px] text-dim">{task.count > 1 ? `${task.count} processes` : '1 process'}</div>
            </div>
          </div>
          <div className="mt-4 grid grid-cols-2 gap-2">
            <Stat label="CPU" value={`${task.cpu.toFixed(1)}%`} />
            <Stat label="Memory" value={formatBytes(task.memory, 1)} />
            <Stat label="GPU" value={gpuSupported ? `${task.gpu.toFixed(1)}%` : '—'} />
            <Stat label="Video memory" value={gpuSupported ? formatBytes(task.gpuMemory, 1) : '—'} />
            <Stat label="Disk" value={rate(task.disk)} />
            <Stat label="Priority" value={task.priority ? PRIORITY_NAME[task.priority] : '—'} />
          </div>
          {task.exe && <div className="mt-3 break-all rounded-xl bg-surface px-3 py-2 font-mono text-[11px] leading-relaxed text-faint">{task.exe}</div>}
          {task.canEnd && (
            <>
              <div className="mb-2 mt-4 text-[12px] font-semibold uppercase tracking-[0.08em] text-faint">Priority</div>
              <Segmented value={task.priority ?? 'normal'} onChange={(v) => void priority(v)} options={PRIORITIES} />
              <p className="mt-2 text-xs leading-relaxed text-faint">High gives it the processor first — handy for a game. Windows resets it when the program restarts.</p>
            </>
          )}
        </div>
      )}
    </Sheet>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-xl bg-surface-2 px-3 py-2.5">
      <div className="text-[11.5px] font-medium text-faint">{label}</div>
      <div className="num mt-0.5 truncate text-[15px] font-semibold">{value}</div>
    </div>
  );
}
