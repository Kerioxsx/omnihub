import { formatRelative } from '@shared/format';
import type { FirewallReport, RemoteDiagnostics, Visit } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { ChevronDown, CircleAlert, CircleCheck, CircleX, Globe, Home, RefreshCw, ShieldCheck, Smartphone, Stethoscope, Wifi } from 'lucide-react';
import { type ReactNode, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader, Skeleton } from '../../components/ui/Card';
import { Checkbox } from '../../components/ui/Form';
import { cx } from '../../lib/cx';
import { useAsync, useEvent, useNow } from '../../lib/hooks';
import { toast } from '../../state/toasts';

type Tone = 'good' | 'warn' | 'bad' | 'neutral';

function StatusIcon({ tone }: { tone: Tone }) {
  if (tone === 'good') return <CircleCheck size={18} className="shrink-0 text-good" aria-label="OK" />;
  if (tone === 'bad') return <CircleX size={18} className="shrink-0 text-bad" aria-label="Problem" />;
  if (tone === 'warn') return <CircleAlert size={18} className="shrink-0 text-warn" aria-label="Warning" />;
  return <CircleAlert size={18} className="shrink-0 text-faint" aria-label="Unknown" />;
}

function Row({ tone, icon: Icon, title, children, action }: { tone: Tone; icon: typeof Wifi; title: ReactNode; children?: ReactNode; action?: ReactNode }) {
  return (
    <div className="flex items-start gap-3 rounded-xl border border-line bg-surface px-3.5 py-3">
      <StatusIcon tone={tone} />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1.5 text-[13px] font-medium text-fg">
          <Icon size={13} className="text-faint" aria-hidden />
          {title}
        </div>
        {children && <div className="mt-0.5 text-[12.5px] leading-relaxed text-dim">{children}</div>}
      </div>
      {action && <div className="shrink-0">{action}</div>}
    </div>
  );
}

function browserOf(ua: string): string {
  const device = /iPhone/.test(ua) ? 'iPhone' : /iPad/.test(ua) ? 'iPad' : /Android/.test(ua) ? 'Android phone' : 'device';
  const browser = /CriOS/.test(ua) ? 'Chrome' : /FxiOS/.test(ua) ? 'Firefox' : /EdgiOS|EdgA/.test(ua) ? 'Edge' : /SamsungBrowser/.test(ua) ? 'Samsung Internet' : /Chrome/.test(ua) ? 'Chrome' : /Safari/.test(ua) ? 'Safari' : 'a browser';
  return `${device}, ${browser}`;
}

function firewallTone(f: FirewallReport): Tone {
  if (!f.supported) return 'neutral';
  if (f.verdict === 'allowed' || f.verdict === 'off') return 'good';
  if (f.verdict === 'unknown') return 'neutral';
  return 'bad';
}

/** Why a phone cannot reach the companion, with one-click fixes. */
export function ConnectionCheck({ defaultOpen }: { defaultOpen?: boolean }) {
  const diag = useAsync(() => api.remote.diagnostics(), []);
  const [busy, setBusy] = useState<string | null>(null);
  const [alsoPublic, setAlsoPublic] = useState(false);
  const [tipsOpen, setTipsOpen] = useState(!!defaultOpen);
  useNow(5000);
  useEvent<Visit>('remote:visit', (v) => {
    if (diag.data) diag.setData({ ...diag.data, visitors: [v, ...diag.data.visitors.filter((x) => x.ip !== v.ip)] });
  });

  const run = async (key: string, fn: () => Promise<unknown>, ok: string) => {
    setBusy(key);
    try {
      await fn();
      toast.success(ok);
      await diag.reload();
    } catch (e) {
      const msg = errorText(e);
      toast.error(/declined/i.test(msg) ? 'Administrator approval was declined' : 'That did not work', /declined/i.test(msg) ? 'Windows needs your approval to change firewall or network settings.' : msg);
    } finally {
      setBusy(null);
    }
  };

  const d: RemoteDiagnostics | undefined = diag.data;
  const network = d?.firewall.networks[0];
  const isPublic = !!d && d.firewall.networks.some((n) => n.category === 'public');
  const primary = d?.addresses.find((a) => a.primary) ?? d?.addresses.find((a) => !a.virtualAdapter);
  const others = d?.addresses.filter((a) => a !== primary) ?? [];
  const visit = d?.visitors[0];
  const scheme = d?.tls ? 'https' : 'http';
  const url = primary && d ? `${scheme}://${primary.ip}:${d.port}` : null;
  const blocked = !!d && d.firewall.supported && (d.firewall.verdict === 'blocked' || d.firewall.verdict === 'noRule');
  const selfPrimary = d?.selfTest.find((t) => t.ip === primary?.ip);
  const selfFailed = !!selfPrimary && !selfPrimary.ok;
  const problems = !!d && ((isPublic && blocked) || blocked || !primary || selfFailed);

  return (
    <Card className={cx('p-5', problems && 'border-warn/40')}>
      <CardHeader
        icon={Stethoscope}
        title="Connection check"
        subtitle={!d ? 'Checking…' : problems ? 'Something on this PC may stop phones from connecting' : 'Phones on your network should be able to connect'}
        actions={<Button size="xs" variant="ghost" icon={RefreshCw} loading={diag.loading && !!d} onClick={() => void diag.reload()}>Check again</Button>}
      />
      <div className="mt-4 space-y-2">
        {!d ? (
          [0, 1, 2, 3].map((i) => <Skeleton key={i} className="h-14" />)
        ) : (
          <>
            {d.firewall.supported && (
              <Row
                tone={!network ? 'warn' : network.category === 'public' ? 'warn' : 'good'}
                icon={Wifi}
                title={network ? `${network.name} · ${network.category === 'public' ? 'Public network' : network.category === 'domain' ? 'Work network' : 'Private network'}` : 'No network connection'}
                action={
                  network?.category === 'public' ? (
                    <Button size="sm" variant="primary" icon={Home} loading={busy === 'private'} onClick={() => void run('private', () => api.remote.makeNetworkPrivate(network.id), `${network.name} is now a private network`)}>
                      Make private
                    </Button>
                  ) : undefined
                }
              >
                {!network
                  ? 'Connect this PC to the same Wi-Fi or router as your phone.'
                  : network.category === 'public'
                    ? 'Windows treats Public networks as untrusted and blocks incoming connections. At home, make it private (Windows asks for approval).'
                    : 'Windows allows incoming connections on this network when the firewall rule below is in place.'}
              </Row>
            )}

            <Row
              tone={firewallTone(d.firewall)}
              icon={ShieldCheck}
              title="Windows Firewall"
              action={
                blocked ? (
                  <Button size="sm" variant="primary" icon={ShieldCheck} loading={busy === 'firewall'} onClick={() => void run('firewall', () => api.remote.fixFirewall(alsoPublic), 'OmniHub is allowed through the firewall')}>
                    Allow OmniHub
                  </Button>
                ) : undefined
              }
            >
              {d.firewall.message}
              {blocked && isPublic && (
                <div className="mt-2">
                  <Checkbox checked={alsoPublic} onChange={setAlsoPublic} label="Also allow on public networks (cafés, hotels)" />
                </div>
              )}
            </Row>

            <Row tone={primary ? (primary.virtualAdapter ? 'warn' : 'good') : 'bad'} icon={Globe} title={primary ? `${primary.ip} on ${primary.interface}` : 'No local network address'}>
              {primary ? (
                <>
                  Phones open <span className="font-mono text-fg">{url}</span>. The QR code uses this address.
                  {others.length > 0 && <span className="text-faint"> Also on this PC: {others.map((a) => `${a.ip} (${a.interface})`).join(', ')}.</span>}
                </>
              ) : (
                'This PC has no Wi-Fi or Ethernet address a phone could reach. Connect it to your router.'
              )}
            </Row>

            {selfPrimary && (
              <Row
                tone={selfPrimary.ok ? 'good' : 'bad'}
                icon={Stethoscope}
                title={selfPrimary.ok ? 'OmniHub answers on this address' : 'OmniHub does not answer on this address'}
                action={
                  !selfPrimary.ok ? (
                    <Button size="sm" variant="primary" icon={RefreshCw} loading={busy === 'restart'} onClick={() => void run('restart', () => api.remote.restart(), 'Phone companion restarted')}>
                      Restart companion
                    </Button>
                  ) : undefined
                }
              >
                {selfPrimary.ok
                  ? `This PC reached ${url} itself, so the companion is listening correctly.`
                  : `Even this PC could not open ${url} (${selfPrimary.error ?? 'no answer'}). Restart the companion; if it stays red, a security program (Norton, McAfee, Avast, Bitdefender, Kaspersky…) may be blocking OmniHub — allow it there.`}
              </Row>
            )}

            <Row tone={!d.running ? 'neutral' : visit ? (visit.allowed ? 'good' : 'bad') : 'neutral'} icon={Smartphone} title={!d.running ? 'Companion is off' : visit ? (visit.allowed ? 'A phone reached this PC' : 'A device was turned away') : 'No phone has reached this PC yet'}>
              {!d.running
                ? 'Turn the companion on, then open the address on your phone.'
                : visit
                  ? visit.allowed
                    ? `${visit.ip} (${browserOf(visit.userAgent)}) ${formatRelative(visit.at)}. The network path works.`
                    : `${visit.ip} is not a local network address, so it was refused. Is the phone on mobile data or a VPN?`
                  : `Open ${url ?? 'the address'} on your phone. If this stays empty, the request never arrives: check the firewall and Wi-Fi above.`}
            </Row>
          </>
        )}
      </div>

      <button type="button" onClick={() => setTipsOpen(!tipsOpen)} className="mt-3 flex w-full items-center gap-1.5 rounded-lg px-1 py-1 text-left text-[12.5px] font-medium text-dim hover:text-fg" aria-expanded={tipsOpen}>
        <ChevronDown size={14} className={cx('transition-transform', tipsOpen && 'rotate-180')} aria-hidden />
        Still “can't be reached” on an iPhone?
      </button>
      <AnimatePresence initial={false}>
        {tipsOpen && (
          <motion.ul initial={{ height: 0, opacity: 0 }} animate={{ height: 'auto', opacity: 1 }} exit={{ height: 0, opacity: 0 }} className="overflow-hidden pl-6 text-[12.5px] leading-relaxed text-dim [&>li]:mt-1.5 [&>li]:list-disc">
            <li>The phone must be on the <b className="text-fg">same Wi-Fi</b> as this PC — not mobile data, and not a guest network (guest Wi-Fi keeps devices apart).</li>
            <li>
              <b className="text-fg">Chrome, Edge or Firefox on iPhone</b> need permission to reach local devices: iPhone Settings → the browser → turn on <b className="text-fg">Local Network</b>. Safari does not need it.
            </li>
            <li>Turn off any VPN on the phone while connecting.</li>
            <li>
              Antivirus suites with their own firewall (Norton, McAfee, Avast, AVG, Bitdefender, Kaspersky, ESET) ignore the Windows Firewall rule. Allow <b className="text-fg">OmniHub</b> in that program, or try once with its firewall paused.
            </li>
            <li>
              The certificate warning is expected (the certificate is made on this PC). Safari: <b className="text-fg">Show Details → visit this website</b>. Chrome: <b className="text-fg">Advanced → Proceed</b>.
            </li>
            <li>Some routers block devices from talking to each other (“AP isolation” or “client isolation”). Turn that off in the router settings.</li>
            <li>As a last resort, switch the companion to plain HTTP in Settings → Phone companion (only on networks you trust).</li>
          </motion.ul>
        )}
      </AnimatePresence>
    </Card>
  );
}
