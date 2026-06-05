const C='takibi-v3';
self.addEventListener('install',e=>{self.skipWaiting();e.waitUntil(caches.open(C).then(c=>c.addAll(['/takibi/','/takibi/index.html','/takibi/manifest.webmanifest'])))});
self.addEventListener('activate',e=>e.waitUntil(self.clients.claim()));
self.addEventListener('fetch',e=>{
  const u=new URL(e.request.url);
  if(u.pathname.startsWith('/takibi/')){ e.respondWith(caches.match(e.request).then(r=>r||fetch(e.request))); }
});
