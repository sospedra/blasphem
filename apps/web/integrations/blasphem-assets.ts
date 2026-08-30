import type { AstroIntegration } from "astro";
import { createHash } from "node:crypto";
import { copyFileSync, createReadStream, existsSync, mkdirSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, extname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const WASM = "blasphem_bg.wasm";
const CODE = ["browser.js", "wasm-engine.js", "blasphem.js", WASM];
const MANIFEST = "manifest.json";

function contentType(name: string): string {
  switch (extname(name)) {
    case ".wasm": return "application/wasm";
    case ".js": return "text/javascript; charset=utf-8";
    case ".json": return "application/json";
    default: return "application/octet-stream";
  }
}

export interface BlasphemAssetsOptions {
  /** packages/blasphem/dist */
  distDir: string;
  /** packages/packs/dist */
  packsDir: string;
}

interface Located {
  base: string;
  /** Served name, such as `core/loader.js` or `en.pack`, to its source path. */
  files: Map<string, string>;
  /** Bytes a judge over every locale with detection downloads: wasm plus every pack file. */
  totalBytes: number;
}

function locate(distDir: string, packsDir: string): Located | null {
  const files = new Map<string, string>();
  for (const name of CODE) files.set(name, resolve(distDir, name));
  const core = resolve(distDir, "core");
  if (existsSync(core)) {
    for (const name of readdirSync(core).filter((entry) => entry.endsWith(".js"))) files.set(`core/${name}`, resolve(core, name));
  }
  const manifestPath = resolve(packsDir, MANIFEST);
  if ([...files.values(), manifestPath].some((path) => !existsSync(path))) return null;
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as { files: Record<string, { bytes: number }> };
  files.set(MANIFEST, manifestPath);
  let totalBytes = statSync(resolve(distDir, WASM)).size;
  for (const [name, record] of Object.entries(manifest.files)) {
    files.set(name, resolve(packsDir, name));
    totalBytes += record.bytes;
  }
  if ([...files.values()].some((path) => !existsSync(path))) return null;
  const digest = createHash("sha256");
  for (const name of [...files.keys()].sort()) digest.update(readFileSync(files.get(name) as string));
  return { base: `/blasphem/${digest.digest("hex").slice(0, 16)}`, files, totalBytes };
}

function assetName(url: string | undefined, located: Located): string | null {
  if (!url?.startsWith(`${located.base}/`)) return null;
  const name = url.slice(located.base.length + 1).split("?")[0];
  return located.files.has(name) ? name : null;
}

/** Serves the package, the wasm, and the packs under one hashed base in dev, and copies them there at build. */
export default function blasphemAssets(options: BlasphemAssetsOptions): AstroIntegration {
  const located = locate(options.distDir, options.packsDir);
  return {
    name: "blasphem-assets",
    hooks: {
      "astro:config:setup": ({ updateConfig, logger }) => {
        if (!located) logger.warn("packages/blasphem/dist or packages/packs/dist is incomplete; the playground will report that the package is not built");
        updateConfig({
          vite: {
            define: {
              __BLASPHEM_BASE__: JSON.stringify(located?.base ?? ""),
              __BLASPHEM_TOTAL_BYTES__: JSON.stringify(located?.totalBytes ?? 0),
            },
          },
        });
      },
      "astro:server:setup": ({ server }) => {
        if (!located) return;
        server.middlewares.use((request, response, next) => {
          const name = assetName(request.url, located);
          if (!name) return next();
          response.setHeader("Content-Type", contentType(name));
          response.setHeader("Cache-Control", "no-store");
          createReadStream(located.files.get(name) as string).pipe(response);
        });
      },
      "astro:build:done": ({ dir, logger }) => {
        if (!located) return;
        const target = resolve(fileURLToPath(dir), located.base.slice(1));
        for (const [name, source] of located.files) {
          const destination = resolve(target, name);
          mkdirSync(dirname(destination), { recursive: true });
          copyFileSync(source, destination);
        }
        logger.info(`copied ${located.files.size} files, ${(located.totalBytes / 1048576).toFixed(2)} MB of wasm and packs, to ${located.base}/`);
      },
    },
  };
}
