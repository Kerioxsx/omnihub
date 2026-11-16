import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { MotionConfig } from 'motion/react';
import './styles.css';
import { App } from './App';
import { applyLook, refreshLook, watchLook } from './lib/look';
import { WallLayer } from './ui/WallLayer';

// The saved look first (no flash), then the PC's latest.
applyLook();
watchLook();
void refreshLook();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <MotionConfig reducedMotion="user">
      <WallLayer />
      <App />
    </MotionConfig>
  </StrictMode>,
);

// Offline shell cache. Only in a secure context (HTTPS with a trusted cert;
// with the PC's self-signed certificate registration fails, which is fine).
if (window.isSecureContext && 'serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    try {
      navigator.serviceWorker.register('/sw.js').catch(() => undefined);
    } catch {
      /* unsupported */
    }
  });
}
