// Root of the phone app: boot → pair or main UI (tabs), live events,
// offline banner, toasts.

import { useCallback, useEffect, useState } from 'react';
import { AnimatePresence, motion } from 'motion/react';
import { WifiOff, RefreshCw, LoaderCircle } from 'lucide-react';
import type { InboxItem, PendingPower, PowerAction } from '@shared/types';
import { client, connectEvents, getToken, pairSecretFromUrl, setToken } from './client';
import { useApp, useInbox, usePower, toast, TAB_ORDER, type Tab } from './state';
import { useUploads, isActive } from './uploads';
import { emitEvent } from './lib/events';
import { useBackHandler } from './lib/back';
import { cx, errorMessage, useInterval } from './lib/util';
import { TabBar } from './ui/TabBar';
import { Toasts } from './ui/Toasts';
import { Logo } from './ui/Logo';
import { Button } from './ui/common';
import { PairScreen } from './screens/Pair';
import { HomeScreen } from './screens/Home';
import { FilesScreen } from './screens/Files';
import { ScreenScreen } from './screens/Screen';
import { PowerScreen } from './screens/Power';
import { MoreScreen, type MorePage } from './screens/More';

type Phase = 'boot' | 'pair' | 'ready' | 'unreachable';

const POWER_DONE: Record<PowerAction, string> = {
  lock: 'PC locked',
  displayOff: 'Display turned off',
  sleep: 'PC is going to sleep',
  hibernate: 'PC is hibernating',
  signOut: 'Signed out',
  restart: 'PC is restarting',
  shutdown: 'PC is shutting down',
};

export function App() {
  const [phase, setPhase] = useState<Phase>('boot');
  const [bootError, setBootError] = useState<string | null>(null);
  const { info, setInfo, setSocket, setOnline, tab, prevTab, setTab, refreshInfo } = useApp();
  const [visited, setVisited] = useState<Set<Tab>>(new Set(['home']));
  const [morePage, setMorePage] = useState<MorePage | null>(null);

  const boot = useCallback(async () => {
    try {
      const i = await client.info();
      setInfo(i);
      setBootError(null);
      const token = getToken();
      if (token && i.paired) {
        if (pairSecretFromUrl()) history.replaceState(history.state, '', location.pathname + location.search);
        setPhase('ready');
      } else {
        if (token) setToken(null);
        setPhase('pair');
      }
    } catch (e) {
      setBootError(errorMessage(e));
      setPhase('unreachable');
    }
  }, [setInfo]);

  useEffect(() => {
    boot();
  }, [boot]);
  useInterval(boot, phase === 'unreachable' ? 5000 : null);

  // Token rejected anywhere → back to pairing.
  useEffect(() => {
    const on = () => {
      setToken(null);
      setPhase((p) => {
        if (p === 'ready') toast.error('This phone is no longer paired', 'Pair it again from the PC.');
        return 'pair';
      });
      refreshInfo();
    };
    window.addEventListener('omnihub:unauthorized', on);
    return () => window.removeEventListener('omnihub:unauthorized', on);
  }, [refreshInfo]);

  useEffect(() => {
    const on = () => setOnline(navigator.onLine);
    window.addEventListener('online', on);
    window.addEventListener('offline', on);
    return () => {
      window.removeEventListener('online', on);
      window.removeEventListener('offline', on);
    };
  }, [setOnline]);

  // Live events while paired.
  useEffect(() => {
    if (phase !== 'ready') return;
    const stop = connectEvents((topic, payload) => {
      emitEvent(topic, payload);
      const power = usePower.getState();
      switch (topic) {
        case 'inbox:new': {
          const item = (payload?.item ?? payload) as InboxItem;
          if (!item?.id) break;
          useInbox.getState().add(item);
          toast.info('Your PC sent a file', item.name, { label: 'Download', run: () => client.receive(item).catch((e) => toast.error("Couldn't download", errorMessage(e))) });
          break;
        }
        case 'power:pending':
          power.setPending(payload as PendingPower);
          break;
        case 'power:cancelled': {
          const had = power.pending;
          if (!payload?.id || had?.id === payload.id) power.setPending(null);
          if (payload?.reason !== 'replaced' && had && !power.cancelling) toast.info(`${had.label} cancelled`, 'Cancelled on the PC.');
          break;
        }
        case 'power:executed': {
          power.setPending(null);
          const a = payload?.action as PowerAction;
          if (payload?.ok) toast.success(POWER_DONE[a] ?? 'Done', payload?.dryRun ? 'Dry run — nothing really happened on the PC.' : undefined);
          else toast.error("The PC couldn't do that", payload?.error ?? undefined);
          break;
        }
      }
    }, setSocket);
    return stop;
  }, [phase, setSocket]);

  // Keep feature flags fresh.
  useEffect(() => {
    const on = () => document.visibilityState === 'visible' && phase === 'ready' && refreshInfo();
    document.addEventListener('visibilitychange', on);
    return () => document.removeEventListener('visibilitychange', on);
  }, [phase, refreshInfo]);
  useInterval(refreshInfo, phase === 'ready' ? 60000 : null);

  const hidden: Tab[] = [];
  if (info && !info.features.screen) hidden.push('screen');
  if (info && !info.features.power) hidden.push('power');
  useEffect(() => {
    if (hidden.includes(tab)) setTab('home');
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hidden.join(), tab]);
  useEffect(() => {
    setVisited((v) => (v.has(tab) ? v : new Set(v).add(tab)));
  }, [tab]);

  const activeUploads = useUploads((s) => s.items.filter((i) => isActive(i.state)).length);
  const inboxCount = useInbox((s) => s.items.length);

  if (phase === 'boot') {
    return (
      <div className="grid h-full place-items-center">
        <div className="flex flex-col items-center gap-5">
          <Logo size={72} className="animate-pulse" />
          <LoaderCircle size={22} className="spin text-faint" />
        </div>
      </div>
    );
  }
  if (phase === 'unreachable') {
    return (
      <div className="px-safe grid h-full place-items-center pb-[var(--safe-bottom)] pt-[var(--safe-top)]">
        <div className="fade-up flex max-w-[340px] flex-col items-center text-center">
          <div className="relative">
            <Logo size={72} className="opacity-60 grayscale-[.4]" />
            <div className="absolute -bottom-1 -right-1 grid h-8 w-8 place-items-center rounded-full border-4 border-[var(--bg)] bg-bad text-white">
              <WifiOff size={14} />
            </div>
          </div>
          <h1 className="mt-6 font-display text-2xl font-bold">Can't reach your PC</h1>
          <p className="mt-2 text-[15px] text-dim">Make sure the PC is on, OmniHub's phone companion is running, and this phone is on the same Wi-Fi.</p>
          {bootError && <p className="mt-3 rounded-xl bg-surface px-3 py-2 text-xs text-faint">{bootError}</p>}
          <Button className="mt-6" icon={<RefreshCw size={18} />} onClick={boot}>
            Try again
          </Button>
          <p className="mt-3 text-xs text-faint">Retrying automatically…</p>
        </div>
      </div>
    );
  }
  if (phase === 'pair') {
    return (
      <>
        <PairScreen
          info={info}
          onPaired={(name) => {
            setTab('home');
            setVisited(new Set(['home']));
            setPhase('ready');
            refreshInfo();
            toast.success(`Paired with ${name}`, 'You can now use your PC from this phone.');
          }}
        />
        <Toasts aboveTabs={false} />
      </>
    );
  }

  const dir = TAB_ORDER.indexOf(tab) >= TAB_ORDER.indexOf(prevTab) ? 'tab-in-right' : 'tab-in-left';
  const panel = (t: Tab, node: React.ReactNode) =>
    visited.has(t) && (
      <div key={t} className={cx('absolute inset-0', tab === t && dir)} hidden={tab !== t} aria-hidden={tab !== t}>
        {node}
      </div>
    );

  return (
    <div className="relative h-full">
      <ConnectionBanner />
      <main className="absolute inset-0 overflow-hidden">
        {panel('home', <HomeScreen active={tab === 'home'} openMore={(p) => (setMorePage(p), setTab('more'))} />)}
        {panel('files', <FilesScreen active={tab === 'files'} />)}
        {!hidden.includes('screen') && panel('screen', <ScreenScreen active={tab === 'screen'} />)}
        {!hidden.includes('power') && panel('power', <PowerScreen active={tab === 'power'} />)}
        {panel(
          'more',
          <MoreScreen
            active={tab === 'more'}
            page={morePage}
            setPage={setMorePage}
            onUnpaired={() => {
              setToken(null);
              setMorePage(null);
              setTab('home');
              setPhase('pair');
              refreshInfo();
            }}
          />,
        )}
      </main>
      <BackToHome active={tab !== 'home' && !(tab === 'more' && morePage)} onBack={() => setTab('home')} />
      <TabBar hidden={hidden} badges={{ files: activeUploads, home: inboxCount }} />
      <Toasts aboveTabs />
    </div>
  );
}

/** Android back on a non-home tab returns to Home. */
function BackToHome({ active, onBack }: { active: boolean; onBack: () => void }) {
  useBackHandler(active, onBack);
  return null;
}

/** "Offline" / "Reconnecting" strip under the status bar. */
function ConnectionBanner() {
  const { online, socket, info } = useApp();
  const [show, setShow] = useState(false);
  const bad = !online || socket !== 'open';
  useEffect(() => {
    if (!bad) return setShow(false);
    const t = setTimeout(() => setShow(true), online ? 3500 : 300);
    return () => clearTimeout(t);
  }, [bad, online]);
  return (
    <AnimatePresence>
      {show && (
        <motion.div
          initial={{ y: -60 }}
          animate={{ y: 0 }}
          exit={{ y: -60 }}
          transition={{ type: 'spring', damping: 28, stiffness: 320 }}
          className="pointer-events-none fixed inset-x-0 top-0 z-[55] flex justify-center px-3 pt-[calc(var(--safe-top)+6px)]"
          role="status"
        >
          <div className={cx('flex items-center gap-2 rounded-full px-3.5 py-1.5 text-xs font-semibold shadow-lg', online ? 'bg-warn text-black' : 'bg-bad text-white')}>
            {online ? <LoaderCircle size={14} className="spin" /> : <WifiOff size={14} />}
            {online ? `Reconnecting to ${info?.name ?? 'your PC'}…` : "You're offline — check Wi-Fi"}
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
