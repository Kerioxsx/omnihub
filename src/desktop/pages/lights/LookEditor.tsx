// What lights show: a colour, a white, an effect, the screen, the music, or
// a light's own scene or music mode, with each one's options.

import type { DeepPartial, GoveeLight, GoveeLook, GoveeMode, LightEffect, MonitorInfo, MusicStyle } from '@shared/types';
import type { LucideIcon } from 'lucide-react';
import { Clapperboard, Mic, MonitorPlay, Music2, Palette, Plus, Sparkles, Sun, X } from 'lucide-react';
import { useMemo, useState } from 'react';
import { Segmented, Select, Switch } from '../../components/ui/Form';
import { cx } from '../../lib/cx';
import { Slider } from '../rig/LightingTab';

const MODES: { value: GoveeMode; label: string; icon: LucideIcon; hint: string; cloud?: boolean }[] = [
  { value: 'color', label: 'Colour', icon: Palette, hint: 'One colour' },
  { value: 'white', label: 'White', icon: Sun, hint: 'Warm to cool' },
  { value: 'effect', label: 'Effect', icon: Sparkles, hint: 'Moving colours' },
  { value: 'screen', label: 'Screen', icon: MonitorPlay, hint: 'Follows your screen' },
  { value: 'music', label: 'Music', icon: Music2, hint: 'Album colours + beats' },
  { value: 'scene', label: 'Scene', icon: Clapperboard, hint: 'The light’s own', cloud: true },
  { value: 'deviceMusic', label: 'Light’s mic', icon: Mic, hint: 'Its own music mode', cloud: true },
];

const SWATCHES = ['#ff0000', '#ff5a00', '#ffb703', '#ffe500', '#7cff00', '#00ff6a', '#00e5ff', '#00a2ff', '#2b59ff', '#7c3aed', '#ff2bd6', '#ff4d6d', '#ffffff'];

const EFFECTS: { value: LightEffect; label: string }[] = [
  { value: 'wave', label: 'Rainbow wave' },
  { value: 'spectrum', label: 'Spectrum' },
  { value: 'gradient', label: 'Flow' },
  { value: 'breathing', label: 'Breathing' },
  { value: 'static', label: 'Still colours' },
];

function Colors({ colors, onChange, max = 6 }: { colors: string[]; onChange: (c: string[]) => void; max?: number }) {
  return (
    <div className="flex flex-wrap items-center gap-2">
      {colors.map((c, i) => (
        <span key={i} className="group relative">
          <input type="color" value={c} onChange={(e) => onChange(colors.map((x, j) => (j === i ? e.target.value : x)))} className="h-8 w-8 cursor-pointer rounded-lg border border-line bg-transparent p-0.5" aria-label={`Colour ${i + 1}`} />
          {colors.length > 1 && (
            <button type="button" onClick={() => onChange(colors.filter((_, j) => j !== i))} className="absolute -right-1.5 -top-1.5 hidden h-4 w-4 place-items-center rounded-full bg-surface-3 text-dim group-hover:grid" aria-label="Remove colour">
              <X size={10} />
            </button>
          )}
        </span>
      ))}
      {colors.length < max && (
        <button type="button" onClick={() => onChange([...colors, SWATCHES[(colors.length * 3) % SWATCHES.length]])} className="grid h-8 w-8 place-items-center rounded-lg border border-dashed border-line text-faint hover:text-fg" aria-label="Add a colour">
          <Plus size={14} />
        </button>
      )}
    </div>
  );
}

export function LookEditor({ look, onChange, lights, monitors, palette, compact }: { look: GoveeLook; onChange: (p: DeepPartial<GoveeLook>) => void; lights: GoveeLight[]; monitors: MonitorInfo[]; palette: string[]; compact?: boolean }) {
  const cloud = lights.some((l) => l.cloud);
  const [filter, setFilter] = useState('');
  const scenes = useMemo(() => {
    const names = new Set<string>();
    for (const l of lights) for (const s of [...l.scenes, ...l.diyScenes, ...l.snapshots]) names.add(s.name);
    return [...names].sort((a, b) => a.localeCompare(b));
  }, [lights]);
  const musicModes = useMemo(() => [...new Set(lights.flatMap((l) => l.musicModes.map((m) => m.name)))], [lights]);
  const shown = scenes.filter((s) => s.toLowerCase().includes(filter.toLowerCase()));

  return (
    <div className="space-y-4">
      <div role="radiogroup" aria-label="What the lights show" className={cx('grid gap-2', compact ? 'grid-cols-4' : 'grid-cols-4 xl:grid-cols-7')}>
        {MODES.map((m) => {
          const active = look.mode === m.value;
          const Icon = m.icon;
          const off = m.cloud && !cloud;
          return (
            <button
              key={m.value}
              type="button"
              role="radio"
              aria-checked={active}
              disabled={off}
              title={off ? 'Needs your Govee API key (Setup)' : m.hint}
              onClick={() => onChange({ mode: m.value })}
              className={cx('flex flex-col items-center gap-1 rounded-xl border px-2 py-2.5 text-center transition-colors disabled:cursor-not-allowed disabled:opacity-40', active ? 'border-accent/60 bg-accent-soft text-fg' : 'border-line bg-surface-2/50 text-dim hover:border-line-strong hover:text-fg')}
            >
              <Icon size={17} className={active ? 'text-accent' : ''} />
              <span className="text-[12.5px] font-semibold">{m.label}</span>
              {!compact && <span className="text-[11px] leading-tight text-faint">{m.hint}</span>}
            </button>
          );
        })}
      </div>

      {look.mode === 'color' && (
        <div className="flex flex-wrap items-center gap-2">
          {SWATCHES.map((c) => (
            <button key={c} type="button" onClick={() => onChange({ color: c })} className={cx('h-8 w-8 rounded-full ring-offset-2 ring-offset-surface transition-transform hover:scale-110', look.color.toLowerCase() === c ? 'ring-2 ring-accent' : 'ring-1 ring-white/15')} style={{ backgroundColor: c }} aria-label={`Colour ${c}`} />
          ))}
          <input type="color" value={look.color} onChange={(e) => onChange({ color: e.target.value })} className="h-8 w-12 cursor-pointer rounded-lg border border-line bg-transparent p-0.5" aria-label="Any colour" />
        </div>
      )}

      {look.mode === 'white' && (
        <div className="space-y-2">
          <Slider label="Colour temperature" value={look.kelvin} min={2000} max={9000} step={100} onChange={(v) => onChange({ kelvin: v })} format={(v) => `${v} K${v <= 3000 ? ' · warm' : v >= 6000 ? ' · cool' : ''}`} />
          <div className="h-2 rounded-full" style={{ background: 'linear-gradient(90deg,#ff8a2b,#ffd6a0,#fff4e6,#e6f0ff,#b7d2ff)' }} />
        </div>
      )}

      {look.mode === 'effect' && (
        <div className="space-y-3">
          <Segmented label="Effect" size="sm" value={look.effect} onChange={(v) => onChange({ effect: v })} options={EFFECTS} />
          {look.effect !== 'wave' && look.effect !== 'spectrum' && <Colors colors={look.colors} onChange={(colors) => onChange({ colors })} />}
          <Slider label="Speed" value={look.speed} min={0.2} max={3} step={0.1} onChange={(v) => onChange({ speed: v })} format={(v) => `${v.toFixed(1)}×`} />
        </div>
      )}

      {look.mode === 'screen' && (
        <div className="space-y-3">
          {monitors.length > 1 && (
            <label className="flex items-center justify-between gap-3 text-[12.5px] font-medium text-fg">
              Monitor
              <Select value={look.monitor} onChange={(e) => onChange({ monitor: e.target.value })} className="w-56">
                <option value="">The main one</option>
                {monitors.map((m) => (
                  <option key={m.name} value={m.name}>
                    {m.name} · {m.width}×{m.height}
                  </option>
                ))}
              </Select>
            </label>
          )}
          <Slider label="Smoothness" value={look.smoothness} min={0} max={100} step={1} onChange={(v) => onChange({ smoothness: v })} format={(v) => (v < 20 ? 'Instant (games)' : v > 70 ? 'Very smooth (films)' : 'Smooth')} />
          <Slider label="Vividness" value={look.saturation} min={50} max={200} step={5} onChange={(v) => onChange({ saturation: v })} format={(v) => `${v}%`} />
          <label className="flex items-center justify-between gap-3 text-[12.5px] font-medium text-fg">
            <span>
              Ignore black bars and dark menus
              <span className="block text-[11.5px] font-normal text-faint">Films in letterbox keep their colour around the screen.</span>
            </span>
            <Switch checked={look.skipDark} onChange={(v) => onChange({ skipDark: v })} label="Ignore black bars" />
          </label>
          <p className="text-[11.5px] text-faint">Each light’s place around the screen (left, top, all around…) is under Lights. The screen is shrunk on the graphics card and never saved.</p>
        </div>
      )}

      {look.mode === 'music' && (
        <div className="space-y-3">
          <Segmented<MusicStyle>
            label="Music style"
            size="sm"
            value={look.style}
            onChange={(v) => onChange({ style: v })}
            options={[
              { value: 'pulse', label: 'Pulse' },
              { value: 'spectrum', label: 'Spectrum' },
              { value: 'flow', label: 'Flow' },
            ]}
          />
          <p className="text-[11.5px] text-faint">{look.style === 'pulse' ? 'The light pulses on every beat, a new colour each time.' : look.style === 'spectrum' ? 'Each segment a range of the sound, bass first.' : 'The colours flow at the song’s tempo; beats flash from the middle.'}</p>
          <label className="flex items-center justify-between gap-3 text-[12.5px] font-medium text-fg">
            <span>
              The album cover’s colours
              <span className="block text-[11.5px] font-normal text-faint">Taken from the song playing, any app.</span>
            </span>
            <Switch checked={look.albumColors} onChange={(v) => onChange({ albumColors: v })} label="Album colours" />
          </label>
          {look.albumColors ? (
            palette.length > 0 && (
              <div className="flex items-center gap-1.5">
                {palette.map((c) => (
                  <span key={c} className="h-5 w-5 rounded-full ring-1 ring-white/15" style={{ backgroundColor: c }} title={c} />
                ))}
                <span className="ml-1 text-[11.5px] text-faint">from the cover now</span>
              </div>
            )
          ) : (
            <Colors colors={look.colors} onChange={(colors) => onChange({ colors })} />
          )}
          <Slider label="Sensitivity" value={look.sensitivity} min={0} max={100} step={1} onChange={(v) => onChange({ sensitivity: v })} format={(v) => `${v}%`} />
        </div>
      )}

      {look.mode === 'scene' && (
        <div className="space-y-2">
          <input value={filter} onChange={(e) => setFilter(e.target.value)} placeholder="Find a scene…" className="h-9 w-full rounded-lg border border-line bg-surface px-3 text-[13px] text-fg outline-none focus:border-accent/60" />
          <div className="flex max-h-48 flex-wrap gap-1.5 overflow-y-auto">
            {shown.length === 0 && <span className="text-[12px] text-faint">{scenes.length ? 'No scene by that name.' : 'Your lights’ scenes appear here once Govee lists them.'}</span>}
            {shown.map((s) => (
              <button key={s} type="button" onClick={() => onChange({ scene: s })} className={cx('rounded-full border px-3 py-1 text-[12.5px] transition-colors', look.scene === s ? 'border-accent/60 bg-accent-soft text-fg' : 'border-line text-dim hover:text-fg')}>
                {s}
              </button>
            ))}
          </div>
        </div>
      )}

      {look.mode === 'deviceMusic' && (
        <div className="space-y-3">
          <div className="flex flex-wrap gap-1.5">
            {musicModes.map((m) => (
              <button key={m} type="button" onClick={() => onChange({ deviceMusic: m })} className={cx('rounded-full border px-3 py-1 text-[12.5px]', (look.deviceMusic || musicModes[0]) === m ? 'border-accent/60 bg-accent-soft text-fg' : 'border-line text-dim hover:text-fg')}>
                {m}
              </button>
            ))}
          </div>
          <Slider label="Microphone sensitivity" value={look.sensitivity} min={0} max={100} step={1} onChange={(v) => onChange({ sensitivity: v })} format={(v) => `${v}%`} />
          <p className="text-[11.5px] text-faint">The light listens with its own microphone (for music not played by this PC).</p>
        </div>
      )}
    </div>
  );
}
