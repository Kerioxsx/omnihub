import { StrictMode, Suspense, lazy, useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import '@shared/theme.css';
import './styles.css';
import { inTauri } from './api';
import { useRoute } from './lib/router';

// Each window loads only what it shows: the glow and case-screen windows
// never parse the app's pages, and the app never parses theirs.
const App = lazy(() => import('./App').then((m) => ({ default: m.App })));
const AmbientOverlay = lazy(() => import('./pages/overlay/AmbientOverlay').then((m) => ({ default: m.AmbientOverlay })));
const CaseScreen = lazy(() => import('./pages/overlay/CaseScreen').then((m) => ({ default: m.CaseScreen })));
const RegionOverlay = lazy(() => import('./pages/overlay/RegionOverlay').then((m) => ({ default: m.RegionOverlay })));

/**
 * The region overlay runs in its own borderless window labelled "overlay"
 * (opened at index.html#/overlay), the screen glow in windows labelled
 * "ambient-…" (#/ambient), a case screen in "casescreen" (#/casescreen). The label check is a fallback in case the hash
 * gets lost; null while it is being resolved.
 */
function useOverlayWindow(): 'overlay' | 'ambient' | 'casescreen' | false | null {
  const [label, setLabel] = useState<'overlay' | 'ambient' | 'casescreen' | false | null>(inTauri ? null : false);
  useEffect(() => {
    if (!inTauri) return;
    let alive = true;
    import('@tauri-apps/api/window')
      .then(({ getCurrentWindow }) => {
        const l = getCurrentWindow().label;
        if (alive) setLabel(l === 'overlay' ? 'overlay' : l === 'casescreen' ? 'casescreen' : l.startsWith('ambient-') ? 'ambient' : false);
      })
      .catch(() => alive && setLabel(false));
    return () => {
      alive = false;
    };
  }, []);
  return label;
}

function Root() {
  const route = useRoute();
  const overlayWindow = useOverlayWindow();
  const overlay = route.id === 'overlay' || overlayWindow === 'overlay';
  const ambient = route.id === 'ambient' || overlayWindow === 'ambient';
  useEffect(() => {
    document.documentElement.classList.toggle('overlay-route', overlay);
    document.documentElement.classList.toggle('ambient-route', ambient);
  }, [overlay, ambient]);
  const view = route.id === 'casescreen' || overlayWindow === 'casescreen' ? <CaseScreen /> : ambient ? <AmbientOverlay /> : overlay ? <RegionOverlay /> : overlayWindow === null ? null : <App />;
  return <Suspense fallback={null}>{view}</Suspense>;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
);
