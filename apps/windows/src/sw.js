self.addEventListener("install", (event) => {
  event.waitUntil(
    caches.open("escudo-launch-prototype-v1").then((cache) =>
      cache.addAll([
        "./index.html",
        "./css/style.css",
        "./js/app.js",
        "./manifest.webmanifest"
      ])
    )
  );
  self.skipWaiting();
});

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
});

self.addEventListener("fetch", (event) => {
  event.respondWith(
    caches.match(event.request).then((cached) => cached || fetch(event.request))
  );
});
