import { formatBytes, formatNumber } from '@shared/format';
import type { CaptureKind, Screenshot } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { AppWindow, Camera, Crop, Monitor, MonitorSmartphone, RefreshCw, Star, Tag, Timer, ImageOff } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { api, errorText, inTauri } from '../../api';
import { Page } from '../../components/Page';
import { ShotThumb } from '../../components/ShotThumb';
import { Button, IconButton } from '../../components/ui/Button';
import { Kbd, Skeleton } from '../../components/ui/Card';
import { SearchInput, Segmented, Select } from '../../components/ui/Form';
import { EmptyState, ErrorState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useAsync, useEvent } from '../../lib/hooks';
import { navigate, useRoute } from '../../lib/router';
import { dayKey, dayLabel } from '../../lib/util';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';
import { Lightbox } from './Lightbox';

type Delay = '0' | '3' | '5';

function exeLabel(exe: string): string {
  return exe.replace(/\.exe$/i, '');
}

export function ScreenshotsPage() {
  const route = useRoute();
  const shots = useAsync(() => api.shots.list({}), []);
  const info = useSettings((s) => s.info);
  const hotkeys = useSettings((s) => s.settings?.screenshots);
  const [q, setQ] = useState('');
  const [tag, setTag] = useState<string | null>(null);
  const [app, setApp] = useState('');
  const [favs, setFavs] = useState(false);
  const [delay, setDelay] = useState<Delay>('0');
  const [countdown, setCountdown] = useState<number | null>(null);
  const [openId, setOpenId] = useState<string | null>(route.params.get('open'));
  const [fresh, setFresh] = useState<Set<string>>(new Set());

  useEffect(() => {
    const id = route.params.get('open');
    if (id) setOpenId(id);
  }, [route.params]);

  useEvent<Screenshot>('screenshots:new', (s) => {
    shots.setData((prev) => [s, ...(prev ?? []).filter((x) => x.id !== s.id)]);
    setFresh((f) => new Set(f).add(s.id));
  });
  useEvent<{ id: string }>('screenshots:deleted', (p) => shots.setData((prev) => (prev ?? []).filter((x) => x.id !== p.id)));
  useEvent('screenshots:synced', () => void shots.reload());

  const all = shots.data ?? [];
  const tags = useMemo(() => {
    const m = new Map<string, number>();
    all.forEach((s) => s.tags.forEach((t) => m.set(t, (m.get(t) ?? 0) + 1)));
    return [...m.entries()].sort((a, b) => b[1] - a[1]);
  }, [all]);
  const apps = useMemo(() => [...new Set(all.map((s) => s.appExe).filter((x): x is string => !!x))].sort(), [all]);
  const list = useMemo(() => {
    const needle = q.trim().toLowerCase();
    return all.filter((s) => (!favs || s.favorite) && (!tag || s.tags.includes(tag)) && (!app || s.appExe === app) && (!needle || `${s.appTitle ?? ''} ${s.appExe ?? ''} ${s.note} ${s.tags.join(' ')}`.toLowerCase().includes(needle)));
  }, [all, q, tag, app, favs]);
  const groups = useMemo(() => {
    const out: { key: string; label: string; items: Screenshot[] }[] = [];
    for (const s of list) {
      const k = dayKey(s.created);
      const g = out[out.length - 1];
      if (g && g.key === k) g.items.push(s);
      else out.push({ key: k, label: dayLabel(s.created), items: [s] });
    }
    return out;
  }, [list]);
  const totalBytes = all.reduce((a, s) => a + s.bytes, 0);

  const capture = async (kind: CaptureKind) => {
    const secs = Number(delay);
    if (secs > 0) {
      setCountdown(secs);
      const t = setInterval(() => setCountdown((c) => (c != null && c > 1 ? c - 1 : null)), 1000);
      setTimeout(() => clearInterval(t), secs * 1000 + 50);
    }
    try {
      const s = await api.shots.capture(kind, secs);
      toast.success('Screenshot saved', `${s.width} × ${s.height} · ${formatBytes(s.bytes)}`, { action: { label: 'Open', run: () => setOpenId(s.id) } });
    } catch (e) {
      toast.error('Screenshot failed', errorText(e));
    } finally {
      setCountdown(null);
    }
  };

  const region = async () => {
    try {
      await api.shots.regionBegin();
      if (!inTauri) navigate('overlay');
    } catch (e) {
      toast.error('Could not start the capture', errorText(e));
    }
  };

  const toggleFav = async (s: Screenshot) => {
    shots.setData((prev) => (prev ?? []).map((x) => (x.id === s.id ? { ...x, favorite: !s.favorite } : x)));
    try {
      await api.shots.update(s.id, { favorite: !s.favorite });
    } catch (e) {
      toast.error('Could not update', errorText(e));
      void shots.reload();
    }
  };

  return (
    <Page
      title="Screenshots"
      subtitle={shots.data ? `${formatNumber(all.length)} screenshots · ${formatBytes(totalBytes)} · ${info?.screenshotDir ?? ''}` : 'Loading…'}
      actions={
        <>
          <Button variant="primary" icon={Crop} onClick={region} title={`Freeze the screen and drag a rectangle (${hotkeys?.hotkeyRegion ?? 'Alt+Shift+S'})`}>
            Region
          </Button>
          <Button icon={Monitor} onClick={() => void capture('screen')} title={hotkeys?.hotkeyFull}>
            Screen
          </Button>
          <Button icon={AppWindow} onClick={() => void capture('window')} title={hotkeys?.hotkeyWindow}>
            Window
          </Button>
          <Button icon={MonitorSmartphone} onClick={() => void capture('allScreens')}>
            All screens
          </Button>
          <div className="flex items-center gap-1.5 pl-1">
            <Timer size={14} className="text-faint" aria-hidden />
            <Segmented
              size="sm"
              label="Delay"
              value={delay}
              onChange={setDelay}
              options={[
                { value: '0', label: 'Now' },
                { value: '3', label: '3 s' },
                { value: '5', label: '5 s' },
              ]}
            />
          </div>
          <IconButton icon={RefreshCw} label="Re-read the screenshot folder" variant="secondary" onClick={() => void api.shots.sync().then((n) => toast.info(n ? `Found ${n} new screenshots` : 'The folder is up to date'))} />
        </>
      }
      headerExtra={
        <div className="flex flex-wrap items-center gap-2">
          <SearchInput value={q} onChange={setQ} placeholder="Search titles, notes, tags" className="w-64" aria-label="Search screenshots" />
          <Select value={app} onChange={(e) => setApp(e.target.value)} aria-label="App" className="w-[170px]">
            <option value="">All apps</option>
            {apps.map((a) => (
              <option key={a} value={a}>
                {exeLabel(a)}
              </option>
            ))}
          </Select>
          <button type="button" onClick={() => setFavs(!favs)} aria-pressed={favs} className={cx('inline-flex h-9 items-center gap-1.5 rounded-[10px] border px-3 text-[13px] transition-colors', favs ? 'border-warn/40 bg-warn/12 text-warn' : 'border-line bg-surface text-dim hover:text-fg')}>
            <Star size={14} fill={favs ? 'currentColor' : 'none'} aria-hidden /> Favourites
          </button>
          <div className="mx-1 h-5 w-px bg-line" />
          <div className="flex flex-wrap items-center gap-1.5">
            {tags.slice(0, 10).map(([t, n]) => (
              <button key={t} type="button" onClick={() => setTag(tag === t ? null : t)} aria-pressed={tag === t} className={cx('inline-flex h-7 items-center gap-1 rounded-full border px-2.5 text-[12px] transition-colors', tag === t ? 'border-accent/40 bg-accent-soft text-accent' : 'border-line bg-surface text-dim hover:text-fg')}>
                <Tag size={11} aria-hidden />
                {t}
                <span className="text-faint">{n}</span>
              </button>
            ))}
          </div>
        </div>
      }
    >
      <AnimatePresence>
        {countdown != null && (
          <motion.div initial={{ opacity: 0, y: -10, scale: 0.95 }} animate={{ opacity: 1, y: 0, scale: 1 }} exit={{ opacity: 0, scale: 0.95 }} className="glass fixed left-1/2 top-6 z-50 flex -translate-x-1/2 items-center gap-3 rounded-full border border-line-strong px-5 py-2.5 shadow-xl">
            <Camera size={16} className="text-accent" aria-hidden />
            <span className="text-[14px] font-medium text-fg">
              Capturing in <span className="font-mono tabular">{countdown}</span>…
            </span>
          </motion.div>
        )}
      </AnimatePresence>

      {shots.error ? (
        <ErrorState error={shots.error} onRetry={shots.reload} />
      ) : !shots.data ? (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-3">
          {Array.from({ length: 12 }, (_, i) => (
            <Skeleton key={i} className="aspect-video rounded-2xl" />
          ))}
        </div>
      ) : list.length === 0 ? (
        all.length === 0 ? (
          <EmptyState
            icon={Camera}
            title="No screenshots yet"
            description={
              <>
                Press <Kbd>{hotkeys?.hotkeyRegion ?? 'Alt+Shift+S'}</Kbd> anywhere to capture a region, or use the buttons above.
              </>
            }
          />
        ) : (
          <EmptyState icon={ImageOff} title="Nothing matches these filters" action={<Button size="sm" onClick={() => (setQ(''), setTag(null), setApp(''), setFavs(false))}>Clear filters</Button>} />
        )
      ) : (
        <div className="space-y-6">
          {groups.map((g) => (
            <section key={g.key}>
              <h2 className="mb-2.5 flex items-baseline gap-2 font-display text-[14px] font-semibold text-fg">
                {g.label}
                <span className="text-[12px] font-normal text-faint">{g.items.length}</span>
              </h2>
              <div className="grid grid-cols-[repeat(auto-fill,minmax(240px,1fr))] gap-3">
                <AnimatePresence initial={false}>
                  {g.items.map((s) => (
                    <motion.div key={s.id} layout initial={fresh.has(s.id) ? { opacity: 0, scale: 0.9 } : false} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0, scale: 0.95 }} transition={{ type: 'spring', stiffness: 320, damping: 28 }} className="group relative">
                      <button type="button" onClick={() => setOpenId(s.id)} className="card card-interactive block w-full overflow-hidden p-0 text-left" aria-label={`Open screenshot ${s.appTitle ?? 'full screen'}`}>
                        <ShotThumb id={s.id} alt={s.appTitle ?? 'Screenshot'} className="aspect-video w-full" exists={s.exists} />
                        <div className="flex items-center gap-2 px-3 py-2">
                          <div className="min-w-0 flex-1">
                            <div className="truncate text-[12.5px] font-medium text-fg">{s.appTitle ?? 'Full screen'}</div>
                            <div className="truncate text-[11px] text-faint">
                              {new Date(s.created * 1000).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })} · {s.width}×{s.height}
                              {s.tags.length > 0 && ` · ${s.tags.join(', ')}`}
                            </div>
                          </div>
                        </div>
                      </button>
                      <button
                        type="button"
                        onClick={() => void toggleFav(s)}
                        aria-label={s.favorite ? 'Remove from favourites' : 'Add to favourites'}
                        aria-pressed={s.favorite}
                        className={cx('absolute right-2 top-2 flex h-8 w-8 items-center justify-center rounded-full bg-black/45 backdrop-blur transition-opacity', s.favorite ? 'text-warn opacity-100' : 'text-white opacity-0 group-hover:opacity-100 focus-visible:opacity-100')}
                      >
                        <Star size={15} fill={s.favorite ? 'currentColor' : 'none'} />
                      </button>
                      {s.note && <div className="pointer-events-none absolute left-2 top-2 max-w-[70%] truncate rounded-lg bg-black/50 px-2 py-0.5 text-[11px] text-white/90 backdrop-blur">{s.note}</div>}
                    </motion.div>
                  ))}
                </AnimatePresence>
              </div>
            </section>
          ))}
        </div>
      )}
      <Lightbox
        shots={list}
        openId={openId}
        onClose={() => {
          setOpenId(null);
          if (route.params.get('open')) navigate('screenshots');
        }}
        onNavigate={setOpenId}
        onChange={(s) => shots.setData((prev) => (prev ?? []).map((x) => (x.id === s.id ? s : x)))}
      />
    </Page>
  );
}
