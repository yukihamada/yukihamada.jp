// バージョンを上げると activate で旧キャッシュを全削除する。
// HTML(ナビゲーション)は network-first にして、記事追加やテンプレ修正が即反映されるようにする。
// 静的アセットは従来通り cache-first + 裏で更新。
const CACHE = 'yh-v5';
const PRECACHE = ['/about', '/favicon.svg', '/favicon-192.png'];

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

// HTML ナビゲーションか? (ページ本体は常に最新を取りに行く)
function isHtmlNav(req) {
  return req.mode === 'navigate' ||
    (req.destination === 'document') ||
    (req.headers.get('accept') || '').includes('text/html');
}

self.addEventListener('fetch', e => {
  if (e.request.method !== 'GET') return;
  const url = new URL(e.request.url);
  // API / WS / 音声(206 range) / 別ホスト はSWを通さない
  if (url.pathname.startsWith('/api/') || url.pathname.startsWith('/ws/')) return;
  if (url.pathname.startsWith('/audio/')) return;
  if (url.hostname !== self.location.hostname) return;

  // ── HTML: network-first（最新を表示。オフライン時のみキャッシュ）──
  if (isHtmlNav(e.request)) {
    e.respondWith(
      fetch(e.request).then(resp => {
        if (resp.ok && resp.status === 200) {
          const clone = resp.clone();
          caches.open(CACHE).then(c => c.put(e.request, clone));
        }
        return resp;
      }).catch(() => caches.match(e.request).then(c => c || caches.match('/')))
    );
    return;
  }

  // ── 静的アセット: cache-first + 裏で更新 ──
  e.respondWith(
    caches.match(e.request).then(cached => {
      const network = fetch(e.request).then(resp => {
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
