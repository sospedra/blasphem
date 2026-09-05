import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { packageRoot, projectRoot } from "./crate.mjs";

export const GLUE_FILES = ["blasphem.js", "blasphem.d.ts", "blasphem_bg.wasm", "blasphem_bg.wasm.d.ts"];
export const PREBUILT_MANIFEST = "blasphem.prebuilt.json";
const sources = resolve(packageRoot, "src");
const manifestPath = resolve(sources, PREBUILT_MANIFEST);
const REBUILD_HINT = "Run: env -u BLASPHEM_WASM_PREBUILT pnpm --filter blasphem build, then commit the four generated binding files and src/blasphem.prebuilt.json.";

// These crates form the WASM runtime's local dependency graph. The language
// tables are embedded even when locale models and packs load at runtime.
const CRATES = ["crates/blasphem-wasm", "crates/blasphem", "crates/blasphem-language"];
const INPUTS = [
  "Cargo.toml",
  "Cargo.lock",
  "rust-toolchain.toml",
  ".cargo",
  ...CRATES.flatMap((crate) => [`${crate}/Cargo.toml`, `${crate}/src`]),
  "crates/blasphem-language/data/eld-tables-v1.bin",
  "packages/javascript/scripts/build.mjs",
  "packages/javascript/scripts/crate.mjs",
  "packages/javascript/scripts/prebuilt.mjs",
  "packages/javascript/scripts/wasm.mjs",
];

function digest(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function filesUnder(path) {
  if (!statSync(resolve(projectRoot, path)).isDirectory()) return [path];
  return readdirSync(resolve(projectRoot, path)).sort().flatMap((name) => filesUnder(`${path}/${name}`));
}

function sourceDigest() {
  const buildScripts = CRATES.map((crate) => `${crate}/build.rs`).filter((path) => existsSync(resolve(projectRoot, path)));
  const files = [...INPUTS, ...buildScripts].flatMap(filesUnder).sort();
  const records = files.map((path) => [path, digest(resolve(projectRoot, path))]);
  return createHash("sha256").update(JSON.stringify(records)).digest("hex");
}

export function writePrebuiltManifest() {
  const artifacts = Object.fromEntries(GLUE_FILES.map((name) => [name, digest(resolve(sources, name))]));
  const manifest = { schema: 1, source_sha256: sourceDigest(), artifacts };
  writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
}

function validatePrebuilt() {
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  if (manifest.schema !== 1) throw new Error("unsupported manifest schema");
  if (manifest.source_sha256 !== sourceDigest()) throw new Error("Rust sources or the WASM build recipe changed");
  for (const name of GLUE_FILES) {
    if (manifest.artifacts?.[name] !== digest(resolve(sources, name))) throw new Error(`${name} does not match its recorded digest`);
  }
}

export function assertPrebuilt() {
  try {
    validatePrebuilt();
  } catch (error) {
    throw new Error(`Prebuilt WASM is missing, stale, or corrupt: ${error.message}. ${REBUILD_HINT}`, { cause: error });
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  assertPrebuilt();
  console.log(`status=verified mode=prebuilt artifacts=${GLUE_FILES.length}`);
}
