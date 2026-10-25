// Roblox shaders, like RoShade and Bloxshade: the official ReShade (the
// build Roblox allows) with OmniHub's presets, put in Roblox's folder and
// back again after each Roblox update.

import type { RobloxRenderer, RobloxShaders as Shaders, ShaderPreset, ShaderStatus } from '@shared/types';
import { AlertTriangle, Sparkles } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText, on } from '../../api';
import { Button } from '../../components/ui/Button';
import { Segmented, Switch } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { toast } from '../../state/toasts';
import { Slider } from '../rig/LightingTab';

const LOOKS: { value: ShaderPreset; label: string; hint: string }[] = [
  { value: 'realistic', label: 'Realistic', hint: 'Shadows in corners and under objects (ambient occlusion), soft bloom, a little contrast and a vignette.' },
  { value: 'vivid', label: 'Vivid', hint: 'Livelier colours, a crisper image and a light glow.' },
  { value: 'cinematic', label: 'Cinematic', hint: 'Warmer and contrasty, with bloom, deep shadows and a vignette, like a film.' },
  { value: 'soft', label: 'Soft', hint: 'A dreamy glow and gentle colours.' },
  { value: 'sharp', label: 'Sharp', hint: 'Only sharper and a touch more colour: the lightest on frame rate.' },
  { value: 'custom', label: 'Custom', hint: 'Starts as Realistic; whatever you change in ReShade’s menu in the game (Home) is kept.' },
];

export function RobloxShaders({ value, onChange, renderer, found }: { value: Shaders; onChange: (s: Shaders) => void; renderer: RobloxRenderer; found: boolean }) {
  const [status, setStatus] = useState<ShaderStatus | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    void api.games.robloxShaders().then(setStatus, () => undefined);
    const off = on<{ text: string }>('games:roblox-shaders', (p) => setBusy(p.text));
    return () => void off.then((f) => f());
  }, []);

  const apply = async (s: Shaders) => {
    setBusy(s.enabled ? 'Getting ReShade ready…' : 'Taking the shaders out…');
    try {
      const st = await api.games.robloxShadersApply(s);
      setStatus(st);
      if (s.enabled) toast.success('Shaders ready', `${st.applied ? 'In Roblox now' : 'Saved'}: start Roblox, then press Home for ReShade’s menu or End to turn them off and on.`);
    } catch (e) {
      toast.error(s.enabled ? 'Shaders not added' : 'Shaders not removed', errorText(e));
    } finally {
      setBusy(null);
    }
  };
  // Turning them on or off takes effect at once; looks are applied with Apply (and before each start).
  const toggle = (enabled: boolean) => {
    const next = { ...value, enabled };
    onChange(next);
    if (found) void apply(next);
  };
  const look = LOOKS.find((l) => l.value === value.preset) ?? LOOKS[0];

  return (
    <div className="space-y-3 border-t border-line px-5 py-4">
      <div className="flex items-start justify-between gap-3">
        <div>
          <div className="flex items-center gap-2 text-[13.5px] font-medium text-fg">
            <Sparkles className="h-4 w-4 text-accent" aria-hidden /> Shaders
          </div>
          <div className="mt-0.5 text-[12px] text-faint">Like RoShade and Bloxshade: ReShade’s effects on Roblox (the official ReShade, which Roblox allows). In the game, Home opens ReShade’s menu and End turns the effects off and on.</div>
        </div>
        <Switch checked={value.enabled} onChange={toggle} disabled={!!busy || !found} label="Shaders" />
      </div>
      {value.enabled && (
        <>
          <Segmented size="sm" label="Look" value={value.preset} onChange={(v) => onChange({ ...value, preset: v })} options={LOOKS.map(({ value: v, label }) => ({ value: v, label }))} />
          <p className="text-[11.5px] text-faint">{look.hint}</p>
          {value.preset !== 'custom' && <Slider label="Strength" value={value.strength} min={25} max={150} step={5} onChange={(v) => onChange({ ...value, strength: v })} format={(v) => `${v}%`} />}
          <div className="flex flex-wrap items-center gap-3">
            <Button size="sm" variant="secondary" icon={Sparkles} loading={!!busy} disabled={!!busy || !found} onClick={() => void apply(value)}>
              Apply now
            </Button>
            <span className="text-[12px] text-dim">{busy ?? (status?.version ? `ReShade ${status.version}${status.applied ? ` · in ${status.applied} Roblox folder${status.applied === 1 ? '' : 's'}` : ''}` : 'ReShade downloads the first time (about 6 MB).')}</span>
          </div>
          {(renderer === 'vulkan' || renderer === 'openGl') && (
            <Callout tone="warn" icon={AlertTriangle} className="text-[12.5px]">
              Shaders work with Direct3D 11: set Graphics mode above to Auto or Direct3D 11.
            </Callout>
          )}
          <p className="text-[11.5px] text-faint">OmniHub puts them back after each Roblox update when you start Roblox from here. They cost some frames per second; Sharp costs the least.</p>
        </>
      )}
      {status && status.foreign.length > 0 && (
        <Callout tone="warn" icon={AlertTriangle} className="text-[12.5px]">
          Another shader tool (RoShade, Bloxshade or a ReShade you installed) is in Roblox’s folder. OmniHub leaves it alone; uninstall it to use these shaders instead.
        </Callout>
      )}
    </div>
  );
}
