// Big Picture: the library full screen, made for the couch. Rows of covers
// you move through with a controller (D-pad / stick, A to play, B to leave)
// or the keyboard (arrows, Enter, Esc). The focused game's art fills the
// screen behind and drifts slowly; covers glide and grow as you move.

import type { LibraryGame, Site } from '@shared/types';
import { Gamepad2, Play, Settings2, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { api, errorText } from '../../../api';
import { cx } from '../../../lib/cx';
import { setWindowFullscreen } from '../../../lib/nowPlaying';
import { toast } from '../../../state/toasts';
import { confirmBack, padPrefs, usePadInput, useConnectedPads } from '../../../lib/pads';
import { Cover, STATUS_LABEL, ago, hours } from './LibraryTab';
import { Glyph, PadPanel, applyLight } from './PadPanel';

const W = 176; // cover width, px
const GAP = 22;
const ROW_H = 350;

type Row = { title: string; games: LibraryGame[] };

let audio: AudioContext | null = null;
/** A soft tick (or a rising chime for "play"). */
function blip(kind: 'move' | 'play' | 'back') {
  if (!padPrefs().sounds) return;
  try {
    audio ??= new AudioContext();
    const t = audio.currentTime;
    const o = audio.createOscillator();
    const g = audio.createGain();
    o.type = 'sine';
    const [f1, f2, len, vol] = kind === 'move' ? [880, 990, 0.05, 0.025] : kind === 'play' ? [520, 1040, 0.35, 0.05] : [660, 440, 0.12, 0.03];
    o.frequency.setValueAtTime(f1, t);
    o.frequency.exponentialRampToValueAtTime(f2, t + len);
    g.gain.setValueAtTime(vol, t);
    g.gain.exponentialRampToValueAtTime(0.0001, t + len);
    o.connect(g).connect(audio.destination);
    o.start(t);
    o.stop(t + len + 0.02);
  } catch {
    /* no sound */
  }
}

/** The focused game's art behind everything, cross-fading as focus moves. */
function Backdrop({ game }: { game: LibraryGame | undefined }) {
  const [layers, setLayers] = useState<LibraryGame[]>([]);
  useEffect(() => {
    if (!game) return;
    // Wait a moment so scrolling fast doesn't flash every game's art.
    const t = setTimeout(() => setLayers((l) => (l[l.length - 1]?.key === game.key ? l : [...l.slice(-1), game])), 160);
    return () => clearTimeout(t);
  }, [game]);
  return (
    <div className="absolute inset-0 overflow-hidden">
      {layers.map((g) => (
        <div key={g.key} className="bp-fade absolute inset-0">
          {g.key.startsWith('site:') ? (
            <div className="bp-drift h-full w-full" style={{ background: `radial-gradient(70% 70% at 75% 30%, ${g.profileId ?? '#6366f1'}88, #050507 75%)` }} />
          ) : g.art.hero ? (
            <img src={g.art.hero} alt="" draggable={false} className="bp-drift h-full w-full object-cover" onError={(e) => (e.currentTarget.style.display = 'none')} />
          ) : (
            <Cover game={g} bare className="bp-drift scale-125 blur-3xl" />
          )}
        </div>
      ))}
      <div className="absolute inset-0 bg-gradient-to-t from-black via-black/55 to-black/10" />
      <div className="absolute inset-0 bg-gradient-to-r from-black/85 via-black/30 to-transparent" />
    </div>
  );
}

function Clock() {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const t = setInterval(() => setNow(new Date()), 15_000);
    return () => clearInterval(t);
  }, []);
  return <span className="tabular-nums">{now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>;
}

function Hint({ k, label }: { k: string; label: string }) {
  return (
    <span className="flex items-center gap-2">
      <span className="grid h-7 min-w-7 place-items-center rounded-full bg-white/15 px-2 text-[12px] font-bold text-white">{k}</span>
      {label}
    </span>
  );
}

/** The list with `i` set to `v` (growing it when rows appear later). */
const setAt = (c: number[], i: number, v: number) => {
  const n = [...c];
  while (n.length <= i) n.push(0);
  n[i] = v;
  return n;
};

/** A quick site shown as a tile in the Watch row. */
const asTile = (s: Site): LibraryGame => ({ key: `site:${s.id}`, name: s.name, source: 'site', app: null, process: s.url, art: { cover: null, hero: null, logo: null, header: null }, status: '', favorite: false, minutes: 0, lastPlayed: 0, profileId: s.color, canLaunch: true });
const isSite = (g: LibraryGame) => g.key.startsWith('site:');

function SiteTile({ game }: { game: LibraryGame }) {
  const color = game.profileId ?? '#6366f1';
  return (
    <div className="flex h-full w-full flex-col justify-end p-4" style={{ background: `radial-gradient(120% 80% at 20% 0%, ${color}, #0b0b10 75%)` }}>
      <span className="font-display text-[24px] font-extrabold leading-tight drop-shadow-lg">{game.name}</span>
      <span className="mt-1 text-[12px] text-white/60">Opens full screen</span>
    </div>
  );
}

export function BigPicture({ games, onClose }: { games: LibraryGame[]; onClose: () => void }) {
  const [sites, setSites] = useState<Site[]>([]);
  useEffect(() => {
    void api.sites.list().then(setSites, () => undefined);
  }, []);
  const rows = useMemo<Row[]>(() => {
    const recent = games.filter((g) => g.lastPlayed).sort((a, b) => b.lastPlayed - a.lastPlayed).slice(0, 15);
    const favs = games.filter((g) => g.favorite).sort((a, b) => a.name.localeCompare(b.name));
    const playing = games.filter((g) => g.status === 'playing' && !g.favorite);
    const all = [...games].sort((a, b) => a.name.localeCompare(b.name));
    return [
      { title: 'Recently played', games: recent },
      { title: 'Favourites', games: favs },
      { title: 'Playing', games: playing },
      { title: 'All games', games: all },
      { title: 'Watch', games: sites.map(asTile) },
    ].filter((r) => r.games.length);
  }, [games, sites]);
  const [row, setRow] = useState(0);
  const [cols, setCols] = useState<number[]>(() => rows.map(() => 0));
  const [launching, setLaunching] = useState<LibraryGame | null>(null);
  const [leaving, setLeaving] = useState(false);
  const [panel, setPanel] = useState(false);
  const pads = useConnectedPads();
  const psKey = pads.filter((x) => x.family === 'playstation').map((x) => `${x.index}${x.id}`).join('|');
  // Colour PlayStation light bars when Big Picture opens or one connects.
  useEffect(() => {
    if (psKey) void applyLight();
  }, [psKey]);
  const col = cols[row] ?? 0;
  const focused = rows[row]?.games[col];

  useEffect(() => {
    void setWindowFullscreen(true);
    return () => void setWindowFullscreen(false);
  }, []);

  const close = () => {
    if (leaving) return;
    blip('back');
    setLeaving(true);
    setTimeout(onClose, 260);
  };
  const move = (dx: number, dy: number) => {
    if (launching) return;
    if (dy) {
      const next = Math.min(rows.length - 1, Math.max(0, row + dy));
      if (next !== row) {
        blip('move');
        setRow(next);
      }
      return;
    }
    const len = rows[row]?.games.length ?? 0;
    const next = Math.min(len - 1, Math.max(0, col + dx));
    if (next !== col) {
      blip('move');
      setCols((c) => setAt(c, row, next));
    }
  };
  const play = (g: LibraryGame | undefined = focused) => {
    if (!g || launching) return;
    if (isSite(g)) {
      blip('play');
      setLaunching(g);
      api.sites.open(g.process, true).then(
        () => setTimeout(() => setLaunching(null), 2200),
        (e) => {
          setLaunching(null);
          toast.error('Couldn’t open it', errorText(e));
        },
      );
      return;
    }
    if (!g.canLaunch) {
      toast.info('OmniHub can’t start this one', 'Open it from its own launcher.');
      return;
    }
    blip('play');
    setLaunching(g);
    api.library.play(g.key).then(
      () => setTimeout(() => setLaunching(null), 3200),
      (e) => {
        setLaunching(null);
        toast.error('Couldn’t start it', errorText(e));
      },
    );
  };

  const lastPad = usePadInput(
    !panel,
    {
      move,
      ok: () => play(),
      back: () => (launching ? setLaunching(null) : close()),
      start: () => play(),
      menu: () => setPanel(true),
      select: () => setPanel(true),
      bumper: (d) => move(d * 5, 0),
    },
    { haptics: true },
  );
  const family = lastPad?.family ?? pads[0]?.family ?? 'xbox';
  const cb = confirmBack(family);
  const side = (i: number) => (i === 1 ? 'right' : 'bottom') as 'right' | 'bottom';

  const ref = useRef({ move, play, close, panel });
  ref.current = { move, play, close, panel };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const map: Record<string, () => void> = {
        ArrowLeft: () => ref.current.move(-1, 0),
        ArrowRight: () => ref.current.move(1, 0),
        ArrowUp: () => ref.current.move(0, -1),
        ArrowDown: () => ref.current.move(0, 1),
        Enter: () => ref.current.play(),
        Escape: () => ref.current.close(),
      };
      if (ref.current.panel) return;
      if (e.key === 'c' || e.key === 'C') {
        e.preventDefault();
        setPanel(true);
        return;
      }
      const f = map[e.key];
      if (f) {
        e.preventDefault();
        e.stopPropagation();
        f();
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, []);
  // Mouse wheel moves along the row.
  const wheelAt = useRef(0);
  const onWheel = (e: React.WheelEvent) => {
    if (Date.now() - wheelAt.current < 90) return;
    wheelAt.current = Date.now();
    const d = Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
    if (e.shiftKey || Math.abs(e.deltaX) > Math.abs(e.deltaY)) move(d > 0 ? 1 : -1, 0);
    else move(0, d > 0 ? 1 : -1);
  };

  return (
    <div className={cx('fixed inset-0 z-[90] select-none overflow-hidden bg-black text-white', leaving ? 'bp-out' : 'bp-in')} onWheel={onWheel} role="dialog" aria-label="Big Picture">
      <Backdrop game={focused} />

      <header className="absolute inset-x-0 top-0 flex items-center justify-between px-14 pt-8 text-[17px] font-semibold text-white/85">
        <span className="flex items-center gap-3 font-display text-[22px] tracking-wide">
          <Gamepad2 size={26} className="text-accent" /> Library
        </span>
        <span className="flex items-center gap-6">
          <Clock />
          <button type="button" onClick={() => setPanel(true)} aria-label="Controllers" title="Controllers (C)" className="grid h-10 w-10 place-items-center rounded-full bg-white/10 transition hover:bg-white/20">
            <Settings2 size={19} />
          </button>
          <button type="button" onClick={close} aria-label="Leave Big Picture" className="grid h-10 w-10 place-items-center rounded-full bg-white/10 transition hover:bg-white/20">
            <X size={20} />
          </button>
        </span>
      </header>

      {focused && (
        <section key={focused.key} className="bp-rise absolute left-14 top-[16vh] max-w-[46vw]">
          {focused.art.logo ? (
            <img
              src={focused.art.logo}
              alt={focused.name}
              draggable={false}
              className="max-h-[17vh] max-w-[38vw] object-contain object-left drop-shadow-[0_6px_28px_rgba(0,0,0,0.8)]"
              onError={(e) => e.currentTarget.replaceWith(Object.assign(document.createElement('h1'), { textContent: focused.name, className: 'font-display text-[clamp(36px,5vw,72px)] font-extrabold leading-none drop-shadow-lg' }))}
            />
          ) : (
            <h1 className="font-display text-[clamp(36px,5vw,72px)] font-extrabold leading-none drop-shadow-lg">{focused.name}</h1>
          )}
          <div className="mt-5 flex flex-wrap items-center gap-x-5 gap-y-1 text-[16px] text-white/75">
            <span>{isSite(focused) ? 'Opens full screen in its own window, with no browser bars' : focused.minutes ? `${hours(focused.minutes)} played` : 'Not played yet'}</span>
            {focused.lastPlayed > 0 && <span>Last played {ago(focused.lastPlayed).toLowerCase()}</span>}
            {focused.status && <span className="rounded-full bg-white/15 px-3 py-0.5 text-[13px] font-semibold">{STATUS_LABEL[focused.status]}</span>}
          </div>
          <button
            type="button"
            onClick={() => play()}
            className="mt-7 flex items-center gap-3 rounded-2xl bg-accent px-9 py-4 text-[19px] font-bold text-white shadow-[0_12px_40px_-8px_var(--accent-glow)] transition hover:scale-[1.03] active:scale-95"
          >
            <Play size={22} fill="currentColor" /> Play
          </button>
        </section>
      )}

      <div className="absolute inset-x-0 bottom-[70px] overflow-visible" style={{ height: ROW_H }}>
        <div className="bp-ease" style={{ transform: `translateY(${-row * ROW_H}px)` }}>
          {rows.map((r, ri) => {
            const at = cols[ri] ?? 0;
            return (
              <div key={r.title} className={cx('bp-ease relative', ri === row ? 'opacity-100' : ri < row ? 'pointer-events-none opacity-0' : 'opacity-35')} style={{ height: ROW_H }}>
                <div className="px-14 pb-10 text-[18px] font-semibold tracking-wide text-white/90">{r.title}</div>
                <div className="bp-ease flex px-14" style={{ gap: GAP, transform: `translateX(${-at * (W + GAP)}px)` }}>
                  {r.games.map((g, gi) => {
                    const on = ri === row && gi === at;
                    return (
                      <button
                        key={g.key}
                        type="button"
                        tabIndex={-1}
                        onClick={() => {
                          if (on) play(g);
                          else {
                            setRow(ri);
                            setCols((c) => setAt(c, ri, gi));
                            blip('move');
                          }
                        }}
                        style={{ width: W, animationDelay: `${Math.min(gi, 8) * 45}ms` }}
                        className={cx(
                          'bp-card bp-ease relative aspect-[2/3] shrink-0 origin-bottom-left overflow-hidden rounded-2xl bg-white/5',
                          on ? 'z-10 scale-[1.14] shadow-[0_24px_60px_-12px_var(--accent-glow)] ring-[3px] ring-accent' : gi < at ? 'pointer-events-none scale-90 opacity-0' : 'scale-100 opacity-85 hover:opacity-100',
                        )}
                      >
                        {isSite(g) ? <SiteTile game={g} /> : <Cover game={g} />}
                        {on && <span className="bp-shine pointer-events-none absolute inset-0" />}
                      </button>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <footer className="absolute inset-x-0 bottom-0 flex items-center justify-end gap-8 bg-gradient-to-t from-black/80 to-transparent px-14 pb-7 pt-10 text-[14px] font-medium text-white/75">
        <Hint k="✥" label="Move" />
        <span className="flex items-center gap-2">
          <Glyph family={family} which={side(cb.ok)} /> Play
        </span>
        <span className="flex items-center gap-2">
          <Glyph family={family} which={side(cb.back)} /> Back
        </span>
        <span className="flex items-center gap-2">
          <Glyph family={family} which="top" /> Controllers
        </span>
      </footer>

      {panel && <PadPanel lastPad={lastPad} onClose={() => setPanel(false)} />}

      {launching && (
        <div className="bp-in absolute inset-0 z-20 grid place-items-center bg-black/75 backdrop-blur-md">
          <div className="flex flex-col items-center gap-7">
            <div className="bp-pulse relative w-[200px] overflow-hidden rounded-2xl ring-[3px] ring-accent">
              <div className="aspect-[2/3]">
                {isSite(launching) ? <SiteTile game={launching} /> : <Cover game={launching} />}
              </div>
            </div>
            <div className="text-center">
              <div className="text-[15px] uppercase tracking-[0.3em] text-white/60">{isSite(launching) ? 'Opening' : 'Starting'}</div>
              <div className="mt-1 font-display text-[34px] font-bold">{launching.name}</div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
