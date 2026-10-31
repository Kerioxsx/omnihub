// Aurora's music hub: a big clock, a search bar above the dock that finds
// songs, albums and artists (an artist's card with a short bio, top songs
// and an album carousel; an album's songs), a picker for where a song plays
// (Spotify, Apple Music, YouTube Music, SoundCloud, Amazon Music, or
// whichever app is playing), and "up next" beside the song: the rest of its
// album and more by the artist.
//
// The catalogue is looked up by the core (media/catalog.rs); playing hands
// the song to the service, the way a link would.

import type { CatAlbum, CatAlbumPage, CatArtist, CatArtistPage, CatResults, CatTrack, CatUpNext, MediaState, MusicSource, PlayItem, VisualSettings } from '@shared/types';
import type { LucideIcon } from 'lucide-react';
import { AnimatePresence, motion } from 'motion/react';
import { AudioLines, Check, ChevronDown, ChevronLeft, ChevronRight, Cloud, Disc3, Globe, ListMusic, Loader2, Music2, Play, Search, Shuffle, Sparkles, UserRound, X } from 'lucide-react';
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { patchVisuals } from '../../lib/aurora/settings';
import { cx } from '../../lib/cx';
import { fmtTime } from '../../lib/nowPlaying';
import { toast } from '../../state/toasts';

// ---------- sources ----------

export const SOURCES: { value: MusicSource; label: string; color: string; icon: LucideIcon; hint: string }[] = [
  { value: 'auto', label: 'Automatic', color: 'linear-gradient(135deg,#a855f7,#22d3ee)', icon: Sparkles, hint: 'The app playing now, else your music app' },
  { value: 'appleMusic', label: 'Apple Music', color: '#fa2d48', icon: Music2, hint: 'Opens the song in Apple Music' },
  { value: 'spotify', label: 'Spotify', color: '#1db954', icon: AudioLines, hint: 'Opens Spotify on the song (press play)' },
  { value: 'spotifyWeb', label: 'Spotify Web Player', color: '#1db954', icon: Globe, hint: 'Spotify in your browser' },
  { value: 'youtubeMusic', label: 'YouTube Music', color: '#ff0033', icon: Play, hint: 'Plays the song on YouTube Music' },
  { value: 'soundcloud', label: 'SoundCloud', color: '#ff5500', icon: Cloud, hint: 'Opens SoundCloud on the song' },
  { value: 'amazonMusic', label: 'Amazon Music', color: '#25d1da', icon: Music2, hint: 'Opens Amazon Music on the song' },
];
const sourceOf = (v: MusicSource) => SOURCES.find((s) => s.value === v) ?? SOURCES[0];

function SourceBadge({ source, size = 22 }: { source: MusicSource; size?: number }) {
  const s = sourceOf(source);
  const Icon = s.icon;
  return (
    <span className="grid shrink-0 place-items-center rounded-[7px] text-white" style={{ width: size, height: size, background: s.color }} aria-hidden>
      <Icon size={size * 0.6} fill={s.value === 'youtubeMusic' ? 'currentColor' : 'none'} />
    </span>
  );
}

function trackItem(t: CatTrack): PlayItem {
  return { kind: 'track', title: t.title, artist: t.artist, album: t.album };
}

/** Play in the chosen service; says where it went. */
async function playIn(item: PlayItem, source: MusicSource, app: string | null) {
  try {
    const to = await api.music.play(item, source, app);
    const label = item.kind === 'artist' ? item.name : item.title;
    toast.success(`${to === 'spotify' ? 'Opened' : 'Playing'} in ${sourceOf(to).label}`, label);
  } catch (e) {
    toast.error('Couldn’t play it', errorText(e));
  }
}

// ---------- the clock ----------

export function HubClock({ h24 }: { h24: boolean }) {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    // On the minute, then every minute.
    let t = setTimeout(function tick() {
      setNow(new Date());
      t = setTimeout(tick, 60_000 - (Date.now() % 60_000) + 20);
    }, 60_000 - (Date.now() % 60_000) + 20);
    return () => clearTimeout(t);
  }, []);
  const h = h24 ? now.getHours() : now.getHours() % 12 || 12;
  const text = `${h24 ? String(h).padStart(2, '0') : h}:${String(now.getMinutes()).padStart(2, '0')}`;
  return (
    <div className="pointer-events-none absolute inset-x-0 top-[3cqh] z-[5] flex flex-col items-center" data-aurora-clock aria-label={`Time ${text}`}>
      <div className="text-[clamp(11px,1.5cqh,15px)] font-medium tracking-wide text-white/70 [text-shadow:0_1px_10px_rgba(0,0,0,.6)]">{now.toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric' })}</div>
      <div
        className="font-display text-[clamp(44px,12cqh,150px)] font-extralight leading-[0.95] tabular-nums tracking-[-0.02em]"
        style={{ color: 'transparent', WebkitTextStroke: '1.5px color-mix(in srgb, var(--aurora-ink, #fff) 85%, transparent)', textShadow: '0 0 30px color-mix(in srgb, var(--aurora-ink, #fff) 35%, transparent)' }}
      >
        {text}
      </div>
    </div>
  );
}

// ---------- pieces ----------

function Cover({ src, className, round }: { src: string; className?: string; round?: boolean }) {
  const [bad, setBad] = useState(false);
  return src && !bad ? (
    <img src={src} alt="" loading="lazy" draggable={false} onError={() => setBad(true)} className={cx('object-cover', round ? 'rounded-full' : 'rounded-[10px]', className)} />
  ) : (
    <div className={cx('grid place-items-center bg-white/10', round ? 'rounded-full' : 'rounded-[10px]', className)}>
      <Music2 size={16} className="text-white/45" />
    </div>
  );
}

function SongRow({ t, onPlay, current, index }: { t: CatTrack; onPlay: () => void; current?: boolean; index?: number }) {
  return (
    <button type="button" onClick={onPlay} className={cx('group flex w-full items-center gap-3 rounded-xl px-2 py-1.5 text-left transition-colors', current ? 'bg-white/[0.14]' : 'hover:bg-white/[0.08]')}>
      {index !== undefined ? (
        <span className="w-5 shrink-0 text-center text-[12px] tabular-nums text-white/45">{current ? <AudioLines size={13} className="mx-auto text-white" /> : index + 1}</span>
      ) : (
        <span className="relative shrink-0">
          <Cover src={t.cover} className="h-10 w-10" />
          <span className="absolute inset-0 grid place-items-center rounded-[10px] bg-black/45 opacity-0 transition-opacity group-hover:opacity-100">
            <Play size={14} fill="currentColor" />
          </span>
        </span>
      )}
      <span className="min-w-0 flex-1">
        <span className={cx('block truncate text-[13px] font-semibold', current && 'text-white')}>
          {t.title}
          {t.explicit && <span className="ml-1.5 rounded-[3px] bg-white/25 px-1 text-[9px] font-bold text-black/80">E</span>}
        </span>
        <span className="block truncate text-[11.5px] text-white/55">
          {t.artist}
          {index === undefined && t.album ? ` · ${t.album}` : ''}
        </span>
      </span>
      {t.durationMs > 0 && <span className="shrink-0 text-[11.5px] tabular-nums text-white/40">{fmtTime(t.durationMs)}</span>}
    </button>
  );
}

const KIND: Record<string, string> = { album: 'Album', single: 'Single', ep: 'EP', compile: 'Compilation' };

/** Albums in a 3D carousel: the one in the middle faces you; wheel, arrows or a click to move, click it to open. */
function AlbumCarousel({ albums, onOpen }: { albums: CatAlbum[]; onOpen: (a: CatAlbum) => void }) {
  const [at, setAt] = useState(0);
  useEffect(() => setAt(0), [albums]);
  const move = (d: number) => setAt((i) => Math.max(0, Math.min(albums.length - 1, i + d)));
  const lastWheel = useRef(0);
  if (!albums.length) return null;
  return (
    <div
      className="relative h-[clamp(170px,26cqh,230px)] select-none [perspective:900px]"
      onWheel={(e) => {
        if (Date.now() - lastWheel.current < 140) return;
        lastWheel.current = Date.now();
        move((Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY) > 0 ? 1 : -1);
      }}
    >
      {albums.map((a, i) => {
        const d = i - at;
        if (Math.abs(d) > 4) return null;
        return (
          <motion.button
            key={a.id}
            type="button"
            onClick={() => (d === 0 ? onOpen(a) : setAt(i))}
            className="absolute left-1/2 top-0 w-[clamp(110px,17cqh,150px)] -translate-x-1/2 text-center"
            animate={{ x: `calc(-50% + ${d * 62}%)`, rotateY: d === 0 ? 0 : d < 0 ? 42 : -42, scale: d === 0 ? 1 : 0.82, z: d === 0 ? 0 : -60, opacity: 1 - Math.abs(d) * 0.16 }}
            transition={{ type: 'spring', stiffness: 260, damping: 30 }}
            style={{ zIndex: 20 - Math.abs(d), transformStyle: 'preserve-3d' }}
            aria-label={d === 0 ? `Open ${a.title}` : a.title}
          >
            <Cover src={a.cover} className={cx('aspect-square w-full border border-white/15 shadow-[0_20px_40px_-15px_rgba(0,0,0,.9)]', d === 0 && 'ring-1 ring-white/40')} />
            <div className={cx('mt-1.5 truncate text-[12px] font-semibold transition-opacity', d === 0 ? 'opacity-100' : 'opacity-0')}>{a.title}</div>
            <div className={cx('truncate text-[10.5px] text-white/55 transition-opacity', d === 0 ? 'opacity-100' : 'opacity-0')}>
              {[a.year, KIND[a.kind] ?? a.kind].filter(Boolean).join(' · ')}
            </div>
          </motion.button>
        );
      })}
      {at > 0 && (
        <button type="button" onClick={() => move(-1)} className="absolute left-0 top-[35%] z-30 grid h-8 w-8 place-items-center rounded-full bg-black/40 hover:bg-black/60" aria-label="Previous album">
          <ChevronLeft size={16} />
        </button>
      )}
      {at < albums.length - 1 && (
        <button type="button" onClick={() => move(1)} className="absolute right-0 top-[35%] z-30 grid h-8 w-8 place-items-center rounded-full bg-black/40 hover:bg-black/60" aria-label="Next album">
          <ChevronRight size={16} />
        </button>
      )}
    </div>
  );
}

function Heading({ children, action }: { children: ReactNode; action?: ReactNode }) {
  return (
    <div className="mb-1.5 mt-3 flex items-center justify-between px-1 first:mt-0">
      <div className="text-[11px] font-bold uppercase tracking-[0.12em] text-white/50">{children}</div>
      {action}
    </div>
  );
}

// ---------- views inside the panel ----------

type View = { kind: 'results' } | { kind: 'artist'; id: string } | { kind: 'album'; id: string };

function useLoad<T>(load: (() => Promise<T>) | null, deps: unknown[]): { data: T | null; error: string | null } {
  const [state, setState] = useState<{ data: T | null; error: string | null }>({ data: null, error: null });
  useEffect(() => {
    if (!load) return;
    let alive = true;
    setState({ data: null, error: null });
    load().then(
      (data) => alive && setState({ data, error: null }),
      (e: unknown) => alive && setState({ data: null, error: errorText(e) }),
    );
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return state;
}

function Loading() {
  return (
    <div className="grid h-32 place-items-center text-white/50">
      <Loader2 size={20} className="animate-spin" />
    </div>
  );
}

function ArtistView({ id, play, open }: { id: string; play: (item: PlayItem) => void; open: (v: View) => void }) {
  const { data, error } = useLoad<CatArtistPage>(() => api.music.artist(id), [id]);
  if (error) return <div className="p-6 text-center text-[13px] text-white/60">{error}</div>;
  if (!data) return <Loading />;
  const { artist, bio, top, albums } = data;
  const shuffle = () => {
    const t = top[Math.floor(Math.random() * top.length)];
    if (t) play(trackItem(t));
  };
  return (
    <div className="grid gap-5 md:grid-cols-[minmax(0,300px)_1fr]">
      <div className="overflow-hidden rounded-[22px] border border-white/15 bg-gradient-to-b from-white/[0.12] to-white/[0.04] p-4">
        <Cover src={artist.picture} className="aspect-square w-[min(100%,clamp(110px,22cqh,220px))] rounded-[18px] shadow-[0_20px_50px_-20px_rgba(0,0,0,.9)]" />
        <div className="mt-3 text-[20px] font-bold leading-tight">{artist.name}</div>
        {artist.fans > 0 && <div className="text-[11.5px] text-white/50">{Intl.NumberFormat(undefined, { notation: 'compact' }).format(artist.fans)} fans</div>}
        {bio && <p className="mt-2 line-clamp-4 text-[12.5px] leading-snug text-white/70">{bio}</p>}
        <div className="mt-3 flex gap-2">
          <button type="button" disabled={!top.length} onClick={() => top[0] && play(trackItem(top[0]))} className="flex h-9 flex-1 items-center justify-center gap-1.5 rounded-full bg-white text-[13px] font-semibold text-black hover:scale-[1.02] disabled:opacity-40">
            <Play size={14} fill="currentColor" /> Play
          </button>
          <button type="button" disabled={!top.length} onClick={shuffle} className="flex h-9 flex-1 items-center justify-center gap-1.5 rounded-full bg-white/[0.14] text-[13px] font-semibold hover:bg-white/25 disabled:opacity-40">
            <Shuffle size={14} /> Shuffle
          </button>
        </div>
      </div>
      <div className="min-w-0">
        <Heading>Top songs</Heading>
        <div className="grid gap-0.5 lg:grid-cols-2">
          {top.slice(0, 10).map((t) => (
            <SongRow key={t.id} t={t} onPlay={() => play(trackItem(t))} />
          ))}
        </div>
        {albums.length > 0 && (
          <>
            <Heading>Albums and singles</Heading>
            <AlbumCarousel albums={albums} onOpen={(a) => open({ kind: 'album', id: a.id })} />
          </>
        )}
      </div>
    </div>
  );
}

function AlbumView({ id, play, open }: { id: string; play: (item: PlayItem) => void; open: (v: View) => void }) {
  const { data, error } = useLoad<CatAlbumPage>(() => api.music.album(id), [id]);
  if (error) return <div className="p-6 text-center text-[13px] text-white/60">{error}</div>;
  if (!data) return <Loading />;
  const { album, tracks } = data;
  return (
    <div className="grid gap-5 md:grid-cols-[minmax(0,260px)_1fr]">
      <div>
        <Cover src={album.cover} className="aspect-square w-[min(100%,clamp(110px,24cqh,240px))] rounded-[18px] border border-white/15 shadow-[0_20px_50px_-20px_rgba(0,0,0,.9)]" />
        <div className="mt-3 text-[18px] font-bold leading-tight">{album.title}</div>
        <button type="button" onClick={() => album.artistId && open({ kind: 'artist', id: album.artistId })} className="text-[13px] text-white/65 hover:text-white hover:underline">
          {album.artist}
        </button>
        <div className="text-[11.5px] text-white/45">{[KIND[album.kind] ?? album.kind, album.year, `${tracks.length} songs`].filter(Boolean).join(' · ')}</div>
        <div className="mt-3 flex gap-2">
          <button type="button" onClick={() => play({ kind: 'album', title: album.title, artist: album.artist })} className="flex h-9 flex-1 items-center justify-center gap-1.5 rounded-full bg-white text-[13px] font-semibold text-black hover:scale-[1.02]">
            <Play size={14} fill="currentColor" /> Play album
          </button>
        </div>
      </div>
      <div className="min-w-0">
        {tracks.map((t, i) => (
          <SongRow key={t.id} t={t} index={i} onPlay={() => play(trackItem(t))} />
        ))}
      </div>
    </div>
  );
}

function ResultsView({ results, play, open }: { results: CatResults; play: (item: PlayItem) => void; open: (v: View) => void }) {
  const { top, tracks, albums, artists } = results;
  if (!top && !tracks.length && !albums.length && !artists.length) return <div className="p-8 text-center text-[13px] text-white/55">Nothing found. Try the artist’s name or another spelling.</div>;
  return (
    <div className="grid gap-5 md:grid-cols-[minmax(0,280px)_1fr]">
      {top && (
        <div>
          <Heading>Top result</Heading>
          <button
            type="button"
            onClick={() => (top.kind === 'artist' ? open({ kind: 'artist', id: top.item.id }) : top.kind === 'album' ? open({ kind: 'album', id: top.item.id }) : play(trackItem(top.item)))}
            className="group w-full rounded-[22px] border border-white/15 bg-gradient-to-b from-white/[0.12] to-white/[0.04] p-4 text-left transition-colors hover:bg-white/[0.12]"
          >
            <Cover src={top.kind === 'artist' ? top.item.picture : top.item.cover} round={top.kind === 'artist'} className="h-28 w-28 shadow-[0_20px_40px_-15px_rgba(0,0,0,.9)]" />
            <div className="mt-3 truncate text-[22px] font-bold leading-tight">{top.kind === 'artist' ? top.item.name : top.item.title}</div>
            <div className="mt-0.5 flex items-center gap-1.5 text-[12px] text-white/55">
              {top.kind === 'artist' ? <UserRound size={12} /> : top.kind === 'album' ? <Disc3 size={12} /> : <Music2 size={12} />}
              {top.kind === 'artist' ? 'Artist' : top.kind === 'album' ? `Album · ${top.item.artist}` : `Song · ${top.item.artist}`}
            </div>
          </button>
          {artists.length > 0 && (
            <>
              <Heading>Artists</Heading>
              <div className="flex gap-3 overflow-x-auto pb-1">
                {artists.slice(0, 6).map((a: CatArtist) => (
                  <button key={a.id} type="button" onClick={() => open({ kind: 'artist', id: a.id })} className="w-[76px] shrink-0 text-center">
                    <Cover src={a.picture} round className="mx-auto h-[68px] w-[68px] border border-white/10" />
                    <div className="mt-1 truncate text-[11.5px] font-medium">{a.name}</div>
                  </button>
                ))}
              </div>
            </>
          )}
        </div>
      )}
      <div className="min-w-0">
        {tracks.length > 0 && (
          <>
            <Heading>Songs</Heading>
            <div className="grid gap-0.5 lg:grid-cols-2">
              {tracks.slice(0, 10).map((t) => (
                <SongRow key={t.id} t={t} onPlay={() => play(trackItem(t))} />
              ))}
            </div>
          </>
        )}
        {albums.length > 0 && (
          <>
            <Heading>Albums</Heading>
            <AlbumCarousel albums={albums} onOpen={(a) => open({ kind: 'album', id: a.id })} />
          </>
        )}
      </div>
    </div>
  );
}

// ---------- the source picker ----------

function SourcePicker({ value, onChange, onClose }: { value: MusicSource; onChange: (s: MusicSource) => void; onClose: () => void }) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 8, scale: 0.97 }}
      animate={{ opacity: 1, y: 0, scale: 1 }}
      exit={{ opacity: 0, y: 8, scale: 0.97 }}
      transition={{ duration: 0.16 }}
      className="absolute bottom-[calc(100%+10px)] left-0 z-40 w-64 overflow-hidden rounded-[18px] border border-white/15 bg-[#17171f]/85 p-1.5 shadow-[0_30px_80px_-20px_rgba(0,0,0,.95)] backdrop-blur-2xl"
      role="menu"
      aria-label="Play in"
    >
      <div className="px-2.5 pb-1 pt-1.5 text-[10.5px] font-bold uppercase tracking-[0.12em] text-white/45">Play songs in</div>
      {SOURCES.filter((s) => s.value !== 'auto').map((s) => (
        <button
          key={s.value}
          type="button"
          role="menuitemradio"
          aria-checked={value === s.value}
          onClick={() => {
            onChange(s.value);
            onClose();
          }}
          className={cx('flex w-full items-center gap-2.5 rounded-xl px-2.5 py-2 text-left text-[13px] font-medium', value === s.value ? 'bg-white/[0.14]' : 'hover:bg-white/[0.08]')}
          title={s.hint}
        >
          <SourceBadge source={s.value} />
          <span className="flex-1">{s.label}</span>
          {value === s.value && <Check size={14} />}
        </button>
      ))}
      <div className="my-1 h-px bg-white/10" />
      <button
        type="button"
        role="menuitemradio"
        aria-checked={value === 'auto'}
        onClick={() => {
          onChange('auto');
          onClose();
        }}
        className={cx('flex w-full items-center gap-2.5 rounded-xl px-2.5 py-2 text-left text-[13px] font-medium', value === 'auto' ? 'bg-white/[0.14]' : 'hover:bg-white/[0.08]')}
      >
        <Sparkles size={16} className="mx-[3px] text-white/80" />
        <span className="flex-1">
          Automatic
          <span className="block text-[11px] font-normal text-white/45">The app playing now, else your music app</span>
        </span>
        {value === 'auto' && <Check size={14} />}
      </button>
    </motion.div>
  );
}

// ---------- search bar + panel ----------

export function HubSearch({ settings, state, full, onBusy }: { settings: VisualSettings['hub']; state: MediaState | null; full: boolean; onBusy: (busy: boolean) => void }) {
  const [q, setQ] = useState('');
  const [open, setOpen] = useState(false);
  const [picker, setPicker] = useState(false);
  const [results, setResults] = useState<CatResults | null>(null);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [stack, setStack] = useState<View[]>([{ kind: 'results' }]);
  const input = useRef<HTMLInputElement>(null);
  const box = useRef<HTMLDivElement>(null);
  const view = stack[stack.length - 1];
  const source = settings.source;

  useEffect(() => onBusy(open || picker), [open, picker, onBusy]);

  // Search as you type (a moment after the last key).
  useEffect(() => {
    const text = q.trim();
    if (text.length < 2) {
      setResults(null);
      setError(null);
      return;
    }
    let alive = true;
    setSearching(true);
    const t = setTimeout(() => {
      api.music.search(text).then(
        (r) => {
          if (!alive) return;
          setResults(r);
          setError(null);
          setStack([{ kind: 'results' }]);
        },
        (e: unknown) => alive && setError(errorText(e)),
      ).finally(() => alive && setSearching(false));
    }, 320);
    return () => {
      alive = false;
      clearTimeout(t);
    };
  }, [q]);

  // "/" or Ctrl+F searches; Esc steps back, then closes.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing = (e.target as HTMLElement | null)?.closest('input, textarea, [contenteditable]');
      if ((e.key === '/' && !typing) || (e.key === 'f' && (e.ctrlKey || e.metaKey))) {
        e.preventDefault();
        setOpen(true);
        input.current?.focus();
      } else if (e.key === 'Escape' && (open || picker)) {
        e.preventDefault();
        e.stopImmediatePropagation();
        if (picker) setPicker(false);
        else if (stack.length > 1) setStack((s) => s.slice(0, -1));
        else {
          setOpen(false);
          input.current?.blur();
        }
      }
    };
    window.addEventListener('keydown', onKey, true);
    return () => window.removeEventListener('keydown', onKey, true);
  }, [open, picker, stack.length]);

  // A click outside closes it.
  useEffect(() => {
    if (!open && !picker) return;
    const onDown = (e: PointerEvent) => {
      if (box.current && !box.current.contains(e.target as Node)) {
        setOpen(false);
        setPicker(false);
      }
    };
    window.addEventListener('pointerdown', onDown);
    return () => window.removeEventListener('pointerdown', onDown);
  }, [open, picker]);

  const play = useCallback(
    (item: PlayItem) => {
      void playIn(item, source, state?.appName ?? null);
      setOpen(false);
    },
    [source, state?.appName],
  );
  const openView = (v: View) => setStack((s) => [...s, v]);
  const showPanel = open && (results || searching || error || view.kind !== 'results');
  const placeholder = `Search ${source === 'auto' ? 'music' : sourceOf(source).label}`;

  return (
    <div ref={box} className={cx('absolute inset-x-0 z-30 flex justify-center px-4', full ? 'bottom-[88px]' : 'bottom-[76px]')} data-aurora-search>
      <div className="relative w-[min(560px,100%)]">
        <AnimatePresence>
          {showPanel && (
            <motion.div
              key="panel"
              initial={{ opacity: 0, y: 14, scale: 0.98 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 14, scale: 0.98 }}
              transition={{ type: 'spring', stiffness: 380, damping: 34 }}
              className="absolute bottom-[calc(100%+10px)] left-1/2 w-[min(980px,94cqw)] -translate-x-1/2 overflow-hidden rounded-[26px] border border-white/[0.14] bg-[#0f0f16]/70 shadow-[0_40px_120px_-30px_rgba(0,0,0,.95),inset_0_1px_0_rgba(255,255,255,.08)] backdrop-blur-3xl"
              role="dialog"
              aria-label="Search results"
            >
              <div className="flex items-center gap-2 border-b border-white/10 px-4 py-2.5">
                {stack.length > 1 ? (
                  <button type="button" onClick={() => setStack((s) => s.slice(0, -1))} className="flex items-center gap-1 rounded-full px-2 py-1 text-[12.5px] text-white/75 hover:bg-white/10" aria-label="Back">
                    <ChevronLeft size={15} /> Back
                  </button>
                ) : (
                  <span className="px-1 text-[12.5px] text-white/60">{searching ? 'Searching…' : results ? `Results for “${q.trim()}”` : ''}</span>
                )}
                <span className="ml-auto flex items-center gap-1.5 text-[11.5px] text-white/45">
                  Plays in <SourceBadge source={source} size={16} /> {sourceOf(source).label}
                </span>
                <button type="button" onClick={() => setOpen(false)} className="grid h-7 w-7 place-items-center rounded-full hover:bg-white/10" aria-label="Close">
                  <X size={15} />
                </button>
              </div>
              <div className="max-h-[min(62cqh,620px)] overflow-y-auto p-4">
                {error ? (
                  <div className="p-6 text-center text-[13px] text-white/60">{error}</div>
                ) : view.kind === 'artist' ? (
                  <ArtistView key={view.id} id={view.id} play={play} open={openView} />
                ) : view.kind === 'album' ? (
                  <AlbumView key={view.id} id={view.id} play={play} open={openView} />
                ) : results ? (
                  <ResultsView results={results} play={play} open={openView} />
                ) : (
                  <Loading />
                )}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
        <AnimatePresence>{picker && <SourcePicker value={source} onChange={(s) => patchVisuals({ hub: { source: s } })} onClose={() => setPicker(false)} />}</AnimatePresence>
        <div className="flex h-11 items-center gap-1.5 rounded-full border border-white/[0.12] bg-[#101016]/55 pl-1.5 pr-3 shadow-[0_18px_50px_-18px_rgba(0,0,0,.9),inset_0_1px_0_rgba(255,255,255,.07)] backdrop-blur-2xl">
          <button type="button" onClick={() => setPicker((p) => !p)} aria-expanded={picker} className="flex h-8 items-center gap-1 rounded-full px-1.5 hover:bg-white/10" aria-label={`Play songs in: ${sourceOf(source).label}`} title={`Play songs in: ${sourceOf(source).label}`}>
            <SourceBadge source={source} />
            <ChevronDown size={13} className="text-white/60" />
          </button>
          <Search size={15} className="shrink-0 text-white/50" aria-hidden />
          <input
            ref={input}
            value={q}
            onChange={(e) => {
              setQ(e.target.value);
              setOpen(true);
            }}
            onFocus={() => setOpen(true)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && results) {
                const top = results.top;
                if (top?.kind === 'artist') openView({ kind: 'artist', id: top.item.id });
                else if (top?.kind === 'album') openView({ kind: 'album', id: top.item.id });
                else if (top) play(trackItem(top.item));
              }
            }}
            placeholder={placeholder}
            className="h-full min-w-0 flex-1 bg-transparent text-[13.5px] text-white outline-none placeholder:text-white/40"
            aria-label={placeholder}
            spellCheck={false}
          />
          {searching && <Loader2 size={14} className="animate-spin text-white/50" />}
          {q && (
            <button type="button" onClick={() => { setQ(''); input.current?.focus(); }} className="grid h-6 w-6 place-items-center rounded-full hover:bg-white/10" aria-label="Clear">
              <X size={13} />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

// ---------- up next ----------

const upNextCache = new Map<string, CatUpNext>();

export function HubUpNext({ state, source, full }: { state: MediaState | null; source: MusicSource; full: boolean }) {
  const key = state ? `${state.artist}\u0000${state.title}` : null;
  const [data, setData] = useState<CatUpNext | null>(key ? (upNextCache.get(key) ?? null) : null);
  const [collapsed, setCollapsed] = useState(false);
  useEffect(() => {
    if (!state || !key) return setData(null);
    const cached = upNextCache.get(key);
    if (cached) return setData(cached);
    setData(null);
    let alive = true;
    const t = setTimeout(() => {
      api.music.upNext(state.title, state.artist).then(
        (r) => {
          upNextCache.set(key, r);
          if (alive) setData(r);
        },
        () => undefined,
      );
    }, 600);
    return () => {
      alive = false;
      clearTimeout(t);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  const current = data?.track?.id;
  const albumTracks = data?.album?.tracks ?? [];
  const more = data?.more ?? [];
  const list = useMemo(() => [...albumTracks.map((t) => ({ t, from: 'album' as const })), ...more.slice(0, 8).map((t) => ({ t, from: 'more' as const }))], [albumTracks, more]);
  if (!state || !list.length) return null;
  const play = (t: CatTrack) => void playIn(trackItem(t), source, state.appName ?? null);
  return (
    <div className={cx('absolute left-4 z-20 hidden w-[min(270px,26cqw)] flex-col @[900px]:flex', full ? 'bottom-[150px] top-[18cqh]' : 'bottom-[130px] top-[16cqh]')} data-aurora-upnext>
      <div className="flex max-h-full flex-col overflow-hidden rounded-[22px] border border-white/[0.12] bg-[#101016]/45 shadow-[0_30px_80px_-30px_rgba(0,0,0,.9)] backdrop-blur-2xl">
        <button type="button" onClick={() => setCollapsed((c) => !c)} className="flex items-center gap-2 px-3.5 py-2.5 text-left" aria-expanded={!collapsed}>
          <ListMusic size={15} className="text-white/70" />
          <span className="min-w-0 flex-1">
            <span className="block text-[13px] font-semibold">Up next</span>
            <span className="block truncate text-[11px] text-white/50">{data?.album ? `From ${data.album.album.title}` : `More by ${state.artist}`}</span>
          </span>
          <ChevronDown size={14} className={cx('text-white/60 transition-transform', collapsed && '-rotate-90')} />
        </button>
        {!collapsed && (
          <div className="min-h-0 overflow-y-auto px-1.5 pb-2">
            {list.map(({ t, from }, i) => (
              <div key={`${from}-${t.id}`}>
                {from === 'more' && (i === 0 || list[i - 1].from === 'album') && <div className="px-2 pb-1 pt-2.5 text-[10.5px] font-bold uppercase tracking-[0.12em] text-white/40">More by {data?.artist?.name ?? state.artist}</div>}
                <SongRow t={t} current={t.id === current} onPlay={() => play(t)} />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
