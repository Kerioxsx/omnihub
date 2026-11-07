// Stretched resolution per game (a lower 4:3 or 16:10 mode, like 1440×1080):
// made as a scene that runs by itself when the game starts, so the screen
// goes back to normal when it closes. The graphics driver has to stretch
// the picture to fill the screen; how is explained next to the button.

import type { LibraryGame, QuickState, Scene, ScreenMode } from '@shared/types';
import { Maximize2, MousePointer2, Trash2 } from 'lucide-react';
import { type PointerEvent as ReactPointerEvent, useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Field, Select } from '../../components/ui/Form';
import { toast } from '../../state/toasts';

const ratio = (m: ScreenMode) => m.width / m.height;
const aspect = (m: ScreenMode) => (Math.abs(ratio(m) - 4 / 3) < 0.02 ? '4:3' : Math.abs(ratio(m) - 5 / 4) < 0.02 ? '5:4' : Math.abs(ratio(m) - 16 / 10) < 0.02 ? '16:10' : `${ratio(m).toFixed(2)}:1`);

/** The narrower modes worth stretching, best refresh rate each. */
export function stretchModes(modes: ScreenMode[]): ScreenMode[] {
  const best = new Map<string, ScreenMode>();
  for (const m of modes) {
    if (ratio(m) > 1.7 || m.height < 720) continue;
    const k = `${m.width}x${m.height}`;
    const cur = best.get(k);
    if (!cur || m.hz > cur.hz) best.set(k, m);
  }
  return [...best.values()].sort((a, b) => b.width * b.height - a.width * a.height);
}

export function StretchedCard() {
  const [games, setGames] = useState<LibraryGame[]>([]);
  const [modes, setModes] = useState<ScreenMode[]>([]);
  const [quick, setQuick] = useState<QuickState | null>(null);
  const [game, setGame] = useState('');
  const [mode, setMode] = useState('');
  useEffect(() => {
    void api.library.games().then(setGames, () => undefined);
    void api.screenModes().then((m) => setModes(stretchModes(m.modes)), () => undefined);
    void api.quick.state().then(setQuick, () => undefined);
  }, []);
  const mine = (quick?.scenes ?? []).filter((s) => s.id.startsWith('stretch-'));
  const save = async (scenes: Scene[]) => {
    if (!quick) return;
    try {
      setQuick(await api.quick.save({ ...quick, scenes }));
    } catch (e) {
      toast.error('Couldn’t save it', errorText(e));
    }
  };
  const add = () => {
    const g = games.find((x) => x.key === game);
    const m = modes.find((x) => `${x.width}x${x.height}` === mode);
    if (!g || !m || !quick) return;
    const id = `stretch-${g.key.replace(/[^a-z0-9]/gi, '').slice(0, 40)}`;
    const scene: Scene = { id, name: `Stretched · ${g.name}`.slice(0, 40), icon: 'monitor', color: '#f59e0b', steps: [{ kind: 'screen', width: m.width, height: m.height, hz: m.hz }], game: g.key };
    void save([...quick.scenes.filter((s) => s.id !== id), scene]).then(() => toast.success(`${g.name} will start at ${m.width}×${m.height}`, 'The screen goes back when the game closes.'));
  };
  return (
    <Card className="p-5">
      <CardHeader icon={Maximize2} title="Stretched resolution" subtitle="Play a game at a lower 4:3 or 16:10 resolution (like 1440×1080) stretched to fill the screen; the screen goes back when the game closes." />
      <div className="mt-4 grid gap-3 sm:grid-cols-[1fr_1fr_auto] sm:items-end">
        <Field label="Game">
          <Select value={game} onChange={(e) => setGame(e.target.value)}>
            <option value="">Pick a game</option>
            {games.map((g) => (
              <option key={g.key} value={g.key}>
                {g.name}
              </option>
            ))}
          </Select>
        </Field>
        <Field label="Resolution">
          <Select value={mode} onChange={(e) => setMode(e.target.value)}>
            <option value="">Pick a resolution</option>
            {modes.map((m) => (
              <option key={`${m.width}x${m.height}`} value={`${m.width}x${m.height}`}>
                {m.width}×{m.height} ({aspect(m)}, {m.hz} Hz)
              </option>
            ))}
          </Select>
        </Field>
        <Button variant="primary" disabled={!game || !mode} onClick={add}>
          Use with this game
        </Button>
      </div>
      <p className="mt-3 text-[12.5px] leading-relaxed text-dim">
        So it stretches instead of showing black bars, set the graphics driver to scale: NVIDIA Control Panel → Adjust desktop size and position → <b>Full-screen</b>, “Perform scaling on: GPU”; or AMD Software → Display → <b>GPU Scaling</b> on, Scaling mode “Full panel”. In the game, pick the same resolution in fullscreen.
      </p>
      {mine.length > 0 && (
        <div className="mt-3 divide-y divide-line rounded-xl border border-line">
          {mine.map((s) => {
            const st = s.steps[0];
            return (
              <div key={s.id} className="flex items-center gap-3 px-3 py-2 text-[13px]">
                <span className="min-w-0 flex-1 truncate">{s.name.replace(/^Stretched · /, '')}</span>
                {st?.kind === 'screen' && (
                  <span className="tabular text-dim">
                    {st.width}×{st.height}
                  </span>
                )}
                <IconButton icon={Trash2} label="Stop stretching this game" onClick={() => quick && void save(quick.scenes.filter((x) => x.id !== s.id))} />
              </div>
            );
          })}
        </div>
      )}
    </Card>
  );
}

/** Polling rate from pointer events: Chromium hands over every report in
 *  `getCoalescedEvents`, so counting them over time gives reports per second. */
export function rateFrom(times: number[]): number {
  if (times.length < 20) return 0;
  const span = times[times.length - 1] - times[0];
  return span > 0 ? ((times.length - 1) * 1000) / span : 0;
}

export function MouseRateCard() {
  const times = useRef<number[]>([]);
  const [now, setNow] = useState(0);
  const [best, setBest] = useState(0);
  const last = useRef(0);
  const onMove = (e: ReactPointerEvent) => {
    const native = e.nativeEvent;
    const list = typeof native.getCoalescedEvents === 'function' ? native.getCoalescedEvents() : [native];
    for (const c of list.length ? list : [native]) times.current.push(c.timeStamp);
    const cut = performance.now() - 500;
    while (times.current.length && times.current[0] < cut) times.current.shift();
    if (performance.now() - last.current > 150) {
      last.current = performance.now();
      const r = rateFrom(times.current);
      setNow(r);
      setBest((b) => Math.max(b, r));
    }
  };
  const label = (hz: number) => (hz === 0 ? '—' : hz > 6000 ? '8000 Hz' : hz > 3000 ? '4000 Hz' : hz > 1500 ? '2000 Hz' : hz > 700 ? '1000 Hz' : hz > 350 ? '500 Hz' : hz > 180 ? '250 Hz' : '125 Hz');
  return (
    <Card className="p-5">
      <CardHeader icon={MousePointer2} title="Mouse polling rate" subtitle="Move the mouse quickly in circles inside the box. Gaming mice report 1000 times a second or more; office mice 125." />
      <div onPointerMove={onMove} onPointerLeave={() => (times.current = [])} className="mt-4 flex h-40 cursor-crosshair select-none items-center justify-center rounded-2xl border border-dashed border-line bg-surface-2/50">
        <div className="text-center">
          <div className="font-display text-[34px] font-bold tabular text-fg">{now ? Math.round(now) : '—'}</div>
          <div className="text-[12px] text-faint">reports per second now · best {best ? `${Math.round(best)} (about ${label(best)})` : '—'}</div>
        </div>
      </div>
      <p className="mt-2 text-[12px] text-faint">Windows and the app round a little, so 1000 Hz mice show about 900–1000. Set the rate in your mouse’s own app.</p>
    </Card>
  );
}
