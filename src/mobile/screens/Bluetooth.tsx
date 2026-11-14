// The PC's Bluetooth devices: connect or disconnect headphones and
// speakers, and forget any device. New devices are paired in Windows.

import { Bluetooth, Gamepad2, Headphones, Keyboard, Laptop, Mouse, Smartphone } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { client, type BtDevice } from '../client';
import { cx, errorMessage, vibrate } from '../lib/util';
import { toast } from '../state';
import { Empty, ListSkeleton } from '../ui/common';
import { SubHeader } from './More';

const ICON: Record<BtDevice['kind'], typeof Headphones> = { audio: Headphones, keyboard: Keyboard, mouse: Mouse, gamepad: Gamepad2, phone: Smartphone, computer: Laptop, other: Bluetooth };

export function BluetoothPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const [list, setList] = useState<BtDevice[] | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const load = useCallback(() => {
    client.bluetooth().then(
      (r) => setList(r.devices),
      (e) => {
        setList([]);
        toast.error(errorMessage(e));
      },
    );
  }, []);
  useEffect(() => {
    if (active) load();
  }, [active, load]);
  const run = async (d: BtDevice, f: () => Promise<{ devices: BtDevice[] }>, ok: string) => {
    setBusy(d.address);
    vibrate(15);
    try {
      setList((await f()).devices);
      toast.success(ok);
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(null);
    }
  };
  return (
    <div className="scroller h-full">
      <SubHeader title="Bluetooth" subtitle="Devices paired with the PC" onBack={onBack} />
      <div className="px-safe pb-tabbar space-y-3">
        {!list ? (
          <ListSkeleton rows={4} />
        ) : list.length === 0 ? (
          <Empty icon={<Bluetooth size={28} />} title="No Bluetooth devices" body="Pair them in Windows: Settings → Bluetooth & devices." />
        ) : (
          list.map((d) => {
            const Icon = ICON[d.kind] ?? Bluetooth;
            const wait = busy === d.address;
            return (
              <div key={d.address} className="card p-4">
                <div className="flex items-center gap-3">
                  <span className={cx('grid h-11 w-11 place-items-center rounded-2xl', d.connected ? 'bg-accent-soft text-accent' : 'bg-surface-2 text-dim')}>
                    <Icon size={20} />
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[16px] font-semibold">{d.name || 'Bluetooth device'}</div>
                    <div className={cx('text-[13px]', d.connected ? 'text-good' : 'text-faint')}>{d.connected ? 'Connected' : 'Not connected'}</div>
                  </div>
                </div>
                <div className="mt-3 flex gap-2">
                  {d.audio && (
                    <button type="button" disabled={wait} onClick={() => void run(d, () => client.btConnect(d.address, d.name, !d.connected), d.connected ? `${d.name} disconnected` : `${d.name} connected`)} className="press flex-1 rounded-xl bg-accent px-3 py-2.5 text-[14px] font-semibold text-white disabled:opacity-50">
                      {wait ? '…' : d.connected ? 'Disconnect' : 'Connect'}
                    </button>
                  )}
                  <button
                    type="button"
                    disabled={wait}
                    onClick={() => {
                      if (window.confirm(`Forget ${d.name}? It will need pairing again in Windows.`)) void run(d, () => client.btForget(d.address, d.name), `${d.name} forgotten`);
                    }}
                    className={cx('press rounded-xl bg-white/[0.06] px-3 py-2.5 text-[14px] font-semibold text-bad disabled:opacity-50', !d.audio && 'flex-1')}
                  >
                    Forget
                  </button>
                </div>
              </div>
            );
          })
        )}
        <p className="px-2 text-center text-xs leading-relaxed text-faint">Headphones and speakers can be connected and disconnected from here. Other devices connect by themselves when they’re turned on.</p>
      </div>
    </div>
  );
}
