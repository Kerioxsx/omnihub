// One game, full screen (like the Steam Deck's game page): its background
// art and logo, Play, playtime, players right now, achievements, your
// screenshots, its soundtrack, mods, sessions, save backups, notes and art.
// B or Esc goes back; LB/RB switch sections.

import type { GameDetail, LibraryGame, SaveBackup, ShelfStatus, SteamAchievement } from '@shared/types';
import { motion } from 'motion/react';
import { ArrowLeft, Clock, Download, FolderPlus, Heart, Image as ImageIcon, Lock, Music, Pause, Play, Puzzle, RotateCcw, Save, Search, Trophy, Users, Wand2, X } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../../api';
import { Button, IconButton } from '../../../components/ui/Button';
import { Badge, Card } from '../../../components/ui/Card';
import { Select, Switch, Textarea, TextInput } from '../../../components/ui/Form';
import { Callout } from '../../../components/ui/States';
import { cx } from '../../../lib/cx';
import { navigate } from '../../../lib/router';
import { confirm } from '../../../state/dialogs';
import { toast } from '../../../state/toasts';
import { Cover, STATUS_LABEL, ago, hours, useGamepadNav } from './LibraryTab';

type Section = 'overview' | 'achievements' | 'screenshots' | 'soundtrack' | 'saves' | 'art';

function Achievements({ game }: { game: LibraryGame }) {
  const [list, setList] = useState<SteamAchievement[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    void api.library.achievements(game.key).then(setList, (e) => setErr(errorText(e)));
  }, [game.key]);
  if (err) return <Callout tone="info">{err}</Callout>;
  if (!list) return <p className="text-[13px] text-white/60">Asking Steam…</p>;
  if (list.length === 0) return <p className="text-[13px] text-white/60">This game has no achievements.</p>;
  const got = list.filter((a) => a.unlocked).length;
  const sorted = [...list].sort((a, b) => Number(b.unlocked) - Number(a.unlocked) || (a.unlocked ? b.unlockedAt - a.unlockedAt : b.percent - a.percent));
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-3">
        <div className="h-2 flex-1 overflow-hidden rounded-full bg-white/10">
          <div className="h-full rounded-full bg-accent" style={{ width: `${(got / list.length) * 100}%` }} />
        </div>
        <span className="text-[13px] tabular-nums text-white/80">
          {got} / {list.length}
        </span>
      </div>
      <div className="grid grid-cols-1 gap-2 lg:grid-cols-2">
        {sorted.map((a) => (
          <div key={a.id} className={cx('flex items-center gap-3 rounded-xl bg-white/5 p-2.5', !a.unlocked && 'opacity-70')}>
            {(a.unlocked ? a.icon : a.iconGray) ? <img src={a.unlocked ? a.icon : a.iconGray} alt="" className="h-11 w-11 rounded-lg" loading="lazy" /> : <div className="grid h-11 w-11 place-items-center rounded-lg bg-white/10">{a.unlocked ? <Trophy size={18} /> : <Lock size={16} />}</div>}
            <div className="min-w-0 flex-1">
              <div className="truncate text-[13.5px] font-semibold text-white">{a.hidden && !a.unlocked ? 'Hidden achievement' : a.name}</div>
              <div className="truncate text-[12px] text-white/60">{a.hidden && !a.unlocked ? 'Keep playing to find out.' : a.description}</div>
            </div>
            <div className="text-right text-[11.5px]">
              <div className={a.percent < 5 ? 'font-semibold text-warn' : 'text-white/70'}>{a.percent.toFixed(1)}%</div>
              {a.unlocked && <div className="text-white/50">{new Date(a.unlockedAt * 1000).toLocaleDateString()}</div>}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function Soundtrack({ game, detail }: { game: LibraryGame; detail: GameDetail }) {
  const audio = useRef<HTMLAudioElement>(null);
  const [now, setNow] = useState<number | null>(null);
  const [playing, setPlaying] = useState(false);
  const play = (i: number) => {
    const el = audio.current;
    if (!el) return;
    if (now === i) {
      if (el.paused) void el.play();
      else el.pause();
      return;
    }
    setNow(i);
    el.src = detail.soundtrack[i].src;
    void el.play();
  };
  useEffect(() => () => audio.current?.pause(), []);
  const q = encodeURIComponent(`${game.name} soundtrack`);
  return (
    <div className="space-y-3">
      <audio ref={audio} onPlay={() => setPlaying(true)} onPause={() => setPlaying(false)} onEnded={() => now !== null && now + 1 < detail.soundtrack.length && play(now + 1)} />
      {detail.soundtrack.length > 0 ? (
        <ol className="divide-y divide-white/5 rounded-xl bg-white/5">
          {detail.soundtrack.map((t, i) => (
            <li key={i}>
              <button type="button" onClick={() => play(i)} className={cx('flex w-full items-center gap-3 px-3 py-2 text-left hover:bg-white/5', now === i && 'text-accent')}>
                <span className="grid h-7 w-7 place-items-center rounded-full bg-white/10">{now === i && playing ? <Pause size={13} /> : <Play size={13} />}</span>
                <span className="flex-1 truncate text-[13px] text-white">{t.name}</span>
              </button>
            </li>
          ))}
        </ol>
      ) : (
        <p className="text-[13px] text-white/60">No soundtrack for it on this PC (soundtracks bought on Steam show up here).</p>
      )}
      <div className="flex flex-wrap gap-2">
        <Button size="sm" icon={Music} onClick={() => void api.app.openUrl(`https://www.youtube.com/results?search_query=${q}`)}>
          Find it on YouTube
        </Button>
        <Button size="sm" icon={Music} onClick={() => void api.app.openUrl(`https://open.spotify.com/search/${q}`)}>
          Find it on Spotify
        </Button>
      </div>
    </div>
  );
}

function Saves({ game, detail, reload }: { game: LibraryGame; detail: GameDetail; reload: () => void }) {
  const [busy, setBusy] = useState<string | null>(null);
  const dirs = detail.entry.saveDirs;
  const setDirs = (next: string[]) => void api.library.patch(game.key, { saveDirs: next }).then(reload, (e) => toast.error('Couldn’t change that', errorText(e)));
  const add = async () => {
    const folder = await api.app.pickFolder('Pick the folder with the saves');
    if (folder) setDirs([...dirs, folder]);
  };
  const backup = async () => {
    setBusy('backup');
    try {
      const b = await api.library.backup(game.key);
      toast.success('Saves backed up', `${b.files} files`);
      reload();
    } catch (e) {
      toast.error('Couldn’t back up', errorText(e));
    } finally {
      setBusy(null);
    }
  };
  const restore = async (b: SaveBackup) => {
    if (!(await confirm({ title: 'Put these saves back?', description: `The saves from ${new Date(b.at).toLocaleString()} replace the ones there now (those are backed up first). Close the game first.`, confirmLabel: 'Put back' }))) return;
    setBusy(b.id);
    try {
      await api.library.restore(game.key, b.id);
      toast.success('Saves put back');
      reload();
    } catch (e) {
      toast.error('Couldn’t put them back', errorText(e));
    } finally {
      setBusy(null);
    }
  };
  return (
    <div className="space-y-3">
      <div className="space-y-1.5">
        {dirs.map((d) => (
          <div key={d} className="flex items-center gap-2 rounded-lg bg-white/5 px-3 py-2 text-[12.5px] text-white">
            <span className="min-w-0 flex-1 truncate">{d}</span>
            <IconButton size="sm" icon={X} label="Stop backing this up" onClick={() => setDirs(dirs.filter((x) => x !== d))} />
          </div>
        ))}
        {detail.suggestedDirs.map((d) => (
          <div key={d} className="flex items-center gap-2 rounded-lg border border-dashed border-white/15 px-3 py-2 text-[12.5px] text-white/70">
            <span className="min-w-0 flex-1 truncate">Looks like its saves: {d}</span>
            <Button size="sm" onClick={() => setDirs([...dirs, d])}>
              Use
            </Button>
          </div>
        ))}
        <Button size="sm" icon={FolderPlus} onClick={() => void add()}>
          Add a save folder
        </Button>
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <Button variant="primary" icon={Save} loading={busy === 'backup'} disabled={dirs.length === 0} onClick={() => void backup()}>
          Back up now
        </Button>
        <Switch checked={detail.entry.autoBackup} disabled={dirs.length === 0} onChange={(v) => void api.library.patch(game.key, { autoBackup: v }).then(reload)} label="Back up after each session" />
        <span className="text-[12.5px] text-white/70">After each session</span>
      </div>
      {detail.backups.length > 0 && (
        <ul className="divide-y divide-white/5 rounded-xl bg-white/5">
          {detail.backups.map((b) => (
            <li key={b.id} className="flex items-center gap-3 px-3 py-2 text-[12.5px]">
              <span className="flex-1 text-white">{new Date(b.at).toLocaleString()}</span>
              <span className="text-white/50">
                {b.files} files · {(b.bytes / 1024 / 1024).toFixed(1)} MB
              </span>
              <Button size="sm" icon={RotateCcw} loading={busy === b.id} onClick={() => void restore(b)}>
                Put back
              </Button>
              <IconButton size="sm" icon={X} label="Delete this backup" onClick={() => void api.library.deleteBackup(game.key, b.id).then(reload)} />
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function ArtAndLimits({ game, detail, reload }: { game: LibraryGame; detail: GameDetail; reload: () => void }) {
  const [limit, setLimit] = useState(String(detail.entry.dailyLimit || ''));
  const pick = async (kind: 'cover' | 'hero' | 'logo') => {
    const [path] = await api.app.pickFiles(`Pick a picture for the ${kind}`);
    if (path) await api.library.setArt(game.key, kind, path).then(reload, (e) => toast.error('Couldn’t use it', errorText(e)));
  };
  return (
    <div className="space-y-4">
      <div className="grid grid-cols-3 gap-3">
        {(['cover', 'hero', 'logo'] as const).map((k) => (
          <Card key={k} className="space-y-2 bg-white/5 p-3">
            <div className="text-[12.5px] font-semibold capitalize text-white">{k === 'hero' ? 'Background' : k}</div>
            <div className="flex gap-1.5">
              <Button size="sm" icon={ImageIcon} onClick={() => void pick(k)}>
                Pick…
              </Button>
              {detail.entry[k] && (
                <Button size="sm" variant="ghost" onClick={() => void api.library.setArt(game.key, k, null).then(reload)}>
                  Default
                </Button>
              )}
            </div>
          </Card>
        ))}
      </div>
      <Button icon={Wand2} onClick={() => void api.library.findArt(game.key).then(() => (toast.success('Artwork found'), reload()), (e) => toast.error('Couldn’t find artwork', errorText(e)))}>
        Find artwork on SteamGridDB
      </Button>
      <div className="rounded-xl bg-white/5 p-3">
        <div className="text-[13px] font-semibold text-white">Daily time limit</div>
        <p className="mb-2 text-[12px] text-white/60">A warning 5 minutes before, then OmniHub closes the game for the rest of the day. Played today: {hours(detail.minutesToday)}.</p>
        <div className="flex items-center gap-2">
          <TextInput aria-label="Minutes a day" type="number" min={0} max={1440} value={limit} onChange={(e) => setLimit(e.target.value)} placeholder="No limit" className="w-32" />
          <span className="text-[12.5px] text-white/70">minutes a day</span>
          <Button size="sm" onClick={() => void api.library.patch(game.key, { dailyLimit: Math.max(0, Math.floor(Number(limit) || 0)) }).then(() => (toast.success(Number(limit) ? 'Limit set' : 'No limit'), reload()))}>
            Save
          </Button>
        </div>
      </div>
    </div>
  );
}

export function GameView({ game, onClose, onChanged }: { game: LibraryGame; onClose: () => void; onChanged: () => void }) {
  const [detail, setDetail] = useState<GameDetail | null>(null);
  const [section, setSection] = useState<Section>('overview');
  const [shot, setShot] = useState<string | null>(null);
  const [notes, setNotes] = useState('');
  const reload = useCallback(() => {
    void api.library.detail(game.key).then(
      (d) => {
        setDetail(d);
        setNotes(d.entry.notes);
      },
      (e) => toast.error('Couldn’t read the game', errorText(e)),
    );
    onChanged();
  }, [game.key, onChanged]);
  useEffect(() => {
    void api.library.detail(game.key).then((d) => {
      setDetail(d);
      setNotes(d.entry.notes);
    });
  }, [game.key]);
  const sections: { id: Section; label: string; show: boolean }[] = [
    { id: 'overview', label: 'Overview', show: true },
    { id: 'achievements', label: 'Achievements', show: game.app !== null },
    { id: 'screenshots', label: `Screenshots${detail?.screenshots.length ? ` (${detail.screenshots.length})` : ''}`, show: true },
    { id: 'soundtrack', label: 'Soundtrack', show: true },
    { id: 'saves', label: 'Saves', show: true },
    { id: 'art', label: 'Art & limits', show: true },
  ];
  const visible = sections.filter((s) => s.show);
  const play = () => void api.library.play(game.key).then(() => toast.success(`Starting ${game.name}`), (e) => toast.error('Couldn’t start it', errorText(e)));
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') (shot ? setShot(null) : onClose());
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose, shot]);
  useGamepadNav(true, {
    move: (dx) => {
      if (!dx) return;
      const i = visible.findIndex((s) => s.id === section);
      setSection(visible[Math.min(visible.length - 1, Math.max(0, i + dx))].id);
    },
    ok: play,
    back: () => (shot ? setShot(null) : onClose()),
    start: play,
  });
  const setStatus = (status: ShelfStatus) => void api.library.patch(game.key, { status }).then(reload);
  return (
    <motion.div className="fixed inset-0 z-[70] overflow-y-auto bg-[#07080c] text-white" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
      <div className="relative h-[46vh] min-h-[300px] w-full overflow-hidden">
        {game.art.hero ? <img src={game.art.hero} alt="" className="absolute inset-0 h-full w-full object-cover" onError={(e) => (e.currentTarget.style.display = 'none')} /> : <Cover game={game} className="absolute inset-0 scale-110 blur-2xl" />}
        <div className="absolute inset-0 bg-gradient-to-t from-[#07080c] via-[#07080c]/40 to-black/30" />
        <button type="button" onClick={onClose} aria-label="Back" className="absolute left-5 top-5 flex items-center gap-2 rounded-full bg-black/50 px-3 py-2 text-[13px] backdrop-blur hover:bg-black/70">
          <ArrowLeft size={16} /> Back
        </button>
        <div className="absolute bottom-6 left-8 right-8 flex items-end gap-6">
          <div className="hidden aspect-[2/3] w-36 shrink-0 overflow-hidden rounded-xl border border-white/15 shadow-2xl sm:block">
            <Cover game={game} />
          </div>
          <div className="min-w-0 flex-1">
            {game.art.logo ? <img src={game.art.logo} alt={game.name} className="max-h-28 max-w-[420px] object-contain drop-shadow-[0_4px_20px_rgba(0,0,0,0.7)]" onError={(e) => e.currentTarget.replaceWith(Object.assign(document.createElement('h1'), { textContent: game.name, className: 'font-display text-[44px] font-extrabold' }))} /> : <h1 className="font-display text-[44px] font-extrabold leading-tight">{game.name}</h1>}
            <div className="mt-3 flex flex-wrap items-center gap-3">
              <Button size="lg" variant="primary" icon={Play} onClick={play} disabled={!game.canLaunch}>
                Play
              </Button>
              <IconButton icon={Heart} label={game.favorite ? 'Remove from favourites' : 'Add to favourites'} variant="secondary" className={game.favorite ? 'text-pink-400' : ''} onClick={() => void api.library.patch(game.key, { favorite: !game.favorite }).then(reload)} />
              <Select aria-label="Status" value={game.status} onChange={(e) => setStatus(e.target.value as ShelfStatus)} className="w-40">
                {(Object.keys(STATUS_LABEL) as ShelfStatus[]).map((s) => (
                  <option key={s} value={s}>
                    {STATUS_LABEL[s]}
                  </option>
                ))}
              </Select>
              {game.app && (
                <Button icon={Puzzle} onClick={() => (onClose(), navigate('mods'))}>
                  Mods
                </Button>
              )}
            </div>
          </div>
        </div>
      </div>

      <div className="mx-auto max-w-[1200px] px-8 pb-16">
        <div className="mb-6 grid grid-cols-2 gap-3 sm:grid-cols-4">
          {[
            [Clock, 'Played', game.minutes ? hours(game.minutes) : 'Not yet'],
            [Clock, 'Last played', ago(game.lastPlayed)],
            [Users, 'Playing now', detail?.players != null ? detail.players.toLocaleString() : game.app ? '…' : '—'],
            [Search, 'Source', ({ steam: 'Steam', epic: 'Epic Games', gog: 'GOG', ubisoft: 'Ubisoft Connect', ea: 'EA app', battlenet: 'Battle.net', xbox: 'Xbox / Game Pass' } as Record<string, string>)[game.source] ?? game.source[0].toUpperCase() + game.source.slice(1)],
          ].map(([I, label, value]) => {
            const Icon = I as typeof Clock;
            return (
              <div key={label as string} className="rounded-xl bg-white/5 px-4 py-3">
                <div className="flex items-center gap-1.5 text-[11.5px] text-white/55">
                  <Icon size={12} /> {label as string}
                </div>
                <div className="mt-0.5 font-display text-[18px] font-semibold tabular-nums">{value as string}</div>
              </div>
            );
          })}
        </div>
        <div className="mb-5 flex gap-1 overflow-x-auto border-b border-white/10" role="tablist">
          {visible.map((s) => (
            <button key={s.id} type="button" role="tab" aria-selected={section === s.id} onClick={() => setSection(s.id)} className={cx('whitespace-nowrap border-b-2 px-3 py-2 text-[13.5px] transition-colors', section === s.id ? 'border-accent text-white' : 'border-transparent text-white/55 hover:text-white')}>
              {s.label}
            </button>
          ))}
        </div>
        {!detail ? (
          <p className="text-[13px] text-white/60">Loading…</p>
        ) : section === 'overview' ? (
          <div className="grid grid-cols-1 gap-6 lg:grid-cols-[minmax(0,1fr)_360px]">
            <div>
              <h3 className="mb-2 text-[14px] font-semibold">Recent sessions</h3>
              {detail.sessions.length === 0 ? (
                <p className="text-[13px] text-white/60">None yet: OmniHub notes sessions while the game runs, from any launcher.</p>
              ) : (
                <ul className="divide-y divide-white/5 rounded-xl bg-white/5">
                  {detail.sessions.map((s, i) => (
                    <li key={i} className="flex justify-between px-3 py-2 text-[13px]">
                      <span>{new Date(s.start).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })}</span>
                      <span className="tabular-nums text-white/70">{hours(Math.round((s.end - s.start) / 60_000))}</span>
                    </li>
                  ))}
                </ul>
              )}
            </div>
            <div>
              <h3 className="mb-2 text-[14px] font-semibold">Notes</h3>
              <Textarea value={notes} onChange={(e) => setNotes(e.target.value)} onBlur={() => notes !== detail.entry.notes && void api.library.patch(game.key, { notes }).then(reload)} placeholder="Where you are, what to do next, codes…" className="min-h-40" />
              {detail.entry.dailyLimit > 0 && <Badge className="mt-2">Limit {detail.entry.dailyLimit} min a day</Badge>}
            </div>
          </div>
        ) : section === 'achievements' ? (
          <Achievements game={game} />
        ) : section === 'screenshots' ? (
          detail.screenshots.length === 0 ? (
            <p className="text-[13px] text-white/60">No screenshots of it yet (OmniHub’s, or Steam’s F12 ones).</p>
          ) : (
            <div className="grid grid-cols-2 gap-3 md:grid-cols-3 xl:grid-cols-4">
              {detail.screenshots.map((s) => (
                <button key={s.src} type="button" onClick={() => setShot(s.src)} className="aspect-video overflow-hidden rounded-lg bg-white/5">
                  <img src={s.src} alt="" loading="lazy" className="h-full w-full object-cover transition-transform hover:scale-105" />
                </button>
              ))}
            </div>
          )
        ) : section === 'soundtrack' ? (
          <Soundtrack game={game} detail={detail} />
        ) : section === 'saves' ? (
          <Saves game={game} detail={detail} reload={reload} />
        ) : (
          <ArtAndLimits game={game} detail={detail} reload={reload} />
        )}
      </div>
      {shot && (
        <button type="button" onClick={() => setShot(null)} className="fixed inset-0 z-[75] grid place-items-center bg-black/90 p-8" aria-label="Close the screenshot">
          <img src={shot} alt="" className="max-h-full max-w-full rounded-lg" />
          <span className="absolute right-6 top-6 flex items-center gap-1 text-[13px] text-white/70">
            <Download size={14} /> Esc to close
          </span>
        </button>
      )}
    </motion.div>
  );
}
