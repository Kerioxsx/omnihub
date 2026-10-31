// Fans: Silent, Default (Auto) and Fast, through LibreHardwareMonitor
// (motherboard and graphics-card fans) and liquidctl (liquid coolers and
// fan hubs), with what each fan does now.

import type { FanProfile } from '@shared/types';
import { ExternalLink, Fan, Gauge, ShieldCheck, Snowflake, Wind } from 'lucide-react';
import { useState } from 'react';
import type { LucideIcon } from 'lucide-react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Switch, TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { toast } from '../../state/toasts';
import { Row } from '../games/shared';
import type { Rig } from './useRig';

const PROFILES: { value: FanProfile; title: string; icon: LucideIcon; blurb: string }[] = [
  { value: 'silent', title: 'Silent', icon: Snowflake, blurb: 'As quiet as the temperatures allow; full speed when it gets hot.' },
  { value: 'auto', title: 'Default (Auto)', icon: Gauge, blurb: 'Your motherboard, graphics card and cooler decide, as without OmniHub.' },
  { value: 'fast', title: 'Fast', icon: Wind, blurb: 'Cooler and louder: more air early, for gaming and heavy work.' },
];

const temp = (t: number | null) => (t == null ? '—' : `${Math.round(t)} °C`);

export function FansTab({ rig }: { rig: Rig }) {
  const s = rig.settings;
  const st = rig.status;
  const [user, setUser] = useState('');
  const [password, setPassword] = useState('');
  if (!s) return null;
  const rpmOf = (name: string) => st?.rpm.find(([n]) => n === name)?.[1];

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader title="Fan profile" subtitle="Every fan OmniHub can reach follows it, and gets handed back to the hardware when OmniHub closes." className="px-5 pt-4" />
        <div role="radiogroup" aria-label="Fan profile" className="grid gap-2 px-5 pb-4 pt-3 sm:grid-cols-3">
          {PROFILES.map((p) => {
            const on = s.fans.profile === p.value;
            const Icon = p.icon;
            return (
              <button key={p.value} type="button" role="radio" aria-checked={on} onClick={() => void rig.update({ fans: { profile: p.value } })} className={cx('rounded-xl border p-3 text-left transition-colors', on ? 'border-accent/60 bg-accent-soft' : 'border-line bg-surface-2/50 hover:border-line-strong hover:bg-surface-2')}>
                <div className="mb-1 flex items-center gap-2 text-[14px] font-semibold text-fg">
                  <Icon size={16} className={on ? 'text-accent' : 'text-dim'} /> {p.title}
                </div>
                <div className="text-[12.5px] leading-snug text-faint">{p.blurb}</div>
              </button>
            );
          })}
        </div>
        <div className="flex items-start gap-2 border-t border-line px-5 py-3 text-[12px] text-faint">
          <ShieldCheck size={14} className="mt-px shrink-0 text-good" />
          No fan goes under 30% (pumps 50%), everything runs flat out from 90 °C, and a fan with no temperature to follow goes back to Auto.
        </div>
      </Card>

      <Card>
        <CardHeader icon={Fan} title="Motherboard and graphics card" subtitle={st?.fans === 'on' ? `Processor ${temp(st.cpuTemp)} · graphics card ${temp(st.gpuTemp)}` : 'Through LibreHardwareMonitor'} className="px-5 pt-4" />
        {st?.fans === 'on' ? (
          <div className="divide-y divide-line">
            {st.controls.map((c) => (
              <Row
                key={c.id}
                title={
                  <span className="flex items-center gap-2">
                    {c.name}
                    {c.role === 'pump' && <Badge tone="info">pump</Badge>}
                  </span>
                }
                hint={`${c.hardware} · ${c.percent == null ? '—' : `${Math.round(c.percent)}%`}${rpmOf(c.name) ? ` · ${Math.round(rpmOf(c.name) ?? 0)} RPM` : ''}`}
              >
                <Switch checked={!c.skipped} onChange={(v) => void rig.update({ fans: { skip: v ? s.fans.skip.filter((k) => k !== c.id) : [...s.fans.skip, c.id] } })} label={`Let OmniHub control ${c.name}`} />
              </Row>
            ))}
            {st.fansMessage && <div className="px-5 py-3 text-[12.5px] text-warn">{st.fansMessage}</div>}
          </div>
        ) : (
          <div className="p-4">
            <Callout
              tone={s.fans.profile === 'auto' ? 'info' : 'warn'}
              title="Open LibreHardwareMonitor with its web server on"
              action={
                <Button size="sm" variant="secondary" icon={ExternalLink} onClick={() => void api.app.openUrl('https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/releases')}>
                  Get it
                </Button>
              }
            >
              It reads the fans and temperatures with its own signed driver; OmniHub loads none. Run it as administrator, then Options → Remote Web Server → Run (port 8085), and keep it in the tray. Untick a fan here if it is a pump plugged into a fan header.
              {st?.fansMessage && <div className="mt-1 text-faint">{st.fansMessage}</div>}
            </Callout>
          </div>
        )}
        <div className="flex flex-wrap gap-2 border-t border-line px-5 py-3 [&>*:first-child]:min-w-[220px] [&>*:first-child]:flex-1">
          <TextInput key={s.fans.lhmUrl} defaultValue={s.fans.lhmUrl} onBlur={(e) => e.target.value !== s.fans.lhmUrl && void rig.update({ fans: { lhmUrl: e.target.value } })} aria-label="LibreHardwareMonitor's address" className="font-mono text-[12.5px]" />
          <div className="flex gap-2">
            <TextInput value={user} onChange={(e) => setUser(e.target.value)} placeholder={s.fans.lhmUser || 'User (if set)'} aria-label="User name" className="w-32" />
            <TextInput type="password" value={password} onChange={(e) => setPassword(e.target.value)} placeholder="Password" aria-label="Password" className="w-32" />
            <Button
              variant="secondary"
              disabled={!user && !password}
              onClick={() =>
                void api.rig.lhmLogin(user, password).then(
                  () => {
                    setPassword('');
                    toast.saved();
                  },
                  (e: unknown) => toast.error('Not saved', errorText(e)),
                )
              }
            >
              Save
            </Button>
          </div>
        </div>
      </Card>

      <Card>
        <CardHeader title="Liquid coolers and fan hubs" subtitle="NZXT Kraken and Smart Device, Corsair, ASUS Ryujin, Lian Li… through liquidctl. They keep running the profile on their own." className="px-5 pt-4" actions={<Switch checked={s.fans.coolers} onChange={(v) => void rig.update({ fans: { coolers: v } })} label="Use liquidctl" />} />
        {s.fans.coolers && st?.coolers === 'on' && (
          <div className="divide-y divide-line">
            {st.coolerDevices.length === 0 && <div className="px-5 py-3 text-[12.5px] text-faint">liquidctl found no cooler or hub.</div>}
            {st.coolerDevices.map((d) => (
              <Row key={d.address} title={d.description} hint={d.status.length ? d.status.map((r) => `${r.key}: ${Math.round(r.value * 10) / 10}${r.unit === '°C' ? ' °C' : ` ${r.unit}`}`).join(' · ') : 'Readings appear in a few seconds.'} />
            ))}
            {st.coolersMessage && <div className="px-5 py-3 text-[12.5px] text-warn">{st.coolersMessage}</div>}
          </div>
        )}
        {s.fans.coolers && st?.coolers === 'missing' && (
          <div className="p-4">
            <Callout tone="info" title="Install liquidctl" action={<Button size="sm" variant="secondary" icon={ExternalLink} onClick={() => void api.app.openUrl('https://github.com/liquidctl/liquidctl#installing-on-windows')}>How</Button>}>
              With Python installed, run <code className="rounded bg-surface-3 px-1">pip install liquidctl</code>. Close the cooler’s own app (NZXT CAM, iCUE) so the two don’t fight.
            </Callout>
          </div>
        )}
      </Card>
    </div>
  );
}
