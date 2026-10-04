// Tasks: what is using the PC right now. CPU, memory, GPU and disk for the
// whole PC (with a minute of history) and for every program, grouped by
// name like Task Manager's "Apps" view.

import { formatBytes } from '@shared/format';
import type { Priority, ProcessGroup, ProcessSort, Usage } from '@shared/types';
import type { LucideIcon } from 'lucide-react';
import { ArrowDown, ArrowUp, Check, Copy, Cpu, FolderOpen, Gauge, HardDrive, MemoryStick, Monitor, Pause, Play, Search, XCircle } from 'lucide-react';
import { useMemo, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button } from '../../components/ui/Button';
import { Card, Skeleton } from '../../components/ui/Card';
import { TextInput } from '../../components/ui/Form';
import { type MenuItem, openMenu } from '../../components/ui/Menu';
import { ErrorState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useAsync, useInterval } from '../../lib/hooks';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { Sparkline } from '../home/Sparkline';

const HISTORY = 60;
const POLL_MS = 1500;

export const PRIORITIES: { value: Priority; label: string }[] = [
  { value: 'high', label: 'High' },
  { value: 'aboveNormal', label: 'Above normal' },
  { value: 'normal', label: 'Normal' },
  { value: 'belowNormal', label: 'Below normal' },
  { value: 'low', label: 'Low' },
];
export const priorityLabel = (p: Priority | null) => PRIORITIES.find((x) => x.value === p)?.label ?? '—';

const rate = (b: number) => (b < 1024 ? '0 MB/s' : `${formatBytes(b, 1)}/s`);
const short = (name: string) => name.replace(/\.exe$/i, '');

type Col = { key: ProcessSort | 'gpuMemory' | 'priority'; label: string; title: string; width: string };
const COLS: Col[] = [
  { key: 'cpu', label: 'CPU', title: 'Share of the whole processor', width: 'w-[76px]' },
  { key: 'memory', label: 'Memory', title: 'Memory in use (RAM)', width: 'w-[92px]' },
  { key: 'gpu', label: 'GPU', title: 'Busiest graphics engine', width: 'w-[76px]' },
  { key: 'gpuMemory', label: 'Video mem.', title: 'Dedicated video memory (VRAM)', width: 'w-[96px]' },
  { key: 'disk', label: 'Disk', title: 'Disk reads and writes', width: 'w-[92px]' },
  { key: 'priority', label: 'Priority', title: 'Windows priority', width: 'w-[108px]' },
];

const PRIORITY_RANK: Record<Priority, number> = { low: 0, belowNormal: 1, normal: 2, aboveNormal: 3, high: 4 };

/** Task Manager–style heat: the busier, the warmer the cell. */
function heat(f: number): string | undefined {
  if (!(f > 0.005)) return undefined;
  const pct = Math.round(Math.min(45, 6 + f * 50));
  return `color-mix(in oklab, var(--accent) ${pct}%, transparent)`;
}

const HUES = [262, 199, 330, 152, 28, 220, 285, 175];
function hueFor(name: string) {
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return HUES[h % HUES.length];
}

export function TasksPage() {
  const [usage, setUsage] = useState<Usage | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [paused, setPaused] = useState(false);
  const [q, setQ] = useState('');
  const [sort, setSort] = useState<{ key: Col['key'] | 'name'; desc: boolean }>({ key: 'cpu', desc: true });
  const [selected, setSelected] = useState<string | null>(null);
  const hist = useRef<{ cpu: number[]; mem: number[]; gpu: number[]; disk: number[] }>({ cpu: [], mem: [], gpu: [], disk: [] });

  const poll = async () => {
    try {
      const u = await api.app.usage('cpu', 1000);
      const h = hist.current;
      const push = (a: number[], v: number) => {
        a.push(v);
        if (a.length > HISTORY) a.shift();
      };
      push(h.cpu, u.cpu);
      push(h.mem, u.memoryTotal ? (u.memoryUsed / u.memoryTotal) * 100 : 0);
      push(h.gpu, u.gpus[0]?.percent ?? 0);
      push(h.disk, u.disk);
      setUsage(u);
      setError(null);
    } catch (e) {
      setError(errorText(e));
    }
  };
  useAsync(poll, []);
  useInterval(() => void poll(), paused ? null : POLL_MS);

  const rows = useMemo(() => {
    if (!usage) return [];
    const needle = q.trim().toLowerCase();
    const list = needle ? usage.processes.filter((p) => p.name.toLowerCase().includes(needle) || p.exe?.toLowerCase().includes(needle)) : [...usage.processes];
    const val = (p: ProcessGroup): number | string => {
      switch (sort.key) {
        case 'name':
          return p.name.toLowerCase();
        case 'priority':
          return p.priority ? PRIORITY_RANK[p.priority] : -1;
        default:
          return p[sort.key];
      }
    };
    list.sort((a, b) => {
      const x = val(a);
      const y = val(b);
      const c = typeof x === 'string' ? x.localeCompare(y as string) : x - (y as number);
      return (sort.desc ? -c : c) || b.memory - a.memory;
    });
    return list;
  }, [usage, q, sort]);

  const end = async (p: ProcessGroup) => {
    const ok = await confirm({
      title: `End ${short(p.name)}?`,
      description: `${p.count > 1 ? `All ${p.count} of its processes are` : 'It is'} closed at once. Unsaved work in it is lost.`,
      tone: 'danger',
      confirmLabel: 'End task',
    });
    if (!ok) return;
    try {
      const n = await api.app.endProcess(p.name);
      toast.success(`Ended ${short(p.name)}`, `${n} process${n > 1 ? 'es' : ''} closed.`);
      void poll();
    } catch (e) {
      toast.error(`Could not end ${short(p.name)}`, errorText(e));
    }
  };

  const setPriority = async (p: ProcessGroup, pr: Priority) => {
    try {
      await api.app.setPriority(p.name, pr);
      toast.success(`${short(p.name)}: ${priorityLabel(pr)} priority`, pr === 'high' ? 'Windows gives it the processor first. It goes back to normal when the program restarts.' : 'Until the program restarts.');
      void poll();
    } catch (e) {
      toast.error(`Could not change ${short(p.name)}`, errorText(e));
    }
  };

  const menu = (e: React.MouseEvent, p: ProcessGroup) => {
    setSelected(p.name);
    const items: MenuItem[] = [
      { label: 'End task', icon: XCircle, danger: true, disabled: !p.canEnd, onSelect: () => void end(p) },
      { kind: 'separator' },
      { kind: 'header', label: 'Priority' },
      ...PRIORITIES.map((x): MenuItem => ({ label: x.label, icon: p.priority === x.value ? Check : undefined, disabled: !p.canEnd, onSelect: () => void setPriority(p, x.value) })),
      { kind: 'separator' },
      { label: 'Open file location', icon: FolderOpen, disabled: !p.exe, onSelect: () => p.exe && void api.app.revealPath(p.exe).catch((er: unknown) => toast.error('Could not open the folder', errorText(er))) },
      { label: 'Copy path', icon: Copy, disabled: !p.exe, onSelect: () => p.exe && void navigator.clipboard.writeText(p.exe).then(() => toast.success('Path copied')) },
    ];
    openMenu(e, items);
  };

  const onKey = (e: React.KeyboardEvent) => {
    if (!rows.length) return;
    const i = rows.findIndex((r) => r.name === selected);
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const n = e.key === 'ArrowDown' ? Math.min(rows.length - 1, i + 1) : Math.max(0, i - 1);
      setSelected(rows[n].name);
      document.getElementById(`task-${rows[n].name}`)?.scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'Delete' && i >= 0 && rows[i].canEnd) {
      e.preventDefault();
      void end(rows[i]);
    }
  };

  const by = (key: Col['key'] | 'name') => setSort((s) => (s.key === key ? { key, desc: !s.desc } : { key, desc: key !== 'name' }));
  const memTotal = usage?.memoryTotal || 1;
  const gpu = usage?.gpus[0];

  return (
    <Page
      title="Tasks"
      subtitle={usage ? `${usage.processCount} processes · ${usage.cpuName || `${usage.cores} threads`}` : 'What is using this PC right now'}
      actions={
        <>
          <TextInput icon={Search} inputSize="sm" placeholder="Find a program" value={q} onChange={(e) => setQ(e.target.value)} className="w-56" aria-label="Find a program" />
          <Button size="sm" variant="secondary" icon={paused ? Play : Pause} onClick={() => setPaused(!paused)}>
            {paused ? 'Resume' : 'Pause'}
          </Button>
        </>
      }
    >
      {error && !usage && <ErrorState error={error} onRetry={() => void poll()} />}
      <div className="grid grid-cols-2 gap-3 lg:grid-cols-4">
        <Metric icon={Cpu} label="CPU" value={usage ? `${Math.round(usage.cpu)}%` : null} sub={usage ? `${usage.cores} threads` : ''} values={hist.current.cpu} max={100} />
        <Metric
          icon={MemoryStick}
          label="Memory"
          value={usage ? formatBytes(usage.memoryUsed, 1) : null}
          sub={usage ? `of ${formatBytes(usage.memoryTotal, 0)} · ${Math.round((usage.memoryUsed / memTotal) * 100)}% in use` : ''}
          values={hist.current.mem}
          max={100}
        />
        <Metric
          icon={Monitor}
          label="GPU"
          value={usage ? (usage.gpuSupported && gpu ? `${Math.round(gpu.percent)}%` : '—') : null}
          sub={usage ? (usage.gpuSupported && gpu ? gpu.name : 'GPU figures need Windows 10 (1709) or later') : ''}
          values={hist.current.gpu}
          max={100}
          extra={gpu ? [gpu.memoryTotal ? `Video memory ${formatBytes(gpu.memoryUsed, 1)} of ${formatBytes(gpu.memoryTotal, 0)}` : '', ...(usage?.gpus.slice(1).map((g) => `${g.name}: ${Math.round(g.percent)}%`) ?? [])].filter(Boolean).join(' · ') : undefined}
        />
        <Metric icon={HardDrive} label="Disk" value={usage ? rate(usage.disk) : null} sub="reads + writes, all programs" values={hist.current.disk} max={Math.max(50 * 1024 * 1024, ...hist.current.disk)} />
      </div>

      <Card className="mt-4 overflow-hidden" onKeyDown={onKey}>
        <div className="flex items-center border-b border-line bg-surface/60 px-3 text-[11.5px] font-semibold uppercase tracking-[0.06em] text-faint" role="row">
          <HeaderCell label="Name" title="Program" active={sort.key === 'name'} desc={sort.desc} onClick={() => by('name')} className="flex-1 !justify-start !pl-9" />
          {COLS.map((c) => (
            <HeaderCell key={c.key} label={c.label} title={c.title} active={sort.key === c.key} desc={sort.desc} onClick={() => by(c.key)} className={c.width} />
          ))}
          <div className="w-9" />
        </div>
        <div className="max-h-[calc(100vh-380px)] min-h-[240px] overflow-y-auto" tabIndex={0} role="grid" aria-label="Running programs">
          {!usage && [0, 1, 2, 3, 4, 5].map((i) => <Skeleton key={i} className="mx-3 my-2 h-8" />)}
          {usage && rows.length === 0 && <div className="py-12 text-center text-[13px] text-faint">No program matches “{q}”.</div>}
          {rows.map((p) => {
            const sel = p.name === selected;
            const hue = hueFor(p.name);
            return (
              <div
                key={p.name}
                id={`task-${p.name}`}
                role="row"
                aria-selected={sel}
                onClick={() => setSelected(p.name)}
                onContextMenu={(e) => menu(e, p)}
                className={cx('group flex h-9 cursor-default items-center border-b border-line/50 px-3 text-[13px] last:border-b-0', sel ? 'bg-accent-soft' : 'hover:bg-surface-2/60')}
              >
                <div className="flex min-w-0 flex-1 items-center gap-2.5">
                  <span className="grid h-6 w-6 shrink-0 place-items-center rounded-md text-[11px] font-bold text-white" style={{ background: `linear-gradient(135deg, hsl(${hue} 70% 55%), hsl(${(hue + 40) % 360} 70% 45%))` }} aria-hidden>
                    {short(p.name).charAt(0).toUpperCase()}
                  </span>
                  <span className="truncate text-fg" title={p.exe ?? p.name}>
                    {short(p.name)}
                  </span>
                  {p.count > 1 && <span className="shrink-0 text-[11px] text-faint">×{p.count}</span>}
                </div>
                <Cell width="w-[76px]" f={p.cpu / 100}>{p.cpu.toFixed(1)}%</Cell>
                <Cell width="w-[92px]" f={p.memory / memTotal}>{formatBytes(p.memory, 1)}</Cell>
                <Cell width="w-[76px]" f={p.gpu / 100}>{usage?.gpuSupported ? `${p.gpu.toFixed(1)}%` : '—'}</Cell>
                <Cell width="w-[96px]" f={gpu?.memoryTotal ? p.gpuMemory / gpu.memoryTotal : 0}>{usage?.gpuSupported ? (p.gpuMemory ? formatBytes(p.gpuMemory, 1) : '0 MB') : '—'}</Cell>
                <Cell width="w-[92px]" f={p.disk / (20 * 1024 * 1024)}>{rate(p.disk)}</Cell>
                <div className={cx('w-[108px] text-right text-[12px]', p.priority === 'high' || p.priority === 'aboveNormal' ? 'text-accent' : p.priority === 'low' || p.priority === 'belowNormal' ? 'text-faint' : 'text-dim')}>{priorityLabel(p.priority)}</div>
                <div className="flex w-9 justify-end">
                  {p.canEnd && (
                    <button type="button" onClick={(e) => (e.stopPropagation(), void end(p))} className={cx('text-faint transition-opacity hover:text-bad focus-visible:opacity-100', sel ? 'opacity-100' : 'opacity-0 group-hover:opacity-100')} aria-label={`End ${short(p.name)}`} title="End task">
                      <XCircle size={15} />
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
        <div className="flex items-center gap-2 border-t border-line px-4 py-2 text-[11.5px] text-faint">
          <Gauge size={13} />
          Right-click a program for priority and its file location. Windows' own processes can't be ended here.
          {paused && <span className="ml-auto text-warn">Paused</span>}
        </div>
      </Card>
    </Page>
  );
}

function Metric({ icon: Icon, label, value, sub, values, max, extra }: { icon: LucideIcon; label: string; value: string | null; sub: string; values: number[]; max: number; extra?: string }) {
  return (
    <Card className="flex flex-col gap-2 p-4">
      <div className="flex items-center gap-2 text-[12px] font-semibold uppercase tracking-[0.08em] text-faint">
        <Icon size={14} /> {label}
      </div>
      {value === null ? <Skeleton className="h-8 w-24" /> : <div className="font-display text-[26px] font-semibold leading-none tabular text-fg">{value}</div>}
      <div className="truncate text-[12px] text-dim" title={sub}>
        {sub || ' '}
      </div>
      <Sparkline values={values} max={max} height={44} label={label} />
      {extra && <div className="truncate text-[11.5px] text-faint" title={extra}>{extra}</div>}
    </Card>
  );
}

function HeaderCell({ label, title, active, desc, onClick, className }: { label: string; title: string; active: boolean; desc: boolean; onClick: () => void; className?: string }) {
  const Arrow = desc ? ArrowDown : ArrowUp;
  return (
    <button type="button" onClick={onClick} title={title} className={cx('flex h-9 items-center justify-end gap-1 px-2 hover:text-dim', active && 'text-fg', className)} aria-sort={active ? (desc ? 'descending' : 'ascending') : 'none'}>
      {label}
      {active && <Arrow size={12} />}
    </button>
  );
}

function Cell({ width, f, children }: { width: string; f: number; children: React.ReactNode }) {
  return (
    <div className={cx('flex h-full items-center justify-end px-2 text-right tabular text-dim', width)} style={{ background: heat(f) }}>
      {children}
    </div>
  );
}
