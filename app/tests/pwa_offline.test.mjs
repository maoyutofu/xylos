import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";
import { test } from "node:test";
import { runInNewContext } from "node:vm";

const dist = new URL("../dist/", import.meta.url);

async function assetPaths(directory, prefix = "") {
  const entries = await readdir(directory, { withFileTypes: true });
  const paths = await Promise.all(entries.map((entry) => {
    if (entry.name.startsWith(".") || entry.name === "sw.js") return [];
    const path = `${prefix}${entry.name}`;
    return entry.isDirectory()
      ? assetPaths(new URL(`${entry.name}/`, directory), `${path}/`)
      : [`/app/${path}`];
  }));
  return paths.flat();
}

test("built PWA preloads every asset and serves it offline", async () => {
  const source = await readFile(new URL("sw.js", dist), "utf8");
  const precache = JSON.parse(source.match(/^const PRECACHE = (\[.*\]);$/m)?.[1] ?? "null");
  assert.ok(precache);
  assert.deepEqual(new Set(precache), new Set(["/app/", ...await assetPaths(dist)]));

  const listeners = new Map();
  const stores = new Map([["unrelated-cache", new Map()], ["xylos-webdav-shell-old", new Map()]]);
  let online = true;
  let claimed = false;
  let skipped = false;
  const response = (path) => ({ path, clone() { return response(path); } });
  const key = (request, ignoreSearch = false) => {
    const url = new URL(typeof request === "string" ? request : request.url, "https://example.test");
    return url.pathname + (ignoreSearch ? "" : url.search);
  };
  const caches = {
    async open(name) {
      if (!stores.has(name)) stores.set(name, new Map());
      const store = stores.get(name);
      return {
        async addAll(paths) {
          for (const path of paths) {
            if (!online) throw new Error("offline");
            store.set(path, response(path));
          }
        },
        async match(request, options) { return store.get(key(request, options?.ignoreSearch)); },
        async put(request, value) { store.set(key(request), value); },
      };
    },
    async keys() { return [...stores.keys()]; },
    async delete(name) { return stores.delete(name); },
  };
  runInNewContext(source, {
    URL, caches,
    self: {
      location: { origin: "https://example.test" },
      addEventListener(type, listener) { listeners.set(type, listener); },
      async skipWaiting() { skipped = true; },
      clients: { async claim() { claimed = true; } },
    },
    async fetch(request) {
      if (!online) throw new Error("offline");
      return response(key(request));
    },
  });

  const lifecycle = async (type) => {
    let pending;
    listeners.get(type)({ waitUntil(promise) { pending = promise; } });
    await pending;
  };
  await lifecycle("install");
  await lifecycle("activate");
  assert.ok(skipped && claimed);
  assert.ok(stores.has("unrelated-cache"));
  assert.ok(!stores.has("xylos-webdav-shell-old"));

  online = false;
  const fetchOffline = async (path, mode = "same-origin") => {
    let pending;
    listeners.get("fetch")({
      request: { url: `https://example.test${path}`, method: "GET", mode },
      respondWith(promise) { pending = promise; },
      waitUntil() {},
    });
    return pending;
  };
  for (const path of precache) assert.equal((await fetchOffline(path)).path, path);
  assert.equal((await fetchOffline("/app/?source=installed", "navigate")).path, "/app/");
  assert.equal((await fetchOffline("/app/deep/link", "navigate")).path, "/app/index.html");
  await assert.rejects(fetchOffline("/app/assets/missing.js"), /offline/);
});
