// Aurora's settings: read from the settings store, changed instantly on
// screen and saved a moment later (sliders send many changes a second).

import type { DeepPartial, VisualSettings } from '@shared/types';
import { api, errorText } from '../../api';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';
import { mergeDeep } from '../util';
import { DEFAULT_VISUALS } from './defaults';

let pending: DeepPartial<VisualSettings> | null = null;
let timer: ReturnType<typeof setTimeout> | undefined;

export function useVisuals(): VisualSettings {
  return useSettings((s) => s.settings?.visuals) ?? DEFAULT_VISUALS;
}

/** Apply now, save shortly (merged with other changes made meanwhile). */
export function patchVisuals(patch: DeepPartial<VisualSettings>): void {
  const st = useSettings.getState();
  if (st.settings) st.set(mergeDeep(st.settings, { visuals: patch }));
  pending = mergeDeep(pending ?? {}, patch);
  clearTimeout(timer);
  timer = setTimeout(flush, 250);
}

function flush() {
  const send = pending;
  pending = null;
  if (!send) return;
  api.app.updateSettings({ visuals: send }).then(
    // Changes made while saving stay on screen.
    (next) => useSettings.getState().set(pending ? mergeDeep(next, { visuals: pending }) : next),
    (e: unknown) => toast.error('Could not save the Aurora settings', errorText(e)),
  );
}

/** Everything back to how it came, except which view is shown. */
export function resetVisuals(): void {
  const view = useSettings.getState().settings?.visuals.view ?? 'aurora';
  // Arrays and nulls replace wholesale in a merge patch, so this resets every value.
  patchVisuals({ ...structuredClone(DEFAULT_VISUALS), view });
}
