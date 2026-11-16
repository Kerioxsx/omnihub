// A crosshair over any game: drawn in its own see-through window that
// clicks pass through. Some games with anti-cheat dislike overlays, so it's
// off by default and says so.

import { Crosshair } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Card, CardHeader } from '../../components/ui/Card';
import { Select, Switch } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { toast } from '../../state/toasts';
import { CrosshairShape } from '../floats/CrosshairShape';
import { type CrosshairLook, readLook, saveLook } from '../floats/crosshair';
import { Slider } from '../rig/LightingTab';

const COLORS = ['#00ff88', '#00e5ff', '#ffff00', '#ff3b3b', '#ff00ff', '#ffffff'];

export function CrosshairCard() {
  const [on, setOn] = useState(false);
  const [look, setLook] = useState<CrosshairLook>(readLook);
  useEffect(() => {
    void api.widgets.get().then((f) => setOn(f.crosshair), () => undefined);
  }, []);
  const change = (l: CrosshairLook) => {
    setLook(l);
    saveLook(l);
  };
  const flip = async (v: boolean) => {
    try {
      setOn(await api.widgets.crosshair(v));
    } catch (e) {
      toast.error('The crosshair didn’t open', errorText(e));
    }
  };
  return (
    <Card className="p-5">
      <CardHeader icon={Crosshair} title="Crosshair overlay" subtitle="Your own crosshair in the middle of the screen, over any game. Clicks go straight through it." actions={<Switch checked={on} onChange={(v) => void flip(v)} label="Crosshair on" />} />
      <div className="mt-4 grid gap-5 md:grid-cols-[160px_1fr]">
        <div className="grid h-[160px] place-items-center rounded-xl border border-line bg-[repeating-conic-gradient(#3a3a46_0_25%,#2a2a33_0_50%)] bg-[length:24px_24px]">
          <CrosshairShape look={look} box={140} />
        </div>
        <div className="space-y-3">
          <div className="flex flex-wrap items-center gap-2">
            <Select value={look.style} onChange={(e) => change({ ...look, style: e.target.value as CrosshairLook['style'] })} className="w-auto" aria-label="Shape">
              <option value="cross">Cross</option>
              <option value="tcross">T-shape</option>
              <option value="dot">Dot</option>
              <option value="circle">Circle</option>
              <option value="chevron">Chevron</option>
            </Select>
            {COLORS.map((c) => (
              <button key={c} type="button" aria-label={`Colour ${c}`} aria-pressed={look.color === c} onClick={() => change({ ...look, color: c })} className={`h-7 w-7 rounded-full border-2 ${look.color === c ? 'border-fg' : 'border-transparent'}`} style={{ background: c }} />
            ))}
            <label className="ml-2 flex items-center gap-2 text-[12.5px] text-dim">
              Outline <Switch checked={look.outline} onChange={(v) => change({ ...look, outline: v })} label="Outline" size="md" />
            </label>
            <label className="flex items-center gap-2 text-[12.5px] text-dim">
              Centre dot <Switch checked={look.dot} onChange={(v) => change({ ...look, dot: v })} label="Centre dot" />
            </label>
          </div>
          <Slider label="Size" value={look.size} min={2} max={40} step={1} onChange={(v) => change({ ...look, size: v })} format={(v) => `${v}px`} />
          <Slider label="Thickness" value={look.thickness} min={1} max={8} step={1} onChange={(v) => change({ ...look, thickness: v })} format={(v) => `${v}px`} />
          <Slider label="Gap" value={look.gap} min={0} max={20} step={1} onChange={(v) => change({ ...look, gap: v })} format={(v) => `${v}px`} />
          <Slider label="Opacity" value={Math.round(look.opacity * 100)} min={20} max={100} step={5} onChange={(v) => change({ ...look, opacity: v / 100 })} format={(v) => `${v}%`} />
        </div>
      </div>
      <Callout tone="warn" className="mt-4 text-[12.5px]">
        Some games with strict anti-cheat (like Valorant, FACEIT or ESEA) may not like overlays. It doesn’t touch the game, but turn it off there if you’re unsure. Games in exclusive full screen draw over it; use borderless.
      </Callout>
    </Card>
  );
}
