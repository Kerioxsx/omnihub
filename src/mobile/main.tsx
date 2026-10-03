import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { MotionConfig } from 'motion/react';
import './styles.css';
import { App } from './App';
import { applyTheme, watchSystemTheme } from './lib/theme';

applyTheme();
watchSystemTheme();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <MotionConfig reducedMotion="user">
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
