// A server's settings: the game's own (server.properties, the common ones
// in plain words and all of them as they are), memory and Java, the way in
// for friends (router port, firewall), backups, and removing it.

import type { SrvBackup, SrvNet, SrvView } from '@shared/types';
import { formatBytes } from '@shared/format';
import { Archive, Cpu, ExternalLink, FolderOpen, Globe, ImagePlus, RotateCcw, Save, ShieldCheck, SlidersHorizontal, Trash2, Wifi } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Spinner } from '../../components/ui/Card';
import { Field, SearchInput, Select, Switch, TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { navigate, useRoute } from '../../lib/router';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { run } from './ServerView';
import { EVERYONE_NOTE, JoinChoice, ago } from './shared';

type Props = [string, string][];

const BOOL = (v: string | undefined) => v === 'true';

function Section({ id, children }: { id: string; children: React.ReactNode }) {
  const route = useRoute();
  const focus = route.params.get('section') === id;
  useEffect(() => {
    if (focus) document.getElementById(`srv-${id}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }, [focus, id]);
  return (
    <div id={`srv-${id}`} className="scroll-mt-4">
      {children}
    </div>
  );
}

function GameSettings({ s }: { s: SrvView }) {
  const [props, setProps] = useState<Props | null>(null);
  const [edits, setEdits] = useState<Record<string, string>>({});
  const [filter, setFilter] = useState('');
  const load = useCallback(() => void api.servers.properties(s.id).then((p) => (setProps(p), setEdits({}))), [s.id]);
  useEffect(load, [load]);
  const get = (k: string, d = '') => edits[k] ?? props?.find(([key]) => key === k)?.[1] ?? d;
  const set = (k: string, v: string) => setEdits((e) => ({ ...e, [k]: v }));
  const changed = Object.entries(edits).filter(([k, v]) => props?.find(([key]) => key === k)?.[1] !== v);
  const save = async () => {
    if (await run('Not saved', () => api.servers.setProperties(s.id, changed), s.state === 'running' ? 'Saved (difficulty, game mode and whitelist apply now; the rest after a restart)' : 'Saved')) load();
  };
  const shown = useMemo(() => (props ?? []).filter(([k]) => !filter || k.includes(filter.toLowerCase())), [props, filter]);
  if (!props) return <Spinner />;
  const toggle = (k: string, label: string, hint?: string) => (
    <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
      <span>
        {label}
        {hint && <span className="block text-[11.5px] text-faint">{hint}</span>}
      </span>
      <Switch checked={BOOL(get(k))} onChange={(v) => set(k, v ? 'true' : 'false')} label={label} />
    </label>
  );
  return (
    <Card>
      <CardHeader icon={SlidersHorizontal} title="Game settings" subtitle="The server’s server.properties." className="px-5 pt-4" actions={changed.length > 0 && <Button size="sm" variant="primary" icon={Save} onClick={() => void save()}>Save {changed.length} change{changed.length === 1 ? '' : 's'}</Button>} />
      <div className="grid gap-3 px-5 pb-4 pt-3 sm:grid-cols-2">
        <Field label="Message in the server list (MOTD)" className="sm:col-span-2">
          <TextInput value={get('motd')} onChange={(e) => set('motd', e.target.value)} maxLength={59} />
        </Field>
        <Field label="Most players at once">
          <TextInput type="number" min={1} value={get('max-players', '20')} onChange={(e) => set('max-players', e.target.value)} />
        </Field>
        <Field label="Port" hint="Friends use it after the address (25565 needs nothing extra).">
          <TextInput type="number" min={1024} max={65535} value={get('server-port', '25565')} onChange={(e) => set('server-port', e.target.value)} />
        </Field>
        <Field label="Game mode for new players">
          <Select value={get('gamemode', 'survival')} onChange={(e) => set('gamemode', e.target.value)}>
            <option value="survival">Survival</option>
            <option value="creative">Creative</option>
            <option value="adventure">Adventure</option>
            <option value="spectator">Spectator</option>
          </Select>
        </Field>
        <Field label="Difficulty">
          <Select value={get('difficulty', 'easy')} onChange={(e) => set('difficulty', e.target.value)}>
            <option value="peaceful">Peaceful</option>
            <option value="easy">Easy</option>
            <option value="normal">Normal</option>
            <option value="hard">Hard</option>
          </Select>
        </Field>
        <Field label={`View distance: ${get('view-distance', '10')} chunks`} hint="Lower is lighter on the PC.">
          <input type="range" min={3} max={32} value={get('view-distance', '10')} onChange={(e) => set('view-distance', e.target.value)} className="w-full accent-[var(--accent)]" aria-label="View distance" />
        </Field>
        <Field label={`Simulation distance: ${get('simulation-distance', '10')} chunks`} hint="How far away crops grow and mobs move.">
          <input type="range" min={3} max={32} value={get('simulation-distance', '10')} onChange={(e) => set('simulation-distance', e.target.value)} className="w-full accent-[var(--accent)]" aria-label="Simulation distance" />
        </Field>
        <Field label="Spawn protection (blocks)" hint="Only operators can build this close to spawn.">
          <TextInput type="number" min={0} value={get('spawn-protection', '16')} onChange={(e) => set('spawn-protection', e.target.value)} />
        </Field>
        <div className="space-y-2 sm:col-span-2">
          <div className="text-[12.5px] font-medium text-fg">Who can join</div>
          <JoinChoice everyone={get('online-mode', 'true') === 'false'} onChange={(e) => set('online-mode', e ? 'false' : 'true')} />
          {get('online-mode', 'true') === 'false' && <Callout tone="info">{EVERYONE_NOTE}</Callout>}
          {changed.some(([k]) => k === 'online-mode') && <div className="text-[11.5px] text-faint">At the next start OmniHub moves everyone’s whitelist, operator and ban entries and their saved things to the new kind of account, so nobody loses anything.</div>}
        </div>
        <div className="space-y-2.5 sm:col-span-2">
          {toggle('pvp', 'Players can hurt each other (PvP)')}
          {toggle('white-list', 'Whitelist', 'Only players on it can join.')}
          {toggle('enforce-whitelist', 'Remove players not on the whitelist', 'When the whitelist changes while they’re on.')}
          {toggle('allow-flight', 'Allow flying', 'Needed by some mods and plugins, or players get kicked for “flying”.')}
          {toggle('enable-command-block', 'Command blocks work')}
          {toggle('hardcore', 'Hardcore', 'One life; only for a new world.')}
        </div>
      </div>
      <details className="border-t border-line px-5 py-3">
        <summary className="cursor-pointer text-[12.5px] font-medium text-dim">All {props.length} settings, as the server has them</summary>
        <div className="mt-3 space-y-2">
          <SearchInput value={filter} onChange={setFilter} placeholder="Find a setting" />
          <div className="max-h-80 space-y-1 overflow-y-auto">
            {shown.map(([k]) => (
              <div key={k} className="grid grid-cols-[minmax(0,1fr)_minmax(0,1.3fr)] items-center gap-2">
                <span className="truncate font-mono text-[12px] text-dim" title={k}>
                  {k}
                </span>
                <TextInput value={get(k)} onChange={(e) => set(k, e.target.value)} inputSize="sm" className="font-mono" type={k.includes('password') || k.includes('secret') ? 'password' : 'text'} aria-label={k} />
              </div>
            ))}
          </div>
        </div>
      </details>
    </Card>
  );
}

function Running({ s }: { s: SrvView }) {
  const [mem, setMem] = useState(s.memoryMb);
  const [flags, setFlags] = useState(s.flags.join(' '));
  useEffect(() => setMem(s.memoryMb), [s.memoryMb]);
  const pickJava = async () => {
    const f = await api.app.pickFiles('Pick java.exe (in a Java folder’s bin)');
    if (f?.[0]) void run('Not used', () => api.servers.update(s.id, { javaPath: f[0] }), 'Java changed');
  };
  const pickIcon = async () => {
    const f = await api.app.pickFiles('Pick a picture for the server list (made 64×64)');
    if (f?.[0]) void run('Not changed', () => api.servers.setIcon(s.id, f[0]), 'Picture changed');
  };
  return (
    <Card>
      <CardHeader icon={Cpu} title="Memory and Java" className="px-5 pt-4" />
      <div className="space-y-4 px-5 pb-4 pt-3">
        <Field label={`Memory: ${(mem / 1024).toFixed(mem % 1024 ? 1 : 0)} GB`} hint="Takes effect on the next start.">
          <input type="range" min={1024} max={16384} step={512} value={mem} onChange={(e) => setMem(Number(e.target.value))} onPointerUp={() => mem !== s.memoryMb && void run('Not saved', () => api.servers.update(s.id, { memoryMb: mem }))} onKeyUp={() => mem !== s.memoryMb && void run('Not saved', () => api.servers.update(s.id, { memoryMb: mem }))} className="w-full accent-[var(--accent)]" aria-label="Memory" />
        </Field>
        <div className="flex flex-wrap items-center gap-2 text-[13px]">
          <span className="text-dim">Java:</span>
          <span className="font-mono text-[12px] text-fg">{s.javaPath ?? `OmniHub’s Java ${s.javaMajor} (Eclipse Temurin)`}</span>
          <div className="flex-1" />
          <Button size="sm" variant="ghost" onClick={() => void pickJava()}>
            Use another Java
          </Button>
          {s.javaPath && (
            <Button size="sm" variant="ghost" onClick={() => void run('Not changed', () => api.servers.update(s.id, { javaPath: '' }))}>
              Use OmniHub’s
            </Button>
          )}
        </div>
        <Field label="Extra start-up flags (advanced)" hint="-X…, -XX:… and -D… flags, separated by spaces.">
          <div className="flex gap-2">
            <TextInput value={flags} onChange={(e) => setFlags(e.target.value)} className="flex-1 font-mono" />
            <Button variant="secondary" disabled={flags === s.flags.join(' ')} onClick={() => void run('Not saved', () => api.servers.update(s.id, { flags: flags.split(/\s+/).filter(Boolean) }), 'Saved')}>
              Save
            </Button>
          </div>
        </Field>
        <div className="flex flex-wrap gap-2">
          <Button size="sm" variant="secondary" icon={ImagePlus} onClick={() => void pickIcon()}>
            Server picture
          </Button>
          <Button size="sm" variant="secondary" icon={FolderOpen} onClick={() => void api.app.openPath(s.dir)}>
            Server folder
          </Button>
        </div>
      </div>
    </Card>
  );
}

function Network({ s }: { s: SrvView }) {
  const [net, setNet] = useState<SrvNet | null>(null);
  const [busy, setBusy] = useState(false);
  const load = useCallback(() => void api.servers.network(s.id).then(setNet, (e: unknown) => toast.error('Network not checked', errorText(e))), [s.id]);
  useEffect(load, [load]);
  const upnp = async (on: boolean) => {
    setBusy(true);
    try {
      setNet(await api.servers.setUpnp(s.id, on));
    } catch (e) {
      toast.error('Not changed', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const fw = async () => {
    setBusy(true);
    try {
      setNet(await api.servers.fixFirewall(s.id));
      toast.success('Firewall allows the server');
    } catch (e) {
      toast.error('Firewall not changed', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Card>
      <CardHeader icon={Globe} title="Friends outside your home" subtitle="Players on your Wi-Fi join with the home address. Others need the router to let them in." className="px-5 pt-4" />
      {!net ? (
        <div className="px-5 py-4">
          <Spinner />
        </div>
      ) : (
        <div className="space-y-3 px-5 pb-4 pt-3 text-[13px]">
          <div className="grid gap-2 sm:grid-cols-2">
            <div className="rounded-xl border border-line px-3.5 py-2.5">
              <div className="text-[11.5px] text-faint">Home network</div>
              <div className="font-mono text-fg">{net.lan.join(', ') || `localhost:${net.port}`}</div>
            </div>
            <div className="rounded-xl border border-line px-3.5 py-2.5">
              <div className="text-[11.5px] text-faint">Internet</div>
              <div className="font-mono text-fg">{net.publicIp ? `${net.publicIp}:${net.port}` : 'Unknown'}</div>
            </div>
          </div>
          <label className="flex items-center justify-between gap-3 text-fg">
            <span>
              Open the port on the router (UPnP)
              <span className="block text-[11.5px] text-faint">{net.router ? `Router: ${net.router}${net.portOpen ? ` · port ${net.port} is open` : ''}` : 'No router answered on this network (UPnP may be off in its settings).'}</span>
            </span>
            <Switch checked={net.upnp} disabled={busy} onChange={(v) => void upnp(v)} label="Open the port on the router" />
          </label>
          {net.upnp && net.upnpError && <Callout tone="warn">{net.upnpError} You can forward TCP port {net.port} to {net.lan[0]?.split(':')[0] ?? 'this PC'} in your router’s settings by hand instead.</Callout>}
          {net.cgnat && <Callout tone="bad">Your internet provider shares one public address between many homes (CGNAT{net.routerIp ? `: the router’s address is ${net.routerIp}` : ''}), so opening ports can’t work. Ask the provider for a public IPv4 address, or use playit.gg, Tailscale or ZeroTier.</Callout>}
          <div className="flex flex-wrap items-center gap-2 border-t border-line pt-3">
            <ShieldCheck size={15} className="text-dim" />
            <span className="flex-1 text-dim">{net.firewall ? net.firewall.message : 'Windows Firewall: start the server once, then check here.'}</span>
            {net.firewall?.supported && net.firewall.verdict !== 'allowed' && net.firewall.verdict !== 'off' && (
              <Button size="sm" variant="secondary" disabled={busy} onClick={() => void fw()}>
                Allow it (asks for admin)
              </Button>
            )}
            {net.firewall?.verdict === 'allowed' && <Badge tone="good">Allowed</Badge>}
          </div>
          <Button size="sm" variant="ghost" icon={Wifi} onClick={load}>
            Check again
          </Button>
        </div>
      )}
    </Card>
  );
}

function Backups({ s }: { s: SrvView }) {
  const [list, setList] = useState<SrvBackup[] | null>(null);
  const load = useCallback(() => void api.servers.backups(s.id).then(setList, () => setList([])), [s.id]);
  useEffect(load, [load, s.lastBackup]);
  const restore = async (b: SrvBackup) => {
    if (s.state !== 'stopped' && s.state !== 'crashed') {
      toast.info('Stop the server first', 'A backup can only be put back while it’s off.');
      return;
    }
    if (await confirm({ title: `Put back ${b.name}?`, description: 'The world goes back to how it was then. The current world is kept beside it in the server folder, in case.', confirmLabel: 'Put it back' })) {
      await run('Not put back', () => api.servers.restore(s.id, b.name), 'Put back');
    }
  };
  const remove = async (b: SrvBackup) => {
    if (await confirm({ title: `Delete ${b.name}?`, confirmLabel: 'Delete', tone: 'danger' })) {
      await run('Not deleted', () => api.servers.removeBackup(s.id, b.name));
      load();
    }
  };
  return (
    <Card>
      <CardHeader
        icon={Archive}
        title="Backups"
        subtitle={s.backupHours ? `Every ${s.backupHours === 24 ? 'day' : s.backupHours === 168 ? 'week' : `${s.backupHours} hours`}, keeping the newest ${s.backupKeep}. In the server’s backups folder.` : 'Scheduled backups are off (Overview).'}
        className="px-5 pt-4"
        actions={<Button size="sm" variant="secondary" icon={Archive} onClick={() => void run('Not backed up', () => api.servers.backup(s.id), 'Backed up').then(load)}>Back up now</Button>}
      />
      <div className="flex items-center gap-2 px-5 pt-3 text-[12.5px] text-dim">
        Keep
        <Select value={String(s.backupKeep)} onChange={(e) => void run('Not saved', () => api.servers.update(s.id, { backupKeep: Number(e.target.value) }))} className="w-24" aria-label="Backups to keep">
          {[3, 5, 7, 14, 30].map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </Select>
        scheduled backups (ones you make by hand stay).
      </div>
      <div className="divide-y divide-line px-2 pb-2 pt-2">
        {list?.length === 0 && <div className="px-3 py-3 text-[12.5px] text-dim">No backups yet.</div>}
        {list?.map((b) => (
          <div key={b.name} className="flex items-center gap-3 px-3 py-2.5">
            <Archive size={15} className="text-dim" />
            <div className="min-w-0 flex-1">
              <div className="truncate font-mono text-[12.5px] text-fg">{b.name}</div>
              <div className="text-[11.5px] text-faint">
                {ago(b.at)} ago · {formatBytes(b.size)} {b.auto ? '· scheduled' : '· by hand'}
              </div>
            </div>
            <Button size="sm" variant="ghost" icon={RotateCcw} onClick={() => void restore(b)}>
              Put back
            </Button>
            <IconButton icon={Trash2} label={`Delete ${b.name}`} onClick={() => void remove(b)} />
          </div>
        ))}
      </div>
    </Card>
  );
}

export function SettingsTab({ s, reload }: { s: SrvView; reload: () => void }) {
  const [name, setName] = useState(s.name);
  const remove = async (files: boolean) => {
    const ok = await confirm({
      title: files ? `Delete ${s.name} and its world?` : `Remove ${s.name} from OmniHub?`,
      description: files ? 'Its folder (world, backups, mods and settings) is deleted for good.' : 'Its folder stays where it is.',
      confirmLabel: files ? 'Delete everything' : 'Remove',
      tone: 'danger',
      typed: files ? s.name : undefined,
    });
    if (ok && (await run('Not removed', () => api.servers.remove(s.id, files)))) {
      reload();
      navigate('servers');
    }
  };
  return (
    <div className="grid gap-4 xl:grid-cols-2">
      <div className="space-y-4">
        {!s.eulaAt && (
          <Callout tone="warn" action={<Button size="sm" variant="secondary" icon={ExternalLink} onClick={() => void api.app.openUrl('https://aka.ms/MinecraftEULA')}>Read it</Button>}>
            <div className="flex flex-wrap items-center gap-3">
              Minecraft servers only run once you accept Mojang’s EULA.
              <Button size="sm" variant="primary" onClick={() => void run('Not saved', () => api.servers.update(s.id, { eula: true }), 'Accepted')}>
                I accept the EULA
              </Button>
            </div>
          </Callout>
        )}
        <Card className="flex items-end gap-2 px-5 py-4">
          <Field label="Name" className="flex-1">
            <TextInput value={name} onChange={(e) => setName(e.target.value)} maxLength={60} />
          </Field>
          <Button variant="secondary" disabled={!name.trim() || name === s.name} onClick={() => void run('Not renamed', () => api.servers.update(s.id, { name }), 'Renamed')}>
            Rename
          </Button>
        </Card>
        <GameSettings s={s} />
      </div>
      <div className="space-y-4">
        <Section id="network">
          <Network s={s} />
        </Section>
        <Section id="backups">
          <Backups s={s} />
        </Section>
        <Running s={s} />
        <Card className="space-y-2 px-5 py-4">
          <div className="text-[13px] font-medium text-fg">Remove this server</div>
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="secondary" disabled={s.state !== 'stopped' && s.state !== 'crashed'} onClick={() => void remove(false)}>
              Remove from OmniHub (keep its folder)
            </Button>
            {s.owned && (
              <Button size="sm" variant="danger" icon={Trash2} disabled={s.state !== 'stopped' && s.state !== 'crashed'} onClick={() => void remove(true)}>
                Delete it and its world
              </Button>
            )}
          </div>
        </Card>
      </div>
    </div>
  );
}
