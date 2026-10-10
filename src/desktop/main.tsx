import { StrictMode, useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import '@shared/theme.css';
import './styles.css';
import { inTauri } from './api';
import { App } from './App';
import { useRoute } from './lib/router';
import { AmbientOverlay } from './pages/overlay/AmbientOverlay';
import { RegionOverlay } from './pages/overlay/RegionOverlay';

/**
 * The region overlay runs in its own borderless window labelled "overlay"
 * (opened at index.html#/overlay), the screen glow in windows labelled
 * "ambient-…" (#/ambient). The label check is a fallback in case the hash
 * gets lost; null while it is being resolved.
 */
function useOverlayWindow(): 'overlay' | 'ambient' | false | null {
  const [label, setLabel] = useState<'overlay' | 'ambient' | false | null>(inTauri ? null : false);
  useEffect(() => {
    if (!inTauri) return;
    let alive = true;
    import('@tauri-apps/api/window')
      .then(({ getCurrentWindow }) => {
        const l = getCurrentWindow().label;
        if (alive) setLabel(l === 'overlay' ? 'overlay' : l.startsWith('ambient-') ? 'ambient' : false);
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
  if (ambient) return <AmbientOverlay />;
  if (overlay) return <RegionOverlay />;
  if (overlayWindow === null) return null;
  return <App />;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
);
