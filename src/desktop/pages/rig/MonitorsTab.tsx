// Monitors: brightness and contrast of each monitor (DDC/CI, the monitor's
// own setting, like Twinkle Tray), and all of them at once.

import type { Display, DisplaySetting } from '@shared/types';
import { Info, Monitor, RefreshCw, Sun } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Callout } from '../../components/ui/States';
import { toast } from '../../state/toasts';
import { Slider } from './LightingTab';

/** Monitors take a message about every 50 ms: send the newest value at most this often while dragging. */
const EVERY_MS = 150;

function useSender() {
  const pending = useRef(new Map<string, { setting: DisplaySetting; value: number }>());
  const timers = useRef(new Map<string, ReturnType<typeof setTimeout>>());
  const busy = useRef(new Set<string>());
  const failed = useRef(false);

  const flush = useCallback(async (key: string) => {
    const next = pending.current.get(key);
    if (!next || busy.current.has(key)) return;
    pending.current.delete(key);
    busy.current.add(key);
    try {
      await api.displays.set(key.split('|')[0], next.setting, next.value);
      failed.current = false;
    } catch (e) {
      if (!failed.current) toast.error('The monitor didn’t change', errorText(e));
      failed.current = true;
    } finally {
      busy.current.delete(key);
      if (pending.current.has(key)) void flush(key);
    }
  }, []);

  return useCallback(
    (id: string, setting: DisplaySetting, value: number) => {
      const key = `${id}|${setting}`;
      pending.current.set(key, { setting, value });
      if (timers.current.has(key)) return;
      void flush(key);
      timers.current.set(
        key,
        setTimeout(() => {
          timers.current.delete(key);
          void flush(key);
        }, EVERY_MS),
      );
    },
    [flush],
  );
}

export function MonitorsTab() {
  const [list, setList] = useState<Display[] | null>(null);
  const [loading, setLoading] = useState(false);
  const send = useSender();

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setList(await api.displays.list());
    } catch (e) {
      toast.error('Couldn’t read the monitors', errorText(e));
    } finally {
      setLoading(false);
    }
  }, []);
  useEffect(() => void load(), [load]);

  const change = (id: string, setting: DisplaySetting, value: number) => {
    setList((l) => l?.map((d) => (d.id === id ? { ...d, [setting]: value } : d)) ?? l);
    send(id, setting, value);
  };
  const reachable = (list ?? []).filter((d) => d.brightness != null);
  const all = reachable.length ? Math.round(reachable.reduce((n, d) => n + (d.brightness ?? 0), 0) / reachable.length) : 0;
  const setAll = (value: number) => {
    setList((l) => l?.map((d) => (d.brightness != null ? { ...d, brightness: value } : d)) ?? l);
    for (const d of reachable) send(d.id, 'brightness', value);
  };

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader
          title="Monitor brightness"
          subtitle="Changes each monitor’s own brightness, like its buttons do (DDC/CI), the same way Twinkle Tray does — both can run together."
          className="px-5 pt-4"
          actions={
            <Button size="sm" variant="ghost" icon={RefreshCw} loading={loading} onClick={() => void load()}>
              Read again
            </Button>
          }
        />
        <div className="space-y-5 px-5 pb-5 pt-3">
          {list === null && <div className="py-4 text-center text-[12.5px] text-faint">Asking the monitors…</div>}
          {list?.length === 0 && <div className="py-4 text-center text-[12.5px] text-faint">No monitors found.</div>}
          {reachable.length > 1 && (
            <div className="rounded-xl border border-line bg-surface-2/50 p-3">
              <Slider label="All monitors" value={all} min={0} max={100} step={1} onChange={setAll} format={(v) => `${v}%`} />
            </div>
          )}
          {list?.map((d) => (
            <div key={d.id} className="space-y-3">
              <div className="flex items-center gap-2">
                <Monitor className="h-4 w-4 text-accent" aria-hidden />
                <span className="truncate text-[13.5px] font-medium text-fg">{d.name}</span>
                {d.primary && <Badge tone="neutral">Main</Badge>}
                {d.builtin && <Badge tone="neutral">Built in</Badge>}
              </div>
              {d.brightness != null ? (
                <>
                  <Slider label="Brightness" value={d.brightness} min={0} max={100} step={1} onChange={(v) => change(d.id, 'brightness', v)} format={(v) => `${v}%`} />
                  {d.contrast != null && <Slider label="Contrast" value={d.contrast} min={0} max={100} step={1} onChange={(v) => change(d.id, 'contrast', v)} format={(v) => `${v}%`} />}
                </>
              ) : (
                <Callout tone="info" icon={Info} className="text-[12.5px]">
                  This monitor doesn’t answer. Turn on <b>DDC/CI</b> in its own menu (the buttons on the monitor), then press Read again. TVs and some HDMI adapters don’t support it.
                </Callout>
              )}
            </div>
          ))}
        </div>
      </Card>
      <p className="flex items-start gap-2 px-1 text-[12px] text-faint">
        <Sun className="mt-0.5 h-3.5 w-3.5 shrink-0" aria-hidden /> If Twinkle Tray’s time-of-day adjustments are on, it changes the brightness back at its set times.
      </p>
    </div>
  );
}
