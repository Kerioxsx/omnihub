// The PC's monitors from the phone: brightness for each (and contrast),
// and one slider for all of them. OmniHub sets the monitors over DDC/CI,
// like Twinkle Tray, so both work together.

import { Monitor, Sun, SunDim } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { client, type Display, type DisplaySetting } from '../client';
import { toast, useApp } from '../state';
import { errorMessage, usePageVisible } from '../lib/util';

/** Monitors take a message about every 50 ms: the newest value goes out at most this often while dragging. */
const EVERY_MS = 150;

export function useDisplays(active: boolean) {
  const enabled = !!useApp((s) => s.info?.features.media);
  const visible = usePageVisible();
  const [list, setList] = useState<Display[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const pending = useRef(new Map<string, number>());
  const timers = useRef(new Map<string, ReturnType<typeof setTimeout>>());
  const busy = useRef(new Set<string>());

  const load = useCallback(async () => {
    try {
      setList(await client.displays());
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);
  useEffect(() => {
    if (enabled && active && visible) void load();
  }, [enabled, active, visible, load]);

  const flush = useCallback(async (key: string) => {
    const value = pending.current.get(key);
    if (value == null || busy.current.has(key)) return;
    pending.current.delete(key);
    busy.current.add(key);
    const [id, setting] = key.split('|') as [string, DisplaySetting];
    try {
      await client.displaySet(id, setting, value);
    } catch (e) {
      toast.error('The monitor didn’t change', errorMessage(e));
    } finally {
      busy.current.delete(key);
      if (pending.current.has(key)) void flush(key);
    }
  }, []);

  const send = (id: string, setting: DisplaySetting, value: number) => {
    const key = `${id}|${setting}`;
    pending.current.set(key, value);
    if (timers.current.has(key)) return;
    void flush(key);
    timers.current.set(
      key,
      setTimeout(() => {
        timers.current.delete(key);
        void flush(key);
      }, EVERY_MS),
    );
  };

  const set = (id: string, setting: DisplaySetting, value: number) => {
    setList((l) => l?.map((d) => (d.id === id ? { ...d, [setting]: value } : d)) ?? l);
    send(id, setting, value);
  };
  const setAll = (value: number) => {
    const ids = (list ?? []).filter((d) => d.brightness != null).map((d) => d.id);
    setList((l) => l?.map((d) => (d.brightness != null ? { ...d, brightness: value } : d)) ?? l);
    for (const id of ids) send(id, 'brightness', value);
  };
  return { enabled, list, error, reload: load, set, setAll };
}

function Level({ label, value, onChange }: { label: string; value: number; onChange: (v: number) => void }) {
  return <input type="range" aria-label={label} min={0} max={100} step={1} value={value} onChange={(e) => onChange(Number(e.target.value))} className="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-surface-3 accent-[var(--accent)]" />;
}

/** Monitor brightness for the top of Volume & brightness. Renders nothing when there's no monitor to set. */
export function BrightnessCard({ api }: { api: ReturnType<typeof useDisplays> }) {
  const list = api.list;
  if (!api.enabled || !list?.length) return null;
  const reachable = list.filter((d) => d.brightness != null);
  const all = reachable.length ? Math.round(reachable.reduce((n, d) => n + (d.brightness ?? 0), 0) / reachable.length) : 0;
  return (
    <div className="card overflow-hidden">
      <div className="flex items-center gap-2 px-4 pt-3.5 text-[15px] font-semibold">
        <Sun size={18} className="text-amber-400" /> Monitor brightness
      </div>
      {reachable.length > 1 && (
        <div className="flex items-center gap-3 px-4 pt-3">
          <SunDim size={18} className="shrink-0 text-dim" />
          <div className="min-w-0 flex-1">
            <div className="flex justify-between text-[13.5px] font-medium">
              <span>All monitors</span>
              <span className="font-mono text-[12.5px] tabular-nums text-dim">{all}</span>
            </div>
            <div className="mt-2">
              <Level label="All monitors" value={all} onChange={api.setAll} />
            </div>
          </div>
        </div>
      )}
      <div className="divide-y divide-[var(--line)] pt-1">
        {list.map((d) => (
          <div key={d.id} className="px-4 py-3">
            <div className="flex items-center gap-2 text-[14px] font-medium">
              <Monitor size={16} className="shrink-0 text-dim" />
              <span className="truncate">{d.name}</span>
              {d.primary && <span className="rounded-full bg-surface-2 px-2 py-0.5 text-[11px] text-dim">Main</span>}
            </div>
            {d.brightness != null ? (
              <div className="mt-2 space-y-2.5">
                <div className="flex items-center gap-3">
                  <span className="w-16 shrink-0 text-[12.5px] text-dim">Brightness</span>
                  <Level label={`${d.name} brightness`} value={d.brightness} onChange={(v) => api.set(d.id, 'brightness', v)} />
                  <span className="w-8 shrink-0 text-right font-mono text-[12px] tabular-nums text-dim">{d.brightness}</span>
                </div>
                {d.contrast != null && (
                  <div className="flex items-center gap-3">
                    <span className="w-16 shrink-0 text-[12.5px] text-dim">Contrast</span>
                    <Level label={`${d.name} contrast`} value={d.contrast} onChange={(v) => api.set(d.id, 'contrast', v)} />
                    <span className="w-8 shrink-0 text-right font-mono text-[12px] tabular-nums text-dim">{d.contrast}</span>
                  </div>
                )}
              </div>
            ) : (
              <p className="mt-1.5 text-[12.5px] leading-relaxed text-faint">This monitor doesn’t answer: turn on DDC/CI in its own menu (the buttons on the monitor).</p>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
