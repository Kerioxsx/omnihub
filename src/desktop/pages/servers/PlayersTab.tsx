// Players: who's on and who's been, the whitelist, operators and bans, and
// one player up close — where they are (teleport them), their health and
// game mode, and their inventory (give, take away).

import type { SrvAction, SrvGamemode, SrvItem, SrvKnown, SrvPlayer, SrvPlayers, SrvView } from '@shared/types';
import { Ban, Crown, Gift, Heart, Home, LogOut, MapPin, MessageSquare, Navigation, Plus, Search, ShieldAlert, ShieldCheck, Trash2, UserCheck, UserMinus, Utensils } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, CardHeader, Spinner } from '../../components/ui/Card';
import { Field, SearchInput, Select, Switch, TextInput } from '../../components/ui/Form';
import { Callout, EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useEvent, useInterval } from '../../lib/hooks';
import { navigate, useRoute } from '../../lib/router';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { run } from './ServerView';
import { agoText, dimLabel, DIMENSIONS, Head, ItemTile } from './shared';

const COMMON_ITEMS = ['minecraft:diamond', 'minecraft:iron_ingot', 'minecraft:golden_apple', 'minecraft:cooked_beef', 'minecraft:torch', 'minecraft:oak_log', 'minecraft:diamond_pickaxe', 'minecraft:diamond_sword', 'minecraft:elytra', 'minecraft:firework_rocket', 'minecraft:ender_pearl', 'minecraft:experience_bottle', 'minecraft:shulker_box', 'minecraft:totem_of_undying', 'minecraft:netherite_ingot', 'minecraft:bread'];

function Bar({ value, max, color, label }: { value: number | null; max: number; color: string; label: string }) {
  if (value === null) return null;
  return (
    <div className="space-y-1">
      <div className="flex justify-between text-[11.5px] text-faint">
        <span>{label}</span>
        <span>
          {Math.round(value)}/{max}
        </span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-surface-2">
        <div className="h-full rounded-full" style={{ width: `${Math.max(0, Math.min(100, (value / max) * 100))}%`, background: color }} />
      </div>
    </div>
  );
}

function Inventory({ p, onPick, picked }: { p: SrvPlayer; onPick: (it: SrvItem) => void; picked: number | null }) {
  const [hover, setHover] = useState<SrvItem | null>(null);
  const at = (slot: number) => p.items.find((i) => i.slot === slot) ?? null;
  const tile = (slot: number) => (
    <div key={slot} onMouseEnter={() => setHover(at(slot))} onMouseLeave={() => setHover(null)}>
      <ItemTile it={at(slot)} onClick={p.online ? () => at(slot) && onPick(at(slot)!) : undefined} selected={picked === slot} />
    </div>
  );
  return (
    <div className="space-y-2">
      <div className="h-5 text-[12.5px] text-dim">{hover ? `${hover.custom ? `“${hover.custom}” · ` : ''}${hover.label}${hover.count > 1 ? ` × ${hover.count}` : ''}${hover.enchanted ? ' · enchanted' : ''}` : `${p.items.length} stack${p.items.length === 1 ? '' : 's'}. Point at one to see what it is.`}</div>
      <div className="flex flex-wrap items-start gap-4">
        <div className="space-y-1.5">
          <div className="text-[11.5px] text-faint">Armour</div>
          {[103, 102, 101, 100].map(tile)}
          <div className="pt-1 text-[11.5px] text-faint">Off hand</div>
          {tile(-106)}
        </div>
        <div className="space-y-1.5">
          <div className="text-[11.5px] text-faint">Inventory</div>
          <div className="grid grid-cols-9 gap-1.5">{Array.from({ length: 27 }, (_, i) => tile(9 + i))}</div>
          <div className="pt-1 text-[11.5px] text-faint">Hotbar</div>
          <div className="grid grid-cols-9 gap-1.5">{Array.from({ length: 9 }, (_, i) => <div key={i} className={cx('rounded-md', p.selected === i && 'ring-2 ring-warn')}>{tile(i)}</div>)}</div>
        </div>
      </div>
    </div>
  );
}

function PlayerPanel({ s, name, known, onChanged }: { s: SrvView; name: string; known: SrvKnown | undefined; onChanged: () => void }) {
  const [p, setP] = useState<SrvPlayer | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [picked, setPicked] = useState<SrvItem | null>(null);
  const [item, setItem] = useState('minecraft:diamond');
  const [count, setCount] = useState(1);
  const [tp, setTp] = useState({ x: '', y: '', z: '', dim: 'minecraft:overworld' });
  const [msg, setMsg] = useState('');
  const load = useCallback(() => void api.servers.player(s.id, name).then((x) => (setP(x), setErr(null)), (e: unknown) => setErr(errorText(e))), [s.id, name]);
  useEffect(() => {
    setP(null);
    setPicked(null);
    load();
  }, [load]);
  useInterval(load, p?.online ? 3000 : null);
  useEffect(() => {
    if (p?.pos) setTp((t) => (t.x ? t : { x: String(Math.round(p.pos![0])), y: String(Math.round(p.pos![1])), z: String(Math.round(p.pos![2])), dim: p.dimension ?? 'minecraft:overworld' }));
  }, [p?.pos, p?.dimension]);

  const act = async (a: SrvAction, ok?: string) => {
    try {
      const reply = await api.servers.act(s.id, name, a);
      toast.success(ok ?? 'Done', reply);
      load();
      onChanged();
    } catch (e) {
      toast.error('Not done', errorText(e));
    }
  };

  if (err) return <Callout tone="bad">{err}</Callout>;
  if (!p) return <Spinner />;
  const online = p.online && s.state === 'running';
  return (
    <div className="space-y-4">
      <Card className="px-5 py-4">
        <div className="flex flex-wrap items-center gap-4">
          <Head name={p.name} size={56} />
          <div className="min-w-0 flex-1 space-y-1">
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-[17px] font-semibold text-fg">{p.name}</span>
              {online ? <Badge tone="good">Online</Badge> : <Badge tone="neutral">Offline</Badge>}
              {known?.op && <Badge tone="accent" icon={Crown}>Operator</Badge>}
              {known?.whitelisted && <Badge tone="info">Whitelisted</Badge>}
              {known?.banned && <Badge tone="bad">Banned{known.banReason ? `: ${known.banReason}` : ''}</Badge>}
            </div>
            <div className="flex flex-wrap items-center gap-1.5 text-[12.5px] text-dim">
              <MapPin size={13} />
              {p.pos ? `${Math.round(p.pos[0])}, ${Math.round(p.pos[1])}, ${Math.round(p.pos[2])} · ${dimLabel(p.dimension)}` : 'Never played here yet'}
              {!p.live && p.pos && <span className="text-faint">(where they left)</span>}
            </div>
          </div>
          {online && (
            <Button size="sm" variant="ghost" icon={MapPin} onClick={() => navigate('servers', { id: s.id, tab: 'map', player: p.name })}>
              On the map
            </Button>
          )}
        </div>
        {(p.health !== null || p.food !== null) && (
          <div className="mt-4 grid gap-3 sm:grid-cols-3">
            <Bar value={p.health} max={20} color="#e5484d" label="Health" />
            <Bar value={p.food} max={20} color="#c8963e" label="Food" />
            <div className="space-y-1 text-[11.5px] text-faint">
              <div>Level {p.xpLevel ?? 0}</div>
              {online ? (
                <Select value={p.gamemode ?? 'survival'} onChange={(e) => void act({ type: 'gamemode', mode: e.target.value as SrvGamemode }, 'Game mode changed')} aria-label="Game mode">
                  <option value="survival">Survival</option>
                  <option value="creative">Creative</option>
                  <option value="adventure">Adventure</option>
                  <option value="spectator">Spectator</option>
                </Select>
              ) : (
                <div className="capitalize text-dim">{p.gamemode ?? '—'}</div>
              )}
            </div>
          </div>
        )}
      </Card>

      <Card className="space-y-3 px-5 py-4">
        <CardHeader title="Inventory" subtitle={online ? 'Read live from the server. Pick a stack to take it away.' : 'As they left it (changes need them online).'} />
        <Inventory p={p} picked={picked?.slot ?? null} onPick={setPicked} />
        {picked && online && (
          <div className="flex flex-wrap items-center gap-2 rounded-xl border border-line px-3 py-2 text-[12.5px]">
            <span className="text-fg">
              {picked.custom ?? picked.label} × {picked.count}
            </span>
            <span className="font-mono text-[11px] text-faint">{picked.id}</span>
            <div className="flex-1" />
            <Button size="sm" variant="danger" icon={Trash2} onClick={() => void act({ type: 'clearSlot', slot: picked.slot }, 'Taken away').then(() => setPicked(null))}>
              Take it away
            </Button>
          </div>
        )}
        {online && (
          <div className="flex flex-wrap items-end gap-2">
            <Field label="Give" className="min-w-[220px] flex-1">
              <TextInput list="srv-items" value={item} onChange={(e) => setItem(e.target.value)} placeholder="minecraft:diamond" className="font-mono" />
            </Field>
            <datalist id="srv-items">
              {COMMON_ITEMS.map((i) => (
                <option key={i} value={i} />
              ))}
            </datalist>
            <Field label="How many" className="w-24">
              <TextInput type="number" min={1} max={6400} value={count} onChange={(e) => setCount(Math.max(1, Math.min(6400, Number(e.target.value) || 1)))} />
            </Field>
            <Button variant="primary" icon={Gift} onClick={() => void act({ type: 'give', item: item.trim(), count }, 'Given')}>
              Give
            </Button>
          </div>
        )}
      </Card>

      {online && (
        <Card className="space-y-3 px-5 py-4">
          <CardHeader title="Move them" icon={Navigation} />
          <div className="flex flex-wrap items-end gap-2">
            {(['x', 'y', 'z'] as const).map((k) => (
              <Field key={k} label={k.toUpperCase()} className="w-24">
                <TextInput value={tp[k]} onChange={(e) => setTp({ ...tp, [k]: e.target.value })} className="font-mono" />
              </Field>
            ))}
            <Field label="World" className="w-36">
              <Select value={tp.dim} onChange={(e) => setTp({ ...tp, dim: e.target.value })}>
                {DIMENSIONS.map((d) => (
                  <option key={d.id} value={`minecraft:${d.id}`}>
                    {d.label}
                  </option>
                ))}
              </Select>
            </Field>
            <Button variant="secondary" icon={Navigation} onClick={() => void act({ type: 'teleport', x: Number(tp.x), y: Number(tp.y), z: Number(tp.z), dimension: tp.dim }, 'Teleported')}>
              Teleport
            </Button>
            <Button variant="ghost" icon={Home} onClick={() => void act({ type: 'spawn' }, 'Sent to spawn')}>
              To spawn
            </Button>
            {s.online.filter((o) => o.name !== name).length > 0 && (
              <Select defaultValue="" onChange={(e) => e.target.value && void act({ type: 'teleportTo', player: e.target.value }, 'Teleported')} className="w-44" aria-label="Teleport to a player">
                <option value="">To a player…</option>
                {s.online
                  .filter((o) => o.name !== name)
                  .map((o) => (
                    <option key={o.name} value={o.name}>
                      {o.name}
                    </option>
                  ))}
              </Select>
            )}
          </div>
          <div className="flex flex-wrap gap-2 border-t border-line pt-3">
            <Button size="sm" variant="secondary" icon={Heart} onClick={() => void act({ type: 'heal' }, 'Healed')}>
              Heal
            </Button>
            <Button size="sm" variant="secondary" icon={Utensils} onClick={() => void act({ type: 'feed' }, 'Fed')}>
              Feed
            </Button>
            <div className="flex min-w-[240px] flex-1 gap-2">
              <TextInput value={msg} onChange={(e) => setMsg(e.target.value)} placeholder={`A private message to ${name}`} icon={MessageSquare} inputSize="sm" className="flex-1" />
              <Button size="sm" variant="secondary" disabled={!msg.trim()} onClick={() => void act({ type: 'message', text: msg }, 'Sent').then(() => setMsg(''))}>
                Send
              </Button>
            </div>
          </div>
        </Card>
      )}

      <Card className="flex flex-wrap gap-2 px-5 py-4">
        {known?.whitelisted ? (
          <Button size="sm" variant="secondary" icon={UserMinus} onClick={() => void act({ type: 'whitelist', on: false }, 'Off the whitelist')}>
            Take off the whitelist
          </Button>
        ) : (
          <Button size="sm" variant="secondary" icon={UserCheck} onClick={() => void act({ type: 'whitelist', on: true }, 'Whitelisted')}>
            Add to the whitelist
          </Button>
        )}
        {known?.op ? (
          <Button size="sm" variant="secondary" icon={ShieldCheck} onClick={() => void act({ type: 'deop' }, 'No longer an operator')}>
            Remove operator
          </Button>
        ) : (
          <Button size="sm" variant="secondary" icon={Crown} onClick={async () => (await confirm({ title: `Make ${name} an operator?`, description: 'Operators can run every command: change the world, ban people and stop the server.', confirmLabel: 'Make operator' })) && void act({ type: 'op' }, 'Now an operator')}>
            Make operator
          </Button>
        )}
        {online && (
          <Button size="sm" variant="secondary" icon={LogOut} onClick={() => void act({ type: 'kick', reason: null }, 'Kicked')}>
            Kick
          </Button>
        )}
        {known?.banned ? (
          <Button size="sm" variant="secondary" icon={Ban} onClick={() => void act({ type: 'pardon' }, 'Unbanned')}>
            Unban
          </Button>
        ) : (
          <Button size="sm" variant="danger" icon={Ban} onClick={async () => (await confirm({ title: `Ban ${name}?`, description: 'They can’t join again until you unban them.', confirmLabel: 'Ban', tone: 'danger' })) && void act({ type: 'ban', reason: null }, 'Banned')}>
            Ban
          </Button>
        )}
      </Card>
    </div>
  );
}

export function PlayersTab({ s }: { s: SrvView }) {
  const route = useRoute();
  const sel = route.params.get('player');
  const [pv, setPv] = useState<SrvPlayers | null>(null);
  const [query, setQuery] = useState('');
  const [add, setAdd] = useState('');
  const load = useCallback(() => void api.servers.players(s.id).then(setPv, () => undefined), [s.id]);
  useEffect(load, [load, s.online.length]);
  useEvent<{ id: string }>('mcserver:changed', (e) => e.id === s.id && load());
  const pick = (name: string) => navigate('servers', { id: s.id, tab: 'players', player: name });
  const shown = (pv?.known ?? []).filter((k) => !query || k.name.toLowerCase().includes(query.toLowerCase()));
  const addToList = async () => {
    const n = add.trim();
    if (!n) return;
    if (await run('Not added', () => api.servers.act(s.id, n, { type: 'whitelist', on: true }), `${n} can join`)) {
      setAdd('');
      load();
    }
  };
  return (
    <div className="grid gap-4 lg:grid-cols-[300px_1fr]">
      <div className="space-y-3">
        <Card className="space-y-3 px-4 py-3.5">
          <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
            <span>
              Whitelist
              <span className="block text-[11.5px] text-faint">{pv?.whitelistOn ? 'Only players on it can join.' : 'Anyone with the address can join.'}</span>
            </span>
            <Switch checked={!!pv?.whitelistOn} onChange={(v) => void run('Not changed', () => api.servers.whitelist(s.id, v)).then(load)} label="Whitelist" />
          </label>
          <div className="flex gap-2">
            <TextInput value={add} onChange={(e) => setAdd(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && void addToList()} placeholder="A friend’s Minecraft name" inputSize="sm" className="flex-1" />
            <Button size="sm" variant="primary" icon={Plus} disabled={!add.trim()} onClick={() => void addToList()}>
              Add
            </Button>
          </div>
          {pv && !pv.onlineMode && <div className="text-[11.5px] text-dim">Players without a paid account can join. Operators are guarded: their names from a new place wait for your OK.</div>}
        </Card>
        {pv?.waiting.map((w) => (
          <Card key={`${w.name}${w.place}`} className="space-y-2.5 border-warn/40 px-4 py-3.5">
            <div className="flex items-start gap-2.5">
              <ShieldAlert size={16} className="mt-0.5 shrink-0 text-warn" />
              <div className="min-w-0 text-[12.5px] text-fg">
                <b>{w.name}</b> tried to join from a new place
                <div className="font-mono text-[11.5px] text-faint">
                  {w.place} · {agoText(w.at)}
                </div>
                <div className="mt-1 text-[11.5px] text-dim">That name is an operator, so they were kept out. Allow it only if it’s really them (ask them where they’re playing from).</div>
              </div>
            </div>
            <div className="flex gap-2">
              <Button size="sm" variant="primary" icon={UserCheck} onClick={() => void run('Not allowed', () => api.servers.act(s.id, w.name, { type: 'allow', place: w.place }), `${w.name} can join from there: ask them to join again`).then(load)}>
                It’s them
              </Button>
              <Button size="sm" variant="ghost" onClick={() => void run('Not changed', () => api.servers.act(s.id, w.name, { type: 'refuse', place: w.place }), 'Kept out').then(load)}>
                Not them
              </Button>
            </div>
          </Card>
        ))}
        <SearchInput value={query} onChange={setQuery} placeholder="Find a player" />
        <Card className="max-h-[60vh] divide-y divide-line overflow-y-auto">
          {pv && shown.length === 0 && <EmptyState compact icon={Search} title="No players yet" description="Players show here once they’ve joined, or when you add them to the whitelist." />}
          {shown.map((k) => (
            <button key={k.name} type="button" onClick={() => pick(k.name)} className={cx('flex w-full items-center gap-2.5 px-3.5 py-2.5 text-left hover:bg-surface-2', sel === k.name && 'bg-accent-soft')}>
              <Head name={k.name} size={28} />
              <div className="min-w-0 flex-1">
                <div className="truncate text-[13px] font-medium text-fg">{k.name}</div>
                <div className="flex gap-1 text-[11px] text-faint">
                  {k.online ? <span className="text-good">online</span> : <span>offline</span>}
                  {k.op && <span>· operator</span>}
                  {k.whitelisted && <span>· whitelisted</span>}
                  {k.banned && <span className="text-bad">· banned</span>}
                </div>
              </div>
            </button>
          ))}
        </Card>
      </div>
      <div>{sel ? <PlayerPanel s={s} name={sel} known={pv?.known.find((k) => k.name.toLowerCase() === sel.toLowerCase())} onChanged={load} /> : <Card><EmptyState icon={Search} title="Pick a player" description="See where they are, what they carry, and manage them." /></Card>}</div>
    </div>
  );
}
