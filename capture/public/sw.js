// Mantém o app abrindo mesmo sem rede. A API nunca é guardada em cache.
const CACHE = "lume-v2";
const SHELL = ["/", "/manifest.webmanifest", "/icon-192.png", "/icon-512.png"];

/** Guarda a página e todos os arquivos que ela carrega (/assets/...), já na instalação. */
async function precache() {
  const c = await caches.open(CACHE);
  const res = await fetch("/", { cache: "no-store" });
  if (!res.ok) return;
  const html = await res.clone().text();
  const assets = [...html.matchAll(/(?:src|href)="(\/assets\/[^"]+)"/g)].map((m) => m[1]);
  await c.put("/", res);
  await c.addAll([...SHELL.slice(1), ...assets]);
}

self.addEventListener("install", (e) => {
  e.waitUntil(precache());
  self.skipWaiting();
});
self.addEventListener("activate", (e) => {
  e.waitUntil(caches.keys().then((ks) => Promise.all(ks.filter((k) => k !== CACHE).map((k) => caches.delete(k)))));
  self.clients.claim();
});
self.addEventListener("fetch", (e) => {
  const url = new URL(e.request.url);
  if (e.request.method !== "GET" || url.origin !== location.origin || url.pathname.startsWith("/api/")) return;
  const isPage = e.request.mode === "navigate";
  e.respondWith(
    fetch(e.request)
      .then((res) => {
        // só guarda respostas boas: um erro momentâneo do servidor não substitui a cópia boa
        if (res.ok) {
          const copy = res.clone();
          caches.open(CACHE).then((c) => c.put(isPage ? "/" : e.request, copy));
        }
        return res;
      })
      .catch(async () => (await caches.match(isPage ? "/" : e.request)) ?? Response.error()),
  );
});
