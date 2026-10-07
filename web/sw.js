// Offline cache: network-first (updates stay immediate, the rev cache-busting
// keeps working), cache fallback when the network is gone. After one full
// load the game plays entirely offline — it has no server to miss.
const CACHE = "souvenir-v1";

self.addEventListener("install", (e) => self.skipWaiting());
self.addEventListener("activate", (e) => e.waitUntil(clients.claim()));

self.addEventListener("fetch", (e) => {
  const req = e.request;
  if (req.method !== "GET" || new URL(req.url).origin !== location.origin) return;
  e.respondWith(
    (async () => {
      const cache = await caches.open(CACHE);
      try {
        const net = await fetch(req);
        if (net.ok) cache.put(req, net.clone());
        return net;
      } catch (err) {
        const hit = await cache.match(req);
        if (hit) return hit;
        if (req.mode === "navigate") {
          const index = await cache.match("./") || await cache.match("index.html");
          if (index) return index;
        }
        throw err;
      }
    })()
  );
});
