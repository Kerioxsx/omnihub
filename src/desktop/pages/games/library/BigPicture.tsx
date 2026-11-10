// Big Picture: OmniHub full screen, made for the couch. Four tabs (Home,
// Library, Watch, Settings) switched with the bumpers; everything else is
// rows you move through with the D-pad or stick (or arrows / mouse). Watch
// opens channels like Netflix or Crunchyroll full screen in the browser you
// pick the first time, shows the series you're part way through, and turns
// the controller into the browser's remote until you press View.

import type { Browser, GoveePreset, LibraryGame, MediaState, Site, TvPrefs, Watching } from '@shared/types';
import { X } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api, errorText, on } from '../../../api';
import { cx } from '../../../lib/cx';
import { setWindowFullscreen, usePlayer } from '../../../lib/nowPlaying';
import { GLYPHS, confirmBack, setPadPrefs, usePadInput, useConnectedPads, usePadPrefs } from '../../../lib/pads';
import { toast } from '../../../state/toasts';
import { Cover } from './LibraryTab';
import { Glyph, PadPanel, applyLight } from './PadPanel';
import { OnScreenKeyboard } from './OnScreenKeyboard';
import { type Item, type NavRow, type Tab, type Target, TABS, THEMES, blip, chunk, setAt, setBpPrefs, targetKey, targetTitle, useBpPrefs } from './bigpicture/model';
import { Cap, Clock, Hint, OrbitMark, RemoteLegend, RowView } from './bigpicture/parts';

type LibFilter = 'all' | 'favorites' | 'playing' | 'recent' | 'new';
const FILTERS: { id: LibFilter; label: string }[] = [
  { id: 'all', label: 'All games' },
  { id: 'favorites', label: 'Favourites' },
  { id: 'playing', label: 'Playing' },
  { id: 'recent', label: 'Recently played' },
  { id: 'new', label: 'Not played yet' },
];
const NO_TV: TvPrefs = { browser: '', continueWatching: false, remote: true };
const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v));
type Opening = { name: string; color: string; browser: string | null; remote: boolean };

export function BigPicture({ games, onClose }: { games: LibraryGame[]; onClose: () => void }) {
  const bp = useBpPrefs();
  const padP = usePadPrefs();
  const theme = THEMES.find((t) => t.id === bp.theme) ?? THEMES[0];
  const [sites, setSites] = useState<Site[]>([]);
  const [tv, setTv] = useState<TvPrefs | null>(null);
  const [browsers, setBrowsers] = useState<Browser[] | null>(null);
  const [watching, setWatching] = useState<Watching[]>([]);
  const [tab, setTabState] = useState<Tab>(() => {
    // Open where you left (Settings aside).
    const t = localStorage.getItem('omnihub.bpTab') as Tab | null;
    return t && t !== 'settings' && TABS.some((x) => x.id === t) ? t : 'home';
  });
  const setTab = (t: Tab) => {
    setTabState(t);
    try {
      localStorage.setItem('omnihub.bpTab', t);
    } catch {
      /* fine */
    }
  };
  // The living room: what's playing, the volume, the lights.
  const [media, setMedia] = useState<MediaState | null>(null);
  const [volume, setVolume] = useState<{ level: number; muted: boolean } | null>(null);
  const [presets, setPresets] = useState<GoveePreset[]>([]);
  useEffect(() => {
    void api.media.state().then((r) => setMedia(r.state), () => undefined);
    void api.media.audio().then((a) => setVolume(a.volume), () => undefined);
    void api.govee.presets().then(setPresets, () => undefined);
    const off = on<{ state: MediaState | null }>('media:state', (p) => setMedia(p.state));
    return () => void off.then((f) => f());
  }, []);
  const [nav, setNav] = useState<Record<string, { row: number; cols: number[] }>>({});
  const [filter, setFilter] = useState<LibFilter>('all');
  const [query, setQuery] = useState('');
  const [typing, setTyping] = useState(false);
  const [panel, setPanel] = useState(false);
  const [leaving, setLeaving] = useState(false);
  const [launching, setLaunching] = useState<LibraryGame | null>(null);
  const [opening, setOpening] = useState<Opening | null>(null);
  const [picker, setPicker] = useState<{ pending: Target | null; draft: TvPrefs } | null>(null);
  const [vw, setVw] = useState(() => window.innerWidth);
  const rootRef = useRef<HTMLDivElement>(null);
  const mainRef = useRef<HTMLElement>(null);
  const pads = useConnectedPads();
  const psKey = pads.filter((x) => x.family === 'playstation').map((x) => `${x.index}${x.id}`).join('|');

  // ---------- data ----------
  useEffect(() => {
    void setWindowFullscreen(true);
    void api.sites.list().then(setSites, () => undefined);
    void api.tv.prefs().then(setTv, () => setTv(NO_TV));
    void api.tv.browsers().then(setBrowsers, () => setBrowsers([]));
    const onResize = () => setVw(window.innerWidth);
    window.addEventListener('resize', onResize);
    return () => {
      window.removeEventListener('resize', onResize);
      void setWindowFullscreen(false);
      // Leaving Big Picture: the controller stops driving the browser.
      api.tv.stopRemote().catch(() => undefined);
    };
  }, []);
  // Colour PlayStation light bars when Big Picture opens or one connects.
  useEffect(() => {
    if (psKey) void applyLight();
  }, [psKey]);
  const refreshWatching = useCallback(() => {
    void api.tv.continueWatching().then(setWatching, () => setWatching([]));
  }, []);
  useEffect(() => {
    if (tv?.continueWatching && tv.browser) refreshWatching();
    else setWatching([]);
  }, [tv?.continueWatching, tv?.browser, refreshWatching]);
  // View on the controller closed the channel and brought OmniHub back.
  useEffect(() => {
    const off = on('couch:home', () => {
      setOpening(null);
      refreshWatching();
    });
    return () => void off.then((f) => f());
  }, [refreshWatching]);
  // Back from the browser some other way (Alt+Tab, closing it).
  useEffect(() => {
    if (!opening) return;
    let away = false;
    const blur = () => (away = true);
    const focus = () => {
      if (!away) return;
      setOpening(null);
      refreshWatching();
    };
    window.addEventListener('blur', blur);
    window.addEventListener('focus', focus);
    const t = setTimeout(() => setOpening(null), 25_000);
    return () => {
      window.removeEventListener('blur', blur);
      window.removeEventListener('focus', focus);
      clearTimeout(t);
    };
  }, [opening, refreshWatching]);

  const color = (id: string) => sites.find((s) => s.id === id)?.color ?? '#6366f1';
  const browserOf = (p: TvPrefs | null) => browsers?.find((b) => b.id === p?.browser) ?? null;
  const browserName = tv?.browser === 'system' ? 'your usual browser' : (browserOf(tv)?.name ?? '');

  const saveTv = (next: TvPrefs) => {
    setTv(next);
    api.tv.setPrefs(next).then(setTv, (e) => toast.error('Couldn’t save that', errorText(e)));
  };

  // ---------- actions ----------
  const close = () => {
    if (leaving) return;
    blip('back');
    setLeaving(true);
    setTimeout(onClose, 260);
  };
  const play = (g: LibraryGame) => {
    if (launching) return;
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
  const openPicker = (pending: Target | null) => {
    const list = browsers ?? [];
    const cur = tv ?? NO_TV;
    const browser = cur.browser || list[0]?.id || 'system';
    // The first time, suggest showing what you're watching (it can be turned off right there).
    const draft = { ...cur, browser, continueWatching: cur.browser ? cur.continueWatching : true };
    const at = Math.max(0, [...list.map((b) => b.id), 'system'].indexOf(browser));
    setNav((n) => ({ ...n, picker: { row: 0, cols: [at] } }));
    setPicker({ pending, draft });
  };
  const openTarget = (t: Target, prefs: TvPrefs | null = tv) => {
    if (t.kind === 'game') return play(t.game);
    if (!prefs) return;
    if (!prefs.browser) return openPicker(t);
    const url = t.kind === 'site' ? t.site.url : t.w.url;
    const b = browserOf(prefs);
    blip('play');
    setOpening({ name: targetTitle(t), color: t.kind === 'site' ? t.site.color : color(t.w.site), browser: b?.name ?? null, remote: !!b && prefs.remote });
    api.tv.open(url).catch((e) => {
      setOpening(null);
      toast.error('Couldn’t open it', errorText(e));
    });
  };
  const finishPicker = () => {
    if (!picker) return;
    const { pending, draft } = picker;
    blip('play');
    setPicker(null);
    saveTv(draft);
    if (pending) openTarget(pending, draft);
  };

  // ---------- rows ----------
  const pad = vw < 900 ? 40 : Math.min(112, vw * 0.06);
  const libCols = clamp(Math.floor((vw - pad + 16) / 204), 3, 9);
  const chCols = clamp(Math.floor((vw - pad + 16) / 296), 2, 6);
  const q = query.trim().toLowerCase();
  const matches = (name: string) => !q || name.toLowerCase().includes(q);
  const recentGames = games.filter((g) => g.lastPlayed).sort((a, b) => b.lastPlayed - a.lastPlayed);
  const when = (t: Target) => (t.kind === 'game' ? t.game.lastPlayed : t.kind === 'watching' ? t.w.at : 0);

  const homeRows = (): NavRow[] => {
    let featured: Target[] = [...recentGames.slice(0, 4).map((game) => ({ kind: 'game', game }) as Target), ...watching.slice(0, 3).map((w) => ({ kind: 'watching', w }) as Target)].sort((a, b) => when(b) - when(a)).slice(0, 5);
    if (!featured.length) featured = games.length ? [...games.filter((g) => g.favorite), ...games].slice(0, 3).map((game) => ({ kind: 'game', game })) : sites.slice(0, 3).map((site) => ({ kind: 'site', site }));
    const rows: NavRow[] = [];
    if (featured.length) rows.push({ id: 'hero', kind: 'hero', items: featured.map((t) => ({ key: `f:${targetKey(t)}`, type: 'feature', target: t, run: () => openTarget(t) })) });
    if (watching.length) rows.push({ id: 'cw', kind: 'strip', title: 'Continue watching', sub: `Where you left off in ${browserName}`, items: watching.slice(0, 14).map((w) => ({ key: `w:${w.url}`, type: 'wide', target: { kind: 'watching', w }, run: () => openTarget({ kind: 'watching', w }) })) });
    if (recentGames.length) rows.push({ id: 'jump', kind: 'strip', title: 'Jump back in', sub: 'Your latest games', items: recentGames.slice(0, 14).map((game) => ({ key: game.key, type: 'wide', target: { kind: 'game', game }, run: () => play(game) })) });
    const favs = games.filter((g) => g.favorite);
    if (favs.length) rows.push({ id: 'favs', kind: 'strip', title: 'Favourites', items: favs.slice(0, 20).map((game) => ({ key: game.key, type: 'wide', target: { kind: 'game', game }, run: () => play(game) })) });
    rows.push({
      id: 'quick',
      kind: 'quick',
      title: 'Quick',
      items: [
        ...(media ? [{ key: 'q-np', type: 'quick' as const, label: media.playing ? 'Now playing' : 'Paused', sub: media.title, icon: 'music', run: () => goTab('room') }] : []),
        { key: 'q-lib', type: 'quick', label: 'Library', sub: `${games.length} games`, icon: 'library', run: () => goTab('library') },
        { key: 'q-watch', type: 'quick', label: 'Watch', sub: `${sites.length} channels`, icon: 'watch', run: () => goTab('watch') },
        { key: 'q-pads', type: 'quick', label: 'Controllers', sub: pads.length ? `${pads.length} connected` : 'None connected', icon: 'pads', run: () => setPanel(true) },
        { key: 'q-set', type: 'quick', label: 'Settings', sub: 'Browser, look, sounds', icon: 'settings', run: () => goTab('settings') },
        { key: 'q-leave', type: 'quick', label: 'Leave', sub: 'Back to the desktop', icon: 'leave', run: close },
      ],
    });
    return rows;
  };

  const libRows = (): NavRow[] => {
    const list = games
      .filter((g) => matches(g.name))
      .filter((g) => (filter === 'favorites' ? g.favorite : filter === 'playing' ? g.status === 'playing' : filter === 'recent' ? g.lastPlayed > 0 : filter === 'new' ? !g.minutes : true))
      .sort((a, b) => (filter === 'recent' ? b.lastPlayed - a.lastPlayed : a.name.localeCompare(b.name)));
    const chips: Item[] = [
      ...FILTERS.map((f) => ({ key: `c-${f.id}`, type: 'chip' as const, label: f.label, selected: filter === f.id, run: () => setFilter(f.id) })),
      { key: 'c-search', type: 'chip', label: q ? `Search: ${query.trim()}  ✕` : 'Search', selected: !!q, run: () => (q ? setQuery('') : setTyping(true)) },
    ];
    const rows: NavRow[] = [{ id: 'chips', kind: 'chips', items: chips }];
    if (!list.length) {
      rows.push({ id: 'none', kind: 'notice', items: [{ key: 'none', type: 'notice', title: 'Nothing here', sub: q ? `No game matches “${query.trim()}”.` : 'No games in this filter yet.', action: 'Show all', run: () => (setFilter('all'), setQuery('')) }] });
      return rows;
    }
    chunk(list, libCols).forEach((part, i) => rows.push({ id: `g${i}`, kind: 'grid', group: 'games', cols: libCols, items: part.map((g) => ({ key: g.key, type: 'cover', game: g, run: () => play(g) })) }));
    return rows;
  };

  const watchRows = (): NavRow[] => {
    const rows: NavRow[] = [];
    const p = tv ?? NO_TV;
    if (!p.browser) rows.push({ id: 'pick', kind: 'notice', items: [{ key: 'pick', type: 'notice', title: 'Pick the browser your channels open in', sub: 'You choose once; change it any time in Settings. Channels open full screen with no bars, and your controller works as the remote.', action: 'Pick', run: () => openPicker(null) }] });
    else if (!p.continueWatching) rows.push({ id: 'cw-off', kind: 'notice', items: [{ key: 'cw-off', type: 'notice', title: 'See the series you’re watching', sub: `OmniHub can read the watch pages of these streaming sites from ${browserName}’s history, on this PC only, to show where you left off. Nothing is sent anywhere.`, action: 'Turn on', run: () => saveTv({ ...p, continueWatching: true }) }] });
    if (watching.length) rows.push({ id: 'cw', kind: 'strip', title: 'Continue watching', sub: `From ${browserName}, newest first`, items: watching.filter((w) => matches(w.show || w.service)).map((w) => ({ key: `w:${w.url}`, type: 'wide', target: { kind: 'watching', w }, run: () => openTarget({ kind: 'watching', w }) })) });
    const ch = sites.filter((s) => matches(s.name));
    const where = p.browser ? `Opens full screen in ${browserName}` : 'Opens full screen';
    chunk(ch, chCols).forEach((part, i) =>
      rows.push({ id: `ch${i}`, kind: 'grid', group: 'channels', cols: chCols, title: i === 0 ? 'Channels' : undefined, sub: i === 0 ? `${where}${p.remote && browserOf(p) ? ' · your controller is the remote, View comes back here' : ''}` : undefined, items: part.map((s) => ({ key: `site:${s.id}`, type: 'channel', site: s, run: () => openTarget({ kind: 'site', site: s }) })) }),
    );
    if (!rows.some((r) => r.items.length)) rows.push({ id: 'none', kind: 'notice', items: [{ key: 'none', type: 'notice', title: 'No channels match', sub: `Nothing called “${query.trim()}”.`, action: 'Clear', run: () => setQuery('') }] });
    rows.push({
      id: 'wq',
      kind: 'quick',
      title: 'Options',
      items: [
        { key: 'wq-browser', type: 'quick', label: 'Browser', sub: browserName || 'Not picked yet', icon: 'browser', run: () => openPicker(null) },
        { key: 'wq-remote', type: 'quick', label: 'Controller remote', sub: p.remote ? 'On' : 'Off', icon: 'remote', run: () => saveTv({ ...p, remote: !p.remote }) },
        { key: 'wq-cw', type: 'quick', label: 'Continue watching', sub: p.browser && p.continueWatching ? `On · ${watching.length} found` : 'Off', icon: 'play', run: () => (p.browser ? saveTv({ ...p, continueWatching: !p.continueWatching }) : openPicker(null)) },
        { key: 'wq-search', type: 'quick', label: 'Search', sub: q ? `“${query.trim()}”` : 'Find a channel', icon: 'search', run: () => setTyping(true) },
      ],
    });
    return rows;
  };

  const roomRows = (): NavRow[] => {
    const act = (a: 'toggle' | 'next' | 'previous') => () => void api.media.control(a).catch((e) => toast.error('The player didn’t answer', errorText(e)));
    const vol = (level: number | null, muted: boolean | null) => () => void api.media.setVolume(level, muted).then(setVolume, (e) => toast.error('Couldn’t change the volume', errorText(e)));
    const lv = volume?.level ?? 0.5;
    const rows: NavRow[] = [
      {
        id: 'np',
        kind: 'quick',
        title: 'Now playing',
        sub: media ? `${media.title}${media.artist ? ` · ${media.artist}` : ''} · ${media.appName}` : 'Nothing is playing on the PC',
        items: [
          { key: 'np-prev', type: 'quick', label: 'Previous', sub: 'Last track', icon: 'prev', run: act('previous') },
          { key: 'np-toggle', type: 'quick', label: media?.playing ? 'Pause' : 'Play', sub: media ? media.title : 'Start your music app first', icon: media?.playing ? 'pause' : 'resume', run: act('toggle') },
          { key: 'np-next', type: 'quick', label: 'Next', sub: 'Skip this one', icon: 'next', run: act('next') },
          {
            key: 'np-lyrics',
            type: 'quick',
            label: 'Lyrics',
            sub: 'Full screen, leaves Big Picture',
            icon: 'lyrics',
            run: () => {
              close();
              setTimeout(() => usePlayer.getState().show(), 320);
            },
          },
        ],
      },
      {
        id: 'vol',
        kind: 'quick',
        title: 'Volume',
        sub: volume ? (volume.muted ? 'Muted' : `${Math.round(volume.level * 100)}%`) : undefined,
        items: [
          { key: 'v-down', type: 'quick', label: 'Quieter', sub: '−10%', icon: 'voldown', run: vol(Math.max(0, lv - 0.1), false) },
          { key: 'v-up', type: 'quick', label: 'Louder', sub: '+10%', icon: 'volup', run: vol(Math.min(1, lv + 0.1), false) },
          { key: 'v-mute', type: 'quick', label: volume?.muted ? 'Unmute' : 'Mute', sub: 'The whole PC', icon: 'mute', run: vol(null, !volume?.muted) },
        ],
      },
    ];
    if (presets.length)
      chunk(presets.slice(0, 12), 4).forEach((part, i) =>
        rows.push({ id: `li${i}`, kind: 'quick', group: 'lights', title: i === 0 ? 'Lights' : undefined, sub: i === 0 ? 'Your saved looks for the Govee lights' : undefined, items: part.map((p) => ({ key: `li-${p.id}`, type: 'quick', label: p.name, sub: p.brightness != null ? `Brightness ${p.brightness}%` : 'Light preset', icon: 'light', run: () => void api.govee.applyPreset(p.id).then(() => blip('play'), (e) => toast.error('The lights didn’t answer', errorText(e))) })) }),
      );
    else rows.push({ id: 'li-none', kind: 'notice', items: [{ key: 'li-none', type: 'notice', title: 'Lights', sub: 'Save a look on the Lights page (a preset) and it shows up here, one press from the couch.', action: 'OK', run: () => undefined }] });
    return rows;
  };

  const settingsRows = (): NavRow[] => {
    const p = tv ?? NO_TV;
    const line = (key: string, label: string, value: string | boolean, run: () => void, sub?: string, title?: string): NavRow => ({ id: key, kind: 'list', title, items: [{ key, type: 'setting', label, sub, value, run }] });
    return [
      line('s-browser', 'Default browser', browserName || 'Not picked', () => openPicker(null), 'Where Netflix, Crunchyroll and the other channels open, full screen.', 'Channels'),
      line('s-cw', 'Continue watching', p.continueWatching, () => (p.browser ? saveTv({ ...p, continueWatching: !p.continueWatching }) : openPicker(null)), 'Reads only the watch pages of these streaming sites from that browser’s history, from a copy on this PC that is deleted straight after. Nothing is sent anywhere.'),
      line('s-remote', 'Controller as the remote', p.remote, () => saveTv({ ...p, remote: !p.remote }), 'In a channel the left stick moves the pointer, A clicks and View comes back. Xbox and other XInput controllers; only while that browser is in front.'),
      { id: 's-theme', kind: 'themes', title: 'Big Picture', sub: 'Background', items: THEMES.map((t) => ({ key: `t-${t.id}`, type: 'theme', id: t.id, label: t.label, selected: t.id === bp.theme, run: () => setBpPrefs({ theme: t.id }) })) },
      line('s-sounds', 'Sounds', padP.sounds, () => setPadPrefs({ sounds: !padP.sounds }), 'Soft ticks as you move.'),
      line('s-motion', 'Reduce motion', bp.reducedMotion, () => setBpPrefs({ reducedMotion: !bp.reducedMotion }), 'No sliding, zooming or drifting art.'),
      line('s-pads', 'Controllers and buttons', 'Open', () => setPanel(true), 'Button layout, vibration, light bar, remapping.'),
      line('s-leave', 'Leave Big Picture', '', close),
    ];
  };

  const pickerRows = (): NavRow[] => {
    if (!picker) return [];
    const d = picker.draft;
    const set = (patch: Partial<TvPrefs>) => setPicker((pk) => (pk ? { ...pk, draft: { ...pk.draft, ...patch } } : pk));
    const list: (Browser | null)[] = [...(browsers ?? []), null];
    return [
      { id: 'p-b', kind: 'browsers', items: list.map((b) => ({ key: `b-${b?.id ?? 'system'}`, type: 'browser', browser: b, selected: d.browser === (b?.id ?? 'system'), run: () => (set({ browser: b?.id ?? 'system' }), setNav((n) => ({ ...n, picker: { row: 3, cols: setAt(n.picker?.cols ?? [], 3, 0) } }))) })) },
      { id: 'p-cw', kind: 'list', items: [{ key: 'p-cw', type: 'setting', label: 'Show what I’m watching', sub: d.browser === 'system' ? 'Needs one of the browsers above.' : 'A Continue watching row from this browser’s history (these streaming sites only).', value: d.continueWatching, run: () => set({ continueWatching: !d.continueWatching }) }] },
      { id: 'p-r', kind: 'list', items: [{ key: 'p-r', type: 'setting', label: 'Use my controller as the remote', sub: d.browser === 'system' ? 'Needs one of the browsers above.' : 'Pointer, click, play / pause, full screen; View closes the channel and comes back.', value: d.remote, run: () => set({ remote: !d.remote }) }] },
      {
        id: 'p-go',
        kind: 'buttons',
        items: [
          { key: 'go', type: 'button', label: picker.pending ? `Continue to ${targetTitle(picker.pending)}` : 'Save', primary: true, run: finishPicker },
          { key: 'cancel', type: 'button', label: 'Not now', run: () => setPicker(null) },
        ],
      },
    ];
  };

  const layer = picker ? 'picker' : tab;
  const tabRows = () => (tab === 'home' ? homeRows() : tab === 'library' ? libRows() : tab === 'watch' ? watchRows() : tab === 'room' ? roomRows() : settingsRows());
  const rows = picker ? pickerRows() : tabRows();
  const st = nav[layer] ?? { row: 0, cols: [] };
  const row = clamp(st.row, 0, Math.max(0, rows.length - 1));
  const col = clamp(st.cols[row] ?? 0, 0, Math.max(0, (rows[row]?.items.length ?? 1) - 1));
  const focusAt = (r: number, c: number) => setNav((n) => ({ ...n, [layer]: { row: r, cols: setAt(n[layer]?.cols ?? [], r, c) } }));

  function goTab(t: Tab) {
    if (t === tab) return;
    blip('tab');
    setTab(t);
  }
  const switchTab = (d: -1 | 1) => {
    if (picker) return;
    const i = TABS.findIndex((t) => t.id === tab);
    goTab(TABS[(i + d + TABS.length) % TABS.length].id);
  };
  const move = (dx: number, dy: number) => {
    if (launching || opening || !rows.length) return;
    if (dy) {
      const next = clamp(row + dy, 0, rows.length - 1);
      if (next === row) return;
      const same = rows[next].group && rows[next].group === rows[row].group;
      const c = same ? Math.min(col, rows[next].items.length - 1) : (st.cols[next] ?? 0);
      blip('move');
      focusAt(next, c);
      return;
    }
    const len = rows[row]?.items.length ?? 1;
    const next = clamp(col + dx, 0, len - 1);
    if (next !== col) {
      blip('move');
      focusAt(row, next);
      return;
    }
    // At the end of a grid row: carry on in the next (or previous) row.
    const to = row + dx;
    if (rows[row]?.group && rows[to]?.group === rows[row].group) {
      blip('move');
      focusAt(to, dx > 0 ? 0 : rows[to].items.length - 1);
    }
  };
  const ok = () => {
    if (launching || opening) return;
    rows[row]?.items[col]?.run?.();
  };
  const back = () => {
    if (launching) return setLaunching(null);
    if (opening) return setOpening(null);
    if (picker) return setPicker(null);
    if (q) return setQuery('');
    if (tab !== 'home') return goTab('home');
    close();
  };
  const search = () => {
    if (picker) return;
    if (tab !== 'library' && tab !== 'watch') setTab('library');
    setTyping(true);
  };
  const pick = (r: number, c: number) => {
    if (r === row && c === col) ok();
    else {
      blip('move');
      focusAt(r, c);
    }
  };

  const lastPad = usePadInput(!panel && !typing, { move, ok, back, start: ok, menu: () => setPanel(true), select: search, bumper: switchTab, trigger: (d) => move(0, d * 3) }, { haptics: true });
  const family = lastPad?.family ?? pads[0]?.family ?? 'xbox';
  const g = GLYPHS[family];

  // Keyboard: arrows, Enter, Esc; [ ] or Tab for tabs; / or S to search; C for controllers.
  const keys = useRef({ move, ok, back, switchTab, search, panel, typing });
  keys.current = { move, ok, back, switchTab, search, panel, typing };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const k = keys.current;
      if (k.panel || k.typing) return;
      const map: Record<string, () => void> = {
        ArrowLeft: () => k.move(-1, 0),
        ArrowRight: () => k.move(1, 0),
        ArrowUp: () => k.move(0, -1),
        ArrowDown: () => k.move(0, 1),
        PageUp: () => k.move(0, -3),
        PageDown: () => k.move(0, 3),
        Enter: k.ok,
        ' ': k.ok,
        Escape: k.back,
        Backspace: k.back,
        '[': () => k.switchTab(-1),
        ']': () => k.switchTab(1),
        Tab: () => k.switchTab(e.shiftKey ? -1 : 1),
        '/': k.search,
        s: k.search,
        S: k.search,
        c: () => setPanel(true),
        C: () => setPanel(true),
      };
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
  const wheelAt = useRef(0);
  const onWheel = (e: React.WheelEvent) => {
    if (Date.now() - wheelAt.current < 90) return;
    wheelAt.current = Date.now();
    const sideways = e.shiftKey || Math.abs(e.deltaX) > Math.abs(e.deltaY);
    const d = sideways ? e.deltaX || e.deltaY : e.deltaY;
    if (sideways) move(d > 0 ? 1 : -1, 0);
    else move(0, d > 0 ? 1 : -1);
  };

  // Keep the focused card in view.
  useEffect(() => {
    if (!picker && row === 0) mainRef.current?.scrollTo({ top: 0, behavior: bp.reducedMotion ? 'auto' : 'smooth' });
    const el = rootRef.current?.querySelector<HTMLElement>(picker ? '.bp2-modal [data-bp-on]' : 'main [data-bp-on]');
    el?.scrollIntoView({ block: 'nearest', inline: 'center', behavior: bp.reducedMotion ? 'auto' : 'smooth' });
  }, [layer, row, col, rows.length, picker, bp.reducedMotion]);

  const ctx = { family, color };
  const head = tab === 'library' ? { eyebrow: 'Your games', title: 'Library', sub: `${games.length} games · ${FILTERS.find((f) => f.id === filter)?.label}` } : tab === 'watch' ? { eyebrow: 'Entertainment', title: 'Watch', sub: tv?.browser ? `Channels open in ${browserName}` : 'Your streaming channels, full screen' } : tab === 'room' ? { eyebrow: 'Living room', title: 'Room', sub: 'Music, volume and lights without leaving the couch' } : tab === 'settings' ? { eyebrow: 'Big Picture', title: 'Settings', sub: 'Changes save straight away' } : null;
  const backLabel = launching || opening ? 'Hide' : picker ? 'Cancel' : q ? 'Clear search' : tab !== 'home' ? 'Home' : 'Leave';
  const side = (i: number) => (i === 1 ? 'right' : 'bottom') as 'right' | 'bottom';
  const cb = confirmBack(family, padP);
  const gutter = { paddingLeft: pad / 2, paddingRight: pad / 2 };

  return (
    <div
      ref={rootRef}
      className={cx('bp2 fixed inset-0 z-[90] flex select-none flex-col overflow-hidden text-[#f7f8ff]', leaving ? 'bp-out' : 'bp-in', bp.reducedMotion && 'bp2-calm')}
      style={{ background: theme.bg, ['--bp-accent' as string]: theme.accent, ['--bp-accent-rgb' as string]: theme.accentRgb }}
      onWheel={onWheel}
      role="dialog"
      aria-label="Big Picture"
    >
      <header className="flex h-[96px] shrink-0 items-center justify-between gap-6" style={gutter}>
        <span className="flex min-w-0 items-center gap-3">
          <OrbitMark />
          <span className="text-[22px] font-black tracking-[0.25em]">OMNIHUB</span>
          <span className="ml-1 hidden whitespace-nowrap rounded-lg border border-[var(--bp-stroke)] px-3 py-2 text-[10px] uppercase tracking-[0.16em] text-[var(--bp-muted)] xl:inline">Big Picture</span>
        </span>
        <nav className="flex items-center gap-2" aria-label="Tabs">
          <Cap className="mr-1 opacity-80">{g.lb}</Cap>
          {TABS.map((t) => (
            <button
              key={t.id}
              type="button"
              tabIndex={-1}
              onClick={() => (setPicker(null), goTab(t.id))}
              className={cx('rounded-xl border px-5 py-3 text-[16px] font-bold transition', t.id === tab ? 'border-[rgba(var(--bp-accent-rgb),0.3)] bg-[#242b42] text-white shadow-[inset_0_-2px_var(--bp-accent)]' : 'border-transparent text-[var(--bp-muted)] hover:bg-[#212940] hover:text-white')}
            >
              {t.label}
            </button>
          ))}
          <Cap className="ml-1 opacity-80">{g.rb}</Cap>
        </nav>
        <span className="flex items-center gap-5 whitespace-nowrap text-[14px] text-[var(--bp-muted)]">
          <span className="hidden items-center lg:flex">
            <span className={cx('mr-2 inline-block h-2 w-2 rounded-full', pads.length ? 'bg-[var(--bp-mint)] shadow-[0_0_12px_var(--bp-mint)]' : 'bg-white/30')} />
            {pads.length ? pads[0].name : 'No controller'}
          </span>
          <span className="text-[17px] font-semibold text-white/85">
            <Clock />
          </span>
          <button type="button" tabIndex={-1} onClick={close} aria-label="Leave Big Picture" className="grid h-10 w-10 place-items-center rounded-xl bg-white/10 text-white transition hover:bg-white/20">
            <X size={20} />
          </button>
        </span>
      </header>

      <main ref={mainRef} className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden pb-12 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden" style={gutter}>
        <div key={tab} className="bp2-page mx-auto max-w-[2000px] pt-1">
          {head && (
            <div className="mb-2">
              <span className="flex items-center gap-2 text-[12px] font-extrabold uppercase tracking-[0.22em] text-[#c6d0e9]">
                <span className="h-2 w-2 rounded-full bg-[var(--bp-mint)] shadow-[0_0_12px_var(--bp-mint)]" />
                {head.eyebrow}
              </span>
              <h1 className="mt-2 text-[clamp(38px,4vw,60px)] font-black leading-[1.05] tracking-[-0.055em]">{head.title}</h1>
              <p className="mt-1 text-[15px] text-[var(--bp-muted)]">{head.sub}</p>
            </div>
          )}
          {rows.length === 0 && <div className="mt-10 rounded-2xl border border-dashed border-[var(--bp-stroke)] p-10 text-center text-[var(--bp-muted)]">Nothing here yet.</div>}
          {(picker ? tabRows() : rows).map((r, ri) => (
            <RowView key={r.id} row={r} ri={ri} on={!picker && ri === row} at={!picker && ri === row ? col : clamp((nav[tab]?.cols ?? [])[ri] ?? 0, 0, r.items.length - 1)} ctx={ctx} pick={picker ? () => undefined : pick} />
          ))}
          {tab === 'watch' && (
            <section className="mt-10 rounded-[20px] border border-[var(--bp-stroke)] p-6" style={{ background: 'linear-gradient(140deg, #141b2d, #0e1424)' }}>
              <h3 className="text-[18px] font-extrabold">Your controller in a channel</h3>
              <p className="mb-5 mt-1 text-[13px] text-[var(--bp-muted)]">{tv?.remote ? 'While the channel’s browser is in front. Xbox controllers and others Windows sees as one (XInput); PlayStation controllers need Steam or DS4Windows for this.' : 'Turned off: use the mouse and keyboard in the browser.'}</p>
              {tv?.remote && family !== 'xbox' && <p className="mb-5 rounded-xl border border-[#b98a2c66] bg-[#4a391233] px-4 py-3 text-[13px] text-[#f1d9a0]">The controller you’re holding isn’t an Xbox-style one. It works here in Big Picture, but inside a channel it only works if Steam (Steam Input) or DS4Windows is running and presenting it as an Xbox controller. Otherwise use the mouse there, or turn the remote off.</p>}
              <RemoteLegend family={family} />
            </section>
          )}
        </div>
      </main>

      <footer className="flex h-[66px] shrink-0 items-center justify-between gap-6 border-t border-white/[0.06] text-[14px] font-semibold text-white/70" style={gutter}>
        <span className="truncate text-[var(--bp-muted)]">{tab === 'watch' || picker ? (tv?.browser ? `Channels open in ${browserName}` : 'Pick a browser the first time you open a channel') : `${games.length} games · ${sites.length} channels`}</span>
        <span className="flex items-center gap-6">
          <Hint k="✥" label="Move" />
          <span className="flex items-center gap-2">
            <Glyph family={family} which={side(cb.ok)} /> Select
          </span>
          <span className="flex items-center gap-2">
            <Glyph family={family} which={side(cb.back)} /> {backLabel}
          </span>
          {!picker && <Hint k={`${g.lb} ${g.rb}`} label="Tabs" />}
          {!picker && <Hint k={g.view} label="Search" />}
          <span className="hidden items-center gap-2 xl:flex">
            <Glyph family={family} which="top" /> Controllers
          </span>
        </span>
      </footer>

      {picker && (
        <div className="bp-in absolute inset-0 z-30 grid place-items-center bg-[#020513c4] p-6 backdrop-blur-md">
          <div className="bp2-modal max-h-[90vh] w-[min(1040px,94vw)] overflow-y-auto rounded-[26px] border border-white/20 p-9 shadow-[0_25px_100px_rgba(0,0,0,0.7)] [scrollbar-width:none]" style={{ background: 'linear-gradient(140deg, #182039, #0f1526)' }}>
            <span className="flex items-center gap-2 text-[12px] font-extrabold uppercase tracking-[0.22em] text-[#c6d0e9]">
              <span className="h-2 w-2 rounded-full bg-[var(--bp-mint)] shadow-[0_0_12px_var(--bp-mint)]" />
              {tv?.browser ? 'Channels' : 'First time here'}
            </span>
            <h2 className="mb-2 mt-3 text-[clamp(30px,3vw,44px)] font-black leading-[1.05] tracking-[-0.05em]">Which browser should channels open in?</h2>
            <p className="mb-6 max-w-[760px] text-[15px] text-[#b9c6df]">Netflix, Crunchyroll and the rest open full screen in it, with no bars. You can change it any time in Big Picture’s Settings or in Quick sites on the desktop.</p>
            {browsers === null ? (
              <div className="py-8 text-[var(--bp-muted)]">Looking for browsers…</div>
            ) : (
              <div className="flex flex-col gap-1">
                {rows.map((r, ri) => (
                  <div key={r.id} className={cx(ri === 1 && 'mt-5', ri === 3 && 'mt-5')}>
                    <RowView row={r} ri={ri} on={ri === row} at={ri === row ? col : clamp(st.cols[ri] ?? 0, 0, r.items.length - 1)} ctx={ctx} pick={pick} />
                  </div>
                ))}
              </div>
            )}
            <p className="mt-6 rounded-xl border border-[#647cb650] bg-[#26334a66] px-4 py-3 text-[12px] leading-relaxed text-[#c9d7ef]">
              Privacy: Continue watching reads only the watch pages of these streaming sites (Netflix, Crunchyroll, Disney+, HBO Max, Prime Video, Paramount+, Apple TV, YouTube videos) from the browser’s history. It reads a copy on this PC and deletes it straight after; nothing is sent anywhere.
            </p>
          </div>
        </div>
      )}

      {panel && <PadPanel lastPad={lastPad} onClose={() => setPanel(false)} />}
      {typing && (
        <OnScreenKeyboard
          value={query}
          title={tab === 'watch' ? 'Find a channel' : 'Find a game'}
          onChange={(v) => {
            setQuery(v);
            setNav((n) => ({ ...n, [tab]: { row: 0, cols: [] } }));
          }}
          onDone={() => setTyping(false)}
        />
      )}

      {launching && (
        <div className="bp-in absolute inset-0 z-20 grid place-items-center bg-black/75 backdrop-blur-md">
          <div className="flex flex-col items-center gap-7">
            <div className="bp-pulse relative w-[200px] overflow-hidden rounded-2xl ring-[3px] ring-[var(--bp-accent)]">
              <div className="aspect-[2/3]">
                <Cover game={launching} />
              </div>
            </div>
            <div className="text-center">
              <div className="text-[15px] uppercase tracking-[0.3em] text-white/60">Starting</div>
              <div className="mt-1 text-[34px] font-black tracking-[-0.03em]">{launching.name}</div>
            </div>
          </div>
        </div>
      )}

      {opening && (
        <div className="bp-in absolute inset-0 z-20 flex flex-col items-center justify-center gap-8 p-8 backdrop-blur-md" style={{ background: `radial-gradient(60% 55% at 50% 25%, ${opening.color}66, transparent 70%), rgba(2,4,12,0.88)` }}>
          <div className="text-center">
            <div className="text-[14px] font-bold uppercase tracking-[0.3em] text-white/60">Opening{opening.browser ? ` in ${opening.browser}` : ''}</div>
            <div className="bp-rise mt-2 text-[clamp(44px,5vw,80px)] font-black uppercase leading-none tracking-[-0.05em]">{opening.name}</div>
          </div>
          {opening.remote ? (
            <div className="w-[min(980px,94vw)] rounded-[22px] border border-white/15 bg-black/40 p-7">
              <div className="mb-5 text-[17px] font-extrabold">Your controller is the remote</div>
              <RemoteLegend family={family} />
            </div>
          ) : (
            <p className="max-w-[640px] text-center text-[16px] text-white/75">Use the mouse and keyboard in the browser. Close it to come back to Big Picture.</p>
          )}
          <span className="flex items-center gap-2 text-[14px] text-white/60">
            <Glyph family={family} which={side(cb.back)} size={24} /> Hide this
          </span>
        </div>
      )}
    </div>
  );
}
