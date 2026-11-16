// Lighting: OmniHub drives every RGB part OpenRGB knows, with effects and
// colour profiles; the 3D view previews a profile while it is edited.

import type { LightEffect, LightProfile } from '@shared/types';
import { Copy, ExternalLink, Lightbulb, Play, Plus, Save, Trash2, X } from 'lucide-react';
import { useEffect, useId, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Switch, TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { EFFECTS, colorAt, hex, palette } from '../../lib/rig/effects';
import { toast } from '../../state/toasts';
import { Row } from '../games/shared';
import { activeProfile, type Rig } from './useRig';

export const KIND_LABEL: Record<string, string> = { motherboard: 'Motherboard', memory: 'Memory', gpu: 'Graphics card', cooler: 'Cooler', ledStrip: 'LED strip', case: 'Case', light: 'Light', storage: 'Storage', keyboard: 'Keyboard', mouse: 'Mouse', mousemat: 'Mouse pad', headset: 'Headset', headsetStand: 'Headset stand', gamepad: 'Controller', speaker: 'Speaker', microphone: 'Microphone', keypad: 'Keypad', accessory: 'Accessory', virtual: 'Virtual', other: 'Other' };

const PERIPHERALS: string[] = ['keyboard', 'mouse', 'mousemat', 'headset', 'headsetStand', 'gamepad', 'speaker', 'microphone', 'keypad'];

/** A strip of the profile's look, for its card. */
function Swatch({ p, className }: { p: LightProfile; className?: string }) {
  const colors = palette(p);
  const stops = Array.from({ length: 8 }, (_, i) => hex(colorAt({ ...p, brightness: 100 }, colors, i / 7, 0, { t: p.effect === 'breathing' ? 1.25 / Math.max(0.2, p.speed) : 0, level: 0.7, beat: 0.4, temp: 45 + i * 6 })));
  return <div className={cx('h-2 rounded-full', className)} style={{ background: p.effect === 'off' ? '#15161c' : `linear-gradient(90deg, ${stops.join(', ')})` }} />;
}

export function Slider({ label, value, min, max, step, onChange, format }: { label: string; value: number; min: number; max: number; step: number; onChange: (v: number) => void; format: (v: number) => string }) {
  const id = useId();
  const pct = ((value - min) / (max - min)) * 100;
  return (
    <div className="space-y-1.5">
      <div className="flex items-baseline justify-between text-[12.5px]">
        <label htmlFor={id} className="font-medium text-fg">
          {label}
        </label>
        <span className="font-mono text-[11.5px] tabular-nums text-faint">{format(value)}</span>
      </div>
      <input id={id} type="range" min={min} max={max} step={step} value={value} onChange={(e) => onChange(Number(e.target.value))} className="h-1.5 w-full cursor-pointer appearance-none rounded-full [&::-webkit-slider-thumb]:h-4 [&::-webkit-slider-thumb]:w-4 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:shadow" style={{ background: `linear-gradient(90deg, var(--color-accent, #3b82f6) ${pct}%, rgba(127,127,140,.25) 0)` }} />
    </div>
  );
}

export function LightingTab({ rig, onPreview }: { rig: Rig; onPreview: (p: LightProfile | null) => void }) {
  const s = rig.settings;
  const st = rig.status;
  const current = activeProfile(rig);
  const [draft, setDraft] = useState<LightProfile | null>(null);
  useEffect(() => onPreview(draft), [draft, onPreview]);
  useEffect(() => () => onPreview(null), [onPreview]);
  if (!s) return null;
  const builtIn = (p: LightProfile) => !p.id.startsWith('my-');

  const pick = (p: LightProfile) => {
    setDraft(null);
    void rig.update({ lighting: { profile: p.id, enabled: true } });
  };
  const save = async () => {
    if (!draft) return;
    try {
      await api.rig.saveProfile(draft);
      rig.reloadProfiles();
      setDraft(null);
      toast.success('Lighting profile saved', draft.name);
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  };
  const remove = async (p: LightProfile) => {
    try {
      await api.rig.deleteProfile(p.id);
      rig.reloadProfiles();
      if (draft?.id === p.id) setDraft(null);
    } catch (e) {
      toast.error('Not removed', errorText(e));
    }
  };
  const effect = EFFECTS.find((e) => e.value === draft?.effect);

  return (
    <div className="space-y-4">
      <Card>
        <Row title="Control my PC's lighting" hint="Motherboard, memory, graphics card, cooler, fans and strips: everything OpenRGB knows, from ASUS, MSI, Gigabyte, Corsair, NZXT, Lian Li, Razer and more.">
          <Switch checked={s.lighting.enabled} onChange={(v) => void rig.update({ lighting: { enabled: v } })} label="Control my PC's lighting" />
        </Row>
        {s.lighting.enabled && st?.lighting === 'on' && (
          <div className="flex flex-wrap items-center gap-2 border-t border-line px-5 py-3 text-[12.5px] text-dim">
            <Badge tone="good">OpenRGB connected</Badge>
            {st.devices.filter((d) => d.on).length} of {st.devices.length} devices lit by OmniHub
            {st.lightingMessage && <span className="text-warn">· {st.lightingMessage}</span>}
          </div>
        )}
        {s.lighting.enabled && st && st.lightingRivals.length > 0 && (
          <div className="border-t border-line p-4">
            <Callout tone="warn" icon={Lightbulb} title={`${st.lightingRivals.join(', ')} ${st.lightingRivals.length > 1 ? 'are' : 'is'} also changing your lights`}>
              Two programs setting the same keyboard or lights make the colours flicker and lag. Close {st.lightingRivals.length > 1 ? 'them' : 'it'} (also from the tray), or turn off {st.lightingRivals.length > 1 ? 'their' : 'its'} lighting control, or untick those devices below so only one program drives each.
              {st.lightingRivals.includes('SteelSeries GG') && (
                <span className="mt-1.5 block">
                  SteelSeries: in SteelSeries GG open <b>Engine</b> → your keyboard and turn off <b>Prism</b> (PrismSync) for it, or quit GG from the tray. Your keyboard’s own settings in GG stay as they are.
                </span>
              )}
            </Callout>
          </div>
        )}
        {s.lighting.enabled && st && (st.lighting === 'missing' || st.lighting === 'error') && (
          <div className="border-t border-line p-4">
            <Callout
              tone="warn"
              icon={Lightbulb}
              title={st.openrgbInstalled ? 'OpenRGB is installed but its SDK server isn’t on' : 'OmniHub lights your PC through OpenRGB'}
              action={
                st.openrgbInstalled ? (
                  <Button size="sm" icon={Play} onClick={() => void api.rig.startOpenRgb().then(() => toast.info('OpenRGB is starting', 'It stays in the tray; the lights connect in a few seconds.'), (e: unknown) => toast.error('OpenRGB did not start', errorText(e)))}>
                    Start OpenRGB
                  </Button>
                ) : (
                  <Button size="sm" variant="secondary" icon={ExternalLink} onClick={() => void api.app.openUrl('https://openrgb.org/releases.html')}>
                    Get OpenRGB
                  </Button>
                )
              }
            >
              {st.openrgbInstalled ? 'Start it here (it runs with its SDK server, in the tray), or in OpenRGB open SDK Server → Start Server.' : 'Install OpenRGB (free, open source), run it once as administrator so it finds the motherboard and memory lights, then come back here. The 3D view already shows each profile.'}
              {st.lightingMessage && st.lighting === 'error' && <div className="mt-1 text-faint">{st.lightingMessage}</div>}
            </Callout>
          </div>
        )}
      </Card>

      <Card>
        <CardHeader title="Profiles" subtitle="Pick one to light the PC with it; edit a copy to make your own." className="px-5 pt-4" actions={current && <Button size="sm" variant="secondary" icon={Plus} onClick={() => setDraft({ ...current, id: '', name: `${current.name} (mine)`, colors: current.colors.length ? current.colors : ['#00a2ff'] })}>New from current</Button>} />
        <div className="grid gap-2 p-4 sm:grid-cols-2 xl:grid-cols-3">
          {rig.profiles.map((p) => {
            const on = p.id === current?.id;
            return (
              <div key={p.id} className={cx('group relative rounded-xl border p-3 transition-colors', on ? 'border-accent/60 bg-accent-soft' : 'border-line bg-surface-2/50 hover:border-line-strong')}>
                <button type="button" onClick={() => pick(p)} className="block w-full text-left" aria-pressed={on}>
                  <div className="mb-2 flex items-center justify-between gap-2">
                    <span className="truncate text-[13.5px] font-semibold text-fg">{p.name}</span>
                    <span className="shrink-0 text-[11px] text-faint">{EFFECTS.find((e) => e.value === p.effect)?.label}</span>
                  </div>
                  <Swatch p={p} />
                </button>
                <div className="absolute right-2 top-9 hidden gap-0.5 group-hover:flex">
                  <IconButton icon={Copy} size="sm" label="Edit a copy" onClick={() => setDraft({ ...p, id: builtIn(p) ? '' : p.id, name: builtIn(p) ? `${p.name} (mine)` : p.name, colors: p.colors.length ? p.colors : ['#00a2ff'] })} />
                  {!builtIn(p) && <IconButton icon={Trash2} size="sm" variant="danger" label="Delete profile" onClick={() => void remove(p)} />}
                </div>
              </div>
            );
          })}
        </div>
      </Card>

      {draft && (
        <Card className="border-accent/40">
          <CardHeader title="Edit profile" subtitle="The 3D view shows it as you change it; Save puts it on the PC." className="px-5 pt-4" actions={<IconButton icon={X} label="Close the editor" onClick={() => setDraft(null)} />} />
          <div className="space-y-4 p-5">
            <TextInput value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} placeholder="Profile name" aria-label="Profile name" maxLength={40} />
            <div className="flex flex-wrap gap-1.5" role="radiogroup" aria-label="Effect">
              {EFFECTS.map((e) => (
                <button key={e.value} type="button" role="radio" aria-checked={draft.effect === e.value} title={e.hint} onClick={() => setDraft({ ...draft, effect: e.value as LightEffect })} className={cx('rounded-lg border px-2.5 py-1.5 text-[12.5px] font-medium transition-colors', draft.effect === e.value ? 'border-accent/60 bg-accent-soft text-fg' : 'border-line text-dim hover:text-fg')}>
                  {e.label}
                </button>
              ))}
            </div>
            {effect && <div className="text-[12.5px] text-faint">{effect.hint}</div>}
            {effect?.colors && (
              <div className="flex flex-wrap items-center gap-2">
                {draft.colors.map((c, i) => (
                  <div key={i} className="relative">
                    <input type="color" value={c} onChange={(e) => setDraft({ ...draft, colors: draft.colors.map((x, j) => (j === i ? e.target.value : x)) })} className="h-9 w-11 cursor-pointer rounded-lg border border-line bg-transparent p-0.5" aria-label={`Colour ${i + 1}`} />
                    {draft.colors.length > 1 && (
                      <button type="button" onClick={() => setDraft({ ...draft, colors: draft.colors.filter((_, j) => j !== i) })} className="absolute -right-1.5 -top-1.5 grid h-4 w-4 place-items-center rounded-full bg-surface-3 text-[10px] text-dim hover:text-fg" aria-label={`Remove colour ${i + 1}`}>
                        ×
                      </button>
                    )}
                  </div>
                ))}
                {draft.colors.length < 8 && <IconButton icon={Plus} label="Add a colour" variant="subtle" onClick={() => setDraft({ ...draft, colors: [...draft.colors, draft.colors.at(-1) ?? '#ffffff'] })} />}
              </div>
            )}
            <div className="grid gap-4 sm:grid-cols-2">
              <Slider label="Speed" value={draft.speed} min={0.2} max={3} step={0.1} onChange={(v) => setDraft({ ...draft, speed: v })} format={(v) => `${v.toFixed(1)}×`} />
              <Slider label="Brightness" value={draft.brightness} min={0} max={100} step={1} onChange={(v) => setDraft({ ...draft, brightness: v })} format={(v) => `${v}%`} />
            </div>
            <Swatch p={draft} className="h-3" />
            <div className="flex justify-end gap-2">
              <Button variant="secondary" onClick={() => setDraft(null)}>
                Cancel
              </Button>
              <Button icon={Save} onClick={() => void save()}>
                Save and use
              </Button>
            </div>
          </div>
        </Card>
      )}

      {st && st.devices.length > 0 && (
        <Card>
          <CardHeader title="Devices" subtitle="Untick anything another program should keep lighting." className="px-5 pt-4" />
          <div className="divide-y divide-line">
            {st.devices.map((d) => {
              const peripheral = PERIPHERALS.includes(d.kind);
              const lit = !s.lighting.skip.includes(d.key) && (s.lighting.peripherals || !peripheral);
              const set = (v: boolean) => void rig.update({ lighting: { skip: v ? s.lighting.skip.filter((k) => k !== d.key) : [...s.lighting.skip, d.key], ...(v && peripheral ? { peripherals: true } : {}) } });
              return (
                <Row key={d.key} title={d.name} hint={`${KIND_LABEL[d.kind] ?? d.kind} · ${d.leds} LED${d.leds === 1 ? '' : 's'}${d.zones.length ? ` · ${d.zones.join(', ')}` : ''}`}>
                  <Switch checked={lit} onChange={set} label={`Light ${d.name}`} />
                </Row>
              );
            })}
            <Row title="Also keyboard, mouse and headset" hint="Off by default, so their own software keeps them.">
              <Switch checked={s.lighting.peripherals} onChange={(v) => void rig.update({ lighting: { peripherals: v } })} label="Also keyboard, mouse and headset" />
            </Row>
          </div>
        </Card>
      )}
    </div>
  );
}
