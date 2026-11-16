// Slow down a program's internet: a Windows QoS policy caps how fast it
// uploads. Windows only shapes what a program sends, so downloads stay as
// they are; making or removing a cap asks for administrator approval.

import type { AppLimit } from '@shared/types';
import { Gauge, X } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Select } from '../../components/ui/Form';
import { useAsync } from '../../lib/hooks';
import { toast } from '../../state/toasts';

export const SPEEDS = [256, 512, 1000, 2000, 5000, 10_000, 25_000, 50_000];
export const speed = (kbps: number) => (kbps >= 1000 ? `${+(kbps / 1000).toFixed(1)} Mbps` : `${kbps} Kbps`);

export function useAppLimits() {
  const limits = useAsync(() => api.netLimits.list(), []);
  const [busy, setBusy] = useState<string | null>(null);
  const set = async (exe: string, kbps: number) => {
    setBusy(exe);
    try {
      limits.setData(await api.netLimits.set(exe, kbps));
      toast.success(`${exe.replace(/\.exe$/i, '')} now uploads at most ${speed(kbps)}`, 'Downloads are not capped — Windows only shapes what a program sends.');
    } catch (e) {
      toast.error(`Couldn't limit ${exe}`, errorText(e));
    } finally {
      setBusy(null);
    }
  };
  const remove = async (exe: string) => {
    setBusy(exe);
    try {
      limits.setData(await api.netLimits.remove(exe));
      toast.success(`${exe.replace(/\.exe$/i, '')} is back to full speed`);
    } catch (e) {
      toast.error(`Couldn't remove the limit on ${exe}`, errorText(e));
    } finally {
      setBusy(null);
    }
  };
  return { list: limits.data ?? [], set, remove, busy };
}

/** The "Slow down" control on a program's row. */
export function LimitControl({ exe, limit, busy, onSet, onRemove }: { exe: string; limit?: AppLimit; busy: boolean; onSet: (kbps: number) => void; onRemove: () => void }) {
  const [open, setOpen] = useState(false);
  const [kbps, setKbps] = useState(limit?.kbps ?? 2000);
  if (limit && !open)
    return (
      <span className="flex shrink-0 items-center gap-1">
        <button type="button" onClick={() => setOpen(true)} className="rounded-md bg-warn/15 px-1.5 py-px text-[11px] font-medium text-warn" title="Change the upload limit">
          ↑ {speed(limit.kbps)}
        </button>
        <IconButton icon={X} label={`Remove the limit on ${exe}`} size="sm" disabled={busy} onClick={onRemove} />
      </span>
    );
  if (!open) return <IconButton icon={Gauge} label={`Slow down ${exe}'s upload`} size="sm" onClick={() => setOpen(true)} />;
  return (
    <span className="flex shrink-0 items-center gap-1.5">
      <Select value={kbps} onChange={(e) => setKbps(Number(e.target.value))} aria-label="Upload limit" className="h-7 w-[110px] text-[12px]">
        {SPEEDS.map((s) => (
          <option key={s} value={s}>
            {speed(s)}
          </option>
        ))}
      </Select>
      <Button
        size="sm"
        variant="primary"
        loading={busy}
        onClick={() => {
          onSet(kbps);
          setOpen(false);
        }}
      >
        Limit
      </Button>
      <IconButton icon={X} label="Cancel" size="sm" onClick={() => setOpen(false)} />
    </span>
  );
}
