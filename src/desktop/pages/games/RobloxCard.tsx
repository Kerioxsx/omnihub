// Roblox Fast Flags: presets plus the individual graphics options Roblox
// still reads from the local settings file, and custom flags.

import type { RobloxFlags, RobloxInstall, RobloxPreset, RobloxRenderer } from '@shared/types';
import { AlertTriangle, Blocks, Save } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Segmented, Select, Switch } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { toast } from '../../state/toasts';
import { Row } from './shared';

const PRESETS: Record<Exclude<RobloxPreset, 'custom'>, Partial<RobloxFlags>> = {
  maxFps: { renderer: 'd3d11', msaa: 0, textureQuality: 0, noGrass: true, lowDetailDistance: true, qualityLevel: 1, exclusiveFullscreen: true, graySky: false, ignoreDisplayScaling: false },
  balanced: { renderer: 'd3d11', msaa: null, textureQuality: null, noGrass: true, lowDetailDistance: false, qualityLevel: null, exclusiveFullscreen: true, graySky: false, ignoreDisplayScaling: false },
  quality: { renderer: 'auto', msaa: 4, textureQuality: 3, noGrass: false, lowDetailDistance: false, qualityLevel: null, exclusiveFullscreen: false, graySky: false, ignoreDisplayScaling: false },
};

/** What the options look like for a preset (Custom shows them as set). */
function effective(f: RobloxFlags): RobloxFlags {
  return f.preset === 'custom' ? f : { ...f, ...PRESETS[f.preset] };
}

export function RobloxCard({ flags, onChange }: { flags: RobloxFlags; onChange: (f: RobloxFlags) => void }) {
  const [install, setInstall] = useState<RobloxInstall | null>(null);
  const [preview, setPreview] = useState<{ flags: Record<string, unknown>; ignored: string[] } | null>(null);
  const [custom, setCustom] = useState(() => (Object.keys(flags.custom).length ? JSON.stringify(flags.custom, null, 2) : ''));
  const [customError, setCustomError] = useState<string | null>(null);
  const [writing, setWriting] = useState(false);

  useEffect(() => {
    void api.games.robloxStatus().then(setInstall, () => undefined);
  }, []);
  useEffect(() => {
    void api.games.robloxPreview(flags).then(setPreview, () => undefined);
  }, [flags]);

  const f = effective(flags);
  // Touching any single option turns the preset into Custom.
  const set = (patch: Partial<RobloxFlags>) => onChange({ ...f, ...patch, preset: 'custom' });

  const applyCustom = () => {
    if (!custom.trim()) {
      setCustomError(null);
      return onChange({ ...flags, custom: {} });
    }
    try {
      const v = JSON.parse(custom) as unknown;
      if (!v || typeof v !== 'object' || Array.isArray(v)) throw new Error('Use an object: { "FlagName": "value" }');
      setCustomError(null);
      onChange({ ...flags, custom: v as RobloxFlags['custom'] });
    } catch (e) {
      setCustomError(e instanceof Error ? e.message : String(e));
    }
  };

  const write = async () => {
    setWriting(true);
    try {
      const files = await api.games.robloxWrite(flags);
      toast.success('Roblox flags written', `${files.length} file${files.length === 1 ? '' : 's'}.${install?.running ? ' Restart Roblox to use them.' : ''}`);
    } catch (e) {
      toast.error('Could not write the flags', errorText(e));
    } finally {
      setWriting(false);
    }
  };

  const level = (v: string) => (v === '' ? null : Number(v));
  return (
    <Card>
      <CardHeader
        icon={Blocks}
        title="Roblox Fast Flags"
        subtitle={install ? (install.found ? `Roblox ${install.version ?? ''}${install.bootstrapper ? ` · managed by ${install.bootstrapper}` : ''}` : 'Roblox not found — install it and start it once') : 'Looking for Roblox…'}
        className="px-5 pt-4"
        actions={
          <>
            <Switch checked={flags.enabled} onChange={(v) => onChange({ ...flags, enabled: v })} label="Write flags before Roblox starts" />
            <Button size="sm" variant="secondary" icon={Save} loading={writing} disabled={!install?.found} onClick={() => void write()}>
              Write now
            </Button>
          </>
        }
      />
      <div className="mt-2 divide-y divide-line">
        <Row title="Preset" hint="Max FPS turns graphics all the way down for the most frames; Balanced keeps the look but drops grass; Quality sharpens textures and edges.">
          <Segmented
            size="sm"
            label="Preset"
            value={flags.preset}
            onChange={(v) => onChange({ ...flags, preset: v })}
            options={[
              { value: 'maxFps', label: 'Max FPS' },
              { value: 'balanced', label: 'Balanced' },
              { value: 'quality', label: 'Quality' },
              { value: 'custom', label: 'Custom' },
            ]}
          />
        </Row>
        <Row title="Graphics API" hint="Direct3D 11 is the most stable on most PCs; Vulkan can be faster on some AMD cards.">
          <Select value={f.renderer} onChange={(e) => set({ renderer: e.target.value as RobloxRenderer })} className="w-40">
            <option value="auto">Automatic</option>
            <option value="d3d11">Direct3D 11</option>
            <option value="vulkan">Vulkan</option>
            <option value="openGl">OpenGL</option>
          </Select>
        </Row>
        <Row title="Graphics quality" hint="Forces Roblox's quality level (1 is the lowest, 21 the highest).">
          <Select value={f.qualityLevel ?? ''} onChange={(e) => set({ qualityLevel: level(e.target.value) })} className="w-40">
            <option value="">Roblox decides</option>
            {[1, 3, 5, 8, 10, 13, 15, 18, 21].map((n) => (
              <option key={n} value={n}>
                Level {n}
              </option>
            ))}
          </Select>
        </Row>
        <Row title="Anti-aliasing">
          <Select value={f.msaa ?? ''} onChange={(e) => set({ msaa: level(e.target.value) })} className="w-40">
            <option value="">Roblox decides</option>
            <option value="0">Off</option>
            <option value="1">1×</option>
            <option value="2">2×</option>
            <option value="4">4×</option>
            <option value="8">8×</option>
          </Select>
        </Row>
        <Row title="Texture quality">
          <Select value={f.textureQuality ?? ''} onChange={(e) => set({ textureQuality: level(e.target.value) })} className="w-40">
            <option value="">Roblox decides</option>
            <option value="0">Lowest</option>
            <option value="1">Low</option>
            <option value="2">Medium</option>
            <option value="3">Highest</option>
          </Select>
        </Row>
        <Row title="No grass">
          <Switch checked={f.noGrass} onChange={(v) => set({ noGrass: v })} label="No grass" />
        </Row>
        <Row title="Simpler distant objects" hint="Lower detail for far-away parts — more FPS in big maps.">
          <Switch checked={f.lowDetailDistance} onChange={(v) => set({ lowDetailDistance: v })} label="Simpler distant objects" />
        </Row>
        <Row title="Plain gray sky">
          <Switch checked={f.graySky} onChange={(v) => set({ graySky: v })} label="Plain gray sky" />
        </Row>
        <Row title="Alt+Enter for exclusive fullscreen" hint="Exclusive fullscreen can lower input delay.">
          <Switch checked={f.exclusiveFullscreen} onChange={(v) => set({ exclusiveFullscreen: v })} label="Exclusive fullscreen" />
        </Row>
        <Row title="Ignore display scaling" hint="Renders at full resolution on scaled screens: sharper, but fewer FPS.">
          <Switch checked={f.ignoreDisplayScaling} onChange={(v) => set({ ignoreDisplayScaling: v })} label="Ignore display scaling" />
        </Row>
        <Row title="Custom flags" hint="Advanced: any other flags as JSON. Roblox ignores flags outside its allowlist." stack>
          <textarea
            value={custom}
            onChange={(e) => setCustom(e.target.value)}
            onBlur={applyCustom}
            spellCheck={false}
            rows={4}
            placeholder={'{\n  "FFlagDebugSkyGray": "True"\n}'}
            className="w-full resize-y rounded-xl border border-line bg-surface p-3 font-mono text-[12px] text-fg outline-none focus:border-accent/60"
          />
          {customError && <div className="text-[12px] text-bad">{customError}</div>}
        </Row>
      </div>
      <div className="space-y-2 px-5 pb-4 pt-2">
        {preview && (
          <details className="rounded-xl bg-surface px-3 py-2 text-[12px] text-dim">
            <summary className="cursor-pointer select-none text-fg">{Object.keys(preview.flags).length} flags will be written</summary>
            <pre className="mt-2 max-h-48 overflow-auto font-mono text-[11.5px] text-faint">{JSON.stringify(preview.flags, null, 2)}</pre>
          </details>
        )}
        {preview && preview.ignored.length > 0 && (
          <Callout tone="warn" icon={AlertTriangle} className="text-[12.5px]">
            Roblox ignores {preview.ignored.join(', ')} — only allowlisted flags work since late 2025.
          </Callout>
        )}
        {install?.running && <div className="text-[12px] text-warn">Roblox is running: flags apply from its next start.</div>}
        <div className="text-[12px] text-faint">Frame-rate cap: set it in Roblox itself (Settings → Maximum Frame Rate).</div>
      </div>
    </Card>
  );
}
