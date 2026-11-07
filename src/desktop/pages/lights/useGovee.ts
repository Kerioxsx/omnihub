// The Lights page's data: settings, live status (held while the page shows),
// presets and the monitors screen sync can follow.

import type { DeepPartial, GoveeLightSettings, GoveePreset, GoveeSettings, GoveeStatus, MonitorInfo } from '@shared/types';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText, on } from '../../api';
import { everyWhileVisible } from '../../lib/hooks';
import { toast } from '../../state/toasts';

export interface Govee {
  settings: GoveeSettings | null;
  status: GoveeStatus | null;
  presets: GoveePreset[];
  monitors: MonitorInfo[];
  update: (patch: DeepPartial<GoveeSettings>) => Promise<void>;
  updateLight: (id: string, patch: DeepPartial<GoveeLightSettings>) => Promise<void>;
  reloadPresets: () => void;
  setStatus: (s: GoveeStatus) => void;
}

export function useGovee(): Govee {
  const [settings, setSettings] = useState<GoveeSettings | null>(null);
  const [status, setStatus] = useState<GoveeStatus | null>(null);
  const [presets, setPresets] = useState<GoveePreset[]>([]);
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const reloadPresets = useCallback(() => void api.govee.presets().then(setPresets, () => undefined), []);

  useEffect(() => {
    let alive = true;
    void api.govee.settings().then((s) => alive && setSettings(s), () => undefined);
    void api.govee.monitors().then((m) => alive && setMonitors(m), () => undefined);
    reloadPresets();
    const hold = () => void api.govee.hold().then((s) => alive && setStatus(s), () => undefined);
    hold();
    const stop = everyWhileVisible(hold, 4000);
    const offStatus = on<GoveeStatus>('govee:status', (s) => alive && setStatus(s));
    const offSettings = on<GoveeSettings>('govee:settings', (s) => alive && setSettings({ ...s, apiSecret: '' }));
    return () => {
      alive = false;
      stop();
      void offStatus.then((u) => u());
      void offSettings.then((u) => u());
    };
  }, [reloadPresets]);

  const update = useCallback(async (patch: DeepPartial<GoveeSettings>) => {
    try {
      setSettings(await api.govee.update(patch));
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  }, []);

  const updateLight = useCallback(async (id: string, patch: DeepPartial<GoveeLightSettings>) => {
    try {
      setSettings(await api.govee.updateLight(id, patch));
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  }, []);

  return { settings, status, presets, monitors, update, updateLight, reloadPresets, setStatus };
}

/** A light's own settings (or the defaults for one never changed). */
export function lightSettings(s: GoveeSettings, id: string): GoveeLightSettings {
  return s.lights.find((l) => l.id === id) ?? { id, name: '', control: true, follow: true, look: s.look, on: true, brightness: 100, segments: 0, stream: 'auto', zone: 'whole', reverse: false, gradient: true };
}
