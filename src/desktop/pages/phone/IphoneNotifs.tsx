// Your iPhone's notifications on the PC, over Bluetooth (Apple's
// notification service, the one watches use). Pair the iPhone in Windows
// once, pick it here, and allow "Share System Notifications" on it.

import { formatRelative } from '@shared/format';
import type { IphoneNotif, IphoneState, PairedPhone } from '@shared/types';
import { Bell, BellOff, Bluetooth, RefreshCw, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader, Dot } from '../../components/ui/Card';
import { Select, Switch } from '../../components/ui/Form';
import { cx } from '../../lib/cx';
import { useEvent, useNow } from '../../lib/hooks';
import { toast } from '../../state/toasts';

const TONE: Record<string, 'good' | 'warn' | 'bad' | 'neutral'> = { connected: 'good', connecting: 'warn', waiting: 'warn', off: 'neutral', unsupported: 'bad' };

export function IphoneNotifs() {
  const [st, setSt] = useState<IphoneState | null>(null);
  const [phones, setPhones] = useState<PairedPhone[] | null>(null);
  const [pick, setPick] = useState('');
  useNow(30_000);
  const load = () => void api.iphone.get().then(setSt, () => undefined);
  const findPhones = () => {
    setPhones(null);
    void api.iphone.paired().then(
      (p) => {
        setPhones(p);
        setPick((cur) => cur || p[0]?.id || '');
      },
      () => setPhones([]),
    );
  };
  useEffect(() => {
    load();
    findPhones();
  }, []);
  useEvent<{ notif: IphoneNotif }>('iphone:notification', ({ notif }) => setSt((s) => (s ? { ...s, notifs: [notif, ...s.notifs.filter((n) => n.uid !== notif.uid)].slice(0, 60) } : s)));
  useEvent<{ uid: number }>('iphone:removed', ({ uid }) => setSt((s) => (s ? { ...s, notifs: s.notifs.filter((n) => n.uid !== uid) } : s)));
  useEvent<IphoneState['status']>('iphone:status', (status) => setSt((s) => (s ? { ...s, status } : s)));

  const connect = async (on: boolean) => {
    try {
      const phone = phones?.find((p) => p.id === pick);
      const prefs = await api.iphone.connect(on && phone ? phone.id : null, phone?.name ?? null);
      setSt((s) => (s ? { ...s, prefs } : s));
    } catch (e) {
      toast.error('Couldn’t do that', errorText(e));
    }
  };
  const on = !!st?.prefs.enabled;
  const status = st?.status;
  return (
    <Card className="p-5">
      <CardHeader
        icon={Bell}
        title="iPhone notifications"
        subtitle={on ? (status?.message || 'Starting…') : 'See your iPhone’s notifications here, over Bluetooth. Nothing to install on the phone.'}
        actions={
          on ? (
            <span className="flex items-center gap-2 text-[12px] text-dim">
              <Dot tone={TONE[status?.state ?? 'off'] ?? 'neutral'} /> {status?.state === 'connected' ? 'Connected' : status?.state === 'off' ? 'Off' : 'Waiting'}
            </span>
          ) : undefined
        }
      />
      {!on ? (
        <div className="mt-4 space-y-3">
          <ol className="list-decimal space-y-1.5 pl-5 text-[13px] text-dim">
            <li>
              Pair your iPhone with this PC in Windows: on the iPhone open Settings → Bluetooth, then on the PC{' '}
              <button type="button" className="font-medium text-accent hover:underline" onClick={() => void api.iphone.openBluetooth()}>
                Bluetooth settings → Add device
              </button>
              .
            </li>
            <li>Pick it below and press Connect.</li>
            <li>On the iPhone, allow notifications when asked (or Settings → Bluetooth → ⓘ next to this PC → Share System Notifications).</li>
          </ol>
          <div className="flex items-center gap-2">
            <Select value={pick} onChange={(e) => setPick(e.target.value)} className="min-w-0 flex-1" aria-label="Paired phone" disabled={!phones?.length}>
              {phones === null ? <option>Looking…</option> : phones.length === 0 ? <option>No paired Bluetooth devices yet</option> : phones.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
            </Select>
            <IconButton icon={RefreshCw} label="Look again" onClick={findPhones} />
            <Button variant="primary" icon={Bluetooth} disabled={!pick} onClick={() => void connect(true)}>
              Connect
            </Button>
          </div>
          <p className="text-[11.5px] text-faint">Notifications stay on this PC, in memory only (the last 60). This uses Apple’s notification service for accessories, the same one watches and Windows Phone Link use.</p>
        </div>
      ) : (
        <div className="mt-4">
          <div className="mb-3 flex items-center gap-3 text-[13px]">
            <span className="min-w-0 flex-1 truncate text-dim">{st?.prefs.deviceName}</span>
            <span className="flex items-center gap-2 text-dim">
              Windows pop-ups
              <Switch checked={!!st?.prefs.toasts} onChange={(v) => void api.iphone.toasts(v).then((prefs) => setSt((s) => (s ? { ...s, prefs } : s)))} label="Windows pop-ups" size="md" />
            </span>
            <Button variant="ghost" icon={BellOff} onClick={() => void connect(false)}>
              Stop
            </Button>
          </div>
          {st?.notifs.length ? (
            <div className="max-h-[420px] space-y-1.5 overflow-y-auto pr-1">
              {st.notifs.map((n) => (
                <div key={n.uid} className="group flex items-start gap-3 rounded-xl border border-line bg-surface px-3 py-2.5">
                  <span className={cx('mt-0.5 grid h-8 w-8 shrink-0 place-items-center rounded-[10px] text-[13px] font-bold text-white')} style={{ background: appColor(n.app) }}>
                    {n.appName.slice(0, 1).toUpperCase()}
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-baseline gap-2 text-[12px] text-faint">
                      <span className="font-medium text-dim">{n.appName}</span>
                      <span>{formatRelative(n.at / 1000)}</span>
                    </div>
                    {n.title && <div className="truncate text-[13.5px] font-medium">{n.title}</div>}
                    <div className="line-clamp-3 text-[13px] text-dim">{n.message}</div>
                  </div>
                  <IconButton icon={X} label="Dismiss (on the iPhone too)" size="sm" className="opacity-0 group-hover:opacity-100" onClick={() => void api.iphone.dismiss(n.uid)} />
                </div>
              ))}
            </div>
          ) : (
            <div className="rounded-xl border border-dashed border-line px-4 py-6 text-center text-[13px] text-faint">{status?.state === 'connected' ? 'No notifications yet. New ones show up here as they arrive on the iPhone.' : 'Waiting for the iPhone. Keep it nearby with Bluetooth on.'}</div>
          )}
        </div>
      )}
    </Card>
  );
}

function appColor(app: string): string {
  const h = [...app].reduce((a, c) => (a * 31 + c.charCodeAt(0)) % 360, 11);
  return `hsl(${h} 55% 45%)`;
}
