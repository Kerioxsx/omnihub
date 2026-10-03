import { AnimatePresence, MotionConfig, motion } from 'motion/react';
import { type ComponentType, useEffect, useState } from 'react';
import { CommandPalette } from './components/CommandPalette';
import { ConfirmHost } from './components/ConfirmHost';
import { GlobalEvents, ThemeController } from './components/GlobalEvents';
import { Logo } from './components/Logo';
import { PowerBanner } from './components/PowerBanner';
import { Sidebar } from './components/Sidebar';
import { Toaster } from './components/Toaster';
import { MenuHost } from './components/ui/Menu';
import { overlayOpen } from './components/ui/Overlay';
import { ErrorState } from './components/ui/States';
import { useStoredState } from './lib/hooks';
import { PAGE_IDS, type RouteId, navigate, useRoute } from './lib/router';
import { AppsPage } from './pages/apps/AppsPage';
import { HomePage } from './pages/home/HomePage';
import { NotesPage } from './pages/notes/NotesPage';
import { Onboarding } from './pages/onboarding/Onboarding';
import { PhonePage } from './pages/phone/PhonePage';
import { ScreenPage } from './pages/screen/ScreenPage';
import { SettingsPage } from './pages/settings/SettingsPage';
import { ScreenshotsPage } from './pages/shots/ScreenshotsPage';
import { StoragePage } from './pages/storage/StoragePage';
import { VaultPage } from './pages/vault/VaultPage';
import { usePalette } from './state/dialogs';
import { useSettings } from './state/settings';

const PAGES: Record<Exclude<RouteId, 'overlay'>, ComponentType> = {
  home: HomePage,
  storage: StoragePage,
  apps: AppsPage,
  screenshots: ScreenshotsPage,
  notes: NotesPage,
  vault: VaultPage,
  phone: PhonePage,
  screen: ScreenPage,
  settings: SettingsPage,
};

const forceOnboarding = new URLSearchParams(window.location.search).get('onboarding') === '1';

function useGlobalShortcuts() {
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
  const [collapsed, setCollapsed] = useStoredState('omnihub.sidebarCollapsed', false);
  const [onboardingForced, setOnboardingForced] = useState(forceOnboarding);
  useGlobalShortcuts();

  useEffect(() => {
    void load();
  }, [load]);

  if (!settings) return <Splash error={error} onRetry={() => void load()} />;

  const pageId = route.id === 'overlay' ? 'home' : route.id;
  const PageComp = PAGES[pageId];
  const showOnboarding = onboardingForced || !settings.general.onboarded;

  return (
    <MotionConfig reducedMotion={settings.general.reducedMotion ? 'always' : 'user'}>
      <ThemeController />
      <GlobalEvents />
      <div className="app-backdrop" />
      <div className="relative z-10 flex h-full">
        <Sidebar collapsed={collapsed} onToggle={() => setCollapsed(!collapsed)} />
        <main className="relative min-w-0 flex-1">
          <AnimatePresence mode="wait" initial={false}>
            <motion.div
              key={pageId}
              className="absolute inset-0"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0, transition: { duration: 0.22, ease: [0.22, 1, 0.36, 1] } }}
              exit={{ opacity: 0, y: -6, transition: { duration: 0.1 } }}
            >
              <PageComp />
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
      <Toaster />
      <AnimatePresence>{showOnboarding && <Onboarding key="onboarding" onDone={() => setOnboardingForced(false)} />}</AnimatePresence>
    </MotionConfig>
  );
}
