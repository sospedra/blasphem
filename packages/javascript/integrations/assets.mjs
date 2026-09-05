import { createHash } from "node:crypto";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { resolveConfiguration, parseManifest, jsdelivrBases } from "../dist/core/index.js";
import { VERSION, WASM_INTEGRITY, MANIFEST_INTEGRITY } from "../dist/version.generated.js";

export const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);

export function installedPacks() {
  const path = require.resolve("@blasphem/packs/package.json");
  const metadata = JSON.parse(readFileSync(path, "utf8"));
  if (metadata.version !== VERSION) throw new Error(`Internal data version must equal ${VERSION}`);
  return resolve(dirname(path), "dist");
}

export function readVerified(root, name, expected) {
  if (!expected) throw new Error(`The installed data does not include ${name}`);
  const bytes = readFileSync(resolve(root, name));
  const digest = createHash("sha256").update(bytes).digest("hex");
  if (bytes.length !== expected.bytes || digest !== expected.sha256) throw new Error(`Integrity mismatch for ${name}`);
  return bytes;
}

export function selectedAssets(config) {
  const root = installedPacks();
  const manifest = parseManifest(readFileSync(resolve(root, "manifest.json")));
  const files = Object.fromEntries(config.files.map((name) => {
    const expected = manifest.files[name];
    if (!expected) throw new Error(`The installed data does not include ${name}`);
    return [name, expected];
  }));
  return { root, manifest: { formatVersion: 1, files } };
}

export function browserAssets(projectRoot, publicBase) {
  const input = JSON.parse(readFileSync(resolve(projectRoot, "package.json"), "utf8")).blasphem;
  const config = resolveConfiguration(input, VERSION);
  const { root, manifest } = selectedAssets(config);
  const base = publicBase.endsWith("/") ? publicBase.slice(0, -1) : publicBase;
  const remote = jsdelivrBases(VERSION);
  const bases = config.assets === "remote" ? remote : { wasm: base, packs: base };
  const names = ["manifest.json", ...config.files];
  const assetUrls = Object.fromEntries(names.map((name) => [name, `${bases.packs}/${name}`]));
  const wasm = { ...WASM_INTEGRITY, url: `${bases.wasm}/blasphem_bg.wasm` };
  const bundle = { ...config, assetUrls: { ...assetUrls, "blasphem_bg.wasm": wasm.url }, wasm, manifest: MANIFEST_INTEGRITY };
  const entries = config.assets === "bundled" ? bundledAssets({ root, manifest, config }) : [];
  const encoded = JSON.stringify(bundle).replaceAll("<", "\\u003c");
  return { bundle, entries: [
    ...entries,
    ["bundle.json", Buffer.from(`${encoded}\n`)],
    ["config.js", Buffer.from(`globalThis.__BLASPHEM_CONFIG__ = ${encoded};\n`)],
    ["NOTICE", readFileSync(resolve(packageRoot, "NOTICE"))],
  ] };
}

function bundledAssets({ root, manifest, config }) {
  return [
    ...config.files.map((name) => [name, readVerified(root, name, manifest.files[name])]),
    ["manifest.json", Buffer.from(`${JSON.stringify(manifest)}\n`)],
    ["blasphem_bg.wasm", readVerified(resolve(packageRoot, "dist"), "blasphem_bg.wasm", WASM_INTEGRITY)],
  ];
}
