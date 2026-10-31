import { AnimatePresence, MotionConfig, motion } from 'motion/react';
import { type ComponentType, Suspense, lazy, useEffect, useState } from 'react';
import { CommandPalette } from './components/CommandPalette';
import { ConfirmHost } from './components/ConfirmHost';
import { BrowserPairDialog } from './components/BrowserPairDialog';
import { DropOverlay } from './components/DropOverlay';
import { ErrorBoundary } from './components/ErrorBoundary';
import { GlobalEvents, ThemeController } from './components/GlobalEvents';
import { Logo } from './components/Logo';
import { PowerBanner } from './components/PowerBanner';
import { isWindowFullscreen, setWindowFullscreen, usePlayer } from './lib/nowPlaying';
import { SendDialog } from './components/SendDialog';
import { Sidebar } from './components/Sidebar';
import { Toaster } from './components/Toaster';
import { MenuHost } from './components/ui/Menu';
import { overlayOpen } from './components/ui/Overlay';
import { ErrorState } from './components/ui/States';
import { useMediaQuery, useStoredState } from './lib/hooks';
import { PAGE_IDS, type RouteId, navigate, useRoute } from './lib/router';
import { usePalette } from './state/dialogs';
import { useSettings } from './state/settings';

// Pages load the first time they are opened.
const AppsPage = lazy(() => import('./pages/apps/AppsPage').then((m) => ({ default: m.AppsPage })));
const GamesPage = lazy(() => import('./pages/games/GamesPage').then((m) => ({ default: m.GamesPage })));
const LightsPage = lazy(() => import('./pages/lights/LightsPage').then((m) => ({ default: m.LightsPage })));
const ModsPage = lazy(() => import('./pages/mods/ModsPage').then((m) => ({ default: m.ModsPage })));
const ServersPage = lazy(() => import('./pages/servers/ServersPage').then((m) => ({ default: m.ServersPage })));
const ChatsPage = lazy(() => import('./pages/chats/ChatsPage').then((m) => ({ default: m.ChatsPage })));
const RigPage = lazy(() => import('./pages/rig/RigPage').then((m) => ({ default: m.RigPage })));
const MusicPage = lazy(() => import('./pages/music/MusicPage').then((m) => ({ default: m.MusicPage })));
const HomePage = lazy(() => import('./pages/home/HomePage').then((m) => ({ default: m.HomePage })));
const NotesPage = lazy(() => import('./pages/notes/NotesPage').then((m) => ({ default: m.NotesPage })));
const PhonePage = lazy(() => import('./pages/phone/PhonePage').then((m) => ({ default: m.PhonePage })));
const ScreenPage = lazy(() => import('./pages/screen/ScreenPage').then((m) => ({ default: m.ScreenPage })));
const SettingsPage = lazy(() => import('./pages/settings/SettingsPage').then((m) => ({ default: m.SettingsPage })));
const ScreenshotsPage = lazy(() => import('./pages/shots/ScreenshotsPage').then((m) => ({ default: m.ScreenshotsPage })));
const StoragePage = lazy(() => import('./pages/storage/StoragePage').then((m) => ({ default: m.StoragePage })));
const ToolsPage = lazy(() => import('./pages/tools/ToolsPage').then((m) => ({ default: m.ToolsPage })));
const TasksPage = lazy(() => import('./pages/tasks/TasksPage').then((m) => ({ default: m.TasksPage })));
const VaultPage = lazy(() => import('./pages/vault/VaultPage').then((m) => ({ default: m.VaultPage })));
const Onboarding = lazy(() => import('./pages/onboarding/Onboarding').then((m) => ({ default: m.Onboarding })));
const PlayerHost = lazy(() => import('./components/music/PlayerHost').then((m) => ({ default: m.PlayerHost })));

const PAGES: Record<Exclude<RouteId, 'overlay' | 'ambient' | 'casescreen' | 'pin'>, ComponentType> = {
  home: HomePage,
  storage: StoragePage,
  apps: AppsPage,
  screenshots: ScreenshotsPage,
  notes: NotesPage,
  music: MusicPage,
  tasks: TasksPage,
  tools: ToolsPage,
  games: GamesPage,
  mods: ModsPage,
  servers: ServersPage,
  chats: ChatsPage,
  rig: RigPage,
  lights: LightsPage,
  vault: VaultPage,
  phone: PhonePage,
  screen: ScreenPage,
  settings: SettingsPage,
};

const forceOnboarding = new URLSearchParams(window.location.search).get('onboarding') === '1';

function useGlobalShortcuts() {
  // Never stuck in full screen: F11 toggles it anywhere, Esc leaves it when
  // no full-screen player is open (the player handles Esc itself). And the
  // app starts windowed (an earlier version could restore full screen).
  useEffect(() => {
    void setWindowFullscreen(false);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'F11') {
        e.preventDefault();
        void isWindowFullscreen().then((on) => setWindowFullscreen(!on));
        return;
      }
      if (e.key === 'Escape' && !document.querySelector('[data-full-player="fullscreen"]')) {
        void isWindowFullscreen().then((on) => (on ? setWindowFullscreen(false) : undefined));
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      if (!mod || e.altKey) return;
      if (e.key.toLowerCase() === 'k') {
        e.preventDefault();
        const p = usePalette.getState();
        p.setOpen(!p.open);
      } else if (/^[1-9]$/.test(e.key) && !e.shiftKey) {
        if (overlayOpen() && !usePalette.getState().open) return;
        e.preventDefault();
        usePalette.getState().setOpen(false);
        navigate(PAGE_IDS[Number(e.key) - 1]);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);
}

function Splash({ error, onRetry }: { error: string | null; onRetry: () => void }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-5">
      <div className="app-backdrop" />
      <motion.div initial={{ scale: 0.9, opacity: 0 }} animate={{ scale: 1, opacity: 1 }} transition={{ type: 'spring', stiffness: 200, damping: 20 }}>
        <Logo size={64} />
      </motion.div>
      {error ? <ErrorState title="OmniHub could not start" error={error} onRetry={onRetry} /> : <div className="h-1 w-40 overflow-hidden rounded-full bg-surface-3 relative indeterminate" />}
    </div>
  );
}

export function App() {
  const settings = useSettings((s) => s.settings);
  const error = useSettings((s) => s.error);
  const load = useSettings((s) => s.load);
  const route = useRoute();
  // The full-screen player loads the first time it opens.
  const playerOpen = usePlayer((p) => p.open);
  const [playerUsed, setPlayerUsed] = useState(false);
  useEffect(() => {
    if (playerOpen) setPlayerUsed(true);
  }, [playerOpen]);
  const [storedCollapsed, setStoredCollapsed] = useStoredState('omnihub.sidebarCollapsed', false);
  // Narrow windows collapse the sidebar automatically unless the user opens it.
  const narrow = useMediaQuery('(max-width: 1199px)');
  const [narrowExpanded, setNarrowExpanded] = useState(false);
  const collapsed = narrow ? !narrowExpanded : storedCollapsed;
  const toggleSidebar = () => (narrow ? setNarrowExpanded(!narrowExpanded) : setStoredCollapsed(!storedCollapsed));
  const [onboardingForced, setOnboardingForced] = useState(forceOnboarding);
  useGlobalShortcuts();

  useEffect(() => {
    void load();
  }, [load]);

  if (!settings) return <Splash error={error} onRetry={() => void load()} />;

  const pageId = route.id === 'overlay' || route.id === 'ambient' || route.id === 'casescreen' || route.id === 'pin' ? 'home' : route.id;
  const PageComp = PAGES[pageId];
  const showOnboarding = onboardingForced || !settings.general.onboarded;

  return (
    <MotionConfig reducedMotion={settings.general.reducedMotion ? 'always' : 'user'}>
      <ThemeController />
      <GlobalEvents />
      <div className="app-backdrop" />
      <div className="relative z-10 flex h-full">
        <Sidebar collapsed={collapsed} onToggle={toggleSidebar} />
        <main className="relative min-w-0 flex-1">
          <AnimatePresence mode="wait" initial={false}>
            <motion.div
              key={pageId}
              className="absolute inset-0"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0, transition: { duration: 0.22, ease: [0.22, 1, 0.36, 1] } }}
              exit={{ opacity: 0, y: -6, transition: { duration: 0.1 } }}
            >
              <ErrorBoundary name={pageId}>
                <Suspense fallback={null}>
                  <PageComp />
                </Suspense>
              </ErrorBoundary>
            </motion.div>
          </AnimatePresence>
          {pageId !== 'home' && (
            <div className="pointer-events-none absolute inset-x-0 top-4 z-40 flex justify-center">
              <PowerBanner />
            </div>
          )}
        </main>
      </div>
      <CommandPalette />
      <ConfirmHost />
      <MenuHost />
      {playerUsed && (
        <Suspense fallback={null}>
          <PlayerHost />
        </Suspense>
      )}
      <SendDialog />
      <DropOverlay />
      <BrowserPairDialog />
      <Toaster />
      <Suspense fallback={null}>
        <AnimatePresence>{showOnboarding && <Onboarding key="onboarding" onDone={() => setOnboardingForced(false)} />}</AnimatePresence>
      </Suspense>
    </MotionConfig>
  );
}
