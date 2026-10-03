import { StrictMode, useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import '@shared/theme.css';
import './styles.css';
import { inTauri } from './api';
import { App } from './App';
import { useRoute } from './lib/router';
import { RegionOverlay } from './pages/overlay/RegionOverlay';

/**
 * The region overlay runs in its own borderless window labelled "overlay"
 * (opened at index.html#/overlay). The label check is a fallback in case the
 * hash gets lost; null while it is being resolved.
 */
function useOverlayWindow(): boolean | null {
  const [label, setLabel] = useState<boolean | null>(inTauri ? null : false);
  useEffect(() => {
    if (!inTauri) return;
    let alive = true;
    import('@tauri-apps/api/window')
      .then(({ getCurrentWindow }) => alive && setLabel(getCurrentWindow().label === 'overlay'))
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
  const overlay = route.id === 'overlay' || overlayWindow === true;
  useEffect(() => {
    document.documentElement.classList.toggle('overlay-route', overlay);
  }, [overlay]);
  if (overlay) return <RegionOverlay />;
  if (overlayWindow === null) return null;
  return <App />;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
);
