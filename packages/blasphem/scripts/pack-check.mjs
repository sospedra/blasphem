import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { packageRoot } from "./crate.mjs";

const REQUIRED_FILES = [
  "LICENSE",
  "NOTICE",
  "README.md",
  "dist/blasphem.d.ts",
  "dist/blasphem.js",
  "dist/blasphem_bg.wasm",
  "dist/blasphem_bg.wasm.d.ts",
  "dist/index.d.ts",
  "dist/index.js",
  "dist/judge.d.ts",
  "dist/judge.js",
  "dist/load.d.ts",
  "dist/load.js",
  "package.json",
];
const FORBIDDEN_PREFIXES = ["crates/", "data/", "reports/", "resources/", "src/", "target/"];

function readManifest() {
  return JSON.parse(readFileSync(resolve(packageRoot, "package.json"), "utf8"));
}

function assertManifest(manifest) {
  if (manifest.name !== "blasphem") throw new Error(`the package name must be "blasphem", found "${manifest.name}"`);
  if (manifest.private !== true) throw new Error("the package must stay private");
  const entry = manifest.exports?.["."];
  if (entry?.types !== "./dist/index.d.ts") throw new Error('exports["."].types must be ./dist/index.d.ts');
  if (entry?.default !== "./dist/index.js") throw new Error('exports["."].default must be ./dist/index.js');
  if (entry?.node !== "./dist/index.js") throw new Error('exports["."].node must be ./dist/index.js');
  if (entry?.browser !== "./dist/index.js") throw new Error('exports["."].browser must be ./dist/index.js');
}

function assertDistribution() {
  const missing = ["dist/blasphem.js", "dist/blasphem_bg.wasm", "dist/index.js"].filter((file) => !existsSync(resolve(packageRoot, file)));
  if (missing.length === 0) return;
  throw new Error(`missing ${missing.join(", ")}. Run: pnpm --filter blasphem run build`);
}

function packedPaths() {
  const output = execFileSync("pnpm", ["pack", "--dry-run", "--json"], { cwd: packageRoot, encoding: "utf8" });
  return JSON.parse(output).files.map((file) => file.path).toSorted();
}

function assertPaths(paths) {
  const missing = REQUIRED_FILES.filter((file) => !paths.includes(file));
  if (missing.length > 0) throw new Error(`the archive is missing ${missing.join(", ")}`);
  const forbidden = paths.filter((path) => FORBIDDEN_PREFIXES.some((prefix) => path.startsWith(prefix)));
  if (forbidden.length > 0) throw new Error(`the archive must not carry ${forbidden.join(", ")}`);
  const unexpected = paths.filter((path) => !REQUIRED_FILES.includes(path));
  if (unexpected.length > 0) throw new Error(`the archive carries unexpected files: ${unexpected.join(", ")}`);
}

assertManifest(readManifest());
assertDistribution();
const paths = packedPaths();
assertPaths(paths);
console.log(`status=packed files=${paths.length}`);
