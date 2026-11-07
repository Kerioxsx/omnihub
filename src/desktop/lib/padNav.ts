// Controller mode: move around the whole app with a gamepad. D-pad or left
// stick moves between buttons and fields, A presses, B goes back (or closes
// a dialog), LB / RB switch pages, Start opens search. Pages with their own
// controller handling (the game library, a key picker listening) claim the
// gamepad and this stays out of the way.

const KEY = 'omnihub.controllerMode';

export function controllerModeOn(): boolean {
  try {
    return localStorage.getItem(KEY) !== 'off';
  } catch {
    return true;
  }
}

export function setControllerMode(on: boolean) {
  try {
    localStorage.setItem(KEY, on ? 'on' : 'off');
  } catch {
    /* private window: on for this session only */
  }
  window.dispatchEvent(new Event('omnihub:controller-mode'));
}

let claims = 0;

/** A page handles the gamepad itself while this is held; call the result to let go. */
export function claimGamepad(): () => void {
  claims += 1;
  let done = false;
  return () => {
    if (!done) {
      done = true;
      claims -= 1;
    }
  };
}

export const gamepadClaimed = () => claims > 0;

const FOCUSABLE = 'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

function visible(el: HTMLElement): boolean {
  const r = el.getBoundingClientRect();
  if (r.width < 2 || r.height < 2) return false;
  if (r.bottom < 0 || r.top > window.innerHeight || r.right < 0 || r.left > window.innerWidth) return false;
  const st = getComputedStyle(el);
  return st.visibility !== 'hidden' && st.display !== 'none' && !el.closest('[aria-hidden="true"], [inert]');
}

/** Where focus can go now: inside the open dialog if there is one. */
export function candidates(): HTMLElement[] {
  const dialogs = [...document.querySelectorAll<HTMLElement>('[role="dialog"], [aria-modal="true"]')].filter(visible);
  const root: ParentNode = dialogs[dialogs.length - 1] ?? document;
  return [...root.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(visible);
}

/**
 * The best next element in a direction from `from`: ahead of it in that
 * direction, scoring distance along the direction plus twice the sideways
 * distance (so the move stays in its row or column).
 */
export function nextInDirection(from: DOMRect, list: { el: HTMLElement; rect: DOMRect }[], dx: number, dy: number): HTMLElement | null {
  const cx = from.left + from.width / 2;
  const cy = from.top + from.height / 2;
  let best: HTMLElement | null = null;
  let bestScore = Infinity;
  for (const { el, rect } of list) {
    const x = rect.left + rect.width / 2;
    const y = rect.top + rect.height / 2;
    const along = dx ? (x - cx) * dx : (y - cy) * dy;
    if (along <= 4) continue;
    const side = dx ? Math.abs(y - cy) : Math.abs(x - cx);
    const score = along + side * 2;
    if (score < bestScore) {
      bestScore = score;
      best = el;
    }
  }
  return best;
}
