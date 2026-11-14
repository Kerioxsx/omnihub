// Minecraft servers hosted on the PC: start, stop and restart them, see who's
// on and where, look in their inventory, give and take things, manage the
// whitelist and the player limit, follow everyone on the live map, and type
// server commands — from the phone.

import { useCallback, useEffect, useRef, useState } from 'react';
import { Ban, Copy, Crown, Gift, Heart, Home, Locate, LogOut, MapPin, Minus, Plus, Play, RefreshCw, RotateCw, SendHorizontal, Server, ShieldAlert, Square, Terminal, Trash2, UserCheck, UserMinus, Users, Utensils } from 'lucide-react';
import type { SrvAction, SrvGamemode, SrvItem, SrvKnown, SrvLine, SrvMap, SrvOnline, SrvPlayer, SrvPlayers, SrvView, SrvWaiting } from '@shared/types';
import { DIMENSIONS, Head, ItemTile, STATE, ago, agoText, busy, dimLabel, flavorLabel } from '@shared/minecraft';
import { client } from '../client';
import { useBackHandler } from '../lib/back';
import { useEvent } from '../lib/events';
import { copyToClipboard, cx, errorMessage, useInterval, usePageVisible, vibrate } from '../lib/util';
import { toast } from '../state';
import { PullToRefresh } from '../ui/PullToRefresh';
import { Sheet } from '../ui/Sheet';
import { Button, Empty, ErrorState, ProgressBar, Segmented, Skeleton, Switch } from '../ui/common';
import { SubHeader } from './More';

type Tab = 'players' | 'map' | 'console';
type Dim = 'overworld' | 'the_nether' | 'the_end';

const TONE: Record<string, string> = {
  good: 'bg-good/15 text-good',
  warn: 'bg-warn/15 text-warn',
  bad: 'bg-bad/15 text-bad',
  info: 'bg-accent-soft text-accent',
  neutral: 'bg-surface-2 text-dim',
};

const ITEMS = ['minecraft:diamond', 'minecraft:iron_ingot', 'minecraft:golden_apple', 'minecraft:cooked_beef', 'minecraft:torch', 'minecraft:oak_log', 'minecraft:diamond_pickaxe', 'minecraft:diamond_sword', 'minecraft:elytra', 'minecraft:firework_rocket', 'minecraft:ender_pearl', 'minecraft:experience_bottle', 'minecraft:totem_of_undying', 'minecraft:netherite_ingot', 'minecraft:bread'];

const QUICK = ['list', 'save-all', 'time set day', 'weather clear', 'difficulty normal'];

function Pill({ s }: { s: SrvView }) {
  const st = STATE[s.state];
  return <span className={cx('rounded-full px-2.5 py-0.5 text-[12px] font-semibold', TONE[st.tone])}>{s.state === 'running' ? `Online · ${s.online.length}/${s.maxPlayers}` : st.label}</span>;
}

function Icon({ s, size = 48 }: { s: SrvView; size?: number }) {
  return (
    <div className="relative grid shrink-0 place-items-center overflow-hidden rounded-2xl bg-surface-2 text-dim" style={{ width: size, height: size }}>
      {s.icon ? <img src={s.icon} alt="" className="h-full w-full object-cover [image-rendering:pixelated]" /> : <Server size={size * 0.45} />}
      <span className={cx('absolute bottom-1 right-1 h-2.5 w-2.5 rounded-full ring-2 ring-surface', s.state === 'running' ? 'bg-good' : s.state === 'crashed' ? 'bg-bad' : s.state === 'stopped' ? 'bg-faint' : 'bg-warn')} />
    </div>
  );
}

/** Buttons that ask for a second tap before something that's hard to undo. */
function useTwoStep() {
  const [armed, setArmed] = useState<string | null>(null);
  useEffect(() => {
    if (!armed) return;
    const t = setTimeout(() => setArmed(null), 3000);
    return () => clearTimeout(t);
  }, [armed]);
  const guard = (key: string, fn: () => void) => () => {
    if (armed === key) {
      setArmed(null);
      fn();
    } else {
      vibrate(8);
      setArmed(key);
    }
  };
  return { armed, guard };
}

export function ServersPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const visible = usePageVisible();
  const live = active && visible;
  const [list, setList] = useState<SrvView[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [open, setOpen] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>('players');
  const [doing, setDoing] = useState<string | null>(null);
  const [ask, setAsk] = useState<{ s: SrvView; what: 'stop' | 'restart' } | null>(null);

  const load = useCallback(async () => {
    try {
      setList(await client.servers());
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);
  useEffect(() => {
    void load();
  }, [load]);
  useEvent<{ id: string }>('mcserver:changed', () => void load());
  // Memory, CPU and who's on keep changing while a server runs.
  const anyUp = !!list?.some((s) => s.state !== 'stopped' && s.state !== 'crashed');
  useInterval(() => void load(), live && (open || list?.some(busy)) ? 3000 : live && anyUp ? 8000 : null);
  useBackHandler(active && !!open, () => setOpen(null));

  const run = async (s: SrvView, what: 'start' | 'stop' | 'restart') => {
    setAsk(null);
    setDoing(`${s.id}:${what}`);
    try {
      const v = await client.serverRun(s.id, what);
      setList((l) => l?.map((x) => (x.id === v.id ? v : x)) ?? l);
      vibrate(12);
      toast.success(what === 'start' ? `Starting ${s.name}` : what === 'stop' ? `Stopping ${s.name}` : `Restarting ${s.name}`, what === 'stop' ? 'The world is saved first.' : 'It takes a moment to be ready.');
    } catch (e) {
      toast.error(`Couldn't ${what} ${s.name}`, errorMessage(e));
    } finally {
      setDoing(null);
    }
  };
  // Stopping kicks everyone: ask first when people are playing.
  const runOrAsk = (s: SrvView, what: 'start' | 'stop' | 'restart') => (what !== 'start' && s.online.length > 0 ? setAsk({ s, what }) : void run(s, what));

  const s = list?.find((x) => x.id === open) ?? null;
  if (open && s) {
    return (
      <div className="flex h-full flex-col">
        <SubHeader title={s.name} subtitle={`${flavorLabel(s.flavor)} ${s.mc}`} onBack={() => setOpen(null)} />
        <div className="scroller min-h-0 flex-1">
          <div className="px-safe pb-tabbar">
            <Overview s={s} doing={doing} onRun={(w) => runOrAsk(s, w)} />
            <Segmented
              className="mt-4"
              value={tab}
              onChange={setTab}
              options={[
                { value: 'players', label: 'Players' },
                { value: 'map', label: 'Map' },
                { value: 'console', label: 'Console' },
              ]}
            />
            <div className="mt-4">
              {tab === 'players' && <PlayersTab s={s} live={live} onView={(v) => setList((l) => l?.map((x) => (x.id === v.id ? v : x)) ?? l)} onRestart={() => runOrAsk(s, 'restart')} />}
              {tab === 'map' && <MapTab s={s} live={live} />}
              {tab === 'console' && <ConsoleTab s={s} live={live} />}
            </div>
          </div>
        </div>
        <AskSheet ask={ask} onClose={() => setAsk(null)} onYes={(a) => void run(a.s, a.what)} />
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <SubHeader title="Minecraft servers" subtitle="Hosted on the PC" onBack={onBack} />
      <PullToRefresh onRefresh={load} className="min-h-0 flex-1">
        <div className="px-safe pb-tabbar">
          {error && !list ? (
            <ErrorState message={error} onRetry={() => void load()} />
          ) : !list ? (
            <div className="space-y-3">
              {[0, 1].map((i) => (
                <Skeleton key={i} className="h-[136px] rounded-3xl" />
              ))}
            </div>
          ) : list.length === 0 ? (
            <Empty icon={<Server size={28} />} title="No servers yet" body="Make one on the PC in OmniHub → Servers. It shows up here to start and manage from your phone." />
          ) : (
            <div className="space-y-3">
              {list.map((x) => (
                <section key={x.id} className="card overflow-hidden">
                  <button className="press flex w-full items-center gap-3.5 p-4 text-left active:bg-surface-2" onClick={() => (setTab('players'), setOpen(x.id))}>
                    <Icon s={x} />
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[17px] font-semibold">{x.name}</div>
                      <div className="mt-1 flex flex-wrap items-center gap-1.5 text-[13px] text-dim">
                        <Pill s={x} />
                        <span className="truncate">
                          {flavorLabel(x.flavor)} {x.mc}
                        </span>
                      </div>
                    </div>
                  </button>
                  {x.state === 'installing' && x.work && <Work s={x} />}
                  {x.problem && x.state !== 'running' && <p className="mx-4 mb-3 rounded-xl bg-bad/12 px-3 py-2 text-[13px] text-bad">{x.problem}</p>}
                  <div className="grid grid-cols-2 gap-2 px-4 pb-4">
                    {x.state === 'stopped' || x.state === 'crashed' ? (
                      <Button className="col-span-2" icon={<Play size={18} />} loading={doing === `${x.id}:start`} onClick={() => runOrAsk(x, 'start')}>
                        Start
                      </Button>
                    ) : (
                      <>
                        <Button variant="ghost" icon={<Square size={17} />} disabled={x.state === 'stopping' || x.state === 'installing'} loading={doing === `${x.id}:stop`} onClick={() => runOrAsk(x, 'stop')}>
                          Stop
                        </Button>
                        <Button variant="ghost" icon={<RotateCw size={17} />} disabled={busy(x)} loading={doing === `${x.id}:restart`} onClick={() => runOrAsk(x, 'restart')}>
                          Restart
                        </Button>
                      </>
                    )}
                  </div>
                </section>
              ))}
            </div>
          )}
          <p className="mt-4 px-2 text-center text-xs leading-relaxed text-faint">Servers keep running while the PC is on, even with OmniHub's window closed. Make servers and add mods, plugins and modpacks on the PC.</p>
        </div>
      </PullToRefresh>
      <AskSheet ask={ask} onClose={() => setAsk(null)} onYes={(a) => void run(a.s, a.what)} />
    </div>
  );
}

function AskSheet({ ask, onClose, onYes }: { ask: { s: SrvView; what: 'stop' | 'restart' } | null; onClose: () => void; onYes: (a: { s: SrvView; what: 'stop' | 'restart' }) => void }) {
  const n = ask?.s.online.length ?? 0;
  return (
    <Sheet
      open={!!ask}
      onClose={onClose}
      title={ask ? `${ask.what === 'stop' ? 'Stop' : 'Restart'} ${ask.s.name}?` : ''}
      subtitle={`${n} ${n === 1 ? 'person is' : 'people are'} playing.`}
      footer={
        <div className="flex gap-2.5">
          <Button variant="ghost" className="flex-1" onClick={onClose}>
            Keep it on
          </Button>
          <Button variant="danger" className="flex-1" onClick={() => ask && onYes(ask)}>
            {ask?.what === 'stop' ? 'Stop' : 'Restart'}
          </Button>
        </div>
      }
    >
      <p className="pb-2 text-[15px] text-dim">{ask?.what === 'stop' ? 'Everyone is disconnected. The world is saved first.' : 'Everyone is disconnected for a minute while it starts again. The world is saved first.'}</p>
    </Sheet>
  );
}

function Work({ s }: { s: SrvView }) {
  if (!s.work) return null;
  return (
    <div className="mx-4 mb-3 space-y-1.5">
      <div className="truncate text-[13px] text-dim">{s.work.text}</div>
      <ProgressBar value={s.work.total ? s.work.done / s.work.total : 0} active />
    </div>
  );
}

function Overview({ s, doing, onRun }: { s: SrvView; doing: string | null; onRun: (what: 'start' | 'stop' | 'restart') => void }) {
  const on = s.state === 'running';
  const copy = async () => {
    if (s.lan && (await copyToClipboard(s.lan))) toast.success('Address copied', 'Friends on your Wi-Fi join with it (Multiplayer → Direct Connection).');
  };
  return (
    <section className="card p-4">
      <div className="flex items-center gap-3.5">
        <Icon s={s} size={56} />
        <div className="min-w-0 flex-1">
          <Pill s={s} />
          <div className="mt-1 truncate text-[13px] text-dim">{on ? (Date.now() / 1000 - s.since < 60 ? 'Just started' : `Up for ${ago(s.since)}`) : s.state === 'stopped' ? 'Not running' : STATE[s.state].label}</div>
        </div>
      </div>
      {s.state === 'installing' && s.work && (
        <div className="-mx-4 mt-3">
          <Work s={s} />
        </div>
      )}
      {s.problem && !on && <p className="mt-3 rounded-xl bg-bad/12 px-3 py-2 text-[13px] text-bad">{s.problem}</p>}
      {on && (
        <div className="mt-3.5 grid grid-cols-3 gap-2 text-center">
          <div className="rounded-xl bg-surface-2 px-2 py-2">
            <div className="num text-[17px] font-bold">
              {s.online.length}/{s.maxPlayers}
            </div>
            <div className="text-[11px] text-dim">Players</div>
          </div>
          <div className="rounded-xl bg-surface-2 px-2 py-2">
            <div className="num text-[17px] font-bold">{s.memoryUsedMb >= 1024 ? `${(s.memoryUsedMb / 1024).toFixed(1)} GB` : `${s.memoryUsedMb} MB`}</div>
            <div className="text-[11px] text-dim">of {(s.memoryMb / 1024).toFixed(s.memoryMb % 1024 ? 1 : 0)} GB</div>
          </div>
          <div className="rounded-xl bg-surface-2 px-2 py-2">
            <div className="num text-[17px] font-bold">{Math.round(s.cpu)}%</div>
            <div className="text-[11px] text-dim">CPU</div>
          </div>
        </div>
      )}
      {s.lan && (
        <button onClick={() => void copy()} className="press mt-3 flex w-full items-center gap-2 rounded-xl bg-surface-2 px-3.5 py-2.5 text-left">
          <div className="min-w-0 flex-1">
            <div className="text-[11.5px] text-dim">Address on your Wi-Fi</div>
            <div className="truncate font-mono text-[14px]">{s.lan}</div>
          </div>
          <Copy size={16} className="text-faint" />
        </button>
      )}
      {s.restartNeeded && on && <p className="mt-3 rounded-xl bg-warn/12 px-3 py-2 text-[13px] text-warn">Some changes take effect after a restart.</p>}
      <div className="mt-3.5 grid grid-cols-2 gap-2">
        {s.state === 'stopped' || s.state === 'crashed' ? (
          <Button className="col-span-2" icon={<Play size={18} />} loading={doing === `${s.id}:start`} onClick={() => onRun('start')}>
            Start
          </Button>
        ) : (
          <>
            <Button variant="ghost" icon={<Square size={17} />} disabled={s.state === 'stopping' || s.state === 'installing'} loading={doing === `${s.id}:stop`} onClick={() => onRun('stop')}>
              Stop
            </Button>
            <Button variant={s.restartNeeded ? 'soft' : 'ghost'} icon={<RotateCw size={17} />} disabled={busy(s)} loading={doing === `${s.id}:restart`} onClick={() => onRun('restart')}>
              Restart
            </Button>
          </>
        )}
      </div>
    </section>
  );
}

// ---------- players ----------

function PlayersTab({ s, live, onView, onRestart }: { s: SrvView; live: boolean; onView: (v: SrvView) => void; onRestart: () => void }) {
  const [pv, setPv] = useState<SrvPlayers | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [add, setAdd] = useState('');
  const [adding, setAdding] = useState(false);
  const [max, setMax] = useState<number | null>(null);
  const [savingMax, setSavingMax] = useState(false);
  const [pick, setPick] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setPv(await client.serverPlayers(s.id));
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, [s.id]);
  useEffect(() => {
    void load();
  }, [load, s.online.length]);
  // Asking keeps the PC reading where everyone is.
  useInterval(() => void load(), live && s.state === 'running' ? 4000 : null);
  useEvent<{ id: string; online: SrvOnline[] }>('mcserver:players', (e) => e.id === s.id && setPv((p) => (p ? { ...p, online: e.online } : p)));
  useEvent<{ id: string }>('mcserver:guard', (e) => e.id === s.id && void load());

  const whitelist = async (on: boolean) => {
    try {
      onView(await client.serverWhitelist(s.id, on));
      void load();
    } catch (e) {
      toast.error('Not changed', errorMessage(e));
    }
  };
  const addName = async () => {
    const n = add.trim();
    if (!n) return;
    setAdding(true);
    try {
      await client.serverAct(s.id, n, { type: 'whitelist', on: true });
      toast.success(`${n} can join`, pv?.whitelistOn ? undefined : 'Turn the whitelist on so only these players can.');
      setAdd('');
      void load();
    } catch (e) {
      toast.error('Not added', errorMessage(e));
    } finally {
      setAdding(false);
    }
  };
  const joinMode = async (everyone: boolean) => {
    try {
      const v = await client.serverJoin(s.id, everyone);
      onView(v);
      void load();
      toast.success(everyone ? 'Everyone can join' : 'Paid accounts only', v.state === 'running' ? 'Restart the server for it to take effect. Nobody loses their things.' : 'From the next start. Nobody loses their things.');
    } catch (e) {
      toast.error('Not changed', errorMessage(e));
    }
  };
  const decide = async (w: SrvWaiting, allow: boolean) => {
    try {
      const { reply } = await client.serverAct(s.id, w.name, { type: allow ? 'allow' : 'refuse', place: w.place });
      toast.success(allow ? `${w.name} can join from there` : 'Kept out', allow ? reply : undefined);
      void load();
    } catch (e) {
      toast.error('Not done', errorMessage(e));
    }
  };
  const saveMax = async () => {
    if (max == null) return;
    setSavingMax(true);
    try {
      const v = await client.serverMaxPlayers(s.id, max);
      onView(v);
      setMax(null);
      toast.success(`Up to ${max} players`, v.restartNeeded && v.state === 'running' ? 'Restart the server for it to take effect.' : undefined);
      void load();
    } catch (e) {
      toast.error('Not changed', errorMessage(e));
    } finally {
      setSavingMax(false);
    }
  };

  if (error && !pv) return <ErrorState message={error} onRetry={() => void load()} />;
  if (!pv) return <Skeleton className="h-[220px] rounded-3xl" />;
  const shownMax = max ?? pv.maxPlayers;
  const offline = pv.known.filter((k) => !pv.online.some((o) => o.name.toLowerCase() === k.name.toLowerCase()));
  return (
    <div className="space-y-4">
      {pv.waiting.map((w) => (
        <section key={`${w.name}${w.place}`} className="card border-warn/40 p-4">
          <div className="flex items-start gap-3">
            <ShieldAlert size={20} className="mt-0.5 shrink-0 text-warn" />
            <div className="min-w-0 flex-1">
              <div className="text-[15px] font-semibold">{w.name} tried to join from a new place</div>
              <div className="font-mono text-[12px] text-faint">
                {w.place} · {agoText(w.at)}
              </div>
              <p className="mt-1 text-[13px] text-dim">That name is an operator, so they were kept out. Allow it only if it's really them.</p>
            </div>
          </div>
          <div className="mt-3 grid grid-cols-2 gap-2">
            <Button size="sm" icon={<UserCheck size={15} />} onClick={() => void decide(w, true)}>
              It's them
            </Button>
            <Button size="sm" variant="ghost" onClick={() => void decide(w, false)}>
              Not them
            </Button>
          </div>
        </section>
      ))}
      <section className="card divide-y divide-line">
        <div className="space-y-2.5 px-4 py-3.5">
          <div className="text-[15px] font-semibold">Who can join</div>
          <Segmented<'paid' | 'everyone'>
            value={pv.onlineMode ? 'paid' : 'everyone'}
            onChange={(v) => void joinMode(v === 'everyone')}
            options={[
              { value: 'paid', label: 'Paid accounts' },
              { value: 'everyone', label: 'Everyone' },
            ]}
          />
          <p className="text-[13px] text-dim">{pv.onlineMode ? 'Mojang checks every player. The safest choice.' : "Friends without a paid Minecraft account can join too. Names aren't checked, so OmniHub guards your operators: their names from a new place wait for your OK. Keep the whitelist on."}</p>
          {s.restartNeeded && s.state === 'running' && (
            <button className="text-[13px] font-semibold text-accent" onClick={onRestart}>
              Restart to apply changes
            </button>
          )}
        </div>
        <div className="flex items-center gap-3 px-4 py-3.5">
          <div className="min-w-0 flex-1">
            <div className="text-[15px] font-semibold">Whitelist</div>
            <div className="text-[13px] text-dim">{pv.whitelistOn ? 'Only players on it can join.' : 'Anyone with the address can join.'}</div>
          </div>
          <Switch on={pv.whitelistOn} onChange={(v) => void whitelist(v)} label="Whitelist" />
        </div>
        <div className="flex gap-2 px-4 py-3">
          <input className="field h-11 flex-1 py-0" value={add} onChange={(e) => setAdd(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && void addName()} placeholder="A friend's Minecraft name" autoCapitalize="off" autoCorrect="off" spellCheck={false} />
          <Button size="md" className="h-11" icon={<Plus size={17} />} disabled={!add.trim()} loading={adding} onClick={() => void addName()}>
            Add
          </Button>
        </div>
        <div className="flex items-center gap-3 px-4 py-3.5">
          <div className="min-w-0 flex-1">
            <div className="text-[15px] font-semibold">Max players</div>
            <div className="text-[13px] text-dim">How many can be on at once.</div>
          </div>
          <div className="flex items-center gap-1.5">
            <button aria-label="One fewer player" className="press grid h-9 w-9 place-items-center rounded-full bg-surface-2" onClick={() => setMax(Math.max(1, shownMax - 1))}>
              <Minus size={16} />
            </button>
            <input
              aria-label="Max players"
              inputMode="numeric"
              className="num h-9 w-14 rounded-xl bg-surface-2 text-center text-[16px] font-bold outline-none"
              value={shownMax}
              onChange={(e) => {
                const n = Number(e.target.value.replace(/\D/g, ''));
                setMax(Math.max(1, Math.min(1000, n || 1)));
              }}
            />
            <button aria-label="One more player" className="press grid h-9 w-9 place-items-center rounded-full bg-surface-2" onClick={() => setMax(Math.min(1000, shownMax + 1))}>
              <Plus size={16} />
            </button>
          </div>
        </div>
        {max != null && max !== pv.maxPlayers && (
          <div className="flex gap-2 px-4 py-3">
            <Button variant="ghost" className="flex-1" size="sm" onClick={() => setMax(null)}>
              Cancel
            </Button>
            <Button className="flex-1" size="sm" loading={savingMax} onClick={() => void saveMax()}>
              Save {max}
            </Button>
          </div>
        )}
      </section>

      <section>
        <h2 className="mb-2 px-1 text-[13px] font-semibold uppercase tracking-[0.08em] text-faint">Online now</h2>
        <div className="card overflow-hidden">
          {pv.online.length === 0 && <p className="px-4 py-4 text-[14px] text-dim">{s.state === 'running' ? 'Nobody is playing right now.' : 'The server is off.'}</p>}
          {pv.online.map((o, n) => (
            <button key={o.name} onClick={() => setPick(o.name)} className={cx('press flex w-full items-center gap-3 px-4 py-3 text-left active:bg-surface-2', n > 0 && 'border-t border-line')}>
              <Head name={o.name} size={36} />
              <div className="min-w-0 flex-1">
                <div className="truncate text-[15px] font-semibold">{o.name}</div>
                <div className="flex items-center gap-1 truncate text-[12.5px] text-dim">
                  <MapPin size={12} />
                  {o.pos ? `${o.pos.map((v) => Math.round(v)).join(', ')} · ${dimLabel(o.dimension)}` : 'Finding them…'}
                </div>
              </div>
              <span className="text-[12px] text-faint">{ago(o.since)}</span>
            </button>
          ))}
        </div>
      </section>

      {offline.length > 0 && (
        <section>
          <h2 className="mb-2 px-1 text-[13px] font-semibold uppercase tracking-[0.08em] text-faint">Everyone else</h2>
          <div className="card overflow-hidden">
            {offline.map((k, n) => (
              <button key={k.name} onClick={() => setPick(k.name)} className={cx('press flex w-full items-center gap-3 px-4 py-3 text-left active:bg-surface-2', n > 0 && 'border-t border-line')}>
                <Head name={k.name} size={32} />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[15px] font-medium">{k.name}</div>
                  <div className="flex gap-1 text-[12px] text-faint">
                    {[k.op && 'operator', k.whitelisted && 'whitelisted', k.banned && 'banned'].filter(Boolean).join(' · ') || 'offline'}
                  </div>
                </div>
              </button>
            ))}
          </div>
        </section>
      )}

      <PlayerSheet s={s} name={pick} known={pv.known.find((k) => k.name.toLowerCase() === pick?.toLowerCase())} onClose={() => setPick(null)} onChanged={() => void load()} />
    </div>
  );
}

function Bar({ value, max, color, label }: { value: number | null; max: number; color: string; label: string }) {
  if (value === null) return null;
  return (
    <div className="flex-1 space-y-1">
      <div className="flex justify-between text-[12px] text-dim">
        <span>{label}</span>
        <span className="num">
          {Math.round(value)}/{max}
        </span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-surface-3">
        <div className="h-full rounded-full" style={{ width: `${Math.max(0, Math.min(100, (value / max) * 100))}%`, background: color }} />
      </div>
    </div>
  );
}

function PlayerSheet({ s, name, known, onClose, onChanged }: { s: SrvView; name: string | null; known: SrvKnown | undefined; onClose: () => void; onChanged: () => void }) {
  const [p, setP] = useState<SrvPlayer | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [picked, setPicked] = useState<SrvItem | null>(null);
  const [item, setItem] = useState('minecraft:diamond');
  const [count, setCount] = useState(1);
  const [msg, setMsg] = useState('');
  const [busyAct, setBusyAct] = useState<string | null>(null);
  const { armed, guard } = useTwoStep();

  const load = useCallback(async () => {
    if (!name) return;
    try {
      setP(await client.serverPlayer(s.id, name));
      setErr(null);
    } catch (e) {
      setErr(errorMessage(e));
    }
  }, [s.id, name]);
  useEffect(() => {
    setP(null);
    setErr(null);
    setPicked(null);
    void load();
  }, [load]);
  useInterval(() => void load(), name && p?.online ? 3000 : null);

  const act = async (a: SrvAction, ok: string) => {
    if (!name) return;
    setBusyAct(a.type);
    try {
      const { reply } = await client.serverAct(s.id, name, a);
      vibrate(10);
      toast.success(ok, reply || undefined);
      void load();
      onChanged();
    } catch (e) {
      toast.error('Not done', errorMessage(e));
    } finally {
      setBusyAct(null);
    }
  };

  const online = !!p?.online && s.state === 'running';
  const at = (slot: number) => p?.items.find((i) => i.slot === slot) ?? null;
  const tile = (slot: number) => <ItemTile key={slot} it={at(slot)} size={31} onClick={online && at(slot) ? () => setPicked(at(slot)) : undefined} selected={picked?.slot === slot} />;
  const others = s.online.filter((o) => o.name !== name);

  return (
    <Sheet open={!!name} onClose={onClose} title={name ?? ''}>
      {err && !p ? (
        <ErrorState message={err} onRetry={() => void load()} />
      ) : !p ? (
        <div className="space-y-3 pb-4">
          <Skeleton className="h-16 rounded-2xl" />
          <Skeleton className="h-40 rounded-2xl" />
        </div>
      ) : (
        <div className="space-y-4 pb-3">
          <div className="flex items-center gap-3.5">
            <Head name={p.name} size={52} />
            <div className="min-w-0 flex-1">
              <div className="flex flex-wrap gap-1.5">
                {online ? <span className={cx('rounded-full px-2 py-0.5 text-[11.5px] font-semibold', TONE.good)}>Online</span> : <span className={cx('rounded-full px-2 py-0.5 text-[11.5px] font-semibold', TONE.neutral)}>Offline</span>}
                {known?.op && <span className={cx('rounded-full px-2 py-0.5 text-[11.5px] font-semibold', TONE.info)}>Operator</span>}
                {known?.whitelisted && <span className={cx('rounded-full px-2 py-0.5 text-[11.5px] font-semibold', TONE.info)}>Whitelisted</span>}
                {known?.banned && <span className={cx('rounded-full px-2 py-0.5 text-[11.5px] font-semibold', TONE.bad)}>Banned</span>}
              </div>
              <div className="mt-1 flex items-center gap-1 text-[13px] text-dim">
                <MapPin size={13} className="shrink-0" />
                <span className="truncate">{p.pos ? `${p.pos.map((v) => Math.round(v)).join(', ')} · ${dimLabel(p.dimension)}` : 'Never played here yet'}</span>
              </div>
              {!p.live && p.pos && <div className="text-[12px] text-faint">Where they left</div>}
            </div>
          </div>

          {(p.health !== null || p.food !== null) && (
            <div className="flex gap-4">
              <Bar value={p.health} max={20} color="#e5484d" label="Health" />
              <Bar value={p.food} max={20} color="#c8963e" label="Food" />
              <div className="w-14 text-right text-[12px] text-dim">
                Level
                <div className="num text-[16px] font-bold text-fg">{p.xpLevel ?? 0}</div>
              </div>
            </div>
          )}
          {online && (
            <Segmented<SrvGamemode>
              value={p.gamemode ?? 'survival'}
              onChange={(m) => void act({ type: 'gamemode', mode: m }, 'Game mode changed')}
              options={[
                { value: 'survival', label: 'Survival' },
                { value: 'creative', label: 'Creative' },
                { value: 'adventure', label: 'Adv.' },
                { value: 'spectator', label: 'Spect.' },
              ]}
            />
          )}

          <div>
            <div className="mb-1.5 flex items-baseline justify-between">
              <h3 className="text-[13px] font-semibold uppercase tracking-[0.08em] text-faint">Inventory</h3>
              <span className="text-[12px] text-faint">{online ? 'Live · tap a stack' : 'As they left it'}</span>
            </div>
            <div className="grid w-fit grid-cols-9 gap-[3px]">{Array.from({ length: 27 }, (_, i) => tile(9 + i))}</div>
            <div className="mt-2 grid w-fit grid-cols-9 gap-[3px]">
              {Array.from({ length: 9 }, (_, i) => (
                <div key={i} className={cx('rounded-md', p.selected === i && 'ring-2 ring-warn')}>
                  {tile(i)}
                </div>
              ))}
            </div>
            <div className="mt-2 flex items-center gap-[3px]">
              <span className="mr-1.5 text-[12px] text-faint">Armour</span>
              {[103, 102, 101, 100].map(tile)}
              <span className="ml-3 mr-1.5 text-[12px] text-faint">Off hand</span>
              {tile(-106)}
            </div>
            {picked && (
              <div className="mt-2.5 flex items-center gap-2 rounded-xl bg-surface-2 px-3 py-2">
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[14px] font-semibold">
                    {picked.custom ?? picked.label} × {picked.count}
                  </div>
                  <div className="truncate font-mono text-[11px] text-faint">
                    {picked.id}
                    {picked.enchanted ? ' · enchanted' : ''}
                  </div>
                </div>
                {online && (
                  <Button size="sm" variant="danger" icon={<Trash2 size={15} />} loading={busyAct === 'clearSlot'} onClick={() => void act({ type: 'clearSlot', slot: picked.slot }, 'Taken away').then(() => setPicked(null))}>
                    Take
                  </Button>
                )}
              </div>
            )}
            {!picked && p.items.length > 0 && <p className="mt-2 text-[12.5px] text-faint">{p.items.length} stacks. Tap one to see what it is.</p>}
          </div>

          {online && (
            <div className="space-y-2">
              <h3 className="text-[13px] font-semibold uppercase tracking-[0.08em] text-faint">Give</h3>
              <div className="flex gap-2">
                <input className="field h-11 min-w-0 flex-1 py-0 font-mono text-[14px]" list="srv-items-phone" value={item} onChange={(e) => setItem(e.target.value)} autoCapitalize="off" autoCorrect="off" spellCheck={false} aria-label="Item" />
                <input className="field h-11 w-16 py-0 text-center" inputMode="numeric" value={count} onChange={(e) => setCount(Math.max(1, Math.min(6400, Number(e.target.value.replace(/\D/g, '')) || 1)))} aria-label="How many" />
              </div>
              <datalist id="srv-items-phone">
                {ITEMS.map((i) => (
                  <option key={i} value={i} />
                ))}
              </datalist>
              <Button className="w-full" icon={<Gift size={17} />} disabled={!item.trim()} loading={busyAct === 'give'} onClick={() => void act({ type: 'give', item: item.trim(), count }, `Gave ${count} ${item.split(':').pop()?.replace(/_/g, ' ')}`)}>
                Give
              </Button>
            </div>
          )}

          {online && (
            <div className="space-y-2">
              <h3 className="text-[13px] font-semibold uppercase tracking-[0.08em] text-faint">Help them</h3>
              <div className="grid grid-cols-3 gap-2">
                <Button size="sm" variant="ghost" icon={<Heart size={15} />} loading={busyAct === 'heal'} onClick={() => void act({ type: 'heal' }, 'Healed')}>
                  Heal
                </Button>
                <Button size="sm" variant="ghost" icon={<Utensils size={15} />} loading={busyAct === 'feed'} onClick={() => void act({ type: 'feed' }, 'Fed')}>
                  Feed
                </Button>
                <Button size="sm" variant="ghost" icon={<Home size={15} />} loading={busyAct === 'spawn'} onClick={() => void act({ type: 'spawn' }, 'Sent to spawn')}>
                  Spawn
                </Button>
              </div>
              {others.length > 0 && (
                <select className="field h-11 py-0" value="" onChange={(e) => e.target.value && void act({ type: 'teleportTo', player: e.target.value }, `Teleported to ${e.target.value}`)} aria-label="Teleport to a player">
                  <option value="">Teleport to a player…</option>
                  {others.map((o) => (
                    <option key={o.name} value={o.name}>
                      {o.name}
                    </option>
                  ))}
                </select>
              )}
              <div className="flex gap-2">
                <input className="field h-11 min-w-0 flex-1 py-0" value={msg} onChange={(e) => setMsg(e.target.value)} placeholder={`A private message to ${p.name}`} />
                <Button size="md" className="h-11" variant="soft" disabled={!msg.trim()} loading={busyAct === 'message'} onClick={() => void act({ type: 'message', text: msg }, 'Sent').then(() => setMsg(''))} aria-label="Send">
                  <SendHorizontal size={17} />
                </Button>
              </div>
            </div>
          )}

          <div className="grid grid-cols-2 gap-2 border-t border-line pt-4">
            {known?.whitelisted ? (
              <Button size="sm" variant="ghost" icon={<UserMinus size={15} />} onClick={() => void act({ type: 'whitelist', on: false }, 'Off the whitelist')}>
                Unwhitelist
              </Button>
            ) : (
              <Button size="sm" variant="ghost" icon={<UserCheck size={15} />} onClick={() => void act({ type: 'whitelist', on: true }, 'Whitelisted')}>
                Whitelist
              </Button>
            )}
            {known?.op ? (
              <Button size="sm" variant="ghost" icon={<Crown size={15} />} onClick={() => void act({ type: 'deop' }, 'No longer an operator')}>
                Remove op
              </Button>
            ) : (
              <Button size="sm" variant="ghost" icon={<Crown size={15} />} onClick={guard('op', () => void act({ type: 'op' }, 'Now an operator'))}>
                {armed === 'op' ? 'Tap again' : 'Make op'}
              </Button>
            )}
            {online && (
              <Button size="sm" variant="ghost" icon={<LogOut size={15} />} onClick={guard('kick', () => void act({ type: 'kick', reason: null }, 'Kicked'))}>
                {armed === 'kick' ? 'Tap again' : 'Kick'}
              </Button>
            )}
            {known?.banned ? (
              <Button size="sm" variant="ghost" icon={<Ban size={15} />} onClick={() => void act({ type: 'pardon' }, 'Unbanned')}>
                Unban
              </Button>
            ) : (
              <Button size="sm" variant="outline" className="border-bad/40 text-bad" icon={<Ban size={15} />} onClick={guard('ban', () => void act({ type: 'ban', reason: null }, 'Banned'))}>
                {armed === 'ban' ? 'Tap again to ban' : 'Ban'}
              </Button>
            )}
          </div>
          <p className="text-center text-[12px] text-faint">Making someone an operator, kicking and banning ask for a second tap.</p>
        </div>
      )}
    </Sheet>
  );
}

// ---------- map ----------

function hue(name: string): number {
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) % 360;
  return h;
}

function MapTab({ s, live }: { s: SrvView; live: boolean }) {
  const [dim, setDim] = useState<Dim>('overworld');
  const [info, setInfo] = useState<SrvMap | null>(null);
  const [players, setPlayers] = useState<SrvOnline[]>([]);
  const [view, setView] = useState({ x: 0, z: 0, scale: 1 });
  const [follow, setFollow] = useState<string | null>(null);
  const [, redraw] = useState(0);
  const [size, setSize] = useState({ w: 0, h: 0 });
  const tiles = useRef(new Map<string, HTMLImageElement | 'loading' | 'missing'>());
  const canvas = useRef<HTMLCanvasElement>(null);
  const box = useRef<HTMLDivElement>(null);
  const pointers = useRef(new Map<number, { x: number; y: number }>());
  const gesture = useRef<{ x: number; y: number; vx: number; vz: number; dist: number; scale: number } | null>(null);
  const centered = useRef(false);

  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setSize({ w: el.clientWidth, h: el.clientHeight }));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  const load = useCallback(async () => {
    try {
      const m = await client.serverMap(s.id, dim);
      setInfo(m);
      setPlayers(m.players);
      if (!centered.current) {
        centered.current = true;
        const p = m.players[0]?.pos;
        const c = p ? [p[0], p[2]] : m.spawn ? [m.spawn[0], m.spawn[2]] : m.regions[0] ? [m.regions[0][0] * 512 + 256, m.regions[0][1] * 512 + 256] : [0, 0];
        setView((v) => ({ ...v, x: c[0], z: c[1] }));
      }
    } catch {
      setInfo((i) => i ?? { regions: [], spawn: null, players: [], supported: true });
    }
  }, [s.id, dim]);
  useEffect(() => {
    tiles.current.clear();
    centered.current = false;
    void load();
  }, [load]);
  // Players every 2 seconds (asking keeps the PC reading their positions);
  // the world's pictures again every 2 minutes.
  useInterval(() => void load(), live && s.state === 'running' ? 2000 : null);
  useInterval(() => {
    tiles.current.clear();
    redraw((n) => n + 1);
  }, live ? 120_000 : null);
  useEvent<{ id: string; online: SrvOnline[] }>('mcserver:players', (e) => e.id === s.id && setPlayers(e.online.filter((o) => (o.dimension ?? 'minecraft:overworld').replace('minecraft:', '') === dim)));

  useEffect(() => {
    if (!follow) return;
    const p = players.find((o) => o.name === follow)?.pos;
    if (p) setView((v) => ({ ...v, x: p[0], z: p[2] }));
  }, [players, follow]);

  const tile = (x: number, z: number) => {
    const k = `${dim}:${x}:${z}`;
    const t = tiles.current.get(k);
    if (t) return t;
    tiles.current.set(k, 'loading');
    client.serverTile(s.id, dim, x, z).then(
      (r) => {
        const img = new Image();
        img.onload = () => {
          tiles.current.set(k, img);
          redraw((n) => n + 1);
        };
        img.src = r.url;
      },
      () => tiles.current.set(k, 'missing'),
    );
    return 'loading';
  };

  useEffect(() => {
    const c = canvas.current;
    if (!c || !info || !size.w) return;
    const dpr = window.devicePixelRatio || 1;
    c.width = size.w * dpr;
    c.height = size.h * dpr;
    const g = c.getContext('2d')!;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.fillStyle = '#15171c';
    g.fillRect(0, 0, size.w, size.h);
    g.imageSmoothingEnabled = view.scale < 1;
    const sx = (bx: number) => (bx - view.x) * view.scale + size.w / 2;
    const sz = (bz: number) => (bz - view.z) * view.scale + size.h / 2;
    for (const [rx, rz] of info.regions) {
      const x0 = sx(rx * 512);
      const z0 = sz(rz * 512);
      const w = 512 * view.scale;
      if (x0 > size.w || z0 > size.h || x0 + w < 0 || z0 + w < 0) continue;
      const t = tile(rx, rz);
      if (t instanceof HTMLImageElement) g.drawImage(t, x0, z0, w, w);
      else if (t === 'loading') {
        g.fillStyle = 'rgba(255,255,255,.03)';
        g.fillRect(x0, z0, w, w);
      }
    }
    if (info.spawn && dim === 'overworld') {
      const x = sx(info.spawn[0]);
      const z = sz(info.spawn[2]);
      g.fillStyle = '#ffd34d';
      g.strokeStyle = '#1b1b1b';
      g.lineWidth = 2;
      g.beginPath();
      g.moveTo(x, z - 9);
      g.lineTo(x + 8, z - 2);
      g.lineTo(x + 5, z - 2);
      g.lineTo(x + 5, z + 7);
      g.lineTo(x - 5, z + 7);
      g.lineTo(x - 5, z - 2);
      g.lineTo(x - 8, z - 2);
      g.closePath();
      g.stroke();
      g.fill();
    }
    g.font = '600 12px Inter, system-ui, sans-serif';
    g.textAlign = 'center';
    for (const o of players) {
      if (!o.pos) continue;
      const x = sx(o.pos[0]);
      const z = sz(o.pos[2]);
      g.fillStyle = `hsl(${hue(o.name)} 70% 55%)`;
      g.strokeStyle = '#fff';
      g.lineWidth = 2.5;
      g.beginPath();
      g.roundRect(x - 10, z - 10, 20, 20, 5);
      g.fill();
      g.stroke();
      g.fillStyle = '#fff';
      g.fillText(o.name[0]?.toUpperCase() ?? '?', x, z + 4);
      const w = g.measureText(o.name).width + 12;
      g.fillStyle = 'rgba(10,12,16,.82)';
      g.beginPath();
      g.roundRect(x - w / 2, z - 34, w, 19, 6);
      g.fill();
      g.fillStyle = '#fff';
      g.fillText(o.name, x, z - 20);
    }
  });

  const zoom = (f: number) => setView((v) => ({ ...v, scale: Math.max(0.125, Math.min(16, v.scale * f)) }));
  // One finger moves the map, two pinch to zoom.
  const startGesture = () => {
    const pts = [...pointers.current.values()];
    const mid = pts.reduce((a, p) => ({ x: a.x + p.x / pts.length, y: a.y + p.y / pts.length }), { x: 0, y: 0 });
    const dist = pts.length > 1 ? Math.hypot(pts[0].x - pts[1].x, pts[0].y - pts[1].y) : 0;
    gesture.current = { x: mid.x, y: mid.y, vx: view.x, vz: view.z, dist, scale: view.scale };
  };

  return (
    <div className="space-y-3">
      <Segmented value={dim} onChange={(d) => (setDim(d), setFollow(null))} options={DIMENSIONS.map((d) => ({ value: d.id as Dim, label: d.label }))} />
      <div className="card relative overflow-hidden">
        <div
          ref={box}
          className="h-[56vh] min-h-[300px] touch-none"
          onPointerDown={(e) => {
            e.currentTarget.setPointerCapture(e.pointerId);
            pointers.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
            setFollow(null);
            startGesture();
          }}
          onPointerMove={(e) => {
            if (!pointers.current.has(e.pointerId)) return;
            pointers.current.set(e.pointerId, { x: e.clientX, y: e.clientY });
            const gs = gesture.current;
            if (!gs) return;
            const pts = [...pointers.current.values()];
            const mid = pts.reduce((a, p) => ({ x: a.x + p.x / pts.length, y: a.y + p.y / pts.length }), { x: 0, y: 0 });
            const scale = pts.length > 1 && gs.dist ? Math.max(0.125, Math.min(16, (gs.scale * Math.hypot(pts[0].x - pts[1].x, pts[0].y - pts[1].y)) / gs.dist)) : gs.scale;
            setView({ scale, x: gs.vx - (mid.x - gs.x) / scale, z: gs.vz - (mid.y - gs.y) / scale });
          }}
          onPointerUp={(e) => {
            pointers.current.delete(e.pointerId);
            if (pointers.current.size) startGesture();
            else gesture.current = null;
          }}
          onPointerCancel={(e) => {
            pointers.current.delete(e.pointerId);
            gesture.current = null;
          }}
        >
          <canvas ref={canvas} className="h-full w-full" style={{ width: size.w, height: size.h }} />
        </div>
        <div className="absolute right-2.5 top-2.5 flex flex-col gap-1.5">
          <button aria-label="Zoom in" className="press grid h-10 w-10 place-items-center rounded-full bg-black/60 text-white backdrop-blur" onClick={() => zoom(2)}>
            <Plus size={18} />
          </button>
          <button aria-label="Zoom out" className="press grid h-10 w-10 place-items-center rounded-full bg-black/60 text-white backdrop-blur" onClick={() => zoom(0.5)}>
            <Minus size={18} />
          </button>
          {info?.spawn && dim === 'overworld' && (
            <button aria-label="Spawn" className="press grid h-10 w-10 place-items-center rounded-full bg-black/60 text-white backdrop-blur" onClick={() => (setFollow(null), setView((v) => ({ ...v, x: info.spawn![0], z: info.spawn![2] })))}>
              <Home size={17} />
            </button>
          )}
          <button
            aria-label="Draw again"
            className="press grid h-10 w-10 place-items-center rounded-full bg-black/60 text-white backdrop-blur"
            onClick={() => {
              tiles.current.clear();
              void load();
              redraw((n) => n + 1);
            }}
          >
            <RefreshCw size={16} />
          </button>
        </div>
        <div className="pointer-events-none absolute bottom-2.5 left-2.5 rounded-lg bg-black/60 px-2 py-0.5 font-mono text-[11.5px] text-white">
          {Math.round(view.x)}, {Math.round(view.z)}
        </div>
        {info && info.regions.length === 0 && (
          <div className="pointer-events-none absolute inset-0 grid place-items-center px-8 text-center text-[14px] text-white/80">{info.supported ? 'Nothing explored here yet. The map fills in as players explore and the server saves.' : 'The map needs Minecraft 1.18 or newer. Positions still show in Players.'}</div>
        )}
      </div>
      {players.length > 0 ? (
        <div className="flex gap-2 overflow-x-auto pb-1">
          {players.map((o) => (
            <button key={o.name} onClick={() => setFollow(follow === o.name ? null : o.name)} className={cx('press flex shrink-0 items-center gap-2 rounded-full py-1.5 pl-1.5 pr-3 text-[13px] font-semibold', follow === o.name ? 'bg-accent-soft text-accent' : 'bg-surface-2')}>
              <Head name={o.name} size={24} className="rounded-full" />
              {o.name}
              <Locate size={13} className={follow === o.name ? '' : 'text-faint'} />
            </button>
          ))}
        </div>
      ) : (
        <p className="px-1 text-[13px] text-dim">{s.state === 'running' ? 'Nobody in this world right now.' : 'The server is off; this is the world as it was saved.'}</p>
      )}
      <p className="px-1 text-[12px] text-faint">The PC draws the map from the world's own files, so it works on every server type. Pinch to zoom; tap a player to follow them.</p>
    </div>
  );
}

// ---------- console ----------

const LEVEL: Record<SrvLine['level'], string> = {
  info: 'text-fg/85',
  warn: 'text-warn',
  error: 'text-bad',
  chat: 'text-accent',
  cmd: 'text-good',
};

function ConsoleTab({ s, live }: { s: SrvView; live: boolean }) {
  const [lines, setLines] = useState<SrvLine[]>([]);
  const [text, setText] = useState('');
  const [sending, setSending] = useState(false);
  const last = useRef<number | undefined>(undefined);
  const scroller = useRef<HTMLDivElement>(null);
  const stick = useRef(true);

  const load = useCallback(async () => {
    try {
      const more = await client.serverConsole(s.id, last.current);
      if (!more.length) return;
      last.current = more[more.length - 1].n;
      setLines((l) => [...l, ...more].slice(-400));
    } catch {
      /* the next poll tries again */
    }
  }, [s.id]);
  useEffect(() => {
    last.current = undefined;
    setLines([]);
    void load();
  }, [load]);
  useInterval(() => void load(), live && s.state !== 'stopped' ? 1500 : null);
  useEffect(() => {
    const el = scroller.current;
    if (el && stick.current) el.scrollTop = el.scrollHeight;
  }, [lines]);

  const send = async (cmd = text) => {
    const t = cmd.trim().replace(/^\//, '');
    if (!t) return;
    setSending(true);
    try {
      await client.serverCommand(s.id, t);
      if (cmd === text) setText('');
      stick.current = true;
      setTimeout(() => void load(), 300);
    } catch (e) {
      toast.error('Not sent', errorMessage(e));
    } finally {
      setSending(false);
    }
  };

  const on = s.state === 'running' || s.state === 'starting';
  return (
    <div className="space-y-3">
      <div
        ref={scroller}
        onScroll={(e) => {
          const el = e.currentTarget;
          stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 40;
        }}
        className="card h-[48vh] min-h-[260px] overflow-y-auto overscroll-contain px-3 py-2.5 font-mono text-[11.5px] leading-[1.45]"
      >
        {lines.length === 0 && <div className="py-6 text-center font-sans text-[14px] text-dim">{on ? 'Waiting for the server…' : 'The server is off. Its messages show here while it runs.'}</div>}
        {lines.map((l) => (
          <div key={l.n} className={cx('whitespace-pre-wrap break-words', LEVEL[l.level])}>
            {l.text}
          </div>
        ))}
      </div>
      <div className="flex gap-2 overflow-x-auto pb-1">
        {QUICK.map((q) => (
          <button key={q} disabled={!on} onClick={() => void send(q)} className="press shrink-0 rounded-full bg-surface-2 px-3 py-1.5 font-mono text-[12.5px] disabled:opacity-40">
            {q}
          </button>
        ))}
      </div>
      <div className="flex gap-2">
        <div className="relative min-w-0 flex-1">
          <Terminal size={16} className="pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-faint" />
          <input
            className="field h-12 py-0 pl-10 font-mono text-[14px]"
            value={text}
            disabled={!on}
            onChange={(e) => setText(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && void send()}
            placeholder={on ? 'A server command, like say hi' : 'Start the server to send commands'}
            autoCapitalize="off"
            autoCorrect="off"
            spellCheck={false}
            enterKeyHint="send"
          />
        </div>
        <Button className="h-12 w-12 px-0" disabled={!on || !text.trim()} loading={sending} onClick={() => void send()} aria-label="Send">
          {!sending && <SendHorizontal size={18} />}
        </Button>
      </div>
      <p className="flex items-center gap-1.5 px-1 text-[12px] text-faint">
        <Users size={13} /> Commands run as the server, like typing them on the PC.
      </p>
    </div>
  );
}
