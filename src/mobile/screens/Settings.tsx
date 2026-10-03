// Settings: this phone, theme, PC info, unpair.

import { useEffect, useState } from 'react';
import { Smartphone, Sun, Moon, SunMoon, ShieldCheck, ShieldAlert, Unlink, Check, X, Info, Download, Server } from 'lucide-react';
import type { Device } from '@shared/types';
import { formatDate } from '@shared/format';
import { client, savedDeviceName } from '../client';
import { useApp, toast } from '../state';
import { Sheet } from '../ui/Sheet';
import { Button, Segmented } from '../ui/common';
import { SubHeader } from './More';
import { getThemePref, setThemePref, type ThemePref } from '../lib/theme';
import { cx, errorMessage } from '../lib/util';

export function SettingsPage({ onBack, onUnpaired }: { onBack: () => void; onUnpaired: () => void }) {
  const info = useApp((s) => s.info);
  const [theme, setTheme] = useState<ThemePref>(getThemePref());
  const [me, setMe] = useState<Device | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    client.me().then(setMe, () => undefined);
  }, []);

  const standalone = typeof window !== 'undefined' && (window.matchMedia?.('(display-mode: standalone)').matches || (navigator as unknown as { standalone?: boolean }).standalone === true);

  const unpair = async () => {
    setBusy(true);
    try {
      await client.unpair();
      toast.success('This phone was unpaired');
    } catch (e) {
      toast.info('Unpaired on this phone', `The PC couldn't be told: ${errorMessage(e)}`);
    } finally {
      setBusy(false);
      setConfirm(false);
      onUnpaired();
    }
  };

  const f = info?.features;
  const features: [string, boolean | undefined][] = [
    ['Uploads', f?.uploads],
    ['Power', f?.power],
    ['Screen', f?.screen],
    ['Control', f?.control],
    ['Apps', f?.apps],
    ['Notes', f?.notes],
    ['Vault', f?.vault],
  ];

  return (
    <div className="scroller h-full">
      <SubHeader title="Settings" onBack={onBack} />
      <div className="px-safe space-y-5 pb-tabbar">
        <section>
          <h2 className="mb-2.5 px-1 text-[13px] font-semibold uppercase tracking-[0.08em] text-faint">This phone</h2>
          <div className="card divide-y divide-[var(--border)]">
            <div className="flex items-center gap-3.5 px-4 py-3.5">
              <div className="grid h-10 w-10 place-items-center rounded-xl bg-accent-soft text-accent">
                <Smartphone size={19} />
              </div>
              <div className="min-w-0 flex-1">
                <div className="truncate text-[15px] font-semibold">{me?.name ?? savedDeviceName()}</div>
                <div className="truncate text-[13px] text-dim">{me ? `Paired ${formatDate(me.created)}` : 'Paired device'}</div>
              </div>
            </div>
            <div className="px-4 py-3.5">
              <div className="mb-2.5 text-[15px] font-semibold">Appearance</div>
              <Segmented
                value={theme}
                onChange={(t) => {
                  setTheme(t);
                  setThemePref(t);
                }}
                options={[
                  { value: 'system', label: <><SunMoon size={15} /> System</> },
                  { value: 'dark', label: <><Moon size={15} /> Dark</> },
                  { value: 'light', label: <><Sun size={15} /> Light</> },
                ]}
              />
            </div>
          </div>
        </section>

        <section>
          <h2 className="mb-2.5 px-1 text-[13px] font-semibold uppercase tracking-[0.08em] text-faint">PC</h2>
          <div className="card divide-y divide-[var(--border)]">
            <KV icon={<Server size={17} />} k="Name" v={info?.name ?? '—'} />
            <KV icon={<Info size={17} />} k="OmniHub" v={info ? `${info.version} · ${info.platform}` : '—'} />
            <KV
              icon={info?.tls ? <ShieldCheck size={17} className="text-good" /> : <ShieldAlert size={17} className="text-warn" />}
              k="Connection"
              v={info?.tls ? 'HTTPS · encrypted (self-signed)' : 'HTTP · not encrypted'}
              vClass={info?.tls ? 'text-good' : 'text-warn'}
            />
            <div className="px-4 py-3.5">
              <div className="mb-2.5 text-[13px] font-semibold text-dim">Allowed from this phone</div>
              <div className="flex flex-wrap gap-1.5">
                {features.map(([name, on]) => (
                  <span key={name} className={cx('inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-xs font-semibold', on ? 'bg-good/12 text-good' : 'bg-surface-2 text-faint')}>
                    {on ? <Check size={12} strokeWidth={3} /> : <X size={12} strokeWidth={3} />} {name}
                  </span>
                ))}
              </div>
              <div className="mt-2.5 text-xs text-faint">Change these in OmniHub → Settings → Phone on the PC.</div>
            </div>
          </div>
          {!info?.tls && <p className="mt-2 px-2 text-xs leading-relaxed text-faint">Turn on HTTPS in OmniHub on the PC to encrypt traffic and enable the vault.</p>}
        </section>

        {!standalone && (
          <section className="card flex items-start gap-3.5 p-4">
            <div className="grid h-10 w-10 shrink-0 place-items-center rounded-xl bg-surface-2 text-dim">
              <Download size={18} />
            </div>
            <div className="text-sm text-dim">
              <div className="font-semibold text-fg">Add to Home screen</div>
              Open your browser menu and choose “Add to Home screen” (Chrome) or Share → “Add to Home Screen” (Safari) for a full-screen app.
            </div>
          </section>
        )}

        <section>
          <Button variant="outline" className="w-full border-bad/40 text-bad" icon={<Unlink size={18} />} onClick={() => setConfirm(true)}>
            Unpair this phone
          </Button>
          <p className="mt-2 px-2 text-center text-xs text-faint">You'll need the PC to pair again.</p>
        </section>
      </div>

      <Sheet
        open={confirm}
        onClose={() => setConfirm(false)}
        title="Unpair this phone?"
        subtitle={`It will lose access to ${info?.name ?? 'the PC'} right away.`}
        footer={
          <div className="flex gap-2.5">
            <Button variant="ghost" className="flex-1" onClick={() => setConfirm(false)}>
              Keep
            </Button>
            <Button variant="danger" className="flex-1" loading={busy} onClick={unpair} icon={<Unlink size={17} />}>
              Unpair
            </Button>
          </div>
        }
      >
        <p className="pb-2 text-[15px] text-dim">Files you downloaded stay on this phone. To use OmniHub again, click “Pair a phone” on the PC and scan the code.</p>
      </Sheet>
    </div>
  );
}

function KV({ icon, k, v, vClass }: { icon: React.ReactNode; k: string; v: string; vClass?: string }) {
  return (
    <div className="flex items-center gap-3 px-4 py-3.5">
      <div className="text-faint">{icon}</div>
      <div className="w-24 shrink-0 text-[15px] text-dim">{k}</div>
      <div className={cx('min-w-0 flex-1 truncate text-right text-[15px] font-medium', vClass)}>{v}</div>
    </div>
  );
}
