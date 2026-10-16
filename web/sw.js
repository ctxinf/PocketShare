/* PocketShare service worker.
 *
 * Scope is the app root, which may be mounted under a path prefix; the server
 * substitutes the placeholder below before serving this file.
 *
 * Caching policy is deliberately narrow. This is a live sharing tool on a bad
 * network: a stale share list or a cached /get body is worse than no answer at
 * all, so *only* the app shell is cached. Every API path is left entirely to
 * the network -- the fetch handler does not even call respondWith for them, so
 * the browser handles those requests as if no worker existed (which keeps
 * streaming, Range requests and upload progress intact).
 */

const PREFIX = (function () {
  const injected = "__MYSHARE_PATH_PREFIX__";
  return injected.indexOf("MYSHARE_PATH_PREFIX") !== -1 ? "" : injected;
})();

// Bumping this drops every previously cached shell copy on activate.
const CACHE = "pocketshare-shell-v2";

const SHELL_URL = PREFIX + "/";
const PRECACHE = [SHELL_URL, PREFIX + "/manifest.webmanifest", PREFIX + "/icon.svg"];

// Paths whose responses must never be cached or served from cache.
const API_PATHS = ["/put", "/get", "/list", "/delete", "/link", "/dl", "/health"];

function isApiPath(pathname) {
  let rest = pathname;
  if (PREFIX && rest.indexOf(PREFIX) === 0) {
    rest = rest.slice(PREFIX.length);
  }
  return API_PATHS.some(function (p) {
    return rest === p || rest.indexOf(p + "/") === 0;
  });
}

self.addEventListener("install", function (event) {
  event.waitUntil(
    caches.open(CACHE).then(function (cache) {
      // A missing optional icon must not fail the whole install, so each entry
      // is added independently and failures are swallowed.
      return Promise.all(
        PRECACHE.map(function (url) {
          return cache.add(new Request(url, { cache: "reload" })).catch(function () {});
        })
      );
    })
  );
});

self.addEventListener("activate", function (event) {
  event.waitUntil(
    caches
      .keys()
      .then(function (keys) {
        return Promise.all(
          keys.map(function (k) {
            return k === CACHE ? null : caches.delete(k);
          })
        );
      })
      .then(function () {
        return self.clients.claim();
      })
  );
});

// Lets the page trigger an immediate update instead of waiting for every tab
// to close (paired with the "有新版本" prompt in index.html).
self.addEventListener("message", function (event) {
  if (event.data === "skip-waiting") {
    self.skipWaiting();
  }
});

self.addEventListener("fetch", function (event) {
  const req = event.request;
  if (req.method !== "GET") return;

  const url = new URL(req.url);
  // Cross-origin (the UI can be pointed at a different server host) and all
  // API traffic go straight to the network, untouched.
  if (url.origin !== self.location.origin) return;
  if (isApiPath(url.pathname)) return;

  // Navigations: network-first so a redeployed shell is picked up on the next
  // online load, with the cached shell as the offline fallback.
  if (req.mode === "navigate") {
    event.respondWith(
      fetch(req)
        .then(function (resp) {
          const copy = resp.clone();
          caches.open(CACHE).then(function (cache) {
            cache.put(SHELL_URL, copy);
          });
          return resp;
        })
        .catch(function () {
          return caches.match(SHELL_URL).then(function (cached) {
            return (
              cached ||
              new Response("离线，且没有缓存的页面", {
                status: 503,
                headers: { "Content-Type": "text/plain; charset=utf-8" }
              })
            );
          });
        })
    );
    return;
  }

  // Static shell assets (manifest, icons): cache-first, refreshed in the
  // background so an icon change eventually lands without blocking startup.
  event.respondWith(
    caches.match(req).then(function (cached) {
      const network = fetch(req)
        .then(function (resp) {
          if (resp && resp.status === 200 && resp.type === "basic") {
            const copy = resp.clone();
            caches.open(CACHE).then(function (cache) {
              cache.put(req, copy);
            });
          }
          return resp;
        })
        .catch(function () {
          return cached;
        });
      return cached || network;
    })
  );
});
