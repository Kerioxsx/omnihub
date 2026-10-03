// OmniHub phone app service worker.
// - /assets/* (content-hashed) and static icons: cache-first.
// - The app shell (navigations): served from cache at once, refreshed in
//   the background so a new version shows up on the next launch.
// - /api/* and /dl/*: never cached (network only).
// Only registered in a secure context; everything works without it.

const VERSION = 'omnihub-v1';
const SHELL = ['/', '/manifest.webmanifest', '/icon.svg', '/icon-192.png', '/icon-512.png'];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(VERSION)
      .then((c) => c.addAll(SHELL))
      .catch(() => undefined)
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith('omnihub-') && k !== VERSION).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

async function cacheFirst(request) {
  const cache = await caches.open(VERSION);
  const hit = await cache.match(request);
  if (hit) return hit;
  const res = await fetch(request);
  if (res.ok && res.type === 'basic') cache.put(request, res.clone()).catch(() => undefined);
  return res;
}

async function shell(request) {
  const cache = await caches.open(VERSION);
  const cached = await cache.match('/');
  const network = fetch(request)
    .then((res) => {
      if (res.ok && res.type === 'basic') cache.put('/', res.clone()).catch(() => undefined);
      return res;
    })
    .catch(() => null);
  if (cached) {
    network.catch(() => undefined);
    return cached;
  }
  const res = await network;
  return res || new Response('OmniHub is offline', { status: 503, headers: { 'content-type': 'text/plain' } });
}

self.addEventListener('fetch', (event) => {
  const { request } = event;
  if (request.method !== 'GET') return;
  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;
  if (url.pathname.startsWith('/api/') || url.pathname.startsWith('/dl/')) return; // network only
  if (request.mode === 'navigate') {
    event.respondWith(shell(request));
    return;
  }
  if (url.pathname.startsWith('/assets/') || SHELL.includes(url.pathname) || url.pathname.startsWith('/icon')) {
    event.respondWith(cacheFirst(request));
  }
});
