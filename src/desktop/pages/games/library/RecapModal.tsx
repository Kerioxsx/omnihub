// The recap when a game closes: how long, how it ran, the clips and
// screenshots from that session, and the automatic save backup.

import type { GameRecap } from '@shared/types';
import { Clapperboard, Gauge, Save, Timer } from 'lucide-react';
import { useState } from 'react';
import { Button } from '../../../components/ui/Button';
import { Modal } from '../../../components/ui/Overlay';
import { useEvent } from '../../../lib/hooks';
import { navigate } from '../../../lib/router';

export function RecapModal() {
  const [r, setR] = useState<GameRecap | null>(null);
  useEvent<GameRecap>('games:recap', setR);
  if (!r) return null;
  const h = Math.floor(r.minutes / 60);
  const stat = (Icon: typeof Timer, label: string, value: string) => (
    <div className="rounded-xl bg-surface px-3 py-2">
      <div className="flex items-center gap-1.5 text-[11.5px] text-faint">
        <Icon size={12} /> {label}
      </div>
      <div className="font-display text-[20px] font-semibold tabular-nums text-fg">{value}</div>
    </div>
  );
  return (
    <Modal open onClose={() => setR(null)} title={`${r.name}: session recap`} description={`${new Date(r.start).toLocaleTimeString(undefined, { timeStyle: 'short' })} – ${new Date(r.end).toLocaleTimeString(undefined, { timeStyle: 'short' })}`} size="lg" footer={<Button variant="primary" onClick={() => setR(null)}>Nice</Button>}>
      <div className="space-y-4">
        {r.art.hero && <img src={r.art.hero} alt="" className="h-32 w-full rounded-xl object-cover" onError={(e) => (e.currentTarget.style.display = 'none')} />}
        <div className="grid grid-cols-3 gap-2">
          {stat(Timer, 'Played', h ? `${h} h ${r.minutes % 60} min` : `${r.minutes} min`)}
          {stat(Gauge, 'Average FPS', r.avgFps ? Math.round(r.avgFps).toString() : '—')}
          {stat(Gauge, '1% low', r.lowFps ? Math.round(r.lowFps).toString() : '—')}
        </div>
        {r.clips.length > 0 && (
          <div>
            <div className="mb-1.5 flex items-center gap-1.5 text-[13px] font-semibold text-fg">
              <Clapperboard size={14} /> Clips
            </div>
            <div className="flex flex-wrap gap-2">
              {r.clips.map((c) => (
                <Button key={c.src || c.name} size="sm" onClick={() => (setR(null), navigate('games', { tab: 'clips' }))}>
                  {c.name}
                </Button>
              ))}
            </div>
          </div>
        )}
        {r.screenshots.length > 0 && (
          <div className="grid grid-cols-4 gap-2">
            {r.screenshots.map((s) => (
              <img key={s.src} src={s.src} alt="" className="aspect-video w-full rounded-lg object-cover" />
            ))}
          </div>
        )}
        {r.backup && (
          <p className="flex items-center gap-1.5 text-[12.5px] text-good">
            <Save size={13} /> Saves backed up.
          </p>
        )}
        {!r.avgFps && <p className="text-[11.5px] text-faint">FPS shows when the game was started with a boost (Games → Boost profiles) and the FPS counter is on.</p>}
      </div>
    </Modal>
  );
}
