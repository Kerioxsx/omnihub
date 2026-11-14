// One server: start and stop it, how friends join, who's on, keeping it up
// around the clock, and its tabs (console, players, map, content, settings).

import type { SrvNet, SrvView } from '@shared/types';
import { ArrowLeft, Archive, Copy, FolderOpen, Gauge, Globe, Map as MapIcon, MessageSquare, Package, Play, Power, RotateCw, ScrollText, Settings2, Square, TriangleAlert, Users, Wifi } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Select, Switch, Tabs, TextInput } from '../../components/ui/Form';
import { ProgressBar } from '../../components/ui/Progress';
import { Callout } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { navigate, useRoute } from '../../lib/router';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { ConsoleTab } from './ConsoleTab';
import { ContentTab } from './ContentTab';
import { MapTab } from './MapTab';
import { PlayersTab } from './PlayersTab';
import { SettingsTab } from './SettingsTab';
import { ago, busy, flavorLabel, Head, ServerIcon, StateBadge, up } from './shared';

type Tab = 'overview' | 'console' | 'players' | 'map' | 'content' | 'settings';

export async function run(what: string, f: () => Promise<unknown>, ok?: string): Promise<boolean> {
  try {
    await f();
    if (ok) toast.success(ok);
    return true;
  } catch (e) {
    toast.error(what, errorText(e));
    return false;
  }
}

function copy(text: string) {
  void navigator.clipboard.writeText(text).then(() => toast.success('Copied', text));
}

function JoinCard({ s }: { s: SrvView }) {
  const [net, setNet] = useState<SrvNet | null>(null);
  useEffect(() => {
    void api.servers.network(s.id).then(setNet, () => setNet(null));
  }, [s.id, s.state, s.upnp]);
  const outside = net?.publicIp ? `${net.publicIp}:${net.port}` : null;
  const outsideState = !net ? null : net.cgnat ? 'cgnat' : net.portOpen ? 'open' : net.upnp ? 'trying' : 'closed';
  return (
    <Card>
      <CardHeader icon={Wifi} title="How friends join" subtitle="In Minecraft: Multiplayer → Add Server, then this address." className="px-5 pt-4" />
      <div className="space-y-2.5 px-5 pb-4 pt-3">
        <div className="flex items-center gap-3 rounded-xl border border-line px-3.5 py-2.5">
          <div className="min-w-0 flex-1">
            <div className="text-[11.5px] text-faint">On your home network (and on this PC: localhost:{s.port})</div>
            <div className="font-mono text-[14px] text-fg">{s.lan ?? `localhost:${s.port}`}</div>
          </div>
          <IconButton icon={Copy} label="Copy the address" onClick={() => copy(s.lan ?? `localhost:${s.port}`)} />
        </div>
        <div className="flex items-center gap-3 rounded-xl border border-line px-3.5 py-2.5">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-1.5 text-[11.5px] text-faint">
              Over the internet
              {outsideState === 'open' && <Badge tone="good">Port open on {net?.router ?? 'the router'}</Badge>}
              {outsideState === 'closed' && <Badge tone="neutral">Port not opened</Badge>}
              {outsideState === 'trying' && <Badge tone="warn">Router didn’t open it</Badge>}
              {outsideState === 'cgnat' && <Badge tone="bad">Shared internet address</Badge>}
            </div>
            <div className="font-mono text-[14px] text-fg">{outside ?? '…'}</div>
          </div>
          {outside && <IconButton icon={Copy} label="Copy the internet address" onClick={() => copy(outside)} />}
          <Button size="sm" variant="ghost" icon={Globe} onClick={() => navigate('servers', { id: s.id, tab: 'settings', section: 'network' })}>
            Set up
          </Button>
        </div>
        {outsideState === 'cgnat' && <Callout tone="warn">Your internet provider shares one public address between many homes (CGNAT), so friends outside your network can’t reach this PC directly. Ask your provider for a public IPv4 address, or use a tunnel such as playit.gg, or a shared network like Tailscale or ZeroTier.</Callout>}
      </div>
    </Card>
  );
}

function Overview({ s, onTab }: { s: SrvView; onTab: (t: Tab, extra?: Record<string, string>) => void }) {
  const [max, setMax] = useState(String(s.maxPlayers));
  const [say, setSay] = useState('');
  useEffect(() => setMax(String(s.maxPlayers)), [s.maxPlayers]);
  const patch = (p: Parameters<typeof api.servers.update>[1]) => void run('Not saved', () => api.servers.update(s.id, p));
  const saveMax = () => {
    const n = Math.round(Number(max));
    if (n >= 1 && n !== s.maxPlayers) void run('Not saved', () => api.servers.setProperties(s.id, [['max-players', String(n)]]), s.state === 'running' ? 'Saved: it takes effect after a restart' : 'Saved');
  };
  const memPct = s.memoryMb ? Math.min(100, (s.memoryUsedMb / s.memoryMb) * 100) : 0;
  return (
    <div className="grid gap-4 lg:grid-cols-[1.25fr_1fr]">
      <div className="space-y-4">
        <JoinCard s={s} />
        <Card>
          <CardHeader icon={Users} title="Players" subtitle={s.state === 'running' ? `${s.online.length} of ${s.maxPlayers} online` : 'The server is off.'} className="px-5 pt-4" actions={<Button size="sm" variant="ghost" onClick={() => onTab('players')}>All players</Button>} />
          <div className="flex flex-wrap gap-2 px-5 pb-4 pt-3">
            {s.online.length === 0 && <div className="text-[12.5px] text-dim">{s.state === 'running' ? 'Nobody is on right now.' : 'Start the server to play.'}</div>}
            {s.online.map((o) => (
              <button key={o.name} type="button" onClick={() => onTab('players', { player: o.name })} className="flex items-center gap-2 rounded-full border border-line py-1 pl-1 pr-3 text-[12.5px] text-fg hover:bg-surface-2">
                <Head name={o.name} size={22} className="rounded-full" />
                {o.name}
                <span className="text-faint">{ago(o.since)}</span>
              </button>
            ))}
          </div>
          <div className="flex items-center gap-3 border-t border-line px-5 py-3">
            <span className="text-[12.5px] text-dim">Most players at once</span>
            <TextInput type="number" min={1} max={1000} value={max} onChange={(e) => setMax(e.target.value)} onBlur={saveMax} onKeyDown={(e) => e.key === 'Enter' && saveMax()} className="w-24" inputSize="sm" aria-label="Most players at once" />
          </div>
          {s.state === 'running' && (
            <div className="flex items-center gap-2 border-t border-line px-5 py-3">
              <TextInput value={say} onChange={(e) => setSay(e.target.value)} placeholder="Say something to everyone on the server" icon={MessageSquare} className="flex-1" onKeyDown={(e) => {
                if (e.key === 'Enter' && say.trim()) void run('Not sent', () => api.servers.command(s.id, `say ${say.trim()}`)).then((ok) => ok && setSay(''));
              }} />
            </div>
          )}
        </Card>
      </div>
      <div className="space-y-4">
        <Card>
          <CardHeader icon={Gauge} title="Running" className="px-5 pt-4" />
          <div className="grid grid-cols-2 gap-3 px-5 pb-4 pt-3 text-[12.5px]">
            <div>
              <div className="text-faint">Up for</div>
              <div className="text-[15px] font-semibold text-fg">{s.state === 'running' ? ago(s.since) : '—'}</div>
            </div>
            <div>
              <div className="text-faint">Processor</div>
              <div className="text-[15px] font-semibold text-fg">{up(s) ? `${Math.round(s.cpu)}%` : '—'}</div>
            </div>
            <div className="col-span-2 space-y-1">
              <div className="flex justify-between text-faint">
                <span>Memory</span>
                <span>
                  {up(s) ? `${(s.memoryUsedMb / 1024).toFixed(1)} of ` : ''}
                  {(s.memoryMb / 1024).toFixed(1)} GB
                </span>
              </div>
              <ProgressBar value={up(s) ? memPct / 100 : 0} tone={memPct > 90 ? 'warn' : 'accent'} />
            </div>
          </div>
        </Card>
        <Card>
          <CardHeader icon={Power} title="Around the clock" subtitle="OmniHub keeps it up while this PC is on." className="px-5 pt-4" />
          <div className="space-y-3 px-5 pb-4 pt-3">
            <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
              <span>
                Start with OmniHub
                <span className="block text-[11.5px] text-faint">OmniHub starts with Windows (Settings), so the server does too.</span>
              </span>
              <Switch checked={s.autoStart} onChange={(v) => patch({ autoStart: v })} label="Start with OmniHub" />
            </label>
            <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
              <span>
                Restart it if it crashes
                <span className="block text-[11.5px] text-faint">Up to three times in ten minutes.</span>
              </span>
              <Switch checked={s.autoRestart} onChange={(v) => patch({ autoRestart: v })} label="Restart if it crashes" />
            </label>
            <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
              <span>
                Keep the PC awake while it runs
                <span className="block text-[11.5px] text-faint">The screen can still turn off.</span>
              </span>
              <Switch checked={s.keepAwake} onChange={(v) => patch({ keepAwake: v })} label="Keep the PC awake" />
            </label>
            <div className="flex items-center justify-between gap-3 text-[13px] text-fg">
              <span>
                Back up the world
                <span className="block text-[11.5px] text-faint">{s.lastBackup ? `Last backup ${ago(s.lastBackup)} ago.` : 'No backup yet.'}</span>
              </span>
              <Select value={String(s.backupHours)} onChange={(e) => patch({ backupHours: Number(e.target.value) })} className="w-40" aria-label="How often to back up">
                <option value="0">Never</option>
                <option value="3">Every 3 hours</option>
                <option value="6">Every 6 hours</option>
                <option value="12">Every 12 hours</option>
                <option value="24">Every day</option>
                <option value="168">Every week</option>
              </Select>
            </div>
          </div>
        </Card>
        <div className="flex flex-wrap gap-2">
          <Button variant="secondary" icon={FolderOpen} onClick={() => void api.app.openPath(s.dir)}>
            Server folder
          </Button>
          <Button variant="secondary" icon={Archive} onClick={() => void run('Not backed up', () => api.servers.backup(s.id), 'Backed up')}>
            Back up now
          </Button>
        </div>
      </div>
    </div>
  );
}

export function ServerView({ s, reload }: { s: SrvView; reload: () => void }) {
  const route = useRoute();
  const tab = (route.params.get('tab') as Tab | null) ?? 'overview';
  const setTab = (t: Tab, extra: Record<string, string> = {}) => navigate('servers', { id: s.id, ...(t === 'overview' ? {} : { tab: t }), ...extra });
  useEvent<{ id: string }>('mcserver:changed', (e) => e.id === s.id && reload());

  const stop = async () => {
    if (s.online.length && !(await confirm({ title: `Stop ${s.name}?`, description: `${s.online.length} player${s.online.length === 1 ? ' is' : 's are'} on. The world is saved first.`, confirmLabel: 'Stop' }))) return;
    void run('Not stopped', () => api.servers.stop(s.id));
  };

  return (
    <Page
      title={
        <span className="flex items-center gap-2">
          <IconButton icon={ArrowLeft} label="All servers" onClick={() => navigate('servers')} />
          <ServerIcon s={s} size={30} />
          {s.name}
        </span>
      }
      subtitle={
        <span className="flex flex-wrap items-center gap-1.5">
          <StateBadge s={s} />
          <span>
            {flavorLabel(s.flavor)} {s.loader && s.flavor !== 'vanilla' ? `${s.loader} · ` : ''}Minecraft {s.mc}
          </span>
          {s.from && <span className="text-faint">· {s.from}</span>}
        </span>
      }
      actions={
        <>
          {(s.state === 'stopped' || s.state === 'crashed') && (
            <Button variant="primary" icon={Play} disabled={!s.launch} onClick={() => void run('Not started', () => api.servers.start(s.id))}>
              Start
            </Button>
          )}
          {up(s) && (
            <>
              <Button variant="secondary" icon={RotateCw} onClick={() => void run('Not restarted', () => api.servers.restart(s.id))}>
                Restart
              </Button>
              <Button variant="secondary" icon={Square} onClick={() => void stop()}>
                Stop
              </Button>
            </>
          )}
          {s.state === 'stopping' && (
            <Button variant="danger" icon={Power} onClick={() => void run('Not ended', () => api.servers.kill(s.id))}>
              End now
            </Button>
          )}
        </>
      }
      headerExtra={
        <Tabs
          value={tab}
          onChange={(t) => setTab(t)}
          items={[
            { value: 'overview', label: 'Overview', icon: Gauge },
            { value: 'console', label: 'Console', icon: ScrollText },
            { value: 'players', label: 'Players', icon: Users },
            { value: 'map', label: 'Map', icon: MapIcon },
            { value: 'content', label: 'Mods & plugins', icon: Package },
            { value: 'settings', label: 'Settings', icon: Settings2 },
          ]}
        />
      }
    >
      <div className="space-y-4">
        {s.work && (
          <Card className="space-y-2 px-5 py-4">
            <div className="text-[13px] text-fg">{s.work.text}</div>
            <ProgressBar value={s.work.total ? s.work.done / s.work.total : null} />
          </Card>
        )}
        {s.problem && !busy(s) && (
          <Callout tone="bad" icon={TriangleAlert} action={<Button size="sm" variant="secondary" onClick={() => setTab('console')}>Console</Button>}>
            {s.problem}
          </Callout>
        )}
        {s.restartNeeded && s.state === 'running' && (
          <Callout tone="warn" action={<Button size="sm" variant="secondary" icon={RotateCw} onClick={() => void run('Not restarted', () => api.servers.restart(s.id))}>Restart now</Button>}>
            Some changes take effect when the server restarts.
          </Callout>
        )}
        {!s.eulaAt && s.state !== 'installing' && (
          <Callout tone="warn" action={<Button size="sm" variant="secondary" onClick={() => setTab('settings')}>Settings</Button>}>
            This server needs Minecraft’s EULA accepted before it starts.
          </Callout>
        )}
        {tab === 'overview' && <Overview s={s} onTab={setTab} />}
        {tab === 'console' && <ConsoleTab s={s} />}
        {tab === 'players' && <PlayersTab s={s} />}
        {tab === 'map' && <MapTab s={s} />}
        {tab === 'content' && <ContentTab s={s} />}
        {tab === 'settings' && <SettingsTab s={s} reload={reload} />}
      </div>
    </Page>
  );
}
