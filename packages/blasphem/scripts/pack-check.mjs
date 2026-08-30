import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { resolve } from "node:path";
import { packageRoot, projectRoot } from "./crate.mjs";

const ENTRY_FILES = ["browser", "node", "native", "wasm-engine", "version.generated"].flatMap((name) => [`dist/${name}.js`, `dist/${name}.d.ts`]);
const GLUE_FILES = ["dist/blasphem.d.ts", "dist/blasphem.js", "dist/blasphem_bg.wasm", "dist/blasphem_bg.wasm.d.ts"];
const FORBIDDEN_PREFIXES = ["crates/", "data/", "reports/", "resources/", "src/", "target/"];
const FORBIDDEN_SUFFIXES = [".pack", ".detect", ".node"];
const NATIVE_PACKAGES = [
  "@blasphem/node-darwin-arm64",
  "@blasphem/node-darwin-x64",
  "@blasphem/node-linux-arm64-gnu",
  "@blasphem/node-linux-arm64-musl",
  "@blasphem/node-linux-x64-gnu",
  "@blasphem/node-linux-x64-musl",
  "@blasphem/node-win32-x64-msvc",
];

function readManifest() {
  return JSON.parse(readFileSync(resolve(packageRoot, "package.json"), "utf8"));
}

function coreFiles() {
  return readdirSync(resolve(projectRoot, "packages/core/src"))
    .filter((name) => name.endsWith(".ts"))
    .flatMap((name) => [`dist/core/${name.replace(/\.ts$/, ".js")}`, `dist/core/${name.replace(/\.ts$/, ".d.ts")}`]);
}

function requiredFiles() {
  return ["LICENSE", "NOTICE", "README.md", "package.json", "bin/blasphem-assets.mjs", ...ENTRY_FILES, ...GLUE_FILES, ...coreFiles()].toSorted();
}

function assertManifest(manifest) {
  if (manifest.name !== "blasphem") throw new Error(`the package name must be "blasphem", found "${manifest.name}"`);
  if (manifest.private !== true) throw new Error("the package must stay private");
  if (manifest.sideEffects !== false) throw new Error("sideEffects must be false");
  const entry = manifest.exports?.["."];
  const expected = { types: "./dist/browser.d.ts", browser: "./dist/browser.js", node: "./dist/node.js", default: "./dist/browser.js" };
  for (const [condition, target] of Object.entries(expected)) {
    if (entry?.[condition] !== target) throw new Error(`exports["."].${condition} must be ${target}`);
  }
  if (manifest.exports?.["./blasphem_bg.wasm"] !== "./dist/blasphem_bg.wasm") throw new Error('exports["./blasphem_bg.wasm"] must point at dist');
  const optional = Object.keys(manifest.optionalDependencies ?? {}).toSorted();
  if (optional.join(",") !== NATIVE_PACKAGES.join(",")) throw new Error(`optionalDependencies must list ${NATIVE_PACKAGES.join(", ")}`);
  if (manifest.dependencies !== undefined) throw new Error("the package must not declare dependencies; @blasphem/packs is the application's choice");
}

function assertDistribution() {
  const missing = ["dist/browser.js", "dist/node.js", "dist/blasphem.js", "dist/blasphem_bg.wasm"].filter((file) => !existsSync(resolve(packageRoot, file)));
  if (missing.length === 0) return;
  throw new Error(`missing ${missing.join(", ")}. Run: pnpm --filter blasphem run build`);
}

function packedPaths() {
  const output = execFileSync("pnpm", ["pack", "--dry-run", "--json"], { cwd: packageRoot, encoding: "utf8" });
  return JSON.parse(output).files.map((file) => file.path).toSorted();
}

function assertPaths(paths) {
  const required = requiredFiles();
  const missing = required.filter((file) => !paths.includes(file));
  if (missing.length > 0) throw new Error(`the archive is missing ${missing.join(", ")}`);
  const forbidden = paths.filter((path) => FORBIDDEN_PREFIXES.some((prefix) => path.startsWith(prefix)) || FORBIDDEN_SUFFIXES.some((suffix) => path.endsWith(suffix)));
  if (forbidden.length > 0) throw new Error(`the archive must not carry ${forbidden.join(", ")}`);
  const unexpected = paths.filter((path) => !required.includes(path));
  if (unexpected.length > 0) throw new Error(`the archive carries unexpected files: ${unexpected.join(", ")}`);
}

function assertBin(manifest) {
  if (manifest.bin?.["blasphem-assets"] !== "./bin/blasphem-assets.mjs") throw new Error("bin.blasphem-assets must point at ./bin/blasphem-assets.mjs");
}

function assertNoImportMetaUrlInBrowserPath() {
  for (const name of ["browser.js", "wasm-engine.js", "blasphem.js"]) {
    const source = readFileSync(resolve(packageRoot, "dist", name), "utf8");
    if (source.includes("import.meta.url")) throw new Error(`dist/${name} must not use import.meta.url; it is reachable through the browser condition`);
  }
  for (const name of readdirSync(resolve(packageRoot, "dist/core")).filter((file) => file.endsWith(".js"))) {
    const source = readFileSync(resolve(packageRoot, "dist/core", name), "utf8");
    if (source.includes("import.meta.url")) throw new Error(`dist/core/${name} must not use import.meta.url`);
  }
}

assertManifest(readManifest());
assertBin(readManifest());
assertDistribution();
assertNoImportMetaUrlInBrowserPath();
const paths = packedPaths();
assertPaths(paths);
console.log(`status=packed files=${paths.length}`);
