import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { fileURLToPath } from 'node:url';

// Phone web app, served by the companion server (embedded from dist-mobile).
// For development run the headless server (`cargo run -p omnihub-core --bin
// omnihub-headless -- --http --pair`) and `npm run dev:mobile`; API calls are
// proxied to it.
const target = process.env.OMNIHUB_SERVER ?? 'http://127.0.0.1:47800';

export default defineConfig({
  root: fileURLToPath(new URL('./mobile', import.meta.url)),
  publicDir: fileURLToPath(new URL('./mobile/public', import.meta.url)),
  plugins: [react(), tailwindcss()],
  resolve: { alias: { '@shared': fileURLToPath(new URL('./src/shared', import.meta.url)) } },
  server: {
    port: 1421,
    host: true,
    proxy: {
      '/api': { target, ws: true, changeOrigin: false, secure: false },
      '/dl': { target, changeOrigin: false, secure: false },
    },
  },
  build: {
    outDir: fileURLToPath(new URL('./dist-mobile', import.meta.url)),
    emptyOutDir: true,
    target: 'es2020',
    chunkSizeWarningLimit: 1500,
  },
});
