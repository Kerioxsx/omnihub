import { StrictMode, useEffect } from 'react';
import { createRoot } from 'react-dom/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import '@shared/theme.css';
import './styles.css';
import { inTauri } from './api';
import { App } from './App';
import { useRoute } from './lib/router';
import { RegionOverlay } from './pages/overlay/RegionOverlay';

/** The region overlay runs in its own borderless window labelled "overlay". */
function isOverlayWindow(): boolean {
  if (!inTauri) return false;
  try {
    return getCurrentWindow().label === 'overlay';
  } catch {
    return false;
  }
}

const overlayWindow = isOverlayWindow();

function Root() {
  const route = useRoute();
  const overlay = overlayWindow || route.id === 'overlay';
  useEffect(() => {
    document.documentElement.classList.toggle('overlay-route', overlay);
  }, [overlay]);
  return overlay ? <RegionOverlay /> : <App />;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
);
