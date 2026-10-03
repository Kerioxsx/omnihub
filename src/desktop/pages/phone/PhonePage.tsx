import { formatBytes, formatDateTime, formatRelative } from '@shared/format';
import type { AuditEntry, Device } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { ArrowDownToLine, ArrowUpFromLine, Check, CircleCheck, CircleX, Copy, Eye, FolderSearch, Globe, History, Lock, MonitorUp, Pencil, QrCode, Send, ShieldCheck, Smartphone, Tablet, Trash, Unlock, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Dot, Skeleton } from '../../components/ui/Card';
import { Select, Switch, TextInput } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { ProgressBar } from '../../components/ui/Progress';
import { EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useAsync, useEvent, useNow } from '../../lib/hooks';
import { navigate, useRoute } from '../../lib/router';
import { copyText } from '../../lib/util';
import { confirm } from '../../state/dialogs';
import { useLive } from '../../state/live';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';
import { PairDialog } from './PairDialog';
import { Permissions } from './Permissions';

function deviceIcon(ua: string) {
  return /iPad|Tablet|SM-T|Tab/i.test(ua) ? Tablet : Smartphone;
}

function Hero({ onPair, onSend }: { onPair: () => void; onSend: () => void }) {
  const remote = useLive((s) => s.remote);
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const [busy, setBusy] = useState(false);
  const running = !!remote?.running;
  const toggle = async (v: boolean) => {
    setBusy(true);
    await update({ remote: { enabled: v } });
    setBusy(false);
  };
  return (
    <Card className="relative overflow-hidden p-6">
      <div className={cx('pointer-events-none absolute -right-24 -top-32 h-80 w-80 rounded-full transition-opacity duration-700', running ? 'opacity-100' : 'opacity-0')} style={{ background: 'radial-gradient(circle, color-mix(in oklab, var(--good) 22%, transparent), transparent 70%)' }} />
      <div className="relative flex items-center gap-6">
        <div className={cx('relative flex h-16 w-16 shrink-0 items-center justify-center rounded-[22px] border transition-colors duration-300', running ? 'border-good/30 bg-good/12 text-good' : 'border-line bg-surface-2 text-faint')}>
          <Smartphone size={30} aria-hidden />
          {running && <span className="absolute -right-1 -top-1 h-4 w-4 rounded-full border-[3px] border-[var(--bg-elev)] bg-good" />}
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-3">
            <h2 className="font-display text-[20px] font-semibold tracking-tight text-fg">{running ? 'Companion is running' : 'Companion is off'}</h2>
            {running && (
              <Badge tone={remote?.tls ? 'good' : 'warn'} icon={remote?.tls ? Lock : Unlock}>
                {remote?.tls ? 'HTTPS' : 'HTTP'}
              </Badge>
            )}
          </div>
          <p className="mt-1 text-[13.5px] text-dim">{running ? `Phones on your network can reach ${settings?.remote.deviceName ?? 'this PC'} at:` : 'Nothing is listening. Turn it on to pair a phone, send files, or watch your screen from a phone browser.'}</p>
          {running && remote && (
            <div className="mt-3 flex flex-wrap items-center gap-2">
              {remote.urls.map((u) => (
                <span key={u} className="inline-flex items-center gap-1 rounded-lg border border-line bg-surface py-0.5 pl-2.5 pr-0.5">
                  <Globe size={13} className="text-faint" aria-hidden />
                  <span className="font-mono text-[12.5px] text-fg">{u}</span>
                  <IconButton icon={Copy} label="Copy address" size="sm" onClick={() => void copyText(u).then(() => toast.success('Address copied'))} />
                </span>
              ))}
              {remote.fingerprint && (
                <span className="inline-flex items-center gap-1.5 text-[11.5px] text-faint" title={remote.fingerprint}>
                  <ShieldCheck size={13} aria-hidden /> SHA-256 <span className="font-mono">{remote.fingerprint.slice(0, 17)}…</span>
                </span>
              )}
            </div>
          )}
        </div>
        <div className="flex flex-col items-end gap-3">
          <Switch size="lg" checked={running} onChange={(v) => void toggle(v)} label="Phone companion" disabled={busy} />
          <div className="flex gap-2">
            <Button icon={Send} onClick={onSend} disabled={!running}>
              Send files
            </Button>
            <Button variant="primary" icon={QrCode} onClick={onPair}>
              Pair a phone
            </Button>
          </div>
        </div>
      </div>
    </Card>
  );
}

function DeviceRow({ d, running }: { d: Device; running: boolean }) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(d.name);
  useEffect(() => setName(d.name), [d.name]);
  const Icon = deviceIcon(d.userAgent);
  const online = running && Date.now() / 1000 - d.lastSeen < 600;
  const save = async () => {
    setEditing(false);
    if (name.trim() && name !== d.name) await api.remote.renameDevice(d.id, name.trim()).catch((e: unknown) => toast.error('Could not rename', errorText(e)));
  };
  const revoke = async () => {
    const ok = await confirm({ title: `Revoke ${d.name}?`, description: 'It is signed out immediately and must pair again with a new PIN to reconnect.', tone: 'danger', confirmLabel: 'Revoke access' });
    if (ok) await api.remote.revokeDevice(d.id).then(() => toast.success(`${d.name} revoked`), (e: unknown) => toast.error('Could not revoke', errorText(e)));
  };
  return (
    <motion.div layout initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, height: 0 }} className="group flex items-center gap-3 rounded-xl px-3 py-2.5 hover:bg-surface-2">
      <span className={cx('relative flex h-10 w-10 shrink-0 items-center justify-center rounded-xl', online ? 'bg-good/12 text-good' : 'bg-surface-3 text-dim')}>
        <Icon size={18} aria-hidden />
        {online && <span className="absolute -right-0.5 -top-0.5 h-3 w-3 rounded-full border-2 border-[var(--bg-elev)] bg-good" />}
      </span>
      <div className="min-w-0 flex-1">
        {editing ? (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void save();
            }}
          >
            <TextInput value={name} onChange={(e) => setName(e.target.value)} onBlur={() => void save()} inputSize="sm" autoFocus aria-label="Device name" />
          </form>
        ) : (
          <div className="flex items-center gap-1.5">
            <span className="truncate text-[13.5px] font-medium text-fg">{d.name}</span>
            <button type="button" onClick={() => setEditing(true)} aria-label={`Rename ${d.name}`} className="text-faint opacity-0 transition-opacity hover:text-fg group-hover:opacity-100">
              <Pencil size={12} />
            </button>
          </div>
        )}
        <div className="truncate text-[12px] text-faint">
          {online ? <span className="text-good">Online</span> : `Last seen ${formatRelative(d.lastSeen)}`} · {d.lastIp} · paired {formatRelative(d.created)}
        </div>
      </div>
      <Button size="xs" variant="ghost" icon={Trash} onClick={() => void revoke()} className="opacity-0 group-hover:opacity-100 focus-visible:opacity-100">
        Revoke
      </Button>
    </motion.div>
  );
}

function Devices({ onPair }: { onPair: () => void }) {
  const devices = useLive((s) => s.devices);
  const running = useLive((s) => !!s.remote?.running);
  const list = devices.filter((d) => !d.revoked);
  return (
    <Card className="p-5">
      <CardHeader icon={Smartphone} title="Paired phones" subtitle={`${list.length} device${list.length === 1 ? '' : 's'} with access`} />
      <div className="mt-3">
        <AnimatePresence initial={false}>
          {list.map((d) => (
            <DeviceRow key={d.id} d={d} running={running} />
          ))}
        </AnimatePresence>
        {!list.length && <EmptyState compact icon={QrCode} title="No phones yet" action={<Button size="sm" variant="primary" onClick={onPair}>Pair a phone</Button>} />}
      </div>
    </Card>
  );
}

function Transfers() {
  const transfers = useLive((s) => s.transfers);
  const clear = useLive((s) => s.clearFinished);
  useNow(1000);
  return (
    <Card className="p-5">
      <CardHeader
        icon={ArrowDownToLine}
        title="Transfers"
        subtitle="Files between this PC and your phones"
        actions={
          transfers.some((t) => t.state !== 'running') ? (
            <Button size="xs" variant="ghost" onClick={clear}>
              Clear finished
            </Button>
          ) : undefined
        }
      />
      <div className="mt-3 space-y-2">
        <AnimatePresence initial={false}>
          {transfers.map((t) => {
            const up = t.direction === 'upload';
            const f = t.size ? t.done / t.size : null;
            return (
              <motion.div key={t.id} layout initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} className="rounded-xl border border-line bg-surface px-3.5 py-2.5">
                <div className="flex items-center gap-2.5">
                  <span className={cx('flex h-7 w-7 items-center justify-center rounded-lg', up ? 'bg-info/12 text-info' : 'bg-accent-soft text-accent')}>{up ? <ArrowDownToLine size={14} /> : <ArrowUpFromLine size={14} />}</span>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[13px] font-medium text-fg">{t.name}</div>
                    <div className="text-[11.5px] text-faint tabular">
                      {up ? `From ${t.device ?? 'phone'}` : `To ${t.device ?? 'phone'}`} · {formatBytes(t.done)} of {formatBytes(t.size)}
                      {t.state === 'running' && t.speed > 0 && ` · ${formatBytes(t.speed)}/s`}
                    </div>
                  </div>
                  {t.state === 'done' ? (
                    up && t.path ? (
                      <IconButton icon={FolderSearch} label="Show in folder" size="sm" onClick={() => void api.app.revealPath(t.path ?? '')} />
                    ) : (
                      <Check size={16} className="text-good" aria-label="Done" />
                    )
                  ) : t.state === 'cancelled' ? (
                    <X size={16} className="text-bad" aria-label="Cancelled" />
                  ) : (
                    <span className="text-[12px] font-medium tabular text-dim">{f != null ? `${Math.round(f * 100)}%` : ''}</span>
                  )}
                </div>
                {t.state === 'running' && <ProgressBar value={f} className="mt-2" height={4} label={`Transfer of ${t.name}`} />}
              </motion.div>
            );
          })}
        </AnimatePresence>
        {!transfers.length && <div className="rounded-xl border border-dashed border-line px-4 py-5 text-center text-[12.5px] text-faint">No transfers yet. Files sent from a phone land in your incoming folder.</div>}
      </div>
    </Card>
  );
}

function Viewers() {
  const viewers = useLive((s) => s.viewers);
  const now = useNow(1000);
  if (!viewers.length) return null;
  return (
    <motion.div initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }}>
      <Card className="border-accent/30 p-5">
        <CardHeader
          icon={Eye}
          title="Watching your screen"
          subtitle={`${viewers.length} active viewer${viewers.length > 1 ? 's' : ''}`}
          actions={
            viewers.length > 1 ? (
              <Button size="xs" variant="danger" onClick={() => void api.remote.stopAllViewers()}>
                Stop all
              </Button>
            ) : undefined
          }
        />
        <div className="mt-3 space-y-2">
          {viewers.map((v) => (
            <div key={v.id} className="flex items-center gap-3 rounded-xl border border-line bg-surface px-3.5 py-2.5">
              <Dot tone="bad" pulse />
              <div className="min-w-0 flex-1">
                <div className="text-[13px] font-medium text-fg">
                  {v.device} {v.controlling && <Badge tone="warn">controlling</Badge>}
                </div>
                <div className="text-[11.5px] text-faint tabular">
                  {v.monitor ? `${v.monitor.name} · ${v.monitor.width}×${v.monitor.height}` : 'All screens'} · for {Math.max(1, Math.round((now / 1000 - v.since) / 60))} min
                </div>
              </div>
              <Button size="sm" variant="danger" icon={MonitorUp} onClick={() => void api.remote.stopViewer(v.id).then(() => toast.success('Stopped sharing with ' + v.device))}>
                Stop sharing
              </Button>
            </div>
          ))}
        </div>
      </Card>
    </motion.div>
  );
}

function AuditLog() {
  const log = useAsync(() => api.app.audit(100), []);
  const [fresh, setFresh] = useState<Set<number>>(new Set());
  useEvent<AuditEntry>('audit:new', (e) => {
    log.setData((prev) => [e, ...(prev ?? [])].slice(0, 200));
    setFresh((s) => new Set(s).add(e.id));
  });
  return (
    <Card className="p-5">
      <CardHeader icon={History} title="Activity" subtitle="Everything phones did, and security events" actions={<Button size="xs" variant="ghost" onClick={() => navigate('settings', { section: 'privacy' })}>Manage</Button>} />
      <div className="mt-3 max-h-[360px] space-y-0.5 overflow-y-auto pr-1">
        {!log.data
          ? [0, 1, 2, 3].map((i) => <Skeleton key={i} className="my-1 h-10" />)
          : log.data.map((e) => (
              <motion.div key={e.id} initial={fresh.has(e.id) ? { opacity: 0, x: -8, backgroundColor: 'var(--accent-soft)' } : false} animate={{ opacity: 1, x: 0, backgroundColor: 'rgba(0,0,0,0)' }} transition={{ duration: 0.8 }} className="flex items-start gap-2.5 rounded-lg px-2 py-1.5">
                {e.ok ? <CircleCheck size={14} className="mt-0.5 shrink-0 text-good" aria-label="Allowed" /> : <CircleX size={14} className="mt-0.5 shrink-0 text-bad" aria-label="Failed" />}
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[12.5px] text-fg">
                    <span className="font-medium">{e.actor}</span> <span className="font-mono text-[11.5px] text-dim">{e.action}</span>
                  </div>
                  <div className="truncate text-[11.5px] text-faint">{e.detail}</div>
                </div>
                <span className="shrink-0 text-[11px] text-faint" title={formatDateTime(e.at)}>
                  {formatRelative(e.at)}
                </span>
              </motion.div>
            ))}
        {log.data && !log.data.length && <div className="py-6 text-center text-[12.5px] text-faint">No activity yet.</div>}
      </div>
    </Card>
  );
}

function SendDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const devices = useLive((s) => s.devices).filter((d) => !d.revoked);
  const [paths, setPaths] = useState<string[]>([]);
  const [target, setTarget] = useState('');
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    if (!open) return;
    setPaths([]);
    void api.app.pickFiles('Choose files to send').then(setPaths, (e: unknown) => toast.error('Could not open the file picker', errorText(e)));
  }, [open]);
  const send = async () => {
    setBusy(true);
    try {
      const items = await api.remote.send(paths, target || null);
      toast.success(`Offered ${items.length} file${items.length === 1 ? '' : 's'}`, `${target ? devices.find((d) => d.id === target)?.name : 'Every phone'} sees them in the companion inbox.`);
      onClose();
    } catch (e) {
      toast.error('Could not send', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Modal
      open={open}
      onClose={onClose}
      title="Send files to a phone"
      icon={Send}
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" icon={Send} disabled={!paths.length} loading={busy} onClick={() => void send()}>
            Send {paths.length || ''}
          </Button>
        </>
      }
    >
      <div className="space-y-3 pb-2">
        <div className="space-y-1.5">
          {paths.map((p) => (
            <div key={p} className="flex items-center gap-2 rounded-lg border border-line bg-surface py-1 pl-3 pr-1">
              <span className="min-w-0 flex-1 truncate font-mono text-[12px] text-dim">{p}</span>
              <IconButton icon={X} label="Remove" size="sm" onClick={() => setPaths(paths.filter((x) => x !== p))} />
            </div>
          ))}
          {!paths.length && <div className="text-[13px] text-faint">No files chosen.</div>}
          <Button size="xs" onClick={() => void api.app.pickFiles('Choose files to send').then((more) => setPaths([...new Set([...paths, ...more])]))}>
            Add files…
          </Button>
        </div>
        <label className="block text-[12.5px] text-dim">
          Send to
          <Select value={target} onChange={(e) => setTarget(e.target.value)} className="mt-1.5" aria-label="Phone">
            <option value="">Every paired phone</option>
            {devices.map((d) => (
              <option key={d.id} value={d.id}>
                {d.name}
              </option>
            ))}
          </Select>
        </label>
      </div>
    </Modal>
  );
}

export function PhonePage() {
  const route = useRoute();
  const [pairOpen, setPairOpen] = useState(false);
  const [sendOpen, setSendOpen] = useState(false);
  const remote = useLive((s) => s.remote);

  useEffect(() => {
    if (route.params.get('pair') === '1') {
      setPairOpen(true);
      navigate('phone');
    }
  }, [route.params]);

  return (
    <Page title="Phone" subtitle="Use your phone as a remote for this PC — files, power, screen and notes, all over your local network.">
      <div className="space-y-3">
        <Hero onPair={() => setPairOpen(true)} onSend={() => setSendOpen(true)} />
        <div className="grid grid-cols-[minmax(0,1fr)_minmax(0,1fr)] items-start gap-3">
          <div className="space-y-3">
            <Viewers />
            <Devices onPair={() => setPairOpen(true)} />
            <Transfers />
            <AuditLog />
          </div>
          <Permissions />
        </div>
        {!remote && <Skeleton className="h-20" />}
      </div>
      <PairDialog open={pairOpen} onClose={() => setPairOpen(false)} />
      <SendDialog open={sendOpen} onClose={() => setSendOpen(false)} />
    </Page>
  );
}
