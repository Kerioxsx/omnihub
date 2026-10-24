// Open apps: every program with a window on the PC. "Close" works like
// clicking the window's ×; "Quit" ends the program (for apps that only hide
// in the tray, like Discord or Steam).

import { AppWindow, Power, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { client, type OpenApp } from '../client';
import { toast, useApp } from '../state';
import { PullToRefresh } from '../ui/PullToRefresh';
import { Empty, ErrorState, ListSkeleton } from '../ui/common';
import { SubHeader } from './More';
import { AppAvatar } from './Sound';
import { cx, errorMessage, useInterval, usePageVisible, vibrate } from '../lib/util';

const icons = new Map<string, string | null>();

function useIcon(key: string): string | null {
  const [icon, setIcon] = useState(() => icons.get(key) ?? null);
  useEffect(() => {
    if (icons.has(key)) return;
    icons.set(key, null);
    client.openAppIcon(key).then(
      (r) => {
        icons.set(key, r.icon);
        setIcon(r.icon);
      },
      () => undefined,
    );
  }, [key]);
  return icon;
}

function AppCard({ app, onClose, onQuit, busy }: { app: OpenApp; onClose: () => void; onQuit: () => void; busy: boolean }) {
  const icon = useIcon(app.key);
  const [arming, setArming] = useState(false);
  useEffect(() => {
    if (!arming) return;
    const t = setTimeout(() => setArming(false), 3000);
    return () => clearTimeout(t);
  }, [arming]);
  return (
    <div className="flex items-center gap-3 px-4 py-3">
      <AppAvatar name={app.name} icon={icon} size={42} />
      <div className="min-w-0 flex-1">
        <div className="truncate text-[15px] font-semibold">{app.name}</div>
        <div className="truncate text-[12.5px] text-dim">{app.titles[0]}{app.titles.length > 1 && ` · +${app.titles.length - 1} more`}</div>
      </div>
      <button type="button" disabled={busy} onClick={onClose} className="press flex h-10 shrink-0 items-center gap-1.5 rounded-xl bg-surface-2 px-3 text-[13.5px] font-semibold disabled:opacity-50" aria-label={`Close ${app.name}`}>
        <X size={16} /> Close
      </button>
      {app.canQuit && (
        <button
          type="button"
          disabled={busy}
          onClick={() => {
            if (!arming) {
              vibrate(8);
              setArming(true);
              return;
            }
            setArming(false);
            onQuit();
          }}
          className={cx('press flex h-10 shrink-0 items-center gap-1.5 rounded-xl px-3 text-[13.5px] font-semibold transition-colors disabled:opacity-50', arming ? 'bg-bad text-white' : 'bg-bad/15 text-bad')}
          aria-label={arming ? `Tap again to quit ${app.name}` : `Quit ${app.name}`}
        >
          <Power size={15} /> {arming ? 'Sure?' : 'Quit'}
        </button>
      )}
    </div>
  );
}

export function OpenAppsPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const enabled = !!useApp((s) => s.info?.features.tasks);
  const visible = usePageVisible();
  const [apps, setApps] = useState<OpenApp[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const load = useCallback(async () => {
    try {
      setApps((await client.openApps()).apps);
      setError(null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);
  useEffect(() => {
    if (enabled && active) void load();
  }, [enabled, active, load]);
  useInterval(() => void load(), enabled && active && visible ? 4000 : null);

  const run = async (app: OpenApp, how: 'close' | 'quit') => {
    setBusy(app.key);
    vibrate(10);
    try {
      if (how === 'close') {
        await client.closeApp(app.key);
        toast.success(`Asked ${app.name} to close`, 'If it asks to save something, answer on the PC. Apps that only hide in the tray need Quit.');
      } else {
        await client.endTask(app.exeName);
        toast.success(`${app.name} quit`);
      }
      setApps((l) => l?.filter((a) => how === 'close' || a.key !== app.key) ?? l);
      setTimeout(() => void load(), 900);
    } catch (e) {
      toast.error(how === 'close' ? `Could not close ${app.name}` : `Could not quit ${app.name}`, errorMessage(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <div className="flex h-full flex-col">
      <SubHeader title="Open apps" subtitle={apps ? `${apps.length} with a window on the PC` : 'Programs on the PC'} onBack={onBack} />
      <PullToRefresh onRefresh={load} className="min-h-0 flex-1">
        <div className="px-safe pb-tabbar">
          {!enabled ? (
            <Empty icon={<AppWindow size={28} />} title="Turned off on the PC" body="Tasks from the phone are off in OmniHub → Phone." />
          ) : error && !apps ? (
            <ErrorState message={error} onRetry={() => void load()} />
          ) : !apps ? (
            <ListSkeleton rows={6} />
          ) : apps.length === 0 ? (
            <Empty icon={<AppWindow size={28} />} title="No open windows" body="Programs with a window on the PC show up here." />
          ) : (
            <div className="card divide-y divide-[var(--line)] overflow-hidden">
              {apps.map((a) => (
                <AppCard key={a.key} app={a} busy={busy === a.key} onClose={() => void run(a, 'close')} onQuit={() => void run(a, 'quit')} />
              ))}
            </div>
          )}
          <p className="mt-4 px-2 text-center text-xs leading-relaxed text-faint">Close works like clicking × on the PC — the app can still ask to save. Quit ends it right away (tap twice), for apps like Discord, Steam or Spotify that only hide when closed.</p>
        </div>
      </PullToRefresh>
    </div>
  );
}
