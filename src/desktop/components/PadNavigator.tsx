// Drives the app from a gamepad while OmniHub has focus (see lib/padNav.ts).

import { useEffect } from 'react';
import { overlayOpen } from './ui/Overlay';
import { NAV } from './nav';
import { navigate, useRoute } from '../lib/router';
import { candidates, controllerModeOn, gamepadClaimed, nextInDirection } from '../lib/padNav';
import { usePalette } from '../state/dialogs';

const REPEAT_FIRST = 380;
const REPEAT_NEXT = 130;

function move(dx: number, dy: number) {
  const list = candidates();
  if (!list.length) return;
  const active = document.activeElement as HTMLElement | null;
  const current = active && active !== document.body && list.includes(active) ? active : null;
  const target = current ? nextInDirection(current.getBoundingClientRect(), list.filter((e) => e !== current).map((el) => ({ el, rect: el.getBoundingClientRect() })), dx, dy) : (list.find((e) => e.closest('main')) ?? list[0]);
  if (target) {
    target.focus({ preventScroll: true });
    target.scrollIntoView({ block: 'nearest', inline: 'nearest', behavior: 'smooth' });
  } else if (dy) {
    // Nothing further that way: scroll the page.
    document.querySelector('main [class*="overflow-y-auto"], main')?.scrollBy({ top: dy * 240, behavior: 'smooth' });
  }
}

function press() {
  const el = document.activeElement as HTMLElement | null;
  if (!el || el === document.body) return move(0, 1);
  if (el instanceof HTMLInputElement && (el.type === 'text' || el.type === 'search' || el.type === 'password' || el.type === 'number')) return el.select();
  el.click();
}

function back() {
  if (overlayOpen()) {
    (document.activeElement ?? document.body).dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  } else {
    window.history.back();
  }
}

export function PadNavigator() {
  const route = useRoute();
  useEffect(() => {
    let frame = 0;
    let on = controllerModeOn();
    const held: Record<string, number> = {};
    const was: Record<string, boolean> = {};
    const onMode = () => (on = controllerModeOn());
    const offMouse = () => document.documentElement.classList.remove('pad-mode');
    window.addEventListener('omnihub:controller-mode', onMode);
    window.addEventListener('mousemove', offMouse);
    const loop = (t: number) => {
      frame = requestAnimationFrame(loop);
      if (!on || !document.hasFocus() || gamepadClaimed()) return;
      for (const pad of navigator.getGamepads?.() ?? []) {
        if (!pad) continue;
        const b = (i: number) => !!pad.buttons[i]?.pressed;
        const ax = pad.axes[0] ?? 0;
        const ay = pad.axes[1] ?? 0;
        const dirs: [string, boolean, number, number][] = [
          ['up', b(12) || ay < -0.6, 0, -1],
          ['down', b(13) || ay > 0.6, 0, 1],
          ['left', b(14) || ax < -0.6, -1, 0],
          ['right', b(15) || ax > 0.6, 1, 0],
        ];
        for (const [name, down, dx, dy] of dirs) {
          if (!down) {
            delete held[name];
            continue;
          }
          const next = held[name];
          if (next === undefined || t >= next) {
            held[name] = t + (next === undefined ? REPEAT_FIRST : REPEAT_NEXT);
            document.documentElement.classList.add('pad-mode');
            move(dx, dy);
          }
        }
        const edge = (i: number, f: () => void) => {
          const key = `${pad.index}:${i}`;
          const now = b(i);
          if (now && !was[key]) {
            document.documentElement.classList.add('pad-mode');
            f();
          }
          was[key] = now;
        };
        edge(0, press);
        edge(1, back);
        edge(9, () => usePalette.getState().setOpen(true));
        const page = (d: number) => {
          const ids = NAV.map((n) => n.id);
          const at = ids.indexOf(route.id as (typeof ids)[number]);
          navigate(ids[(at + d + ids.length) % ids.length]);
        };
        edge(4, () => page(-1));
        edge(5, () => page(1));
      }
    };
    frame = requestAnimationFrame(loop);
    return () => {
      cancelAnimationFrame(frame);
      window.removeEventListener('omnihub:controller-mode', onMode);
      window.removeEventListener('mousemove', offMouse);
    };
  }, [route.id]);
  return null;
}
