// The game library, Steam Deck style: every installed game as a cover in a
// big grid, with filters, search and controller navigation (D-pad / stick to
// move, A to open, B to close, Start to play). A game opens full screen
// (GameView). Friends online and wishlist sales sit beside it.

import type { LibraryGame, ShelfStatus, SteamAccount, SteamFriend, WishItem } from '@shared/types';
import { Gamepad2, Heart, KeyRound, Maximize2, Search, Sparkles, Star, Tag, Tv, Users } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api, errorText } from '../../../api';
import { Button } from '../../../components/ui/Button';
import { Badge, Card } from '../../../components/ui/Card';
import { Field, Segmented, TextInput } from '../../../components/ui/Form';
import { Modal } from '../../../components/ui/Overlay';
import { EmptyState } from '../../../components/ui/States';
import { cx } from '../../../lib/cx';
import { usePadInput } from '../../../lib/pads';
import { useEvent } from '../../../lib/hooks';
import { toast } from '../../../state/toasts';
import { BigPicture } from './BigPicture';
import { SitesDialog } from './SitesDialog';
import { GameView } from './GameView';

export const STATUS_LABEL: Record<ShelfStatus, string> = { '': 'No status', playing: 'Playing', backlog: 'Backlog', finished: 'Finished', dropped: 'Dropped' };

export const hours = (m: number) => (m >= 60 ? `${(m / 60).toFixed(m >= 600 ? 0 : 1)} h` : `${m} min`);

export function ago(ms: number): string {
  if (!ms) return 'Never';
  const d = (Date.now() - ms) / 86_400_000;
  return d < 1 ? 'Today' : d < 2 ? 'Yesterday' : d < 30 ? `${Math.floor(d)} days ago` : new Date(ms).toLocaleDateString();
}

/** A game's tall cover: its art, else its wide banner, else its name on colour. */
export function Cover({ game, className, bare }: { game: LibraryGame; className?: string; bare?: boolean }) {
  // Steam lacks the high-res cover for some games; its normal one usually exists.
  const tries = useMemo(() => {
    const c = game.art.cover;
    const lo = c?.includes('_2x.jpg') ? c.replace('_2x.jpg', '.jpg') : undefined;
    return [c, lo, game.art.header].filter(Boolean) as string[];
  }, [game.art.cover, game.art.header]);
  const [n, setN] = useState(0);
  useEffect(() => setN(0), [tries]);
  const hue = useMemo(() => [...game.name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 7), [game.name]);
  if (n < tries.length) {
    return <img src={tries[n]} alt="" loading="lazy" draggable={false} onError={() => setN(n + 1)} className={cx('h-full w-full object-cover', className)} />;
  }
  return (
    <div className={cx('flex h-full w-full items-end p-3', className)} style={{ background: `linear-gradient(160deg, hsl(${hue} 55% 38%), hsl(${(hue + 50) % 360} 60% 16%))` }}>
      {!bare && <span className="font-display text-[17px] font-bold leading-tight text-white drop-shadow transition-opacity group-hover:opacity-0 group-focus-visible:opacity-0">{game.name}</span>}
    </div>
  );
}

type Filter = 'all' | 'favorites' | 'playing' | 'backlog' | 'finished' | 'recent';

/** The library's controller input (shared with Big Picture: every controller type, Nintendo layout, dead zone). */
export function useGamepadNav(active: boolean, handlers: { move: (dx: number, dy: number) => void; ok: () => void; back: () => void; start: () => void; select?: () => void }) {
  usePadInput(active, handlers);
}

function KeysDialog({ open, onClose, account, onSaved }: { open: boolean; onClose: () => void; account: SteamAccount | null; onSaved: () => void }) {
  const [steam, setSteam] = useState('');
  const [sgdb, setSgdb] = useState('');
  const save = async (which: 'steam' | 'sgdb', key: string) => {
    try {
      await api.library.setKey(which, key);
      toast.success(key ? 'Key kept on this PC' : 'Key removed', key ? 'Encrypted for your Windows account. It never leaves this PC except to Steam / SteamGridDB.' : undefined);
      which === 'steam' ? setSteam('') : setSgdb('');
      onSaved();
    } catch (e) {
      toast.error('Couldn’t use that key', errorText(e));
    }
  };
  return (
    <Modal open={open} onClose={onClose} icon={KeyRound} title="Steam extras" description="Optional. Your keys stay on this PC, encrypted for your Windows account, and go only to Steam or SteamGridDB." size="md">
      <div className="space-y-4">
        <p className="text-[12.5px] text-dim">Signed in to Steam here as <b className="text-fg">{account?.user?.name || 'nobody'}</b>. Wishlist sales need nothing more; achievements and friends need your own Steam Web API key (and a public profile).</p>
        <Field label={`Steam Web API key ${account?.hasKey ? '(saved)' : ''}`} htmlFor="k-steam" hint="steamcommunity.com/dev/apikey">
          <div className="flex gap-2">
            <TextInput id="k-steam" type="password" value={steam} onChange={(e) => setSteam(e.target.value)} placeholder={account?.hasKey ? '••••••••' : '32 letters and digits'} />
            <Button disabled={!steam} onClick={() => void save('steam', steam)}>
              Save
            </Button>
            {account?.hasKey && (
              <Button variant="ghost" onClick={() => void save('steam', '')}>
                Remove
              </Button>
            )}
          </div>
        </Field>
        <Field label={`SteamGridDB key ${account?.hasSgdb ? '(saved)' : ''}`} htmlFor="k-sgdb" hint="steamgriddb.com → Preferences → API: artwork for non-Steam games">
          <div className="flex gap-2">
            <TextInput id="k-sgdb" type="password" value={sgdb} onChange={(e) => setSgdb(e.target.value)} placeholder={account?.hasSgdb ? '••••••••' : 'Your key'} />
            <Button disabled={!sgdb} onClick={() => void save('sgdb', sgdb)}>
              Save
            </Button>
            {account?.hasSgdb && (
              <Button variant="ghost" onClick={() => void save('sgdb', '')}>
                Remove
              </Button>
            )}
          </div>
        </Field>
      </div>
    </Modal>
  );
}

function FriendsCard({ account }: { account: SteamAccount | null }) {
  const [friends, setFriends] = useState<SteamFriend[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    if (!account?.hasKey) return;
    void api.library.friends().then(setFriends, (e) => setErr(errorText(e)));
  }, [account?.hasKey]);
  const online = (friends ?? []).filter((f) => f.state > 0);
  return (
    <Card className="p-4">
      <div className="mb-2 flex items-center gap-2 text-[13.5px] font-semibold text-fg">
        <Users size={15} className="text-accent" /> Friends online
      </div>
      {!account?.hasKey ? (
        <p className="text-[12px] text-faint">Add your Steam Web API key (Steam extras) to see who’s playing what.</p>
      ) : err ? (
        <p className="text-[12px] text-bad">{err}</p>
      ) : online.length === 0 ? (
        <p className="text-[12px] text-faint">{friends ? 'Nobody online right now.' : 'Loading…'}</p>
      ) : (
        <ul className="space-y-2">
          {online.slice(0, 12).map((f) => (
            <li key={f.steamid} className="flex items-center gap-2">
              {f.avatar ? <img src={f.avatar} alt="" className="h-8 w-8 rounded-lg" /> : <div className="h-8 w-8 rounded-lg bg-surface-2" />}
              <div className="min-w-0">
                <div className="truncate text-[13px] text-fg">{f.name}</div>
                <div className={cx('truncate text-[11.5px]', f.game ? 'text-good' : 'text-faint')}>{f.game ? `Playing ${f.game}` : 'Online'}</div>
              </div>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}

function WishlistCard({ account }: { account: SteamAccount | null }) {
  const [items, setItems] = useState<WishItem[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    if (!account?.user) return;
    void api.library.wishlist().then(setItems, (e) => setErr(errorText(e)));
  }, [account?.user]);
  return (
    <Card className="p-4">
      <div className="mb-2 flex items-center gap-2 text-[13.5px] font-semibold text-fg">
        <Tag size={15} className="text-accent" /> Wishlist
      </div>
      {!account?.user ? (
        <p className="text-[12px] text-faint">Sign in to Steam on this PC to see your wishlist. Sales show up as notifications.</p>
      ) : err ? (
        <p className="text-[12px] text-bad">{err}</p>
      ) : !items ? (
        <p className="text-[12px] text-faint">Loading…</p>
      ) : items.length === 0 ? (
        <p className="text-[12px] text-faint">Your wishlist is empty (or private).</p>
      ) : (
        <ul className="space-y-1.5">
          {items.slice(0, 10).map((w) => (
            <li key={w.app}>
              <button type="button" onClick={() => void api.app.openUrl(`https://store.steampowered.com/app/${w.app}`)} className="flex w-full items-center gap-2 rounded-lg px-1 py-1 text-left hover:bg-surface-2">
                <span className="min-w-0 flex-1 truncate text-[13px] text-fg">{w.name}</span>
                {w.discount > 0 && <Badge tone="good">−{w.discount}%</Badge>}
                <span className="text-[12px] tabular-nums text-dim">{w.price ?? ''}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </Card>
  );
}

export function LibraryTab() {
  const [games, setGames] = useState<LibraryGame[] | null>(null);
  const [account, setAccount] = useState<SteamAccount | null>(null);
  const [filter, setFilter] = useState<Filter>('all');
  const [q, setQ] = useState('');
  const [open, setOpen] = useState<string | null>(null);
  const [focus, setFocus] = useState(0);
  const [keys, setKeys] = useState(false);
  const [big, setBig] = useState(false);
  const [sitesOpen, setSitesOpen] = useState(false);
  const grid = useRef<HTMLDivElement>(null);
  const load = useCallback(() => {
    void api.library.games().then(setGames, (e) => toast.error('Couldn’t read your games', errorText(e)));
    void api.library.account().then(setAccount, () => undefined);
  }, []);
  useEffect(load, [load]);
  useEvent('library:changed', load);

  const shown = useMemo(() => {
    let v = games ?? [];
    if (q.trim()) v = v.filter((g) => g.name.toLowerCase().includes(q.trim().toLowerCase()));
    if (filter === 'favorites') v = v.filter((g) => g.favorite);
    else if (filter === 'playing' || filter === 'backlog' || filter === 'finished') v = v.filter((g) => g.status === filter);
    return [...v].sort((a, b) => (filter === 'recent' ? b.lastPlayed - a.lastPlayed : Number(b.favorite) - Number(a.favorite) || a.name.localeCompare(b.name)));
  }, [games, q, filter]);
  // The latest game with background art, else the latest game.
  const recent = useMemo(() => {
    const played = [...(games ?? [])].filter((g) => g.lastPlayed).sort((a, b) => b.lastPlayed - a.lastPlayed);
    return played.slice(0, 3).find((g) => g.art.hero) ?? played[0];
  }, [games]);

  const columns = () => {
    const el = grid.current;
    if (!el || el.children.length < 2) return 1;
    const first = (el.children[0] as HTMLElement).offsetTop;
    let n = 0;
    while (n < el.children.length && (el.children[n] as HTMLElement).offsetTop === first) n++;
    return Math.max(1, n);
  };
  useGamepadNav(!open && !keys && !big, {
    move: (dx, dy) =>
      setFocus((f) => {
        const next = Math.min(shown.length - 1, Math.max(0, f + dx + dy * columns()));
        (grid.current?.children[next] as HTMLElement | undefined)?.focus();
        return next;
      }),
    ok: () => shown[focus] && setOpen(shown[focus].key),
    back: () => undefined,
    start: () => shown[focus] && void api.library.play(shown[focus].key),
    select: () => setBig(true),
  });

  if (games && games.length === 0) return <EmptyState icon={Gamepad2} title="No games found" description="OmniHub looks for Steam, Epic, Riot, Minecraft and Roblox games. Install one, or add games from the Games tab." />;
  const current = games?.find((g) => g.key === open) ?? null;
  return (
    <div className="grid grid-cols-1 gap-5 2xl:grid-cols-[minmax(0,1fr)_300px]">
      <div className="min-w-0 space-y-4">
        {recent && filter === 'all' && !q && (
          <button type="button" onClick={() => setOpen(recent.key)} className="group relative block h-48 w-full overflow-hidden rounded-2xl border border-line text-left">
            {recent.art.hero ? <img src={recent.art.hero} alt="" className="absolute inset-0 h-full w-full object-cover transition-transform duration-700 group-hover:scale-105" onError={(e) => (e.currentTarget.style.display = 'none')} /> : <Cover game={recent} bare className="absolute inset-0" />}
            <div className="absolute inset-0 bg-gradient-to-r from-black/80 via-black/30 to-transparent" />
            <div className="absolute bottom-4 left-5">
              <div className="text-[12px] font-medium uppercase tracking-wider text-white/70">Jump back in</div>
              <div className="font-display text-[26px] font-bold text-white">{recent.name}</div>
              <div className="text-[12.5px] text-white/70">
                {hours(recent.minutes)} · {ago(recent.lastPlayed)}
              </div>
            </div>
          </button>
        )}
        <div className="flex flex-wrap items-center gap-2">
          <Segmented
            label="Show"
            value={filter}
            onChange={setFilter}
            options={[
              { value: 'all', label: 'All' },
              { value: 'recent', label: 'Recent' },
              { value: 'favorites', label: 'Favourites' },
              { value: 'playing', label: 'Playing' },
              { value: 'backlog', label: 'Backlog' },
              { value: 'finished', label: 'Finished' },
            ]}
          />
          <div className="relative ml-auto">
            <Search size={14} className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-faint" />
            <TextInput aria-label="Search games" value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search" className="w-56 pl-8" />
          </div>
          <Button icon={Maximize2} onClick={() => setBig(true)} title="Full screen, for a controller (View button)">
            Big Picture
          </Button>
          <Button icon={Tv} onClick={() => setSitesOpen(true)} title="Netflix, YouTube, Crunchyroll… in Big Picture and on your phone">
            Sites
          </Button>
          <Button icon={KeyRound} onClick={() => setKeys(true)}>
            Steam extras
          </Button>
        </div>
        <div ref={grid} className="grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-4">
          {shown.map((g, i) => (
            <button
              key={g.key}
              type="button"
              onClick={() => setOpen(g.key)}
              onFocus={() => setFocus(i)}
              aria-label={`${g.name}${g.status ? `, ${STATUS_LABEL[g.status]}` : ''}`}
              className={cx('group relative aspect-[2/3] overflow-hidden rounded-xl border border-line bg-surface-2 text-left shadow-[0_10px_30px_-18px_rgba(0,0,0,0.8)] outline-none transition-transform duration-200 hover:-translate-y-1 hover:shadow-[0_18px_40px_-16px_var(--accent-glow)] focus-visible:-translate-y-1 focus-visible:ring-2 focus-visible:ring-accent', focus === i && 'ring-1 ring-accent/40')}
            >
              <Cover game={g} />
              <div className="absolute inset-x-0 bottom-0 bg-gradient-to-t from-black/85 to-transparent px-2.5 pb-2 pt-8 opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100">
                <div className="truncate text-[12.5px] font-semibold text-white">{g.name}</div>
                <div className="text-[11px] text-white/70">{g.minutes ? hours(g.minutes) : 'Not played yet'}</div>
              </div>
              <div className="absolute left-1.5 top-1.5 flex gap-1">
                {g.favorite && (
                  <span className="grid h-6 w-6 place-items-center rounded-full bg-black/60 text-pink-400">
                    <Heart size={12} fill="currentColor" />
                  </span>
                )}
                {g.status === 'playing' && (
                  <span className="grid h-6 w-6 place-items-center rounded-full bg-black/60 text-good">
                    <Star size={12} fill="currentColor" />
                  </span>
                )}
              </div>
            </button>
          ))}
        </div>
        {games && shown.length === 0 && <p className="py-8 text-center text-[13px] text-faint">No games match.</p>}
        <p className="flex items-center gap-1.5 text-[11.5px] text-faint">
          <Sparkles size={12} /> With a controller: D-pad or stick to move, A to open, B to go back, Start to play.
        </p>
      </div>
      <div className="space-y-4">
        <FriendsCard account={account} />
        <WishlistCard account={account} />
      </div>
      {current && <GameView game={current} onClose={() => setOpen(null)} onChanged={load} />}
      {big && games && <BigPicture games={games} onClose={() => setBig(false)} />}
      <SitesDialog open={sitesOpen} onClose={() => setSitesOpen(false)} />
      <KeysDialog open={keys} onClose={() => setKeys(false)} account={account} onSaved={load} />
    </div>
  );
}
