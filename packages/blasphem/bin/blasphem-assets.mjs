#!/usr/bin/env node
// Copies blasphem_bg.wasm and the installed @blasphem/packs into one directory
// for self-hosting. Usage: blasphem-assets <directory>, for example
// `blasphem-assets public/blasphem` in a prebuild script.
import { copyFileSync, mkdirSync, readdirSync, statSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const target = process.argv[2];
if (!target) {
  console.error("usage: blasphem-assets <directory>");
  process.exit(2);
}

// Resolve from the caller's project, where both packages are installed.
const require = createRequire(resolve(process.cwd(), "package.json"));
const wasm = require.resolve("blasphem/blasphem_bg.wasm");
let packs;
try {
  packs = dirname(require.resolve("@blasphem/packs/manifest.json"));
} catch {
  console.error("@blasphem/packs is not installed. Run: pnpm add @blasphem/packs");
  process.exit(1);
}

const destination = resolve(process.cwd(), target);
mkdirSync(destination, { recursive: true });
let bytes = 0;
const names = readdirSync(packs).filter((name) => name === "manifest.json" || name.endsWith(".pack") || name.endsWith(".detect"));
for (const name of names) {
  copyFileSync(resolve(packs, name), resolve(destination, name));
  bytes += statSync(resolve(packs, name)).size;
}
copyFileSync(wasm, resolve(destination, "blasphem_bg.wasm"));
bytes += statSync(wasm).size;
console.log(`status=copied files=${names.length + 1} mb=${(bytes / 1048576).toFixed(2)} to=${target}`);
export {};
