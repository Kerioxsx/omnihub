// Night light on a schedule: warmer (and optionally dimmer) screens from
// evening to morning, fading in and out, and the monitors' own brightness
// for the night. "Now" turns it on or off until the schedule next changes.

import type { NightInfo, NightSettings } from '@shared/types';
import { Moon, Sunrise } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Switch, TextInput } from '../../components/ui/Form';
import { toast } from '../../state/toasts';
import { Slider } from './LightingTab';

export function NightLightCard() {
  const [info, setInfo] = useState<NightInfo | null>(null);
  useEffect(() => {
    void api.nightLight.get().then(setInfo, () => undefined);
  }, []);
  if (!info) return null;
  const s = info.settings;
  const save = async (next: NightSettings) => {
    setInfo({ ...info, settings: next });
    try {
      setInfo(await api.nightLight.save(next));
    } catch (e) {
      toast.error('Couldn’t save the night light', errorText(e));
    }
  };
  const now = async (on: boolean | null) => {
    try {
      setInfo(await api.nightLight.now(on));
    } catch (e) {
      toast.error('The screens didn’t change', errorText(e));
    }
  };
  const on = info.status.level > 0;
  return (
    <Card className="p-5">
      <CardHeader
        icon={Moon}
        title="Night light"
        subtitle="Warmer screens in the evening, easier on the eyes and on sleep. Fades in and out."
        actions={
          <div className="flex items-center gap-2">
            {on && <Badge tone="warn">{`On · ${info.status.kelvin} K`}</Badge>}
            <Button size="sm" variant={on ? 'secondary' : 'primary'} icon={on ? Sunrise : Moon} onClick={() => void now(!on)}>
              {on ? 'Off for now' : 'On now'}
            </Button>
            {info.status.manual !== null && (
              <Button size="sm" variant="ghost" onClick={() => void now(null)}>
                Follow the schedule
              </Button>
            )}
          </div>
        }
      />
      <div className="mt-4 grid gap-4 md:grid-cols-2">
        <div className="space-y-3">
          <div className="flex items-center justify-between gap-3 text-[13px] text-fg">
            Every night
            <Switch checked={s.enabled} onChange={(v) => void save({ ...s, enabled: v })} label="Night light on a schedule" />
          </div>
          <div className="flex items-center gap-2 text-[13px] text-dim">
            From
            <TextInput type="time" value={s.from} onChange={(e) => void save({ ...s, from: e.target.value })} className="w-[136px]" aria-label="From" />
            to
            <TextInput type="time" value={s.to} onChange={(e) => void save({ ...s, to: e.target.value })} className="w-[136px]" aria-label="To" />
          </div>
          <Slider label="Fade" value={s.fade} min={0} max={90} step={5} onChange={(v) => void save({ ...s, fade: v })} format={(v) => (v ? `${v} min` : 'Instant')} />
        </div>
        <div className="space-y-3">
          <Slider label="Warmth" value={6500 - s.kelvin} min={0} max={4600} step={100} onChange={(v) => void save({ ...s, kelvin: 6500 - v })} format={() => `${s.kelvin} K`} />
          <Slider label="Dim the picture" value={100 - s.dim} min={0} max={70} step={5} onChange={(v) => void save({ ...s, dim: 100 - v })} format={(v) => (v ? `${100 - v}%` : 'No')} />
          <div className="flex items-center justify-between gap-3 text-[13px] text-fg">
            <span>
              Monitor brightness at night <span className="text-faint">(DDC/CI, put back in the morning)</span>
            </span>
            <Switch checked={s.screenBrightness !== null} onChange={(v) => void save({ ...s, screenBrightness: v ? 30 : null })} label="Monitor brightness at night" />
          </div>
          {s.screenBrightness !== null && <Slider label="At night" value={s.screenBrightness} min={0} max={100} step={5} onChange={(v) => void save({ ...s, screenBrightness: v })} format={(v) => `${v}%`} />}
        </div>
      </div>
      <p className="mt-3 text-[11.5px] text-faint">Turn off Windows’ own Night light so the two don’t stack. Some games reset the colours while they run; OmniHub sets them again every 10 minutes.</p>
    </Card>
  );
}
