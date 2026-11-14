// Your desktop: small widgets that sit on it (under other windows; drag
// them anywhere), and your OmniHub theme as a live wallpaper behind the
// icons.

import type { Floats, WidgetKind } from '@shared/types';
import { Battery, Clock, Gauge, Image, Music2, Plus, Sparkles, Thermometer, Trash2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Switch } from '../../components/ui/Form';
import { toast } from '../../state/toasts';

const KINDS: { kind: WidgetKind; label: string; icon: typeof Clock }[] = [
  { kind: 'temps', label: 'Temperatures', icon: Thermometer },
  { kind: 'music', label: 'Now playing', icon: Music2 },
  { kind: 'counter', label: 'Counter', icon: Sparkles },
  { kind: 'battery', label: 'Batteries', icon: Battery },
  { kind: 'network', label: 'Network speed', icon: Gauge },
  { kind: 'clock', label: 'Clock', icon: Clock },
];

export function DesktopTab() {
  const [f, setF] = useState<Floats | null>(null);
  useEffect(() => {
    void api.widgets.get().then(setF, () => undefined);
  }, []);
  if (!f) return null;
  const run = async (p: Promise<Floats>) => {
    try {
      setF(await p);
    } catch (e) {
      toast.error('That didn’t work', errorText(e));
    }
  };
  return (
    <div className="space-y-4">
      <Card className="p-5">
        <CardHeader icon={Plus} title="Desktop widgets" subtitle="Small cards that stay on your desktop, under your windows. Drag them where you like; they come back when OmniHub starts." />
        <div className="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-6">
          {KINDS.map((k) => (
            <button key={k.kind} type="button" onClick={() => void run(api.widgets.add(k.kind))} className="flex flex-col items-center gap-2 rounded-2xl border border-line bg-surface-2/40 px-3 py-4 text-[12.5px] font-medium text-dim transition-colors hover:border-accent/50 hover:text-fg">
              <k.icon size={20} />
              {k.label}
            </button>
          ))}
        </div>
        {f.widgets.length > 0 && (
          <div className="mt-4 divide-y divide-line rounded-xl border border-line">
            {f.widgets.map((w) => {
              const k = KINDS.find((x) => x.kind === w.kind);
              return (
                <div key={w.id} className="flex items-center gap-3 px-3.5 py-2">
                  {k && <k.icon size={15} className="text-dim" />}
                  <span className="flex-1 text-[13px] text-fg">{k?.label ?? w.kind}</span>
                  <IconButton icon={Trash2} label="Remove" onClick={() => void run(api.widgets.remove(w.id))} />
                </div>
              );
            })}
          </div>
        )}
      </Card>
      <Card className="p-5">
        <CardHeader
          icon={Image}
          title="Live wallpaper"
          subtitle="Your OmniHub theme’s animated background (Bleach, Minecraft, Synthwave…) behind your desktop icons. It uses a little of the graphics card; turn it off while gaming if you like."
          actions={<Switch checked={f.wallpaper} onChange={(v) => void api.widgets.wallpaper(v).then((on) => setF({ ...f, wallpaper: on }), (e: unknown) => toast.error('The wallpaper didn’t start', errorText(e)))} label="Live wallpaper" />}
        />
        <p className="mt-3 text-[12px] text-faint">Change the theme in Settings → Appearance; the wallpaper follows it. Turning it off brings back your usual wallpaper.</p>
      </Card>
      <p className="px-1 text-[11.5px] text-faint">
        <Button size="xs" variant="ghost" onClick={() => void api.widgets.get().then(setF)}>
          Refresh
        </Button>
      </p>
    </div>
  );
}
