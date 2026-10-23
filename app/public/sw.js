const CACHE_PREFIX = "xylos-webdav-shell-";
const CACHE = "xylos-webdav-shell-dev";
const PRECACHE = ["/app/", "/app/index.html", "/app/manifest.webmanifest"];

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open(CACHE).then((cache) => cache.addAll(PRECACHE)).then(() => self.skipWaiting())
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(keys.filter((key) => key.startsWith(CACHE_PREFIX) && key !== CACHE)
        .map((key) => caches.delete(key)))
    ).then(() => self.clients.claim())
  );
});

self.addEventListener("fetch", (event) => {
  const request = event.request;
  const url = new URL(request.url);
  if (request.method !== "GET" || url.origin !== self.location.origin ||
      !url.pathname.startsWith("/app/")) return;

  event.respondWith((async () => {
    const cache = await caches.open(CACHE);
    const cached = await cache.match(request, { ignoreSearch: request.mode === "navigate" });
    if (cached) return cached;

    try {
      const response = await fetch(request);
      if (response.ok && PRECACHE.includes(url.pathname)) {
        event.waitUntil(cache.put(request, response.clone()));
      }
      return response;
    } catch (error) {
      if (request.mode === "navigate") {
        const shell = await cache.match("/app/index.html");
        if (shell) return shell;
      }
      throw error;
    }
  })());
});
