import { createHash } from "node:crypto";
import { readFile, readdir, writeFile } from "node:fs/promises";

const dist = new URL("./dist/", import.meta.url);
const source = new URL("./public/sw.js", import.meta.url);

async function files(directory, prefix = "") {
  const entries = await readdir(directory, { withFileTypes: true });
  const paths = await Promise.all(entries.map(async (entry) => {
    if (entry.name.startsWith(".")) return [];
    const relative = `${prefix}${entry.name}`;
    if (entry.isDirectory()) return files(new URL(`${entry.name}/`, directory), `${relative}/`);
    return entry.isFile() && relative !== "sw.js" ? [relative] : [];
  }));
  return paths.flat().sort();
}

const assets = await files(dist);
const precache = ["/app/", ...assets.map((path) => `/app/${path}`)];
const hash = createHash("sha256");
for (const path of assets) {
  hash.update(path);
  hash.update(await readFile(new URL(path, dist)));
}
const version = hash.digest("hex").slice(0, 16);
let worker = await readFile(source, "utf8");
if (!worker.includes('const CACHE = "xylos-webdav-shell-dev";') ||
    !worker.includes('const PRECACHE = ["/app/", "/app/index.html", "/app/manifest.webmanifest"];')) {
  throw new Error("Service Worker template markers are missing");
}
worker = worker.replace(
  'const CACHE = "xylos-webdav-shell-dev";',
  `const CACHE = "xylos-webdav-shell-${version}";`,
).replace(
  'const PRECACHE = ["/app/", "/app/index.html", "/app/manifest.webmanifest"];',
  `const PRECACHE = ${JSON.stringify(precache)};`,
);
await writeFile(new URL("sw.js", dist), worker);
