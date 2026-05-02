const CACHE = 'yh-v3';
const PRECACHE = ['/', '/blog', '/about', '/favicon.svg', '/favicon-192.png'];

self.addEventListener('install', e => {
  e.waitUntil(caches.open(CACHE).then(c => c.addAll(PRECACHE)).catch(() => {}));
  self.skipWaiting();
});

self.addEventListener('activate', e => {
  e.waitUntil(
    caches.keys().then(keys =>
      Promise.all(keys.filter(k => k !== CACHE).map(k => caches.delete(k)))
    )
  );
  self.clients.claim();
});

self.addEventListener('fetch', e => {
  if (e.request.method !== 'GET') return;
  const url = new URL(e.request.url);
  // Skip API, WebSocket, analytics, audio files (range requests return 206)
  if (url.pathname.startsWith('/api/') || url.pathname.startsWith('/ws/')) return;
  if (url.pathname.startsWith('/audio/')) return;
  if (url.hostname !== self.location.hostname) return;

  e.respondWith(
    caches.match(e.request).then(cached => {
      const network = fetch(e.request).then(resp => {
        // Only cache complete responses (not 206 partial, not redirects)
        if (resp.ok && resp.status === 200) {
          const clone = resp.clone();
          caches.open(CACHE).then(c => c.put(e.request, clone));
        }
        return resp;
      });
      return cached || network;
    })
  );
});
