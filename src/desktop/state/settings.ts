import { create } from 'zustand';
import type { AppInfoDetails, DeepPartial, Settings } from '@shared/types';
import { api, errorText } from '../api';
import { mergeDeep } from '../lib/util';
import { toast } from './toasts';

interface SettingsState {
  settings: Settings | null;
  info: AppInfoDetails | null;
  error: string | null;
  load: () => Promise<void>;
  /** Optimistically apply a patch, persist it, and show a subtle "Saved" toast. */
  update: (patch: DeepPartial<Settings>, opts?: { silent?: boolean }) => Promise<boolean>;
  set: (s: Settings) => void;
}

export const useSettings = create<SettingsState>((set, get) => ({
  settings: null,
  info: null,
  error: null,
  load: async () => {
    try {
      const [settings, info] = await Promise.all([api.app.settings(), api.app.info()]);
      set({ settings, info, error: null });
    } catch (e) {
      set({ error: errorText(e) });
    }
  },
  update: async (patch, opts) => {
    const before = get().settings;
    if (before) set({ settings: mergeDeep(before, patch) });
    try {
      const next = await api.app.updateSettings(patch);
      set({ settings: next });
      if (!opts?.silent) toast.saved();
      return true;
    } catch (e) {
      if (before) set({ settings: before });
      toast.error('Could not save the setting', errorText(e));
      return false;
    }
  },
  set: (settings) => set({ settings }),
}));

/** Selector helper: the loaded settings (components render only after load). */
export function useLoadedSettings(): Settings {
  const s = useSettings((st) => st.settings);
  if (!s) throw new Error('settings not loaded');
  return s;
}
