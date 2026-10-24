// Lights: every Govee light found, live. What they show (a colour, a white,
// an effect, the screen, the music with the album's colours and beats, or
// a light's own scene), presets, each light's own settings, timers, setup.

import type { GoveePreset } from '@shared/types';
import { AlarmClock, Lightbulb, Moon, Plus, Power, Settings2, Sparkles, Trash2, Wand2 } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Switch, Tabs } from '../../components/ui/Form';
import { Callout, EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { navigate, useRoute } from '../../lib/router';
import { toast } from '../../state/toasts';
import { Slider } from '../rig/LightingTab';
import { LightCard } from './LightCard';
import { LiveStrip } from './LiveStrip';
import { LookEditor } from './LookEditor';
import { SetupTab } from './SetupTab';
import { TimersTab } from './TimersTab';
import { useGovee } from './useGovee';

type Tab = 'show' | 'lights' | 'timers' | 'setup';
const TABS: Tab[] = ['show', 'lights', 'timers', 'setup'];

function PresetChip({ p, active, onApply, onDelete }: { p: GoveePreset; active: boolean; onApply: () => void; onDelete?: () => void }) {
  const swatch = p.look.mode === 'color' ? [p.look.color] : p.look.mode === 'white' ? ['#fff1dc'] : p.look.mode === 'screen' ? ['#1d4ed8', '#7c3aed', '#f97316'] : p.look.mode === 'music' ? ['#ff2bd6', '#7c3aed', '#00d4ff'] : p.look.colors.length ? p.look.colors : ['#ff0000', '#00ff00', '#0000ff'];
  return (
    <div className={cx('group relative flex items-center gap-2 rounded-xl border px-3 py-2 transition-colors', active ? 'border-accent/60 bg-accent-soft' : 'border-line bg-surface-2/50 hover:border-line-strong')}>
      <button type="button" onClick={onApply} className="flex items-center gap-2 text-left">
        <span className="h-5 w-5 shrink-0 rounded-full ring-1 ring-white/15" style={{ background: swatch.length > 1 ? `conic-gradient(${swatch.join(',')},${swatch[0]})` : swatch[0] }} />
        <span className="text-[13px] font-medium text-fg">{p.name}</span>
      </button>
      {onDelete && <IconButton icon={Trash2} label={`Remove ${p.name}`} size="sm" className="opacity-0 group-hover:opacity-100" onClick={onDelete} />}
    </div>
  );
}

export function LightsPage() {
  const route = useRoute();
  const tab = (TABS.find((t) => t === route.params.get('tab')) ?? 'show') as Tab;
  const govee = useGovee();
  const s = govee.settings;
  const st = govee.status;
  const [naming, setNaming] = useState<string | null>(null);
  const lights = st?.lights ?? [];
  const live = s && ['screen', 'music', 'effect'].includes(s.look.mode);

  const apply = async (id: string) => {
    try {
      await api.govee.applyPreset(id);
    } catch (e) {
      toast.error('Could not apply it', errorText(e));
    }
  };
  const savePreset = async () => {
    if (!s || !naming?.trim()) return;
    try {
      await api.govee.savePreset({ id: '', name: naming.trim(), look: s.look, brightness: s.brightness });
      setNaming(null);
      govee.reloadPresets();
      toast.saved();
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  };

  return (
    <Page title="Lights" subtitle="Your Govee lights: colours, the screen and the music, live.">
      <div className="space-y-4">
        <Card className="overflow-hidden">
          <div className="flex flex-wrap items-center gap-4 px-5 py-4">
            <div className="grid h-11 w-11 place-items-center rounded-2xl bg-accent-soft text-accent">
              <Lightbulb size={22} />
            </div>
            <div className="min-w-0 flex-1">
              <div className="flex flex-wrap items-center gap-2 text-[15px] font-semibold text-fg">
                {s?.enabled ? 'OmniHub controls your lights' : 'OmniHub isn’t controlling your lights'}
                {st?.sleepUntil && (
                  <Badge tone="info" icon={Moon}>
                    Sleep timer
                  </Badge>
                )}
              </div>
              <div className="text-[12.5px] text-faint">{lights.length ? `${lights.length} light${lights.length === 1 ? '' : 's'} · ${lights.filter((l) => l.lan).length} on your network${st?.hasKey ? ` · ${lights.filter((l) => l.cloud).length} in your Govee account` : ''}` : 'Looking for Govee lights on your network…'}</div>
            </div>
            {s && (
              <div className="flex items-center gap-5">
                <label className="flex items-center gap-2 text-[12.5px] text-dim">
                  <Power size={14} /> All on
                  <Switch checked={s.on} onChange={(v) => void govee.update({ on: v, enabled: true })} label="All lights on" disabled={!s.enabled} />
                </label>
                <Switch size="lg" checked={s.enabled} onChange={(v) => void govee.update({ enabled: v })} label="OmniHub controls the lights" />
              </div>
            )}
          </div>
          {s && (
            <div className="grid gap-4 border-t border-line px-5 py-4 md:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)] md:items-center">
              <Slider label="Brightness (all lights)" value={s.brightness} min={1} max={100} step={1} onChange={(v) => void govee.update({ brightness: v })} format={(v) => `${v}%`} />
              <div className="space-y-1.5">
                {lights.slice(0, 6).map((l) => (
                  <div key={l.id} className="flex items-center gap-3">
                    <span className="w-28 truncate text-[11.5px] text-faint">{l.name}</span>
                    <LiveStrip id={l.id} fallback={l.color ? [l.color] : undefined} className="flex-1" />
                  </div>
                ))}
              </div>
            </div>
          )}
          {s && !s.enabled && lights.length > 0 && (
            <div className="border-t border-line px-5 py-3">
              <Callout tone="info" title="Your lights are as you left them">
                Switch on control (top right) and pick what they show; switch it off and they’re left alone again.
              </Callout>
            </div>
          )}
        </Card>

        <Tabs
          value={tab}
          onChange={(t) => navigate('lights', { tab: t })}
          items={[
            { value: 'show', label: 'What they show', icon: Sparkles },
            { value: 'lights', label: `Lights${lights.length ? ` (${lights.length})` : ''}`, icon: Lightbulb },
            { value: 'timers', label: 'Timers', icon: AlarmClock },
            { value: 'setup', label: 'Setup', icon: Settings2 },
          ]}
        />

        {tab === 'show' && s && (
          <div className="grid gap-4 xl:grid-cols-[minmax(0,1.5fr)_minmax(0,1fr)]">
            <Card>
              <CardHeader title="All lights" subtitle={live ? 'Live: OmniHub sends the colours up to 30 times a second, eased so changes glide.' : 'Lights that follow all lights show this.'} className="px-5 pt-4" />
              <div className="px-5 pb-5 pt-3">
                <LookEditor look={s.look} onChange={(p) => void govee.update({ look: p, enabled: true, preset: '' })} lights={lights} monitors={govee.monitors} palette={st?.palette ?? []} />
                {st?.screenMessage && s.look.mode === 'screen' && <div className="mt-3 text-[12px] text-warn">{st.screenMessage}</div>}
                {st?.musicMessage && s.look.mode === 'music' && <div className="mt-3 text-[12px] text-warn">{st.musicMessage}</div>}
              </div>
            </Card>
            <Card>
              <CardHeader
                icon={Wand2}
                title="Presets"
                subtitle="One click for all lights. Yours are kept with OmniHub."
                className="px-5 pt-4"
                actions={
                  naming == null && (
                    <Button size="sm" variant="secondary" icon={Plus} onClick={() => setNaming('')}>
                      Save current
                    </Button>
                  )
                }
              />
              <div className="space-y-3 px-5 pb-5 pt-3">
                {naming != null && (
                  <div className="flex gap-2">
                    <input autoFocus value={naming} onChange={(e) => setNaming(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && void savePreset()} placeholder="Name it (Movie night…)" className="h-9 flex-1 rounded-lg border border-line bg-surface px-3 text-[13px] text-fg outline-none focus:border-accent/60" />
                    <Button size="sm" variant="primary" disabled={!naming.trim()} onClick={() => void savePreset()}>
                      Save
                    </Button>
                    <Button size="sm" variant="ghost" onClick={() => setNaming(null)}>
                      Cancel
                    </Button>
                  </div>
                )}
                <div className="flex flex-wrap gap-2">
                  {govee.presets.map((p) => (
                    <PresetChip
                      key={p.id}
                      p={p}
                      active={s.preset === p.id}
                      onApply={() => void apply(p.id)}
                      onDelete={p.id.startsWith('my-') ? () => void api.govee.deletePreset(p.id).then(govee.reloadPresets, (e: unknown) => toast.error('Not removed', errorText(e))) : undefined}
                    />
                  ))}
                </div>
              </div>
            </Card>
          </div>
        )}

        {tab === 'lights' &&
          (lights.length === 0 ? (
            <Card>
              <EmptyState
                icon={Lightbulb}
                title="No Govee lights found yet"
                description="Switch on LAN Control for each light in the Govee Home app (the light → settings → LAN Control), on the same Wi-Fi as this PC. Lights in your Govee account appear too once you add your API key (Setup)."
                action={
                  <Button variant="secondary" onClick={() => navigate('lights', { tab: 'setup' })}>
                    Setup
                  </Button>
                }
              />
            </Card>
          ) : (
            <div className="grid gap-4 lg:grid-cols-2">
              {lights.map((l) => (
                <LightCard key={l.id} light={l} govee={govee} monitors={govee.monitors} />
              ))}
            </div>
          ))}

        {tab === 'timers' && <TimersTab govee={govee} />}
        {tab === 'setup' && <SetupTab govee={govee} />}
      </div>
    </Page>
  );
}
