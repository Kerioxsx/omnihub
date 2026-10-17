// The My PC page's data: settings, live status (held while the page shows),
// specs, lighting profiles and the screen media library.

import type { DeepPartial, LightProfile, RigMedia, RigSettings, RigStatus, Specs } from '@shared/types';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText, on } from '../../api';
import { toast } from '../../state/toasts';

export interface Rig {
  settings: RigSettings | null;
  status: RigStatus | null;
  specs: Specs | null;
  profiles: LightProfile[];
  media: RigMedia[];
  update: (patch: DeepPartial<RigSettings>) => Promise<void>;
  reloadProfiles: () => void;
  reloadMedia: () => void;
  redetect: () => Promise<void>;
}

export function useRig(): Rig {
  const [settings, setSettings] = useState<RigSettings | null>(null);
  const [status, setStatus] = useState<RigStatus | null>(null);
  const [specs, setSpecs] = useState<Specs | null>(null);
  const [profiles, setProfiles] = useState<LightProfile[]>([]);
  const [media, setMedia] = useState<RigMedia[]>([]);

  const reloadProfiles = useCallback(() => void api.rig.profiles().then(setProfiles, () => undefined), []);
  const reloadMedia = useCallback(() => void api.rig.media().then(setMedia, () => undefined), []);

  useEffect(() => {
    let alive = true;
    void api.rig.settings().then((s) => alive && setSettings(s), () => undefined);
    void api.rig.specs().then((s) => alive && setSpecs(s), () => undefined);
    reloadProfiles();
    reloadMedia();
    const hold = () => void api.rig.hold().then((s) => alive && setStatus(s), () => undefined);
    hold();
    const timer = setInterval(hold, 4000);
    const offStatus = on<RigStatus>('rig:status', (s) => alive && setStatus(s));
    const offSettings = on<RigSettings>('rig:settings', (s) => alive && setSettings(s));
    return () => {
      alive = false;
      clearInterval(timer);
      void offStatus.then((u) => u());
      void offSettings.then((u) => u());
    };
  }, [reloadProfiles, reloadMedia]);

  const update = useCallback(async (patch: DeepPartial<RigSettings>) => {
    try {
      setSettings(await api.rig.update(patch));
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  }, []);

  const redetect = useCallback(async () => {
    try {
      setSpecs(await api.rig.specs(true));
    } catch (e) {
      toast.error('Could not read the PC’s parts', errorText(e));
    }
  }, []);

  return { settings, status, specs, profiles, media, update, reloadProfiles, reloadMedia, redetect };
}

/** The profile in use, or the one being edited. */
export function activeProfile(rig: Rig): LightProfile | null {
  const id = rig.settings?.lighting.profile;
  return rig.profiles.find((p) => p.id === id) ?? rig.profiles[0] ?? null;
}

/** Average fan speed, for the 3D fans. */
export function averageRpm(status: RigStatus | null): number | null {
  const fans = status?.rpm.filter(([n]) => !/pump/i.test(n)).map(([, r]) => r) ?? [];
  return fans.length ? fans.reduce((a, b) => a + b, 0) / fans.length : null;
}
