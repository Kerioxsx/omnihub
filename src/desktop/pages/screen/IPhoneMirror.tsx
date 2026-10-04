// iPhone → PC: an AirPlay receiver (UxPlay) the iPhone picks in Control
// Center → Screen Mirroring.

import { formatBytes } from '@shared/format';
import type { AirPlayCheck, AirPlayStatus, FirewallReport, InstallProgress } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { Apple, ChevronDown, CircleCheck, Download, FolderSearch, LoaderCircle, Maximize2, PictureInPicture2, Play, RefreshCw, ShieldCheck, Smartphone, Square, Trash, Wifi, WifiOff } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Dot, Skeleton } from '../../components/ui/Card';
import { Checkbox, Field, Select, Switch, TextInput } from '../../components/ui/Form';
import { ProgressBar } from '../../components/ui/Progress';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useAsync, useEvent } from '../../lib/hooks';
import { confirm } from '../../state/dialogs';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';

function modelName(model: string): string {
  if (model.startsWith('iPhone')) return 'iPhone';
  if (model.startsWith('iPad')) return 'iPad';
  if (model.startsWith('Mac')) return 'Mac';
  return 'device';
}

function Steps({ name }: { name: string }) {
  const steps = ['Open Control Center on the iPhone (swipe down from the top-right corner).', 'Tap Screen Mirroring (two overlapping rectangles).', `Choose “${name}”.`];
  return (
    <ol className="space-y-2">
      {steps.map((s, i) => (
        <li key={s} className="flex items-start gap-3 text-[13px] text-dim">
          <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-accent-soft text-[12px] font-bold text-accent">{i + 1}</span>
          <span className="pt-0.5">{s}</span>
        </li>
      ))}
    </ol>
  );
}

function FirewallRow({ report, onFix, busy }: { report: FirewallReport; onFix: (pub: boolean) => void; busy: boolean }) {
  const [pub, setPub] = useState(false);
  const blocked = report.supported && (report.verdict === 'blocked' || report.verdict === 'noRule');
  if (!report.supported) return null;
  return (
    <div className={cx('flex items-start gap-3 rounded-xl border px-3.5 py-3', blocked ? 'border-warn/35 bg-warn/8' : 'border-line bg-surface')}>
      <ShieldCheck size={17} className={cx('mt-0.5 shrink-0', blocked ? 'text-warn' : 'text-good')} aria-hidden />
      <div className="min-w-0 flex-1 text-[12.5px] text-dim">
        <div className="font-medium text-fg">Windows Firewall</div>
        {report.message}
        {blocked && report.networks.some((n) => n.category === 'public') && (
          <div className="mt-1.5">
            <Checkbox checked={pub} onChange={setPub} label="Also allow on public networks" />
          </div>
        )}
      </div>
      {blocked && (
        <Button size="sm" variant="primary" loading={busy} onClick={() => onFix(pub)}>
          Allow receiver
        </Button>
      )}
    </div>
  );
}

/** Whether an iPhone on the same Wi-Fi can find the receiver, and what to do if not. */
function CheckRow({ check, address, name, outdated }: { check: AirPlayCheck | null; address: string | null; name: string; outdated: boolean }) {
  if (!check) {
    return (
      <div className="flex items-center gap-3 rounded-xl border border-line bg-surface px-3.5 py-3 text-[12.5px] text-dim">
        <LoaderCircle size={16} className="shrink-0 animate-spin text-accent" aria-hidden />
        Checking that iPhones on your Wi-Fi can find “{name}”{address ? ` at ${address}` : ''}…
      </div>
    );
  }
  const ok = check.announced && check.rightAddress && check.reachable;
  const problem = !check.announced
    ? `iPhones can't see “${name}” on the network of ${check.ip} yet.`
    : !check.rightAddress
      ? `“${name}” is announced with another address than ${check.ip} (a VPN or virtual adapter), so iPhones can't connect.`
      : `The receiver doesn't answer on ${check.ip}.`;
  return (
    <div className={cx('flex items-start gap-3 rounded-xl border px-3.5 py-3', ok ? 'border-line bg-surface' : 'border-warn/35 bg-warn/8')}>
      {ok ? <Wifi size={17} className="mt-0.5 shrink-0 text-good" aria-hidden /> : <WifiOff size={17} className="mt-0.5 shrink-0 text-warn" aria-hidden />}
      <div className="min-w-0 flex-1 text-[12.5px] text-dim">
        <div className="font-medium text-fg">{ok ? `iPhones on this Wi-Fi can find “${name}”` : 'iPhones may not find this PC'}</div>
        {ok ? (
          <>Announced at {check.ip}. If it still doesn't show in Screen Mirroring, check the points below.</>
        ) : (
          <>
            {problem}
            {outdated && ' Update the AirPlay receiver above first — the update fixes this on PCs with a VPN or several network adapters.'}
          </>
        )}
        <ul className="mt-1.5 list-disc space-y-0.5 pl-4 text-[12px]">
          <li>The iPhone is on the same Wi-Fi as this PC — not a guest network or mobile data.</li>
          <li>VPNs are off on the iPhone and on this PC.</li>
          <li>Windows Firewall allows the receiver (below), and this network is Private.</li>
          <li>The router doesn't isolate Wi-Fi devices (“AP isolation” or “client isolation” off).</li>
        </ul>
      </div>
    </div>
  );
}

function Installer({ status, progress }: { status: AirPlayStatus; progress: InstallProgress | null }) {
  const update = useSettings((s) => s.update);
  const start = () =>
    void api.screen.airplayInstall().catch((e: unknown) => {
      toast.error('Could not start the download', errorText(e));
    });
  const pickOwn = async () => {
    const files = await api.app.pickFiles('Choose uxplay.exe').catch(() => [] as string[]);
    if (files[0]) await update({ screen: { uxplayPath: files[0] } });
  };
  const frac = progress && progress.total ? progress.done / progress.total : null;
  return (
    <div className="mt-4 space-y-4">
      <p className="text-[13px] leading-relaxed text-dim">
        iPhones mirror their screen to AirPlay receivers. OmniHub uses <b className="text-fg">UxPlay</b>, an open-source receiver, packaged with what it needs by OmniHub's release pipeline from the official source. Nothing is installed on the iPhone.
      </p>
      {progress ? (
        <div className="rounded-xl border border-line bg-surface px-4 py-3">
          <div className="flex items-center justify-between text-[12.5px]">
            <span className="font-medium text-fg">{progress.phase === 'download' ? 'Downloading the AirPlay receiver…' : progress.phase === 'verify' ? 'Checking the download…' : 'Installing…'}</span>
            {progress.total > 0 && (
              <span className="tabular text-faint">
                {formatBytes(progress.done)} of {formatBytes(progress.total)}
              </span>
            )}
          </div>
          <ProgressBar value={progress.phase === 'download' ? frac : null} className="mt-2" label="Install progress" />
        </div>
      ) : (
        <div className="flex flex-wrap items-center gap-2.5">
          <Button variant="primary" icon={Download} onClick={start}>
            Install AirPlay receiver (about 70 MB)
          </Button>
          <Button variant="ghost" icon={FolderSearch} onClick={() => void pickOwn()}>
            Use my own UxPlay…
          </Button>
        </div>
      )}
      <p className="text-[11.5px] text-faint">
        The download is checked against its published SHA-256 before it is installed. UxPlay is GPL-3.0 software; its source and licences come with it. {status.supported ? '' : 'Mirroring works on Windows only.'}
      </p>
    </div>
  );
}

export function IPhoneMirror() {
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const status = useAsync(() => api.screen.airplayStatus(), []);
  const firewall = useAsync(() => (status.data?.installed ? api.screen.airplayFirewall() : Promise.resolve(null)), [status.data?.installed]);
  const [progress, setProgress] = useState<InstallProgress | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [more, setMore] = useState(false);

  useEffect(() => {
    if (status.data?.install) setProgress(status.data.install);
  }, [status.data?.install]);
  useEvent<InstallProgress & { error?: string }>('airplay:install', (p) => {
    if (p.phase === 'done') {
      setProgress(null);
      toast.success('AirPlay receiver installed', 'Start it, then pick this PC on your iPhone.');
      void status.reload();
    } else if (p.phase === 'failed') {
      setProgress(null);
      toast.error('The AirPlay receiver was not installed', p.error);
      void status.reload();
    } else setProgress(p);
  });
  useEvent('airplay:changed', () => void status.reload());
  useEvent<{ name: string; model: string }>('airplay:client', (c) => toast.success(`${c.name} is mirroring`, `Its ${modelName(c.model)} screen is open in a window on this PC.`));

  if (!settings || !status.data) {
    return (
      <Card className="p-5">
        <Skeleton className="h-40" />
      </Card>
    );
  }
  const s = status.data;
  const o = settings.screen.airplay;
  const setOpt = (patch: Partial<typeof o>) => void update({ screen: { airplay: patch } }, { silent: true });

  const run = async (key: string, fn: () => Promise<unknown>, ok?: string) => {
    setBusy(key);
    try {
      await fn();
      if (ok) toast.success(ok);
      await status.reload();
    } catch (e) {
      toast.error('That did not work', errorText(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <Card className="p-5">
      <CardHeader
        icon={Apple}
        title="iPhone → PC"
        subtitle="Mirror an iPhone or iPad with AirPlay — nothing to install on the phone"
        actions={s.running ? <Badge tone={s.mirroring ? 'accent' : 'good'}>{s.mirroring ? 'Mirroring' : 'Waiting for iPhone'}</Badge> : s.installed ? <Badge>Ready</Badge> : undefined}
      />

      {!s.installed ? (
        <Installer status={s} progress={progress} />
      ) : (
        <div className="mt-4 space-y-4">
          <AnimatePresence mode="wait">
            {s.running ? (
              <motion.div key="on" initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} className="grid gap-4 md:grid-cols-[minmax(0,1fr)_auto]">
                <div className="space-y-3">
                  {s.client ? (
                    <div className="flex items-center gap-3 rounded-xl border border-accent/30 bg-accent-soft px-4 py-3">
                      <Smartphone size={20} className="shrink-0 text-accent" aria-hidden />
                      <div className="min-w-0 flex-1">
                        <div className="text-[14px] font-semibold text-fg">{s.client.name}</div>
                        <div className="text-[12px] text-dim">{s.mirroring ? `Mirroring its ${modelName(s.client.model)} screen in a window on this PC` : 'Connected — start Screen Mirroring on the phone'}</div>
                      </div>
                      <Dot tone="good" pulse />
                    </div>
                  ) : (
                    <Steps name={s.name} />
                  )}
                  <div className="flex flex-wrap items-center gap-2">
                    <Button size="sm" icon={PictureInPicture2} onClick={() => void api.screen.airplayPlace('pip')}>
                      Corner (picture-in-picture)
                    </Button>
                    <Button size="sm" icon={Maximize2} onClick={() => void api.screen.airplayPlace('center')}>
                      Center
                    </Button>
                    <span className="ml-1 inline-flex items-center gap-2 text-[12.5px] text-dim">
                      <Switch checked={settings.screen.airplayKeepOnTop} onChange={(v) => void api.screen.airplayKeepOnTop(v)} label="Keep on top" />
                      Keep on top of games
                    </span>
                  </div>
                  <p className="text-[11.5px] text-faint">Alt+Enter in the iPhone window switches full screen. AirPlay adds roughly 0.1–0.3 s of delay: great for watching and slower games; the iPhone cannot be controlled from the PC (Apple does not allow it).</p>
                </div>
                {s.pin && !s.client && (
                  <div className="flex flex-col items-center justify-center rounded-2xl border border-line bg-surface px-5 py-4">
                    <div className="text-[11px] font-semibold uppercase tracking-wider text-faint">AirPlay code</div>
                    <div className="mt-2 flex gap-1.5" aria-label={`Code ${s.pin.split('').join(' ')}`}>
                      {s.pin.split('').map((d, i) => (
                        <span key={i} className="flex h-12 w-10 items-center justify-center rounded-lg border border-line-strong bg-surface-2 font-mono text-[24px] font-semibold text-fg">
                          {d}
                        </span>
                      ))}
                    </div>
                    <div className="mt-2 max-w-[180px] text-center text-[11px] text-faint">Type it on the iPhone the first time; it is remembered after.</div>
                  </div>
                )}
              </motion.div>
            ) : (
              <motion.div key="off" initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} className="grid gap-3 md:grid-cols-2">
                <Field label="Name on the iPhone" htmlFor="ap-name">
                  <TextInput id="ap-name" value={o.name} onChange={(e) => setOpt({ name: e.target.value })} maxLength={40} />
                </Field>
                <Field label="Quality" htmlFor="ap-q" hint={o.quality === '4k' ? 'Needs a recent iPhone and a fast PC (H.265).' : undefined}>
                  <Select id="ap-q" value={o.quality} onChange={(e) => setOpt({ quality: e.target.value as typeof o.quality })}>
                    <option value="1080p">1080p (recommended)</option>
                    <option value="1440p">1440p</option>
                    <option value="4k">4K</option>
                  </Select>
                </Field>
                <div className="flex flex-wrap gap-x-5 gap-y-2 md:col-span-2">
                  <Checkbox checked={o.audio} onChange={(v) => setOpt({ audio: v })} label="Play the iPhone's sound here" />
                  <Checkbox checked={o.requirePin} onChange={(v) => setOpt({ requirePin: v })} label="Ask for a code (recommended)" />
                  <Checkbox checked={o.lowLatency} onChange={(v) => setOpt({ lowLatency: v })} label="Lowest delay" />
                </div>
              </motion.div>
            )}
          </AnimatePresence>

          {s.outdated && (
            <Callout tone="warn" title="Update the AirPlay receiver">
              <div className="flex flex-wrap items-center gap-3">
                <span className="min-w-0 flex-1">This version fixes iPhones not finding the PC when it has a VPN, a second network card or virtual adapters. About 70 MB, checked before it installs.</span>
                {progress ? (
                  <span className="text-[12px] text-dim">{progress.phase === 'download' ? `Downloading… ${progress.total ? Math.round((progress.done / progress.total) * 100) : 0}%` : progress.phase === 'verify' ? 'Checking…' : 'Installing…'}</span>
                ) : (
                  <Button size="sm" variant="primary" icon={RefreshCw} loading={busy === 'update'} onClick={() => void run('update', () => api.screen.airplayInstall())}>
                    Update receiver
                  </Button>
                )}
              </div>
            </Callout>
          )}

          {s.running && <CheckRow check={s.check} address={s.address} name={s.name} outdated={s.outdated} />}

          {s.error && (
            <Callout tone="bad" title="The receiver reported a problem">
              <span className="font-mono text-[12px]">{s.error}</span>
            </Callout>
          )}

          {firewall.data && <FirewallRow report={firewall.data} busy={busy === 'fw'} onFix={(pub) => void run('fw', () => api.screen.airplayFixFirewall(pub).then((r) => firewall.setData(r)), 'The AirPlay receiver is allowed through the firewall')} />}

          <div className="flex flex-wrap items-center gap-2.5">
            {s.running ? (
              <Button variant="danger" icon={Square} loading={busy === 'stop'} onClick={() => void run('stop', () => api.screen.airplayStop())}>
                Stop receiver
              </Button>
            ) : (
              <Button variant="primary" icon={Play} loading={busy === 'start'} disabled={!s.supported} onClick={() => void run('start', () => api.screen.airplayStart())}>
                Start receiver
              </Button>
            )}
            <span className="inline-flex items-center gap-2 text-[12.5px] text-dim">
              <Switch checked={settings.screen.airplayAutoStart} onChange={(v) => void update({ screen: { airplayAutoStart: v } })} label="Start with OmniHub" />
              Start with OmniHub
            </span>
            <span className="inline-flex items-center gap-2 text-[12.5px] text-dim">
              <Switch checked={settings.screen.airplayPip} onChange={(v) => void update({ screen: { airplayPip: v } }, { silent: true })} label="Open in the corner" />
              Open in the corner
            </span>
            <button type="button" onClick={() => setMore(!more)} className="ml-auto flex items-center gap-1 text-[12px] text-faint hover:text-fg" aria-expanded={more}>
              Details <ChevronDown size={13} className={cx('transition-transform', more && 'rotate-180')} />
            </button>
          </div>
          {more && (
            <div className="space-y-2 rounded-xl border border-line bg-surface px-4 py-3 text-[12px] text-dim">
              <div className="flex items-center gap-2">
                <CircleCheck size={13} className="text-good" />
                {s.version ?? 'UxPlay'} · {s.source === 'addon' ? 'OmniHub add-on' : s.source === 'custom' ? 'your own copy' : 'found on this PC'}
                <span className="truncate font-mono text-[11px] text-faint">{s.path}</span>
              </div>
              {s.log.length > 0 && <pre className="max-h-40 overflow-auto whitespace-pre-wrap rounded-lg bg-surface-2 p-2 font-mono text-[11px] text-faint">{s.log.slice(-12).join('\n')}</pre>}
              <div className="flex gap-2">
                {settings.screen.uxplayPath && (
                  <Button size="xs" variant="ghost" onClick={() => void update({ screen: { uxplayPath: null } })}>
                    Stop using my own UxPlay
                  </Button>
                )}
                {s.source === 'addon' && (
                  <Button
                    size="xs"
                    variant="ghost"
                    icon={Trash}
                    onClick={async () => {
                      if (await confirm({ title: 'Remove the AirPlay receiver?', description: 'It can be downloaded again at any time.', tone: 'danger', confirmLabel: 'Remove' })) void run('rm', () => api.screen.airplayUninstall(), 'AirPlay receiver removed');
                    }}
                  >
                    Remove add-on
                  </Button>
                )}
              </div>
            </div>
          )}
        </div>
      )}
      {!s.installed && (
        <div className="mt-4 rounded-xl border border-dashed border-line px-4 py-3 text-[12px] text-faint">
          Android instead? Use <b className="text-dim">Phone → PC (Android)</b> below — it also lets you control the phone.
        </div>
      )}
    </Card>
  );
}
