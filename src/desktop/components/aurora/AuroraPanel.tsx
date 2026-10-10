// Aurora's settings: a compact glass panel at the side of the player, laid
// out like the reference's: the light first (gradient, animation, thickness,
// glow, colours), then the visual, the lyrics and the rest. Every change
// shows at once and is saved (lib/aurora/settings.ts).

import type { AmbientDisplay, ColorMode, Effects, LyricFont, SceneStyle, VisualSettings, VisualStatus } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { ChevronDown, FolderOpen, RotateCcw, ShieldCheck, X } from 'lucide-react';
import { type ReactNode, useEffect, useId, useState } from 'react';
import { api } from '../../api';
import { DEFAULT_EFFECTS } from '../../lib/aurora/defaults';
import { patchVisuals, resetVisuals, useVisuals } from '../../lib/aurora/settings';
import { cx } from '../../lib/cx';
import { useSettings } from '../../state/settings';
import { FONTS } from './AuroraLyrics';

// ---------- small glass controls ----------

function Toggle({ label, hint, checked, onChange, disabled }: { label: string; hint?: ReactNode; checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <div className={cx('flex items-start justify-between gap-3 py-1.5', disabled && 'opacity-45')}>
      <span className="min-w-0">
        <span className="block text-[12.5px] font-medium text-white/90">{label}</span>
        {hint && <span className="mt-0.5 block text-[11px] leading-snug text-white/45">{hint}</span>}
      </span>
      <MiniSwitch label={label} checked={checked} onChange={onChange} disabled={disabled} />
    </div>
  );
}

function MiniSwitch({ label, checked, onChange, disabled }: { label: string; checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cx('relative mt-0.5 h-[20px] w-[34px] shrink-0 rounded-full transition-colors focus-visible:outline focus-visible:outline-2 focus-visible:outline-white/70', checked ? 'bg-[#3b82f6]' : 'bg-white/15')}
    >
      <span className={cx('absolute top-[3px] h-[14px] w-[14px] rounded-full bg-white shadow transition-all', checked ? 'left-[17px]' : 'left-[3px]')} />
    </button>
  );
}

function Slider({ label, value, min, max, step = 0.01, onChange, format, disabled }: { label: string; value: number; min: number; max: number; step?: number; onChange: (v: number) => void; format?: (v: number) => string; disabled?: boolean }) {
  const id = useId();
  const pct = ((value - min) / (max - min)) * 100;
  return (
    <div className={cx('py-1.5', disabled && 'opacity-45')}>
      <div className="mb-1 flex items-baseline justify-between text-[12px]">
        <label htmlFor={id} className="font-medium text-white/85">
          {label}
        </label>
        <span className="font-mono text-[11px] tabular-nums text-white/50">{format ? format(value) : value.toFixed(2)}</span>
      </div>
      <input
        id={id}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(Number(e.target.value))}
        className="h-1 w-full cursor-pointer appearance-none rounded-full bg-white/15 [&::-webkit-slider-thumb]:h-3.5 [&::-webkit-slider-thumb]:w-3.5 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-full [&::-webkit-slider-thumb]:bg-white [&::-webkit-slider-thumb]:shadow"
        style={{ background: `linear-gradient(90deg, #3b82f6 ${pct}%, rgba(255,255,255,.15) 0)` }}
      />
    </div>
  );
}

/** A labelled dropdown, as in the reference's panel. */
function Dropdown<T extends string | number>({ label, value, options, onChange }: { label: string; value: T; options: { value: T; label: string }[]; onChange: (v: T) => void }) {
  const id = useId();
  return (
    <div className="flex items-center justify-between gap-3 py-1.5">
      <label htmlFor={id} className="text-[12.5px] font-medium text-white/90">
        {label}
      </label>
      <span className="relative">
        <select
          id={id}
          value={String(value)}
          onChange={(e) => {
            const o = options.find((x) => String(x.value) === e.target.value);
            if (o) onChange(o.value);
          }}
          className="h-7 cursor-pointer appearance-none rounded-[8px] border border-white/10 bg-white/[0.08] py-0 pl-2.5 pr-7 text-[12px] font-medium text-white outline-none hover:bg-white/[0.12] focus-visible:border-white/40 [&>option]:bg-[#16161f]"
        >
          {options.map((o) => (
            <option key={String(o.value)} value={String(o.value)}>
              {o.label}
            </option>
          ))}
        </select>
        <ChevronDown size={13} className="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-white/60" />
      </span>
    </div>
  );
}

function Swatch({ label, value, onChange, big }: { label: string; value: string; onChange: (v: string) => void; big?: boolean }) {
  return (
    <label
      className={cx('relative block shrink-0 cursor-pointer overflow-hidden rounded-full border border-white/25 shadow-inner', big ? 'h-12 w-12' : 'h-8 w-8')}
      title={label}
      style={{ background: big ? `conic-gradient(from 0deg, ${value}, ${value})` : value }}
    >
      <input type="color" aria-label={label} value={value} onChange={(e) => onChange(e.target.value)} className="absolute inset-0 h-full w-full cursor-pointer opacity-0" />
    </label>
  );
}

function Section({ title, children, right }: { title: string; children: ReactNode; right?: ReactNode }) {
  return (
    <section className="border-t border-white/[0.08] px-4 py-3 first:border-t-0">
      <div className="mb-1 flex items-center justify-between">
        <h3 className="text-[11px] font-bold uppercase tracking-[0.08em] text-white/45">{title}</h3>
        {right}
      </div>
      {children}
    </section>
  );
}

const pct = (v: number) => `${Math.round(v * 100)}%`;
const times = (v: number) => `×${v.toFixed(2)}`;
const px = (v: number) => `${Math.round(v * 10) / 10} px`;

// ---------- sections ----------

/** The reference's main block: the light around the screen and its colours. */
function LightSection({ v }: { v: VisualSettings }) {
  const c = v.color;
  const g = v.glow;
  const modes: { value: ColorMode; label: string }[] = [
    { value: 'album', label: 'Album colours' },
    { value: 'duo', label: '2 colours' },
    { value: 'multi', label: 'Multicolour' },
    { value: 'single', label: '1 colour' },
  ];
  const override = c.mode !== 'album';
  return (
    <Section title="Light" right={<MiniSwitch label="Edge light" checked={g.enabled} onChange={(enabled) => patchVisuals({ glow: { enabled } })} />}>
      <Dropdown label="Gradient" value={c.mode} options={modes} onChange={(mode) => patchVisuals({ color: { mode } })} />
      <Dropdown
        label="Animation"
        value={g.animation}
        options={[
          { value: 'music', label: 'Music sync' },
          { value: 'idle', label: 'Idle' },
          { value: 'none', label: 'No animation' },
        ]}
        onChange={(animation) => patchVisuals({ glow: { animation } })}
      />
      <Slider label="Thickness" value={g.thickness} min={0.5} max={10} step={0.5} onChange={(thickness) => patchVisuals({ glow: { thickness } })} format={px} />
      <Slider label="Glow" value={g.glow} min={0} max={1.2} onChange={(glow) => patchVisuals({ glow: { glow } })} format={pct} />
      <Toggle label="Override album colours" hint="Your own colours for everything, still following the music." checked={override} onChange={(on) => patchVisuals({ color: { mode: on ? 'duo' : 'album' } })} />
      {(c.mode === 'duo' || c.mode === 'single' || c.mode === 'album') && (
        <div className={cx('flex items-center gap-4 py-2', c.mode === 'album' && 'opacity-45')}>
          <div className="flex flex-col items-center gap-1">
            <Swatch big label="Primary colour" value={c.primary} onChange={(primary) => patchVisuals({ color: { primary, mode: c.mode === 'album' ? 'duo' : c.mode } })} />
            <span className="text-[10.5px] text-white/50">Primary</span>
          </div>
          {c.mode !== 'single' && (
            <div className="flex flex-col items-center gap-1">
              <Swatch big label="Secondary colour" value={c.secondary} onChange={(secondary) => patchVisuals({ color: { secondary, mode: c.mode === 'album' ? 'duo' : c.mode } })} />
              <span className="text-[10.5px] text-white/50">Secondary</span>
            </div>
          )}
          <div className="h-8 flex-1 rounded-full border border-white/10" style={{ background: `linear-gradient(90deg, ${c.primary}, ${c.mode === 'single' ? c.primary : c.secondary})` }} aria-hidden />
        </div>
      )}
      {c.mode === 'multi' && (
        <div className="flex items-center gap-2 py-1.5">
          {c.colors.map((col, i) => (
            <Swatch key={i} label={`Colour ${i + 1}`} value={col} onChange={(x) => patchVisuals({ color: { colors: c.colors.map((o, k) => (k === i ? x : o)) } })} />
          ))}
          <div className="h-8 flex-1 rounded-full border border-white/10" style={{ background: `linear-gradient(90deg, ${c.colors.join(', ')})` }} aria-hidden />
        </div>
      )}
      {c.mode === 'album' && <p className="pb-1 text-[11px] leading-snug text-white/45">Colours come from each song's cover and blend smoothly into the next song's.</p>}
      <Slider label="Vividness" value={c.vividness} min={0} max={1.5} onChange={(vividness) => patchVisuals({ color: { vividness } })} format={pct} />
    </Section>
  );
}

function VisualSection({ v }: { v: VisualSettings }) {
  const s = v.scene;
  const set = (p: Partial<VisualSettings['scene']>) => patchVisuals({ scene: p });
  const styles: { value: SceneStyle; label: string }[] = [
    { value: 'visual', label: 'Visual' },
    { value: 'fisheye', label: 'Fisheye' },
    { value: 'fisheyeVisual', label: 'Fisheye Visual' },
    { value: 'minimal', label: 'Minimal' },
    { value: 'ambient', label: 'Ambient' },
  ];
  return (
    <Section title="Visual" right={<MiniSwitch label="Visual" checked={s.enabled} onChange={(enabled) => set({ enabled })} />}>
      <div className={cx(!s.enabled && 'pointer-events-none opacity-45')}>
        <Dropdown label="Style" value={s.style} options={styles} onChange={(style) => set({ style })} />
        <p className="pb-1 text-[11px] leading-snug text-white/45">
          {s.style === 'minimal' ? 'Mostly dark: the cover small and dim.' : s.style === 'ambient' ? 'Slow colour fields from the palette.' : 'The cover art, pulsing and glitching with the music.'}
        </p>
        <Slider label="Intensity" value={s.intensity} min={0} max={1.5} onChange={(intensity) => set({ intensity })} format={pct} />
        <Slider label="Speed" value={s.speed} min={0.25} max={2.5} onChange={(speed) => set({ speed })} format={times} />
        <Slider label="Softness" value={s.blur} min={0} max={1} onChange={(blur) => set({ blur })} format={pct} />
        <Slider label="Saturation" value={s.saturation} min={0} max={1.6} onChange={(saturation) => set({ saturation })} format={pct} />
        <Slider label="Brightness" value={s.opacity} min={0.1} max={1} onChange={(opacity) => set({ opacity })} format={pct} />
        <Toggle label="Use the cover art" hint="Off: colour fields only." checked={s.artwork} onChange={(artwork) => set({ artwork })} />
        <Toggle label="Spectrum line" checked={s.waveform} onChange={(waveform) => set({ waveform })} />
      </div>
    </Section>
  );
}

function LyricsSection({ v }: { v: VisualSettings }) {
  const l = v.lyrics;
  const set = (p: Partial<VisualSettings['lyrics']>) => patchVisuals({ lyrics: p });
  const stack = l.layout === 'stack';
  return (
    <Section title="Lyrics" right={<MiniSwitch label="Lyrics" checked={l.visible} onChange={(visible) => set({ visible })} />}>
      <div className={cx(!l.visible && 'pointer-events-none opacity-45')}>
        <Dropdown
          label="Layout"
          value={l.layout}
          options={[
            { value: 'stack', label: 'One at a time' },
            { value: 'lines', label: 'Lines' },
          ]}
          onChange={(layout) => set({ layout })}
        />
        <Dropdown label="Font" value={l.font} options={(Object.keys(FONTS) as LyricFont[]).map((k) => ({ value: k, label: FONTS[k].label }))} onChange={(font) => set({ font })} />
        <Slider label="Size" value={l.size} min={0.5} max={1.8} onChange={(size) => set({ size })} format={pct} />
        <Dropdown
          label="Weight"
          value={l.weight}
          options={[
            { value: 500, label: 'Medium' },
            { value: 700, label: 'Bold' },
            { value: 800, label: 'Heavy' },
            { value: 900, label: 'Black' },
          ]}
          onChange={(weight) => set({ weight })}
        />
        <Dropdown
          label="Highlight"
          value={l.timing === 'line' || !l.wordHighlight ? 'line' : 'word'}
          options={[
            { value: 'word', label: 'Word by word' },
            { value: 'line', label: 'Whole line' },
          ]}
          onChange={(x) => set(x === 'word' ? { timing: 'auto', wordHighlight: true } : { timing: 'line', wordHighlight: false })}
        />
        <Toggle
          label="Estimate words when lyrics only time lines"
          hint="Most lyrics online time each line, not each word. This spreads a line's words over it by their syllables, so the highlight still moves word by word — close, not exact. Off: the whole line lights up."
          checked={l.estimateWords}
          disabled={l.timing === 'line' || !l.wordHighlight}
          onChange={(estimateWords) => set({ estimateWords })}
        />
        <Dropdown
          label="Emphasis"
          value={l.emphasis}
          options={[
            { value: 'glow', label: 'Glow' },
            { value: 'box', label: 'Colour box' },
            { value: 'color', label: 'Colour' },
          ]}
          onChange={(emphasis) => set({ emphasis })}
        />
        <div className="flex items-center justify-between py-1.5">
          <span className="text-[12.5px] font-medium text-white/90">Highlight colour</span>
          <span className="flex items-center gap-2">
            <button type="button" onClick={() => set({ highlightColor: null })} className={cx('h-7 rounded-full px-2.5 text-[11px] font-semibold', l.highlightColor == null ? 'bg-white text-black' : 'bg-white/10 text-white/70 hover:text-white')}>
              From the music
            </button>
            <Swatch label="Highlight colour" value={l.highlightColor ?? '#ffffff'} onChange={(highlightColor) => set({ highlightColor })} />
          </span>
        </div>
        <Slider label="Glow" value={l.glow} min={0} max={1.5} onChange={(glow) => set({ glow })} format={pct} />
        <Dropdown
          label="Shown"
          value={l.lines}
          options={
            stack
              ? [
                  { value: 1, label: 'Only what is sung' },
                  { value: 2, label: '+ what comes next' },
                  { value: 3, label: '+ before and next' },
                ]
              : [
                  { value: 1, label: 'Current line' },
                  { value: 2, label: '+ next line' },
                  { value: 3, label: '+ next two' },
                ]
          }
          onChange={(lines) => set({ lines })}
        />
        <Dropdown
          label="Position"
          value={l.place}
          options={[
            { value: 'upper', label: 'Upper' },
            { value: 'center', label: 'Centre' },
            { value: 'lower', label: 'Lower' },
          ]}
          onChange={(place) => set({ place })}
        />
        <Slider label="Move sideways" value={l.offsetX} min={-30} max={30} step={1} onChange={(offsetX) => set({ offsetX })} format={(x) => `${x > 0 ? '+' : ''}${Math.round(x)}%`} />
        <Slider label="Move up or down" value={l.offsetY} min={-25} max={25} step={1} onChange={(offsetY) => set({ offsetY })} format={(x) => `${x > 0 ? '+' : ''}${Math.round(x)}%`} />
        <Slider label="Backing" value={l.backing} min={0} max={0.9} onChange={(backing) => set({ backing })} format={pct} />
        <Slider label="Transition speed" value={l.transition} min={0.25} max={2.5} onChange={(transition) => set({ transition })} format={times} />
        <Slider label="Movement" value={l.motion} min={0} max={1.5} onChange={(motion) => set({ motion })} format={(x) => (x < 0.005 ? 'still' : pct(x))} />
        <p className="pb-1 text-[11px] leading-snug text-white/45">How much the lyrics bounce on beats, sway with the music and pop in. Reduced motion keeps them still.</p>
        <Toggle label="Show by themselves" hint="Hiding lyrics with L lasts for this song; the next song with lyrics shows them again." checked={l.autoShow} onChange={(autoShow) => set({ autoShow })} />
        <p className="pt-1 text-[11px] text-white/40">Shortcuts: L shows or hides the lyrics, V switches views.</p>
      </div>
    </Section>
  );
}

function SoundSection({ v, status }: { v: VisualSettings; status: VisualStatus | null }) {
  const listening = v.audioReactive && v.enabled;
  const line = !listening
    ? 'Not listening. The light moves on its own.'
    : status?.state === 'listening'
      ? `Listening to ${status.source ?? 'the PC'}`
      : status?.state === 'unavailable'
        ? `Can't hear the PC: ${status.error ?? 'no sound device'}. The light moves on its own.`
        : status?.state === 'starting'
          ? 'Starting…'
          : 'Listens while Aurora is on screen.';
  return (
    <Section title="Sound">
      <Toggle label="Follow the music" hint="Bass widens the glow and swells the cover, beats lift them, highs shimmer." checked={v.audioReactive} onChange={(on) => patchVisuals({ audioReactive: on })} />
      <div className="mb-1 flex items-start gap-2 rounded-[10px] bg-white/[0.06] px-2.5 py-2 text-[11px] leading-snug text-white/60" data-capture-status>
        <span className={cx('mt-1 h-1.5 w-1.5 shrink-0 rounded-full', listening && status?.state === 'listening' ? 'bg-emerald-400' : listening && status?.state === 'unavailable' ? 'bg-amber-400' : 'bg-white/30')} />
        <span>{line}</span>
      </div>
      <div className="flex gap-2 rounded-[10px] px-0.5 py-1 text-[10.5px] leading-snug text-white/40">
        <ShieldCheck size={13} className="mt-px shrink-0" />
        <span>Aurora reads the sound Windows is already playing (the mix you hear), on this PC only. It never uses the microphone, records or sends sound anywhere, and stops when Aurora is off screen.</span>
      </div>
      <Slider label="Sensitivity" value={v.sensitivity} min={0.2} max={3} onChange={(x) => patchVisuals({ sensitivity: x })} format={times} disabled={!v.audioReactive} />
      <Slider label="Bass response" value={v.bass} min={0} max={2} onChange={(x) => patchVisuals({ bass: x })} format={times} disabled={!v.audioReactive} />
      <Slider label="Smoothing" value={v.smoothing} min={0} max={1} onChange={(x) => patchVisuals({ smoothing: x })} format={pct} disabled={!v.audioReactive} />
    </Section>
  );
}

function ComfortSection({ v }: { v: VisualSettings }) {
  return (
    <Section title="Motion & comfort">
      <Toggle label="Reduced motion" hint="No glitching, pulsing or shimmer; slow colour changes; lyrics fade instead of moving." checked={v.reducedMotion} onChange={(on) => patchVisuals({ reducedMotion: on })} />
      <Toggle label="No flashes" hint="Beats never brighten the light suddenly." checked={v.noFlashes} onChange={(on) => patchVisuals({ noFlashes: on })} />
      <Slider label="Animation speed" value={v.speed} min={0.25} max={2.5} onChange={(x) => patchVisuals({ speed: x })} format={times} />
      <Toggle label="Keep the screen on" hint="While the player shows Aurora and music plays." checked={v.keepAwake} onChange={(keepAwake) => patchVisuals({ keepAwake })} />
    </Section>
  );
}

function EdgeSection({ v }: { v: VisualSettings }) {
  const g = v.glow;
  const set = (p: Partial<VisualSettings['glow']>) => patchVisuals({ glow: p });
  return (
    <Section title="Edge light layers">
      <div className={cx(!g.enabled && 'pointer-events-none opacity-45')}>
        <Slider label="Brightness" value={g.intensity} min={0} max={1.5} onChange={(intensity) => set({ intensity })} format={pct} />
        <Slider label="Edge" value={g.edge} min={0} max={1} onChange={(edge) => set({ edge })} format={pct} />
        <Slider label="Inner highlight" value={g.highlight} min={0} max={1} onChange={(highlight) => set({ highlight })} format={pct} />
        <Slider label="Glow reach" value={g.glowSize} min={2} max={80} step={1} onChange={(glowSize) => set({ glowSize })} format={px} />
        <Slider label="Bloom" value={g.bloom} min={0} max={1} onChange={(bloom) => set({ bloom })} format={pct} />
        <Slider label="Bloom reach" value={g.bloomSize} min={10} max={300} step={2} onChange={(bloomSize) => set({ bloomSize })} format={px} />
        <Slider label="Corner rounding" value={g.radius} min={0} max={40} step={1} onChange={(radius) => set({ radius })} format={px} />
      </div>
    </Section>
  );
}

function DesktopSection({ v }: { v: VisualSettings }) {
  const d = v.desktop;
  const set = (p: Partial<VisualSettings['desktop']>) => patchVisuals({ desktop: p });
  const [displays, setDisplays] = useState<AmbientDisplay[]>([]);
  useEffect(() => {
    void api.ambient.displays().then(setDisplays, () => setDisplays([]));
  }, []);
  return (
    <Section title="Around your screen">
      <Toggle label="Glow around the screen" hint="The edge light around your monitor, over every app, while music plays. Clicks go straight through it." checked={d.enabled} onChange={(enabled) => set({ enabled })} />
      <div className={cx(!d.enabled && 'pointer-events-none opacity-45')}>
        <Toggle label="Lyrics on the desktop" hint="Floating lyrics over your apps." checked={d.lyrics} onChange={(lyrics) => set({ lyrics })} />
        <Dropdown
          label="Display"
          value={d.display}
          options={[{ value: 'primary', label: 'Main display' }, { value: 'all', label: 'Every display' }, ...displays.map((m) => ({ value: m.name, label: m.label }))]}
          onChange={(display) => set({ display })}
        />
        <Toggle label="Stay above the taskbar" checked={d.clearTaskbar} onChange={(clearTaskbar) => set({ clearTaskbar })} />
        <Toggle label="Step aside for full-screen apps and games" hint="Hides while a game, video or presentation fills the screen, and during game boosts." checked={d.hideFullscreen} onChange={(hideFullscreen) => set({ hideFullscreen })} />
        <Toggle label="Turn on when OmniHub starts" checked={d.startWithApp} onChange={(startWithApp) => set({ startWithApp })} />
      </div>
    </Section>
  );
}

const EFFECTS: { key: keyof Effects; label: string }[] = [
  { key: 'zoom', label: 'Beat zoom' },
  { key: 'bloom', label: 'Glow on bright parts' },
  { key: 'echo', label: 'Echo behind' },
  { key: 'ripple', label: 'Ripples' },
  { key: 'twist', label: 'Twist' },
  { key: 'shake', label: 'Shake' },
  { key: 'glitch', label: 'Glitch' },
  { key: 'split', label: 'Colour split' },
  { key: 'pixelate', label: 'Pixelate on beats' },
  { key: 'fisheye', label: 'Fisheye' },
  { key: 'kaleidoscope', label: 'Kaleidoscope' },
  { key: 'halftone', label: 'Halftone dots' },
  { key: 'scanlines', label: 'Scanlines' },
  { key: 'duotone', label: 'Duotone (palette colours)' },
];

const CALM: Effects = { zoom: 0.3, glitch: 0, split: 0, fisheye: 0, ripple: 0, kaleidoscope: 0, halftone: 0, pixelate: 0, shake: 0, echo: 0.25, scanlines: 0, duotone: 0, twist: 0, bloom: 0.25 };
const WILD: Effects = { zoom: 1, glitch: 0.7, split: 0.8, fisheye: 0.3, ripple: 0.6, kaleidoscope: 0, halftone: 0.25, pixelate: 0.45, shake: 0.5, echo: 0.7, scanlines: 0.3, duotone: 0, twist: 0.45, bloom: 0.75 };

/** What happens to the cover: each effect with its own strength. */
function EffectsSection({ v }: { v: VisualSettings }) {
  const fx = { ...DEFAULT_EFFECTS, ...v.scene.effects };
  const set = (effects: Partial<Effects>) => patchVisuals({ scene: { effects } });
  const shuffle = () => {
    const r = () => Math.round(Math.random() * 100) / 100;
    // Lively but not chaos: a few strong effects, the rest light.
    const picks = EFFECTS.map((e) => e.key).sort(() => Math.random() - 0.5);
    const next = Object.fromEntries(picks.map((k, i) => [k, i < 4 ? 0.5 + r() * 0.5 : i < 8 ? r() * 0.3 : 0])) as unknown as Effects;
    set({ ...next, zoom: Math.max(0.3, next.zoom) });
  };
  const presets: [string, () => void][] = [
    ['Calm', () => set(CALM)],
    ['Default', () => set(DEFAULT_EFFECTS)],
    ['Wild', () => set(WILD)],
    ['Shuffle', shuffle],
  ];
  return (
    <Section title="Effects">
      <div className={cx(!v.scene.enabled && 'pointer-events-none opacity-45')}>
        <div className="mb-1 flex gap-1" role="group" aria-label="Effect presets">
          {presets.map(([label, run]) => (
            <button key={label} type="button" onClick={run} className="h-7 flex-1 rounded-[8px] bg-white/[0.08] text-[11.5px] font-semibold text-white/80 hover:bg-white/15">
              {label}
            </button>
          ))}
        </div>
        {EFFECTS.map(({ key, label }) => (
          <Slider key={key} label={label} value={fx[key]} min={0} max={1} onChange={(x) => set({ [key]: x })} format={(x) => (x < 0.005 ? 'off' : pct(x))} />
        ))}
        <p className="pt-1 text-[11px] leading-snug text-white/40">Effects follow the beat and the bass. Reduced motion turns the moving ones off.</p>
      </div>
    </Section>
  );
}

function SourcesSection() {
  const media = useSettings((s) => s.settings?.media);
  const update = useSettings((s) => s.update);
  if (!media) return null;
  return (
    <Section title="Lyrics and covers">
      <div className="py-1.5">
        <div className="text-[12.5px] font-medium text-white/90">Your .lrc files</div>
        <div className="mt-0.5 text-[11px] leading-snug text-white/45">Checked first. Matched by the file's tags or its name (“Artist - Title.lrc”), and only when its length fits the song.</div>
        <div className="mt-1.5 flex items-center gap-2">
          <div className="min-w-0 flex-1 truncate rounded-[9px] bg-white/[0.07] px-2.5 py-1.5 font-mono text-[11px] text-white/70" title={media.lrcFolder ?? undefined}>
            {media.lrcFolder ?? 'No folder chosen'}
          </div>
          <button
            type="button"
            onClick={() =>
              void api.app.pickFolder('Folder with your .lrc lyrics files').then((p) => {
                if (p) void update({ media: { lrcFolder: p } });
              })
            }
            className="grid h-8 w-8 shrink-0 place-items-center rounded-[9px] bg-white/10 text-white/80 hover:bg-white/20"
            aria-label="Choose a folder"
            title="Choose a folder"
          >
            <FolderOpen size={14} />
          </button>
          {media.lrcFolder && (
            <button
              type="button"
              onClick={() => void update({ media: { lrcFolder: null } })}
              className="grid h-8 w-8 shrink-0 place-items-center rounded-[9px] bg-white/10 text-white/80 hover:bg-white/20"
              aria-label="Stop using the folder"
              title="Stop using the folder"
            >
              <X size={14} />
            </button>
          )}
        </div>
      </div>
      <Toggle
        label="Look lyrics up online"
        hint="From LRCLIB, a free, open lyrics library. Only the song's title, artist, album and length are sent."
        checked={media.lyricsOnline}
        onChange={(lyricsOnline) => void update({ media: { lyricsOnline } })}
      />
      <Toggle
        label="Sharper covers"
        hint="Players give Windows a small cover. This finds the same cover at 1200×1200 in Apple's iTunes catalogue (only the title, artist and album are sent) and uses it when it matches."
        checked={media.hiresArt}
        onChange={(hiresArt) => void update({ media: { hiresArt } })}
      />
    </Section>
  );
}

// ---------- the panel ----------

export function AuroraPanel({ open, onClose, status, full }: { open: boolean; onClose: () => void; status: VisualStatus | null; full: boolean }) {
  const v = useVisuals();
  const [confirm, setConfirm] = useState(false);
  useEffect(() => {
    if (!confirm) return;
    const t = setTimeout(() => setConfirm(false), 3500);
    return () => clearTimeout(t);
  }, [confirm]);
  return (
    <AnimatePresence>
      {open && (
        <motion.aside
          initial={{ opacity: 0, x: 24, scale: 0.98 }}
          animate={{ opacity: 1, x: 0, scale: 1 }}
          exit={{ opacity: 0, x: 24, scale: 0.98 }}
          transition={{ type: 'spring', stiffness: 380, damping: 34 }}
          className={cx(
            'absolute right-3 z-40 flex w-[320px] max-w-[calc(100%-24px)] flex-col overflow-hidden rounded-[18px] border border-white/[0.12] bg-[#121218]/60 text-white shadow-[0_30px_80px_-20px_rgba(0,0,0,.85),inset_0_1px_0_rgba(255,255,255,.08)] backdrop-blur-2xl backdrop-saturate-150',
            full ? 'bottom-24 top-16' : 'bottom-20 top-14',
          )}
          aria-label="Aurora settings"
          data-aurora-panel
        >
          <div className="flex items-center gap-3 border-b border-white/[0.08] px-4 py-3">
            <div className="min-w-0 flex-1">
              <div className="text-[14px] font-bold">Aurora</div>
              <div className="text-[11px] text-white/45">Light and lyrics that follow the music</div>
            </div>
            <MiniSwitch label="Aurora on" checked={v.enabled} onChange={(enabled) => patchVisuals({ enabled })} />
            <button type="button" onClick={onClose} className="grid h-7 w-7 place-items-center rounded-full bg-white/10 hover:bg-white/20" aria-label="Close Aurora settings">
              <X size={14} />
            </button>
          </div>
          <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain [scrollbar-width:thin]">
            {!v.enabled && <p className="px-4 pt-3 text-[11.5px] leading-snug text-amber-200/80">Aurora is off: no sound is analysed and nothing is drawn. The lyrics still show.</p>}
            <LightSection v={v} />
            <VisualSection v={v} />
            <EffectsSection v={v} />
            <LyricsSection v={v} />
            <SoundSection v={v} status={status} />
            <ComfortSection v={v} />
            <EdgeSection v={v} />
            <DesktopSection v={v} />
            <SourcesSection />
          </div>
          <div className="border-t border-white/[0.08] p-3">
            <button
              type="button"
              onClick={() => {
                if (!confirm) return setConfirm(true);
                setConfirm(false);
                resetVisuals();
              }}
              className={cx('flex h-8 w-full items-center justify-center gap-2 rounded-[10px] text-[12px] font-semibold transition-colors', confirm ? 'bg-rose-500/80 text-white' : 'bg-white/10 text-white/80 hover:bg-white/15')}
            >
              <RotateCcw size={13} />
              {confirm ? 'Click again to reset everything' : 'Reset to defaults'}
            </button>
          </div>
        </motion.aside>
      )}
    </AnimatePresence>
  );
}
