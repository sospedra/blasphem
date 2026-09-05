#!/usr/bin/env node
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { parseArgs } from "node:util";
import { packageRoot, selectedAssets, readVerified } from "../integrations/assets.mjs";
import { resolveConfiguration } from "../dist/core/index.js";
import { nativeTarget } from "../dist/native.js";
import { VERSION } from "../dist/version.generated.js";

function argumentsConfig() {
  const { values } = parseArgs({ options: {
    locales: { type: "string" }, output: { type: "string" }, "no-detect": { type: "boolean", default: false },
  } });
  if (!values.locales || !values.output) throw new Error("usage: blasphem-export --locales en,es --output ./vendor [--no-detect]");
  const locales = values.locales === "all" ? "all" : values.locales.split(",");
  return { output: resolve(values.output), config: resolveConfiguration({ locales, detectLanguage: !values["no-detect"] }, VERSION) };
}

function exportPacks(staging, config) {
  const { root, manifest } = selectedAssets(config);
  const target = resolve(staging, "node_modules/@blasphem/packs");
  const dist = resolve(target, "dist");
  mkdirSync(dist, { recursive: true });
  for (const name of config.files) writeFileSync(resolve(dist, name), readVerified(root, name, manifest.files[name]));
  writeFileSync(resolve(dist, "manifest.json"), `${JSON.stringify(manifest)}\n`);
  const names = ["manifest.json", ...config.files];
  const files = names.map((name) => `${JSON.stringify(name)}: new URL(${JSON.stringify(`./${name}`)}, import.meta.url)`);
  writeFileSync(resolve(dist, "files.js"), `export const FILES = {${files.join(",")}};\nexport const MANIFEST = FILES["manifest.json"];\n`);
  writeFileSync(resolve(dist, "files.d.ts"), "export declare const FILES: Readonly<Record<string, URL>>;\nexport declare const MANIFEST: URL;\n");
  const metadata = JSON.parse(readFileSync(resolve(dirname(root), "package.json"), "utf8"));
  delete metadata.scripts;
  delete metadata.devDependencies;
  writeFileSync(resolve(target, "package.json"), `${JSON.stringify(metadata, null, 2)}\n`);
  cpSync(resolve(packageRoot, "NOTICE"), resolve(target, "NOTICE"));
}

function exportNative(staging) {
  const target = nativeTarget();
  if (!target) return;
  const name = `@blasphem/node-${target}`;
  const require = createRequire(import.meta.url);
  const entry = (() => {
    try { return require.resolve(name); }
    catch { return null; }
  })();
  if (!entry) return;
  cpSync(dirname(entry), resolve(staging, "node_modules", name), { recursive: true });
}

function exportRuntime(staging, config) {
  const target = resolve(staging, "node_modules/blasphem");
  mkdirSync(target, { recursive: true });
  cpSync(resolve(packageRoot, "dist"), resolve(target, "dist"), { recursive: true });
  for (const name of ["LICENSE", "NOTICE", "README.md"]) cpSync(resolve(packageRoot, name), resolve(target, name));
  const original = JSON.parse(readFileSync(resolve(packageRoot, "package.json"), "utf8"));
  const metadata = {
    ...original, bin: undefined, scripts: undefined, devDependencies: undefined, optionalDependencies: undefined,
    main: "./dist/node.js", module: undefined, types: "./dist/node.d.ts",
    exports: { ".": { types: "./dist/node.d.ts", default: "./dist/node.js" }, "./package.json": "./package.json" },
    dependencies: { "@blasphem/packs": VERSION }, files: ["dist", "README.md", "LICENSE", "NOTICE"],
  };
  writeFileSync(resolve(target, "package.json"), `${JSON.stringify(metadata, null, 2)}\n`);
  writeFileSync(resolve(staging, "package.json"), `${JSON.stringify({ private: true, type: "module", blasphem: config }, null, 2)}\n`);
}

const { output, config } = argumentsConfig();
if (existsSync(output)) throw new Error(`Output already exists: ${output}`);
mkdirSync(dirname(output), { recursive: true });
const staging = mkdtempSync(resolve(dirname(output), ".blasphem-export-"));
try {
  exportPacks(staging, config);
  exportRuntime(staging, config);
  exportNative(staging);
  renameSync(staging, output);
} finally {
  rmSync(staging, { recursive: true, force: true });
}
console.log(`status=exported locales=${config.locales.join(",")} detect=${config.detectLanguage} to=${output}`);
