import type { AstroIntegration } from "astro";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, extname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { browserAssets } from "../../../packages/javascript/integrations/assets.mjs";

const CODE = ["browser.js", "browser-cache.js", "browser-config.js", "wasm-engine.js", "blasphem.js", "version.generated.js"];

interface Located {
  base: string;
  files: Map<string, Uint8Array>;
  totalBytes: number;
}

function contentType(name: string): string {
  switch (extname(name)) {
    case ".wasm": return "application/wasm";
    case ".js": return "text/javascript; charset=utf-8";
    case ".json": return "application/json";
    default: return "application/octet-stream";
  }
}

function locate(dist: string, project: { root: string; base: string }): Located | null {
  if (!existsSync(resolve(dist, "browser.js"))) return null;
  const core = readdirSync(resolve(dist, "core")).filter((name) => name.endsWith(".js")).map((name) => `core/${name}`);
  const code = [...CODE, ...core].map((name): [string, Uint8Array] => [name, readFileSync(resolve(dist, name))]);
  const initial = browserAssets(project.root, `${project.base}blasphem`);
  const digest = createHash("sha256");
  for (const [, bytes] of [...code, ...initial.entries]) digest.update(bytes);
  const base = `${project.base}blasphem/${digest.digest("hex").slice(0, 16)}`;
  const { bundle, entries } = browserAssets(project.root, base);
  const generated: [string, Uint8Array] = ["bundle.generated.js", Buffer.from(`export const BUNDLE = ${JSON.stringify(bundle)};\n`)];
  const files = new Map([...code, ...entries, generated]);
  const dataFiles = entries.filter(([name]) => [".wasm", ".pack", ".detect"].includes(extname(name)));
  const totalBytes = dataFiles.reduce((sum, [, bytes]) => sum + bytes.byteLength, 0);
  return { base, files, totalBytes };
}

function assetName(url: string | undefined, located: Located): string | null {
  if (!url?.startsWith(`${located.base}/`)) return null;
  const name = url.slice(located.base.length + 1).split("?")[0];
  return located.files.has(name) ? name : null;
}

export default function blasphemAssets(options: { distDir: string }): AstroIntegration {
  let located: Located | null = null;
  return {
    name: "blasphem-assets",
    hooks: {
      "astro:config:setup": ({ config, updateConfig, logger }) => {
        const base = config.base.endsWith("/") ? config.base : `${config.base}/`;
        located = locate(options.distDir, { root: fileURLToPath(config.root), base });
        if (!located) logger.warn("Build the blasphem package before the playground.");
        updateConfig({ vite: { define: {
          __BLASPHEM_BASE__: JSON.stringify(located?.base ?? ""),
          __BLASPHEM_TOTAL_BYTES__: JSON.stringify(located?.totalBytes ?? 0),
        } } });
      },
      "astro:server:setup": ({ server }) => {
        server.middlewares.use((request, response, next) => {
          if (!located) return next();
          const name = assetName(request.url, located);
          if (!name) return next();
          response.setHeader("Content-Type", contentType(name));
          response.setHeader("Cache-Control", "no-store");
          response.end(located.files.get(name));
        });
      },
      "astro:build:done": ({ dir, logger }) => {
        if (!located) return;
        const target = resolve(fileURLToPath(dir), located.base.slice(1));
        for (const [name, bytes] of located.files) {
          const destination = resolve(target, name);
          mkdirSync(dirname(destination), { recursive: true });
          writeFileSync(destination, bytes);
        }
        logger.info(`copied ${located.files.size} selected files to ${located.base}/`);
      },
    },
  };
}
