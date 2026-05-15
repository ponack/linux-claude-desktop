const CACHE = "lcd-companion-v1";

// Scope-relative precache entries (works for both / and /linux-claude-desktop/)
self.addEventListener("install", (e) => {
  const scope = new URL(self.registration.scope).pathname;
  e.waitUntil(
    caches.open(CACHE).then((c) =>
      c.addAll([scope, scope + "index.html"]).catch(() => {})
    )
  );
  self.skipWaiting();
});

self.addEventListener("activate", (e) => {
  e.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)))
    )
  );
  self.clients.claim();
});

self.addEventListener("fetch", (e) => {
  const { request } = e;
  if (request.method !== "GET") return;

  const url = new URL(request.url);
  const isSameOrigin = url.origin === self.location.origin;
  const isApiRead = url.pathname.startsWith("/api/");

  if (isSameOrigin) {
    // App shell: stale-while-revalidate
    e.respondWith(
      caches.open(CACHE).then(async (cache) => {
        const cached = await cache.match(request);
        const network = fetch(request)
          .then((resp) => {
            if (resp.ok) cache.put(request, resp.clone());
            return resp;
          })
          .catch(() => cached);
        return cached || network;
      })
    );
  } else if (isApiRead) {
    // Cross-origin API reads: network-first, cached fallback
    e.respondWith(
      fetch(request)
        .then((resp) => {
          if (resp.ok) {
            const clone = resp.clone();
            caches.open(CACHE).then((c) => c.put(request, clone));
          }
          return resp;
        })
        .catch(() => caches.match(request))
    );
  }
  // Cross-origin non-API (e.g. POST): don't intercept
});

// Background Sync: tell all open tabs to flush the message queue
self.addEventListener("sync", (e) => {
  if (e.tag === "flush-queue") {
    e.waitUntil(
      self.clients
        .matchAll({ includeUncontrolled: true, type: "window" })
        .then((clients) =>
          clients.forEach((c) => c.postMessage({ type: "flush-queue" }))
        )
    );
  }
});
