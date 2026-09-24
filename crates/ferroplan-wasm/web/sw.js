// ferroplan demo — the OFFLINE copy.
//
// Registered only when the visitor asks ("Save offline" in the header); until
// then this file is inert and the site behaves exactly as before. Once
// registered it precaches the whole demo -- the three pages, the example
// corpus and the wasm bundle -- and answers every same-origin request from
// that copy first, so the planner keeps working with no network at all.
//
// Freshness: a request that is answered from the cache is also re-fetched in
// the background and the copy replaced (stale-while-revalidate), and a new
// deploy ships a new BUILD id, which installs alongside the old copy and
// replaces it whole on activation. Nothing here is ever a partial site.
//
// Cross-origin requests (the Google Fonts stylesheet and its files) pass
// straight to the network: their responses are opaque, cannot be inspected,
// and the pages read fine in the system fallback fonts without them.

// Replaced by the Pages workflow with the deploying commit; 'dev' when served
// from a checkout.
const BUILD = '__BUILD_ID__';
const CACHE = 'ferroplan-demo-' + BUILD;

// Everything the three pages need. Relative to this file's directory, which
// is the registration scope (/demo/ on the site, / from a checkout).
const SHELL = [
  './',
  './index.html',
  './examples.js',
  './worker.js',
  './village-live.html',
  './village-data.js',
  './bazaar-live.html',
  './manifest.webmanifest',
  './icon.svg',
  './pkg/ferroplan_wasm.js',
  './pkg/ferroplan_wasm_bg.wasm',
];

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE).then((cache) => cache.addAll(SHELL)).then(() => self.skipWaiting())
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys()
      .then((keys) => Promise.all(
        keys.filter((k) => k.startsWith('ferroplan-demo-') && k !== CACHE).map((k) => caches.delete(k))
      ))
      .then(() => self.clients.claim())
  );
});

self.addEventListener('fetch', (event) => {
  const req = event.request;
  if (req.method !== 'GET') return;
  const url = new URL(req.url);
  if (url.origin !== self.location.origin) return;
  event.respondWith(
    caches.open(CACHE).then(async (cache) => {
      // `ignoreSearch`: the pages link to themselves with hashes and the odd
      // cache-busting query; one copy serves all of them.
      const hit = await cache.match(req, { ignoreSearch: true });
      const refresh = fetch(req).then((res) => {
        if (res && res.ok && res.type === 'basic') cache.put(req, res.clone());
        return res;
      }).catch(() => undefined);
      if (hit) {
        event.waitUntil(refresh);
        return hit;
      }
      const res = await refresh;
      if (res) return res;
      // Offline and never cached: a navigation falls back to the front page
      // rather than the browser's dinosaur; anything else is honestly absent.
      if (req.mode === 'navigate') {
        const home = await cache.match('./index.html');
        if (home) return home;
      }
      return new Response('offline, and this file is not in the saved copy', {
        status: 503, headers: { 'Content-Type': 'text/plain' },
      });
    })
  );
});

// The page asks; the worker answers with what it holds. `size` is the sum of
// the cached bodies, read once -- fine for a dozen files.
self.addEventListener('message', async (event) => {
  if (!event.data || event.data.type !== 'status') return;
  const cache = await caches.open(CACHE);
  const reqs = await cache.keys();
  let size = 0;
  for (const r of reqs) {
    const res = await cache.match(r);
    if (res) size += (await res.clone().arrayBuffer()).byteLength;
  }
  const port = event.ports && event.ports[0];
  if (port) port.postMessage({ build: BUILD, files: reqs.length, size });
});
