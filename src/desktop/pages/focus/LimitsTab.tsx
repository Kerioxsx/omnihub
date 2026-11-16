// Screen-time limits: minutes a day for an app or a website. Only what's in
// front counts. Five minutes before, a warning; at the limit the app closes
// (a website's browser window is minimized each time it comes back).

import type { LimitKind, LimitUse, ScreenLimit } from '@shared/types';
import { AppWindow, Globe, Hourglass, Plus, Trash } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Segmented, Switch, TextInput } from '../../components/ui/Form';
import { ProgressBar } from '../../components/ui/Progress';
import { EmptyState } from '../../components/ui/States';
import { useInterval } from '../../lib/hooks';
import { toast } from '../../state/toasts';

const mins = (m: number) => (m >= 60 ? `${Math.floor(m / 60)} h${m % 60 ? ` ${Math.round(m % 60)} min` : ''}` : `${Math.round(m)} min`);

export function LimitsTab() {
  const [list, setList] = useState<LimitUse[] | null>(null);
  const [kind, setKind] = useState<LimitKind>('app');
  const [target, setTarget] = useState('');
  const [minutes, setMinutes] = useState(60);
  const load = () => void api.screenTime.list().then(setList, () => undefined);
  useEffect(load, []);
  useInterval(load, 30_000);

  const save = async (l: ScreenLimit) => {
    try {
      setList(await api.screenTime.save(l));
      return true;
    } catch (e) {
      toast.error('Couldn’t save that limit', errorText(e));
      return false;
    }
  };
  const add = async () => {
    if (await save({ id: '', kind, target, minutes, enabled: true })) setTarget('');
  };

  return (
    <div className="space-y-4">
      <Card className="p-5">
        <CardHeader icon={Hourglass} title="Add a limit" subtitle="Minutes a day in front of an app or a website. The count starts again at midnight." />
        <div className="mt-4 flex flex-wrap items-center gap-2">
          <Segmented
            label="Kind"
            value={kind}
            onChange={setKind}
            options={[
              { value: 'app', label: 'App' },
              { value: 'site', label: 'Website' },
            ]}
          />
          <TextInput value={target} onChange={(e) => setTarget(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && void add()} placeholder={kind === 'app' ? 'Program, like TikTok or discord.exe' : 'A word in the page title, like YouTube'} className="min-w-[240px] flex-1" aria-label={kind === 'app' ? 'Program' : 'Website'} />
          <TextInput type="number" min={5} max={1440} step={5} value={minutes} onChange={(e) => setMinutes(Number(e.target.value))} className="w-24" aria-label="Minutes a day" />
          <span className="text-[12.5px] text-faint">min a day</span>
          <Button variant="primary" icon={Plus} disabled={!target.trim()} onClick={() => void add()}>
            Add
          </Button>
        </div>
        <p className="mt-3 text-[12px] text-faint">Websites are recognised by the browser window’s title (Chrome, Edge, Firefox, Brave, Opera, Vivaldi, Arc). OmniHub can’t close a single tab, so the window is minimized instead.</p>
      </Card>

      {list && list.length === 0 ? (
        <EmptyState icon={Hourglass} title="No limits yet" description="Add an app or a website above." />
      ) : (
        <Card className="divide-y divide-line">
          {list?.map(({ limit: l, usedMinutes, over }) => (
            <div key={l.id} className="flex flex-wrap items-center gap-4 px-5 py-3.5">
              <span className="grid h-9 w-9 place-items-center rounded-lg bg-surface-2 text-dim">{l.kind === 'app' ? <AppWindow size={17} /> : <Globe size={17} />}</span>
              <div className="min-w-[180px] flex-1">
                <div className="flex items-center gap-2 text-[13.5px] font-medium">
                  {l.kind === 'app' ? l.target.replace(/\.exe$/, '') : l.target}
                  {over && l.enabled && <Badge tone="bad">Time’s up</Badge>}
                </div>
                <div className="mt-1.5 flex items-center gap-3">
                  <ProgressBar value={Math.min(100, (usedMinutes / l.minutes) * 100)} tone={over ? 'bad' : usedMinutes >= l.minutes - 5 ? 'warn' : 'accent'} className="max-w-[260px] flex-1" label={`${l.target} used`} />
                  <span className="text-[12px] tabular-nums text-faint">
                    {mins(usedMinutes)} of {mins(l.minutes)}
                  </span>
                </div>
              </div>
              <TextInput type="number" min={5} max={1440} step={5} defaultValue={l.minutes} className="w-20" aria-label="Minutes a day" onBlur={(e) => Number(e.target.value) !== l.minutes && void save({ ...l, minutes: Number(e.target.value) })} />
              <Switch label={`Limit ${l.target}`} checked={l.enabled} onChange={(enabled) => void save({ ...l, enabled })} />
              <IconButton icon={Trash} label="Remove" size="sm" variant="danger" onClick={() => void api.screenTime.remove(l.id).then(setList)} />
            </div>
          ))}
        </Card>
      )}
    </div>
  );
}
