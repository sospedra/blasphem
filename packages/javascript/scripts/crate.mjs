import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export const projectRoot = resolve(packageRoot, "../..");
export const crateManifest = resolve(projectRoot, "crates/blasphem-wasm/Cargo.toml");

function manifestValue(manifest, key) {
  const prefix = `${key} = `;
  const line = manifest.split("\n").find((candidate) => candidate.startsWith(prefix));
  if (!line) throw new Error(`crates/blasphem-wasm/Cargo.toml has no "${key}" entry`);
  return JSON.parse(line.slice(prefix.length).trim());
}

export function readCrate() {
  const manifest = readFileSync(crateManifest, "utf8");
  const name = manifestValue(manifest, "name");
  const requirement = manifestValue(manifest, "wasm-bindgen");
  if (!requirement.startsWith("=")) {
    throw new Error(`wasm-bindgen must pin an exact version, found "${requirement}"`);
  }
  return { name, libName: name.replaceAll("-", "_"), wasmBindgenVersion: requirement.slice(1) };
}
