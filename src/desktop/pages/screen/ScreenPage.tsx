import type { AdbDevice, ScrcpyOptions } from '@shared/types';
import { motion } from 'motion/react';
import { Apple, Cable, Copy, ExternalLink, Gamepad2, Gauge, Link2, Monitor, MonitorSmartphone, MousePointer2, Play, RefreshCw, ScreenShare, Smartphone, Square, Wifi, Zap } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Dot, Skeleton } from '../../components/ui/Card';
import { Checkbox, Field, Segmented, Select, Switch, TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useAsync, useStoredState } from '../../lib/hooks';
import { navigate } from '../../lib/router';
import { copyText } from '../../lib/util';
import { useLive } from '../../state/live';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';

const LATENCY: { label: string; min: number; max: number; note: string; color: string }[] = [
  { label: 'Sunshine + Moonlight', min: 10, max: 40, note: 'up to 4K60, hardware encoder', color: 'var(--accent-2)' },
  { label: 'scrcpy (Android → PC)', min: 35, max: 70, note: 'USB or Wi-Fi', color: 'var(--accent)' },
  { label: 'Built-in browser stream', min: 40, max: 90, note: 'any phone browser, good Wi-Fi', color: 'color-mix(in oklab, var(--accent) 55%, var(--accent-2))' },
];
const AXIS = 120;

function PcToPhone() {
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const remote = useLive((s) => s.remote);
  const viewers = useLive((s) => s.viewers);
  const presets = useAsync(() => api.screen.presets(), []);
  const monitors = useAsync(() => api.screen.monitors(), []);
  if (!settings) return null;
  const r = settings.remote;
  return (
    <Card className="p-5">
      <CardHeader icon={ScreenShare} title="PC → Phone" subtitle="Watch (and optionally control) this PC from any phone browser" actions={<Badge tone={remote?.running ? 'good' : 'neutral'}>{remote?.running ? 'Companion on' : 'Companion off'}</Badge>} />
      <p className="mt-3 text-[13px] leading-relaxed text-dim">
        The built-in stream needs no app: open the companion on a paired phone and tap <span className="text-fg">Screen</span>. It sends compressed frames over your Wi-Fi — ideal for checking on a download, a render or a game lobby.
      </p>
      {!remote?.running && (
        <Button size="sm" className="mt-3" icon={Smartphone} onClick={() => navigate('phone')}>
          Set up the phone companion
        </Button>
      )}
      <div className="mt-4 divide-y divide-line rounded-xl border border-line bg-surface">
        <div className="flex items-center gap-3 px-4 py-3">
          <Monitor size={16} className="text-dim" />
          <div className="flex-1">
            <div className="text-[13.5px] font-medium text-fg">Allow phones to watch the screen</div>
            <div className="text-[12px] text-faint">Paired phones only. You can stop any viewer at any time.</div>
          </div>
          <Switch checked={r.allowScreen} onChange={(v) => void update({ remote: { allowScreen: v } })} label="Allow screen sharing" />
        </div>
        <div className="flex items-center gap-3 px-4 py-3">
          <MousePointer2 size={16} className={r.allowControl ? 'text-warn' : 'text-dim'} />
          <div className="flex-1">
            <div className="text-[13.5px] font-medium text-fg">Allow remote control</div>
            <div className="text-[12px] text-faint">Tap to click, drag to move, on-screen keyboard. Off by default.</div>
          </div>
          <Switch checked={r.allowControl} disabled={!r.allowScreen} onChange={(v) => void update({ remote: { allowControl: v } })} label="Allow remote control" />
        </div>
      </div>
      <div className="mt-4">
        <div className="mb-2 text-[12px] font-medium uppercase tracking-wider text-faint">Default quality</div>
        <div className="grid grid-cols-2 gap-2">
          {presets.data
            ? presets.data.map((p) => {
                const active = settings.screen.preset === p.id;
                return (
                  <button key={p.id} type="button" onClick={() => void update({ screen: { preset: p.id } })} aria-pressed={active} className={cx('rounded-xl border px-3 py-2.5 text-left transition-colors', active ? 'accent-ring border-transparent bg-surface-2' : 'border-line bg-surface hover:border-line-strong')}>
                    <div className="text-[13px] font-medium text-fg">{p.label}</div>
                    <div className="text-[11.5px] text-faint tabular">
                      ≤ {p.maxWidth}px wide · quality {p.quality} · {p.fps} fps
                    </div>
                  </button>
                );
              })
            : [0, 1, 2, 3].map((i) => <Skeleton key={i} className="h-14" />)}
        </div>
        <div className="mt-3 flex items-center gap-2 text-[12.5px] text-dim">
          Frame-rate cap
          <Select value={String(settings.screen.maxFps)} onChange={(e) => void update({ screen: { maxFps: Number(e.target.value) } })} className="w-[110px]" aria-label="Max fps">
            {[15, 24, 30, 45, 60].map((f) => (
              <option key={f} value={f}>
                {f} fps
              </option>
            ))}
          </Select>
          {monitors.data && <span className="ml-auto text-[12px] text-faint">{monitors.data.map((m) => `${m.name} ${m.width}×${m.height}${m.primary ? ' (main)' : ''}`).join(' · ')}</span>}
        </div>
      </div>
      {viewers.length > 0 && (
        <div className="mt-4 space-y-2">
          {viewers.map((v) => (
            <div key={v.id} className="flex items-center gap-3 rounded-xl border border-bad/30 bg-bad/8 px-3.5 py-2.5">
              <Dot tone="bad" pulse />
              <div className="flex-1 text-[13px] text-fg">
                {v.device} is watching {v.monitor?.name ?? 'the screen'}
                {v.controlling ? ' and controlling it' : ''}
              </div>
              <Button size="sm" variant="danger" onClick={() => void api.remote.stopViewer(v.id)}>
                Stop sharing
              </Button>
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}

function Expectations() {
  return (
    <Card className="p-5">
      <CardHeader icon={Gauge} title="Realistic expectations" subtitle="Typical glass-to-glass latency" />
      <p className="mt-3 text-[13px] leading-relaxed text-dim">
        No stream is zero-latency: every frame is <span className="text-fg">captured, encoded, sent over the network and decoded</span>. Wi-Fi quality matters more than anything else — 5 GHz, close to the router, or a cable for the PC.
      </p>
      <div className="mt-5 space-y-4" role="img" aria-label="Latency ranges: Sunshine plus Moonlight 10 to 40 ms, scrcpy 35 to 70 ms, built-in stream 40 to 90 ms">
        {LATENCY.map((l, i) => (
          <div key={l.label}>
            <div className="mb-1.5 flex items-baseline justify-between text-[12.5px]">
              <span className="font-medium text-fg">{l.label}</span>
              <span className="tabular text-dim">
                {l.min}–{l.max} ms
              </span>
            </div>
            <div className="relative h-2.5 rounded-full bg-surface-3">
              <motion.div className="absolute inset-y-0 rounded-full" style={{ background: l.color, left: `${(l.min / AXIS) * 100}%` }} initial={{ width: 0 }} animate={{ width: `${((l.max - l.min) / AXIS) * 100}%` }} transition={{ type: 'spring', stiffness: 80, damping: 18, delay: 0.1 + i * 0.08 }} />
            </div>
            <div className="mt-1 text-[11.5px] text-faint">{l.note}</div>
          </div>
        ))}
        <div className="relative h-4 text-[10.5px] text-faint tabular">
          {[0, 30, 60, 90, 120].map((t) => (
            <span key={t} className="absolute -translate-x-1/2" style={{ left: `${(t / AXIS) * 100}%` }}>
              {t}
            </span>
          ))}
        </div>
      </div>
      <div className="mt-2 text-[12px] text-faint">For games, use Sunshine + Moonlight. For a quick look from the couch, the built-in stream is enough.</div>
    </Card>
  );
}

function Sunshine() {
  const s = useAsync(() => api.screen.sunshineStatus(), []);
  const st = s.data;
  return (
    <Card className="p-5">
      <CardHeader icon={Gamepad2} title="Sunshine + Moonlight" subtitle="Low-latency game streaming (open source)" actions={<IconButton icon={RefreshCw} label="Check again" size="sm" onClick={() => void s.reload()} />} />
      {st ? (
        <>
          <div className="mt-3 flex items-center gap-2 rounded-xl border border-line bg-surface px-3.5 py-2.5">
            <Dot tone={st.running ? 'good' : st.installed ? 'warn' : 'neutral'} />
            <span className="shrink-0 text-[13px] text-fg">{st.running ? 'Sunshine is running' : st.installed ? 'Installed, not running' : 'Not installed'}</span>
            {st.path && <span className="ml-auto min-w-0 truncate font-mono text-[11px] text-faint" title={st.path}>{st.path}</span>}
          </div>
          <p className="mt-3 text-[12.5px] leading-relaxed text-dim">Sunshine runs on this PC and uses your GPU's hardware encoder; Moonlight is the client on your phone, tablet or TV. Pair them once with a PIN in Sunshine's web UI.</p>
          <div className="mt-3 flex flex-wrap gap-2">
            {st.installed && (
              <Button size="sm" variant="primary" iconRight={ExternalLink} onClick={() => void api.app.openUrl(st.webUi)}>
                Open Sunshine web UI
              </Button>
            )}
            <Button size="sm" iconRight={ExternalLink} onClick={() => void api.app.openUrl('https://app.lizardbyte.dev')}>
              {st.installed ? 'Sunshine website' : 'Get Sunshine'}
            </Button>
            <Button size="sm" iconRight={ExternalLink} onClick={() => void api.app.openUrl('https://moonlight-stream.org')}>
              Moonlight for your phone
            </Button>
          </div>
        </>
      ) : (
        <Skeleton className="mt-3 h-24" />
      )}
    </Card>
  );
}

type PresetId = 'lowLatency' | 'quality' | 'battery' | 'custom';
const PRESET_HINT: Record<PresetId, string> = { lowLatency: '1920px · 10 Mbit/s · 60 fps · H.264', quality: 'Native resolution · 24 Mbit/s · 60 fps · H.265', battery: '1280px · 4 Mbit/s · 30 fps · H.264', custom: 'Your own values' };

function Scrcpy() {
  const status = useAsync(() => api.screen.scrcpyStatus(), []);
  const [serial, setSerial] = useState('');
  const [opts, setOpts] = useStoredState<Omit<ScrcpyOptions, 'serial'>>('omnihub.scrcpy', { preset: 'lowLatency', maxSize: 1920, bitrateMbps: 10, maxFps: 60, codec: 'h264', audio: true, turnScreenOff: false, stayAwake: true, fullscreen: false, alwaysOnTop: false, control: true });
  const [busy, setBusy] = useState<string | null>(null);
  const [addr, setAddr] = useState('');
  const [pairAddr, setPairAddr] = useState('');
  const [code, setCode] = useState('');
  const st = status.data;
  const devices = st?.devices ?? [];

  useEffect(() => {
    if (!serial && devices[0]) setSerial(devices[0].serial);
  }, [devices, serial]);

  const run = async (key: string, fn: () => Promise<unknown>, ok?: (r: unknown) => string) => {
    setBusy(key);
    try {
      const r = await fn();
      if (ok) toast.success(ok(r));
      await status.reload();
    } catch (e) {
      toast.error('scrcpy', errorText(e));
    } finally {
      setBusy(null);
    }
  };
  const set = (p: Partial<typeof opts>) => setOpts({ ...opts, ...p });
  const custom = opts.preset === 'custom';
  const selected: AdbDevice | undefined = devices.find((d) => d.serial === serial);

  return (
    <Card className="p-5">
      <CardHeader icon={MonitorSmartphone} title="Phone → PC (Android)" subtitle="Mirror and control an Android phone with scrcpy" actions={<IconButton icon={RefreshCw} label="Refresh devices" size="sm" onClick={() => void status.reload()} />} />
      {!st ? (
        <Skeleton className="mt-4 h-40" />
      ) : !st.found ? (
        <Callout tone="info" icon={Zap} className="mt-4" title="scrcpy is not installed">
          Install it with winget, then click refresh:
          <div className="mt-2 flex items-center gap-1 rounded-lg border border-line bg-surface py-0.5 pl-3 pr-0.5">
            <code className="flex-1 font-mono text-[12.5px] text-fg">{st.installHint}</code>
            <IconButton icon={Copy} label="Copy command" size="sm" onClick={() => void copyText(st.installHint).then(() => toast.success('Command copied'))} />
          </div>
        </Callout>
      ) : (
        <div className="mt-4 grid grid-cols-1 gap-5 xl:grid-cols-[minmax(0,1.15fr)_minmax(0,1fr)]">
          <div className="space-y-4">
            <div className="flex items-center gap-2 text-[12.5px] text-dim">
              <Badge tone="good">scrcpy {st.version}</Badge>
              <span className="truncate font-mono text-[11px] text-faint" title={st.path ?? ''}>
                {st.path}
              </span>
              <span className="ml-auto flex items-center gap-1.5 whitespace-nowrap text-[11.5px] text-faint">
                <Copy size={11} />
                <button type="button" className="hover:text-fg" onClick={() => void copyText(st.installHint).then(() => toast.success('Command copied'))}>
                  Update: {st.installHint.replace('install', 'upgrade')}
                </button>
              </span>
            </div>
            <div>
              <div className="mb-1.5 text-[12px] font-medium uppercase tracking-wider text-faint">Devices (adb)</div>
              <div className="space-y-1.5" role="radiogroup" aria-label="Device">
                {devices.map((d) => (
                  <button key={d.serial} type="button" role="radio" aria-checked={serial === d.serial} onClick={() => setSerial(d.serial)} className={cx('flex w-full items-center gap-3 rounded-xl border px-3 py-2 text-left transition-colors', serial === d.serial ? 'accent-ring border-transparent bg-surface-2' : 'border-line bg-surface hover:border-line-strong')}>
                    {d.wireless ? <Wifi size={16} className="text-info" /> : <Cable size={16} className="text-dim" />}
                    <span className="min-w-0 flex-1">
                      <span className="block text-[13px] font-medium text-fg">{d.model}</span>
                      <span className="block font-mono text-[11px] text-faint">{d.serial}</span>
                    </span>
                    <Badge tone={d.state === 'device' ? 'good' : 'warn'}>{d.state === 'device' ? (d.wireless ? 'Wi-Fi' : 'USB') : d.state}</Badge>
                  </button>
                ))}
                {!devices.length && <div className="rounded-xl border border-dashed border-line px-3 py-4 text-[12.5px] text-faint">No device. Enable Developer options → USB debugging on the phone, plug it in and accept the prompt.</div>}
              </div>
            </div>
            <div>
              <div className="mb-1.5 text-[12px] font-medium uppercase tracking-wider text-faint">Preset</div>
              <Segmented<PresetId>
                label="Preset"
                value={opts.preset as PresetId}
                onChange={(v) => set({ preset: v })}
                options={[
                  { value: 'lowLatency', label: 'Low latency' },
                  { value: 'quality', label: 'Quality' },
                  { value: 'battery', label: 'Battery' },
                  { value: 'custom', label: 'Custom' },
                ]}
              />
              <div className="mt-1.5 text-[12px] text-faint">{PRESET_HINT[opts.preset as PresetId]}</div>
              {custom && (
                <div className="mt-3 grid grid-cols-4 gap-2">
                  <Field label="Max size (px)" htmlFor="sc-size">
                    <TextInput id="sc-size" type="number" min={0} value={opts.maxSize ?? ''} onChange={(e) => set({ maxSize: e.target.value ? Number(e.target.value) : null })} inputSize="sm" />
                  </Field>
                  <Field label="Bitrate (Mbit/s)" htmlFor="sc-bit">
                    <TextInput id="sc-bit" type="number" min={1} value={opts.bitrateMbps ?? ''} onChange={(e) => set({ bitrateMbps: e.target.value ? Number(e.target.value) : null })} inputSize="sm" />
                  </Field>
                  <Field label="Max fps" htmlFor="sc-fps">
                    <TextInput id="sc-fps" type="number" min={1} max={240} value={opts.maxFps ?? ''} onChange={(e) => set({ maxFps: e.target.value ? Number(e.target.value) : null })} inputSize="sm" />
                  </Field>
                  <Field label="Codec" htmlFor="sc-codec">
                    <Select id="sc-codec" value={opts.codec ?? 'h264'} onChange={(e) => set({ codec: e.target.value as 'h264' | 'h265' | 'av1' })}>
                      <option value="h264">H.264</option>
                      <option value="h265">H.265</option>
                      <option value="av1">AV1</option>
                    </Select>
                  </Field>
                </div>
              )}
            </div>
            <div className="grid grid-cols-3 gap-2">
              <Checkbox checked={opts.audio} onChange={(v) => set({ audio: v })} label="Forward audio" />
              <Checkbox checked={opts.control} onChange={(v) => set({ control: v })} label="Mouse & keyboard" />
              <Checkbox checked={opts.turnScreenOff} onChange={(v) => set({ turnScreenOff: v })} label="Phone screen off" />
              <Checkbox checked={opts.stayAwake} onChange={(v) => set({ stayAwake: v })} label="Stay awake" />
              <Checkbox checked={opts.fullscreen} onChange={(v) => set({ fullscreen: v })} label="Fullscreen" />
              <Checkbox checked={opts.alwaysOnTop} onChange={(v) => set({ alwaysOnTop: v })} label="Always on top" />
            </div>
            <div className="flex items-center gap-2">
              {st.running ? (
                <Button variant="danger" icon={Square} loading={busy === 'stop'} onClick={() => void run('stop', () => api.screen.scrcpyStop(), () => 'Mirroring stopped')}>
                  Stop mirroring
                </Button>
              ) : (
                <Button variant="primary" icon={Play} loading={busy === 'launch'} disabled={!devices.length} onClick={() => void run('launch', () => api.screen.scrcpyLaunch({ ...opts, serial: serial || null }), () => `Mirroring ${selected?.model ?? 'the phone'}`)}>
                  Start mirroring
                </Button>
              )}
              {st.running && (
                <span className="flex items-center gap-2 text-[12.5px] text-good">
                  <Dot tone="good" pulse /> scrcpy window is open
                </span>
              )}
            </div>
          </div>
          <div className="space-y-4 rounded-xl border border-line bg-surface p-4">
            <div className="flex items-center gap-2 text-[13.5px] font-semibold text-fg">
              <Wifi size={15} className="text-info" /> Go wireless
            </div>
            <div>
              <div className="text-[12.5px] text-dim">Phone plugged in by USB? Switch it to Wi-Fi in one click, then unplug.</div>
              <Button size="sm" className="mt-2" icon={Wifi} disabled={!selected || selected.wireless} loading={busy === 'wireless'} onClick={() => void run('wireless', () => api.screen.scrcpyWireless(serial), (r) => `Now on Wi-Fi at ${String(r)} — you can unplug the cable`)}>
                Switch {selected && !selected.wireless ? selected.model : 'USB device'} to Wi-Fi
              </Button>
            </div>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                if (addr) void run('connect', () => api.screen.scrcpyConnect(addr), (r) => String(r));
              }}
            >
              <Field label="Connect by address" htmlFor="sc-addr" hint="The phone's IP and port, e.g. 192.168.1.60:5555">
                <div className="flex gap-2">
                  <TextInput id="sc-addr" value={addr} onChange={(e) => setAddr(e.target.value)} placeholder="192.168.1.60:5555" className="flex-1" inputSize="sm" />
                  <Button size="sm" type="submit" icon={Link2} loading={busy === 'connect'} disabled={!addr}>
                    Connect
                  </Button>
                </div>
              </Field>
            </form>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                if (pairAddr && code) void run('pair', () => api.screen.scrcpyPair(pairAddr, code), (r) => String(r));
              }}
            >
              <div className="mb-1.5 text-[12.5px] font-medium text-dim">Android 11+ wireless pairing</div>
              <div className="mb-2 text-[11.5px] leading-relaxed text-faint">Developer options → Wireless debugging → Pair device with pairing code.</div>
              <div className="grid grid-cols-[minmax(0,1fr)_96px_auto] gap-2">
                <TextInput value={pairAddr} onChange={(e) => setPairAddr(e.target.value)} placeholder="192.168.1.60:37199" aria-label="Pairing address" inputSize="sm" />
                <TextInput value={code} onChange={(e) => setCode(e.target.value.replace(/\D/g, '').slice(0, 6))} placeholder="Code" aria-label="Pairing code" inputSize="sm" className="[&_input]:font-mono" />
                <Button size="sm" type="submit" loading={busy === 'pair'} disabled={!pairAddr || code.length !== 6}>
                  Pair
                </Button>
              </div>
            </form>
          </div>
        </div>
      )}
    </Card>
  );
}

export function ScreenPage() {
  return (
    <Page title="Screen share" subtitle="Stream this PC to your phone, or mirror an Android phone on this PC.">
      <div className="space-y-3">
        <div className="grid grid-cols-12 items-start gap-3">
          <div className="col-span-12 xl:col-span-7">
            <PcToPhone />
          </div>
          <div className="col-span-12 space-y-3 xl:col-span-5">
            <Expectations />
            <Sunshine />
          </div>
        </div>
        <Scrcpy />
        <Callout tone="info" icon={Apple} title="iPhone and iPad">
          iOS has no public API for low-level screen mirroring or input, so scrcpy-style control isn't possible. The built-in browser stream works on iPhone (PC → phone). For phone → PC, AirPlay receiver apps exist, but they are limited (no control, variable latency).
        </Callout>
      </div>
    </Page>
  );
}
