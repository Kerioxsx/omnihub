// One Govee light: what it shows now, its switch and brightness, whether it
// follows all the lights or has its own look, where it sits around the
// screen, and its own settings from Govee (gradient, night light, a
// speaker's volume…).

import type { GoveeExtra, GoveeLight, GoveeLightSettings, GoveeLook, GoveeSettings, GoveeZone, MonitorInfo } from '@shared/types';
import { ChevronDown, Cloud, Eye, Pencil, Radio, Volume2, Wifi } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card } from '../../components/ui/Card';
import { Segmented, Select, Switch, TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { toast } from '../../state/toasts';
import { Slider } from '../rig/LightingTab';
import { LiveStrip } from './LiveStrip';
import { LookEditor } from './LookEditor';
import type { Govee } from './useGovee';
import { lightSettings } from './useGovee';

const ZONES: { value: GoveeZone; label: string }[] = [
  { value: 'whole', label: 'The whole screen' },
  { value: 'around', label: 'All around (behind the screen)' },
  { value: 'left', label: 'Left side' },
  { value: 'right', label: 'Right side' },
  { value: 'top', label: 'Top' },
  { value: 'bottom', label: 'Bottom' },
  { value: 'across', label: 'Across, left to right' },
];

function ExtraControl({ light, extra }: { light: GoveeLight; extra: GoveeExtra }) {
  const value = light.values[extra.instance];
  const [v, setV] = useState<number>(typeof value === 'number' ? value : extra.min ?? 0);
  const send = (val: unknown) => void api.govee.control(light.id, extra.type, extra.instance, val).catch((e: unknown) => toast.error(`Could not change ${extra.label}`, errorText(e)));
  const label = (
    <span className="flex items-center gap-1.5 text-[12.5px] font-medium text-fg">
      {extra.sound && <Volume2 size={13} className="text-accent" />}
      {extra.label}
    </span>
  );
  if (extra.kind === 'toggle') {
    return (
      <div className="flex items-center justify-between gap-3">
        {label}
        <Switch checked={value === 1 || value === true} onChange={(on) => send(on ? 1 : 0)} label={extra.label} />
      </div>
    );
  }
  if (extra.kind === 'range') {
    return (
      <div onPointerUp={() => send(v)} onKeyUp={() => send(v)} className="flex items-end gap-2.5">
        {extra.sound && <Volume2 size={15} className="mb-0.5 shrink-0 text-accent" />}
        <div className="flex-1">
          <Slider label={extra.label} value={v} min={extra.min ?? 0} max={extra.max ?? 100} step={1} onChange={setV} format={(x) => `${x}${extra.unit === 'percent' ? '%' : ''}`} />
        </div>
      </div>
    );
  }
  return (
    <div className="flex items-center justify-between gap-3">
      {label}
      <Select value={JSON.stringify(value ?? null)} onChange={(e) => send(JSON.parse(e.target.value))} className="w-40">
        {value == null && <option value="null">Choose…</option>}
        {extra.choices.map((c) => (
          <option key={c.name} value={JSON.stringify(c.value)}>
            {c.name}
          </option>
        ))}
      </Select>
    </div>
  );
}

export function LightCard({ light, govee, monitors }: { light: GoveeLight; govee: Govee; monitors: MonitorInfo[] }) {
  const s = govee.settings as GoveeSettings;
  const ls: GoveeLightSettings = lightSettings(s, light.id);
  const [open, setOpen] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const set = (patch: Parameters<Govee['updateLight']>[1]) => void govee.updateLight(light.id, patch);
  const look: GoveeLook = ls.follow ? s.look : ls.look;
  const on = s.enabled ? ls.on && (!ls.follow || s.on) : !!light.on;
  const brightness = ls.follow ? s.brightness : ls.brightness;
  const fallback = look.mode === 'color' ? [look.color] : light.color ? [light.color] : undefined;
  const segs = ls.segments || light.segments || 10;

  return (
    <Card className="overflow-hidden">
      <div className="space-y-3 px-5 py-4">
        <div className="flex items-start gap-3">
          <div className="min-w-0 flex-1">
            {renaming ? (
              <TextInput
                autoFocus
                defaultValue={ls.name || light.name}
                inputSize="sm"
                onBlur={(e) => {
                  setRenaming(false);
                  if (e.target.value.trim() !== light.name) set({ name: e.target.value.trim() });
                }}
                onKeyDown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()}
                aria-label="Name"
              />
            ) : (
              <div className="flex items-center gap-1.5">
                <span className="truncate text-[14.5px] font-semibold text-fg">{light.name}</span>
                <IconButton icon={Pencil} label="Rename" size="sm" onClick={() => setRenaming(true)} />
              </div>
            )}
            <div className="mt-0.5 flex flex-wrap items-center gap-1.5 text-[11.5px] text-faint">
              <span className="font-mono">{light.sku || 'Govee'}</span>
              {light.ip && <span className="font-mono">· {light.ip}</span>}
              {light.lan && (
                <Badge tone="good" icon={Wifi}>
                  LAN
                </Badge>
              )}
              {light.cloud && (
                <Badge tone="info" icon={Cloud}>
                  Cloud
                </Badge>
              )}
              {light.streaming && (
                <Badge tone="accent" icon={Radio}>
                  Live
                </Badge>
              )}
            </div>
          </div>
          <IconButton icon={Eye} label="Blink it so you can tell which it is" onClick={() => void api.govee.identify(light.id)} disabled={!light.lan} />
          <Switch checked={on && ls.control} onChange={(v) => set(ls.follow && v && !s.on ? { follow: false, on: true, look: s.look } : { on: v, control: true })} label={`${light.name} on`} disabled={!s.enabled} />
        </div>

        <LiveStrip id={light.id} fallback={fallback} className={cx(!on && 'opacity-30')} />

        {light.message && <div className="text-[12px] text-warn">{light.message}</div>}

        <div className="grid gap-3 sm:grid-cols-[1fr_auto] sm:items-end">
          <Slider label={ls.follow ? 'Brightness (all lights)' : 'Brightness'} value={brightness} min={1} max={100} step={1} onChange={(v) => (ls.follow ? void govee.update({ brightness: v }) : set({ brightness: v }))} format={(v) => `${v}%`} />
          <label className="flex items-center gap-2 text-[12.5px] text-dim">
            Same as all lights
            <Switch checked={ls.follow} onChange={(v) => set(v ? { follow: true } : { follow: false, look: s.look, brightness: s.brightness })} label="Same as all lights" />
          </label>
        </div>
      </div>

      {!ls.follow && (
        <div className="border-t border-line px-5 py-4">
          <LookEditor look={ls.look} onChange={(p) => set({ look: p })} lights={[light]} monitors={monitors} palette={govee.status?.palette ?? []} compact />
        </div>
      )}

      <button type="button" onClick={() => setOpen((o) => !o)} className="flex w-full items-center justify-between border-t border-line px-5 py-2.5 text-[12.5px] text-dim hover:bg-surface-2/50 hover:text-fg" aria-expanded={open}>
        Placement, segments{light.extras.length ? ', and its own settings' : ''}
        <ChevronDown size={15} className={cx('transition-transform', open && 'rotate-180')} />
      </button>
      {open && (
        <div className="space-y-3.5 border-t border-line px-5 py-4">
          <label className="flex items-center justify-between gap-3 text-[12.5px] font-medium text-fg">
            Place around the screen
            <Select value={ls.zone} onChange={(e) => set({ zone: e.target.value as GoveeZone })} className="w-60">
              {ZONES.map((z) => (
                <option key={z.value} value={z.value}>
                  {z.label}
                </option>
              ))}
            </Select>
          </label>
          <label className="flex items-center justify-between gap-3 text-[12.5px] font-medium text-fg">
            <span>
              Segments
              <span className="block text-[11.5px] font-normal text-faint">{light.segments ? `Govee says ${light.segments}.` : 'If live colours don’t show, try the number your light has.'}</span>
            </span>
            <Select value={ls.segments} onChange={(e) => set({ segments: Number(e.target.value) })} className="w-32">
              <option value={0}>Auto ({light.segments ?? 10})</option>
              {[1, 3, 4, 5, 6, 7, 8, 10, 12, 14, 15, 16, 20, 24, 30, 40, 48, 60, 72, 84].map((n) => (
                <option key={n} value={n}>
                  {n}
                </option>
              ))}
            </Select>
          </label>
          <div className="flex items-center justify-between gap-3 text-[12.5px] font-medium text-fg">
            Live colours
            <Segmented
              label="Live colours"
              size="sm"
              value={ls.stream}
              onChange={(v) => set({ stream: v })}
              options={[
                { value: 'auto', label: 'Auto' },
                { value: 'segments', label: `Per segment (${segs})` },
                { value: 'single', label: 'One colour' },
              ]}
            />
          </div>
          <p className="-mt-1 text-[11.5px] text-faint">Auto: a colour per segment for lights Govee’s account (your API key) or OmniHub knows can take it, one colour for the rest, which every light takes. A lamp with many LEDs that isn’t known: try Per segment; if it then stays still, go back to Auto.</p>
          <div className="flex flex-wrap gap-x-6 gap-y-2">
            <label className="flex items-center gap-2 text-[12.5px] text-dim">
              <Switch checked={ls.gradient} onChange={(v) => set({ gradient: v })} label="Blend segments" /> Blend segments
            </label>
            <label className="flex items-center gap-2 text-[12.5px] text-dim">
              <Switch checked={ls.reverse} onChange={(v) => set({ reverse: v })} label="Reverse" /> Runs the other way
            </label>
            <label className="flex items-center gap-2 text-[12.5px] text-dim">
              <Switch checked={ls.control} onChange={(v) => set({ control: v })} label="OmniHub controls it" /> OmniHub controls it
            </label>
          </div>
          {light.extras.length > 0 && (
            <div className="space-y-3 rounded-xl border border-line bg-surface-2/40 p-3.5">
              <div className="text-[11.5px] font-semibold uppercase tracking-wide text-faint">Its own settings (Govee)</div>
              {light.extras.map((x) => (
                <ExtraControl key={x.instance} light={light} extra={x} />
              ))}
            </div>
          )}
          {!light.lan && (
            <Callout tone="info" title="Turn on LAN Control for live colours">
              In the Govee Home app, open this light → settings (⚙) → <b>LAN Control</b>. Screen, music and effects stream over your network; scenes and the rest work through Govee’s cloud.
            </Callout>
          )}
          {light.cloud && light.scenes.length + light.diyScenes.length > 0 && (
            <label className="flex items-center justify-between gap-3 text-[12.5px] font-medium text-fg">
              A scene on this light only
              <Select value="" onChange={(e) => e.target.value && set({ follow: false, look: { mode: 'scene', scene: e.target.value } })} className="w-52">
                <option value="">Choose…</option>
                {[...light.scenes, ...light.diyScenes].map((c) => (
                  <option key={c.name} value={c.name}>
                    {c.name}
                  </option>
                ))}
              </Select>
            </label>
          )}
          {light.lan && (
            <Button size="sm" variant="secondary" icon={Eye} onClick={() => void api.govee.identify(light.id)}>
              Blink it
            </Button>
          )}
        </div>
      )}
    </Card>
  );
}
