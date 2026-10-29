// My PC: the PC in 3D, its lighting, fans and screens, and what it is made of.

import type { LightProfile } from '@shared/types';
import { Box, Fan, Lightbulb, Monitor, MonitorSmartphone, Thermometer } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { rigMediaUrl } from '../../api';
import { Page } from '../../components/Page';
import { Badge } from '../../components/ui/Card';
import { Tabs } from '../../components/ui/Form';
import type { PartId } from '../../lib/rig/scene3d';
import { navigate, useRoute } from '../../lib/router';
import { BuildTab } from './BuildTab';
import { FansTab } from './FansTab';
import { LightingTab } from './LightingTab';
import { MonitorsTab } from './MonitorsTab';
import { PcView } from './PcView';
import { ScreensTab } from './ScreensTab';
import { activeProfile, averageRpm, useRig } from './useRig';

type Tab = 'lighting' | 'fans' | 'monitors' | 'screens' | 'build';
const TABS: Tab[] = ['lighting', 'fans', 'monitors', 'screens', 'build'];
const FAN_LABEL = { silent: 'Silent', auto: 'Default (Auto)', fast: 'Fast' } as const;

export function RigPage() {
  const route = useRoute();
  const tab = (TABS.find((t) => t === route.params.get('tab')) ?? 'lighting') as Tab;
  const rig = useRig();
  const [preview, setPreview] = useState<LightProfile | null>(null);
  const [hover, setHover] = useState<PartId | null>(null);
  const onPreview = useCallback((p: LightProfile | null) => setPreview(p), []);
  const s = rig.settings;
  const st = rig.status;
  const current = activeProfile(rig);
  const shown = preview ?? (s?.lighting.enabled === false ? null : current);
  const mediaId = s ? (s.screens.lcdShow.kind === 'media' ? s.screens.lcdShow.id : s.screens.monitorShow.kind === 'media' ? s.screens.monitorShow.id : null) : null;
  const m = rig.media.find((x) => x.id === mediaId);
  const media = useMemo(() => (m ? { url: rigMediaUrl(m.id), video: m.kind === 'video' } : null), [m]);
  const rpm = averageRpm(st);

  return (
    <Page title="My PC" subtitle="Your PC in 3D, with its lighting, fans, monitors and screens." scroll={false}>
      <div className="grid min-h-0 flex-1 gap-5 lg:grid-cols-[minmax(0,1.05fr)_minmax(380px,1fr)]">
        <div className="relative min-h-[320px] overflow-hidden rounded-2xl border border-line bg-[radial-gradient(ellipse_at_50%_35%,rgba(90,110,190,.16),transparent_65%),linear-gradient(180deg,#0a0b12,#05060a)]">
          <PcView className="absolute inset-0" specs={rig.specs} model={s?.model ?? null} profile={shown} rpm={st?.fans === 'on' ? rpm : null} temp={st?.cpuTemp ?? null} media={media} highlight={hover} />
          <div className="pointer-events-none absolute inset-x-3 top-3 flex flex-wrap gap-1.5">
            {s && (
              <Badge tone={s.lighting.enabled ? 'accent' : 'neutral'} icon={Lightbulb}>
                {preview ? `Preview: ${preview.name}` : s.lighting.enabled ? (current?.name ?? 'Lights') : 'Lights not controlled'}
              </Badge>
            )}
            {s && (
              <Badge tone="neutral" icon={Fan}>
                {FAN_LABEL[s.fans.profile]}
                {rpm ? ` · ${Math.round(rpm)} RPM` : ''}
              </Badge>
            )}
            {st?.cpuTemp != null && (
              <Badge tone={st.cpuTemp > 85 ? 'bad' : st.cpuTemp > 75 ? 'warn' : 'good'} icon={Thermometer}>
                CPU {Math.round(st.cpuTemp)} °C{st.gpuTemp != null ? ` · GPU ${Math.round(st.gpuTemp)} °C` : ''}
              </Badge>
            )}
          </div>
          <div className="pointer-events-none absolute bottom-3 left-4 text-[11.5px] text-white/40">Drag to turn · scroll to zoom</div>
        </div>
        <div className="flex min-h-0 flex-col">
          <Tabs
            className="mb-4 shrink-0"
            value={tab}
            onChange={(t) => navigate('rig', { tab: t })}
            items={[
              { value: 'lighting', label: 'Lighting', icon: Lightbulb },
              { value: 'fans', label: 'Fans', icon: Fan },
              { value: 'monitors', label: 'Monitors', icon: Monitor },
              { value: 'screens', label: 'Screens', icon: MonitorSmartphone },
              { value: 'build', label: 'Build', icon: Box },
            ]}
          />
          <div className="min-h-0 flex-1 overflow-y-auto pb-6 pr-1">
            {tab === 'lighting' && <LightingTab rig={rig} onPreview={onPreview} />}
            {tab === 'fans' && <FansTab rig={rig} />}
            {tab === 'monitors' && <MonitorsTab />}
            {tab === 'screens' && <ScreensTab rig={rig} />}
            {tab === 'build' && <BuildTab rig={rig} onHover={setHover} />}
          </div>
        </div>
      </div>
    </Page>
  );
}
