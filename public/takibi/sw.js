const C='takibi-v5';
self.addEventListener('install',e=>{self.skipWaiting();e.waitUntil(caches.open(C).then(c=>c.addAll(['/takibi/','/takibi/index.html','/takibi/manifest.webmanifest'])))});
self.addEventListener('activate',e=>e.waitUntil((async()=>{
  // 旧バージョンのキャッシュを消す（caches.match が古い index.html を返す事故を防ぐ）
  const keys=await caches.keys();
  await Promise.all(keys.filter(k=>k!==C).map(k=>caches.delete(k)));
  await self.clients.claim();
})()));
self.addEventListener('fetch',e=>{
  const u=new URL(e.request.url);
  if(u.pathname.startsWith('/api/'))return;            // API は常にネットワーク（feed/react はキャッシュしない）
  if(u.pathname.startsWith('/takibi/')){ e.respondWith(caches.match(e.request,{cacheName:C}).then(r=>r||fetch(e.request))); }
});
