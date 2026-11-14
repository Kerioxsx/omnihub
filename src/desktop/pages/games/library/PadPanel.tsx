// Big Picture's controller panel: the controllers connected (with their own
// button symbols and a vibration test), the Nintendo layout, stick dead
// zone, vibration strength, PlayStation light colour and a live button test.
// Driven by the controller itself: up/down picks a row, left/right changes
// it, confirm presses, back closes.

import type { LightResult } from '@shared/types';
import { Gamepad2, X } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../../api';
import { cx } from '../../../lib/cx';
import { DEFAULT_PREFS, GLYPHS, type PadInfo, accentHex, padPrefs, rumble, setPadPrefs, usePadInput, usePadPrefs, useConnectedPads } from '../../../lib/pads';
import { toast } from '../../../state/toasts';
import { PadStudio } from './PadStudio';

const LIGHTS: [string, string][] = [
  ['theme', 'Theme'],
  ['#ff6a00', 'Orange'],
  ['#ff1e3c', 'Red'],
  ['#1e64ff', 'Blue'],
  ['#8b3dff', 'Purple'],
  ['#14d27a', 'Green'],
  ['#9ff3ff', 'Ice'],
  ['#ffffff', 'White'],
  ['#000000', 'Off'],
];
const VIBES: [number, string][] = [
  [0, 'Off'],
  [0.35, 'Light'],
  [0.6, 'Medium'],
  [1, 'Strong'],
];

/** Colour the PlayStation controllers' light bars with the current preference. */
export async function applyLight(quiet = true): Promise<LightResult | null> {
  const p = padPrefs();
  try {
    const r = await api.padsSetLight(p.lightMatchTheme ? accentHex() : p.light, p.lightBluetooth);
    if (!quiet && r.skippedBluetooth && !r.set) toast.info('Your PlayStation controller is on Bluetooth', 'Plug it in with a cable, or turn on “Also over Bluetooth” below.');
    return r;
  } catch (e) {
    if (!quiet) toast.error('Couldn’t change the light', errorText(e));
    return null;
  }
}

export function Glyph({ family, which, size = 28 }: { family: PadInfo['family']; which: 'bottom' | 'right' | 'left' | 'top'; size?: number }) {
  const g = GLYPHS[family];
  const i = ['bottom', 'right', 'left', 'top'].indexOf(which);
  const color = g.colors?.[i];
  return (
    <span className="grid shrink-0 place-items-center rounded-full bg-white/15 font-bold" style={{ width: size, height: size, fontSize: size * 0.48, color: color ?? '#fff', boxShadow: color ? `inset 0 0 0 2px ${color}55` : undefined }}>
      {g[which]}
    </span>
  );
}

function FaceDiamond({ family, pressed }: { family: PadInfo['family']; pressed: boolean[] }) {
  const cell = (which: 'bottom' | 'right' | 'left' | 'top', idx: number, cls: string) => (
    <div className={cx('absolute transition-transform', cls, pressed[idx] && 'scale-125 drop-shadow-[0_0_10px_var(--accent-glow)]')}>
      <Glyph family={family} which={which} size={30} />
    </div>
  );
  return (
    <div className="relative h-[92px] w-[92px] shrink-0">
      {cell('top', 3, 'left-[31px] top-0')}
      {cell('left', 2, 'left-0 top-[31px]')}
      {cell('right', 1, 'right-0 top-[31px]')}
      {cell('bottom', 0, 'bottom-0 left-[31px]')}
    </div>
  );
}

/** Which buttons the controller is holding down right now. */
function useLiveButtons(index: number | null): boolean[] {
  const [pressed, setPressed] = useState<boolean[]>([]);
  useEffect(() => {
    if (index == null) return;
    let frame = 0;
    let prev = '';
    const loop = () => {
      const pad = navigator.getGamepads?.()[index];
      const now = pad ? pad.buttons.map((b) => b.pressed) : [];
      const key = now.map(Number).join('');
      if (key !== prev) {
        prev = key;
        setPressed(now);
      }
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
  }, [index]);
  return pressed;
}

type Row = { id: string; label: string; hint?: string; render: (focused: boolean) => React.ReactNode; left?: () => void; right?: () => void; ok?: () => void };

export function PadPanel({ onClose, lastPad }: { onClose: () => void; lastPad: PadInfo | null }) {
  const pads = useConnectedPads();
  const p = usePadPrefs();
  const [row, setRow] = useState(0);
  const [studio, setStudio] = useState(false);
  const tester = lastPad?.index ?? pads[0]?.index ?? null;
  const pressed = useLiveButtons(tester);
  const testerPad = pads.find((x) => x.index === tester) ?? null;
  const anyPs = pads.some((x) => x.family === 'playstation');
  const anyNintendo = pads.some((x) => x.family === 'nintendo');

  const lightKey = p.lightMatchTheme ? 'theme' : p.light.toLowerCase();
  const lightAt = Math.max(0, LIGHTS.findIndex(([k]) => k === lightKey));
  const setLight = (i: number) => {
    const [k] = LIGHTS[(i + LIGHTS.length) % LIGHTS.length];
    setPadPrefs(k === 'theme' ? { lightMatchTheme: true } : { lightMatchTheme: false, light: k });
    void applyLight(false);
  };
  const vibAt = VIBES.reduce((best, [v], i) => (Math.abs(v - p.vibration) < Math.abs(VIBES[best][0] - p.vibration) ? i : best), 0);
  const setVib = (i: number) => {
    const v = VIBES[Math.max(0, Math.min(VIBES.length - 1, i))][0];
    setPadPrefs({ vibration: v });
    setTimeout(() => rumble(tester, 1, 220), 30);
  };
  const setDz = (d: number) => setPadPrefs({ deadzone: Math.round(Math.max(0.15, Math.min(0.6, p.deadzone + d)) * 100) / 100 });

  const rows: Row[] = useMemo(() => {
    const out: Row[] = [];
    out.push({
      id: 'studio',
      label: 'Button studio (3D)',
      hint: 'Your controller in 3D: see every button and stick live, and change what each button does.',
      ok: () => setStudio(true),
      right: () => setStudio(true),
      render: () => <span className="rounded-full bg-accent px-3 py-1 text-[13px] font-semibold text-white">Open</span>,
    });
    out.push({
      id: 'test',
      label: 'Test vibration',
      hint: testerPad?.rumble === false ? 'This controller doesn’t report vibration to Windows apps.' : 'A short buzz on the controller you’re holding.',
      ok: () => rumble(tester, 1, 400),
      render: () => <span className="text-white/70">Press to feel it</span>,
    });
    out.push({
      id: 'vib',
      label: 'Vibration',
      left: () => setVib(vibAt - 1),
      right: () => setVib(vibAt + 1),
      render: () => (
        <span className="flex gap-1.5">
          {VIBES.map(([, l], i) => (
            <span key={l} className={cx('rounded-full px-3 py-1 text-[14px]', i === vibAt ? 'bg-accent text-white' : 'bg-white/10 text-white/70')}>
              {l}
            </span>
          ))}
        </span>
      ),
    });
    out.push({
      id: 'dz',
      label: 'Stick dead zone',
      hint: 'Raise it if the selection drifts by itself.',
      left: () => setDz(-0.05),
      right: () => setDz(0.05),
      render: () => (
        <span className="flex items-center gap-3">
          <span className="h-2 w-40 overflow-hidden rounded-full bg-white/10">
            <span className="block h-full rounded-full bg-accent" style={{ width: `${((p.deadzone - 0.15) / 0.45) * 100}%` }} />
          </span>
          <span className="w-10 text-right tabular-nums text-white/80">{Math.round(p.deadzone * 100)}%</span>
        </span>
      ),
    });
    out.push({
      id: 'nin',
      label: 'Nintendo layout',
      hint: 'On Nintendo controllers, A (right) confirms and B (bottom) goes back, like on a Switch.',
      ok: () => setPadPrefs({ nintendoConfirmRight: !p.nintendoConfirmRight }),
      left: () => setPadPrefs({ nintendoConfirmRight: !p.nintendoConfirmRight }),
      right: () => setPadPrefs({ nintendoConfirmRight: !p.nintendoConfirmRight }),
      render: () => <Toggle on={p.nintendoConfirmRight} />,
    });
    out.push({
      id: 'light',
      label: 'PlayStation light',
      hint: anyPs ? 'The light bar of DualShock 4, DualSense and DualSense Edge.' : 'For DualShock 4, DualSense and DualSense Edge (none connected).',
      left: () => setLight(lightAt - 1),
      right: () => setLight(lightAt + 1),
      ok: () => void applyLight(false),
      render: () => (
        <span className="flex gap-1.5">
          {LIGHTS.map(([k, l], i) => (
            <span key={k} title={l} className={cx('grid h-7 w-7 place-items-center rounded-full border-2 text-[10px] font-bold', i === lightAt ? 'scale-110 border-white' : 'border-white/20')} style={{ background: k === 'theme' ? 'var(--accent)' : k }}>
              {k === 'theme' ? 'T' : k === '#000000' ? '×' : ''}
            </span>
          ))}
        </span>
      ),
    });
    out.push({
      id: 'lightbt',
      label: 'Light also over Bluetooth',
      hint: 'Over Bluetooth this puts the controller in the mode Steam uses: some older games that read it directly won’t see it until you turn it off and on.',
      ok: () => {
        setPadPrefs({ lightBluetooth: !p.lightBluetooth });
        if (!p.lightBluetooth) void applyLight(false);
      },
      left: () => setPadPrefs({ lightBluetooth: !p.lightBluetooth }),
      right: () => setPadPrefs({ lightBluetooth: !p.lightBluetooth }),
      render: () => <Toggle on={p.lightBluetooth} />,
    });
    out.push({
      id: 'sounds',
      label: 'Navigation sounds',
      ok: () => setPadPrefs({ sounds: !p.sounds }),
      left: () => setPadPrefs({ sounds: !p.sounds }),
      right: () => setPadPrefs({ sounds: !p.sounds }),
      render: () => <Toggle on={p.sounds} />,
    });
    out.push({ id: 'reset', label: 'Reset controller settings', ok: () => setPadPrefs(DEFAULT_PREFS), render: () => <span className="text-white/60">Back to the defaults</span> });
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [p, tester, testerPad?.rumble, anyPs, lightAt, vibAt]);

  usePadInput(!studio, {
    move: (dx, dy) => {
      if (dy) setRow((r) => Math.max(0, Math.min(rows.length - 1, r + dy)));
      else if (dx < 0) rows[row]?.left?.();
      else rows[row]?.right?.();
    },
    ok: () => rows[row]?.ok?.(),
    back: onClose,
    menu: onClose,
    select: onClose,
  });
  useEffect(() => {
    if (studio) return;
    const onKey = (e: KeyboardEvent) => {
      const map: Record<string, () => void> = {
        ArrowUp: () => setRow((r) => Math.max(0, r - 1)),
        ArrowDown: () => setRow((r) => Math.min(rows.length - 1, r + 1)),
        ArrowLeft: () => rows[row]?.left?.(),
        ArrowRight: () => rows[row]?.right?.(),
        Enter: () => rows[row]?.ok?.(),
        Escape: onClose,
      };
      const f = map[e.key];
      if (f) {
        e.preventDefault();
        e.stopImmediatePropagation();
        f();
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [rows, row, onClose, studio]);

  return (
    <div className="bp-in absolute inset-0 z-30 flex justify-end bg-black/60 backdrop-blur-sm" role="dialog" aria-label="Controllers">
      <div className="bp-rise flex h-full w-[min(640px,100%)] flex-col overflow-y-auto border-l border-white/10 bg-[#0c0c12]/95 px-10 py-9">
        <div className="flex items-center justify-between">
          <h2 className="flex items-center gap-3 font-display text-[28px] font-bold">
            <Gamepad2 size={28} className="text-accent" /> Controllers
          </h2>
          <button type="button" onClick={onClose} aria-label="Close" className="grid h-10 w-10 place-items-center rounded-full bg-white/10 hover:bg-white/20">
            <X size={20} />
          </button>
        </div>

        <div className="mt-6 space-y-2">
          {pads.length === 0 ? (
            <p className="rounded-2xl bg-white/5 px-5 py-4 text-[15px] text-white/70">No controller found. Connect one with a cable or Bluetooth and press any button on it.</p>
          ) : (
            pads.map((pad) => (
              <div key={pad.index} className={cx('flex items-center gap-4 rounded-2xl px-5 py-4', pad.index === tester ? 'bg-accent/15 ring-1 ring-accent/50' : 'bg-white/5')}>
                <FaceDiamond family={pad.family} pressed={pad.index === tester ? pressed : []} />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[17px] font-semibold">{pad.name}</div>
                  <div className="text-[13px] text-white/60">
                    Player {pad.index + 1} · {pad.standard ? 'Standard layout' : 'Raw layout'}
                    {pad.rumble ? ' · Vibration' : ''}
                    {pad.light ? ' · Light bar' : ''}
                  </div>
                </div>
              </div>
            ))
          )}
        </div>

        <div className="mt-6 space-y-1.5">
          {rows.map((r, i) => (
            <button
              key={r.id}
              type="button"
              onClick={() => (setRow(i), r.ok?.())}
              onMouseMove={() => row !== i && setRow(i)}
              className={cx('flex w-full items-center gap-4 rounded-2xl px-5 py-3.5 text-left transition', i === row ? 'bg-white/12 ring-2 ring-accent' : 'bg-white/[0.04] hover:bg-white/[0.08]')}
            >
              <div className="min-w-0 flex-1">
                <div className="text-[16px] font-semibold">{r.label}</div>
                {r.hint && i === row && <div className="mt-0.5 text-[12.5px] text-white/60">{r.hint}</div>}
              </div>
              {r.render(i === row)}
            </button>
          ))}
        </div>

        <div className="mt-6 space-y-2 text-[12.5px] leading-relaxed text-white/50">
          {anyNintendo && <p>Switch 2 controllers: buttons and sticks work where Windows reports them; their HD rumble has no public way in for Windows apps, so OmniHub can’t drive it.</p>}
          <p>Elite controllers’ paddles work as whatever buttons you set in the Xbox Accessories app. If Steam is open, its controller settings may change how a controller appears here.</p>
        </div>
      </div>
      {studio && <PadStudio padIndex={tester} onClose={() => setStudio(false)} />}
    </div>
  );
}

function Toggle({ on }: { on: boolean }) {
  return (
    <span className={cx('relative inline-flex h-7 w-12 items-center rounded-full transition', on ? 'bg-accent' : 'bg-white/20')}>
      <span className={cx('absolute h-5 w-5 rounded-full bg-white transition-all', on ? 'left-6' : 'left-1')} />
    </span>
  );
}
