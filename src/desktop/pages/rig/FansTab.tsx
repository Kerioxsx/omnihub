// Fans: Silent, Default (Auto) and Fast, through LibreHardwareMonitor
// (motherboard and graphics-card fans) and liquidctl (liquid coolers and
// fan hubs), with what each fan does now.

import type { FanProfile, HeatSettings, QuietStatus } from '@shared/types';
import { ExternalLink, Fan, Gauge, Moon, ShieldCheck, Snowflake, Sun, Thermometer, Wind } from 'lucide-react';
import { useEffect, useState } from 'react';
import type { LucideIcon } from 'lucide-react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Switch, TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useEvent } from '../../lib/hooks';
import { toast } from '../../state/toasts';
import { Row } from '../games/shared';
import type { Rig } from './useRig';

const PROFILES: { value: FanProfile; title: string; icon: LucideIcon; blurb: string }[] = [
  { value: 'lowest', title: 'Lowest', icon: Moon, blurb: 'The quietest safe speed. The graphics card runs its own (it often stops its fans when cool).' },
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
        <div role="radiogroup" aria-label="Fan profile" className="grid grid-cols-2 gap-2 px-5 pb-4 pt-3">
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

      <QuietCard />
      <HeatCard />

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

/** Quiet mode: screens off and fans at their lowest, the PC still on. */
function minsLeft(at: number | null): number | null {
  return at == null ? null : Math.max(0, Math.ceil((at - Date.now()) / 60_000));
}

function QuietCard() {
  const [q, setQ] = useState<QuietStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [, tick] = useState(0);
  useEffect(() => {
    void api.quiet.status().then(setQ, () => undefined);
    const t = setInterval(() => tick((n) => n + 1), 20_000);
    return () => clearInterval(t);
  }, []);
  useEvent<QuietStatus>('quiet:changed', setQ);
  const run = async (f: () => Promise<QuietStatus>) => {
    setBusy(true);
    try {
      setQ(await f());
    } catch (e) {
      toast.error('Couldn’t do that', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const left = minsLeft(q?.timer ?? null);
  const sched = q?.schedule ?? { enabled: false, start: '23:00', end: '07:30' };
  return (
    <Card className="overflow-hidden">
      <div className="flex flex-wrap items-center gap-4 p-5">
        <span className="grid h-11 w-11 place-items-center rounded-xl bg-indigo-500/15 text-indigo-300">
          <Moon size={20} />
        </span>
        <div className="min-w-[220px] flex-1">
          <div className="font-medium">Quiet mode</div>
          <div className="text-[12.5px] text-dim">
            Screens off and fans at their lowest safe speed; the PC stays on. Fans never stop completely (the processor would overheat) and speed up if it gets warm. Ends when you touch the mouse or keyboard, or from your phone’s Power page.
          </div>
        </div>
        <Button loading={busy} variant={q?.on ? 'secondary' : 'primary'} icon={q?.on ? Sun : Moon} onClick={() => void run(() => api.quiet.set(!q?.on))}>
          {q?.on ? 'Turn off' : 'Quiet now'}
        </Button>
      </div>
      <div className="divide-y divide-line border-t border-line">
        <Row title="Sleep timer" hint={left != null ? `Quiet mode in ${left} min.` : 'Quiet mode after a while, for falling asleep to a video or music.'}>
          <div className="flex flex-wrap items-center gap-1.5">
            {[15, 30, 60, 90].map((m) => (
              <Button key={m} size="sm" variant="secondary" disabled={busy} onClick={() => void run(() => api.quiet.setTimer(m))}>
                {m < 60 ? `${m} min` : `${m / 60} h`.replace('1.5 h', '1½ h')}
              </Button>
            ))}
            {left != null && (
              <Button size="sm" variant="ghost" disabled={busy} onClick={() => void run(() => api.quiet.setTimer(null))}>
                Cancel
              </Button>
            )}
          </div>
        </Row>
        <Row title="Every night" hint="Quiet at the first time, back to normal at the second (unless you ended it yourself).">
          <div className="flex items-center gap-2">
            <TextInput type="time" aria-label="Quiet from" value={sched.start} className="w-[136px]" onChange={(e) => void run(() => api.quiet.setSchedule({ ...sched, start: e.target.value }))} />
            <span className="text-faint">to</span>
            <TextInput type="time" aria-label="Quiet until" value={sched.end} className="w-[136px]" onChange={(e) => void run(() => api.quiet.setSchedule({ ...sched, end: e.target.value }))} />
            <Switch label="Quiet every night" checked={sched.enabled} onChange={(enabled) => void run(() => api.quiet.setSchedule({ ...sched, enabled }))} />
          </div>
        </Row>
      </div>
    </Card>
  );
}

function HeatCard() {
  const [h, setH] = useState<HeatSettings | null>(null);
  useEffect(() => {
    void api.heat.get().then(setH, () => undefined);
  }, []);
  const save = async (next: HeatSettings) => {
    setH(next);
    try {
      setH(await api.heat.set(next));
    } catch (e) {
      toast.error('Couldn’t save that', errorText(e));
    }
  };
  if (!h) return null;
  return (
    <Card className="overflow-hidden">
      <CardHeader icon={Thermometer} title="Overheat alerts" subtitle="A notification here and on your phone when a part stays above its limit for 20 seconds (again at most every 10 minutes). Uses the same temperature readings as the fans." className="px-5 py-4" actions={<Switch label="Overheat alerts" checked={h.enabled} onChange={(enabled) => void save({ ...h, enabled })} />} />
      {h.enabled && (
        <div className="grid gap-4 px-5 pb-5 sm:grid-cols-2">
          {([['cpu', 'Processor', 60, 105], ['gpu', 'Graphics card', 60, 100]] as const).map(([k, label, min, max]) => (
            <label key={k} className="block">
              <div className="mb-1.5 flex items-center justify-between text-[12.5px]">
                <span className="text-dim">{label}</span>
                <span className="font-semibold tabular-nums">{h[k]} °C</span>
              </div>
              <input type="range" min={min} max={max} step={1} value={h[k]} onChange={(e) => setH({ ...h, [k]: Number(e.target.value) })} onPointerUp={() => void save(h)} onKeyUp={() => void save(h)} className="w-full accent-[var(--accent)]" aria-label={`${label} limit`} />
            </label>
          ))}
        </div>
      )}
    </Card>
  );
}
