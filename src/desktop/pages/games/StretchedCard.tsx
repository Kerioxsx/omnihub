// Play resolution: the whole PC switches to 4K, 1440p (2K), 1080p, 720p or a
// stretched 4:3 mode, so games in windowed fullscreen (borderless) run at it
// and alt-tab stays instant. Chosen per game (switches when the game starts,
// back when it closes) or right now with a button. The switch asks the
// graphics card to scale, which skips the monitor's black re-sync.

import type { LibraryGame, QuickState, Scene, ScreenMode } from '@shared/types';
import { Check, Maximize2, MousePointer2, RotateCcw, Trash2, Zap } from 'lucide-react';
import { type PointerEvent as ReactPointerEvent, useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Field, Select } from '../../components/ui/Form';
import { cx } from '../../lib/cx';
import { toast } from '../../state/toasts';

const ratio = (m: { width: number; height: number }) => m.width / m.height;
const aspect = (m: { width: number; height: number }) => (Math.abs(ratio(m) - 4 / 3) < 0.02 ? '4:3' : Math.abs(ratio(m) - 5 / 4) < 0.02 ? '5:4' : Math.abs(ratio(m) - 16 / 10) < 0.02 ? '16:10' : Math.abs(ratio(m) - 16 / 9) < 0.02 ? '16:9' : ratio(m) > 2.2 ? '21:9' : `${ratio(m).toFixed(2)}:1`);

const NAMES: Record<string, string> = {
  '3840x2160': '4K',
  '3440x1440': 'Ultrawide 1440p',
  '2560x1440': '1440p (2K)',
  '2560x1080': 'Ultrawide 1080p',
  '1920x1080': '1080p',
  '1600x900': '900p',
  '1280x720': '720p',
  '1920x1440': 'Stretched 1440',
  '1440x1080': 'Stretched 1080',
  '1280x960': 'Stretched 960',
  '1680x1050': '1050p',
};

export interface PlayRes {
  width: number;
  height: number;
  /** Highest refresh rate offered at this size. */
  hz: number;
  name: string;
  stretched: boolean;
}

/** Each resolution the screen offers once (named ones first, then the rest), biggest first. */
export function playResolutions(modes: ScreenMode[]): PlayRes[] {
  const best = new Map<string, PlayRes>();
  for (const m of modes) {
    if (m.height < 720) continue;
    const k = `${m.width}x${m.height}`;
    const cur = best.get(k);
    if (!cur || m.hz > cur.hz) best.set(k, { width: m.width, height: m.height, hz: m.hz, name: NAMES[k] ?? `${m.width}×${m.height}`, stretched: ratio(m) < 1.7 && ratio(m) > 1.2 && Math.abs(ratio(m) - 16 / 10) > 0.02 });
  }
  return [...best.values()].sort((a, b) => Number(!(`${a.width}x${a.height}` in NAMES)) - Number(!(`${b.width}x${b.height}` in NAMES)) || b.width * b.height - a.width * a.height);
}

/** Kept for older imports: the narrower modes worth stretching. */
export function stretchModes(modes: ScreenMode[]): ScreenMode[] {
  return playResolutions(modes).filter((r) => r.stretched).map((r) => ({ width: r.width, height: r.height, hz: r.hz }));
}

export function PlayResolutionCard() {
  const [games, setGames] = useState<LibraryGame[]>([]);
  const [res, setRes] = useState<PlayRes[]>([]);
  const [current, setCurrent] = useState<ScreenMode | null>(null);
  const [quick, setQuick] = useState<QuickState | null>(null);
  const [game, setGame] = useState('');
  const [pick, setPick] = useState('');
  const [busy, setBusy] = useState(false);
  const [changed, setChanged] = useState(false);
  const load = () =>
    void api.screenModes().then((m) => {
      setRes(playResolutions(m.modes));
      setCurrent(m.current);
    }, () => undefined);
  useEffect(() => {
    void api.library.games().then(setGames, () => undefined);
    void api.quick.state().then(setQuick, () => undefined);
    load();
  }, []);
  const mine = (quick?.scenes ?? []).filter((s) => s.id.startsWith('stretch-') || s.id.startsWith('playres-'));
  const chosen = res.find((r) => `${r.width}x${r.height}` === pick);
  const save = async (scenes: Scene[]) => {
    if (!quick) return;
    try {
      setQuick(await api.quick.save({ ...quick, scenes }));
    } catch (e) {
      toast.error('Couldn’t save it', errorText(e));
    }
  };
  const forGame = () => {
    const g = games.find((x) => x.key === game);
    if (!g || !chosen || !quick) return;
    const id = `playres-${g.key.replace(/[^a-z0-9]/gi, '').slice(0, 40)}`;
    // hz 0: keep your refresh rate (the quickest switch), else the highest at that size.
    const scene: Scene = { id, name: `${chosen.name} · ${g.name}`.slice(0, 40), icon: 'monitor', color: '#38bdf8', steps: [{ kind: 'screen', width: chosen.width, height: chosen.height, hz: 0 }], game: g.key };
    const others = quick.scenes.filter((s) => s.id !== id && s.id !== `stretch-${g.key.replace(/[^a-z0-9]/gi, '').slice(0, 40)}`);
    void save([...others, scene]).then(() => toast.success(`${g.name} will play at ${chosen.name}`, 'The PC switches when the game starts and comes back when it closes.'));
  };
  const now = async () => {
    if (!chosen) return;
    setBusy(true);
    try {
      const m = await api.screenSetResolution(chosen.width, chosen.height);
      setCurrent(m);
      setChanged(true);
      toast.success(`The PC is at ${chosen.name}`, `${m.width}×${m.height} at ${m.hz} Hz. “Back to normal” undoes it.`);
    } catch (e) {
      toast.error('Not switched', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const back = async () => {
    setBusy(true);
    try {
      const m = await api.screenRestore();
      setCurrent(m);
      setChanged(false);
    } catch (e) {
      toast.error('Couldn’t switch back', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const named = res.filter((r) => !r.name.includes('×'));
  const others = res.filter((r) => r.name.includes('×'));
  return (
    <Card className="p-5">
      <CardHeader
        icon={Maximize2}
        title="Play resolution"
        subtitle="Switch the whole PC to 4K, 1440p (2K), 1080p or a stretched 4:3 mode, so games in windowed fullscreen run at it and alt-tab stays instant. Per game, or right now."
        actions={current ? <span className="tabular text-[12.5px] text-dim">Now {current.width}×{current.height} · {current.hz} Hz</span> : undefined}
      />
      <div className="mt-4 flex flex-wrap gap-2">
        {named.map((r) => {
          const k = `${r.width}x${r.height}`;
          const on = pick === k;
          const isNow = current?.width === r.width && current?.height === r.height;
          return (
            <button key={k} type="button" onClick={() => setPick(k)} aria-pressed={on} className={cx('rounded-xl border px-3 py-2 text-left transition-colors', on ? 'border-accent bg-accent-soft' : 'border-line hover:border-line-strong')}>
              <span className="flex items-center gap-1.5 text-[13.5px] font-semibold text-fg">
                {r.name}
                {isNow && <Check size={13} className="text-good" aria-label="current" />}
              </span>
              <span className="tabular block text-[11.5px] text-dim">
                {r.width}×{r.height} · {aspect(r)} · up to {r.hz} Hz
              </span>
            </button>
          );
        })}
        {others.length > 0 && (
          <Select value={named.some((r) => `${r.width}x${r.height}` === pick) ? '' : pick} onChange={(e) => setPick(e.target.value)} className="w-44 self-center" aria-label="Other resolutions">
            <option value="">Other…</option>
            {others.map((r) => (
              <option key={`${r.width}x${r.height}`} value={`${r.width}x${r.height}`}>
                {r.width}×{r.height} ({aspect(r)})
              </option>
            ))}
          </Select>
        )}
      </div>
      <div className="mt-4 grid gap-3 sm:grid-cols-[1fr_auto_auto] sm:items-end">
        <Field label="For a game (switches when it starts, back when it closes)">
          <Select value={game} onChange={(e) => setGame(e.target.value)}>
            <option value="">Pick a game</option>
            {games.map((g) => (
              <option key={g.key} value={g.key}>
                {g.name}
              </option>
            ))}
          </Select>
        </Field>
        <Button variant="primary" disabled={!game || !chosen} onClick={forGame}>
          Use with this game
        </Button>
        <div className="flex gap-2">
          <Button variant="secondary" icon={Zap} disabled={!chosen || busy} loading={busy && !changed} onClick={() => void now()}>
            Switch now
          </Button>
          <Button variant="ghost" icon={RotateCcw} disabled={busy || !changed} onClick={() => void back()}>
            Back to normal
          </Button>
        </div>
      </div>
      <p className="mt-3 text-[12.5px] leading-relaxed text-dim">
        In the game, choose <b>windowed fullscreen</b> (borderless): it uses the PC’s resolution, so there’s nothing to change in the game and alt-tab is instant. OmniHub keeps your refresh rate when the screen offers it and asks the graphics card to scale, so the switch takes about a second without the monitor going black. For stretched 4:3, also set scaling to full screen in the driver (NVIDIA: Adjust desktop size and position → Full-screen, scaling on GPU; AMD: GPU Scaling on, Full panel). 4K on a lower-resolution monitor appears here once NVIDIA DSR or AMD VSR is on.
      </p>
      {mine.length > 0 && (
        <div className="mt-3 divide-y divide-line rounded-xl border border-line">
          {mine.map((s) => {
            const st = s.steps[0];
            return (
              <div key={s.id} className="flex items-center gap-3 px-3 py-2 text-[13px]">
                <span className="min-w-0 flex-1 truncate">{s.name.replace(/^Stretched · /, '').replace(/^[^·]+· /, '')}</span>
                {st?.kind === 'screen' && (
                  <span className="tabular text-dim">
                    {NAMES[`${st.width}x${st.height}`] ?? `${st.width}×${st.height}`}
                  </span>
                )}
                <IconButton icon={Trash2} label="Stop switching for this game" onClick={() => quick && void save(quick.scenes.filter((x) => x.id !== s.id))} />
              </div>
            );
          })}
        </div>
      )}
    </Card>
  );
}

/** Older name, same card. */
export const StretchedCard = PlayResolutionCard;

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
