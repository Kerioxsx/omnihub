// Controllers: which one is plugged in (Xbox, Elite, PlayStation 4/5, Edge,
// Switch Pro, Joy-Con, Switch 2, GameCube, Steam, anything else), its button
// symbols, the player's preferences (Nintendo layout, stick dead zone,
// vibration, PlayStation light colour) and one input hook the library and
// Big Picture share. Buttons and sticks come from the browser's gamepad
// support (XInput for Xbox pads; Sony and Nintendo pads through their own
// drivers in Chromium); anything it doesn't know falls back to the raw layout.

import { useEffect, useRef, useState, useSyncExternalStore } from 'react';
import { claimGamepad } from './padNav';

export type PadFamily = 'xbox' | 'playstation' | 'nintendo' | 'generic';
export type PadKind = 'xbox' | 'xbox360' | 'xboxElite' | 'ds4' | 'dualsense' | 'dualsenseEdge' | 'switchPro' | 'joyconL' | 'joyconR' | 'joyconPair' | 'switch2Pro' | 'joycon2' | 'gamecube' | 'nso' | 'steam' | 'generic';

export interface PadInfo {
  index: number;
  id: string;
  kind: PadKind;
  family: PadFamily;
  name: string;
  /** The browser lays its buttons out the standard way. */
  standard: boolean;
  /** The browser can make it vibrate. */
  rumble: boolean;
  /** Has a light bar OmniHub can colour. */
  light: boolean;
}

const KNOWN: Record<string, [PadKind, string]> = {
  '054c:05c4': ['ds4', 'DualShock 4'],
  '054c:09cc': ['ds4', 'DualShock 4'],
  '054c:0ba0': ['ds4', 'DualShock 4 (wireless adapter)'],
  '054c:0ce6': ['dualsense', 'DualSense'],
  '054c:0df2': ['dualsenseEdge', 'DualSense Edge'],
  '045e:028e': ['xbox360', 'Xbox 360 Controller'],
  '045e:02e3': ['xboxElite', 'Xbox Elite Controller'],
  '045e:0b00': ['xboxElite', 'Xbox Elite Series 2'],
  '045e:0b05': ['xboxElite', 'Xbox Elite Series 2'],
  '045e:0b22': ['xboxElite', 'Xbox Elite Series 2'],
  '057e:2006': ['joyconL', 'Joy-Con (L)'],
  '057e:2007': ['joyconR', 'Joy-Con (R)'],
  '057e:2008': ['joyconPair', 'Joy-Con (L+R)'],
  '057e:200e': ['joyconPair', 'Joy-Con charging grip'],
  '057e:2009': ['switchPro', 'Switch Pro Controller'],
  '057e:2017': ['nso', 'SNES Controller (Switch Online)'],
  '057e:2019': ['nso', 'N64 Controller (Switch Online)'],
  '057e:2066': ['joycon2', 'Joy-Con 2'],
  '057e:2067': ['joycon2', 'Joy-Con 2'],
  '057e:2069': ['switch2Pro', 'Switch 2 Pro Controller'],
  '057e:2073': ['gamecube', 'GameCube Controller (Switch Online)'],
  '28de:1102': ['steam', 'Steam Controller'],
  '28de:1142': ['steam', 'Steam Controller'],
  '28de:1205': ['steam', 'Steam Deck'],
};

const VENDOR: Record<string, [PadFamily, string]> = {
  '054c': ['playstation', 'PlayStation controller'],
  '045e': ['xbox', 'Xbox controller'],
  '057e': ['nintendo', 'Nintendo controller'],
  '28de': ['generic', 'Steam controller'],
  '2dc8': ['generic', '8BitDo controller'],
  '0f0d': ['generic', 'HORI controller'],
  '20d6': ['generic', 'PowerA controller'],
  '0e6f': ['generic', 'PDP controller'],
  '046d': ['generic', 'Logitech controller'],
  '1532': ['generic', 'Razer controller'],
  '24c6': ['generic', 'PowerA controller'],
};

const FAMILY: Record<PadKind, PadFamily> = {
  xbox: 'xbox',
  xbox360: 'xbox',
  xboxElite: 'xbox',
  ds4: 'playstation',
  dualsense: 'playstation',
  dualsenseEdge: 'playstation',
  switchPro: 'nintendo',
  joyconL: 'nintendo',
  joyconR: 'nintendo',
  joyconPair: 'nintendo',
  switch2Pro: 'nintendo',
  joycon2: 'nintendo',
  gamecube: 'nintendo',
  nso: 'nintendo',
  steam: 'generic',
  generic: 'generic',
};

/** Vendor and product ("054c", "0ce6") from a gamepad id, in Chromium's or Firefox's format. */
export function vendorProduct(id: string): [string, string] | null {
  const c = id.match(/Vendor:\s*([0-9a-f]{4})\s*Product:\s*([0-9a-f]{4})/i);
  if (c) return [c[1].toLowerCase(), c[2].toLowerCase()];
  const f = id.match(/^([0-9a-f]{1,4})-([0-9a-f]{1,4})-/i);
  if (f) return [f[1].toLowerCase().padStart(4, '0'), f[2].toLowerCase().padStart(4, '0')];
  return null;
}

export function identify(p: Pick<Gamepad, 'index' | 'id' | 'mapping'> & { vibrationActuator?: unknown }): PadInfo {
  const vp = vendorProduct(p.id);
  const lower = p.id.toLowerCase();
  let kind: PadKind = 'generic';
  let name = p.id.replace(/\s*\(.*\)\s*$/, '').trim() || 'Controller';
  let family: PadFamily = 'generic';
  const known = vp && KNOWN[`${vp[0]}:${vp[1]}`];
  if (known) {
    [kind, name] = known;
    family = FAMILY[kind];
  } else if (lower.includes('xinput') || lower.includes('xbox')) {
    kind = lower.includes('elite') ? 'xboxElite' : 'xbox';
    family = 'xbox';
    if (lower.includes('xinput')) name = 'Xbox controller';
  } else if (lower.includes('dualsense')) {
    kind = 'dualsense';
    family = 'playstation';
  } else if (lower.includes('joy-con') || lower.includes('joycon')) {
    kind = lower.includes('l+r') ? 'joyconPair' : 'joyconL';
    family = 'nintendo';
  } else if (lower.includes('pro controller')) {
    kind = 'switchPro';
    family = 'nintendo';
  } else if (vp && VENDOR[vp[0]]) {
    [family, name] = VENDOR[vp[0]];
  }
  return { index: p.index, id: p.id, kind, family, name, standard: p.mapping === 'standard', rumble: !!p.vibrationActuator, light: family === 'playstation' };
}

/** The face buttons by position, and the other labels, for each family. */
export const GLYPHS: Record<PadFamily, { bottom: string; right: string; left: string; top: string; start: string; view: string; lb: string; rb: string; colors?: [string, string, string, string] }> = {
  xbox: { bottom: 'A', right: 'B', left: 'X', top: 'Y', start: '☰', view: '⧉', lb: 'LB', rb: 'RB', colors: ['#3fbf5f', '#e5484d', '#3b82f6', '#f5b400'] },
  playstation: { bottom: '✕', right: '○', left: '□', top: '△', start: 'Options', view: 'Create', lb: 'L1', rb: 'R1', colors: ['#8ab4ff', '#ff6b6b', '#f39ad8', '#3ddc97'] },
  nintendo: { bottom: 'B', right: 'A', left: 'Y', top: 'X', start: '+', view: '−', lb: 'L', rb: 'R' },
  generic: { bottom: '1', right: '2', left: '3', top: '4', start: 'Start', view: 'Select', lb: 'L1', rb: 'R1' },
};

// ---------- preferences ----------

export interface PadPrefs {
  /** Nintendo controllers confirm with A (the right button), like on a Switch. */
  nintendoConfirmRight: boolean;
  /** How far a stick must move to count (0–1). */
  deadzone: number;
  /** Vibration strength (0 = off). */
  vibration: number;
  /** PlayStation light bar colour. */
  light: string;
  /** Use the theme's colour for the light bar. */
  lightMatchTheme: boolean;
  /** Also colour controllers on Bluetooth (see pads.rs). */
  lightBluetooth: boolean;
  /** Soft sounds when moving in Big Picture. */
  sounds: boolean;
}

const PREFS_KEY = 'omnihub.pads';
export const DEFAULT_PREFS: PadPrefs = { nintendoConfirmRight: true, deadzone: 0.35, vibration: 0.6, light: '#ff6a00', lightMatchTheme: true, lightBluetooth: false, sounds: true };

let prefs: PadPrefs = (() => {
  try {
    return { ...DEFAULT_PREFS, ...JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}') };
  } catch {
    return DEFAULT_PREFS;
  }
})();
const listeners = new Set<() => void>();

export const padPrefs = () => prefs;

export function setPadPrefs(patch: Partial<PadPrefs>) {
  prefs = { ...prefs, ...patch };
  try {
    localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
  } catch {
    /* kept for this session */
  }
  listeners.forEach((l) => l());
}

export function usePadPrefs(): PadPrefs {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l);
      return () => listeners.delete(l);
    },
    () => prefs,
  );
}

/** The buttons (standard layout) that confirm and go back for this family. */
export function confirmBack(family: PadFamily, p: PadPrefs = prefs): { ok: number; back: number } {
  return family === 'nintendo' && p.nintendoConfirmRight ? { ok: 1, back: 0 } : { ok: 0, back: 1 };
}

// ---------- vibration ----------

type Actuator = { playEffect?: (type: string, params: Record<string, number>) => Promise<unknown> };

/** A short vibration; `level` 0–1 is scaled by the player's preference. */
export function rumble(index: number | null, level: number, ms: number) {
  const strength = prefs.vibration * level;
  if (index == null || strength <= 0) return;
  const pad = navigator.getGamepads?.()[index];
  const act = (pad as unknown as { vibrationActuator?: Actuator } | null)?.vibrationActuator;
  void act?.playEffect?.('dual-rumble', { startDelay: 0, duration: ms, strongMagnitude: Math.min(1, strength), weakMagnitude: Math.min(1, strength * 0.8) })?.catch(() => undefined);
}

// ---------- the connected controllers ----------

export function connectedPads(): PadInfo[] {
  return [...(navigator.getGamepads?.() ?? [])].filter((p): p is Gamepad => !!p && p.connected).map(identify);
}

export function useConnectedPads(): PadInfo[] {
  const [pads, setPads] = useState<PadInfo[]>(() => connectedPads());
  useEffect(() => {
    const update = () => setPads(connectedPads());
    window.addEventListener('gamepadconnected', update);
    window.addEventListener('gamepaddisconnected', update);
    const t = setInterval(update, 1500);
    return () => {
      window.removeEventListener('gamepadconnected', update);
      window.removeEventListener('gamepaddisconnected', update);
      clearInterval(t);
    };
  }, []);
  return pads;
}

// ---------- input ----------

/** A DirectInput hat on an axis (Chromium reports it on axes[9]): -1 up, clockwise, > 1 centred. */
export function hatDirection(v: number | undefined): [number, number] {
  if (v == null || v > 1.05 || v < -1.05) return [0, 0];
  const step = Math.round((v + 1) / (2 / 7));
  return ([[0, -1], [1, -1], [1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0], [-1, -1]] as [number, number][])[step] ?? [0, 0];
}

export interface PadHandlers {
  move: (dx: number, dy: number) => void;
  ok: () => void;
  back: () => void;
  start?: () => void;
  /** Top face button (Y / △ / X on Nintendo). */
  menu?: () => void;
  /** View / Create / − . */
  select?: () => void;
  /** Bumpers: -1 left, 1 right. */
  bumper?: (dir: -1 | 1) => void;
}

/**
 * One loop for the gamepads: D-pad or left stick moves (repeating while
 * held), the confirm / back buttons follow the Nintendo-layout preference,
 * and the last controller used is reported (for button symbols).
 */
export function usePadInput(active: boolean, handlers: PadHandlers, opts: { haptics?: boolean } = {}): PadInfo | null {
  const ref = useRef(handlers);
  ref.current = handlers;
  const [last, setLast] = useState<PadInfo | null>(null);
  const lastId = useRef<string>('');
  useEffect(() => {
    if (!active) return;
    const release = claimGamepad();
    let frame = 0;
    let held: Record<string, number> = {};
    // Buttons already down when this view opened (the press that opened it)
    // count only after they're let go.
    let first = true;
    const loop = (t: number) => {
      const p = prefs;
      for (const pad of navigator.getGamepads?.() ?? []) {
        if (!pad || !pad.connected) continue;
        const info = identify(pad);
        const b = (i: number) => !!pad.buttons[i]?.pressed;
        const ax = pad.axes[0] ?? 0;
        const ay = pad.axes[1] ?? 0;
        const dz = Math.max(0.2, Math.min(0.8, p.deadzone)) + 0.15;
        const [hx, hy] = info.standard ? [0, 0] : hatDirection(pad.axes[9]);
        const { ok, back } = confirmBack(info.family, p);
        const acts: [string, boolean, () => void][] = [
          ['l', b(14) || ax < -dz || hx < 0, () => ref.current.move(-1, 0)],
          ['r', b(15) || ax > dz || hx > 0, () => ref.current.move(1, 0)],
          ['u', b(12) || ay < -dz || hy < 0, () => ref.current.move(0, -1)],
          ['d', b(13) || ay > dz || hy > 0, () => ref.current.move(0, 1)],
          ['a', b(ok), () => ref.current.ok()],
          ['b', b(back), () => ref.current.back()],
          ['s', b(9), () => ref.current.start?.()],
          ['y', b(3), () => ref.current.menu?.()],
          ['v', b(8), () => ref.current.select?.()],
          ['lb', b(4), () => ref.current.bumper?.(-1)],
          ['rb', b(5), () => ref.current.bumper?.(1)],
        ];
        for (const [k0, on, act] of acts) {
          const k = `${pad.index}:${k0}`;
          if (!on) {
            delete held[k];
            continue;
          }
          if (first) {
            held[k] = Infinity;
            continue;
          }
          if (lastId.current !== `${pad.index}|${pad.id}`) {
            lastId.current = `${pad.index}|${pad.id}`;
            setLast(info);
          }
          if (held[k] === undefined) {
            held[k] = t + 380;
            act();
            if (opts.haptics) rumble(pad.index, 'lrud'.includes(k0) ? 0.25 : 0.55, 'lrud'.includes(k0) ? 30 : 60);
          } else if ('lrud'.includes(k0) && t > held[k]) {
            held[k] = t + 110;
            act();
          }
        }
      }
      first = false;
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);
    return () => {
      cancelAnimationFrame(frame);
      held = {};
      release();
    };
  }, [active, opts.haptics]);
  return last;
}

/** The theme's accent colour as "#rrggbb" (for the light bar). */
export function accentHex(): string {
  const v = getComputedStyle(document.documentElement).getPropertyValue('--accent').trim();
  if (/^#[0-9a-f]{6}$/i.test(v)) return v;
  if (/^#[0-9a-f]{3}$/i.test(v)) return `#${[...v.slice(1)].map((c) => c + c).join('')}`;
  const m = v.match(/(\d+)[,\s]+(\d+)[,\s]+(\d+)/);
  return m ? `#${[m[1], m[2], m[3]].map((n) => Number(n).toString(16).padStart(2, '0')).join('')}` : DEFAULT_PREFS.light;
}
