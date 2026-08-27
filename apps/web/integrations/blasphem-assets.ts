import type { AstroIntegration } from "astro";
import { createHash } from "node:crypto";
import { copyFileSync, createReadStream, existsSync, mkdirSync, readFileSync, statSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ENTRY = "index.js";
const WASM = "blasphem_bg.wasm";
const SHIPPED = [ENTRY, "judge.js", "load.js", "blasphem.js", WASM];
const JAVASCRIPT = "text/javascript; charset=utf-8";

function contentType(name: string): string {
  return name === WASM ? "application/wasm" : JAVASCRIPT;
}

export interface BlasphemAssetsOptions {
  distDir: string;
}

interface Located {
  base: string;
  wasmBytes: number;
}

function locate(distDir: string): Located | null {
  const paths = SHIPPED.map((name) => resolve(distDir, name));
  if (paths.some((path) => !existsSync(path))) return null;
  const digest = createHash("sha256");
  for (const path of paths) digest.update(readFileSync(path));
  return {
    base: `/blasphem/${digest.digest("hex").slice(0, 16)}`,
    wasmBytes: statSync(resolve(distDir, WASM)).size,
  };
}

function assetName(url: string | undefined, base: string): string | null {
  if (!url?.startsWith(`${base}/`)) return null;
  const name = url.slice(base.length + 1).split("?")[0];
  return SHIPPED.includes(name) ? name : null;
}

export default function blasphemAssets(options: BlasphemAssetsOptions): AstroIntegration {
  const located = locate(options.distDir);
  return {
    name: "blasphem-assets",
    hooks: {
      "astro:config:setup": ({ updateConfig, logger }) => {
        if (!located) logger.warn("packages/blasphem/dist is incomplete; the playground will report that the package is not built");
        updateConfig({
          vite: {
            define: {
              __BLASPHEM_BASE__: JSON.stringify(located?.base ?? ""),
              __BLASPHEM_WASM_BYTES__: JSON.stringify(located?.wasmBytes ?? 0),
            },
          },
        });
      },
      "astro:server:setup": ({ server }) => {
        if (!located) return;
        server.middlewares.use((request, response, next) => {
          const name = assetName(request.url, located.base);
          if (!name) return next();
          response.setHeader("Content-Type", contentType(name));
          response.setHeader("Cache-Control", "no-store");
          createReadStream(resolve(options.distDir, name)).pipe(response);
        });
      },
      "astro:build:done": ({ dir, logger }) => {
        if (!located) return;
        const target = resolve(fileURLToPath(dir), located.base.slice(1));
        mkdirSync(target, { recursive: true });
        for (const name of SHIPPED) copyFileSync(resolve(options.distDir, name), resolve(target, name));
        logger.info(`copied ${SHIPPED.join(", ")} to ${located.base}/`);
      },
    },
  };
}
