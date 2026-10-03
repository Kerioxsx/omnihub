import { formatBytes, formatRelative } from '@shared/format';
import type { Note, SystemStats, VolumeInfo } from '@shared/types';
import { motion } from 'motion/react';
import type { LucideIcon } from 'lucide-react';
import { ArrowRight, Camera, Cpu, Crop, HardDrive, Lightbulb, LockKeyhole, MemoryStick, QrCode, Send, Smartphone, StickyNote, Usb, Zap } from 'lucide-react';
import { useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { PowerBanner } from '../../components/PowerBanner';
import { ShotThumb } from '../../components/ShotThumb';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Dot, SectionTitle, Skeleton } from '../../components/ui/Card';
import { Switch } from '../../components/ui/Form';
import { ProgressRing } from '../../components/ui/Progress';
import { EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useAsync, useEvent, useInterval } from '../../lib/hooks';
import { navigate } from '../../lib/router';
import { formatUptime, greeting, plainSnippet } from '../../lib/util';
import { connectedCount, useLive } from '../../state/live';
import { useSettings } from '../../state/settings';
import { jobFraction, rootKey, useStorage } from '../../state/storage';
import { toast } from '../../state/toasts';
import { Sparkline } from './Sparkline';

const stagger = { hidden: {}, show: { transition: { staggerChildren: 0.05 } } };
const rise = { hidden: { opacity: 0, y: 12 }, show: { opacity: 1, y: 0, transition: { type: 'spring' as const, stiffness: 260, damping: 26 } } };

function DriveCard({ v }: { v: VolumeInfo }) {
  const summaries = useStorage((s) => s.summaries);
  const job = useStorage((s) => s.job);
  const volumes = useStorage((s) => s.volumes);
  const used = v.total - v.free;
  const f = v.total ? used / v.total : 0;
  const s = summaries[rootKey(v.root)];
  const scanning = job && rootKey(job.root) === rootKey(v.root);
  const jf = scanning && job ? jobFraction(job, volumes) : null;
  const scan = () => {
    navigate('storage', { root: v.root });
    void useStorage.getState().scan(v.root, v.mftCapable ? 'fast' : 'standard');
  };
  return (
    <motion.div variants={rise}>
      <Card interactive className="flex h-full flex-col gap-3 p-4">
        <div className="flex items-center gap-3.5">
          <ProgressRing value={scanning ? jf : f} size={62} stroke={6} color={!scanning && f > 0.9 ? 'var(--bad)' : !scanning && f > 0.8 ? 'var(--warn)' : undefined} label={`${v.root} used`}>
            <span className="text-[13px] font-semibold tabular text-fg">{scanning ? (jf != null ? `${Math.round(jf * 100)}%` : '…') : `${Math.round(f * 100)}%`}</span>
          </ProgressRing>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-1.5">
              {v.kind === 'removable' ? <Usb size={14} className="text-faint" aria-hidden /> : <HardDrive size={14} className="text-faint" aria-hidden />}
              <span className="truncate text-[14px] font-semibold text-fg">
                {v.root.replace(/\\$/, '')} {v.label}
              </span>
            </div>
            <div className="mt-1 text-[13px] font-medium text-fg tabular">{formatBytes(v.free)} free</div>
            <div className="text-[12px] text-dim tabular">
              of {formatBytes(v.total)} · {v.fileSystem}
            </div>
          </div>
        </div>
        <div className="flex items-center gap-1.5 text-[11.5px] text-faint">
          <span className={cx('h-1.5 w-1.5 rounded-full', scanning ? 'bg-accent' : s ? 'bg-good' : 'bg-faint')} />
          {scanning ? 'Scanning now…' : s ? `Scanned ${formatRelative(s.scannedAt)}${s.fromCache ? ' (cached)' : ''}` : 'Not scanned yet'}
        </div>
        <div className="mt-auto flex gap-2">
          <Button size="sm" variant="secondary" icon={v.mftCapable ? Zap : HardDrive} onClick={scan} disabled={!!job} className="flex-1">
            Scan
          </Button>
          <Button size="sm" variant="ghost" iconRight={ArrowRight} onClick={() => navigate('storage', { root: v.root })}>
            Open
          </Button>
        </div>
      </Card>
    </motion.div>
  );
}

function SystemCard() {
  const [stats, setStats] = useState<SystemStats | null>(null);
  const [cpuHist, setCpuHist] = useState<number[]>([]);
  const poll = async () => {
    try {
      const s = await api.app.stats();
      setStats(s);
      setCpuHist((h) => [...h.slice(-29), s.cpu]);
    } catch {
      /* keep the last values */
    }
  };
  useInterval(() => void poll(), 3000);
  useAsync(poll, []);
  const mem = stats ? stats.memory.used / stats.memory.total : 0;
  return (
    <Card className="flex h-full flex-col p-4">
      <CardHeader icon={Cpu} title="This PC" subtitle={stats ? stats.os : 'Loading…'} />
      {stats ? (
        <>
          <div className="mt-4 grid grid-cols-2 gap-3">
            <div className="flex items-center gap-3">
              <ProgressRing value={stats.cpu / 100} size={48} stroke={5} label="CPU usage">
                <Cpu size={15} className="text-dim" aria-hidden />
              </ProgressRing>
              <div>
                <div className="text-[11px] uppercase tracking-wider text-faint">CPU</div>
                <div className="font-display text-[19px] font-semibold tabular text-fg">{Math.round(stats.cpu)}%</div>
              </div>
            </div>
            <div className="flex items-center gap-3">
              <ProgressRing value={mem} size={48} stroke={5} label="Memory usage">
                <MemoryStick size={15} className="text-dim" aria-hidden />
              </ProgressRing>
              <div>
                <div className="text-[11px] uppercase tracking-wider text-faint">Memory</div>
                <div className="font-display text-[19px] font-semibold tabular text-fg">
                  {formatBytes(stats.memory.used, 1).replace(' GB', '')}
                  <span className="text-[12px] font-normal text-faint"> / {formatBytes(stats.memory.total, 0)}</span>
                </div>
              </div>
            </div>
          </div>
          <div className="mt-auto pt-3">
            <Sparkline values={cpuHist} max={100} height={34} label="CPU over the last 90 seconds" />
            <div className="mt-1 flex justify-between text-[11px] text-faint">
              <span>CPU · last 90 s</span>
              <span>Up {formatUptime(stats.uptime)}</span>
            </div>
          </div>
        </>
      ) : (
        <div className="mt-4 space-y-3">
          <Skeleton className="h-12" />
          <Skeleton className="h-10" />
        </div>
      )}
    </Card>
  );
}

interface Action {
  icon: LucideIcon;
  title: string;
  hint: string;
  run: () => void | Promise<void>;
  grad: string;
}

function QuickActions() {
  const vault = useLive((s) => s.vault);
  const volumes = useStorage((s) => s.volumes);
  const sys = volumes.find((v) => v.root.toUpperCase().startsWith('C:')) ?? volumes[0];
  const actions: Action[] = [
    {
      icon: Zap,
      title: `Scan ${sys ? sys.root.replace(/\\$/, '') : 'C:'}`,
      hint: 'Fast MFT scan',
      grad: 'from-violet-500/25 to-violet-500/0 text-violet-300 light:text-violet-600',
      run: () => {
        if (!sys) return;
        navigate('storage', { root: sys.root });
        void useStorage.getState().scan(sys.root, sys.mftCapable ? 'fast' : 'standard');
      },
    },
    {
      icon: Crop,
      title: 'Screenshot',
      hint: 'Capture a region',
      grad: 'from-cyan-500/25 to-cyan-500/0 text-cyan-300 light:text-cyan-600',
      run: async () => {
        try {
          await api.shots.regionBegin();
          if (!('__TAURI_INTERNALS__' in window)) navigate('overlay');
        } catch (e) {
          toast.error('Could not start the capture', errorText(e));
        }
      },
    },
    { icon: Lightbulb, title: 'New idea', hint: 'Write it down for Claude', grad: 'from-amber-500/25 to-amber-500/0 text-amber-300 light:text-amber-600', run: () => navigate('notes', { new: 'idea' }) },
    { icon: QrCode, title: 'Pair phone', hint: 'QR code + PIN', grad: 'from-emerald-500/25 to-emerald-500/0 text-emerald-300 light:text-emerald-600', run: () => navigate('phone', { pair: '1' }) },
    {
      icon: LockKeyhole,
      title: vault?.unlocked ? 'Lock vault' : 'Open vault',
      hint: vault?.exists ? (vault.unlocked ? 'Lock it now' : 'Locked') : 'Set up encryption',
      grad: 'from-rose-500/25 to-rose-500/0 text-rose-300 light:text-rose-600',
      run: async () => {
        if (vault?.unlocked) {
          await api.vault.lock();
          toast.info('Vault locked');
        } else navigate('vault');
      },
    },
  ];
  return (
    <motion.div variants={rise} className="grid grid-cols-5 gap-3">
      {actions.map((a) => (
        <motion.button
          key={a.title}
          type="button"
          onClick={() => void a.run()}
          whileHover={{ y: -2 }}
          whileTap={{ scale: 0.98 }}
          transition={{ type: 'spring', stiffness: 400, damping: 28 }}
          className="card card-interactive group flex items-center gap-3 px-4 py-3.5 text-left"
        >
          <span className={cx('flex h-10 w-10 shrink-0 items-center justify-center rounded-xl border border-line bg-gradient-to-br', a.grad)}>
            <a.icon size={18} aria-hidden />
          </span>
          <span className="min-w-0">
            <span className="block truncate text-[13.5px] font-semibold text-fg">{a.title}</span>
            <span className="block truncate text-[12px] text-faint">{a.hint}</span>
          </span>
        </motion.button>
      ))}
    </motion.div>
  );
}

function NotesCard() {
  const notes = useAsync(() => api.notes.list({ limit: 5 }), []);
  useEvent('notes:changed', () => void notes.reload());
  const list = notes.data ?? [];
  const row = (n: Note) => (
    <button key={n.id} type="button" onClick={() => navigate('notes', { id: n.id, tab: n.kind })} className="group flex w-full items-start gap-3 rounded-xl px-2.5 py-2 text-left transition-colors hover:bg-surface-2">
      <span className={cx('mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-lg', n.kind === 'idea' ? 'bg-warn/12 text-warn' : 'bg-info/12 text-info')}>{n.kind === 'idea' ? <Lightbulb size={14} aria-hidden /> : <StickyNote size={14} aria-hidden />}</span>
      <span className="min-w-0 flex-1">
        <span className="flex items-center gap-2">
          <span className="truncate text-[13.5px] font-medium text-fg">{n.title || 'Untitled'}</span>
          {n.exportedAt && (
            <Badge tone="accent" icon={Send} className="h-[18px] text-[10.5px]">
              Sent
            </Badge>
          )}
        </span>
        <span className="block truncate text-[12px] text-faint">{plainSnippet(n.body) || 'Empty'}</span>
      </span>
      <span className="shrink-0 pt-0.5 text-[11.5px] text-faint">{formatRelative(n.updated)}</span>
    </button>
  );
  return (
    <Card className="flex h-full flex-col p-4">
      <CardHeader
        icon={Lightbulb}
        title="Notes & ideas"
        subtitle="Recently edited"
        actions={
          <Button size="xs" variant="ghost" iconRight={ArrowRight} onClick={() => navigate('notes')}>
            All notes
          </Button>
        }
      />
      <div className="mt-2 flex-1 space-y-0.5">
        {notes.data ? list.length ? list.map(row) : <EmptyState icon={StickyNote} title="No notes yet" compact /> : [0, 1, 2, 3].map((i) => <Skeleton key={i} className="my-1 h-11" />)}
      </div>
    </Card>
  );
}

function PhoneCard() {
  const remote = useLive((s) => s.remote);
  const devices = useLive((s) => s.devices);
  const viewers = useLive((s) => s.viewers);
  const update = useSettings((s) => s.update);
  const running = !!remote?.running;
  const n = connectedCount(remote, devices, viewers);
  const paired = devices.filter((d) => !d.revoked);
  return (
    <Card className="flex h-full flex-col p-4">
      <CardHeader icon={Smartphone} title="Phone companion" subtitle={running ? `${remote?.tls ? 'HTTPS' : 'HTTP'} on port ${remote?.port}` : 'Off — nothing is listening'} actions={<Switch checked={running} onChange={(v) => void update({ remote: { enabled: v } })} label="Phone companion" />} />
      <div className="mt-3 flex items-center gap-2 rounded-xl border border-line bg-surface px-3 py-2">
        <Dot tone={running ? (n ? 'good' : 'accent') : 'neutral'} pulse={running && n > 0} />
        <span className="text-[13px] text-fg">{running ? (n ? `${n} phone${n > 1 ? 's' : ''} connected` : 'Waiting for a phone') : 'Companion is off'}</span>
        {running && remote?.urls[0] && <span className="ml-auto truncate font-mono text-[11.5px] text-faint">{remote.urls[0].replace(/^https?:\/\//, '')}</span>}
      </div>
      <div className="mt-2 flex-1 space-y-0.5">
        {paired.slice(0, 3).map((d) => {
          const online = running && Date.now() / 1000 - d.lastSeen < 600;
          return (
            <div key={d.id} className="flex items-center gap-2.5 rounded-lg px-2 py-1.5">
              <Smartphone size={15} className="text-faint" aria-hidden />
              <span className="flex-1 truncate text-[13px] text-fg">{d.name}</span>
              <span className={cx('text-[11.5px]', online ? 'text-good' : 'text-faint')}>{online ? 'online' : formatRelative(d.lastSeen)}</span>
            </div>
          );
        })}
        {!paired.length && <div className="px-2 py-3 text-[12.5px] text-faint">No phones paired yet.</div>}
      </div>
      {!running && <p className="mt-2 text-[12px] leading-relaxed text-faint">Turn it on to browse files, send photos, control power and watch your screen from any phone browser on your Wi-Fi.</p>}
      <Button size="sm" variant={running ? 'secondary' : 'primary'} icon={QrCode} className="mt-2" onClick={() => navigate('phone', { pair: '1' })}>
        Pair a phone
      </Button>
    </Card>
  );
}

function ShotsStrip() {
  const shots = useAsync(() => api.shots.list({ limit: 12 }), []);
  useEvent('screenshots:new', () => void shots.reload());
  useEvent('screenshots:deleted', () => void shots.reload());
  const list = shots.data ?? [];
  return (
    <motion.div variants={rise}>
      <SectionTitle
        actions={
          <Button size="xs" variant="ghost" iconRight={ArrowRight} onClick={() => navigate('screenshots')}>
            Gallery
          </Button>
        }
      >
        Recent screenshots
      </SectionTitle>
      <div className="fade-x -mx-2 flex gap-3 overflow-x-auto px-2 pb-2">
        {shots.data
          ? list.map((s) => (
              <button key={s.id} type="button" onClick={() => navigate('screenshots', { open: s.id })} className="group card card-interactive w-[196px] shrink-0 overflow-hidden p-0 text-left">
                <ShotThumb id={s.id} alt={s.appTitle ?? 'Screenshot'} className="aspect-video w-full" exists={s.exists} />
                <div className="px-2.5 py-2">
                  <div className="truncate text-[12px] font-medium text-fg">{s.appTitle ?? 'Full screen'}</div>
                  <div className="text-[11px] text-faint">{formatRelative(s.created)}</div>
                </div>
              </button>
            ))
          : [0, 1, 2, 3, 4].map((i) => <Skeleton key={i} className="h-[150px] w-[196px] shrink-0 rounded-2xl" />)}
        {shots.data && !list.length && (
          <Card className="flex w-full items-center gap-3 px-4 py-6 text-[13px] text-dim">
            <Camera size={18} className="text-faint" aria-hidden /> No screenshots yet — press Alt+Shift+S to capture a region.
          </Card>
        )}
      </div>
    </motion.div>
  );
}

export function HomePage() {
  const volumes = useStorage((s) => s.volumes);
  const loaded = useStorage((s) => s.volumesLoaded);
  const info = useSettings((s) => s.info);
  const power = useLive((s) => s.power);
  const date = useMemo(() => new Date().toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long' }), []);
  return (
    <Page title={<span className="gradient-text">{greeting()}, Alex</span>} subtitle={`${date} · ${info?.hostname ?? ''}`}>
      <motion.div variants={stagger} initial="hidden" animate="show" className="space-y-6">
        {power && <PowerBanner inline />}
        <div className="grid grid-cols-12 gap-3">
          <motion.div variants={stagger} className="col-span-8 grid grid-cols-3 gap-3">
            {loaded ? volumes.map((v) => <DriveCard key={v.root} v={v} />) : [0, 1, 2].map((i) => <Skeleton key={i} className="h-[140px] rounded-2xl" />)}
          </motion.div>
          <motion.div variants={rise} className="col-span-4">
            <SystemCard />
          </motion.div>
        </div>
        <QuickActions />
        <motion.div variants={rise} className="grid grid-cols-12 gap-3">
          <div className="col-span-7">
            <NotesCard />
          </div>
          <div className="col-span-5">
            <PhoneCard />
          </div>
        </motion.div>
        <ShotsStrip />
      </motion.div>
    </Page>
  );
}
