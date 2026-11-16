// Auto clicker: as fast as every 0.01 ms (Windows sets the real ceiling, and
// the speed reached is shown), any button, single or double, at the mouse or
// at a point, with limits. Its key starts and stops it from anywhere; Esc
// and pushing the mouse into the top-left corner always stop it.

import type { ClickConfig, ClickStatus, KeyBind } from '@shared/types';
import { Crosshair, MousePointerClick, Play, Square } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { Field, Segmented, Select, TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { toast } from '../../state/toasts';
import { KeyPicker } from '../counters/shared';

type Unit = 'ms' | 's' | 'min';
const UNIT_MS: Record<Unit, number> = { ms: 1, s: 1000, min: 60_000 };
const PRESETS: [string, number][] = [
  ['0.01 ms (fastest)', 0.01],
  ['1 ms', 1],
  ['10 ms', 10],
  ['50 ms', 50],
  ['100 ms', 100],
  ['1 s', 1000],
];

export function AutoClickerTab() {
  const [cfg, setCfg] = useState<ClickConfig | null>(null);
  const [status, setStatus] = useState<ClickStatus | null>(null);
  const [unit, setUnit] = useState<Unit>('ms');
  const [interval, setIntervalText] = useState('100');
  const [toggleLabel, setToggleLabel] = useState('F6');
  const [picking, setPicking] = useState(0);
  useEffect(() => {
    void api.autoclick.config().then((c) => {
      setCfg(c);
      const u: Unit = c.intervalMs >= 60_000 && c.intervalMs % 60_000 === 0 ? 'min' : c.intervalMs >= 1000 && c.intervalMs % 1000 === 0 ? 's' : 'ms';
      setUnit(u);
      setIntervalText(String(c.intervalMs / UNIT_MS[u]));
      void api.counters.keyName(c.toggleKey).then(setToggleLabel);
    });
    void api.autoclick.status().then(setStatus);
  }, []);
  useEvent<ClickStatus>('autoclick:status', setStatus);

  const running = status?.running ?? false;
  const apply = async (next: ClickConfig) => {
    setCfg(next);
    try {
      await api.autoclick.setConfig(next);
    } catch (e) {
      toast.error('Couldn’t use that', errorText(e));
    }
  };
  const setIntervalMs = (text: string, u = unit) => {
    setIntervalText(text);
    const v = Number(text) * UNIT_MS[u];
    if (cfg && Number.isFinite(v) && v >= 0.01) void apply({ ...cfg, intervalMs: v, jitterMs: Math.min(cfg.jitterMs, v) });
  };
  const pickPoint = () => {
    let left = 3;
    setPicking(left);
    const t = setInterval(() => {
      left -= 1;
      setPicking(left);
      if (left <= 0) {
        clearInterval(t);
        void api.autoclick.cursor().then((p) => {
          if (p && cfg) void apply({ ...cfg, at: p });
          else toast.error('Couldn’t read where the mouse is');
        });
      }
    }, 1000);
  };
  const start = async () => {
    try {
      await api.autoclick.start();
    } catch (e) {
      toast.error('Couldn’t start', errorText(e));
    }
  };
  if (!cfg) return null;
  const fast = cfg.intervalMs < 1;
  return (
    <div className="grid grid-cols-1 gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
      <Card className="space-y-4 p-5">
        <Field label="Click every" htmlFor="ac-interval">
          <div className="flex gap-2">
            <TextInput id="ac-interval" type="number" min={0.01} step="any" value={interval} disabled={running} onChange={(e) => setIntervalMs(e.target.value)} className="w-40" />
            <Segmented
              label="Unit"
              value={unit}
              onChange={(u) => {
                setUnit(u);
                setIntervalMs(interval, u);
              }}
              options={[
                { value: 'ms', label: 'ms' },
                { value: 's', label: 's' },
                { value: 'min', label: 'min' },
              ]}
            />
          </div>
        </Field>
        <div className="flex flex-wrap gap-1.5">
          {PRESETS.map(([label, ms]) => (
            <Button key={label} size="sm" variant={Math.abs(cfg.intervalMs - ms) < 1e-9 ? 'primary' : 'secondary'} disabled={running} onClick={() => (setUnit('ms'), setIntervalMs(String(ms), 'ms'))}>
              {label}
            </Button>
          ))}
        </div>
        {fast && (
          <Callout tone="info">
            Below 1 ms Windows can’t take one click per interval: it clicks as fast as it can (often several thousand a second, depending on the PC) and keeps one CPU core busy while it runs. The real speed shows on the right. Most games only count a few dozen clicks a second.
          </Callout>
        )}
        <div className="grid grid-cols-2 gap-4">
          <Field label="Button">
            <Segmented
              label="Button"
              value={cfg.button}
              onChange={(b) => void apply({ ...cfg, button: b })}
              options={[
                { value: 'left', label: 'Left' },
                { value: 'right', label: 'Right' },
                { value: 'middle', label: 'Middle' },
              ]}
            />
          </Field>
          <Field label="Click type" htmlFor="ac-type">
            <Select id="ac-type" value={cfg.double ? 'double' : 'single'} disabled={running} onChange={(e) => void apply({ ...cfg, double: e.target.value === 'double' })}>
              <option value="single">Single</option>
              <option value="double">Double</option>
            </Select>
          </Field>
          <Field label="Random ± ms (looks less robotic)" htmlFor="ac-jitter">
            <TextInput id="ac-jitter" type="number" min={0} step="any" value={cfg.jitterMs} disabled={running} onChange={(e) => void apply({ ...cfg, jitterMs: Math.min(cfg.intervalMs, Math.max(0, Number(e.target.value) || 0)) })} />
          </Field>
          <Field label="Start/stop key (works anywhere)">
            <KeyPicker
              label="Start/stop key"
              value={{ vk: cfg.toggleKey, label: toggleLabel }}
              onChange={(k: KeyBind | null) => {
                if (!k) return;
                if (k.vk === 27) return toast.error('Esc always stops it', 'Pick another key to start and stop.');
                setToggleLabel(k.label);
                void apply({ ...cfg, toggleKey: k.vk });
              }}
            />
          </Field>
          <Field label="Stop after clicks (0: never)" htmlFor="ac-max">
            <TextInput id="ac-max" type="number" min={0} value={cfg.maxClicks} disabled={running} onChange={(e) => void apply({ ...cfg, maxClicks: Math.max(0, Math.floor(Number(e.target.value) || 0)) })} />
          </Field>
          <Field label="Stop after seconds (0: never)" htmlFor="ac-secs">
            <TextInput id="ac-secs" type="number" min={0} value={cfg.maxSeconds} disabled={running} onChange={(e) => void apply({ ...cfg, maxSeconds: Math.max(0, Math.floor(Number(e.target.value) || 0)) })} />
          </Field>
        </div>
        <Field label="Where">
          <div className="flex flex-wrap items-center gap-2">
            <Segmented
              label="Where"
              value={cfg.at ? 'point' : 'mouse'}
              onChange={(v) => (v === 'mouse' ? void apply({ ...cfg, at: null }) : pickPoint())}
              options={[
                { value: 'mouse', label: 'Where the mouse is' },
                { value: 'point', label: 'At a point' },
              ]}
            />
            {cfg.at && <span className="text-[12.5px] text-dim">x {cfg.at[0]}, y {cfg.at[1]}</span>}
            <Button size="sm" icon={Crosshair} disabled={running || picking > 0} onClick={pickPoint}>
              {picking > 0 ? `Point the mouse… ${picking}` : 'Pick a point'}
            </Button>
          </div>
        </Field>
      </Card>

      <Card className="flex flex-col items-center gap-4 p-5 text-center">
        <div className={`grid h-24 w-24 place-items-center rounded-full ${running ? 'accent-gradient text-white shadow-[0_0_40px_-8px_var(--accent-glow)]' : 'bg-surface-2 text-dim'}`}>
          <MousePointerClick size={40} aria-hidden />
        </div>
        <div>
          <div className="font-display text-[34px] font-bold tabular-nums leading-none text-fg">{(status?.clicks ?? 0).toLocaleString()}</div>
          <div className="mt-1 text-[12.5px] text-dim">clicks</div>
        </div>
        <div className="grid w-full grid-cols-2 gap-2 text-[12.5px]">
          <div className="rounded-xl bg-surface px-3 py-2">
            <div className="text-faint">Real speed</div>
            <div className="font-semibold tabular-nums text-fg">{running || status?.clicks ? `${Math.round(status?.rate ?? 0).toLocaleString()}/s` : '—'}</div>
          </div>
          <div className="rounded-xl bg-surface px-3 py-2">
            <div className="text-faint">Time</div>
            <div className="font-semibold tabular-nums text-fg">{(status?.seconds ?? 0).toFixed(1)} s</div>
          </div>
        </div>
        {running ? (
          <Button size="lg" variant="danger" icon={Square} className="w-full" onClick={() => void api.autoclick.stop()}>
            Stop ({toggleLabel} or Esc)
          </Button>
        ) : (
          <Button size="lg" variant="primary" icon={Play} className="w-full" onClick={() => void start()}>
            Start ({toggleLabel})
          </Button>
        )}
        {status?.stopped && !running && <p className="text-[12px] text-dim">{status.stopped}</p>}
        <p className="text-[11.5px] leading-snug text-faint">Always stops with Esc, or by pushing the mouse into the top-left corner of the screen. {toggleLabel} starts and stops it from any app while OmniHub is running. Some online games ban auto clickers.</p>
      </Card>
    </div>
  );
}
