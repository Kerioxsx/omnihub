// Apps: grid of launchable programs on the PC, icons loaded lazily.

import { useCallback, useEffect, useMemo, useState } from 'react';
import { Search, X, AppWindow, Rocket } from 'lucide-react';
import { client } from '../client';
import { toast } from '../state';
import { PullToRefresh } from '../ui/PullToRefresh';
import { Sheet } from '../ui/Sheet';
import { Button, Empty, ErrorState, Skeleton } from '../ui/common';
import { SubHeader } from './More';
import { cx, errorMessage, limiter, useInView, vibrate } from '../lib/util';

type App = { id: string; name: string; publisher: string; source: string };

const icons = new Map<string, string | null>();
const iconQueue = limiter(4);
const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

const HUES = [262, 199, 330, 152, 28, 220, 285, 175];
function hueFor(name: string) {
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return HUES[h % HUES.length];
}

function AppIcon({ app, size = 56 }: { app: App; size?: number }) {
  const [src, setSrc] = useState<string | null | undefined>(icons.get(app.id));
  const ref = useInView<HTMLDivElement>(() => {
    if (icons.has(app.id)) return setSrc(icons.get(app.id));
    iconQueue(() => client.appIcon(app.id)).then(
      (r) => {
        icons.set(app.id, r.icon);
        setSrc(r.icon);
      },
      () => {
        icons.set(app.id, null);
        setSrc(null);
      },
    );
  });
  const hue = hueFor(app.name);
  return (
    <div ref={ref} className="grid shrink-0 place-items-center overflow-hidden rounded-[18px]" style={{ width: size, height: size, background: src ? 'var(--surface-2)' : `linear-gradient(135deg, hsl(${hue} 70% 55%), hsl(${(hue + 40) % 360} 70% 45%))` }}>
      {src ? (
        <img src={src} alt="" className="h-[70%] w-[70%] object-contain" draggable={false} />
      ) : src === undefined ? (
        <div className="skeleton h-full w-full rounded-none" />
      ) : (
        <span className="font-display text-[22px] font-bold text-white">{app.name.trim().charAt(0).toUpperCase()}</span>
      )}
    </div>
  );
}

export function AppsPage({ onBack }: { onBack: () => void }) {
  const [apps, setApps] = useState<App[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [q, setQ] = useState('');
  const [pick, setPick] = useState<App | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    try {
      const r = await client.apps();
      setApps(r.apps.sort((a, b) => collator.compare(a.name, b.name)));
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);
  useEffect(() => {
    load();
  }, [load]);

  const shown = useMemo(() => {
    const s = q.trim().toLowerCase();
    return (apps ?? []).filter((a) => !s || a.name.toLowerCase().includes(s) || a.publisher.toLowerCase().includes(s));
  }, [apps, q]);

  const launch = async () => {
    if (!pick) return;
    setBusy(true);
    try {
      const r = await client.launch(pick.id);
      vibrate(12);
      toast.success(`Opening ${r.launched || pick.name}`, 'on the PC');
      setPick(null);
    } catch (e) {
      toast.error(`Couldn't open ${pick.name}`, errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="h-full">
      <PullToRefresh onRefresh={load}>
        <SubHeader title="Apps" subtitle={apps ? `${apps.length} on the PC` : 'On the PC'} onBack={onBack} />
        <div className="px-safe pb-tabbar">
          <div className="relative">
            <Search size={17} className="pointer-events-none absolute left-3.5 top-1/2 -translate-y-1/2 text-faint" />
            <input className="field h-11 py-0 pl-10 pr-10" type="search" placeholder="Search apps" value={q} onChange={(e) => setQ(e.target.value)} enterKeyHint="search" />
            {q && (
              <button aria-label="Clear search" onClick={() => setQ('')} className="absolute right-2 top-1/2 grid h-8 w-8 -translate-y-1/2 place-items-center rounded-full text-faint">
                <X size={16} />
              </button>
            )}
          </div>
          <div className="mt-4">
            {error && !apps ? (
              <ErrorState message={error} onRetry={load} />
            ) : !apps ? (
              <div className="grid grid-cols-4 gap-x-2 gap-y-5">
                {Array.from({ length: 16 }, (_, i) => (
                  <div key={i} className="flex flex-col items-center gap-2">
                    <Skeleton className="h-14 w-14 rounded-[18px]" />
                    <Skeleton className="h-2.5 w-12" />
                  </div>
                ))}
              </div>
            ) : shown.length === 0 ? (
              <Empty icon={<AppWindow size={28} />} title={q ? 'No matches' : 'No apps found'} body={q ? `Nothing named like “${q}”.` : 'OmniHub found no launchable apps on this PC.'} />
            ) : (
              <div className="grid grid-cols-4 gap-x-2 gap-y-5">
                {shown.map((a) => (
                  <button key={a.id} onClick={() => setPick(a)} className="press flex min-w-0 flex-col items-center gap-1.5 text-center">
                    <AppIcon app={a} />
                    <span className="line-clamp-2 w-full break-words text-[11.5px] font-medium leading-tight">{a.name}</span>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>
      </PullToRefresh>
      <Sheet
        open={!!pick}
        onClose={() => setPick(null)}
        footer={
          <Button className="w-full" size="lg" loading={busy} onClick={launch} icon={<Rocket size={19} />}>
            Open on the PC
          </Button>
        }
      >
        {pick && (
          <div className="flex flex-col items-center pb-2 pt-1 text-center">
            <AppIcon app={pick} size={84} />
            <div className="mt-4 font-display text-xl font-bold">{pick.name}</div>
            <div className={cx('mt-1 text-sm text-dim')}>{[pick.publisher, pick.source === 'store' ? 'Microsoft Store' : pick.source === 'startMenu' ? 'Start menu' : 'Desktop app'].filter(Boolean).join(' · ')}</div>
            <div className="mt-3 rounded-xl bg-surface px-3 py-2 text-xs text-faint">It opens on the PC's screen, not on this phone.</div>
          </div>
        )}
      </Sheet>
    </div>
  );
}
