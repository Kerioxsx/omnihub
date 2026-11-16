// Extras: a sensitivity converter between shooters, Fortnite item-shop
// alerts, and a price tracker for PC parts.

import { formatRelative } from '@shared/format';
import type { PriceWatch, WatchStore } from '@shared/types';
import { Bell, CalendarClock, Crosshair, ExternalLink, Plus, Radio, RefreshCw, ShoppingBag, Tag, Trash } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Select, Switch, TextInput } from '../../components/ui/Form';
import { EmptyState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { toast } from '../../state/toasts';
import { AimTrainerCard } from './AimTrainer';
import { MouseRateCard, StretchedCard } from './StretchedCard';
import { DealsCard } from './DealsCard';
import { CrosshairCard } from './CrosshairCard';
import { GameAlertsCard, ReadyCard } from './GameAlertsCard';

/** Degrees the camera turns per mouse count at sensitivity 1 (Fortnite: per 1%). */
const GAMES: { id: string; name: string; yaw: number; unit?: string; digits: number }[] = [
  { id: 'cs2', name: 'Counter-Strike 2', yaw: 0.022, digits: 3 },
  { id: 'valorant', name: 'VALORANT', yaw: 0.07, digits: 3 },
  { id: 'fortnite', name: 'Fortnite', yaw: 0.005555, unit: '%', digits: 1 },
  { id: 'apex', name: 'Apex Legends', yaw: 0.022, digits: 2 },
  { id: 'ow2', name: 'Overwatch 2', yaw: 0.0066, digits: 2 },
  { id: 'cod', name: 'Call of Duty', yaw: 0.0066, digits: 2 },
];

function SensitivityCard() {
  const [from, setFrom] = useState('valorant');
  const [sens, setSens] = useState('0.35');
  const [dpi, setDpi] = useState('800');
  const [newDpi, setNewDpi] = useState('');
  const src = GAMES.find((g) => g.id === from) ?? GAMES[0];
  const s = Number(sens);
  const d = Number(dpi);
  const nd = Number(newDpi) || d;
  const ok = s > 0 && d > 0;
  // Degrees per inch of mouse movement stays the same everywhere.
  const degPerInch = ok ? s * src.yaw * d : 0;
  const cm360 = ok ? (360 / degPerInch) * 2.54 : 0;
  return (
    <Card className="p-5">
      <CardHeader icon={Crosshair} title="Sensitivity converter" subtitle="The same aim in every game: the mouse moves the same distance for a full turn. eDPI is sensitivity × DPI." />
      <div className="mt-4 flex flex-wrap items-end gap-3">
        <label className="space-y-1 text-[12px] text-dim">
          <span>From</span>
          <Select value={from} onChange={(e) => setFrom(e.target.value)} className="w-48">
            {GAMES.map((g) => (
              <option key={g.id} value={g.id}>
                {g.name}
              </option>
            ))}
          </Select>
        </label>
        <label className="space-y-1 text-[12px] text-dim">
          <span>Sensitivity{src.unit ? ` (${src.unit})` : ''}</span>
          <TextInput inputMode="decimal" value={sens} onChange={(e) => setSens(e.target.value)} className="w-28" />
        </label>
        <label className="space-y-1 text-[12px] text-dim">
          <span>Mouse DPI</span>
          <TextInput inputMode="numeric" value={dpi} onChange={(e) => setDpi(e.target.value)} className="w-24" />
        </label>
        <label className="space-y-1 text-[12px] text-dim">
          <span>New DPI (optional)</span>
          <TextInput inputMode="numeric" value={newDpi} onChange={(e) => setNewDpi(e.target.value)} placeholder={dpi} className="w-24" />
        </label>
        {ok && (
          <div className="ml-auto text-right">
            <div className="text-[11px] uppercase tracking-wider text-faint">Full turn</div>
            <div className="font-display text-[22px] font-bold tabular-nums">{cm360.toFixed(1)} cm</div>
          </div>
        )}
      </div>
      {ok && (
        <div className="mt-4 overflow-hidden rounded-xl border border-line">
          <table className="w-full text-[13px]">
            <thead className="bg-surface-2 text-left text-[11.5px] uppercase tracking-wider text-faint">
              <tr>
                <th className="px-3 py-2 font-semibold">Game</th>
                <th className="px-3 py-2 font-semibold">Sensitivity at {nd} DPI</th>
                <th className="px-3 py-2 font-semibold">eDPI</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-line">
              {GAMES.map((g) => {
                const v = degPerInch / (g.yaw * nd);
                return (
                  <tr key={g.id} className={cx(g.id === from && 'bg-accent/10')}>
                    <td className="px-3 py-2 font-medium">{g.name}</td>
                    <td className="px-3 py-2 tabular-nums">
                      <button type="button" title="Copy" className="hover:text-accent" onClick={() => void navigator.clipboard.writeText(v.toFixed(g.digits)).then(() => toast.success('Copied'))}>
                        {v.toFixed(g.digits)}
                        {g.unit ?? ''}
                      </button>
                    </td>
                    <td className="px-3 py-2 tabular-nums text-dim">{g.unit ? "—" : Math.round(v * nd)}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
      <p className="mt-3 text-[11.5px] text-faint">For hip-fire with the game’s default field of view. Fortnite’s figure is the common approximation (its percentage scale isn’t published). Turn off mouse acceleration in Windows and in the game for this to hold.</p>
    </Card>
  );
}

function ShopCard({ w, setW }: { w: WatchStore; setW: (w: WatchStore) => void }) {
  const [item, setItem] = useState('');
  const [busy, setBusy] = useState(false);
  const save = async (wanted: string[], alerts: boolean) => {
    try {
      setW(await api.watch.setShop(wanted, alerts));
    } catch (e) {
      toast.error('Couldn’t save that', errorText(e));
    }
  };
  const add = () => {
    const t = item.trim();
    if (t && !w.shopWanted.some((x) => x.toLowerCase() === t.toLowerCase())) void save([...w.shopWanted, t], w.shopAlerts);
    setItem('');
  };
  const check = async () => {
    setBusy(true);
    try {
      setW(await api.watch.checkShop());
    } catch (e) {
      toast.error('Couldn’t read the item shop', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const wanted = new Set(w.shopWanted.map((x) => x.toLowerCase()));
  const hit = (name: string) => [...wanted].some((x) => name.toLowerCase().includes(x));
  return (
    <Card className="p-5">
      <CardHeader
        icon={ShoppingBag}
        title="Fortnite item shop alerts"
        subtitle="A notification here and on your phone when something you want is in the shop. Checked once a day after the shop changes (midnight UTC). Shop data comes from fortnite-api.com, a fan-run service."
        actions={<Switch label="Item shop alerts" checked={w.shopAlerts} onChange={(a) => void save(w.shopWanted, a)} />}
      />
      <div className="mt-4 flex flex-wrap items-center gap-1.5">
        {w.shopWanted.map((x) => (
          <button key={x} type="button" title="Remove" className="rounded-full border border-line bg-surface-2 px-2.5 py-1 text-[12.5px] hover:border-bad/50 hover:text-bad" onClick={() => void save(w.shopWanted.filter((y) => y !== x), w.shopAlerts)}>
            {x} ×
          </button>
        ))}
        <TextInput value={item} onChange={(e) => setItem(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && add()} placeholder="Skin, emote, pickaxe…" className="h-8 w-52" aria-label="Item to watch" />
        <Button size="sm" variant="secondary" icon={Plus} disabled={!item.trim()} onClick={add}>
          Watch
        </Button>
        <Button size="sm" variant="ghost" icon={RefreshCw} loading={busy} className="ml-auto" onClick={() => void check()}>
          Check today’s shop
        </Button>
      </div>
      {w.shopNow.length > 0 && (
        <div className="mt-4">
          <div className="mb-1.5 text-[11.5px] uppercase tracking-wider text-faint">In the shop {w.shopDate}</div>
          <div className="flex max-h-40 flex-wrap gap-1.5 overflow-y-auto">
            {w.shopNow.map((n) => (
              <span key={n} className={cx('rounded-md px-2 py-0.5 text-[12px]', hit(n) ? 'bg-good/20 font-semibold text-good' : 'bg-surface-2 text-dim')}>
                {n}
              </span>
            ))}
          </div>
        </div>
      )}
    </Card>
  );
}

function money(v: number | null, c: string | null) {
  if (v == null) return '—';
  try {
    return new Intl.NumberFormat(undefined, { style: 'currency', currency: c || 'USD' }).format(v);
  } catch {
    return `${v.toFixed(2)}${c ? ` ${c}` : ''}`;
  }
}

function PriceCard({ w, setW }: { w: WatchStore; setW: (w: WatchStore) => void }) {
  const [url, setUrl] = useState('');
  const [name, setName] = useState('');
  const [target, setTarget] = useState('');
  const [busy, setBusy] = useState<string | null>(null);
  const run = async (what: string, f: () => Promise<WatchStore>) => {
    setBusy(what);
    try {
      setW(await f());
      return true;
    } catch (e) {
      toast.error('Couldn’t do that', errorText(e));
      return false;
    } finally {
      setBusy(null);
    }
  };
  const add = async () => {
    const watch: PriceWatch = { id: '', name: name.trim(), url: url.trim(), target: Number(target) || null, price: null, currency: null, lowest: null, history: [], checked: null, error: null };
    if (await run('add', () => api.watch.savePrice(watch))) {
      setUrl('');
      setName('');
      setTarget('');
    }
  };
  return (
    <Card className="p-5">
      <CardHeader
        icon={Tag}
        title="PC parts price tracker"
        subtitle="Paste a product page from any shop. OmniHub reads its price every 6 hours and tells you when it drops (3% or more) or falls under your target. Pages that hide their price from programs can’t be tracked."
        actions={
          w.prices.length > 0 ? (
            <Button size="sm" variant="ghost" icon={RefreshCw} loading={busy === 'check'} onClick={() => void run('check', api.watch.checkPrices)}>
              Check now
            </Button>
          ) : undefined
        }
      />
      <div className="mt-4 flex flex-wrap gap-2">
        <TextInput value={url} onChange={(e) => setUrl(e.target.value)} placeholder="https://… product page" className="min-w-[260px] flex-[2]" aria-label="Product page" />
        <TextInput value={name} onChange={(e) => setName(e.target.value)} placeholder="Name (optional)" className="min-w-[140px] flex-1" aria-label="Name" />
        <TextInput inputMode="decimal" value={target} onChange={(e) => setTarget(e.target.value)} placeholder="Target price" className="w-32" aria-label="Target price" />
        <Button variant="primary" icon={Plus} loading={busy === 'add'} disabled={!url.trim()} onClick={() => void add()}>
          Track
        </Button>
      </div>
      {w.prices.length === 0 ? (
        <EmptyState compact icon={Tag} title="Nothing tracked yet" description="A graphics card, an SSD, a monitor…" />
      ) : (
        <div className="mt-4 divide-y divide-line rounded-xl border border-line">
          {w.prices.map((p) => {
            const under = p.target != null && p.price != null && p.price <= p.target;
            return (
              <div key={p.id} className="flex flex-wrap items-center gap-3 px-4 py-3">
                <div className="min-w-[180px] flex-1">
                  <div className="flex items-center gap-2 text-[13.5px] font-medium">
                    {p.name}
                    {under && <Badge tone="good">Under your target</Badge>}
                  </div>
                  <div className="text-[11.5px] text-faint">
                    {p.error ? <span className="text-warn">{p.error}</span> : p.checked ? `Checked ${formatRelative(p.checked * 1000)}` : 'Not checked yet'}
                    {p.lowest != null && ` · lowest ${money(p.lowest, p.currency)}`}
                    {p.target != null && ` · target ${money(p.target, p.currency)}`}
                  </div>
                </div>
                <div className="font-display text-[18px] font-bold tabular-nums">{money(p.price, p.currency)}</div>
                <IconButton icon={ExternalLink} label="Open the page" size="sm" onClick={() => void api.app.openUrl(p.url)} />
                <IconButton icon={Trash} label="Stop tracking" size="sm" variant="danger" onClick={() => void run('rm', () => api.watch.removePrice(p.id))} />
              </div>
            );
          })}
        </div>
      )}
    </Card>
  );
}

function StreamersCard({ w, setW }: { w: WatchStore; setW: (w: WatchStore) => void }) {
  const [platform, setPlatform] = useState<'twitch' | 'youtube'>('twitch');
  const [handle, setHandle] = useState('');
  const [busy, setBusy] = useState(false);
  const add = async () => {
    try {
      setW(await api.watch.saveStreamer({ platform, handle: handle.trim() }));
      setHandle('');
    } catch (e) {
      toast.error('Couldn’t add them', errorText(e));
    }
  };
  const check = async () => {
    setBusy(true);
    try {
      setW(await api.watch.checkStreamers());
    } catch (e) {
      toast.error('Couldn’t check', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Card className="p-5">
      <CardHeader icon={Radio} title="When creators go live" subtitle="A notification here and on your phone when someone you follow starts streaming. Checked every 10 minutes." actions={w.streamers.length > 0 && <Button size="sm" variant="ghost" icon={RefreshCw} loading={busy} onClick={() => void check()}>Check now</Button>} />
      <div className="mt-4 flex flex-wrap items-center gap-2">
        <Select value={platform} onChange={(e) => setPlatform(e.target.value as 'twitch' | 'youtube')} className="w-auto" aria-label="Platform">
          <option value="twitch">Twitch</option>
          <option value="youtube">YouTube</option>
        </Select>
        <TextInput value={handle} onChange={(e) => setHandle(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && void add()} placeholder={platform === 'youtube' ? 'Channel handle, like @MrBeast' : 'Twitch name, like shroud'} className="min-w-[220px] flex-1" aria-label="Creator" />
        <Button variant="primary" icon={Plus} disabled={!handle.trim()} onClick={() => void add()}>
          Add
        </Button>
      </div>
      {w.streamers.length > 0 && (
        <div className="mt-3 divide-y divide-line rounded-xl border border-line">
          {w.streamers.map((st) => (
            <div key={st.id} className="flex items-center gap-3 px-3.5 py-2.5">
              <span className={cx('h-2 w-2 shrink-0 rounded-full', st.live ? 'animate-pulse bg-bad' : 'bg-surface-3')} aria-hidden />
              <div className="min-w-0 flex-1">
                <div className="truncate text-[13.5px] font-medium text-fg">
                  {st.handle} <span className="font-normal text-faint">· {st.platform === 'youtube' ? 'YouTube' : 'Twitch'}</span>
                </div>
                <div className="truncate text-[12px] text-dim">{st.live ? st.title || 'Live now' : 'Offline'}</div>
              </div>
              <IconButton icon={ExternalLink} label="Open their page" onClick={() => void api.app.openUrl(st.platform === 'youtube' ? `https://www.youtube.com/@${st.handle.replace(/^@/, '')}/live` : `https://www.twitch.tv/${st.handle.toLowerCase()}`)} />
              <IconButton icon={Trash} label="Remove" onClick={() => void api.watch.removeStreamer(st.id).then(setW)} />
            </div>
          ))}
        </div>
      )}
      <p className="mt-3 text-[11.5px] text-faint">Read from their public pages (no account or key needed), so a page change on Twitch or YouTube can delay an alert.</p>
    </Card>
  );
}

function RecapCard({ w, setW }: { w: WatchStore; setW: (w: WatchStore) => void }) {
  const save = async (time: string) => {
    try {
      setW(await api.watch.setRecap(time));
    } catch (e) {
      toast.error('Couldn’t save', errorText(e));
    }
  };
  const preview = async () => {
    try {
      const r = await api.watch.recapNow();
      toast.info(r.title, r.body);
    } catch (e) {
      toast.error('No recap', errorText(e));
    }
  };
  return (
    <Card className="p-5">
      <CardHeader icon={CalendarClock} title="Daily recap" subtitle="An evening summary on your phone: what you played, internet used, and your PC’s health." actions={<Button size="sm" variant="ghost" onClick={() => void preview()}>Show today’s</Button>} />
      <div className="mt-4 flex items-center gap-3 text-[13px] text-fg">
        Every day
        <Switch checked={!!w.recapTime} onChange={(v) => void save(v ? '21:00' : '')} label="Daily recap" />
        {w.recapTime && (
          <>
            at
            <TextInput type="time" value={w.recapTime} onChange={(e) => void save(e.target.value)} className="w-[136px]" aria-label="Recap time" />
          </>
        )}
      </div>
      <div className="mt-3 flex items-center gap-3 border-t border-line pt-3 text-[13px] text-fg">
        Weekly report, Sunday evening <span className="text-faint">(games, frame rates, clips, internet, health)</span>
        <Switch checked={w.weeklyOn} onChange={(v) => void api.watch.setWeekly(v).then(setW, (e: unknown) => toast.error('Couldn’t save', errorText(e)))} label="Weekly report" />
        <Button
          size="sm"
          variant="ghost"
          onClick={() =>
            void api.watch.weeklyNow().then(
              (r) => toast.info(r.title, r.body),
              (e: unknown) => toast.error('No report', errorText(e)),
            )
          }
        >
          Show this week’s
        </Button>
      </div>
    </Card>
  );
}

export function ExtrasTab() {
  const [w, setW] = useState<WatchStore | null>(null);
  useEffect(() => {
    void api.watch.get().then(setW, () => undefined);
  }, []);
  const content = useMemo(
    () =>
      w && (
        <>
          <GameAlertsCard w={w} setW={setW} />
          <StreamersCard w={w} setW={setW} />
          <RecapCard w={w} setW={setW} />
          <ShopCard w={w} setW={setW} />
          <PriceCard w={w} setW={setW} />
        </>
      ),
    [w],
  );
  return (
    <div className="space-y-4">
      <ReadyCard />
      <SensitivityCard />
      <AimTrainerCard />
      <MouseRateCard />
      <StretchedCard />
      <DealsCard />
      <CrosshairCard />
      {content}
      <p className="flex items-center gap-1.5 px-1 text-[11.5px] text-faint">
        <Bell size={12} /> Alerts also reach your phone while OmniHub’s page is open on it.
      </p>
    </div>
  );
}
