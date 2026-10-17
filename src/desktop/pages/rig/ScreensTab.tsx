// Screens inside the PC: a case display Windows sees as a monitor (OmniHub
// fills it with a window) and a cooler's LCD (through liquidctl), each with
// a picture, GIF or video from the library, the PC's stats or the song.

import type { RigMedia, ScreenShow } from '@shared/types';
import { Film, ImagePlus, MonitorSmartphone, Trash2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText, rigMediaUrl } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Select, Switch } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { toast } from '../../state/toasts';
import { Row } from '../games/shared';
import { Slider } from './LightingTab';
import type { Rig } from './useRig';

export function MediaThumb({ m, className }: { m: RigMedia; className?: string }) {
  return m.kind === 'video' ? <video src={rigMediaUrl(m.id)} muted loop autoPlay playsInline className={cx('object-cover', className)} /> : <img src={rigMediaUrl(m.id)} alt={m.name} className={cx('object-cover', className)} draggable={false} />;
}

/** "media:<id>" / "stats" … for a <select>. */
const showKey = (s: ScreenShow) => (s.kind === 'media' ? `media:${s.id}` : s.kind);
const fromKey = (k: string): ScreenShow => (k.startsWith('media:') ? { kind: 'media', id: k.slice(6) } : ({ kind: k } as ScreenShow));

export function ScreensTab({ rig }: { rig: Rig }) {
  const s = rig.settings;
  const st = rig.status;
  const [displays, setDisplays] = useState<{ name: string; label: string; primary: boolean }[]>([]);
  useEffect(() => void api.rig.displays().then(setDisplays, () => undefined), []);
  if (!s) return null;
  const lcds = st?.coolerDevices.filter((d) => d.lcd) ?? [];
  const add = async () => {
    try {
      const m = await api.rig.addMedia();
      if (m) {
        rig.reloadMedia();
        toast.success('Added to your screens', m.name);
      }
    } catch (e) {
      toast.error('Not added', errorText(e));
    }
  };
  const remove = async (m: RigMedia) => {
    try {
      await api.rig.removeMedia(m.id);
      rig.reloadMedia();
    } catch (e) {
      toast.error('Not removed', errorText(e));
    }
  };
  const pictures = rig.media.filter((m) => m.kind !== 'video');

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader title="Your pictures and videos" subtitle="PNG, JPEG, GIF, WebP, MP4 or WebM, kept in OmniHub’s folder." className="px-5 pt-4" actions={<Button size="sm" icon={ImagePlus} onClick={() => void add()}>Add</Button>} />
        {rig.media.length ? (
          <div className="grid grid-cols-3 gap-2 p-4 sm:grid-cols-4 xl:grid-cols-5">
            {rig.media.map((m) => (
              <div key={m.id} className="group relative aspect-square overflow-hidden rounded-xl border border-line bg-black">
                <MediaThumb m={m} className="h-full w-full" />
                <div className="absolute inset-x-0 bottom-0 flex items-center justify-between bg-gradient-to-t from-black/80 to-transparent px-2 pb-1.5 pt-5 text-[11.5px] text-white">
                  <span className="flex min-w-0 items-center gap-1 truncate">
                    {m.kind === 'video' && <Film size={11} />}
                    {m.name}
                  </span>
                  <IconButton icon={Trash2} size="sm" variant="danger" label={`Remove ${m.name}`} onClick={() => void remove(m)} className="opacity-0 group-hover:opacity-100" />
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="px-5 pb-4 text-[12.5px] text-faint">Nothing yet: add a picture, GIF or video for the case screen or the cooler’s display.</div>
        )}
      </Card>

      <Card>
        <CardHeader icon={MonitorSmartphone} title="Case screen" subtitle="A screen inside the case that Windows shows as a monitor (HYTE Y70 Touch, Lian Li or other sensor panels)." className="px-5 pt-4" />
        <div className="divide-y divide-line">
          <Row title="Which screen">
            <Select value={s.screens.monitor} onChange={(e) => void rig.update({ screens: { monitor: e.target.value } })} className="w-64">
              <option value="">None</option>
              {displays.map((d) => (
                <option key={d.name} value={d.name} disabled={d.primary}>
                  {d.label}
                </option>
              ))}
            </Select>
          </Row>
          <Row title="Show">
            <Select value={showKey(s.screens.monitorShow)} onChange={(e) => void rig.update({ screens: { monitorShow: fromKey(e.target.value) } })} className="w-64" disabled={!s.screens.monitor}>
              <option value="off">Nothing (screen free for Windows)</option>
              <option value="stats">PC stats: temperatures, fans, loads</option>
              <option value="nowPlaying">The song playing, with its cover</option>
              {rig.media.map((m) => (
                <option key={m.id} value={`media:${m.id}`}>
                  {m.kind === 'video' ? 'Video' : m.kind === 'gif' ? 'GIF' : 'Picture'}: {m.name}
                </option>
              ))}
            </Select>
          </Row>
          <Row title="Fill the screen" hint="Off: the whole picture, with bars where the shapes differ.">
            <Switch checked={s.screens.fill} onChange={(v) => void rig.update({ screens: { fill: v } })} label="Fill the screen" />
          </Row>
        </div>
      </Card>

      <Card>
        <CardHeader title="Cooler screen" subtitle="NZXT Kraken Z, Kraken 2023 and 2024 Elite: a picture, a GIF or the liquid temperature, through liquidctl." className="px-5 pt-4" />
        {lcds.length ? (
          <div className="divide-y divide-line">
            <Row title="Cooler">
              <Select value={s.screens.lcd} onChange={(e) => void rig.update({ screens: { lcd: e.target.value } })} className="w-64">
                <option value="">None</option>
                {lcds.map((d) => (
                  <option key={d.address} value={d.address}>
                    {d.description}
                  </option>
                ))}
              </Select>
            </Row>
            <Row title="Show">
              <Select value={showKey(s.screens.lcdShow)} onChange={(e) => void rig.update({ screens: { lcdShow: fromKey(e.target.value) } })} className="w-64" disabled={!s.screens.lcd}>
                <option value="liquid">Liquid temperature</option>
                {pictures.map((m) => (
                  <option key={m.id} value={`media:${m.id}`}>
                    {m.kind === 'gif' ? 'GIF' : 'Picture'}: {m.name}
                  </option>
                ))}
              </Select>
            </Row>
            <div className="px-5 py-3">
              <Slider label="Brightness" value={s.screens.lcdBrightness} min={0} max={100} step={5} onChange={(v) => void rig.update({ screens: { lcdBrightness: v } })} format={(v) => `${v}%`} />
            </div>
            {st?.lcdMessage && <div className="px-5 py-3 text-[12.5px] text-warn">{st.lcdMessage}</div>}
          </div>
        ) : (
          <div className="p-4">
            <Callout tone="info" title="No cooler screen found">
              With liquidctl installed (Fans tab), a Kraken with a screen shows up here. Other brands’ screens (Corsair iCUE LINK, Lian Li Galahad II LCD, ASUS Ryujin) are set in their own apps; the 3D view still shows your picture on the pump.
            </Callout>
          </div>
        )}
      </Card>
    </div>
  );
}
