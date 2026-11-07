// The button studio: your controller in 3D, live (buttons light up, sticks
// tilt, triggers pull), and what each button does in OmniHub's controller
// screens. Press a button on the controller, or click it on the model, to
// pick it; left / right changes what it does; Start finishes. The model's
// colours and the PlayStation light bar can be changed too.

import { RotateCcw, X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { cx } from '../../../lib/cx';
import { claimGamepad } from '../../../lib/padNav';
import { BUTTON_NAMES, PadScene, buttonLabel } from '../../../lib/padScene';
import { ACTIONS, ACTION_LABEL, DEFAULT_PREFS, type PadFamily, accentHex, actionFor, setButtonAction, setPadPrefs, usePadPrefs, useConnectedPads } from '../../../lib/pads';
import { applyLight } from './PadPanel';

const FAMILIES: [PadFamily, string][] = [
  ['xbox', 'Xbox'],
  ['playstation', 'PlayStation'],
  ['nintendo', 'Nintendo'],
  ['generic', 'Other'],
];
const BODIES = ['#24242c', '#f2f2f5', '#0f172a', '#7f1d1d', '#1e3a8a', '#14532d', '#4c1d95', '#ff6500'];
const TRIMS = ['#3a3a46', '#d4d4d8', '#ff6500', '#e11d48', '#38bdf8', '#a3e635', '#c084fc', '#fbbf24'];
/** Buttons in the order the list shows them (the D-pad is navigation). */
const ORDER = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 16, 12, 13, 14, 15];

export function PadStudio({ padIndex, onClose }: { padIndex: number | null; onClose: () => void }) {
  const pads = useConnectedPads();
  const live = pads.find((p) => p.index === padIndex) ?? pads[0] ?? null;
  const [family, setFamily] = useState<PadFamily>(live?.family ?? 'xbox');
  const p = usePadPrefs();
  const [sel, setSel] = useState(0);
  const canvas = useRef<HTMLCanvasElement>(null);
  const scene = useRef<PadScene | null>(null);
  const lightColor = p.lightMatchTheme ? accentHex() : p.light;

  // The 3D model.
  useEffect(() => {
    const c = canvas.current;
    if (!c) return;
    const s = new PadScene(c);
    scene.current = s;
    s.onPick = (b) => setSel(ORDER.indexOf(b));
    let frame = 0;
    const tick = () => {
      const pad = live ? navigator.getGamepads?.()[live.index] ?? null : null;
      s.show(pad, accentHex());
      frame = requestAnimationFrame(tick);
    };
    frame = requestAnimationFrame(tick);
    return () => {
      cancelAnimationFrame(frame);
      s.dispose();
      scene.current = null;
    };
  }, [live?.index]);
  useEffect(() => {
    scene.current?.build(family, { ...p.model, light: lightColor });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [family, live?.index]);
  useEffect(() => {
    scene.current?.recolor({ ...p.model, light: lightColor });
  }, [p.model, lightColor]);
  useEffect(() => {
    scene.current?.select(ORDER[sel] ?? null);
  }, [sel]);

  const cycle = (dir: 1 | -1) => {
    const b = ORDER[sel];
    const cur = actionFor(b, family);
    const next = ACTIONS[(ACTIONS.indexOf(cur) + dir + ACTIONS.length) % ACTIONS.length];
    setButtonAction(family, b, next);
  };
  const ref = useRef({ cycle, onClose, family });
  ref.current = { cycle, onClose, family };

  // The controller drives the studio directly: any button picks itself,
  // the D-pad moves and changes, Start finishes.
  useEffect(() => {
    const release = claimGamepad();
    let frame = 0;
    const held = new Set<number>();
    let first = true;
    const loop = () => {
      const pad = live ? navigator.getGamepads?.()[live.index] : null;
      if (pad) {
        for (let i = 0; i < Math.min(17, pad.buttons.length); i++) {
          const down = pad.buttons[i]?.pressed;
          if (!down) {
            held.delete(i);
            continue;
          }
          if (held.has(i)) continue;
          held.add(i);
          if (first) continue;
          if (i === 9) ref.current.onClose();
          else if (i === 12) setSel((s) => Math.max(0, s - 1));
          else if (i === 13) setSel((s) => Math.min(ORDER.length - 1, s + 1));
          else if (i === 14) ref.current.cycle(-1);
          else if (i === 15) ref.current.cycle(1);
          else setSel(ORDER.indexOf(i));
        }
      }
      first = false;
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);
    const onKey = (e: KeyboardEvent) => {
      const map: Record<string, () => void> = {
        ArrowUp: () => setSel((s) => Math.max(0, s - 1)),
        ArrowDown: () => setSel((s) => Math.min(ORDER.length - 1, s + 1)),
        ArrowLeft: () => ref.current.cycle(-1),
        ArrowRight: () => ref.current.cycle(1),
        Escape: () => ref.current.onClose(),
      };
      const f = map[e.key];
      if (f) {
        e.preventDefault();
        e.stopImmediatePropagation();
        f();
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener('keydown', onKey, true);
      release();
    };
  }, [live?.index]);

  return (
    <div className="bp-in absolute inset-0 z-40 flex bg-[#08080d]/96 backdrop-blur-md" role="dialog" aria-label="Button studio">
      <div className="relative min-w-0 flex-1">
        <canvas ref={canvas} className="h-full w-full cursor-grab active:cursor-grabbing" aria-label="Your controller in 3D" />
        <div className="absolute left-8 top-7">
          <h2 className="font-display text-[28px] font-bold">Button studio</h2>
          <p className="text-[14px] text-white/60">{live ? live.name : 'No controller connected: you can still set things up'}</p>
        </div>
        <div className="absolute left-8 top-[110px] flex gap-1.5">
          {FAMILIES.map(([f, l]) => (
            <button key={f} type="button" onClick={() => setFamily(f)} className={cx('rounded-full px-3.5 py-1.5 text-[13px] font-semibold transition', f === family ? 'bg-accent text-white' : 'bg-white/10 text-white/70 hover:bg-white/15')}>
              {l}
            </button>
          ))}
        </div>
        <p className="absolute bottom-6 left-8 max-w-[560px] text-[13px] text-white/50">Press a button on your controller (or click it on the model) to pick it. ◀ ▶ changes what it does. Start finishes. Drag to turn the model. These buttons are for OmniHub’s controller screens; games keep their own controls.</p>
      </div>

      <div className="flex w-[min(460px,44%)] flex-col border-l border-white/10 bg-black/40">
        <div className="flex items-center justify-between px-7 pb-3 pt-7">
          <span className="text-[13px] font-semibold uppercase tracking-[0.12em] text-white/50">Buttons</span>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setPadPrefs({ remap: { ...p.remap, [family]: {} } })}
              className="flex items-center gap-1.5 rounded-full bg-white/10 px-3 py-1.5 text-[12.5px] hover:bg-white/15"
            >
              <RotateCcw size={13} /> Reset
            </button>
            <button type="button" onClick={onClose} aria-label="Done" className="grid h-9 w-9 place-items-center rounded-full bg-white/10 hover:bg-white/20">
              <X size={18} />
            </button>
          </div>
        </div>
        <div className="min-h-0 flex-1 space-y-1 overflow-y-auto px-5 pb-4">
          {ORDER.map((b, i) => {
            const act = actionFor(b, family);
            const changed = p.remap[family]?.[b] !== undefined;
            return (
              <button
                key={b}
                type="button"
                onClick={() => setSel(i)}
                ref={(el) => {
                  if (el && i === sel) el.scrollIntoView({ block: 'nearest' });
                }}
                className={cx('flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition', i === sel ? 'bg-white/12 ring-2 ring-accent' : 'hover:bg-white/[0.06]')}
              >
                <span className="grid h-8 min-w-8 place-items-center rounded-full bg-white/15 px-2 text-[13px] font-bold">{buttonLabel(family, b)}</span>
                <span className="min-w-0 flex-1 truncate text-[13.5px] text-white/70">{BUTTON_NAMES[b]}</span>
                <span className="flex items-center gap-1.5 text-[13.5px] font-semibold">
                  {i === sel && (
                    <span
                      role="button"
                      tabIndex={-1}
                      onClick={(e) => {
                        e.stopPropagation();
                        cycle(-1);
                      }}
                      className="px-1 text-white/50 hover:text-white"
                    >
                      ◀
                    </span>
                  )}
                  <span className={cx(changed ? 'text-accent' : act === 'none' ? 'text-white/35' : 'text-white')}>{ACTION_LABEL[act]}</span>
                  {i === sel && (
                    <span
                      role="button"
                      tabIndex={-1}
                      onClick={(e) => {
                        e.stopPropagation();
                        cycle(1);
                      }}
                      className="px-1 text-white/50 hover:text-white"
                    >
                      ▶
                    </span>
                  )}
                </span>
              </button>
            );
          })}
        </div>
        <div className="space-y-3 border-t border-white/10 px-7 py-5">
          <Swatches label="Model colour" colors={BODIES} value={p.model.body} onPick={(body) => setPadPrefs({ model: { ...p.model, body } })} />
          <Swatches label="Buttons and sticks" colors={TRIMS} value={p.model.trim} onPick={(trim) => setPadPrefs({ model: { ...p.model, trim } })} />
          {family === 'playstation' && (
            <Swatches
              label="Light bar (your real controller too)"
              colors={['#ff6a00', '#ff1e3c', '#1e64ff', '#8b3dff', '#14d27a', '#9ff3ff', '#ffffff']}
              value={p.lightMatchTheme ? '' : p.light}
              onPick={(light) => {
                setPadPrefs({ lightMatchTheme: false, light });
                void applyLight(false);
              }}
            />
          )}
          <button type="button" onClick={() => setPadPrefs({ model: DEFAULT_PREFS.model })} className="text-[12px] text-white/45 hover:text-white/80">
            Default colours
          </button>
        </div>
      </div>
    </div>
  );
}

function Swatches({ label, colors, value, onPick }: { label: string; colors: string[]; value: string; onPick: (c: string) => void }) {
  return (
    <div>
      <div className="mb-1.5 text-[12.5px] text-white/60">{label}</div>
      <div className="flex flex-wrap gap-1.5">
        {colors.map((c) => (
          <button key={c} type="button" title={c} onClick={() => onPick(c)} className={cx('h-7 w-7 rounded-full border-2 transition', value.toLowerCase() === c ? 'scale-110 border-white' : 'border-white/20 hover:border-white/50')} style={{ background: c }} />
        ))}
      </div>
    </div>
  );
}
